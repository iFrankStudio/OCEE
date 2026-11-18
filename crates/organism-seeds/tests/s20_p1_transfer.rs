//! S20: two independently learned immutable P1 programs genuinely travel
//! to new worlds. Counts include explicit paid WRITE import costs. Exact
//! service seeds are used by the TEST only; VM observes only HELLO/C feedback.
use ocee_organism_seeds::s20_p1_transfer::{s20_p1_dual_tool_transfer,s20_manifest};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::{Family,Operation};
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

struct Run { world:World, total_gas:u64, import_gas:u64 }
fn write(world:&mut World,key:String,bytes:Vec<u8>){
    let out=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None,
    }));
    assert_eq!(out.code,Code::Ok,"must import only legally authored objects");
}
fn tool(world:&World,name:&str)->(String,Program){
    let path=format!("/self/tools/{name}");
    let hash=String::from_utf8(world.state.objects[&path].bytes.clone()).unwrap();
    let program:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    (hash,program)
}
fn run(world_seed:u64,protocol_seed:u64,
    donor_eval:Option<&World>,donor_result:Option<&World>)->Run{
    let program=s20_p1_dual_tool_transfer();
    let initial=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed:world_seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,protocol_seed);
    write(&mut world,format!("/self/versions/{initial}"),program.canonical_bytes());
    write(&mut world,"/self/active".into(),initial.into_bytes());
    let offset=world.audit.len();
    if let Some(donor)=donor_eval{
        let (hash,p)=tool(donor,"p1-eval-frame");
        write(&mut world,format!("/self/versions/{hash}"),p.canonical_bytes());
        write(&mut world,"/self/tools/p1-eval-frame".into(),hash.into_bytes());
    }
    if let Some(donor)=donor_result{
        let (hash,p)=tool(donor,"p1-result-frame");
        write(&mut world,format!("/self/versions/{hash}"),p.canonical_bytes());
        write(&mut world,"/self/tools/p1-result-frame".into(),hash.into_bytes());
    }
    let import_gas=world.audit[offset..].iter().map(|e|
        e.result.gas_charged+e.result.vm_gas_charged).sum();
    let expected_self=2+2*usize::from(donor_eval.is_some())
        +2*usize::from(donor_result.is_some());
    assert_eq!(world.state.objects.keys().filter(|key|
        key.starts_with("/self/")).count(),expected_self,
        "a fresh episode must inherit exactly the whitelisted objects");
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
    assert!(world.state.organisms[&ORGANISM_ID].pending_feedback.is_none());
    assert!(world.state.organisms[&ORGANISM_ID].last_return.is_none());
    assert!(world.state.mailboxes.is_empty() && world.state.service_events.is_empty());
    assert!(!world.state.objects.contains_key("/self/results/p1-job0/value_le"));
    for _ in 0..16384{
        let out=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(out.code,Code::InvalidCode|Code::Denied),
            "S20 VM error protocol={protocol_seed} world={world_seed} tick={} {:?}",
            world.state.step,out.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished) {break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"S20 active VM did not terminate");
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash(),
        "S20 must be deterministically replayable");
    let total_gas=world.audit.iter().map(|event|
        event.result.gas_charged+event.result.vm_gas_charged).sum();
    Run {world,total_gas,import_gas}
}
fn correct(world:&World){
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"successful RESULT required");
    let request:WorkItem=serde_json::from_slice(
        &world.state.objects["/public/E001/job0"].bytes).unwrap();
    let output=&world.state.objects["/self/results/p1-job0/value_le"].bytes;
    let value=u32::from_le_bytes(output.as_slice().try_into().unwrap());
    let independent=request.values.iter().enumerate().fold(0u32,|sum,(i,&v)|
        sum.wrapping_add(v.rotate_left((i%32) as u32)));
    assert_eq!(value,independent,"must return actual B-computed FOLD32");
    assert_eq!(world.state.service_events.iter()
        .map(|s|s.b_gas_charged).sum::<u64>(),20,
        "only one real B FOLD32 request should be charged");
    for name in ["p1-eval-frame","p1-result-frame"] {
        let (_,p)=tool(world,name);
        assert!(p.validate().is_ok());
    }
}
fn uses_result_exec(world:&World,hash:&str)->bool{
    world.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,input,..})
            if program_hash==hash && input==&vec![17,0,0,0]
    ))
}
#[test]
fn cold_vm_generates_distinct_verified_remote_and_result_programs() {
    assert_eq!(s20_manifest().program_hash,
        format!("sha256:{}",s20_p1_dual_tool_transfer().content_hash().unwrap()));
    for seed in [42u64,272] {
        let r=run(seed,seed,None,None);
        correct(&r.world);
        let (remote,remote_program)=tool(&r.world,"p1-eval-frame");
        let (result,result_program)=tool(&r.world,"p1-result-frame");
        assert_ne!(remote,result);
        let reference_result=run_isolated(&result_program,vec![17,0,0,0],2_000)
            .unwrap().0;
        let Value::Bytes(wire)=reference_result else {panic!("result tool bytes")};
        let decoded=r.world.state.protocol_service.as_ref().unwrap()
            .protocol.decode(&wire).unwrap();
        assert_eq!(decoded.operation,Operation::Result);
        assert_eq!(decoded.request_id,17);
        assert!(decoded.body.is_empty());
        assert!(uses_result_exec(&r.world,&result),
            "initial RESULT specialization should have been verified by EXEC");
        assert_ne!(remote_program,result_program);
        println!("S20 cold seed={seed}: a_gas={} ticks={} remote_hash={} result_hash={} B_gas=20",
            r.total_gas,r.world.state.step,remote,result);
    }
}
#[test]
fn two_executables_reused_with_new_public_work_beat_single_executable_reuse(){
    let donor=run(42,42,None,None);
    correct(&donor.world);
    let cold=run(128,42,None,None);
    let single=run(128,42,Some(&donor.world),None);
    let dual=run(128,42,Some(&donor.world),Some(&donor.world));
    for r in [&cold,&single,&dual]{correct(&r.world);}
    assert_ne!(donor.world.state.objects["/public/E001/job0"].bytes,
        dual.world.state.objects["/public/E001/job0"].bytes,
        "fresh episode must use new public task data");
    for name in ["p1-eval-frame","p1-result-frame"] {
        assert_eq!(tool(&donor.world,name).0,tool(&dual.world,name).0);
    }
    assert_eq!(tool(&single.world,"p1-result-frame").0,
        tool(&donor.world,"p1-result-frame").0,
        "single-tool variant should relearn same RESULT parameters");
    let result_hash=tool(&donor.world,"p1-result-frame").0;
    assert!(uses_result_exec(&dual.world,&result_hash));
    assert!(dual.import_gas>single.import_gas);
    assert!(dual.total_gas<single.total_gas,
        "second executable must amortize its paid import cost");
    assert!(dual.total_gas<cold.total_gas);
    assert!(dual.world.state.step<single.world.state.step);
    println!("S20 compatible cold={} single={} dual={} import_single={} import_dual={} ticks={}/{}/{}",
        cold.total_gas,single.total_gas,dual.total_gas,
        single.import_gas,dual.import_gas,
        cold.world.state.step,single.world.state.step,dual.world.state.step);
}
#[test]
fn stale_remote_and_result_tools_are_rejected_and_relearned(){
    let stale=run(272,272,None,None);
    correct(&stale.world);
    let cold=run(128,42,None,None);
    let migrated=run(128,42,Some(&stale.world),Some(&stale.world));
    correct(&cold.world);
    correct(&migrated.world);
    for name in ["p1-eval-frame","p1-result-frame"] {
        assert_ne!(tool(&stale.world,name).0,tool(&migrated.world,name).0);
        assert_eq!(tool(&cold.world,name).0,tool(&migrated.world,name).0);
        assert_eq!(migrated.world.state.objects[
            &format!("/self/tools/{name}")].version,1,
            "stale registry should be replaced using CAS(0)");
    }
    assert!(migrated.total_gas>cold.total_gas);
    println!("S20 both-stale: cold={} recovered={} import={} ticks={}/{}",
        cold.total_gas,migrated.total_gas,migrated.import_gas,
        cold.world.state.step,migrated.world.state.step);
}
#[test]
fn only_stale_result_tool_falls_back_without_relearning_remote(){
    let good=run(42,42,None,None);
    let wrong=run(272,272,None,None);
    let mixed=run(128,42,Some(&good.world),Some(&wrong.world));
    let good_dual=run(128,42,Some(&good.world),Some(&good.world));
    correct(&mixed.world);
    correct(&good_dual.world);
    assert_eq!(tool(&mixed.world,"p1-eval-frame").0,
        tool(&good.world,"p1-eval-frame").0);
    assert_eq!(tool(&mixed.world,"p1-eval-frame").0,
        tool(&good_dual.world,"p1-eval-frame").0);
    assert_ne!(tool(&mixed.world,"p1-result-frame").0,
        tool(&wrong.world,"p1-result-frame").0);
    assert_eq!(tool(&mixed.world,"p1-result-frame").0,
        tool(&good.world,"p1-result-frame").0);
    assert_eq!(mixed.world.state.objects["/self/tools/p1-eval-frame"].version,0);
    assert_eq!(mixed.world.state.objects["/self/tools/p1-result-frame"].version,1);
    assert!(mixed.total_gas>good_dual.total_gas);
    // Even the stale tool is actually called once through isolated EXEC;
    // the real C error, not privileged protocol state, rejects it.
    let stale_hash=tool(&wrong.world,"p1-result-frame").0;
    assert!(uses_result_exec(&mixed.world,&stale_hash));
    println!("S20 result-only stale: both_valid={} only_result_stale={} import={}",
        good_dual.total_gas,mixed.total_gas,mixed.import_gas);
}

