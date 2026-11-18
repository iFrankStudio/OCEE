//! S19B: only the immutable executable and registry cross fresh worlds.
//! All compatibility choices happen in the active metered VM.
use ocee_organism_seeds::s19_p1_transfer::{
    s19_p1_transfer_aware,s19_transfer_manifest
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

struct Run{world:World,gas:u64,import_gas:u64}
fn write(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    })).code,Code::Ok);
}
fn tool(w:&World)->(String,Program){
    let hash=String::from_utf8(
        w.state.objects["/self/tools/p1-eval-frame"].bytes.clone()).unwrap();
    let program:Program=serde_json::from_slice(&w.state.objects[
        &format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    (hash,program)
}
fn run(world_seed:u64,protocol_seed:u64,donor:Option<&World>)->Run{
    let mut w=World::with_protocol(Config{
        seed:world_seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,protocol_seed);
    let program=s19_p1_transfer_aware();
    let initial=program.content_hash().unwrap();
    write(&mut w,format!("/self/versions/{initial}"),
        program.canonical_bytes());
    write(&mut w,"/self/active".into(),initial.into_bytes());
    let mut import_gas=0u64;
    if let Some(source)=donor{
        let (hash,tool)=tool(source);
        let offset=w.audit.len();
        write(&mut w,format!("/self/versions/{hash}"),tool.canonical_bytes());
        write(&mut w,"/self/tools/p1-eval-frame".into(),hash.into_bytes());
        import_gas=w.audit[offset..].iter().map(|r|
            r.result.gas_charged+r.result.vm_gas_charged).sum();
        assert!(import_gas>0);
        assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
        assert!(w.state.organisms[&ORGANISM_ID].pending_feedback.is_none());
        assert!(w.state.service_events.is_empty());
        assert!(!w.state.objects.contains_key("/self/results/p1-job0/value_le"));
    }
    for _ in 0..16_384 {
        let reply=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::Denied|Code::InvalidCode),
            "seed={protocol_seed} world={world_seed} tick={} {:?}",
            w.state.step,reply.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded S19 transfer run");
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    let gas=w.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum();
    Run{world:w,gas,import_gas}
}
fn assert_correct(w:&World){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    let work:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let actual=u32::from_le_bytes(w.state.objects[
        "/self/results/p1-job0/value_le"].bytes.as_slice().try_into().unwrap());
    let expected=work.values.iter().enumerate().fold(0u32,|s,(j,&v)|
        s.wrapping_add(v.rotate_left((j%32) as u32)));
    assert_eq!(actual,expected);
    assert_eq!(w.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>(),20);
}
#[test]
fn same_protocol_fresh_world_reuses_only_its_verified_executable(){
    assert_eq!(s19_transfer_manifest().program_hash,
        format!("sha256:{}",s19_p1_transfer_aware().content_hash().unwrap()));
    let donor=run(42,42,None);
    assert_correct(&donor.world);
    let cold=run(128,42,None); // fresh world RNG, same P1 protocol instance
    let reused=run(128,42,Some(&donor.world));
    assert_correct(&cold.world);
    assert_correct(&reused.world);
    assert_eq!(tool(&donor.world).0,tool(&reused.world).0);
    assert!(reused.import_gas>0);
    assert!(reused.gas<cold.gas,
        "including import: reused {} vs cold {}",reused.gas,cold.gas);
    assert!(reused.world.state.step<cold.world.state.step);
    // Probe uses the already existing tool on an empty workload and
    // receives BadBody; the protocol still only charges B for real work.
    assert!(reused.world.audit.iter().filter(|r|matches!(
        &r.action,Some(Action::Exec{..}))).count()>=2);
    println!("S19 compatible inherited: cold_a_gas={} reused_a_gas={} import_a_gas={} cold_ticks={} reused_ticks={}",
        cold.gas,reused.gas,reused.import_gas,
        cold.world.state.step,reused.world.state.step);
}
#[test]
fn incompatible_inherited_p1_tool_is_rejected_then_relearned(){
    let donor=run(272,272,None); // cheap, different P1 key and opcode
    assert_correct(&donor.world);
    let stale_hash=tool(&donor.world).0;
    let cold=run(128,42,None);
    let transferred=run(128,42,Some(&donor.world));
    assert_correct(&cold.world);
    assert_correct(&transferred.world);
    let new_hash=tool(&transferred.world).0;
    assert_ne!(stale_hash,new_hash);
    assert_eq!(new_hash,tool(&cold.world).0);
    assert_eq!(transferred.world.state.objects[
        "/self/tools/p1-eval-frame"].version,1,
        "stale registry hash must be replaced with first version CAS(0)");
    assert!(transferred.world.audit.iter().any(|r|matches!(
        &r.action,Some(Action::Exec{program_hash,..}) if program_hash==&stale_hash
    )),"must actually test inherited candidate");
    assert!(transferred.import_gas>0);
    assert!(transferred.gas>cold.gas,
        "negative migration should not be reported as free improvement");
    println!("S19 incompatible inherited: cold_a_gas={} fallback_a_gas={} import_a_gas={} cold_ticks={} fallback_ticks={}",
        cold.gas,transferred.gas,transferred.import_gas,
        cold.world.state.step,transferred.world.state.step);
}
