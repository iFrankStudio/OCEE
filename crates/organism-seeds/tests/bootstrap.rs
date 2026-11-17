use ocee_organism_seeds::{s0_bootstrap,s0_manifest};
use ocee_protocol_sim::{Family,FrameKind,Operation};
use ocee_world_kernel::{Action,ActionResult,Code,Config,Message,World,ORGANISM_ID};

#[test]
fn seeded_s0_program_replays_public_hello_and_persists_a_reply_without_host_actions() {
    for family in [Family::P0,Family::P1,Family::P2] {
        for seed in [7,42] {
            let mut world=World::with_protocol(Config::default(),family,seed);
            let initial=s0_bootstrap();
            let hash=initial.content_hash().unwrap();
            assert_eq!(world.tick(Some(Action::Write{
                actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
                expected_version:None,bytes:initial.canonical_bytes(),
            })).code,Code::Ok);
            assert_eq!(world.tick(Some(Action::Write{
                actor:ORGANISM_ID,key:"/self/active".into(),
                expected_version:None,bytes:hash.as_bytes().to_vec(),
            })).code,Code::Ok);
            let mut actions=0;
            for _ in 0..32 {
                let response=world.tick_autonomous(ORGANISM_ID);
                if world.audit.last().unwrap().action.is_some(){actions+=1;}
                assert!(!matches!(response.code,Code::InvalidCode|Code::Denied));
                if world.state.organisms[&ORGANISM_ID].vm_continuation
                    .as_ref().is_some_and(|c|c.finished) {break;}
            }
            assert!(actions>=5,"seed must perform its own world interactions");
            let saved_sample=&world.state.objects["/self/observations/hello"].bytes;
            let published:ocee_world_kernel::HelloSample=
                serde_json::from_slice(saved_sample).unwrap();
            let saved_reply=&world.state.objects["/self/observations/reply"].bytes;
            let action_reply:ActionResult=serde_json::from_slice(saved_reply).unwrap();
            assert_eq!(action_reply.code,Code::Ok);
            let messages:Vec<Message>=serde_json::from_slice(&action_reply.payload).unwrap();
            assert_eq!(messages.len(),1);
            let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
            assert_eq!(proto.decode(&published.request).unwrap().operation,Operation::Hello);
            let decoded=proto.decode(&messages[0].bytes).unwrap();
            assert_eq!(decoded.kind,FrameKind::Response);
            assert_eq!(decoded.operation,Operation::Hello);
            assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
                Some(hash.clone()));
            assert_eq!(world.state.objects.keys().filter(
                |k|k.starts_with("/self/versions/")).count(),1);
            // The initial algorithm performs an interaction; no new tool was invented.
            assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
                Some(ocee_organism_vm::Value::U32(1)));
            let restored=World::from_snapshot(&world.snapshot_json()).unwrap();
            assert_eq!(world.state_hash(),restored.state_hash());
            assert_eq!(world.audit,restored.audit);
        }
    }
}

#[test]
fn bootstrap_manifest_reports_only_explicitly_seeded_capabilities() {
    let p=s0_bootstrap();
    p.validate().unwrap();
    let m=s0_manifest();
    assert_eq!(m.kind,"S0-bootstrap");
    assert_eq!(m.program_hash,format!("sha256:{}",p.content_hash().unwrap()));
    assert!(m.known_protocol_families.is_empty());
    assert!(m.initial_tool_hashes.is_empty());
    assert!(m.research_status.contains("NOT_SELF_INNOVATION"));
}
