//! S5: task rule is hidden. One unchanged VM program selects among a typed
//! opcode vocabulary using only public examples, creates/records an executable,
//! then applies it to an unlabelled held-out input. No host-provided tool switch.
use ocee_organism_seeds::s5_search::{
    s5_sample_search,s5_one_candidate_control,s5_manifest
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,Code,Config,TransformTask,World,ORGANISM_ID,hidden_transform
};

fn run_case(seed:u64,program:Program)->World {
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    program.validate().unwrap();
    let h=program.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{h}"),
        expected_version:None,bytes:program.canonical_bytes(),
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:h.as_bytes().to_vec(),
    })).code,Code::Ok);
    for _ in 0..1200 {
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
            "unexpected world outcome: {:?} at tick {}",result.code,world.state.step);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {return world;}
    }
    panic!("S5 did not finish with fixed 12k A gas/epoch budget");
}
fn tool_op(world:&World)->Op {
    let h=String::from_utf8(
        world.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
    let raw=&world.state.objects[&format!("/self/versions/{h}")].bytes;
    let program:Program=serde_json::from_slice(raw).unwrap();
    assert_eq!(program.content_hash().unwrap(),h);
    assert_eq!(program.canonical_bytes(),*raw);
    program.functions[0].blocks[0].instructions[1].op.clone()
}
#[test]
fn same_initial_seed_program_finds_three_distinct_rules_and_reuses_tools(){
    let p=s5_sample_search();
    let program_hash=p.content_hash().unwrap();
    let man=s5_manifest();
    assert_eq!(man.program_hash,format!("sha256:{program_hash}"));
    assert!(man.known_protocol_families.is_empty());
    assert!(man.research_status.contains("NOT_NOVEL_ALGORITHM"));
    for seed in [3u64,4,5,7,8,9] {
        let world=run_case(seed,p.clone());
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(program_hash.clone()),"no researcher program swap allowed");
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)));
        let task:TransformTask=serde_json::from_slice(
            &world.state.objects["/public/E001/transform"].bytes).unwrap();
        assert_eq!(task.examples.len(),2);
        // The public object contains neither the hidden rule nor held-out answer.
        let public=String::from_utf8(
            world.state.objects["/public/E001/transform"].bytes.clone()).unwrap();
        assert!(!public.contains("rule"));
        assert!(!public.contains("held_out_output"));

        let expected_op=match seed%3 {
            0=>Op::Len(0),
            1=>Op::XorFold(0),
            _=>Op::ByteAt(0,1),
        };
        assert_eq!(tool_op(&world),expected_op);
        let evidence=&world.state.objects["/self/results/transform-held-out"].bytes;
        let exec_result:ActionResult=serde_json::from_slice(evidence).unwrap();
        assert_eq!(exec_result.code,Code::Ok);
        let result:Value=serde_json::from_slice(&exec_result.payload).unwrap();
        assert_eq!(result,
            Value::U32(hidden_transform(seed,&task.held_out_input)),
            "a held-out evaluation never used by candidate selection");
        assert_eq!(world.state.objects.keys()
            .filter(|key|key.starts_with("/self/versions/")).count(),
            (seed % 3) as usize + 2); // initial + zero-based position + selected
        assert!(world.audit.iter().any(|entry|
            matches!(&entry.action,Some(Action::Exec{..}))),
            "tool evaluation must happen by world EXEC");
        assert_eq!(World::from_snapshot(&world.snapshot_json())
            .unwrap().state_hash(),world.state_hash());
    }
}
#[test]
fn single_candidate_control_can_only_solve_len_rule() {
    for (seed,expected_success) in [(3u64,true),(4,false),(5,false)] {
        let world=run_case(seed,s5_one_candidate_control());
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(u32::from(expected_success))));
        assert_eq!(world.state.objects.contains_key("/self/tools/transform-v1"),
            expected_success);
        assert_eq!(world.state.objects.contains_key("/self/results/transform-held-out"),
            expected_success);
    }
}
