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
