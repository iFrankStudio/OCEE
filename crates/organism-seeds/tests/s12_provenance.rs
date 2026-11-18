//! S12 proof boundaries: subject-program and declared-source binding reject
//! cross-version replay and mislabeled reports, but do NOT authenticate sources
//! or protect against same-version privileged common-mode forgery.
use ocee_organism_seeds::s12_provenance::{
    s12_program_bound_provenance,s12_no_program_binding_control,s12_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,BoundaryWitness,Code,Config,World,ORGANISM_ID,hidden_compose_boundary
};

#[derive(Clone,Copy)]
enum Tamper {
    None,PrimaryStale,SecondaryStale,PrimaryWrongSource,
    SecondaryWrongSource,AlignedFalse,
}
fn run(seed:u64,program:Program,tamper:Tamper)->World{
    program.validate().unwrap();
    let init_hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{init_hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:init_hash.as_bytes().to_vec()
    })).code,Code::Ok);
    let mut changed=false;
    for _ in 0..4096 {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::InvalidCode{
            assert!(matches!(world.audit.last().and_then(|a|a.action.as_ref()),
                Some(Action::Exec{..})),"only isolated tool execution may fault");
        }
        assert_ne!(result.code,Code::Denied);
        if !changed && !matches!(tamper,Tamper::None)
            && world.state.objects.contains_key("/public/E001/semantic-witness")
            && world.state.objects.contains_key("/public/E001/semantic-audit") {
            let primary="/public/E001/semantic-witness";
            let secondary="/public/E001/semantic-audit";
            let stale=init_hash.clone(); // valid immutable initial program, NOT the probed tool
            let mutate=|w:&mut World,key:&str,op:fn(&mut BoundaryWitness,&str)|{
                let obj=w.state.objects.get_mut(key).unwrap();
                let mut evidence:BoundaryWitness=serde_json::from_slice(&obj.bytes).unwrap();
                op(&mut evidence,&stale);
                obj.bytes=serde_json::to_vec(&evidence).unwrap();
            };
            match tamper {
                Tamper::PrimaryStale=>mutate(&mut world,primary,|e,h|
                    e.subject_program_hash=h.into()),
                Tamper::SecondaryStale=>mutate(&mut world,secondary,|e,h|
                    e.subject_program_hash=h.into()),
                Tamper::PrimaryWrongSource=>mutate(&mut world,primary,|e,_|
                    e.source_id="sim-unknown/v1".into()),
                Tamper::SecondaryWrongSource=>mutate(&mut world,secondary,|e,_|
                    e.source_id="sim-primary/v1".into()),
                Tamper::AlignedFalse=>{
                    mutate(&mut world,primary,|e,_|{
                        e.first=Some(0);e.second=Some(0);
                    });
                    mutate(&mut world,secondary,|e,_|e.first=Some(0));
                },
                Tamper::None=>unreachable!(),
            }
            changed=true;
        }
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {
            if !matches!(tamper,Tamper::None){assert!(changed);}
            return world;
        }
    }
    panic!("S12 did not terminate in 4096 ticks for seed {seed}");
}
fn tool(world:&World)->Program{
    let hash=String::from_utf8(
        world.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    p
}
#[test]
fn correctly_bound_reports_retain_repair_and_abstention_behavior(){
    let initial=s12_program_bound_provenance();
    let hash=initial.content_hash().unwrap();
    assert_eq!(s12_manifest().program_hash,format!("sha256:{hash}"));
    assert!(s12_manifest().research_status.contains("NOT_M0"));
    for (seed,expect) in [(5u64,2u32),(17,2),(2,2),(0,1),(20,3),(23,3)]{
        let world=run(seed,initial.clone(),Tamper::None);
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash.clone()));
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expect)),"seed {seed}");
        let registered=String::from_utf8(
            world.audit.iter().find_map(|r|{
                if let Some(Action::Write{key,bytes,..})=&r.action {
                    if key=="/self/tools/transform-v1"{return Some(bytes.clone());}
                }None
            }).expect("tool registered")).unwrap();
        for (path,source) in [
            ("/public/E001/semantic-witness","sim-primary/v1"),
            ("/public/E001/semantic-audit","sim-secondary/v1"),
        ] {
            let raw=&world.state.objects[path].bytes;
            let evidence:BoundaryWitness=serde_json::from_slice(raw).unwrap();
            assert_eq!(evidence.source_id,source);
            assert_eq!(evidence.subject_program_hash,registered);
        }
        if expect==2 {
            assert_eq!(world.state.objects["/self/tools/transform-v1"].version,1);
            assert_eq!(run_isolated(&tool(&world),vec![],2000).unwrap().0,
                Value::U32(hidden_compose_boundary(seed,&[])));
        }else{
            assert_eq!(world.state.objects["/self/tools/transform-v1"].version,0);
        }
        assert_eq!(world.state_hash(),
            World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn cross_program_replay_and_mislabeled_sources_are_refused_before_cas(){
    for (attack,reason) in [
        (Tamper::PrimaryStale,b"stale_program".as_slice()),
        (Tamper::SecondaryStale,b"stale_program".as_slice()),
        (Tamper::PrimaryWrongSource,b"source_mismatch".as_slice()),
        (Tamper::SecondaryWrongSource,b"source_mismatch".as_slice()),
    ]{
        let w=run(5,s12_program_bound_provenance(),attack);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(3)));
        assert_eq!(w.state.objects["/self/repair/provenance/status"].bytes,reason);
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
        assert!(!w.audit.iter().any(|r|matches!(
            &r.action,Some(Action::Write{key,expected_version:Some(0),..})
            if key=="/self/tools/transform-v1"
        )));
    }
    // With the S12 identity/version checks disabled, a report with an
    // obsolete program hash can still lead to tool adoption.
    let control=run(5,s12_no_program_binding_control(),Tamper::PrimaryStale);
    assert_eq!(control.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)));
    assert_eq!(control.state.objects["/self/tools/transform-v1"].version,1);
}
#[test]
fn metadata_without_authentication_cannot_reject_coordinated_forgery(){
    let w=run(5,s12_program_bound_provenance(),Tamper::AlignedFalse);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)),"matching metadata does not make false reports true");
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
    assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(5,&[]),1);
}
