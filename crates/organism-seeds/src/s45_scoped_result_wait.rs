//! Dual-bounded Phase0 REMOTE_EVAL discovery and Phase2 RESULT waiting.
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
    // Both Phase0 REMOTE_EVAL discovery and Phase2 RESULT scanning
    // must NOT consume one shared 128-slot wait counter across
    // dozens of distinct opcode candidates.
    // S41/S42B already rerouted Empty RECEIVE through its phase
    // classifier. Keep phase-1 request-ID recovery untouched and change
    // ONLY the non-phase-1 fallback into the S45 phase dispatcher.
    let check=f.blocks.iter_mut().find(|b|b.id=="s41_check_drop")
        .expect("S42B Empty feedback must classify phase");
    let Terminator::Branch{if_false,..}=&mut check.terminator
        else{panic!("S42B Empty phase classifier must branch")};
    assert_eq!(if_false.as_str(),"s23_receive_retry");
    *if_false="s45_wait_dispatch".into();
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
                true_args:carried(),if_false:"s45_remote_gate".into(),
                false_args:carried()}},
        // Phase 0 was also using the global S23 128 counter, which is
        // exhausted BEFORE REMOTE_EVAL under certain novel P2 seeds.
        // Phase 1 remains on S44's exact-id outcome and S23 bounded path,
        // and Phase 3 compatibility checks retain the legacy path.
        Block{id:"s45_remote_gate".into(),params:route(),
            instructions:vec![
                Instruction{id:15010,op:Op::ConstU32(0)},
                Instruction{id:15011,op:Op::EqU32(602,15010)},
            ],
            terminator:Terminator::Branch{
                condition:15011,if_true:"s45_remote_retry".into(),
                true_args:carried(),if_false:"s23_receive_retry".into(),
                false_args:carried()}},
        Block{id:"s45_remote_retry".into(),params:route(),
            instructions:vec![],
            terminator:Terminator::RepeatScoped{
                // Strict per-REMOTE_EVAL-opcode and aggregate bounds.
                // Changing opcode cannot reset the 896 total budget.
                limit:6,aggregate_limit:896,scope:601,
                body:"s23_retry_wait".into(),body_args:carried(),
                done:"failed".into(),done_args:vec![]}},
        Block{id:"s45_result_retry".into(),params:route(),
            instructions:vec![],
            terminator:Terminator::RepeatScoped{
                // Distinct candidate opcode (register 601) has a local
                // finite budget, plus 896 total waits for all candidates.
                limit:6,aggregate_limit:896,scope:601,
                body:"s23_retry_wait".into(),body_args:carried(),
                done:"failed".into(),done_args:vec![]}},
    ]);
    program.validate().expect("S45 typed scoped Phase0 and Phase2 waits");
    program
}
pub fn s45_scoped_result_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s45_p2_scoped_result_wait();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S45 P2 hash"))
}
