//! S14: a fresh world inherits only the previous organism's selected
//! immutable tool and registry pointer, at ordinary gas/storage cost.
//! D inherits these two self-owned objects; C starts without either one.
//! No hidden world state, evaluator, continuation, or execution feedback crosses.
use ocee_organism_seeds::s14_transfer::{s14_episode_transfer,s14_manifest};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID,hidden_compose};

struct Run{world:World,gas:u64,ticks:u64,import_gas:u64}
fn do_write(w:&mut World,key:String,bytes:Vec<u8>){
    let result=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,expected_version:None,bytes
    }));
    assert_eq!(result.code,Code::Ok);
}
fn selected_tool(w:&World)->(String,Program){
    let hash=String::from_utf8(w.state.objects[
        "/self/tools/transform-v1"].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(&w.state.objects[
        &format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
fn run(seed:u64,donor:Option<&World>)->Run{
    let initial=s14_episode_transfer();
    let initial_hash=initial.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    do_write(&mut world,format!("/self/versions/{initial_hash}"),
             initial.canonical_bytes());
    do_write(&mut world,"/self/active".into(),initial_hash.into_bytes());

    let mut import_gas=0;
    if let Some(source)=donor {
        let (hash,program)=selected_tool(source);
        // Explicitly designated cross-episode payload, no whole-state clone.
        let before=world.audit.len();
        do_write(&mut world,format!("/self/versions/{hash}"),program.canonical_bytes());
        do_write(&mut world,"/self/tools/transform-v1".into(),hash.into_bytes());
        import_gas=world.audit[before..].iter().map(|r|
            r.result.gas_charged+r.result.vm_gas_charged).sum();
        assert_eq!(world.state.organisms[&ORGANISM_ID].vm_continuation,None);
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,None);
        // Fresh worlds generate their own /public fixtures during setup.
        // Only the initial program, inherited tool, registry and active pointer
        // may exist under /self before the second episode starts.
        assert_eq!(world.state.objects.keys().filter(|key|
            key.starts_with("/self/")).count(),4);
        assert!(!world.state.objects.contains_key("/self/results/transform-held-out"));
        assert_eq!(world.state.config.seed,seed);
        assert!(world.state.mailboxes.is_empty());
        assert!(world.state.service_events.is_empty());
    }
    for _ in 0..3072 {
        let result=world.tick_autonomous(ORGANISM_ID);
        assert_ne!(result.code,Code::Denied,"permission failure at {}",world.state.step);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded S14 run failed to terminate");
    let gas=world.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum();
    let ticks=world.state.step;
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    Run{world,gas,ticks,import_gas}
}
fn check_tool(w:&World,seed:u64){
    let (_hash,program)=selected_tool(w);
    for bytes in [&[1,2,3,4,5,6][..],&[99,0,13][..],&[200,7,41,17,9][..]]{
        assert_eq!(run_isolated(&program,bytes.to_vec(),2000).unwrap().0,
            Value::U32(hidden_compose(seed,bytes)));
    }
}
#[test]
fn same_rule_new_world_reuses_prior_self_generated_tool_and_costs_less(){
    assert_eq!(s14_manifest().program_hash,
        format!("sha256:{}",s14_episode_transfer().content_hash().unwrap()));
    let old=run(17,None);
    assert_eq!(old.world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    let baseline=run(35,None); // 35%18==17, same semantics, different world.
    let inherited=run(35,Some(&old.world));
    assert_eq!(baseline.world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert_eq!(inherited.world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(2)));
    assert_eq!(selected_tool(&old.world).0,selected_tool(&inherited.world).0);
    check_tool(&baseline.world,35);
    check_tool(&inherited.world,35);
    assert!(inherited.import_gas>0,"inheritance is not a free host mutation");
    assert!(inherited.gas<baseline.gas,
        "full lifecycle gas: inherited {} vs fresh {}",inherited.gas,baseline.gas);
    assert!(inherited.ticks<baseline.ticks,
        "full lifecycle ticks: inherited {} vs fresh {}",inherited.ticks,baseline.ticks);
    assert!(inherited.world.audit.iter().any(|r|matches!(
        &r.action,Some(Action::Exec{..})
    )),"inherited tool must be actually executed again");
    println!("S14 same-rule target35: fresh_gas={} inherited_gas={} import_gas={} fresh_ticks={} inherited_ticks={} fresh_storage={} inherited_storage={}",
        baseline.gas,inherited.gas,inherited.import_gas,baseline.ticks,inherited.ticks,
        baseline.world.state.organisms[&ORGANISM_ID].storage_used,
        inherited.world.state.organisms[&ORGANISM_ID].storage_used);
}
#[test]
fn different_rule_rejects_old_semantics_then_relearns_and_rebinds_registry(){
    let old=run(0,None);
    let stale_hash=selected_tool(&old.world).0;
    let fresh=run(17,None);
    let inherited=run(17,Some(&old.world));
    assert_eq!(fresh.world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert_eq!(inherited.world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    let repaired_hash=selected_tool(&inherited.world).0;
    assert_ne!(stale_hash,repaired_hash,"old tool was incompatible");
    assert_eq!(repaired_hash,selected_tool(&fresh.world).0);
    assert_eq!(inherited.world.state.objects["/self/tools/transform-v1"].version,1,
        "old registry must be replaced with a checked CAS, not silently kept");
    check_tool(&inherited.world,17);
    assert!(inherited.world.audit.iter().any(|r|matches!(
        &r.action,Some(Action::Exec{program_hash,..}) if program_hash==&stale_hash
    )),"incompatible inherited tool was never tested");
    println!("S14 negative transfer: fresh_gas={} inherited_gas={} import_gas={} fresh_ticks={} inherited_ticks={}",
        fresh.gas,inherited.gas,inherited.import_gas,fresh.ticks,inherited.ticks);
}
