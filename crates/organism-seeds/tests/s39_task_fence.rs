//! S39: task-fenced remote messages reject drift both before SEND and
//! when C consumes an already-sent B request. The root installs a fence
//! only from a genuinely issued active VM Probe receipt.
use ocee_organism_seeds::{
    s36_active_probe::{s36_active_probe_root,
        s36_no_probe_control,s36_probe_bank_root,EVIDENCE},
    s37_evidence_guard::s37_verified_probe_root,
    s38_task_recovery::{s38_task_recovery_root,s38_no_renew_control},
    s39_task_fenced_remote::s39_task_fenced_remote_root,
    s33_adaptive_depth::DEPTH_TASK,
    s32_composite_b_operand::{
        s32_p1_composite_consumer,s32_p2_composite_consumer,REMOTE_VALUES},
    s30_composed_transfer::REGISTRY,
    s31_causal_remote::PREPARED_COUNT,
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::{Family,Operation,fold32};
use ocee_world_kernel::{
    Action,Code,Config,NodeId,Object,TransformExample,TransformTask,
    World,ORGANISM_ID,hidden_compose,
};

fn rule(seed:u64,bytes:&[u8])->u32{
    hidden_compose(seed,bytes).wrapping_add(
        ((seed%54)/18) as u32 * bytes.len() as u32)
}
fn install_public_fixtures(w:&mut World,seed:u64){
    // The ONLY TWO PREPUBLISHED LABELS intentionally cannot distinguish any of
    // the three unary primitives. It is not enough to match both labels.
    let a=vec![1];let b=vec![2,0];
    for input in [&a,&b] {
        assert_eq!(input.len() as u32,
            input.iter().fold(0u32,|acc,&x|acc^u32::from(x)));
        assert_eq!(input.len() as u32,u32::from(input[0]));
    }
    let task=TransformTask{
        task_id:"s36-interactive-probe-v1".into(),
        examples:vec![
            TransformExample{input:a.clone(),output:rule(seed,&a)},
            TransformExample{input:b.clone(),output:rule(seed,&b)},
        ],
        held_out_input:vec![4,5,6,7,8],
    };
    assert!(w.state.objects.insert(DEPTH_TASK.into(),Object{
        owner:0,version:0,bytes:serde_json::to_vec(&task).unwrap()
    }).is_none());

}
fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None,
    })).code,Code::Ok);
}
fn a_gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn boot(family:Family,seed:u64,enhanced:bool,allow_query:bool)->(World,String){
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let a=p1.content_hash().unwrap();
    let b=p2.content_hash().unwrap();
    let root=if enhanced {
        s39_task_fenced_remote_root(&a,&b)
    } else {
        s36_no_probe_control(&a,&b)
    };
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    install_public_fixtures(&mut w,seed);
    // Researcher's explicit query-unavailable control, never a change to
    // the organism's frozen initial program.
    if !allow_query {w.state.probe_count=1;}
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run_until_switch_or_end(w:&mut World){
    for _ in 0..12_000 {
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected S36 VM trap step={} code={:?} block={:?}",
            w.state.step,r.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .map(|x|(&x.block,x.pc)));
        if w.state.objects["/self/active"].version==1 ||
           w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished) {break;}
    }
}
fn finish(w:&mut World){
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected S36 specialist VM trap step={} code={:?}",
            w.state.step,r.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded VM must finish");
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn stored(w:&World)->(String,Program){
    let h=String::from_utf8(w.state.objects[REGISTRY].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{h}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),h);
    (h,p)
}
fn fourth(w:&World)->u32{
    let b=&w.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(b.len(),16);
    u32::from_le_bytes(b[12..16].try_into().unwrap())
}
fn probe_actions(w:&World)->usize{
    w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::Probe{..}))).count()
}
fn successful_probes(w:&World)->usize{
    w.audit.iter().filter(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Probe{..}))).count()
}
fn cache_reads(w:&World)->usize{
    w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::Read{key,..}) if key==EVIDENCE)).count()
}
fn cache_writes(w:&World)->usize{
    w.audit.iter().filter(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,..})
            if key==EVIDENCE)).count()
}
fn assert_acquired_evidence(w:&World,seed:u64,probe_input:&[u8]){
    assert_eq!(w.state.probe_count,1);
    assert_eq!(probe_actions(w),1,"one real query, not a fabricated label");
    assert_eq!(successful_probes(w),1);
    assert_eq!(cache_writes(w),1);
    assert!(!w.state.objects.contains_key(
        "/public/E001/compose-counterexample"),
        "S36 third public label must not have been prepublished");
    let action=w.audit.iter().find(|a|matches!(&a.action,
        Some(Action::Probe{..}))).unwrap();
    let Some(Action::Probe{input,..})=&action.action
        else {unreachable!()};
    assert_eq!(input,probe_input,
        "VM-selected query must be visible in actual world action");
    assert_eq!(action.result.code,Code::Ok);
    assert_eq!(action.result.gas_charged,
        40+4*(probe_input.len() as u64));
    let reply:TransformExample=serde_json::from_slice(
        &action.result.payload).unwrap();
    assert_eq!(reply.input,probe_input);
    assert_eq!(reply.output,rule(seed,probe_input));
    let cached:TransformExample=serde_json::from_slice(
        &w.state.objects[EVIDENCE].bytes).unwrap();
    assert_eq!(cached,reply);
    let cached_at=w.audit.iter().find(|a|
        matches!(&a.action,Some(Action::Write{key,..})
            if key==EVIDENCE)).unwrap().step;
    assert!(action.step<cached_at);
    let acquired_at=w.audit.iter().position(|a|
        matches!(&a.action,Some(Action::Probe{..}))).unwrap();
    let evaluated_after=w.audit.iter().skip(acquired_at+1).any(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Exec{..})));
    assert!(evaluated_after,"queried label must be used in actual candidate EXEC");
}

