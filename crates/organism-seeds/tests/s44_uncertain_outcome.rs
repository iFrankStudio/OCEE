//! S44 native causal controls for an active P2 VM.
#![forbid(unsafe_code)]
use ocee_organism_seeds::{
    s44_outcome_wait::{s44_p2_causal_wait_consumer,s44_outcome_aware_root},
    s45_scoped_result_wait::{s45_p2_scoped_result_wait,s45_scoped_result_root},
    s40_dropped_recovery::s40_p1_consumer,
    s43b_active_receipts::REQUEST_KEY_PREFIX,
    s33_adaptive_depth::DEPTH_TASK,
    s32_composite_b_operand::REMOTE_VALUES,
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::{Family,Operation,fold32};
use ocee_world_kernel::{Action,Code,Config,NodeId,Object,TransformExample,
    TransformTask,World,RequestOutcome,RequestState,ORGANISM_ID,hidden_compose};

fn rule(seed:u64,input:&[u8])->u32{
    hidden_compose(seed,input).wrapping_add(
        ((seed%54)/18) as u32*input.len() as u32)
}
fn boot(delay:u64)->World{
    boot_with(delay,s44_p2_causal_wait_consumer(),s44_outcome_aware_root())
}
fn boot_s45(delay:u64)->World{
    boot_with(delay,s45_p2_scoped_result_wait(),s45_scoped_result_root())
}
fn boot_with(delay:u64,p2:Program,root:Program)->World{
    boot_with_protocol_seed(delay,42,p2,root)
}
fn boot_with_protocol_seed(delay:u64,protocol_seed:u64,p2:Program,root:Program)->World{
    boot_with_protocol_seed_budget(delay,protocol_seed,100_000,p2,root)
}
fn boot_with_protocol_seed_budget(delay:u64,protocol_seed:u64,
    a_gas_per_epoch:u64,p2:Program,root:Program)->World{
    let seed=55;
    let p1=s40_p1_consumer();
    let mut w=World::with_protocol(Config{
        seed,message_delay:delay,a_gas_per_epoch,..Config::default()
    },Family::P2,protocol_seed);
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


// Regression of the earlier genuinely red S44 global C delay=6 case.
// This entire test runs at delay 6: no resetting world delay after ACK.
#[test]
fn s45_global_six_step_delay_validates_real_b_and_result_once(){
    let mut w=boot_s45(6);
    finish(&mut w);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),
        "global6: b={} outcomes={:?} counters={:?} tail={:?}",
        b_total(&w),outcomes(&w),
        w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().map(|c|&c.loop_counters),
        w.audit.iter().rev().take(8)
            .map(|a|format!("{:?}:{:?}",a.action,a.result.code))
            .collect::<Vec<_>>());
    assert_eq!(b_total(&w),24);
    assert!(w.state.guarded_restarts.is_empty());
    let cont=w.state.organisms[&ORGANISM_ID].vm_continuation
        .as_ref().unwrap();
    let global=cont.loop_counters.get("s45_result_retry").copied().unwrap_or(0);
    assert!(global>0 && global<=896,"global scoped wait must be used");
    assert!(cont.loop_counters.keys()
        .filter(|k|k.starts_with("s45_result_retry:")).count()>1,
        "distinct RESULT opcode hypotheses must have separate budgets");
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let mut remote=Vec::new();
    let mut result=Vec::new();
    for a in &w.audit{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
            else{continue};
        if a.result.code!=Code::Ok{continue;}
        let Ok(frame)=decoder.decode(bytes)else{continue};
        let id=u64::from_le_bytes(a.result.payload[..8].try_into().unwrap());
        match frame.operation{
            Operation::RemoteEval if frame.body.len()==18=>remote.push(id),
            Operation::Result=>result.push(id),
            _=>(),
        }
    }
    assert_eq!(remote.len(),1,"no second B request is allowed");
    assert!(!result.is_empty());
    let confirmed=w.audit.iter().filter_map(|a|match &a.action{
        Some(Action::ConfirmResponseFor{request_id,..})
            if a.result.code==Code::Ok=>Some(*request_id),
        _=>None,
    }).collect::<Vec<_>>();
    assert!(confirmed.contains(&remote[0]));
    assert!(result.iter().any(|id|confirmed.contains(id)));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}

#[test]
fn s45_baseline_two_step_latency_keeps_correct_result(){
    let mut w=boot_s45(2);
    finish(&mut w);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_total(&w),24);
    assert!(w.state.guarded_restarts.is_empty());
    let actual=u32::from_le_bytes(w.state.objects[
        "/self/results/p2-job0/value_le"].bytes[..4].try_into().unwrap());
    let operands=w.state.objects[REMOTE_VALUES].bytes.chunks_exact(4)
        .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(actual,fold32(&operands).unwrap());
}

#[test]
fn s45_global_six_step_delay_and_real_task_drop_relearns_exactly_once(){
    let mut w=boot_s45(6);
    let old_id=actual_job_id(&mut w);
    // The initial request is already a genuine C-bound SEND. Change the
    // public task before delayed C execution: it MUST be fenced at B0.
    let task=w.state.objects.get_mut(DEPTH_TASK).expect("public task");
    let mut changed:TransformTask=serde_json::from_slice(&task.bytes).unwrap();
    changed.task_id="s37-changed-rule-v2".into();
    for example in &mut changed.examples{
        example.output=2*example.input.len() as u32;
    }
    task.bytes=serde_json::to_vec(&changed).unwrap();
    task.version+=1;
    finish(&mut w);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),
        "global6+drift: B={} drops={:?} restarts={:?} pending={:?}",
        b_total(&w),w.state.fence_drops,w.state.guarded_restarts,
        w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().map(|c|&c.loop_counters));
    assert_eq!(w.state.fence_drops.len(),1);
    assert_eq!(w.state.fence_drops[0].message_id,old_id);
    assert_eq!(w.state.guarded_restarts.get(&ORGANISM_ID),Some(&1));
    assert_eq!(b_total(&w),24);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let remote_sends=w.audit.iter().filter_map(|record|{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&record.action
            else{return None};
        if record.result.code!=Code::Ok{return None}
        let Ok(frame)=decoder.decode(bytes)else{return None};
        if frame.operation!=Operation::RemoteEval || frame.body.len()!=18{
            return None;
        }
        Some(u64::from_le_bytes(record.result.payload[..8]
            .try_into().unwrap()))
    }).collect::<Vec<_>>();
    assert_eq!(remote_sends.len(),2,"old fenced then fresh new job");
    assert_eq!(remote_sends[0],old_id);
    assert_ne!(remote_sends[0],remote_sends[1]);
    let actual_new=remote_sends[1];
    assert!(outcomes(&w).iter().any(|r|r.request_id==old_id &&
        r.state==RequestState::Dropped));
    assert_eq!(w.audit.iter().filter(|a|matches!(&a.action,
        Some(Action::RestartRootFor{request_id,..})
            if *request_id==old_id) && a.result.code==Code::Ok).count(),1);
    assert!(w.audit.iter().any(|a|matches!(&a.action,
        Some(Action::ConfirmResponseFor{request_id,..})
            if *request_id==actual_new) && a.result.code==Code::Ok));
    assert!(!w.audit.iter().any(|a|matches!(&a.action,
        Some(Action::ConfirmResponseFor{request_id,..})
            if *request_id==old_id) && a.result.code==Code::Ok));
    assert_eq!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count(),1);
    let actual=u32::from_le_bytes(w.state.objects[
        "/self/results/p2-job0/value_le"].bytes[..4].try_into().unwrap());
    let operands=w.state.objects[REMOTE_VALUES].bytes.chunks_exact(4)
        .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(actual,fold32(&operands).unwrap());
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}

#[test]
fn s45_delayed_phase2_forged_reply_fails_closed_after_real_b(){
    let mut w=boot_s45(6);
    let mut result_send_id=None;
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode));
        let Some(record)=w.audit.last()else{continue};
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&record.action
            else{continue};
        if record.result.code!=Code::Ok{continue}
        let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
        let Ok(frame)=decoder.decode(bytes)else{continue};
        if frame.operation==Operation::Result{
            result_send_id=Some(u64::from_le_bytes(record.result.payload[..8]
                .try_into().unwrap()));
            break;
        }
    }
    let result_send_id=result_send_id.expect("P2 must send a real RESULT opcode candidate");
    assert_eq!(b_total(&w),24,"B completed before RESULT exploration");
    // Researcher-only untrusted injection into the actual phase2 mailbox.
    // Delivery by Receive is NOT evidence of a legitimate service event.
    let forged=ocee_world_kernel::Message{
        id:u64::MAX-8,actor:ORGANISM_ID,
        source:NodeId::C,destination:NodeId::A,
        bytes:vec![0xde,0xad,0xbe,0xef],
        sent_step:w.state.step,deliver_step:w.state.step,
        sequence_no:u64::MAX-8,
    };
    w.state.mailboxes.entry(NodeId::A).or_default().insert(0,forged);
    finish(&mut w);
    assert!(w.audit.iter().any(|a|matches!(&a.action,
        Some(Action::ConfirmResponseFor{request_id,..})
            if *request_id==result_send_id) && a.result.code==Code::Conflict),
        "S43B confirmation must reject unowned reply inside S45");
    assert_eq!(b_total(&w),24);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(!w.state.objects.contains_key(
        "/self/results/p2-job0/value_le"),
        "forged RESULT must not produce a success artifact");
    assert_ne!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}

#[test]
fn s45_eight_step_delay_exploratory_liveness_boundary_and_causal_safety(){
    // Exploratory boundary probe: delay 8 is beyond the previously
    // certified delay-6 positive control. Never turn failure into success.
    let mut w=boot_s45(8);
    finish(&mut w);
    let returned=w.state.organisms[&ORGANISM_ID].last_return.clone();
    let b=b_total(&w);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let remote_ids=w.audit.iter().filter_map(|a|{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
            else{return None};
        if a.result.code!=Code::Ok{return None}
        let Ok(frame)=decoder.decode(bytes)else{return None};
        if frame.operation!=Operation::RemoteEval || frame.body.len()!=18{
            return None;
        }
        Some(u64::from_le_bytes(a.result.payload[..8].try_into().unwrap()))
    }).collect::<Vec<_>>();
    let cont=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
    let result_waits=cont.loop_counters.get("s45_result_retry")
        .copied().unwrap_or(0);
    println!("S45_DELAY_BOUNDARY delay=8 return={returned:?} b={b} remote_ids={remote_ids:?} result_waits={result_waits} restarts={:?}",
        w.state.guarded_restarts);
    assert!(result_waits<=896,"aggregate bounded budget exceeded");
    assert!(remote_ids.len()<=1,"cannot replay a potentially committed B request");
    assert!(b<=24,"B charged more than once");
    assert!(w.state.guarded_restarts.is_empty());
    if returned==Some(Value::U32(1)){
        assert_eq!(b,24,"success must have real paid B");
        assert_eq!(remote_ids.len(),1);
        let actual=u32::from_le_bytes(w.state.objects[
            "/self/results/p2-job0/value_le"].bytes[..4].try_into().unwrap());
        let operands=w.state.objects[REMOTE_VALUES].bytes.chunks_exact(4)
            .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(actual,fold32(&operands).unwrap());
    }else{
        assert!(!w.state.objects.contains_key(
            "/self/results/p2-job0/value_le"),
            "unsuccessful path cannot write a claimed successful RESULT");
    }
}

