//! S30: integrate the previously native-validated S6 18-candidate
//! two-unary/one-binary graph search into an identical P1/P2 active root.
//!
//! New mechanisms relative to S6: inherited program registry, target-side
//! two-example semantic revalidation, fallback to search after mismatch,
//! paid CAS replacement of a mismatching inherited registry, and independent
//! target-seed-dependent out-of-distribution prediction before real C->B work.
//!
//! All 18 candidate operator triples, template slots, and search order remain
//! explicit researcher priors. NOT open-ended algorithm invention / E001 G1.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::{s6_compose::s6_composition_search,s28_cross_utility::s28_cross_family_root};

pub const REGISTRY:&str="/self/tools/transform-v1";
pub const OOD_PREDICTION:&str="/self/results/s30-extra-held-out";

fn bytes(s:&[u8])->Op{
    Op::ConstBytes{hex:s.iter().map(|b|format!("{x:02x}")).collect()}
}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn blk(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(next:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:next.into(),args}
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),
        true_args:ya,if_false:no.into(),false_args:na}
}
// [candidate index, skeleton, unary bank, ex0 Bytes, y0 U32,
//  ex1 Bytes, y1 U32, compose held-out Bytes].
fn state(base:u32)->Vec<Param>{
    [Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::U32,
     Type::Bytes,Type::U32,Type::Bytes]
        .iter().enumerate().map(|(i,t)|p(base+i as u32,*t)).collect()
}
fn ids(start:u32)->Vec<u32>{(start..start+8).collect()}
fn ext(base:u32,extra:&[(u32,Type)])->Vec<Param>{
    let mut v=state(base);
    v.extend(extra.iter().map(|&(id,ty)|p(id,ty)));
    v
}
fn append(mut v:Vec<u32>,x:u32)->Vec<u32>{v.push(x);v}

