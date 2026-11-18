//! S10: an active probe detects an incorrect *successful* tool execution.
//! This is supervised by a simulated environmental evaluator, not evidence
//! of independent truth discovery. Exact initial VM identity stays fixed.
use ocee_organism_seeds::s10_semantic::{
    s10_active_semantic_probe,s10_no_semantic_evidence_control,s10_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,BoundaryWitness,Code,Config,World,ORGANISM_ID,
    hidden_compose_boundary
};

fn run(seed:u64,p:Program)->World {
    p.validate().unwrap();
    let hash=p.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:p.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    assert!(!world.state.objects.contains_key("/public/E001/semantic-witness"));
    for _ in 0..4096 {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::InvalidCode {
            assert!(matches!(world.audit.last().and_then(|r|r.action.as_ref()),
                Some(Action::Exec{..})),"active VM must not crash");
        }
        assert_ne!(result.code,Code::Denied);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){return world;}
    }
    panic!("S10 did not finish within 4096 ticks, seed {seed}");
}
fn tool(world:&World)->(String,Program){
    let hash=String::from_utf8(world.state.objects[
        "/self/tools/transform-v1"].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
#[test]
fn active_probe_repairs_silent_wrong_output_and_observed_faults_but_abstains_without_witness(){
    let initial=s10_active_semantic_probe();
    let hash=initial.content_hash().unwrap();
    assert_eq!(s10_manifest().program_hash,format!("sha256:{hash}"));

    for (seed,expected_state) in [(17u64,2u32),(2,2),(5,2),(0,1),(20,3),(23,3)] {
        let w=run(seed,initial.clone());
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash.clone()),"no host-driven VM replacement allowed");
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expected_state)),"seed {seed} state");
        assert!(w.state.organisms[&ORGANISM_ID].storage_used<=256*1024);
        let witness:BoundaryWitness=serde_json::from_slice(
            &w.state.objects["/public/E001/semantic-witness"].bytes).unwrap();
        assert_eq!(witness.observed_after_step,
            w.audit.iter().find(|r|matches!(
                &r.action,Some(Action::Exec{input,..}) if input.is_empty()
            )).expect("actual empty-input probe required").step);
        let read=w.audit.iter().find(|r|matches!(
            &r.action,Some(Action::Read{key,..})
            if key=="/public/E001/semantic-witness"
        )).expect("individual must explicitly read the witness");
        assert!(read.step>witness.observed_after_step);

        if expected_state==2 {
            assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
            let (_,p)=tool(&w);
            let expected=hidden_compose_boundary(seed,&[]);
            assert_eq!(run_isolated(&p,vec![],2000).unwrap().0,
                Value::U32(expected));
            assert_eq!(w.state.objects.contains_key(
                "/self/repair/semantic/observation"),true);
            for input in [&[99,0,13][..],&[1,2,3,4,5,6][..]]{
                assert_eq!(run_isolated(&p,input.to_vec(),2000).unwrap().0,
                    Value::U32(hidden_compose_boundary(seed,input)));
            }
            if seed==17 {
                let observed:ActionResult=serde_json::from_slice(
                    &w.state.objects["/self/repair/semantic/observation"].bytes
                ).unwrap();
                assert_eq!(observed.code,Code::Ok,
                    "the reported mismatch must be a successful, non-faulting execution");
                let actual:Value=serde_json::from_slice(&observed.payload).unwrap();
                assert_eq!(actual,Value::U32(0));
                assert_eq!(expected,1);
                assert!(w.audit.iter().any(|r|matches!(
                    &r.action,Some(Action::Exec{input,..})
                    if input.is_empty()&&r.result.code==Code::Ok
                )));
            }
        } else {
            assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
            assert!(!w.state.objects.contains_key("/self/repair/semantic/observation"));
            if expected_state==3 {
                assert!(w.state.objects.contains_key("/self/repair/boundary/status"));
            } else {
                assert!(!w.state.objects.contains_key("/self/repair/boundary/status"));
                let (_,p)=tool(&w);
                assert_eq!(run_isolated(&p,vec![],2000).unwrap().0,
                    Value::U32(hidden_compose_boundary(seed,&[])));
            }
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn without_semantic_evidence_a_successful_but_wrong_tool_remains_undetected(){
    let w=run(17,s10_no_semantic_evidence_control());
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    let (_,p)=tool(&w);
    assert_eq!(run_isolated(&p,vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(17,&[]),1);
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
    assert!(!w.audit.iter().any(|r|matches!(
        &r.action,Some(Action::Read{key,..})
        if key=="/public/E001/semantic-witness"
    )));
}

#[test]
fn consistent_false_semantic_witness_poisoning_remains_possible(){
    // Researcher-only adversarial fixture, not a permission granted to VM.
    // S10's active probe detects a silent mismatch, but the same correlated
    // authority can still provide a mutually consistent false answer.
    let p=s10_active_semantic_probe();
    let hash=p.content_hash().unwrap();
    let seed=17u64;
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:p.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    let mut injected=false;
    for _ in 0..4096 {
        let result=w.tick_autonomous(ORGANISM_ID);
        assert_ne!(result.code,Code::Denied);
        if !injected && w.state.objects.contains_key("/public/E001/semantic-witness"){
            let obj=w.state.objects.get_mut("/public/E001/semantic-witness").unwrap();
            let mut witness:BoundaryWitness=serde_json::from_slice(&obj.bytes).unwrap();
            assert_eq!((witness.first,witness.second),(Some(1),Some(1)));
            // Both observations are false, but match the faulty tool's Ok(0).
            witness.first=Some(0);
            witness.second=Some(0);
            obj.bytes=serde_json::to_vec(&witness).unwrap();
            injected=true;
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    assert!(injected);
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation
        .as_ref().is_some_and(|c|c.finished));
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
    let (_,adopted)=tool(&w);
    assert_eq!(run_isolated(&adopted,vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(seed,&[]),1);
    assert!(w.state.organisms[&ORGANISM_ID].storage_used<=256*1024);
}
