use ocee_organism_seeds::p0_probe::s1_p0_probe;
use ocee_protocol_sim::{Family,Operation};
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};

#[test]
fn seeded_feedback_guided_probe_finds_unknown_p0_opcode_without_hidden_values() {
    for seed in [7u64,42] {
        // Development-only resource headroom, not the fixed E001 experiment budget.
        let mut world=World::with_protocol(Config{
            a_gas_per_epoch:100_000,..Config::default()
        },Family::P0,seed);
        let program=s1_p0_probe();
        program.validate().unwrap();
        let hash=program.content_hash().unwrap();
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:program.canonical_bytes(),
        })).code,Code::Ok);
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:"/self/active".into(),
            expected_version:None,bytes:hash.as_bytes().to_vec(),
        })).code,Code::Ok);

        let mut sent=0usize;
        let mut finished=false;
        for _ in 0..800 {
            let result=world.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
                "unexpected VM failure at step {}, result {:?}",world.state.step,result.code);
            if matches!(world.audit.last().unwrap().action,Some(Action::Send{..})){sent+=1;}
            if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .is_some_and(|c|c.finished) {
                finished=true;break;
            }
        }
        assert!(finished,"bounded search must terminate");
        assert!(sent>1 && sent<=112,"sent {sent}");
        let discovered=&world.state.objects["/self/discoveries/p0-operation-candidate"].bytes;
        let protocol=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let decoded=protocol.decode(discovered).unwrap();
        assert_eq!(decoded.operation,Operation::RemoteEval);
        assert!(decoded.body.is_empty(),"it discovered an opcode, not valid workload");
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(ocee_organism_vm::Value::U32(1)));
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash));
        // No program version has been invented in this probe.
        assert_eq!(world.state.objects.keys().filter(
            |k|k.starts_with("/self/versions/")).count(),1);
        let replay=World::from_snapshot(&world.snapshot_json()).unwrap();
        assert_eq!(world.state_hash(),replay.state_hash());
    }
}
