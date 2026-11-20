//! S40: when an S39-fenced REMOTE_EVAL request was really dropped by
//! the service, the running P2 protocol specialist itself requests one
//! bounded restart of the ORIGINAL root. The root revalidates the OLD
//! tool and OLD evidence against the now-changed task, uses S38's licensed
//! re-probe/CAS to relearn, and transmits under S39's new task fence.
//!
//! The kernel does NOT silently retry or invent correct operands.
//! Its RestartRoot checks a real audit-visible dropped message, prohibits
//! multiple restarts, retains tool/version/evidence provenance and clears
//! only per-attempt derived B operands. A bounded receive timeout is a
//! researcher-seeded trigger, not general-purpose failure diagnosis.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s32_composite_b_operand::{s32_p1_composite_consumer,s32_p2_composite_consumer};
use crate::s39_task_fenced_remote::s39_task_fenced_remote_root;

fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|x|format!("{x:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn br(c:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
/// The P1 specialist is unmodified: it does not have the P2 bounded
/// receive-state machinery and the S40 recovery study targets P2.
pub fn s40_p1_consumer()->Program{s32_p1_composite_consumer()}

/// Only a REAL dropped-message outcome authorizes a recovery request.
/// An ordinary timeout/no-response has no restart capability.
pub fn s40_p2_recovery_consumer()->Program{
    let mut program=s32_p2_composite_consumer();
    let f=&mut program.functions[0];
    let header=f.blocks.iter_mut().find(|b|b.id=="s23_receive_retry")
        .expect("P2 real bounded Receive loop");
    let Terminator::RepeatBounded{done,done_args,..}=&mut header.terminator
        else{panic!("P2 receive retry must be bounded")};
    *done="s40_fence_status".into();
    *done_args=vec![];
    f.blocks.extend([
        blk("s40_fence_status",vec![],vec![],
            y(WorldCall::FenceStatus,vec![],"s40_fence_status_reply",vec![])),
        blk("s40_fence_status_reply",vec![p(10900,Type::Bytes)],vec![
            ins(10901,lit(b"code")),
            ins(10902,Op::JsonFieldString(10900,10901)),
            ins(10903,lit(b"Ok")),
            ins(10904,Op::EqBytes(10902,10903)),
        ],br(10904,"s40_restart",vec![],"failed",vec![])),
        blk("s40_restart",vec![],vec![],
            y(WorldCall::RestartRoot,vec![],"s40_restart_result",vec![])),
        // Successful RestartRoot atomically invalidates this specialist's
        // VM continuation, so this block can only see a denied request.
        blk("s40_restart_result",vec![p(10910,Type::Bytes)],vec![
            ins(10911,lit(b"code")),
            ins(10912,Op::JsonFieldString(10910,10911)),
            ins(10913,lit(b"Ok")),
            ins(10914,Op::EqBytes(10912,10913)),
        ],br(10914,"failed",vec![],"failed",vec![])),
    ]);
    program.validate().expect("S40 P2 bounded dropped-request recovery");
    program
}
pub fn s40_dropped_recovery_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s40_p2_recovery_consumer();
    s39_task_fenced_remote_root(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("P2 hash"))
}
