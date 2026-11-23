//! Organism program storage and execution bridge for the P2 slice.
//! A program may create world intents; only the kernel commits them.
use crate::{Action, ActionResult, Code, NodeId, Object, World, WorldState};
use ocee_organism_vm::{
    Continuation, Program, SliceResult, Value, VmError, WorldCall,
};

const VM_SLICE_GAS: u64 = 2_000;

fn ascii_hash(bytes: &[u8]) -> Result<String, Code> {
    let candidate=std::str::from_utf8(bytes).map_err(|_|Code::InvalidCode)?;
    if candidate.len()!=64 || !candidate.bytes()
        .all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)) {
        return Err(Code::InvalidCode);
    }
    Ok(candidate.to_owned())
}

fn decode_candidate(bytes:&[u8],hash:&str) -> Result<Program,Code> {
    let program:Program=serde_json::from_slice(bytes).map_err(|_|Code::InvalidCode)?;
    if program.canonical_bytes()!=bytes {return Err(Code::InvalidCode);}
    if program.content_hash().map_err(|_|Code::InvalidCode)?!=hash {
        return Err(Code::InvalidCode);
    }
    Ok(program)
}

fn stored_program(state:&WorldState,actor:u64,hash:&str)->Result<Program,Code> {
    let key=format!("/self/versions/{hash}");
    let object=state.objects.get(&key).ok_or(Code::NotFound)?;
    if object.owner!=actor {return Err(Code::Denied);}
    decode_candidate(&object.bytes,hash)
}

/// Validate special object writes before gas is debited, and return selected
/// program hash when the CAS write is for /self/active.
pub(crate) fn preflight_write(
    state:&WorldState, actor:u64, key:&str, bytes:&[u8], old:Option<&Object>
)->Result<Option<String>,Code> {
    if key=="/self/active" {
        let hash=ascii_hash(bytes)?;
        stored_program(state,actor,&hash)?;
        return Ok(Some(hash));
    }
    if let Some(hash)=key.strip_prefix("/self/versions/") {
        if old.is_some() {return Err(Code::Conflict);} // version objects are immutable
        let claimed=ascii_hash(hash.as_bytes())?;
        decode_candidate(bytes,&claimed)?;
    }
    if key.starts_with("/self/tools/") {
        let hash=ascii_hash(bytes)?;
        stored_program(state,actor,&hash)?;
    }
    Ok(None)
}

pub(crate) fn after_write(state:&mut WorldState, actor:u64, active:Option<String>) {
    if let Some(hash)=active {
        if state.initial_root_hash.is_none() {
            state.initial_root_hash=Some(hash.clone());
        }
        if let Some(organism)=state.organisms.get_mut(&actor) {
            organism.active_program_hash=Some(hash);
            organism.vm_continuation=None;
            organism.pending_feedback=None;
            organism.last_return=None;
        }
    }
}