fn verification_calls(w:&World)->usize{
    w.audit.iter().filter(|r|matches!(&r.action,
        Some(Action::VerifyProbe{..}))).count()
}
fn verification_code(w:&World,code:Code)->usize{
    w.audit.iter().filter(|r|r.result.code==code &&
        matches!(&r.action,Some(Action::VerifyProbe{..}))).count()
}

fn candidate_writes(w:&World)->usize{
    // The root later generates a separate S28 Len utility. Its version
    // WRITE must NOT be counted as another S36 search candidate.
    // Every searched candidate is written before the synthesized fourth
    // operand is stored in REMOTE_VALUES.
    let finish=w.audit.iter().find(|a|
        matches!(&a.action,Some(Action::Write{key,..})
            if key==REMOTE_VALUES)).map(|a|a.step).unwrap_or(u64::MAX);
    w.audit.iter().filter(|a|a.step>=4 && a.step<finish &&
        matches!(&a.action,Some(Action::Write{key,..})
            if key.starts_with("/self/versions/"))).count()
}
fn check_constructed(w:&World,seed:u64,stages:u32)->(String,Program){
    assert_eq!(w.state.objects["/self/active"].version,1);
    assert_eq!(w.state.objects[PREPARED_COUNT].bytes,[4,0]);
    let (h,p)=stored(w);
    let ins=&p.functions[0].blocks[0].instructions;
    assert_eq!(ins.len(),(4+2*stages) as usize);
    let input:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    assert_eq!(fourth(w),rule(seed,&input.held_out_input),
        "third public counterexample must resolve false generalization");
    (h,p)
}
fn real_b(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_gas(w),24);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let frames=w.audit.iter().filter_map(|a|{
        if let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action{
            if let Ok(f)=decoder.decode(bytes) {
                if f.operation==Operation::RemoteEval && f.body.len()==18 &&
                    u16::from_le_bytes(f.body[..2].try_into().unwrap())==4{
                    return Some(f.body);
                }
            }
        }
        None
    }).collect::<Vec<_>>();
    let frame=frames.last().expect("real remote frame");
    assert_eq!(&frame[2..],w.state.objects[REMOTE_VALUES].bytes.as_slice());
    let values=frame[2..].chunks_exact(4)
        .map(|b|u32::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    let key=match family{
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!(),
    };
    let actual=u32::from_le_bytes(w.state.objects[key].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(actual,fold32(&values).unwrap());
}
fn import(w:&mut World,h:&str,p:&Program,key:&str)->u64{
    let before=a_gas(w);
    put(w,format!("/self/versions/{h}"),p.canonical_bytes());
    put(w,key.into(),h.as_bytes().to_vec());
    a_gas(w)-before
}
fn boot_control_s37(family:Family,seed:u64)->(World,String){
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let root=s38_no_renew_control(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    install_public_fixtures(&mut w,seed);
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run_until_cached(w:&mut World){
    for _ in 0..15_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if w.audit.last().is_some_and(|a|a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Write{key,..})
                if key==EVIDENCE)){
            return;
        }
    }
    panic!("active root never saved the queried label");
}
fn run_until_original_probe_scored(w:&mut World){
    // After the first paid query+cache WRITE, its authentic Probe
    // receipt stays in the VM continuation. Wait for that first
    // candidate's actual third-label EXEC before contaminating the
    // persistent cache: the NEXT candidate must READ the poisoned
    // object, not reuse the legitimate in-flight receipt.
    let start=w.audit.len();
    for _ in 0..15_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if w.audit.iter().skip(start).any(|a|a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Exec{..}))){
            return;
        }
    }
    panic!("first challenge EXEC after legitimate query never occurred");
}
fn fake_label(input:&[u8],output:u32)->Vec<u8>{
    serde_json::to_vec(&TransformExample{input:input.to_vec(),output})
        .unwrap()
}
fn change_world_task_rule(w:&mut World){
    // Explicit researcher-side privileged intervention; no /public WRITE
    // is available to the organism. This genuinely changes the kernel
    // oracle rule, not merely the task description.
    let obj=w.state.objects.get_mut(DEPTH_TASK).expect("task fixture");
    let mut task:TransformTask=serde_json::from_slice(&obj.bytes).unwrap();
    assert_eq!(task.task_id,"s36-interactive-probe-v1");
    task.task_id="s37-changed-rule-v2".into();
    for ex in &mut task.examples{ex.output=2*ex.input.len() as u32;}
    obj.bytes=serde_json::to_vec(&task).unwrap();
    obj.version+=1;
}
fn task_truth(w:&World,input:&[u8])->u32{
    let task:TransformTask=serde_json::from_slice(
        &w.state.objects[DEPTH_TASK].bytes).unwrap();
    if task.task_id=="s37-changed-rule-v2"{
        2*input.len() as u32
    }else{rule(w.state.config.seed,input)}
}

