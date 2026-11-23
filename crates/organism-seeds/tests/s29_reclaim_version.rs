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


/// Fixed finite researcher-authored maintenance program, not an organism-
/// invented policy. Its four actual VM WorldCalls pay one certificate build
/// plus three independently metered indexed retirement actions.
fn indexed_maintenance(targets:&[String])->Program{
    assert_eq!(targets.len(),3);
    let mut blocks=Vec::<Block>::new();
    blocks.push(Block{
        id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
        instructions:vec![],
        terminator:Terminator::Yield{
            call:WorldCall::IndexVersionRefs,args:vec![],
            next:"certified".into(),next_args:vec![]
        }
    });
    let code_ok=|register:u32|vec![
        Instruction{id:3,op:lit(b"code")},
        Instruction{id:4,op:Op::JsonFieldString(register,3)},
        Instruction{id:5,op:lit(b"Ok")},
        Instruction{id:6,op:Op::EqBytes(4,5)}
    ];
    blocks.push(Block{
        id:"certified".into(),params:vec![Param{id:2,ty:Type::Bytes}],
        instructions:code_ok(2),
        terminator:Terminator::Branch{
            condition:6,if_true:"delete0".into(),true_args:vec![],
            if_false:"failed".into(),false_args:vec![]
        }
    });
    for (i,target) in targets.iter().enumerate(){
        blocks.push(Block{
            id:format!("delete{i}"),params:vec![],
            instructions:vec![Instruction{id:1,op:lit(target.as_bytes())}],
            terminator:Terminator::Yield{
                call:WorldCall::ReclaimIndexedVersion,args:vec![1],
                next:format!("reply{i}"),next_args:vec![]
            }
        });
        blocks.push(Block{
            id:format!("reply{i}"),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:code_ok(2),
            terminator:Terminator::Branch{
                condition:6,
                if_true:if i+1==targets.len(){
                    "done".into()
                }else{format!("delete{}",i+1)},
                true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        });
    }
    for (name,value,reg) in [("done",1,7),("failed",0,8)]{
        blocks.push(Block{
            id:name.into(),params:vec![],
            instructions:vec![Instruction{
                id:reg,op:Op::ConstU32(value)
            }],
            terminator:Terminator::Return{value:reg}
        });
    }
    let p=Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],
            returns:Type::U32,blocks
        }]
    };
    p.validate().expect("bounded typed indexed maintenance root");
    p
}
fn version_key(hash:&str)->String{format!("/self/versions/{hash}")}
fn extra_unreferenced_tools()->Vec<Program>{
    let mut p0=s29_template();
    p0.functions[0].blocks[0].instructions[1].op=Op::XorFold(0);
    let mut p1=s29_template();
    // ByteAt requires U32 index register 1; instruction 1 is ConstU32(0).
    p1.functions[0].blocks[0].instructions[1].op=Op::ByteAt(0,1);
    for p in [&p0,&p1]{p.validate().unwrap();}
    assert_ne!(p0.content_hash().unwrap(),p1.content_hash().unwrap());
    vec![p0,p1]
}
fn test_amortized_indexed_reclaim(){
    let (mut w,old_hash,new_hash,previous_root)=boot_repaired();
    let mut targets=vec![old_hash.clone()];
    for program in extra_unreferenced_tools(){
        let hash=program.content_hash().unwrap();
        assert!(!w.state.objects.contains_key(&version_key(&hash)));
        put(&mut w,version_key(&hash),program.canonical_bytes());
        targets.push(hash);
    }
    assert_eq!(targets.len(),3);
    assert!(targets.iter().all(|hash|hash!=&previous_root &&
        hash!=&new_hash));
    let root=indexed_maintenance(&targets);
    let root_hash=root.content_hash().unwrap();
    put(&mut w,version_key(&root_hash),root.canonical_bytes());
    // Same paid, monotonic active-version discovery and real CAS as the
    // previous S29 GC study; no stale test-fixture assumption about v0.
    let read=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(read.code,Code::Ok);
    let version:serde_json::Value=
        serde_json::from_slice(&read.payload).unwrap();
    alter(&mut w,"/self/active",root_hash.as_bytes().to_vec(),
        version["version"].as_u64().unwrap());
    let mut legacy=World::from_snapshot(&w.snapshot_json()).unwrap();
    assert_eq!(legacy.state_hash(),w.state_hash(),
        "paired GC fee comparison must start from identical world state");
    let lengths=targets.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64
    ).collect::<Vec<_>>();
    let expected_freed=lengths.iter().sum::<u64>();
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    let b_before=b_gas(&w);
    let after_start=w.state.step;
    run(&mut w,48);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    let actions=w.audit.iter().filter(|r|r.step>=after_start &&
        matches!(&r.action,
            Some(Action::IndexVersionRefs{..})|
            Some(Action::ReclaimIndexedVersion{..})))
        .collect::<Vec<_>>();
    assert_eq!(actions.len(),4,
        "real activity must build one certificate then collect 3");
    assert!(actions.iter().all(|r|r.result.code==Code::Ok));
    assert!(matches!(&actions[0].action,Some(Action::IndexVersionRefs{..})));
    let fast_action_a=actions.iter().map(|r|r.result.gas_charged).sum::<u64>();
    let scan_a=actions[0].result.gas_charged;
    let indexed_fees=actions[1..].iter().map(|r|r.result.gas_charged)
        .collect::<Vec<_>>();
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used-expected_freed);
    assert!(targets.iter().all(|h|
        !w.state.objects.contains_key(&version_key(h))));
    assert!(w.state.version_ref_index.is_some(),
        "removing a target alone cannot create a new reference");
    assert_eq!(b_gas(&w),b_before);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());

    let mut legacy_fees=Vec::new();
    for hash in &targets{
        let outcome=legacy.tick(Some(Action::ReclaimVersion{
            actor:ORGANISM_ID,program_hash:hash.clone()
        }));
        assert_eq!(outcome.code,Code::Ok,
            "matched full-scan baseline must delete same actual versions");
        legacy_fees.push(outcome.gas_charged);
    }
    let baseline_a=legacy_fees.iter().sum::<u64>();
    assert!(baseline_a>fast_action_a,
        "no claim of improvement without charged A saving on matched world");
    assert_eq!(legacy.state.organisms[&ORGANISM_ID].storage_used,
        w.state.organisms[&ORGANISM_ID].storage_used);
    assert!(targets.iter().all(|h|
        !legacy.state.objects.contains_key(&version_key(h))));
    assert_eq!(b_gas(&legacy),b_before);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    assert_eq!(legacy.state_hash(),World::from_snapshot(
        &legacy.snapshot_json()).unwrap().state_hash());
    let (vm_a,action_a)=spend(&w);
    println!("S29_INDEX AMORTIZED root={root_hash} \
        old={old_hash} new={new_hash} targets={targets:?} \
        lengths={lengths:?} freed={expected_freed} \
        index_scan_bytes={} build_A={scan_a} \
        fast_gc_A={indexed_fees:?} fast_total_A={fast_action_a} \
        old_full_gc_A={legacy_fees:?} old_total_A={baseline_a} \
        saved_A={} B={} run_VM_A={vm_a} run_action_A={action_a} \
        world_step={}",
        w.state.version_ref_index.as_ref().unwrap().scanned_bytes,
        baseline_a-fast_action_a,b_gas(&w),w.state.step);
}
fn tiny_unregistered()->(World,String){
    let tool=s29_template();
    let hash=tool.content_hash().unwrap();
    let mut w=World::new(Config::default());
    put(&mut w,version_key(&hash),tool.canonical_bytes());
    (w,hash)
}
fn index_refs(w:&mut World)->ocee_world_kernel::ActionResult{
    w.tick(Some(Action::IndexVersionRefs{actor:ORGANISM_ID}))
}
fn fast_reclaim(w:&mut World,hash:&str)->ocee_world_kernel::ActionResult{
    w.tick(Some(Action::ReclaimIndexedVersion{
        actor:ORGANISM_ID,program_hash:hash.into()
    }))
}
fn test_stale_object_index(){
    let (mut w,hash)=tiny_unregistered();
    // A fake self-owned purported "index" cannot become trusted metadata.
    put(&mut w,"/self/index".into(),b"{\"safe\":true}".to_vec());
    let cert=index_refs(&mut w);
    assert_eq!(cert.code,Code::Ok);
    let before=w.state.organisms[&ORGANISM_ID].storage_used;
    assert!(w.state.version_ref_index.is_some());
    put(&mut w,"/self/pins/later".into(),hash.as_bytes().to_vec());
    assert!(w.state.version_ref_index.is_none(),
        "real object WRITE must invalidate certificate before fast GC");
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    assert!(used>before);
    assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict);
    assert_eq!(reclaim(&mut w,&hash).code,Code::Conflict,
        "old full scan agrees there is a newly introduced live ref");
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,used);
    assert!(w.state.objects.contains_key(&version_key(&hash)));
    let result=index_refs(&mut w);
    assert_eq!(result.code,Code::Ok);
    assert!(w.state.version_ref_index.as_ref().unwrap()
        .referenced.contains(&hash));
    assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict,
        "fresh index must also block known live reference");
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_INDEX STALE_WRITE stale=Conflict old_full=Conflict \
        rebuilding_refs=detected forged_self_index=ignored no_delete=true");
}
fn test_stale_message_index(){
    let (mut w,hash)=tiny_unregistered();
    assert_eq!(index_refs(&mut w).code,Code::Ok);
    assert!(w.state.version_ref_index.is_some());
    let sent=w.tick(Some(Action::Send{
        actor:ORGANISM_ID,destination:NodeId::C,
        bytes:format!("future_exec:{hash}").into_bytes()
    }));
    assert_eq!(sent.code,Code::Ok);
    assert!(w.state.version_ref_index.is_none(),
        "successful SEND must invalidate index");
    let original_used=w.state.organisms[&ORGANISM_ID].storage_used;
    assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict);
    assert_eq!(reclaim(&mut w,&hash).code,Code::Conflict,
        "pending transport actually protects the raw hash");
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,original_used);
    assert!(w.state.objects.contains_key(&version_key(&hash)));
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_INDEX STALE_MESSAGE successful_SEND=true \
        index_invalidated=true fast=Conflict full=Conflict");
}
fn test_index_guards(){
    let (mut w,hash)=tiny_unregistered();
    put(&mut w,"/self/active".into(),hash.as_bytes().to_vec());
    assert_eq!(index_refs(&mut w).code,Code::Ok);
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict,
        "active root protected even if own bytes exempt from indexing");
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,used);
    assert_eq!(fast_reclaim(&mut w,"not hex").code,Code::InvalidCode);
    assert_eq!(fast_reclaim(&mut w,&"d".repeat(64)).code,Code::NotFound);
    // Active root switches invalidate any old root exemption even if
    // the candidate program remains installed under a valid hash.
    let mut second=s29_template();
    second.functions[0].blocks[0].instructions[1].op=Op::Len(0);
    let other=second.content_hash().unwrap();
    put(&mut w,version_key(&other),second.canonical_bytes());
    assert!(w.state.version_ref_index.is_none());
    alter(&mut w,"/self/active",other.as_bytes().to_vec(),0);
    assert_eq!(index_refs(&mut w).code,Code::Ok);
    assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict,
        "initial recovery root protected after active switch");
    let mut clean=tiny_unregistered().0;
    let candidate=s29_template().content_hash().unwrap();
    assert_eq!(index_refs(&mut clean).code,Code::Ok);
    clean.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=0;
    let before=clean.state.organisms[&ORGANISM_ID].storage_used;
    assert_eq!(fast_reclaim(&mut clean,&candidate).code,Code::Gas);
    assert!(clean.state.objects.contains_key(&version_key(&candidate)));
    assert_eq!(before,clean.state.organisms[&ORGANISM_ID].storage_used);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    assert_eq!(clean.state_hash(),World::from_snapshot(
        &clean.snapshot_json()).unwrap().state_hash());
    println!("S29_INDEX GUARDS active=Conflict initial=Conflict \
        stale_on_root_change=true malformed=InvalidCode missing=NotFound \
        low_Gas=Gas no_partial=true");
}

#[test]
#[ignore="charged native paired reference-certificate GC and stale-index controls"]
fn s29_indexed_gc_native(){
    let arm=std::env::var("OCEE_S29_INDEX_ARM").expect("scoped CI matrix");
    match arm.as_str(){
        "AMORTIZED"=>test_amortized_indexed_reclaim(),
        "STALE_WRITE"=>test_stale_object_index(),
        "STALE_MESSAGE"=>test_stale_message_index(),
        "GUARDS"=>test_index_guards(),
        x=>panic!("unknown arm {x}")
    }
}


