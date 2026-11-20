//! S31: VM-computed count is on the CAUSAL path of *real* P1/P2 B frames.
//! Compare same initial world, same donor import, same number of real WRITE
//! ticks and same payload size. The only intervention changes the bytes of
//! a root-generated object after root has switched to a specialist.
//! The intervention is the researcher counterfactual, NOT organism action.
use ocee_organism_seeds::{
    s31_causal_remote::{s31_shared_root,s31_p1_count_consumer,
        s31_p2_count_consumer,PREPARED_COUNT},
    s30_composed_transfer::REGISTRY,
    s28_cross_utility::UTILITY_REGISTRY
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::{Family,Operation,ParseError};
use ocee_world_kernel::{
    Action,Code,Config,NodeId,WorkItem,World,ORGANISM_ID,
};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    })).code,Code::Ok);
}
fn gas(w:&World)->u64{
    w.audit.iter().map(|r|r.result.gas_charged+r.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn root()->(Program,Program,Program){
    let p1=s31_p1_count_consumer();
    let p2=s31_p2_count_consumer();
    let root=s31_shared_root(&p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    (p1,p2,root)
}
fn boot(family:Family,seed:u64)->(World,String){
    let (p1,p2,root)=root();
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    for p in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",p.content_hash().unwrap()),
            p.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run_until_switch(w:&mut World){
    for _ in 0..4_096{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "S31 root trap step={} code={:?}",w.state.step,r.code);
        if w.state.objects["/self/active"].version==1 {break;}
    }
    assert_eq!(w.state.objects["/self/active"].version,1,
        "the active root must autonomously switch to a family specialist");
    let count=&w.state.objects[PREPARED_COUNT];
    assert_eq!(count.bytes,&[3,0]);
    assert_eq!(count.version,0);
    let size_hash=String::from_utf8(
        w.state.objects[UTILITY_REGISTRY].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{size_hash}")].bytes).unwrap();
    assert!(matches!(p.functions[0].blocks[0].instructions[0].op,
        Op::Len(0)),"real generic length executable, not host raw length");
    let proof_exec=w.audit.iter().position(|r|r.result.code==Code::Ok
        && matches!(&r.action,Some(Action::Exec{program_hash,..})
            if program_hash==&size_hash))
        .expect("root must really EXEC the generated Len tool");
    let proof_write=w.audit.iter().position(|r|r.result.code==Code::Ok
        && matches!(&r.action,Some(Action::Write{key,..})
            if key==PREPARED_COUNT))
        .expect("root must WRITE result after real EXEC");
    assert!(proof_exec<proof_write);
    assert!(w.audit.iter().any(|r|r.result.code==Code::Ok &&
        matches!(&r.action,Some(Action::Write{key,..})
            if key==&format!("/self/versions/{size_hash}"))),
        "the candidate version itself must be VM-generated this episode");
}
fn run_to_finish(w:&mut World){
    for _ in 0..12_000 {
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "S31 specialized VM trap step={} code={:?}",w.state.step,r.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),
        "bounded specialist must terminate also when B rejects bad body");
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
}
fn work(w:&World)->WorkItem{
    serde_json::from_slice(&w.state.objects["/public/E001/job0"].bytes).unwrap()
}
fn final_b_requests(w:&World)->Vec<(u16,Vec<u8>)>{
    let p=&w.state.protocol_service.as_ref().unwrap().protocol;
    w.audit.iter().filter_map(|r|{
        if let Some(Action::Send{
            destination:NodeId::C,bytes,..
        })=&r.action{
            if let Ok(decoded)=p.decode(bytes){
                if decoded.operation==Operation::RemoteEval &&
                    decoded.body.len()==14{
                    return Some((
                        u16::from_le_bytes(decoded.body[..2].try_into().unwrap()),
                        decoded.body[2..].to_vec()
                    ));
                }
            }
        }
        None
    }).collect()
}
fn verify_success(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_gas(w),20);
    let requested=final_b_requests(w);
    assert!(!requested.is_empty(),"there must be a real C wire request");
    let job=work(w);
    assert!(requested.iter().any(|(n,bytes)|*n==3 && bytes==&job.values_le));
    assert!(w.audit.iter().any(|r|r.result.code==Code::Ok &&
        matches!(&r.action,Some(Action::Read{key,..})
            if key==PREPARED_COUNT) &&
        r.step>w.audit.iter().find(|a|
            matches!(&a.action,Some(Action::Write{key,..})
                if key=="/self/active" && a.step>=4)
        ).unwrap().step),"family specialist must consume root work object");
    let key=match family {
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!()
    };
    let expected=job.values.iter().enumerate().fold(0u32,|s,(i,&x)|
        s.wrapping_add(x.rotate_left((i%32) as u32)));
    let got=u32::from_le_bytes(w.state.objects[key].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(got,expected);
}
fn paid_set(w:&mut World,value:u16)->u64{
    let before=gas(w);
    let ret=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:PREPARED_COUNT.into(),
        bytes:value.to_le_bytes().to_vec(),expected_version:Some(0),
    }));
    assert_eq!(ret.code,Code::Ok);
    gas(w)-before
}
fn import_composed(w:&mut World,h:&str,p:&Program){
    put(w,format!("/self/versions/{h}"),p.canonical_bytes());
    put(w,REGISTRY.into(),h.as_bytes().to_vec());
}
#[test]
fn s31_program_generated_count_controls_real_b_payload_in_both_protocol_families(){
    // Real source P1 contributes its VM-generated *S30* graph in addition
    // to the VM-generated Len executable produced by every root episode.
    let (mut p1,root_hash)=boot(Family::P1,1);
    run_until_switch(&mut p1);
    let composed_hash=String::from_utf8(
        p1.state.objects[REGISTRY].bytes.clone()).unwrap();
    let composed:Program=serde_json::from_slice(
        &p1.state.objects[&format!("/self/versions/{composed_hash}")].bytes)
        .unwrap();
    let (mut p2,root_target)=boot(Family::P2,19);
    assert_eq!(root_hash,root_target);
    import_composed(&mut p2,&composed_hash,&composed);
    run_until_switch(&mut p2);
    run_to_finish(&mut p1);
    run_to_finish(&mut p2);
    verify_success(&p1,Family::P1);
    verify_success(&p2,Family::P2);
    assert_eq!(p2.state.objects[REGISTRY].bytes,
        composed_hash.as_bytes(),"P2 must reuse source P1 graph");
    println!("S31 real-wire source=P1_seed1 A={} B=20; target=P2_seed19 A={} B=20; root={root_hash}, composed={composed_hash}",
        gas(&p1),gas(&p2));

    // Surgical, schedule-matched counterfactual. Correct and poisoned P2
    // are bit-identical BEFORE intervention: same root, same import, same
    // discovery/Len EXEC, same B history (zero). The intervention is a
    // researcher-controlled WRITE, not an autonomous organism action.
    let (mut control,rc)=boot(Family::P2,19);
    let (mut poisoned,rp)=boot(Family::P2,19);
    assert_eq!(rc,rp);
    import_composed(&mut control,&composed_hash,&composed);
    import_composed(&mut poisoned,&composed_hash,&composed);
    run_until_switch(&mut control);
    run_until_switch(&mut poisoned);
    assert_eq!(control.state_hash(),poisoned.state_hash());
    assert_eq!(b_gas(&control),0);
    let paid_good=paid_set(&mut control,3);
    let paid_bad=paid_set(&mut poisoned,2);
    assert_eq!(paid_good,paid_bad,"identical paid WRITE gas");
    assert_eq!(control.state.step,poisoned.state.step);
    assert_eq!(control.state.nodes,poisoned.state.nodes);
    assert_eq!(control.state.organisms,poisoned.state.organisms);
    assert_eq!(control.state.objects.len(),poisoned.state.objects.len());
    run_to_finish(&mut control);
    run_to_finish(&mut poisoned);
    verify_success(&control,Family::P2);
    let good=final_b_requests(&control);
    let bad=final_b_requests(&poisoned);
    let public=work(&control);
    assert!(good.iter().any(|(n,values)|*n==3 && values==&public.values_le));
    assert!(bad.iter().any(|(n,values)|*n==2 && values==&public.values_le),
        "actual C wire payload must contain poisoned count, not constant 3");
    assert_eq!(b_gas(&poisoned),0,
        "B must reject mismatched body length before metered FOLD32");
    assert!(poisoned.state.service_events.iter().any(|e|
        e.diagnostic==Some(ParseError::BadBody)));
    assert!(!poisoned.state.objects.contains_key(
        "/self/results/p2-job0/value_le"),
        "bad input may not produce fabricated successful result");
    assert_ne!(poisoned.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    println!("S31 CAUSAL_INTERVENTION matched initial state, equal paid WRITE={paid_good} A gas; good(A={},ticks={},B=20,count=3) bad(A={},ticks={},B=0,count=2,diagnostic=BadBody)",
        gas(&control),control.state.step,gas(&poisoned),poisoned.state.step);
}
