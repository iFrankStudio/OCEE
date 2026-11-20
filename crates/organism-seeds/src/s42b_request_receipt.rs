//! S42B: the P2 VM persists the actual eight-byte C SEND receipt for
//! its phase-1 REMOTE_EVAL, then supplies that same opaque u64 ID to
//! both failure inspection and its one-shot root-restart request.
//!
//! All retry strategies, graph grammars and task experts remain seeded.
//! This is causal request attribution, NOT generalized fault recovery.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s41_early_drop::s41_p2_early_drop_consumer;
use crate::s40_dropped_recovery::{s40_p1_consumer,s40_root_with_specialists};

pub const PENDING_REMOTE_ID:&str="/self/work/s42b-remote-request-id";
fn lit(x:&[u8])->Op{Op::ConstBytes{hex:x.iter().map(|b|format!("{b:02x}")).collect()}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn jump(target:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:target.into(),args}
}
fn br(condition:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
pub fn s42b_p2_explicit_request_consumer()->Program{
    let mut program=s41_p2_early_drop_consumer();
    let f=&mut program.functions[0];
    let (b,u)=(Type::Bytes,Type::U32);
    let route=||vec![p(600,b),p(601,u),p(602,u)];
    let route_args=||vec![600,601,602];

    // S23 shares this SEND feedback across discovery and real work.
    // Only phase=1 (real REMOTE_EVAL ACK) may establish pending B ID.
    // Save the precise eight bytes from this world SEND, never an
    // inferred count, truncated u32 or host-side inspection of the log.
    let wait=f.blocks.iter_mut().find(|x|x.id=="s22_wait1")
        .expect("S23 P2 SEND feedback");
    assert!(matches!(&wait.terminator,Terminator::Yield{
        call:WorldCall::Observe,next,..} if next=="s22_wait2"));
    wait.instructions=vec![
        i(12000,Op::ConstU32(1)),
        i(12001,Op::EqU32(602,12000)),
    ];
    wait.terminator=br(12001,"s42b_send_check",vec![600,601,602,603],
        "s42b_continue_wait",route_args());

    // Disable S40's legacy actor-level timeout recovery in THIS
    // experiment. Only the explicit ID path can request RestartRootFor.
    let retry=f.blocks.iter_mut().find(|x|x.id=="s23_receive_retry")
        .expect("bounded P2 RECEIVE wait");
    let Terminator::RepeatBounded{done,done_args,..}=&mut retry.terminator
        else{panic!("P2 RECEIVE wait must remain bounded")};
    *done="failed".into();done_args.clear();

    // First empty RECEIVE: ask for failure status only during phase 1.
    let empty=f.blocks.iter_mut().find(|x|x.id=="s41_check_drop")
        .expect("S41 paid early detection");
    empty.instructions=vec![
        i(12002,Op::ConstU32(1)),
        i(12003,Op::EqU32(11102,12002)),
    ];
    empty.terminator=br(12003,"s42b_read_pending",
        vec![11100,11101,11102],"s23_receive_retry",
        vec![11100,11101,11102]);

    f.blocks.extend([
        blk("s42b_send_check",vec![p(600,b),p(601,u),p(602,u),p(603,b)],
            vec![
                i(12004,lit(b"code")),
                i(12005,Op::JsonFieldString(603,12004)),
                i(12006,lit(b"Ok")),
                i(12007,Op::EqBytes(12005,12006)),
            ],br(12007,"s42b_send_extract",vec![600,601,602,603],
                "failed",vec![])),
        blk("s42b_send_extract",vec![p(600,b),p(601,u),p(602,u),p(603,b)],
            vec![
                i(12008,lit(b"payload")),
                i(12009,Op::JsonFieldBytes(603,12008)),
                i(12010,Op::Len(12009)),
                i(12011,Op::ConstU32(8)),
                i(12012,Op::EqU32(12010,12011)),
            ],br(12012,"s42b_store_pending",vec![600,601,602,12009],
                "failed",vec![])),
        blk("s42b_store_pending",vec![
            p(600,b),p(601,u),p(602,u),p(12013,b)],
            vec![
                i(12014,lit(PENDING_REMOTE_ID.as_bytes())),
                i(12015,Op::ConstU32(u32::MAX)),
            ],y(WorldCall::Write,vec![12014,12013,12015],
                "s42b_send_stored",route_args())),
        blk("s42b_send_stored",vec![
            p(600,b),p(601,u),p(602,u),p(12016,b)],
            vec![
                i(12017,lit(b"code")),
                i(12018,Op::JsonFieldString(12016,12017)),
                i(12019,lit(b"Ok")),
                i(12020,Op::EqBytes(12018,12019)),
            ],br(12020,"s42b_continue_wait",route_args(),
                "failed",vec![])),
        blk("s42b_continue_wait",route(),vec![],
            y(WorldCall::Observe,vec![],"s22_wait2",route_args())),
        blk("s42b_read_pending",route(),vec![
                i(12021,lit(PENDING_REMOTE_ID.as_bytes())),
            ],y(WorldCall::Read,vec![12021],"s42b_pending_reply",route_args())),
        blk("s42b_pending_reply",vec![
            p(600,b),p(601,u),p(602,u),p(12022,b)],
            vec![
                i(12023,lit(b"code")),
                i(12024,Op::JsonFieldString(12022,12023)),
                i(12025,lit(b"Ok")),
                i(12026,Op::EqBytes(12024,12025)),
            ],br(12026,"s42b_pending_extract",
                vec![600,601,602,12022],"failed",vec![])),
        blk("s42b_pending_extract",vec![
            p(600,b),p(601,u),p(602,u),p(12027,b)],
            vec![
                i(12028,lit(b"payload")),
                i(12029,Op::JsonFieldBytes(12027,12028)),
                i(12030,Op::Len(12029)),
                i(12031,Op::ConstU32(8)),
                i(12032,Op::EqU32(12030,12031)),
            ],br(12032,"s42b_status_for",
                vec![600,601,602,12029],"failed",vec![])),
        blk("s42b_status_for",vec![
            p(600,b),p(601,u),p(602,u),p(12033,b)],
            vec![],y(WorldCall::FenceStatusFor,vec![12033],
                "s42b_status_result",vec![600,601,602,12033])),
        blk("s42b_status_result",vec![
            p(600,b),p(601,u),p(602,u),p(12034,b),p(12035,b)],
            vec![
                i(12036,lit(b"code")),
                i(12037,Op::JsonFieldString(12035,12036)),
                i(12038,lit(b"Ok")),
                i(12039,Op::EqBytes(12037,12038)),
            ],br(12039,"s42b_restart_for",vec![12034],
                "s23_receive_retry",route_args())),
        blk("s42b_restart_for",vec![p(12040,b)],vec![],
            y(WorldCall::RestartRootFor,vec![12040],
                "s40_restart_result",vec![])),
    ]);
    program.validate().expect("S42B typed request-id VM recovery path");
    program
}
pub fn s42b_explicit_request_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s42b_p2_explicit_request_consumer();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S42B P2 hash"))
}
