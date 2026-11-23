//! Real P1->P2 inherited executable repair under public feedback.
//! Test host supplies ONLY a donor executable actually produced by a
//! preceding P1 organism; the target VM selects the new repair algorithm.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s29_public_transform::{
    s29_evidence_selected_root,s29_feedback_repair_root,
    s29_second_label_repair_root,
    s29_template,REGISTRY,PREDICTION
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    hidden_transform,Action,Code,Config,TransformTask,WorkItem,World,ORGANISM_ID
};

fn put(w:&mut World,path:String,bytes:Vec<u8>){
    let result=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:path,bytes,expected_version:None
    }));
    assert_eq!(result.code,Code::Ok,"every imported version pays");
}
fn boot(family:Family,seed:u64,repair:bool)->(World,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let p1_hash=p1.content_hash().unwrap();
    let p2_hash=p2.content_hash().unwrap();
    let root=if repair{
        s29_feedback_repair_root(&p1_hash,&p2_hash)
    }else{
        s29_evidence_selected_root(&p1_hash,&p2_hash)
    };
    let hash=root.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,42);
    for p in [p1,p2,root]{
        put(&mut world,
            format!("/self/versions/{}",p.content_hash().unwrap()),
            p.canonical_bytes());
    }
    put(&mut world,"/self/active".into(),hash.clone().into_bytes());
    (world,hash)
}
fn run(w:&mut World)->(usize,usize){
    let mut invalid=0usize;
    let mut denied=0usize;
    for _ in 0..16_384{
        let action=w.tick_autonomous(ORGANISM_ID);
        if action.code==Code::InvalidCode{invalid+=1;}
        if action.code==Code::Denied{denied+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation
        .as_ref().is_some_and(|c|c.finished),
        "bounded autonomous P1/P2 source or target must finish");
    assert_eq!(invalid,0);
    assert_eq!(denied,0);
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    (invalid,denied)
}
fn paid_a(w:&World)->(u64,u64){
    (
        w.audit.iter().map(|a|a.result.vm_gas_charged).sum(),
        w.audit.iter().map(|a|a.result.gas_charged).sum()
    )
}
fn bgas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn actual_donor(seed:u64)->(String,Program,usize,u64){
    let (mut donor,_)=boot(Family::P1,seed,false);
    run(&mut donor);
    assert_eq!(donor.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"donor must genuinely complete its P1 run");
    assert_eq!(bgas(&donor),20,"donor must genuinely use B");
    let hash=String::from_utf8(
        donor.state.objects[REGISTRY].bytes.clone()).unwrap();
    let bytes=&donor.state.objects[
        &format!("/self/versions/{hash}")].bytes;
    let program:Program=serde_json::from_slice(bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    assert!(donor.audit.iter().any(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{key,..})
            if key==&format!("/self/versions/{hash}"))),
        "source executable must be created by active P1 VM");
    let expected_op=if seed==42{Op::Len(0)}else{Op::XorFold(0)};
    assert_eq!(program.functions[0].blocks[0].instructions[1].op,
        expected_op);
    let calls=donor.audit.iter().filter(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Exec{program_hash,..})
            if program_hash==&hash)).count();
    assert_eq!(calls,3,"two public labels + sealed heldout");
    (hash,program,calls,donor.state.step)
}
fn repaired_len_hash()->String{
    let mut program=s29_template();
    program.functions[0].blocks[0].instructions[1].op=Op::Len(0);
    program.content_hash().unwrap()
}
fn audit_exec(w:&World,hash:&str)->Vec<(Vec<u8>,u32,u64)>{
    w.audit.iter().filter_map(|r|{
        let Some(Action::Exec{program_hash,input,..})=&r.action else{
            return None
        };
        if program_hash!=hash{return None;}
        assert_eq!(r.result.code,Code::Ok,
            "targeted donor/repaired tool must truly Exec Ok");
        let v:Value=serde_json::from_slice(&r.result.payload).unwrap();
        let Value::U32(n)=v else{panic!("pure op returned other type")};
        Some((input.clone(),n,r.result.gas_charged))
    }).collect()
}
fn registration(w:&World,expected_version:Option<u64>)
    ->Vec<Code>{
    w.audit.iter().filter_map(|a|match &a.action{
        Some(Action::Write{key,expected_version:v,..})
            if key==REGISTRY&&v==&expected_version=>{
                Some(a.result.code)
            },
        _=>None
    }).collect()
}

