//! S36 research: label alias -> falsely trusted real B work in the S34
//! control, corrected by a third PUBLIC labelled counterexample. The ACTIVE
//! VM does extra paid Read/Exec, checks new and imported executables, then
//! uses the selected graph as an actual B FOLD32 operand across P1 and P2.
//!
//! Test host seeds labels and verifies hidden outcomes only afterwards;
//! it never installs the correct program into a normal world's registry.
use ocee_organism_seeds::{
    s36_active_probe::{s36_active_probe_root,
        s36_no_probe_control,s36_probe_bank_root,EVIDENCE},
    s37_evidence_guard::s37_verified_probe_root,
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
        s37_verified_probe_root(&a,&b)
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
#[test]
fn s37_verified_probe_preserves_cross_family_semantics_and_repair(){
    let mut donors=Vec::new();
    for (src,dst,stages,expected_searches) in [
        (1u64,55u64,0u32,2usize),
        (19u64,73u64,1u32,20usize),
        (37u64,91u64,2u32,38usize),
    ]{
        let (mut source,root)=boot(Family::P1,src,true,true);
        run_until_switch_or_end(&mut source);
        let (h,p)=check_constructed(&source,src,stages);
        assert_eq!(candidate_writes(&source),expected_searches,
            "false first match must be probed and rejected");
        assert_eq!(cache_reads(&source),2,
            "first candidate obtains query, next candidate reuses cached label");
        assert_acquired_evidence(&source,src,&[7,8,9]);
        assert_eq!(verification_calls(&source),2);
        assert_eq!(verification_code(&source,Code::Ok),2);
        assert!(source.audit.iter().any(|a|a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{h}"))));

        let (mut target,root2)=boot(Family::P2,dst,true,true);
        assert_eq!(root,root2,"identical frozen P1/P2 initial code");
        let import_a=import(&mut target,&h,&p,REGISTRY);
        run_until_switch_or_end(&mut target);
        assert_eq!(stored(&target).0,h);
        check_constructed(&target,dst,stages);
        assert_eq!(cache_reads(&target),1);
        assert_acquired_evidence(&target,dst,&[7,8,9]);
        assert_eq!(verification_calls(&target),1);
        assert_eq!(verification_code(&target,Code::Ok),1);
        assert!(!target.audit.iter().any(|a|a.step>=6 &&
            a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{h}"))),
            "target must revalidate, not independently recreate donor");
        finish(&mut source);finish(&mut target);
        real_b(&source,Family::P1);real_b(&target,Family::P2);
        println!("S36 transfer stages={stages} P1={src} P2={dst} root={root} tool={h} source_A={} target_A={} import_A={import_a} target_ticks={} B_both=24 queries_source=1_target=1",
            a_gas(&source),a_gas(&target),target.state.step);
        donors.push((stages,h,p,root));
    }
    assert_eq!(donors[0].3,donors[1].3);
    assert_eq!(donors[1].3,donors[2].3);
    assert_ne!(donors[0].1,donors[1].1);
    assert_ne!(donors[1].1,donors[2].1);

    // Ablation: old two-label learner succeeds at remote B's deterministic
    // FOLD32 calculation but the FOURTH operand has the wrong meaning.
    let (mut old,oldroot)=boot(Family::P2,55,false,true);
    run_until_switch_or_end(&mut old);
    assert_eq!(probe_actions(&old),0);
    let old_fourth=fourth(&old);
    let input:TransformTask=serde_json::from_slice(
        &old.state.objects["/public/E001/transform"].bytes).unwrap();
    let truth=rule(55,&input.held_out_input);
    assert_ne!(old_fourth,truth,
        "same two labels must permit a real false-generalization witness");
    finish(&mut old);real_b(&old,Family::P2);
    println!("S36 TWO-LABEL-ONLY FALSE_GENERALIZATION root={oldroot} generated_fourth={old_fourth} hidden_true_fourth={truth} B=24 A={} despite wrong task semantics",
        a_gas(&old));

    // A stale imported tool can fit both old labels, yet fail the third.
    // The target must reject it BEFORE final remote data assembly, search
    // the correct code, and CAS replace its registered stale version.
    let (mut old_source,_)=boot(Family::P1,1,false,true);
    run_until_switch_or_end(&mut old_source);
    let (wrong_hash,wrong_program)=stored(&old_source);
    assert_ne!(wrong_hash,donors[0].1);
    let (mut repair,repair_root)=boot(Family::P2,55,true,true);
    assert_eq!(repair_root,donors[0].3);
    let wrong_import_a=import(&mut repair,&wrong_hash,&wrong_program,REGISTRY);
    run_until_switch_or_end(&mut repair);
    assert_eq!(stored(&repair).0,donors[0].1,
        "target probe rejection must replace two-label-fitted old code");
    assert_eq!(repair.state.objects[REGISTRY].version,1,
        "semantic repair must be a real paid versioned CAS");
    assert!(repair.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==REGISTRY)));
    assert!(repair.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Exec{program_hash,..})
            if program_hash==&wrong_hash)));
    assert_eq!(fourth(&repair),truth);
    assert_eq!(cache_reads(&repair),3,
        "stale, candidate0 and candidate1 reuse one acquired label");
    assert_acquired_evidence(&repair,55,&[7,8,9]);
    assert_eq!(verification_calls(&repair),3);
    finish(&mut repair);real_b(&repair,Family::P2);
    println!("S36 stale_two_label_fit REJECTED by extra public probe; repaired_hash={} import_A={} A={} ticks={} B=24 registry_v1",
        donors[0].1,wrong_import_a,a_gas(&repair),repair.state.step);

    // Interactive oracle exhausted by a researcher intervention: the
    // VM may try once, but cannot invent or assume an environment label.
    let (mut missing,_)=boot(Family::P2,55,true,false);
    run_until_switch_or_end(&mut missing);finish(&mut missing);
    assert_eq!(probe_actions(&missing),1);
    assert_eq!(successful_probes(&missing),0);
    assert!(!missing.state.objects.contains_key(EVIDENCE));
    assert_eq!(missing.state.objects["/self/active"].version,0);
    assert!(!missing.state.objects.contains_key(REMOTE_VALUES));
    assert_eq!(b_gas(&missing),0);
    assert_ne!(missing.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    println!("S36 probe_budget_exhausted ABSTAIN A={} ticks={} B0",
        a_gas(&missing),missing.state.step);

    // Equal-charge visible vs sham inheritance still establishes paid
    // executable reuse without relying on content-hash coincidence.
    let (_,h,p,_)=&donors[2];
    let (mut visible,rv)=boot(Family::P2,91,true,true);
    let (mut sham,rs)=boot(Family::P2,91,true,true);
    assert_eq!(rv,rs);
    assert_eq!(visible.state_hash(),sham.state_hash());
    let cv=import(&mut visible,h,p,REGISTRY);
    let cs=import(&mut sham,h,p,"/self/tools/transform-vx");
    assert_eq!(cv,cs);
    assert_eq!(visible.state.step,sham.state.step);
    assert_eq!(visible.state.nodes,sham.state.nodes);
    assert_eq!(visible.state.organisms,sham.state.organisms);
    run_until_switch_or_end(&mut visible);
    run_until_switch_or_end(&mut sham);
    assert_eq!(stored(&visible).0,*h);
    assert_eq!(stored(&sham).0,*h);
    assert_eq!(cache_reads(&visible),1);
    assert_eq!(cache_reads(&sham),2);
    assert_acquired_evidence(&visible,91,&[7,8,9]);
    assert_acquired_evidence(&sham,91,&[7,8,9]);
    assert_eq!(verification_calls(&visible),1);
    assert_eq!(verification_calls(&sham),2);
    finish(&mut visible);finish(&mut sham);
    real_b(&visible,Family::P2);real_b(&sham,Family::P2);
    assert!(a_gas(&visible)<a_gas(&sham));
    println!("S36 equal-charge reuse visible_A={} sham_A={} saving={} import_A={} ticks_D={} ticks_sham={} B_both24",
        a_gas(&visible),a_gas(&sham),a_gas(&sham)-a_gas(&visible),
        cv,visible.state.step,sham.state.step);
}