/// An organism-owned cache object can be forged without any paid query.
/// The old S36 root will trust it and successfully perform B with the
/// wrong operand. The S37 root must refuse it using kernel provenance.

fn renews(w:&World)->Vec<&ocee_world_kernel::AuditRecord>{
    w.audit.iter().filter(|a|
        matches!(&a.action,Some(Action::RenewProbe{..}))).collect()
}
fn cached_receipt(w:&World)->Vec<u8>{
    w.state.objects[EVIDENCE].bytes.clone()
}
fn s38_post_recovery_invariants(w:&World,seed:u64){
    assert_eq!(w.state.probe_count,2);
    assert_eq!(probe_actions(w),2);
    assert_eq!(successful_probes(w),2);
    assert_eq!(cache_writes(w),2);
    assert_eq!(w.state.objects[EVIDENCE].version,1);
    assert_eq!(renews(w).len(),1);
    assert_eq!(renews(w)[0].result.code,Code::Ok);
    assert_eq!(renews(w)[0].result.gas_charged,7);
    assert_eq!(verification_code(w,Code::Conflict),1);
    assert!(verification_code(w,Code::Ok)>=1);
    assert!(w.state.probe_renewed_for.is_some());
    let probes=w.audit.iter().filter(|r|
        matches!(&r.action,Some(Action::Probe{..}))).collect::<Vec<_>>();
    assert_eq!(probes.len(),2);
    assert!(probes[0].step < renews(w)[0].step &&
        renews(w)[0].step<probes[1].step);
    for p in &probes {
        assert_eq!(p.result.gas_charged,52);
        assert!(matches!(&p.action,Some(Action::Probe{input,..}) if input==&[7,8,9]));
    }
    let v1:TransformExample=serde_json::from_slice(&probes[0].result.payload).unwrap();
    let v2:TransformExample=serde_json::from_slice(&probes[1].result.payload).unwrap();
    assert_eq!(v1.output,rule(seed,&[7,8,9]));
    assert_eq!(v2.output,6);
    assert_ne!(v1.output,v2.output);
    let cached:TransformExample=serde_json::from_slice(&cached_receipt(w)).unwrap();
    assert_eq!(cached,v2);
    assert_eq!(w.state.probe_record.as_ref().unwrap().receipt,cached_receipt(w));
    let task_hash=w.state.probe_record.as_ref().unwrap().task_hash.clone();
    assert_eq!(w.state.probe_renewed_for.as_ref(),Some(&task_hash));
    let second_write=w.audit.iter().find(|r|
        matches!(&r.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==EVIDENCE));
    assert!(second_write.is_some(),"new evidence requires real paid CAS v0->v1");
    assert_eq!(second_write.unwrap().result.code,Code::Ok);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}