/// Every world gets the same complete initial root and the same researcher
/// preinstalled P1 and P2 specialists, independent of actual protocol family.
///
/// The only imported item after boot is the VM-generated *composite tool*
/// and its registry reference, charged as ordinary world WRITE actions.
pub fn s30_composed_cross_family_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s6_composition_search();
    let downstream=s28_cross_family_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;

    // S6's parse still obtains all 2 public labels; now it probes
    // for an inherited executable before entering the S6 search.
    let parse=f.blocks.iter_mut().find(|x|x.id=="parse").unwrap();
    parse.instructions.push(i(1500,bytes(REGISTRY.as_bytes())));
    parse.terminator=y(WorldCall::Read,vec![1500],"s30_registry",
        vec![29,27,28,21,22,23,24,26]);

    // S6's plain registry WRITE cannot replace a previously imported,
    // semantically wrong tool. Check actual registry existence and choose
    // expected_version=None (u32::MAX) or CAS expected_version=0.
    let register=f.blocks.iter_mut().find(|x|x.id=="register").unwrap();
    register.terminator=y(WorldCall::Read,vec![1002],
        "s30_before_register",vec![1000,1001]);

    // Reuse S6's genuine held-out tool execution, then run the resulting
    // graph on DIFFERENT bytes coming from the target world's unary
    // transform fixture (not the fixed S6 held-out vector).
    let apply=f.blocks.iter_mut().find(|x|x.id=="apply").unwrap();
    apply.terminator=y(WorldCall::Exec,vec![1100,1101],
        "s30_held_reply",vec![1100]);

    // S28's common root performs the public job length guard and then
    // uses public HELLO evidence to switch to one of the two equally
    // preinstalled protocol specialists, for real remote FOLD32.
    let mut tail=downstream.functions.into_iter().next().unwrap().blocks;
    let entry_pos=tail.iter().position(|x|x.id=="entry").unwrap();
    tail[entry_pos].id="s30_after".into();
    f.blocks.extend(tail);

    f.blocks.extend([
        // Attempt existing composite tool, but never trust provenance/hash
        // without validating its actual behavior on the TARGET examples.
        blk("s30_registry",ext(2000,&[(2008,b)]),vec![
            i(2010,bytes(b"code")),
            i(2011,Op::JsonFieldString(2008,2010)),
            i(2012,bytes(b"Ok")),
            i(2013,Op::EqBytes(2011,2012)),
        ],br(2013,"s30_registry_hit",append(ids(2000),2008),
            "loop",ids(2000))),
        blk("s30_registry_hit",ext(2100,&[(2108,b)]),vec![
            i(2110,bytes(b"payload")),
            i(2111,Op::JsonFieldBytes(2108,2110)),
        ],j("s30_reuse0",append(ids(2100),2111))),
        blk("s30_reuse0",ext(2200,&[(2208,b)]),vec![],
            y(WorldCall::Exec,vec![2208,2203],"s30_reuse0_reply",
                append(ids(2200),2208))),
        blk("s30_reuse0_reply",ext(2300,&[(2308,b),(2309,b)]),vec![
            i(2310,bytes(b"code")),
            i(2311,Op::JsonFieldString(2309,2310)),
            i(2312,bytes(b"Ok")),
            i(2313,Op::EqBytes(2311,2312)),
        ],br(2313,"s30_score0",append(append(ids(2300),2308),2309),
            "loop",ids(2300))),
        blk("s30_score0",ext(2400,&[(2408,b),(2409,b)]),vec![
            i(2410,bytes(b"payload")),
            i(2411,Op::JsonFieldBytes(2409,2410)),
            i(2412,bytes(b"U32")),
            i(2413,Op::JsonFieldU32(2411,2412)),
            i(2414,Op::EqU32(2413,2404)),
        ],br(2414,"s30_reuse1",append(ids(2400),2408),
            "loop",ids(2400))),
        blk("s30_reuse1",ext(2500,&[(2508,b)]),vec![],
            y(WorldCall::Exec,vec![2508,2505],"s30_reuse1_reply",
                append(ids(2500),2508))),
        blk("s30_reuse1_reply",ext(2600,&[(2608,b),(2609,b)]),vec![
            i(2610,bytes(b"code")),
            i(2611,Op::JsonFieldString(2609,2610)),
            i(2612,bytes(b"Ok")),
            i(2613,Op::EqBytes(2611,2612)),
        ],br(2613,"s30_score1",append(append(ids(2600),2608),2609),
            "loop",ids(2600))),
        blk("s30_score1",ext(2700,&[(2708,b),(2709,b)]),vec![
            i(2710,bytes(b"payload")),
            i(2711,Op::JsonFieldBytes(2709,2710)),
            i(2712,bytes(b"U32")),
            i(2713,Op::JsonFieldU32(2711,2712)),
            i(2714,Op::EqU32(2713,2706)),
        ],br(2714,"apply",vec![2708,2707],
            "loop",ids(2700))),
        // S6's candidate loop either discovers a new tool, or re-discovers
        // a better one after the inherited tool failed target validation.
        blk("s30_before_register",vec![p(3000,b),p(3001,b),p(3002,b)],vec![
            i(3003,bytes(b"code")),
            i(3004,Op::JsonFieldString(3002,3003)),
            i(3005,bytes(b"Ok")),
            i(3006,Op::EqBytes(3004,3005)),
        ],br(3006,"s30_repair_register",vec![3000,3001],
            "s30_fresh_register",vec![3000,3001])),
        blk("s30_repair_register",vec![p(3100,b),p(3101,b)],vec![
            i(3102,Op::ConstU32(0)),
        ],j("s30_set_register",vec![3100,3101,3102])),
        blk("s30_fresh_register",vec![p(3200,b),p(3201,b)],vec![
            i(3202,Op::ConstU32(u32::MAX)),
        ],j("s30_set_register",vec![3200,3201,3202])),
        blk("s30_set_register",vec![p(3300,b),p(3301,b),p(3302,u)],vec![
            i(3303,bytes(REGISTRY.as_bytes())),
        ],y(WorldCall::Write,vec![3303,3300,3302],
            "s30_register_ack",vec![3300,3301])),
        blk("s30_register_ack",vec![p(3400,b),p(3401,b),p(3402,b)],vec![
            i(3403,bytes(b"code")),
            i(3404,Op::JsonFieldString(3402,3403)),
            i(3405,bytes(b"Ok")),
            i(3406,Op::EqBytes(3404,3405)),
        ],br(3406,"apply",vec![3400,3401],"fail",vec![])),
        // For both search and inherited reuse, require successful EXEC on
        // S6's unseen input. Save its complete metered feedback, then make
        // another prediction from target-seed-dependent public input bytes.
        blk("s30_held_reply",vec![p(3500,b),p(3501,b)],vec![
            i(3502,bytes(b"code")),
            i(3503,Op::JsonFieldString(3501,3502)),
            i(3504,bytes(b"Ok")),
            i(3505,Op::EqBytes(3503,3504)),
        ],br(3505,"s30_save_held",vec![3500,3501],
            "fail",vec![])),
        blk("s30_save_held",vec![p(3600,b),p(3601,b)],vec![
            i(3602,bytes(b"/self/results/transform-held-out")),
            i(3603,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![3602,3601,3603],
            "s30_saved_held",vec![3600])),
        blk("s30_saved_held",vec![p(3700,b),p(3701,b)],vec![
            i(3702,bytes(b"code")),
            i(3703,Op::JsonFieldString(3701,3702)),
            i(3704,bytes(b"Ok")),
            i(3705,Op::EqBytes(3703,3704)),
        ],br(3705,"s30_extra_read",vec![3700],"fail",vec![])),
        blk("s30_extra_read",vec![p(3800,b)],vec![
            i(3801,bytes(b"/public/E001/transform")),
        ],y(WorldCall::Read,vec![3801],"s30_extra_parse",
            vec![3800])),
        blk("s30_extra_parse",vec![p(3900,b),p(3901,b)],vec![
            i(3902,bytes(b"payload")),
            i(3903,Op::JsonFieldBytes(3901,3902)),
            i(3904,bytes(b"held_out_input")),
            i(3905,Op::JsonFieldBytes(3903,3904)),
        ],y(WorldCall::Exec,vec![3900,3905],"s30_extra_result",vec![])),
        blk("s30_extra_result",vec![p(4000,b)],vec![
            i(4001,bytes(b"code")),
            i(4002,Op::JsonFieldString(4000,4001)),
            i(4003,bytes(b"Ok")),
            i(4004,Op::EqBytes(4002,4003)),
        ],br(4004,"s30_extra_good",vec![4000],
            "fail",vec![])),
        blk("s30_extra_good",vec![p(4100,b)],vec![
            i(4101,bytes(b"payload")),
            i(4102,Op::JsonFieldBytes(4100,4101)),
            i(4103,bytes(b"U32")),
            i(4104,Op::JsonFieldJson(4102,4103)),
            i(4105,bytes(OOD_PREDICTION.as_bytes())),
            i(4106,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![4105,4104,4106],
            "s30_extra_saved",vec![])),
        blk("s30_extra_saved",vec![p(4200,b)],vec![
            i(4201,bytes(b"code")),
            i(4202,Op::JsonFieldString(4200,4201)),
            i(4203,bytes(b"Ok")),
            i(4204,Op::EqBytes(4202,4203)),
        ],br(4204,"s30_after",vec![],"fail",vec![])),
    ]);
    root.validate().expect("S30 S6 search + P1/P2 router and repair must typecheck");
    root
}
