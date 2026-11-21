//! S44 native causal controls for an active P2 VM.
#![forbid(unsafe_code)]
use ocee_organism_seeds::{
    s44_outcome_wait::{s44_p2_causal_wait_consumer,s44_outcome_aware_root},
    s40_dropped_recovery::s40_p1_consumer,
    s43b_active_receipts::REQUEST_KEY_PREFIX,
    s33_adaptive_depth::DEPTH_TASK,
    s32_composite_b_operand::REMOTE_VALUES,
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::{Family,Operation,fold32};
use ocee_world_kernel::{Action,Code,Config,NodeId,Object,TransformExample,
    TransformTask,World,RequestOutcome,RequestState,ORGANISM_ID,hidden_compose};

fn rule(seed:u64,input:&[u8])->u32{
    hidden_compose(seed,input).wrapping_add(
        ((seed%54)/18) as u32*input.len() as u32)
}
fn boot(delay:u64)->World{
    let seed=55;
    let p1=s40_p1_consumer();
    let p2=s44_p2_causal_wait_consumer();
    let root=s44_outcome_aware_root();
    let mut w=World::with_protocol(Config{
        seed,message_delay:delay,a_gas_per_epoch:100_000,..Config::default()
    },Family::P2,42);
    let a=vec![1];let b=vec![2,0];
    let task=TransformTask{
        task_id:"s36-interactive-probe-v1".into(),
        examples:vec![
            TransformExample{input:a.clone(),output:rule(seed,&a)},
            TransformExample{input:b.clone(),output:rule(seed,&b)},
        ],
        held_out_input:vec![4,5,6,7,8],
    };
    w.state.objects.insert(DEPTH_TASK.into(),Object{
        owner:0,version:0,bytes:serde_json::to_vec(&task).unwrap()
    });
    for program in [p1,p2,root.clone()]{
        let hash=program.content_hash().unwrap();
        let result=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:format!("/self/versions/{hash}"),
            bytes:program.canonical_bytes(),expected_version:None
        }));
        assert_eq!(result.code,Code::Ok);
    }
    let set=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        bytes:root.content_hash().unwrap().into_bytes(),
        expected_version:None
    }));
    assert_eq!(set.code,Code::Ok);
    w
}
fn b_total(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn outcomes(w:&World)->Vec<RequestOutcome>{
    w.audit.iter().filter_map(|record|{
        if !matches!(&record.action,Some(Action::RequestOutcomeFor{..})){
            return None;
        }
        assert_eq!(record.result.code,Code::Ok);
        assert_eq!(record.result.gas_charged,2);
        Some(serde_json::from_slice::<RequestOutcome>(
            &record.result.payload).unwrap())
    }).collect()
}
fn finish(w:&mut World){
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "VM trap step {} code {:?}, block {:?}",
            w.state.step,r.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation
                .as_ref().map(|c|c.block.as_str()));
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation
        .as_ref().is_some_and(|c|c.finished));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn actual_job_id(w:&mut World)->u64{
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        let Some(last)=w.audit.last() else{continue};
        let Some(Action::Send{bytes,destination:NodeId::C,..})=&last.action
            else{continue};
        if last.result.code!=Code::Ok{continue;}
        let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
        let Ok(frame)=decoder.decode(bytes)else{continue};
        if frame.operation==Operation::RemoteEval &&
            frame.body.len()==18 &&
            u16::from_le_bytes(frame.body[..2].try_into().unwrap())==4 {
            assert_eq!(last.result.payload.len(),8);
            return u64::from_le_bytes(last.result.payload[..8].try_into().unwrap());
        }
    }
    panic!("active P2 VM did not send actual REMOTE_EVAL");
}
#[test]
fn s44_real_commit_and_inflight_are_wait_not_resend(){
    let mut w=boot(2);
    let request_id=actual_job_id(&mut w);
    // Delay only the real B request and its ACK. A global delay of six
    // also exhausts the legacy S23 RESULT opcode-scan retry budget;
    // that is a separate bounded-liveness limitation, not S44's gate.
    let queued=w.state.pending_deliveries.iter_mut()
        .find(|m|m.id==request_id && m.destination==NodeId::C)
        .expect("actual REMOTE_EVAL must still be in transit");
    queued.deliver_step+=8;
    w.state.config.message_delay=8;
    let mut confirmed_ack=false;
    for _ in 0..1_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        if w.audit.last().is_some_and(|a|
            a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::ConfirmResponseFor{
                request_id:id,..}) if *id==request_id)){
            confirmed_ack=true;break;
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    assert!(confirmed_ack,
        "actual delayed REMOTE_EVAL ACK must pass S43B confirmation: {:?}",
        outcomes(&w));
    // Preserve the original bounded Phase2 RESULT discovery conditions.
    w.state.config.message_delay=2;
    finish(&mut w);
    let states=outcomes(&w);
    let tail=w.audit.iter().rev().take(24)
        .map(|a|format!("{}:{:?}:{:?}",
            a.step,a.action.as_ref().map(|action|match action{
                Action::Read{key,..}=>format!("Read:{key}"),
                Action::Write{key,..}=>format!("Write:{key}"),
                Action::Send{..}=>"Send".into(),
                Action::Receive{..}=>"Receive".into(),
                Action::RequestOutcomeFor{request_id,..}=>
                    format!("Outcome:{request_id}"),
                Action::ConfirmResponseFor{request_id,..}=>
                    format!("Confirm:{request_id}"),
                _=>"Other".into(),
            }),a.result.code)).collect::<Vec<_>>();
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),
        "scoped ACK delay diagnostic: b={} states={:?} restarts={:?} tail={:?}",
        b_total(&w),states,w.state.guarded_restarts,tail);
    assert_eq!(b_total(&w),24);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(!states.is_empty(),"P2 must query its own live request");
    assert!(states.iter().any(|s|s.state==RequestState::InFlight));
    assert!(states.iter().any(|s|s.state==RequestState::CommittedB),
        "post-execution unreceived response must not trigger retry");
    assert!(!states.iter().any(|s|matches!(
        s.state,RequestState::Dropped|RequestState::Unknown)));
    assert_eq!(w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::RestartRootFor{..}))).count(),0);
    let actual=u32::from_le_bytes(w.state.objects[
        "/self/results/p2-job0/value_le"].bytes[..4].try_into().unwrap());
    let values=w.state.objects[REMOTE_VALUES].bytes.chunks_exact(4)
        .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(actual,fold32(&values).unwrap());
}
#[test]
fn s44_only_true_drop_authorizes_relearning(){
    let mut w=boot(2);
    let old_id=actual_job_id(&mut w);
    let task=w.state.objects.get_mut(DEPTH_TASK).unwrap();
    let mut changed:TransformTask=serde_json::from_slice(&task.bytes).unwrap();
    changed.task_id="s37-changed-rule-v2".into();
    for ex in &mut changed.examples{ex.output=2*ex.input.len() as u32;}
    task.bytes=serde_json::to_vec(&changed).unwrap();task.version+=1;
    finish(&mut w);
    assert_eq!(w.state.fence_drops.len(),1);
    assert_eq!(w.state.fence_drops[0].message_id,old_id);
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    assert_eq!(b_total(&w),24);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    let all=outcomes(&w);
    assert!(all.iter().any(|s|s.request_id==old_id &&
        s.state==RequestState::Dropped));
    let root=w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::RestartRootFor{request_id,..}) if *request_id==old_id))
        .collect::<Vec<_>>();
    assert_eq!(root.len(),1);
    assert_eq!(root[0].result.code,Code::Ok);
    assert_eq!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count(),1);
}
#[test]
fn s44_unknown_receipt_fails_closed_without_blind_replay(){
    let mut w=boot(2);
    let actual=actual_job_id(&mut w);
    // Delay only this remote request to guarantee an Empty RECEIVE.
    w.state.pending_deliveries.iter_mut()
        .find(|m|m.id==actual && m.destination==NodeId::C)
        .expect("actual request queued").deliver_step+=8;
    // Advance only until the active VM has persisted the precise receipt.
    let mut key=None;
    for _ in 0..128{
        let existing=w.state.objects.keys()
            .find(|candidate|candidate.starts_with(REQUEST_KEY_PREFIX) &&
                candidate.contains("01")).cloned();
        if let Some(existing)=existing{key=Some(existing);break;}
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        assert!(!w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished));
    }
    let key=key.expect("bounded VM did not persist phase1 receipt");
    assert_eq!(w.state.objects[&key].bytes,actual.to_le_bytes());
    // Privileged fault injection: destroy the organism's saved evidence,
    // NOT the kernel's own SEND record or B service event.
    w.state.objects.get_mut(&key).unwrap().bytes=u64::MAX.to_le_bytes().to_vec();
    finish(&mut w);
    let states=outcomes(&w);
    assert!(states.iter().any(|s|s.request_id==u64::MAX &&
        s.state==RequestState::Unknown));
    assert!(w.state.guarded_restarts.is_empty());
    assert!(!w.audit.iter().any(|a|matches!(&a.action,
        Some(Action::RestartRootFor{..}))));
    assert!(!w.state.objects.contains_key(
        "/self/results/p2-job0/value_le"));
    assert_ne!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
}
