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
    boot_repaired_seed(129)
}
fn boot_repaired_seed(seed:u64)->(World,String,String,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s29_atomic_publish_repair_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let root_hash=root.content_hash().unwrap();
    let old=wrong_tool();let old_hash=old.content_hash().unwrap();
    let (new_hash,_)=intended_len();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
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
    // The task rule varies by seed: the fixed Len hash is a valid
    // strict expectation ONLY on the original seed129. Do not force a
    // researcher tool choice into the P2 program on seeds130/131.
    let selected=String::from_utf8(w.state.objects[REGISTRY].bytes.clone())
        .expect("real P2 registry must hold canonical ASCII program hash");
    if seed==129{
        assert_eq!(selected,new_hash,
            "preserve the original seed129 strict Len selection");
    }
    assert_eq!(w.state.objects[REGISTRY].version,1,
        "real P2 must publish its own selected repair via paid CAS");
    assert!(w.state.objects.contains_key(&format!("/self/versions/{old_hash}")));
    let selected_object=w.state.objects.get(&version_key(&selected))
        .expect("selected P2 output must really be an installed tool");
    let selected_program:Program=serde_json::from_slice(&selected_object.bytes)
        .expect("actual seed-specific selected program must be executable");
    assert_eq!(selected_program.canonical_bytes(),selected_object.bytes);
    assert_eq!(selected_program.content_hash().unwrap(),selected);
    assert_eq!(b_gas(&w),20);
    let tr:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let pred:u32=serde_json::from_slice(
        &w.state.objects[PREDICTION].bytes).unwrap();
    assert_eq!(pred,hidden_transform(seed,&tr.held_out_input),
        "all registered P2 roots must truly solve their seed's hidden task");
    (w,old_hash,selected,root_hash)
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
        assert_eq!(empty["total"].as_u64(),Some(expected.len() as u64));
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
            (w.state.objects[&version_key(h)].bytes.len(),h.as_str().to_owned()))
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
    // Infer byte savings only from genuine, audited kernel GC receipts,
    // never from researcher-provided size guesses or secret oracle values.
    let reclaimed=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::ReclaimIndexedVersion{..})) &&
        e.result.code==Code::Ok)
        .map(|e|u64::from_le_bytes(
            e.result.payload.as_slice().try_into()
                .expect("real GC receipt is u64 reclaimed bytes")))
        .sum::<u64>();
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


/// DEV S29: A researcher-installed typed resource gate wraps the same
/// runtime-discovered catalog GC policy. No target program hash appears in
/// its bytecode; no new world call or privileged environment probe is added.
/// Both fixed thresholds are preregistered researcher choices, not learning.
fn resource_budget_catalog_root()->Program{
    let mut p=catalog_discovery_root();
    let func=&mut p.functions[0];
    assert_eq!(func.blocks[0].id,"entry");
    func.blocks[0].id="catalog_entry".into();
    let prefix=vec![
        Block{
            id:"entry".into(),
            params:vec![Param{id:0,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:1,op:lit(b"/sys/resource-status")}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![1],
                next:"status_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"status_reply".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:10,op:lit(b"code")},
                Instruction{id:11,op:Op::JsonFieldString(2,10)},
                Instruction{id:12,op:lit(b"Ok")},
                Instruction{id:13,op:Op::EqBytes(11,12)}
            ],
            terminator:Terminator::Branch{
                condition:13,if_true:"parse_resource_status".into(),
                true_args:vec![2],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"parse_resource_status".into(),
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
                Instruction{id:18,op:Op::Sub(15,13)},
                Instruction{id:19,op:Op::ConstU32(4096)},
                Instruction{id:20,op:Op::Div(18,19)},
                Instruction{id:21,op:Op::ConstU32(0)},
                Instruction{id:22,op:Op::EqU32(20,21)},
                Instruction{id:23,op:Op::ConstU32(3000)},
                Instruction{id:24,op:Op::Div(17,23)},
                Instruction{id:25,op:Op::EqU32(24,21)},
                Instruction{id:26,op:lit(b"")}
            ],
            // Quota/gas starvation is a terminal strategy state, not an
            // excuse to attempt an expensive index with inadequate A.
            terminator:Terminator::Branch{
                condition:25,if_true:"failed".into(),
                true_args:vec![],if_false:"check_storage_pressure".into(),
                false_args:vec![22,26]
            }
        },
        Block{
            id:"check_storage_pressure".into(),
            params:vec![
                Param{id:1,ty:Type::Bool},
                Param{id:2,ty:Type::Bytes}
            ],
            instructions:vec![],
            terminator:Terminator::Branch{
                condition:1,if_true:"catalog_entry".into(),
                true_args:vec![2],if_false:"failed".into(),
                false_args:vec![]
            }
        }
    ];
    let mut old=std::mem::take(&mut func.blocks);
    func.blocks=prefix;
    func.blocks.append(&mut old);
    p.validate().expect("resource-gated runtime discovery must validate");
    p
}

/// Actual P2 repair and charged installation, as in earlier S29 experiments,
/// but with a SINGLE researcher-supplied generic resource-gated program.
fn setup_resource_budget_world()->(World,Vec<String>,String,String,u64){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut candidates=vec![old_hash];
    for tool in extra_unreferenced_tools(){
        let h=tool.content_hash().unwrap();
        assert!(!w.state.objects.contains_key(&version_key(&h)));
        put(&mut w,version_key(&h),tool.canonical_bytes());
        candidates.push(h);
    }
    let p=resource_budget_catalog_root();
    let serialized=p.canonical_bytes();
    assert!(candidates.iter().all(|h|
        !serialized.windows(64).any(|v|v==h.as_bytes())),
        "candidate identity must not be injected into gated VM code");
    let root=p.content_hash().unwrap();
    put(&mut w,version_key(&root),serialized);
    let meta=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(meta.code,Code::Ok);
    let value:serde_json::Value=
        serde_json::from_slice(&meta.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        value["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+2048;
    assert_eq!(candidates.iter().map(|h|
        w.state.objects[&version_key(h)].bytes.len() as u64)
        .sum::<u64>(),834);
    assert_eq!(b_gas(&w),20);
    (w,candidates,root,new_hash,used)
}
const S29_RESOURCE_READ:&str="/sys/resource-status";
fn resource_status_receipt(w:&World,start:u64)->(u64,u64,u64,u64){
    let records=w.audit.iter().filter(|a|a.step>=start &&
        matches!(&a.action,Some(Action::Read{key,..})
            if key==S29_RESOURCE_READ))
        .collect::<Vec<_>>();
    assert_eq!(records.len(),1,
        "active VM must consult real own-resource status once");
    assert_eq!(records[0].result.code,Code::Ok);
    assert_eq!(records[0].result.gas_charged,2,
        "resource status is really paid: 2 A");
    let parsed:serde_json::Value=
        serde_json::from_slice(&records[0].result.payload).unwrap();
    let used=parsed["used"].as_u64().unwrap();
    let capacity=parsed["capacity"].as_u64().unwrap();
    let a=parsed["a_available"].as_u64().unwrap();
    assert!(capacity>=used);
    (used,capacity,a,records[0].step)
}
fn advance_to_paid_resource_status(w:&mut World,start:u64){
    let mut found=false;
    for _ in 0..12{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::Read{key,..}) if key==S29_RESOURCE_READ){
            assert_eq!(r.code,Code::Ok);
            found=true;break;
        }
    }
    assert!(found,"true VM must pay for current own-resource status");
    let _=resource_status_receipt(w,start);
}
fn test_resource_budget_gate_arm(arm:&str){
    let (mut w,candidates,root,new_hash,used)=setup_resource_budget_world();
    let (should_execute,expected_free)=match arm{
        "PRESSURE_EXECUTE"|"RACE_AFTER_SENSOR"=>(true,2048u64),
        "PROTECTED_ONLY"=>(true,2048u64),
        "SPARE_SKIP"=>(false,8192u64),
        "LOW_A_SKIP"=>(false,2048u64),
        _=>panic!("unknown preregistered resource gate arm {arm}")
    };
    if arm=="SPARE_SKIP"{
        // Researcher-controlled world configuration, not an organism action.
        w.state.nodes.get_mut(&NodeId::A).unwrap()
            .storage_capacity=used+8192;
    }
    if arm=="LOW_A_SKIP"{
        // A real deterministic scheduled resource event followed by a REAL
        // epoch grant. Never overwrite the gas balance by test fiat.
        use ocee_world_kernel::{EventKind,ScheduledEvent};
        let length=w.state.config.epoch_length;
        let next=(w.state.step/length+1)*length;
        w.schedule_event(ScheduledEvent{
            due_step:next,priority:0,sequence_no:0,
            kind:EventKind::ResourceChange{
                node:NodeId::A,gas_per_epoch:2400
            }
        });
        while w.state.step<=next {w.tick(None);}
        assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,2400);
        assert_eq!(w.state.nodes[&NodeId::A].gas_available,2400);
        assert_eq!(w.state.last_granted_epoch,Some(next/length));
    }
    let mut fully_protected_pin_bytes=0u64;
    if arm=="PROTECTED_ONLY"{
        // First-run research fixture mistake (#45202464053): pinning only
        // three researcher-added orphans did NOT guarantee that the real
        // existing P2 world had no other reclaimable installed version.
        // Enumerate ALL actual actor-owned installed versions in this
        // researcher-only negative-control setup, and pay for one 64-byte
        // full raw-hash pin per version, including the original three.
        let actual_versions=w.state.objects.iter().filter_map(|(k,o)|{
            let hash=k.strip_prefix("/self/versions/")?;
            (o.owner==ORGANISM_ID && hash.len()==64).then_some(hash.to_owned())
        }).collect::<Vec<_>>();
        assert!(candidates.iter().all(|h|actual_versions.contains(h)));
        assert!(!actual_versions.is_empty());
        fully_protected_pin_bytes=actual_versions.len() as u64 * 64;
        assert!(fully_protected_pin_bytes<=2048);
        for (i,h) in actual_versions.iter().enumerate(){
            put(&mut w,format!("/self/pins/s29-gate-{i}"),
                h.as_bytes().to_vec());
        }
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
            used+fully_protected_pin_bytes);
    }
    let start=w.state.step;
    let (vm_before,action_before)=spend(&w);
    let mut pinned:Option<String>=None;
    if arm=="RACE_AFTER_SENSOR"{
        advance_to_paid_resource_status(&mut w,start);
        let target=candidates.iter().min_by_key(|h|
            (w.state.objects[&version_key(h)].bytes.len(),h.as_str().to_owned()))
            .unwrap().clone();
        let r=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:"/self/pins/s29-gate-race".into(),
            expected_version:None,bytes:target.as_bytes().to_vec()
        }));
        assert_eq!(r.code,Code::Ok);
        assert_eq!(r.gas_charged,6);
        pinned=Some(target);
    }
    let replay_point=World::from_snapshot(&w.snapshot_json()).unwrap();
    run(&mut w,100);
    // Same frozen VM bytes, continuation and world state must replay.
    let mut replay=replay_point;
    run(&mut replay,100);
    assert_eq!(w.snapshot_json(),replay.snapshot_json());
    assert_eq!(w.audit,replay.audit);
    let (reported_used,cap,a_remaining,observed_step)=
        resource_status_receipt(&w,start);
    if arm=="PROTECTED_ONLY"{
        assert_eq!(reported_used,used+fully_protected_pin_bytes);
        assert_eq!(cap-reported_used,2048-fully_protected_pin_bytes);
    }else{
        assert_eq!(reported_used,used);
        assert_eq!(cap-reported_used,expected_free);
    }
    if arm=="LOW_A_SKIP"{
        assert!(a_remaining<3000);
    }else{
        assert!(a_remaining>=3000);
    }
    let cat=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_CATALOG_PAGE0))
        .collect::<Vec<_>>();
    let index=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::IndexVersionRefs{..})))
        .collect::<Vec<_>>();
    let gc=discovery_gc_audit(&w,start);
    if should_execute{
        assert_eq!(cat.len(),1);
        assert_eq!(index.len(),1);
        assert_eq!(cat[0].result.code,Code::Ok);
        assert_eq!(index[0].result.code,Code::Ok);
        assert!(cat[0].result.gas_charged>0);
        assert!(index[0].result.gas_charged>0);
        let (observed_candidates,_)=catalog_entries(&w,start);
        assert!(gc.iter().all(|(h,_,_)|
            observed_candidates.contains(h)),
            "no program hash may be injected outside paid inventory");
        assert!(gc.len()<=8);
        if arm=="PROTECTED_ONLY"{
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(0)));
            assert!(!gc.is_empty());
            assert!(gc.iter().all(|(_,code,_)|*code==Code::Conflict));
            assert!(candidates.iter().all(|h|
                w.state.objects.contains_key(&version_key(h))));
        }else{
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(1)));
            assert!(gc.iter().any(|(_,code,fee)|
                *code==Code::Ok&&*fee>0));
            if let Some(pin)=pinned.as_ref(){
                assert!(gc.iter().any(|(h,code,_)|
                    h==pin&&*code==Code::Conflict),
                    "true pin introduced after sensor must block later GC");
                assert!(w.state.objects.contains_key(&version_key(pin)));
            }
        }
    }else{
        assert!(cat.is_empty(),"spare or low-A policy must not read catalog");
        assert!(index.is_empty(),"spare or low-A policy must not scan index");
        assert!(gc.is_empty(),"spare or low-A policy must not attempt GC");
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(0)));
        assert!(candidates.iter().all(|h|
            w.state.objects.contains_key(&version_key(h))));
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,used);
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(count_internal_active_switches(&w,start),0);
    assert_eq!(b_gas(&w),20);
    let reclaimed=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::ReclaimIndexedVersion{..})) &&
        e.result.code==Code::Ok)
        .map(|e|u64::from_le_bytes(
            e.result.payload.as_slice().try_into().unwrap()))
        .sum::<u64>();
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used + fully_protected_pin_bytes +
        if pinned.is_some(){64}else{0} - reclaimed);
    let (vm_after,action_after)=spend(&w);
    println!("S29_RESOURCE_GATE {arm} root={root} \
        observed_step={observed_step} free={} a_remaining={} \
        sensor_A=2 catalog_reads={} index_count={} GC={gc:?} \
        reclaimed_B={} VM_A={} Action_A={} \
        used={} capacity={} B={} last_return={:?} \
        replay_snapshot=true",
        cap-reported_used,a_remaining,cat.len(),index.len(),reclaimed,
        vm_after-vm_before,action_after-action_before,
        w.state.organisms[&ORGANISM_ID].storage_used,
        w.state.nodes[&NodeId::A].storage_capacity,b_gas(&w),
        w.state.organisms[&ORGANISM_ID].last_return);
}
#[test]
#[ignore="preregistered charged resource and gas policy gate with native P2 controls"]
fn s29_resource_budget_gate_native(){
    let arm=std::env::var("OCEE_S29_GATE_ARM")
        .expect("five-arm genuine own-resource budget gate");
    match arm.as_str(){
        "PRESSURE_EXECUTE"|"SPARE_SKIP"|"LOW_A_SKIP"|
        "RACE_AFTER_SENSOR"|"PROTECTED_ONLY"=>
            test_resource_budget_gate_arm(&arm),
        other=>panic!("unknown resource gate arm {other}")
    }
}


