//! S21B: only public HELLO observations enter the active VM.
//! The private Protocol decoder is a research-side test oracle AFTER C
//! processed the newly constructed request; never a solver input.
use ocee_organism_seeds::s21_p2_tlv::{
    s21_p2_infer_and_validate_new_hello,
    s21_p2_new_hello_no_live_check_ablation,s21b_manifest
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::{Family,FrameKind,Operation};
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};

fn run(seed:u64,ablate:bool)->World{
    let program=if ablate{s21_p2_new_hello_no_live_check_ablation()}
        else{s21_p2_infer_and_validate_new_hello()};
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".to_owned(),hash.into_bytes()),
    ]{
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    for _ in 0..512{
        let action=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(action.code,Code::InvalidCode|Code::Denied),
            "seed={seed} VM fault at {}: {:?}",world.state.step,action.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    world
}

#[test]
fn active_p2_vm_constructs_nonreplayed_hello_and_validates_real_c_reply(){
    assert_eq!(s21b_manifest().program_hash,
        format!("sha256:{}",s21_p2_infer_and_validate_new_hello()
            .content_hash().unwrap()));
    for seed in [7u64,42,99,272] {
        let w=run(seed,false);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
        assert!(w.state.objects.contains_key("/self/discoveries/p2-tlv-roles"));
        let data=&w.state.objects["/self/observations/p2-hello-nonreplay"].bytes;
        let private_oracle=w.state.protocol_service.as_ref().unwrap();
        let decoded=private_oracle.protocol.decode(data).unwrap();
        assert_eq!(decoded.kind,FrameKind::Response);
        assert_eq!(decoded.operation,Operation::Hello);
        assert_eq!(decoded.request_id,73);
        assert_eq!(decoded.body,vec![0,1]);
        let sends=w.audit.iter().filter_map(|record|match &record.action{
            Some(Action::Send{bytes,..})=>Some(bytes.clone()),
            _=>None,
        }).collect::<Vec<_>>();
        assert_eq!(sends.len(),1,"a newly formed HELLO, no opcode scan");
        let request=private_oracle.protocol.decode(&sends[0]).unwrap();
        assert_eq!(request.kind,FrameKind::Request);
        assert_eq!(request.operation,Operation::Hello);
        assert_eq!(request.request_id,73);
        assert!(request.body.is_empty());
        let public_hello:ocee_world_kernel::HelloSample=serde_json::from_slice(
            &w.state.objects["/public/C/hello"].bytes).unwrap();
        assert_ne!(sends[0],public_hello.request,
            "must create a new request, not merely replay HELLO");
        let vm:u64=w.audit.iter().map(|event|event.result.vm_gas_charged).sum();
        let total:u64=w.audit.iter().map(|event|
            event.result.vm_gas_charged+event.result.gas_charged).sum();
        assert!(vm>0 && total>=vm);
        assert_eq!(w.state.service_events.iter()
            .map(|record|record.b_gas_charged).sum::<u64>(),0);
        println!("S21B P2 seed={seed} VM_gas={vm} A_gas={total} ticks={} SEND=1 B_gas=0",
            w.state.step);
    }
}
#[test]
fn valid_c_reply_but_wrong_organism_verifier_must_not_claim_success(){
    let w=run(42,true);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(0)));
    assert!(w.state.objects.contains_key("/self/discoveries/p2-tlv-roles"));
    assert!(!w.state.objects.contains_key("/self/observations/p2-hello-nonreplay"));
    let messages=w.audit.iter().filter(|record|matches!(
        record.action,Some(Action::Send{..})
    )).count();
    assert_eq!(messages,1);
    assert_eq!(w.state.service_events.iter()
        .map(|record|record.b_gas_charged).sum::<u64>(),0);
}
