//! S19 development regression: continuation and immutable tool survive a world restart.
//! This is deterministic execution persistence, not autonomous migration or invention.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};

fn finish(world:&mut World) {
    for _ in 0..16_384 {
        let r=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::InvalidCode|Code::Denied),
            "unexpected VM result at step {}: {:?}",world.state.step,r.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished) { return; }
    }
    panic!("S19 did not finish");
}

#[test]
fn restart_during_specialization_preserves_tool_and_b_billing() {
    let program=s19_p1_build_and_use_tool();
    let hash=program.content_hash().unwrap();
    let mut uninterrupted=World::with_protocol(Config{
        seed:42,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,42);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".to_owned(),hash.into_bytes()),
    ] {
        assert_eq!(uninterrupted.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    // Synchronize on an actual semantic phase, not a fixed tick count.
    // Seed 42 may register the tool before tick 10. A restart strictly
    // after the immutable version is stored but before tool adoption probes
    // the vulnerable transition without assuming search duration.
    let mut inside_specialization=false;
    for _ in 0..1024 {
        let reply=uninterrupted.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::InvalidCode|Code::Denied));
        let versions=uninterrupted.state.objects.keys().filter(
            |key|key.starts_with("/self/versions/")).count();
        if versions==2 &&
            !uninterrupted.state.objects.contains_key("/self/tools/p1-eval-frame") {
            inside_specialization=true;
            break;
        }
    }
    assert!(inside_specialization,
        "must capture the unregistered generated program, not assume a tick");
    let snapshot=uninterrupted.snapshot_json();
    let mut restarted=World::from_snapshot(&snapshot).unwrap();
    assert_eq!(uninterrupted.state_hash(),restarted.state_hash());
    finish(&mut uninterrupted);
    finish(&mut restarted);
    assert_eq!(uninterrupted.state_hash(),restarted.state_hash());
    for world in [&uninterrupted,&restarted] {
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
        assert!(world.state.objects.contains_key("/self/tools/p1-eval-frame"));
        assert!(world.state.objects.contains_key("/self/results/p1-job0/value_le"));
        assert_eq!(world.state.service_events.iter()
            .map(|e|e.b_gas_charged).sum::<u64>(),20);
    }
}