/// Four isolated paid native cases in a single narrowly scoped workflow.
#[test]
#[ignore = "S29 inherited wrong tool feedback repair and CAS branch controls"]
fn s29_inherited_semantics_repair_native(){
    let arm=std::env::var("OCEE_S29_REPAIR_ARM").expect("matrix arm");
    assert!(["FRESH","MATCHED","WRONG","STALE"].contains(&arm.as_str()));
    let donor=if arm=="FRESH"{None}else{
        Some(actual_donor(if arm=="MATCHED"{42}else{43}))
    };
    let (mut target,root_hash)=boot(Family::P2,129,true);
    let learned_hash=repaired_len_hash();
    let mut imported_gas=0u64;
    let mut imported_root=None;
    if let Some((hash,program,calls,source_step))=&donor{
        assert_eq!(*calls,3);
        assert!(*source_step>0);
        let pre=paid_a(&target);
        put(&mut target,
            format!("/self/versions/{hash}"),program.canonical_bytes());
        put(&mut target,REGISTRY.into(),hash.as_bytes().to_vec());
        let post=paid_a(&target);
        imported_gas=post.0+post.1-pre.0-pre.1;
        imported_root=Some(hash.clone());
        assert!(imported_gas>0);
        assert_eq!(target.state.objects[REGISTRY].version,0);
        assert_ne!(hash,&s29_template().content_hash().unwrap());
    }
    if arm=="STALE"{
        let old=imported_root.as_ref().unwrap();
        let ret=target.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:REGISTRY.into(),
            bytes:old.as_bytes().to_vec(),
            expected_version:Some(0)
        }));
        assert_eq!(ret.code,Code::Ok,
            "researcher explicitly creates a legal prior version update");
        assert_eq!(target.state.objects[REGISTRY].version,1);
    }
    let genesis=paid_a(&target);
    assert!(target.state.nodes[&ocee_world_kernel::NodeId::A]
        .gas_available<=12_000);
    let actor_start_step=target.state.step;
    let start_hash=target.state_hash();
    assert_eq!(start_hash,World::from_snapshot(&target.snapshot_json())
        .unwrap().state_hash());
    let (_,_) =run(&mut target);
    let original=target.state.organisms[&ORGANISM_ID].last_return.clone();
    let final_registry=target.state.objects.get(REGISTRY).map(|o|
        (String::from_utf8(o.bytes.clone()).unwrap(),o.version)
    );
    let repaired_calls=audit_exec(&target,&learned_hash);
    let imported_calls=imported_root.as_ref()
        .filter(|h|h.as_str()!=learned_hash)
        .map(|h|audit_exec(&target,h)).unwrap_or_default();
    // Count ONLY active target-program version writes, not the
    // researcher-paid donor import that happened before autonomy began.
    let w_version=target.audit.iter().filter(|r|
        r.step>actor_start_step && r.result.code==Code::Ok &&
        matches!(&r.action,Some(Action::Write{key,..})
            if key==&format!("/self/versions/{learned_hash}"))).count();
    let cas0=registration(&target,Some(0));
    let missing=registration(&target,None);
    let (vm_a,action_a)=paid_a(&target);
    let b=bgas(&target);
    let task:TransformTask=serde_json::from_slice(
        &target.state.objects["/public/E001/transform"].bytes).unwrap();
    let heldout=target.state.objects.get(PREDICTION).map(|x|
        serde_json::from_slice::<u32>(&x.bytes).unwrap());
    let expected=hidden_transform(129,&task.held_out_input);
    let remote=target.state.objects.get("/self/results/p2-job0/value_le")
        .map(|obj|{
            assert_eq!(obj.bytes.len(),4);
            u32::from_le_bytes(obj.bytes.as_slice().try_into().unwrap())
        });
    let work:WorkItem=serde_json::from_slice(
        &target.state.objects["/public/E001/job0"].bytes).unwrap();
    let correct=work.values.iter().enumerate().fold(0u32,|acc,(i,v)|
        acc.wrapping_add(v.rotate_left((i%32) as u32)));
    let denied=target.audit.iter().filter(|e|
        matches!(e.result.code,Code::Denied|Code::InvalidCode)).count();
    // A one-way rejection can leave an immutable orphan newly-produced
    // program version but MUST NOT publish it via the wrong registry.
    let generated=target.state.objects.contains_key(
        &format!("/self/versions/{learned_hash}"));
    println!(
        "S29_REPAIR_NATIVE arm={arm} root={root_hash} origin={start_hash} \\
         old_tool={imported_root:?} new_tool={learned_hash} \\
         genesis_VM={} genesis_action={} imported_A={imported_gas} \\
         registry={final_registry:?} cas0={cas0:?} missing={missing:?} \\
         old_exec={imported_calls:?} repaired_exec={repaired_calls:?} \\
         version_ok_writes={w_version} generated_version={generated} \\
         heldout={heldout:?} heldout_truth={expected} B={b} \\
         remote_result={remote:?} remote_truth={correct} \\
         A_vm={vm_a} A_action={action_a} A_total={} \\
         invalid_denied={denied} return={original:?} step={}",
        genesis.0,genesis.1,vm_a+action_a,target.state.step
    );
    assert_eq!(denied,0);
    assert!(target.state.guarded_restarts.is_empty());
    let expected_labels=task.examples.iter().map(|e|
        (e.input.clone(),e.output)).collect::<Vec<_>>();
    match arm.as_str(){
        "FRESH"=>{
            assert_eq!(original,Some(Value::U32(1)));
            assert_eq!(final_registry,Some((learned_hash.clone(),0)));
            assert!(imported_calls.is_empty());
            assert_eq!(repaired_calls.len(),3);
            assert_eq!(cas0,vec![Code::Conflict]);
            assert_eq!(missing,vec![Code::Ok]);
            assert_eq!(w_version,1);
        },
        "MATCHED"=>{
            assert_eq!(original,Some(Value::U32(1)));
            assert_eq!(final_registry,Some((learned_hash.clone(),0)));
            assert!(imported_calls.is_empty());
            assert_eq!(repaired_calls.len(),3);
            assert!(cas0.is_empty());
            assert_eq!(missing,vec![Code::Ok],
                "only the researcher donor import, no organism registry updates");
            assert_eq!(w_version,0,
                "the organism reused an inherited version");
        },
        "WRONG"=>{
            assert_eq!(original,Some(Value::U32(1)));
            assert_eq!(final_registry,Some((learned_hash.clone(),1)));
            assert_eq!(imported_calls.len(),1,
                "wrong tool must first yield real disagreeing feedback");
            assert_ne!(imported_calls[0].1,expected_labels[0].1);
            assert_eq!(repaired_calls.len(),3);
            assert_eq!(cas0,vec![Code::Ok]);
            assert_eq!(missing,vec![Code::Ok],
                "researcher donor import only; repair must use CAS0");
            assert_eq!(w_version,1);
        },
        "STALE"=>{
            assert_eq!(original,Some(Value::U32(0)));
            assert_eq!(final_registry,Some((
                imported_root.as_ref().unwrap().clone(),1)));
            assert_eq!(imported_calls.len(),1);
            assert_ne!(imported_calls[0].1,expected_labels[0].1);
            assert!(repaired_calls.is_empty());
            assert_eq!(cas0,vec![Code::Ok,Code::Conflict],
                "first Ok is researcher creating version1; second conflicts");
            assert_eq!(missing,vec![Code::Ok,Code::Conflict],
                "fallback creation cannot overwrite stale registry");
            assert_eq!(w_version,1,
                "the unadopted immutable version is still charged/retained");
            assert_eq!(heldout,None);
            assert_eq!(remote,None);
            assert_eq!(b,0);
        },
        _=>unreachable!()
    }
    if arm!="STALE"{
        for (i,(input,value,_cost)) in repaired_calls.iter().take(2).enumerate(){
            assert_eq!(*input,expected_labels[i].0);
            assert_eq!(*value,expected_labels[i].1);
        }
        assert_eq!(repaired_calls[2].0,task.held_out_input);
        assert_eq!(repaired_calls[2].1,expected);
        assert_eq!(heldout,Some(expected));
        assert_eq!(b,20,"real B must be used after successful repair");
        assert_eq!(remote,Some(correct),
            "real P2 result must match independent world task FOLD32");
    }
    assert_eq!(target.state_hash(),
        World::from_snapshot(&target.snapshot_json()).unwrap().state_hash());
}


