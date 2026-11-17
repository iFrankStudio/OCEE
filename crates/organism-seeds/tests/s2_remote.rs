//! S2 developmental chain: VM S1 discovers opcode -> researcher injects a
//! second *pre-designed* S2 program -> VM submits an actual remote task.
use ocee_organism_seeds::{p0_probe::s1_p0_probe,s2_p0::s2_p0_remote_eval};
use ocee_protocol_sim::{Decoded,Family,FrameKind,Operation};
use ocee_world_kernel::{Action,Code,Config,Message,WorkItem,World,NodeId,ORGANISM_ID};

fn install(world:&mut World,program:ocee_organism_vm::Program,old:Option<u64>)->String {
    program.validate().unwrap();
    let hash=program.content_hash().unwrap();
    assert_eq!(world.tick(Some(Action::Write{actor:ORGANISM_ID,
        key:format!("/self/versions/{hash}"),expected_version:None,
        bytes:program.canonical_bytes()})).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{actor:ORGANISM_ID,
        key:"/self/active".into(),expected_version:old,
        bytes:hash.as_bytes().to_vec()})).code,Code::Ok);
    hash
}
fn run_to_completion(world:&mut World,budget:usize) {
    let mut complete=false;
    for _ in 0..budget {
        let response=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(response.code,Code::InvalidCode|Code::Denied|Code::Gas),
            "unexpected code {:?} at step {}",response.code,world.state.step);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {
            complete=true;break;
        }
    }
    assert!(complete,"VM did not terminate within bounded development ticks");
}
#[test]
fn s2_executes_real_b_job_from_s1_discovery_not_hidden_opcode_injection(){
    for seed in [7u64,42] {
        let mut world=World::with_protocol(Config{
            seed,a_gas_per_epoch:100_000,..Config::default()
        },Family::P0,seed);
        let s1_hash=install(&mut world,s1_p0_probe(),None);
        run_to_completion(&mut world,800);
        let discovered=world.state.objects
            .get("/self/discoveries/p0-operation-candidate")
            .expect("S1 persisted a candidate")
            .bytes.clone();
        let expected_work:WorkItem=serde_json::from_slice(
            &world.state.objects["/public/E001/job0"].bytes).unwrap();
        assert_eq!(expected_work.count,3);
        assert_eq!(expected_work.values.len(),3);
        assert_eq!(expected_work.values_le.len(),12);
        // The orchestrator changes program version at a study boundary.
        // This transition is NOT counted as an organism-created tool.
        let s2_hash=install(&mut world,s2_p0_remote_eval(),Some(0));
        assert_ne!(s1_hash,s2_hash);
        run_to_completion(&mut world,32);
        assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
            Some(ocee_organism_vm::Value::U32(1)));
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(s2_hash));
        let recorded=&world.state.objects["/self/results/job0/ack"].bytes;
        let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let ack=proto.decode(recorded).expect("S2 stored the accepted ACK");
        assert_eq!(ack.kind,FrameKind::Response);
        assert_eq!(ack.operation,Operation::RemoteEval);
        assert_eq!(ack.body,vec![0]); // accepted, not yet a computed value
        let S1_decoded=proto.decode(&discovered).unwrap();
        assert_eq!(ack.request_id,S1_decoded.request_id);
        let accepted=world.state.service_events.iter()
            .filter(|e|e.b_gas_charged>0).collect::<Vec<_>>();
        assert_eq!(accepted.len(),1,"a real B job must be executed once");
        assert_eq!(accepted[0].b_gas_charged,20);

        // The RESEARCHER, not S2, independently checks B's final value
        // through the hidden reference encoder. This is deliberately separate.
        let result_poll=proto.encode(&Decoded{kind:FrameKind::Request,
            operation:Operation::Result,request_id:ack.request_id,body:vec![]}).unwrap();
        assert_eq!(world.tick(Some(Action::Send{actor:ORGANISM_ID,
            destination:NodeId::C,bytes:result_poll})).code,Code::Ok);
        for _ in 0..3 {world.tick(None);}
        let response=world.tick(Some(Action::Receive{actor:ORGANISM_ID,max_count:1}));
        assert_eq!(response.code,Code::Ok);
        let messages:Vec<Message>=serde_json::from_slice(&response.payload).unwrap();
        let decoded=proto.decode(&messages[0].bytes).unwrap();
        assert_eq!(decoded.operation,Operation::Result);
        assert_eq!(decoded.body[0],0);
        let actual=u32::from_le_bytes(decoded.body[1..5].try_into().unwrap());
        let expected=expected_work.values.iter().copied().enumerate()
            .fold(0u32,|acc,(i,v)|acc.wrapping_add(v.rotate_left(i as u32)));
        assert_eq!(actual,expected);
        assert_eq!(world.state.objects.keys()
            .filter(|k|k.starts_with("/self/versions/")).count(),2);
        let restored=World::from_snapshot(&world.snapshot_json()).unwrap();
        assert_eq!(world.state_hash(),restored.state_hash());
    }
}
