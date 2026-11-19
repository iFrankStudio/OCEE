//! S22 development trials: private protocol read access only inside the
//! research evaluator, never in the active VM or its work search.
use ocee_organism_seeds::s22_p2_remote::{
    s22_p2_discover_and_execute,s22_p2_no_badbody_selection,
    s22_p2_no_result_shape,s22_manifest,
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::{Family,FrameKind,Operation};
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn run(seed:u64,mode:u8)->World {
    let program=match mode{
        0=>s22_p2_discover_and_execute(),
        1=>s22_p2_no_badbody_selection(),
        2=>s22_p2_no_result_shape(),
        _=>panic!("unknown test mode")
    };
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".into(),hash.into_bytes()),
    ] {
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    for _ in 0..16_384{
        let reply=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(reply.code,Code::InvalidCode|Code::Denied),
            "seed={seed} mode={mode} tick={} code={:?}",
            world.state.step,reply.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished),"S22 bounded VM must terminate");
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    world
}

fn b_gas(world:&World)->u64 {
    world.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
#[test]
fn active_vm_p2_learns_both_hidden_operations_and_executes_real_b_fold32() {
    assert_eq!(s22_manifest().program_hash,
        format!("sha256:{}",s22_p2_discover_and_execute().content_hash().unwrap()));
    for seed in [7u64,42,99] {
        let w=run(seed,0);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)),"seed={seed}");
        assert_eq!(b_gas(&w),20);
        let roles=&w.state.objects["/self/discoveries/p2-tlv-roles"].bytes;
        assert_eq!(roles.len(),11);
        let op=&w.state.objects["/self/discoveries/p2-remote-opcode"].bytes;
        let result_op=&w.state.objects["/self/discoveries/p2-result-opcode"].bytes;
        assert_eq!(op.len(),1);
        assert_eq!(result_op.len(),1);
        assert_ne!(op,result_op);
        let service=w.state.protocol_service.as_ref().unwrap();
        let sends=w.audit.iter().filter_map(|e|match &e.action{
            Some(Action::Send{bytes,..})=>Some(bytes.as_slice()),_=>None,
        }).collect::<Vec<_>>();
        let valid_eval=sends.iter().filter_map(|wire|service.protocol.decode(wire).ok())
            .filter(|decoded|decoded.operation==Operation::RemoteEval &&
                decoded.kind==FrameKind::Request && decoded.body.len()==14)
            .collect::<Vec<_>>();
        assert_eq!(valid_eval.len(),1,"exactly one remote job via C");
        assert_eq!(valid_eval[0].request_id,17);
        assert_eq!(valid_eval[0].body[..2],[3,0]);
        let work:WorkItem=serde_json::from_slice(
            &w.state.objects["/public/E001/job0"].bytes).unwrap();
        assert_eq!(valid_eval[0].body[2..],work.values_le);
        let expected=work.values.iter().enumerate().fold(0u32,|acc,(i,&v)|
            acc.wrapping_add(v.rotate_left((i%32) as u32)));
        let raw=&w.state.objects["/self/results/p2-job0/value_le"].bytes;
        assert_eq!(raw.len(),4);
        let actual=u32::from_le_bytes(raw.as_slice().try_into().unwrap());
        assert_eq!(actual,expected,"must compare real returned result, not guess");
        let charged_a:u64=w.audit.iter()
            .map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum();
        let charged_vm:u64=w.audit.iter().map(|e|e.result.vm_gas_charged).sum();
        assert!(charged_vm>0 && charged_a>=charged_vm);
        println!("S22 full P2 seed={seed}: ticks={} sends={} A_gas={charged_a} VM_gas={charged_vm} B_gas=20 remote_op={} result_op={}",
            w.state.step,sends.len(),op[0],result_op[0]);
    }
}
#[test]
fn blinding_badbody_selection_prevents_p2_remote_b_execution() {
    let w=run(42,1);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert_eq!(b_gas(&w),0);
    assert!(!w.state.objects.contains_key("/self/discoveries/p2-remote-opcode"));
    assert!(!w.state.objects.contains_key("/self/results/p2-job0/value_le"));
    assert!(w.state.objects.contains_key("/self/discoveries/p2-tlv-roles"));
}
#[test]
fn wrong_result_length_must_not_store_a_false_b_answer() {
    let w=run(42,2);
    assert_eq!(b_gas(&w),20,"B can do real work but VM must reject result");
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(!w.state.objects.contains_key("/self/results/p2-job0/value_le"));
    assert!(w.state.objects.contains_key("/self/results/p2-remote-eval-ack"));
}