#[test]
fn s45_protocol_seed_paired_ablation_at_global_eight_step_delay(){
    // Exploratory, PRE-FIXED paired seeds. Only the privileged P2
    // protocol seed differs, never the world task, VM capabilities,
    // delay, budgets or the service evaluator. Both arms run on each
    // instance; no success is assumed from a green safety-only test.
    for protocol_seed in [17_u64,93_u64]{
        for (arm,p2,root) in [
            ("S44",s44_p2_causal_wait_consumer(),s44_outcome_aware_root()),
            ("S45",s45_p2_scoped_result_wait(),s45_scoped_result_root())
        ]{
            let program_hash=p2.content_hash().expect("exact frozen P2 graph");
            let mut w=boot_with_protocol_seed(8,protocol_seed,p2,root);
            finish(&mut w);
            let last=w.state.organisms[&ORGANISM_ID].last_return.clone();
            let b=b_total(&w);
            let decoder=&w.state.protocol_service.as_ref()
                .expect("P2 service").protocol;
            let remote=w.audit.iter().filter_map(|a|{
                let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
                    else{return None};
                if a.result.code!=Code::Ok{return None;}
                let Ok(frame)=decoder.decode(bytes)else{return None};
                if frame.operation!=Operation::RemoteEval ||
                    frame.body.len()!=18{return None;}
                Some(u64::from_le_bytes(a.result.payload[..8]
                    .try_into().unwrap()))
            }).collect::<Vec<_>>();
            let confirmations=w.audit.iter().filter_map(|a|match &a.action{
                Some(Action::ConfirmResponseFor{request_id,..})
                    if a.result.code==Code::Ok=>Some(*request_id),
                _=>None,
            }).collect::<Vec<_>>();
            let continuation=w.state.organisms[&ORGANISM_ID]
                .vm_continuation.as_ref().unwrap();
            let s23_wait=continuation.loop_counters.get("s23_receive_retry")
                .copied().unwrap_or(0);
            let s45_wait=continuation.loop_counters.get("s45_result_retry")
                .copied().unwrap_or(0);
            let success=last==Some(Value::U32(1));
            println!(
                "S45_PAIRED_ABLATION protocol_seed={protocol_seed} arm={arm} \
                 delay=8 success={success} return={last:?} B={b} \
                 remote_ids={remote:?} confirmed={confirmations:?} \
                 s23_wait={s23_wait} s45_wait={s45_wait} \
                 step={} hash={program_hash} restarts={:?}",
                w.state.step,w.state.guarded_restarts);
            assert!(b<=24,"B cannot be charged twice in any arm");
            assert!(remote.len()<=1,
                "an unconfirmed successful B request must not be resent");
            assert!(w.state.guarded_restarts.is_empty(),
                "no task drift was introduced in this experiment");
            assert!(s23_wait<=128);
            assert!(s45_wait<=896);
            if success{
                assert_eq!(b,24,"success requires true B resources");
                assert_eq!(remote.len(),1,"one real REMOTE_EVAL required");
                assert!(confirmations.contains(&remote[0]),
                    "success requires a C-sourced actual ACK");
                let result=u32::from_le_bytes(w.state.objects[
                    "/self/results/p2-job0/value_le"].bytes[..4]
                    .try_into().unwrap());
                let operands=w.state.objects[REMOTE_VALUES].bytes
                    .chunks_exact(4)
                    .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
                    .collect::<Vec<_>>();
                assert_eq!(result,fold32(&operands).unwrap());
            }else{
                assert!(!w.state.objects.contains_key(
                    "/self/results/p2-job0/value_le"),
                    "a failed arm cannot leave a claimed successful RESULT");
            }
            assert_eq!(w.state_hash(),
                World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
        }
    }
}


#[test]
fn s45_phase0_scoped_wait_restores_two_fixed_protocol_seed_regressions(){
    // Post-failure development regression: seeds 17 and 93 were
    // PREDECLARED in the paired-ablation snapshot 519f9ccb. That
    // frozen S44/S45 comparison genuinely returned 0/4 successes
    // before Phase0 waiting was changed. Never reclassify it.
    for protocol_seed in [17_u64,93_u64]{
        let mut w=boot_with_protocol_seed(8,protocol_seed,
            s45_p2_scoped_result_wait(),s45_scoped_result_root());
        finish(&mut w);
        let ret=w.state.organisms[&ORGANISM_ID].last_return.clone();
        let counters=w.state.organisms[&ORGANISM_ID]
            .vm_continuation.as_ref().unwrap().loop_counters.clone();
        let phase0=counters.get("s45_remote_retry").copied().unwrap_or(0);
        let phase2=counters.get("s45_result_retry").copied().unwrap_or(0);
        let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
        let remote=w.audit.iter().filter_map(|record|{
            let Some(Action::Send{destination:NodeId::C,bytes,..})
                =&record.action else{return None};
            if record.result.code!=Code::Ok{return None};
            let Ok(frame)=decoder.decode(bytes)else{return None};
            if frame.operation!=Operation::RemoteEval ||
                frame.body.len()!=18{return None};
            Some(u64::from_le_bytes(record.result.payload[..8]
                .try_into().unwrap()))
        }).collect::<Vec<_>>();
        let result_ids=w.audit.iter().filter_map(|record|{
            let Some(Action::Send{destination:NodeId::C,bytes,..})
                =&record.action else{return None};
            if record.result.code!=Code::Ok{return None};
            let Ok(frame)=decoder.decode(bytes)else{return None};
            if frame.operation!=Operation::Result{return None};
            Some(u64::from_le_bytes(record.result.payload[..8]
                .try_into().unwrap()))
        }).collect::<Vec<_>>();
        let confirmed=w.audit.iter().filter_map(|record|match &record.action{
            Some(Action::ConfirmResponseFor{request_id,..})
                if record.result.code==Code::Ok=>Some(*request_id),
            _=>None,
        }).collect::<Vec<_>>();
        println!("S45_PHASE0_REGRESSION protocol_seed={protocol_seed} \
            delay=8 return={ret:?} B={} remote_ids={remote:?} \
            phase0_wait={phase0} phase2_wait={phase2} \
            confirmed={confirmed:?} step={}",
            b_total(&w),w.state.step);
        assert!(phase0>0 && phase0<=896,
            "the previously shared Phase0 wait must use scoped budget");
        assert!(phase2<=896);
        assert_eq!(w.state.guarded_restarts.len(),0);
        assert_eq!(b_total(&w),24,
            "one real successful B commit only");
        assert_eq!(ret,Some(Value::U32(1)),
            "the previously failing fixed P2 seed must now succeed");
        assert_eq!(remote.len(),1,"B request cannot be blindly resent");
        assert_eq!(w.state.service_events.iter()
            .filter(|e|e.b_gas_charged>0).count(),1);
        assert!(confirmed.contains(&remote[0]));
        assert!(result_ids.iter().any(|id|confirmed.contains(id)),
            "actual RESULT candidate must have C-backed reply confirmation");
        let actual=u32::from_le_bytes(w.state.objects[
            "/self/results/p2-job0/value_le"].bytes[..4].try_into().unwrap());
        let operands=w.state.objects[REMOTE_VALUES].bytes.chunks_exact(4)
            .map(|x|u32::from_le_bytes(x.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(actual,fold32(&operands).unwrap());
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
}

 
/// Frozen development holdout, not E001 confirmatory evidence.
#[test]
#[ignore = "explicit frozen protocol-seed CI matrix only; not default workspace"]
fn p2_frozen_holdout_independent_protocol_instance(){
    let protocol_seed:u64=std::env::var("OCEE_P2_HOLDOUT_SEED")
        .expect("explicit holdout seed from frozen CI matrix")
        .parse().expect("holdout seed integer");
    assert!([311_u64,587,941,2027].contains(&protocol_seed),
        "unregistered seed: frozen holdout has exactly four entries");
    // Precommitted 2026-11-22: no tuning of the Phase0/Phase2 S45
    // graph, even when a held-out protocol is hard or unlearnable.
    let program=s45_p2_scoped_result_wait();
    let hash=program.content_hash().unwrap();
    let root=s45_scoped_result_root();
    let mut world=boot_with_protocol_seed(8,protocol_seed,program,root);
    // Keep the pre-existing world fixture's 14k-step budget and
    // safety/fault checks. No host-driven protocol hints or retries.
    finish(&mut world);
    let ret=world.state.organisms[&ORGANISM_ID].last_return.clone();
    let success=ret==Some(Value::U32(1));
    let b=b_total(&world);
    let service=&world.state.protocol_service.as_ref().unwrap().protocol;
    let mut remote_ids=Vec::<u64>::new();
    let mut result_ids=Vec::<u64>::new();
    for record in &world.audit{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&record.action
            else{continue};
        if record.result.code!=Code::Ok{continue;}
        assert_eq!(record.result.payload.len(),8,
            "successful SEND returns a true u64 receipt");
        let Ok(frame)=service.decode(bytes) else {continue;};
        let actual_id=u64::from_le_bytes(
            record.result.payload[..8].try_into().unwrap());
        match frame.operation{
            Operation::RemoteEval if frame.body.len()==18=>
                remote_ids.push(actual_id),
            Operation::Result=>result_ids.push(actual_id),
            _=>(),
        }
    }
    let verified_ids=world.audit.iter().filter_map(|record|{
        match &record.action{
            Some(Action::ConfirmResponseFor{request_id,..})
                if record.result.code==Code::Ok=>Some(*request_id),
            _=>None,
        }
    }).collect::<Vec<_>>();
    let counters=&world.state.organisms[&ORGANISM_ID]
        .vm_continuation.as_ref().unwrap().loop_counters;
    let p0=counters.get("s45_remote_retry").copied().unwrap_or(0);
    let p2=counters.get("s45_result_retry").copied().unwrap_or(0);
    let legacy=counters.get("s23_receive_retry").copied().unwrap_or(0);
    let positive_service_events=world.state.service_events.iter()
        .filter(|event|event.b_gas_charged>0).count();
    let a_vm=world.audit.iter()
        .map(|record|record.result.vm_gas_charged).sum::<u64>();
    let a_actions=world.audit.iter()
        .map(|record|record.result.gas_charged).sum::<u64>();
    let total_sent=world.audit.iter().filter(|record|
        matches!(&record.action,Some(Action::Send{destination:NodeId::C,..}))
        && record.result.code==Code::Ok).count();
    println!(
        "P2_FROZEN_HOLDOUT seed={protocol_seed} success={success} \
         return={ret:?} B={b} b_events={positive_service_events} \
         remote_ids={remote_ids:?} result_ids={result_ids:?} \
         confirmed={verified_ids:?} phase0_wait={p0} \
         phase2_wait={p2} legacy_wait={legacy} tick={} \
         A_vm={a_vm} A_actions={a_actions} sends={total_sent} \
         program_hash={hash} restarts={:?}",
        world.state.step,world.state.guarded_restarts);
    // HARD SAFETY: a clean no-drift run never duplicates B or retries
    // an uncertain request, even when it cannot discover the opcode.
    assert!(b<=24,"any B overcharge invalidates the held-out trial");
    assert!(remote_ids.len()<=1,"no potentially committed B replay");
    assert!(positive_service_events<=1,"one B event at most");
    assert!(world.state.guarded_restarts.is_empty(),
        "no task drift means no authorized restart");
    assert!(p0<=896 && p2<=896 && legacy<=128);
    for prefix in ["s45_remote_retry:","s45_result_retry:"]{
        for (key,&count) in counters.iter().filter(|(k,_)|k.starts_with(prefix)){
            assert!(count<=6,"scoped cap violated: {key}={count}");
        }
    }
    if success{
        assert_eq!(b,24,"successful task requires B's real gas");
        assert_eq!(positive_service_events,1);
        assert_eq!(remote_ids.len(),1);
        assert!(verified_ids.contains(&remote_ids[0]),
            "real RemoteEval ACK must be kernel-authenticated");
        assert!(result_ids.iter().any(|id|verified_ids.contains(id)),
            "a real RESULT must have a kernel-authenticated reply");
        let bytes=&world.state.objects[
            "/self/results/p2-job0/value_le"].bytes;
        assert!(bytes.len()>=4);
        let result=u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let operands=world.state.objects[REMOTE_VALUES].bytes
            .chunks_exact(4).map(|chunk|
                u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(result,fold32(&operands).unwrap(),
            "success requires independent real operand verification");
    }else{
        assert!(!world.state.objects.contains_key(
            "/self/results/p2-job0/value_le"),
            "failed task cannot create a false successful result");
    }
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
}


/// Developmental resource pressure calibration. Never substitute its
/// pre-provisioned genome for E001's fully charged genesis.
#[test]
#[ignore = "expensive two-seed resource profile; run only in targeted CI"]
fn p2_a_gas_12k_then_4k_existing_activity_under_real_epoch_grants(){
    use ocee_world_kernel::{EventKind,ScheduledEvent};
    let seed:u64=std::env::var("OCEE_P2_GAS_SEED")
        .expect("CI pins the selected development seed")
        .parse().expect("numeric seed");
    assert!([2027_u64,587].contains(&seed));
    let p2=s45_p2_scoped_result_wait();
    let program_hash=p2.content_hash().unwrap();
    let root=s45_scoped_result_root();
    let mut w=boot_with_protocol_seed(8,seed,p2,root);
    let genesis_steps=w.state.step;
    let genesis_events=w.audit.len();
    let genome_install_a=w.audit.iter()
        .map(|r|r.result.gas_charged+r.result.vm_gas_charged)
        .sum::<u64>();

    // The legacy fixture registered a complete seeded genome with
    // 100k/epoch. Treat that as OFF-LEDGER GENESIS for this test.
    // At time of first autonomous tick, eliminate the 100k credit
    // and set A's actual account and replenishment ceiling to 12k.
    // This is optimistic for E001: genesis was not independently
    // made affordable. Do not report an end-to-end resource pass.
    assert_eq!(w.state.config.a_gas_per_epoch,100_000);
    assert!(genesis_steps<8192);
    w.state.config.a_gas_per_epoch=12_000;
    let node=w.state.nodes.get_mut(&NodeId::A).unwrap();
    node.gas_per_epoch=12_000;
    node.gas_available=12_000;
    w.schedule_event(ScheduledEvent{
        due_step:8192,priority:0,sequence_no:0,
        kind:EventKind::ResourceChange{
            node:NodeId::A,gas_per_epoch:4_000
        }
    });

    // Preserve the previous 14_000 autonomous-tick observation limit.
    // If the organism is still active at the limit, report censored;
    // never silently grant more gas/ticks or count as a completed task.
    let mut gas_refusals=0u64;
    let mut invalid=0u64;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        if r.code==Code::Gas{gas_refusals+=1;}
        if matches!(r.code,Code::Denied|Code::InvalidCode){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let returned=host.last_return.clone();
    let success=finished&&returned==Some(Value::U32(1));
    let b=b_total(&w);
    let scoped=host.vm_continuation.as_ref()
        .map(|c|c.loop_counters.clone()).unwrap_or_default();
    let phase0=scoped.get("s45_remote_retry").copied().unwrap_or(0);
    let phase2=scoped.get("s45_result_retry").copied().unwrap_or(0);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let mut remote=Vec::new();
    let mut results=Vec::new();
    for record in w.audit.iter().skip(genesis_events){
        let Some(Action::Send{destination:NodeId::C,bytes,..})
            =&record.action else{continue};
        if record.result.code!=Code::Ok{continue;}
        let Ok(frame)=decoder.decode(bytes)else{continue};
        assert_eq!(record.result.payload.len(),8);
        let id=u64::from_le_bytes(record.result.payload[..8]
            .try_into().unwrap());
        match frame.operation{
            Operation::RemoteEval if frame.body.len()==18=>remote.push(id),
            Operation::Result=>results.push(id),
            _=>{},
        }
    }
    let confirmed=w.audit.iter().skip(genesis_events)
        .filter_map(|a|match &a.action{
            Some(Action::ConfirmResponseFor{request_id,..})
                if a.result.code==Code::Ok=>Some(*request_id),
            _=>None,
        }).collect::<Vec<_>>();
    let operating_vm=w.audit.iter().skip(genesis_events)
        .map(|a|a.result.vm_gas_charged).sum::<u64>();
    let operating_actions=w.audit.iter().skip(genesis_events)
        .map(|a|a.result.gas_charged).sum::<u64>();
    let a=&w.state.nodes[&NodeId::A];
    let after_downshift=w.state.step>=8193;
    println!(
        "P2_A_GAS_PRESSURE seed={seed} limit=14000 \
         initial_epoch=12000 downshift_at=8192 final_epoch={} \
         genesis_unmetered_a={genome_install_a} \
         finished={finished} success={success} return={returned:?} \
         B={b} remote_ids={remote:?} result_candidates={} \
         confirmed_remote={} confirmed_result={} \
         phase0_wait={phase0} phase2_wait={phase2} \
         A_vm={operating_vm} A_actions={operating_actions} \
         gas_refusals={gas_refusals} invalid={invalid} \
         final_step={} final_a_available={} program_hash={program_hash}",
        a.gas_per_epoch,results.len(),
        remote.iter().any(|id|confirmed.contains(id)),
        results.iter().any(|id|confirmed.contains(id)),
        w.state.step,a.gas_available);
    // Invariants apply even when a resource-limited organism stops
    // learning, refuses an action, or is censored at 14k ticks.
    assert_eq!(invalid,0,"low A quota must not cause invalid VM execution");
    assert!(b<=24);
    assert!(remote.len()<=1,"no duplicate B request");
    assert!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count()<=1);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(phase0<=896 && phase2<=896);
    for prefix in ["s45_remote_retry:","s45_result_retry:"]{
        for (key,value) in scoped.iter().filter(|(k,_)|k.starts_with(prefix)){
            assert!(*value<=6,"scoped wait safety {key}={value}");
        }
    }
    if after_downshift{
        assert_eq!(a.gas_per_epoch,4_000,
            "true resource event must be applied at epoch boundary");
    }else{
        assert_eq!(a.gas_per_epoch,12_000);
    }
    if success{
        assert_eq!(b,24);
        assert_eq!(remote.len(),1);
        assert!(confirmed.contains(&remote[0]));
        assert!(results.iter().any(|id|confirmed.contains(id)));
        let actual=u32::from_le_bytes(w.state.objects[
            "/self/results/p2-job0/value_le"].bytes[..4]
            .try_into().unwrap());
        let operands=w.state.objects[REMOTE_VALUES].bytes
            .chunks_exact(4).map(|chunk|
                u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(actual,fold32(&operands).unwrap());
    }else{
        assert!(!w.state.objects.contains_key(
            "/self/results/p2-job0/value_le"),
            "no valid final result can be written on failure or censoring");
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// Separate strict-boot developmental control: initial genome Writes
/// really draw on the 12k epoch-0 A budget. No free genesis refill.
#[test]
#[ignore = "targeted charged-genesis calibration, never default workspace"]
fn p2_charged_genesis_in_12k_a_epoch_with_no_credit_reset(){
    charged_genesis_probe(2027,false);
}

/// Falsifiable combination of TWO independently validated constraints:
/// paid genesis AND a real 12k -> 4k epoch change in ONE VM continuation.
#[test]
#[ignore = "one high-cost development seed under paid genesis + downshift"]
fn p2_charged_genesis_crosses_real_4k_downshift_with_b_once(){
    charged_genesis_probe(587,true);
}
fn charged_genesis_probe(protocol_seed:u64,require_downshift:bool){
    use ocee_world_kernel::{EventKind,ScheduledEvent};
    assert!([2027,587].contains(&protocol_seed));
    let mut w=boot_with_protocol_seed_budget(8,protocol_seed,12_000,
        s45_p2_scoped_result_wait(),s45_scoped_result_root());
    assert_eq!(w.state.step,4,"exactly four real genesis writes");
    let genesis_a=w.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum::<u64>();
    let available=w.state.nodes[&NodeId::A].gas_available;
    assert!(genesis_a>0 && genesis_a<12_000);
    assert_eq!(available+genesis_a,12_000,
        "no charged setup gas can be silently reset/refunded");
    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,12_000);
    let genome_hash=s45_p2_scoped_result_wait().content_hash().unwrap();
    w.schedule_event(ScheduledEvent{
        due_step:8192,priority:0,sequence_no:0,
        kind:EventKind::ResourceChange{
            node:NodeId::A,gas_per_epoch:4_000
        }
    });
    let genesis_events=w.audit.len();
    let mut gas_refusals=0u64;
    let mut invalid=0u64;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        if r.code==Code::Gas{gas_refusals+=1;}
        if matches!(r.code,Code::Denied|Code::InvalidCode){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let returned=host.last_return.clone();
    let success=finished&&returned==Some(Value::U32(1));
    let b=b_total(&w);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let mut remote=Vec::new();
    let mut result=Vec::new();
    for record in w.audit.iter().skip(genesis_events){
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&record.action
            else{continue};
        if record.result.code!=Code::Ok{continue;}
        let Ok(frame)=decoder.decode(bytes)else{continue};
        let id=u64::from_le_bytes(record.result.payload[..8]
            .try_into().unwrap());
        match frame.operation{
            Operation::RemoteEval if frame.body.len()==18=>remote.push(id),
            Operation::Result=>result.push(id),
            _=>(),
        }
    }
    let confirmed=w.audit.iter().skip(genesis_events).filter_map(
        |a|match &a.action{
            Some(Action::ConfirmResponseFor{request_id,..})
                if a.result.code==Code::Ok=>Some(*request_id),
            _=>None,
        }).collect::<Vec<_>>();
    let a_vm=w.audit.iter().skip(genesis_events)
        .map(|a|a.result.vm_gas_charged).sum::<u64>();
    let a_act=w.audit.iter().skip(genesis_events)
        .map(|a|a.result.gas_charged).sum::<u64>();
    let next_epoch=w.state.nodes[&NodeId::A].gas_per_epoch;
    println!(
        "P2_CHARGED_GENESIS seed={protocol_seed} delay=8 initial_epoch=12000 \
         genesis_a={genesis_a} initial_remaining={available} \
         success={success} finished={finished} return={returned:?} \
         B={b} remote_ids={remote:?} result_ids={result:?} \
         confirmed_remote={} confirmed_result={} \
         A_vm={a_vm} A_actions={a_act} gas_refusals={gas_refusals} \
         invalid={invalid} step={} final_epoch={next_epoch} \
         program_hash={genome_hash}",
        remote.iter().any(|id|confirmed.contains(id)),
        result.iter().any(|id|confirmed.contains(id)),w.state.step);
    assert_eq!(invalid,0);
    assert!(b<=24 && remote.len()<=1);
    assert!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count()<=1);
    assert!(w.state.guarded_restarts.is_empty());
    if w.state.step>=8193{
        assert_eq!(next_epoch,4_000);
    }else{
        assert_eq!(next_epoch,12_000);
    }
    // In the combined high-cost case, counting the configuration event
    // alone is insufficient. The organism must actually execute charged
    // VM instructions AFTER the 4k epoch became effective.
    let post_downshift_vm=w.audit.iter().filter(|r|
        r.step>=8192 && r.result.vm_gas_charged>0
    ).map(|r|r.result.vm_gas_charged).sum::<u64>();
    let post_downshift_ticks=w.audit.iter().filter(|r|
        r.step>=8192 && r.result.vm_gas_charged>0
    ).count();
    println!(
        "P2_CHARGED_DOWNSHIFT seed={protocol_seed} required={require_downshift} \
         event_step=8192 crossed={} post_vm={post_downshift_vm} \
         post_ticks={post_downshift_ticks} final_epoch={next_epoch} \
         completed={success} B={b} final_step={}",
        w.state.step>=8193,w.state.step);
    if require_downshift{
        assert!(w.state.step>=8193,
            "charged-genesis instance did not reach the scheduled cut");
        assert_eq!(next_epoch,4_000);
        assert!(post_downshift_vm>0 && post_downshift_ticks>0,
            "no actual VM progress under the new 4k epoch limit");
        assert!(success,
            "paid genesis and real 4k downshift must both preserve task liveness");
    }
    if success{
        assert_eq!(b,24);
        assert_eq!(remote.len(),1);
        assert!(confirmed.contains(&remote[0]));
        assert!(result.iter().any(|id|confirmed.contains(id)));
        let actual=u32::from_le_bytes(w.state.objects[
            "/self/results/p2-job0/value_le"].bytes[..4]
            .try_into().unwrap());
        let operands=w.state.objects[REMOTE_VALUES].bytes
            .chunks_exact(4)
            .map(|bytes|u32::from_le_bytes(bytes.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(actual,fold32(&operands).unwrap());
    }else{
        assert!(!w.state.objects.contains_key(
            "/self/results/p2-job0/value_le"));
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}

/// Independent *researcher-authored* local FOLD32 for exactly four little-
/// endian U32s. Uses ONLY generic VM operations: ByteAt, Add, Div, ConstU32.
/// No VM-specific Fold32 shortcut, no host-computed answer passed as input,
/// and no privileged protocol opcode. This benchmark is NOT organism learning.
fn local_fold4_vm_program()->Program{
    use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Terminator,Type};
    let mut code=Vec::new();
    let mut reg=1u32;
    let mut emit=|op:Op|{
        let id=reg;
        reg+=1;
        code.push(Instruction{id,op});
        id
    };
    let zero=emit(Op::ConstU32(0));
    let mut sum=zero;
    for word in 0u32..4{
        let mut value=zero;
        // Reconstruct an arbitrary LE u32 by Horner's method in base 256.
        for byte in (0u32..4).rev(){
            if byte!=3 {
                for _ in 0..8{value=emit(Op::Add(value,value));}
            }
            let index=emit(Op::ConstU32(word*4+byte));
            let digit=emit(Op::ByteAt(0,index));
            value=emit(Op::Add(value,digit));
        }
        // rotate_left(word) = wrapping(v * 2^word) + (v >> (32-word)).
        // Only word 0..3 used. All operations are metered ordinary VM ops.
        let rotated=if word==0 {value} else{
            let mut low=value;
            for _ in 0..word{low=emit(Op::Add(low,low));}
            let divisor=emit(Op::ConstU32(1_u32<<(32-word)));
            let high=emit(Op::Div(value,divisor));
            emit(Op::Add(low,high))
        };
        sum=emit(Op::Add(sum,rotated));
    }
    let program=Program{
        format_version:1,
        entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),
                params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:code,
                terminator:Terminator::Return{value:sum}
            }]
        }]
    };
    program.validate().expect("pure typed bounded local FOLD32");
    program
}

/// Marginal calculation-cost counterfactual at the EXACT operand bytes
/// actually generated by the paid-genesis S45 organism, not a toy input
/// chosen to make local arithmetic easier. This is not a whole-task paired
/// control, as the local program is deliberately researcher-supplied later.
#[test]
#[ignore = "targeted matched-operand local-vs-offload resource calibration"]
fn p2_matched_local_fold4_cost_after_real_paid_s45_task(){
    use ocee_organism_vm::run_isolated;
    let local=local_fold4_vm_program();
    let local_hash=local.content_hash().unwrap();
    let pure_cases=[
        [0_u32,0,0,0],
        [u32::MAX,u32::MAX,u32::MAX,u32::MAX],
        [0x8000_0000,0x7fff_ffff,0x8000_0001,1],
    ];
    for values in pure_cases{
        let bytes=values.iter().flat_map(|v|v.to_le_bytes())
            .collect::<Vec<_>>();
        let (out,gas)=run_isolated(&local,bytes,2000)
            .expect("generic local baseline must compute arbitrary U32s");
        assert_eq!(out,Value::U32(fold32(&values).unwrap()));
        assert!(gas>0 && gas<2000);
    }
    // A fully CHARGED 12k genesis, no top-up. The known development
    // instance 2027 was selected for inexpensive matched-input acquisition.
    let mut w=boot_with_protocol_seed_budget(8,2027,12_000,
        s45_p2_scoped_result_wait(),s45_scoped_result_root());
    let genesis_cost=w.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum::<u64>();
    assert_eq!(w.state.nodes[&NodeId::A].gas_available+genesis_cost,
        12_000,"real first-epoch charged genesis");
    let original_genome_hash=s45_p2_scoped_result_wait().content_hash().unwrap();
    let mut gas_refusals=0u64;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "original organism VM must not trap");
        if r.code==Code::Gas{gas_refusals+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(b_total(&w),24);
    assert_eq!(w.state.service_events.iter()
        .filter(|e|e.b_gas_charged>0).count(),1);
    let remote_operands=w.state.objects.get(REMOTE_VALUES)
        .expect("real root wrote learned P1-derived fourth operand")
        .bytes.clone();
    assert_eq!(remote_operands.len(),16);
    let values=remote_operands.chunks_exact(4).map(|x|
        u32::from_le_bytes(x.try_into().unwrap())).collect::<Vec<_>>();
    assert_eq!(values.len(),4);
    let verified=fold32(&values).unwrap();
    let committed=u32::from_le_bytes(
        w.state.objects["/self/results/p2-job0/value_le"].bytes[..4]
        .try_into().unwrap());
    assert_eq!(committed,verified);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let mut remote_ids=Vec::new();
    let mut result_ids=Vec::new();
    let mut c_send_count=0usize;
    let mut c_send_bandwidth=0u64;
    for a in &w.audit{
        let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action
            else{continue};
        if a.result.code!=Code::Ok{continue;}
        c_send_count+=1;
        c_send_bandwidth+=a.result.bandwidth_charged;
        let Ok(frame)=decoder.decode(bytes)else{continue};
        let id=u64::from_le_bytes(a.result.payload[..8]
            .try_into().unwrap());
        match frame.operation{
            Operation::RemoteEval if frame.body.len()==18=>remote_ids.push(id),
            Operation::Result=>result_ids.push(id),
            _=>(),
        }
    }
    let confirmed=w.audit.iter().filter_map(|a|match &a.action{
        Some(Action::ConfirmResponseFor{request_id,..})
            if a.result.code==Code::Ok=>Some(*request_id),
        _=>None,
    }).collect::<Vec<_>>();
    assert_eq!(remote_ids.len(),1);
    assert!(confirmed.contains(&remote_ids[0]));
    assert!(result_ids.iter().any(|id|confirmed.contains(id)));
    let remote_a_vm=w.audit.iter()
        .map(|r|r.result.vm_gas_charged).sum::<u64>();
    let remote_a_actions=w.audit.iter()
        .map(|r|r.result.gas_charged).sum::<u64>();
    let remote_a_total=remote_a_vm+remote_a_actions;
    let remote_finish_step=w.state.step;
    let original_world_hash=w.state_hash();
    assert_eq!(original_world_hash,
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    // Counterfactual branches from the exact already-generated operand
    // object. This action is issued by the external RESEARCHER, not
    // disguised as a causal decision of the original organism.
    let before_local=w.snapshot_json();
    let mut local_world=World::from_snapshot(&before_local).unwrap();
    let install=local_world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{local_hash}"),
        expected_version:None,bytes:local.canonical_bytes()
    }));
    assert_eq!(install.code,Code::Ok,
        "real A must afford a charged local program install");
    let executed=local_world.tick(Some(Action::Exec{
        actor:ORGANISM_ID,program_hash:local_hash.clone(),
        input:remote_operands,
    }));
    assert_eq!(executed.code,Code::Ok,
        "real A must afford a charged isolated pure VM computation");
    let output:Value=serde_json::from_slice(&executed.payload).unwrap();
    assert_eq!(output,Value::U32(verified));
    assert_eq!(output,Value::U32(committed));
    assert_eq!(b_total(&local_world),24,
        "the local marginal counterfactual must NOT create another B charge");
    assert_eq!(local_world.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),
        "researcher execution must not alter the original organism return");
    assert_eq!(original_world_hash,
        World::from_snapshot(&before_local).unwrap().state_hash());
    assert_eq!(local_world.state_hash(),
        World::from_snapshot(&local_world.snapshot_json()).unwrap().state_hash());
    let marginal_local_gas=install.gas_charged+
        executed.gas_charged;
    assert!(marginal_local_gas>0 && marginal_local_gas<12_000);
    println!(
        "P2_MATCHED_LOCAL seed=2027 delay=8 remote_success=true \
         original_program_hash={original_genome_hash} original_step={remote_finish_step} \
         original_A_vm={remote_a_vm} original_A_actions={remote_a_actions} \
         original_A_total={remote_a_total} original_B=24 \
         original_C_sends={c_send_count} original_C_send_bandwidth={c_send_bandwidth} \
         original_gas_refusals={gas_refusals} remote_id={} \
         local_program_hash={local_hash} local_bytes={} \
         local_install_A={} local_exec_A={} local_total_A={} \
         local_exec_step={} same_operand_count={} same_result={verified}",
        remote_ids[0],local.canonical_bytes().len(),
        install.gas_charged,executed.gas_charged,
        marginal_local_gas,local_world.state.step,values.len());
}


/// Common paid learning root and specialist capabilities; a RESEARCHER
/// intervention on /self/compute-route selects L or R at the normal
/// S45 root handover AFTER learning real fourth remote operand.
fn equal_origin_route_root(local_hash:&str)->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(v:&[u8])->Op{Op::ConstBytes{hex:v.iter().map(|b|format!("{b:02x}")).collect()}}
    fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
    fn p(id:u32,ty:Type)->Param{Param{id,ty}}
    fn b(id:&str,params:Vec<Param>,ins:Vec<Instruction>,term:Terminator)->Block{
        Block{id:id.into(),params,instructions:ins,terminator:term}
    }
    fn branch(c:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
        Terminator::Branch{condition:c,if_true:t.into(),true_args:ta,
            if_false:f.into(),false_args:fa}
    }
    fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
        Terminator::Yield{call,args,next:next.into(),next_args:carry}
    }
    let mut root=s45_scoped_result_root();
    let f=&mut root.functions[0];
    let sw=f.blocks.iter_mut().find(|b|b.id=="switch_active")
        .expect("original S40 paid switch");
    assert_eq!(sw.params[0].id,700);
    let mut remote=sw.clone();
    remote.id="equal_remote_switch".into();
    // switch_active is ALSO used for the original P1 handover.
    // Only divert the actual final P2 target, preserving the early
    // common learning phase in both arms.
    let p2_hash=s45_p2_scoped_result_wait().content_hash().unwrap();
    sw.instructions=vec![
        i(17100,lit(p2_hash.as_bytes())),
        i(17101,Op::EqBytes(700,17100))
    ];
    sw.terminator=branch(17101,"equal_route_read",vec![700],
        "equal_remote_switch",vec![700]);
    f.blocks.extend([
        remote,
        b("equal_route_read",vec![p(700,Type::Bytes)],vec![
            i(17102,lit(b"/self/compute-route"))
        ],y(WorldCall::Read,vec![17102],"equal_route_reply",vec![700])),
        b("equal_route_reply",vec![p(700,Type::Bytes),p(17110,Type::Bytes)],vec![
            i(17111,lit(b"code")),i(17112,Op::JsonFieldString(17110,17111)),
            i(17113,lit(b"Ok")),i(17114,Op::EqBytes(17112,17113))
        ],branch(17114,"equal_route_decode",vec![700,17110],"failed",vec![])),
        b("equal_route_decode",vec![p(700,Type::Bytes),p(17120,Type::Bytes)],vec![
            i(17121,lit(b"payload")),i(17122,Op::JsonFieldBytes(17120,17121)),
            i(17123,lit(b"L")),i(17124,Op::EqBytes(17122,17123)),
        ],branch(17124,"equal_local_read",vec![],
            "equal_route_remote_gate",vec![700,17122])),
        b("equal_route_remote_gate",vec![p(700,Type::Bytes),p(17130,Type::Bytes)],vec![
            i(17131,lit(b"R")),i(17132,Op::EqBytes(17130,17131)),
        ],branch(17132,"equal_remote_switch",vec![700],"failed",vec![])),
        b("equal_local_read",vec![],vec![i(17201,lit(REMOTE_VALUES.as_bytes()))],
            y(WorldCall::Read,vec![17201],"equal_local_read_reply",vec![])),
        b("equal_local_read_reply",vec![p(17210,Type::Bytes)],vec![
            i(17211,lit(b"code")),i(17212,Op::JsonFieldString(17210,17211)),
            i(17213,lit(b"Ok")),i(17214,Op::EqBytes(17212,17213))
        ],branch(17214,"equal_local_exec",vec![17210],"failed",vec![])),
        b("equal_local_exec",vec![p(17220,Type::Bytes)],vec![
            i(17221,lit(b"payload")),i(17222,Op::JsonFieldBytes(17220,17221)),
            i(17223,lit(local_hash.as_bytes())),
        ],y(WorldCall::Exec,vec![17223,17222],"equal_local_exec_reply",vec![])),
        b("equal_local_exec_reply",vec![p(17300,Type::Bytes)],vec![
            i(17301,lit(b"code")),i(17302,Op::JsonFieldString(17300,17301)),
            i(17303,lit(b"Ok")),i(17304,Op::EqBytes(17302,17303))
        ],branch(17304,"equal_local_pack",vec![17300],"failed",vec![])),
        b("equal_local_pack",vec![p(17310,Type::Bytes)],vec![
            i(17311,lit(b"payload")),i(17312,Op::JsonFieldBytes(17310,17311)),
            i(17313,lit(b"U32")),i(17314,Op::JsonFieldU32(17312,17313)),
            i(17315,Op::ConstU32(65536)),i(17316,Op::Mod(17314,17315)),
            i(17317,Op::Div(17314,17315)),i(17318,Op::PackU16Le(17316)),
            i(17319,Op::PackU16Le(17317)),i(17320,Op::Concat(17318,17319)),
            i(17321,lit(b"/self/results/p2-job0/value_le")),
            i(17322,Op::ConstU32(u32::MAX))
        ],y(WorldCall::Write,vec![17321,17320,17322],
            "equal_local_saved",vec![])),
        b("equal_local_saved",vec![p(17400,Type::Bytes)],vec![
            i(17401,lit(b"code")),i(17402,Op::JsonFieldString(17400,17401)),
            i(17403,lit(b"Ok")),i(17404,Op::EqBytes(17402,17403))
        ],branch(17404,"equal_local_done",vec![],"failed",vec![])),
        b("equal_local_done",vec![],vec![i(17410,Op::ConstU32(1))],
            Terminator::Return{value:17410}),
    ]);
    root.validate().expect("typed common A-only/A+B routed root");
    root
}

