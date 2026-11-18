//! S11: two separate simulated reference channels must agree before adoption.
//! A researcher-only test mutation contaminates either one or both reports;
//! this is NOT an organism's permission to edit world-owned public evidence.
use ocee_organism_seeds::s11_audit::{
    s11_corroborated_semantic_repair,s11_no_secondary_audit_control,s11_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,BoundaryWitness,Code,Config,World,ORGANISM_ID,hidden_compose_boundary
};

#[derive(Clone,Copy)]
enum Injection {None,PrimaryFalse, BothFalse}
fn run(seed:u64,program:Program,injection:Injection)->World{
    let hash=program.content_hash().unwrap();
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
    assert!(!world.state.objects.contains_key("/public/E001/semantic-witness"));
    assert!(!world.state.objects.contains_key("/public/E001/semantic-audit"));
    let mut poisoned=false;
    for _ in 0..4096 {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::InvalidCode {
            assert!(matches!(world.audit.last().and_then(|r|r.action.as_ref()),
                Some(Action::Exec{..})),"only isolated tool faults expected");
        }
        assert_ne!(result.code,Code::Denied);
        if !poisoned && !matches!(injection,Injection::None)
            && world.state.objects.contains_key("/public/E001/semantic-witness")
            && world.state.objects.contains_key("/public/E001/semantic-audit"){
            // After the actual EXEC and before the organism reads either
            // source, simulate a privileged attacker falsifying source values.
            let first=world.state.objects.get_mut("/public/E001/semantic-witness").unwrap();
            let mut data:BoundaryWitness=serde_json::from_slice(&first.bytes).unwrap();
            assert_eq!(data.first,Some(1),"seed 5 must have true answer 1");
            data.first=Some(0);data.second=Some(0);
            first.bytes=serde_json::to_vec(&data).unwrap();
            if matches!(injection,Injection::BothFalse) {
                let second=world.state.objects.get_mut("/public/E001/semantic-audit").unwrap();
                let mut data:BoundaryWitness=serde_json::from_slice(&second.bytes).unwrap();
                assert_eq!(data.first,Some(1));
                data.first=Some(0);
                second.bytes=serde_json::to_vec(&data).unwrap();
            }
            poisoned=true;
        }
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){
            if !matches!(injection,Injection::None){assert!(poisoned);}
            return world;
        }
    }
    panic!("S11 did not terminate, seed {seed}");
}
fn tool(world:&World)->Program {
    let hash=String::from_utf8(world.state.objects[
        "/self/tools/transform-v1"].bytes.clone()).unwrap();
    let program:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    program
}
#[test]
fn separate_simulated_auditor_enables_repairs_and_preserves_abstention(){
    let initial=s11_corroborated_semantic_repair();
    let hash=initial.content_hash().unwrap();
    assert_eq!(s11_manifest().program_hash,format!("sha256:{hash}"));
    assert!(s11_manifest().research_status.contains("NOT_M0"));
    for (seed,expected) in [(5u64,2u32),(17,2),(2,2),(0,1),(20,3),(23,3)]{
        let w=run(seed,initial.clone(),Injection::None);
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash.clone()),"no researcher stage handoff");
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expected)),"seed {seed}");
        assert!(w.state.organisms[&ORGANISM_ID].storage_used<=262_144);
        let raw=&w.state.objects["/public/E001/semantic-audit"].bytes;
        let audit:BoundaryWitness=serde_json::from_slice(raw).unwrap();
        assert_eq!(audit.first,Some(hidden_compose_boundary(seed,&[])));
        assert_eq!(audit.input,vec![]);
        let executed=w.audit.iter().find(|r|matches!(
            &r.action,Some(Action::Exec{input,..}) if input.is_empty()
        )).expect("probe required");
        assert_eq!(audit.observed_after_step,executed.step);
        let registry=&w.state.objects["/self/tools/transform-v1"];
        assert_eq!(registry.version,if expected==2{1}else{0});
        if expected==2 {
            assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,
                Value::U32(hidden_compose_boundary(seed,&[])));
            assert!(!w.state.objects.contains_key("/self/repair/audit/status"));
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
    let withheld=run(5,s11_no_secondary_audit_control(),Injection::None);
    assert_eq!(withheld.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(3)));
    assert_eq!(withheld.state.objects["/self/tools/transform-v1"].version,0);
    assert_eq!(withheld.state.objects["/self/repair/audit/status"].bytes,b"missing");
}
#[test]
fn poisoning_one_source_is_detected_and_prevents_cas_adoption(){
    let w=run(5,s11_corroborated_semantic_repair(),Injection::PrimaryFalse);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(3)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
    assert_eq!(w.state.objects["/self/repair/audit/status"].bytes,b"mismatch");
    let a:BoundaryWitness=serde_json::from_slice(
        &w.state.objects["/public/E001/semantic-witness"].bytes).unwrap();
    let b:BoundaryWitness=serde_json::from_slice(
        &w.state.objects["/public/E001/semantic-audit"].bytes).unwrap();
    assert_eq!(a.first,Some(0));assert_eq!(b.first,Some(1));
    assert!(!w.audit.iter().any(|r|matches!(
        &r.action,Some(Action::Write{key,expected_version:Some(0),..})
        if key=="/self/tools/transform-v1"
    )));
}
#[test]
fn simultaneous_corruption_of_both_sources_is_a_known_remaining_failure(){
    // Negative result: agreement across two report paths is not authenticity.
    let w=run(5,s11_corroborated_semantic_repair(),Injection::BothFalse);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)),"organism trusts two correlated false reports");
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
    assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(5,&[]),1,
        "the accepted tool is semantically wrong under co-corruption");
}