// DEV S29: A researcher-authored, one-activation bounded feedback policy.
// It does NOT know any installed program hash, receive a GC eligibility
// oracle, or possess a new instruction/action. Negative discovery is only
// inferred from actual charged Conflicts and persisted via paid own CAS.
const S29_DRY_META:&str="/sys/self-object/maintenance/s29-dry-run";
const S29_DRY_KEY:&str="/self/maintenance/s29-dry-run";
fn dry_run_cooldown_root()->Program{
    let mut p=resource_budget_catalog_root();
    let f=&mut p.functions[0];
    let old_entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    old_entry.id="maintenance_entry".into();
    let exhausted=f.blocks.iter_mut().find(|b|b.id=="search_header").unwrap();
    assert!(matches!(&exhausted.terminator,Terminator::RepeatBounded{..}));
    // The predecessor's RepeatBounded counter would remain at 8 in the
    // SAME VM continuation after the first episode. Existing RepeatScoped
    // instead charges/limits each candidate-index scope (U32 register 3)
    // to max three visits across all three episodes, and max 24 globally.
    // The existing actual catalog count still caps each episode at 8.
    exhausted.terminator=Terminator::RepeatScoped{
        limit:3,aggregate_limit:24,scope:3,
        body:"check_count".into(),body_args:vec![1,2,3],
        done:"mark_dry".into(),done_args:vec![]
    };
    let no_more=f.blocks.iter_mut().find(|b|b.id=="check_count").unwrap();
    if let Terminator::Branch{if_true,true_args,..}=&mut no_more.terminator{
        assert_eq!(if_true,"failed");
        *if_true="mark_dry".into();
        true_args.clear();
    }else{panic!("existing finite search guard lost");}
    let response_code=|reg:u32,expected:&[u8]|vec![
        Instruction{id:30,op:lit(b"code")},
        Instruction{id:31,op:Op::JsonFieldString(reg,30)},
        Instruction{id:32,op:lit(expected)},
        Instruction{id:33,op:Op::EqBytes(31,32)}
    ];
    let reply_param=||vec![Param{id:2,ty:Type::Bytes}];
    let prefix=vec![
        Block{
            id:"entry".into(),
            params:vec![Param{id:0,ty:Type::Bytes}],
            instructions:vec![],
            terminator:Terminator::Jump{
                target:"round_header".into(),args:vec![]
            }
        },
        Block{
            id:"round_header".into(),params:vec![],
            instructions:vec![],
            terminator:Terminator::RepeatBounded{
                limit:3,body:"probe_memory".into(),body_args:vec![],
                done:"failed".into(),done_args:vec![]
            }
        },
        Block{
            id:"probe_memory".into(),params:vec![],
            instructions:vec![Instruction{id:10,op:lit(S29_DRY_META.as_bytes())}],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![10],
                next:"probe_memory_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"probe_memory_reply".into(),params:reply_param(),
            instructions:response_code(2,b"Ok"),
            terminator:Terminator::Branch{
                condition:33,if_true:"probe_memory_exists".into(),
                true_args:vec![2],if_false:"probe_memory_missing".into(),
                false_args:vec![2]
            }
        },
        Block{
            id:"probe_memory_missing".into(),params:reply_param(),
            instructions:response_code(2,b"NotFound"),
            terminator:Terminator::Branch{
                condition:33,if_true:"start_uncached".into(),
                true_args:vec![],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"start_uncached".into(),params:vec![],
            instructions:vec![Instruction{id:1,op:lit(b"")}],
            terminator:Terminator::Jump{
                target:"maintenance_entry".into(),args:vec![1]
            }
        },
        Block{
            id:"probe_memory_exists".into(),params:reply_param(),
            instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"bytes")},
                Instruction{id:13,op:Op::JsonFieldBytes(11,12)},
                Instruction{id:14,op:lit(b"version")},
                Instruction{id:15,op:Op::JsonFieldU32(11,14)},
                Instruction{id:16,op:lit(b"1")},
                Instruction{id:17,op:Op::EqBytes(13,16)}
            ],
            terminator:Terminator::Branch{
                condition:17,if_true:"cooldown_reset".into(),
                true_args:vec![15],if_false:"probe_memory_zero".into(),
                false_args:vec![13]
            }
        },
        Block{
            id:"probe_memory_zero".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:10,op:lit(b"0")},
                Instruction{id:11,op:Op::EqBytes(2,10)}
            ],
            terminator:Terminator::Branch{
                condition:11,if_true:"start_uncached".into(),
                true_args:vec![],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"cooldown_reset".into(),
            params:vec![Param{id:1,ty:Type::U32}],
            instructions:vec![
                Instruction{id:10,op:lit(S29_DRY_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"0")}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,1],
                next:"cooldown_reset_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"cooldown_reset_reply".into(),params:reply_param(),
            instructions:response_code(2,b"Ok"),
            terminator:Terminator::Branch{
                condition:33,if_true:"wait_next".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        }
    ];
    // Only a completely exhausted real bounded catalog search can mark
    // negative memory; unrelated gas, invalid input, missing certificate,
    // corrupt feedback or lost CAS must fail without poisoning the record.
    let suffix=vec![
        Block{
            id:"mark_dry".into(),params:vec![],
            instructions:vec![Instruction{id:10,op:lit(S29_DRY_META.as_bytes())}],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![10],
                next:"mark_dry_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"mark_dry_reply".into(),params:reply_param(),
            instructions:response_code(2,b"Ok"),
            terminator:Terminator::Branch{
                condition:33,if_true:"mark_dry_existing".into(),
                true_args:vec![2],if_false:"mark_dry_missing".into(),
                false_args:vec![2]
            }
        },
        Block{
            id:"mark_dry_missing".into(),params:reply_param(),
            instructions:response_code(2,b"NotFound"),
            terminator:Terminator::Branch{
                condition:33,if_true:"mark_dry_create".into(),
                true_args:vec![],if_false:"failed".into(),
                false_args:vec![]
            }
        },
        Block{
            id:"mark_dry_existing".into(),params:reply_param(),
            instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"version")},
                Instruction{id:13,op:Op::JsonFieldU32(11,12)}
            ],
            terminator:Terminator::Jump{
                target:"mark_dry_write".into(),args:vec![13]
            }
        },
        Block{
            id:"mark_dry_create".into(),params:vec![],
            instructions:vec![Instruction{id:1,op:Op::ConstU32(u32::MAX)}],
            terminator:Terminator::Jump{
                target:"mark_dry_write".into(),args:vec![1]
            }
        },
        Block{
            id:"mark_dry_write".into(),
            params:vec![Param{id:1,ty:Type::U32}],
            instructions:vec![
                Instruction{id:10,op:lit(S29_DRY_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"1")}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,1],
                next:"mark_dry_write_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"mark_dry_write_reply".into(),params:reply_param(),
            instructions:response_code(2,b"Ok"),
            terminator:Terminator::Branch{
                condition:33,if_true:"wait_next".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"wait_next".into(),params:vec![],instructions:vec![],
            terminator:Terminator::Yield{
                call:WorldCall::Observe,args:vec![],
                next:"after_observe".into(),next_args:vec![]
            }
        },
        Block{
            id:"after_observe".into(),params:reply_param(),
            instructions:vec![],
            terminator:Terminator::Jump{
                target:"round_header".into(),args:vec![]
            }
        }
    ];
    let mut old=std::mem::take(&mut f.blocks);
    f.blocks=prefix;
    f.blocks.append(&mut old);
    f.blocks.extend(suffix);
    p.validate().expect("bounded three-round fully typed paid feedback root");
    p
}
fn setup_dry_run_world()->(World,Vec<String>,String,String,u64,Vec<String>){
    let (mut w,old_hash,new_hash,_)=boot_repaired();
    let mut candidates=vec![old_hash];
    for tool in extra_unreferenced_tools(){
        let h=tool.content_hash().unwrap();
        put(&mut w,version_key(&h),tool.canonical_bytes());
        candidates.push(h);
    }
    let p=dry_run_cooldown_root();
    let raw=p.canonical_bytes();
    assert!(candidates.iter().all(|h|
        !raw.windows(64).any(|window|window==h.as_bytes())));
    let root=p.content_hash().unwrap();
    put(&mut w,version_key(&root),raw);
    let metadata=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(metadata.code,Code::Ok);
    let v:serde_json::Value=
        serde_json::from_slice(&metadata.payload).unwrap();
    alter(&mut w,"/self/active",root.as_bytes().to_vec(),
        v["version"].as_u64().unwrap());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+2048;
    let versions=w.state.objects.iter().filter_map(|(k,obj)|{
        let h=k.strip_prefix("/self/versions/")?;
        (h.len()==64 && obj.owner==ORGANISM_ID).then_some(h.to_owned())
    }).collect::<Vec<_>>();
    assert!(candidates.iter().all(|h|versions.contains(h)));
    assert!(versions.len()<=16);
    assert_eq!(b_gas(&w),20);
    (w,candidates,root,new_hash,used,versions)
}
fn paid_pin_all_dry(w:&mut World,versions:&[String],filler:bool){
    for (i,hash) in versions.iter().enumerate(){
        put(w,format!("/self/pins/s29-dry-{i}"),
            if filler{vec![b'-';64]}else{hash.as_bytes().to_vec()});
    }
}
fn paid_memory_value(w:&World)->(Vec<u8>,u64){
    let obj=w.state.objects.get(S29_DRY_KEY)
        .expect("real paid VM must own its negative-feedback memory");
    assert_eq!(obj.owner,ORGANISM_ID);
    (obj.bytes.clone(),obj.version)
}
fn advance_to_dry_write(w:&mut World,expected:&[u8],max:u64){
    let mut found=false;
    for _ in 0..max{
        let x=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(x.code,Code::Denied|Code::InvalidCode),
            "bounded VM must execute only legitimate world capabilities");
        if matches!(&w.audit.last().unwrap().action,
            Some(Action::Write{key,bytes,..})
                if key==S29_DRY_KEY && bytes==expected){
            assert_eq!(x.code,Code::Ok);
            assert!(x.gas_charged>0);
            found=true;break;
        }
    }
    assert!(found,"a paid, actual VM CAS must be found");
}
fn collect_dry_calls(w:&World,start:u64)
    ->(Vec<u64>,Vec<(u64,u64)>,Vec<(String,Code,u64)>)
{
    let catalogs=w.audit.iter().filter(|e|e.step>=start &&
        matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_CATALOG_PAGE0))
        .map(|e|e.step).collect::<Vec<_>>();
    let indexes=w.audit.iter().filter(|e|e.step>=start &&
        matches!(e.action,Some(Action::IndexVersionRefs{..})))
        .map(|e|(e.step,e.result.gas_charged)).collect::<Vec<_>>();
    let gc=discovery_gc_audit(w,start);
    (catalogs,indexes,gc)
}
fn test_dry_run_arm(arm:&str){
    let (mut w,candidates,root,new_hash,used,versions)=setup_dry_run_world();
    let start=w.state.step;
    if arm=="PAIRED_INDISTINGUISHABILITY"{
        let mut alternate=World::from_snapshot(&w.snapshot_json()).unwrap();
        paid_pin_all_dry(&mut w,&versions,false);
        paid_pin_all_dry(&mut alternate,&versions,true);
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
            alternate.state.organisms[&ORGANISM_ID].storage_used);
        assert_eq!(w.state.nodes[&NodeId::A].gas_available,
            alternate.state.nodes[&NodeId::A].gas_available);
        for key in [S29_RESOURCE_READ,S29_CATALOG_PAGE0]{
            let pinned=paid_catalog_read(&mut w,key);
            let opaque=paid_catalog_read(&mut alternate,key);
            assert_eq!(pinned.code,Code::Ok);
            assert_eq!(opaque.code,Code::Ok);
            assert_eq!(pinned.payload,opaque.payload);
            assert_eq!(pinned.gas_charged,opaque.gas_charged);
        }
        let target=candidates.iter().find(|h|
            w.state.objects[&version_key(h)].bytes.len()==276)
            .expect("genuine old small unreferenced version").clone();
        let idx1=index_refs(&mut w);
        let idx2=index_refs(&mut alternate);
        assert_eq!(idx1.code,Code::Ok);
        assert_eq!(idx2.code,Code::Ok);
        assert!(idx1.gas_charged>0 && idx2.gas_charged>0);
        assert_eq!(idx1.gas_charged,idx2.gas_charged,
            "same physical scan bytes, different source references");
        assert!(w.state.version_ref_index.as_ref().unwrap()
            .referenced.contains(&target));
        assert!(!alternate.state.version_ref_index.as_ref().unwrap()
            .referenced.contains(&target));
        let veto=fast_reclaim(&mut w,&target);
        let release=fast_reclaim(&mut alternate,&target);
        assert_eq!(veto.code,Code::Conflict);
        assert_eq!(release.code,Code::Ok);
        assert_eq!(u64::from_le_bytes(
            release.payload.as_slice().try_into().unwrap()),276);
        assert!(w.state.objects.contains_key(&version_key(&target)));
        assert!(!alternate.state.objects.contains_key(&version_key(&target)));
        for ww in [&w,&alternate]{
            assert_eq!(ww.state.objects[REGISTRY].bytes,new_hash.as_bytes());
            assert_eq!(b_gas(ww),20);
            assert_eq!(ww.state_hash(),World::from_snapshot(
                &ww.snapshot_json()).unwrap().state_hash());
        }
        println!("S29_DRY PAIRED_INDISTINGUISHABILITY owner_versions={} \
            same_64B_pin_sizes=true same_paid_sensor_and_catalog=true \
            same_paid_index_A={} pinned=Conflict opaque=Ok \
            released=276B B20 snapshots_ok=true",
            versions.len(),idx1.gas_charged);
        return;
    }
    paid_pin_all_dry(&mut w,&versions,false);
    assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
        used+64*versions.len() as u64);
    assert!(w.state.nodes[&NodeId::A].storage_capacity >=
        w.state.organisms[&ORGANISM_ID].storage_used);
    let memory_start=w.state.step;
    let (vm_before,action_before)=spend(&w);
    advance_to_dry_write(&mut w,b"1",200);
    assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));
    let (vm_after_first,action_after_first)=spend(&w);
    let (first_cat,first_index,first_gc)=collect_dry_calls(&w,memory_start);
    assert_eq!(first_cat.len(),1);
    assert_eq!(first_index.len(),1);
    assert_eq!(first_gc.len(),8);
    assert!(first_gc.iter().all(|(_,code,fee)|
        *code==Code::Conflict && *fee==0));
    let before_second=w.state.step;
    if arm=="STALE_MEMORY_CAS"{
        // Resume through the real Observe and have the VM read the cached
        // monotonic version; then a researcher wins the intervening CAS.
        let mut found=false;
        for _ in 0..25{
            let res=w.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(res.code,Code::Denied|Code::InvalidCode));
            let e=w.audit.last().unwrap();
            if matches!(&e.action,Some(Action::Read{key,..})
                if key==S29_DRY_META) && e.result.code==Code::Ok{
                let obj:serde_json::Value=
                    serde_json::from_slice(&e.result.payload).unwrap();
                assert_eq!(obj["version"].as_u64(),Some(0));
                assert_eq!(obj["bytes"],serde_json::json!([49]));
                found=true;break;
            }
        }
        assert!(found,"cached version must be seen by live VM");
        alter(&mut w,S29_DRY_KEY,b"0".to_vec(),0);
        let replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        run(&mut w,60);
        let mut same=replay;
        run(&mut same,60);
        assert_eq!(w.snapshot_json(),same.snapshot_json());
        assert_eq!(w.audit,same.audit);
        assert_eq!(paid_memory_value(&w),(b"0".to_vec(),1));
        let writes=w.audit.iter().filter(|e|e.step>=before_second &&
            matches!(&e.action,Some(Action::Write{key,bytes,..})
                if key==S29_DRY_KEY && bytes==b"0"))
            .collect::<Vec<_>>();
        assert_eq!(writes.len(),2,
            "researcher paid CAS and stale VM CAS both audited");
        assert_eq!(writes[0].result.code,Code::Ok);
        assert_eq!(writes[1].result.code,Code::Conflict);
        assert_eq!(writes[1].result.gas_charged,0);
        let (_,indexes,gc)=collect_dry_calls(&w,memory_start);
        assert_eq!(indexes.len(),1);
        assert_eq!(gc.len(),8);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(0)));
        assert_eq!(b_gas(&w),20);
        println!("S29_DRY STALE_MEMORY_CAS first_index_A={} \
            memory_write_first=Ok researcher=Ok \
            stale_VM=Conflict/0A version=1 \
            total_index_count=1 gc=8 no_delete=true \
            B=20 snapshot_replay=true",first_index[0].1);
        return;
    }
    advance_to_dry_write(&mut w,b"0",70);
    assert_eq!(paid_memory_value(&w),(b"0".to_vec(),1));
    let (vm_after_second,action_after_second)=spend(&w);
    let (second_cat,second_index,second_gc)=collect_dry_calls(&w,memory_start);
    assert_eq!(second_cat.len(),1,"cooldown must skip full catalog");
    assert_eq!(second_index.len(),1,"cooldown must skip all full scans");
    assert_eq!(second_gc.len(),8,"cooldown must skip GC itself");
    let second_vm=vm_after_second-vm_after_first;
    let second_a=action_after_second-action_after_first;
    assert!(second_vm+second_a<200,
        "recorded cooldown must avoid the 2,945 A zero-yield pass");
    let mut released:Option<String>=None;
    if arm=="RELEASE_AFTER_COOLDOWN"{
        let target=candidates.iter().find(|h|
            w.state.objects[&version_key(h)].bytes.len()==276)
            .expect("same real old P2 orphan").clone();
        let index=versions.iter().position(|h|h==&target).unwrap();
        let pin_key=format!("/self/pins/s29-dry-{index}");
        assert_eq!(w.state.objects[&pin_key].bytes,target.as_bytes());
        let removed=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:pin_key,expected_version:Some(0),
            bytes:vec![b'-';64]
        }));
        assert_eq!(removed.code,Code::Ok);
        assert!(removed.gas_charged>0);
        assert!(w.state.version_ref_index.is_none(),
            "ordinary full-hash-length write must invalidate certificate");
        released=Some(target);
    }else{
        assert_eq!(arm,"DRY_REPEAT");
    }
    let replay=World::from_snapshot(&w.snapshot_json()).unwrap();
    run(&mut w,260);
    let mut same=replay;
    run(&mut same,260);
    assert_eq!(w.snapshot_json(),same.snapshot_json());
    assert_eq!(w.audit,same.audit);
    let (all_cat,all_index,all_gc)=collect_dry_calls(&w,memory_start);
    assert_eq!(all_cat.len(),2);
    assert_eq!(all_index.len(),2);
    if let Some(target)=released{
        assert_eq!(paid_memory_value(&w),(b"0".to_vec(),1));
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        assert!(all_gc.iter().any(|(h,code,fee)|
            h==&target && *code==Code::Ok && *fee>0));
        assert!(!w.state.objects.contains_key(&version_key(&target)));
        for candidate in &candidates{
            if candidate!=&target{
                assert!(w.state.objects.contains_key(&version_key(candidate)));
            }
        }
        let free=w.audit.iter().filter(|e|e.step>=memory_start &&
            matches!(&e.action,Some(Action::ReclaimIndexedVersion{..})) &&
            e.result.code==Code::Ok)
            .map(|e|u64::from_le_bytes(
                e.result.payload.as_slice().try_into().unwrap()))
            .sum::<u64>();
        assert_eq!(free,276);
    }else{
        assert_eq!(paid_memory_value(&w),(b"1".to_vec(),2));
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(0)));
        assert_eq!(all_gc.len(),16);
        assert!(all_gc.iter().all(|(_,code,fee)|
            *code==Code::Conflict && *fee==0));
        assert!(candidates.iter().all(|h|
            w.state.objects.contains_key(&version_key(h))));
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
    assert_eq!(w.state.objects["/self/active"].bytes,root.as_bytes());
    assert_eq!(b_gas(&w),20);
    assert_eq!(count_internal_active_switches(&w,memory_start),0);
    assert_eq!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap()
        .loop_counters.get("round_header"),Some(&3));
    let (vm_final,a_final)=spend(&w);
    println!("S29_DRY {arm} root={root} rounds=3 \
        index_count={} catalog_count={} gc_count={} \
        first_VM_A={} first_Action_A={} \
        skipped_middle_VM_A={} skipped_middle_Action_A={} \
        total_VM_A={} total_Action_A={} \
        memory={:?} recovered={} B=20 snapshots_full_equal=true",
        all_index.len(),all_cat.len(),all_gc.len(),
        vm_after_first-vm_before,action_after_first-action_before,
        second_vm,second_a,vm_final-vm_before,a_final-action_before,
        paid_memory_value(&w),
        w.state.organisms[&ORGANISM_ID].last_return==Some(Value::U32(1)));
}
#[test]
#[ignore="paid real-world dry-run memory cooldown with complete source safety"]
fn s29_dry_run_memory_native(){
    let arm=std::env::var("OCEE_S29_DRY_ARM")
        .expect("frozen four-arm native dry-run feedback experiment");
    match arm.as_str(){
        "PAIRED_INDISTINGUISHABILITY"|"DRY_REPEAT"|
        "RELEASE_AFTER_COOLDOWN"|"STALE_MEMORY_CAS"=>
            test_dry_run_arm(&arm),
        other=>panic!("unknown dry-run negative-feedback arm {other}")
    }
}


// This is the FIRST native pilot of the already-frozen
// MAINTENANCE-MATCHED-LIFETIME-UTILITY-PROTOCOL, not its full five-tape,
// three-seed calibration. It uses truly matched exogenous tick schedules.
fn lifetime_always_retry_root()->Program{
    let mut p=dry_run_cooldown_root();
    let block=p.functions[0].blocks.iter_mut()
        .find(|b|b.id=="probe_memory_exists").unwrap();
    block.terminator=Terminator::Jump{
        target:"start_uncached".into(),args:vec![]
    };
    p.validate().expect("always retry without cached-negative skipping");
    p
}
fn lifetime_no_maintenance_root()->Program{
    let p=Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],
            returns:Type::U32,blocks:vec![Block{
                id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![Instruction{id:1,op:Op::ConstU32(0)}],
                terminator:Terminator::Return{value:1}
            }]
        }]
    };
    p.validate().unwrap();p
}
const S29_LIFE_TASK:&str="/self/outcomes/s29-lifetime-useful-artifact";
fn lifetime_set_active(w:&mut World,root:&str){
    let r=w.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/sys/active-version".into()
    }));
    assert_eq!(r.code,Code::Ok);
    let info:serde_json::Value=
        serde_json::from_slice(&r.payload).unwrap();
    alter(w,"/self/active",root.as_bytes().to_vec(),
        info["version"].as_u64().unwrap());
}
fn lifetime_setup(seed:u64)->(World,Vec<String>,String,[String;3],String){
    let (mut w,old_hash,new_hash,_)=boot_repaired_seed(seed);
    let mut introduced=vec![old_hash];
    for p in extra_unreferenced_tools(){
        let h=p.content_hash().unwrap();
        let key=version_key(&h);
        let canonical=p.canonical_bytes();
        if let Some(existing)=w.state.objects.get(&key){
            // The real seed130/131 P2 root may have already generated
            // this exact program: never write duplicate immutable code.
            assert_eq!(existing.bytes,canonical);
        }else{
            put(&mut w,key,canonical);
        }
        introduced.push(h);
    }
    let roots=[
        lifetime_always_retry_root(),
        dry_run_cooldown_root(),
        lifetime_no_maintenance_root()
    ];
    let mut hashes=Vec::new();
    for p in roots{
        let hash=p.content_hash().unwrap();
        let bytes=p.canonical_bytes();
        assert!(introduced.iter().all(|h|
            !bytes.windows(64).any(|window|window==h.as_bytes())),
            "policy bytecode is forbidden to encode a fixture's target hash");
        put(&mut w,version_key(&hash),bytes);
        hashes.push(hash);
    }
    let policy_hashes:[String;3]=hashes.try_into().unwrap();
    let all=w.state.objects.iter().filter_map(|(k,obj)|{
        let h=k.strip_prefix("/self/versions/")?;
        (obj.owner==ORGANISM_ID&&h.len()==64).then_some(h.to_owned())
    }).collect::<Vec<_>>();
    assert!(introduced.iter().all(|h|all.contains(h)));
    assert!(policy_hashes.iter().all(|h|all.contains(h)));
    // The same physical pin objects exist in EVERY policy arm, and include
    // all three maintenance roots (inactive as well as active). Thus the
    // roots are not confounded with additional reclaimable code candidates.
    paid_pin_all_dry(&mut w,&all,false);
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    // Seed-independent, PREDECLARED researcher decision rule:
    // choose smallest physical candidate NOT registered by real P2;
    // tie-break lexical hash. The VM never receives target identity.
    let target=introduced.iter().filter(|h|*h!=&new_hash)
        .min_by_key(|h|(
            w.state.objects[&version_key(h)].bytes.len(),(*h).clone()
        )).expect("there must be a nonregistered candidate").clone();
    let target_bytes=w.state.objects[&version_key(&target)].bytes.len();
    assert!(target_bytes>85,"a reclaimed version must unlock 280B task");
    // Discarded RESEARCHER-ONLY fork verifies actual reference safety and
    // real catalog eligibility. No trusted answer is added to VM state.
    let mut validate=World::from_snapshot(&w.snapshot_json()).unwrap();
    let inventory=paid_catalog_read(&mut validate,S29_CATALOG_PAGE0);
    assert_eq!(inventory.code,Code::Ok);
    assert!(inventory.gas_charged>0);
    let listing:serde_json::Value=
        serde_json::from_slice(&inventory.payload).unwrap();
    assert!(listing["entries"].as_array().unwrap().iter()
        .any(|entry|entry["hash"].as_str()==Some(target.as_str())),
        "selected researcher validation target must really appear on paid page0");
    let pin_index=all.iter().position(|h|h==&target).unwrap();
    let pin_path=format!("/self/pins/s29-dry-{pin_index}");
    let release=validate.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:pin_path,
        expected_version:Some(0),bytes:vec![b'-';64]
    }));
    assert_eq!(release.code,Code::Ok);
    assert_eq!(index_refs(&mut validate).code,Code::Ok);
    let reclaimed=fast_reclaim(&mut validate,&target);
    assert_eq!(reclaimed.code,Code::Ok,
        "true prebranch research clone proves chosen candidate is safely orphaned");
    assert_eq!(u64::from_le_bytes(
        reclaimed.payload.as_slice().try_into().unwrap()),
        target_bytes as u64);
    assert_eq!(b_gas(&w),20);
    (w,all,target,policy_hashes,new_hash)
}
#[derive(Debug,Clone,Default,Eq,PartialEq)]
struct LifetimeIndexProbe{
    samples:u64,
    post_tick_peak:usize,
    storage_peak:u64,
}
impl LifetimeIndexProbe{
    fn sample(&mut self,w:&World){
        let len=w.state.version_ref_index.as_ref()
            .map(|cert|serde_json::to_vec(cert).unwrap().len())
            .unwrap_or(0);
        self.post_tick_peak=self.post_tick_peak.max(len);
        self.storage_peak=self.storage_peak.max(
            w.state.organisms[&ORGANISM_ID].storage_used);
        self.samples+=1;
    }
}
fn lifetime_advance_to_action(
    w:&mut World,kind:&str,max:u64,probe:&mut LifetimeIndexProbe
){
    let mut hit=false;
    for _ in 0..max{
        let result=w.tick_autonomous(ORGANISM_ID);
        probe.sample(w);
        assert!(!matches!(result.code,Code::Denied|Code::InvalidCode),
            "maintenance world calls must remain typed and authorized");
        let evt=w.audit.last().unwrap();
        let triggered=match kind{
            "DRY_MARK"=>matches!(&evt.action,
                Some(Action::Write{key,bytes,..})
                    if key==S29_DRY_KEY&&bytes==b"1" &&
                        evt.result.code==Code::Ok),
            "COOLDOWN_RESET"=>matches!(&evt.action,
                Some(Action::Write{key,bytes,..})
                    if key==S29_DRY_KEY&&bytes==b"0" &&
                        evt.result.code==Code::Ok),
            "GC_SUCCESS"=>matches!(&evt.action,
                Some(Action::ReclaimIndexedVersion{..})
                    if evt.result.code==Code::Ok),
            "NO_OP"=>w.state.organisms[&ORGANISM_ID]
                .vm_continuation.as_ref().is_some_and(|c|c.finished),
            _=>panic!("unknown frozen advance goal {kind}")
        };
        if triggered{hit=true;break;}
    }
    assert!(hit,"VM must reach requested paid checkpoint {kind}");
}
fn lifetime_pad_to(w:&mut World,due:u64,probe:&mut LifetimeIndexProbe){
    assert!(w.state.step<=due,"VM missed the frozen exogenous event tick");
    while w.state.step<due{
        w.tick(None);
        probe.sample(w);
    }
    assert_eq!(w.state.step,due);
}
fn lifetime_finish(
    w:&mut World,policy:usize,tape:&str,pin_key:&str,target:&str,
    release_step:u64,task_step:u64,horizon:u64,
    probe:&mut LifetimeIndexProbe
)->(Code,u64){
    // Single exact external pin-CAS action at release_step, irrespective of
    // which policy was selected and whether it has discovered the target.
    lifetime_pad_to(w,release_step,probe);
    let original=w.state.objects.get(pin_key).unwrap();
    assert_eq!(original.bytes,target.as_bytes());
    let version=original.version;
    let data=if matches!(tape,"RELEASE_BEFORE_SECOND"|"RELEASE_AFTER_COOLDOWN"){
        vec![b'-';64]
    }else{
        assert_eq!(tape,"ALL_PINNED");
        target.as_bytes().to_vec()
    };
    let mutation=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:pin_key.into(),
        expected_version:Some(version),bytes:data
    }));
    probe.sample(w);
    assert_eq!(mutation.code,Code::Ok);
    assert!(mutation.gas_charged>0);
    // One bounded VM service opportunity after the same exogenous release.
    if policy==0{
        if matches!(tape,"RELEASE_BEFORE_SECOND"|"RELEASE_AFTER_COOLDOWN"){
            lifetime_advance_to_action(w,"GC_SUCCESS",75,probe);
        }else{
            lifetime_advance_to_action(w,"DRY_MARK",75,probe);
        }
    }else if policy==1{
        if tape=="RELEASE_AFTER_COOLDOWN"{
            lifetime_advance_to_action(w,"GC_SUCCESS",75,probe);
        }else{
            lifetime_advance_to_action(w,"COOLDOWN_RESET",75,probe);
            let (mark,version)=paid_memory_value(w);
            assert_eq!(mark,b"0".to_vec());
            assert_eq!(version,1);
        }
    }
    lifetime_pad_to(w,task_step,probe);
    // This is an EXTERNAL, identically timed useful storage artifact
    // submission (a bounded synthetic proxy), not a self-created task.
    let task=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:S29_LIFE_TASK.into(),
        expected_version:None,bytes:vec![0x55;280]
    }));
    probe.sample(w);
    assert!(matches!(task.code,Code::Ok|Code::Quota));
    if task.code==Code::Ok{
        assert_eq!(w.state.objects[S29_LIFE_TASK].bytes.len(),280);
    }else{
        assert!(!w.state.objects.contains_key(S29_LIFE_TASK));
    }
    // The cooldown policy may discover the released target in its *third*
    // episode, but this cannot retroactively make the already failed task
    // succeed. Record any later GC separately.
    for _ in 0..95{
        let done=w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished);
        if done{break;}
        let r=w.tick_autonomous(ORGANISM_ID);
        probe.sample(w);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
    }
    assert!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().is_some_and(|c|c.finished));
    lifetime_pad_to(w,horizon,probe);
    assert_eq!(b_gas(w),20);
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
    (task.code,task.gas_charged)
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct LifetimeMeasured{
    policy:&'static str,
    vm_a:u64,maintenance_action_a:u64,exogenous_a:u64,
    task_ok:bool,task_cost_a:u64,
    gc_attempts:usize,gc_successes:usize,gc_bytes:u64,
    catalog_count:usize,index_count:usize,
    used:u64,a_available:u64,
    policy_bytes:usize,index_serialized_final_unbilled:usize,
    index_serialized_post_tick_peak_unbilled:usize,
    index_post_tick_samples:u64,
}
fn measure_lifetime(
    w:&World,start:u64,policy:&'static str,policy_bytes:usize,
    task_code:Code,task_cost_a:u64,peak:&LifetimeIndexProbe
)->LifetimeMeasured{
    let mut vm_a=0u64;
    let mut maintenance_a=0u64;
    let mut external_a=0u64;
    let mut gcs=0usize;
    let mut successes=0usize;
    let mut reclaimed=0u64;
    let mut catalog=0usize;
    let mut indexes=0usize;
    for e in w.audit.iter().filter(|a|a.step>=start){
        vm_a+=e.result.vm_gas_charged;
        let exogenous=match &e.action{
            Some(Action::Write{key,..})=>
                key.starts_with("/self/pins/s29-dry-") ||
                key==S29_LIFE_TASK,
            _=>false
        };
        if exogenous{
            external_a+=e.result.gas_charged;
        }else{
            maintenance_a+=e.result.gas_charged;
        }
        if matches!(&e.action,Some(Action::Read{key,..})
            if key==S29_CATALOG_PAGE0){catalog+=1;}
        if matches!(&e.action,Some(Action::IndexVersionRefs{..})){
            indexes+=1;
        }
        if matches!(&e.action,Some(Action::ReclaimIndexedVersion{..})){
            gcs+=1;
            if e.result.code==Code::Ok{
                successes+=1;
                reclaimed+=u64::from_le_bytes(
                    e.result.payload.as_slice().try_into().unwrap());
            }
        }
    }
    // This is the ENDPOINT certificate serialization, not a sampled
    // lifetime peak: its actual peak still needs dedicated instrumentation
    // in the full multi-tape study. Not charged to organism physical quota.
    let index_bytes=w.state.version_ref_index.as_ref()
        .map(|x|serde_json::to_vec(x).unwrap().len()).unwrap_or(0);
    assert!(peak.samples>0);
    if indexes>0{
        assert!(peak.post_tick_peak>0,
            "real paid indexes must have nonzero post-tick certificate high-water");
    }
    LifetimeMeasured{
        policy,vm_a,maintenance_action_a:maintenance_a,
        exogenous_a:external_a,task_ok:task_code==Code::Ok,
        task_cost_a,gc_attempts:gcs,gc_successes:successes,
        gc_bytes:reclaimed,catalog_count:catalog,index_count:indexes,
        used:w.state.organisms[&ORGANISM_ID].storage_used,
        a_available:w.state.nodes[&NodeId::A].gas_available,
        policy_bytes,index_serialized_final_unbilled:index_bytes,
        index_serialized_post_tick_peak_unbilled:peak.post_tick_peak,
        index_post_tick_samples:peak.samples
    }
}
fn test_matched_lifetime_pilot(tape:&str,seed:u64){
    use sha2::{Digest,Sha256};
    let (base,versions,target,roots,new_hash)=lifetime_setup(seed);
    let initial_hash=base.state_hash();
    let pin_idx=versions.iter().position(|v|v==&target).unwrap();
    let pin_key=format!("/self/pins/s29-dry-{pin_idx}");
    let target_bytes=base.state.objects[&version_key(&target)]
        .bytes.len() as u64;
    let start_storage=base.state.organisms[&ORGANISM_ID].storage_used;
    let capacity=base.state.nodes[&NodeId::A].storage_capacity;
    assert_eq!(capacity-start_storage,195);
    let start_a=base.state.nodes[&NodeId::A].gas_available;
    let mut metrics=Vec::<LifetimeMeasured>::new();
    let policies=["ALWAYS_RETRY","NEGATIVE_COOLDOWN","NO_MAINTENANCE"];
    let mut common_schedule:Option<(u64,u64,u64)>=None;
    let mut common_branch_a:Option<u64>=None;
    for (p,policy) in policies.iter().enumerate(){
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,&roots[p]);
        let mut peak=LifetimeIndexProbe::default();
        peak.sample(&w);
        let study_start=w.state.step;
        let (release_step,task_step,horizon)=if tape=="RELEASE_AFTER_COOLDOWN"{
            (study_start+140,study_start+205,study_start+295)
        }else{
            (study_start+90,study_start+155,study_start+245)
        };
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,
            start_storage);
        let active_a=w.state.nodes[&NodeId::A].gas_available;
        assert!(active_a<start_a,
            "each root activation must pay real A");
        if let Some(previous)=common_branch_a{
            assert_eq!(active_a,previous,
                "all active root CAS operations have the same paid A");
        }else{common_branch_a=Some(active_a);}
        if let Some(sched)=common_schedule{
            assert_eq!(sched,(release_step,task_step,horizon));
        }else{
            common_schedule=Some((release_step,task_step,horizon));
        }
        if p==0 || p==1{
            lifetime_advance_to_action(&mut w,"DRY_MARK",80,&mut peak);
            assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));
        }else{
            lifetime_advance_to_action(&mut w,"NO_OP",10,&mut peak);
        }
        if tape=="RELEASE_AFTER_COOLDOWN"{
            // The exogenous release is expressly AFTER the verified
            // second round, not simply after researcher-idled ticks.
            if p==0{
                lifetime_advance_to_action(&mut w,"DRY_MARK",80,&mut peak);
                assert_eq!(paid_memory_value(&w),(b"1".to_vec(),1));
            }else if p==1{
                lifetime_advance_to_action(&mut w,"COOLDOWN_RESET",80,&mut peak);
                assert_eq!(paid_memory_value(&w),(b"0".to_vec(),1));
            }
            let pre_release_indexes=w.audit.iter()
                .filter(|e|e.step>=study_start &&
                    matches!(e.action,Some(Action::IndexVersionRefs{..})))
                .count();
            assert_eq!(pre_release_indexes,if p==0{2}else if p==1{1}else{0},
                "after-cooldown tape must prove actual pre-release episode");
        }
        let before_release=World::from_snapshot(
            &w.snapshot_json()).unwrap();
        let mut replay_peak=peak.clone();
        let (task,task_a)=lifetime_finish(
            &mut w,p,tape,&pin_key,&target,
            release_step,task_step,horizon,&mut peak
        );
        let mut replay=before_release;
        let (replay_task,replay_a)=lifetime_finish(
            &mut replay,p,tape,&pin_key,&target,
            release_step,task_step,horizon,&mut replay_peak
        );
        assert_eq!(peak,replay_peak,
            "actual post-tick index certificate peak must replay");
        assert_eq!((task,task_a),(replay_task,replay_a));
        assert_eq!(w.snapshot_json(),replay.snapshot_json());
        assert_eq!(w.audit,replay.audit);
        assert_eq!(w.state.objects[REGISTRY].bytes,new_hash.as_bytes());
        let root_program_len=base.state.objects[&version_key(&roots[p])]
            .bytes.len();
        metrics.push(measure_lifetime(
            &w,study_start,policy,root_program_len,task,task_a,&peak
        ));
    }
    let (release_step,task_step,horizon)=common_schedule.unwrap();
    let tape_bytes=serde_json::to_vec(&serde_json::json!({
        "seed":seed,"protocol":42,"family":"P2",
        "tape":tape,"release_step":release_step,
        "task_step":task_step,"horizon":horizon,
        "released_program":target,"pin_key":pin_key,
        "task_key":S29_LIFE_TASK,"task_bytes":280
    })).unwrap();
    let tape_sha=format!("{:x}",Sha256::digest(&tape_bytes));
    let shared_versions=versions.len();
    let always=&metrics[0];
    let cool=&metrics[1];
    let none=&metrics[2];
    assert!(always.vm_a>0&&cool.vm_a>0);
    assert_eq!(none.index_count,0);
    assert_eq!(none.gc_attempts,0);
    assert!(always.index_count<=3,
        "strictly bounded three episodes");
    assert_eq!(cool.index_count,2);
    if tape=="ALL_PINNED"{
        assert!(always.vm_a+always.maintenance_action_a >
            cool.vm_a+cool.maintenance_action_a,
            "when all versions remain pinned the cached negative result must save A");
        assert!(!always.task_ok&&!cool.task_ok&&!none.task_ok);
        assert_eq!((always.gc_bytes,cool.gc_bytes,none.gc_bytes),
            (0,0,0));
        assert_eq!(always.index_count,3);
        assert_eq!(cool.index_count,2);
    }else if tape=="RELEASE_BEFORE_SECOND"{
        assert!(always.task_ok);
        assert!(!cool.task_ok);
        assert!(!none.task_ok);
        assert_eq!(always.gc_bytes,target_bytes);
        assert_eq!(cool.gc_bytes,target_bytes,
            "late third-episode discovery still does not rescue earlier task");
        assert_eq!(none.gc_bytes,0);
    }else{
        assert_eq!(tape,"RELEASE_AFTER_COOLDOWN");
        assert!(always.task_ok&&cool.task_ok,
            "both real maintenance policies must reclaim before a later task");
        assert!(!none.task_ok);
        assert_eq!(always.gc_bytes,target_bytes);
        assert_eq!(cool.gc_bytes,target_bytes);
        assert_eq!(none.gc_bytes,0);
        assert_eq!(always.index_count,3);
        assert_eq!(cool.index_count,2);
    }
    assert_eq!(b_gas(&base),20);
    println!("S29_LIFETIME_PILOT tape={tape} seed={seed} base_snapshot_SHA={initial_hash} \
        event_tape_SHA256={tape_sha} \
        policy_hashes={roots:?} release_step={release_step} \
        task_step={task_step} horizon={horizon} \
        shared_installed={shared_versions} initial_used={start_storage} \
        actual_registered_P2_tool={new_hash} selected_research_orphan={target} \
        orphan_physical_bytes={target_bytes} \
        initial_capacity={capacity} initial_A={start_a} \
        metrics={metrics:?} true_P2_B=20 paired_replay=true \
        all_exogenous_actions_identical=true");
}
#[test]
#[ignore="registered paired P2 lifetime pilot, two tapes, three paid VM policies"]
fn s29_lifetime_matched_pilot_native(){
    let tape=std::env::var("OCEE_S29_LIFE_TAPE")
        .expect("pair-matched frozen exogenous tape");
    let seed: u64=std::env::var("OCEE_S29_LIFE_SEED")
        .unwrap_or_else(|_|"129".into()).parse()
        .expect("preregistered P2 seed is numeric");
    assert!([129,130,131].contains(&seed),
        "only three preregistered matched lifetime seeds");
    match tape.as_str(){
        "ALL_PINNED"|"RELEASE_BEFORE_SECOND"|"RELEASE_AFTER_COOLDOWN"=>
            test_matched_lifetime_pilot(&tape,seed),
        other=>panic!("unregistered lifetime tape {other}")
    }
}