/// Separate native CI job per route, same initial snapshot hash and
/// initial program hashes. Neither route is the organism's own choice.
#[test]
#[ignore = "explicit same-snapshot A-only vs A+B full-task pair"]
fn p2_equal_origin_researcher_route_full_task_pair(){
    let arm=std::env::var("OCEE_P2_PAIR_ROUTE").expect("matrix route");
    assert!(arm=="L" || arm=="R");
    let local=local_fold4_vm_program();
    let local_hash=local.content_hash().unwrap();
    let root=equal_origin_route_root(&local_hash);
    let root_hash=root.content_hash().unwrap();
    let p2=s45_p2_scoped_result_wait();
    let p2_hash=p2.content_hash().unwrap();
    let mut base=boot_with_protocol_seed_budget(8,2027,12_000,p2,root);
    let install=base.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{local_hash}"),
        expected_version:None,bytes:local.canonical_bytes()
    }));
    assert_eq!(install.code,Code::Ok);
    let genesis=base.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum::<u64>();
    let remaining=base.state.nodes[&NodeId::A].gas_available;
    assert_eq!(genesis+remaining,12_000);
    assert_eq!(base.state.step,5);
    let before_hash=base.state_hash();
    let mut w=World::from_snapshot(&base.snapshot_json()).unwrap();
    assert_eq!(w.state_hash(),before_hash);
    let route=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/compute-route".into(),
        expected_version:None,bytes:arm.as_bytes().to_vec()
    }));
    assert_eq!(route.code,Code::Ok);
    assert_eq!(route.gas_charged,3);
    let mut gas_refusals=0u64;
    let mut invalid=0u64;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        if r.code==Code::Gas{gas_refusals+=1;}
        if matches!(r.code,Code::InvalidCode|Code::Denied){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref().is_some_and(|c|c.finished);
    let ret=host.last_return.clone();
    let success=finished&&ret==Some(Value::U32(1));
    let a_vm=w.audit.iter().map(|r|r.result.vm_gas_charged).sum::<u64>();
    let a_actions=w.audit.iter().map(|r|r.result.gas_charged).sum::<u64>();
    let a_total=a_vm+a_actions;
    let b=b_total(&w);
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    let mut c_sends=0usize;
    let mut bandwidth=0u64;
    let mut remote_ids=Vec::<u64>::new();
    let mut result_ids=Vec::<u64>::new();
    let mut local_exec=Vec::<u64>::new();
    for record in &w.audit{
        match &record.action{
            Some(Action::Send{destination:NodeId::C,bytes,..})
                if record.result.code==Code::Ok=>{
                c_sends+=1;bandwidth+=record.result.bandwidth_charged;
                if let Ok(frame)=decoder.decode(bytes){
                    let id=u64::from_le_bytes(record.result.payload[..8]
                        .try_into().unwrap());
                    match frame.operation{
                        Operation::RemoteEval if frame.body.len()==18=>remote_ids.push(id),
                        Operation::Result=>result_ids.push(id),
                        _=>()
                    }
                }
            },
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&local_hash && record.result.code==Code::Ok=>{
                assert_eq!(input.len(),16);
                local_exec.push(record.result.gas_charged)
            },
            _=>()
        }
    }
    let local_exec_gas=w.audit.iter().filter_map(|r|match &r.action{
        Some(Action::Exec{program_hash,..}) if program_hash==&local_hash
            && r.result.code==Code::Gas=>Some((r.step,r.result.gas_charged)),
        _=>None
    }).collect::<Vec<_>>();
    let failed_local_attempts=w.audit.iter().filter_map(|r|match &r.action{
        Some(Action::Exec{program_hash,..}) if program_hash==&local_hash
            && r.result.code!=Code::Ok && r.result.code!=Code::Gas=>
                Some((r.step,r.result.code)),
        _=>None
    }).collect::<Vec<_>>();
    let confirmed=w.audit.iter().filter_map(|r|match &r.action{
        Some(Action::ConfirmResponseFor{request_id,..})
            if r.result.code==Code::Ok=>Some(*request_id),
        _=>None
    }).collect::<Vec<_>>();
    let bytes=w.state.objects.get(REMOTE_VALUES)
        .expect("common root computed fourth operand").bytes.clone();
    assert_eq!(bytes.len(),16);
    let values=bytes.chunks_exact(4).map(|c|
        u32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>();
    let expected=fold32(&values).unwrap();
    let output=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|obj|{
            assert_eq!(obj.bytes.len(),4);
            u32::from_le_bytes(obj.bytes[..4].try_into().unwrap())
        });
    println!(
        "P2_EQUAL_ORIGIN arm={arm} success={success} finished={finished} \\
         return={ret:?} prehash={before_hash} root_hash={root_hash} \\
         p2_hash={p2_hash} local_hash={local_hash} \\
         genesis_A={genesis} route_A={} remaining={remaining} \\
         A_vm={a_vm} A_actions={a_actions} A_total={a_total} B={b} \\
         C_sends={c_sends} C_send_bandwidth={bandwidth} \\
         remote_ids={remote_ids:?} result_ids={result_ids:?} \\
         local_exec={local_exec:?} local_exec_gas={local_exec_gas:?} \\
         local_exec_bad={failed_local_attempts:?} gas_refusals={gas_refusals} \\
         invalid={invalid} final_step={} output={output:?} expected={expected}",
        route.gas_charged,w.state.step);
    assert_eq!(invalid,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(success,"full task must succeed in both fixed-route arms");
    assert_eq!(output,Some(expected));
    if arm=="L"{
        assert_eq!(b,0);
        assert_eq!(c_sends,0);
        assert!(remote_ids.is_empty() && result_ids.is_empty());
        assert_eq!(local_exec,vec![165]);
        assert!(confirmed.is_empty());
    }else{
        assert_eq!(b,24);
        assert_eq!(w.state.service_events.iter()
            .filter(|event|event.b_gas_charged>0).count(),1);
        assert_eq!(remote_ids.len(),1);
        assert!(confirmed.contains(&remote_ids[0]));
        assert!(result_ids.iter().any(|id|confirmed.contains(id)));
        assert!(local_exec.is_empty());
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// Follow-up to the paid L/R pair: the ACTIVE VM attempts its actually
/// installed local tool and selects the P2 service ONLY after an observed
/// NotFound response. The investigator never writes a route byte.
/// This is fixed researcher-authored availability fallback, NOT a learned
/// cost policy or autonomous creation of a tool.
fn p2_feedback_availability_route_root(local_hash:&str)->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type};
    fn lit(v:&[u8])->Op{
        Op::ConstBytes{hex:v.iter().map(|b|format!("{b:02x}")).collect()}
    }
    let mut root=equal_origin_route_root(local_hash);
    let f=&mut root.functions[0];
    let sw=f.blocks.iter_mut().find(|b|b.id=="switch_active").unwrap();
    // P1 still hands over to its ordinary specialist. Only P2 tries A first.
    sw.terminator=Terminator::Branch{
        condition:17101,if_true:"equal_local_read".into(),true_args:vec![],
        if_false:"equal_remote_switch".into(),false_args:vec![700]
    };
    // No investigator route object is consulted; remove the unreachable
    // route reader and gates instead of leaking another hidden permission.
    f.blocks.retain(|b|!matches!(b.id.as_str(),
        "equal_route_read"|"equal_route_reply"|
        "equal_route_decode"|"equal_route_remote_gate"));
    let exec_reply=f.blocks.iter_mut().find(|b|
        b.id=="equal_local_exec_reply").unwrap();
    exec_reply.terminator=Terminator::Branch{
        condition:17304,if_true:"equal_local_pack".into(),
        true_args:vec![17300],if_false:"availability_missing_gate".into(),
        false_args:vec![17300]
    };
    let p2_hash=s45_p2_scoped_result_wait().content_hash().unwrap();
    f.blocks.push(Block{
        id:"availability_missing_gate".into(),
        params:vec![Param{id:17500,ty:Type::Bytes}],
        instructions:vec![
            Instruction{id:17501,op:lit(b"code")},
            Instruction{id:17502,op:Op::JsonFieldString(17500,17501)},
            Instruction{id:17503,op:lit(b"NotFound")},
            Instruction{id:17504,op:Op::EqBytes(17502,17503)},
            Instruction{id:17505,op:lit(p2_hash.as_bytes())}
        ],
        terminator:Terminator::Branch{
            condition:17504,if_true:"equal_remote_switch".into(),
            true_args:vec![17505],if_false:"availability_gas_gate".into(),
            false_args:vec![17500]
        }
    });
    // The native red run #44998024761 proved that this real EXEC can
    // debit 85 A gas and return Gas at step 41, just before the epoch
    // replenishment. Gas means temporary budget pressure, NOT a
    // missing algorithm. Keep the same local route and retry at most 8
    // times; each attempt remains an ordinary paid world Exec action.
    // In particular, Gas is NEVER treated as NotFound/remote consent.
    f.blocks.push(Block{
        id:"availability_gas_gate".into(),
        params:vec![Param{id:17510,ty:Type::Bytes}],
        instructions:vec![
            Instruction{id:17511,op:lit(b"code")},
            Instruction{id:17512,op:Op::JsonFieldString(17510,17511)},
            Instruction{id:17513,op:lit(b"Gas")},
            Instruction{id:17514,op:Op::EqBytes(17512,17513)}
        ],
        terminator:Terminator::Branch{
            condition:17514,if_true:"availability_exec_retry".into(),
            true_args:vec![],if_false:"failed".into(),false_args:vec![]
        }
    });
    f.blocks.push(Block{
        id:"availability_exec_retry".into(),params:vec![],instructions:vec![],
        terminator:Terminator::RepeatBounded{
            limit:8,body:"equal_local_read".into(),body_args:vec![],
            done:"failed".into(),done_args:vec![]
        }
    });
    root.validate().expect("finite typed VM feedback-routed root");
    root
}

/// Two bounded real-world runs: local executable genuinely present or
/// genuinely absent at paid genesis. They are NOT equal-initial-capability
/// cost controls; they test an organism-controlled branch on actual EXEC
/// feedback (Ok vs NotFound), not researcher selection of L/R.
/// CI must inspect both full task results; safety green alone is not enough.
#[test]
#[ignore = "charged VM choice from actual local EXEC availability; expensive"]
fn p2_feedback_route_from_real_exec_availability(){
    let condition=std::env::var("OCEE_P2_AVAILABILITY").expect("availability");
    assert!(condition=="PRESENT"||condition=="ABSENT");
    let local=local_fold4_vm_program();
    let local_hash=local.content_hash().unwrap();
    let root=p2_feedback_availability_route_root(&local_hash);
    let root_hash=root.content_hash().unwrap();
    let p2=s45_p2_scoped_result_wait();
    let p2_hash=p2.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(8,2027,12_000,p2,root);
    if condition=="PRESENT"{
        let r=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{local_hash}"),
            expected_version:None,bytes:local.canonical_bytes()
        }));
        assert_eq!(r.code,Code::Ok);
    }
    let genesis=w.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let prehash=w.state_hash();
    assert_eq!(prehash,World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    let mut gas_refusals=0u64;
    let mut invalid=0u64;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        if r.code==Code::Gas{gas_refusals+=1;}
        if matches!(r.code,Code::InvalidCode|Code::Denied){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref().is_some_and(|c|c.finished);
    let success=finished&&host.last_return==Some(Value::U32(1));
    let a_vm=w.audit.iter().map(|r|r.result.vm_gas_charged).sum::<u64>();
    let a_actions=w.audit.iter().map(|r|r.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    let mut exec_ok=0usize;
    let mut exec_not_found=0usize;
    let mut c_sends=0usize;
    let mut remote_ids=Vec::<u64>::new();
    let mut result_ids=Vec::<u64>::new();
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    for r in &w.audit{
        match &r.action{
            Some(Action::Exec{program_hash,input,..}) if program_hash==&local_hash=>{
                assert_eq!(input.len(),16);
                if r.result.code==Code::Ok{exec_ok+=1;}
                if r.result.code==Code::NotFound{exec_not_found+=1;}
            },
            Some(Action::Send{destination:NodeId::C,bytes,..})
                if r.result.code==Code::Ok=>{
                c_sends+=1;
                if let Ok(frame)=decoder.decode(bytes){
                    let id=u64::from_le_bytes(r.result.payload[..8].try_into().unwrap());
                    match frame.operation{
                        Operation::RemoteEval if frame.body.len()==18=>
                            remote_ids.push(id),
                        Operation::Result=>result_ids.push(id),
                        _=>()
                    }
                }
            },
            _=>()
        }
    }
    let confirmed=w.audit.iter().filter_map(|r|match &r.action{
        Some(Action::ConfirmResponseFor{request_id,..})
            if r.result.code==Code::Ok=>Some(*request_id),
        _=>None
    }).collect::<Vec<_>>();
    let bytes=&w.state.objects.get(REMOTE_VALUES)
        .expect("common root generated actual fourth operand").bytes;
    assert_eq!(bytes.len(),16);
    let values=bytes.chunks_exact(4).map(|c|
        u32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>();
    let expected=fold32(&values).unwrap();
    let observed=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|o|u32::from_le_bytes(o.bytes[..4].try_into().unwrap()));
    println!(
        "P2_FEEDBACK_ROUTE condition={condition} success={success} \\
         genesis_A={genesis} prehash={prehash} root_hash={root_hash} \\
         p2_hash={p2_hash} local_hash={local_hash} A_vm={a_vm} \\
         A_actions={a_actions} A_total={} B={b} C_sends={c_sends} \\
         exec_ok={exec_ok} exec_not_found={exec_not_found} \\
         remote_ids={remote_ids:?} result_ids={result_ids:?} \\
         confirmed_ids={confirmed:?} gas_refusals={gas_refusals} \\
         step={} observed={observed:?} expected={expected}",
        a_vm+a_actions,w.state.step);
    if !success {
        println!("P2_FEEDBACK_FAILURE_DIAG condition={condition} objects={} \
            remote_values_present={} root_hash={root_hash}",
            w.state.objects.len(),w.state.objects.contains_key(REMOTE_VALUES));
        for r in w.audit.iter().rev().filter(|r|r.action.is_some()).take(18){
            let action=format!("{:?}",r.action);
            println!("P2_FEEDBACK_AUDIT step={} code={:?} gas={} vm_gas={} action={}",
                r.step,r.result.code,r.result.gas_charged,
                r.result.vm_gas_charged,action.chars().take(180).collect::<String>());
        }
    }
    assert_eq!(invalid,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(success,"actual full task, not just safe failure, must complete");
    assert_eq!(observed,Some(expected));
    if condition=="PRESENT"{
        assert_eq!(exec_ok,1);
        assert_eq!(exec_not_found,0);
        assert_eq!(c_sends,0);
        assert_eq!(b,0);
        assert!(confirmed.is_empty());
    }else{
        assert_eq!(exec_ok,0);
        assert_eq!(exec_not_found,1);
        assert_eq!(b,24);
        assert!(c_sends>0);
        assert_eq!(remote_ids.len(),1);
        assert!(confirmed.contains(&remote_ids[0]));
        assert!(result_ids.iter().any(|id|confirmed.contains(id)));
        assert_eq!(w.state.service_events.iter()
            .filter(|e|e.b_gas_charged>0).count(),1);
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// All arms share this content-identical active root. The RESEARCHER
/// supplies a candidate hash as paid private data; the ACTOR reads it,
/// then executes that content-addressed program on self-created operands.
fn p2_semantic_ok_registry_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(x:&[u8])->Op{
        Op::ConstBytes{hex:x.iter().map(|v|format!("{v:02x}")).collect()}
    }
    let honest_hash=local_fold4_vm_program().content_hash().unwrap();
    let mut root=p2_feedback_availability_route_root(&honest_hash);
    let f=&mut root.functions[0];
    let local=f.blocks.iter_mut().find(|b|b.id=="equal_local_exec").unwrap();
    local.instructions=vec![
        Instruction{id:18001,op:lit(b"/self/work/local-fold-candidate")}
    ];
    local.terminator=Terminator::Yield{
        call:WorldCall::Read,args:vec![18001],
        next:"semantic_candidate_status".into(),next_args:vec![17220]
    };
    f.blocks.push(Block{
        id:"semantic_candidate_status".into(),
        params:vec![
            Param{id:17220,ty:Type::Bytes},Param{id:18020,ty:Type::Bytes}
        ],
        instructions:vec![
            Instruction{id:18021,op:lit(b"code")},
            Instruction{id:18022,op:Op::JsonFieldString(18020,18021)},
            Instruction{id:18023,op:lit(b"Ok")},
            Instruction{id:18024,op:Op::EqBytes(18022,18023)}
        ],
        terminator:Terminator::Branch{
            condition:18024,if_true:"semantic_candidate_exec".into(),
            true_args:vec![17220,18020],if_false:"failed".into(),
            false_args:vec![]
        }
    });
    f.blocks.push(Block{
        id:"semantic_candidate_exec".into(),
        params:vec![
            Param{id:17220,ty:Type::Bytes},Param{id:18020,ty:Type::Bytes}
        ],
        instructions:vec![
            Instruction{id:18031,op:lit(b"payload")},
            Instruction{id:18032,op:Op::JsonFieldBytes(18020,18031)},
            Instruction{id:18033,op:lit(b"payload")},
            Instruction{id:18034,op:Op::JsonFieldBytes(17220,18033)}
        ],
        terminator:Terminator::Yield{
            call:WorldCall::Exec,args:vec![18032,18034],
            next:"equal_local_exec_reply".into(),next_args:vec![]
        }
    });
    root.validate().expect("semantic negative-control root must validate");
    root
}

/// Canonical executable that returns real FOLD32 plus one, still U32.
/// A false-success result must NOT be manufactured as host fake feedback.
fn corrupted_fold4_ok()->Program{
    use ocee_organism_vm::{Instruction,Op,Terminator};
    let mut p=local_fold4_vm_program();
    let b=&mut p.functions[0].blocks[0];
    let original=match b.terminator{
        Terminator::Return{value}=>value,_=>panic!("unexpected FOLD32 shape")
    };
    let id=b.instructions.iter().map(|i|i.id).max().unwrap()+1;
    b.instructions.push(Instruction{id,op:Op::ConstU32(1)});
    b.instructions.push(Instruction{id:id+1,op:Op::Add(original,id)});
    b.terminator=Terminator::Return{value:id+1};
    p.validate().expect("wrong algorithm must be well-typed");
    p
}

/// The CORRUPT arm passes when genuine, charged, semantically WRONG
/// output was accepted by the old active root as successful.
#[test]
#[ignore = "explicit three-arm charged semantic-Ok false-success control"]
fn p2_semantic_ok_counterexample_native(){
    let arm=std::env::var("OCEE_P2_SEMANTIC_ARM").expect("matrix arm");
    assert!(["VALID","ABSENT","CORRUPT"].contains(&arm.as_str()));
    let honest=local_fold4_vm_program();
    let altered=corrupted_fold4_ok();
    let honest_hash=honest.content_hash().unwrap();
    let altered_hash=altered.content_hash().unwrap();
    assert_ne!(honest_hash,altered_hash);
    let candidate=if arm=="CORRUPT"{&altered}else{&honest};
    let hash=candidate.content_hash().unwrap();
    let root=p2_semantic_ok_registry_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root);
    if arm!="ABSENT"{
        let r=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:candidate.canonical_bytes()
        }));
        assert_eq!(r.code,Code::Ok);
    }
    let r=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/work/local-fold-candidate".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    }));
    assert_eq!(r.code,Code::Ok);
    let genesis=w.audit.iter().map(|x|
        x.result.gas_charged+x.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let prehash=w.state_hash();
    assert_eq!(prehash,World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    let mut invalid=0u64;
    let mut gas_refusals=0u64;
    for _ in 0..14_000{
        let result=w.tick_autonomous(ORGANISM_ID);
        if result.code==Code::Gas{gas_refusals+=1;}
        if matches!(result.code,Code::InvalidCode|Code::Denied){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let claimed=host.vm_continuation.as_ref()
        .is_some_and(|c|c.finished)&&host.last_return==Some(Value::U32(1));
    let actual_bytes=&w.state.objects.get(REMOTE_VALUES)
        .expect("root-generated operands").bytes;
    assert_eq!(actual_bytes.len(),16);
    let values=actual_bytes.chunks_exact(4).map(|p|
        u32::from_le_bytes(p.try_into().unwrap())).collect::<Vec<_>>();
    let expected=fold32(&values).unwrap();
    let output=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|obj|{
            assert_eq!(obj.bytes.len(),4);
            u32::from_le_bytes(obj.bytes[..4].try_into().unwrap())
        });
    let a_vm=w.audit.iter().map(|x|x.result.vm_gas_charged).sum::<u64>();
    let a_actions=w.audit.iter().map(|x|x.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    let mut exec_ok=Vec::<u32>::new();
    let mut missing=0usize;
    let mut c_send=0usize;
    let mut remote=Vec::<u64>::new();
    let mut result_ids=Vec::<u64>::new();
    let decoder=&w.state.protocol_service.as_ref().unwrap().protocol;
    for item in &w.audit{
        match &item.action{
            Some(Action::Exec{program_hash,input,..}) if program_hash==&hash=>{
                assert_eq!(input.as_slice(),actual_bytes.as_slice());
                if item.result.code==Code::Ok{
                    let Value::U32(n)=serde_json::from_slice::<Value>(
                        &item.result.payload).unwrap() else{
                            panic!("the pure program must return typed U32")
                        };
                    exec_ok.push(n);
                }
                if item.result.code==Code::NotFound{missing+=1;}
            },
            Some(Action::Send{destination:NodeId::C,bytes,..})
                if item.result.code==Code::Ok=>{
                c_send+=1;
                if let Ok(frame)=decoder.decode(bytes){
                    let id=u64::from_le_bytes(
                        item.result.payload[..8].try_into().unwrap());
                    match frame.operation{
                        Operation::RemoteEval if frame.body.len()==18=>remote.push(id),
                        Operation::Result=>result_ids.push(id),_=>()
                    }
                }
            },
            _=>()
        }
    }
    let confirmed=w.audit.iter().filter_map(|x|match &x.action{
        Some(Action::ConfirmResponseFor{request_id,..})
            if x.result.code==Code::Ok=>Some(*request_id),_=>None
    }).collect::<Vec<_>>();
    let wrong_claim=claimed&&output!=Some(expected);
    println!(
        "P2_SEMANTIC_OK arm={arm} claimed={claimed} correct={} \\
         wrong_claim={wrong_claim} root={root_hash} hash={hash} \\
         honest={honest_hash} corrupt={altered_hash} prehash={prehash} \\
         genesis_A={genesis} A_vm={a_vm} A_actions={a_actions} A_total={} \\
         B={b} C_sends={c_send} exec_ok={exec_ok:?} missing={missing} \\
         remote={remote:?} results={result_ids:?} confirmed={confirmed:?} \\
         Gas_refusals={gas_refusals} step={} expected={expected} output={output:?}",
        output==Some(expected),a_vm+a_actions,w.state.step);
    assert_eq!(invalid,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(claimed,"must actually finish and claim U32(1)");
    match arm.as_str(){
        "VALID"=>{
            assert_eq!(exec_ok,vec![expected]);
            assert_eq!(missing,0);
            assert_eq!(output,Some(expected));
            assert!(!wrong_claim);
            assert_eq!(b,0);
            assert_eq!(c_send,0);
        },
        "ABSENT"=>{
            assert!(exec_ok.is_empty());
            assert_eq!(missing,1);
            assert_eq!(output,Some(expected));
            assert!(!wrong_claim);
            assert_eq!(b,24);
            assert_eq!(remote.len(),1);
            assert!(confirmed.contains(&remote[0]));
            assert!(result_ids.iter().any(|id|confirmed.contains(id)));
            assert_eq!(w.state.service_events.iter().filter(|e|
                e.b_gas_charged>0).count(),1);
        },
        "CORRUPT"=>{
            let wrong=expected.wrapping_add(1);
            assert_eq!(exec_ok,vec![wrong]);
            assert_eq!(missing,0);
            assert_eq!(output,Some(wrong));
            assert!(wrong_claim,
                "a green test means the negative counterexample IS real");
            assert_eq!(b,0);
            assert_eq!(c_send,0);
        },
        _=>unreachable!()
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// A deliberately different VM FOLD32 implementation, not another hash
/// of the same bytecode. Horner's per-byte base-256 reconstruction is
/// replaced by per-byte powers of two; rotation is a repeated carry
/// through bit 31 instead of a final high/low-bit decomposition.
/// Neither algorithm has a privileged native FOLD32 instruction.
fn alternate_fold4_vm_program()->Program{
    use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Terminator,Type};
    let mut code=Vec::<Instruction>::new();
    let mut reg=1u32;
    let mut emit=|op:Op|{
        let id=reg;
        reg+=1;
        code.push(Instruction{id,op});
        id
    };
    let zero=emit(Op::ConstU32(0));
    let mut sum=zero;
    for word in 0u32..4{
        let mut value=zero;
        for byte in 0u32..4{
            let index=emit(Op::ConstU32(word*4+byte));
            let mut piece=emit(Op::ByteAt(0,index));
            for _ in 0..(byte*8){
                piece=emit(Op::Add(piece,piece));
            }
            value=emit(Op::Add(value,piece));
        }
        let mut rotated=value;
        for _ in 0..word{
            let msb=emit(Op::ConstU32(0x8000_0000));
            let carry=emit(Op::Div(rotated,msb));
            let doubled=emit(Op::Add(rotated,rotated));
            rotated=emit(Op::Add(doubled,carry));
        }
        sum=emit(Op::Add(sum,rotated));
    }
    let p=Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],
            returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),
                params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:code,
                terminator:Terminator::Return{value:sum}
            }]
        }]
    };
    p.validate().expect("distinct pure VM algorithm");
    p
}