/// The researcher's *fixed* typed policy graph decides at runtime using
/// the kernel's actual charged own-resource observation. The researcher
/// supplied the candidate hashes and thresholds; only this bounded
/// resource-conditional branch is an endogenous VM action.
fn pressure_gc_policy(targets:&[String])->Program{
    let mut root=indexed_maintenance(targets);
    let f=&mut root.functions[0];
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry")
        .expect("existing indexed maintenance entry");
    entry.instructions=vec![
        Instruction{id:40,op:lit(b"/sys/resource-status")}
    ];
    entry.terminator=Terminator::Yield{
        call:WorldCall::Read,args:vec![40],
        next:"policy_sensor_reply".into(),next_args:vec![]
    };
    // Existing "certified" and "delete0/1/2" follow real paid
    // IndexVersionRefs unchanged, with all failure paths returning U32(0).
    f.blocks.extend([
        Block{
            id:"policy_sensor_reply".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:3,op:lit(b"code")},
                Instruction{id:4,op:Op::JsonFieldString(2,3)},
                Instruction{id:5,op:lit(b"Ok")},
                Instruction{id:6,op:Op::EqBytes(4,5)}
            ],
            terminator:Terminator::Branch{
                condition:6,if_true:"policy_resources".into(),
                true_args:vec![2],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"policy_resources".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"used")},
                Instruction{id:13,op:Op::JsonFieldU32(11,12)},
                Instruction{id:14,op:lit(b"capacity")},
                Instruction{id:15,op:Op::JsonFieldU32(11,14)},
                Instruction{id:16,op:lit(b"a_available")},
                Instruction{id:17,op:Op::JsonFieldU32(11,16)},
                Instruction{id:18,op:Op::ConstU32(4)},
                Instruction{id:19,op:Op::Div(15,18)},
                Instruction{id:20,op:Op::Sub(15,13)},
                Instruction{id:21,op:Op::Div(20,19)},
                Instruction{id:22,op:Op::ConstU32(0)},
                Instruction{id:23,op:Op::EqU32(21,22)}
            ],
            terminator:Terminator::Branch{
                condition:23,if_true:"policy_budget".into(),
                true_args:vec![17],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"policy_budget".into(),
            params:vec![Param{id:17,ty:Type::U32}],
            instructions:vec![
                Instruction{id:24,op:Op::ConstU32(2200)},
                Instruction{id:25,op:Op::Div(17,24)},
                Instruction{id:26,op:Op::ConstU32(0)},
                Instruction{id:27,op:Op::EqU32(25,26)}
            ],
            terminator:Terminator::Branch{
                condition:27,if_true:"failed".into(),true_args:vec![],
                if_false:"policy_begin_index".into(),false_args:vec![]
            }
        },
        Block{
            id:"policy_begin_index".into(),params:vec![],
            instructions:vec![],
            terminator:Terminator::Yield{
                call:WorldCall::IndexVersionRefs,args:vec![],
                next:"certified".into(),next_args:vec![]
            }
        }
    ]);
    root.validate().expect("resource policy must be static, typed, bounded");
    root
}

/// Preparation is identical for the four experimental arms until the
/// researcher-controlled resource environment differs. No hidden task
/// label is ever passed into the policy VM.
fn prepare_pressure_policy()->(World,Vec<String>,String,String,u64){
    let (mut w,old_hash,new_hash,_previous_root)=boot_repaired();
    let mut targets=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        targets.push(hash);
    }
    let root=pressure_gc_policy(&targets);
    let policy_hash=root.content_hash().unwrap();
    put(&mut w,version_key(&policy_hash),root.canonical_bytes());
    let meta=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(meta.code,Code::Ok);
    let reply:serde_json::Value=
        serde_json::from_slice(&meta.payload).unwrap();
    alter(&mut w,"/self/active",policy_hash.as_bytes().to_vec(),
        reply["version"].as_u64().unwrap());
    assert!(targets.iter().all(|h|w.state.objects.contains_key(
        &version_key(h))));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    let reclaimable_bytes=targets.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64
    ).sum::<u64>();
    assert_eq!(reclaimable_bytes,834);
    (w,targets,policy_hash,new_hash,reclaimable_bytes)
}
fn policy_actions(w:&World,from:u64)->(Vec<&ocee_world_kernel::AuditRecord>,
    Vec<&ocee_world_kernel::AuditRecord>,
    Vec<&ocee_world_kernel::AuditRecord>){
    let mut sensor=Vec::new();let mut index=Vec::new();let mut collect=Vec::new();
    for r in &w.audit {
        if r.step<from{continue;}
        match &r.action{
            Some(Action::Read{key,..}) if key=="/sys/resource-status"=>
                sensor.push(r),
            Some(Action::IndexVersionRefs{..})=>index.push(r),
            Some(Action::ReclaimIndexedVersion{..})=>collect.push(r),
            _=>{}
        }
    }
    (sensor,index,collect)
}
fn policy_status(r:&ocee_world_kernel::AuditRecord)
    ->(u64,u64,u64){
    assert_eq!(r.result.code,Code::Ok);
    assert_eq!(r.result.gas_charged,2);
    let v:serde_json::Value=serde_json::from_slice(&r.result.payload).unwrap();
    (v["used"].as_u64().unwrap(),
     v["capacity"].as_u64().unwrap(),
     v["a_available"].as_u64().unwrap())
}
fn run_pressure_arm(arm:&str){
    let (mut w,targets,root,new_hash,bytes)=prepare_pressure_policy();
    let before_used=w.state.organisms[&ORGANISM_ID].storage_used;
    let a=&mut w.state.nodes.get_mut(&NodeId::A).unwrap();
    match arm {
        "PRESSURE"|"INVALIDATED"|"LOW_GAS"=>{
            a.storage_capacity=before_used+256;
        },
        "RELAXED"=>{a.storage_capacity=before_used*2;},
        _=>unreachable!()
    }
    if arm=="LOW_GAS"{a.gas_available=1500;}
    let pre_b=b_gas(&w);
    let start=w.state.step;
    let first_hash=&targets[0];
    if arm=="INVALIDATED" {
        let mut saw_index=false;
        for _ in 0..24 {
            let tick=w.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(tick.code,Code::Denied|Code::InvalidCode));
            if matches!(&w.audit.last().unwrap().action,
                Some(Action::IndexVersionRefs{..})){
                assert_eq!(tick.code,Code::Ok);
                saw_index=true;break;
            }
        }
        assert!(saw_index,"pressure branch must actually certify index");
        assert!(w.state.version_ref_index.is_some());
        put(&mut w,"/self/pins/policy-race".into(),
            first_hash.as_bytes().to_vec());
        assert!(w.state.version_ref_index.is_none(),
            "legitimate competing WRITE must invalidate index before GC");
    }
    run(&mut w,64);
    let (sensor,index,gc)=policy_actions(&w,start);
    assert_eq!(sensor.len(),1,"one actual metered resource observation");
    let (used,cap,available)=policy_status(sensor[0]);
    assert_eq!(used,before_used);
    assert_eq!(cap,w.state.nodes[&NodeId::A].storage_capacity);
    let pressure=cap-used<cap/4;
    let budget=available>=2200;
    let expected=(arm=="PRESSURE"||arm=="INVALIDATED");
    assert_eq!(pressure&&budget,expected,
        "real observed resource fields drive this fixed VM branch");
    assert_eq!(b_gas(&w),pre_b,
        "maintenance must not redo P2 B computation");
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    let issued_gc=gc.iter().map(|r|(r.result.code,r.result.gas_charged))
        .collect::<Vec<_>>();
    match arm{
        "PRESSURE"=>{
            assert_eq!(index.len(),1);
            assert_eq!(index[0].result.code,Code::Ok);
            assert_eq!(gc.len(),3);
            assert!(gc.iter().all(|r|r.result.code==Code::Ok));
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(1)));
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                before_used-bytes);
            assert!(targets.iter().all(|h|!w.state.objects.contains_key(
                &version_key(h))));
        },
        "RELAXED"|"LOW_GAS"=>{
            assert!(index.is_empty()&&gc.is_empty(),
                "unnecessary maintenance should never pay the scan");
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(0)));
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,before_used);
            assert!(targets.iter().all(|h|w.state.objects.contains_key(
                &version_key(h))));
        },
        "INVALIDATED"=>{
            assert_eq!(index.len(),1);
            assert_eq!(index[0].result.code,Code::Ok);
            assert_eq!(gc.len(),1);
            assert_eq!(gc[0].result.code,Code::Conflict);
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(0)));
            assert!(targets.iter().all(|h|w.state.objects.contains_key(
                &version_key(h))));
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                before_used+64,"only researcher-supplied pin adds storage");
        },
        _=>unreachable!()
    }
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (vm,action)=spend(&w);
    println!("S29_POLICY arm={arm} root={root} \
        observed_used={used} observed_capacity={cap} \
        observed_A={available} pressure={pressure} budget={budget} \
        initial_storage={before_used} last_storage={} \
        reclaimable_bytes={bytes} index_count={} gc={issued_gc:?} \
        index_A={} B={} VM_A={vm} world_action_A={action} \
        return={:?} step={}",
        w.state.organisms[&ORGANISM_ID].storage_used,
        index.len(),index.iter().map(|x|x.result.gas_charged).sum::<u64>(),
        b_gas(&w),w.state.organisms[&ORGANISM_ID].last_return,w.state.step);
    // After the audit references have been consumed, this is an ordinary
    // true paid EXEC against the currently installed repaired Len program.
    let new_exec=w.tick(Some(Action::Exec{
        actor:ORGANISM_ID,program_hash:new_hash.clone(),
        input:vec![18,2,7,4]
    }));
    assert_eq!(new_exec.code,Code::Ok);
    assert_eq!(serde_json::from_slice::<Value>(&new_exec.payload).unwrap(),
        Value::U32(4));
}
#[test]
#[ignore="charged VM own-resource pressure maintenance policy with real race control"]
fn s29_resource_policy_native(){
    let arm=std::env::var("OCEE_S29_POLICY_ARM")
        .expect("scoped experimental arm");
    assert!(["PRESSURE","RELAXED","LOW_GAS","INVALIDATED"]
        .contains(&arm.as_str()));
    run_pressure_arm(&arm);
}


const S29_RISK_KEY:&str="/self/maintenance/risk-s29";

/// Researcher-designed one-episode backoff using actual VM read/write
/// feedback. No privileged oracle or new kernel action.
fn feedback_backoff_root(targets:&[String])->Program {
    let mut p=pressure_gc_policy(targets);
    let f=&mut p.functions[0];
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    let resource_code=std::mem::replace(&mut entry.instructions,
        vec![Instruction{id:50,op:lit(S29_RISK_KEY.as_bytes())}]);
    let resource_yield=std::mem::replace(&mut entry.terminator,
        Terminator::Yield{
            call:WorldCall::Read,args:vec![50],
            next:"risk_reply".into(),next_args:vec![]
        });
    let first_reply=f.blocks.iter_mut().find(|b|b.id=="reply0").unwrap();
    first_reply.terminator=Terminator::Branch{
        condition:6,if_true:"delete1".into(),true_args:vec![],
        if_false:"risk_failure_class".into(),false_args:vec![2]
    };
    let pars=|id|vec![Param{id,ty:Type::Bytes}];
    let check_code=|reg|vec![
        Instruction{id:3,op:lit(b"code")},
        Instruction{id:4,op:Op::JsonFieldString(reg,3)},
        Instruction{id:5,op:lit(b"Ok")},
        Instruction{id:6,op:Op::EqBytes(4,5)}
    ];
    f.blocks.extend([
        Block{
            id:"policy_read_resources".into(),params:vec![],
            instructions:resource_code,terminator:resource_yield
        },
        Block{
            id:"risk_reply".into(),params:pars(2),
            instructions:check_code(2),
            terminator:Terminator::Branch{
                condition:6,if_true:"risk_parse".into(),true_args:vec![2],
                if_false:"risk_missing".into(),false_args:vec![2]
            }
        },
        Block{
            id:"risk_missing".into(),params:pars(2),
            instructions:vec![
                Instruction{id:3,op:lit(b"code")},
                Instruction{id:4,op:Op::JsonFieldString(2,3)},
                Instruction{id:5,op:lit(b"NotFound")},
                Instruction{id:6,op:Op::EqBytes(4,5)}
            ],
            terminator:Terminator::Branch{
                condition:6,if_true:"policy_read_resources".into(),
                true_args:vec![],if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"risk_parse".into(),params:pars(2),
            instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"1")},
                Instruction{id:13,op:Op::EqBytes(11,12)}
            ],
            terminator:Terminator::Branch{
                condition:13,if_true:"risk_cooldown".into(),
                true_args:vec![],if_false:"risk_zero".into(),
                false_args:vec![11]
            }
        },
        Block{
            id:"risk_zero".into(),params:pars(2),
            instructions:vec![
                Instruction{id:10,op:lit(b"0")},
                Instruction{id:11,op:Op::EqBytes(2,10)}
            ],
            terminator:Terminator::Branch{
                condition:11,if_true:"policy_read_resources".into(),
                true_args:vec![],if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"risk_cooldown".into(),params:vec![],
            instructions:vec![
                Instruction{id:10,op:lit(S29_RISK_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"0")},
                Instruction{id:12,op:Op::ConstU32(0)}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,12],
                next:"risk_written_cooldown".into(),next_args:vec![]
            }
        },
        Block{
            id:"risk_written_cooldown".into(),params:pars(2),
            instructions:vec![],
            terminator:Terminator::Jump{
                target:"failed".into(),args:vec![]
            }
        },
        Block{
            id:"risk_failure_class".into(),params:pars(2),
            instructions:vec![
                Instruction{id:3,op:lit(b"code")},
                Instruction{id:4,op:Op::JsonFieldString(2,3)},
                Instruction{id:5,op:lit(b"Conflict")},
                Instruction{id:6,op:Op::EqBytes(4,5)}
            ],
            terminator:Terminator::Branch{
                condition:6,if_true:"risk_set".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"risk_set".into(),params:vec![],
            instructions:vec![
                Instruction{id:10,op:lit(S29_RISK_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"1")},
                Instruction{id:12,op:Op::ConstU32(u32::MAX)}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,12],
                next:"risk_written_failure".into(),next_args:vec![]
            }
        },
        Block{
            id:"risk_written_failure".into(),params:pars(2),
            instructions:vec![],
            terminator:Terminator::Jump{
                target:"failed".into(),args:vec![]
            }
        }
    ]);
    p.validate().expect("bounded typed feedback-memory policy program");
    p
}

