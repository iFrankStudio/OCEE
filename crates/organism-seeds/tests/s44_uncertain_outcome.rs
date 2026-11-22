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
            true_args:vec![17505],if_false:"failed".into(),
            false_args:vec![]
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
        for r in w.audit.iter().rev().take(42){
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
