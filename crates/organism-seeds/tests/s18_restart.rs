//! S18 restart-safety regression: continuation and service queues must survive
//! an ordinary world snapshot during active protocol exploration.
use ocee_organism_seeds::s18_p1_vm::s18_p1_discover_and_invoke;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};

fn installed(seed:u64)->World {
    let program=s18_p1_discover_and_invoke();
    let hash=program.content_hash().unwrap();
    let mut w=World::with_protocol(Config{seed,a_gas_per_epoch:12_000,..Config::default()},Family::P1,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(w.tick(Some(Action::Write{actor:ORGANISM_ID,key,expected_version:None,bytes})).code,Code::Ok);
    }
    w
}
fn finish(w:&mut World){
    for _ in 0..16_384 {
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::InvalidCode|Code::Denied));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished) {
            return;
        }
    }
    panic!("S18 did not terminate within bound");
}
#[test]
fn active_p1_discovery_survives_mid_scan_snapshot_without_duplicate_b_work(){
    let mut uninterrupted=installed(42);
    for _ in 0..10 {
        let r=uninterrupted.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::InvalidCode|Code::Denied));
    }
    assert!(uninterrupted.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|!c.finished));
    let mut restored=World::from_snapshot(&uninterrupted.snapshot_json()).unwrap();
    assert_eq!(restored.state_hash(),uninterrupted.state_hash());
    finish(&mut uninterrupted);
    finish(&mut restored);
    assert_eq!(uninterrupted.state_hash(),restored.state_hash());
    assert_eq!(restored.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert_eq!(restored.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>(),20);
}
