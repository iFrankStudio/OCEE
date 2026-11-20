//! S41: early paid failure check on the first truly Empty RECEIVE versus
//! the S40 128-poll timeout, and lifetime-prefix cleanup with unlisted
//! newly minted derived records. Independent real C/B causal checks.
use ocee_organism_seeds::{
    s36_active_probe::{s36_active_probe_root,
        s36_no_probe_control,s36_probe_bank_root,EVIDENCE},
    s37_evidence_guard::s37_verified_probe_root,
    s38_task_recovery::{s38_task_recovery_root,s38_no_renew_control},
    s39_task_fenced_remote::s39_task_fenced_remote_root,
    s40_dropped_recovery::{s40_dropped_recovery_root,
        s40_p1_consumer,s40_p2_recovery_consumer},
    s41_early_drop::{s41_early_drop_root,s41_p2_early_drop_consumer},
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
    let p1=s40_p1_consumer();
    let p2=s41_p2_early_drop_consumer();
    let a=p1.content_hash().unwrap();
    let b=p2.content_hash().unwrap();
    let root=if enhanced {
        s41_early_drop_root()
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
    // Real pre-S40 reference: S39 HAS task fences but NO dropped-message
    // recovery. An unfenced S38 reference would legitimately B24 with
    // stale semantics and is the wrong negative control for this claim.
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let root=s39_task_fenced_remote_root(
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


fn recovery_diagnostics(w:&World)->String{
    let last=w.audit.iter().rev().take(16)
        .map(|a|format!("{}:{:?}:{:?}",
            a.step,
            a.action.as_ref().map(|x|match x{
                Action::Read{key,..}=>format!("Read({key})"),
                Action::Write{key,..}=>format!("Write({key})"),
                Action::Send{..}=>"Send".into(),
                Action::Exec{..}=>"Exec".into(),
                Action::Probe{..}=>"Probe".into(),
                Action::VerifyProbe{..}=>"Verify".into(),
                Action::RenewProbe{..}=>"Renew".into(),
                Action::ArmTaskFence{..}=>"Arm".into(),
                Action::FenceStatus{..}=>"Status".into(),
                Action::RestartRoot{..}=>"Restart".into(),
                _=>"Other".into(),
            }),a.result.code)).collect::<Vec<_>>();
    format!("step={} active_v={} restarts={:?} drop={} probe_count={} reg_v={:?} evidence_v={:?} next_B={} cont={:?} last_return={:?} last_actions={last:?}",
        w.state.step,w.state.objects["/self/active"].version,
        w.state.guarded_restarts,w.state.fence_drops.len(),
        w.state.probe_count,w.state.objects.get(REGISTRY).map(|x|x.version),
        w.state.objects.get(EVIDENCE).map(|x|x.version),
        b_gas(w),
        w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .map(|c|(&c.block,c.pc,c.finished)),
        w.state.organisms[&ORGANISM_ID].last_return)
}

fn recover_to_completed_b(w:&mut World){
    // A bounded P2 timeout (128 polls) and one complete relearning fit
    // within 1200 further steps. Fail immediately if the VM terminates
    // without B, retaining actual audited stage/codes for diagnosis.
    for _ in 0..1_200 {
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "bad VM recovery step {} {:?} active_v={} cont={:?}",
            w.state.step,r.code,
            w.state.objects["/self/active"].version,
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .map(|c|(&c.block,c.pc)));
        if b_gas(w)==24 &&
            w.state.objects["/self/active"].version==3 &&
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .is_some_and(|c|c.finished){break;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished) {
            panic!("S40 terminated without new B: {}",
                recovery_diagnostics(w));
        }
    }
    assert_eq!(b_gas(w),24,"S40 recovery did not complete: {}",
        recovery_diagnostics(w));
    assert_eq!(w.state.objects["/self/active"].version,3);
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn actual_remote_sends(w:&World)->Vec<(u64,Vec<u8>)>{
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    w.audit.iter().filter_map(|a|{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
            else{return None};
        if a.result.code!=Code::Ok{return None}
        let Ok(frame)=decoder.decode(bytes) else{return None};
        if frame.operation!=Operation::RemoteEval ||
            frame.body.len()!=18 ||
            u16::from_le_bytes(frame.body[..2].try_into().ok()?)!=4{
            return None;
        }
        let id=u64::from_le_bytes(a.result.payload[..8].try_into().ok()?);
        Some((id,frame.body))
    }).collect()
}
fn restart_actions(w:&World)->Vec<&ocee_world_kernel::AuditRecord>{
    w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::RestartRoot{..}))).collect()
}
fn inspect_actions(w:&World)->Vec<&ocee_world_kernel::AuditRecord>{
    w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::FenceStatus{..}))).collect()
}
fn check_no_duplicate_after(w:&mut World){
    let old=w.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>();
    assert_eq!(old,24);
    for _ in 0..300 {
        w.tick_autonomous(ORGANISM_ID);
    }
    assert_eq!(b_gas(w),24);
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn ensure_drop_recovery(w:&World,old_id:u64){
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    assert_eq!(w.state.fence_drops.len(),1);
    assert_eq!(w.state.fence_drops[0].message_id,old_id);
    assert_eq!(w.state.fence_drops[0].actor,ORGANISM_ID);
    assert_eq!(restart_actions(w).len(),1);
    assert_eq!(restart_actions(w)[0].result.code,Code::Ok);
    assert_eq!(restart_actions(w)[0].result.gas_charged,8);
    assert_eq!(inspect_actions(w).len(),1);
    assert_eq!(inspect_actions(w)[0].result.code,Code::Ok);
    assert_eq!(w.state.probe_count,2);
    assert_eq!(w.state.objects[EVIDENCE].version,1);
    assert_eq!(w.state.probe_record.as_ref().unwrap().receipt,
        w.state.objects[EVIDENCE].bytes);
    assert_eq!(w.state.probe_renewed_for.as_ref(),
        Some(&w.state.probe_record.as_ref().unwrap().task_hash));
    assert_eq!(w.state.task_fences[&ORGANISM_ID],
        w.state.probe_record.as_ref().unwrap().task_hash);
    assert_eq!(fourth(w),10);
    assert!(w.state.guarded_sends.is_empty());
    let sends=actual_remote_sends(w);
    assert_eq!(sends.len(),2,"one old, one newly synthesized job");
    assert_eq!(sends[0].0,old_id);
    assert_ne!(sends[1].0,old_id);
    assert_eq!(u32::from_le_bytes(sends[0].1[14..18].try_into().unwrap()),48);
    assert_eq!(u32::from_le_bytes(sends[1].1[14..18].try_into().unwrap()),10);
    assert_eq!(w.state.service_events.iter().filter(|e|e.b_gas_charged>0).count(),1);
    assert!(!w.state.service_events.iter().any(|e|
        e.request_message_id==old_id && e.b_gas_charged>0));
    assert!(w.state.service_events.iter().any(|e|
        e.request_message_id==sends[1].0 && e.b_gas_charged==24));
    assert!(w.audit.iter().any(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,expected_version:Some(0),..})
            if key==REGISTRY)));
    assert!(w.audit.iter().any(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,expected_version:Some(0),..})
            if key==EVIDENCE)));
    assert!(w.state.objects["/self/active"].version==3);
}


