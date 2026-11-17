//! S2 developmental chain: VM S1 discovers opcode -> researcher injects a
//! second *pre-designed* S2 program -> VM submits an actual remote task.
use ocee_organism_seeds::{p0_probe::s1_p0_probe,s2_p0::s2_p0_remote_eval};
use ocee_protocol_sim::{Family,FrameKind,Operation};
use ocee_world_kernel::{Action,Code,Config,WorkItem,World,ORGANISM_ID};

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
        run_to_completion(&mut world,800);
        if world.state.organisms[&ORGANISM_ID].last_return!=Some(ocee_organism_vm::Value::U32(1)){
            let p=world.state.protocol_service.as_ref().unwrap().protocol.clone();
            let sent_results=world.audit.iter().filter_map(|a|{
                if let Some(Action::Send{bytes,..})=&a.action {
                    p.decode(bytes).ok().map(|f|f.operation)
                }else{None}
            }).filter(|op|*op==Operation::Result).count();
            let received=world.audit.iter().filter_map(|a|{
                if let Some(Action::Receive{..})=&a.action {
                    if a.result.code!=Code::Ok{return None;}
                    let messages:Vec<ocee_world_kernel::Message>=
                        serde_json::from_slice(&a.result.payload).ok()?;
                    let msg=messages.first()?;
                    Some((p.decode(&msg.bytes).ok().map(|d|(d.operation,d.body)),
                        msg.bytes.len()))
                }else{None}
            }).collect::<Vec<_>>();
            let result_sends=world.audit.iter().filter_map(|a|{
                if let Some(Action::Send{bytes,..})=&a.action {
                    if p.decode(bytes).ok().is_some_and(|d|d.operation==Operation::Result) {
                        return Some(a.step);
                    }
                }
                None
            }).collect::<Vec<_>>();
            let relevant=world.audit.iter().filter(|a|result_sends.iter()
                .any(|s|a.step==*s || a.step==s+4))
                .map(|a|{
                    let sent=if let Some(Action::Send{bytes,..})=&a.action {
                        p.decode(bytes).ok().map(|f|(f.kind,f.operation,f.request_id,f.body))
                    }else{None};
                    let reply=if let Some(Action::Receive{..})=&a.action {
                        if a.result.code==Code::Ok {
                            serde_json::from_slice::<Vec<ocee_world_kernel::Message>>(
                                &a.result.payload).ok().and_then(|ms|{
                                ms.first().map(|m|(m.id,m.sent_step,m.bytes.len(),
                                    p.decode(&m.bytes).ok().map(|d|(d.kind,d.operation,d.request_id,d.body)),
                                    String::from_utf8_lossy(&m.bytes).to_string()))
                            })
                        }else{None}
                    }else{None};
                    (a.step,sent,reply)
                }).collect::<Vec<_>>();
            let relevant_events=world.state.service_events.iter().filter(|e|
                result_sends.iter().any(|s|e.step==s+2)).collect::<Vec<_>>();
            let window=world.audit.iter().filter(|a|result_sends.iter()
                .any(|s|a.step>=*s && a.step<=s+8))
                .map(|a|format!("step={} action={:?} code={:?} payload_len={}",
                    a.step,a.action.as_ref().map(|x|match x{
                        Action::Send{..}=>"Send",Action::Receive{..}=>"Receive",
                        Action::Observe{..}=>"Observe",Action::Write{..}=>"Write",
                        _=>"Other"
                    }),a.result.code,a.result.payload.len()))
                .collect::<Vec<_>>();
            panic!("S2 returned {:?}; result queries={sent_results}, query steps={result_sends:?}, relevant={relevant:?}, result_events={relevant_events:?}, B events={:?}, valid replies={:?}, window={window:?}",
                world.state.organisms[&ORGANISM_ID].last_return,
                world.state.service_events.iter().rev().take(5).collect::<Vec<_>>(),
                received.iter().filter(|x|x.0.is_some()).collect::<Vec<_>>());
        }
        assert_eq!(world.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(s2_hash));
        let recorded=&world.state.objects["/self/results/job0/ack"].bytes;
        let proto=world.state.protocol_service.as_ref().unwrap().protocol.clone();
        let ack=proto.decode(recorded).expect("S2 stored the accepted ACK");
        assert_eq!(ack.kind,FrameKind::Response);
        assert_eq!(ack.operation,Operation::RemoteEval);
        assert_eq!(ack.body,vec![0]); // accepted, not yet a computed value
        let s1_decoded=proto.decode(&discovered).unwrap();
        assert_eq!(ack.request_id,s1_decoded.request_id);
        let accepted=world.state.service_events.iter()
            .filter(|e|e.b_gas_charged>0).collect::<Vec<_>>();
        assert_eq!(accepted.len(),1,"a real B job must be executed once");
        assert_eq!(accepted[0].b_gas_charged,20);

        // S2, not the test driver, probes RESULT, receives and persists its value.
        let actual_bytes=&world.state.objects["/self/results/job0/value_le"].bytes;
        assert_eq!(actual_bytes.len(),4);
        let actual=u32::from_le_bytes(actual_bytes.as_slice().try_into().unwrap());
        // Separate arithmetic evaluation; not provided to the organism.
        let expected=expected_work.values.iter().copied().enumerate()
            .fold(0u32,|acc,(i,v)|acc.wrapping_add(v.rotate_left(i as u32)));
        assert_eq!(actual,expected);
        let queried=world.audit.iter().filter_map(|r|{
            if let Some(Action::Send{bytes,..})=&r.action {
                proto.decode(bytes).ok()
            }else{None}
        }).any(|f|f.operation==Operation::Result);
        assert!(queried,"S2 must query RESULT via normal world SEND");
        assert!(world.audit.iter().any(|r|
            matches!(&r.action,Some(Action::Write{key,..})
                if key=="/self/results/job0/value_le")),
            "S2 must persist the computed result through world WRITE");
        assert_eq!(world.state.objects.keys()
            .filter(|k|k.starts_with("/self/versions/")).count(),2);
        let restored=World::from_snapshot(&world.snapshot_json()).unwrap();
        assert_eq!(world.state_hash(),restored.state_hash());
    }
}