fn old_boot(family:Family,seed:u64)->(World,String){
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let root=s38_task_recovery_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    install_public_fixtures(&mut w,seed);
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn arm_count(w:&World)->usize {
    w.audit.iter().filter(|r|matches!(&r.action,
        Some(Action::ArmTaskFence{..}))).count()
}
fn wait_until_armed(w:&mut World){
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if w.state.task_fences.contains_key(&ORGANISM_ID){
            let latest=w.audit.last().unwrap();
            assert!(matches!(&latest.action,Some(Action::ArmTaskFence{..})));
            assert_eq!(latest.result.code,Code::Ok);
            assert_eq!(latest.result.gas_charged,3);
            assert_eq!(arm_count(w),1);
            assert_eq!(w.state.task_fences[&ORGANISM_ID],
                w.state.probe_record.as_ref().unwrap().task_hash);
            return
        }
    }
    panic!("active VM never armed a task-version fence");
}
fn until_remote_values(w:&mut World){
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if w.state.objects.contains_key(REMOTE_VALUES){return;}
    }
    panic!("candidate never generated actual B operand");
}
fn real_job_send_id(w:&mut World)->u64{
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "trap during specialist step={} {:?}",w.state.step,r.code);
        if let Some(a)=w.audit.last(){
            if a.result.code!=Code::Ok{continue;}
            let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
                else{continue;};
            let p=&w.state.protocol_service.as_ref().unwrap().protocol;
            if let Ok(frame)=p.decode(bytes){
                if frame.operation==Operation::RemoteEval &&
                    frame.body.len()==18 &&
                    u16::from_le_bytes(frame.body[0..2].try_into().unwrap())==4{
                    assert_eq!(a.result.payload.len(),8);
                    return u64::from_le_bytes(a.result.payload[..8].try_into().unwrap());
                }
            }
        }
    }
    panic!("no real C remote FOLD32 SEND from active specialist");
}
fn drive_bounded(w:&mut World,n:usize){
    for _ in 0..n{
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected VM trap at tick {} {:?}",w.state.step,r.code);
    }
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}