fn setup_backoff_world(include_fixed_baseline:bool)
    ->(World,Vec<String>,String,Option<String>,String,u64)
{
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut targets=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        targets.push(hash);
    }
    let policy=feedback_backoff_root(&targets);
    let root=policy.content_hash().unwrap();
    put(&mut w,version_key(&root),policy.canonical_bytes());
    let baseline=if include_fixed_baseline{
        let p=pressure_gc_policy(&targets);
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        Some(hash)
    }else{None};
    let r=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(r.code,Code::Ok);
    let payload:serde_json::Value=
        serde_json::from_slice(&r.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        payload["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap()
        .storage_capacity=used+2048;
    assert_eq!(b_gas(&w),20);
    (w,targets,root,baseline,new_hash,used)
}
fn reactivate_program(w:&mut World,hash:&str){
    let meta=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(meta.code,Code::Ok);
    let payload:serde_json::Value=
        serde_json::from_slice(&meta.payload).unwrap();
    alter(w,"/self/active",hash.as_bytes().to_vec(),
        payload["version"].as_u64().unwrap());
}
fn gc_actions_since(w:&World,start:u64)->(usize,Vec<Code>,Vec<(Vec<u8>,Code,u64)>){
    let mut count=0;
    let mut results=Vec::<Code>::new();
    let mut risk_writes=Vec::<(Vec<u8>,Code,u64)>::new();
    for e in &w.audit{
        if e.step<start{continue;}
        match &e.action{
            Some(Action::IndexVersionRefs{..})=>count+=1,
            Some(Action::ReclaimIndexedVersion{..})=>
                results.push(e.result.code),
            Some(Action::Write{key,bytes,..}) if key==S29_RISK_KEY=>
                risk_writes.push((bytes.clone(),e.result.code,
                    e.result.gas_charged)),
            _=>{}
        }
    }
    (count,results,risk_writes)
}
fn verify_memory(w:&World,bytes:&[u8],version:u64){
    let obj=&w.state.objects[S29_RISK_KEY];
    assert_eq!(obj.owner,ORGANISM_ID);
    assert_eq!(obj.version,version);
    assert_eq!(obj.bytes,bytes);
}
fn run_first_real_conflict(w:&mut World,targets:&[String]){
    let start=w.state.step;
    let mut did_index=false;
    for _ in 0..32{
        let tick=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(tick.code,Code::Denied|Code::InvalidCode));
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::IndexVersionRefs{..})){
            assert_eq!(tick.code,Code::Ok);
            did_index=true;break;
        }
    }
    assert!(did_index,"active VM must genuinely build paid index");
    put(w,"/self/pins/s29-backoff".into(),
        targets[0].as_bytes().to_vec());
    assert!(w.state.version_ref_index.is_none());
    run(w,64);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    let (scans,gc,mem)=gc_actions_since(w,start);
    assert_eq!(scans,1);
    assert_eq!(gc,vec![Code::Conflict]);
    assert_eq!(mem.len(),1);
    assert_eq!(mem[0].0,b"1");
    assert_eq!(mem[0].1,Code::Ok);
    assert!(mem[0].2>0);
    verify_memory(w,b"1",0);
    assert!(targets.iter().all(|h|w.state.objects.contains_key(
        &version_key(h))));
}


fn test_backoff_recovery(){
    let (mut w,targets,root,_,new_hash,used)=setup_backoff_world(false);
    let total_target=targets.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64
    ).sum::<u64>();
    assert_eq!(total_target,834);
    run_first_real_conflict(&mut w,&targets);
    // The actually paid ordinary /self/ memory survives a world snapshot.
    w=World::from_snapshot(&w.snapshot_json()).unwrap();
    verify_memory(&w,b"1",0);
    let step2=w.state.step;
    reactivate_program(&mut w,&root);
    run(&mut w,64);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    verify_memory(&w,b"0",1);
    let (scan2,gc2,mem2)=gc_actions_since(&w,step2);
    assert_eq!(scan2,0);
    assert!(gc2.is_empty());
    assert_eq!(mem2.len(),1);
    assert_eq!(mem2[0].0,b"0");
    assert_eq!(mem2[0].1,Code::Ok);
    assert!(mem2[0].2>0);
    // The environment later releases its real pin with paid CAS;
    // the organism has no hidden ability to change that external fact.
    let original=w.state.organisms[&ORGANISM_ID].storage_used;
    alter(&mut w,"/self/pins/s29-backoff",vec![b'-';64],0);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,original);
    let start3=w.state.step;
    reactivate_program(&mut w,&root);
    run(&mut w,64);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    verify_memory(&w,b"0",1);
    let (scan3,gc3,mem3)=gc_actions_since(&w,start3);
    assert_eq!(scan3,1);
    assert_eq!(gc3,vec![Code::Ok,Code::Ok,Code::Ok]);
    assert!(mem3.is_empty());
    assert!(targets.iter().all(|h|!w.state.objects.contains_key(
        &version_key(h))));
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1-total_target);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (vm,action)=spend(&w);
    println!("S29_BACKOFF RECOVER root={root} \
        targets={targets:?} episodes=[Conflict,Skipped,Recovered] \
        index_count=[1,0,1] memory=[(1,v0),(0,v1),(0,v1)] \
        freed={total_target} initial_storage={used} \
        final_storage={} B={} paid_VM_A={vm} paid_Action_A={action} \
        step={}",w.state.organisms[&ORGANISM_ID].storage_used,
        b_gas(&w),w.state.step);
}
fn test_paired_reattempt(){
    let (mut w,targets,root,baseline,new_hash,_used)=
        setup_backoff_world(true);
    let baseline=baseline.expect("preinstalled fixed control root");
    run_first_real_conflict(&mut w,&targets);
    let mut control=World::from_snapshot(&w.snapshot_json()).unwrap();
    assert_eq!(w.state_hash(),control.state_hash());
    let start=w.state.step;
    reactivate_program(&mut w,&root);
    run(&mut w,64);
    let (scan_skip,gc_skip,mem)=gc_actions_since(&w,start);
    assert_eq!(scan_skip,0);
    assert!(gc_skip.is_empty());
    assert_eq!(mem.len(),1);
    assert_eq!(mem[0].0,b"0");
    verify_memory(&w,b"0",1);
    let start_control=control.state.step;
    reactivate_program(&mut control,&baseline);
    run(&mut control,64);
    let (scan_fixed,gc_fixed,_) =
        gc_actions_since(&control,start_control);
    assert_eq!(scan_fixed,1,
        "fixed threshold must pay for an unnecessary rescan");
    assert_eq!(gc_fixed,vec![Code::Conflict]);
    let skip_fee=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::IndexVersionRefs{..})))
        .map(|e|e.result.gas_charged).sum::<u64>();
    let fixed_fee=control.audit.iter().filter(|e|
        e.step>=start_control && matches!(&e.action,
            Some(Action::IndexVersionRefs{..})))
        .map(|e|e.result.gas_charged).sum::<u64>();
    assert_eq!(skip_fee,0);
    assert!(fixed_fee>=1600);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        control.state.organisms[&ORGANISM_ID].storage_used);
    assert!(targets.iter().all(|h|
        w.state.objects.contains_key(&version_key(h)) &&
        control.state.objects.contains_key(&version_key(h))));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(b_gas(&control),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    assert_eq!(control.state_hash(),World::from_snapshot(
        &control.snapshot_json()).unwrap().state_hash());
    println!("S29_BACKOFF PAIRED same_snapshot_before=true \
        remembered_skip_scan_A={skip_fee} fixed_scan_A={fixed_fee} \
        fixed_gc={gc_fixed:?} both_freed=0 VM_roots_not_equivalent=true");
}
fn test_corrupt_memory(){
    let (mut w,targets,root,_,_new,used)=setup_backoff_world(false);
    put(&mut w,S29_RISK_KEY.into(),vec![2u8]);
    let start=w.state.step;
    run(&mut w,64);
    let (scans,gc,memory)=gc_actions_since(&w,start);
    assert_eq!(scans,0);
    assert!(gc.is_empty()&&memory.is_empty());
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+1);
    assert!(targets.iter().all(|h|w.state.objects.contains_key(
        &version_key(h))));
    verify_memory(&w,&[2],0);
    assert_eq!(b_gas(&w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_BACKOFF CORRUPT same_root={root} \
        opaque_self_memory_fail_closed=true index=0 gc=0");
}
#[test]
#[ignore="native paid backoff memory across independently activated VM episodes"]
fn s29_feedback_backoff_native(){
    let arm=std::env::var("OCEE_S29_BACKOFF_ARM")
        .expect("scoped research arm");
    match arm.as_str(){
        "RACE_BACKOFF_RECOVER"=>test_backoff_recovery(),
        "PAIRED_REATTEMPT"=>test_paired_reattempt(),
        "CORRUPT_MEMORY"=>test_corrupt_memory(),
        other=>panic!("unknown research arm {other}")
    }
}


/// One researcher-provided fixed graph, one INITIAL paid activation only.
/// The following three episodes are entries into an already-active VM
/// continuation, bounded by the existing native RepeatBounded verifier.
fn self_retry_maintenance_root(targets:&[String])->Program{
    let mut p=feedback_backoff_root(targets);
    let f=&mut p.functions[0];
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry")
        .expect("program entry");
    let original_code=std::mem::take(&mut entry.instructions);
    let original_call=std::mem::replace(&mut entry.terminator,
        Terminator::Jump{
            target:"retry_header".into(),args:vec![]
        });
    let failed=f.blocks.iter_mut().find(|b|b.id=="failed")
        .expect("bounded feedback policy return0");
    failed.instructions.clear();
    failed.terminator=Terminator::Yield{
        call:WorldCall::Observe,args:vec![],
        next:"retry_observed".into(),next_args:vec![]
    };
    f.blocks.extend([
        Block{
            id:"retry_header".into(),params:vec![],
            instructions:vec![],
            terminator:Terminator::RepeatBounded{
                limit:3,body:"retry_body".into(),body_args:vec![],
                done:"retry_exhausted".into(),done_args:vec![]
            }
        },
        Block{
            id:"retry_body".into(),params:vec![],
            instructions:original_code,
            terminator:original_call
        },
        Block{
            id:"retry_observed".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![],
            terminator:Terminator::Jump{
                target:"retry_header".into(),args:vec![]
            }
        },
        Block{
            id:"retry_exhausted".into(),params:vec![],
            instructions:vec![Instruction{id:1,op:Op::ConstU32(0)}],
            terminator:Terminator::Return{value:1}
        }
    ]);
    p.validate().expect("finite cyclic graph verified by RepeatBounded");
    p
}
fn setup_self_retry_world()->(World,Vec<String>,String,String,u64){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut targets=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        targets.push(hash);
    }
    let p=self_retry_maintenance_root(&targets);
    let root=p.content_hash().unwrap();
    put(&mut w,version_key(&root),p.canonical_bytes());
    let meta=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(meta.code,Code::Ok);
    let payload:serde_json::Value=
        serde_json::from_slice(&meta.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        payload["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap()
        .storage_capacity=used+2048;
    assert_eq!(b_gas(&w),20);
    assert_eq!(targets.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64
    ).sum::<u64>(),834);
    (w,targets,root,new_hash,used)
}
fn advance_to_true_index(w:&mut World){
    let mut found=false;
    for _ in 0..36{
        let x=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(x.code,Code::Denied|Code::InvalidCode));
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::IndexVersionRefs{..})){
            assert_eq!(x.code,Code::Ok);
            found=true;break;
        }
    }
    assert!(found,"active VM must construct a genuinely metered index");
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("retry_header"),Some(&1));
}
fn true_race_and_risk_one(w:&mut World,targets:&[String]){
    advance_to_true_index(w);
    put(w,"/self/pins/s29-self-retry".into(),
        targets[0].as_bytes().to_vec());
    assert!(w.state.version_ref_index.is_none());
    let mut wrote=false;
    for _ in 0..32{
        let x=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(x.code,Code::Denied|Code::InvalidCode));
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::Write{key,bytes,..})
                if key==S29_RISK_KEY&&bytes==b"1"){
            assert_eq!(x.code,Code::Ok);
            wrote=true;break;
        }
    }
    assert!(wrote,"VM must persist actual Conflict feedback as risk1");
    verify_memory(w,b"1",0);
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("retry_header"),Some(&1));
}
fn advance_to_true_cooldown(w:&mut World){
    let mut wrote=false;
    for _ in 0..32{
        let x=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(x.code,Code::Denied|Code::InvalidCode));
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::Write{key,bytes,..})
                if key==S29_RISK_KEY&&bytes==b"0"){
            assert_eq!(x.code,Code::Ok);
            wrote=true;break;
        }
    }
    assert!(wrote,"next cycle must skip GC and write risk0");
    verify_memory(w,b"0",1);
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("retry_header"),Some(&2));
}
fn count_internal_active_switches(w:&World,start:u64)->usize{
    w.audit.iter().filter(|e|e.step>=start&&
        matches!(&e.action,Some(Action::Write{key,..})
            if key=="/self/active")).count()
}
fn count_retries_observed(w:&World,start:u64)->usize{
    w.audit.iter().filter(|e|e.step>=start&&
        matches!(&e.action,Some(Action::Observe{..}))).count()
}
fn assert_self_retry_safety(w:&World,start:u64,targets:&[String],
    expected_gcs:&[Code],expected_scans:usize){
    let (scans,retired,history)=gc_actions_since(w,start);
    assert_eq!(scans,expected_scans);
    assert_eq!(retired,expected_gcs);
    assert!(history.iter().all(|(_,code,gas)|*gas>0 ||
        *code==Code::Conflict));
    assert_eq!(count_internal_active_switches(w,start),0,
        "researcher may only have activated the root ONCE");
    assert!(count_retries_observed(w,start)>=2,
        "retry requires real charged Observe ticks and persisted loop state");
    assert_eq!(b_gas(w),20,"P2 B must not be recharged");
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("retry_header"),Some(&3));
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    for h in targets{
        if expected_gcs.ends_with(&[Code::Ok,Code::Ok,Code::Ok]){
            assert!(!w.state.objects.contains_key(&version_key(h)));
        }else{
            assert!(w.state.objects.contains_key(&version_key(h)));
        }
    }
}
fn test_single_activation_release(){
    let (mut w,targets,root,new_hash,used)=setup_self_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    advance_to_true_cooldown(&mut w);
    // The environment, NOT the organism, releases the external pin.
    alter(&mut w,"/self/pins/s29-self-retry",vec![b'-';64],0);
    run(&mut w,64);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    verify_memory(&w,b"0",1);
    assert_self_retry_safety(&w,start,&targets,
        &[Code::Conflict,Code::Ok,Code::Ok,Code::Ok],2);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1-834);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    let (vm,action)=spend(&w);
    println!("S29_SELF_RETRY RELEASE root={root} \
        one_activation=true attempts=3 paid_observe={} \
        index_count=2 gc=[Conflict,Ok,Ok,Ok] risk=[(1,v0),(0,v1)] \
        reclaimed=834 initial_storage={used} final_storage={} \
        b={} vm_A={vm} action_A={action} step={}",
        count_retries_observed(&w,start),
        w.state.organisms[&ORGANISM_ID].storage_used,b_gas(&w),w.state.step);
}
fn test_single_activation_pin_stays(){
    let (mut w,targets,root,_,used)=setup_self_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    advance_to_true_cooldown(&mut w);
    // Original pin is never removed, so a new valid index MUST
    // conservatively report a live reference and refuse retirement.
    run(&mut w,64);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_self_retry_safety(&w,start,&targets,
        &[Code::Conflict,Code::Conflict],2);
    verify_memory(&w,b"0",1);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1);
    let writes=gc_actions_since(&w,start).2;
    assert_eq!(writes.len(),3,
        "third conflict attempts predefined CAS None and cannot overwrite v1");
    assert_eq!(writes[2].0,b"1");
    assert_eq!(writes[2].1,Code::Conflict);
    assert_eq!(writes[2].2,0);
    println!("S29_SELF_RETRY PIN_STAYS root={root} \
        one_activation=true bounded_attempts=3 gc=[Conflict,Conflict] \
        third_memory_CAS=Conflict no_delete=true \
        paid_observe={} B={} step={}",
        count_retries_observed(&w,start),b_gas(&w),w.state.step);
}
fn test_single_activation_snapshot_resume(){
    let (mut w,targets,root,_,used)=setup_self_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    let mut copy=World::from_snapshot(&w.snapshot_json()).unwrap();
    assert_eq!(copy.snapshot_json(),w.snapshot_json());
    assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
        Some(root.clone()));
    // Both copies persist SAME continuation in a non-finished state.
    assert!(!w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap().finished);
    advance_to_true_cooldown(&mut w);
    advance_to_true_cooldown(&mut copy);
    assert_eq!(w.snapshot_json(),copy.snapshot_json());
    alter(&mut w,"/self/pins/s29-self-retry",vec![b'-';64],0);
    alter(&mut copy,"/self/pins/s29-self-retry",vec![b'-';64],0);
    assert_eq!(w.snapshot_json(),copy.snapshot_json());
    run(&mut w,64);
    run(&mut copy,64);
    assert_eq!(w.snapshot_json(),copy.snapshot_json(),
        "deterministic replay preserves pending feedback and bounded counters");
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_self_retry_safety(&w,start,&targets,
        &[Code::Conflict,Code::Ok,Code::Ok,Code::Ok],2);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1-834);
    println!("S29_SELF_RETRY SNAPSHOT root={root} \
        snapshot_continuation_same=true final_world_and_audit_identical=true \
        loops=3 GC=[Conflict,Ok,Ok,Ok] reclaimed=834 B={} step={}",
        b_gas(&w),w.state.step);
}
#[test]
#[ignore="real native bounded autonomous tick-continuation self retry without reactivation"]
fn s29_self_retry_native(){
    let arm=std::env::var("OCEE_S29_SELF_RETRY_ARM")
        .expect("dedicated CI matrix arm");
    match arm.as_str(){
        "ONE_ACTIVATION_RELEASE"=>test_single_activation_release(),
        "PIN_STAYS"=>test_single_activation_pin_stays(),
        "SNAPSHOT_CONTINUATION"=>test_single_activation_snapshot_resume(),
        other=>panic!("unknown test arm {other}")
    }
}