/// Isolated candidate EXEC: pure computation plus authorized snapshot reads.
/// Reads consume gas but neither world steps nor externally visible effects.
pub(crate) fn isolated_exec(world:&mut World, actor:u64, hash:&str, input:&[u8])->ActionResult {
    let program=match stored_program(&world.state,actor,hash) {
        Ok(p)=>p,
        Err(code)=>return ActionResult::fail(code),
    };
    let mut continuation=match Continuation::start(&program,input.to_vec()) {
        Ok(c)=>c,
        Err(_)=>return ActionResult::fail(Code::InvalidCode),
    };
    let budget=world.state.nodes.get(&NodeId::A).expect("A")
        .gas_available.min(VM_SLICE_GAS);
    let mut consumed=0u64;
    let mut feedback:Option<Vec<u8>>=None;

    // Static P2 graph is acyclic; this is an additional hard action count bound.
    for _ in 0..128 {
        let slice=continuation.resume(&program,feedback.take(),budget-consumed);
        let (gas,code,output,next)=match slice {
            Ok(SliceResult::Returned{output,gas_used})=>
                (gas_used,Code::Ok,Some(serde_json::to_vec(&output).expect("value")),None),
            Ok(SliceResult::Exhausted{gas_used})=>
                (gas_used,Code::Gas,None,None),
            Ok(SliceResult::Faulted{error,gas_used})=>
                (gas_used,match error {
                    VmError::OutputTooLarge=>Code::Quota,
                    _=>Code::InvalidCode,
                },None,None),
            Ok(SliceResult::Yielded{call,args,gas_used})=>
                (gas_used,Code::Ok,None,Some((call,args))),
            Err(_)=> (0,Code::InvalidCode,None,None),
        };
        consumed+=gas;
        if let Some((call,args))=next {
            let observation:Result<ActionResult,Code>=match call {
                WorldCall::Observe if args.is_empty()=>{
                    let a=&world.state.nodes[&NodeId::A];
                    let sample=if world.state.protocol_service.is_some() {
                        Some("/public/C/hello")
                    }else{None};
                    let view=serde_json::to_vec(&(world.state.step,a.gas_available,
                        a.storage_capacity,sample)).expect("observable");
                    Ok(ActionResult::ok(view,1,0))
                },
                WorldCall::Read if args.len()==1=>{
                    let key=utf8(&args[0]);
                    key.and_then(|key|{
                        let object=world.state.objects.get(&key).ok_or(Code::NotFound)?;
                        if key!="/public/C/hello" &&
                            (object.owner!=actor || !key.starts_with("/self/")) {
                            return Err(Code::Denied);
                        }
                        if object.bytes.len()>4096 {return Err(Code::Quota);}
                        let fee=1+(object.bytes.len() as u64).saturating_add(63)/64;
                        Ok(ActionResult::ok(object.bytes.clone(),fee,0))
                    })
                },
                _=>Err(Code::IsolatedSideEffect),
            };
            match observation {
                Ok(r)=>{
                    if consumed.saturating_add(r.gas_charged)>budget {
                        return charge_isolated(world,consumed,Code::Gas,vec![]);
                    }
                    consumed+=r.gas_charged;
                    feedback=Some(serde_json::to_vec(&r).expect("read feedback"));
                    continue;
                },
                Err(code)=>return charge_isolated(world,consumed,code,vec![]),
            }
        }
        return charge_isolated(world,consumed,code,output.unwrap_or_default());
    }
    charge_isolated(world,consumed,Code::Gas,vec![])
}

fn charge_isolated(world:&mut World,gas:u64,code:Code,payload:Vec<u8>)->ActionResult {
    if let Err(err)=world.spend_a_gas(gas){return ActionResult::fail(err);}
    let mut result=ActionResult::fail(code);
    result.payload=payload;
    result.gas_charged=gas;
    result
}

