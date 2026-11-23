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
fn versioned_self_retry_root(targets:&[String])->Program{
    let mut root=self_retry_maintenance_root(targets);
    let f=&mut root.functions[0];
    let block=|f:&mut Function,name:&str|
        f.blocks.iter_mut().find(|b|b.id==name)
            .expect("preexisting resource feedback block");
    {
        let b=block(f,"retry_body");
        b.instructions=vec![Instruction{
            id:50,op:lit(S29_OWN_META.as_bytes())
        }];
        assert!(matches!(&b.terminator,Terminator::Yield{
            call:WorldCall::Read,..}));
    }
    {
        let b=block(f,"risk_parse");
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
        let b=block(f,"risk_cooldown");
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
        let b=block(f,"risk_written_cooldown");
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
        let b=block(f,"risk_set");
        b.instructions=vec![Instruction{
            id:40,op:lit(S29_OWN_META.as_bytes())
        }];
        b.terminator=Terminator::Yield{
            call:WorldCall::Read,args:vec![40],
            next:"risk_set_meta".into(),next_args:vec![]
        };
    }
    {
        let b=block(f,"risk_written_failure");
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