#[test]
fn s39_normal_fenced_cross_family_transfer_real_b_and_v2_relearn(){
    let (mut p1,root)=boot(Family::P1,1,true,true);
    run_until_switch_or_end(&mut p1);
    let (hash,program)=stored(&p1);
    assert_eq!(p1.state.task_fences.len(),1);
    let (mut p2,root2)=boot(Family::P2,55,true,true);
    assert_eq!(root,root2);
    let paid=import(&mut p2,&hash,&program,REGISTRY);
    run_until_switch_or_end(&mut p2);
    assert_eq!(stored(&p2).0,hash);
    assert_eq!(p2.state.task_fences.len(),1);
    finish(&mut p1);finish(&mut p2);
    real_b(&p1,Family::P1);real_b(&p2,Family::P2);
    for world in [&p1,&p2]{
        assert_eq!(arm_count(world),1);
        assert_eq!(world.state.task_fences[&ORGANISM_ID],
            world.state.probe_record.as_ref().unwrap().task_hash);
        assert!(world.state.guarded_sends.is_empty(),
            "all guarded remote messages must have been consumed");
        assert!(world.state.fence_drops.is_empty());
    }
    println!("S39 normal task fence root={root} source_A={} target_A={} import_A={paid} B_each24",a_gas(&p1),a_gas(&p2));

    // Preserves S38's bounded active re-query and new-version capability.
    let (mut upgraded,_)=boot(Family::P2,55,true,true);
    run_until_cached(&mut upgraded);
    change_world_task_rule(&mut upgraded);
    run_until_switch_or_end(&mut upgraded);
    assert_eq!(fourth(&upgraded),10);
    s38_post_recovery_invariants(&upgraded,55);
    finish(&mut upgraded);real_b(&upgraded,Family::P2);
    assert_eq!(arm_count(&upgraded),1);
    assert!(upgraded.state.fence_drops.is_empty());
    assert_eq!(upgraded.state.task_fences[&ORGANISM_ID],
        upgraded.state.probe_record.as_ref().unwrap().task_hash);
    println!("S39 v2 renewed, then fence arm and B24 A={} ticks={}",a_gas(&upgraded),upgraded.state.step);
}

#[test]
fn s39_change_after_fence_before_send_rejected_old_s38_sends_wrong_b(){
    let (mut guard,_)=boot(Family::P2,55,true,true);
    let (mut legacy,_)=old_boot(Family::P2,55);
    wait_until_armed(&mut guard);
    until_remote_values(&mut legacy);
    assert_eq!(b_gas(&guard),0);
    assert_eq!(b_gas(&legacy),0);
    let old_fourth=fourth(&guard);
    assert_eq!(old_fourth,fourth(&legacy));
    change_world_task_rule(&mut guard);
    change_world_task_rule(&mut legacy);
    assert_ne!(old_fourth,10,"new v2 task must disagree");
    // The original signed task hash no longer equals the current task.
    drive_bounded(&mut guard,1000);
    assert_eq!(b_gas(&guard),0);
    assert!(guard.audit.iter().any(|r|r.result.code==Code::Conflict &&
        matches!(&r.action,Some(Action::Send{..}))),
        "the real outgoing SEND is denied atomically by the kernel");
    assert!(!guard.audit.iter().any(|r|r.result.code==Code::Ok &&
        matches!(&r.action,Some(Action::Send{..}))),
        "the guarded organism may not commit a new send after task drift");
    assert_eq!(arm_count(&guard),1);
    assert!(guard.state.guarded_sends.is_empty());

    run_until_switch_or_end(&mut legacy);finish(&mut legacy);
    real_b(&legacy,Family::P2);
    let task:TransformTask=serde_json::from_slice(
        &legacy.state.objects["/public/E001/transform"].bytes).unwrap();
    assert_ne!(fourth(&legacy),task_truth(&legacy,&task.held_out_input));
    println!("S39 before-send TOCTOU: guarded_B0_Conflict, old_S38_B24_wrong={} true_v2={}",
        fourth(&legacy),task_truth(&legacy,&task.held_out_input));
}

