//! S29: observable operator selection by active VM, no host-injected winner.
//! Three public transform families run across P1->P2. The world-only
//! oracle is consulted AFTER autonomous execution, never by the organism.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s29_public_transform::{
    s29_evidence_selected_root,s29_template,REGISTRY,PREDICTION
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    hidden_transform,Action,Code,Config,TransformTask,WorkItem,World,ORGANISM_ID
};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None,
    })).code,Code::Ok,"all imports and initial versions pay real WRITE");
}
fn gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn bgas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn boot(family:Family,seed:u64)->(World,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s29_evidence_selected_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,42);
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run(w:&mut World){
    for _ in 0..16_384{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "S29 VM step={} code={:?} block={:?}",
            w.state.step,r.code,w.state.organisms[&ORGANISM_ID]
                .vm_continuation.as_ref().map(|c|c.block.as_str()));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"must finish with capped logical ticks");
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
}
fn stored(w:&World)->(String,Program){
    let hash=String::from_utf8(w.state.objects[REGISTRY].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
fn expected_tool_op(p:&Program,op:&Op){
    assert_eq!(&p.functions[0].blocks[0].instructions[1].op,op,
        "tool content must embody selected operation, not just a label");
}
fn transform(w:&World)->TransformTask{
    serde_json::from_slice(&w.state.objects["/public/E001/transform"].bytes).unwrap()
}
fn verify_prediction(w:&World,seed:u64){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)),
        "a real task after choosing the portable transform must succeed");
    let task=transform(w);
    assert_eq!(task.examples.len(),2);
    let got:u32=serde_json::from_slice(&w.state.objects[PREDICTION].bytes).unwrap();
    let expected=hidden_transform(seed,&task.held_out_input);
    assert_eq!(got,expected,
        "independent held-out oracle is only used AFTER the VM saved prediction");
}
fn verify_remote(w:&World,family:Family){
    assert_eq!(bgas(w),20);
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let expected=job.values.iter().enumerate().fold(0u32,|sum,(i,&x)|
        sum.wrapping_add(x.rotate_left((i%32) as u32)));
    let path=match family{
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!(),
    };
    let value=u32::from_le_bytes(w.state.objects[path].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(value,expected);
}
fn import(w:&mut World,hash:&str,p:&Program,registry:&str)->u64{
    let pre=gas(w);
    put(w,format!("/self/versions/{hash}"),p.canonical_bytes());
    put(w,registry.into(),hash.as_bytes().to_vec());
    gas(w)-pre
}
fn exec_before_dispatch(w:&World,hash:&str)->usize{
    let end=w.audit.iter().position(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,..})
            if key=="/self/active" && a.step>=4))
        .unwrap_or(w.audit.len());
    w.audit[..end].iter().filter(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Exec{program_hash,..})
            if program_hash==hash)).count()
}
fn version_write_since(w:&World,hash:&str,step:u64,code:Code)->bool{
    let key=format!("/self/versions/{hash}");
    w.audit.iter().any(|a|a.step>=step && a.result.code==code &&
        matches!(&a.action,Some(Action::Write{key:k,..}) if k==&key))
}
#[test]
fn active_p1_selects_three_ops_and_p2_reuses_sealed_programs_on_new_inputs(){
    let specs=[
        (42u64,129u64,Op::Len(0)),
        (43u64,130u64,Op::XorFold(0)),
        (44u64,131u64,Op::ByteAt(0,1)),
    ];
    let mut source_hashes=std::collections::BTreeSet::new();
    let mut chosen=std::collections::BTreeSet::new();
    let mut first_donor=None;
    let mut xor_donor=None;
    for (source_seed,target_seed,operator) in specs {
        let (mut donor,root_source)=boot(Family::P1,source_seed);
        run(&mut donor);
        verify_prediction(&donor,source_seed);
        verify_remote(&donor,Family::P1);
        let (hash,program)=stored(&donor);
        expected_tool_op(&program,&operator);
        assert_eq!(exec_before_dispatch(&donor,&hash),3,
            "two independent training examples plus held-out EXEC");
        assert!(version_write_since(&donor,&hash,4,Code::Ok));
        assert_ne!(program.content_hash().unwrap(),
            s29_template().content_hash().unwrap());
        source_hashes.insert(root_source.clone());
        chosen.insert(hash.clone());

        let (mut inherited,root_target)=boot(Family::P2,target_seed);
        assert_eq!(root_target,root_source,
            "same initial organism hash across distinct family/semantic tasks");
        let begin=inherited.state.step;
        let import_gas=import(&mut inherited,&hash,&program,REGISTRY);
        assert!(import_gas>0);
        run(&mut inherited);
        verify_prediction(&inherited,target_seed);
        verify_remote(&inherited,Family::P2);
        assert_eq!(stored(&inherited).0,hash);
        assert_eq!(exec_before_dispatch(&inherited,&hash),3,
            "imported tool must validate on target's DIFFERENT examples");
        assert!(!version_write_since(&inherited,&hash,begin+2,Code::Ok),
            "target must NOT regenerate the inherited executable");
        assert!(!version_write_since(&inherited,&hash,begin+2,Code::Conflict));
        assert_ne!(transform(&donor).held_out_input,
            transform(&inherited).held_out_input,
            "held-out task bytes must genuinely differ between worlds");
        println!("S29 transfer {:?} P1_seed={} P2_seed={} donor_hash={} target_A={} target_ticks={} import_A={} B=20",
            operator,source_seed,target_seed,hash,
            gas(&inherited),inherited.state.step,import_gas);
        if source_seed==42 {first_donor=Some((hash,program));}
        if source_seed==43 {xor_donor=Some((hash,program));}
    }
    assert_eq!(source_hashes.len(),1,"all tasks use same initial root");
    assert_eq!(chosen.len(),3,"labels must produce THREE distinct executable algorithms");

    // Equal-cost, identical-schedule but invisible tool counterfactual.
    let (hash,prog)=first_donor.expect("length donor");
    let (mut visible,hv)=boot(Family::P2,129);
    let (mut hidden,hh)=boot(Family::P2,129);
    assert_eq!(hv,hh);
    let vcost=import(&mut visible,&hash,&prog,REGISTRY);
    let hcost=import(&mut hidden,&hash,&prog,
        "/self/tools/portable-transform-s2x");
    assert_eq!(vcost,hcost);
    assert_eq!(visible.state.step,hidden.state.step);
    assert_eq!(visible.state.nodes,hidden.state.nodes);
    assert_eq!(visible.state.organisms,hidden.state.organisms);
    run(&mut visible);
    run(&mut hidden);
    verify_prediction(&visible,129);verify_prediction(&hidden,129);
    verify_remote(&visible,Family::P2);verify_remote(&hidden,Family::P2);
    let key=format!("/self/versions/{hash}");
    assert!(hidden.audit.iter().any(|a|a.result.code==Code::Conflict &&
        matches!(&a.action,Some(Action::Write{key:k,..}) if k==&key)),
        "sham must reselect same op and collide with previously stored hash");
    assert!(gas(&visible)<gas(&hidden),
        "same cost and schedule, registry visibility must save paid work");
    println!("S29 matched-sham visible_A={} hidden_A={} delta_A={} import_A={} visible_ticks={} sham_ticks={}",
        gas(&visible),gas(&hidden),
        gas(&visible) as i64-gas(&hidden) as i64,vcost,
        visible.state.step,hidden.state.step);

    // A valid inherited tool from the WRONG task must not be trusted by
    // hash/provenance alone; it is EXEC'd and rejected by public labels.
    let (xor_hash,xor_program)=xor_donor.expect("XOR donor");
    let (mut incompatible,_)=boot(Family::P2,129);
    import(&mut incompatible,&xor_hash,&xor_program,REGISTRY);
    run(&mut incompatible);
    assert_eq!(incompatible.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(bgas(&incompatible),0);
    assert!(exec_before_dispatch(&incompatible,&xor_hash)>0);
    assert!(!incompatible.state.objects.contains_key(PREDICTION));
    println!("S29 wrong-semantics tool rejected before B; A={} ticks={} B=0",
        gas(&incompatible),incompatible.state.step);
}