const S29_OWN_META:&str="/sys/self-object/maintenance/risk-s29";

/// A fixed, verifier-bounded single-activation policy. Unlike the old
/// hard-coded None/0 CAS, every existing-object write uses the actual
/// version from a charged atomic, actor-owned kernel observation.
fn versioned_block<'a>(f:&'a mut Function,name:&str)->&'a mut Block{
    f.blocks.iter_mut().find(|b|b.id==name)
        .expect("preexisting resource feedback block")
}
fn versioned_self_retry_root(targets:&[String])->Program{
    let mut root=self_retry_maintenance_root(targets);
    let f=&mut root.functions[0];
    {
        let b=versioned_block(f,"retry_body");
        b.instructions=vec![Instruction{
            id:50,op:lit(S29_OWN_META.as_bytes())
        }];
        assert!(matches!(&b.terminator,Terminator::Yield{
            call:WorldCall::Read,..}));
    }
    {
        let b=versioned_block(f,"risk_parse");
        b.instructions=vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:lit(b"1")},
            Instruction{id:14,op:lit(b"bytes")},
            Instruction{id:15,op:Op::JsonFieldBytes(11,14)},
            Instruction{id:16,op:lit(b"version")},
            Instruction{id:17,op:Op::JsonFieldU32(11,16)},
            Instruction{id:13,op:Op::EqBytes(15,12)}
        ];
        b.terminator=Terminator::Branch{
            condition:13,if_true:"risk_cooldown".into(),
            true_args:vec![17],if_false:"risk_zero".into(),
            false_args:vec![15]
        };
    }
    {
        let b=versioned_block(f,"risk_cooldown");
        b.params=vec![Param{id:30,ty:Type::U32}];
        b.instructions=vec![
            Instruction{id:10,op:lit(S29_RISK_KEY.as_bytes())},
            Instruction{id:11,op:lit(b"0")}
        ];
        b.terminator=Terminator::Yield{
            call:WorldCall::Write,args:vec![10,11,30],
            next:"risk_written_cooldown".into(),next_args:vec![]
        };
    }
    {
        let b=versioned_block(f,"risk_written_cooldown");
        b.instructions=vec![
            Instruction{id:3,op:lit(b"code")},
            Instruction{id:4,op:Op::JsonFieldString(2,3)},
            Instruction{id:5,op:lit(b"Ok")},
            Instruction{id:6,op:Op::EqBytes(4,5)}
        ];
        b.terminator=Terminator::Branch{
            condition:6,if_true:"failed".into(),true_args:vec![],
            if_false:"halt_cas".into(),false_args:vec![]
        };
    }
    {
        let b=versioned_block(f,"risk_set");
        b.instructions=vec![Instruction{
            id:40,op:lit(S29_OWN_META.as_bytes())
        }];
        b.terminator=Terminator::Yield{
            call:WorldCall::Read,args:vec![40],
            next:"risk_set_meta".into(),next_args:vec![]
        };
    }
    {
        let b=versioned_block(f,"risk_written_failure");
        b.instructions=vec![
            Instruction{id:3,op:lit(b"code")},
            Instruction{id:4,op:Op::JsonFieldString(2,3)},
            Instruction{id:5,op:lit(b"Ok")},
            Instruction{id:6,op:Op::EqBytes(4,5)}
        ];
        b.terminator=Terminator::Branch{
            condition:6,if_true:"failed".into(),true_args:vec![],
            if_false:"halt_cas".into(),false_args:vec![]
        };
    }
    let params=|id:u32|vec![Param{id,ty:Type::Bytes}];
    f.blocks.extend([
        Block{
            id:"risk_set_meta".into(),params:params(2),
            instructions:vec![
                Instruction{id:3,op:lit(b"code")},
                Instruction{id:4,op:Op::JsonFieldString(2,3)},
                Instruction{id:5,op:lit(b"Ok")},
                Instruction{id:6,op:Op::EqBytes(4,5)}
            ],
            terminator:Terminator::Branch{
                condition:6,if_true:"risk_set_existing".into(),
                true_args:vec![2],if_false:"risk_set_missing".into(),
                false_args:vec![2]
            }
        },
        Block{
            id:"risk_set_missing".into(),params:params(2),
            instructions:vec![
                Instruction{id:3,op:lit(b"code")},
                Instruction{id:4,op:Op::JsonFieldString(2,3)},
                Instruction{id:5,op:lit(b"NotFound")},
                Instruction{id:6,op:Op::EqBytes(4,5)}
            ],
            terminator:Terminator::Branch{
                condition:6,if_true:"risk_set_create".into(),
                true_args:vec![],if_false:"halt_cas".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"risk_set_existing".into(),params:params(2),
            instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"bytes")},
                Instruction{id:13,op:Op::JsonFieldBytes(11,12)},
                Instruction{id:14,op:lit(b"version")},
                Instruction{id:15,op:Op::JsonFieldU32(11,14)},
                Instruction{id:16,op:lit(b"0")},
                Instruction{id:17,op:Op::EqBytes(13,16)}
            ],
            terminator:Terminator::Branch{
                condition:17,if_true:"risk_set_write_existing".into(),
                true_args:vec![15],if_false:"risk_set_check_one".into(),
                false_args:vec![13,15]
            }
        },
        Block{
            id:"risk_set_check_one".into(),params:vec![
                Param{id:13,ty:Type::Bytes},
                Param{id:15,ty:Type::U32}
            ],
            instructions:vec![
                Instruction{id:16,op:lit(b"1")},
                Instruction{id:17,op:Op::EqBytes(13,16)}
            ],
            terminator:Terminator::Branch{
                condition:17,if_true:"risk_set_write_existing".into(),
                true_args:vec![15],if_false:"halt_cas".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"risk_set_write_existing".into(),
            params:vec![Param{id:30,ty:Type::U32}],
            instructions:vec![
                Instruction{id:10,op:lit(S29_RISK_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"1")}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,30],
                next:"risk_written_failure".into(),next_args:vec![]
            }
        },
        Block{
            id:"risk_set_create".into(),params:vec![],
            instructions:vec![
                Instruction{id:10,op:lit(S29_RISK_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"1")},
                Instruction{id:12,op:Op::ConstU32(u32::MAX)}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,12],
                next:"risk_written_failure".into(),next_args:vec![]
            }
        },
        Block{
            id:"halt_cas".into(),params:vec![],
            instructions:vec![Instruction{
                id:1,op:Op::ConstU32(0)
            }],
            terminator:Terminator::Return{value:1}
        }
    ]);
    root.validate().expect("real CAS versioned maintenance graph valid");
    root
}

fn setup_versioned_retry_world()->(World,Vec<String>,String,String,u64){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut targets=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        targets.push(hash);
    }
    let p=versioned_self_retry_root(&targets);
    let root=p.content_hash().unwrap();
    put(&mut w,version_key(&root),p.canonical_bytes());
    let meta=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(meta.code,Code::Ok);
    let payload:serde_json::Value=
        serde_json::from_slice(&meta.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        payload["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap()
        .storage_capacity=used+2048;
    assert_eq!(b_gas(&w),20);
    assert_eq!(targets.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64
    ).sum::<u64>(),834);
    (w,targets,root,new_hash,used)
}