#[test]
fn s39_change_after_real_remote_send_before_service_consumption_drops_b(){
    let (mut guard,_)=boot(Family::P2,55,true,true);
    let (mut legacy,_)=old_boot(Family::P2,55);
    let guarded_id=real_job_send_id(&mut guard);
    let old_id=real_job_send_id(&mut legacy);
    assert_eq!(b_gas(&guard),0);
    assert_eq!(b_gas(&legacy),0);
    assert_eq!(arm_count(&guard),1);
    assert!(guard.state.guarded_sends.contains_key(&guarded_id));
    assert!(!legacy.state.guarded_sends.contains_key(&old_id));
    assert!(guard.state.pending_deliveries.iter().any(|m|m.id==guarded_id));
    assert!(legacy.state.pending_deliveries.iter().any(|m|m.id==old_id));

    // RESEARCHER-DRIVEN environment revision injected precisely after
    // the causal 4-u32 SEND but before the queued request is consumed.
    change_world_task_rule(&mut guard);
    change_world_task_rule(&mut legacy);
    drive_bounded(&mut guard,1300);
    assert_eq!(b_gas(&guard),0);
    assert!(guard.state.fence_drops.iter().any(|d|d.message_id==guarded_id &&
        d.expected_hash!=d.observed_hash.clone().unwrap()));
    assert!(!guard.state.guarded_sends.contains_key(&guarded_id));
    assert!(!guard.state.service_events.iter().any(|e|
        e.request_message_id==guarded_id && e.b_gas_charged>0));

    drive_bounded(&mut legacy,1300);
    real_b(&legacy,Family::P2);
    let task:TransformTask=serde_json::from_slice(
        &legacy.state.objects["/public/E001/transform"].bytes).unwrap();
    assert_ne!(fourth(&legacy),task_truth(&legacy,&task.held_out_input));
    assert!(legacy.state.service_events.iter().any(|e|
        e.request_message_id==old_id && e.b_gas_charged>0));
    println!("S39 in-flight TOCTOU: guarded request_id={guarded_id} dropped before B; legacy_request_id={old_id} B24_stale_operand={} v2truth={}",
        fourth(&legacy),task_truth(&legacy,&task.held_out_input));
}

#[test]
fn s39_direct_arm_requires_issued_current_receipt_and_charges(){
    let (mut w,_)=boot(Family::P2,55,true,true);
    let wrong=fake_label(&[7,8,9],99);
    let missing=w.tick(Some(Action::ArmTaskFence{
        actor:ORGANISM_ID,evidence:wrong.clone(),
    }));
    assert_eq!(missing.code,Code::NotFound);
    assert_eq!(missing.gas_charged,3);
    assert!(w.state.task_fences.is_empty());
    let issued=w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9],
    }));
    assert_eq!(issued.code,Code::Ok);
    let forged=w.tick(Some(Action::ArmTaskFence{
        actor:ORGANISM_ID,evidence:wrong,
    }));
    assert_eq!(forged.code,Code::Conflict);
    assert_eq!(forged.gas_charged,3);
    let accepted=w.tick(Some(Action::ArmTaskFence{
        actor:ORGANISM_ID,evidence:issued.payload.clone(),
    }));
    assert_eq!(accepted.code,Code::Ok);
    assert_eq!(accepted.gas_charged,3);
    let old_hash=w.state.task_fences[&ORGANISM_ID].clone();
    change_world_task_rule(&mut w);
    let stale=w.tick(Some(Action::ArmTaskFence{
        actor:ORGANISM_ID,evidence:issued.payload,
    }));
    assert_eq!(stale.code,Code::Conflict);
    assert_eq!(w.state.task_fences[&ORGANISM_ID],old_hash);
    // Even a raw externally supplied Send cannot bypass an established
    // fence, including the normal zero-length payload preflight.
    let before=w.state.pending_deliveries.len();
    let denied=w.tick(Some(Action::Send{
        actor:ORGANISM_ID,destination:NodeId::C,bytes:vec![1,2,3]
    }));
    assert_eq!(denied.code,Code::Conflict);
    assert_eq!(denied.gas_charged,0);
    assert_eq!(denied.bandwidth_charged,0);
    assert_eq!(w.state.pending_deliveries.len(),before);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}
