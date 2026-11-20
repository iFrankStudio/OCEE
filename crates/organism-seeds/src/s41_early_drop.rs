//! S41: first-empty feedback checks audited service-drop status before
//! returning to the existing 128-iteration bounded Receive loop.
//! Empty checks now pay A1 even if negative. A genuine kernel-authored
//! TaskFenceDrop lets the P2 VM request its ONE S40 root restart without
//! waiting for all 128 Receive polls. Kernel recovery invalidates *all*
//! per-attempt owned /self/work/, /self/results/, /self/discoveries/
//! but retains immutable code, learned tools and original probe evidence.
//!
//! These are researcher-supplied lifecycle prefixes, not an organism
//! discovering storage lifetime or a general event subscription API.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s40_dropped_recovery::{s40_p1_consumer,s40_p2_recovery_consumer,
    s40_root_with_specialists};

fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
         terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}

pub fn s41_p2_early_drop_consumer()->Program {
    let mut program=s40_p2_recovery_consumer();
    let f=&mut program.functions[0];
    let feedback=f.blocks.iter_mut().find(|b|b.id=="s22_feedback")
        .expect("S23 feedback handler");
    let Terminator::Branch{if_true,true_args,..}=&mut feedback.terminator
        else{panic!("S23 must branch on Code::Empty")};
    assert_eq!(if_true.as_str(),"s23_receive_retry");
    assert_eq!(true_args.as_slice(),&[600,601,602]);
    *if_true="s41_check_drop".into();

    let (b,u)=(Type::Bytes,Type::U32);
    f.blocks.extend([
        block("s41_check_drop",vec![p(11100,b),p(11101,u),p(11102,u)],
            vec![],Terminator::Yield{
                call:WorldCall::FenceStatus,args:vec![],
                next:"s41_check_reply".into(),
                next_args:vec![11100,11101,11102],
            }),
        block("s41_check_reply",vec![
            p(11200,b),p(11201,u),p(11202,u),p(11203,b)
        ],vec![
            i(11204,lit(b"code")),
            i(11205,Op::JsonFieldString(11203,11204)),
            i(11206,lit(b"Ok")),
            i(11207,Op::EqBytes(11205,11206)),
        ],Terminator::Branch{
            condition:11207,
            if_true:"s40_restart".into(),true_args:vec![],
            if_false:"s23_receive_retry".into(),
            false_args:vec![11200,11201,11202],
        }),
    ]);
    program.validate().expect("S41 paid early drop check with bounded no-drop retry");
    program
}
pub fn s41_early_drop_root()->Program {
    let p1=s40_p1_consumer();
    let p2=s41_p2_early_drop_consumer();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S41 P2 hash"))
}
