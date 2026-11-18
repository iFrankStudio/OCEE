//! S18 active VM P1 protocol discovery. Unlike S15-S17 reference solvers,
//! every candidate and decision is executed through the metered organism VM.
use ocee_organism_seeds::s18_p1_vm::{
    s18_p1_discover_and_invoke,s18_p1_no_diagnostic_control,
    s18_p1_no_result_shape_control,s18_manifest
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::{Family,Operation,FrameKind};
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn run_program(seed:u64,program:ocee_organism_vm::Program)->World {
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P1,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ]{
        let res=world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        }));
        assert_eq!(res.code,Code::Ok,"initial program installation");
    }
    let mut finished=false;
    for _ in 0..16_384 {
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
            "VM failure seed={seed} step={} code={:?}",
            world.state.step,result.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|continuation|continuation.finished){
            finished=true;
            break;
        }
    }
    assert!(finished,"S18 bounded VM must finish");
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    world
}

fn run(seed:u64,diagnostics:bool)->World {
    run_program(seed,if diagnostics{s18_p1_discover_and_invoke()}
        else{s18_p1_no_diagnostic_control()})
}

#[test]
fn result_shape_ablation_preserves_b_execution_but_blocks_result_storage(){
    let w=run_program(42,s18_p1_no_result_shape_control());
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(w.state.objects.contains_key("/self/discoveries/p1-operation-candidate"));
    assert!(w.state.objects.contains_key("/self/results/p1-remote-eval-ack"));
    assert!(!w.state.objects.contains_key("/self/results/p1-job0/value_le"));
    assert_eq!(w.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>(),20);
}

#[test]
fn active_vm_learns_p1_remote_opcode_then_submits_actual_b_job(){
    assert_eq!(s18_manifest().program_hash,
        format!("sha256:{}",s18_p1_discover_and_invoke().content_hash().unwrap()));
    for seed in [7u64,42,99] {
        let w=run(seed,true);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)),
            "active S18 VM did not complete seed={seed}");
        let private_service=w.state.protocol_service.as_ref().unwrap();
        let learned=&w.state.objects["/self/discoveries/p1-operation-candidate"].bytes;
        let decoded=private_service.protocol.decode(learned).unwrap();
        assert_eq!(decoded.kind,FrameKind::Request);
        assert_eq!(decoded.operation,Operation::RemoteEval);
        assert!(decoded.body.is_empty(),"discovery probe should be a malformed body");
        let ack=&w.state.objects["/self/results/p1-remote-eval-ack"].bytes;
        let decoded_ack=private_service.protocol.decode(ack).unwrap();
        assert_eq!(decoded_ack.kind,FrameKind::Response);
        assert_eq!(decoded_ack.operation,Operation::RemoteEval);
        assert_eq!(decoded_ack.body,vec![0]);
        let result=&w.state.objects["/self/results/p1-job0/value_le"].bytes;
        assert_eq!(result.len(),4);
        let value=u32::from_le_bytes(result.as_slice().try_into().unwrap());
        let work:WorkItem=serde_json::from_slice(
            &w.state.objects["/public/E001/job0"].bytes
        ).unwrap();
        let reference=work.values.iter().enumerate().fold(0u32,|acc,(i,&v)|
            acc.wrapping_add(v.rotate_left((i%32) as u32)));
        assert_eq!(value,reference,"result must be actual B-computed u32");
        assert!(w.audit.iter().any(|item|matches!(
            &item.action,Some(Action::Send{..})
        )),"real protocol messages must exist");
        assert_eq!(w.state.service_events.iter()
            .map(|event|event.b_gas_charged).sum::<u64>(),20,
            "only actual C->B FOLD32 invocation may charge B once");
        let vm_cost:u64=w.audit.iter().map(|event|event.result.vm_gas_charged).sum();
        let a_cost:u64=w.audit.iter().map(|event|
            event.result.vm_gas_charged+event.result.gas_charged).sum();
        let sends=w.audit.iter().filter(|event|matches!(
            event.action,Some(Action::Send{..})
        )).count();
        assert!(sends>1 && sends<=225);
        assert!(vm_cost>0,"VM decision work cannot be free host side search");
        println!("S18 active P1 seed={seed} scans={sends} ticks={} VM_gas={vm_cost} A_gas={a_cost} B_gas=20",
            w.state.step);
    }
}
#[test]
fn disabling_vm_diagnostic_selection_prevents_b_invocation(){
    let w=run(42,false);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(!w.state.objects.contains_key("/self/discoveries/p1-operation-candidate"));
    assert!(!w.state.objects.contains_key("/self/results/p1-remote-eval-ack"));
    assert!(!w.state.objects.contains_key("/self/results/p1-job0/value_le"));
    assert_eq!(w.state.service_events.iter().map(|e|e.b_gas_charged).sum::<u64>(),0);
    assert!(w.audit.iter().filter(|event|
        matches!(event.action,Some(Action::Send{..}))).count()>0);
}