fn s41_slow_s40_boot(seed:u64)->(World,String){
    // Exact same world seed, task, P1 specialist and initial quotas.
    // Only the P2 failure detector differs: S40 waits 128 polls.
    let p1=s40_p1_consumer();
    let p2=s40_p2_recovery_consumer();
    let root=s40_dropped_recovery_root();
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },Family::P2,42);
    install_public_fixtures(&mut w,seed);
    for p in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            p.content_hash().unwrap()),p.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn wait_delta(w:&World)->u64{
    let drop=w.state.fence_drops.first().expect("real S39 dropped request");
    let reboot=restart_actions(w).first().expect("VM-requested root restart");
    assert!(reboot.step>drop.step);
    reboot.step-drop.step
}
fn paid_statuses(w:&World)->Vec<&ocee_world_kernel::AuditRecord>{
    let out=inspect_actions(w);
    assert!(!out.is_empty(),"P2 genuinely inspected kernel failure status");
    for a in &out{
        assert!(matches!(a.result.code,Code::Empty|Code::Ok));
        assert_eq!(a.result.gas_charged,1,
            "even negative drop-status queries have visible A cost");
    }
    out
}
fn accepted_once(w:&World,old_id:u64){
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    assert_eq!(w.state.fence_drops.len(),1);
    assert_eq!(w.state.fence_drops[0].message_id,old_id);
    assert_eq!(w.state.fence_drops[0].actor,ORGANISM_ID);
    assert_eq!(restart_actions(w).len(),1);
    assert_eq!(restart_actions(w)[0].result.code,Code::Ok);
    assert_eq!(restart_actions(w)[0].result.gas_charged,8);
    assert!(paid_statuses(w).iter().any(|a|a.result.code==Code::Ok));
    assert_eq!(w.state.probe_count,2);
    assert_eq!(w.state.objects[EVIDENCE].version,1);
    assert_eq!(w.state.probe_record.as_ref().unwrap().receipt,
        w.state.objects[EVIDENCE].bytes);
    assert_eq!(w.state.task_fences[&ORGANISM_ID],
        w.state.probe_record.as_ref().unwrap().task_hash);
    assert_eq!(fourth(w),10);
    assert_eq!(b_gas(w),24);
    assert!(w.state.guarded_sends.is_empty());
    let sent=actual_remote_sends(w);
    assert_eq!(sent.len(),2);
    assert_eq!(sent[0].0,old_id);
    assert_ne!(sent[0].0,sent[1].0);
    assert_eq!(u32::from_le_bytes(sent[0].1[14..18].try_into().unwrap()),48);
    assert_eq!(u32::from_le_bytes(sent[1].1[14..18].try_into().unwrap()),10);
    assert!(!w.state.service_events.iter().any(|e|
        e.request_message_id==old_id && e.b_gas_charged>0));
    assert_eq!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count(),1);
    assert!(w.state.service_events.iter().any(|e|
        e.request_message_id==sent[1].0 && e.b_gas_charged==24));
    assert!(w.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,expected_version:Some(0),..})
            if key==EVIDENCE)));
    assert!(w.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,expected_version:Some(0),..})
            if key==REGISTRY)));
    assert_eq!(w.state.objects["/self/active"].version,3);
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}