fn as_bytes(v:&Value)->Result<&[u8],Code> {
    if let Value::Bytes(bytes)=v {Ok(bytes)} else {Err(Code::BadRequest)}
}
fn as_num(v:&Value)->Result<u32,Code> {
    if let Value::U32(num)=v {Ok(*num)} else {Err(Code::BadRequest)}
}
fn utf8(v:&Value)->Result<String,Code> {
    String::from_utf8(as_bytes(v)?.to_vec()).map_err(|_|Code::BadRequest)
}
fn into_action(actor:u64,call:WorldCall,args:&[Value])->Result<Action,Code> {
    let action=match call {
        WorldCall::Observe if args.is_empty() => Action::Observe{actor},
        WorldCall::Read if args.len()==1 => Action::Read{
            actor,key:utf8(&args[0])?
        },
        WorldCall::Write if args.len()==3 => {
            let version=as_num(&args[2])?;
            Action::Write{
                actor,key:utf8(&args[0])?,
                bytes:as_bytes(&args[1])?.to_vec(),
                expected_version:if version==u32::MAX{None}else{Some(u64::from(version))},
            }
        },
        WorldCall::PublishTool if args.len()==3=>{
            let version=as_num(&args[2])?;
            Action::PublishTool{
                actor,registry_key:utf8(&args[0])?,
                program_bytes:as_bytes(&args[1])?.to_vec(),
                expected_version:if version==u32::MAX{
                    None
                }else{Some(u64::from(version))},
            }
        },
        WorldCall::ReclaimVersion if args.len()==1=>{
            Action::ReclaimVersion{actor,program_hash:utf8(&args[0])?}
        },
        WorldCall::Send if args.len()==1 => Action::Send{
            actor,destination:NodeId::C,bytes:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::Receive if args.is_empty()=>Action::Receive{actor,max_count:1},
        WorldCall::Exec if args.len()==2=>Action::Exec{
            actor,program_hash:utf8(&args[0])?,input:as_bytes(&args[1])?.to_vec()
        },
        WorldCall::Probe if args.len()==1=>Action::Probe{
            actor,input:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::VerifyProbe if args.len()==1=>Action::VerifyProbe{
            actor,evidence:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::RenewProbe if args.len()==1=>Action::RenewProbe{
            actor,evidence:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::ArmTaskFence if args.len()==1=>Action::ArmTaskFence{
            actor,evidence:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::FenceStatus if args.is_empty()=>Action::FenceStatus{actor},
        WorldCall::RestartRoot if args.is_empty()=>Action::RestartRoot{actor},
        WorldCall::FenceStatusFor if args.len()==1=>{
            let raw:[u8;8]=as_bytes(&args[0])?.try_into()
                .map_err(|_|Code::BadRequest)?;
            Action::FenceStatusFor{actor,request_id:u64::from_le_bytes(raw)}
        },
        WorldCall::RestartRootFor if args.len()==1=>{
            let raw:[u8;8]=as_bytes(&args[0])?.try_into()
                .map_err(|_|Code::BadRequest)?;
            Action::RestartRootFor{actor,request_id:u64::from_le_bytes(raw)}
        },
        WorldCall::RequestOutcomeFor if args.len()==1=>{
            let raw:[u8;8]=as_bytes(&args[0])?.try_into()
                .map_err(|_|Code::BadRequest)?;
            Action::RequestOutcomeFor{actor,request_id:u64::from_le_bytes(raw)}
        },
        WorldCall::ConfirmResponseFor if args.len()==2=>{
            let raw:[u8;8]=as_bytes(&args[0])?.try_into()
                .map_err(|_|Code::BadRequest)?;
            Action::ConfirmResponseFor{
                actor,request_id:u64::from_le_bytes(raw),
                reply:as_bytes(&args[1])?.to_vec(),
            }
        },
        _=>return Err(Code::BadRequest),
    };
    Ok(action)
}
pub(crate) struct VmTick {
    pub action:Option<Action>,
    pub gas_used:u64,
    pub status:Code,
}

/// Called from the single tick pipeline after due events and epoch grant.
impl World {
    pub(crate) fn prepare_organism_action(&mut self,actor:u64)->VmTick {
        let Some(host)=self.state.organisms.get(&actor) else {
            return VmTick{action:None,gas_used:0,status:Code::Denied};
        };
        let Some(hash)=host.active_program_hash.clone() else {
            return VmTick{action:None,gas_used:0,status:Code::Empty};
        };
        let program=match stored_program(&self.state,actor,&hash) {
            Ok(p)=>p,
            Err(code)=>return VmTick{action:None,gas_used:0,status:code},
        };
        let mut cont=match host.vm_continuation.clone() {
            Some(c)=>c,
            None=>match Continuation::start(&program,vec![]){
                Ok(c)=>c,
                Err(_)=>return VmTick{action:None,gas_used:0,status:Code::InvalidCode},
            },
        };
        if cont.finished {
            return VmTick{action:None,gas_used:0,status:Code::Empty};
        }
        let feedback=host.pending_feedback.clone();
        let gas_budget=self.state.nodes.get(&NodeId::A).expect("A")
            .gas_available.min(VM_SLICE_GAS);
        let slice=match cont.resume(&program,feedback,gas_budget) {
            Ok(s)=>s,
            Err(_)=>return VmTick{action:None,gas_used:0,status:Code::InvalidCode},
        };
        let used=match &slice {
            SliceResult::Returned{gas_used,..}|
            SliceResult::Yielded{gas_used,..}|
            SliceResult::Exhausted{gas_used}|
            SliceResult::Faulted{gas_used,..}=>*gas_used,
        };
        // debit all VM microsteps before the external action starts
        if self.spend_a_gas(used).is_err() {
            return VmTick{action:None,gas_used:0,status:Code::Gas};
        }
        let host=self.state.organisms.get_mut(&actor).expect("actor still exists");
        host.vm_continuation=Some(cont);
        host.pending_feedback=None;
        match slice {
            SliceResult::Returned{output,..}=>{
                host.last_return=Some(output);
                VmTick{action:None,gas_used:used,status:Code::Ok}
            },
            SliceResult::Exhausted{..}=>VmTick{action:None,gas_used:used,status:Code::Gas},
            SliceResult::Faulted{..}=>VmTick{action:None,gas_used:used,status:Code::InvalidCode},
            SliceResult::Yielded{call,args,..}=>{
                match into_action(actor,call,&args) {
                    Ok(action)=>VmTick{action:Some(action),gas_used:used,status:Code::Ok},
                    Err(code)=>{
                        host.pending_feedback=Some(serde_json::to_vec(&ActionResult::fail(code))
                            .expect("feedback"));
                        VmTick{action:None,gas_used:used,status:code}
                    },
                }
            }
        }
    }
}


/// Single world action that commits BOTH an immutable content-addressed
/// program and its owned tool-registry CAS. All checks and the aggregate
/// A fee happen before either object or storage_used is modified.
/// No extra filesystem/host capability is exposed to the organism.
pub(crate) fn atomic_publish_tool(
    world:&mut World,actor:u64,registry_key:String,
    program_bytes:Vec<u8>,expected_version:Option<u64>
)->ActionResult {
    if !registry_key.starts_with("/self/tools/") {
        return ActionResult::fail(Code::Denied);
    }
    let old_registry=world.state.objects.get(&registry_key);
    if old_registry.is_some_and(|o|o.owner!=actor) {
        return ActionResult::fail(Code::Denied);
    }
    let previous_version=old_registry.map(|o|o.version);
    if previous_version!=expected_version {
        return ActionResult::fail(Code::Conflict);
    }
    let new_registry_version=match previous_version {
        None=>0,
        Some(v)=>match v.checked_add(1) {
            Some(n)=>n,
            None=>return ActionResult::fail(Code::BadRequest),
        },
    };
    let program:Program=match serde_json::from_slice(&program_bytes) {
        Ok(p)=>p,
        Err(_)=>return ActionResult::fail(Code::InvalidCode),
    };
    if program.canonical_bytes()!=program_bytes {
        return ActionResult::fail(Code::InvalidCode);
    }
    let hash=match program.content_hash() {
        Ok(h)=>h,
        Err(_)=>return ActionResult::fail(Code::InvalidCode),
    };
    // Already content-addressed and domain-separated by Program::content_hash.
    let version_key=format!("/self/versions/{hash}");
    let existing=world.state.objects.get(&version_key);
    if existing.is_some_and(|o|o.owner!=actor) {
        return ActionResult::fail(Code::Denied);
    }
    if existing.is_some_and(|o|o.bytes!=program_bytes) {
        return ActionResult::fail(Code::InvalidCode);
    }
    let old_registry_len=old_registry.map_or(0,|o|o.bytes.len() as u64);
    let needs_version=existing.is_none();
    let install_bytes=if needs_version{program_bytes.len() as u64}else{0};
    let used=world.state.organisms.get(&actor)
        .expect("valid actor has home storage").storage_used;
    let next_used=used.checked_sub(old_registry_len)
        .and_then(|n|n.checked_add(64))
        .and_then(|n|n.checked_add(install_bytes));
    let Some(next_used)=next_used else{
        return ActionResult::fail(Code::Quota);
    };
    let cap=world.state.nodes[&NodeId::A].storage_capacity;
    if next_used>cap{return ActionResult::fail(Code::Quota);}
    // Exactly the cost of the previous pair of real writes, with no
    // redundant immutable-program write when that hash already exists.
    let registry_fee=2+64u64.div_ceil(16);
    let install_fee=if needs_version {
        2+(program_bytes.len() as u64).div_ceil(16)
    }else{0};
    let fee=registry_fee+install_fee;
    if let Err(code)=world.spend_a_gas(fee) {
        return ActionResult::fail(code);
    }
    // No fallible action after debit. One deterministic tick atomically
    // changes the two objects, registry version and charged storage.
    if needs_version {
        world.state.objects.insert(version_key,Object{
            owner:actor,version:0,bytes:program_bytes
        });
    }
    world.state.objects.insert(registry_key,Object{
        owner:actor,version:new_registry_version,
        bytes:hash.as_bytes().to_vec()
    });
    world.state.organisms.get_mut(&actor)
        .expect("valid actor").storage_used=next_used;
    ActionResult::ok(hash.into_bytes(),fee,0)
}


/// Reclaim one actor-owned immutable version *after* checking world-known
/// references in the same deterministic tick. This is intentionally
/// conservative, not a general tracing GC for arbitrary encoded pointers.
/// In particular, the currently running REQUESTING root's own immutable
/// bytecode is exempt from the raw-string scan because it necessarily
/// contains the to-be-retired hash as its action argument; active/initial
/// root identity itself is still protected independently.
pub(crate) fn reclaim_owned_version(
    world:&mut World,actor:u64,hash:&str
)->ActionResult {
    let hash=match ascii_hash(hash.as_bytes()){
        Ok(s)=>s,
        Err(code)=>return ActionResult::fail(code),
    };
    let version_key=format!("/self/versions/{hash}");
    let Some(target)=world.state.objects.get(&version_key) else{
        return ActionResult::fail(Code::NotFound);
    };
    if target.owner!=actor{return ActionResult::fail(Code::Denied);}
    if decode_candidate(&target.bytes,&hash).is_err(){
        return ActionResult::fail(Code::InvalidCode);
    }
    if world.state.initial_root_hash.as_deref()==Some(hash.as_str()) ||
        world.state.organisms.values().any(|o|
            o.active_program_hash.as_deref()==Some(hash.as_str()) ||
            o.vm_continuation.as_ref().is_some_and(|c|
                c.program_hash==hash))
    {
        return ActionResult::fail(Code::Conflict);
    }
    let requester's_active=world.state.organisms.get(&actor)
        .and_then(|o|o.active_program_hash.as_ref())
        .map(|active|format!("/self/versions/{active}"));
    let contains_hash=|bytes:&[u8]|->bool{
        bytes.windows(64).any(|w|w==hash.as_bytes())
    };
    let mut scanned_bytes=0u64;
    for (key,obj) in &world.state.objects{
        if key==&version_key{continue;}
        // The invoking program may contain the target literal solely to
        // request collection. Caller is responsible not to EXEC it again.
        if requester's_active.as_deref()==Some(key.as_str()){continue;}
        scanned_bytes=match scanned_bytes
            .checked_add(key.len() as u64)
            .and_then(|v|v.checked_add(obj.bytes.len() as u64)){
                Some(n)=>n,
                None=>return ActionResult::fail(Code::Quota),
            };
        if contains_hash(key.as_bytes())||contains_hash(&obj.bytes){
            return ActionResult::fail(Code::Conflict);
        }
    }
    // Transport can contain an old executable version identity that
    // will be consumed later. A historical audit record is not a lease.
    for msg in world.state.pending_deliveries.iter().chain(
        world.state.mailboxes.values().flatten()){
        scanned_bytes=match scanned_bytes.checked_add(msg.bytes.len() as u64){
            Some(n)=>n,None=>return ActionResult::fail(Code::Quota),
        };
        if contains_hash(&msg.bytes){return ActionResult::fail(Code::Conflict);}
    }
    let target_len=target.bytes.len() as u64;
    let used=world.state.organisms[&actor].storage_used;
    let Some(next_used)=used.checked_sub(target_len) else{
        return ActionResult::fail(Code::Quota);
    };
    let scan_fee=1+scanned_bytes.div_ceil(64);
    let reclaim_fee=2+target_len.div_ceil(64);
    let Some(fee)=scan_fee.checked_add(reclaim_fee) else{
        return ActionResult::fail(Code::Quota);
    };
    if let Err(code)=world.spend_a_gas(fee){
        return ActionResult::fail(code);
    }
    // No fallible operations beyond this point; object and used accounting
    // are applied within the SAME ordinary audited world action.
    world.state.objects.remove(&version_key);
    world.state.organisms.get_mut(&actor)
        .expect("validated actor").storage_used=next_used;
    ActionResult::ok(target_len.to_le_bytes().to_vec(),fee,0)
}
