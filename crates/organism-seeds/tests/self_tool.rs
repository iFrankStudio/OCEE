//! Provisional S1->tool proof: one pre-seeded VM organism discovers an opcode,
//! synthesizes a *new executable* derived from a generic framed-message template,
//! self-tests it via metered EXEC, and registers it. No researcher handoff.
//! The frame algorithm itself is seeded; this does NOT prove invention.
use ocee_organism_seeds::p0_probe::{
    s1_p0_tool_builder,s1_p0_tool_builder_no_feedback
};
use ocee_organism_vm::{Value,Program};
use ocee_protocol_sim::{Family,Operation,FrameKind};
use ocee_world_kernel::{Action,Code,Config,World,NodeId,ORGANISM_ID};

fn bootstrap(world:&mut World,p:Program)->String {
    p.validate().unwrap();
    let hash=p.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:p.canonical_bytes(),
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec(),
    })).code,Code::Ok);
    hash
}
fn run(world:&mut World,max_ticks:usize) {
    for _ in 0..max_ticks {
        let outcome=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(outcome.code,Code::InvalidCode|Code::Denied|Code::Gas),
            "VM failed at {}: {:?}",world.state.step,outcome.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|vm|vm.finished) {return;}
    }
    panic!("program failed to finish within {max_ticks} ticks");
}
fn payload(id:u32,values:&[u32])->Vec<u8> {
    let mut b=id.to_le_bytes().to_vec();
    b.extend_from_slice(&(values.len() as u16).to_le_bytes());
    for &x in values {b.extend_from_slice(&x.to_le_bytes());}
    b
}
fn evaluated_frame(world:&mut World,hash:&str,input:Vec<u8>)->Vec<u8>{
    let result=world.tick(Some(Action::Exec{
        actor:ORGANISM_ID,program_hash:hash.to_owned(),input
    }));
    assert_eq!(result.code,Code::Ok);
    let value:Value=serde_json::from_slice(&result.payload).unwrap();
    match value {Value::Bytes(bytes)=>bytes, _=>panic!("expected frame bytes")}
}
#[test]
fn vm_specializes_and_registers_reusable_tool_then_runs_two_distinct_workloads() {
    for seed in [7u64,42,99] {
        let mut world=World::with_protocol(Config{
            seed,a_gas_per_epoch:100_000,..Config::default()
        },Family::P0,seed);
        let initial=bootstrap(&mut world,s1_p0_tool_builder());
        run(&mut world,900);
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(initial.clone())); // no human switches it to a new stage
        let tool_hash=String::from_utf8(world.state.objects[
            "/self/tools/p0-eval-frame"].bytes.clone()).unwrap();
        assert_ne!(tool_hash,initial);
        let tool_path=format!("/self/versions/{tool_hash}");
        let built=&world.state.objects[&tool_path].bytes;
        let program:Program=serde_json::from_slice(built).unwrap();
        assert_eq!(program.content_hash().unwrap(),tool_hash);
        assert_eq!(program.canonical_bytes(),*built);
        let writes=world.audit.iter().filter_map(|a|{
            if let Some(Action::Write{key,..})=&a.action {Some(key.as_str())} else {None}
        }).collect::<Vec<_>>();
        assert!(writes.contains(&tool_path.as_str()));
        assert!(writes.contains(&"/self/tools/p0-eval-frame"));
        // An isolated VM EXEC must have occurred before registration.
        assert!(world.audit.iter().any(|a|matches!(
            &a.action,Some(Action::Exec{program_hash,..}) if program_hash==&tool_hash
        )));

        let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let cases=[(77u32,vec![9u32]),(78u32,vec![11u32,12,13,14])];
        let before=world.state.service_events.iter()
            .filter(|x|x.b_gas_charged>0).count();
        for (id,values) in cases {
            let input=payload(id,&values);
            let wire=evaluated_frame(&mut world,&tool_hash,input.clone());
            let decoded=proto.decode(&wire).expect("generated frame accepted by P0 decoder");
            assert_eq!(decoded.kind,FrameKind::Request);
            assert_eq!(decoded.operation,Operation::RemoteEval);
            assert_eq!(decoded.request_id,id);
            assert_eq!(decoded.body,input[4..]);
            assert_eq!(world.tick(Some(Action::Send{
                actor:ORGANISM_ID,destination:NodeId::C,bytes:wire
            })).code,Code::Ok);
            // Give the service a chance to process and return a response.
            for _ in 0..5 {world.tick(None);}
        }
        let after=world.state.service_events.iter()
            .filter(|x|x.b_gas_charged>0).count();
        assert_eq!(after-before,2,"both unseen workloads must use B");
        assert!(world.state.service_events.iter().any(|x|x.b_gas_charged==12));
        assert!(world.state.service_events.iter().any(|x|x.b_gas_charged==24));
        let restored=World::from_snapshot(&world.snapshot_json()).unwrap();
        assert_eq!(restored.state_hash(),world.state_hash());
    }
}
#[test]
fn diagnostic_blind_control_must_not_synthesize_or_register_a_tool(){
    let mut world=World::with_protocol(Config{
        seed:7,a_gas_per_epoch:100_000,..Config::default()
    },Family::P0,7);
    let initial=bootstrap(&mut world,s1_p0_tool_builder_no_feedback());
    run(&mut world,900);
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert_eq!(world.state.objects.keys().filter(|k|k.starts_with("/self/versions/")).count(),1);
    assert!(!world.state.objects.contains_key("/self/tools/p0-eval-frame"));
    assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,Some(initial));
}