fn versioned_meta_reads(w:&World,start:u64)->Vec<(Code,Option<(u64,Vec<u8>)>,u64)>{
    w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_OWN_META))
    .map(|e|{
        let value=if e.result.code==Code::Ok{
            let v:serde_json::Value=
                serde_json::from_slice(&e.result.payload).unwrap();
            let version=v["version"].as_u64().unwrap();
            let bytes=v["bytes"].as_array().unwrap().iter()
                .map(|n|n.as_u64().unwrap() as u8)
                .collect::<Vec<_>>();
            Some((version,bytes))
        }else{None};
        (e.result.code,value,e.result.gas_charged)
    }).collect()
}
fn test_versioned_repeat_conflict(){
    let (mut w,targets,root,new_hash,used)=setup_versioned_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    advance_to_true_cooldown(&mut w);
    run(&mut w,80);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    verify_memory(&w,b"1",2);
    let (indexes,gc,writes)=gc_actions_since(&w,start);
    assert_eq!(indexes,2);
    assert_eq!(gc,vec![Code::Conflict,Code::Conflict]);
    assert_eq!(writes.len(),3);
    assert_eq!(writes.iter().map(|x|x.0.clone()).collect::<Vec<_>>(),
        vec![b"1".to_vec(),b"0".to_vec(),b"1".to_vec()]);
    assert!(writes.iter().all(|x|x.1==Code::Ok&&x.2>0));
    let observed=versioned_meta_reads(&w,start);
    assert!(observed.iter().filter(|x|
        x.1.as_ref().is_some_and(|y|
            y.0==1&&y.1==b"0")).count()>=2);
    assert!(observed.iter().filter(|x|
        x.1.as_ref().is_some_and(|y|
            y.0==0&&y.1==b"1")).count()>=1);
    assert!(observed.iter().filter(|x|x.0==Code::Ok)
        .all(|x|x.2>=3));
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("retry_header"),Some(&3));
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1);
    assert!(targets.iter().all(|x|w.state.objects.contains_key(
        &version_key(x))));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (vm,action)=spend(&w);
    println!("S29_VERSIONED REPEAT_CONFLICT root={root} \
        indexes={indexes} gc={gc:?} memory_writes={writes:?} \
        meta_reads={observed:?} risk=1v2 no_delete=true \
        storage={} B={} VM_A={vm} Action_A={action} step={}",
        w.state.organisms[&ORGANISM_ID].storage_used,b_gas(&w),
        w.state.step);
}
fn test_versioned_stale_cas(){
    let (mut w,targets,root,new_hash,_)=setup_versioned_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    advance_to_true_cooldown(&mut w);
    let mut third_meta=false;
    for _ in 0..64{
        let tick=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(tick.code,Code::Denied|Code::InvalidCode));
        let e=w.audit.last().unwrap();
        if matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_OWN_META) && e.result.code==Code::Ok {
            let v:serde_json::Value=
                serde_json::from_slice(&e.result.payload).unwrap();
            if v["version"].as_u64()==Some(1) &&
                gc_actions_since(&w,start).1.len()==2{
                assert_eq!(v["bytes"],serde_json::json!([48]));
                third_meta=true;break;
            }
        }
    }
    assert!(third_meta,
        "must pause immediately after VM reads current v1 on third failure");
    verify_memory(&w,b"0",1);
    // This is a real, paid concurrent actor CAS between the VM's
    // atomic own-object observation and the next VM write.
    alter(&mut w,S29_RISK_KEY,b"1".to_vec(),1);
    verify_memory(&w,b"1",2);
    let attempted=w.tick_autonomous(ORGANISM_ID);
    assert_eq!(attempted.code,Code::Conflict,
        "kernel must reject VM's now-stale expected version=1");
    let last=w.audit.last().unwrap();
    assert!(matches!(&last.action,Some(Action::Write{
        key,bytes,expected_version:Some(1),..
    }) if key==S29_RISK_KEY&&bytes==b"1"));
    run(&mut w,24);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    verify_memory(&w,b"1",2);
    assert_eq!(count_internal_active_switches(&w,start),0);
    let (indexes,gc,writes)=gc_actions_since(&w,start);
    assert_eq!(indexes,2);
    assert_eq!(gc,vec![Code::Conflict,Code::Conflict]);
    assert_eq!(writes.len(),4,
        "three VM risk writes plus one competing real actor write");
    assert_eq!(writes.last().unwrap().1,Code::Conflict);
    assert_eq!(writes.last().unwrap().2,0);
    assert!(targets.iter().all(|x|w.state.objects.contains_key(
        &version_key(x))));
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_VERSIONED STALE_CAS root={root} \
        observed_v1_then_external_real_CAS_v2=true \
        rejected_VM_expected_v1=true no_false_memory_success=true \
        gc={gc:?} no_delete=true risk1v2=true B={} step={}",
        b_gas(&w),w.state.step);
}
fn test_versioned_release_replay(){
    let (mut w,targets,root,new_hash,used)=setup_versioned_retry_world();
    let start=w.state.step;
    true_race_and_risk_one(&mut w,&targets);
    let mut copy=World::from_snapshot(&w.snapshot_json()).unwrap();
    assert_eq!(w.snapshot_json(),copy.snapshot_json());
    advance_to_true_cooldown(&mut w);
    advance_to_true_cooldown(&mut copy);
    assert_eq!(w.snapshot_json(),copy.snapshot_json());
    alter(&mut w,"/self/pins/s29-self-retry",vec![b'-';64],0);
    alter(&mut copy,"/self/pins/s29-self-retry",vec![b'-';64],0);
    assert_eq!(w.snapshot_json(),copy.snapshot_json());
    run(&mut w,64);
    run(&mut copy,64);
    assert_eq!(w.snapshot_json(),copy.snapshot_json(),
        "snapshotted pending feedback+CAS version must replay exactly");
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    verify_memory(&w,b"0",1);
    let (indexes,gc,writes)=gc_actions_since(&w,start);
    assert_eq!(indexes,2);
    assert_eq!(gc,vec![Code::Conflict,Code::Ok,Code::Ok,Code::Ok]);
    assert_eq!(writes.len(),2);
    assert!(writes.iter().all(|x|x.1==Code::Ok&&x.2>0));
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64+1-834);
    assert!(targets.iter().all(|x|!w.state.objects.contains_key(
        &version_key(x))));
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(b_gas(&w),20);
    let (vm,action)=spend(&w);
    println!("S29_VERSIONED RELEASE_REPLAY root={root} \
        index_count={indexes} gc={gc:?} freed=834 \
        same_continuation_snapshot_and_audit=true no_reactivation=true \
        final_storage={} B={} VM_A={vm} Action_A={action} step={}",
        w.state.organisms[&ORGANISM_ID].storage_used,
        b_gas(&w),w.state.step);
}
fn test_versioned_metadata_guards(){
    let mut w=World::new(Config::default());
    put(&mut w,S29_RISK_KEY.into(),b"1".to_vec());
    let lookup=|w:&mut World,key:&str|w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:key.into()
    }));
    let result=lookup(&mut w,S29_OWN_META);
    assert_eq!(result.code,Code::Ok);
    assert_eq!(result.gas_charged,3);
    let obj:serde_json::Value=
        serde_json::from_slice(&result.payload).unwrap();
    assert_eq!(obj["bytes"],serde_json::json!([49]));
    assert_eq!(obj["version"],serde_json::json!(0));
    let raw=lookup(&mut w,S29_RISK_KEY);
    assert_eq!(raw.code,Code::Ok);
    assert_eq!(raw.payload,b"1");
    for bad in [
        "/sys/self-object/","/sys/self-object/../public/C/hello",
        "/sys/self-object/maintenance//risk-s29",
        "/sys/self-object/maintenance/./risk-s29"
    ]{
        assert_eq!(lookup(&mut w,bad).code,Code::BadRequest);
    }
    assert_eq!(lookup(&mut w,
        "/sys/self-object/maintenance/absent").code,Code::NotFound);
    // Trusted research fixtures only: the actor did not synthesize these
    // malicious ownership/overflow records via ordinary paid actions.
    w.state.objects.get_mut(S29_RISK_KEY).unwrap().owner=999;
    assert_eq!(lookup(&mut w,S29_OWN_META).code,Code::Denied);
    {
        let o=w.state.objects.get_mut(S29_RISK_KEY).unwrap();
        o.owner=ORGANISM_ID;
        o.version=(u32::MAX as u64)+1;
    }
    assert_eq!(lookup(&mut w,S29_OWN_META).code,Code::Quota);
    w.state.objects.get_mut(S29_RISK_KEY).unwrap().version=0;
    assert_eq!(lookup(&mut w,S29_OWN_META).code,Code::Ok);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    println!("S29_VERSIONED METADATA_GUARDS paid_3A=true \
        raw_read_unchanged=true malformed_path_denied=true \
        foreign_owner_denied=true version_overflow_quota=true \
        no_kernel_privilege_bypass=true");
}
#[test]
#[ignore="paid owned-object CAS metadata and repeated memory transitions"]
fn s29_versioned_memory_native(){
    let arm=std::env::var("OCEE_S29_VERSIONED_ARM")
        .expect("scoped frozen native experiment");
    match arm.as_str(){
        "REPEAT_CONFLICT"=>test_versioned_repeat_conflict(),
        "STALE_CAS"=>test_versioned_stale_cas(),
        "RELEASE_AND_REPLAY"=>test_versioned_release_replay(),
        "METADATA_GUARDS"=>test_versioned_metadata_guards(),
        other=>panic!("unknown versioned metadata arm {other}")
    }
}


