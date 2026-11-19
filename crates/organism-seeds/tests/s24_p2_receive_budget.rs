//! Regression: seed99 previously failed when all RECEIVE=Empty events
//! shared one monotone lifetime counter. No private protocol oracle is
//! consulted by the VM; costs and persisted budgets are audited.
use ocee_organism_seeds::s22_p2_remote::s22_p2_discover_and_execute;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};
#[test]
fn seed99_real_b_result_survives_three_metered_retry_budgets(){
    let p=s22_p2_discover_and_execute();
    let hash=p.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed:99,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,99);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),p.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,bytes,expected_version:None,
        })).code,Code::Ok);
    }
    for _ in 0..16_384 {
        let result=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    let agent=&w.state.organisms[&ORGANISM_ID];
    let cont=agent.vm_continuation.as_ref().unwrap();
    assert!(cont.finished);
    let keys=["s22_remote_receive_retry","s22_ack_receive_retry",
        "s22_result_receive_retry"];
    let counters=keys.map(|k|*cont.loop_counters.get(k).unwrap_or(&0));
    let count=counters.iter().sum::<u32>();
    let a_gas=w.audit.iter().map(|e|
        e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
    let b_gas=w.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>();
    println!("S24 seed99 counters={counters:?} total={count} ticks={} A_gas={a_gas} B_gas={b_gas} return={:?}",
        w.state.step,agent.last_return);
    assert_eq!(agent.last_return,Some(Value::U32(1)));
    assert_eq!(b_gas,20);
    assert!(count>128,"exercise the previous shared-lifetime failure");
    assert!(counters.iter().all(|n|*n<=256));
    assert!(!cont.loop_counters.contains_key("s22_receive_retry"));
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let expected=job.values.iter().enumerate().fold(0u32,|total,(i,&x)|
        total.wrapping_add(x.rotate_left((i%32) as u32)));
    let bytes=&w.state.objects["/self/results/p2-job0/value_le"].bytes;
    assert_eq!(bytes.len(),4);
    assert_eq!(u32::from_le_bytes(bytes.as_slice().try_into().unwrap()),expected);
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
