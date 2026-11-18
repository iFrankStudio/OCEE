//! Development-only transport fault injection after real C processing.
//! The test harness, not the organism, mutates one queued C reply.
use ocee_organism_seeds::s21_p2_tlv::s21_p2_infer_and_validate_new_hello;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID,NodeId};

fn trial(inject: bool) -> (World,usize) {
    let program=s21_p2_infer_and_validate_new_hello();
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed:42,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,42);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".to_string(),hash.into_bytes()),
    ] {
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes
        })).code,Code::Ok);
    }
    let mut altered=0;
    for _ in 0..512 {
        // Inject only after the real C service has produced its response,
        // before the transport delivers it to the organism.
        if inject {
            for message in &mut world.state.pending_deliveries {
                if message.source==NodeId::C && message.destination==NodeId::A
                    && altered==0 && message.bytes.len()>=6
                    && message.bytes[0]!=b'{' {
                    let old_len=u16::from_be_bytes([message.bytes[1],message.bytes[2]]);
                    let new_len=old_len.checked_add(3).unwrap();
                    message.bytes[1..3].copy_from_slice(&new_len.to_be_bytes());
                    message.bytes.extend_from_slice(&[0x7f,0,0]);
                    altered+=1;
                }
            }
        }
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
            "VM fault at step {}: {:?}",world.state.step,result.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    (world,altered)
}

#[test]
fn extra_outer_tlv_after_real_c_reply_is_not_accepted() {
    let (normal,normal_injections)=trial(false);
    assert_eq!(normal_injections,0);
    assert_eq!(normal.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert!(normal.state.objects.contains_key("/self/observations/p2-hello-nonreplay"));
    let (mutated,injections)=trial(true);
    assert_eq!(injections,1,"must actually inject into the real C reply");
    assert_eq!(mutated.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(!mutated.state.objects.contains_key("/self/observations/p2-hello-nonreplay"));
    assert_eq!(mutated.state.service_events.len(),normal.state.service_events.len());
    assert_eq!(mutated.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>(),0);
}