/// After failures in the frozen three-seed maintenance matrix, inspect the
/// real public-example-selected P2 program, not the seed129 Len assumption.
/// This is diagnostic only: no host-selected replacement and no maintenance.
#[test]
#[ignore="three frozen P2 seed outcomes: actual selected tool, hidden oracle, billing"]
fn s29_crossseed_p2_baseline_native(){
    let seed:u64=std::env::var("OCEE_S29_BASE_SEED")
        .expect("one of exactly three preregistered P2 baseline seeds")
        .parse().expect("seed must be numeric");
    assert!([129,130,131].contains(&seed));
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s29_atomic_publish_repair_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let initial=wrong_tool();
    let old_hash=initial.content_hash().unwrap();
    let (len_hash,_)=intended_len();
    let root_hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,42);
    for p in [p1,p2,root,initial]{
        put(&mut w,version_key(&p.content_hash().unwrap()),
            p.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),root_hash.as_bytes().to_vec());
    put(&mut w,REGISTRY.into(),old_hash.as_bytes().to_vec());
    run(&mut w,16_384);
    let selected=String::from_utf8(w.state.objects[REGISTRY].bytes.clone())
        .expect("registered tool must have canonical ASCII hex hash");
    assert_eq!(selected.len(),64);
    assert!(selected.bytes().all(|b|b.is_ascii_hexdigit()&&
        !b.is_ascii_uppercase()));
    let installed=w.state.objects.get(&version_key(&selected))
        .expect("VM-selected published tool must really exist in storage");
    assert_eq!(installed.owner,ORGANISM_ID);
    let program:Program=serde_json::from_slice(&installed.bytes)
        .expect("VM-selected tool must be real Program object");
    assert_eq!(program.canonical_bytes(),installed.bytes);
    assert_eq!(program.content_hash().unwrap(),selected);
    let selected_op=format!("{:?}",
        &program.functions[0].blocks[0].instructions[1].op);
    let tr:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let prediction:u32=serde_json::from_slice(
        &w.state.objects[PREDICTION].bytes).unwrap();
    let hidden=hidden_transform(seed,&tr.held_out_input);
    let last=w.state.organisms[&ORGANISM_ID].last_return.clone();
    let published=w.audit.iter().filter(|e|
        matches!(&e.action,Some(Action::PublishTool{..})))
        .collect::<Vec<_>>();
    let publ_ok=published.iter().filter(|e|e.result.code==Code::Ok).count();
    let b=b_gas(&w);
    let reg_v=w.state.objects[REGISTRY].version;
    let old_retained=selected==old_hash;
    let expected_len=selected==len_hash;
    println!("S29_CROSSSEED_P2 seed={seed} root={root_hash} \
        actual_selected_tool={selected} selected_op={selected_op} \
        original_wrong_retained={old_retained} expected_len={expected_len} \
        registry_version={reg_v} publish_attempts={} publish_ok={publ_ok} \
        completed={last:?} p2_real_B={b} prediction={prediction} \
        hidden_oracle={hidden} hidden_matches={} \
        installed_bytes={} snapshot_replay=true",
        published.len(),prediction==hidden,installed.bytes.len());
    assert_eq!(last,Some(Value::U32(1)));
    assert_eq!(prediction,hidden,
        "real P2 held-out semantics must work on this predetermined seed");
    assert_eq!(b,20,"P2 service must have real correct B20 result");
    assert_eq!(w.state_hash(),World::from_snapshot(
        &w.snapshot_json()).unwrap().state_hash());
}


// Complementary seed-aware matched tapes. External actions are driven by
// fixed step offsets, NOT the policy's actual response times.
const LIFE_SECOND_PIN:&str="/self/pins/s29-lifetime-secondary";
const LIFE_SECOND_TASK:&str="/self/outcomes/s29-lifetime-second-artifact";

fn lifetime_extra_versions(w:&World)->Vec<String>{
    w.state.objects.iter().filter_map(|(k,o)|{
        let h=k.strip_prefix("/self/versions/")?;
        (o.owner==ORGANISM_ID&&h.len()==64).then_some(h.to_owned())
    }).collect()
}
fn lifetime_external_write(w:&mut World,key:&str,bytes:Vec<u8>,
    expected:Option<u64>,probe:&mut LifetimeIndexProbe)->ocee_world_kernel::ActionResult{
    let r=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:key.into(),bytes,expected_version:expected
    }));
    probe.sample(w);
    r
}
fn lifetime_has_transport(w:&World,target:&str)->bool{
    w.state.pending_deliveries.iter()
        .chain(w.state.mailboxes.values().flatten())
        .any(|m|m.bytes.windows(64).any(|b|b==target.as_bytes()))
}
fn lifetime_finish_remaining(w:&mut World,probe:&mut LifetimeIndexProbe,horizon:u64){
    while w.state.step<horizon && !w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().is_some_and(|c|c.finished){
        let r=w.tick_autonomous(ORGANISM_ID);
        probe.sample(w);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
    }
    assert!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().is_some_and(|c|c.finished));
    lifetime_pad_to(w,horizon,probe);
}
fn lifetime_due_task(w:&mut World,key:&str,probe:&mut LifetimeIndexProbe)
    ->(Code,u64){
    let r=lifetime_external_write(w,key,vec![0x55;280],None,probe);
    assert!(matches!(r.code,Code::Ok|Code::Quota),
        "frozen useful task must receive real acceptance or quota refusal");
    assert_eq!(w.state.objects.contains_key(key),r.code==Code::Ok);
    (r.code,r.gas_charged)
}
fn lifetime_prune_setup(w:&mut World,versions:&[String],registered:&str)
    ->(u64,u64,Vec<String>){
    let start=spend(w);
    let mut candidates=vec![wrong_tool().content_hash().unwrap()];
    candidates.extend(extra_unreferenced_tools().into_iter()
        .map(|p|p.content_hash().unwrap()));
    candidates.retain(|h|h!=registered);
    candidates.sort();
    candidates.dedup();
    assert!(candidates.len()>=2);
    for h in &candidates{
        let idx=versions.iter().position(|x|x==h).unwrap();
        alter(w,&format!("/self/pins/s29-dry-{idx}"),vec![b'-';64],0);
    }
    assert_eq!(index_refs(w).code,Code::Ok);
    let mut bytes=0u64;
    for h in &candidates{
        let len=w.state.objects[&version_key(h)].bytes.len() as u64;
        let r=fast_reclaim(w,h);
        assert_eq!(r.code,Code::Ok,
            "actual preregistered nonregistered candidate must be prunable: {h}");
        assert_eq!(u64::from_le_bytes(r.payload.as_slice().try_into().unwrap()),len);
        assert!(!w.state.objects.contains_key(&version_key(h)));
        bytes+=len;
    }
    let end=spend(w);
    assert_eq!(start.0,end.0);
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    // Natural epoch grant, never set a balance. Use the actual next epoch,
    // with one real tick at the boundary to receive its ordinary allocation.
    let epoch=w.state.config.epoch_length;
    let next=(w.state.step/epoch+1)*epoch;
    while w.state.step<=next{w.tick(None);}
    assert_eq!(w.state.nodes[&NodeId::A].gas_available,
        w.state.nodes[&NodeId::A].gas_per_epoch);
    assert_eq!(b_gas(w),20);
    (bytes,end.1-start.1,candidates)
}
fn lifetime_extra_run(w:&mut World,p:usize,tape:&str,
    first_pin:&str,target:&str,start:u64,probe:&mut LifetimeIndexProbe)
    ->(Vec<(Code,u64)>,u64,u64){
    let mut tasks=Vec::new();
    let mut exo_a=0u64;
    let mut exo_c=0u64;
    let mixed=tape=="MIXED_MULTIHOLDER_TRANSPORT";
    let target_key=version_key(target);
    lifetime_pad_to(w,start+90,probe);
    assert_eq!(w.state.objects[first_pin].bytes,target.as_bytes());
    let r=lifetime_external_write(w,first_pin,
        if mixed{vec![b'-';64]}else{target.as_bytes().to_vec()},
        Some(0),probe);
    assert_eq!(r.code,Code::Ok);exo_a+=r.gas_charged;
    if mixed{
        assert_eq!(w.state.step,start+91);
        assert_eq!(w.state.objects[LIFE_SECOND_PIN].bytes,target.as_bytes());
        let sent=w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,
            bytes:target.as_bytes().to_vec()
        }));
        probe.sample(w);
        assert_eq!(sent.code,Code::Ok);
        exo_a+=sent.gas_charged;exo_c+=sent.bandwidth_charged;
        assert_eq!(exo_c,64);
        assert_eq!(w.state.step,start+92);
        let r=lifetime_external_write(w,LIFE_SECOND_PIN,vec![b'-';64],Some(0),probe);
        assert_eq!(r.code,Code::Ok);exo_a+=r.gas_charged;
        assert!(w.state.pending_deliveries.iter().any(|m|m.bytes==target.as_bytes()));
        assert!(w.state.objects.contains_key(&target_key));
        if p==0{
            let mut certified=false;
            let mut conflicted=false;
            let mut marked=false;
            // Actual active VM must try the candidate during transport.
            while w.state.step<start+155{
                let r=w.tick_autonomous(ORGANISM_ID);probe.sample(w);
                assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::IndexVersionRefs{..})){
                    assert_eq!(r.code,Code::Ok);
                    let idx=w.state.version_ref_index.as_ref().unwrap();
                    assert!(idx.transport_refs.contains(target));
                    assert!(idx.referenced.contains(target));
                    certified=true;
                }
                if matches!(&e.action,Some(Action::ReclaimIndexedVersion{program_hash,..})
                    if program_hash==target){
                    assert_eq!(r.code,Code::Conflict);
                    assert!(w.state.pending_deliveries.iter()
                        .any(|m|m.bytes==target.as_bytes()));
                    conflicted=true;
                }
                if matches!(&e.action,Some(Action::Write{key,bytes,..})
                    if key==S29_DRY_KEY&&bytes==b"1"&&r.code==Code::Ok){
                    marked=true;break;
                }
            }
            assert!(certified&&conflicted&&marked,
                "real second-round VM index and candidate veto must occur before due task");
        }else if p==1{
            lifetime_advance_to_action(w,"COOLDOWN_RESET",60,probe);
        }
        assert!(w.state.objects.contains_key(&target_key));
        lifetime_pad_to(w,start+155,probe);
        assert!(lifetime_has_transport(w,target));
        let first=lifetime_due_task(w,S29_LIFE_TASK,probe);
        assert_eq!(first.0,Code::Quota);
        exo_a+=first.1;tasks.push(first);
        lifetime_pad_to(w,start+205,probe);
        assert!(!lifetime_has_transport(w,target),
            "frozen real transport must have cleared before third episode");
        assert_eq!(b_gas(w),20,"malformed raw hash must not cause another B charge");
        if p<2{
            lifetime_advance_to_action(w,"GC_SUCCESS",60,probe);
            assert!(!w.state.objects.contains_key(&target_key));
        }else{
            assert!(w.state.objects.contains_key(&target_key));
        }
        lifetime_pad_to(w,start+270,probe);
        let second=lifetime_due_task(w,LIFE_SECOND_TASK,probe);
        assert_eq!(second.0,if p<2{Code::Ok}else{Code::Quota});
        exo_a+=second.1;tasks.push(second);
        lifetime_finish_remaining(w,probe,start+335);
    }else{
        assert_eq!(tape,"NO_RECLAIMABLE_PRUNED");
        if p==0{
            lifetime_advance_to_action(w,"DRY_MARK",60,probe);
        }else if p==1{
            lifetime_advance_to_action(w,"COOLDOWN_RESET",60,probe);
        }
        lifetime_pad_to(w,start+155,probe);
        let result=lifetime_due_task(w,S29_LIFE_TASK,probe);
        assert_eq!(result.0,Code::Quota);
        exo_a+=result.1;tasks.push(result);
        lifetime_finish_remaining(w,probe,start+245);
    }
    assert_eq!(b_gas(w),20);
    (tasks,exo_a,exo_c)
}
#[test]
#[ignore="frozen complementary matched P2 lifetime tapes across fixed seeds"]
fn s29_lifetime_complement_native(){
    use sha2::{Digest,Sha256};
    let seed:u64=std::env::var("OCEE_S29_LIFE_SEED").unwrap().parse().unwrap();
    assert!([129,130,131].contains(&seed));
    let tape=std::env::var("OCEE_S29_EXTRA_TAPE").unwrap();
    assert!(matches!(tape.as_str(),"MIXED_MULTIHOLDER_TRANSPORT"|"NO_RECLAIMABLE_PRUNED"));
    let mixed=tape=="MIXED_MULTIHOLDER_TRANSPORT";
    let (mut base,original_versions,orphan,roots,registered)=lifetime_setup(seed);
    let (pruned_bytes,pruned_a,removed)=if mixed{
        put(&mut base,LIFE_SECOND_PIN.into(),orphan.as_bytes().to_vec());
        base.state.config.message_delay=95;
        let used=base.state.organisms[&ORGANISM_ID].storage_used;
        base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
        (0,0,Vec::new())
    }else{
        lifetime_prune_setup(&mut base,&original_versions,&registered)
    };
    let target=if mixed{orphan}else{registered.clone()};
    let pin_idx=original_versions.iter().position(|h|h==&target).unwrap();
    let first_pin=format!("/self/pins/s29-dry-{pin_idx}");
    let target_bytes=base.state.objects[&version_key(&target)].bytes.len() as u64;
    let installed=lifetime_extra_versions(&base);
    let base_hash=base.state_hash();
    let initial_used=base.state.organisms[&ORGANISM_ID].storage_used;
    let capacity=base.state.nodes[&NodeId::A].storage_capacity;
    assert_eq!(capacity-initial_used,195);
    let mut branch_a=None;
    let mut branch_start=None;
    let mut rows=Vec::new();
    for (p,policy) in ["ALWAYS_RETRY","NEGATIVE_COOLDOWN","NO_MAINTENANCE"]
        .iter().enumerate(){
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,&roots[p]);
        let start=w.state.step;
        let a=w.state.nodes[&NodeId::A].gas_available;
        if let Some(prev)=branch_a{assert_eq!(a,prev);}else{branch_a=Some(a);}
        if let Some(prev)=branch_start{assert_eq!(start,prev);}else{branch_start=Some(start);}
        if !mixed{
            // Actual full-index refusal for all *remaining* installed
            // versions, including entries outside the VM's first page.
            let mut verify=World::from_snapshot(&w.snapshot_json()).unwrap();
            assert_eq!(index_refs(&mut verify).code,Code::Ok);
            for h in &installed{
                assert_eq!(fast_reclaim(&mut verify,h).code,Code::Conflict,
                    "pruned-inventory control must have no reclaimable version");
            }
            assert_eq!(lifetime_extra_versions(&verify),installed);
        }
        let mut probe=LifetimeIndexProbe::default();
        probe.sample(&w);
        if p<2{
            lifetime_advance_to_action(&mut w,"DRY_MARK",80,&mut probe);
            assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));
        }else{
            lifetime_advance_to_action(&mut w,"NO_OP",10,&mut probe);
        }
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let mut replay_probe=probe.clone();
        let results=lifetime_extra_run(&mut w,p,&tape,&first_pin,&target,start,&mut probe);
        let replay_results=lifetime_extra_run(&mut replay,p,&tape,&first_pin,&target,start,&mut replay_probe);
        assert_eq!(results,replay_results);
        assert_eq!(probe,replay_probe);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());
        assert_eq!(w.audit,replay.audit);
        assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
        let (tasks,external_a,external_c)=results;
        let mut vm=0u64;let mut world=0u64;
        let mut gc=0u64;let mut indexes=0usize;let mut attempts=0usize;
        for e in w.audit.iter().filter(|e|e.step>=start){
            vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
            if matches!(&e.action,Some(Action::IndexVersionRefs{..})){indexes+=1;}
            if let Some(Action::ReclaimIndexedVersion{program_hash,..})=&e.action{
                attempts+=1;
                if e.result.code==Code::Ok{
                    assert!(mixed&&p<2&&program_hash==&target);
                    assert!(e.step>=start+205,"cannot reclaim during transport lease");
                    gc+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());
                }
            }
        }
        assert_eq!(gc,if mixed&&p<2{target_bytes}else{0});
        if p==2{assert_eq!(indexes,0);assert_eq!(attempts,0);}
        else{assert!(indexes>0);assert!(probe.post_tick_peak>0);}
        assert!(removed.iter().all(|h|!w.state.objects.contains_key(&version_key(h))));
        let maintenance=vm+world-external_a;
        println!("S29_COMPLEMENT_ROW seed={seed} tape={tape} policy={policy} \
            tasks={tasks:?} VM_A={vm} maintenance_action_A={} maintenance_total_A={maintenance} \
            external_A={external_a} external_C={external_c} gc_bytes={gc} index_count={indexes} \
            gc_attempts={attempts} storage_peak={} storage_final={} index_peak_unbilled={} \
            A_remaining={} replay=true B=20",
            world-external_a,probe.storage_peak,w.state.organisms[&ORGANISM_ID].storage_used,
            probe.post_tick_peak,w.state.nodes[&NodeId::A].gas_available);
        rows.push((maintenance,indexes));
    }
    if !mixed{
        assert!(rows[1].0<rows[0].0);
    }
    let schedule=serde_json::json!({
        "seed":seed,"tape":tape,"start":branch_start.unwrap(),
        "target":target,"first_pin":first_pin,"second_pin":LIFE_SECOND_PIN,
        "message_delay":base.state.config.message_delay,
        "event_offsets":if mixed{vec![90,91,92,155,205,270,335]}else{vec![90,155,245]},
        "task_bytes":280,"task_keys":[S29_LIFE_TASK,LIFE_SECOND_TASK],
        "release_bytes_hex":"2d repeated 64 times",
        "mixed_send_bytes":if mixed{Some(target.as_str())}else{None}
    });
    let tape_sha=format!("{:x}",Sha256::digest(serde_json::to_vec(&schedule).unwrap()));
    println!("S29_COMPLEMENT_DONE seed={seed} tape={tape} base={base_hash} tape_sha={tape_sha} \
        roots={roots:?} registered={registered} installed={} pruned_bytes={pruned_bytes} \
        pruned_setup_A={pruned_a} removed={removed:?} initial_used={initial_used} capacity={capacity} \
        start_A={} rows={rows:?}",installed.len(),branch_a.unwrap());
}