/// Adversarial common-mode error, preserving the independent alternate
/// calculation graph but appending the same erroneous +1 semantics.
fn alternate_corrupted_fold4()->Program{
    use ocee_organism_vm::{Instruction,Op,Terminator};
    let mut p=alternate_fold4_vm_program();
    let block=&mut p.functions[0].blocks[0];
    let value=match block.terminator{
        Terminator::Return{value}=>value,
        _=>panic!("alternate FOLD32 return changed")
    };
    let id=block.instructions.iter().map(|i|i.id).max().unwrap()+1;
    block.instructions.push(Instruction{id,op:Op::ConstU32(1)});
    block.instructions.push(Instruction{id:id+1,op:Op::Add(value,id)});
    block.terminator=Terminator::Return{value:id+1};
    p.validate().expect("wrong alternate must still validate");
    p
}

/// The root itself compares TWO actual isolated VM Exec replies. The
/// researcher's fold32 oracle is not available to any root instruction.
fn p2_metered_disagreement_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(bytes:&[u8])->Op{
        Op::ConstBytes{hex:bytes.iter().map(|x|format!("{x:02x}")).collect()}
    }
    fn param(id:u32)->Param{Param{id,ty:Type::Bytes}}
    fn inst(id:u32,op:Op)->Instruction{Instruction{id,op}}
    fn branch(c:u32,yes:&str,yes_args:Vec<u32>,no:&str,no_args:Vec<u32>)->Terminator{
        Terminator::Branch{
            condition:c,if_true:yes.into(),true_args:yes_args,
            if_false:no.into(),false_args:no_args
        }
    }
    let mut root=p2_semantic_ok_registry_root();
    let f=&mut root.functions[0];
    let first=f.blocks.iter_mut().find(|b|b.id=="equal_local_exec_reply").unwrap();
    // Keep its genuine Exec Ok decoding, change ONLY the acceptance gate.
    first.terminator=branch(17304,"dual_secondary_key_read",vec![17300],
        "failed",vec![]);
    f.blocks.extend([
        Block{
            id:"dual_secondary_key_read".into(),params:vec![param(19000)],
            instructions:vec![inst(19001,lit(b"/self/work/local-fold-verifier"))],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![19001],
                next:"dual_secondary_key_reply".into(),next_args:vec![19000]
            }
        },
        Block{
            id:"dual_secondary_key_reply".into(),
            params:vec![param(19000),param(19010)],
            instructions:vec![
                inst(19011,lit(b"code")),
                inst(19012,Op::JsonFieldString(19010,19011)),
                inst(19013,lit(b"Ok")),
                inst(19014,Op::EqBytes(19012,19013))
            ],
            terminator:branch(19014,"dual_secondary_input_read",
                vec![19000,19010],"failed",vec![])
        },
        Block{
            id:"dual_secondary_input_read".into(),
            params:vec![param(19000),param(19010)],
            instructions:vec![
                inst(19021,lit(b"payload")),
                inst(19022,Op::JsonFieldBytes(19010,19021)),
                inst(19023,lit(REMOTE_VALUES.as_bytes()))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![19023],
                next:"dual_secondary_input_reply".into(),
                next_args:vec![19000,19022]
            }
        },
        Block{
            id:"dual_secondary_input_reply".into(),
            params:vec![param(19000),param(19030),param(19031)],
            instructions:vec![
                inst(19032,lit(b"code")),
                inst(19033,Op::JsonFieldString(19031,19032)),
                inst(19034,lit(b"Ok")),
                inst(19035,Op::EqBytes(19033,19034))
            ],
            terminator:branch(19035,"dual_secondary_exec",
                vec![19000,19030,19031],"failed",vec![])
        },
        Block{
            id:"dual_secondary_exec".into(),
            params:vec![param(19000),param(19030),param(19031)],
            instructions:vec![
                inst(19041,lit(b"payload")),
                inst(19042,Op::JsonFieldBytes(19031,19041))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Exec,args:vec![19030,19042],
                next:"dual_secondary_reply".into(),next_args:vec![19000]
            }
        },
        Block{
            id:"dual_secondary_reply".into(),
            params:vec![param(19000),param(19050)],
            instructions:vec![
                inst(19051,lit(b"code")),
                inst(19052,Op::JsonFieldString(19050,19051)),
                inst(19053,lit(b"Ok")),
                inst(19054,Op::EqBytes(19052,19053))
            ],
            terminator:branch(19054,"dual_agreement",
                vec![19000,19050],"failed",vec![])
        },
        Block{
            id:"dual_agreement".into(),
            params:vec![param(19000),param(19060)],
            instructions:vec![
                inst(19061,lit(b"payload")),
                inst(19062,Op::JsonFieldBytes(19000,19061)),
                inst(19063,Op::JsonFieldBytes(19060,19061)),
                inst(19064,Op::EqBytes(19062,19063))
            ],
            // Only identical genuine typed VM payloads may reach the
            // old success artifact write. A conflicting/unknown result
            // must NOT do any blind remote retry or forge a success.
            terminator:branch(19064,"equal_local_pack",vec![19000],
                "failed",vec![])
        }
    ]);
    root.validate().expect("finite typed two-Exec comparison root");
    root
}