#[test]
fn three_fresh_episodes_accumulate_both_tools_without_state_clone(){
    let first=run(42,42,None,None);
    let second_cold=run(128,42,None,None);
    let third_cold=run(129,42,None,None);
    let second=run(128,42,Some(&first.world),Some(&first.world));
    let third=run(129,42,Some(&second.world),Some(&second.world));
    for episode in [&first,&second,&third,&second_cold,&third_cold]{
        correct(&episode.world);
    }
    let first_work=&first.world.state.objects["/public/E001/job0"].bytes;
    assert_ne!(*first_work,second.world.state.objects["/public/E001/job0"].bytes);
    assert_ne!(second.world.state.objects["/public/E001/job0"].bytes,
        third.world.state.objects["/public/E001/job0"].bytes);
    for name in ["p1-eval-frame","p1-result-frame"]{
        let hash=tool(&first.world,name).0;
        assert_eq!(tool(&second.world,name).0,hash);
        assert_eq!(tool(&third.world,name).0,hash);
    }
    assert!(second.import_gas>0 && third.import_gas>0);
    let cumulative_inherited=first.total_gas+second.total_gas+third.total_gas;
    let cumulative_cold=first.total_gas+second_cold.total_gas+third_cold.total_gas;
    assert!(cumulative_inherited<cumulative_cold);
    assert!(third.world.state.step<third_cold.world.state.step);
    println!("S20 3-world P1 cumulative: inherited={} all_cold={} second_import={} third_import={} second_dual={} third_dual={} second_cold={} third_cold={}",
        cumulative_inherited,cumulative_cold,
        second.import_gas,third.import_gas,
        second.total_gas,third.total_gas,
        second_cold.total_gas,third_cold.total_gas);
}

