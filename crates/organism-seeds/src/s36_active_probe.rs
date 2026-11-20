//! S36: active choice of an informative probe input, using a REAL, paid,
//! audit-visible environment Probe call instead of S35's researcher-seeded
//! third labelled example. The root never reads a stored answer/oracle.
//!
//! The organism first compares Len and XorFold on two explicitly proposed
//! probe inputs; their disagreement determines which input to submit.
//! A metered one-query oracle replies with that input's label, which the
//! organism caches as real paid self storage. Newly synthesized and
//! inherited candidate executables must both pass that acquired label.
//!
//! This is a bounded, researcher-defined pairwise disagreement strategy,
//! not free-form hypothesis inference or autonomous invention of probes.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall
};
use crate::s35_counterexample::s35_counterexample_root;
use crate::s34_incremental_depth::s34_incremental_depth_root;

pub const EVIDENCE:&str="/self/evidence/s36-active-probe";

fn lit(data:&[u8])->Op{
    Op::ConstBytes{hex:data.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn br(c:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn state(base:u32)->Vec<Param>{
    [Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::U32,
     Type::Bytes,Type::U32,Type::Bytes,Type::Bytes,Type::U32]
        .iter().enumerate().map(|(j,ty)|p(base+j as u32,*ty)).collect()
}
fn args(base:u32)->Vec<u32>{(base..base+10).collect()}
fn ext(base:u32,extras:&[(u32,Type)])->Vec<Param>{
    let mut out=state(base);
    out.extend(extras.iter().map(|&(id,ty)|p(id,ty)));
    out
}
/// Same root bytes on both P1 and P2 for all ordinary worlds. The two
/// default suggestions are NOT hidden labels; only raw input bytes.
/// First [1] fails to separate Len from XorFold; second [7,8,9] separates
/// the competing primitive hypotheses, so the VM selects the latter.
pub fn s36_active_probe_root(p1_hash:&str,p2_hash:&str)->Program{
    s36_probe_bank_root(p1_hash,p2_hash,&[1],&[7,8,9])
}
/// Mechanism controls can change candidate inputs, without ever passing
/// a reference answer or a world-seed-specific selected probe to the VM.
pub fn s36_probe_bank_root(p1_hash:&str,p2_hash:&str,
    first:&[u8],second:&[u8])->Program {
    let mut root=s35_counterexample_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let (b,u)=(Type::Bytes,Type::U32);

    // S35 was a READ of a PRE-PUBLISHED third label. Replace it with
    // a private per-world cache lookup. The cache is created ONLY after
    // this organism makes and pays for a real Probe request.
    let lookup=f.blocks.iter_mut().find(|x|x.id=="s35_probe_read").unwrap();
    lookup.instructions[0].op=lit(EVIDENCE.as_bytes());
    let Terminator::Yield{next,..}=&mut lookup.terminator
        else {panic!("S35 public Read expected")};
    *next="s36_cache_check".into();

    f.blocks.extend([
        block("s36_cache_check",ext(6000,&[(6010,b)]),vec![
            i(6011,lit(b"code")),
            i(6012,Op::JsonFieldString(6010,6011)),
            i(6013,lit(b"Ok")),
            i(6014,Op::EqBytes(6012,6013)),
        ],br(6014,"s35_probe_parse",
            [args(6000),vec![6010]].concat(),
            "s36_cache_miss", [args(6000),vec![6010]].concat())),
        block("s36_cache_miss",ext(6100,&[(6110,b)]),vec![
            i(6111,lit(b"code")),
            i(6112,Op::JsonFieldString(6110,6111)),
            i(6113,lit(b"NotFound")),
            i(6114,Op::EqBytes(6112,6113)),
        ],br(6114,"s36_first",args(6100),
            "fail",vec![])),
        // Compare the two explicit surviving generic primitive hypotheses.
        // A probe where they agree supplies NO discriminating information.
        block("s36_first",state(6200),vec![
            i(6210,lit(first)),
            i(6211,Op::Len(6210)),
            i(6212,Op::XorFold(6210)),
            i(6213,Op::EqU32(6211,6212)),
        ],br(6213,"s36_second",args(6200),
            "s36_query",[args(6200),vec![6210]].concat())),
        block("s36_second",state(6300),vec![
            i(6310,lit(second)),
            i(6311,Op::Len(6310)),
            i(6312,Op::XorFold(6310)),
            i(6313,Op::EqU32(6311,6312)),
        ],br(6313,"fail",vec![],
            "s36_query",[args(6300),vec![6310]].concat())),
        block("s36_query",ext(6400,&[(6410,b)]),vec![],
            y(WorldCall::Probe,vec![6410],"s36_probe_receipt",args(6400))),
        block("s36_probe_receipt",ext(6500,&[(6510,b)]),vec![
            i(6511,lit(b"code")),
            i(6512,Op::JsonFieldString(6510,6511)),
            i(6513,lit(b"Ok")),
            i(6514,Op::EqBytes(6512,6513)),
        ],br(6514,"s36_cache_write",
            [args(6500),vec![6510]].concat(),
            "fail",vec![])),
        block("s36_cache_write",ext(6600,&[(6610,b)]),vec![
            i(6611,lit(b"payload")),
            i(6612,Op::JsonFieldBytes(6610,6611)),
            i(6613,lit(EVIDENCE.as_bytes())),
            i(6614,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![6613,6612,6614],
            "s36_cache_ack",[args(6600),vec![6610]].concat())),
        block("s36_cache_ack",ext(6700,&[(6710,b),(6711,b)]),vec![
            i(6712,lit(b"code")),
            i(6713,Op::JsonFieldString(6711,6712)),
            i(6714,lit(b"Ok")),
            i(6715,Op::EqBytes(6713,6714)),
        ],br(6715,"s35_probe_parse",
            [args(6700),vec![6710]].concat(),
            "fail",vec![])),
    ]);
    root.validate().expect("S36 conditional probing and cache must typecheck");
    root
}
pub fn s36_no_probe_control(p1_hash:&str,p2_hash:&str)->Program{
    s34_incremental_depth_root(p1_hash,p2_hash)
}
