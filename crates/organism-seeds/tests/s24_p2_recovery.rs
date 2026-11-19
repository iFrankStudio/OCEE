//! S24: a genuine C response is modified by the researcher-side transport
//! fault injector, not by the organism. The same seeded S22 active VM must
//! branch to failure without a VM InvalidCode or an invented B answer.
use ocee_organism_seeds::s22_p2_remote::s22_p2_discover_and_execute;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID,NodeId};

#[test]
fn extra_inner_tlv_in_real_c_ack_produces_recoverable_rejection_not_vm_fault() {
    let program=s22_p2_discover_and_execute();
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed:42,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,42);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".to_string(),hash.into_bytes()),
    ]{
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,bytes,expected_version:None,
        })).code,Code::Ok);
    }
    let mut injected=0u32;
    for _ in 0..4096{
        // Only the binary ACK for the first correctly recognized real
        // REMOTE_EVAL request can reach this branch: all earlier opcode
        // probes generated visible JSON error diagnoses.
        for m in &mut world.state.pending_deliveries{
            if injected==0 && m.source==NodeId::C && m.destination==NodeId::A
                && m.bytes.len()>=6 && m.bytes[0]!=b'{' {
                let mut len=u16::from_be_bytes([m.bytes[1],m.bytes[2]]);
                len=len.checked_add(3).unwrap();
                m.bytes[1..3].copy_from_slice(&len.to_be_bytes());
                let mut inner=u16::from_be_bytes([m.bytes[4],m.bytes[5]]);
                assert_eq!(usize::from(inner)+6,m.bytes.len());
                inner=inner.checked_add(3).unwrap();
                m.bytes[4..6].copy_from_slice(&inner.to_be_bytes());
                m.bytes.extend_from_slice(&[0x7f,0,0]);
                injected+=1;
            }
        }
        let outcome=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(outcome.code,Code::InvalidCode|Code::Denied),
            "untrusted C bytes must cause an organism-level refusal, not VM fault");
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert_eq!(injected,1,"must actually inject a real queued C binary ACK");
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished));
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(world.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>(),20,
        "remote B work happened, but the malformed ACK was rejected");
    assert!(!world.state.objects.contains_key("/self/results/p2-job0/value_le"));
    assert!(!world.state.objects.contains_key("/self/results/p2-remote-eval-ack"));
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    let vm_gas:u64=world.audit.iter().map(|e|e.result.vm_gas_charged).sum();
    assert!(vm_gas>0);
    println!("S24 real C ACK injected, recovered U32(0): tick={} vm_gas={} B_gas=20",
        world.state.step,vm_gas);
}