/// Frozen protocol: AGREE_VALID / DISAGREE / AGREE_WRONG / UNKNOWN.
/// Green DISAGREE and UNKNOWN mean a genuine two-stage observation
/// refused success. Green AGREE_WRONG means agreement is NOT correctness.
#[test]
#[ignore = "four charged native trajectories of organism-visible agreement"]
fn p2_metered_disagreement_native(){
    use ocee_organism_vm::run_isolated;
    let arm=std::env::var("OCEE_P2_AGREEMENT_ARM").expect("matrix arm");
    assert!(["AGREE_VALID","DISAGREE","AGREE_WRONG","UNKNOWN"]
        .contains(&arm.as_str()));
    let first_good=local_fold4_vm_program();
    let first_wrong=corrupted_fold4_ok();
    let second_good=alternate_fold4_vm_program();
    let second_wrong=alternate_corrupted_fold4();
    // Generic sanity checks are not an oracle granted to the organism.
    for input in [
        [0u32,0,0,0],
        [u32::MAX,u32::MAX,u32::MAX,u32::MAX],
        [0x8000_0000,0x7fff_ffff,0x8000_0001,1]
    ]{
        let bytes=input.iter().flat_map(|v|v.to_le_bytes()).collect();
        let (observed,_)=run_isolated(&second_good,bytes,4000)
            .expect("alternate code must run on representative U32s");
        assert_eq!(observed,Value::U32(fold32(&input).unwrap()));
    }
    let primary=if arm=="AGREE_WRONG"{&first_wrong}else{&first_good};
    let verifier=if arm=="AGREE_VALID"||arm=="UNKNOWN"{
        &second_good
    }else{&second_wrong};
    let primary_hash=primary.content_hash().unwrap();
    let verifier_hash=verifier.content_hash().unwrap();
    assert_ne!(primary_hash,verifier_hash,
        "both candidates need distinct canonical code");
    let root=p2_metered_disagreement_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root);
    for candidate in [Some(primary),if arm=="UNKNOWN"{None}else{Some(verifier)}]
        .into_iter().flatten(){
        let hash=candidate.content_hash().unwrap();
        let r=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:candidate.canonical_bytes()
        }));
        assert_eq!(r.code,Code::Ok,"all installed candidates are paid");
    }
    for (key,hash) in [
        ("/self/work/local-fold-candidate",&primary_hash),
        ("/self/work/local-fold-verifier",&verifier_hash)
    ]{
        let r=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:key.into(),
            expected_version:None,bytes:hash.as_bytes().to_vec()
        }));
        assert_eq!(r.code,Code::Ok,"both registry reads must be paid");
    }
    let genesis=w.audit.iter().map(|a|
        a.result.gas_charged+a.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let prehash=w.state_hash();
    assert_eq!(prehash,World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    let mut invalid=0usize;
    let mut gas_refusals=0usize;
    for _ in 0..14_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        if r.code==Code::Gas{gas_refusals+=1;}
        if matches!(r.code,Code::InvalidCode|Code::Denied){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref().is_some_and(|c|c.finished);
    let returned=host.last_return.clone();
    let claimed_success=finished && returned==Some(Value::U32(1));
    let raw=&w.state.objects.get(REMOTE_VALUES)
        .expect("world must generate the true four operands").bytes;
    assert_eq!(raw.len(),16);
    let inputs=raw.chunks_exact(4).map(|x|
        u32::from_le_bytes(x.try_into().unwrap())).collect::<Vec<_>>();
    let correct=fold32(&inputs).unwrap();
    let saved=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|obj|{
            assert_eq!(obj.bytes.len(),4);
            u32::from_le_bytes(obj.bytes[..4].try_into().unwrap())
        });
    let mut observed=[Vec::<(Code,Option<u32>)>::new(),
        Vec::<(Code,Option<u32>)>::new()];
    let mut c_sends=0usize;
    for a in &w.audit{
        if let Some(Action::Exec{program_hash,input,..})=&a.action{
            if program_hash==&primary_hash||program_hash==&verifier_hash{
                assert_eq!(input.as_slice(),raw.as_slice(),
                    "each real Exec must use the actual common operands");
                let val=if a.result.code==Code::Ok{
                    let Value::U32(v)=serde_json::from_slice::<Value>(
                        &a.result.payload).unwrap() else{
                        panic!("only typed U32 results can be accepted")
                    };
                    Some(v)
                }else{None};
                let slot=if program_hash==&primary_hash{0}else{1};
                observed[slot].push((a.result.code,val));
            }
        }
        if matches!(&a.action,Some(Action::Send{destination:NodeId::C,..}))
            && a.result.code==Code::Ok{c_sends+=1;}
    }
    let a_vm=w.audit.iter().map(|a|a.result.vm_gas_charged).sum::<u64>();
    let a_actions=w.audit.iter().map(|a|a.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    println!(
        "P2_DISAGREEMENT arm={arm} root={root_hash} primary={primary_hash} \\
         verifier={verifier_hash} genesis_A={genesis} prehash={prehash} \\
         a_vm={a_vm} a_actions={a_actions} a_total={} B={b} \\
         C_sends={c_sends} first={:?} second={:?} finished={finished} \\
         returned={returned:?} claimed_success={claimed_success} \\
         correct={correct} stored={saved:?} invalid={invalid} \\
         gas_refusals={gas_refusals} world_step={}",
        a_vm+a_actions,observed[0],observed[1],w.state.step
    );
    assert_eq!(invalid,0);
    assert_eq!(b,0,"unknown or disagreement may not cause a B call");
    assert_eq!(c_sends,0);
    assert!(w.state.guarded_restarts.is_empty());
    // A first Exec may be paid to retry on insufficient A (Gas),
    // but there must be a single actual OK primary computation.
    assert_eq!(observed[0].iter()
        .filter(|(code,_)|*code==Code::Ok).count(),1);
    match arm.as_str(){
        "AGREE_VALID"=>{
            assert_eq!(observed[0].last(),Some(&(Code::Ok,Some(correct))));
            assert_eq!(observed[1],vec![(Code::Ok,Some(correct))]);
            assert!(claimed_success);
            assert_eq!(saved,Some(correct));
        },
        "DISAGREE"=>{
            assert_eq!(observed[0].last(),Some(&(Code::Ok,Some(correct))));
            assert_eq!(observed[1],
                vec![(Code::Ok,Some(correct.wrapping_add(1)))]);
            assert!(finished);
            assert!(!claimed_success);
            assert_eq!(returned,Some(Value::U32(0)));
            assert_eq!(saved,None,"no false success artifact on conflict");
        },
        "AGREE_WRONG"=>{
            let wrong=correct.wrapping_add(1);
            assert_eq!(observed[0].last(),Some(&(Code::Ok,Some(wrong))));
            assert_eq!(observed[1],vec![(Code::Ok,Some(wrong))]);
            assert!(claimed_success,
                "common-mode wrong agreement must be exposed");
            assert_eq!(saved,Some(wrong),"agreement is not truth");
        },
        "UNKNOWN"=>{
            assert_eq!(observed[0].last(),Some(&(Code::Ok,Some(correct))));
            assert_eq!(observed[1],vec![(Code::NotFound,None)]);
            assert!(finished);
            assert!(!claimed_success);
            assert_eq!(returned,Some(Value::U32(0)));
            assert_eq!(saved,None,
                "unavailable verifier is not a successful validation");
        },
        _=>unreachable!()
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// Public task-law anchor: FOLD32 of four zero words is exactly zero.
/// This is a RESEARCHER-AUTHORED domain constraint, not a newly learned
/// theorem or a world API that leaks the unknown job's true answer.
/// The active root really calls its own installed tool on zero bytes,
/// pays A gas, and accepts only the genuine typed zero response.
fn p2_zero_anchor_witness_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(v:&[u8])->Op{
        Op::ConstBytes{hex:v.iter().map(|x|format!("{x:02x}")).collect()}
    }
    fn p(id:u32)->Param{Param{id,ty:Type::Bytes}}
    fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
    fn branch(
        cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>
    )->Terminator{
        Terminator::Branch{
            condition:cond,if_true:yes.into(),true_args:ya,
            if_false:no.into(),false_args:na
        }
    }
    let zero_response=serde_json::to_vec(&Value::U32(0)).unwrap();
    let mut root=p2_metered_disagreement_root();
    let f=&mut root.functions[0];
    let compare=f.blocks.iter_mut().find(|b|b.id=="dual_agreement")
        .expect("two real Exec results must already have been compared");
    compare.terminator=branch(19064,"zero_anchor_registry_read",
        vec![19000],"failed",vec![]);
    f.blocks.extend([
        Block{
            id:"zero_anchor_registry_read".into(),
            params:vec![p(19500)],
            instructions:vec![
                ins(19501,lit(b"/self/work/local-fold-candidate"))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![19501],
                next:"zero_anchor_registry_reply".into(),
                next_args:vec![19500]
            }
        },
        Block{
            id:"zero_anchor_registry_reply".into(),
            params:vec![p(19500),p(19510)],
            instructions:vec![
                ins(19511,lit(b"code")),
                ins(19512,Op::JsonFieldString(19510,19511)),
                ins(19513,lit(b"Ok")),
                ins(19514,Op::EqBytes(19512,19513))
            ],
            terminator:branch(19514,"zero_anchor_exec",
                vec![19500,19510],"failed",vec![])
        },
        Block{
            id:"zero_anchor_exec".into(),
            params:vec![p(19500),p(19520)],
            instructions:vec![
                ins(19521,lit(b"payload")),
                ins(19522,Op::JsonFieldBytes(19520,19521)),
                ins(19523,lit(&[0u8;16]))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Exec,args:vec![19522,19523],
                next:"zero_anchor_status".into(),next_args:vec![19500]
            }
        },
        Block{
            id:"zero_anchor_status".into(),
            params:vec![p(19500),p(19530)],
            instructions:vec![
                ins(19531,lit(b"code")),
                ins(19532,Op::JsonFieldString(19530,19531)),
                ins(19533,lit(b"Ok")),
                ins(19534,Op::EqBytes(19532,19533))
            ],
            terminator:branch(19534,"zero_anchor_value",
                vec![19500,19530],"failed",vec![])
        },
        Block{
            id:"zero_anchor_value".into(),
            params:vec![p(19500),p(19540)],
            instructions:vec![
                ins(19541,lit(b"payload")),
                ins(19542,Op::JsonFieldBytes(19540,19541)),
                ins(19543,lit(&zero_response)),
                ins(19544,Op::EqBytes(19542,19543))
            ],
            terminator:branch(19544,"equal_local_pack",
                vec![19500],"failed",vec![])
        }
    ]);
    // The S45 lineage retains blocks whose edges became unreachable
    // after the researcher changed the P2 handover to two local tools.
    // The VM has a HARD 128-block program limit, so remove ONLY blocks
    // not reachable from the function's original entry. This is a
    // semantics-preserving control-flow projection, not a raised limit.
    // Jump/Yield/Branch/Repeat edges are all visited explicitly.
    let mut reachable=std::collections::BTreeSet::<String>::new();
    let mut pending=vec![f.blocks[0].id.clone()];
    while let Some(id)=pending.pop(){
        if !reachable.insert(id.clone()){continue;}
        let block=f.blocks.iter().find(|candidate|candidate.id==id)
            .expect("every reachable target must exist");
        match &block.terminator{
            Terminator::Return{..}=>(),
            Terminator::Jump{target,..}=>pending.push(target.clone()),
            Terminator::Branch{if_true,if_false,..}=>{
                pending.push(if_true.clone());
                pending.push(if_false.clone());
            },
            Terminator::RepeatBounded{body,done,..} |
            Terminator::RepeatScoped{body,done,..}=>{
                pending.push(body.clone());
                pending.push(done.clone());
            },
            Terminator::Yield{next,..}=>pending.push(next.clone())
        }
    }
    let original_block_count=f.blocks.len();
    f.blocks.retain(|candidate|reachable.contains(&candidate.id));
    assert!(f.blocks.len()<original_block_count,
        "this corrective patch must really prune dead blocks");
    root.validate().expect("typed paid zero-anchor witness root");
    root
}

/// A deliberately adversarial shared fault that defeats a zero anchor:
/// output F(x)+F(x), hence F(0)=0 yet general nonzero results are wrong.
/// Two distinct program graphs receive the same semantic defect.
fn doubled_fold4(mut p:Program)->Program{
    use ocee_organism_vm::{Instruction,Op,Terminator};
    let block=&mut p.functions[0].blocks[0];
    let old=match block.terminator{
        Terminator::Return{value}=>value,
        _=>panic!("pure candidate must return a U32")
    };
    let next=block.instructions.iter().map(|i|i.id).max().unwrap()+1;
    block.instructions.push(Instruction{
        id:next,op:Op::Add(old,old)
    });
    block.terminator=Terminator::Return{value:next};
    p.validate().expect("typed zero-preserving wrong tool");
    p
}

/// Four arms, one active root: an honest success, detected common +1,
/// zero-preserving shared x2 failure, and the earlier disagreement guard.
/// The only expected answer in the ROOT is the public zero-input law.
/// In particular, no true job answer crosses from host into the VM.
#[test]
#[ignore = "four real paid native zero-anchor trajectories"]
fn p2_zero_anchor_witness_native(){
    let arm=std::env::var("OCEE_P2_ZERO_ANCHOR_ARM").expect("matrix arm");
    assert!(["VALID","COMMON_PLUS_ONE","COMMON_DOUBLE","DISAGREE"]
        .contains(&arm.as_str()));
    let first_good=local_fold4_vm_program();
    let second_good=alternate_fold4_vm_program();
    let first_plus=corrupted_fold4_ok();
    let second_plus=alternate_corrupted_fold4();
    let first_double=doubled_fold4(local_fold4_vm_program());
    let second_double=doubled_fold4(alternate_fold4_vm_program());
    let (primary,secondary)=match arm.as_str(){
        "VALID"=>(&first_good,&second_good),
        "COMMON_PLUS_ONE"=>(&first_plus,&second_plus),
        "COMMON_DOUBLE"=>(&first_double,&second_double),
        "DISAGREE"=>(&first_good,&second_plus),
        _=>unreachable!()
    };
    let primary_hash=primary.content_hash().unwrap();
    let secondary_hash=secondary.content_hash().unwrap();
    assert_ne!(primary_hash,secondary_hash);
    let root=p2_zero_anchor_witness_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root);
    for tool in [primary,secondary]{
        let hash=tool.content_hash().unwrap();
        let install=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:tool.canonical_bytes()
        }));
        assert_eq!(install.code,Code::Ok);
    }
    for (key,value) in [
        ("/self/work/local-fold-candidate",&primary_hash),
        ("/self/work/local-fold-verifier",&secondary_hash)
    ]{
        let write=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:key.into(),expected_version:None,
            bytes:value.as_bytes().to_vec()
        }));
        assert_eq!(write.code,Code::Ok);
    }
    let genesis=w.audit.iter().map(|a|
        a.result.gas_charged+a.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let init_hash=w.state_hash();
    assert_eq!(init_hash,World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash());
    let mut invalid=0usize;
    let mut gas_denied=0usize;
    for _ in 0..14_000{
        let action=w.tick_autonomous(ORGANISM_ID);
        if action.code==Code::Gas{gas_denied+=1;}
        if matches!(action.code,Code::InvalidCode|Code::Denied){
            invalid+=1;
        }
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let claimed=finished&&host.last_return==Some(Value::U32(1));
    let raw=&w.state.objects.get(REMOTE_VALUES)
        .expect("actual organism-created operands").bytes;
    assert_eq!(raw.len(),16);
    let inputs=raw.chunks_exact(4).map(|b|
        u32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
    let correct=fold32(&inputs).unwrap();
    assert_ne!(correct,0);
    let saved=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|obj|u32::from_le_bytes(obj.bytes[..4].try_into().unwrap()));
    let mut task_first=Vec::<u32>::new();
    let mut task_second=Vec::<u32>::new();
    let mut anchor_first=Vec::<(Code,Option<u32>,u64)>::new();
    let mut anchor_second=Vec::<(Code,Option<u32>,u64)>::new();
    let mut send_count=0usize;
    for audit in &w.audit{
        match &audit.action{
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&primary_hash ||
                   program_hash==&secondary_hash=>{
                let value=if audit.result.code==Code::Ok{
                    let Value::U32(n)=serde_json::from_slice::<Value>(
                        &audit.result.payload).unwrap() else{
                        panic!("only typed U32 payloads are allowed")
                    };
                    Some(n)
                }else{None};
                let target=program_hash==&primary_hash;
                if input.as_slice()==raw.as_slice(){
                    if let Some(n)=value{
                        if target{task_first.push(n)}
                        else{task_second.push(n)}
                    }
                }else if input.as_slice()==[0u8;16]{
                    if target{
                        anchor_first.push((audit.result.code,value,
                            audit.result.gas_charged));
                    }else{
                        anchor_second.push((audit.result.code,value,
                            audit.result.gas_charged));
                    }
                }else{
                    panic!("any other Exec input violates frozen protocol");
                }
            },
            Some(Action::Send{destination:NodeId::C,..})
                if audit.result.code==Code::Ok=>send_count+=1,
            _=>()
        }
    }
    let a_vm=w.audit.iter().map(|a|a.result.vm_gas_charged).sum::<u64>();
    let a_actions=w.audit.iter().map(|a|a.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    println!(
        "P2_ZERO_ANCHOR arm={arm} root={root_hash} primary={primary_hash} \\
         secondary={secondary_hash} init_hash={init_hash} \\
         genesis_A={genesis} A_vm={a_vm} A_actions={a_actions} \\
         A_total={} B={b} C_sends={send_count} \\
         first={task_first:?} second={task_second:?} \\
         witness_primary={anchor_first:?} witness_secondary={anchor_second:?} \\
         finished={finished} return={:?} claimed={claimed} \\
         actual_correct={correct} stored={saved:?} \\
         invalid={invalid} gas_denied={gas_denied} step={}",
        a_vm+a_actions,host.last_return,w.state.step
    );
    assert_eq!(invalid,0);
    assert_eq!(b,0);
    assert_eq!(send_count,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(finished);
    let correct_result=correct;
    let plus=correct.wrapping_add(1);
    let double=correct.wrapping_mul(2);
    match arm.as_str(){
        "VALID"=>{
            assert_eq!(task_first,vec![correct_result]);
            assert_eq!(task_second,vec![correct_result]);
            assert_eq!(anchor_first.iter().filter(|(c,v,_)|
                *c==Code::Ok&&*v==Some(0)).count(),1);
            assert!(anchor_second.is_empty());
            assert!(claimed);
            assert_eq!(saved,Some(correct_result));
        },
        "COMMON_PLUS_ONE"=>{
            assert_eq!(task_first,vec![plus]);
            assert_eq!(task_second,vec![plus]);
            assert_eq!(anchor_first.iter().filter(|(c,v,_)|
                *c==Code::Ok&&*v==Some(1)).count(),1);
            assert!(!claimed);
            assert_eq!(host.last_return,Some(Value::U32(0)));
            assert_eq!(saved,None);
        },
        "COMMON_DOUBLE"=>{
            assert_ne!(double,correct_result);
            assert_eq!(task_first,vec![double]);
            assert_eq!(task_second,vec![double]);
            assert_eq!(anchor_first.iter().filter(|(c,v,_)|
                *c==Code::Ok&&*v==Some(0)).count(),1);
            assert!(claimed,
                "zero-preserving shared fault must expose a false success");
            assert_eq!(saved,Some(double));
        },
        "DISAGREE"=>{
            assert_eq!(task_first,vec![correct_result]);
            assert_eq!(task_second,vec![plus]);
            assert!(anchor_first.is_empty()&&anchor_second.is_empty(),
                "disagreement must exit before extra witness");
            assert!(!claimed);
            assert_eq!(host.last_return,Some(Value::U32(0)));
            assert_eq!(saved,None);
        },
        _=>unreachable!()
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// Publicly derivable task-local relation:
/// for FOLD32, toggling the lowest bit of word0 changes its contribution
/// by +1 or -1 modulo 2^32. No target answer is supplied to the root.
/// Existing two real Exec calls and the paid zero anchor are prerequisites.
fn p2_task_metamorphic_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(v:&[u8])->Op{
        Op::ConstBytes{hex:v.iter().map(|n|format!("{n:02x}")).collect()}
    }
    fn b(id:u32)->Param{Param{id,ty:Type::Bytes}}
    fn u(id:u32)->Param{Param{id,ty:Type::U32}}
    fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
    fn br(c:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
        Terminator::Branch{
            condition:c,if_true:t.into(),true_args:ta,
            if_false:f.into(),false_args:fa
        }
    }
    let mut root=p2_zero_anchor_witness_root();
    let f=&mut root.functions[0];
    // Preserve the already-paid registry hash across the original zero
    // witness, rather than looking it up a second time.
    let zexec=f.blocks.iter_mut().find(|x|x.id=="zero_anchor_exec").unwrap();
    zexec.terminator=Terminator::Yield{
        call:WorldCall::Exec,args:vec![19522,19523],
        next:"zero_anchor_status".into(),next_args:vec![19500,19522]
    };
    let zstatus=f.blocks.iter_mut().find(|x|x.id=="zero_anchor_status").unwrap();
    zstatus.params=vec![b(19500),b(19600),b(19530)];
    zstatus.terminator=br(19534,"zero_anchor_value",
        vec![19500,19600,19530],"failed",vec![]);
    let zvalue=f.blocks.iter_mut().find(|x|x.id=="zero_anchor_value").unwrap();
    zvalue.params=vec![b(19500),b(19600),b(19540)];
    zvalue.terminator=br(19544,"meta_input_read",
        vec![19500,19600],"failed",vec![]);
    f.blocks.extend([
        Block{
            id:"meta_input_read".into(),params:vec![b(19610),b(19611)],
            instructions:vec![i(19612,lit(REMOTE_VALUES.as_bytes()))],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![19612],
                next:"meta_input_status".into(),next_args:vec![19610,19611]
            }
        },
        Block{
            id:"meta_input_status".into(),
            params:vec![b(19620),b(19621),b(19622)],
            instructions:vec![
                i(19623,lit(b"code")),
                i(19624,Op::JsonFieldString(19622,19623)),
                i(19625,lit(b"Ok")),
                i(19626,Op::EqBytes(19624,19625))
            ],
            terminator:br(19626,"meta_prepare",
                vec![19620,19621,19622],"failed",vec![])
        },
        Block{
            id:"meta_prepare".into(),
            params:vec![b(19700),b(19701),b(19702)],
            instructions:vec![
                i(19703,lit(b"payload")),
                i(19704,Op::JsonFieldBytes(19702,19703)),
                i(19705,Op::ConstU32(0)),
                i(19706,Op::ConstU32(1)),
                i(19707,Op::ByteAt(19704,19705)),
                i(19708,Op::Xor(19707,19706)),
                i(19709,Op::ReplaceByte(19704,19705,19708)),
                i(19710,Op::And(19707,19706)),
                i(19711,Op::Add(19710,19710)),
                i(19712,Op::Sub(19706,19711)),
                i(19713,Op::JsonFieldBytes(19700,19703)),
                i(19714,lit(b"U32")),
                i(19715,Op::JsonFieldU32(19713,19714)),
                i(19716,Op::Add(19715,19712))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Exec,args:vec![19701,19709],
                next:"meta_exec_status".into(),next_args:vec![19700,19716]
            }
        },
        Block{
            id:"meta_exec_status".into(),
            params:vec![b(19800),u(19801),b(19802)],
            instructions:vec![
                i(19803,lit(b"code")),
                i(19804,Op::JsonFieldString(19802,19803)),
                i(19805,lit(b"Ok")),
                i(19806,Op::EqBytes(19804,19805))
            ],
            terminator:br(19806,"meta_compare",
                vec![19800,19801,19802],"failed",vec![])
        },
        Block{
            id:"meta_compare".into(),
            params:vec![b(19810),u(19811),b(19812)],
            instructions:vec![
                i(19813,lit(b"payload")),
                i(19814,Op::JsonFieldBytes(19812,19813)),
                i(19815,lit(b"U32")),
                i(19816,Op::JsonFieldU32(19814,19815)),
                i(19817,Op::EqU32(19816,19811))
            ],
            terminator:br(19817,"equal_local_pack",vec![19810],
                "failed",vec![])
        }
    ]);
    // Preserve strict VM hard limits; all new blocks are reached only
    // after actual dual Exec and zero witness succeed.

    // The native first run #45099227328 rejected the inherited root at
    // MAX_BLOCKS=128 before any world task. Reduce only same-edge parser
    // blocks, keeping all FOUR world calls and the public ±1 relation.
    // A missing/non-Ok feedback without the expected typed payload
    // traps before any success artifact and is NOT accepted as Ok.
    let registry=f.blocks.iter_mut().find(|x|
        x.id=="zero_anchor_registry_reply").unwrap();
    registry.instructions=vec![
        i(19901,lit(b"payload")),
        i(19902,Op::JsonFieldBytes(19510,19901)),
        i(19903,lit(&[0u8;16]))
    ];
    registry.terminator=Terminator::Yield{
        call:WorldCall::Exec,args:vec![19902,19903],
        next:"zero_anchor_status".into(),next_args:vec![19500,19902]
    };
    f.blocks.retain(|x|x.id!="zero_anchor_exec");
    let zstatus=f.blocks.iter_mut().find(|x|
        x.id=="zero_anchor_status").unwrap();
    let zero_expected=serde_json::to_vec(&Value::U32(0)).unwrap();
    zstatus.instructions=vec![
        i(19904,lit(b"payload")),
        i(19905,Op::JsonFieldBytes(19530,19904)),
        i(19906,lit(&zero_expected)),
        i(19907,Op::EqBytes(19905,19906))
    ];
    zstatus.terminator=br(19907,"meta_input_read",
        vec![19500,19600],"failed",vec![]);
    f.blocks.retain(|x|x.id!="zero_anchor_value");
    let input_status=f.blocks.iter_mut().find(|x|
        x.id=="meta_input_status").unwrap();
    input_status.instructions=vec![
        i(19703,lit(b"payload")),
        i(19704,Op::JsonFieldBytes(19622,19703)),
        i(19705,Op::ConstU32(0)),
        i(19706,Op::ConstU32(1)),
        i(19707,Op::ByteAt(19704,19705)),
        i(19708,Op::Xor(19707,19706)),
        i(19709,Op::ReplaceByte(19704,19705,19708)),
        i(19710,Op::And(19707,19706)),
        i(19711,Op::Add(19710,19710)),
        i(19712,Op::Sub(19706,19711)),
        i(19713,Op::JsonFieldBytes(19620,19703)),
        i(19714,lit(b"U32")),
        i(19715,Op::JsonFieldU32(19713,19714)),
        i(19716,Op::Add(19715,19712))
    ];
    input_status.terminator=Terminator::Yield{
        call:WorldCall::Exec,args:vec![19621,19709],
        next:"meta_exec_status".into(),next_args:vec![19620,19716]
    };
    f.blocks.retain(|x|x.id!="meta_prepare");
    let morph_response=f.blocks.iter_mut().find(|x|
        x.id=="meta_exec_status").unwrap();
    morph_response.instructions=vec![
        i(19813,lit(b"payload")),
        i(19814,Op::JsonFieldBytes(19802,19813)),
        i(19815,lit(b"U32")),
        i(19816,Op::JsonFieldU32(19814,19815)),
        i(19817,Op::EqU32(19816,19801))
    ];
    morph_response.terminator=br(19817,
        "equal_local_pack",vec![19800],"failed",vec![]);
    f.blocks.retain(|x|x.id!="meta_compare");
    // The new root still has the same 128-block hard limit and
    // must have an entirely valid statically typed graph.

    // Native forensic Run #45099701115 established paid P1 Write at
    // step27 returned Code::Gas with only the initial 12k A budget.
    // Shorten identifiers of ONLY researcher-added P2 witness blocks
    // to pay for fewer program bytes during genesis. Every Jump/Branch/
    // Repeat/Yield edge is changed bijectively with its block ID;
    // neither actual computation nor task nor VM gas policy changes.
    // No stored continuation is carried across branches of this test.
    let short_names=[
        ("zero_anchor_registry_read","zr"),
        ("zero_anchor_registry_reply","zq"),
        ("zero_anchor_status","zs"),
        ("meta_input_read","mr"),
        ("meta_input_status","mi"),
        ("meta_exec_status","me"),
        ("dual_secondary_key_read","dr"),
        ("dual_secondary_key_reply","dq"),
        ("dual_secondary_input_read","di"),
        ("dual_secondary_input_reply","dj"),
        ("dual_secondary_exec","de"),
        ("dual_secondary_reply","df"),
        ("dual_agreement","dg")
    ];
    let compact=|name:&mut String|{
        for (old,new) in short_names{
            if name==old{*name=new.into();break;}
        }
    };
    for block in &mut f.blocks{
        compact(&mut block.id);
        match &mut block.terminator{
            Terminator::Return{..}=>(),
            Terminator::Jump{target,..}=>compact(target),
            Terminator::Branch{if_true,if_false,..}=>{
                compact(if_true);
                compact(if_false);
            },
            Terminator::RepeatBounded{body,done,..}|
            Terminator::RepeatScoped{body,done,..}=>{
                compact(body);
                compact(done);
            },
            Terminator::Yield{next,..}=>compact(next)
        }
    }

    println!("P2_METAMORPHIC_GRAPH_BLOCKS total={}",f.blocks.len());
    root.validate().expect("typed bounded active metamorphic root");
    root
}

/// A zero-preserving common bug that is also invariant to toggling byte0:
/// FOLD32(input) + sum(input[1..15]). The sum is zero for all-zero inputs,
/// and unchanged by the exact metamorphic mutation, but nonzero on the
/// frozen real task input. Two independently coded implementations carry
/// the same semantic defect.
fn invariant_byte_tail_fault(mut p:Program)->Program{
    use ocee_organism_vm::{Instruction,Op,Terminator};
    let block=&mut p.functions[0].blocks[0];
    let correct_reg=match block.terminator{
        Terminator::Return{value}=>value,
        _=>panic!("pure FOLD32 must return U32")
    };
    let mut id=block.instructions.iter().map(|i|i.id).max().unwrap()+1;
    let mut acc=id;
    block.instructions.push(Instruction{id,op:Op::ConstU32(0)});
    id+=1;
    for index in 1u32..16{
        let idx=id;
        block.instructions.push(Instruction{
            id:idx,op:Op::ConstU32(index)
        });
        id+=1;
        let byte=id;
        block.instructions.push(Instruction{
            id:byte,op:Op::ByteAt(0,idx)
        });
        id+=1;
        acc=id;
        let old=id-3;
        // At the first iteration the previous accumulator is the zero
        // literal immediately before this three-instruction suffix.
        block.instructions.push(Instruction{
            id:acc,op:Op::Add(old,byte)
        });
        id+=1;
    }
    block.instructions.push(Instruction{
        id,op:Op::Add(correct_reg,acc)
    });
    block.terminator=Terminator::Return{value:id};
    p.validate().expect("zero-preserving context-dependent wrong tool");
    p
}

/// Actual paid metamorphic Exec on ACTIVE organism-created task bytes.
/// Both root and all world-visible bytes are identical across four arms
/// except researcher-installed legitimate programs and paid registry.
#[test]
#[ignore = "four charged task-metamorphic native world trajectories"]
fn p2_task_metamorphic_native(){
    let arm=std::env::var("OCEE_P2_META_ARM").expect("matrix arm");
    assert!(["VALID","COMMON_DOUBLE","COMMON_PLUS_ONE","INVARIANT_FAULT"]
        .contains(&arm.as_str()));
    let a_good=local_fold4_vm_program();
    let b_good=alternate_fold4_vm_program();
    let a_plus=corrupted_fold4_ok();
    let b_plus=alternate_corrupted_fold4();
    let a_double=doubled_fold4(local_fold4_vm_program());
    let b_double=doubled_fold4(alternate_fold4_vm_program());
    let a_invariant=invariant_byte_tail_fault(local_fold4_vm_program());
    let b_invariant=invariant_byte_tail_fault(alternate_fold4_vm_program());
    let (primary,verifier)=match arm.as_str(){
        "VALID"=>(&a_good,&b_good),
        "COMMON_DOUBLE"=>(&a_double,&b_double),
        "COMMON_PLUS_ONE"=>(&a_plus,&b_plus),
        "INVARIANT_FAULT"=>(&a_invariant,&b_invariant),
        _=>unreachable!()
    };
    let primary_hash=primary.content_hash().unwrap();
    let verifier_hash=verifier.content_hash().unwrap();
    assert_ne!(primary_hash,verifier_hash);
    let root=p2_task_metamorphic_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root
    );
    for prog in [primary,verifier]{
        let hash=prog.content_hash().unwrap();
        let response=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
            bytes:prog.canonical_bytes(),expected_version:None
        }));
        assert_eq!(response.code,Code::Ok,
            "researcher-installed executable must pay its A cost");
    }
    for (key,value) in [
        ("/self/work/local-fold-candidate",&primary_hash),
        ("/self/work/local-fold-verifier",&verifier_hash)
    ]{
        let response=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:key.into(),
            bytes:value.as_bytes().to_vec(),expected_version:None
        }));
        assert_eq!(response.code,Code::Ok);
    }
    let genesis=w.audit.iter().map(|r|
        r.result.gas_charged+r.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let prehash=w.state_hash();
    assert_eq!(prehash,
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    let mut gas_denied=0usize;
    let mut invalid=0usize;
    for _ in 0..14_000{
        let response=w.tick_autonomous(ORGANISM_ID);
        if response.code==Code::Gas{gas_denied+=1;}
        if matches!(response.code,Code::Denied|Code::InvalidCode){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let organism=&w.state.organisms[&ORGANISM_ID];
    let finished=organism.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let last=organism.last_return.clone();
    let claimed=finished&&last==Some(Value::U32(1));
    if !w.state.objects.contains_key(REMOTE_VALUES){
        println!(
            "P2_META_EARLY_FAILURE arm={arm} finished={finished} \\
             last={last:?} step={} genesis_A={genesis} \\
             gas_denied={gas_denied} invalid={invalid} \\
             audit_count={} restarts={:?} continuation={:?}",
            w.state.step,w.audit.len(),w.state.guarded_restarts,
            w.state.organisms[&ORGANISM_ID].vm_continuation
                .as_ref().map(|c|(&c.block,&c.loop_counters)));
        for rec in w.audit.iter().rev().filter(|x|x.action.is_some()).take(30){
            println!(
                "P2_META_EARLY_AUDIT step={} code={:?} vm_gas={} A_gas={} action={:?}",
                rec.step,rec.result.code,rec.result.vm_gas_charged,
                rec.result.gas_charged,rec.action);
        }
        panic!("root did not produce P2 input; preserve genuine native failure");
    }
    let raw=&w.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(raw.len(),16);
    let mut transformed=raw.clone();
    transformed[0]^=1;
    let old0=raw[0];
    let delta=if old0&1==0{1u32}else{u32::MAX};
    let values=raw.chunks_exact(4).map(|x|
        u32::from_le_bytes(x.try_into().unwrap())).collect::<Vec<_>>();
    let correct=fold32(&values).unwrap();
    let mut new_values=values.clone();
    new_values[0]^=1;
    let transformed_true=fold32(&new_values).unwrap();
    assert_eq!(transformed_true,correct.wrapping_add(delta));
    let tail_bias=raw.iter().skip(1).map(|v|u32::from(*v)).sum::<u32>();
    assert!(tail_bias>0,"frozen adversarial invariant fault must be observable");
    let saved=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|x|{
            assert_eq!(x.bytes.len(),4);
            u32::from_le_bytes(x.bytes[..4].try_into().unwrap())
        });
    let mut primary_task=Vec::<(Code,Option<u32>)>::new();
    let mut verifier_task=Vec::<(Code,Option<u32>)>::new();
    let mut anchor=Vec::<(Code,Option<u32>,u64)>::new();
    let mut morph=Vec::<(Code,Option<u32>,u64)>::new();
    let mut sends=0usize;
    for event in &w.audit{
        match &event.action{
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&primary_hash ||
                    program_hash==&verifier_hash=>{
                let output=if event.result.code==Code::Ok{
                    let Value::U32(value)=serde_json::from_slice::<Value>(
                        &event.result.payload).unwrap() else{
                        panic!("all candidate results must be genuine typed U32")
                    };
                    Some(value)
                }else{None};
                if input.as_slice()==raw.as_slice(){
                    if program_hash==&primary_hash{
                        primary_task.push((event.result.code,output));
                    }else{
                        verifier_task.push((event.result.code,output));
                    }
                }else if input.as_slice()==[0u8;16]{
                    assert_eq!(program_hash,&primary_hash);
                    anchor.push((event.result.code,output,
                        event.result.gas_charged));
                }else if input.as_slice()==transformed.as_slice(){
                    assert_eq!(program_hash,&primary_hash);
                    morph.push((event.result.code,output,
                        event.result.gas_charged));
                }else{
                    panic!("world Exec must use only real, zero or flipped input")
                }
            },
            Some(Action::Send{destination:NodeId::C,..})
                if event.result.code==Code::Ok=>sends+=1,
            _=>()
        }
    }
    let total_vm=w.audit.iter().map(|r|r.result.vm_gas_charged).sum::<u64>();
    let total_actions=w.audit.iter().map(|r|r.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    println!(
        "P2_METAMORPHIC arm={arm} root={root_hash} \\
         primary={primary_hash} verifier={verifier_hash} prehash={prehash} \\
         genesis_A={genesis} A_vm={total_vm} A_actions={total_actions} \\
         A_total={} B={b} C_SEND={sends} old_bit={} delta={delta} \\
         first={primary_task:?} second={verifier_task:?} \\
         zero={anchor:?} morph={morph:?} expected={correct} \\
         expected_morph={transformed_true} tail_bias={tail_bias} \\
         returned={last:?} finished={finished} claimed={claimed} \\
         saved={saved:?} gas_refused={gas_denied} invalid={invalid} step={}",
        total_vm+total_actions,old0&1,w.state.step
    );
    assert_eq!(invalid,0);
    assert_eq!(b,0);
    assert_eq!(sends,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert!(finished);
    let result_primary=match arm.as_str(){
        "VALID"=>correct,
        "COMMON_DOUBLE"=>correct.wrapping_mul(2),
        "COMMON_PLUS_ONE"=>correct.wrapping_add(1),
        "INVARIANT_FAULT"=>correct.wrapping_add(tail_bias),
        _=>unreachable!()
    };
    assert_eq!(primary_task.iter().filter(|(c,v)|
        *c==Code::Ok&&*v==Some(result_primary)).count(),1);
    assert_eq!(verifier_task.iter().filter(|(c,v)|
        *c==Code::Ok&&*v==Some(result_primary)).count(),1);
    if arm=="COMMON_PLUS_ONE"{
        assert_eq!(anchor.iter().filter(|(c,v,_)|
            *c==Code::Ok&&*v==Some(1)).count(),1);
        assert!(morph.is_empty(),"zero failure must skip metamorphic work");
        assert!(!claimed&&saved.is_none()&&last==Some(Value::U32(0)));
    }else{
        assert_eq!(anchor.iter().filter(|(c,v,_)|
            *c==Code::Ok&&*v==Some(0)).count(),1);
        let expected_morphed=match arm.as_str(){
            "VALID"=>transformed_true,
            "COMMON_DOUBLE"=>transformed_true.wrapping_mul(2),
            "INVARIANT_FAULT"=>transformed_true.wrapping_add(tail_bias),
            _=>unreachable!()
        };
        assert_eq!(morph.iter().filter(|(c,v,_)|
            *c==Code::Ok&&*v==Some(expected_morphed)).count(),1,
            "metamorphic proof must be a genuine paid Exec");
        match arm.as_str(){
            "VALID"=>{
                assert!(claimed&&saved==Some(correct));
            },
            "COMMON_DOUBLE"=>{
                assert!(!claimed&&saved.is_none()&&last==Some(Value::U32(0)));
            },
            "INVARIANT_FAULT"=>{
                assert!(claimed && saved==Some(result_primary));
                assert_ne!(saved,Some(correct),
                    "frozen negative control must be a false success");
            },
            _=>unreachable!()
        }
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}


/// After genuine dual-tool agreement, zero witness, and byte0 relation,
/// make a second paid EXEC at *the original* byte1 xor 1 task input.
/// This is task-linked metamorphic evidence, not a privileged truth API.
fn p2_two_coordinate_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(v:&[u8])->Op{
        Op::ConstBytes{hex:v.iter().map(|x|format!("{x:02x}")).collect()}
    }
    fn bi(id:u32)->Param{Param{id,ty:Type::Bytes}}
    fn ui(id:u32)->Param{Param{id,ty:Type::U32}}
    fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
    let mut root=p2_task_metamorphic_root();
    let f=&mut root.functions[0];
    let input=f.blocks.iter_mut().find(|b|b.id=="mi")
        .expect("previous typed byte0 input block");
    let Terminator::Yield{call:_,args:_,next:_,next_args}=
        &mut input.terminator else{panic!("byte0 really Execs")};
    assert_eq!(next_args.as_slice(),&[19620,19716]);
    // Carry exactly the bytes we already paid to Read and candidate hash.
    // No host selection, no extra free or unmetered world Read.
    *next_args=vec![19620,19621,19704,19716];
    let reply=f.blocks.iter_mut().find(|b|b.id=="me")
        .expect("byte0 real Exec response");
    reply.params=vec![
        bi(19800),bi(20200),bi(20201),ui(19801),bi(19802)
    ];
    reply.terminator=Terminator::Branch{
        condition:19817,
        if_true:"x1".into(),true_args:vec![19800,20200,20201],
        if_false:"failed".into(),false_args:vec![]
    };
    // Strict VM program cap was 126 blocks; add ONLY these two blocks.
    // Flip byte1 low bit => word0 changes exactly +/-256 modulo 2^32.
    let mut instructions=vec![
        ins(20103,Op::ConstU32(1)),
        ins(20104,Op::ByteAt(20102,20103)),
        ins(20105,Op::Xor(20104,20103)),
        ins(20106,Op::ReplaceByte(20102,20103,20105)),
        ins(20107,Op::And(20104,20103)),
        ins(20108,Op::Add(20107,20107)),
        ins(20109,Op::Sub(20103,20108))
    ];
    let mut shifted=20109;
    for id in 20110..=20117{
        instructions.push(ins(id,Op::Add(shifted,shifted)));
        shifted=id;
    }
    assert_eq!(shifted,20117);
    instructions.extend([
        ins(20118,lit(b"payload")),
        ins(20119,Op::JsonFieldBytes(20100,20118)),
        ins(20120,lit(b"U32")),
        ins(20121,Op::JsonFieldU32(20119,20120)),
        ins(20122,Op::Add(20121,shifted))
    ]);
    f.blocks.push(Block{
        id:"x1".into(),
        params:vec![bi(20100),bi(20101),bi(20102)],
        instructions,
        terminator:Terminator::Yield{
            call:WorldCall::Exec,args:vec![20101,20106],
            next:"x2".into(),next_args:vec![20100,20122]
        }
    });
    f.blocks.push(Block{
        id:"x2".into(),
        params:vec![bi(20210),ui(20211),bi(20212)],
        instructions:vec![
            ins(20213,lit(b"payload")),
            ins(20214,Op::JsonFieldBytes(20212,20213)),
            ins(20215,lit(b"U32")),
            ins(20216,Op::JsonFieldU32(20214,20215)),
            ins(20217,Op::EqU32(20216,20211))
        ],
        terminator:Terminator::Branch{
            condition:20217,
            if_true:"equal_local_pack".into(),true_args:vec![20210],
            if_false:"failed".into(),false_args:vec![]
        }
    });
    assert_eq!(f.blocks.len(),128,"two real Exec check blocks within VM cap");
    root.validate().expect("strictly typed paid 2-coordinate root");
    root
}

/// Two distinct candidate implementations can share an error invariant
/// under BOTH byte0 and byte1 xor 1: add sum(input[2..15]) after FOLD32.
fn invariant_tail_from_byte2(p:Program)->Program{
    use ocee_organism_vm::{Instruction,Op,Terminator};
    let mut p=invariant_byte_tail_fault(p);
    let block=&mut p.functions[0].blocks[0];
    let ret=match block.terminator{
        Terminator::Return{value}=>value,
        _=>panic!("expected pure VM return")
    };
    let id=block.instructions.iter().map(|i|i.id).max().unwrap()+1;
    block.instructions.push(Instruction{id,op:Op::ConstU32(1)});
    block.instructions.push(Instruction{
        id:id+1,op:Op::ByteAt(0,id)
    });
    block.instructions.push(Instruction{
        id:id+2,op:Op::Sub(ret,id+1)
    });
    block.terminator=Terminator::Return{value:id+2};
    p.validate().expect("two-coordinate-invariant adversarial tool");
    p
}

/// Prespecified native four-arm evidence on the unchanged full paid S45
/// P1/P2 task. The separate host oracle is NEVER visible to active VM.
#[test]
#[ignore = "paid active organism two-coordinate metamorphic gate"]
fn p2_two_coordinate_native(){
    let arm=std::env::var("OCEE_P2_COORD_ARM").expect("matrix arm");
    assert!(["VALID","INVARIANT_FAULT","RESIDUAL_FAULT","COMMON_DOUBLE"]
        .contains(&arm.as_str()));
    let a_correct=local_fold4_vm_program();
    let b_correct=alternate_fold4_vm_program();
    let a_invariant=invariant_byte_tail_fault(local_fold4_vm_program());
    let b_invariant=invariant_byte_tail_fault(alternate_fold4_vm_program());
    let a_residual=invariant_tail_from_byte2(local_fold4_vm_program());
    let b_residual=invariant_tail_from_byte2(alternate_fold4_vm_program());
    let a_double=doubled_fold4(local_fold4_vm_program());
    let b_double=doubled_fold4(alternate_fold4_vm_program());
    let (primary,secondary)=match arm.as_str(){
        "VALID"=>(&a_correct,&b_correct),
        "INVARIANT_FAULT"=>(&a_invariant,&b_invariant),
        "RESIDUAL_FAULT"=>(&a_residual,&b_residual),
        "COMMON_DOUBLE"=>(&a_double,&b_double),
        _=>unreachable!()
    };
    let first_hash=primary.content_hash().unwrap();
    let second_hash=secondary.content_hash().unwrap();
    assert_ne!(first_hash,second_hash);
    let root=p2_two_coordinate_root();
    let root_hash=root.content_hash().unwrap();
    let mut world=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root
    );
    for candidate in [primary,secondary]{
        let hash=candidate.content_hash().unwrap();
        let installed=world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
            bytes:candidate.canonical_bytes(),expected_version:None
        }));
        assert_eq!(installed.code,Code::Ok,"candidate installation is paid");
    }
    for (path,hash) in [
        ("/self/work/local-fold-candidate",&first_hash),
        ("/self/work/local-fold-verifier",&second_hash)
    ]{
        let stored=world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:path.into(),
            bytes:hash.as_bytes().to_vec(),expected_version:None
        }));
        assert_eq!(stored.code,Code::Ok,"content hash registration is paid");
    }
    let genesis=world.audit.iter()
        .map(|x|x.result.gas_charged+x.result.vm_gas_charged).sum::<u64>();
    assert_eq!(genesis+world.state.nodes[&NodeId::A].gas_available,12_000);
    let before=world.state_hash();
    assert_eq!(before,
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    let mut refused=0usize;
    let mut invalid=0usize;
    for _ in 0..14_000{
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::Gas{refused+=1;}
        if matches!(result.code,Code::Denied|Code::InvalidCode){invalid+=1;}
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let organism=&world.state.organisms[&ORGANISM_ID];
    let finished=organism.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let returned=organism.last_return.clone();
    let claimed=finished&&returned==Some(Value::U32(1));
    if !world.state.objects.contains_key(REMOTE_VALUES){
        println!(
            "P2_COORD_EARLY arm={arm} finished={finished} ret={returned:?} \\
             step={} genesis={genesis} refusal={refused} invalid={invalid}",
            world.state.step
        );
        for x in world.audit.iter().rev().filter(|a|a.action.is_some()).take(8){
            println!(
                "P2_COORD_EARLY_AUDIT step={} code={:?} vm={} action_gas={} \\
                 action={:?}",
                x.step,x.result.code,x.result.vm_gas_charged,
                x.result.gas_charged,x.action.as_ref().map(|a|std::mem::discriminant(a))
            );
        }
        panic!("full P1->P2 task must generate the true work object");
    }
    let raw=&world.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(raw.len(),16);
    let mut first_mutated=raw.clone();
    first_mutated[0]^=1;
    let mut second_mutated=raw.clone();
    second_mutated[1]^=1;
    assert_ne!(first_mutated,second_mutated);
    let input=raw.chunks_exact(4).map(|x|
        u32::from_le_bytes(x.try_into().unwrap())).collect::<Vec<_>>();
    let true_value=fold32(&input).unwrap();
    let mut changed0=input.clone();
    changed0[0]^=1;
    let mut changed1=input.clone();
    changed1[0]^=256;
    let true0=fold32(&changed0).unwrap();
    let true1=fold32(&changed1).unwrap();
    let delta0=if raw[0]&1==0{1_u32}else{u32::MAX};
    let delta1=if raw[1]&1==0{256_u32}else{0xffff_ff00_u32};
    assert_eq!(true0,true_value.wrapping_add(delta0));
    assert_eq!(true1,true_value.wrapping_add(delta1));
    let bias1=raw.iter().skip(1).map(|v|u32::from(*v)).sum::<u32>();
    let bias2=raw.iter().skip(2).map(|v|u32::from(*v)).sum::<u32>();
    assert!(bias1>bias2&&bias2>0,"frozen shared error needs nonzero terms");
    let stored=world.state.objects.get("/self/results/p2-job0/value_le")
        .map(|o|{
            assert_eq!(o.bytes.len(),4);
            u32::from_le_bytes(o.bytes[..4].try_into().unwrap())
        });
    let mut first=Vec::<u32>::new();
    let mut second=Vec::<u32>::new();
    let mut zero=Vec::<(Code,Option<u32>,u64)>::new();
    let mut mutation0=Vec::<(Code,Option<u32>,u64)>::new();
    let mut mutation1=Vec::<(Code,Option<u32>,u64)>::new();
    let mut c_send=0usize;
    for record in &world.audit{
        match &record.action{
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&first_hash||program_hash==&second_hash=>{
                let output=if record.result.code==Code::Ok{
                    let Value::U32(n)=serde_json::from_slice::<Value>(
                        &record.result.payload).unwrap() else{
                        panic!("typed FOLD32 tool output only")
                    };
                    Some(n)
                }else{None};
                if input.as_slice()==raw.as_slice(){
                    if program_hash==&first_hash{
                        if let Some(n)=output{first.push(n)}
                    }else if let Some(n)=output{second.push(n)}
                }else if input.as_slice()==[0u8;16]{
                    assert_eq!(program_hash,&first_hash);
                    zero.push((record.result.code,output,record.result.gas_charged));
                }else if input.as_slice()==first_mutated.as_slice(){
                    assert_eq!(program_hash,&first_hash);
                    mutation0.push((record.result.code,output,record.result.gas_charged));
                }else if input.as_slice()==second_mutated.as_slice(){
                    assert_eq!(program_hash,&first_hash);
                    mutation1.push((record.result.code,output,record.result.gas_charged));
                }else{
                    panic!("all targeted Exec inputs must be genuine or active-VM mutated");
                }
            },
            Some(Action::Send{destination:NodeId::C,..})
                if record.result.code==Code::Ok=>c_send+=1,
            _=>()
        }
    }
    let actual_expected=match arm.as_str(){
        "VALID"=>true_value,
        "INVARIANT_FAULT"=>true_value.wrapping_add(bias1),
        "RESIDUAL_FAULT"=>true_value.wrapping_add(bias2),
        "COMMON_DOUBLE"=>true_value.wrapping_mul(2),
        _=>unreachable!()
    };
    let first_mut_expected=match arm.as_str(){
        "VALID"=>true0,
        "INVARIANT_FAULT"=>true0.wrapping_add(bias1),
        "RESIDUAL_FAULT"=>true0.wrapping_add(bias2),
        "COMMON_DOUBLE"=>true0.wrapping_mul(2),
        _=>unreachable!()
    };
    let second_mut_expected=match arm.as_str(){
        "VALID"=>true1,
        "INVARIANT_FAULT"=>true1.wrapping_add(
            bias1.wrapping_add(if raw[1]&1==0{1}else{u32::MAX})),
        "RESIDUAL_FAULT"=>true1.wrapping_add(bias2),
        "COMMON_DOUBLE"=>true1.wrapping_mul(2),
        _=>unreachable!()
    };
    let vm_a=world.audit.iter().map(|x|x.result.vm_gas_charged).sum::<u64>();
    let actions_a=world.audit.iter().map(|x|x.result.gas_charged).sum::<u64>();
    let b=b_total(&world);
    println!(
        "P2_TWO_COORD arm={arm} root={root_hash} primary={first_hash} \\
         secondary={second_hash} prehash={before} init_A={genesis} \\
         vm_A={vm_a} actions_A={actions_a} total_A={} B={b} C={c_send} \\
         task_first={first:?} task_second={second:?} zero={zero:?} \\
         changed_byte0={mutation0:?} changed_byte1={mutation1:?} \\
         real_original={true_value} delta0={delta0} delta1={delta1} \\
         bias1={bias1} bias2={bias2} return={returned:?} \\
         finished={finished} claimed={claimed} stored={stored:?} \\
         gas_refused={refused} invalid={invalid} step={}",
        vm_a+actions_a,world.state.step
    );
    assert!(finished);
    assert_eq!(invalid,0);
    assert_eq!(b,0);
    assert_eq!(c_send,0);
    assert!(world.state.guarded_restarts.is_empty());
    assert_eq!(first,vec![actual_expected]);
    assert_eq!(second,vec![actual_expected]);
    assert_eq!(zero.len(),1);
    assert_eq!(zero[0].0,Code::Ok);
    assert_eq!(zero[0].1,Some(0));
    assert!(zero[0].2>0);
    assert_eq!(mutation0.len(),1);
    assert_eq!(mutation0[0].0,Code::Ok);
    assert_eq!(mutation0[0].1,Some(first_mut_expected));
    assert!(mutation0[0].2>0);
    if arm=="COMMON_DOUBLE"{
        assert!(mutation1.is_empty(),
            "byte0 violation must prevent additional charged checks");
        assert!(!claimed);
        assert_eq!(returned,Some(Value::U32(0)));
        assert_eq!(stored,None);
    }else{
        assert_eq!(mutation1.len(),1,"new coordinate must actually Exec");
        assert_eq!(mutation1[0].0,Code::Ok);
        assert_eq!(mutation1[0].1,Some(second_mut_expected));
        assert!(mutation1[0].2>0);
        match arm.as_str(){
            "VALID"=>{
                assert!(claimed);
                assert_eq!(stored,Some(true_value));
            },
            "INVARIANT_FAULT"=>{
                assert_ne!(second_mut_expected,
                    actual_expected.wrapping_add(delta1));
                assert!(!claimed);
                assert_eq!(returned,Some(Value::U32(0)));
                assert_eq!(stored,None);
            },
            "RESIDUAL_FAULT"=>{
                assert_eq!(second_mut_expected,
                    actual_expected.wrapping_add(delta1));
                assert!(claimed,
                    "frozen adversarial residual false success MUST be observable");
                assert_eq!(stored,Some(actual_expected));
                assert_ne!(stored,Some(true_value));
            },
            _=>unreachable!()
        }
    }
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
}


/// An independently constructed, metered pure-VM reference verifier.
/// Input = actual 16B little-endian FOLD32 operands || 4B claimed result.
/// It uses the alternate power-of-two/bit-carry rotation program, rather
/// than trusting the candidate's output or a VM-native FOLD opcode.
/// IMPORTANT: the researcher authors/trusts this verifier; it is not an
/// organism-discovered proof nor an untrusted proof certificate.
fn exact_fold4_claim_verifier()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type};
    let mut program=alternate_fold4_vm_program();
    let f=&mut program.functions[0];
    let entry=&mut f.blocks[0];
    let computed=match entry.terminator{
        Terminator::Return{value}=>value,
        _=>panic!("reference algorithm must terminate normally")
    };
    let mut reg=entry.instructions.iter().map(|i|i.id).max().unwrap()+1;
    let mut emit=|op:Op|{
        let id=reg;
        entry.instructions.push(Instruction{id,op});
        reg+=1;
        id
    };
    let zero=emit(Op::ConstU32(0));
    let mut claim=zero;
    for byte in 0u32..4{
        let index=emit(Op::ConstU32(16+byte));
        let mut part=emit(Op::ByteAt(0,index));
        for _ in 0..(8*byte){
            part=emit(Op::Add(part,part));
        }
        claim=emit(Op::Add(claim,part));
    }
    let equals=emit(Op::EqU32(computed,claim));
    entry.terminator=Terminator::Branch{
        condition:equals,
        if_true:"ok".into(),true_args:vec![],
        if_false:"bad".into(),false_args:vec![]
    };
    f.blocks.push(Block{
        id:"ok".into(),params:vec![],
        instructions:vec![Instruction{id:reg,op:Op::ConstU32(1)}],
        terminator:Terminator::Return{value:reg}
    });
    f.blocks.push(Block{
        id:"bad".into(),params:vec![],
        instructions:vec![Instruction{id:reg+1,op:Op::ConstU32(0)}],
        terminator:Terminator::Return{value:reg+1}
    });
    program.validate().expect("genuine full-input reference checker");
    program
}