#[test]
fn s41_p2_first_empty_detects_real_drop_faster_than_s40_timeout_and_keeps_b_once(){
    let (mut fast,root)=boot(Family::P2,55,true,true);
    let (mut slow,_)=s41_slow_s40_boot(55);
    // Unknown names invented for this experiment are NOT in S40's
    // five-entry hard-coded cleanup list; S41 must expire these by
    // namespace while preserving unrelated durable records.
    let transient=[
        ("/self/results/s41-new-prediction",vec![1,7,1]),
        ("/self/work/s41-new-temp",vec![9,9,9,9]),
        ("/self/discoveries/s41-new-kind",vec![8,8]),
    ];
    for (key,value) in &transient {
        put(&mut fast,(*key).into(),value.clone());
    }
    let durable=[
        ("/self/notes/s41-remember",vec![5,6,7]),
        ("/self/evidence/s41-independent",vec![3,1,4]),
        ("/self/workbench/s41-prefix-not-match",vec![2,7]),
    ];
    for (key,value) in &durable{
        put(&mut fast,(*key).into(),value.clone());
    }
    let old_id=real_job_send_id(&mut fast);
    let control_id=real_job_send_id(&mut slow);
    assert_eq!(b_gas(&fast),0);
    assert_eq!(b_gas(&slow),0);
    change_world_task_rule(&mut fast);
    change_world_task_rule(&mut slow);
    recover_to_completed_b(&mut fast);
    recover_to_completed_b(&mut slow);
    accepted_once(&fast,old_id);
    accepted_once(&slow,control_id);
    real_b(&fast,Family::P2);
    real_b(&slow,Family::P2);
    let fast_latency=wait_delta(&fast);
    let slow_latency=wait_delta(&slow);
    assert!(fast_latency+40<slow_latency,
        "S41 early poll must beat S40 128-Receive wait: early={fast_latency} slow={slow_latency}");
    for (key,_) in &transient{
        assert!(!fast.state.objects.contains_key(*key),
            "new, unlisted ephemeral state must be invalidated");
    }
    for (key,value) in &durable{
        assert_eq!(&fast.state.objects[*key].bytes,value,
            "non-ephemeral durable self data must survive restart");
    }
    assert_eq!(fast.state.objects[REGISTRY].version,1);
    assert_eq!(fast.state.objects[EVIDENCE].version,1);
    let persisted=fast.state.objects.iter()
        .filter(|(_,o)|o.owner==ORGANISM_ID)
        .map(|(_,o)|o.bytes.len() as u64).sum::<u64>();
    assert_eq!(fast.state.organisms[&ORGANISM_ID].storage_used,persisted);
    let denied=fast.tick(Some(Action::RestartRoot{actor:ORGANISM_ID}));
    assert_eq!(denied.code,Code::Quota);
    check_no_duplicate_after(&mut fast);
    println!("S41 early real drop→RestartRoot: {} ticks vs S40 {} ticks; P2 early total A={} ticks={} vs slow A={} ticks={}; oldB0/newB24; root={}",
        fast_latency,slow_latency,a_gas(&fast),fast.state.step,
        a_gas(&slow),slow.state.step,root);
}