fn boot_second_repair(seed:u64)->(World,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s29_second_label_repair_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    let hash=root.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,42);
    for p in [p1,p2,root]{
        put(&mut world,
            format!("/self/versions/{}",p.content_hash().unwrap()),
            p.canonical_bytes());
    }
    put(&mut world,"/self/active".into(),hash.clone().into_bytes());
    (world,hash)
}

/// Intentionally a *researcher-installed adversary*, NOT a tool that the
/// previous P1 donor organism learned. ByteAt(Bytes, index1) happens
/// to return 3 for the first target example [32,3,8] (Len==3), but
/// returns 2 for the second example [18,2,7,4] (Len==4).
/// The target active VM never receives the intended answer or this label.
fn first_collision_second_fault()->Program{
    let mut p=s29_template();
    let block=&mut p.functions[0].blocks[0];
    block.instructions[0].op=Op::ConstU32(1);
    block.instructions[1].op=Op::ByteAt(0,1);
    p.validate().expect("ordinary typed adversarial VM tool");
    p
}

/// Test four distinct *real charged* paths of the same bounded active root:
/// reuse actual donor, first-label donor mismatch, second-only mismatch,
/// and second-only mismatch with a stale CAS-protected version.
#[test]
#[ignore = "charged second public example collision and feedback repair"]
fn s29_second_label_mismatch_native(){
    let arm=std::env::var("OCEE_S29_SECOND_ARM").expect("scoped native arm");
    assert!(["MATCHED","FIRST_MISMATCH","SECOND_MISMATCH","SECOND_STALE"]
        .contains(&arm.as_str()));
    let donor=match arm.as_str(){
        "MATCHED"=>Some(actual_donor(42)),
        "FIRST_MISMATCH"=>Some(actual_donor(43)),
        _=>None
    };
    let adversary=first_collision_second_fault();
    let adversarial_hash=adversary.content_hash().unwrap();
    let new_hash=repaired_len_hash();
    assert_ne!(adversarial_hash,new_hash);
    let (mut w,root_hash)=boot_second_repair(129);
    let inherited_hash=if let Some((hash,program,_,_))=&donor{
        put(&mut w,
            format!("/self/versions/{hash}"),program.canonical_bytes());
        hash.clone()
    }else{
        put(&mut w,format!("/self/versions/{adversarial_hash}"),
            adversary.canonical_bytes());
        adversarial_hash.clone()
    };
    assert_ne!(inherited_hash,
        s29_template().content_hash().unwrap());
    put(&mut w,REGISTRY.into(),inherited_hash.as_bytes().to_vec());
    assert_eq!(w.state.objects[REGISTRY].version,0);
    if arm=="SECOND_STALE"{
        let ret=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:REGISTRY.into(),
            expected_version:Some(0),
            bytes:inherited_hash.as_bytes().to_vec()
        }));
        assert_eq!(ret.code,Code::Ok);
        assert_eq!(w.state.objects[REGISTRY].version,1);
    }
    let start_step=w.state.step;
    let prehash=w.state_hash();
    assert_eq!(prehash,World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    let (init_vm,init_action)=paid_a(&w);
    assert_eq!(init_vm,0);
    run(&mut w);
    let last=w.state.organisms[&ORGANISM_ID].last_return.clone();
    let registry=w.state.objects.get(REGISTRY)
        .map(|o|(String::from_utf8(o.bytes.clone()).unwrap(),o.version));
    let old_exec=if inherited_hash==new_hash{
        Vec::new()
    }else{audit_exec(&w,&inherited_hash)};
    let new_exec=audit_exec(&w,&new_hash);
    let targeted_version=format!("/self/versions/{new_hash}");
    let new_version_writes=w.audit.iter().filter(|r|
        r.step>start_step&&r.result.code==Code::Ok &&
        matches!(&r.action,Some(Action::Write{key,..})
            if key==&targeted_version)).count();
    let actual_registry_cas0=registration(&w,Some(0));
    let actual_registry_create=registration(&w,None);
    let transform_reads=w.audit.iter().filter(|a|
        a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Read{key,..})
            if key=="/public/E001/transform")).count();
    let transform:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let samples=transform.examples.iter()
        .map(|x|(x.input.clone(),x.output)).collect::<Vec<_>>();
    assert_eq!(samples.len(),2);
    assert_eq!(samples[0],(vec![32,3,8],3));
    assert_eq!(samples[1],(vec![18,2,7,4],4));
    let heldout=w.state.objects.get(PREDICTION)
        .map(|o|serde_json::from_slice::<u32>(&o.bytes).unwrap());
    let heldout_truth=hidden_transform(129,&transform.held_out_input);
    let remote=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|o|u32::from_le_bytes(o.bytes.as_slice().try_into().unwrap()));
    let work:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let remote_truth=work.values.iter().enumerate().fold(0u32,|acc,(i,x)|
        acc.wrapping_add(x.rotate_left((i%32) as u32)));
    let (vm_a,action_a)=paid_a(&w);
    let b=bgas(&w);
    let invalid=w.audit.iter().filter(|r|
        matches!(r.result.code,Code::InvalidCode|Code::Denied)).count();
    println!(
        "S29_SECOND arm={arm} root={root_hash} initial={prehash} \\
         old={inherited_hash} new={new_hash} \\
         initial_A={} initial_vm={} old_exec={old_exec:?} \\
         new_exec={new_exec:?} new_version_writes={new_version_writes} \\
         cas0={actual_registry_cas0:?} create={actual_registry_create:?} \\
         reads_public_transform={transform_reads} registry={registry:?} \\
         heldout={heldout:?} truth_heldout={heldout_truth} \\
         B={b} remote={remote:?} remote_truth={remote_truth} \\
         vm_A={vm_a} action_A={action_a} total_A={} \\
         invalid={invalid} result={last:?} world_step={}",
        init_vm+init_action,init_vm,vm_a+action_a,w.state.step
    );
    assert_eq!(invalid,0);
    assert!(w.state.guarded_restarts.is_empty());
    let second_only=arm=="SECOND_MISMATCH"||arm=="SECOND_STALE";
    assert_eq!(transform_reads,if second_only{2}else{1},
        "second mismatch must actually pay a new public task Read");
    if second_only{
        assert_eq!(old_exec.len(),2,"both old Exec replies must be genuine");
        assert_eq!(old_exec[0].0,samples[0].0);
        assert_eq!(old_exec[0].1,samples[0].1,
            "first-example collision is required");
        assert_eq!(old_exec[1].0,samples[1].0);
        assert_eq!(old_exec[1].1,2,"second example must contradict label 4");
        assert_ne!(old_exec[1].1,samples[1].1);
    }
    match arm.as_str(){
        "MATCHED"=>{
            assert!(old_exec.is_empty());
            assert_eq!(new_exec.len(),3);
            assert_eq!(new_version_writes,0,
                "paid donor import is not target program generation");
            assert!(actual_registry_cas0.is_empty());
            assert_eq!(actual_registry_create,vec![Code::Ok]);
            assert_eq!(registry,Some((new_hash.clone(),0)));
        },
        "FIRST_MISMATCH"=>{
            assert_eq!(old_exec.len(),1);
            assert_eq!(old_exec[0].1,43);
            assert_eq!(new_exec.len(),3);
            assert_eq!(new_version_writes,1);
            assert_eq!(actual_registry_cas0,vec![Code::Ok]);
            assert_eq!(registry,Some((new_hash.clone(),1)));
        },
        "SECOND_MISMATCH"=>{
            assert_eq!(new_exec.len(),3);
            assert_eq!(new_version_writes,1);
            assert_eq!(actual_registry_cas0,vec![Code::Ok]);
            assert_eq!(registry,Some((new_hash.clone(),1)));
        },
        "SECOND_STALE"=>{
            assert!(new_exec.is_empty(),
                "the CAS rejected adoption before repaired Exec");
            assert_eq!(new_version_writes,1,
                "orphan candidate is paid, not a rolled back transaction");
            assert_eq!(actual_registry_cas0,
                vec![Code::Ok,Code::Conflict],
                "first Ok was researcher version bump, second active CAS0");
            assert_eq!(actual_registry_create,
                vec![Code::Ok,Code::Conflict]);
            assert_eq!(registry,Some((inherited_hash.clone(),1)));
            assert_eq!(heldout,None);
            assert_eq!(remote,None);
            assert_eq!(b,0);
            assert_eq!(last,Some(Value::U32(0)));
        },
        _=>unreachable!()
    }
    if arm!="SECOND_STALE"{
        for (i,(input,out,gas)) in new_exec.iter().take(2).enumerate(){
            assert_eq!(*input,samples[i].0);
            assert_eq!(*out,samples[i].1);
            assert!(*gas>0);
        }
        assert_eq!(new_exec[2].0,transform.held_out_input);
        assert_eq!(heldout,Some(heldout_truth));
        assert_eq!(b,20,"real P2 B task must complete after repair");
        assert_eq!(remote,Some(remote_truth));
        assert_eq!(last,Some(Value::U32(1)));
    }
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
}
