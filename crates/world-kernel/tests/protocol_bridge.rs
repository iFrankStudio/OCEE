//! Researcher-generated reference messages only; no protocol knowledge is seeded to an organism.
use ocee_protocol_sim::{Decoded, Family, FrameKind, Operation, ParseError, ServiceOutput};
use ocee_world_kernel::{Action, Code, Config, NodeId, TransportDiagnostic, World, ORGANISM_ID};

fn send(world:&mut World,frame:Vec<u8>) {
    assert_eq!(world.tick(Some(Action::Send{
        actor:ORGANISM_ID,destination:NodeId::C,bytes:frame,
    })).code,Code::Ok);
}
fn receive(world:&mut World)->Vec<u8>{
    let response=world.tick(Some(Action::Receive{actor:ORGANISM_ID,max_count:1}));
    assert_eq!(response.code,Code::Ok);
    let messages:Vec<ocee_world_kernel::Message>=serde_json::from_slice(&response.payload).unwrap();
    assert_eq!(messages.len(),1);
    assert_eq!(messages[0].source,NodeId::C);
    messages[0].bytes.clone()
}

#[test]
fn all_protocol_families_use_b_without_leaking_or_bypassing_world() {
    for family in [Family::P0,Family::P1,Family::P2] {
        let mut world=World::with_protocol(Config::default(),family,2026);
        // Only this external test driver knows hidden reference-frame structure.
        let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let mut body=3u16.to_le_bytes().to_vec();
        for value in [1u32,2,3] {body.extend(value.to_le_bytes());}
        let eval=proto.encode(&Decoded{
            kind:FrameKind::Request,operation:Operation::RemoteEval,
            request_id:17,body,
        }).unwrap();
        assert_eq!(world.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::B,bytes:eval.clone(),
        })).code,Code::Denied);
        assert_eq!(world.state.service_events.len(),0);

        send(&mut world,eval); // k=1, reaches C at k=3
        world.tick(None); // k=2
        world.tick(None); // k=3: service consumes request and spends B gas
        assert_eq!(world.state.service_events.len(),1);
        assert_eq!(world.state.service_events[0].b_gas_charged,20);
        assert_eq!(world.state.mailboxes.get(&NodeId::A).map_or(0,Vec::len),0);

        world.tick(None); // k=4
        let reply=receive(&mut world); // k=5
        let ack=proto.decode(&reply).unwrap();
        assert_eq!(ack.operation,Operation::RemoteEval);
        assert_eq!(ack.request_id,17);
        assert_eq!(ack.body,vec![0]);

        let poll=proto.encode(&Decoded{
            kind:FrameKind::Request,operation:Operation::Result,
            request_id:17,body:vec![],
        }).unwrap();
        send(&mut world,poll); // k=6
        world.tick(None); // k=7
        world.tick(None); // k=8: request processed
        assert_eq!(world.state.service_events.len(),2);
        assert_eq!(world.state.service_events[1].b_gas_charged,0);
        let saved=world.snapshot_json();
        let mut restored=World::from_snapshot(&saved).unwrap();
        world.tick(None); // k=9
        restored.tick(None);
        let response=receive(&mut world); // k=10
        let response_from_copy=receive(&mut restored);
        assert_eq!(response,response_from_copy);
        assert_eq!(world.state_hash(),restored.state_hash());
        assert_eq!(world.audit,restored.audit);
        let parsed=proto.decode(&response).unwrap();
        assert_eq!(parsed.operation,Operation::Result);
        assert_eq!(parsed.body[0],0);
        let result=u32::from_le_bytes(parsed.body[1..5].try_into().unwrap());
        let independent_result=1u32.wrapping_add(2u32.rotate_left(1))
            .wrapping_add(3u32.rotate_left(2));
        assert_eq!(result,independent_result);
        assert_eq!(world.state.nodes[&NodeId::B].gas_available,50_000-20);
    }
}

#[test]
fn malformed_c_messages_get_bounded_diagnostics_without_b_execution() {
    let mut world=World::with_protocol(Config::default(),Family::P2,99);
    send(&mut world,vec![0xff,0x00,0x00]); // not a P2 marker
    world.tick(None);
    world.tick(None);
    assert_eq!(world.state.service_events.len(),1);
    assert_eq!(world.state.service_events[0].b_gas_charged,0);
    assert_eq!(world.state.service_events[0].diagnostic,Some(ParseError::BadFrame));
    world.tick(None);
    let body=receive(&mut world);
    let diag:TransportDiagnostic=serde_json::from_slice(&body).unwrap();
    assert_eq!(diag.code,ParseError::BadFrame);
    assert_eq!(world.state.nodes[&NodeId::B].gas_available,50_000);
}


#[test]
fn public_hello_example_is_discoverable_and_does_not_expose_private_service_state() {
    let mut world=World::with_protocol(Config::default(),Family::P1,2026);
    let first=world.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/public/C/hello".into(),
    }));
    assert_eq!(first.code,Code::Ok);
    let sample:ocee_world_kernel::HelloSample=serde_json::from_slice(&first.payload).unwrap();
    assert_eq!(sample.published_step,0);
    let protocol=world.state.protocol_service.as_ref().unwrap().protocol.clone();
    let request=protocol.decode(&sample.request).unwrap();
    let response=protocol.decode(&sample.response).unwrap();
    assert_eq!(request.operation,Operation::Hello);
    assert_eq!(response.operation,Operation::Hello);
    assert_eq!(request.request_id,response.request_id);
    assert_eq!(response.body[0],0);

    // Hidden service definition is not addressable with world READ.
    assert_ne!(world.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/world/protocol_service".into(),
    })).code,Code::Ok);
    let mut world_copy=World::from_snapshot(&world.snapshot_json()).unwrap();
    for _ in 0..254 {world.tick(None);world_copy.tick(None);}
    assert_eq!(world.state_hash(),world_copy.state_hash());
    assert_eq!(world.state.step,256);
    let updated=world.tick(Some(Action::Read{
        actor:ORGANISM_ID,key:"/public/C/hello".into(),
    }));
    assert_eq!(updated.code,Code::Ok);
    let sample2:ocee_world_kernel::HelloSample=serde_json::from_slice(&updated.payload).unwrap();
    assert_eq!(sample2.published_step,256);
    assert_ne!(protocol.decode(&sample.request).unwrap().request_id,
        protocol.decode(&sample2.request).unwrap().request_id);
}
