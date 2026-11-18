//! S12 focused end-to-end provenance checks and adversarial metadata controls.
use ocee_organism_seeds::s12_provenance::{
    s12_program_bound_provenance,s12_no_program_binding_control,s12_manifest
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,BoundaryWitness,Code,Config,World,ORGANISM_ID};

#[derive(Clone,Copy)]
enum Attack {None,PrimarySource,SecondarySource,PrimaryHash,SecondaryHash,BothFalse}
fn run(seed:u64,program:Program,attack:Attack)->World {
    let hash=program.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes
        })).code,Code::Ok);
    }
    let mut injected=false;
    for _ in 0..4096 {
        let result=w.tick_autonomous(ORGANISM_ID);
        assert_ne!(result.code,Code::Denied);
        if !injected && !matches!(attack,Attack::None)
            && w.state.objects.contains_key("/public/E001/semantic-witness")
            && w.state.objects.contains_key("/public/E001/semantic-audit") {
            let targets:&[&str]=match attack {
                Attack::PrimarySource|Attack::PrimaryHash=>&["/public/E001/semantic-witness"],
                Attack::SecondarySource|Attack::SecondaryHash=>&["/public/E001/semantic-audit"],
                Attack::BothFalse=>&["/public/E001/semantic-witness","/public/E001/semantic-audit"],
                Attack::None=>unreachable!(),
            };
            // Replay a real, stored and content-addressed old program:
            // the initial active VM is valid, but is NOT the registered tool.
            let stale=w.state.organisms[&ORGANISM_ID]
                .active_program_hash.clone().expect("active version exists");
            assert!(w.state.objects.contains_key(&format!("/self/versions/{stale}")));
            let registered=String::from_utf8(
                w.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
            assert_ne!(stale,registered);
            for &key in targets {
                let object=w.state.objects.get_mut(key).unwrap();
                let mut evidence:BoundaryWitness=serde_json::from_slice(&object.bytes).unwrap();
                match attack {
                    Attack::PrimarySource|Attack::SecondarySource=>evidence.source_id="sim-forged/v1".into(),
                    Attack::PrimaryHash|Attack::SecondaryHash=>evidence.subject_program_hash=stale.clone(),
                    Attack::BothFalse=>{
                        assert_eq!(evidence.first,Some(1));
                        evidence.first=Some(0);
                        if evidence.second.is_some(){evidence.second=Some(0);}
                    },
                    Attack::None=>unreachable!(),
                }
                object.bytes=serde_json::to_vec(&evidence).unwrap();
            }
            injected=true;
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {
            if !matches!(attack,Attack::None){assert!(injected);}
            return w;
        }
    }
    panic!("S12 did not terminate");
}
#[test]
fn provenance_binding_rejects_mislabeled_and_cross_version_reports() {
    let p=s12_program_bound_provenance();
    assert_eq!(s12_manifest().program_hash,
        format!("sha256:{}",p.content_hash().unwrap()));
    for (seed,expected) in [(5,2),(17,2),(0,1),(20,3),(23,3)] {
        let w=run(seed,p.clone(),Attack::None);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(expected)),"seed {seed}");
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,
            if expected==2{1}else{0});
        assert!(w.state.organisms[&ORGANISM_ID].storage_used<=262_144);
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
    for (attack,reason) in [
        (Attack::PrimarySource,b"source_mismatch".as_slice()),
        (Attack::SecondarySource,b"source_mismatch"),
        (Attack::PrimaryHash,b"stale_program"),
        (Attack::SecondaryHash,b"stale_program"),
    ] {
        let w=run(5,p.clone(),attack);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
        assert_eq!(w.state.objects["/self/repair/provenance/status"].bytes,reason);
    }
}
#[test]
fn binding_is_not_authentication_and_ablation_still_accepts_mislabeled_reports() {
    let w=run(5,s12_no_program_binding_control(),Attack::PrimaryHash);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(2)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
    let poisoned=run(5,s12_program_bound_provenance(),Attack::BothFalse);
    assert_eq!(poisoned.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(2)));
    assert_eq!(poisoned.state.objects["/self/tools/transform-v1"].version,1);
}