#[test]
fn s41_p1_donor_preserved_and_p2_early_repair_imported_v1_code(){
    let (mut donor,root)=boot(Family::P1,1,true,true);
    run_until_switch_or_end(&mut donor);
    let (hash,program)=stored(&donor);
    let (mut target,root2)=boot(Family::P2,55,true,true);
    assert_eq!(root,root2,"same frozen initial root for P1/P2");
    let import_cost=import(&mut target,&hash,&program,REGISTRY);
    let old_id=real_job_send_id(&mut target);
    assert_eq!(stored(&target).0,hash);
    change_world_task_rule(&mut target);
    recover_to_completed_b(&mut target);
    accepted_once(&target,old_id);
    assert_ne!(stored(&target).0,hash);
    assert_eq!(target.state.objects[REGISTRY].version,1);
    finish(&mut donor);real_b(&donor,Family::P1);
    real_b(&target,Family::P2);
    assert!(wait_delta(&target)<40,
        "early detector must not need a 128-poll timeout");
    println!("S41 imported P1 v1 tool restored via early P2 signal: import_A={} target_A={} target_ticks={} donor_A={} B_each24",
        import_cost,a_gas(&target),target.state.step,a_gas(&donor));
}

#[test]
fn s41_no_drop_status_charged_but_no_spurious_restart_or_loss(){
    let (mut w,_)=boot(Family::P2,55,true,true);
    let probe=w.tick(Some(Action::FenceStatus{actor:ORGANISM_ID}));
    assert_eq!(probe.code,Code::Empty);
    assert_eq!(probe.gas_charged,1);
    let denied=w.tick(Some(Action::RestartRoot{actor:ORGANISM_ID}));
    assert_eq!(denied.code,Code::Denied);
    run_until_switch_or_end(&mut w);finish(&mut w);
    real_b(&w,Family::P2);
    assert!(w.state.fence_drops.is_empty());
    assert!(w.state.guarded_restarts.is_empty());
    assert_eq!(restart_actions(&w).len(),1,
        "only explicit denied research preflight, no active VM restart");
    assert_eq!(restart_actions(&w)[0].result.code,Code::Denied);
    assert_eq!(w.state.objects["/self/active"].version,1);
    assert_eq!(b_gas(&w),24);
    println!("S41 healthy world A={} B24, charged negative query, never restarts",
        a_gas(&w));
}

