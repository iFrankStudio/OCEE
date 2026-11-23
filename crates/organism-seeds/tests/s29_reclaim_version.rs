//! Conservative, metered version retirement in the actual deterministic world.
//! This validates the explicit reference safety boundary, NOT general GC of
//! encoded unknown pointers or a self-invented organism maintenance policy.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s29_public_transform::{
    s29_atomic_publish_repair_root,s29_template,REGISTRY,PREDICTION
};
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,Value,WorldCall
};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,Code,Config,NodeId,World,ORGANISM_ID,hidden_transform,TransformTask
};
fn lit(b:&[u8])->Op{
    Op::ConstBytes{hex:b.iter().map(|x|format!("{x:02x}")).collect()}
}
fn put(w:&mut World,key:String,bytes:Vec<u8>){
    let r=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    }));
    assert_eq!(r.code,Code::Ok,"researcher-installed fixture must be paid");
}
fn alter(w:&mut World,key:&str,bytes:Vec<u8>,version:u64){
    let r=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:key.into(),bytes,
        expected_version:Some(version)
    }));
    assert_eq!(r.code,Code::Ok,"CAS change must be real and metered");
}
fn spend(w:&World)->(u64,u64){
    (w.audit.iter().map(|a|a.result.vm_gas_charged).sum(),
     w.audit.iter().map(|a|a.result.gas_charged).sum())
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn run(w:&mut World,max:u64){
    let mut completed=false;
    for _ in 0..max{
        let x=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(x.code,Code::Denied|Code::InvalidCode),
            "autonomous VM must not execute a forbidden call");
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|x|x.finished){
            completed=true;break;
        }
    }
    assert!(completed,"active root must complete in frozen bounded ticks");
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}
fn wrong_tool()->Program{
    // Researcher-supplied legitimate but semantically wrong tool:
    // [32,3,8] -> 3 (first sample collision), [18,2,7,4] -> 2.
    let mut tool=s29_template();
    tool.functions[0].blocks[0].instructions[0].op=Op::ConstU32(1);
    tool.functions[0].blocks[0].instructions[1].op=Op::ByteAt(0,1);
    tool.validate().unwrap();tool
}
fn intended_len()->(String,Program){
    let mut p=s29_template();
    p.functions[0].blocks[0].instructions[1].op=Op::Len(0);
    (p.content_hash().unwrap(),p)
}
fn boot_repaired()->(World,String,String,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s29_atomic_publish_repair_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let root_hash=root.content_hash().unwrap();
    let old=wrong_tool();let old_hash=old.content_hash().unwrap();
    let (new_hash,_)=intended_len();
    let mut w=World::with_protocol(Config{
        seed:129,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,42);
    for p in [p1,p2,root,old]{
        put(&mut w,format!("/self/versions/{}",
            p.content_hash().unwrap()),p.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),root_hash.as_bytes().to_vec());
    put(&mut w,REGISTRY.into(),old_hash.as_bytes().to_vec());
    run(&mut w,16_384);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects[REGISTRY].version,1);
    assert!(w.state.objects.contains_key(&format!("/self/versions/{old_hash}")));
    assert!(w.state.objects.contains_key(&format!("/self/versions/{new_hash}")));
    assert_eq!(b_gas(&w),20);
    let tr:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let pred:u32=serde_json::from_slice(
        &w.state.objects[PREDICTION].bytes).unwrap();
    assert_eq!(pred,hidden_transform(129,&tr.held_out_input));
    (w,old_hash,new_hash,root_hash)
}
/// A researcher preinstalls this static, audited maintenance routine.
/// Its *real* VM requests ReclaimVersion, but the decision to switch to
/// maintenance is host-supplied, not an endogenous adaptive goal.
fn maintainer(version:&str)->Program{
    let main=Function{
        name:"main".into(),args:vec![Type::Bytes],
        returns:Type::U32,
        blocks:vec![
            Block{
                id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![Instruction{id:1,op:lit(version.as_bytes())}],
                terminator:Terminator::Yield{
                    call:WorldCall::ReclaimVersion,args:vec![1],
                    next:"reply".into(),next_args:vec![]
                }
            },
            Block{
                id:"reply".into(),params:vec![Param{id:2,ty:Type::Bytes}],
                instructions:vec![
                    Instruction{id:3,op:lit(b"code")},
                    Instruction{id:4,op:Op::JsonFieldString(2,3)},
                    Instruction{id:5,op:lit(b"Ok")},
                    Instruction{id:6,op:Op::EqBytes(4,5)}
                ],
                terminator:Terminator::Branch{
                    condition:6,if_true:"accepted".into(),true_args:vec![],
                    if_false:"refused".into(),false_args:vec![]
                }
            },
            Block{
                id:"accepted".into(),params:vec![],
                instructions:vec![Instruction{id:7,op:Op::ConstU32(1)}],
                terminator:Terminator::Return{value:7}
            },
            Block{
                id:"refused".into(),params:vec![],
                instructions:vec![Instruction{id:8,op:Op::ConstU32(0)}],
                terminator:Terminator::Return{value:8}
            },
        ]
    };
    let p=Program{format_version:1,entry_function:"main".into(),
        functions:vec![main]};
    p.validate().expect("typed maintenance VM is statically bounded");p
}
fn activate_maintenance(w:&mut World,target:&str)->String{
    let p=maintainer(target);
    let hash=p.content_hash().unwrap();
    if !w.state.objects.contains_key(&format!("/self/versions/{hash}")){
        put(w,format!("/self/versions/{hash}"),p.canonical_bytes());
    }
    // A real S29/P2 root can already have switched /self/active more than
    // once in the world. Do NOT assume version 0; pay for the existing
    // world-provided active-version receipt and CAS exactly that version.
    let read=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(read.code,Code::Ok,"paid active version observation");
    let reply:serde_json::Value=serde_json::from_slice(&read.payload).unwrap();
    let current=reply.get("version").and_then(|v|v.as_u64())
        .expect("kernel-issued monotonic version");
    alter(w,"/self/active",hash.as_bytes().to_vec(),current);
    hash
}
fn reclaim_audit(w:&World,target:&str)->Vec<(Code,u64,Vec<u8>)>{
    w.audit.iter().filter_map(|r|match &r.action{
        Some(Action::ReclaimVersion{program_hash,..})
            if program_hash==target=>Some((
                r.result.code,r.result.gas_charged,r.result.payload.clone())),
        _=>None
    }).collect()
}
fn reclaim(w:&mut World,hash:&str)->ocee_world_kernel::ActionResult{
    w.tick(Some(Action::ReclaimVersion{
        actor:ORGANISM_ID,program_hash:hash.into()
    }))
}
fn test_post_repair(){
    let (mut w,old_hash,new_hash,root)=boot_repaired();
    let size=w.state.objects[&format!("/self/versions/{old_hash}")]
        .bytes.len() as u64;
    let prev_b=b_gas(&w);
    let maintenance=activate_maintenance(&mut w,&old_hash);
    assert_ne!(maintenance,root);
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    run(&mut w,24);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    let trail=reclaim_audit(&w,&old_hash);
    assert_eq!(trail.len(),1);
    assert_eq!(trail[0].0,Code::Ok);
    assert!(trail[0].1>0);
    assert_eq!(u64::from_le_bytes(trail[0].2.as_slice()
        .try_into().unwrap()),size);
    assert!(!w.state.objects.contains_key(
        &format!("/self/versions/{old_hash}")));
    assert_eq!(used-w.state.organisms[&ORGANISM_ID].storage_used,size);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),prev_b);
    let missing=w.tick(Some(Action::Exec{
        actor:ORGANISM_ID,program_hash:old_hash.clone(),
        input:vec![32,3,8]
    }));
    assert_eq!(missing.code,Code::NotFound);
    let preserved=w.tick(Some(Action::Exec{
        actor:ORGANISM_ID,program_hash:new_hash.clone(),
        input:vec![18,2,7,4]
    }));
    assert_eq!(preserved.code,Code::Ok);
    assert_eq!(serde_json::from_slice::<Value>(&preserved.payload).unwrap(),
        Value::U32(4));
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (vm,action)=spend(&w);
    println!("S29_RECLAIM POST_REPAIR old={old_hash} active={maintenance} \
        new={new_hash} freed={size} gc_A={} total_VM={vm} \
        total_action={action} B={} step={}",
        trail[0].1,b_gas(&w),w.state.step);
}
fn test_registered(){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    put(&mut w,"/self/tools/second-reference".into(),
        old_hash.as_bytes().to_vec());
    let _=activate_maintenance(&mut w,&old_hash);
    let stored=w.state.organisms[&ORGANISM_ID].storage_used;
    run(&mut w,24);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(reclaim_audit(&w,&old_hash)[0].0,Code::Conflict);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,stored);
    assert!(w.state.objects.contains_key(
        &format!("/self/versions/{old_hash}")));
    // Only an actual paid CAS moves the second reference away. Changing
    // a label is not an oracle: the new tool version already exists.
    alter(&mut w,"/self/tools/second-reference",
        new_hash.as_bytes().to_vec(),0);
    let _=activate_maintenance(&mut w,&old_hash);
    let before=w.state.organisms[&ORGANISM_ID].storage_used;
    run(&mut w,24);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    let trail=reclaim_audit(&w,&old_hash);
    assert_eq!(trail.len(),2);
    assert_eq!(trail[1].0,Code::Ok);
    assert!(trail[1].1>0);
    assert_eq!(before-w.state.organisms[&ORGANISM_ID].storage_used,
        u64::from_le_bytes(trail[1].2.as_slice().try_into().unwrap()));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    println!("S29_RECLAIM REGISTERED first=Conflict second=Ok \
        after_valid_secondary_CAS=true freed={} B={}",
        u64::from_le_bytes(trail[1].2.as_slice().try_into().unwrap()),
        b_gas(&w));
}
fn test_roots(){
    let p=s29_template();let root=p.content_hash().unwrap();
    let mut w=World::new(Config::default());
    put(&mut w,format!("/self/versions/{root}"),p.canonical_bytes());
    put(&mut w,"/self/active".into(),root.as_bytes().to_vec());
    let before=w.state.organisms[&ORGANISM_ID].storage_used;
    let direct=reclaim(&mut w,&root);
    assert_eq!(direct.code,Code::Conflict);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,before);
    // Original root stays pinned for safe restart after a paid active switch.
    let maintenance=activate_maintenance(&mut w,&root);
    assert_ne!(maintenance,root);
    let initial=w.state.initial_root_hash.clone();
    assert_eq!(initial,Some(root.clone()));
    run(&mut w,24);
    assert_eq!(reclaim_audit(&w,&root).last().unwrap().0,Code::Conflict);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    // Also protects the currently running maintenance root itself.
    assert_eq!(reclaim(&mut w,&maintenance).code,Code::Conflict);
    assert_eq!(reclaim(&mut w,"bad hash").code,Code::InvalidCode);
    assert_eq!(reclaim(&mut w,&"f".repeat(64)).code,Code::NotFound);
    assert!(w.state.objects.contains_key(&format!("/self/versions/{root}")));
    assert!(w.state.objects.contains_key(
        &format!("/self/versions/{maintenance}")));
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_RECLAIM ROOTS active=Conflict initial=Conflict \
        ongoing_maintenance=Conflict malformed=InvalidCode missing=NotFound");
}
fn test_raw_refs(){
    let p=s29_template();let hash=p.content_hash().unwrap();
    let mut w=World::new(Config::default());
    put(&mut w,format!("/self/versions/{hash}"),p.canonical_bytes());
    put(&mut w,"/self/pins/explicit".into(),
        format!("pinned:{hash}").into_bytes());
    let before=w.state.organisms[&ORGANISM_ID].storage_used;
    assert_eq!(reclaim(&mut w,&hash).code,Code::Conflict);
    assert_eq!(before,w.state.organisms[&ORGANISM_ID].storage_used);
    alter(&mut w,"/self/pins/explicit",b"released".to_vec(),0);
    // A pending actual SEND with raw hash must block as well.
    let sent=w.tick(Some(Action::Send{
        actor:ORGANISM_ID,destination:NodeId::C,
        bytes:format!("in-flight:{hash}").into_bytes()
    }));
    assert_eq!(sent.code,Code::Ok);
    assert_eq!(reclaim(&mut w,&hash).code,Code::Conflict);
    assert!(w.state.objects.contains_key(&format!("/self/versions/{hash}")));
    // Independent world: after paid pin release and no in-flight messages,
    // the original program may be truly collected.
    let mut clean=World::new(Config::default());
    put(&mut clean,format!("/self/versions/{hash}"),p.canonical_bytes());
    put(&mut clean,"/self/pins/explicit".into(),hash.as_bytes().to_vec());
    assert_eq!(reclaim(&mut clean,&hash).code,Code::Conflict);
    alter(&mut clean,"/self/pins/explicit",b"released".to_vec(),0);
    let before=clean.state.organisms[&ORGANISM_ID].storage_used;
    let result=reclaim(&mut clean,&hash);
    assert_eq!(result.code,Code::Ok);
    assert!(result.gas_charged>0);
    assert_eq!(before-clean.state.organisms[&ORGANISM_ID].storage_used,
        p.canonical_bytes().len() as u64);
    // Gas rejection after every protective check must not remove the data.
    let mut no_gas=World::new(Config::default());
    put(&mut no_gas,format!("/self/versions/{hash}"),p.canonical_bytes());
    no_gas.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=0;
    let before=no_gas.state.organisms[&ORGANISM_ID].storage_used;
    assert_eq!(reclaim(&mut no_gas,&hash).code,Code::Gas);
    assert!(no_gas.state.objects.contains_key(
        &format!("/self/versions/{hash}")));
    assert_eq!(before,no_gas.state.organisms[&ORGANISM_ID].storage_used);
    for each in [&w,&clean,&no_gas]{
        assert_eq!(each.state_hash(),World::from_snapshot(
            &each.snapshot_json()).unwrap().state_hash());
    }
    println!("S29_RECLAIM OPAQUE_REF pin=Conflict message=Conflict \
        released=Ok bytes={} charge_A={} gas_rejected=Gas no_partial=true",
        p.canonical_bytes().len(),result.gas_charged);
}
#[test]
#[ignore = "charged world native reference-safe version reclamation"]
fn s29_reclaim_version_native(){
    let arm=std::env::var("OCEE_S29_GC_ARM").expect("scoped test arm");
    match arm.as_str(){
        "POST_REPAIR"=>test_post_repair(),
        "REGISTERED"=>test_registered(),
        "ROOTS"=>test_roots(),
        "OPAQUE_REF"=>test_raw_refs(),
        other=>panic!("unexpected arm {other}")
    }
}
