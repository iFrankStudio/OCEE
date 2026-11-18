//! S13: a world-observation-derived challenge ties each simulated report
//! to this particular tool probe. This catches unmodified same-version
//! cross-episode replay, but is not a proof of authenticated truth.
use ocee_organism_seeds::s13_challenge::{
    s13_challenge_bound_repair,s13_no_challenge_control,s13_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,BoundaryWitness,Code,Config,World,ORGANISM_ID,hidden_compose_boundary
};

#[derive(Clone)]
enum Attack {
    None,
    Replay(Vec<u8>,Vec<u8>),
    CoordinatedFalse,
}
fn run(seed:u64,program:Program,extra_pre_ticks:u32,attack:Attack)->World{
    program.validate().expect("initial graph");
    let hash=program.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    // Exogenous world's unrelated activity shifts the later Observe
    // payload, without changing the seed or S6-generated tool algorithm.
    for _ in 0..extra_pre_ticks {
        assert_eq!(w.tick(Some(Action::Observe{actor:ORGANISM_ID})).code,Code::Ok);
    }
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.as_bytes().to_vec())
    ] {
        assert_eq!(w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes
        })).code,Code::Ok);
    }
    let mut tampered=false;
    for _ in 0..4096 {
        let result=w.tick_autonomous(ORGANISM_ID);
        if result.code==Code::InvalidCode {
            assert!(matches!(w.audit.last().and_then(|r|r.action.as_ref()),
                Some(Action::Exec{..})),"active VM must not fault");
        }
        assert_ne!(result.code,Code::Denied);
        if !tampered && !matches!(attack,Attack::None) &&
            w.state.objects.contains_key("/public/E001/semantic-witness") &&
            w.state.objects.contains_key("/public/E001/semantic-audit") {
            let p="/public/E001/semantic-witness";
            let a="/public/E001/semantic-audit";
            match &attack {
                Attack::Replay(primary,secondary)=>{
                    let current:BoundaryWitness=serde_json::from_slice(
                        &w.state.objects[p].bytes).unwrap();
                    let prior:BoundaryWitness=serde_json::from_slice(primary).unwrap();
                    assert_eq!(prior.subject_program_hash,current.subject_program_hash,
                        "replay MUST be for the same immutable tool version");
                    assert_eq!(prior.source_id,current.source_id);
                    assert_ne!(prior.probe_challenge,current.probe_challenge,
                        "replayed report must have a genuinely stale challenge");
                    let prior_secondary:BoundaryWitness=serde_json::from_slice(secondary).unwrap();
                    let current_secondary:BoundaryWitness=serde_json::from_slice(
                        &w.state.objects[a].bytes).unwrap();
                    assert_eq!(prior_secondary.subject_program_hash,
                        current_secondary.subject_program_hash);
                    assert_ne!(prior_secondary.probe_challenge,
                        current_secondary.probe_challenge);
                    w.state.objects.get_mut(p).unwrap().bytes=primary.clone();
                    w.state.objects.get_mut(a).unwrap().bytes=secondary.clone();
                },
                Attack::CoordinatedFalse=>{
                    for path in [p,a] {
                        let obj=w.state.objects.get_mut(path).unwrap();
                        let mut report:BoundaryWitness=
                            serde_json::from_slice(&obj.bytes).unwrap();
                        assert_eq!(report.first,Some(1));
                        report.first=Some(0);
                        if report.second.is_some(){report.second=Some(0);}
                        obj.bytes=serde_json::to_vec(&report).unwrap();
                    }
                },
                Attack::None=>unreachable!(),
            }
            tampered=true;
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|x|x.finished){
            if !matches!(attack,Attack::None){assert!(tampered);}
            return w;
        }
    }
    panic!("S13 failed to finish in 4096 logical ticks for seed {seed}");
}
fn witness(w:&World,path:&str)->BoundaryWitness{
    serde_json::from_slice(&w.state.objects[path].bytes).unwrap()
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
fn fresh_challenge_preserves_existing_repair_and_abstention(){
    let initial=s13_challenge_bound_repair();
    let initial_hash=initial.content_hash().unwrap();
    assert_eq!(s13_manifest().program_hash,format!("sha256:{initial_hash}"));
    assert!(s13_manifest().research_status.contains("NOT_M0"));
    for (seed,expect) in [(5u64,2u32),(17,2),(2,2),(0,1),(20,3),(23,3)]{
        let w=run(seed,initial.clone(),0,Attack::None);
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(initial_hash.clone()));
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expect)),"seed {seed}");
        let saved=String::from_utf8(
            w.state.objects["/self/repair/probe-nonce"].bytes.clone()).unwrap();
        assert_eq!(saved.len(),64);
        for path in ["/public/E001/semantic-witness","/public/E001/semantic-audit"]{
            assert_eq!(witness(&w,path).probe_challenge,Some(saved.clone()));
        }
        let version=w.state.objects["/self/tools/transform-v1"].version;
        assert_eq!(version,if expect==2{1}else{0});
        if expect==2{
            assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,
                Value::U32(hidden_compose_boundary(seed,&[])));
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn same_tool_version_old_reports_are_rejected_but_s12_ablation_accepts_them(){
    let earlier=run(5,s13_challenge_bound_repair(),0,Attack::None);
    assert_eq!(earlier.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)));
    let prior=Attack::Replay(
        earlier.state.objects["/public/E001/semantic-witness"].bytes.clone(),
        earlier.state.objects["/public/E001/semantic-audit"].bytes.clone()
    );
    let normal=run(5,s13_challenge_bound_repair(),2,Attack::None);
    assert_eq!(normal.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)));
    assert_ne!(witness(&earlier,"/public/E001/semantic-witness").probe_challenge,
        witness(&normal,"/public/E001/semantic-witness").probe_challenge);

    let replayed=run(5,s13_challenge_bound_repair(),2,prior.clone());
    assert_eq!(replayed.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(3)),"must abstain on same-tool old challenge");
    assert_eq!(replayed.state.objects["/self/tools/transform-v1"].version,0);
    assert_eq!(replayed.state.objects["/self/repair/provenance/status"].bytes,
        b"stale_program","reuse current abstention status for stale evidence");

    let control=run(5,s13_no_challenge_control(),2,prior);
    assert_eq!(control.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)),"S12 without fresh challenge wrongly accepts replay");
    assert_eq!(control.state.objects["/self/tools/transform-v1"].version,1);
}
#[test]
fn matching_challenge_cannot_authenticate_a_coordinated_forgery(){
    let w=run(5,s13_challenge_bound_repair(),1,Attack::CoordinatedFalse);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)),"both sources can still forge values together");
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
    assert_eq!(run_isolated(&tool(&w),vec![],2000).unwrap().0,Value::U32(0));
    assert_eq!(hidden_compose_boundary(5,&[]),1);
}
