//! S39: close the verified-evidence / real remote B TOCTOU gap.
//!
//! After the final learned operand is assembled, the active VM READs its
//! actually queried evidence and pays to ARM a task-version fence. The
//! kernel only arms it when the receipt exactly matches the private issued
//! ProbeRecord AND the current public task hash. Then every SEND from that
//! organism is checked against the version, and each queued C message
//! is checked again at service consumption (before B gas is spent).
//!
//! The fence is a simulation-kernel transaction guard, not a distributed
//! atomic protocol; S38 task renewal and graph learning are unchanged.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s38_task_recovery::s38_task_recovery_root;
use crate::s36_active_probe::EVIDENCE;

fn lit(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|x|format!("{x:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,t:Type)->Param{Param{id,ty:t}}
fn blk(name:&str,params:Vec<Param>,ins:Vec<Instruction>,term:Terminator)->Block {
    Block{id:name.into(),params,instructions:ins,terminator:term}
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator {
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
pub fn s39_task_fenced_remote_root(p1_hash:&str,p2_hash:&str)->Program {
    let mut root=s38_task_recovery_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];

    // This block runs after a REAL Write of the fourth B operand and
    // before the actual remote protocol specialist becomes active.
    // An additional prior VerifyProbe at this point is not sufficient:
    // the task might change in transit. ARM applies to all later Sends.
    let result=f.blocks.iter_mut().find(|b|b.id=="s32_values_saved")
        .expect("S32 causal output committed");
    let Terminator::Branch{if_true,..}=&mut result.terminator
        else {panic!("expected successful operand write branch")};
    *if_true="s39_fence_read".into();

    f.blocks.extend([
        blk("s39_fence_read",vec![],vec![
            i(10001,lit(EVIDENCE.as_bytes())),
        ],Terminator::Yield{
            call:WorldCall::Read,args:vec![10001],
            next:"s39_fence_read_result".into(),next_args:vec![],
        }),
        blk("s39_fence_read_result",vec![p(10010,Type::Bytes)],vec![
            i(10011,lit(b"code")),
            i(10012,Op::JsonFieldString(10010,10011)),
            i(10013,lit(b"Ok")),
            i(10014,Op::EqBytes(10012,10013)),
        ],br(10014,"s39_fence_arm",vec![10010],"fail",vec![])),
        blk("s39_fence_arm",vec![p(10020,Type::Bytes)],vec![
            i(10021,lit(b"payload")),
            i(10022,Op::JsonFieldBytes(10020,10021)),
        ],Terminator::Yield{
            call:WorldCall::ArmTaskFence,args:vec![10022],
            next:"s39_fence_ack".into(),next_args:vec![],
        }),
        blk("s39_fence_ack",vec![p(10030,Type::Bytes)],vec![
            i(10031,lit(b"code")),
            i(10032,Op::JsonFieldString(10030,10031)),
            i(10033,lit(b"Ok")),
            i(10034,Op::EqBytes(10032,10033)),
        ],br(10034,"s30_after",vec![],"fail",vec![])),
    ]);
    root.validate().expect("S39 typed task-bound remote fence root");
    root
}
pub fn s39_unfenced_control(p1_hash:&str,p2_hash:&str)->Program {
    s38_task_recovery_root(p1_hash,p2_hash)
}
