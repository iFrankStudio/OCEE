//! S38: evidence-proven, one-shot reauthorization when task version changes.
//! The true environment v1->v2 rule is changed only by a researcher
//! intervention. Active VM must pay, re-probe, CAS its evidence, revalidate
//! the new receipt, and change the actual remote B operand.
//! No invented arbitrary quota resets or task edits by the organism.
use ocee_organism_seeds::{
    s36_active_probe::{s36_active_probe_root,
        s36_no_probe_control,s36_probe_bank_root,EVIDENCE},
    s37_evidence_guard::s37_verified_probe_root,
    s38_task_recovery::{s38_task_recovery_root,s38_no_renew_control},
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
        s38_task_recovery_root(&a,&b)
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

#[test]
fn s38_task_rule_drift_prompts_active_second_query_and_restores_real_b(){
    let (mut recovered,root)=boot(Family::P2,55,true,true);
    let (mut halted,_)=boot_control_s37(Family::P2,55);
    run_until_cached(&mut recovered);run_until_cached(&mut halted);
    assert_eq!(recovered.state.probe_count,1);
    assert_eq!(halted.state.probe_count,1);
    change_world_task_rule(&mut recovered);
    change_world_task_rule(&mut halted);
    assert_eq!(recovered.state.objects[DEPTH_TASK].bytes,
        halted.state.objects[DEPTH_TASK].bytes);

    run_until_switch_or_end(&mut recovered);
    assert_eq!(fourth(&recovered),10);
    assert_eq!(stored(&recovered).1.functions[0].blocks[0].instructions.len(),4);
    s38_post_recovery_invariants(&recovered,55);
    finish(&mut recovered);
    real_b(&recovered,Family::P2);

    run_until_switch_or_end(&mut halted);finish(&mut halted);
    assert_eq!(renews(&halted).len(),0);
    assert_eq!(verification_code(&halted,Code::Conflict),1);
    assert_eq!(halted.state.probe_count,1);
    assert_eq!(b_gas(&halted),0);
    assert!(!halted.state.objects.contains_key(REMOTE_VALUES));
    println!("S38 v1->v2 active recovery root={root} new_fourth={} A={} ticks={} B=24; S37 unchanged B0 A={} renew_fee=7 probe2_fee=52",
        fourth(&recovered),a_gas(&recovered),recovered.state.step,
        a_gas(&halted));
}

#[test]
fn s38_cross_family_v1_donor_relearned_for_new_task_and_cas_registry(){
    // Donor learns the real v1 program by the original two labels plus
    // active query, not a host-provided right answer.
    let (mut donor,root)=boot(Family::P1,1,true,true);
    run_until_switch_or_end(&mut donor);
    let (old_hash,old_program)=stored(&donor);
    assert_eq!(fourth(&donor),rule(1,&[4,5,6,7,8]));
    assert_eq!(donor.state.probe_count,1);
    let (mut p2,root2)=boot(Family::P2,55,true,true);
    assert_eq!(root,root2);
    let import_cost=import(&mut p2,&old_hash,&old_program,REGISTRY);
    run_until_cached(&mut p2);
    // The P2 explicitly changes its PUBLIC task and its HIDDEN oracle
    // between issuing and validating its first environment receipt.
    change_world_task_rule(&mut p2);
    run_until_switch_or_end(&mut p2);
    s38_post_recovery_invariants(&p2,55);
    assert_eq!(p2.state.objects[REGISTRY].version,1,
        "new semantics must CAS-replace inherited v1 executable");
    assert!(p2.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==REGISTRY)));
    let (new_hash,new_program)=stored(&p2);
    assert_ne!(new_hash,old_hash);
    assert_eq!(new_program.functions[0].blocks[0].instructions.len(),4);
    assert_eq!(fourth(&p2),10);
    finish(&mut p2);real_b(&p2,Family::P2);
    finish(&mut donor);real_b(&donor,Family::P1);
    println!("S38 inherited v1 source_P1=A{} target_P2_new_v2=A{} ticks={} import_A={} old_program={} new_program={} registry_v1 B_each24",
        a_gas(&donor),a_gas(&p2),p2.state.step,
        import_cost,old_hash,new_hash);
}