#[test]
fn snapshot_after_result_program_write_before_result_registry_is_exact(){
    let initial_program=s20_p1_dual_tool_transfer();
    let hash=initial_program.content_hash().unwrap();
    let mut baseline=World::with_protocol(Config{
        seed:42,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,42);
    write(&mut baseline,format!("/self/versions/{hash}"),
        initial_program.canonical_bytes());
    write(&mut baseline,"/self/active".into(),hash.into_bytes());
    let mut found_milestone=false;
    for _ in 0..2048{
        let reply=baseline.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::InvalidCode|Code::Denied));
        let program_count=baseline.state.objects.keys()
            .filter(|k|k.starts_with("/self/versions/")).count();
        if program_count==3 &&
            !baseline.state.objects.contains_key("/self/tools/p1-result-frame"){
            found_milestone=true;
            break;
        }
    }
    assert!(found_milestone,
        "capture exact unregistered second executable rather than arbitrary tick");
    assert!(baseline.state.objects.contains_key("/self/tools/p1-eval-frame"));
    let snapshot=baseline.snapshot_json();
    let mut restarted=World::from_snapshot(&snapshot).unwrap();
    assert_eq!(baseline.state_hash(),restarted.state_hash());
    for world in [&mut baseline,&mut restarted]{
        for _ in 0..2048{
            let reply=world.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(reply.code,Code::InvalidCode|Code::Denied));
            if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .is_some_and(|c|c.finished){break;}
        }
        assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished));
        correct(world);
    }
    assert_eq!(baseline.state_hash(),restarted.state_hash());
    assert_eq!(tool(&baseline,"p1-result-frame").0,
        tool(&restarted,"p1-result-frame").0);
    println!("S20 exact result-registration restart step={} terminal_hash={} B_gas=20",
        baseline.state.step,baseline.state_hash());
}
