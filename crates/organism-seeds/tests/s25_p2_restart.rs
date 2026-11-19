//! S25: deterministic rehydration under actual high-latency C communication.
//! Unlike previous snapshot-after-completion checks, this captures a live
//! active VM continuation with >128 real Empty RECEIVEs already consumed.
//! No private B/protocol data is supplied to its execution policy.
use ocee_organism_seeds::s22_p2_remote::s22_p2_discover_and_execute;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

const RETRIES:[&str;3]=[
    "s22_remote_receive_retry","s22_ack_receive_retry",
    "s22_result_receive_retry"
];
fn counts(world:&World)->[u32;3]{
    let vm=world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .expect("running VM continuation must persist");
    RETRIES.map(|key|vm.loop_counters.get(key).copied().unwrap_or(0))
}
fn a_gas(world:&World)->u64{
    world.audit.iter().map(|event|
        event.result.gas_charged+event.result.vm_gas_charged).sum()
}
fn b_gas(world:&World)->u64{
    world.state.service_events.iter().map(|event|event.b_gas_charged).sum()
}
fn tick(world:&mut World){
    let result=world.tick_autonomous(ORGANISM_ID);
    assert!(!matches!(result.code,Code::Denied|Code::InvalidCode),
        "a delayed C message cannot cause a VM fault: step={}",world.state.step);
}
fn complete(world:&mut World) {
    for _ in 0..16_384 {
        tick(world);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished) {break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished),"S25 bounded resumed VM must terminate");
}
fn verify_result(world:&World){
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_gas(world),20,"resuming cannot pay B a second time");
    let item:WorkItem=serde_json::from_slice(
        &world.state.objects["/public/E001/job0"].bytes).unwrap();
    let expected=item.values.iter().enumerate().fold(0u32,|sum,(i,&v)|
        sum.wrapping_add(v.rotate_left((i%32) as u32)));
    let actual=&world.state.objects["/self/results/p2-job0/value_le"].bytes;
    assert_eq!(actual.len(),4);
    assert_eq!(u32::from_le_bytes(actual.as_slice().try_into().unwrap()),expected);
}
#[test]
fn restart_after_more_than_128_real_empty_receives_preserves_billing_and_answer() {
    let program=s22_p2_discover_and_execute();
    let hash=program.content_hash().unwrap();
    let mut live=World::with_protocol(Config{
        seed:99,message_delay:6,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,99);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(live.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes
        })).code,Code::Ok);
    }
    let mut milestone=false;
    for _ in 0..16_384 {
        tick(&mut live);
        let current=counts(&live);
        if current.iter().sum::<u32>()>128 &&
            !live.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .is_some_and(|v|v.finished){
            milestone=true;
            break;
        }
        if live.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert!(milestone,"must observe a *live*, unfinished continuation after >128 real Empty waits");
    let before=counts(&live);
    assert!(before.iter().sum::<u32>()>128);
    assert!(before.iter().all(|x|*x<=256));
    assert_eq!(b_gas(&live),20,
        "checkpoint must take place after B computation, during RESULT polling");
    assert!(!live.state.objects.contains_key("/self/results/p2-job0/value_le"),
        "must not snapshot a task already finished");
    let before_hash=live.state_hash();
    let before_gas=a_gas(&live);
    let before_step=live.state.step;
    let snapshot=live.snapshot_json();
    let mut resumed=World::from_snapshot(&snapshot).unwrap();
    assert_eq!(before_hash,resumed.state_hash());
    assert_eq!(counts(&resumed),before);
    assert_eq!(resumed.state.step,before_step);
    assert_eq!(a_gas(&resumed),before_gas);
    assert_eq!(b_gas(&resumed),20);

    // Both independent trajectories continue from the same exact state.
    // The environment must not reset mailboxes, RNG, budgets or pending C.
    complete(&mut live);
    complete(&mut resumed);
    verify_result(&live);
    verify_result(&resumed);
    assert_eq!(live.state_hash(),resumed.state_hash(),
        "resumed future must be observationally identical to uninterrupted future");
    assert_eq!(live.audit,resumed.audit,"all resource charges and events must match");
    let final_counts=counts(&live);
    assert!(final_counts.iter().sum::<u32>()>128);
    assert!(final_counts.iter().all(|x|*x<=256));
    assert!(a_gas(&live)>before_gas);
    println!("S25 delayed-P2 mid-RX checkpoint step={before_step} counters={before:?} initial_A_gas={before_gas} final_step={} final_counters={final_counts:?} final_A_gas={} B_gas=20 exact_hash={}",
        live.state.step,a_gas(&live),live.state_hash());
}