/// Deliberately compromised trust anchor: same typed U32 signature but
/// unconditional 'accept'. The active root must demonstrate that it
/// cannot infer correctness from Exec Ok without trusting/checking its
/// verifier's identity and semantic specification.
fn compromised_claim_verifier()->Program{
    use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Terminator,Type};
    let program=Program{
        format_version:1,
        entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),
            args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),
                params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![Instruction{
                    id:1,op:Op::ConstU32(1)
                }],
                terminator:Terminator::Return{value:1}
            }]
        }]
    };
    program.validate().expect("compromised but well-typed verifier");
    program
}

/// Active root uses the existing real feedback-chosen primary tool,
/// then a *different*, paid pure-VM checker on exact operands + claim.
/// Its checker hash is loaded from a normal paid registry; the host does
/// not inject any answer and the root is identical across four arms.
fn p2_paid_exact_claim_checker_root()->Program{
    use ocee_organism_vm::{Block,Instruction,Op,Param,Terminator,Type,WorldCall};
    fn lit(v:&[u8])->Op{
        Op::ConstBytes{hex:v.iter().map(|x|format!("{x:02x}")).collect()}
    }
    fn p(id:u32)->Param{Param{id,ty:Type::Bytes}}
    fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
    fn br(c:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
        Terminator::Branch{
            condition:c,if_true:t.into(),true_args:ta,
            if_false:f.into(),false_args:fa
        }
    }
    let mut root=p2_semantic_ok_registry_root();
    let f=&mut root.functions[0];
    let accepted=f.blocks.iter_mut()
        .find(|x|x.id=="equal_local_exec_reply")
        .expect("candidate must make genuine paid Exec");
    accepted.terminator=br(17304,"ck_key_read",vec![17300],
        "failed",vec![]);
    f.blocks.extend([
        Block{
            id:"ck_key_read".into(),params:vec![p(21000)],
            instructions:vec![
                i(21001,lit(b"/self/work/exact-fold-checker"))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![21001],
                next:"ck_key_reply".into(),next_args:vec![21000]
            }
        },
        Block{
            id:"ck_key_reply".into(),params:vec![p(21000),p(21010)],
            instructions:vec![
                i(21011,lit(b"code")),
                i(21012,Op::JsonFieldString(21010,21011)),
                i(21013,lit(b"Ok")),
                i(21014,Op::EqBytes(21012,21013))
            ],
            terminator:br(21014,"ck_job_read",
                vec![21000,21010],"failed",vec![])
        },
        Block{
            id:"ck_job_read".into(),params:vec![p(21000),p(21020)],
            instructions:vec![
                i(21021,lit(b"payload")),
                i(21022,Op::JsonFieldBytes(21020,21021)),
                i(21023,lit(REMOTE_VALUES.as_bytes()))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Read,args:vec![21023],
                next:"ck_job_reply".into(),next_args:vec![21000,21022]
            }
        },
        Block{
            id:"ck_job_reply".into(),
            params:vec![p(21000),p(21030),p(21031)],
            instructions:vec![
                i(21032,lit(b"code")),
                i(21033,Op::JsonFieldString(21031,21032)),
                i(21034,lit(b"Ok")),
                i(21035,Op::EqBytes(21033,21034))
            ],
            terminator:br(21035,"ck_verify_exec",
                vec![21000,21030,21031],"failed",vec![])
        },
        Block{
            id:"ck_verify_exec".into(),
            params:vec![p(21000),p(21040),p(21041)],
            instructions:vec![
                i(21042,lit(b"payload")),
                i(21043,Op::JsonFieldBytes(21041,21042)),
                i(21044,Op::JsonFieldBytes(21000,21042)),
                i(21045,lit(b"U32")),
                i(21046,Op::JsonFieldU32(21044,21045)),
                i(21047,Op::ConstU32(65536)),
                i(21048,Op::Mod(21046,21047)),
                i(21049,Op::Div(21046,21047)),
                i(21050,Op::PackU16Le(21048)),
                i(21051,Op::PackU16Le(21049)),
                i(21052,Op::Concat(21050,21051)),
                i(21053,Op::Concat(21043,21052))
            ],
            terminator:Terminator::Yield{
                call:WorldCall::Exec,args:vec![21040,21053],
                next:"ck_verify_reply".into(),next_args:vec![21000]
            }
        },
        Block{
            id:"ck_verify_reply".into(),
            params:vec![p(21000),p(21060)],
            instructions:vec![
                i(21061,lit(b"code")),
                i(21062,Op::JsonFieldString(21060,21061)),
                i(21063,lit(b"Ok")),
                i(21064,Op::EqBytes(21062,21063))
            ],
            terminator:br(21064,"ck_accept",
                vec![21000,21060],"failed",vec![])
        },
        Block{
            id:"ck_accept".into(),
            params:vec![p(21000),p(21070)],
            instructions:vec![
                i(21071,lit(b"payload")),
                i(21072,Op::JsonFieldBytes(21070,21071)),
                i(21073,lit(&serde_json::to_vec(&Value::U32(1)).unwrap())),
                i(21074,Op::EqBytes(21072,21073))
            ],
            terminator:br(21074,"equal_local_pack",
                vec![21000],"failed",vec![])
        }
    ]);
    // The inherited S45 root retains retired routes. Cut only true
    // entry-unreachable blocks, never the branch to the shared P1
    // learning chain, its paid actions, or the exact checker path.
    let mut reachable=std::collections::BTreeSet::<String>::new();
    let mut pending=vec![f.blocks[0].id.clone()];
    while let Some(id)=pending.pop(){
        if !reachable.insert(id.clone()){continue;}
        let node=f.blocks.iter().find(|b|b.id==id)
            .expect("all reachable jump targets must exist");
        match &node.terminator{
            Terminator::Return{..}=>(),
            Terminator::Jump{target,..}=>pending.push(target.clone()),
            Terminator::Branch{if_true,if_false,..}=>{
                pending.push(if_true.clone());
                pending.push(if_false.clone());
            },
            Terminator::RepeatBounded{body,done,..}|
            Terminator::RepeatScoped{body,done,..}=>{
                pending.push(body.clone());
                pending.push(done.clone());
            },
            Terminator::Yield{next,..}=>pending.push(next.clone())
        }
    }
    f.blocks.retain(|b|reachable.contains(&b.id));
    println!("P2_EXACT_CHECKER_ROOT blocks={} bytes={}",
        f.blocks.len(),root.canonical_bytes().len());
    root.validate().expect("exact checker must fit P2 typed root limits");
    root
}