#[test]
fn s41_reverted_task_stale_drop_does_not_authorize_replay(){
    let (mut w,_)=boot(Family::P2,55,true,true);
    let id=real_job_send_id(&mut w);
    let original=w.state.objects[DEPTH_TASK].clone();
    change_world_task_rule(&mut w);
    for _ in 0..40 {
        w.tick_autonomous(ORGANISM_ID);
        if w.state.fence_drops.iter().any(|e|e.message_id==id){break;}
    }
    assert_eq!(w.state.fence_drops.len(),1);
    w.state.objects.insert(DEPTH_TASK.into(),original);
    let denial=w.tick(Some(Action::RestartRoot{actor:ORGANISM_ID}));
    assert_eq!(denial.code,Code::Denied);
    // On VM's own early fault check, it may see the recorded drop
    // but the kernel must still refuse an unjustified restart.
    drive_bounded(&mut w,800);
    assert_eq!(b_gas(&w),0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(!restart_actions(&w).iter()
        .any(|a|a.result.code==Code::Ok));
    assert_eq!(w.state.probe_count,1);
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


#[test]
fn s42b_vm_carries_real_u64_send_receipt_through_recovery_and_b_once(){
    use ocee_organism_seeds::s42b_request_receipt::{
        s42b_explicit_request_root,s42b_p2_explicit_request_consumer,
        PENDING_REMOTE_ID,
    };
    let p1=s40_p1_consumer();
    let p2=s42b_p2_explicit_request_consumer();
    let root=s42b_explicit_request_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed:55,a_gas_per_epoch:100_000,..Config::default()
    },Family::P2,42);
    install_public_fixtures(&mut w,55);
    // Ensure the VM's Bytes ABI preserves all 64 bits of the real ID.
    w.state.next_sequence=(u32::MAX as u64)+100;
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),root_hash.into_bytes());
    let old_id=real_job_send_id(&mut w);
    assert!(old_id>u32::MAX as u64);
    change_world_task_rule(&mut w);
    recover_to_completed_b(&mut w);
    assert_eq!(b_gas(&w),24);
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    let statuses=w.audit.iter().filter_map(|a|match &a.action{
        Some(Action::FenceStatusFor{request_id,..})=>Some((a,*request_id)),
        _=>None,
    }).collect::<Vec<_>>();
    assert!(!statuses.is_empty(),"active VM must use explicit ID status");
    assert!(statuses.iter().all(|(a,id)|*id==old_id &&
        a.result.gas_charged==1));
    assert!(statuses.iter().any(|(a,_)|a.result.code==Code::Ok));
    let restarts=w.audit.iter().filter_map(|a|match &a.action{
        Some(Action::RestartRootFor{request_id,..})=>Some((a,*request_id)),
        _=>None,
    }).collect::<Vec<_>>();
    assert_eq!(restarts.len(),1);
    assert_eq!(restarts[0].1,old_id);
    assert_eq!(restarts[0].0.result.code,Code::Ok);
    assert_eq!(restarts[0].0.result.gas_charged,8);
    assert!(!w.audit.iter().any(|a|matches!(&a.action,
        Some(Action::FenceStatus{..}|Action::RestartRoot{..}))),
        "S42B specialist must never fall back to actor-level recovery");
    assert_eq!(w.state.fence_drops.len(),1);
    assert_eq!(w.state.fence_drops[0].message_id,old_id);
    let sent=actual_remote_sends(&w);
    let persisted=&w.state.objects[PENDING_REMOTE_ID].bytes;
    assert_eq!(persisted.len(),8);
    assert_eq!(u64::from_le_bytes(persisted.as_slice().try_into().unwrap()),
        sent[1].0,"new attempt replaces the expired old request ID");
    assert_eq!(sent.len(),2);
    assert_eq!(sent[0].0,old_id);
    assert_ne!(sent[1].0,old_id);
    assert!(!w.state.service_events.iter().any(|e|
        e.request_message_id==old_id && e.b_gas_charged>0));
    assert_eq!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count(),1);
    assert_eq!(w.state.objects["/self/active"].version,3);
    real_b(&w,Family::P2);
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
