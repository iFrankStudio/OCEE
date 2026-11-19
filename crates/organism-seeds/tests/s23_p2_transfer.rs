//! S23 P2 tool validation and paid transfer: executable data, not copied
//! continuation or B results. The test's private oracle is used AFTER the
//! active VM completes; the organism never sees protocol seed/opcode.
use ocee_organism_seeds::s23_p2_transfer::{s23_p2_remote_tool_transfer,s23_manifest};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::{Family,Operation};
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

struct Run{world:World,a_gas:u64,import_gas:u64}
fn write(world:&mut World,key:String,bytes:Vec<u8>){
    let res=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,expected_version:None,bytes,
    }));
    assert_eq!(res.code,Code::Ok,"initial/import WRITE denied");
}
fn get_tool(world:&World)->(String,Program){
    let hash=String::from_utf8(
        world.state.objects["/self/tools/p2-eval-frame"].bytes.clone()).unwrap();
    let program:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),hash);
    (hash,program)
}
fn trial(world_seed:u64,proto_seed:u64,donor:Option<&World>)->Run{
    let p=s23_p2_remote_tool_transfer();
    let h=p.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed:world_seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,proto_seed);
    write(&mut world,format!("/self/versions/{h}"),p.canonical_bytes());
    write(&mut world,"/self/active".into(),h.into_bytes());
    let before=world.audit.len();
    if let Some(from)=donor{
        let (hash,tool)=get_tool(from);
        write(&mut world,format!("/self/versions/{hash}"),tool.canonical_bytes());
        write(&mut world,"/self/tools/p2-eval-frame".into(),hash.into_bytes());
        assert_eq!(world.state.objects.keys().filter(
            |k|k.starts_with("/self/")).count(),4);
        assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
        assert!(world.state.organisms[&ORGANISM_ID].pending_feedback.is_none());
        assert!(world.state.organisms[&ORGANISM_ID].last_return.is_none());
        assert!(world.state.mailboxes.is_empty());
        assert!(world.state.service_events.is_empty());
        assert!(!world.state.objects.contains_key("/self/results/p2-job0/value_le"));
    }
    let import_gas=world.audit[before..].iter()
        .map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum();
    for _ in 0..16384 {
        let r=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "S23 VM fault world={world_seed} protocol={proto_seed} step={} {:?}",
            world.state.step,r.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    let a_gas=world.audit.iter().map(|e|
        e.result.gas_charged+e.result.vm_gas_charged).sum();
    Run{world,a_gas,import_gas}
}
fn validate(world:&World){
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"must complete B result and acknowledgement");
    let roles=&world.state.objects["/self/discoveries/p2-tlv-roles"].bytes;
    assert_eq!(roles.len(),11);
    let learned=&world.state.objects["/self/discoveries/p2-remote-opcode"].bytes;
    assert_eq!(learned.len(),1);
    let (hash,program)=get_tool(world);
    assert!(program.validate().is_ok());
    let observed=world.state.protocol_service.as_ref().unwrap();
    let work:WorkItem=serde_json::from_slice(
        &world.state.objects["/public/E001/job0"].bytes).unwrap();
    let mut body=vec![3,0];
    body.extend(&work.values_le);
    let (encoded,used_gas)=run_isolated(&program,body,3000).unwrap();
    assert!(used_gas>0);
    let Value::Bytes(frame)=encoded else {panic!("tool must output bytes")};
    let decoded=observed.protocol.decode(&frame).unwrap();
    assert_eq!(decoded.operation,Operation::RemoteEval);
    assert_eq!(decoded.request_id,17);
    assert_eq!(&decoded.body[..2],[3,0]);
    assert_eq!(decoded.body[2..],work.values_le);
    let sents=world.audit.iter().filter_map(|e|match &e.action {
        Some(Action::Send{bytes,..})=>Some(bytes.clone()),_=>None,
    }).collect::<Vec<_>>();
    assert!(sents.contains(&frame),"real B task must use EXEC-generated frame");
    assert!(world.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,..}) if program_hash==&hash
    )),"VM must actually EXEC its sealed specialized program");
    let value=&world.state.objects["/self/results/p2-job0/value_le"].bytes;
    let observed_value=u32::from_le_bytes(value.as_slice().try_into().unwrap());
    let independent=work.values.iter().enumerate().fold(0u32,|x,(i,&v)|
        x.wrapping_add(v.rotate_left((i%32) as u32)));
    assert_eq!(observed_value,independent);
    assert_eq!(world.state.service_events.iter()
        .map(|e|e.b_gas_charged).sum::<u64>(),20);
}
#[test]
fn active_vm_specializes_seals_probes_and_execs_a_real_p2_tool(){
    assert_eq!(s23_manifest().program_hash,
        format!("sha256:{}",s23_p2_remote_tool_transfer().content_hash().unwrap()));
    for seed in [42u64,99]{
        let run=trial(seed,seed,None);
        validate(&run.world);
        assert_eq!(run.world.state.objects["/self/tools/p2-eval-frame"].version,0);
        assert!(run.world.state.objects.contains_key(
            "/self/candidates/p2-eval-hash"));
        println!("S23 cold P2 seed={seed} A_gas={} ticks={} import_gas={} B_gas=20",
            run.a_gas,run.world.state.step,run.import_gas);
    }
}
#[test]
fn compatible_executable_is_live_verified_before_new_episode_use(){
    let donor=trial(42,42,None);
    validate(&donor.world);
    let cold=trial(128,42,None);
    let reused=trial(128,42,Some(&donor.world));
    validate(&cold.world);
    validate(&reused.world);
    assert_ne!(donor.world.state.objects["/public/E001/job0"].bytes,
        reused.world.state.objects["/public/E001/job0"].bytes);
    assert_eq!(get_tool(&donor.world).0,get_tool(&reused.world).0);
    assert!(reused.import_gas>0);
    assert!(reused.a_gas<cold.a_gas,
        "tool inheritance must beat S23 cold start despite paid import");
    assert!(reused.world.state.step<cold.world.state.step);
    assert_eq!(reused.world.state.objects["/self/tools/p2-eval-frame"].version,0);
    assert!(!reused.world.state.objects.contains_key("/self/candidates/p2-eval-hash"),
        "successful inheritance must not secretly rerun cold specialization");
    println!("S23 compatible cold={} reused={} import={} ticks_cold={} ticks_reuse={}",
        cold.a_gas,reused.a_gas,reused.import_gas,
        cold.world.state.step,reused.world.state.step);
}
#[test]
fn incompatible_executable_is_rejected_then_relearned_with_cas(){
    let stale=trial(7,7,None);
    validate(&stale.world);
    let cold=trial(128,42,None);
    let transferred=trial(128,42,Some(&stale.world));
    validate(&cold.world);
    validate(&transferred.world);
    let bad_hash=get_tool(&stale.world).0;
    let good_hash=get_tool(&cold.world).0;
    assert_ne!(bad_hash,good_hash);
    assert_eq!(get_tool(&transferred.world).0,good_hash);
    assert_eq!(transferred.world.state.objects["/self/tools/p2-eval-frame"].version,1,
        "must replace only stale registry with CAS expected version0");
    assert!(transferred.world.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,..}) if program_hash==&bad_hash
    )),"stale tool must actually be checked via EXEC");
    assert!(transferred.a_gas>cold.a_gas,
        "negative transfer has extra validation/import cost");
    println!("S23 incompatible cold={} transferred={} import={} ticks_cold={} ticks_stale={}",
        cold.a_gas,transferred.a_gas,transferred.import_gas,
        cold.world.state.step,transferred.world.state.step);
}
