//! S24: a real C response is mutated by test-only transport fault injection
//! AFTER remote B work, never by the active organism or its policy inputs.
//! Contrast fail-closed behavior with a separately seeded bounded RESULT
//! recovery policy; neither is autonomous algorithm invention.
use ocee_organism_seeds::s22_p2_remote::{
    s22_p2_discover_and_execute,s24_p2_retry_result_after_bad_ack,
    s24_recovery_manifest,
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID,NodeId};

fn trial(recover:bool)->World {
    let program=if recover {s24_p2_retry_result_after_bad_ack()}
        else {s22_p2_discover_and_execute()};
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
        // C has already processed the legitimate job and queued the first
        // binary ACK. Earlier rejected opcode probes are JSON diagnoses.
        for m in &mut world.state.pending_deliveries{
            if injected==0 && m.source==NodeId::C && m.destination==NodeId::A
                && m.bytes.len()>=6 && m.bytes[0]!=b'{' {
                let mut length=u16::from_be_bytes([m.bytes[1],m.bytes[2]]);
                length=length.checked_add(3).unwrap();
                m.bytes[1..3].copy_from_slice(&length.to_be_bytes());
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
            "recoverable untrusted C bytes must not cause a VM fault");
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert_eq!(injected,1,"must actually mutate one real queued C ACK");
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished));
    assert_eq!(world.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>(),20,
        "B must execute exactly once, never repeat work due to a bad ACK");
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    world
}
#[test]
fn extra_inner_tlv_in_real_c_ack_produces_normal_refusal_not_vm_fault(){
    let world=trial(false);
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert!(!world.state.objects.contains_key("/self/results/p2-job0/value_le"));
    assert!(!world.state.objects.contains_key("/self/results/p2-remote-eval-ack"));
    println!("S24 corrupted C ACK refused normally at tick={} B_gas=20",
        world.state.step);
}
#[test]
fn seeded_bounded_result_retry_salvages_real_b_answer_after_corrupted_ack(){
    assert_eq!(s24_recovery_manifest().program_hash,
        format!("sha256:{}",s24_p2_retry_result_after_bad_ack()
            .content_hash().unwrap()));
    let world=trial(true);
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"only an independently validated RESULT may count");
    assert!(!world.state.objects.contains_key("/self/results/p2-remote-eval-ack"),
        "corrupted ACK must NEVER be marked as valid");
    let work:WorkItem=serde_json::from_slice(
        &world.state.objects["/public/E001/job0"].bytes).unwrap();
    let expected=work.values.iter().enumerate().fold(0u32,|a,(i,&v)|
        a.wrapping_add(v.rotate_left((i%32) as u32)));
    let raw=&world.state.objects["/self/results/p2-job0/value_le"].bytes;
    assert_eq!(u32::from_le_bytes(raw.as_slice().try_into().unwrap()),expected);
    assert_eq!(world.state.objects["/self/discoveries/p2-result-opcode"].bytes.len(),1);
    let vm:u64=world.audit.iter().map(|e|e.result.vm_gas_charged).sum();
    let total:u64=world.audit.iter().map(|e|
        e.result.vm_gas_charged+e.result.gas_charged).sum();
    assert!(vm>0);
    println!("S24 damaged ACK / recovered RESULT: tick={} a_gas={} vm_gas={} B_gas=20",
        world.state.step,total,vm);
}
