//! S9 minimal end-to-end experiment: external post-fault evidence, no
//! privileged oracle available to the organism, and conservative abstention.
use ocee_organism_seeds::s9_witness::{
    s9_external_witness_repair,s9_no_witness_control,s9_manifest,
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,Code,Config,World,ORGANISM_ID,hidden_compose_boundary,
};

fn run(seed:u64,program:Program)->World {
    program.validate().unwrap();
    let hash=program.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes(),
    })).code,Code::Ok);
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec(),
    })).code,Code::Ok);
    for _ in 0..4096 {
        let result=w.tick_autonomous(ORGANISM_ID);
        assert_ne!(result.code,Code::Denied);
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){return w;}
    }
    panic!("S9 did not terminate, seed {seed}");
}
fn adopted(w:&World)->Program {
    let hash=String::from_utf8(w.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
    serde_json::from_slice(&w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap()
}
#[test]
fn witness_guides_repair_or_abstention_under_standard_budget(){
    let program=s9_external_witness_repair();
    assert_eq!(s9_manifest().program_hash,
        format!("sha256:{}",program.content_hash().unwrap()));
    // Use S7-confirmed faulty rule indices 2 and 5; +18 preserves
    // the rule but changes (seed/3)%4 witness mode.
    for (seed,expect) in [(2u64,0u32),(5,1),(20,3),(23,3),(38,0),(41,1)] {
        let w=run(seed,program.clone());
        let initial=program.content_hash().unwrap();
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(initial),"no host program replacement");
        let witness=w.state.objects.get("/public/E001/boundary-witness")
            .unwrap_or_else(||panic!("witness missing after seed {seed}"));
        let witness:ocee_world_kernel::BoundaryWitness=
            serde_json::from_slice(&witness.bytes).unwrap();
        assert_eq!(witness.observed_after_step+1,
            w.audit.iter().find(|e|matches!(&e.action,
                Some(Action::Exec{input,..}) if input.is_empty()
            ) && e.result.code==Code::InvalidCode).unwrap().step+1);
        let version=w.state.objects["/self/tools/transform-v1"].version;
        let return_value=w.state.organisms[&ORGANISM_ID].last_return.clone();
        if expect==3 {
            assert_eq!(return_value,Some(Value::U32(3)));
            assert_eq!(version,0,"insufficient evidence must not adopt");
            assert!(w.state.objects.contains_key("/self/repair/boundary/status"));
        } else {
            assert_eq!(return_value,Some(Value::U32(2)));
            assert_eq!(version,1);
            let tool=adopted(&w);
            assert_eq!(run_isolated(&tool,vec![],2000).unwrap().0,Value::U32(expect));
            for input in [&[99,0,13][..],&[1,2,3,4,5,6][..]] {
                assert_eq!(run_isolated(&tool,input.to_vec(),2000).unwrap().0,
                    Value::U32(hidden_compose_boundary(seed,input)));
            }
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
        assert!(w.state.organisms[&ORGANISM_ID].storage_used<=256*1024);
    }
}
#[test]
fn no_witness_means_no_repair_even_if_world_has_an_answer(){
    let w=run(5,s9_no_witness_control());
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
}