/// Frozen S29 follow-up: an unrelated one-byte own-object write is compared
/// before and after a real paid reference-index build. No hash pin, new
/// world/VM capability, reference-index mutation, or artificial CAS state.
fn test_benign_churn_arm(arm:&str){
    let (mut w,targets,root,new_hash,used)=setup_versioned_retry_world();
    let start=w.state.step;
    let (vm_before,action_before)=spend(&w);
    let noise="/self/noise/s29-churn";
    let mut branch_replay=false;
    if arm=="BEFORE_INDEX"{
        put(&mut w,noise.into(),b"x".to_vec());
    }
    if matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"){
        // Pause only after the VM's actual charged kernel Index action.
        advance_to_true_index(&mut w);
        assert!(w.state.version_ref_index.is_some());
        put(&mut w,noise.into(),b"x".to_vec());
        assert!(w.state.version_ref_index.is_none(),
            "unrelated paid own-object write should invalidate index");
        let mut memory_persisted=false;
        for _ in 0..40{
            let outcome=w.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(outcome.code,Code::Denied|Code::InvalidCode));
            if matches!(&w.audit.last().unwrap().action,
                Some(Action::Write{key,bytes,..})
                    if key==S29_RISK_KEY&&bytes==b"1"){
                assert_eq!(outcome.code,Code::Ok);
                memory_persisted=true;break;
            }
        }
        assert!(memory_persisted,
            "real stale certificate must cause Conflict and paid risk1 CAS");
        verify_memory(&w,b"1",0);
        assert_eq!(gc_actions_since(&w,start).1,vec![Code::Conflict]);
        if arm=="EPOCH_GAP"{
            let old_epoch=w.state.step/w.state.config.epoch_length;
            let continuation_before=serde_json::to_string(
                &w.state.organisms[&ORGANISM_ID].vm_continuation).unwrap();
            // The world keeps turning without this organism acting.
            // One extra tick makes the next epoch's grant real.
            while w.state.step/w.state.config.epoch_length==old_epoch{
                w.tick(None);
            }
            w.tick(None);
            assert!(w.state.step/w.state.config.epoch_length>old_epoch);
            assert_eq!(serde_json::to_string(
                &w.state.organisms[&ORGANISM_ID].vm_continuation).unwrap(),
                continuation_before,
                "the host's no-actor ticks cannot advance the VM");
            verify_memory(&w,b"1",0);
            let mut copy=World::from_snapshot(&w.snapshot_json()).unwrap();
            run(&mut w,96);
            run(&mut copy,96);
            assert_eq!(w.snapshot_json(),copy.snapshot_json(),
                "post-epoch live continuation/audit must replay exactly");
            branch_replay=true;
        }
    }
    if !branch_replay{
        run(&mut w,96);
    }
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"same bounded VM must reclaim all three");
    let (indexes,gc,risk_writes)=gc_actions_since(&w,start);
    let expected_gc=if matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"){
        vec![Code::Conflict,Code::Ok,Code::Ok,Code::Ok]
    }else{vec![Code::Ok,Code::Ok,Code::Ok]};
    assert_eq!(gc,expected_gc);
    assert_eq!(indexes,
        if matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"){2}else{1});
    if matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"){
        verify_memory(&w,b"0",1);
        assert_eq!(risk_writes.len(),2);
        assert_eq!(
            risk_writes.iter().map(|(bytes,code,_)|(bytes.clone(),*code))
                .collect::<Vec<_>>(),
            vec![(b"1".to_vec(),Code::Ok),(b"0".to_vec(),Code::Ok)]);
        assert!(risk_writes.iter().all(|(_,_,cost)|*cost>0));
    }else{
        assert!(!w.state.objects.contains_key(S29_RISK_KEY));
        assert!(risk_writes.is_empty());
    }
    for hash in &targets{
        assert!(!w.state.objects.contains_key(&version_key(hash)));
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+u64::from(arm!="QUIET")
            +u64::from(matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"))-834);
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(b_gas(&w),20,"original P2 service may not run twice");
    let mut index_gas=Vec::new();
    let mut gc_gas=Vec::new();
    let mut environment_gas=Vec::new();
    for event in w.audit.iter().filter(|e|e.step>=start){
        match &event.action{
            Some(Action::IndexVersionRefs{..})=>
                index_gas.push(event.result.gas_charged),
            Some(Action::ReclaimIndexedVersion{..})=>
                gc_gas.push((event.result.code,event.result.gas_charged)),
            Some(Action::Write{key,..}) if key==noise=>
                environment_gas.push(event.result.gas_charged),
            _=>{}
        }
    }
    assert_eq!(index_gas.len(),indexes);
    assert!(index_gas.iter().all(|cost|*cost>0));
    assert_eq!(environment_gas.len(),if arm=="QUIET"{0}else{1});
    if arm!="QUIET"{assert!(environment_gas[0]>0);}
    assert_eq!(gc_gas.iter().map(|x|x.0).collect::<Vec<_>>(),gc);
    assert!(gc_gas.iter().filter(|x|x.0==Code::Ok).all(|x|x.1>0));
    if matches!(arm,"AFTER_INDEX"|"EPOCH_GAP"){
        let ix=w.audit.iter().find(|e|e.step>=start&&
            matches!(e.action,Some(Action::IndexVersionRefs{..})))
            .unwrap().step;
        let write=w.audit.iter().find(|e|e.step>=start&&
            matches!(&e.action,Some(Action::Write{key,..})
                if key==noise)).unwrap().step;
        let conflict=w.audit.iter().find(|e|e.step>=start&&
            matches!(e.action,Some(Action::ReclaimIndexedVersion{..})))
            .unwrap().step;
        assert!(ix<write&&write<conflict,
            "researcher mutation must follow paid index before real GC");
    }
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (vm_after,action_after)=spend(&w);
    println!("S29_CHURN {arm} root={root} indexes={indexes} \
        index_A={index_gas:?} gc={gc:?} gc_A={gc_gas:?} \
        researcher_noise_A={environment_gas:?} \
        maintenance_plus_environment_VM_A={} \
        maintenance_plus_environment_Action_A={} \
        storage_delta={} B={} final_step={} final_epoch={} replay={} \
        all_834B_reclaimed=true",
        vm_after-vm_before,action_after-action_before,
        (w.state.organisms[&ORGANISM_ID].storage_used as i64)-(used as i64),
        b_gas(&w),w.state.step,
        w.state.step/w.state.config.epoch_length,branch_replay);
}
#[test]
#[ignore="paid S29 unrelated write timing and cross-epoch continuation"]
fn s29_benign_churn_native(){
    let arm=std::env::var("OCEE_S29_CHURN_ARM")
        .expect("frozen benign index churn arm");
    match arm.as_str(){
        "QUIET"|"BEFORE_INDEX"|"AFTER_INDEX"|"EPOCH_GAP"=>
            test_benign_churn_arm(&arm),
        _=>panic!("unknown benign churn arm {arm}")
    }
}


/// Preregistered S29: keep the kernel's exact 64-byte raw-hash reference
/// guarantee while testing metered, strictly shorter own-object writes.
/// This reuses the previous *real* P2/B20 boot and the unchanged active
/// risk-aware maintenance root. Only the kernel Write behavior differs.
fn test_certified_short_write_arm(arm:&str){
    let (mut w,targets,root,new_hash,used)=setup_versioned_retry_world();
    let start=w.state.step;
    let (vm_before,action_before)=spend(&w);
    let noise="/self/noise/s29-churn";
    if arm=="OLD_REF_REMOVED"{
        put(&mut w,noise.into(),targets[0].as_bytes().to_vec());
    }
    advance_to_true_index(&mut w);
    let before_certificate=w.state.version_ref_index.clone()
        .expect("actual paid Index must install a trusted certificate");
    assert_eq!(before_certificate.active_hash,Some(root.clone()));
    let mut write_a=Vec::<u64>::new();
    match arm{
        "SHORT_WRITE"|"FAILED_CAS"=>{
            let added=w.tick(Some(Action::Write{
                actor:ORGANISM_ID,key:noise.into(),
                expected_version:None,bytes:b"x".to_vec()
            }));
            assert_eq!(added.code,Code::Ok);
            assert_eq!(added.gas_charged,4,
                "successful one-byte certified own write is base 3+1 A");
            write_a.push(added.gas_charged);
            assert_eq!(w.state.version_ref_index,Some(before_certificate.clone()),
                "proof-carrying short new state cannot introduce 64B raw hash");
            if arm=="FAILED_CAS"{
                let before=w.state.objects[noise].clone();
                let rejected=w.tick(Some(Action::Write{
                    actor:ORGANISM_ID,key:noise.into(),
                    expected_version:None,bytes:b"y".to_vec()
                }));
                assert_eq!(rejected.code,Code::Conflict);
                assert_eq!(rejected.gas_charged,0);
                assert_eq!(w.state.objects[noise],before);
                assert_eq!(w.state.version_ref_index,Some(before_certificate),
                    "rejected short CAS cannot mutate a live certificate");
                write_a.push(rejected.gas_charged);
            }
            run(&mut w,100);
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(1)));
            let (indexes,gc,risk)=gc_actions_since(&w,start);
            assert_eq!(indexes,1);
            assert_eq!(gc,vec![Code::Ok,Code::Ok,Code::Ok]);
            assert!(risk.is_empty());
            assert!(!w.state.objects.contains_key(S29_RISK_KEY));
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+1-834);
            assert!(targets.iter().all(|h|
                !w.state.objects.contains_key(&version_key(h))));
            // Same world/VM root and P2 business as the frozen timing pair.
            let (vm_after,action_after)=spend(&w);
            assert_eq!(vm_after-vm_before,191);
            assert_eq!(action_after-action_before,1697);
        },
        "LIVE_HASH_PIN"=>{
            let pinned=w.tick(Some(Action::Write{
                actor:ORGANISM_ID,key:noise.into(),
                expected_version:None,bytes:targets[0].as_bytes().to_vec()
            }));
            assert_eq!(pinned.code,Code::Ok);
            assert_eq!(pinned.gas_charged,6,
                "full 64-byte reference must use ordinary metered Write");
            write_a.push(pinned.gas_charged);
            assert!(w.state.version_ref_index.is_none(),
                "a full live hash must invalidate the certificate");
            run(&mut w,100);
            let (indexes,gc,risk)=gc_actions_since(&w,start);
            assert_eq!(indexes,2);
            assert_eq!(gc,vec![Code::Conflict,Code::Conflict]);
            assert_eq!(risk.len(),3);
            verify_memory(&w,b"1",2);
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(0)));
            assert!(targets.iter().all(|h|
                w.state.objects.contains_key(&version_key(h))));
        },
        "OLD_REF_REMOVED"=>{
            assert!(before_certificate.referenced.contains(&targets[0]),
                "the paid original index must prove the old hash was live");
            let pin=w.state.objects[noise].clone();
            assert_eq!(pin.bytes,targets[0].as_bytes());
            let replaced=w.tick(Some(Action::Write{
                actor:ORGANISM_ID,key:noise.into(),
                expected_version:Some(pin.version),bytes:b"x".to_vec()
            }));
            assert_eq!(replaced.code,Code::Ok);
            assert_eq!(replaced.gas_charged,4);
            write_a.push(replaced.gas_charged);
            assert_eq!(w.state.version_ref_index,Some(before_certificate.clone()),
                "short replacement must retain conservative old reference");
            let blocked=fast_reclaim(&mut w,&targets[0]);
            assert_eq!(blocked.code,Code::Conflict,
                "an old certified reference must not be forgotten");
            assert_eq!(blocked.gas_charged,0);
            assert!(w.state.objects.contains_key(&version_key(&targets[0])));
            // Rebuilding the WHOLE original paid index removes the
            // conservative false positive, rather than erasing it by hand.
            let new_index=index_refs(&mut w);
            assert_eq!(new_index.code,Code::Ok);
            assert!(new_index.gas_charged>0);
            let current=w.state.version_ref_index.as_ref().unwrap();
            assert!(!current.referenced.contains(&targets[0]));
            let freed=fast_reclaim(&mut w,&targets[0]);
            assert_eq!(freed.code,Code::Ok);
            assert!(freed.gas_charged>0);
            assert_eq!(u64::from_le_bytes(
                freed.payload.as_slice().try_into().unwrap()),279);
            assert!(!w.state.objects.contains_key(&version_key(&targets[0])));
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+64-63-279);
        },
        other=>panic!("unknown certified short write arm {other}"),
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(b_gas(&w),20,"P2 B may not run twice");
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let (indexes,gc,_)=gc_actions_since(&w,start);
    let (vm_after,action_after)=spend(&w);
    println!("S29_SHORT_CERT {arm} root={root} \
        index_count={indexes} gc={gc:?} certified_write_gas={write_a:?} \
        VM_A={} Action_A={} storage_delta={} B={} final_step={} \
        snapshot_consistent=true",
        vm_after-vm_before,action_after-action_before,
        (w.state.organisms[&ORGANISM_ID].storage_used as i64)-(used as i64),
        b_gas(&w),w.state.step);
}

#[test]
#[ignore="paid 64B raw-hash certificate short-write preservation"]
fn s29_certified_short_write_native(){
    let arm=std::env::var("OCEE_S29_SHORT_ARM")
        .expect("preregistered native certified short write arm");
    match arm.as_str(){
        "SHORT_WRITE"|"LIVE_HASH_PIN"|"FAILED_CAS"|"OLD_REF_REMOVED"=>
            test_certified_short_write_arm(&arm),
        _=>panic!("unrecognized certified short write arm {arm}")
    }
}


