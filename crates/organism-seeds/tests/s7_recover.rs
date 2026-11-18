//! S7 scientific unit: an S6-trained tool remains broken on []; the same
//! running VM observes actual failed EXEC feedback, inserts generic guard
//! control-flow blocks, checks a public regression example, then CAS-adopts
//! the new tool version. None of these steps is a researcher program swap.
use ocee_organism_seeds::s7_recover::{
    s7_empty_input_recovery,s7_no_repair_control,s7_manifest
};
use ocee_organism_vm::{Program,Type,Value,VmError,Terminator,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,Code,Config,World,ORGANISM_ID,hidden_compose
};

fn run(seed:u64,program:Program)->World {
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },Family::P0,seed);
    program.validate().unwrap();
    let hash=program.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    for _ in 0..1500 {
        let r=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::InvalidCode|Code::Denied),
            "host VM itself failed at step {}: {:?}",world.state.step,r.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|v|v.finished) {return world;}
    }
    panic!("S7 did not finish after 1500 bounded ticks");
}
fn registry_hash(world:&World)->String {
    String::from_utf8(world.state.objects["/self/tools/transform-v1"]
        .bytes.clone()).unwrap()
}
fn get_version(world:&World,hash:&str)->Program {
    let object=&world.state.objects[&format!("/self/versions/{hash}")];
    let p:Program=serde_json::from_slice(&object.bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    p
}
#[test]
fn same_vm_detects_failing_empty_input_and_adopts_guarded_control_flow(){
    let initial=s7_empty_input_recovery();
    let hash=initial.content_hash().unwrap();
    let manifest=s7_manifest();
    assert_eq!(manifest.program_hash,format!("sha256:{hash}"));
    assert!(manifest.research_status.contains("NOT_M0"));
    for seed in [2u64,5,16] {
        let world=run(seed,initial.clone());
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash.clone()),"host must not swap active VM program");
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(2)),"seed {seed}: failed to repair");

        let original_hash=world.audit.iter().filter_map(|entry|{
            if let Some(Action::Write{key,bytes,..})=&entry.action {
                if key=="/self/tools/transform-v1" {
                    return Some(String::from_utf8(bytes.clone()).unwrap());
                }
            }
            None
        }).next().expect("first registration");
        let repaired_hash=registry_hash(&world);
        assert_ne!(original_hash,repaired_hash);
        let original=get_version(&world,&original_hash);
        let repaired=get_version(&world,&repaired_hash);
        assert_eq!(run_isolated(&original,vec![],2000),Err(VmError::Range));
        assert_eq!(run_isolated(&repaired,vec![],2000).unwrap().0,Value::U32(0));
        assert_eq!(repaired.functions[0].blocks.len(),3);
        assert_eq!(repaired.functions[0].blocks[0].id,"guard");
        assert!(matches!(&repaired.functions[0].blocks[0].terminator,
            Terminator::Branch{if_true,if_false,..}
            if if_true=="empty"&&if_false=="entry"));
        assert!(repaired.functions[0].blocks[1].id=="empty");
        assert_eq!(repaired.functions[0].returns,Type::U32);
        assert_eq!(world.state.objects["/self/tools/transform-v1"].version,1,
            "tool adopted via versioned CAS, original retained for rollback");
        let fault:ActionResult=serde_json::from_slice(
            &world.state.objects["/self/repair/empty/probe"].bytes).unwrap();
        assert_eq!(fault.code,Code::InvalidCode);
        assert!(world.audit.iter().any(|e|matches!(
            &e.action,Some(Action::Exec{program_hash,input,..})
            if program_hash==&original_hash && input.is_empty()
                && e.result.code==Code::InvalidCode
        )), "actual world fault must trigger repair");
        assert!(world.audit.iter().any(|e|matches!(
            &e.action,Some(Action::Exec{program_hash,input,..})
            if program_hash==&repaired_hash && input.is_empty()
                && e.result.code==Code::Ok
        )), "repaired program must pass empty probe in VM");

        let novel_inputs: [&[u8];4]=[
            &[1,2,3,4,5,6], &[99,0,13],
            &[200,7,41,17,9], &[0,0,0,0,0,1,2,3]
        ];
        for input in novel_inputs {
            let value=run_isolated(&repaired,input.to_vec(),2000).unwrap().0;
            assert_eq!(value,Value::U32(hidden_compose(seed,input)),
                "regression input failed seed {seed} input {input:?}");
        }
        assert_eq!(world.state_hash(),
            World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn disabling_in_world_graph_repair_preserves_fault_and_previous_tool(){
    let world=run(2,s7_no_repair_control());
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(world.state.objects["/self/tools/transform-v1"].version,0);
    let old=get_version(&world,&registry_hash(&world));
    assert_eq!(run_isolated(&old,vec![],2000),Err(VmError::Range));
    let observed:ActionResult=serde_json::from_slice(
        &world.state.objects["/self/repair/empty/probe"].bytes).unwrap();
    assert_eq!(observed.code,Code::InvalidCode);
}
#[test]
fn a_function_that_already_handles_empty_does_not_mutate_its_tool(){
    let world=run(0,s7_empty_input_recovery());
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(world.state.objects["/self/tools/transform-v1"].version,0);
    assert!(!world.state.objects.contains_key("/self/repair/empty/probe"));
    let current=get_version(&world,&registry_hash(&world));
    assert_eq!(current.functions[0].blocks.len(),1);
    assert_eq!(run_isolated(&current,vec![],2000).unwrap().0,Value::U32(0));
}
