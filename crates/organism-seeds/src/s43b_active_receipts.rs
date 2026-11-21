//! S43B: active P2 VM binds each real ACK and each RESULT candidate
//! response to the exact preceding u64 WorldCall::Send receipt, using
//! the paid S43A ConfirmResponseFor action before parsing wire bytes.
//!
//! A phase/opcode-scoped /self/work/ key avoids overwriting evidence
//! when S22 searches many possible RESULT opcodes. This is still a
//! researcher-defined bounded causal policy, not autonomous diagnosis.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s42b_request_receipt::s42b_p2_explicit_request_consumer;
use crate::s40_dropped_recovery::{s40_p1_consumer,s40_root_with_specialists};

pub const REQUEST_KEY_PREFIX:&str="/self/work/s43b-message-";
fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|x|format!("{x:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(name:&str,params:Vec<Param>,ops:Vec<Instruction>,term:Terminator)->Block{
    Block{id:name.into(),params,instructions:ops,terminator:term}
}
fn br(condition:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn route()->Vec<Param>{
    vec![p(600,Type::Bytes),p(601,Type::U32),
        p(602,Type::U32),p(603,Type::Bytes)]
}
fn args()->Vec<u32>{vec![600,601,602,603]}
fn phase_1_or_2(phase:u32,base:u32)->Vec<Instruction>{
    // ((phase - 1) / 2)==0 selects exactly 1 and 2 for phase in
    // {0,1,2,3}. U32 subtraction wraps; phase 0 becomes MAX.
    vec![
        i(base,Op::ConstU32(1)),
        i(base+1,Op::Sub(phase,base)),
        i(base+2,Op::ConstU32(2)),
        i(base+3,Op::Div(base+1,base+2)),
        i(base+4,Op::ConstU32(0)),
        i(base+5,Op::EqU32(base+3,base+4)),
    ]
}
fn evidence_key(phase:u32,opcode:u32,base:u32)->(Vec<Instruction>,u32){
    // Every S22 RESULT candidate uses a distinct u32 opcode, hence a
    // fresh write-without-version key. The inherited phase-1 B request
    // also gets a fresh key. No unmetered implicit overwrite.
    (vec![
        i(base,lit(REQUEST_KEY_PREFIX.as_bytes())),
        i(base+1,Op::PackByte(phase)),
        i(base+2,Op::HexEncode(base+1)),
        i(base+3,Op::Concat(base,base+2)),
        i(base+4,Op::PackByte(opcode)),
        i(base+5,Op::HexEncode(base+4)),
        i(base+6,Op::Concat(base+3,base+5)),
    ],base+6)
}

pub fn s43b_p2_confirmed_reply_consumer()->Program{
    let mut program=s42b_p2_explicit_request_consumer();
    let f=&mut program.functions[0];
    let (b,u)=(Type::Bytes,Type::U32);

    // S42B already captures the exact SEND feedback and verifies length
    // before persisting. Extend capture to phase 2 (RESULT discovery)
    // while excluding phase 0/3 probes. No additional graph block.
    let wait=f.blocks.iter_mut().find(|v|v.id=="s22_wait1").unwrap();
    wait.instructions=phase_1_or_2(602,13000);
    wait.terminator=br(13005,"s42b_send_check",args(),
        "s42b_continue_wait",vec![600,601,602]);

    // The original S42B had one fixed phase-1 key. The RESULT opcode
    // scan sends many requests, so key each (phase,opcode) uniquely.
    let saved=f.blocks.iter_mut().find(|v|v.id=="s42b_store_pending").unwrap();
    let (mut key,key_reg)=evidence_key(602,601,13100);
    key.push(i(13107,Op::ConstU32(u32::MAX)));
    saved.instructions=key;
    saved.terminator=y(WorldCall::Write,vec![key_reg,12013,13107],
        "s42b_send_stored",vec![600,601,602]);

    // After Empty, S42B's request-specific failure query must read
    // precisely the same key used by the actual phase-1 SEND.
    let pending=f.blocks.iter_mut().find(|v|v.id=="s42b_read_pending").unwrap();
    let (key,key_reg)=evidence_key(602,601,13120);
    pending.instructions=key;
    pending.terminator=y(WorldCall::Read,vec![key_reg],
        "s42b_pending_reply",vec![11100,11101,11102]);

    // The S23 status guard has already established a successful RECEIVE.
    // Intercept before S23 parses protocol frames or accepts an ACK/RESULT.
    let status=f.blocks.iter_mut().find(|v|v.id=="s23_feedback_check_ok").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut status.terminator
        else{panic!("S23 status decision must remain a Branch")};
    assert_eq!(if_true.as_str(),"s23_feedback_payload");
    assert_eq!(true_args.as_slice(),args().as_slice());
    *if_true="s43b_reply_gate".into();

    // These actor-level recovery blocks cannot be reached from the S42B
    // explicit-ID graph. Remove them to stay inside the VM's 128-block
    // static maximum instead of increasing a global quota.
    const DEAD:&[&str]=&[
        "s40_fence_status","s40_fence_status_reply",
        "s40_restart","s41_check_reply",
    ];
    for block in DEAD{
        assert!(f.blocks.iter().any(|v|v.id==*block),
            "S42B legacy block missing: {block}");
    }
    f.blocks.retain(|v|!DEAD.contains(&v.id.as_str()));

    let (mut read_key,key_reg)=evidence_key(602,601,13140);
    // Preserve original full RECEIVE JSON in register 603 across
    // the WorldCall::Read and WorldCall::ConfirmResponseFor yields.
    f.blocks.extend([
        blk("s43b_reply_gate",route(),phase_1_or_2(602,13200),
            br(13205,"s43b_confirm_read",args(),
                "s23_feedback_payload",args())),
        blk("s43b_confirm_read",route(),{
                let mut instructions=Vec::new();
                instructions.append(&mut read_key);
                instructions
            },y(WorldCall::Read,vec![key_reg],
                "s43b_confirm_read_result",args())),
        blk("s43b_confirm_read_result",{
            let mut params=route();params.push(p(13300,b));params
        },vec![
            i(13301,lit(b"code")),
            i(13302,Op::JsonFieldString(13300,13301)),
            i(13303,lit(b"Ok")),
            i(13304,Op::EqBytes(13302,13303)),
        ],br(13304,"s43b_confirm_payload",
            vec![600,601,602,603,13300],"failed",vec![])),
        blk("s43b_confirm_payload",{
            let mut params=route();params.push(p(13300,b));params
        },vec![
            i(13310,lit(b"payload")),
            i(13311,Op::JsonFieldBytes(13300,13310)),
            i(13312,Op::JsonFieldBytes(603,13310)),
            i(13313,Op::ConstU32(0)),
            i(13314,Op::JsonIndex(13312,13313)),
        ],y(WorldCall::ConfirmResponseFor,vec![13311,13314],
            "s43b_confirm_verdict",args())),
        blk("s43b_confirm_verdict",{
            let mut params=route();params.push(p(13320,b));params
        },vec![
            i(13321,lit(b"code")),
            i(13322,Op::JsonFieldString(13320,13321)),
            i(13323,lit(b"Ok")),
            i(13324,Op::EqBytes(13322,13323)),
        ],br(13324,"s23_feedback_payload",args(),"failed",vec![])),
    ]);
    program.validate().expect("S43B request-correlated P2 ACK/RESULT");
    program
}

pub fn s43b_confirmed_reply_root()->Program{
    let p1=s40_p1_consumer();
    let p2=s43b_p2_confirmed_reply_consumer();
    s40_root_with_specialists(
        &p1.content_hash().expect("P1 hash"),
        &p2.content_hash().expect("S43B P2 hash"))
}
