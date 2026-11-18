//! S13: reject an earlier authentic report for the SAME immutable tool,
//! SAME input, and SAME answer by comparing its execution step with the
//! activity VM's own kernel-issued EXEC receipt.
//! All attacks run inside deterministic simulated world; no actual identity
//! authentication or trust-root isolation is claimed.
use ocee_organism_seeds::s13_receipt::{
    s13_execution_receipt_binding,s13_without_execution_receipt,s13_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,BoundaryWitness,Code,Config,World,ORGANISM_ID,
    hidden_compose_boundary
};

#[derive(Clone,Copy)]
enum Attack {None,PriorGenuineExec,SameStepBothFalse}
struct Outcome{world:World,stale_step:Option<u64>}
fn run(seed:u64,program:Program,attack:Attack)->Outcome {
    let hash=program.content_hash().unwrap();
    program.validate().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    let mut injected=false;
    let mut stale_step=None;
    for _ in 0..4096 {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::InvalidCode {
            assert!(matches!(world.audit.last().and_then(|e|e.action.as_ref()),
                Some(Action::Exec{..})),"only isolated tool EXEC may fault");
        }
        assert_ne!(result.code,Code::Denied);
        let registry="/self/tools/transform-v1";
        if !injected && matches!(attack,Attack::PriorGenuineExec)
            && world.state.objects.contains_key(registry)
            && !world.state.objects.contains_key("/public/E001/semantic-witness"){
            let tool_hash=String::from_utf8(
                world.state.objects[registry].bytes.clone()).unwrap();
            let prev=world.state.step;
            let response=world.tick(Some(Action::Exec{
                actor:ORGANISM_ID,program_hash:tool_hash.clone(),input:vec![]
            }));
            assert!(matches!(response.code,Code::Ok|Code::InvalidCode));
            assert_eq!(response.exec_step,Some(u32::try_from(prev).unwrap()));
            let witness:BoundaryWitness=serde_json::from_slice(
                &world.state.objects["/public/E001/semantic-witness"].bytes).unwrap();
            assert_eq!(witness.observed_after_step,prev);
            assert_eq!(witness.subject_program_hash,tool_hash);
            stale_step=Some(prev);
            injected=true;
        }
        if !injected && matches!(attack,Attack::SameStepBothFalse)
            && world.state.objects.contains_key("/public/E001/semantic-witness")
            && world.state.objects.contains_key("/public/E001/semantic-audit"){
            // Only a privileged researcher can mutate world-owned public
            // objects; their declared source/hash/step remain unchanged.
            for path in ["/public/E001/semantic-witness",
                         "/public/E001/semantic-audit"]{
                let obj=world.state.objects.get_mut(path).unwrap();
                let mut witness:BoundaryWitness=serde_json::from_slice(&obj.bytes).unwrap();
                assert_eq!(witness.first,Some(1),"seed 5 target must be 1");
                witness.first=Some(0);
                if witness.second.is_some(){witness.second=Some(0);}
                obj.bytes=serde_json::to_vec(&witness).unwrap();
            }
            injected=true;
        }
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){
            if !matches!(attack,Attack::None){assert!(injected);}
            return Outcome{world,stale_step};
        }
    }
    panic!("S13 failed to finish in bounded 4096 ticks, seed {seed}");
}
fn tool(w:&World)->Program{
    let hash=String::from_utf8(
        w.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    p
}
#[test]
fn fresh_probe_receipt_allows_correct_repairs_and_abstention(){
    let p=s13_execution_receipt_binding();
    assert_eq!(s13_manifest().program_hash,
        format!("sha256:{}",p.content_hash().unwrap()));
    assert!(s13_manifest().research_status.contains("NOT_M0"));
    for (seed,expected) in [(5u64,2u32),(17,2),(2,2),(0,1),(20,3),(23,3)]{
        let w=run(seed,p.clone(),Attack::None).world;
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expected)),"seed {seed}");
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,
            if expected==2 {1}else{0});
        let first=w.audit.iter().find(|rec|matches!(
            &rec.action,Some(Action::Exec{input,..}) if input.is_empty()
        )).expect("active semantic probe required");
        let receipt:ActionResult=serde_json::from_slice(
            &w.state.objects["/self/repair/probe/receipt"].bytes).unwrap();
        assert_eq!(receipt.exec_step,Some(u32::try_from(first.step).unwrap()));
        for key in ["/public/E001/semantic-witness","/public/E001/semantic-audit"]{
            let ev:BoundaryWitness=serde_json::from_slice(
                &w.state.objects[key].bytes).unwrap();
            assert_eq!(ev.observed_after_step,first.step);
        }
        if expected==2 {
            assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,
                Value::U32(hidden_compose_boundary(seed,&[])));
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn real_previous_execution_same_version_and_input_is_rejected(){
    for seed in [5u64,17]{
        let observed=run(seed,s13_execution_receipt_binding(),Attack::PriorGenuineExec);
        let w=observed.world;
        let old=observed.stale_step.expect("actual prior EXEC step");
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(3)),"seed {seed} must refuse stale evidence");
        assert_eq!(w.state.objects["/self/repair/provenance/status"].bytes,
            b"replayed_execution");
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
        let receipt:ActionResult=serde_json::from_slice(
            &w.state.objects["/self/repair/probe/receipt"].bytes).unwrap();
        assert!(u64::from(receipt.exec_step.unwrap())>old);
        let report:BoundaryWitness=serde_json::from_slice(
            &w.state.objects["/public/E001/semantic-witness"].bytes).unwrap();
        assert_eq!(report.observed_after_step,old);
    }
    // Without receipt binding, a *genuine but earlier* report for exactly
    // the same program and input is accepted: controlled comparison.
    let old=run(5,s13_without_execution_receipt(),Attack::PriorGenuineExec).world;
    assert_eq!(old.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(2)));
    assert_eq!(old.state.objects["/self/tools/transform-v1"].version,1);
}
#[test]
fn valid_receipt_does_not_authenticate_two_coordinated_false_reports(){
    let w=run(5,s13_execution_receipt_binding(),Attack::SameStepBothFalse).world;
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(2)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
    assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(5,&[]),1);
}