/// Native preregistered S29: exact, metered *source* membership differences.
/// Unlike the older single global 'referenced' set, a source-aware certificate
/// cannot forget another holder when one owned object stops referencing a hash.
/// Researcher-issued world actions below are NOT autonomous VM choices.
fn test_source_delta_arm(arm:&str) {
    let pin="/self/pins/s29-source-delta";
    let pin2="/self/pins/s29-source-delta-secondary";
    if arm=="TRANSPORT_LEASE" {
        let mut w=World::new(Config{message_delay:32,..Config::default()});
        let p=wrong_tool();
        let hash=p.content_hash().unwrap();
        put(&mut w,version_key(&hash),p.canonical_bytes());
        put(&mut w,pin.into(),hash.as_bytes().to_vec());
        let send=w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,
            bytes:hash.as_bytes().to_vec()
        }));
        assert_eq!(send.code,Code::Ok);
        assert!(send.gas_charged>0);
        assert!(w.state.pending_deliveries.iter().any(|m|
            m.bytes==hash.as_bytes()));
        let index=index_refs(&mut w);
        assert_eq!(index.code,Code::Ok);
        assert!(index.gas_charged>0);
        let tracked=w.state.version_ref_index.as_ref().unwrap();
        assert!(tracked.transport_refs.contains(&hash));
        assert!(tracked.referenced.contains(&hash));
        let updated=w.tick(Some(Action::WriteIndexed{
            actor:ORGANISM_ID,key:pin.into(),
            expected_version:Some(0),bytes:b"x".to_vec()
        }));
        assert_eq!(updated.code,Code::Ok);
        assert!(updated.gas_charged>3);
        let kept=w.state.version_ref_index.as_ref().unwrap();
        assert!(kept.referenced.contains(&hash),
            "pending transport reference must survive source CAS");
        assert_eq!(fast_reclaim(&mut w,&hash).code,Code::Conflict);
        assert!(w.state.objects.contains_key(&version_key(&hash)));
        assert_eq!(w.state_hash(),World::from_snapshot(
            &w.snapshot_json()).unwrap().state_hash());
        println!("S29_SOURCE_DELTA TRANSPORT_LEASE index_A={} write_A={} \
            C_send_id={:?} referenced_after_local_unpin=true \
            GC=Conflict B=0 no_delete=true",index.gas_charged,
            updated.gas_charged,send.payload);
        return;
    }
    if arm=="VM_DELTA" {
        // Unlike setup_versioned_retry_world, no previous maintenance root
        // containing the target literal is left inactive after the switch.
        let (mut w,old_hash,new_hash,_)=boot_repaired();
        put(&mut w,pin.into(),old_hash.as_bytes().to_vec());
        let root=Program{format_version:1,entry_function:"main".into(),
            functions:vec![Function{
                name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
                blocks:vec![
                    Block{id:"entry".into(),
                        params:vec![Param{id:0,ty:Type::Bytes}],
                        instructions:vec![
                            Instruction{id:10,op:lit(pin.as_bytes())},
                            Instruction{id:11,op:lit(b"x")},
                            Instruction{id:12,op:Op::ConstU32(0)}
                        ],
                        terminator:Terminator::Yield{
                            call:WorldCall::WriteIndexed,args:vec![10,11,12],
                            next:"reply".into(),next_args:vec![]
                        }
                    },
                    Block{id:"reply".into(),
                        params:vec![Param{id:2,ty:Type::Bytes}],
                        instructions:vec![
                            Instruction{id:3,op:lit(b"code")},
                            Instruction{id:4,op:Op::JsonFieldString(2,3)},
                            Instruction{id:5,op:lit(b"Ok")},
                            Instruction{id:6,op:Op::EqBytes(4,5)}
                        ],
                        terminator:Terminator::Branch{
                            condition:6,if_true:"good".into(),
                            true_args:vec![],if_false:"bad".into(),
                            false_args:vec![]
                        }
                    },
                    Block{id:"good".into(),params:vec![],
                        instructions:vec![Instruction{id:7,op:Op::ConstU32(1)}],
                        terminator:Terminator::Return{value:7}
                    },
                    Block{id:"bad".into(),params:vec![],
                        instructions:vec![Instruction{id:7,op:Op::ConstU32(0)}],
                        terminator:Terminator::Return{value:7}
                    },
                ]
            }]
        };
        root.validate().expect("typed VM WriteIndexed graph must validate");
        let root_hash=root.content_hash().unwrap();
        put(&mut w,version_key(&root_hash),root.canonical_bytes());
        let active=w.tick(Some(Action::Read{
            actor:ORGANISM_ID,key:"/sys/active-version".into()
        }));
        assert_eq!(active.code,Code::Ok);
        let value:serde_json::Value=serde_json::from_slice(&active.payload).unwrap();
        alter(&mut w,"/self/active",root_hash.as_bytes().to_vec(),
            value["version"].as_u64().unwrap());
        let start=w.state.step;
        let index=index_refs(&mut w);
        assert_eq!(index.code,Code::Ok);
        assert!(w.state.version_ref_index.as_ref().unwrap()
            .referenced.contains(&old_hash));
        run(&mut w,32);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        let vm_write=w.audit.iter().filter(|e|e.step>=start &&
            matches!(&e.action,Some(Action::WriteIndexed{key,..}) if key==pin))
            .collect::<Vec<_>>();
        assert_eq!(vm_write.len(),1);
        assert_eq!(vm_write[0].result.code,Code::Ok);
        let vm_write_fee=vm_write[0].result.gas_charged;
        let vm_slice_fee=vm_write[0].result.vm_gas_charged;
        assert!(vm_slice_fee>0);
        assert!(vm_write_fee>0);
        assert!(!w.state.version_ref_index.as_ref().unwrap()
            .referenced.contains(&old_hash));
        // End immutable audit borrows before the next mutable world action.
        drop(vm_write);
        let collected=fast_reclaim(&mut w,&old_hash);
        assert_eq!(collected.code,Code::Ok);
        assert_eq!(u64::from_le_bytes(
            collected.payload.as_slice().try_into().unwrap()),279);
        assert!(!w.state.objects.contains_key(&version_key(&old_hash)));
        assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
        assert_eq!(b_gas(&w),20);
        assert_eq!(w.state_hash(),World::from_snapshot(
            &w.snapshot_json()).unwrap().state_hash());
        println!("S29_SOURCE_DELTA VM_DELTA root={root_hash} \
            worldcall_actual=true fee={} vm_fee={} \
            paid_index={} rescued_279B=true B=20",
            vm_write_fee,vm_slice_fee,index.gas_charged);
        return;
    }
    let (mut w,targets,root,new_hash,used)=setup_versioned_retry_world();
    let target=&targets[0];
    let start=w.state.step;
    let (vm_start,action_start)=spend(&w);
    let (old_pin_count,is_legacy)=match arm{
        "LEGACY_WRITE"|"DELTA_SINGLE"=>(1,true),
        "DELTA_MULTI"=>(2,true),
        "DELTA_ADD_AND_STALE"=>(0,false),
        _=>panic!("unrecognized source delta arm {arm}")
    };
    if old_pin_count>0{
        put(&mut w,pin.into(),target.as_bytes().to_vec());
    }
    if old_pin_count>1{
        put(&mut w,pin2.into(),target.as_bytes().to_vec());
    }
    let index=index_refs(&mut w);
    assert_eq!(index.code,Code::Ok);
    assert!(index.gas_charged>0);
    let initial=w.state.version_ref_index.clone().unwrap();
    assert!(initial.object_refs.is_some());
    assert_eq!(initial.referenced.contains(target),is_legacy);
    let metadata_before=serde_json::to_vec(&initial).unwrap().len();
    let mut fees=Vec::<u64>::new();
    let direct_write=|w:&mut World,key:&str,bytes:Vec<u8>,
        version:Option<u64>,indexed:bool| {
        w.tick(Some(if indexed{
            Action::WriteIndexed{actor:ORGANISM_ID,
                key:key.into(),expected_version:version,bytes}
        }else{
            Action::Write{actor:ORGANISM_ID,
                key:key.into(),expected_version:version,bytes}
        }))
    };
    match arm{
        "LEGACY_WRITE"=>{
            let result=direct_write(&mut w,pin,b"x".to_vec(),Some(0),false);
            assert_eq!(result.code,Code::Ok);
            fees.push(result.gas_charged);
            let idx=w.state.version_ref_index.as_ref().unwrap();
            assert!(idx.referenced.contains(target),
                "legacy short Write must retain conservative old membership");
            assert_eq!(fast_reclaim(&mut w,target).code,Code::Conflict);
            let rebuilt=index_refs(&mut w);
            assert_eq!(rebuilt.code,Code::Ok);
            assert!(rebuilt.gas_charged>0);
            assert!(!w.state.version_ref_index.as_ref().unwrap()
                .referenced.contains(target));
            let deleted=fast_reclaim(&mut w,target);
            assert_eq!(deleted.code,Code::Ok);
            assert_eq!(u64::from_le_bytes(
                deleted.payload.as_slice().try_into().unwrap()),279);
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+1-279);
        },
        "DELTA_SINGLE"=>{
            let result=direct_write(&mut w,pin,b"x".to_vec(),Some(0),true);
            assert_eq!(result.code,Code::Ok);
            assert!(result.gas_charged>3);
            fees.push(result.gas_charged);
            let idx=w.state.version_ref_index.as_ref().unwrap();
            assert_eq!(idx.built_step,initial.built_step,
                "no second paid full scan");
            assert!(!idx.referenced.contains(target));
            let deleted=fast_reclaim(&mut w,target);
            assert_eq!(deleted.code,Code::Ok);
            assert_eq!(u64::from_le_bytes(
                deleted.payload.as_slice().try_into().unwrap()),279);
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+1-279);
        },
        "DELTA_MULTI"=>{
            let one=direct_write(&mut w,pin,b"x".to_vec(),Some(0),true);
            assert_eq!(one.code,Code::Ok);
            fees.push(one.gas_charged);
            assert!(w.state.version_ref_index.as_ref().unwrap()
                .referenced.contains(target),
                "second distinct object must keep the reference alive");
            assert_eq!(fast_reclaim(&mut w,target).code,Code::Conflict);
            let two=direct_write(&mut w,pin2,b"x".to_vec(),Some(0),true);
            assert_eq!(two.code,Code::Ok);
            fees.push(two.gas_charged);
            assert!(!w.state.version_ref_index.as_ref().unwrap()
                .referenced.contains(target));
            assert_eq!(fast_reclaim(&mut w,target).code,Code::Ok);
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+2-279);
        },
        "DELTA_ADD_AND_STALE"=>{
            let added=direct_write(&mut w,pin,target.as_bytes().to_vec(),
                None,true);
            assert_eq!(added.code,Code::Ok);
            fees.push(added.gas_charged);
            assert!(w.state.version_ref_index.as_ref().unwrap()
                .referenced.contains(target),
                "long indexed write must ADD new live hash to certificate");
            assert_eq!(fast_reclaim(&mut w,target).code,Code::Conflict);
            let before_obj=w.state.objects[pin].clone();
            let before_idx=w.state.version_ref_index.clone();
            let before_storage=w.state.organisms[&ORGANISM_ID].storage_used;
            let stale=direct_write(&mut w,pin,b"x".to_vec(),None,true);
            assert_eq!(stale.code,Code::Conflict);
            assert_eq!(stale.gas_charged,0);
            assert_eq!(w.state.objects[pin],before_obj);
            assert_eq!(w.state.version_ref_index,before_idx);
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                before_storage);
            fees.push(stale.gas_charged);
            let removed=direct_write(&mut w,pin,b"x".to_vec(),Some(0),true);
            assert_eq!(removed.code,Code::Ok);
            fees.push(removed.gas_charged);
            assert!(!w.state.version_ref_index.as_ref().unwrap()
                .referenced.contains(target));
            assert_eq!(fast_reclaim(&mut w,target).code,Code::Ok);
            assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
                used+1-279);
        },
        _=>unreachable!()
    }
    assert!(!w.state.objects.contains_key(&version_key(target)));
    for h in targets.iter().skip(1){
        assert!(w.state.objects.contains_key(&version_key(h)));
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let metadata_after=w.state.version_ref_index.as_ref()
        .map_or(0,|idx|serde_json::to_vec(idx).unwrap().len());
    let (vm_end,action_end)=spend(&w);
    let indices=w.audit.iter().filter(|e|e.step>=start &&
        matches!(e.action,Some(Action::IndexVersionRefs{..})))
        .count();
    assert_eq!(indices,if arm=="LEGACY_WRITE"{2}else{1});
    println!("S29_SOURCE_DELTA {arm} root={root} indexes={indices} \
        initial_index_A={} write_A={fees:?} \
        VM_A={} Action_A={} certificate_bytes_before={} \
        certificate_bytes_after={} storage_delta={} B={} \
        final_step={} snapshot_consistent=true",
        index.gas_charged,vm_end-vm_start,action_end-action_start,
        metadata_before,metadata_after,
        (w.state.organisms[&ORGANISM_ID].storage_used as i64)-(used as i64),
        b_gas(&w),w.state.step);
}
#[test]
#[ignore="paid source-aware indexed CAS and multiple real-reference safety"]
fn s29_source_delta_native(){
    let arm=std::env::var("OCEE_S29_DELTA_ARM")
        .expect("frozen per-source indexed reference delta arm");
    match arm.as_str(){
        "LEGACY_WRITE"|"DELTA_SINGLE"|"DELTA_MULTI"|
        "DELTA_ADD_AND_STALE"|"VM_DELTA"|"TRANSPORT_LEASE"=>
            test_source_delta_arm(&arm),
        _=>panic!("unknown source-delta arm {arm}")
    }
}