/// Four native paid full-task trajectories. A successful compromised
/// verifier arm is intentionally a scientifically negative false success.
#[test]
#[ignore = "four paid full-result checker and compromised verifier trajectories"]
fn p2_exact_checker_native(){
    use ocee_organism_vm::run_isolated;
    let arm=std::env::var("OCEE_P2_EXACT_ARM").expect("matrix condition");
    assert!(["VALID","PLUS_ONE","RESIDUAL_FAULT","COMPROMISED_CHECKER"]
        .contains(&arm.as_str()));
    let correct=local_fold4_vm_program();
    let plus_one=corrupted_fold4_ok();
    let residual=invariant_tail_from_byte2(local_fold4_vm_program());
    let reference=exact_fold4_claim_verifier();
    let compromised=compromised_claim_verifier();
    let candidate=match arm.as_str(){
        "VALID"=>&correct,
        "PLUS_ONE"=>&plus_one,
        "RESIDUAL_FAULT"|"COMPROMISED_CHECKER"=>&residual,
        _=>unreachable!()
    };
    let checker=if arm=="COMPROMISED_CHECKER"{
        &compromised
    }else{&reference};
    let candidate_hash=candidate.content_hash().unwrap();
    let checker_hash=checker.content_hash().unwrap();
    assert_ne!(checker_hash,candidate_hash);
    for numbers in [
        [0u32,0,0,0],
        [u32::MAX,u32::MAX,u32::MAX,u32::MAX],
        [1,2,3,4],
        [0x8000_0000,0x7fff_ffff,0x8000_0001,1]
    ]{
        let truth=fold32(&numbers).unwrap();
        let args=numbers.iter().flat_map(|x|x.to_le_bytes())
            .chain(truth.to_le_bytes()).collect::<Vec<_>>();
        let (out,gas)=run_isolated(&reference,args.clone(),20_000)
            .expect("independent real checker must accept true claim");
        assert_eq!(out,Value::U32(1));
        assert!(gas>0);
        let wrong=numbers.iter().flat_map(|x|x.to_le_bytes())
            .chain(truth.wrapping_add(1).to_le_bytes())
            .collect::<Vec<_>>();
        let (out,_) =run_isolated(&reference,wrong,20_000)
            .expect("independent exact checker must reject wrong claim");
        assert_eq!(out,Value::U32(0));
    }
    let root=p2_paid_exact_claim_checker_root();
    let root_hash=root.content_hash().unwrap();
    let mut w=boot_with_protocol_seed_budget(
        8,2027,12_000,s45_p2_scoped_result_wait(),root
    );
    for tool in [candidate,checker]{
        let hash=tool.content_hash().unwrap();
        let installed=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:tool.canonical_bytes()
        }));
        assert_eq!(installed.code,Code::Ok,"pure program install must pay");
    }
    for (path,value) in [
        ("/self/work/local-fold-candidate",&candidate_hash),
        ("/self/work/exact-fold-checker",&checker_hash)
    ]{
        let stored=w.tick(Some(Action::Write{
            actor:ORGANISM_ID,
            key:path.into(),expected_version:None,
            bytes:value.as_bytes().to_vec()
        }));
        assert_eq!(stored.code,Code::Ok,"paid registry write");
    }
    let genesis=w.audit.iter().map(|a|
        a.result.vm_gas_charged+a.result.gas_charged).sum::<u64>();
    assert_eq!(genesis+w.state.nodes[&NodeId::A].gas_available,12_000);
    let before=w.state_hash();
    assert_eq!(before,
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    let mut gas_refused=0usize;
    let mut invalid=0usize;
    for _ in 0..14_000{
        let outcome=w.tick_autonomous(ORGANISM_ID);
        if outcome.code==Code::Gas{gas_refused+=1;}
        if matches!(outcome.code,Code::Denied|Code::InvalidCode){invalid+=1;}
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished){break;}
    }
    let host=&w.state.organisms[&ORGANISM_ID];
    let finished=host.vm_continuation.as_ref()
        .is_some_and(|c|c.finished);
    let returned=host.last_return.clone();
    let claimed=finished&&returned==Some(Value::U32(1));
    if !w.state.objects.contains_key(REMOTE_VALUES){
        println!(
            "P2_EXACT_EARLY arm={arm} return={returned:?} finished={finished} \\
             init_A={genesis} step={} Gas={gas_refused} Invalid={invalid}",
            w.state.step
        );
        for x in w.audit.iter().rev().filter(|a|a.action.is_some()).take(16){
            println!(
                "P2_EXACT_EARLY_AUDIT step={} code={:?} vm={} actionA={} action={:?}",
                x.step,x.result.code,x.result.vm_gas_charged,
                x.result.gas_charged,x.action.as_ref().map(std::mem::discriminant)
            );
        }
        panic!("real task must create operand object");
    }
    let original=&w.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(original.len(),16);
    let operands=original.chunks_exact(4).map(|v|
        u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>();
    let truth=fold32(&operands).unwrap();
    let bias=original.iter().skip(2).map(|x|u32::from(*x)).sum::<u32>();
    assert!(bias>0,"frozen residual attack is nonzero");
    let candidate_expected=match arm.as_str(){
        "VALID"=>truth,
        "PLUS_ONE"=>truth.wrapping_add(1),
        "RESIDUAL_FAULT"|"COMPROMISED_CHECKER"=>truth.wrapping_add(bias),
        _=>unreachable!()
    };
    let expected_checker=if arm=="VALID"||arm=="COMPROMISED_CHECKER"{
        1
    }else{0};
    let mut tool_out=Vec::<(Code,Option<u32>,u64)>::new();
    let mut checker_out=Vec::<(Code,Option<u32>,u64)>::new();
    let mut checker_inputs=Vec::<Vec<u8>>::new();
    let mut c_sends=0usize;
    for a in &w.audit{
        match &a.action{
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&candidate_hash=>{
                assert_eq!(input.as_slice(),original.as_slice());
                let output=if a.result.code==Code::Ok{
                    let Value::U32(n)=serde_json::from_slice::<Value>(
                        &a.result.payload).unwrap() else{
                        panic!("tool must return typed U32")
                    };
                    Some(n)
                }else{None};
                tool_out.push((a.result.code,output,a.result.gas_charged));
            },
            Some(Action::Exec{program_hash,input,..})
                if program_hash==&checker_hash=>{
                checker_inputs.push(input.clone());
                let output=if a.result.code==Code::Ok{
                    let Value::U32(n)=serde_json::from_slice::<Value>(
                        &a.result.payload).unwrap() else{
                        panic!("checker must return typed U32")
                    };
                    Some(n)
                }else{None};
                checker_out.push((a.result.code,output,a.result.gas_charged));
            },
            Some(Action::Send{destination:NodeId::C,..})
                if a.result.code==Code::Ok=>c_sends+=1,
            _=>()
        }
    }
    let mut exact_input=original.clone();
    exact_input.extend(candidate_expected.to_le_bytes());
    let stored=w.state.objects.get("/self/results/p2-job0/value_le")
        .map(|o|{
            assert_eq!(o.bytes.len(),4);
            u32::from_le_bytes(o.bytes[..4].try_into().unwrap())
        });
    let vm_a=w.audit.iter().map(|a|a.result.vm_gas_charged).sum::<u64>();
    let actions_a=w.audit.iter().map(|a|a.result.gas_charged).sum::<u64>();
    let b=b_total(&w);
    println!(
        "P2_EXACT_CHECKER arm={arm} root={root_hash} first={candidate_hash} \\
         checker={checker_hash} prehash={before} genesis_A={genesis} \\
         vm_A={vm_a} actions_A={actions_a} total_A={} \\
         B={b} C_sends={c_sends} tool={tool_out:?} checker={checker_out:?} \\
         exact_input={checker_inputs:?} truth={truth} bias={bias} \\
         claimed={claimed} return={returned:?} stored={stored:?} \\
         Gas_refused={gas_refused} invalid={invalid} step={}",
        vm_a+actions_a,w.state.step
    );
    assert!(finished);
    assert_eq!(invalid,0);
    assert_eq!(b,0);
    assert_eq!(c_sends,0);
    assert!(w.state.guarded_restarts.is_empty());
    assert_eq!(tool_out.iter().filter(|(c,v,_)|
        *c==Code::Ok&&*v==Some(candidate_expected)).count(),1);
    assert_eq!(checker_inputs,vec![exact_input],
        "whole-input claim must actually reach paid checker");
    assert_eq!(checker_out.len(),1);
    assert_eq!(checker_out[0].0,Code::Ok);
    assert_eq!(checker_out[0].1,Some(expected_checker));
    assert!(checker_out[0].2>0);
    if expected_checker==1{
        assert!(claimed);
        assert_eq!(stored,Some(candidate_expected));
        if arm=="COMPROMISED_CHECKER"{
            assert_ne!(stored,Some(truth),
                "compromised checker must reveal false success");
        }else{
            assert_eq!(stored,Some(truth));
        }
    }else{
        assert!(!claimed);
        assert_eq!(returned,Some(Value::U32(0)));
        assert_eq!(stored,None);
    }
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