#[test]
fn s38_nonoriginal_or_tampered_receipt_cannot_buy_new_query(){
    let (mut forged,_)=boot(Family::P2,55,true,true);
    // Owner-preloaded cache without an issued Probe must not qualify
    // for any renewal even if a researcher changes the task ID.
    put(&mut forged,EVIDENCE.into(),fake_label(&[7,8,9],6));
    change_world_task_rule(&mut forged);
    run_until_switch_or_end(&mut forged);finish(&mut forged);
    assert_eq!(probe_actions(&forged),0);
    assert_eq!(renews(&forged).len(),0);
    assert_eq!(b_gas(&forged),0);
    assert_eq!(verification_code(&forged,Code::NotFound),1);

    let (mut tampered,_)=boot(Family::P2,55,true,true);
    run_until_cached(&mut tampered);
    let fake=fake_label(&[7,8,9],6);
    assert_eq!(tampered.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:EVIDENCE.into(),bytes:fake,
        expected_version:Some(0),
    })).code,Code::Ok);
    change_world_task_rule(&mut tampered);
    run_until_switch_or_end(&mut tampered);finish(&mut tampered);
    assert_eq!(probe_actions(&tampered),1);
    assert_eq!(renews(&tampered).len(),1);
    assert_eq!(renews(&tampered)[0].result.code,Code::Conflict);
    assert_eq!(renews(&tampered)[0].result.gas_charged,7);
    assert!(tampered.state.probe_renewed_for.is_none());
    assert_eq!(tampered.state.probe_count,1);
    assert_eq!(b_gas(&tampered),0);
    assert!(!tampered.state.objects.contains_key(REMOTE_VALUES));
    println!("S38 provenance rejection: fabricated cache NotFound, tampered original RenewProbe Conflict, both B0");
}

#[test]
fn s38_kernel_one_shot_authorization_and_version_budget_guards(){
    let (mut w,_)=boot(Family::P2,55,true,true);
    let first=w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    }));
    assert_eq!(first.code,Code::Ok);
    assert_eq!(first.gas_charged,52);
    let no_version=w.tick(Some(Action::RenewProbe{
        actor:ORGANISM_ID,evidence:first.payload.clone()
    }));
    assert_eq!(no_version.code,Code::Denied);
    assert_eq!(no_version.gas_charged,0);
    change_world_task_rule(&mut w);
    let wrong=w.tick(Some(Action::RenewProbe{
        actor:ORGANISM_ID,evidence:fake_label(&[7,8,9],6)
    }));
    assert_eq!(wrong.code,Code::Conflict);
    assert_eq!(wrong.gas_charged,7);
    assert!(w.state.probe_renewed_for.is_none());
    // Even a permitted v2 label does not reset the quota without the
    // original genuine old receipt and an explicit successful renewal.
    assert_eq!(w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    })).code,Code::Quota);
    let yes=w.tick(Some(Action::RenewProbe{
        actor:ORGANISM_ID,evidence:first.payload.clone()
    }));
    assert_eq!(yes.code,Code::Ok);
    assert_eq!(yes.gas_charged,7);
    assert_eq!(w.state.probe_count,1);
    let second=w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    }));
    assert_eq!(second.code,Code::Ok);
    assert_eq!(second.gas_charged,52);
    assert_eq!(serde_json::from_slice::<TransformExample>(&second.payload)
        .unwrap().output,6);
    assert_eq!(w.state.probe_count,2);
    assert_eq!(w.tick(Some(Action::VerifyProbe{
        actor:ORGANISM_ID,evidence:first.payload,
    })).code,Code::Conflict);
    assert_eq!(w.tick(Some(Action::VerifyProbe{
        actor:ORGANISM_ID,evidence:second.payload,
    })).code,Code::Ok);
    assert_eq!(w.tick(Some(Action::RenewProbe{
        actor:ORGANISM_ID,evidence:fake_label(&[7,8,9],6),
    })).code,Code::Quota);
    assert_eq!(w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    })).code,Code::Quota);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S38 strict v2 grant: first 52A, renewal 7A, second 52A, no third, original receipt stale");
}