// DEV S29: This researcher-supplied *policy graph* embeds NO candidate
// program hashes. It discovers candidates only from real, paid, actor-owned
// kernel Read metadata, and treats paid GC feedback as the safety authority.
// One page, at most eight tries. Not endogenous goal creation or general GC.
fn catalog_discovery_root()->Program {
    let params=||vec![
        Param{id:1,ty:Type::Bytes},
        Param{id:2,ty:Type::U32},
        Param{id:3,ty:Type::U32}
    ];
    let feedback_params=||vec![
        Param{id:1,ty:Type::Bytes},
        Param{id:2,ty:Type::U32},
        Param{id:3,ty:Type::U32},
        Param{id:4,ty:Type::Bytes}
    ];
    let code_match=|feedback:u32,expected:&[u8]|vec![
        Instruction{id:20,op:lit(b"code")},
        Instruction{id:21,op:Op::JsonFieldString(feedback,20)},
        Instruction{id:22,op:lit(expected)},
        Instruction{id:23,op:Op::EqBytes(21,22)}
    ];
    let f=Function {
        name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
        blocks:vec![
            Block{
                id:"entry".into(),
                params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![Instruction{
                    id:1,op:lit(b"/sys/self-version-catalog/0")
                }],
                terminator:Terminator::Yield{
                    call:WorldCall::Read,args:vec![1],
                    next:"catalog_reply".into(),next_args:vec![]
                }
            },
            Block{
                id:"catalog_reply".into(),
                params:vec![Param{id:2,ty:Type::Bytes}],
                instructions:code_match(2,b"Ok"),
                terminator:Terminator::Branch{
                    condition:23,if_true:"parse_catalog".into(),
                    true_args:vec![2],if_false:"failed".into(),
                    false_args:vec![]
                }
            },
            Block{
                id:"parse_catalog".into(),
                params:vec![Param{id:2,ty:Type::Bytes}],
                instructions:vec![
                    Instruction{id:10,op:lit(b"payload")},
                    Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                    Instruction{id:12,op:lit(b"entries")},
                    Instruction{id:13,op:Op::JsonFieldJson(11,12)},
                    Instruction{id:14,op:lit(b"count")},
                    Instruction{id:15,op:Op::JsonFieldU32(11,14)},
                    Instruction{id:16,op:Op::ConstU32(0)}
                ],
                terminator:Terminator::Jump{
                    target:"build_index".into(),
                    args:vec![13,15,16]
                }
            },
            Block{
                id:"build_index".into(),params:params(),instructions:vec![],
                terminator:Terminator::Yield{
                    call:WorldCall::IndexVersionRefs,args:vec![],
                    next:"index_reply".into(),next_args:vec![1,2,3]
                }
            },
            Block{
                id:"index_reply".into(),params:feedback_params(),
                instructions:code_match(4,b"Ok"),
                terminator:Terminator::Branch{
                    condition:23,if_true:"search_header".into(),
                    true_args:vec![1,2,3],if_false:"failed".into(),
                    false_args:vec![]
                }
            },
            Block{
                id:"search_header".into(),params:params(),instructions:vec![],
                terminator:Terminator::RepeatBounded{
                    limit:8,body:"check_count".into(),body_args:vec![1,2,3],
                    done:"failed".into(),done_args:vec![]
                }
            },
            Block{
                id:"check_count".into(),params:params(),
                instructions:vec![Instruction{
                    id:4,op:Op::EqU32(3,2)
                }],
                terminator:Terminator::Branch{
                    condition:4,if_true:"failed".into(),true_args:vec![],
                    if_false:"choose_hash".into(),false_args:vec![1,2,3]
                }
            },
            Block{
                id:"choose_hash".into(),params:params(),
                instructions:vec![
                    Instruction{id:4,op:Op::JsonIndex(1,3)},
                    Instruction{id:5,op:lit(b"hash")},
                    Instruction{id:6,op:Op::JsonFieldString(4,5)}
                ],
                terminator:Terminator::Yield{
                    call:WorldCall::ReclaimIndexedVersion,args:vec![6],
                    next:"gc_reply".into(),next_args:vec![1,2,3]
                }
            },
            Block{
                id:"gc_reply".into(),params:feedback_params(),
                instructions:code_match(4,b"Ok"),
                terminator:Terminator::Branch{
                    condition:23,if_true:"succeeded".into(),
                    true_args:vec![],if_false:"gc_conflict".into(),
                    false_args:vec![1,2,3,4]
                }
            },
            Block{
                id:"gc_conflict".into(),params:feedback_params(),
                instructions:code_match(4,b"Conflict"),
                terminator:Terminator::Branch{
                    condition:23,if_true:"advance".into(),
                    true_args:vec![1,2,3],if_false:"gc_missing".into(),
                    false_args:vec![1,2,3,4]
                }
            },
            Block{
                id:"gc_missing".into(),params:feedback_params(),
                instructions:code_match(4,b"NotFound"),
                terminator:Terminator::Branch{
                    condition:23,if_true:"advance".into(),
                    true_args:vec![1,2,3],if_false:"failed".into(),
                    false_args:vec![]
                }
            },
            Block{
                id:"advance".into(),params:params(),
                instructions:vec![
                    Instruction{id:4,op:Op::ConstU32(1)},
                    Instruction{id:5,op:Op::Add(3,4)}
                ],
                terminator:Terminator::Jump{
                    target:"search_header".into(),args:vec![1,2,5]
                }
            },
            Block{
                id:"succeeded".into(),params:vec![],
                instructions:vec![Instruction{
                    id:1,op:Op::ConstU32(1)
                }],
                terminator:Terminator::Return{value:1}
            },
            Block{
                id:"failed".into(),params:vec![],
                instructions:vec![Instruction{
                    id:1,op:Op::ConstU32(0)
                }],
                terminator:Terminator::Return{value:1}
            }
        ]
    };
    let p=Program{format_version:1,entry_function:"main".into(),
        functions:vec![f]};
    p.validate().expect("finite policy must pass static typed verifier");
    p
}
const S29_CATALOG_PAGE0:&str="/sys/self-version-catalog/0";
fn catalog_research_setup()->(World,Vec<String>,String,String,u64){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut candidates=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let h=p.content_hash().unwrap();
        assert!(!w.state.objects.contains_key(&version_key(&h)));
        put(&mut w,version_key(&h),p.canonical_bytes());
        candidates.push(h);
    }
    let policy=catalog_discovery_root();
    let policy_bytes=policy.canonical_bytes();
    assert!(candidates.iter().all(|hash|
        !policy_bytes.windows(64).any(|w|w==hash.as_bytes())),
        "researcher must not preload candidate identity into VM bytecode");
    let root=policy.content_hash().unwrap();
    put(&mut w,version_key(&root),policy_bytes);
    let metadata=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(metadata.code,Code::Ok);
    let parsed:serde_json::Value=
        serde_json::from_slice(&metadata.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        parsed["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+2048;
    assert_eq!(b_gas(&w),20);
    assert_eq!(candidates.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64)
        .sum::<u64>(),834);
    (w,candidates,root,new_hash,used)
}
fn paid_catalog_read(w:&mut World,key:&str)->ocee_world_kernel::ActionResult {
    w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:key.into()
    }))
}
fn catalog_entries(w:&World,start:u64)->(Vec<String>,u64){
    let actual=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_CATALOG_PAGE0))
        .collect::<Vec<_>>();
    assert_eq!(actual.len(),1,
        "the active graph may read the catalog only once");
    assert_eq!(actual[0].result.code,Code::Ok);
    assert!(actual[0].result.gas_charged>0);
    let parsed:serde_json::Value=
        serde_json::from_slice(&actual[0].result.payload).unwrap();
    let items=parsed["entries"].as_array().expect("real paid entries");
    assert_eq!(parsed["page"],0);
    assert_eq!(parsed["count"].as_u64().unwrap() as usize,items.len());
    (items.iter().map(|v|v["hash"].as_str().unwrap().to_owned())
        .collect(),actual[0].result.gas_charged)
}
fn advance_to_catalog_index(w:&mut World,start:u64){
    let mut observed=false;
    for _ in 0..24{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::InvalidCode|Code::Denied));
        if matches!(w.audit.last().unwrap().action,
            Some(Action::IndexVersionRefs{..})){
            assert_eq!(r.code,Code::Ok);
            observed=true;break;
        }
    }
    assert!(observed,"real active policy must pay to certify after inventory");
    let _=catalog_entries(w,start);
}
fn discovery_gc_audit(w:&World,start:u64)->Vec<(String,Code,u64)>{
    w.audit.iter().filter_map(|e|{
        if e.step<start{return None;}
        match &e.action{
            Some(Action::ReclaimIndexedVersion{program_hash,..})=>
                Some((program_hash.clone(),e.result.code,
                    e.result.gas_charged)),
            _=>None
        }
    }).collect()
}
fn test_catalog_discovery_arm(arm:&str){
    let (mut w,candidates,root,new_hash,used)=catalog_research_setup();
    let start=w.state.step;
    let (vm_begin,action_begin)=spend(&w);
    if arm=="CATALOG_CONTRACT"{
        let first=paid_catalog_read(&mut w,S29_CATALOG_PAGE0);
        assert_eq!(first.code,Code::Ok);
        assert!(first.gas_charged>0);
        let parsed:serde_json::Value=
            serde_json::from_slice(&first.payload).unwrap();
        let mut expected=w.state.objects.iter().filter_map(|(key,obj)|{
            let hash=key.strip_prefix("/self/versions/")?;
            (obj.owner==ORGANISM_ID && hash.len()==64).then_some(
                (obj.bytes.len() as u64,hash.to_owned()))
        }).collect::<Vec<_>>();
        expected.sort();
        assert_eq!(parsed["total"].as_u64().unwrap(),expected.len() as u64);
        let rows=parsed["entries"].as_array().unwrap();
        assert_eq!(rows.len(),expected.len().min(8));
        for (i,v) in rows.iter().enumerate(){
            assert_eq!(v["hash"],expected[i].1);
            assert_eq!(v["bytes"].as_u64().unwrap(),expected[i].0);
            assert!(!v.as_object().unwrap().contains_key("referenced"));
            assert!(!v.as_object().unwrap().contains_key("protected"));
        }
        let repeat=paid_catalog_read(&mut w,S29_CATALOG_PAGE0);
        assert_eq!(repeat.code,Code::Ok);
        assert_eq!(repeat.payload,first.payload);
        assert_eq!(repeat.gas_charged,first.gas_charged);
        let later=paid_catalog_read(&mut w,
            "/sys/self-version-catalog/127");
        assert_eq!(later.code,Code::Ok);
        let empty:serde_json::Value=
            serde_json::from_slice(&later.payload).unwrap();
        assert_eq!(empty["count"],0);
        assert_eq!(empty["total"],expected.len());
        for malformed in [
            "/sys/self-version-catalog/00",
            "/sys/self-version-catalog/nope",
            "/sys/self-version-catalog/128"
        ]{
            assert_eq!(paid_catalog_read(&mut w,malformed).code,
                Code::BadRequest);
        }
        // Researcher-injected gas starvation is a negative test fixture,
        // NOT a paid organism action. It must not mutate owned state.
        let before_obj=w.state.objects.clone();
        let before_storage=w.state.organisms[&ORGANISM_ID].storage_used;
        let before_index=w.state.version_ref_index.clone();
        w.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=0;
        let refused=paid_catalog_read(&mut w,S29_CATALOG_PAGE0);
        assert_eq!(refused.code,Code::Gas);
        assert_eq!(refused.gas_charged,0);
        assert!(refused.payload.is_empty());
        assert_eq!(w.state.objects,before_obj);
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
            before_storage);
        assert_eq!(w.state.version_ref_index,before_index);
        assert_eq!(b_gas(&w),20);
        assert_eq!(w.state_hash(),World::from_snapshot(
            &w.snapshot_json()).unwrap().state_hash());
        println!("S29_CATALOG CONTRACT root={root} count={} first_A={}             empty_page=Ok malformed=BadRequest starvation=Gas B=20",
            expected.len(),first.gas_charged);
        return;
    }
    if arm=="PIN_AND_CONTINUATION"{
        let pinned=candidates.iter().min_by_key(|h|
            (w.state.objects[&version_key(h)].bytes.len(),*h))
            .unwrap().clone();
        // The pin is installed by the RESEARCHER; the fixed VM program
        // has no literal target hash or privileged reference-status oracle.
        put(&mut w,"/self/pins/s29-catalog".into(),
            pinned.as_bytes().to_vec());
        let started=w.state.step;
        advance_to_catalog_index(&mut w,started);
        let before_snap=w.snapshot_json();
        let mut restore=World::from_snapshot(&before_snap).unwrap();
        run(&mut w,96);
        run(&mut restore,96);
        assert_eq!(w.snapshot_json(),restore.snapshot_json());
        assert_eq!(w.audit,restore.audit);
        let gc=discovery_gc_audit(&w,start);
        assert!(gc.iter().any(|(h,code,_)|
            h==&pinned && *code==Code::Conflict),
            "pinned discovered candidate must be vetoed by real kernel");
        assert!(w.state.objects.contains_key(&version_key(&pinned)));
        assert!(gc.iter().any(|(_,code,_)|*code==Code::Ok),
            "after a pinned cheapest candidate there are other candidates");
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        println!("S29_CATALOG PIN_AND_CONTINUATION root={root} pinned={pinned}             attempts={gc:?} epoch={} deterministic_snapshot=true",
            w.state.step/w.state.config.epoch_length);
    }else if arm=="INVALIDATED_CERT"{
        advance_to_catalog_index(&mut w,start);
        let pin=&candidates[0];
        put(&mut w,"/self/pins/s29-catalog".into(),
            pin.as_bytes().to_vec());
        assert!(w.state.version_ref_index.is_none());
        run(&mut w,96);
        let gc=discovery_gc_audit(&w,start);
        assert!(!gc.is_empty());
        assert!(gc.iter().all(|(_,code,fee)|
            *code==Code::Conflict && *fee==0));
        assert!(candidates.iter().all(|h|
            w.state.objects.contains_key(&version_key(h))));
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(0)));
        println!("S29_CATALOG INVALIDATED_CERT root={root}             attempts={gc:?} no_delete=true");
    }else if arm=="DYNAMIC_RECOVERY"{
        run(&mut w,96);
        let gc=discovery_gc_audit(&w,start);
        assert!(!gc.is_empty());
        assert!(gc.iter().any(|(_,code,fee)|
            *code==Code::Ok && *fee>0));
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
    }else{panic!("unknown catalog arm {arm}");}
    let (catalog_hashes,catalog_fee)=catalog_entries(&w,start);
    let gc=discovery_gc_audit(&w,start);
    assert!(gc.len()<=8);
    assert!(gc.iter().all(|(hash,_,_)|catalog_hashes.contains(hash)));
    assert_eq!(w.audit.iter().filter(|e|e.step>=start &&
        matches!(e.action,Some(Action::IndexVersionRefs{..})))
        .count(),1);
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(b_gas(&w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    let reclaimed=gc.iter().filter(|(_,code,_)|*code==Code::Ok)
        .map(|(h,_,_)|{
            let initial=candidates.iter().find(|a|*a==h);
            assert!(initial.is_some(),
                "in controlled fixture only installed orphan candidates reclaim");
            match h==&candidates[1]{
                true=>w.state.organisms[&ORGANISM_ID].storage_used,
                false=>w.state.organisms[&ORGANISM_ID].storage_used
            };
            // Old fixture tools: exactly 276 or 279 bytes.
            if h==&candidates[1]{
                279u64
            }else if h==&candidates[2]{
                276u64
            }else{279u64}
        }).sum::<u64>();
    assert_eq!(used +
        if arm=="PIN_AND_CONTINUATION"||arm=="INVALIDATED_CERT"{
            64
        }else{0}
        - w.state.organisms[&ORGANISM_ID].storage_used,reclaimed);
    let (vm_end,action_end)=spend(&w);
    println!("S29_CATALOG {arm} root={root} catalog_A={catalog_fee}         attempted={gc:?} reclaimed={reclaimed}B VM_A={} Action_A={}         remaining={} B={} step={} epoch={} replay=true",
        vm_end-vm_begin,action_end-action_begin,
        w.state.organisms[&ORGANISM_ID].storage_used,b_gas(&w),
        w.state.step,w.state.step/w.state.config.epoch_length);
}
#[test]
#[ignore="preregistered paid owner-inventory autonomous bounded candidate discovery"]
fn s29_paid_catalog_discovery_native(){
    let arm=std::env::var("OCEE_S29_CATALOG_ARM")
        .expect("four-arm paid own version catalog preregistration");
    match arm.as_str(){
        "CATALOG_CONTRACT"|"DYNAMIC_RECOVERY"|
        "PIN_AND_CONTINUATION"|"INVALIDATED_CERT"=>
            test_catalog_discovery_arm(&arm),
        other=>panic!("unexpected inventory arm {other}")
    }
}