// Own-task quota feedback pilot: both policies actually attempt the same
// synthetic task, inherit the same paid dry-run memory, and differ only
// on whether a true Quota receipt can override that negative memory.
fn quota_task_root(override_cooldown:bool)->Program{
    let mut p=dry_run_cooldown_root();
    let f=&mut p.functions[0];
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    let task_ops=||vec![
        Instruction{id:10,op:lit(S29_LIFE_TASK.as_bytes())},
        Instruction{id:11,op:lit(&[0x55;280])},
        Instruction{id:12,op:Op::ConstU32(u32::MAX)}
    ];
    let code_ops=|expected:&[u8]|vec![
        Instruction{id:20,op:lit(b"code")},
        Instruction{id:21,op:Op::JsonFieldString(2,20)},
        Instruction{id:22,op:lit(expected)},
        Instruction{id:23,op:Op::EqBytes(21,22)}
    ];
    let reply=||vec![Param{id:2,ty:Type::Bytes}];
    entry.instructions=task_ops();
    entry.terminator=Terminator::Yield{
        call:WorldCall::Write,args:vec![10,11,12],
        next:"task_initial_reply".into(),next_args:vec![]
    };
    let round=f.blocks.iter_mut().find(|b|b.id=="round_header").unwrap();
    round.terminator=Terminator::RepeatBounded{
        limit:1,body:"probe_memory".into(),body_args:vec![],
        done:"failed".into(),done_args:vec![]
    };
    let success=f.blocks.iter_mut().find(|b|b.id=="succeeded").unwrap();
    success.instructions.clear();
    success.terminator=Terminator::Jump{
        target:"task_retry".into(),args:vec![]
    };
    if override_cooldown{
        let memory=f.blocks.iter_mut().find(|b|b.id=="probe_memory_exists").unwrap();
        if let Terminator::Branch{if_true,true_args,..}=&mut memory.terminator{
            assert_eq!(if_true,"cooldown_reset");
            *if_true="start_uncached".into();
            true_args.clear();
        }else{panic!("expected real negative memory decision");}
    }
    f.blocks.extend([
        Block{
            id:"task_initial_reply".into(),params:reply(),
            instructions:code_ops(b"Ok"),
            terminator:Terminator::Branch{
                condition:23,if_true:"task_ok".into(),true_args:vec![],
                if_false:"task_quota".into(),false_args:vec![2]
            }
        },
        Block{
            id:"task_quota".into(),params:reply(),
            instructions:code_ops(b"Quota"),
            terminator:Terminator::Branch{
                condition:23,if_true:"round_header".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"task_retry".into(),params:vec![],instructions:task_ops(),
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,12],
                next:"task_retry_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"task_retry_reply".into(),params:reply(),
            instructions:code_ops(b"Ok"),
            terminator:Terminator::Branch{
                condition:23,if_true:"task_ok".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        },
        Block{
            id:"task_ok".into(),params:vec![],
            instructions:vec![Instruction{id:1,op:Op::ConstU32(1)}],
            terminator:Terminator::Return{value:1}
        }
    ]);
    p.validate().expect("typed one-opportunity own task feedback policy");
    p
}
fn quota_wait_next_grant(w:&mut World){
    let epoch=w.state.config.epoch_length;
    let next=(w.state.step/epoch+1)*epoch;
    while w.state.step<=next{w.tick(None);}
    assert_eq!(w.state.nodes[&NodeId::A].gas_available,
        w.state.nodes[&NodeId::A].gas_per_epoch);
}
#[test]
#[ignore="frozen seed129 paired own-task quota override pilot"]
fn s29_task_quota_override_native(){
    let arm=std::env::var("OCEE_S29_TASK_ARM").unwrap();
    assert!(matches!(arm.as_str(),"RELEASED"|"PINNED"|"SPACE"));
    let (mut base,versions,target,previous_roots,registered)=lifetime_setup(129);
    // Preinstall both researcher-authored policy programs and both pins in
    // the same physical world. Capacity is a fixture constraint, not a VM gift.
    let before=spend(&base);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    let mut roots=Vec::new();
    for (i,enable) in [false,true].into_iter().enumerate(){
        let p=quota_task_root(enable);
        let raw=p.canonical_bytes();
        assert!(!raw.windows(64).any(|x|x==target.as_bytes()));
        let hash=p.content_hash().unwrap();
        put(&mut base,version_key(&hash),raw);
        put(&mut base,format!("/self/pins/s29-quota-root-{i}"),hash.as_bytes().to_vec());
        roots.push(hash);
    }
    assert_ne!(roots[0],roots[1]);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut base);
    lifetime_set_active(&mut base,&previous_roots[1]);
    let dry_start=base.state.step;
    let mut dry_probe=LifetimeIndexProbe::default();
    lifetime_advance_to_action(&mut base,"DRY_MARK",80,&mut dry_probe);
    assert_eq!(paid_memory_value(&base),(b"1".to_vec(),0));
    assert!(base.audit.iter().any(|e|e.step>=dry_start &&
        matches!(&e.action,Some(Action::IndexVersionRefs{..}))&&e.result.code==Code::Ok));
    assert!(base.audit.iter().any(|e|e.step>=dry_start &&
        matches!(&e.action,Some(Action::ReclaimIndexedVersion{..}))&&e.result.code==Code::Conflict));
    assert_eq!(lifetime_extra_versions(&base).len(),versions.len()+2);
    let target_idx=versions.iter().position(|x|x==&target).unwrap();
    let target_len=base.state.objects[&version_key(&target)].bytes.len() as u64;
    let pin=format!("/self/pins/s29-dry-{target_idx}");
    alter(&mut base,&pin,if arm=="RELEASED"{vec![b'-';64]}else{target.as_bytes().to_vec()},0);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    let free=if arm=="SPACE"{512}else{195};
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+free;
    quota_wait_next_grant(&mut base);
    let end=spend(&base);
    let base_hash=base.state_hash();
    let original=lifetime_extra_versions(&base);
    let mut start_a=None;
    let mut common_start=None;
    for (i,root) in roots.iter().enumerate(){
        let label=if i==0{"FIXED_COOLDOWN"}else{"QUOTA_OVERRIDE"};
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,root);
        let start=w.state.step;
        let a=w.state.nodes[&NodeId::A].gas_available;
        if let Some(prev)=start_a{assert_eq!(a,prev);}else{start_a=Some(a);}
        if let Some(prev)=common_start{assert_eq!(start,prev);}else{common_start=Some(start);}
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let mut peak=LifetimeIndexProbe::default();
        peak.sample(&w);
        let mut replay_peak=peak.clone();
        lifetime_finish_remaining(&mut w,&mut peak,start+90);
        lifetime_finish_remaining(&mut replay,&mut replay_peak,start+90);
        assert_eq!(peak,replay_peak);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());
        assert_eq!(w.audit,replay.audit);
        let success=arm=="SPACE"||(arm=="RELEASED"&&i==1);
        let mut tasks=Vec::new();
        let mut vm=0u64;let mut action=0u64;let mut task_a=0u64;
        let mut gc=0u64;let mut indexes=0usize;let mut conflicts=0usize;
        let mut successful_gc_step=None;
        for e in w.audit.iter().filter(|e|e.step>=start){
            vm+=e.result.vm_gas_charged;action+=e.result.gas_charged;
            if let Some(Action::Write{key,bytes,..})=&e.action{
                if key==S29_LIFE_TASK{
                    assert_eq!(bytes,&vec![0x55;280]);
                    tasks.push((e.step,e.result.code,e.result.gas_charged));
                    task_a+=e.result.gas_charged;
                }
            }
            if matches!(&e.action,Some(Action::IndexVersionRefs{..})){
                assert_eq!(e.result.code,Code::Ok);indexes+=1;
            }
            if let Some(Action::ReclaimIndexedVersion{program_hash,..})=&e.action{
                if e.result.code==Code::Ok{
                    assert_eq!(program_hash,&target);
                    gc+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());
                    successful_gc_step=Some(e.step);
                }else if e.result.code==Code::Conflict{conflicts+=1;}
            }
        }
        assert_eq!(tasks.len(),if arm=="RELEASED"&&i==1{2}else{1});
        assert_eq!(tasks[0].1,if arm=="SPACE"{Code::Ok}else{Code::Quota});
        if tasks.len()==2{
            assert_eq!(tasks[1].1,Code::Ok);
            assert!(tasks[0].0<successful_gc_step.unwrap()&&successful_gc_step.unwrap()<tasks[1].0);
        }
        assert_eq!(indexes,if i==1&&arm!="SPACE"{1}else{0});
        assert_eq!(gc,if arm=="RELEASED"&&i==1{target_len}else{0});
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(if success{1}else{0})));
        assert_eq!(w.state.objects.contains_key(S29_LIFE_TASK),success);
        if success{assert_eq!(w.state.objects[S29_LIFE_TASK].bytes,vec![0x55;280]);}
        for h in &original{
            assert_eq!(w.state.objects.contains_key(&version_key(h)),
                !(gc>0&&h==&target));
        }
        assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
        let memory=paid_memory_value(&w);
        if arm=="SPACE"{assert_eq!(memory,(b"1".to_vec(),0));}
        else if i==0{assert_eq!(memory,(b"0".to_vec(),1));}
        else if arm=="PINNED"{assert_eq!(memory,(b"1".to_vec(),1));}
        else{assert_eq!(memory,(b"1".to_vec(),0));}
        assert_eq!(b_gas(&w),20);
        println!("S29_QUOTA_ROW arm={arm} policy={label} root={root} start={start} tasks={tasks:?} success={success} VM_A={vm} world_A={action} task_A={task_a} non_task_action_A={} non_task_total_A={} indexes={indexes} conflicts={conflicts} GC_B={gc} memory={memory:?} storage_peak={} index_peak_unbilled={} replay=true B=20",
            action-task_a,vm+action-task_a,peak.storage_peak,peak.post_tick_peak);
    }
    println!("S29_QUOTA_DONE arm={arm} seed=129 base={base_hash} target={target} target_B={target_len} free={free} start_A={} setup_VM_A={} setup_world_A={} roots={roots:?}",
        start_a.unwrap(),end.0-before.0,end.1-before.1);
}


// Earned binary history causal transfer: training branches get their memory
// from real task/GC receipts, not researcher writes. The probe's task path is
// distinct, and physical object maps are normalized by actual paid actions.
const S29_HISTORY_PROBE_TASK:&str="/self/outcomes/s29-history-probe";
fn history_training_root()->Program{
    let mut p=quota_task_root(true);
    let f=&mut p.functions[0];
    let reply=f.blocks.iter_mut().find(|b|b.id=="task_retry_reply").unwrap();
    if let Terminator::Branch{if_true,..}=&mut reply.terminator{
        assert_eq!(if_true,"task_ok");
        *if_true="record_recovery".into();
    }else{panic!("expected actual successful task receipt");}
    f.blocks.extend([
        Block{
            id:"record_recovery".into(),params:vec![],
            instructions:vec![
                Instruction{id:10,op:lit(S29_DRY_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"0")},
                Instruction{id:12,op:Op::ConstU32(u32::MAX)}
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Write,args:vec![10,11,12],
                next:"record_recovery_reply".into(),next_args:vec![]
            }
        },
        Block{
            id:"record_recovery_reply".into(),
            params:vec![Param{id:2,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:20,op:lit(b"code")},
                Instruction{id:21,op:Op::JsonFieldString(2,20)},
                Instruction{id:22,op:lit(b"Ok")},
                Instruction{id:23,op:Op::EqBytes(21,22)}
            ],
            terminator:Terminator::Branch{
                condition:23,if_true:"task_ok".into(),true_args:vec![],
                if_false:"failed".into(),false_args:vec![]
            }
        }
    ]);
    p.validate().expect("real retry success can create paid positive history");
    p
}
fn history_probe_root(blind:bool)->Program{
    let mut p=quota_task_root(blind);
    for id in ["entry","task_retry"]{
        let b=p.functions[0].blocks.iter_mut().find(|b|b.id==id).unwrap();
        assert_eq!(b.instructions[0].id,10);
        b.instructions[0].op=lit(S29_HISTORY_PROBE_TASK.as_bytes());
    }
    p.validate().unwrap();
    p
}
#[derive(Debug,PartialEq,Eq)]
struct HistoryReceiptMetrics{
    tasks:Vec<(u64,Code,u64)>,
    vm:u64,world:u64,task_a:u64,
    gc:u64,indexes:usize,conflicts:usize,
    memory_writes:Vec<(u64,Vec<u8>,Code,u64)>,
}
fn history_measure(w:&World,start:u64,task:&str,target:&str)->HistoryReceiptMetrics{
    let mut m=HistoryReceiptMetrics{
        tasks:vec![],vm:0,world:0,task_a:0,gc:0,indexes:0,conflicts:0,
        memory_writes:vec![]
    };
    for e in w.audit.iter().filter(|e|e.step>=start){
        m.vm+=e.result.vm_gas_charged;
        m.world+=e.result.gas_charged;
        if let Some(Action::Write{key,bytes,..})=&e.action{
            if key==task{
                assert_eq!(bytes,&vec![0x55;280]);
                m.tasks.push((e.step,e.result.code,e.result.gas_charged));
                m.task_a+=e.result.gas_charged;
            }
            if key==S29_DRY_KEY{
                m.memory_writes.push((e.step,bytes.clone(),e.result.code,e.result.gas_charged));
            }
        }
        if matches!(&e.action,Some(Action::IndexVersionRefs{..})){
            assert_eq!(e.result.code,Code::Ok);m.indexes+=1;
        }
        if let Some(Action::ReclaimIndexedVersion{program_hash,..})=&e.action{
            if e.result.code==Code::Ok{
                assert_eq!(program_hash,target);
                m.gc+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());
            }else if e.result.code==Code::Conflict{m.conflicts+=1;}
        }
    }
    m
}
fn history_replay_episode(w:&mut World,task:&str,target:&str)
    ->(HistoryReceiptMetrics,LifetimeIndexProbe){
    let start=w.state.step;
    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
    let mut peak=LifetimeIndexProbe::default();
    peak.sample(w);
    let mut replay_peak=peak.clone();
    lifetime_finish_remaining(w,&mut peak,start+90);
    lifetime_finish_remaining(&mut replay,&mut replay_peak,start+90);
    assert_eq!(w.snapshot_json(),replay.snapshot_json());
    assert_eq!(w.audit,replay.audit);
    assert_eq!(peak,replay_peak);
    assert_eq!(b_gas(w),20);
    (history_measure(w,start,task,target),peak)
}
fn history_same_physical(a:&World,b:&World){
    assert_eq!(a.state.step,b.state.step);
    assert_eq!(a.state.nodes,b.state.nodes);
    assert_eq!(a.state.pending_deliveries,b.state.pending_deliveries);
    assert_eq!(a.state.mailboxes,b.state.mailboxes);
    assert_eq!(a.state.organisms[&ORGANISM_ID].storage_used,
        b.state.organisms[&ORGANISM_ID].storage_used);
    assert_eq!(lifetime_extra_versions(a),lifetime_extra_versions(b));
    let mut ao=a.state.objects.clone();
    let mut bo=b.state.objects.clone();
    let am=ao.remove(S29_DRY_KEY).unwrap();
    let bm=bo.remove(S29_DRY_KEY).unwrap();
    assert_eq!(am.owner,bm.owner);
    assert_eq!(am.version,bm.version);
    assert_eq!(am.bytes.len(),bm.bytes.len());
    assert_ne!(am.bytes,bm.bytes);
    assert_eq!(ao,bo,"all other actual object bytes and versions must match");
    assert_eq!(a.state.version_ref_index,b.state.version_ref_index);
}
#[test]
#[ignore="frozen earned task repair history causal probe at matched physical state"]
fn s29_earned_task_history_native(){
    let arm=std::env::var("OCEE_S29_HISTORY_ARM").unwrap();
    assert!(matches!(arm.as_str(),"RELEASED"|"PINNED"));
    let (mut base,versions,target,_,registered)=lifetime_setup(129);
    assert!(!base.state.objects.contains_key(S29_DRY_KEY));
    assert!(!base.state.objects.contains_key(S29_LIFE_TASK));
    assert!(!base.state.objects.contains_key(S29_HISTORY_PROBE_TASK));
    let initial_spend=spend(&base);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    let mut roots=Vec::new();
    for (i,p) in [history_training_root(),history_probe_root(false),
        history_probe_root(true)].into_iter().enumerate(){
        let raw=p.canonical_bytes();
        assert!(!raw.windows(64).any(|x|x==target.as_bytes()));
        let h=p.content_hash().unwrap();
        put(&mut base,version_key(&h),raw);
        put(&mut base,format!("/self/pins/s29-history-root-{i}"),h.as_bytes().to_vec());
        roots.push(h);
    }
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut base);
    let install_spend=spend(&base);
    let base_hash=base.state_hash();
    let pin_idx=versions.iter().position(|x|x==&target).unwrap();
    let pin=format!("/self/pins/s29-dry-{pin_idx}");
    let target_bytes=base.state.objects[&version_key(&target)].bytes.clone();
    let target_len=target_bytes.len() as u64;
    let installed=lifetime_extra_versions(&base);
    let mut histories=Vec::new();
    let mut training_start=None;
    for good in [true,false]{
        let label=if good{"SUCCESS"}else{"FAILURE"};
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        alter(&mut w,&pin,if good{vec![b'-';64]}else{target.as_bytes().to_vec()},0);
        lifetime_set_active(&mut w,&roots[0]);
        if let Some(prev)=training_start{
            assert_eq!(w.state.step,prev);
        }else{training_start=Some(w.state.step);}
        let (m,peak)=history_replay_episode(&mut w,S29_LIFE_TASK,&target);
        assert_eq!(m.tasks[0].1,Code::Quota);
        assert_eq!(m.tasks.len(),if good{2}else{1});
        assert_eq!(m.gc,if good{target_len}else{0});
        assert_eq!(m.indexes,1);
        assert_eq!(m.memory_writes.len(),1);
        assert_eq!(m.memory_writes[0].2,Code::Ok);
        assert!(m.memory_writes[0].3>0);
        if good{
            assert_eq!(m.tasks[1].1,Code::Ok);
            assert!(m.memory_writes[0].0>m.tasks[1].0);
        }
        assert_eq!(paid_memory_value(&w),(if good{b"0".to_vec()}else{b"1".to_vec()},0));
        assert_eq!(w.state.objects.contains_key(S29_LIFE_TASK),good);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(if good{1}else{0})));
        for h in &installed{
            assert_eq!(w.state.objects.contains_key(&version_key(h)),
                !(good&&h==&target));
        }
        let pre=spend(&w);
        let before_hash=w.state_hash();
        // Paid normalization of actual objects, never the memory.
        if good{
            alter(&mut w,S29_LIFE_TASK,vec![],0);
            put(&mut w,version_key(&target),target_bytes.clone());
        }else{
            put(&mut w,S29_LIFE_TASK.into(),vec![]);
            alter(&mut w,S29_LIFE_TASK,vec![],0);
        }
        alter(&mut w,&pin,if arm=="RELEASED"{vec![b'-';64]}else{target.as_bytes().to_vec()},1);
        assert_eq!(w.state.objects[S29_LIFE_TASK].version,1);
        assert_eq!(w.state.objects[S29_LIFE_TASK].bytes,Vec::<u8>::new());
        assert_eq!(w.state.objects[&version_key(&target)].bytes,target_bytes);
        assert_eq!(lifetime_extra_versions(&w),installed);
        let used=w.state.organisms[&ORGANISM_ID].storage_used;
        w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
        quota_wait_next_grant(&mut w);
        let post=spend(&w);
        assert_eq!(paid_memory_value(&w),(if good{b"0".to_vec()}else{b"1".to_vec()},0));
        println!("S29_HISTORY_TRAIN arm={arm} history={label} metrics={m:?} memory={:?} training_end={before_hash} normalized={} normalize_VM_A={} normalize_world_A={} index_peak_unbilled={} storage_peak={} replay=true",
            paid_memory_value(&w),w.state_hash(),post.0-pre.0,post.1-pre.1,
            peak.post_tick_peak,peak.storage_peak);
        histories.push(w);
    }
    // Training spent different resources, and left a different real memory;
    // the probe begins at the same current physical condition.
    history_same_physical(&histories[0],&histories[1]);
    for (policy,root) in [("CONDITIONED",&roots[1]),("BLIND",&roots[2])]{
        let mut positive=World::from_snapshot(&histories[0].snapshot_json()).unwrap();
        let mut negative=World::from_snapshot(&histories[1].snapshot_json()).unwrap();
        lifetime_set_active(&mut positive,root);
        lifetime_set_active(&mut negative,root);
        history_same_physical(&positive,&negative);
        let start=positive.state.step;
        for (good,w) in [(true,&mut positive),(false,&mut negative)]{
            let history=if good{"SUCCESS"}else{"FAILURE"};
            assert_eq!(w.state.step,start);
            let before_hash=w.state_hash();
            let (m,peak)=history_replay_episode(w,S29_HISTORY_PROBE_TASK,&target);
            let scans=good||policy=="BLIND";
            let success=arm=="RELEASED"&&scans;
            assert_eq!(m.tasks[0].1,Code::Quota);
            assert_eq!(m.tasks.len(),if success{2}else{1});
            if success{assert_eq!(m.tasks[1].1,Code::Ok);}
            assert_eq!(m.indexes,usize::from(scans));
            assert_eq!(m.gc,if success{target_len}else{0});
            assert_eq!(w.state.objects.contains_key(S29_HISTORY_PROBE_TASK),success);
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(if success{1}else{0})));
            assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
            for h in &installed{
                assert_eq!(w.state.objects.contains_key(&version_key(h)),
                    !(success&&h==&target));
            }
            println!("S29_HISTORY_PROBE arm={arm} policy={policy} history={history} root={root} base={before_hash} start={start} success={success} metrics={m:?} non_task_A={} memory={:?} index_peak_unbilled={} storage_peak={} replay=true B=20",
                m.vm+m.world-m.task_a,paid_memory_value(w),peak.post_tick_peak,peak.storage_peak);
        }
    }
    println!("S29_HISTORY_DONE arm={arm} seed=129 base={base_hash} target={target} target_B={target_len} roots={roots:?} install_VM_A={} install_world_A={} normalized_objects_match_except_earned_memory=true",
        install_spend.0-initial_spend.0,install_spend.1-initial_spend.1);
}


