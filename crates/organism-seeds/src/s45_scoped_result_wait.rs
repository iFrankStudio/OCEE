//! Dual-bounded, per-result-opcode waiting derived from the S44 active VM.
//! Do not resend an unconfirmed committed request, and never bypass
//! S43B's true SEND -> service -> RECEIVE confirmation gate.
#![forbid(unsafe_code)]
use crate::s44_outcome_wait::s44_p2_causal_wait_consumer;
use crate::s40_dropped_recovery::{s40_p1_consumer,s40_root_with_specialists};
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type};

fn route()->Vec<Param>{
    vec![Param{id:600,ty:Type::Bytes},Param{id:601,ty:Type::U32},
        Param{id:602,ty:Type::U32}]
}
fn carried()->Vec<u32>{vec![600,601,602]}

pub fn s45_p2_scoped_result_wait()->Program{
    let mut program=s44_p2_causal_wait_consumer();
    let f=&mut program.functions[0];
    // Phase 2 RESULT scans must NOT consume the same 128-slot counter
    // across dozens of distinct opcode candidates.
    let feedback=f.blocks.iter_mut().find(|b|b.id=="s22_feedback")
        .expect("S23 has Empty feedback guard");
    let Terminator::Branch{if_true,..}=&mut feedback.terminator
        else{panic!("S23 feedback must branch")};
    assert_eq!(if_true.as_str(),"s23_receive_retry");
    *if_true="s45_wait_dispatch".into();
    for id in ["s44_inflight","s44_committed","s44_handled"]{
        let b=f.blocks.iter_mut().find(|x|x.id==id).unwrap();
        let Terminator::Branch{if_true,..}=&mut b.terminator
            else{panic!("S44 must branch on exact request state")};
        assert_eq!(if_true.as_str(),"s23_receive_retry");
        *if_true="s45_wait_dispatch".into();
    }
    f.blocks.extend([
        Block{id:"s45_wait_dispatch".into(),params:route(),
            instructions:vec![
                Instruction{id:15000,op:Op::ConstU32(2)},
                Instruction{id:15001,op:Op::EqU32(602,15000)},
            ],
            terminator:Terminator::Branch{
                condition:15001,if_true:"s45_result_retry".into(),
                true_args:carried(),if_false:"s23_receive_retry".into(),
                false_args:carried()}},
        Block{id:"s45_result_retry".into(),params:route(),
            instructions:vec![],
            terminator:Terminator::RepeatScoped{
                // Distinct candidate opcode (register 601) has a local
                // finite budget, plus 896 total waits for all candidates.
                limit:6,aggregate_limit:896,scope:601,
                body:"s23_retry_wait".into(),body_args:carried(),
                done:"failed".into(),done_args:vec![]}},
    ]);
    program.validate().expect("S45 typed dual-bound result scan");
    program
}
pub fn s45_scoped_result_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s45_p2_scoped_result_wait();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S45 P2 hash"))
}
