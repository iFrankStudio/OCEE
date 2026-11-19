//! Regression: seed99 previously failed when all RECEIVE=Empty events
//! shared one monotone lifetime counter. No private protocol oracle is
//! consulted by the VM; costs and persisted budgets are audited.
use ocee_organism_seeds::s22_p2_remote::s22_p2_discover_and_execute;
use ocee_organism_vm::{Value,Program,Block,Param,Terminator,Type,WorldCall};
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
        ("/self/active".into(),hash.clone().into_bytes()),
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
    // The successful phase-separated graph had ZERO observed Empty retries
    // in the first CI run. Do not claim exhaustion was the proven root
    // cause; execution cost/layout also changes scheduling in this world.
    // A separate reconstructed old-graph control below tests that claim.
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


/// Rebuild the historical shared-128 graph using the exact same old blocks
/// on the current core. The original program's source is preserved in git
/// commit 6abfdfaad6634b8be2abb9244c4e9fd339ab8854.
fn historic_shared_128_program()->Program {
    let mut p=s22_p2_discover_and_execute();
    let f=&mut p.functions[0];
    let remove=[
        "s22_retry_phase","s22_retry_phase_other",
        "s22_remote_receive_retry","s22_remote_retry_wait",
        "s22_ack_receive_retry","s22_ack_retry_wait",
        "s22_result_receive_retry","s22_result_retry_wait",
    ];
    f.blocks.retain(|b|!remove.contains(&b.id.as_str()));
    let feedback=f.blocks.iter_mut().find(|b|b.id=="s22_feedback").unwrap();
    feedback.terminator=Terminator::Branch{
        condition:7604,
        if_true:"s22_receive_retry".into(),true_args:vec![600,601,602],
        if_false:"s22_feedback_check_ok".into(),
        false_args:vec![600,601,602,603],
    };
    let idx=f.blocks.iter().position(|b|b.id=="s22_feedback_check_ok").unwrap();
    let route=||vec![
        Param{id:600,ty:Type::Bytes},
        Param{id:601,ty:Type::U32},
        Param{id:602,ty:Type::U32},
    ];
    f.blocks.insert(idx,Block{
        id:"s22_receive_retry".into(),params:route(),instructions:vec![],
        terminator:Terminator::RepeatBounded{
            limit:128,body:"s22_retry_wait".into(),
            body_args:vec![600,601,602],
            done:"failed".into(),done_args:vec![],
        },
    });
    f.blocks.insert(idx+1,Block{
        id:"s22_retry_wait".into(),params:route(),instructions:vec![],
        terminator:Terminator::Yield{
            call:WorldCall::Observe,args:vec![],
            next:"s22_receive".into(),next_args:vec![600,601,602],
        },
    });
    p.validate().unwrap();
    p
}

#[test]
fn historical_shared_128_counter_is_observed_not_assumed(){
    let p=historic_shared_128_program();
    let hash=p.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed:99,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,99);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),p.canonical_bytes()),
        ("/self/active".into(),hash.clone().into_bytes()),
    ]{
        assert_eq!(w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,bytes,expected_version:None,
        })).code,Code::Ok);
    }
    for _ in 0..16_384{
        let result=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    let cont=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
    assert!(cont.finished);
    let count=cont.loop_counters.get("s22_receive_retry").copied().unwrap_or(0);
    let ret=&w.state.organisms[&ORGANISM_ID].last_return;
    let gas=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged)
        .sum::<u64>();
    let b=w.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>();
    println!("S24 historical shared128 seed99: retry_count={count} return={ret:?} A_gas={gas} B_gas={b} ticks={} hash={hash}",w.state.step);
    // This control records whether old failure occurs because its shared
    // counter reached 128, rather than assuming a cause from a single
    // functional failure. The last historical CI saw U32(0) for seed99.
    assert_eq!(*ret,Some(Value::U32(0)),"the old seed99 regression must reproduce");
    assert_eq!(count,128,"demonstrate exhaustion of the historical retry cap");
}
