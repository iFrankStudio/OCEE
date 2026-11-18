//! S6: prove a bounded finite grammar can generate a multi-node executable
//! from public I/O examples. Not general AI program synthesis or E001 evidence.
use ocee_organism_seeds::s6_compose::{
    s6_composition_search,s6_single_operand_control,s6_manifest
};
use ocee_organism_vm::{Op,Program,Value,VmError,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,Code,Config,World,TransformTask,ORGANISM_ID,hidden_compose
};

fn run(seed:u64,program:Program)->World {
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },Family::P0,seed);
    let hash=program.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    for _ in 0..1600 {
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
            "error {:?} tick {}",result.code,world.state.step);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|v|v.finished) {return world;}
    }
    panic!("S6 finite search exceeded bounded run");
}
#[test]
fn single_seed_program_builds_two_input_compositions_with_held_out_validation() {
    let p=s6_composition_search();
    let hash=p.content_hash().unwrap();
    let manifest=s6_manifest();
    assert_eq!(manifest.program_hash,format!("sha256:{hash}"));
    assert!(manifest.research_status.contains("NOT_E001_M0"));
    // Exhaust the whole researcher-defined 3×3×2 grammar, not a selected subset.
    for seed in 0u64..18 {
        let world=run(seed,p.clone());
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(hash.clone())); // active program is never installed by researcher
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)),"seed {seed}");
        let task:TransformTask=serde_json::from_slice(
            &world.state.objects["/public/E001/compose"].bytes).unwrap();
        assert_eq!(task.examples.len(),2);
        let raw=&world.state.objects["/public/E001/compose"].bytes;
        assert!(!String::from_utf8_lossy(raw).contains("rule"));
        assert!(!String::from_utf8_lossy(raw).contains("held_out_output"));

        let selected=String::from_utf8(world.state.objects[
            "/self/tools/transform-v1"].bytes.clone()).unwrap();
        let code=&world.state.objects[&format!("/self/versions/{selected}")].bytes;
        let candidate:Program=serde_json::from_slice(code).unwrap();
        assert_eq!(candidate.content_hash().unwrap(),selected);
        let inst=&candidate.functions[0].blocks[0].instructions;
        assert!(matches!(&inst[1].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
        assert!(matches!(&inst[2].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
        assert!(matches!(&inst[3].op,Op::Add(2,3)|Op::Xor(2,3)));
        assert!(world.audit.iter().any(|event|matches!(
            &event.action,Some(Action::Write{key,..})
                if key=="/self/tools/transform-v1"
        )), "VM must register, not host");

        let feedback:ActionResult=serde_json::from_slice(
            &world.state.objects["/self/results/transform-held-out"].bytes).unwrap();
        assert_eq!(feedback.code,Code::Ok);
        let held_out:Value=serde_json::from_slice(&feedback.payload).unwrap();
        assert_eq!(held_out,
            Value::U32(hidden_compose(seed,&task.held_out_input)),
            "held-out result for seed {seed} not available to the agent");
        // Independent post-selection audit: never reveal these data to the VM.
        // Use distinct lengths, first bytes, and XOR reductions to detect
        // candidate expressions that merely fit the two public examples.
        let independent_inputs: [&[u8];4] = [
            &[1,2,3,4,5,6],
            &[99,0,13],
            &[0,0,0,0,0,1,2,3],
            &[200,7,41,17,9],
        ];
        for input in independent_inputs {
            let (actual,_gas)=run_isolated(&candidate,input.to_vec(),2000)
                .expect("selected tool must execute on novel inputs");
            assert_eq!(actual,Value::U32(hidden_compose(seed,input)),
                "independent input audit failed for seed {seed}, input {input:?}");
        }
        assert_eq!(world.state_hash(),World::from_snapshot(&world.snapshot_json())
            .unwrap().state_hash());
    }
}
#[test]
fn disabling_second_operand_prevents_solving_a_genuine_composition(){
    let world=run(1,s6_single_operand_control()); // XOR(input)+LEN(input)
    assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert!(!world.state.objects.contains_key("/self/tools/transform-v1"));
    assert!(!world.state.objects.contains_key("/self/results/transform-held-out"));
}


#[test]
fn empty_input_exposes_unhandled_boundary_in_s6_first_byte_grammar(){
    // Negative result, not a failing build: the published TransformTask has
    // no minimum-input-size contract, while ByteAt(0,0) faults on empty data.
    // The independent world reference currently defines first(empty)=0.
    let seed=2u64; // hidden task includes first-byte contribution
    let world=run(seed,s6_composition_search());
    let selected=String::from_utf8(
        world.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap();
    let candidate:Program=serde_json::from_slice(
        &world.state.objects[&format!("/self/versions/{selected}")].bytes).unwrap();
    assert_eq!(hidden_compose(seed,&[]),0);
    assert_eq!(run_isolated(&candidate,vec![],2000),
        Err(VmError::Range));
    // Report this as a counterexample to *full Bytes-domain* generality;
    // the six seeded positive tests only cover non-empty inputs.
}
