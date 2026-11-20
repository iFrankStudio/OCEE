//! S38: bounded task-v2 recovery AFTER a verified S37 cache goes stale.
//!
//! When an existing receipt fails because task bytes changed, the active
//! VM submits the actual old cached receipt to a fee-charging RenewProbe.
//! The kernel grants at most one *extra* query when the old receipt is
//! genuinely issued, current public task is the approved v2 at version1,
//! and there has been no previous renewal. Nothing automatically resets
//! the world budget. Then the same frozen VM chooses an informative input,
//! pays for a second Probe, updates existing evidence by CAS, verifies
//! the new receipt, and searches/adopts new code as before.
//!
//! This remains research-prewired bounded recovery, not arbitrary service
//! discovery, runtime control of public tasks, or open-ended evolution.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s36_active_probe::EVIDENCE;
use crate::s37_evidence_guard::s37_verified_probe_root;

fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn inst(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,term:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator:term}
}
fn state(base:u32)->Vec<Param>{
    [Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::U32,
     Type::Bytes,Type::U32,Type::Bytes,Type::Bytes,Type::U32]
        .iter().enumerate().map(|(j,ty)|param(base+j as u32,*ty)).collect()
}
fn ids(base:u32)->Vec<u32>{(base..base+10).collect()}
fn ext(base:u32,extra:&[(u32,Type)])->Vec<Param>{
    let mut p=state(base);
    p.extend(extra.iter().map(|&(id,ty)|param(id,ty)));
    p
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
pub fn s38_task_recovery_root(p1_hash:&str,p2_hash:&str)->Program {
    let mut root=s37_verified_probe_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let (u,b)=(Type::U32,Type::Bytes);
    let verify=f.blocks.iter_mut().find(|x|x.id=="s37_verify_result")
        .expect("S37 verified cache");
    // Only an explicit stale/tampered Conflict is eligible to request
    // an environment-issued renewal; NotFound and other errors still fail.
    let Terminator::Branch{if_false,false_args,..}=&mut verify.terminator
        else{panic!("S37 gate must branch")};
    *if_false="s38_conflict_check".into();
    *false_args=[ids(7100),vec![7110,7111]].concat();

    f.blocks.extend([
        blk("s38_conflict_check",ext(8000,&[(8010,b),(8011,b)]),vec![
            inst(8012,lit(b"code")),
            inst(8013,Op::JsonFieldString(8011,8012)),
            inst(8014,lit(b"Conflict")),
            inst(8015,Op::EqBytes(8013,8014)),
        ],br(8015,"s38_renew_request",
            [ids(8000),vec![8010]].concat(),"fail",vec![])),
        blk("s38_renew_request",ext(8100,&[(8110,b)]),vec![
            inst(8111,lit(b"payload")),
            inst(8112,Op::JsonFieldBytes(8110,8111)),
        ],y(WorldCall::RenewProbe,vec![8112],
            "s38_renew_receipt",ids(8100))),
        blk("s38_renew_receipt",ext(8200,&[(8210,b)]),vec![
            inst(8211,lit(b"code")),
            inst(8212,Op::JsonFieldString(8210,8211)),
            inst(8213,lit(b"Ok")),
            inst(8214,Op::EqBytes(8212,8213)),
        ],br(8214,"s38_first",ids(8200),"fail",vec![])),
        // Same S36 Len-vs-XorFold probe selection, not an oracle answer.
        // Both suggestions and comparison vocabulary are researcher priors.
        blk("s38_first",state(8300),vec![
            inst(8310,lit(&[1])),
            inst(8311,Op::Len(8310)),
            inst(8312,Op::XorFold(8310)),
            inst(8313,Op::EqU32(8311,8312)),
        ],br(8313,"s38_second",ids(8300),
            "s38_query",[ids(8300),vec![8310]].concat())),
        blk("s38_second",state(8400),vec![
            inst(8410,lit(&[7,8,9])),
            inst(8411,Op::Len(8410)),
            inst(8412,Op::XorFold(8410)),
            inst(8413,Op::EqU32(8411,8412)),
        ],br(8413,"fail",vec![],
            "s38_query",[ids(8400),vec![8410]].concat())),
        blk("s38_query",ext(8500,&[(8510,b)]),vec![],
            y(WorldCall::Probe,vec![8510],"s38_probe_receipt",ids(8500))),
        blk("s38_probe_receipt",ext(8600,&[(8610,b)]),vec![
            inst(8611,lit(b"code")),
            inst(8612,Op::JsonFieldString(8610,8611)),
            inst(8613,lit(b"Ok")),
            inst(8614,Op::EqBytes(8612,8613)),
        ],br(8614,"s38_cache_replace",
            [ids(8600),vec![8610]].concat(),"fail",vec![])),
        blk("s38_cache_replace",ext(8700,&[(8710,b)]),vec![
            inst(8711,lit(b"payload")),
            inst(8712,Op::JsonFieldBytes(8710,8711)),
            inst(8713,lit(EVIDENCE.as_bytes())),
            inst(8714,Op::ConstU32(0)),
        ],y(WorldCall::Write,vec![8713,8712,8714],
            "s38_cache_ack",[ids(8700),vec![8710]].concat())),
        blk("s38_cache_ack",ext(8800,&[(8810,b),(8811,b)]),vec![
            inst(8812,lit(b"code")),
            inst(8813,Op::JsonFieldString(8811,8812)),
            inst(8814,lit(b"Ok")),
            inst(8815,Op::EqBytes(8813,8814)),
        ],br(8815,"s38_fresh_verify_prepare",
            [ids(8800),vec![8810]].concat(),"fail",vec![])),
        // The second verification is deliberately A DIFFERENT block.
        // Returning to S37's original verification branch would allow an
        // unbounded reauthorization cycle and is correctly rejected by
        // the static VM control-flow checker.
        blk("s38_fresh_verify_prepare",ext(9000,&[(9010,b)]),vec![
            inst(9011,lit(b"payload")),
            inst(9012,Op::JsonFieldBytes(9010,9011)),
        ],y(WorldCall::VerifyProbe,vec![9012],
            "s38_fresh_verify_result",[ids(9000),vec![9010]].concat())),
        blk("s38_fresh_verify_result",ext(9100,&[(9110,b),(9111,b)]),vec![
            inst(9112,lit(b"code")),
            inst(9113,Op::JsonFieldString(9111,9112)),
            inst(9114,lit(b"Ok")),
            inst(9115,Op::EqBytes(9113,9114)),
        ],br(9115,"s35_probe_parse",
            [ids(9100),vec![9110]].concat(),"fail",vec![])),
    ]);
    root.validate().expect("S38 typed one-shot task revision recovery");
    root
}
pub fn s38_no_renew_control(p1_hash:&str,p2_hash:&str)->Program {
    s37_verified_probe_root(p1_hash,p2_hash)
}
