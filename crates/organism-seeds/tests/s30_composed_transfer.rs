//! S30: bounded two-node composed executable, source P1 -> P2.
 //! The single initial root contains the same P1/P2 specialists everywhere.
 //! The source world chooses among 18 researcher-provided graph combinations.
 //! The target checks inherited semantics, predicts new public bytes, and
 //! can replace an outdated tool by paid CAS + renewed search.
use ocee_organism_seeds::{
    s19_p1_tool::s19_p1_build_and_use_tool,
    s23_p2_transfer::s23_p2_remote_tool_transfer,
    s30_composed_transfer::{s30_composed_cross_family_root,REGISTRY,OOD_PREDICTION},
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,Code,Config,TransformTask,WorkItem,World,ORGANISM_ID,
    hidden_compose,
};

fn put(world:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None,
    })).code,Code::Ok,"every imported and initial version uses metered WRITE");
}
fn gas(world:&World)->u64{
    world.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn b_gas(world:&World)->u64{
    world.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn boot(family:Family,seed:u64)->(World,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s30_composed_cross_family_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let hash=root.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    for program in [p1,p2,root]{
        put(&mut world,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut world,"/self/active".into(),hash.clone().into_bytes());
    (world,hash)
}
fn run(world:&mut World){
    for _ in 0..16_384{
        let r=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "unexpected world VM trap step={} code={:?} block={:?}",
            world.state.step,r.code,world.state.organisms[&ORGANISM_ID]
                .vm_continuation.as_ref().map(|c|c.block.as_str()));
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded active VM eventually terminates");
    assert_eq!(world.state_hash(),World::from_snapshot(
        &world.snapshot_json()).unwrap().state_hash(),"replay snapshot identity");
}
fn stored(world:&World)->(String,Program){
    let hash=String::from_utf8(world.state.objects[REGISTRY].bytes.clone()).unwrap();
    let program:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    (hash,program)
}
fn verify_composed_shape(program:&Program){
    let ins=&program.functions[0].blocks[0].instructions;
    assert!(matches!(ins[1].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    assert!(matches!(ins[2].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    assert!(matches!(ins[3].op,Op::Add(2,3)|Op::Xor(2,3)));
}
fn verify_task(world:&World,seed:u64,family:Family){
    let context=&world.state.organisms[&ORGANISM_ID];
    assert_eq!(context.last_return,Some(Value::U32(1)),
        "world did not finish correct composed+remote tasks; seed={seed}");
    assert_eq!(b_gas(world),20,"one real B request");
    let task:TransformTask=serde_json::from_slice(
        &world.state.objects["/public/E001/compose"].bytes).unwrap();
    let result:ActionResult=serde_json::from_slice(
        &world.state.objects["/self/results/transform-held-out"].bytes).unwrap();
    assert_eq!(result.code,Code::Ok);
    let value:Value=serde_json::from_slice(&result.payload).unwrap();
    assert_eq!(value,Value::U32(hidden_compose(seed,&task.held_out_input)),
        "S6 held-out oracle only after active agent");
    let extra:TransformTask=serde_json::from_slice(
        &world.state.objects["/public/E001/transform"].bytes).unwrap();
    let actual_extra:u32=serde_json::from_slice(
        &world.state.objects[OOD_PREDICTION].bytes).unwrap();
    assert_eq!(actual_extra,hidden_compose(seed,&extra.held_out_input),
        "target-seed-varying held-out input reference, never seen by VM");
    let job:WorkItem=serde_json::from_slice(
        &world.state.objects["/public/E001/job0"].bytes).unwrap();
    let oracle=job.values.iter().enumerate().fold(0u32,|s,(i,&v)|
        s.wrapping_add(v.rotate_left((i%32) as u32)));
    let result_key=match family{
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!(),
    };
    let got=u32::from_le_bytes(world.state.objects[result_key].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(got,oracle,"B result must have real independent FOLD32 value");
}
fn import(w:&mut World,hash:&str,p:&Program,registry:&str)->u64{
    let before=gas(w);
    put(w,format!("/self/versions/{hash}"),p.canonical_bytes());
    put(w,registry.into(),hash.as_bytes().to_vec());
    gas(w)-before
}
fn exec_before_switch(w:&World,hash:&str)->usize{
    let cutoff=w.audit.iter().position(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{key,..})
            if key=="/self/active" && e.step>=4))
        .unwrap_or(w.audit.len());
    w.audit[..cutoff].iter().filter(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Exec{program_hash,..})
            if program_hash==hash)).count()
}
fn generated_after_boot(w:&World,hash:&str)->bool{
    let key=format!("/self/versions/{hash}");
    w.audit.iter().any(|e|e.step>=6 && e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{key:k,..}) if k==&key))
}
#[test]
fn s30_composed_graph_transfer_repair_and_new_inputs(){
    let mut sources=Vec::<(u64,String,Program,String)>::new();
    for (source_seed,target_seed) in [(1u64,19u64),(10u64,28u64)]{
        let (mut source,root_hash)=boot(Family::P1,source_seed);
        run(&mut source);
        verify_task(&source,source_seed,Family::P1);
        let (hash,program)=stored(&source);
        verify_composed_shape(&program);
        assert!(generated_after_boot(&source,&hash));
        assert!(exec_before_switch(&source,&hash)>=4,
            "candidate must be used on two labelled examples + two fresh tasks");
        let (mut target,target_root)=boot(Family::P2,target_seed);
        assert_eq!(target_root,root_hash);
        let import_cost=import(&mut target,&hash,&program,REGISTRY);
        assert!(import_cost>0);
        run(&mut target);
        verify_task(&target,target_seed,Family::P2);
        assert_eq!(stored(&target).0,hash);
        assert_eq!(exec_before_switch(&target,&hash),4,
            "target must actually revalidate 2 labels then 2 new payloads");
        assert!(!generated_after_boot(&target,&hash),
            "target must reuse imported executable, not rebuild it");
        let src_extra:TransformTask=serde_json::from_slice(
            &source.state.objects["/public/E001/transform"].bytes).unwrap();
        let dst_extra:TransformTask=serde_json::from_slice(
            &target.state.objects["/public/E001/transform"].bytes).unwrap();
        assert_ne!(src_extra.held_out_input,dst_extra.held_out_input,
            "the target must face genuinely new bytes");
        println!("S30 transfer source_seed={source_seed} target_seed={target_seed} root={root_hash} tool={hash} import_A={import_cost} target_A={} ticks={} B=20",
            gas(&target),target.state.step);
        sources.push((source_seed,hash,program,root_hash));
    }
    assert_eq!(sources[0].3,sources[1].3);
    assert_ne!(sources[0].1,sources[1].1,"different binary combinators must be distinct");

    let (_,correct_hash,correct_program,root_hash)=&sources[0];
    let (_,wrong_hash,wrong_program,_)=&sources[1];

    // Same full initial program and identical two paid WRITE import ticks.
    // The registry visibility is the only difference in the same-rule pair.
    let (mut visible,hv)=boot(Family::P2,19);
    let (mut sham,hs)=boot(Family::P2,19);
    assert_eq!(hv,*root_hash);
    assert_eq!(hv,hs);
    assert_eq!(visible.state_hash(),sham.state_hash());
    let cost_visible=import(&mut visible,correct_hash,correct_program,REGISTRY);
    let cost_sham=import(&mut sham,correct_hash,correct_program,
        "/self/tools/transform-v0");
    assert_eq!(cost_visible,cost_sham);
    assert_eq!(visible.state.step,sham.state.step);
    assert_eq!(visible.state.nodes,sham.state.nodes);
    assert_eq!(visible.state.organisms,sham.state.organisms);
    run(&mut visible);run(&mut sham);
    verify_task(&visible,19,Family::P2);
    verify_task(&sham,19,Family::P2);
    assert_eq!(stored(&visible).0,*correct_hash);
    assert_eq!(stored(&sham).0,*correct_hash);
    assert!(sham.audit.iter().any(|e|e.result.code==Code::Conflict &&
        matches!(&e.action,Some(Action::Write{key,..})
            if key==&format!("/self/versions/{correct_hash}"))),
        "sham re-synthesizes identical tool but immutable version already exists");
    assert!(gas(&visible)<gas(&sham),
        "visible reuse must save total paid A cost versus identical import sham");
    println!("S30 matched visible_A={} sham_A={} D_minus_sham={} import_both={} visible_ticks={} sham_ticks={} B_each=20",
        gas(&visible),gas(&sham),
        gas(&visible) as i64-gas(&sham) as i64,cost_visible,
        visible.state.step,sham.state.step);

    // Wrong inherited binary composition is a *candidate*, not authority.
    // Target must reject it on its own public labels, regenerate from the
    // same 18-search vocabulary, and CAS-overwrite the old registry.
    let (mut repair,hr)=boot(Family::P2,19);
    assert_eq!(hr,hv);
    let wrong_cost=import(&mut repair,wrong_hash,wrong_program,REGISTRY);
    assert!(wrong_cost>0);
    run(&mut repair);
    verify_task(&repair,19,Family::P2);
    assert!(exec_before_switch(&repair,wrong_hash)>0,
        "inherited wrong tool must be genuinely probed before correction");
    let (repaired_hash,repaired_program)=stored(&repair);
    verify_composed_shape(&repaired_program);
    assert_ne!(repaired_hash,*wrong_hash,
        "cannot just keep stale inherited hash");
    assert_eq!(repaired_hash,*correct_hash,
        "same target task should select same validated immutable expression");
    assert_eq!(repair.state.objects[REGISTRY].version,1,
        "in-place paid CAS from inherited version0 to replacement version1");
    assert!(repair.audit.iter().any(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==REGISTRY)),"actual CAS replacement is necessary");
    assert!(generated_after_boot(&repair,&repaired_hash));
    println!("S30 semantic repair wrong_hash={wrong_hash} corrected_hash={repaired_hash} target_A={} ticks={} B=20 registry_v1",
        gas(&repair),repair.state.step);
}