// Recovery revision does not reinterpret the already committed task as a
// failure if the separately metered history update loses its CAS.
fn recovery_revision_root()->Program{
    let mut p=quota_task_root(true);
    let f=&mut p.functions[0];
    let retry=f.blocks.iter_mut().find(|b|b.id=="task_retry_reply").unwrap();
    if let Terminator::Branch{if_true,..}=&mut retry.terminator{
        assert_eq!(if_true,"task_ok");*if_true="revision_read".into();
    }else{panic!("must follow real successful retry");}
    let reply=||vec![Param{id:2,ty:Type::Bytes}];
    f.blocks.extend([
        Block{id:"revision_read".into(),params:vec![],
            instructions:vec![Instruction{id:10,op:lit(S29_DRY_META.as_bytes())}],
            terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],
                next:"revision_read_reply".into(),next_args:vec![]}},
        Block{id:"revision_read_reply".into(),params:reply(),
            instructions:vec![
                Instruction{id:20,op:lit(b"code")},
                Instruction{id:21,op:Op::JsonFieldString(2,20)},
                Instruction{id:22,op:lit(b"Ok")},
                Instruction{id:23,op:Op::EqBytes(21,22)}
            ],
            terminator:Terminator::Branch{condition:23,if_true:"revision_parse".into(),
                true_args:vec![2],if_false:"task_ok".into(),false_args:vec![]}},
        Block{id:"revision_parse".into(),params:reply(),instructions:vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:lit(b"bytes")},
            Instruction{id:13,op:Op::JsonFieldBytes(11,12)},
            Instruction{id:14,op:lit(b"version")},
            Instruction{id:15,op:Op::JsonFieldU32(11,14)},
            Instruction{id:16,op:lit(b"1")},
            Instruction{id:17,op:Op::EqBytes(13,16)}
        ],terminator:Terminator::Branch{condition:17,if_true:"revision_cas".into(),
            true_args:vec![15],if_false:"revision_zero".into(),false_args:vec![13,15]}},
        Block{id:"revision_zero".into(),params:vec![
            Param{id:1,ty:Type::Bytes},Param{id:2,ty:Type::U32}],
            instructions:vec![Instruction{id:10,op:lit(b"0")},
                Instruction{id:11,op:Op::EqBytes(1,10)}],
            terminator:Terminator::Branch{condition:11,if_true:"revision_cas".into(),
                true_args:vec![2],if_false:"task_ok".into(),false_args:vec![]}},
        Block{id:"revision_cas".into(),params:vec![Param{id:1,ty:Type::U32}],
            instructions:vec![
                Instruction{id:10,op:lit(S29_DRY_KEY.as_bytes())},
                Instruction{id:11,op:lit(b"0")}],
            terminator:Terminator::Yield{call:WorldCall::Write,args:vec![10,11,1],
                next:"revision_cas_reply".into(),next_args:vec![]}},
        Block{id:"revision_cas_reply".into(),params:reply(),instructions:vec![],
            terminator:Terminator::Jump{target:"task_ok".into(),args:vec![]}}
    ]);
    p.validate().expect("bounded typed positive recovery CAS");
    p
}
fn recovery_revision_episode(w:&mut World,race:bool,target:&str)
    ->(HistoryReceiptMetrics,LifetimeIndexProbe,Option<(u64,u64)>){
    let start=w.state.step;
    let horizon=start+90;
    let mut peak=LifetimeIndexProbe::default();
    peak.sample(w);
    let mut exogenous=None;
    while w.state.step<horizon && !w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().is_some_and(|c|c.finished){
        let r=w.tick_autonomous(ORGANISM_ID);peak.sample(w);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if race && exogenous.is_none() && w.state.objects.contains_key(S29_LIFE_TASK)
            && matches!(&w.audit.last().unwrap().action,
                Some(Action::Read{key,..}) if key==S29_DRY_META)
        {
            assert_eq!(r.code,Code::Ok);
            assert_eq!(paid_memory_value(w),(b"1".to_vec(),0));
            let step=w.state.step;
            let r=lifetime_external_write(w,S29_DRY_KEY,b"1".to_vec(),Some(0),&mut peak);
            assert_eq!(r.code,Code::Ok);
            assert!(r.gas_charged>0);
            exogenous=Some((step,r.gas_charged));
        }
    }
    assert!(w.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().is_some_and(|c|c.finished));
    lifetime_pad_to(w,horizon,&mut peak);
    assert_eq!(exogenous.is_some(),race);
    assert_eq!(b_gas(w),20);
    (history_measure(w,start,S29_LIFE_TASK,target),peak,exogenous)
}
#[test]
#[ignore="frozen actual recovery memory revision and stale CAS"]
fn s29_recovery_memory_revision_native(){
    let arm=std::env::var("OCEE_S29_REVISION_ARM").unwrap();
    assert!(matches!(arm.as_str(),"RELEASED"|"PINNED"|"STALE_CAS"));
    let (mut base,versions,target,old_roots,registered)=lifetime_setup(129);
    let before=spend(&base);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    let mut roots=Vec::new();
    for (i,p) in [quota_task_root(true),recovery_revision_root()].into_iter().enumerate(){
        let raw=p.canonical_bytes();
        assert!(!raw.windows(64).any(|x|x==target.as_bytes()));
        let h=p.content_hash().unwrap();
        put(&mut base,version_key(&h),raw);
        put(&mut base,format!("/self/pins/s29-revision-root-{i}"),h.as_bytes().to_vec());
        roots.push(h);
    }
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut base);
    lifetime_set_active(&mut base,&old_roots[1]);
    let dry_start=base.state.step;
    let mut peak=LifetimeIndexProbe::default();
    lifetime_advance_to_action(&mut base,"DRY_MARK",80,&mut peak);
    assert_eq!(paid_memory_value(&base),(b"1".to_vec(),0));
    assert!(base.audit.iter().any(|e|e.step>=dry_start&&
        matches!(&e.action,Some(Action::IndexVersionRefs{..}))&&e.result.code==Code::Ok));
    assert!(base.audit.iter().any(|e|e.step>=dry_start&&
        matches!(&e.action,Some(Action::ReclaimIndexedVersion{..}))&&e.result.code==Code::Conflict));
    let idx=versions.iter().position(|h|h==&target).unwrap();
    let target_len=base.state.objects[&version_key(&target)].bytes.len() as u64;
    let pin=format!("/self/pins/s29-dry-{idx}");
    let released=arm!="PINNED";
    alter(&mut base,&pin,if released{vec![b'-';64]}else{target.as_bytes().to_vec()},0);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut base);
    let base_hash=base.state_hash();
    let after=spend(&base);
    let installed=lifetime_extra_versions(&base);
    let configs=if arm=="STALE_CAS"{vec![(1,false),(1,true)]}
        else{vec![(0,false),(1,false)]};
    let mut start_a=None;
    let mut start_step=None;
    for (i,race) in configs{
        let root=&roots[i];
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,root);
        let start=w.state.step;
        let a=w.state.nodes[&NodeId::A].gas_available;
        if let Some(prev)=start_a{assert_eq!(a,prev);}else{start_a=Some(a);}
        if let Some(prev)=start_step{assert_eq!(start,prev);}else{start_step=Some(start);}
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let (m,peak,exo)=recovery_revision_episode(&mut w,race,&target);
        let (rm,rp,re)=recovery_revision_episode(&mut replay,race,&target);
        assert_eq!(m,rm);assert_eq!(peak,rp);assert_eq!(exo,re);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        assert_eq!(m.tasks[0].1,Code::Quota);
        assert_eq!(m.tasks.len(),if released{2}else{1});
        if released{assert_eq!(m.tasks[1].1,Code::Ok);}
        assert_eq!(m.gc,if released{target_len}else{0});assert_eq!(m.indexes,1);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(if released{1}else{0})));
        assert_eq!(w.state.objects.contains_key(S29_LIFE_TASK),released);
        for h in &installed{
            assert_eq!(w.state.objects.contains_key(&version_key(h)),!(released&&h==&target));
        }
        assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
        let writes=m.memory_writes.iter().filter(|x|exo.map_or(true,|(step,_)|x.0!=step))
            .collect::<Vec<_>>();
        if !released{
            assert_eq!(writes.len(),1);assert_eq!(writes[0].1,b"1");
            assert_eq!(writes[0].2,Code::Ok);
            assert_eq!(paid_memory_value(&w),(b"1".to_vec(),1));
        }else if i==0{
            assert!(writes.is_empty());
            assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));
        }else{
            assert_eq!(writes.len(),1);
            assert!(writes[0].0>m.tasks[1].0);
            assert_eq!(writes[0].1,b"0");
            if race{
                assert_eq!(writes[0].2,Code::Conflict);
                assert_eq!(paid_memory_value(&w),(b"1".to_vec(),1));
            }else{
                assert_eq!(writes[0].2,Code::Ok);
                assert!(writes[0].3>0);
                assert_eq!(paid_memory_value(&w),(b"0".to_vec(),1));
            }
        }
        let exo_a=exo.map_or(0,|(_,fee)|fee);
        println!("S29_REVISION_ROW arm={arm} policy={} race={race} root={root} start={start} metrics={m:?} exogenous={exo:?} non_task_A={} final_memory={:?} index_peak_unbilled={} storage_peak={} replay=true B=20",
            if i==0{"STALE"}else{"REVISE"},m.vm+m.world-m.task_a-exo_a,paid_memory_value(&w),
            peak.post_tick_peak,peak.storage_peak);
    }
    println!("S29_REVISION_DONE arm={arm} seed=129 base={base_hash} roots={roots:?} target={target} target_B={target_len} start_A={} setup_VM_A={} setup_world_A={}",
        start_a.unwrap(),after.0-before.0,after.1-before.1);
}


// One continuous bounded VM, no researcher-selected task or root after
// initial activation. 1 = failed/pending skip, 2 = failed/skip consumed,
// 0 = actual successful recovery. Distinguish outcome from retry eligibility.
const CONT_TASK_PREFIX:&str="/self/outcomes/s29-continuous-";
fn continuous_task_root()->Program{
    let mut p=recovery_revision_root();
    let f=&mut p.functions[0];
    let memory=f.blocks.iter_mut().find(|b|b.id=="probe_memory_exists").unwrap();
    if let Terminator::Branch{if_true,true_args,..}=&mut memory.terminator{
        assert_eq!(if_true,"start_uncached");
        *if_true="cooldown_reset".into();*true_args=vec![15];
    }else{panic!("expected history branch");}
    let reset=f.blocks.iter_mut().find(|b|b.id=="cooldown_reset").unwrap();
    reset.instructions[1].op=lit(b"2");
    let zero=f.blocks.iter_mut().find(|b|b.id=="probe_memory_zero").unwrap();
    if let Terminator::Branch{if_false,false_args,..}=&mut zero.terminator{
        *if_false="probe_memory_cooled".into();*false_args=vec![2];
    }else{panic!("expected valid history check");}
    f.blocks.push(Block{id:"probe_memory_cooled".into(),
        params:vec![Param{id:2,ty:Type::Bytes}],
        instructions:vec![Instruction{id:10,op:lit(b"2")},
            Instruction{id:11,op:Op::EqBytes(2,10)}],
        terminator:Terminator::Branch{condition:11,if_true:"start_uncached".into(),
            true_args:vec![],if_false:"failed".into(),false_args:vec![]}});
    let revise=f.blocks.iter_mut().find(|b|b.id=="revision_zero").unwrap();
    // Existing 0 is already a real success. Only 1 or 2 need revision.
    revise.instructions[0].op=lit(b"2");
    let round=f.blocks.iter_mut().find(|b|b.id=="round_header").unwrap();
    round.terminator=Terminator::Jump{target:"probe_memory".into(),args:vec![]};
    let after=f.blocks.iter_mut().find(|b|b.id=="after_observe").unwrap();
    after.terminator=Terminator::Jump{target:"episode_end".into(),args:vec![]};
    for id in ["failed","task_ok"]{
        let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
        b.instructions.clear();
        b.terminator=Terminator::Jump{target:"episode_end".into(),args:vec![]};
    }
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    entry.id="episode_task".into();entry.params.clear();
    for id in ["episode_task","task_retry"]{
        let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
        b.instructions=vec![
            Instruction{id:10,op:lit(CONT_TASK_PREFIX.as_bytes())},
            Instruction{id:13,op:Op::ConstU32(48)},
            Instruction{id:14,op:Op::Add(99,13)},
            Instruction{id:15,op:Op::PackByte(14)},
            Instruction{id:16,op:Op::Concat(10,15)},
            Instruction{id:11,op:lit(&[0x55;280])},
            Instruction{id:12,op:Op::ConstU32(u32::MAX)}
        ];
        if let Terminator::Yield{args,..}=&mut b.terminator{*args=vec![16,11,12];}
        else{panic!("task must remain a real world Write");}
    }
    // Thread a task-slot register through every existing block and edge.
    // The verifier requires feedback Bytes last, so insert before it.
    let yield_targets=f.blocks.iter().filter_map(|b|match &b.terminator{
        Terminator::Yield{next,..}=>Some(next.clone()),_=>None
    }).collect::<std::collections::BTreeSet<_>>();
    for b in &mut f.blocks{
        let pos=if yield_targets.contains(&b.id){b.params.len()-1}else{b.params.len()};
        b.params.insert(pos,Param{id:99,ty:Type::U32});
        match &mut b.terminator{
            Terminator::Jump{args,..}=>args.push(99),
            Terminator::Branch{true_args,false_args,..}=>{
                true_args.push(99);false_args.push(99);
            },
            Terminator::RepeatBounded{body_args,done_args,..} |
            Terminator::RepeatScoped{body_args,done_args,..}=>{
                body_args.push(99);done_args.push(99);
            },
            Terminator::Yield{next_args,..}=>next_args.push(99),
            Terminator::Return{..}=>panic!("all episode returns must join next task")
        }
    }
    let slot=||vec![Param{id:99,ty:Type::U32}];
    f.blocks.insert(0,Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
        instructions:vec![Instruction{id:99,op:Op::ConstU32(0)}],
        terminator:Terminator::Jump{target:"episode_header".into(),args:vec![99]}});
    f.blocks.extend([
        Block{id:"episode_header".into(),params:slot(),instructions:vec![],
            terminator:Terminator::RepeatBounded{limit:3,body:"episode_task".into(),
                body_args:vec![99],done:"episode_return".into(),done_args:vec![99]}},
        Block{id:"episode_end".into(),params:slot(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(2)},
            Instruction{id:11,op:Op::EqU32(99,10)}
        ],terminator:Terminator::Branch{condition:11,if_true:"episode_next".into(),
            true_args:vec![99],if_false:"episode_delay".into(),false_args:vec![99]}},
        Block{id:"episode_delay".into(),params:slot(),instructions:vec![],
            terminator:Terminator::RepeatScoped{limit:40,aggregate_limit:80,scope:99,
                body:"episode_observe".into(),body_args:vec![99],
                done:"episode_next".into(),done_args:vec![99]}},
        Block{id:"episode_observe".into(),params:slot(),instructions:vec![],
            terminator:Terminator::Yield{call:WorldCall::Observe,args:vec![],
                next:"episode_observe_reply".into(),next_args:vec![99]}},
        Block{id:"episode_observe_reply".into(),params:vec![
            Param{id:99,ty:Type::U32},Param{id:2,ty:Type::Bytes}],
            instructions:vec![],terminator:Terminator::Jump{
                target:"episode_delay".into(),args:vec![99]}},
        Block{id:"episode_next".into(),params:slot(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(1)},
            Instruction{id:11,op:Op::Add(99,10)}
        ],terminator:Terminator::Jump{target:"episode_header".into(),args:vec![11]}},
        Block{id:"episode_return".into(),params:slot(),instructions:vec![],
            terminator:Terminator::Return{value:99}}
    ]);
    p.validate().expect("single bounded three-task typed continuation");
    p
}
fn continuous_task_episode(w:&mut World,root:&str,pin:&str,target:&str,release:bool)
    ->(LifetimeIndexProbe,u64){
    let start=w.state.step;
    let mut peak=LifetimeIndexProbe::default();
    peak.sample(w);
    let mut external=None;
    while w.state.step<start+190{
        if w.state.step==start+25{
            assert_eq!(paid_memory_value(w),(b"1".to_vec(),0));
            assert!(!w.state.objects.contains_key(&format!("{CONT_TASK_PREFIX}1")));
            let r=lifetime_external_write(w,pin,
                if release{vec![b'-';64]}else{target.as_bytes().to_vec()},Some(0),&mut peak);
            assert_eq!(r.code,Code::Ok);
            external=Some(r.gas_charged);
        }else if w.state.organisms[&ORGANISM_ID]
            .vm_continuation.as_ref().is_some_and(|c|c.finished){
            w.tick(None);peak.sample(w);
        }else{
            let r=w.tick_autonomous(ORGANISM_ID);peak.sample(w);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        }
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(root));
    }
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
    assert_eq!(b_gas(w),20);
    (peak,external.expect("frozen external pin CAS"))
}
#[test]
#[ignore="frozen three-task continuous outcome/scheduling separation"]
fn s29_continuous_task_history_native(){
    let arm=std::env::var("OCEE_S29_CONTINUOUS_ARM").unwrap();
    assert!(matches!(arm.as_str(),"RELEASED"|"PINNED"));
    let released=arm=="RELEASED";
    let (mut w,versions,target,_,registered)=lifetime_setup(129);
    let before=spend(&w);
    let p=continuous_task_root();
    let raw=p.canonical_bytes();
    assert!(!raw.windows(64).any(|x|x==target.as_bytes()));
    let root=p.content_hash().unwrap();
    let root_bytes=raw.len();
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    put(&mut w,version_key(&root),raw);
    put(&mut w,"/self/pins/s29-continuous-root".into(),root.as_bytes().to_vec());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut w);
    let base_hash=w.state_hash();
    let after=spend(&w);
    assert!(!w.state.objects.contains_key(S29_DRY_KEY));
    let installed=lifetime_extra_versions(&w);
    let idx=versions.iter().position(|h|h==&target).unwrap();
    let pin=format!("/self/pins/s29-dry-{idx}");
    let target_len=w.state.objects[&version_key(&target)].bytes.len() as u64;
    lifetime_set_active(&mut w,&root);
    let start=w.state.step;
    let start_a=w.state.nodes[&NodeId::A].gas_available;
    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
    let (peak,external)=continuous_task_episode(&mut w,&root,&pin,&target,released);
    let (rp,re)=continuous_task_episode(&mut replay,&root,&pin,&target,released);
    assert_eq!(peak,rp);assert_eq!(external,re);
    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
    let mut tasks=Vec::new();let mut marks=Vec::new();let mut indices=Vec::new();
    let mut vm=0u64;let mut world=0u64;let mut task_a=0u64;let mut reclaimed=0u64;
    let mut gc_step=None;let mut conflicts=0;
    for e in w.audit.iter().filter(|e|e.step>=start){
        vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
        if let Some(Action::Write{key,bytes,..})=&e.action{
            assert_ne!(key,"/self/active");
            if key.starts_with(CONT_TASK_PREFIX){
                assert_eq!(bytes,&vec![0x55;280]);
                tasks.push((e.step,key.clone(),e.result.code,e.result.gas_charged));
                task_a+=e.result.gas_charged;
            }
            if key==S29_DRY_KEY{
                assert_eq!(e.result.code,Code::Ok);assert!(e.result.gas_charged>0);
                marks.push((e.step,bytes.clone(),e.result.gas_charged));
            }
        }
        if matches!(&e.action,Some(Action::IndexVersionRefs{..})){
            assert_eq!(e.result.code,Code::Ok);indices.push(e.step);
        }
        if let Some(Action::ReclaimIndexedVersion{program_hash,..})=&e.action{
            if e.result.code==Code::Ok{
                assert_eq!(program_hash,&target);
                reclaimed+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());
                gc_step=Some(e.step);
            }else if e.result.code==Code::Conflict{conflicts+=1;}
        }
    }
    assert_eq!(tasks.len(),if released{4}else{3});
    for (i,t) in tasks.iter().take(3).enumerate(){
        assert_eq!(t.1,format!("{CONT_TASK_PREFIX}{i}"));
        assert_eq!(t.2,Code::Quota);
    }
    assert_eq!(marks.len(),3);
    assert_eq!(marks[0].1,b"1");
    assert_eq!(marks[1].1,b"2");
    assert_eq!(marks[2].1,if released{b"0"}else{b"1"});
    assert_eq!(indices.len(),2);
    assert!(tasks[0].0<indices[0]&&indices[0]<marks[0].0&&marks[0].0<start+25);
    assert!(start+25<tasks[1].0&&tasks[1].0<marks[1].0&&marks[1].0<tasks[2].0);
    assert!(tasks[2].0<indices[1]&&indices[1]<marks[2].0);
    if released{
        assert_eq!(tasks[3].1,format!("{CONT_TASK_PREFIX}2"));
        assert_eq!(tasks[3].2,Code::Ok);
        assert!(indices[1]<gc_step.unwrap()&&gc_step.unwrap()<tasks[3].0);
        assert!(tasks[3].0<marks[2].0);
    }
    assert_eq!(reclaimed,if released{target_len}else{0});
    assert_eq!(paid_memory_value(&w),(if released{b"0".to_vec()}else{b"1".to_vec()},2));
    for i in 0..3{
        assert_eq!(w.state.objects.contains_key(&format!("{CONT_TASK_PREFIX}{i}")),released&&i==2);
    }
    for h in &installed{
        assert_eq!(w.state.objects.contains_key(&version_key(h)),!(released&&h==&target));
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
    println!("S29_CONTINUOUS_DONE arm={arm} seed=129 root={root} root_B={root_bytes} base={base_hash} start={start} start_A={start_a} tasks={tasks:?} marks={marks:?} indexes={indices:?} conflicts={conflicts} gc_B={reclaimed} gc_step={gc_step:?} VM_A={vm} world_A={world} task_A={task_a} external_A={external} non_task_A={} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} final_memory={:?} one_root=true replay=true B=20",
        vm+world-task_a-external,after.0-before.0,after.1-before.1,
        peak.storage_peak,peak.post_tick_peak,paid_memory_value(&w));
}


// Actual execution of the tool earned by the P2 baseline, not a supplied
// answer. Retry re-reads registry and re-executes; a correct computation
// only counts as useful task completion after the actual storage commit.
fn continuous_semantic_root()->Program{
    let mut p=continuous_task_root();
    let f=&mut p.functions[0];
    for (entry,suffix) in [("episode_task","initial"),("task_retry","retry")]{
        let old=f.blocks.iter_mut().find(|b|b.id==entry).unwrap();
        let reg=format!("semantic_{suffix}_registry");
        let exec=format!("semantic_{suffix}_exec");
        let result=format!("semantic_{suffix}_result");
        let store=format!("semantic_{suffix}_store");
        let output_reply=match &old.terminator{
            Terminator::Yield{next,..}=>next.clone(),
            _=>panic!("original task is a real Write")
        };
        old.instructions=vec![Instruction{id:10,op:lit(REGISTRY.as_bytes())}];
        old.terminator=Terminator::Yield{call:WorldCall::Read,args:vec![10],
            next:reg.clone(),next_args:vec![99]};
        let reply=||vec![Param{id:99,ty:Type::U32},Param{id:2,ty:Type::Bytes}];
        let code=||vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"Ok")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ];
        f.blocks.extend([
            Block{id:reg,params:reply(),instructions:code(),
                terminator:Terminator::Branch{condition:23,if_true:exec.clone(),
                    true_args:vec![99,2],if_false:"failed".into(),false_args:vec![99]}},
            Block{id:exec,params:reply(),instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:Op::ConstU32(7)},
                Instruction{id:13,op:Op::Add(99,12)},
                Instruction{id:14,op:Op::PackByte(13)},
                Instruction{id:15,op:lit(&[11,19,23,31])},
                Instruction{id:16,op:Op::ConstU32(0)},
                Instruction{id:17,op:Op::ConstU32(2)},
                Instruction{id:18,op:Op::Add(99,17)},
                Instruction{id:19,op:Op::Slice(15,16,18)},
                Instruction{id:24,op:Op::Concat(14,19)}
            ],terminator:Terminator::Yield{call:WorldCall::Exec,args:vec![11,24],
                next:result.clone(),next_args:vec![99]}},
            Block{id:result,params:reply(),instructions:code(),
                terminator:Terminator::Branch{condition:23,if_true:store.clone(),
                    true_args:vec![99,2],if_false:"failed".into(),false_args:vec![99]}},
            Block{id:store,params:reply(),instructions:vec![
                Instruction{id:10,op:lit(b"payload")},
                Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
                Instruction{id:12,op:lit(b"U32")},
                Instruction{id:13,op:Op::JsonFieldU32(11,12)},
                Instruction{id:14,op:Op::ConstU32(256)},
                Instruction{id:15,op:Op::Div(13,14)},
                Instruction{id:16,op:Op::ConstU32(0)},
                Instruction{id:17,op:Op::EqU32(15,16)}
            ],terminator:Terminator::Branch{condition:17,if_true:format!("semantic_{suffix}_write"),
                true_args:vec![99,13],if_false:"failed".into(),false_args:vec![99]}},
            Block{id:format!("semantic_{suffix}_write"),
                params:vec![Param{id:99,ty:Type::U32},Param{id:2,ty:Type::U32}],
                instructions:vec![
                    Instruction{id:10,op:lit(CONT_TASK_PREFIX.as_bytes())},
                    Instruction{id:13,op:Op::ConstU32(48)},
                    Instruction{id:14,op:Op::Add(99,13)},
                    Instruction{id:15,op:Op::PackByte(14)},
                    Instruction{id:16,op:Op::Concat(10,15)},
                    Instruction{id:17,op:Op::PackByte(2)},
                    Instruction{id:18,op:lit(&[0x55;279])},
                    Instruction{id:11,op:Op::Concat(17,18)},
                    Instruction{id:12,op:Op::ConstU32(u32::MAX)}
                ],terminator:Terminator::Yield{call:WorldCall::Write,args:vec![16,11,12],
                    next:output_reply,next_args:vec![99]}}
        ]);
    }
    p.validate().expect("continuous registered-tool computation and actual Write");
    p
}
fn semantic_input(slot:usize)->Vec<u8>{
    let mut input=vec![7+slot as u8];
    input.extend_from_slice(&[11,19,23,31][..2+slot]);
    input
}
#[test]
#[ignore="frozen continuous registered-tool utility across three actual P2 seeds"]
fn s29_continuous_registered_tool_native(){
    let seed:u64=std::env::var("OCEE_S29_LIFE_SEED").unwrap().parse().unwrap();
    assert!([129,130,131].contains(&seed));
    let arm=std::env::var("OCEE_S29_CONTINUOUS_ARM").unwrap();
    assert!(matches!(arm.as_str(),"RELEASED"|"PINNED"));
    let released=arm=="RELEASED";
    let (mut w,versions,target,_,registered)=lifetime_setup(seed);
    let setup_before=spend(&w);
    let p=continuous_semantic_root();
    let raw=p.canonical_bytes();
    assert!(!raw.windows(64).any(|x|x==target.as_bytes()||x==registered.as_bytes()));
    let root=p.content_hash().unwrap();
    let root_bytes=raw.len();
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    put(&mut w,version_key(&root),raw);
    put(&mut w,"/self/pins/s29-semantic-root".into(),root.as_bytes().to_vec());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut w);
    let setup_after=spend(&w);
    let base=w.state_hash();
    let installed=lifetime_extra_versions(&w);
    let idx=versions.iter().position(|h|h==&target).unwrap();
    let pin=format!("/self/pins/s29-dry-{idx}");
    let target_len=w.state.objects[&version_key(&target)].bytes.len() as u64;
    lifetime_set_active(&mut w,&root);
    let start=w.state.step;
    let start_a=w.state.nodes[&NodeId::A].gas_available;
    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
    let (peak,external)=continuous_task_episode(&mut w,&root,&pin,&target,released);
    let (rp,re)=continuous_task_episode(&mut replay,&root,&pin,&target,released);
    assert_eq!(peak,rp);assert_eq!(external,re);
    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
    let mut tasks=Vec::new();let mut execs=Vec::new();let mut marks=Vec::new();
    let mut indexes=Vec::new();let mut reg_reads=Vec::new();
    let mut gc=0u64;let mut gc_step=None;let mut vm=0u64;let mut world=0u64;
    let mut task_a=0u64;let mut exec_a=0u64;let mut read_a=0u64;
    for e in w.audit.iter().filter(|e|e.step>=start){
        vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
        match &e.action{
            Some(Action::Read{key,..}) if key==REGISTRY=>{
                assert_eq!(e.result.code,Code::Ok);
                assert_eq!(e.result.payload,registered.as_bytes());
                read_a+=e.result.gas_charged;reg_reads.push(e.step);
            },
            Some(Action::Exec{program_hash,input,..})=>{
                assert_eq!(program_hash,&registered);
                let slot=execs.len().min(2);
                assert_eq!(input,&semantic_input(slot));
                assert_eq!(e.result.code,Code::Ok);
                let actual:Value=serde_json::from_slice(&e.result.payload).unwrap();
                let expected=hidden_transform(seed,input);
                assert_eq!(actual,Value::U32(expected));
                assert!(expected<=255);
                exec_a+=e.result.gas_charged;
                execs.push((e.step,slot,expected,e.result.gas_charged));
            },
            Some(Action::Write{key,bytes,..})=>{
                assert_ne!(key,"/self/active");
                if let Some(raw)=key.strip_prefix(CONT_TASK_PREFIX){
                    let slot:usize=raw.parse().unwrap();
                    assert!(slot<3);
                    assert_eq!(bytes.len(),280);
                    assert_eq!(bytes[0],hidden_transform(seed,&semantic_input(slot)) as u8);
                    assert!(bytes[1..].iter().all(|v|*v==0x55));
                    assert_eq!(execs.last().unwrap().1,slot);
                    assert!(execs.last().unwrap().0<e.step);
                    tasks.push((e.step,slot,e.result.code,e.result.gas_charged));
                    task_a+=e.result.gas_charged;
                }else if key==S29_DRY_KEY{
                    assert_eq!(e.result.code,Code::Ok);
                    assert!(e.result.gas_charged>0);
                    marks.push((e.step,bytes.clone(),e.result.gas_charged));
                }
            },
            Some(Action::IndexVersionRefs{..})=>{
                assert_eq!(e.result.code,Code::Ok);indexes.push(e.step);
            },
            Some(Action::ReclaimIndexedVersion{program_hash,..}) if e.result.code==Code::Ok=>{
                assert_eq!(program_hash,&target);
                gc+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());
                gc_step=Some(e.step);
            },
            _=>{}
        }
    }
    let expected_count=if released{4}else{3};
    assert_eq!(tasks.len(),expected_count);
    assert_eq!(execs.len(),expected_count);
    assert_eq!(reg_reads.len(),expected_count);
    for i in 0..expected_count{
        assert!(reg_reads[i]<execs[i].0&&execs[i].0<tasks[i].0);
        assert_eq!(tasks[i].1,i.min(2));
        assert_eq!(tasks[i].2,if i==3{Code::Ok}else{Code::Quota});
    }
    assert_eq!(indexes.len(),2);assert_eq!(marks.len(),3);
    assert_eq!(marks[0].1,b"1");assert_eq!(marks[1].1,b"2");
    assert_eq!(marks[2].1,if released{b"0"}else{b"1"});
    assert!(indexes[0]<marks[0].0&&marks[0].0<start+25);
    assert!(start+25<tasks[1].0&&tasks[1].0<marks[1].0&&marks[1].0<tasks[2].0);
    assert_eq!(gc,if released{target_len}else{0});
    if released{assert!(indexes[1]<gc_step.unwrap()&&gc_step.unwrap()<execs[3].0
        &&tasks[3].0<marks[2].0);}
    assert_eq!(paid_memory_value(&w),(if released{b"0".to_vec()}else{b"1".to_vec()},2));
    for slot in 0..3{
        let key=format!("{CONT_TASK_PREFIX}{slot}");
        assert_eq!(w.state.objects.contains_key(&key),released&&slot==2);
        if released&&slot==2{
            let bytes=&w.state.objects[&key].bytes;
            assert_eq!(bytes.len(),280);
            assert_eq!(bytes[0],hidden_transform(seed,&semantic_input(slot)) as u8);
            assert!(bytes[1..].iter().all(|v|*v==0x55));
        }
    }
    for h in &installed{
        assert_eq!(w.state.objects.contains_key(&version_key(h)),!(released&&h==&target));
    }
    assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
    println!("S29_SEMANTIC_CONTINUOUS seed={seed} arm={arm} root={root} root_B={root_bytes} base={base} registered={registered} target={target} start={start} start_A={start_a} tasks={tasks:?} execs={execs:?} reads={reg_reads:?} marks={marks:?} indexes={indexes:?} gc_B={gc} gc_step={gc_step:?} VM_A={vm} world_A={world} task_A={task_a} exec_A={exec_a} registry_read_A={read_a} external_A={external} other_world_A={} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} final_memory={:?} one_root=true replay=true B=20",
        world-task_a-exec_a-read_a-external,setup_after.0-setup_before.0,
        setup_after.1-setup_before.1,peak.storage_peak,peak.post_tick_peak,paid_memory_value(&w));
}


