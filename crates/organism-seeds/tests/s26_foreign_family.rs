//! S26A: actual active VM must quarantine a successfully executable P1
//! encoder imported under a P2 tool registry. The research host only moves
//! the donor's sealed bytes and registry hash; no secret P2 data is passed.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,expected_version:None,bytes,
    })).code,Code::Ok);
}
fn boot(family:Family,seed:u64,proto:u64,p:Program)->World{
    let hash=p.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,proto);
    put(&mut w,format!("/self/versions/{hash}"),p.canonical_bytes());
    put(&mut w,"/self/active".into(),hash.into_bytes());
    w
}
fn finish(w:&mut World){
    for _ in 0..16_384{
        let reply=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::Denied|Code::InvalidCode),
            "a wrong-family tool is recoverable: step={} code={:?} block={:?}",
            w.state.step,reply.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .map(|c|c.block.as_str()));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn a_gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn tool(w:&World,path:&str)->(String,Program){
    let hash=String::from_utf8(w.state.objects[path].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
fn verify_b(w:&World){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"the eventual B task must succeed");
    assert_eq!(b_gas(w),20,"no duplicate B tasks on fallback");
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let independent=job.values.iter().enumerate().fold(0u32,|s,(j,&n)|
        s.wrapping_add(n.rotate_left((j%32) as u32)));
    let bytes=&w.state.objects["/self/results/p2-job0/value_le"].bytes;
    assert_eq!(u32::from_le_bytes(bytes.as_slice().try_into().unwrap()),independent);
}
#[test]
fn foreign_p1_encoder_is_metered_rejected_and_relearned_by_real_p2_vm(){
    // The real P1 active organism first creates and registers its executable.
    let mut donor=boot(Family::P1,42,42,s19_p1_build_and_use_tool());
    finish(&mut donor);
    assert_eq!(donor.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    let (foreign_hash,foreign_program)=tool(&donor,"/self/tools/p1-eval-frame");

    // Paired receiver worlds have identical world/protocol/job seeds, initial
    // organism and budgets. Only D has the paid imported foreign tool.
    let mut cold=boot(Family::P2,128,42,s23_p2_remote_tool_transfer());
    let mut imported=boot(Family::P2,128,42,s23_p2_remote_tool_transfer());
    put(&mut imported,format!("/self/versions/{foreign_hash}"),
        foreign_program.canonical_bytes());
    put(&mut imported,"/self/tools/p2-eval-frame".into(),
        foreign_hash.clone().into_bytes());
    let import_cost=a_gas(&imported)-a_gas(&cold);
    assert!(import_cost>0);
    finish(&mut cold);
    finish(&mut imported);
    verify_b(&cold);
    verify_b(&imported);
    assert_eq!(cold.state.objects["/public/E001/job0"].bytes,
        imported.state.objects["/public/E001/job0"].bytes);
    let (cold_hash,_)=tool(&cold,"/self/tools/p2-eval-frame");
    let (learned_hash,_)=tool(&imported,"/self/tools/p2-eval-frame");
    assert_eq!(learned_hash,cold_hash,"foreign P1 graph must not be adopted");
    assert_ne!(foreign_hash,learned_hash);
    assert_eq!(imported.state.objects["/self/tools/p2-eval-frame"].version,1,
        "failed inherited tool must be CAS replaced");
    assert!(imported.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,..}) if program_hash==&foreign_hash
    )),"the active VM must really EXEC the foreign inherited tool");
    assert!(imported.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,..}) if program_hash==&learned_hash
    )),"the active VM must EXEC its relearned P2 tool");
    assert!(a_gas(&imported)>a_gas(&cold),
        "foreign tool's paid import/verification must show negative transfer");
    println!("S26A foreign P1->P2 cold_A={} imported_A={} import_A={} delta_A={} cold_ticks={} imported_ticks={} B=20",
        a_gas(&cold),a_gas(&imported),import_cost,
        a_gas(&imported)-a_gas(&cold),cold.state.step,imported.state.step);
}
