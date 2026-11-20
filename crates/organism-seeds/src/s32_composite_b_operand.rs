//! S32: a VM-synthesized two-unary-one-binary program must compute a real
//! fourth FOLD32 operand. The root assembles 3 public U32s + that computed
//! U32; the root's separately VM-generated Len utility computes count=4.
//! Neither P1 nor P2 specialist may recover the old public-only values_le
//! for the final B request; both consume the same newly assembled work item.
//!
//! S6's 18-combination grammar, S30's target validation/repair and S31's
//! Len->count ABI are researcher priors. The selected graph and held-out
//! fourth operand originate from active VM execution, not host injection.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s31_causal_remote::{
    s31_shared_root,s31_p1_count_consumer,s31_p2_count_consumer,PREPARED_COUNT
};

pub const REMOTE_VALUES:&str="/self/work/remote-values-le";

fn lit(s:&[u8])->Op{Op::ConstBytes{hex:s.iter().map(|b|format!("{b:02x}")).collect()}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn br(condition:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
    Terminator::Branch{condition,if_true:t.into(),true_args:ta,if_false:f.into(),false_args:fa}
}

/// Root truly learns a graph under public I/O examples, EXECs it on a
/// target-specific held-out input, and packs that novel U32 into the
/// *actual* remote job's value bytes. S31's independent VM-generated Len
/// then computes the new count4 from the full 16 bytes.
pub fn s32_composite_operands_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s31_shared_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;

    // Already validated the composite candidate with two public labels.
    // This response is the real isolated EXEC on an unseen target input.
    let generated=f.blocks.iter_mut().find(|x|x.id=="s30_extra_good").unwrap();
    generated.instructions.push(i(9800,Op::JsonFieldU32(4102,4103)));
    generated.terminator=y(WorldCall::Write,vec![4105,4104,4106],
        "s30_extra_saved",vec![9800]);

    let confirmed=f.blocks.iter_mut().find(|x|x.id=="s30_extra_saved").unwrap();
    // Yield continuation receives carried values first, then the
    // WorldCall feedback Bytes. The original S30 block had only feedback.
    confirmed.params=vec![p(9801,u),p(4200,b)];
    confirmed.terminator=br(4204,"s32_pack_operand",vec![9801],
        "fail",vec![]);

    // First produce exactly four bytes of the learned U32. The generic
    // byte VM currently has only PackU16Le; compose two halves by ordinary
    // arithmetic and concatenation. The choice of little endian is an
    // explicit prior of the published FOLD32 work ABI.
    f.blocks.extend([
        block("s32_pack_operand",vec![p(9810,u)],vec![
            i(9811,Op::PackU16Le(9810)),
            i(9812,Op::ConstU32(65_536)),
            i(9813,Op::Div(9810,9812)),
            i(9814,Op::PackU16Le(9813)),
            i(9815,Op::Concat(9811,9814)),
            i(9816,lit(b"/public/E001/job0")),
        ],y(WorldCall::Read,vec![9816],"s32_public_job",vec![9815])),
        block("s32_public_job",vec![p(9820,b),p(9821,b)],vec![
            i(9822,lit(b"payload")),
            i(9823,Op::JsonFieldBytes(9821,9822)),
            i(9824,lit(b"values_le")),
            i(9825,Op::JsonFieldBytes(9823,9824)),
            i(9826,Op::Concat(9825,9820)),
            i(9827,lit(REMOTE_VALUES.as_bytes())),
            i(9828,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9827,9826,9828],
            "s32_values_saved",vec![])),
        block("s32_values_saved",vec![p(9830,b)],vec![
            i(9831,lit(b"code")),
            i(9832,Op::JsonFieldString(9830,9831)),
            i(9833,lit(b"Ok")),
            i(9834,Op::EqBytes(9832,9833)),
        ],br(9834,"s30_after",vec![],"fail",vec![])),
    ]);

    // Read the freshly assembled fourth operand along with the three
    // public work values. S31 independently generates and EXECs Len on
    // these 16 bytes, rather than trusting a host-provided count.
    let after=f.blocks.iter_mut().find(|x|x.id=="s30_after").unwrap();
    let start=after.instructions.iter_mut().find(|x|x.id==5000).unwrap();
    start.op=lit(REMOTE_VALUES.as_bytes());
    let length_job=f.blocks.iter_mut().find(|x|x.id=="s28_job").unwrap();
    length_job.instructions=vec![
        i(5002,lit(b"payload")),
        i(5005,Op::JsonFieldBytes(5001,5002)),
        i(5006,lit(b"/self/tools/shared-payload-len")),
    ];
    root.validate().expect("S32 causal operand assembler root");
    root
}

/// Both preinstalled specialists have identical knowledge of the shared
/// work-object contract; neither has a literal answer or host-computed U32.
pub fn s32_p1_composite_consumer()->Program{
    let mut program=s31_p1_count_consumer();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let job=f.blocks.iter_mut().find(|x|x.id=="s19_job").unwrap();
    job.instructions=vec![i(6097,lit(REMOTE_VALUES.as_bytes()))];
    job.terminator=y(WorldCall::Read,vec![6097],
        "s32_p1_values_reply",vec![6090,6091]);
    f.blocks.extend([
        block("s32_p1_values_reply",vec![
            p(6090,b),p(6091,u),p(6099,b)
        ],vec![
            i(9840,lit(b"code")),
            i(9841,Op::JsonFieldString(6099,9840)),
            i(9842,lit(b"Ok")),
            i(9843,Op::EqBytes(9841,9842)),
        ],br(9843,"s32_p1_work_values",vec![6090,6091,6099],
            "failure",vec![])),
        block("s32_p1_work_values",vec![
            p(6090,b),p(6091,u),p(6099,b)
        ],vec![
            i(9850,lit(b"payload")),
            i(9851,Op::JsonFieldBytes(6099,9850)),
            i(9852,lit(PREPARED_COUNT.as_bytes())),
        ],y(WorldCall::Read,vec![9852],
            "s31_p1_count_status",vec![6090,6091,9851])),
    ]);
    program.validate().expect("S32 P1 must use actual composite work bytes");
    program
}

pub fn s32_p2_composite_consumer()->Program{
    let mut program=s31_p2_count_consumer();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let job=f.blocks.iter_mut().find(|x|x.id=="s22_job_send").unwrap();
    job.instructions=vec![i(675,lit(REMOTE_VALUES.as_bytes()))];
    job.terminator=y(WorldCall::Read,vec![675],
        "s32_p2_values_reply",vec![600,601]);
    f.blocks.extend([
        block("s32_p2_values_reply",vec![
            p(600,b),p(601,u),p(603,b)
        ],vec![
            i(9860,lit(b"code")),
            i(9861,Op::JsonFieldString(603,9860)),
            i(9862,lit(b"Ok")),
            i(9863,Op::EqBytes(9861,9862)),
        ],br(9863,"s32_p2_work_values",vec![600,601,603],
            "failed",vec![])),
        block("s32_p2_work_values",vec![
            p(600,b),p(601,u),p(603,b)
        ],vec![
            i(9870,lit(b"payload")),
            i(9871,Op::JsonFieldBytes(603,9870)),
            i(9872,lit(PREPARED_COUNT.as_bytes())),
        ],y(WorldCall::Read,vec![9872],
            "s31_p2_count_status",vec![600,601,9871])),
    ]);
    program.validate().expect("S32 P2 must use actual composite work bytes");
    program
}
