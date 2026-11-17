//! S4: a VM program uses generic graph-node rewriting *after* an executable
//! but functionally wrong frame-builder fails an explicit output test.
//! This is a two-hypothesis seeded search; no claim of open-ended synthesis.
use ocee_organism_seeds::s4_structural::{
    s4_p0_structural_repair,s4_p0_no_rewrite_control,s4_manifest
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::{Family,FrameKind,Operation};
use ocee_world_kernel::{Action,ActionResult,Code,Config,Message,World,ORGANISM_ID};

fn boot(program:Program,seed:u64)->World {
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },Family::P0,seed);
    program.validate().unwrap();
    let hash=program.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{actor:ORGANISM_ID,
        key:format!("/self/versions/{hash}"),expected_version:None,
        bytes:program.canonical_bytes()})).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{actor:ORGANISM_ID,
        key:"/self/active".into(),expected_version:None,
        bytes:hash.as_bytes().to_vec()})).code,Code::Ok);
    world
}
fn run(world:&mut World) {
    for _ in 0..1100 {
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::Denied|Code::InvalidCode|Code::Gas),
            "VM failed: step={} result={:?}",world.state.step,result.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {return;}
    }
    panic!("S4 did not terminate");
}
fn versions(world:&World)->Vec<Program> {
    world.state.objects.iter().filter(|(key,_)|key.starts_with("/self/versions/"))
        .map(|(_,obj)|serde_json::from_slice(&obj.bytes).unwrap()).collect()
}
fn checksum_op(program:&Program)->Option<&Op>{
    let first=program.functions.first()?;
    if first.blocks.len()!=1 {return None;}
    first.blocks[0].instructions.iter().find(|i|i.id==6).map(|i|&i.op)
}
#[test]
fn feedback_triggers_actual_rewrite_after_failed_functional_test_and_tool_reuse(){
    let manifest=s4_manifest();
    assert!(manifest.built_in_modules.iter().any(|s|s.contains("hypothesis")));
    assert!(manifest.research_status.contains("NOT_M0"));
    for seed in [7u64,42] {
        let mut world=boot(s4_p0_structural_repair(),seed);
        let active=world.state.organisms[&ORGANISM_ID]
            .active_program_hash.clone().unwrap();
        run(&mut world);
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(active)); // no researcher handoff
        let variants=versions(&world);
        assert_eq!(variants.len(),3); // seed, valid-but-wrong draft, corrected draft
        assert!(variants.iter().any(|p|matches!(checksum_op(p),Some(Op::Len(5)))));
        assert!(variants.iter().any(|p|matches!(checksum_op(p),Some(Op::XorFold(5)))));

        let registered=String::from_utf8(
            world.state.objects["/self/tools/p0-eval-frame"].bytes.clone()).unwrap();
        let correct=world.state.objects.get(&format!("/self/versions/{registered}")).unwrap();
        let corrected:Program=serde_json::from_slice(&correct.bytes).unwrap();
        assert!(matches!(checksum_op(&corrected),Some(Op::XorFold(5))));
        assert!(world.audit.iter().any(|r|matches!(
            &r.action,Some(Action::Exec{program_hash,..})
                if program_hash!=&registered && r.result.code==Code::Ok
        )), "bad structure must have executed successfully but failed output test");
        let ack_raw=&world.state.objects["/self/results/generated-tool/ack"].bytes;
        let action:ActionResult=serde_json::from_slice(ack_raw).unwrap();
        assert_eq!(action.code,Code::Ok);
        let messages:Vec<Message>=serde_json::from_slice(&action.payload).unwrap();
        let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let decoded=proto.decode(&messages[0].bytes).unwrap();
        assert_eq!(decoded.kind,FrameKind::Response);
        assert_eq!(decoded.operation,Operation::RemoteEval);
        assert_eq!(decoded.request_id,100);
        assert_eq!(decoded.body,vec![0]);
        assert_eq!(world.state.service_events.iter()
            .filter(|ev|ev.b_gas_charged==20).count(),1);
        assert_eq!(World::from_snapshot(&world.snapshot_json()).unwrap().state_hash(),
            world.state_hash());
    }
}
#[test]
fn no_structural_rewrite_control_rejects_executable_but_wrong_candidate(){
    let mut world=boot(s4_p0_no_rewrite_control(),7);
    run(&mut world);
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert_eq!(versions(&world).len(),2); // parent and invalid phenotype
    assert!(!world.state.objects.contains_key("/self/tools/p0-eval-frame"));
    assert!(world.audit.iter().any(|r|matches!(&r.action,Some(Action::Exec{..}))
        && r.result.code==Code::Ok));
    assert_eq!(world.state.service_events.iter()
        .filter(|ev|ev.b_gas_charged>0).count(),0);
}
