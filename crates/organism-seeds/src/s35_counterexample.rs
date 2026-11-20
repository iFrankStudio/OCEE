//! S35: a public counterexample exposes two-sample aliasing. A normal
//! VM checks the third label through paid Read/Exec before adopting an
//! executable or trusting an inherited executable from another protocol.
//! All candidate stages, order, bounds, target and P1/P2 experts remain
//! researcher provided. No autonomous challenge generation is claimed.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s34_incremental_depth::s34_incremental_depth_root;

pub const CHALLENGE:&str="/public/E001/compose-counterexample";
fn bytes(data:&[u8])->Op{
    Op::ConstBytes{hex:data.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
// Canonical state [index, skeleton, unary bank, in0, y0, in1, y1,
// heldout, candidate hash, mode]. mode0=new candidate; mode1=inherited.
fn state(base:u32)->Vec<Param>{
    [Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::U32,
     Type::Bytes,Type::U32,Type::Bytes,Type::Bytes,Type::U32]
        .iter().enumerate().map(|(k,ty)|p(base+k as u32,*ty)).collect()
}
fn ids(base:u32)->Vec<u32>{(base..base+10).collect()}
fn ext(base:u32,extras:&[(u32,Type)])->Vec<Param>{
    let mut v=state(base);
    v.extend(extras.iter().map(|(id,ty)|p(*id,*ty)));
    v
}
fn loop_args(base:u32)->Vec<u32>{
    vec![base,base+1,base+2,base+3,base+4,base+5,base+6,base+7]
}
pub fn s35_counterexample_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s34_incremental_depth_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let (b,u)=(Type::Bytes,Type::U32);
    // After fitting BOTH public training labels the candidate must
    // survive another public labelled input. Do not register it yet.
    let search=f.blocks.iter_mut().find(|x|x.id=="score1").unwrap();
    search.instructions.push(i(815,Op::ConstU32(0)));
    let Terminator::Branch{if_true,true_args,..}=&mut search.terminator
        else {panic!("expected search score branch")};
    *if_true="s35_probe_read".into();
    *true_args=vec![800,802,803,804,805,806,807,808,801,815];

    // Same challenge is MANDATORY for inherited code. The old two
    // examples can match even if the transferred program is semantically
    // wrong for a new input; such hashes must not be trusted.
    let inherited=f.blocks.iter_mut().find(|x|x.id=="s30_score1").unwrap();
    inherited.instructions.push(i(2715,Op::ConstU32(1)));
    let Terminator::Branch{if_true,true_args,..}=&mut inherited.terminator
        else {panic!("expected inherited score branch")};
    *if_true="s35_probe_read".into();
    *true_args=vec![2700,2701,2702,2703,2704,2705,2706,2707,2708,2715];

    f.blocks.extend([
        blk("s35_probe_read",state(5000),vec![
            i(5010,bytes(CHALLENGE.as_bytes())),
        ],y(WorldCall::Read,vec![5010],"s35_probe_receipt",ids(5000))),
        blk("s35_probe_receipt",ext(5100,&[(5110,b)]),vec![
            i(5111,bytes(b"code")),
            i(5112,Op::JsonFieldString(5110,5111)),
            i(5113,bytes(b"Ok")),
            i(5114,Op::EqBytes(5112,5113)),
        ],br(5114,"s35_probe_parse",
            [ids(5100),vec![5110]].concat(),"fail",vec![])),
        // Missing challenge fails closed; no remote B request.
        blk("s35_probe_parse",ext(5200,&[(5210,b)]),vec![
            i(5211,bytes(b"payload")),
            i(5212,Op::JsonFieldBytes(5210,5211)),
            i(5213,bytes(b"input")),
            i(5214,Op::JsonFieldBytes(5212,5213)),
            i(5215,bytes(b"output")),
            i(5216,Op::JsonFieldU32(5212,5215)),
        ],y(WorldCall::Exec,vec![5208,5214],"s35_probe_exec",
            [ids(5200),vec![5216]].concat())),
        blk("s35_probe_exec",ext(5300,&[(5310,u),(5311,b)]),vec![
            i(5312,bytes(b"code")),
            i(5313,Op::JsonFieldString(5311,5312)),
            i(5314,bytes(b"Ok")),
            i(5315,Op::EqBytes(5313,5314)),
        ],br(5315,"s35_probe_score",
            [ids(5300),vec![5310,5311]].concat(),
            "s35_probe_reject",ids(5300))),
        blk("s35_probe_score",ext(5400,&[(5410,u),(5411,b)]),vec![
            i(5412,bytes(b"payload")),
            i(5413,Op::JsonFieldBytes(5411,5412)),
            i(5414,bytes(b"U32")),
            i(5415,Op::JsonFieldU32(5413,5414)),
            i(5416,Op::EqU32(5415,5410)),
        ],br(5416,"s35_probe_accept",ids(5400),
            "s35_probe_reject",ids(5400))),
        blk("s35_probe_accept",state(5500),vec![
            i(5510,Op::ConstU32(1)),
            i(5511,Op::EqU32(5509,5510)),
        ],br(5511,"apply",vec![5508,5507],
            "register",vec![5508,5507])),
        blk("s35_probe_reject",state(5600),vec![
            i(5610,Op::ConstU32(1)),
            i(5611,Op::EqU32(5609,5610)),
        ],br(5611,"loop",loop_args(5600),
            "advance",loop_args(5600))),
    ]);
    root.validate().expect("S35 explicit public counterexample gate");
    root
}
pub fn s35_no_challenge_control(p1_hash:&str,p2_hash:&str)->Program{
    s34_incremental_depth_root(p1_hash,p2_hash)
}