// Bounded polling of ordinary world-owned offers. The input and identity
// travel in the continuation, not in host-selected VM calls or baked literals.
const OFFER_PUBLIC:&str="/public/E001/transform";
const OFFER_OUTCOME:&str="/self/outcomes/s29-offer-";
fn public_offer_root()->Program{
    let mut p=continuous_semantic_root();
    let f=&mut p.functions[0];
    let header=f.blocks.iter_mut().find(|b|b.id=="episode_header").unwrap();
    if let Terminator::RepeatBounded{body,..}=&mut header.terminator{
        *body="offer_poll_header".into();
    }else{panic!("expected bounded opportunity loop");}
    let end=f.blocks.iter_mut().find(|b|b.id=="episode_end").unwrap();
    end.instructions.clear();
    end.terminator=Terminator::Jump{target:"episode_next".into(),args:vec![99]};
    f.blocks.retain(|b|!matches!(b.id.as_str(),
        "episode_delay"|"episode_observe"|"episode_observe_reply"));
    for suffix in ["initial","retry"]{
        let b=f.blocks.iter_mut().find(|b|b.id==format!("semantic_{suffix}_exec")).unwrap();
        b.instructions=vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)}
        ];
        if let Terminator::Yield{args,..}=&mut b.terminator{*args=vec![11,97];}
        else{panic!("expected actual EXEC");}
        let b=f.blocks.iter_mut().find(|b|b.id==format!("semantic_{suffix}_write")).unwrap();
        b.instructions=vec![
            Instruction{id:10,op:lit(OFFER_OUTCOME.as_bytes())},
            Instruction{id:15,op:Op::HexEncode(98)},
            Instruction{id:16,op:Op::Concat(10,15)},
            Instruction{id:17,op:Op::PackByte(2)},
            Instruction{id:18,op:lit(&[0x55;279])},
            Instruction{id:11,op:Op::Concat(17,18)},
            Instruction{id:12,op:Op::ConstU32(u32::MAX)}
        ];
    }
    // Preserve the exact received input and identity across all yields,
    // including a real GC and retry. Feedback remains the last parameter.
    let yield_targets=f.blocks.iter().filter_map(|b|match &b.terminator{
        Terminator::Yield{next,..}=>Some(next.clone()),_=>None
    }).collect::<std::collections::BTreeSet<_>>();
    for b in &mut f.blocks{
        if b.id=="entry"{
            b.instructions.extend([
                Instruction{id:97,op:lit(b"")},Instruction{id:98,op:lit(b"")}
            ]);
        }else{
            let pos=if yield_targets.contains(&b.id){b.params.len()-1}else{b.params.len()};
            b.params.insert(pos,Param{id:97,ty:Type::Bytes});
            b.params.insert(pos+1,Param{id:98,ty:Type::Bytes});
        }
        match &mut b.terminator{
            Terminator::Jump{args,..}=>args.extend([97,98]),
            Terminator::Branch{true_args,false_args,..}=>{
                true_args.extend([97,98]);false_args.extend([97,98]);
            },
            Terminator::RepeatBounded{body_args,done_args,..} |
            Terminator::RepeatScoped{body_args,done_args,..}=>{
                body_args.extend([97,98]);done_args.extend([97,98]);
            },
            Terminator::Yield{next_args,..}=>next_args.extend([97,98]),
            Terminator::Return{..}=>{}
        }
    }
    let context=||vec![Param{id:99,ty:Type::U32},Param{id:97,ty:Type::Bytes},
        Param{id:98,ty:Type::Bytes}];
    let feedback=||{
        let mut v=context();v.push(Param{id:2,ty:Type::Bytes});v
    };
    f.blocks.extend([
        Block{id:"offer_poll_header".into(),params:context(),instructions:vec![],
            terminator:Terminator::RepeatScoped{limit:32,aggregate_limit:96,scope:99,
                body:"offer_read".into(),body_args:vec![99,97,98],
                done:"episode_return".into(),done_args:vec![99,97,98]}},
        Block{id:"offer_read".into(),params:context(),
            instructions:vec![Instruction{id:10,op:lit(OFFER_PUBLIC.as_bytes())}],
            terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],
                next:"offer_reply".into(),next_args:vec![99,97,98]}},
        Block{id:"offer_reply".into(),params:feedback(),instructions:vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"Ok")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ],terminator:Terminator::Branch{condition:23,if_true:"offer_parse".into(),
            true_args:vec![99,97,98,2],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}},
        Block{id:"offer_parse".into(),params:feedback(),instructions:vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:lit(b"task_id")},
            Instruction{id:13,op:Op::JsonFieldString(11,12)},
            Instruction{id:14,op:Op::EqBytes(13,98)}
        ],terminator:Terminator::Branch{condition:14,if_true:"offer_poll_header".into(),
            true_args:vec![99,97,98],if_false:"offer_fresh".into(),
            false_args:vec![99,97,98,11,13]}},
        Block{id:"offer_fresh".into(),params:{
            let mut v=context();v.extend([Param{id:1,ty:Type::Bytes},Param{id:2,ty:Type::Bytes}]);v
        },instructions:vec![
            Instruction{id:10,op:Op::Len(2)},
            Instruction{id:11,op:Op::ConstU32(1)},
            Instruction{id:12,op:Op::EqU32(10,11)}
        ],terminator:Terminator::Branch{condition:12,if_true:"offer_input".into(),
            true_args:vec![99,97,98,1,2],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}},
        Block{id:"offer_input".into(),params:{
            let mut v=context();v.extend([Param{id:1,ty:Type::Bytes},Param{id:2,ty:Type::Bytes}]);v
        },instructions:vec![
            Instruction{id:10,op:lit(b"held_out_input")},
            Instruction{id:11,op:Op::JsonFieldBytes(1,10)},
            Instruction{id:12,op:Op::Len(11)},
            Instruction{id:13,op:Op::ConstU32(0)},
            Instruction{id:14,op:Op::EqU32(12,13)}
        ],terminator:Terminator::Branch{condition:14,if_true:"offer_poll_header".into(),
            true_args:vec![99,97,98],if_false:"offer_input_bound".into(),
            false_args:vec![99,97,98,11,2]}},
        Block{id:"offer_input_bound".into(),params:{
            let mut v=context();v.extend([Param{id:1,ty:Type::Bytes},Param{id:2,ty:Type::Bytes}]);v
        },instructions:vec![
            Instruction{id:10,op:Op::Len(1)},
            Instruction{id:11,op:Op::ConstU32(256)},
            Instruction{id:12,op:Op::Div(10,11)},
            Instruction{id:13,op:Op::ConstU32(0)},
            Instruction{id:14,op:Op::EqU32(12,13)}
        ],terminator:Terminator::Branch{condition:14,if_true:"episode_task".into(),
            true_args:vec![99,1,2],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}}
    ]);
    p.validate().expect("bounded offer polling, retained input and identity");
    p
}
fn offer_fixture(slot:usize)->TransformTask{
    let (id,input)=match slot{
        0=>("a",vec![44,7,2]),1=>("b",vec![13,9,4,8]),
        2=>("c",vec![27,2,1,4,8]),_=>panic!("unknown frozen offer")
    };
    TransformTask{task_id:id.into(),examples:vec![],held_out_input:input}
}
fn offer_publish(w:&mut World,slot:usize){
    // Explicit environment-only fixture publication, not an organism action.
    let old=w.state.objects.get(OFFER_PUBLIC).expect("real public object");
    assert_eq!(old.owner,0);
    let version=old.version+1;
    w.state.version_ref_index=None;
    w.state.objects.insert(OFFER_PUBLIC.into(),ocee_world_kernel::Object{
        owner:0,version,bytes:serde_json::to_vec(&offer_fixture(slot)).unwrap()
    });
}
fn offer_episode(w:&mut World,root:&str,pin:&str,target:&str,arrivals:bool)
    ->(LifetimeIndexProbe,u64,Vec<(u64,u64,usize,String)>){
    let start=w.state.step;
    let mut peak=LifetimeIndexProbe::default();peak.sample(w);
    let mut external=None;let mut publications=Vec::new();
    while w.state.step<start+150{
        let offset=w.state.step-start;
        if [0,28,48].contains(&offset){
            let slot=if arrivals{match offset{0=>0,28=>1,48=>2,_=>unreachable!()}}else{0};
            offer_publish(w,slot);
            let obj=&w.state.objects[OFFER_PUBLIC];
            publications.push((w.state.step,obj.version,slot,w.state_hash()));
            peak.sample(w);
        }
        if offset==22{
            assert_eq!(paid_memory_value(w),(b"1".to_vec(),0));
            let r=lifetime_external_write(w,pin,vec![b'-';64],Some(0),&mut peak);
            assert_eq!(r.code,Code::Ok);assert!(r.gas_charged>0);
            external=Some(r.gas_charged);
        }else if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished){
            w.tick(None);peak.sample(w);
        }else{
            let r=w.tick_autonomous(ORGANISM_ID);peak.sample(w);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        }
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(root));
    }
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(if arrivals{3}else{1})));
    assert_eq!(b_gas(w),20);
    (peak,external.expect("fixed pin release"),publications)
}
#[test]
#[ignore="frozen public offer driven registered tool task pilot"]
fn s29_public_offer_driven_native(){
    use sha2::{Digest,Sha256};
    let arm=std::env::var("OCEE_S29_OFFER_ARM").unwrap();
    assert!(matches!(arm.as_str(),"ARRIVALS"|"DUPLICATE_ONLY"));
    let arrivals=arm=="ARRIVALS";
    let (mut w,versions,target,_,registered)=lifetime_setup(129);
    let before=spend(&w);
    let p=public_offer_root();let raw=p.canonical_bytes();let root_b=raw.len();
    assert!(!raw.windows(64).any(|x|x==target.as_bytes()||x==registered.as_bytes()));
    let root=p.content_hash().unwrap();
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    put(&mut w,version_key(&root),raw);
    put(&mut w,"/self/pins/s29-public-offer-root".into(),root.as_bytes().to_vec());
    let used=w.state.organisms[&ORGANISM_ID].storage_used;
    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut w);
    let setup=spend(&w);let base=w.state_hash();
    let installed=lifetime_extra_versions(&w);
    let idx=versions.iter().position(|h|h==&target).unwrap();
    let pin=format!("/self/pins/s29-dry-{idx}");
    let target_len=w.state.objects[&version_key(&target)].bytes.len() as u64;
    lifetime_set_active(&mut w,&root);
    let start=w.state.step;let start_a=w.state.nodes[&NodeId::A].gas_available;
    let schedule=serde_json::json!({"arm":arm,"start":start,"publications":[
        [0,0],[28,if arrivals{1}else{0}],[48,if arrivals{2}else{0}]],
        "offers":[offer_fixture(0),offer_fixture(1),offer_fixture(2)],
        "release_offset":22,"pin":pin,"target":target,"horizon":150});
    let schedule_hash=format!("{:x}",Sha256::digest(serde_json::to_vec(&schedule).unwrap()));
    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
    let (peak,external,publications)=offer_episode(&mut w,&root,&pin,&target,arrivals);
    let (rp,re,rpub)=offer_episode(&mut replay,&root,&pin,&target,arrivals);
    assert_eq!(peak,rp);assert_eq!(external,re);assert_eq!(publications,rpub);
    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
    let mut reads=Vec::new();let mut execs=Vec::new();let mut tasks=Vec::new();
    let mut marks=Vec::new();let mut indexes=Vec::new();let mut gc=0;let mut gc_step=None;
    let mut vm=0u64;let mut world=0u64;let mut poll_a=0u64;let mut exec_a=0u64;let mut task_a=0u64;
    for e in w.audit.iter().filter(|e|e.step>=start){
        vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
        match &e.action{
            Some(Action::Read{key,..}) if key==OFFER_PUBLIC=>{
                assert_eq!(e.result.code,Code::Ok);
                let offer:TransformTask=serde_json::from_slice(&e.result.payload).unwrap();
                assert!(offer.examples.is_empty());
                let slot=if arrivals&&e.step>=start+48{2}else if arrivals&&e.step>=start+28{1}else{0};
                assert_eq!(offer,offer_fixture(slot));
                reads.push((e.step,offer.task_id));poll_a+=e.result.gas_charged;
            },
            Some(Action::Exec{program_hash,input,..})=>{
                assert_eq!(program_hash,&registered);assert_eq!(e.result.code,Code::Ok);
                let slot=execs.len().min(2);
                let offer=offer_fixture(slot);
                assert_eq!(input,&offer.held_out_input);
                let expected=hidden_transform(129,input);
                assert_eq!(serde_json::from_slice::<Value>(&e.result.payload).unwrap(),Value::U32(expected));
                assert!(reads.iter().any(|(step,id)|*step<e.step&&id==&offer.task_id));
                execs.push((e.step,slot,expected));exec_a+=e.result.gas_charged;
            },
            Some(Action::Write{key,bytes,..})=>{
                assert_ne!(key,"/self/active");
                if key.starts_with(OFFER_OUTCOME){
                    let slot=tasks.len().min(2);let offer=offer_fixture(slot);
                    assert_eq!(key,&format!("{OFFER_OUTCOME}{:02x}",offer.task_id.as_bytes()[0]));
                    assert_eq!(bytes.len(),280);assert_eq!(bytes[0],hidden_transform(129,&offer.held_out_input) as u8);
                    assert!(bytes[1..].iter().all(|x|*x==0x55));
                    assert_eq!(execs.last().unwrap().1,slot);
                    tasks.push((e.step,slot,e.result.code,e.result.gas_charged));
                    task_a+=e.result.gas_charged;
                }else if key==S29_DRY_KEY{
                    assert_eq!(e.result.code,Code::Ok);assert!(e.result.gas_charged>0);
                    marks.push((e.step,bytes.clone(),e.result.gas_charged));
                }
            },
            Some(Action::IndexVersionRefs{..})=>{
                assert_eq!(e.result.code,Code::Ok);indexes.push(e.step);
            },
            Some(Action::ReclaimIndexedVersion{program_hash,..}) if e.result.code==Code::Ok=>{
                assert_eq!(program_hash,&target);
                gc+=u64::from_le_bytes(e.result.payload.as_slice().try_into().unwrap());gc_step=Some(e.step);
            },
            _=>{}
        }
    }
    assert_eq!(tasks.len(),if arrivals{4}else{1});
    assert_eq!(execs.len(),tasks.len());
    assert!(reads.len()>tasks.len());
    assert_eq!(indexes.len(),if arrivals{2}else{1});
    assert_eq!(marks.len(),if arrivals{3}else{1});
    assert_eq!(marks[0].1,b"1");assert!(marks[0].0<start+22);
    if arrivals{
        assert_eq!(tasks[0].2,Code::Quota);assert_eq!(tasks[1].2,Code::Quota);
        assert_eq!(tasks[2].2,Code::Quota);assert_eq!(tasks[3].2,Code::Ok);
        assert!(tasks[1].0>=start+28&&tasks[2].0>=start+48);
        assert_eq!(marks[1].1,b"2");assert_eq!(marks[2].1,b"0");
        assert!(indexes[1]<gc_step.unwrap()&&gc_step.unwrap()<execs[3].0&&tasks[3].0<marks[2].0);
    }else{assert_eq!(tasks[0].2,Code::Quota);}
    assert_eq!(gc,if arrivals{target_len}else{0});
    assert_eq!(paid_memory_value(&w),if arrivals{(b"0".to_vec(),2)}else{(b"1".to_vec(),0)});
    for slot in 0..3{
        let offer=offer_fixture(slot);
        let key=format!("{OFFER_OUTCOME}{:02x}",offer.task_id.as_bytes()[0]);
        assert_eq!(w.state.objects.contains_key(&key),arrivals&&slot==2);
        if arrivals&&slot==2{assert_eq!(w.state.objects[&key].bytes[0],5);}
    }
    for h in &installed{assert_eq!(w.state.objects.contains_key(&version_key(h)),!(arrivals&&h==&target));}
    assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
    println!("S29_OFFER_DONE arm={arm} root={root} root_B={root_b} base={base} start={start} start_A={start_a} schedule={schedule_hash} publications={publications:?} reads={reads:?} execs={execs:?} tasks={tasks:?} marks={marks:?} indexes={indexes:?} gc_B={gc} VM_A={vm} world_A={world} poll_A={poll_a} exec_A={exec_a} task_A={task_a} external_A={external} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} replay=true B=20",
        setup.0-before.0,setup.1-before.1,peak.storage_peak,peak.post_tick_peak);
}


