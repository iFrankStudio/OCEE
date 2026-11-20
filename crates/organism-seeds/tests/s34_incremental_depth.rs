//! S34: researcher-created public three-depth task labels, not a host oracle.
//! Three composed program lengths (4, 6, 8 instructions) are learned by a frozen root,
//! inherited across P1->P2, and directly control the fourth real B operand.
//! Shapes, search order, depth rule and task specification are researcher
//! priors. The active VM reads only inputs and output labels, not the rule.
use ocee_organism_seeds::{
    s32_composite_b_operand::{
        s32_p1_composite_consumer,s32_p2_composite_consumer,REMOTE_VALUES},
    s34_incremental_depth::{s34_incremental_depth_root,s34_at_most_one_stage},
    s33_adaptive_depth::DEPTH_TASK,
    s30_composed_transfer::REGISTRY,
    s31_causal_remote::PREPARED_COUNT,
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::{Family,Operation,fold32};
use ocee_world_kernel::{
    Action,Code,Config,NodeId,Object,TransformExample,TransformTask,World,
    ORGANISM_ID,hidden_compose
};

fn rule(seed:u64,bytes:&[u8])->u32{
    let stages=((seed%54)/18) as u32;
    hidden_compose(seed,bytes).wrapping_add(stages.wrapping_mul(bytes.len() as u32))
}
fn fixture(w:&mut World,seed:u64){
    // A development-only PUBLIC simulated task. The hidden rule is used
    // only by the researcher/environment fixture and test-side verifier.
    let a=vec![13,7,11];let b=vec![18,9,4,6];
    let task=TransformTask{
        task_id:"s34-incremental-three-depth-public-v1".into(),
        examples:vec![
            TransformExample{output:rule(seed,&a),input:a},
            TransformExample{output:rule(seed,&b),input:b},
        ],
        held_out_input:vec![7,8,9,10,3,1],
    };
    assert!(w.state.objects.insert(DEPTH_TASK.into(),Object{
        owner:0,version:0,bytes:serde_json::to_vec(&task).unwrap()
    }).is_none());
}
fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    })).code,Code::Ok);
}
fn a_gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn programs(shallow_only:bool)->(Program,Program,Program){
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let h1=p1.content_hash().unwrap();let h2=p2.content_hash().unwrap();
    let root=if shallow_only{
        s34_at_most_one_stage(&h1,&h2)
    }else{
        s34_incremental_depth_root(&h1,&h2)
    };
    (p1,p2,root)
}
fn boot(family:Family,seed:u64,shallow_only:bool)->(World,String){
    let (p1,p2,root)=programs(shallow_only);
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    fixture(&mut w,seed);
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",program.content_hash().unwrap()),
            program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run_until_switch(w:&mut World,seed:u64){
    for _ in 0..10_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected S34 root trap seed={seed},step={} code={:?}, vm={:?}, audit_last={:?}",
            w.state.step,r.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation
                .as_ref().map(|c|(&c.block,c.pc,&c.awaiting)),
            w.audit.last().map(|a|(&a.action,&a.result)));
        if w.state.objects["/self/active"].version==1{break;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
}
fn run_to_end(w:&mut World){
    for _ in 0..13_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected S34 specialist trap step={} code={:?}",
            w.state.step,r.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"must terminate within bounded ticks");
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}
fn stored(w:&World)->(String,Program){
    let hash=String::from_utf8(w.state.objects[REGISTRY].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
fn check_learnt(w:&World,seed:u64,stages:u32)->(String,Program){
    assert_eq!(w.state.objects["/self/active"].version,1);
    assert_eq!(&w.state.objects[PREPARED_COUNT].bytes,&[4,0]);
    let (hash,tool)=stored(w);
    let ins=&tool.functions[0].blocks[0].instructions;
    assert_eq!(ins.len(),(4+2*stages) as usize,
        "must select actual structure from public task labels");
    assert!(matches!(ins[1].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    assert!(matches!(ins[2].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    for step in 0..stages {
        let unary=5+2*step;
        let accumulator=4+2*step;
        assert_eq!(ins[(unary-1) as usize].id,unary);
        assert_eq!(ins[(unary-1) as usize].op,Op::Len(0));
        assert_eq!(ins[unary as usize].id,unary+1);
        assert_eq!(ins[unary as usize].op,Op::Add(accumulator,unary));
    }
    let input:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let expected=rule(seed,&input.held_out_input);
    let values=&w.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(values.len(),16);
    assert_eq!(u32::from_le_bytes(values[12..].try_into().unwrap()),expected,
        "learned depth must predict target's unlabelled fourth operand");
    (hash,tool)
}
fn verify_real_b(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_gas(w),24);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let frames=w.audit.iter().filter_map(|a|{
        if let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action{
            if let Ok(d)=decoder.decode(bytes) {
                if d.operation==Operation::RemoteEval && d.body.len()==18
                    && u16::from_le_bytes(d.body[..2].try_into().unwrap())==4{
                    return Some(d.body);
                }
            }
        }
        None
    }).collect::<Vec<_>>();
    assert!(!frames.is_empty(),"actual C request missing");
    let frame=&frames[frames.len()-1];
    assert_eq!(&frame[2..],w.state.objects[REMOTE_VALUES].bytes.as_slice());
    let nums=frame[2..].chunks_exact(4).map(|b|
        u32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
    let key=match family{
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!()
    };
    let result=u32::from_le_bytes(w.state.objects[key].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(result,fold32(&nums).unwrap());
}
fn import(w:&mut World,h:&str,p:&Program,key:&str)->u64{
    let before=a_gas(w);
    put(w,format!("/self/versions/{h}"),p.canonical_bytes());
    put(w,key.into(),h.as_bytes().to_vec());
    a_gas(w)-before
}
#[test]
fn s34_vm_extends_program_stages_for_real_remote_b(){
    let mut donors=Vec::new();
    for (src,dst,stages) in [(1u64,55u64,0u32),(19u64,73u64,1u32),(37u64,91u64,2u32)]{
        let (mut source,h1)=boot(Family::P1,src,false);
        run_until_switch(&mut source,src);
        let (hash,tool)=check_learnt(&source,src,stages);
        assert!(source.audit.iter().any(|e|e.result.code==Code::Ok &&
            matches!(&e.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{hash}"))));
        let (mut target,h2)=boot(Family::P2,dst,false);
        assert_eq!(h1,h2,"initial root identical in all P1/P2 worlds");
        let import_cost=import(&mut target,&hash,&tool,REGISTRY);
        run_until_switch(&mut target,dst);
        assert_eq!(stored(&target).0,hash);
        check_learnt(&target,dst,stages);
        assert!(!target.audit.iter().any(|e|e.step>=6 &&
            e.result.code==Code::Ok &&
            matches!(&e.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{hash}"))),
            "target must use transferred executable, not synthesize it again");
        run_to_end(&mut source);run_to_end(&mut target);
        verify_real_b(&source,Family::P1);
        verify_real_b(&target,Family::P2);
        println!("S34 native depth={} P1_seed={src} P2_seed={dst} root={h1} selected_hash={hash} import_A={import_cost} P1_A={} P2_A={} P2_ticks={} B_both=24",
            stages,a_gas(&source),a_gas(&target),target.state.step);
        donors.push((stages,hash,tool,h1));
    }
    assert_ne!(donors[0].1,donors[1].1);
    assert_eq!(donors[0].3,donors[1].3);
    assert_eq!(donors[0].3,donors[2].3);
    assert_ne!(donors[1].1,donors[2].1);

    // Mechanistic negative: prevent the deep shape (18 only) while
    // preserving the same public environment/task and expert programs.
    let (mut shallow,_)=boot(Family::P2,91,true);
    run_until_switch(&mut shallow,91);
    run_to_end(&mut shallow);
    assert_ne!(shallow.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_gas(&shallow),0);
    assert_eq!(shallow.state.objects["/self/active"].version,0);
    assert!(!shallow.state.objects.contains_key(REMOTE_VALUES));
    // Exclude the three initial program boot WRITEs. A failure to solve
    // after fewer than 18 attempts might merely be an unrelated trap.
    // Require the whole shallow grammar to be tried and rejected.
    let shallow_trials=shallow.audit.iter().filter(|e|e.step>=4 &&
        matches!(&e.action,Some(Action::Write{key,..})
            if key.starts_with("/self/versions/"))).count();
    assert_eq!(shallow_trials,36,
        "negative depth control must exhaust all 18 shallow candidates");
    println!("S34 shallow_only negative A={} ticks={} B=0",
        a_gas(&shallow),shallow.state.step);

    // Equal-cost same-schedule registry visibility sham: target is able
    // to learn the missing deep program itself but must pay new search.
    let (_,good_hash,good_tool,_)=&donors[2];
    let (mut visible,rv)=boot(Family::P2,91,false);
    let (mut sham,rs)=boot(Family::P2,91,false);
    assert_eq!(rv,rs);
    assert_eq!(visible.state_hash(),sham.state_hash());
    let cost_visible=import(&mut visible,good_hash,good_tool,REGISTRY);
    let cost_sham=import(&mut sham,good_hash,good_tool,
        "/self/tools/transform-vx");
    assert_eq!(cost_visible,cost_sham);
    assert_eq!(visible.state.step,sham.state.step);
    assert_eq!(visible.state.nodes,sham.state.nodes);
    assert_eq!(visible.state.organisms,sham.state.organisms);
    run_until_switch(&mut visible,91);
    run_until_switch(&mut sham,91);
    check_learnt(&visible,91,2);
    check_learnt(&sham,91,2);
    run_to_end(&mut visible);run_to_end(&mut sham);
    verify_real_b(&visible,Family::P2);
    verify_real_b(&sham,Family::P2);
    assert!(a_gas(&visible)<a_gas(&sham));
    println!("S34 deep_sham visible_A={} sham_A={} saving={} import_A={} ticks_visible={} ticks_sham={} B_each=24",
        a_gas(&visible),a_gas(&sham),
        a_gas(&sham)-a_gas(&visible),cost_visible,
        visible.state.step,sham.state.step);

    // An incompatible shallow inherited tool must be checked against the
    // target's public labels, rejected, re-synthesized as a deep graph,
    // and CAS-replaced before performing real C/B work.
    let (_,wrong_hash,wrong_tool,_)=&donors[1];
    let (mut repair,repair_root)=boot(Family::P2,91,false);
    assert_eq!(repair_root,rv);
    import(&mut repair,wrong_hash,wrong_tool,REGISTRY);
    run_until_switch(&mut repair,91);
    let (fresh_hash,fresh)=check_learnt(&repair,91,2);
    assert_ne!(fresh_hash,*wrong_hash);
    assert_eq!(fresh.functions[0].blocks[0].instructions.len(),8);
    assert_eq!(repair.state.objects[REGISTRY].version,1);
    assert!(repair.audit.iter().any(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==REGISTRY)));
    run_to_end(&mut repair);
    verify_real_b(&repair,Family::P2);
    println!("S34 wrong_topology repaired_hash={} A={} ticks={} B24 registry_v1",
        fresh_hash,a_gas(&repair),repair.state.step);
}
