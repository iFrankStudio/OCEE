//! Organism program storage and execution bridge for the P2 slice.
//! A program may create world intents; only the kernel commits them.
use crate::{Action, ActionResult, Code, NodeId, Object, World, WorldState};
use ocee_organism_vm::{
    Continuation, Program, SliceResult, Value, VmError, WorldCall, MAX_VALUE_BYTES,
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
        if let Some(organism)=state.organisms.get_mut(&actor) {
            organism.active_program_hash=Some(hash);
            organism.vm_continuation=None;
            organism.pending_feedback=None;
        }
    }
}

/// Pure isolated EXEC for the initial P2 slice. Authorized snapshot READ during
/// EXEC is deferred; all yielded world I/O is rejected rather than performed.
pub(crate) fn isolated_exec(world:&mut World, actor:u64, hash:&str)->ActionResult {
    let program=match stored_program(&world.state,actor,hash) {
        Ok(p)=>p,
        Err(code)=>return ActionResult::fail(code),
    };
    let mut continuation=match Continuation::start(&program,vec![]) {
        Ok(c)=>c,
        Err(_)=>return ActionResult::fail(Code::InvalidCode),
    };
    let gas=world.state.nodes.get(&NodeId::A).expect("A").gas_available.min(VM_SLICE_GAS);
    let result=match continuation.resume(&program,None,gas) {
        Ok(v)=>v,
        Err(VmError::OutputTooLarge)=>return ActionResult::fail(Code::Quota),
        Err(_)=>return ActionResult::fail(Code::InvalidCode),
    };
    match result {
        SliceResult::Returned{output,gas_used}=>{
            if let Err(code)=world.spend_a_gas(gas_used){return ActionResult::fail(code);}
            ActionResult::ok(serde_json::to_vec(&output).expect("value"),gas_used,0)
        },
        SliceResult::Yielded{gas_used,..}=>{
            if let Err(code)=world.spend_a_gas(gas_used){return ActionResult::fail(code);}
            let mut r=ActionResult::fail(Code::IsolatedSideEffect);
            r.gas_charged=gas_used;
            r
        },
        SliceResult::Exhausted{gas_used}=>{
            if let Err(code)=world.spend_a_gas(gas_used){return ActionResult::fail(code);}
            let mut r=ActionResult::fail(Code::Gas);
            r.gas_charged=gas_used;
            r
        }
    }
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
        WorldCall::Send if args.len()==1 => Action::Send{
            actor,destination:NodeId::C,bytes:as_bytes(&args[0])?.to_vec()
        },
        WorldCall::Receive if args.is_empty()=>Action::Receive{actor,max_count:1},
        WorldCall::Exec if args.len()==1=>Action::Exec{
            actor,program_hash:utf8(&args[0])?
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
            SliceResult::Exhausted{gas_used}=>*gas_used,
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
                VmTick{action:None,gas_used:used,status:if matches!(output,Value::Bytes(ref x) if x.len()>MAX_VALUE_BYTES){
                    Code::Quota
                }else{Code::Ok}}
            },
            SliceResult::Exhausted{..}=>VmTick{action:None,gas_used:used,status:Code::Gas},
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
