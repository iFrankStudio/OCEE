//! S19: active organism specializes a seeded P1 executable and uses it.
use ocee_organism_seeds::s19_p1_tool::{
    s19_p1_build_and_use_tool,s19_p1_broken_template_control,s19_manifest,
    s19_seeded_frame_template
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::{Family,Operation};
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn run(seed:u64,broken:bool)->World {
    let prog=if broken{s19_p1_broken_template_control()}
        else{s19_p1_build_and_use_tool()};
    let hash=prog.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),prog.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    for _ in 0..16_384 {
        let reply=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::InvalidCode|Code::Denied),
            "seed={seed} step={} VM code={:?}",w.state.step,reply.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"S19 did not terminate seed={seed}");
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    w
}
fn gas(w:&World)->u64 {
    w.audit.iter().map(|a|a.result.gas_charged+a.result.vm_gas_charged).sum()
}
#[test]
fn vm_specializes_writes_executes_and_registers_a_real_p1_tool() {
    assert_eq!(s19_manifest().program_hash,
        format!("sha256:{}",s19_p1_build_and_use_tool().content_hash().unwrap()));
    assert!(s19_seeded_frame_template().validate().is_ok());
    for seed in [7u64,42,99] {
        let w=run(seed,false);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)),
            "seed={seed}");
        let tool_hash=String::from_utf8(
            w.state.objects["/self/tools/p1-eval-frame"].bytes.clone()).unwrap();
        let raw=&w.state.objects[&format!("/self/versions/{tool_hash}")].bytes;
        let tool:Program=serde_json::from_slice(raw).unwrap();
        assert_eq!(tool.content_hash().unwrap(),tool_hash);
        assert_ne!(tool,s19_seeded_frame_template(),
            "VM must have specialized at least one actual graph operator");
        // The tool is evaluated in the VM; the researcher does not use it
        // to choose a hidden opcode or generate real network actions.
        let call=w.audit.iter().filter_map(|r|match &r.action{
            Some(Action::Exec{program_hash,input,..}) if program_hash==&tool_hash=>
                Some(input.clone()),
            _=>None,
        }).collect::<Vec<_>>();
        assert_eq!(call.len(),2,"one isolated validation, one real job");
        let work:WorkItem=serde_json::from_slice(
            &w.state.objects["/public/E001/job0"].bytes).unwrap();
        let mut job=vec![17,0,0,0,3,0];
        job.extend_from_slice(&work.values_le);
        assert_eq!(call[1],job);
        let wire=run_isolated(&tool,job,2_000).unwrap().0;
        let Value::Bytes(wire)=wire else {panic!("tool must emit P1 bytes")};
        let decoded=w.state.protocol_service.as_ref().unwrap()
            .protocol.decode(&wire).unwrap();
        assert_eq!(decoded.operation,Operation::RemoteEval);
        assert_eq!(decoded.request_id,17);
        assert_eq!(decoded.body[..2],[3,0]);
        assert!(w.audit.iter().any(|a|matches!(
            &a.action,Some(Action::Send{bytes,..}) if bytes==&wire
        )),"a real world SEND must contain the tool's output");
        assert_eq!(w.state.service_events.iter()
            .map(|event|event.b_gas_charged).sum::<u64>(),20);
        let result=&w.state.objects["/self/results/p1-job0/value_le"].bytes;
        let actual=u32::from_le_bytes(result.as_slice().try_into().unwrap());
        let reference=work.values.iter().enumerate().fold(0u32,|sum,(j,&n)|
            sum.wrapping_add(n.rotate_left((j%32) as u32)));
        assert_eq!(actual,reference);
        let vm_gas:u64=w.audit.iter().map(|r|r.result.vm_gas_charged).sum();
        let exec_gas:u64=w.audit.iter().filter(|r|matches!(
            r.action,Some(Action::Exec{..}))).map(|r|r.result.gas_charged).sum();
        println!("S19 cold seed={seed} ticks={} a_gas={} vm_gas={vm_gas} exec_gas={exec_gas} tool_bytes={} B_gas=20",
            w.state.step,gas(&w),raw.len());
    }
}
#[test]
fn isolated_exec_ok_is_insufficient_for_broken_tool_adoption() {
    let w=run(42,true);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(w.state.objects.contains_key("/self/discoveries/p1-operation-candidate"));
    assert!(!w.state.objects.contains_key("/self/tools/p1-eval-frame"));
    assert!(!w.state.objects.contains_key("/self/results/p1-job0/value_le"));
    assert_eq!(w.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>(),0);
    let exec=w.audit.iter().filter(|e|matches!(e.action,Some(Action::Exec{..})))
        .collect::<Vec<_>>();
    assert_eq!(exec.len(),1);
    assert_eq!(exec[0].result.code,Code::Ok,
        "broken tool must actually execute successfully before rejection");
}