// Distinguish attempted ID from actually committed artifact. Retained input
// binds same-ID retry; real current space is paid and read, not guessed.
fn pending_offer_root()->Program{
    let mut p=public_offer_root();
    let f=&mut p.functions[0];
    let parse=f.blocks.iter_mut().find(|b|b.id=="offer_parse").unwrap();
    if let Terminator::Branch{if_true,true_args,..}=&mut parse.terminator{
        assert_eq!(if_true,"offer_poll_header");
        *if_true="pending_same_input".into();*true_args=vec![99,97,98,11];
    }else{panic!("expected actual same ID branch");}
    let context=||vec![Param{id:99,ty:Type::U32},
        Param{id:97,ty:Type::Bytes},Param{id:98,ty:Type::Bytes}];
    let reply=||{
        let mut v=context();v.push(Param{id:2,ty:Type::Bytes});v
    };
    f.blocks.extend([
        Block{id:"pending_same_input".into(),params:reply(),instructions:vec![
            Instruction{id:10,op:lit(b"held_out_input")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:Op::EqBytes(11,97)}
        ],terminator:Terminator::Branch{condition:12,if_true:"pending_artifact".into(),
            true_args:vec![99,97,98],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}},
        Block{id:"pending_artifact".into(),params:context(),instructions:vec![
            Instruction{id:10,op:lit(OFFER_OUTCOME.as_bytes())},
            Instruction{id:11,op:Op::HexEncode(98)},
            Instruction{id:12,op:Op::Concat(10,11)}
        ],terminator:Terminator::Yield{call:WorldCall::Read,args:vec![12],
            next:"pending_artifact_reply".into(),next_args:vec![99,97,98]}},
        Block{id:"pending_artifact_reply".into(),params:reply(),instructions:vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"NotFound")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ],terminator:Terminator::Branch{condition:23,if_true:"pending_resources".into(),
            true_args:vec![99,97,98],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}},
        Block{id:"pending_resources".into(),params:context(),
            instructions:vec![Instruction{id:10,op:lit(b"/sys/resource-status")}],
            terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],
                next:"pending_resource_reply".into(),next_args:vec![99,97,98]}},
        Block{id:"pending_resource_reply".into(),params:reply(),instructions:vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"Ok")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ],terminator:Terminator::Branch{condition:23,if_true:"pending_space".into(),
            true_args:vec![99,97,98,2],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98]}},
        Block{id:"pending_space".into(),params:reply(),instructions:vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:lit(b"capacity")},
            Instruction{id:13,op:Op::JsonFieldU32(11,12)},
            Instruction{id:14,op:lit(b"used")},
            Instruction{id:15,op:Op::JsonFieldU32(11,14)},
            Instruction{id:16,op:Op::Sub(13,15)},
            Instruction{id:17,op:Op::ConstU32(280)},
            Instruction{id:18,op:Op::Div(16,17)},
            Instruction{id:19,op:Op::ConstU32(0)},
            Instruction{id:20,op:Op::EqU32(18,19)}
        ],terminator:Terminator::Branch{condition:20,if_true:"offer_poll_header".into(),
            true_args:vec![99,97,98],if_false:"episode_task".into(),
            false_args:vec![99,97,98]}}
    ]);
    p.validate().expect("typed same-ID pending recovery");
    p
}
fn pending_offer_publish(w:&mut World,conflict:bool){
    offer_publish(w,0);
    if conflict{
        let obj=w.state.objects.get_mut(OFFER_PUBLIC).unwrap();
        let mut offer=offer_fixture(0);
        offer.held_out_input=offer_fixture(1).held_out_input;
        obj.bytes=serde_json::to_vec(&offer).unwrap();
    }
}
fn pending_offer_episode(w:&mut World,root:&str,arm:&str)
    ->(LifetimeIndexProbe,Vec<(u64,u64,u64,String)>){
    let start=w.state.step;
    let mut peak=LifetimeIndexProbe::default();peak.sample(w);
    let mut publications=Vec::new();
    while w.state.step<start+180{
        let offset=w.state.step-start;
        if offset==0 || offset==28{
            if offset==28{
                assert_eq!(paid_memory_value(w),(b"1".to_vec(),0));
                if arm!="NO_SPACE"{
                    w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity+=512;
                }
            }
            pending_offer_publish(w,offset==28&&arm=="ID_CONFLICT");
            publications.push((w.state.step,w.state.objects[OFFER_PUBLIC].version,
                w.state.nodes[&NodeId::A].storage_capacity,w.state_hash()));
            peak.sample(w);
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished){
            w.tick(None);peak.sample(w);
        }else{
            let r=w.tick_autonomous(ORGANISM_ID);peak.sample(w);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        }
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(root));
    }
    assert_eq!(b_gas(w),20);
    (peak,publications)
}
#[test]
#[ignore="frozen same-ID pending outcome and resource retry"]
fn s29_pending_offer_resource_native(){
    let arm=std::env::var("OCEE_S29_PENDING_ARM").unwrap();
    assert!(matches!(arm.as_str(),"SPACE"|"NO_SPACE"|"ID_CONFLICT"));
    let (mut base,_,target,_,registered)=lifetime_setup(129);
    let setup_before=spend(&base);
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+65_536;
    let mut roots=Vec::new();
    for (i,p) in [public_offer_root(),pending_offer_root()].into_iter().enumerate(){
        let bytes=p.canonical_bytes();
        assert!(!bytes.windows(64).any(|x|x==target.as_bytes()||x==registered.as_bytes()));
        let h=p.content_hash().unwrap();
        let len=bytes.len();
        put(&mut base,version_key(&h),bytes);
        put(&mut base,format!("/self/pins/s29-pending-root-{i}"),h.as_bytes().to_vec());
        roots.push((h,len));
    }
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+195;
    quota_wait_next_grant(&mut base);
    let setup_after=spend(&base);let base_hash=base.state_hash();
    let installed=lifetime_extra_versions(&base);
    let key=format!("{OFFER_OUTCOME}61");
    let input=offer_fixture(0).held_out_input;
    let mut common_start=None;
    for (i,(root,len)) in roots.iter().enumerate(){
        let pending=i==1;
        let success=pending&&arm=="SPACE";
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,root);
        let start=w.state.step;
        let start_a=w.state.nodes[&NodeId::A].gas_available;
        if let Some(prev)=common_start{assert_eq!((start,start_a),prev);}
        else{common_start=Some((start,start_a));}
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let (peak,pubs)=pending_offer_episode(&mut w,root,&arm);
        let (rp,rpubs)=pending_offer_episode(&mut replay,root,&arm);
        assert_eq!(peak,rp);assert_eq!(pubs,rpubs);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(if success{2}else{1})));
        let mut vm=0u64;let mut world=0u64;let mut task_a=0u64;
        let mut poll_a=0u64;let mut lookup_a=0u64;let mut sensor_a=0u64;let mut exec_a=0u64;
        let mut reads=0;let mut lookups=Vec::new();let mut sensors=Vec::new();
        let mut execs=Vec::new();let mut tasks=Vec::new();let mut marks=Vec::new();
        let mut indexes=0;
        for e in w.audit.iter().filter(|e|e.step>=start){
            vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
            match &e.action{
                Some(Action::Read{key,..}) if key==OFFER_PUBLIC=>{
                    assert_eq!(e.result.code,Code::Ok);poll_a+=e.result.gas_charged;reads+=1;
                    let mut expected=offer_fixture(0);
                    if arm=="ID_CONFLICT"&&e.step>=start+28{
                        expected.held_out_input=offer_fixture(1).held_out_input;
                    }
                    assert_eq!(serde_json::from_slice::<TransformTask>(&e.result.payload).unwrap(),expected);
                },
                Some(Action::Read{key:k,..}) if k==&key=>{
                    assert!(pending);assert!(matches!(e.result.code,Code::Ok|Code::NotFound));
                    lookup_a+=e.result.gas_charged;lookups.push((e.step,e.result.code));
                },
                Some(Action::Read{key,..}) if key=="/sys/resource-status"=>{
                    assert_eq!(e.result.code,Code::Ok);sensor_a+=e.result.gas_charged;
                    let p:serde_json::Value=serde_json::from_slice(&e.result.payload).unwrap();
                    let free=p["capacity"].as_u64().unwrap()-p["used"].as_u64().unwrap();
                    sensors.push((e.step,free));
                },
                Some(Action::Exec{program_hash,input:actual,..})=>{
                    assert_eq!(program_hash,&registered);assert_eq!(actual,&input);
                    assert_eq!(e.result.code,Code::Ok);
                    assert_eq!(serde_json::from_slice::<Value>(&e.result.payload).unwrap(),
                        Value::U32(hidden_transform(129,&input)));
                    execs.push(e.step);exec_a+=e.result.gas_charged;
                },
                Some(Action::Write{key:k,bytes,..})=>{
                    assert_ne!(k,"/self/active");
                    if k.starts_with(OFFER_OUTCOME){
                        assert_eq!(k,&key);assert_eq!(bytes.len(),280);
                        assert_eq!(bytes[0],hidden_transform(129,&input) as u8);
                        assert!(bytes[1..].iter().all(|x|*x==0x55));
                        tasks.push((e.step,e.result.code,e.result.gas_charged));task_a+=e.result.gas_charged;
                    }else if k==S29_DRY_KEY{
                        assert_eq!(e.result.code,Code::Ok);assert_eq!(bytes,b"1");
                        marks.push(e.step);
                    }
                },
                Some(Action::IndexVersionRefs{..})=>{
                    assert_eq!(e.result.code,Code::Ok);indexes+=1;
                },
                Some(Action::ReclaimIndexedVersion{..})=>assert_eq!(e.result.code,Code::Conflict),
                _=>{}
            }
        }
        assert_eq!(tasks.len(),if success{2}else{1});assert_eq!(execs.len(),tasks.len());
        assert_eq!(tasks[0].1,Code::Quota);assert_eq!(indexes,1);assert_eq!(marks.len(),1);
        assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));
        assert_eq!(w.state.objects.contains_key(&key),success);
        if success{
            assert_eq!(tasks[1].1,Code::Ok);assert!(tasks[1].0>=start+28);
            assert!(sensors.iter().any(|(step,free)|*step>=start+28&&*step<execs[1]&&*free>=280));
            assert!(lookups.iter().any(|(step,code)|*step>tasks[1].0&&*code==Code::Ok));
        }
        if pending&&arm=="NO_SPACE"{assert!(sensors.iter().all(|(_,free)|*free<280));}
        if arm=="ID_CONFLICT"{assert!(execs.iter().all(|step|*step<start+28));}
        if !pending{assert!(lookups.is_empty());assert_eq!(sensors.len(),1);}
        for h in &installed{assert!(w.state.objects.contains_key(&version_key(h)));}
        assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
        println!("S29_PENDING_DONE arm={arm} policy={} root={root} root_B={len} base={base_hash} start={start} start_A={start_a} publications={pubs:?} reads={reads} lookups={lookups:?} sensors={sensors:?} execs={execs:?} tasks={tasks:?} marks={marks:?} VM_A={vm} world_A={world} poll_A={poll_a} lookup_A={lookup_a} sensor_A={sensor_a} exec_A={exec_a} task_A={task_a} non_task_A={} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} replay=true B=20",
            if pending{"PENDING"}else{"LAST_ID"},vm+world-task_a,setup_after.0-setup_before.0,
            setup_after.1-setup_before.1,peak.storage_peak,peak.post_tick_peak);
    }
}


// Durable input-bound artifact. Cache is valid only in the declared
// single-writer, non-mutating receipt environment; fresh roots revalidate.
fn bound_receipt_root(cache:bool)->Program{
    let mut p=pending_offer_root();
    let f=&mut p.functions[0];
    for suffix in ["initial","retry"]{
        let b=f.blocks.iter_mut().find(|b|b.id==format!("semantic_{suffix}_write")).unwrap();
        b.instructions.retain(|i|i.id!=18&&i.id!=11&&i.id!=12);
        b.instructions.extend([
            Instruction{id:18,op:Op::Sha256(97)},
            Instruction{id:19,op:Op::Concat(17,18)},
            Instruction{id:20,op:lit(&[0x55;247])},
            Instruction{id:11,op:Op::Concat(19,20)},
            Instruction{id:12,op:Op::ConstU32(u32::MAX)}
        ]);
    }
    let yt=f.blocks.iter().filter_map(|b|match &b.terminator{
        Terminator::Yield{next,..}=>Some(next.clone()),_=>None
    }).collect::<std::collections::BTreeSet<_>>();
    for b in &mut f.blocks{
        assert!(b.params.iter().all(|x|x.id!=96)&&b.instructions.iter().all(|x|x.id!=96&&x.id!=95));
        if b.id=="entry"{b.instructions.push(Instruction{id:96,op:Op::ConstU32(0)});}
        else{
            let pos=if yt.contains(&b.id){b.params.len()-1}else{b.params.len()};
            b.params.insert(pos,Param{id:96,ty:Type::U32});
        }
        match &mut b.terminator{
            Terminator::Jump{args,..}=>args.push(96),
            Terminator::Branch{true_args,false_args,..}=>{true_args.push(96);false_args.push(96);},
            Terminator::RepeatBounded{body_args,done_args,..}|
            Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.push(96);done_args.push(96);},
            Terminator::Yield{next_args,..}=>next_args.push(96),
            Terminator::Return{..}=>{}
        }
    }
    let ctx=||vec![Param{id:99,ty:Type::U32},Param{id:97,ty:Type::Bytes},
        Param{id:98,ty:Type::Bytes},Param{id:96,ty:Type::U32}];
    let reply=||{let mut x=ctx();x.push(Param{id:2,ty:Type::Bytes});x};
    // Non-yield targets that already carry extra parameters have the
    // status last; new targets always use the context order above.
    let fresh=f.blocks.iter_mut().find(|b|b.id=="offer_input_bound").unwrap();
    fresh.instructions.push(Instruction{id:95,op:Op::ConstU32(0)});
    if let Terminator::Branch{if_true,true_args,..}=&mut fresh.terminator{
        *if_true="receipt_fresh_read".into();*true_args=vec![99,1,2,95];
    }else{panic!("expected accepted new ID");}
    let ok=f.blocks.iter_mut().find(|b|b.id=="task_ok").unwrap();
    ok.instructions.push(Instruction{id:95,op:Op::ConstU32(1)});
    ok.terminator=Terminator::Jump{target:"episode_end".into(),args:vec![99,97,98,95]};
    let ret=f.blocks.iter_mut().find(|b|b.id=="episode_return").unwrap();
    ret.instructions=vec![Instruction{id:10,op:Op::Add(99,99)},
        Instruction{id:11,op:Op::Add(10,10)},Instruction{id:12,op:Op::Add(11,96)}];
    ret.terminator=Terminator::Return{value:12};
    let same=f.blocks.iter_mut().find(|b|b.id=="pending_same_input").unwrap();
    if cache{
        if let Terminator::Branch{if_true,..}=&mut same.terminator{*if_true="receipt_cached".into();}
        else{panic!("expected same input");}
    }
    let pending=f.blocks.iter_mut().find(|b|b.id=="pending_artifact_reply").unwrap();
    // NotFound still enters the ordinary paid resource gate; actual
    // existing bytes must pass the same identity verification as fresh IDs.
    if let Terminator::Branch{if_false,false_args,..}=&mut pending.terminator{
        *if_false="receipt_existing_code".into();
        *false_args=vec![99,97,98,96,2];
    }else{panic!("expected NotFound branch");}
    let key_ops=||vec![
        Instruction{id:10,op:lit(OFFER_OUTCOME.as_bytes())},
        Instruction{id:11,op:Op::HexEncode(98)},
        Instruction{id:12,op:Op::Concat(10,11)}
    ];
    f.blocks.extend([
        Block{id:"receipt_fresh_read".into(),params:ctx(),instructions:key_ops(),
            terminator:Terminator::Yield{call:WorldCall::Read,args:vec![12],
                next:"receipt_fresh_reply".into(),next_args:vec![99,97,98,96]}},
        Block{id:"receipt_fresh_reply".into(),params:reply(),instructions:vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"NotFound")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ],terminator:Terminator::Branch{condition:23,if_true:"episode_task".into(),
            true_args:vec![99,97,98,96],if_false:"receipt_existing_code".into(),
            false_args:vec![99,97,98,96,2]}},
        Block{id:"receipt_existing_code".into(),params:reply(),instructions:vec![
            Instruction{id:20,op:lit(b"code")},
            Instruction{id:21,op:Op::JsonFieldString(2,20)},
            Instruction{id:22,op:lit(b"Ok")},
            Instruction{id:23,op:Op::EqBytes(21,22)}
        ],terminator:Terminator::Branch{condition:23,if_true:"receipt_shape".into(),
            true_args:vec![99,97,98,96,2],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98,96]}},
        Block{id:"receipt_shape".into(),params:reply(),instructions:vec![
            Instruction{id:10,op:lit(b"payload")},
            Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:Op::Len(11)},
            Instruction{id:13,op:Op::ConstU32(280)},
            Instruction{id:14,op:Op::EqU32(12,13)}
        ],terminator:Terminator::Branch{condition:14,if_true:"receipt_digest".into(),
            true_args:vec![99,97,98,96,11],if_false:"receipt_collision".into(),
            false_args:vec![99,97,98,96]}},
        Block{id:"receipt_digest".into(),params:reply(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(1)},
            Instruction{id:11,op:Op::ConstU32(32)},
            Instruction{id:12,op:Op::Slice(2,10,11)},
            Instruction{id:13,op:Op::Sha256(97)},
            Instruction{id:14,op:Op::EqBytes(12,13)}
        ],terminator:Terminator::Branch{condition:14,if_true:"receipt_matched".into(),
            true_args:vec![99,97,98,96],if_false:"receipt_collision".into(),
            false_args:vec![99,97,98,96]}},
        Block{id:"receipt_matched".into(),params:ctx(),
            instructions:vec![Instruction{id:95,op:Op::ConstU32(1)}],
            terminator:Terminator::Jump{target:"offer_poll_header".into(),args:vec![99,97,98,95]}},
        Block{id:"receipt_collision".into(),params:ctx(),
            instructions:vec![Instruction{id:95,op:Op::ConstU32(2)}],
            terminator:Terminator::Jump{target:"offer_poll_header".into(),args:vec![99,97,98,95]}}
    ]);
    if cache{
        f.blocks.push(Block{id:"receipt_cached".into(),params:ctx(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(0)},
            Instruction{id:11,op:Op::EqU32(96,10)}
        ],terminator:Terminator::Branch{condition:11,if_true:"pending_artifact".into(),
            true_args:vec![99,97,98,96],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98,96]}});
    }
    p.validate().expect("typed content-bound artifact and receipt cache");
    p
}
fn receipt_offer(arm:&str,phase:usize,offset:u64)->(usize,bool){
    if arm=="SPACE"{return (0,false);}
    let slot=if offset==16{if phase==0{1}else{2}}else{0};
    (slot,arm=="ID_CONFLICT"&&slot==0&&(phase==1||offset==32))
}
fn receipt_episode(w:&mut World,root:&str,arm:&str,phase:usize)
    ->(LifetimeIndexProbe,Vec<(u64,u64,usize,bool,String)>){
    let start=w.state.step;
    let mut peak=LifetimeIndexProbe::default();peak.sample(w);
    let mut publications=Vec::new();
    while w.state.step<start+180{
        let offset=w.state.step-start;
        if (arm=="SPACE"&&[0,28].contains(&offset))||
            (arm!="SPACE"&&[0,16,32].contains(&offset)){
            let (slot,conflict)=receipt_offer(arm,phase,offset);
            if arm=="SPACE"&&offset==28{
                assert_eq!(paid_memory_value(w),(b"1".to_vec(),0));
                w.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity+=512;
            }
            offer_publish(w,slot);
            if conflict{
                let mut offer=offer_fixture(0);offer.held_out_input=offer_fixture(1).held_out_input;
                w.state.objects.get_mut(OFFER_PUBLIC).unwrap().bytes=serde_json::to_vec(&offer).unwrap();
            }
            publications.push((w.state.step,w.state.objects[OFFER_PUBLIC].version,
                slot,conflict,w.state_hash()));peak.sample(w);
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished){
            w.tick(None);peak.sample(w);
        }else{
            let r=w.tick_autonomous(ORGANISM_ID);peak.sample(w);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        }
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(root));
    }
    assert_eq!(b_gas(w),20);
    (peak,publications)
}
fn assert_bound_artifact(bytes:&[u8],slot:usize){
    use sha2::{Digest,Sha256};
    let input=offer_fixture(slot).held_out_input;
    assert_eq!(bytes.len(),280);
    assert_eq!(bytes[0],hidden_transform(129,&input) as u8);
    assert_eq!(&bytes[1..33],Sha256::digest(&input).as_slice());
    assert!(bytes[33..].iter().all(|x|*x==0x55));
}
#[test]
#[ignore="frozen durable receipt identity and continuation caching"]
fn s29_content_bound_receipt_native(){
    receipt_native_impl(32,false,false,false);
}

#[test]
#[ignore="frozen bounded receipt polling budget revision"]
fn s29_bounded_receipt_poll_native(){
    receipt_native_impl(20,false,false,false);
}

#[test]
#[ignore="original receipt cache independently observed"]
fn s29_original_receipt_cache_native(){
    receipt_native_impl(32,true,false,false);
}

#[test]
#[ignore="frozen generic own-prefix receipt observation"]
fn s29_paid_prefix_receipt_native(){
    receipt_native_impl(32,false,true,false);
}

fn prefix_receipt_root(cache:bool)->Program{
    let mut p=bound_receipt_root(cache);
    let f=&mut p.functions[0];
    for id in ["receipt_fresh_read","pending_artifact"]{
        let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
        b.instructions.iter_mut().find(|i|i.id==10).unwrap().op=
            lit(b"/sys/self-prefix/33/outcomes/s29-offer-");
    }
    let b=f.blocks.iter_mut().find(|b|b.id=="receipt_shape").unwrap();
    b.instructions.iter_mut().find(|i|i.id==13).unwrap().op=Op::ConstU32(37);
    if let Terminator::Branch{if_true,..}=&mut b.terminator{*if_true="receipt_prefix_size".into();}
    else{panic!("expected receipt shape guard");}
    let params=b.params.clone();
    f.blocks.push(Block{id:"receipt_prefix_size".into(),params,instructions:vec![
        Instruction{id:10,op:Op::ConstU32(0)},
        Instruction{id:11,op:Op::ConstU32(4)},
        Instruction{id:12,op:Op::Slice(2,10,11)},
        Instruction{id:13,op:lit(&280u32.to_le_bytes())},
        Instruction{id:14,op:Op::EqBytes(12,13)}
    ],terminator:Terminator::Branch{condition:14,if_true:"receipt_digest".into(),
        true_args:vec![99,97,98,96,2],if_false:"receipt_collision".into(),
        false_args:vec![99,97,98,96]}});
    let b=f.blocks.iter_mut().find(|b|b.id=="receipt_digest").unwrap();
    b.instructions.iter_mut().find(|i|i.id==10).unwrap().op=Op::ConstU32(5);
    p.validate().expect("typed generic prefix receipt verifier");p
}

#[test]
#[ignore="frozen exact public payload memo at unchanged sensing cadence"]
fn s29_public_payload_memo_native(){
    receipt_native_impl(32,false,true,true);
}

fn public_payload_memo_root()->Program{
    let mut p=prefix_receipt_root(true);
    let f=&mut p.functions[0];
    let yields=f.blocks.iter().filter_map(|b|match &b.terminator{
        Terminator::Yield{next,..}=>Some(next.clone()),_=>None
    }).collect::<std::collections::BTreeSet<_>>();
    for b in &mut f.blocks{
        assert!(b.params.iter().all(|p|p.id!=94)&&b.instructions.iter().all(|i|i.id!=94));
        if b.id=="entry"{b.instructions.push(Instruction{id:94,op:lit(b"")});}
        else{
            let pos=if yields.contains(&b.id){b.params.len()-1}else{b.params.len()};
            b.params.insert(pos,Param{id:94,ty:Type::Bytes});
        }
        match &mut b.terminator{
            Terminator::Jump{args,..}=>args.push(94),
            Terminator::Branch{true_args,false_args,..}=>{true_args.push(94);false_args.push(94);},
            Terminator::RepeatBounded{body_args,done_args,..}|
            Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.push(94);done_args.push(94);},
            Terminator::Yield{next_args,..}=>next_args.push(94),
            Terminator::Return{..}=>{}
        }
    }
    let ctx=||vec![Param{id:99,ty:Type::U32},Param{id:97,ty:Type::Bytes},
        Param{id:98,ty:Type::Bytes},Param{id:96,ty:Type::U32},Param{id:94,ty:Type::Bytes}];
    let with_payload=||{let mut ps=ctx();ps.push(Param{id:11,ty:Type::Bytes});ps};
    let b=f.blocks.iter_mut().find(|b|b.id=="offer_parse").unwrap();
    assert_eq!(b.instructions.iter().map(|i|i.id).collect::<Vec<_>>(),vec![10,11,12,13,14]);
    let parse_ops=b.instructions[2..].to_vec();
    let mut parse_term=b.terminator.clone();
    if let Terminator::Branch{true_args,false_args,..}=&mut parse_term{
        assert_eq!(true_args.pop(),Some(94));assert_eq!(false_args.pop(),Some(94));
        true_args.push(11);false_args.push(11);
    }else{panic!("expected exact original offer parser");}
    b.instructions.truncate(2);
    b.instructions.push(Instruction{id:15,op:Op::EqBytes(11,94)});
    b.terminator=Terminator::Branch{condition:15,if_true:"memo_check_status".into(),
        true_args:vec![99,97,98,96,94,11],if_false:"memo_parse".into(),
        false_args:vec![99,97,98,96,94,11]};
    f.blocks.extend([
        Block{id:"memo_check_status".into(),params:with_payload(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(0)},Instruction{id:12,op:Op::EqU32(96,10)}
        ],terminator:Terminator::Branch{condition:12,if_true:"memo_parse".into(),
            true_args:vec![99,97,98,96,94,11],if_false:"offer_poll_header".into(),
            false_args:vec![99,97,98,96,94]}},
        Block{id:"memo_parse".into(),params:with_payload(),instructions:parse_ops,terminator:parse_term}
    ]);
    p.validate().expect("typed exact public payload memo");p
}

#[test]
#[ignore="frozen resource-accounted memo equivalence revision"]
fn s29_public_memo_resource_accounting_native(){
    assert_eq!(std::env::var("OCEE_S29_RECEIPT_ARM").unwrap(),"SPACE");
    receipt_native_impl_accounted(32,false,true,true,true);
}

fn receipt_native_impl(poll_limit:u32,cache_only:bool,prefix:bool,memo:bool){
    receipt_native_impl_accounted(poll_limit,cache_only,prefix,memo,false);
}