/// This is a *direct* kernel guard test, not an autonomous organism action.
/// The main research test above checks that the organism actually chooses
/// and sends the query itself.
#[test]
fn s36_probe_restricts_scope_length_single_budget_and_snapshot(){
    let mut unrelated=World::with_protocol(Config{
        seed:55,a_gas_per_epoch:100_000,..Config::default()
    },Family::P2,42);
    assert_eq!(unrelated.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    })).code,Code::Denied);
    let (mut w,_)=boot(Family::P2,55,true,true);
    for input in [vec![],vec![1;9]]{
        assert_eq!(w.tick(Some(Action::Probe{
            actor:ORGANISM_ID,input
        })).code,Code::BadRequest);
        assert_eq!(w.state.probe_count,0);
    }
    let returned=w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![7,8,9]
    }));
    assert_eq!(returned.code,Code::Ok);
    assert_eq!(returned.gas_charged,52);
    let example:TransformExample=serde_json::from_slice(
        &returned.payload).unwrap();
    assert_eq!(example.input,vec![7,8,9]);
    assert_eq!(example.output,rule(55,&[7,8,9]));
    assert_eq!(w.state.probe_count,1);
    let snapshot=World::from_snapshot(&w.snapshot_json()).unwrap();
    assert_eq!(snapshot.state_hash(),w.state_hash());
    let forbidden=w.tick(Some(Action::Probe{
        actor:ORGANISM_ID,input:vec![8,7,6]
    }));
    assert_eq!(forbidden.code,Code::Quota);
    assert_eq!(forbidden.gas_charged,0);
    assert_eq!(w.state.probe_count,1);
}

#[test]
fn s36_changed_probe_bank_affects_vm_branch_not_hidden_answer(){
    // Counterfactual grammar change: the first proposal ALREADY
    // differentiates Len and XorFold, so a second input is unnecessary.
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let root=s36_probe_bank_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap(),
        &[7,8,9],&[1]);
    let mut w=World::with_protocol(Config{
        seed:55,a_gas_per_epoch:100_000,..Config::default()
    },Family::P2,42);
    install_public_fixtures(&mut w,55);
    for program in [p1,p2,root.clone()]{
        put(&mut w,format!("/self/versions/{}",program.content_hash().unwrap()),
            program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),
        root.content_hash().unwrap().into_bytes());
    run_until_switch_or_end(&mut w);
    check_constructed(&w,55,0);
    assert_acquired_evidence(&w,55,&[7,8,9]);
    finish(&mut w);
    real_b(&w,Family::P2);
    println!("S36 probe bank first_distinguishing used first proposal; real B24 A={}",
        a_gas(&w));
}
