//! S44: active P2 VM examines its own causal request outcome after an
//! empty RECEIVE. A committed remote B operation is NEVER taken as proof
//! that a resend is safe; only a positively observed task-fence Drop can
//! reach the already-guarded one-shot RestartRootFor path.
//!
//! The VM still does not have an unbounded event loop, new network powers,
//! or a generalized task planner. All branches cost metered world calls.
#![forbid(unsafe_code)]

use crate::s43b_active_receipts::s43b_p2_confirmed_reply_consumer;
use crate::s40_dropped_recovery::{s40_p1_consumer,s40_root_with_specialists};
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};

fn literal(x:&[u8])->Op{
    Op::ConstBytes{hex:x.iter().map(|b|format!("{b:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn branch(condition:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn route()->Vec<Param>{
    vec![param(600,Type::Bytes),param(601,Type::U32),
        param(602,Type::U32)]
}
fn route_with_state()->Vec<Param>{
    let mut p=route();
    p.push(param(14013,Type::Bytes));p
}
fn route_args()->Vec<u32>{vec![600,601,602]}

pub fn s44_p2_causal_wait_consumer()->Program{
    let mut program=s43b_p2_confirmed_reply_consumer();
    let f=&mut program.functions[0];

    // Preserve the ID that S42B actually captured from successful SEND.
    // Previously the P2 empty-mailbox path only inspected task-fence
    // status. Query ALL distinct outcomes, then selectively decide.
    let request=f.blocks.iter_mut().find(|b|b.id=="s42b_status_for")
        .expect("S42B request-scoped empty-mailbox status");
    let Terminator::Yield{call,args,next,..}=&mut request.terminator
        else{panic!("expected paid request-status yield")};
    assert_eq!(*call,WorldCall::FenceStatusFor);
    assert_eq!(args.as_slice(),&[12033]);
    assert_eq!(next.as_str(),"s42b_status_result");
    *call=WorldCall::RequestOutcomeFor;

    // Keep the same typed parameters, but S43A returns Code::Ok plus a
    // JSON payload containing the status enum, not a FenceStatus code.
    let verdict=f.blocks.iter_mut().find(|b|b.id=="s42b_status_result")
        .expect("S42B feedback receives exact request u64");
    verdict.instructions=vec![
        ins(14000,literal(b"code")),
        ins(14001,Op::JsonFieldString(12035,14000)),
        ins(14002,literal(b"Ok")),
        ins(14003,Op::EqBytes(14001,14002)),
    ];
    verdict.terminator=branch(14003,"s44_decode_outcome",
        vec![600,601,602,12034,12035],"failed",vec![]);

    let mut reply=route();
    reply.push(param(12034,Type::Bytes));
    reply.push(param(12035,Type::Bytes));
    f.blocks.extend([
        block("s44_decode_outcome",reply,vec![
            ins(14010,literal(b"payload")),
            ins(14011,Op::JsonFieldBytes(12035,14010)),
            ins(14012,literal(b"state")),
            ins(14013,Op::JsonFieldString(14011,14012)),
            ins(14014,literal(b"Dropped")),
            ins(14015,Op::EqBytes(14013,14014)),
        ],branch(14015,"s42b_restart_for",vec![12034],
            "s44_inflight",vec![600,601,602,14013])),
        block("s44_inflight",route_with_state(),vec![
            ins(14020,literal(b"InFlight")),
            ins(14021,Op::EqBytes(14013,14020)),
        ],branch(14021,"s23_receive_retry",route_args(),
            "s44_committed",vec![600,601,602,14013])),
        block("s44_committed",route_with_state(),vec![
            ins(14030,literal(b"CommittedB")),
            ins(14031,Op::EqBytes(14013,14030)),
        ],branch(14031,"s23_receive_retry",route_args(),
            "s44_handled",vec![600,601,602,14013])),
        block("s44_handled",route_with_state(),vec![
            ins(14040,literal(b"HandledNoB")),
            ins(14041,Op::EqBytes(14013,14040)),
        ],branch(14041,"s23_receive_retry",route_args(),
            "failed",vec![])),
    ]);
    program.validate().expect("S44 typed paid four-state wait/recover");
    program
}

pub fn s44_outcome_aware_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s44_p2_causal_wait_consumer();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S44 P2 hash"))
}