fn receipt_native_impl_accounted(poll_limit:u32,cache_only:bool,prefix:bool,memo:bool,accounted:bool){
    let arm=std::env::var("OCEE_S29_RECEIPT_ARM").unwrap();
    assert!(matches!(arm.as_str(),"SPACE"|"REORDER"|"ID_CONFLICT"));
    let (mut base,_,target,_,registered)=lifetime_setup(129);
    let before=spend(&base);
    let roots=(if memo{[prefix_receipt_root(true),public_payload_memo_root()]}
        else if prefix{[prefix_receipt_root(false),prefix_receipt_root(true)]}
        else{[bound_receipt_root(false),bound_receipt_root(true)]}).into_iter()
        .map(|mut p|{
            if poll_limit!=32{
                let b=p.functions[0].blocks.iter_mut().find(|b|b.id=="offer_poll_header").unwrap();
                if let Terminator::RepeatScoped{limit,aggregate_limit,..}=&mut b.terminator{
                    assert_eq!((*limit,*aggregate_limit),(32,96));
                    *limit=poll_limit;*aggregate_limit=3*poll_limit;
                }else{panic!("expected bounded polling header");}
                p.validate().expect("typed revised polling root");
            }
            (p.content_hash().unwrap(),p.canonical_bytes())
        }).collect::<Vec<_>>();
    let total=roots.iter().map(|(_,b)|b.len() as u64+64).sum::<u64>();
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+total;
    quota_wait_next_grant(&mut base);
    for (i,(h,b)) in roots.iter().enumerate(){
        assert!(!b.windows(64).any(|x|x==registered.as_bytes()||x==target.as_bytes()));
        put(&mut base,version_key(h),b.clone());
        put(&mut base,format!("/self/pins/s29-receipt-root-{i}"),h.as_bytes().to_vec());
    }
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+if arm=="SPACE"{195}else{1200};
    quota_wait_next_grant(&mut base);
    let after=spend(&base);let base_hash=base.state_hash();
    let inventory=lifetime_extra_versions(&base);
    let mut measurements=Vec::new();
    let mut baseline=Vec::new();
    for (policy,(root,raw)) in roots.iter().enumerate(){
        if cache_only&&policy==0{continue;}
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        for phase in 0..if arm=="SPACE"{1}else{2}{
            if phase==1{quota_wait_next_grant(&mut w);}
            lifetime_set_active(&mut w,root);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let start=w.state.step;let start_a=w.state.nodes[&NodeId::A].gas_available;
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let (peak,pubs)=receipt_episode(&mut w,root,&arm,phase);
            // Observational diagnostic only: preserve the frozen endpoint and all assertions.
            let org=&w.state.organisms[&ORGANISM_ID];
            let mut codes=std::collections::BTreeMap::<String,usize>::new();
            let mut actions=std::collections::BTreeMap::<String,usize>::new();
            let mut trace=Vec::new();
            let mut vm_a=0u64;let mut action_a=0u64;
            for e in w.audit.iter().filter(|e|e.step>=start){
                *codes.entry(format!("{:?}",e.result.code)).or_default()+=1;
                vm_a+=e.result.vm_gas_charged;action_a+=e.result.gas_charged;
                let kind=match &e.action{
                    Some(Action::Read{key,..})=>format!("Read:{key}"),
                    Some(Action::Write{key,..})=>format!("Write:{key}"),
                    Some(Action::Exec{..})=>"Exec".to_string(),
                    Some(Action::IndexVersionRefs{..})=>"IndexVersionRefs".to_string(),
                    Some(Action::ReclaimIndexedVersion{..})=>"ReclaimIndexedVersion".to_string(),
                    Some(_)=>"Other".to_string(),None=>"NoAction".to_string()
                };
                *actions.entry(kind.clone()).or_default()+=1;
                trace.push((e.step,kind,format!("{:?}",e.result.code),e.result.vm_gas_charged,e.result.gas_charged));
            }
            let continuation=serde_json::to_value(&org.vm_continuation).unwrap();
            let continuation_summary=continuation.as_object().map(|m|m.iter()
                .map(|(k,v)|(k.clone(),if v.is_array(){serde_json::json!({"array_len":v.as_array().unwrap().len()})}
                    else if v.is_object(){serde_json::json!({"object_keys":v.as_object().unwrap().keys().collect::<Vec<_>>()})}
                    else{v.clone()})).collect::<std::collections::BTreeMap<_,_>>());
            println!("S29_RECEIPT_DIAG arm={arm} policy={policy} phase={phase} root={root} start={start} end={} start_A={start_a} remaining_A={} last_return={:?} continuation={continuation_summary:?} codes={codes:?} actions={actions:?} VM_A={vm_a} world_A={action_a} first={:?} tail={:?}",
                w.state.step,w.state.nodes[&NodeId::A].gas_available,org.last_return,
                &trace[..trace.len().min(20)],&trace[trace.len().saturating_sub(24)..]);
            let (rp,rpubs)=receipt_episode(&mut replay,root,&arm,phase);
            assert_eq!(peak,rp);assert_eq!(pubs,rpubs);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            let expected_slots=if arm=="SPACE"{vec![0,0]}else if phase==0{vec![0,1]}else{vec![2]};
            let terminal=4*(expected_slots.len() as u32)+if arm=="ID_CONFLICT"{2}else{1};
            assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(terminal)));
            let mut tasks=Vec::new();let mut execs=Vec::new();let mut lookups=Vec::new();
            let mut poll_a=0u64;let mut lookup_a=0u64;let mut exec_a=0u64;let mut task_a=0u64;
            let mut vm=0u64;let mut world=0u64;let mut indexes=0;
            let mut marks=Vec::new();
            for e in w.audit.iter().filter(|e|e.step>=start){
                vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
                match &e.action{
                    Some(Action::Read{key,..}) if key==OFFER_PUBLIC=>{
                        assert_eq!(e.result.code,Code::Ok);poll_a+=e.result.gas_charged;
                    },
                    Some(Action::Read{key,..}) if key.starts_with(OFFER_OUTCOME)||
                        key.starts_with("/sys/self-prefix/33/outcomes/s29-offer-")=>{
                        assert!(matches!(e.result.code,Code::Ok|Code::NotFound));
                        let normalized=if prefix{
                            let suffix=key.strip_prefix("/sys/self-prefix/33/outcomes/s29-offer-").unwrap();
                            if e.result.code==Code::Ok{
                                assert_eq!(e.result.gas_charged,3);
                                assert_eq!(e.result.payload.len(),37);
                                assert_eq!(&e.result.payload[..4],&280u32.to_le_bytes());
                            }
                            format!("{OFFER_OUTCOME}{suffix}")
                        }else{assert!(key.starts_with(OFFER_OUTCOME));key.clone()};
                        lookup_a+=e.result.gas_charged;lookups.push((e.step,normalized,e.result.code));
                    },
                    Some(Action::Exec{program_hash,input,..})=>{
                        assert_eq!(program_hash,&registered);
                        let slot=expected_slots[execs.len()];
                        assert_eq!(input,&offer_fixture(slot).held_out_input);
                        assert_eq!(e.result.code,Code::Ok);
                        assert_eq!(serde_json::from_slice::<Value>(&e.result.payload).unwrap(),
                            Value::U32(hidden_transform(129,input)));
                        execs.push((e.step,slot));exec_a+=e.result.gas_charged;
                    },
                    Some(Action::Write{key,bytes,..})=>{
                        assert_ne!(key,"/self/active");
                        if key.starts_with(OFFER_OUTCOME){
                            let slot=expected_slots[tasks.len()];
                            assert_eq!(key,&format!("{OFFER_OUTCOME}{:02x}",b'a'+slot as u8));
                            assert_bound_artifact(bytes,slot);
                            assert_eq!(execs.last().unwrap().1,slot);
                            let code=if arm=="SPACE"&&tasks.is_empty(){Code::Quota}else{Code::Ok};
                            assert_eq!(e.result.code,code);
                            tasks.push((e.step,slot,e.result.code,e.result.gas_charged));
                            task_a+=e.result.gas_charged;
                        }else if key==S29_DRY_KEY{
                            assert_eq!(e.result.code,Code::Ok);assert_eq!(bytes,b"1");marks.push(e.step);
                        }
                    },
                    Some(Action::IndexVersionRefs{..})=>{assert_eq!(e.result.code,Code::Ok);indexes+=1;},
                    Some(Action::ReclaimIndexedVersion{..})=>assert_eq!(e.result.code,Code::Conflict),
                    _=>{}
                }
            }
            assert_eq!(tasks.len(),expected_slots.len());assert_eq!(execs.len(),tasks.len());
            assert_eq!(indexes,usize::from(arm=="SPACE"));
            assert_eq!(marks.len(),indexes);
            if arm=="SPACE"{assert_eq!(paid_memory_value(&w),(b"1".to_vec(),0));}
            else{assert!(!w.state.objects.contains_key(S29_DRY_KEY));}
            for slot in 0..3{
                let key=format!("{OFFER_OUTCOME}{:02x}",b'a'+slot as u8);
                let exists=if arm=="SPACE"{slot==0}else{slot<=1||phase==1};
                assert_eq!(w.state.objects.contains_key(&key),exists);
                if exists{let obj=&w.state.objects[&key];assert_eq!(obj.version,0);assert_bound_artifact(&obj.bytes,slot);}
            }
            for h in &inventory{assert!(w.state.objects.contains_key(&version_key(h)));}
            assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
            let ok_reads=lookups.iter().filter(|(_,_,c)|*c==Code::Ok).count();
            let non_task=vm+world-task_a;
            if memo{
                let mut balance=start_a;
                let mut scans=0;
                let timeline=w.audit.iter().filter(|e|e.step>=start)
                    .map(|e|{
                        let mut payload=e.result.payload.clone();
                        let mut fee=e.result.gas_charged;
                        if accounted{
                            balance=balance.checked_sub(e.result.vm_gas_charged+fee).unwrap();
                            match &e.action{
                                Some(Action::Read{key,..}) if key=="/sys/resource-status"=>{
                                    assert_eq!(e.result.code,Code::Ok);assert_eq!(fee,2);
                                    let mut v:serde_json::Value=serde_json::from_slice(&payload).unwrap();
                                    assert_eq!(v["a_available"].as_u64(),Some(balance));
                                    v["a_available"]=serde_json::Value::Null;
                                    payload=serde_json::to_vec(&v).unwrap();
                                },
                                Some(Action::Observe{..})=>{
                                    assert_eq!(e.result.code,Code::Ok);assert_eq!(fee,1);
                                    let mut v:serde_json::Value=serde_json::from_slice(&payload).unwrap();
                                    assert_eq!(v[0].as_u64(),Some(e.step));
                                    assert_eq!(v[1].as_u64(),Some(balance));
                                    v[1]=serde_json::Value::Null;
                                    payload=serde_json::to_vec(&v).unwrap();
                                },
                                Some(Action::IndexVersionRefs{..})=>{
                                    assert_eq!(e.result.code,Code::Ok);
                                    let scanned=u64::from_le_bytes(payload.as_slice().try_into().unwrap());
                                    assert_eq!(fee,2+scanned.div_ceil(64)+inventory.len() as u64);
                                    let adjusted=scanned+raw.len() as u64;
                                    println!("S29_MEMO_INDEX_ACCOUNT policy={policy} step={} scanned_B={scanned} fee_A={fee} installed={} active_root_B={} adjusted_B={adjusted}",e.step,inventory.len(),raw.len());
                                    payload=adjusted.to_le_bytes().to_vec();
                                    fee=0; // only comparison projection; real fee stays in non_task
                                    scans+=1;
                                },
                                _=>{}
                            }
                        }
                        (e.step,e.action.clone(),e.result.code,payload,fee)
                    }).collect::<Vec<_>>();
                if accounted{
                    assert_eq!(scans,1);
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                }
                if policy==0{baseline.push((timeline,non_task));}
                else{
                    assert_eq!(timeline,baseline[phase].0,"exact world timeline must be preserved");
                    assert!(non_task<baseline[phase].1,"memo must reduce cost at equal world trajectory");
                }
                println!("S29_PUBLIC_MEMO arm={arm} phase={phase} policy={policy} root_B={} non_task_A={non_task} VM_A={vm} world_A={world} task_A={task_a} final_A={} matched={}",
                    raw.len(),w.state.nodes[&NodeId::A].gas_available,policy==1);
            }
            if arm=="SPACE"{measurements.push((ok_reads,non_task));}
            if arm!="SPACE"{
                // At least one real on-disk lookup of a repeated identity
                // after intervening offer, and after the explicit restart.
                assert!(lookups.iter().any(|(step,key,code)|*step>=start+32&&
                    key==&format!("{OFFER_OUTCOME}61")&&*code==Code::Ok));
                if phase==1{assert!(lookups.iter().any(|(step,key,code)|*step<start+16&&
                    key==&format!("{OFFER_OUTCOME}61")&&*code==Code::Ok));}
            }
            println!("S29_RECEIPT_DONE arm={arm} phase={phase} policy={} root={root} root_B={} base={base_hash} start={start} start_A={start_a} pubs={pubs:?} tasks={tasks:?} execs={execs:?} lookups={lookups:?} receipt_status={terminal} VM_A={vm} world_A={world} poll_A={poll_a} lookup_A={lookup_a} exec_A={exec_a} task_A={task_a} non_task_A={non_task} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} replay=true B=20",
                if memo{if policy==0{"CACHE_BASE"}else{"PUBLIC_MEMO"}}
                else if policy==0{"VERIFY"}else{"CACHE"},raw.len(),after.0-before.0,after.1-before.1,
                peak.storage_peak,peak.post_tick_peak);
        }
    }
    if arm=="SPACE"&&!cache_only&&!memo{
        assert_eq!(measurements.len(),2);
        assert!(measurements[1].0<measurements[0].0);
        assert!(measurements[1].1<measurements[0].1);
    }
}


// Pacing is executed by the real VM, not the host scheduler. Only a
// confirmed unchanged receipt enters the paid bounded delay.
fn feedback_paced_root()->Program{
    let mut p=prefix_receipt_root(true);
    let f=&mut p.functions[0];
    let h=f.blocks.iter_mut().find(|b|b.id=="offer_poll_header").unwrap();
    if let Terminator::RepeatScoped{limit,aggregate_limit,..}=&mut h.terminator{
        assert_eq!((*limit,*aggregate_limit),(32,96));
        *limit=12;*aggregate_limit=36;
    }else{panic!("expected scoped polling");}
    let b=f.blocks.iter_mut().find(|b|b.id=="receipt_cached").unwrap();
    if let Terminator::Branch{if_false,..}=&mut b.terminator{
        assert_eq!(if_false,"offer_poll_header");
        *if_false="paced_wait_0".into();
    }else{panic!("expected confirmed receipt branch");}
    let ctx=b.params.clone();
    assert_eq!(ctx.iter().map(|p|p.id).collect::<Vec<_>>(),vec![99,97,98,96]);
    for i in 0..3{
        let wait=format!("paced_wait_{i}");
        let reply=format!("paced_reply_{i}");
        let next=if i==2{"offer_poll_header".to_string()}else{format!("paced_wait_{}",i+1)};
        f.blocks.push(Block{id:wait,params:ctx.clone(),instructions:vec![],
            terminator:Terminator::Yield{call:WorldCall::Observe,args:vec![],
                next:reply.clone(),next_args:vec![99,97,98,96]}});
        let mut params=ctx.clone();params.push(Param{id:2,ty:Type::Bytes});
        f.blocks.push(Block{id:reply,params,instructions:vec![],
            terminator:Terminator::Jump{target:next,args:vec![99,97,98,96]}});
    }
    p.validate().expect("typed feedback-paced perception");p
}
// A finite feedback-conditioned counter, reset by real fresh identity.
fn adaptive_perception_root()->Program{
    let mut p=feedback_paced_root();
    let f=&mut p.functions[0];
    let yields=f.blocks.iter().filter_map(|b|match &b.terminator{
        Terminator::Yield{next,..}=>Some(next.clone()),_=>None
    }).collect::<std::collections::BTreeSet<_>>();
    for b in &mut f.blocks{
        assert!(b.params.iter().all(|p|p.id!=93&&p.id!=92)&&
            b.instructions.iter().all(|i|i.id!=93&&i.id!=92));
        if b.id=="entry"{b.instructions.push(Instruction{id:93,op:Op::ConstU32(0)});}
        else{
            let pos=if yields.contains(&b.id){b.params.len()-1}else{b.params.len()};
            b.params.insert(pos,Param{id:93,ty:Type::U32});
        }
        match &mut b.terminator{
            Terminator::Jump{args,..}=>args.push(93),
            Terminator::Branch{true_args,false_args,..}=>{true_args.push(93);false_args.push(93);},
            Terminator::RepeatBounded{body_args,done_args,..}|
            Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.push(93);done_args.push(93);},
            Terminator::Yield{next_args,..}=>next_args.push(93),
            Terminator::Return{..}=>{}
        }
    }
    let fresh=f.blocks.iter_mut().find(|b|b.id=="receipt_fresh_read").unwrap();
    fresh.instructions.push(Instruction{id:92,op:Op::ConstU32(0)});
    if let Terminator::Yield{next_args,..}=&mut fresh.terminator{
        assert_eq!(next_args.pop(),Some(93));next_args.push(92);
    }else{panic!("expected fresh receipt lookup");}
    let cache=f.blocks.iter_mut().find(|b|b.id=="receipt_cached").unwrap();
    if let Terminator::Branch{if_false,..}=&mut cache.terminator{
        assert_eq!(if_false,"paced_wait_0");*if_false="adaptive_repeat".into();
    }else{panic!("expected confirmed repeat");}
    let ctx=cache.params.clone();
    assert_eq!(ctx.iter().map(|p|p.id).collect::<Vec<_>>(),vec![99,97,98,96,93]);
    f.blocks.extend([
        Block{id:"adaptive_repeat".into(),params:ctx.clone(),instructions:vec![
            Instruction{id:10,op:Op::ConstU32(2)},Instruction{id:11,op:Op::EqU32(93,10)}
        ],terminator:Terminator::Branch{condition:11,if_true:"paced_wait_0".into(),
            true_args:vec![99,97,98,96,93],if_false:"adaptive_increment".into(),
            false_args:vec![99,97,98,96,93]}},
        Block{id:"adaptive_increment".into(),params:ctx,instructions:vec![
            Instruction{id:10,op:Op::ConstU32(1)},Instruction{id:11,op:Op::Add(93,10)}
        ],terminator:Terminator::Jump{target:"offer_poll_header".into(),
            args:vec![99,97,98,96,11]}}
    ]);
    p.validate().expect("typed finite perception backoff");p
}
fn paced_tape(arm:&str)->Vec<(u64,usize)>{
    match arm{
        "STABLE"=>vec![(0,0),(16,1)],
        "PULSE"=>vec![(0,0),(7,1),(9,0)],
        "LATE"=>vec![(0,0),(40,1)],
        "SHIFTED_PULSE"=>vec![(0,0),(24,1),(26,0)],
        _=>panic!("unknown frozen tape")
    }
}
fn paced_episode(w:&mut World,root:&str,tape:&[(u64,usize)])
    ->(LifetimeIndexProbe,Vec<(u64,usize,u64,String)>,Option<u64>){
    let start=w.state.step;let mut peak=LifetimeIndexProbe::default();peak.sample(w);
    let mut pubs=Vec::new();let mut finished=None;
    while w.state.step<start+120{
        let offset=w.state.step-start;
        if let Some((_,slot))=tape.iter().find(|(at,_)|*at==offset){
            offer_publish(w,*slot);
            pubs.push((w.state.step,*slot,w.state.objects[OFFER_PUBLIC].version,w.state_hash()));
            peak.sample(w);
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished){
            if finished.is_none(){finished=Some(w.state.step);}
            w.tick(None);
        }else{
            let r=w.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        }
        peak.sample(w);
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(root));
    }
    assert_eq!(b_gas(w),20);
    (peak,pubs,finished)
}
#[test]
#[ignore="frozen matched feedback-paced perception"]
fn s29_feedback_paced_perception_native(){
    paced_perception_impl(false);
}
#[test]
#[ignore="frozen finite feedback-conditioned perception backoff"]
fn s29_adaptive_perception_native(){
    paced_perception_impl(true);
}
fn paced_perception_impl(adaptive:bool){
    let arm=std::env::var("OCEE_S29_PACED_ARM").unwrap();
    let tape=paced_tape(&arm);
    let (mut base,_,target,_,registered)=lifetime_setup(129);
    let before=spend(&base);
    assert!(adaptive||arm!="SHIFTED_PULSE");
    let mut programs=vec![prefix_receipt_root(true),feedback_paced_root()];
    if adaptive{programs.push(adaptive_perception_root());}
    let roots=programs.into_iter()
        .map(|p|(p.content_hash().unwrap(),p.canonical_bytes())).collect::<Vec<_>>();
    let total=roots.iter().map(|(_,b)|b.len() as u64+64).sum::<u64>();
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+total;
    quota_wait_next_grant(&mut base);
    for (i,(h,b)) in roots.iter().enumerate(){
        assert!(!b.windows(64).any(|x|x==registered.as_bytes()||x==target.as_bytes()));
        put(&mut base,version_key(h),b.clone());
        put(&mut base,format!("/self/pins/s29-paced-root-{i}"),h.as_bytes().to_vec());
    }
    let used=base.state.organisms[&ORGANISM_ID].storage_used;
    base.state.nodes.get_mut(&NodeId::A).unwrap().storage_capacity=used+1200;
    quota_wait_next_grant(&mut base);
    let after=spend(&base);let base_hash=base.state_hash();
    let inventory=lifetime_extra_versions(&base);
    let mut common_start=None;let mut costs=Vec::new();
    for (policy,(root,raw)) in roots.iter().enumerate(){
        let paced=policy!=0;
        let expected_b=match arm.as_str(){
            "STABLE"=>true,"PULSE"=>policy!=1,"LATE"=>paced,
            "SHIFTED_PULSE"=>policy!=2,_=>unreachable!()};
        let label=["DENSE","PACED","ADAPT"][policy];
        let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
        lifetime_set_active(&mut w,root);
        let start=w.state.step;let start_a=w.state.nodes[&NodeId::A].gas_available;
        if let Some(prev)=common_start{assert_eq!((start,start_a),prev);}
        else{common_start=Some((start,start_a));}
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let (peak,pubs,finished)=paced_episode(&mut w,root,&tape);
        let (rp,rpubs,rf)=paced_episode(&mut replay,root,&tape);
        assert_eq!(peak,rp);assert_eq!(pubs,rpubs);assert_eq!(finished,rf);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        let mut reads=Vec::new();let mut execs=Vec::new();let mut tasks=Vec::new();
        let mut lookups=Vec::new();let mut observations=Vec::new();
        let mut vm=0u64;let mut world=0u64;let mut task_a=0u64;
        let mut poll_a=0u64;let mut observe_a=0u64;let mut lookup_a=0u64;
        let mut gas=0;
        for e in w.audit.iter().filter(|e|e.step>=start){
            vm+=e.result.vm_gas_charged;world+=e.result.gas_charged;
            if e.result.code==Code::Gas{gas+=1;}
            match &e.action{
                Some(Action::Read{key,..}) if key==OFFER_PUBLIC=>{
                    assert_eq!(e.result.code,Code::Ok);
                    let slot=tape.iter().rev().find(|(at,_)|start+*at<=e.step).unwrap().1;
                    assert_eq!(serde_json::from_slice::<TransformTask>(&e.result.payload).unwrap(),offer_fixture(slot));
                    reads.push((e.step,slot));poll_a+=e.result.gas_charged;
                },
                Some(Action::Read{key,..}) if key.starts_with("/sys/self-prefix/33/outcomes/s29-offer-")=>{
                    assert!(matches!(e.result.code,Code::Ok|Code::NotFound));
                    if e.result.code==Code::Ok{
                        assert_eq!(e.result.gas_charged,3);
                        assert_eq!(e.result.payload.len(),37);
                    }
                    lookups.push((e.step,key.clone(),e.result.code));lookup_a+=e.result.gas_charged;
                },
                Some(Action::Observe{..})=>{
                    assert!(paced);assert_eq!(e.result.code,Code::Ok);
                    observations.push(e.step);observe_a+=e.result.gas_charged;
                },
                Some(Action::Exec{program_hash,input,..})=>{
                    assert_eq!(program_hash,&registered);assert_eq!(e.result.code,Code::Ok);
                    let slot=execs.len();assert!(slot<2);
                    assert_eq!(input,&offer_fixture(slot).held_out_input);
                    assert_eq!(serde_json::from_slice::<Value>(&e.result.payload).unwrap(),
                        Value::U32(hidden_transform(129,input)));
                    assert!(reads.iter().any(|(at,s)|*at<e.step&&*s==slot));
                    execs.push((e.step,slot));
                },
                Some(Action::Write{key,bytes,..})=>{
                    let slot=tasks.len();assert!(slot<2);
                    assert_eq!(key,&format!("{OFFER_OUTCOME}{:02x}",b'a'+slot as u8));
                    assert_eq!(e.result.code,Code::Ok);assert_eq!(e.result.gas_charged,20);
                    assert_bound_artifact(bytes,slot);
                    tasks.push((e.step,slot));task_a+=e.result.gas_charged;
                },
                Some(Action::IndexVersionRefs{..})|Some(Action::ReclaimIndexedVersion{..})=>
                    panic!("roomy tape must not maintain"),
                _=>{}
            }
        }
        let non_task=vm+world-task_a;
        let latencies=tasks.iter().map(|(step,slot)|step-start-tape.iter().find(|(_,s)|s==slot).unwrap().0)
            .collect::<Vec<_>>();
        println!("S29_PACED arm={arm} policy={} root={root} root_B={} base={base_hash} start={start} start_A={start_a} final_A={} finished={finished:?} return={:?} pubs={pubs:?} reads={reads:?} observes={observations:?} lookups={lookups:?} execs={execs:?} tasks={tasks:?} latencies={latencies:?} gas={gas} VM_A={vm} world_A={world} task_A={task_a} non_task_A={non_task} poll_A={poll_a} observe_A={observe_a} lookup_A={lookup_a} setup_VM_A={} setup_world_A={} storage_peak={} index_peak_unbilled={} replay=true B=20",
            label,raw.len(),w.state.nodes[&NodeId::A].gas_available,
            w.state.organisms[&ORGANISM_ID].last_return,after.0-before.0,after.1-before.1,
            peak.storage_peak,peak.post_tick_peak);
        assert!(finished.is_some());assert_eq!(gas,0);
        let expected=if expected_b{2}else{1};
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(4*expected as u32+1)));
        assert_eq!(execs.len(),expected);assert_eq!(tasks.len(),expected);
        assert_eq!(reads.iter().any(|(_,s)|*s==1),expected_b);
        if paced{assert!(!observations.is_empty());}else{assert!(observations.is_empty());}
        if policy==2{
            assert!(reads.iter().any(|(t,_)|*t==start+5));
            assert!(reads.iter().any(|(t,_)|*t==start+6));
            assert!(reads.iter().any(|(t,_)|*t==start+7));
            println!("S29_ADAPTIVE arm={arm} first_reads={:?} first_wait={:?}",
                &reads[..reads.len().min(8)],observations.first());
        }
        for slot in 0..3{
            let key=format!("{OFFER_OUTCOME}{:02x}",b'a'+slot as u8);
            assert_eq!(w.state.objects.contains_key(&key),slot<expected);
            if slot<expected{let obj=&w.state.objects[&key];assert_eq!(obj.version,0);assert_bound_artifact(&obj.bytes,slot);}
        }
        for h in &inventory{assert!(w.state.objects.contains_key(&version_key(h)));}
        assert_eq!(w.state.objects[REGISTRY].bytes,registered.as_bytes());
        assert!(!w.state.objects.contains_key(S29_DRY_KEY));
        costs.push(non_task);
    }
    if arm=="STABLE"{assert!(costs[1]<costs[0]);if adaptive{assert!(costs[2]<costs[0]);}}
}
