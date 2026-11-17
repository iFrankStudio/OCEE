//! S5: finite *sample-driven* graph operator search performed by one VM organism.
//!
//! Researcher seed: a typed one-node program skeleton, an operator vocabulary,
//! a bounded enumeration strategy, generic JSON feedback parsing and adoption.
//! Not seeded: task's actual rule, training outputs, selected candidate hash,
//! held-out output. No ready-to-run candidate program is installed by the host.
//! This is closed-vocabulary hypothesis selection, NOT novel algorithm invention.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall,
};
fn param(id:u32,ty:Type)->Param {Param{id,ty}}
fn inst(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn bytes(s:&[u8])->Op {
    Op::ConstBytes{hex:s.iter().map(|b|format!("{b:02x}")).collect()}
}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:id.into(),params,instructions,terminator}
}
fn jump(target:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:target.into(),args}
}
fn state(start:u32)->Vec<Param> {
    let types=[
        Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::Bytes,
        Type::U32,Type::Bytes,Type::U32,Type::Bytes,
    ];
    types.iter().enumerate().map(|(i,t)|param(start+i as u32,*t)).collect()
}
fn ids(start:u32)->Vec<u32> {(start..start+9).collect()}
fn loop_state(start:u32)->Vec<Param> {
    let types=[
        Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,
        Type::U32,Type::Bytes,Type::U32,Type::Bytes,
    ];
    types.iter().enumerate().map(|(i,t)|param(start+i as u32,*t)).collect()
}
fn advance_args(start:u32)->Vec<u32> {
    vec![start,start+2,start+3,start+4,start+5,start+6,start+7,start+8]
}
fn base_program()->Program {
    Program{format_version:1,entry_function:"main".into(),functions:vec![
        Function{name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
        blocks:vec![block("entry",vec![param(0,Type::Bytes)],vec![
            inst(1,Op::ConstU32(0)),
            inst(2,Op::Len(0)), // generic initial skeleton, not the answer
        ],Terminator::Return{value:2})]}
    ]}
}
/// Vocabulary is a small, explicitly seeded and type-compatible list.
/// The order is fixed across tasks and does not depend on hidden rule or seed.
pub fn operator_vocabulary()->Vec<Op> {
    vec![Op::Len(0),Op::XorFold(0),Op::ByteAt(0,1)]
}
pub fn s5_sample_search()->Program {
    let base=base_program();
    base.validate().expect("base program");
    let vocabulary=operator_vocabulary();
    let candidates=serde_json::to_vec(&vocabulary).expect("operations serialize");
    let (b,u)=(Type::Bytes,Type::U32);
    let blocks=vec![
        block("entry",vec![param(0,b)],vec![
            inst(1,bytes(b"/public/E001/transform")),
        ],Terminator::Yield{call:WorldCall::Read,args:vec![1],
            next:"parse".into(),next_args:vec![]}),
        block("parse",vec![param(10,b)],vec![
            inst(11,bytes(b"payload")),
            inst(12,Op::JsonFieldBytes(10,11)),
            inst(13,bytes(b"examples")),
            inst(14,Op::JsonFieldJson(12,13)),
            inst(15,Op::ConstU32(0)),
            inst(16,Op::ConstU32(1)),
            inst(17,Op::JsonIndex(14,15)),
            inst(18,Op::JsonIndex(14,16)),
            inst(19,bytes(b"input")),
            inst(20,bytes(b"output")),
            inst(21,Op::JsonFieldBytes(17,19)),
            inst(22,Op::JsonFieldU32(17,20)),
            inst(23,Op::JsonFieldBytes(18,19)),
            inst(24,Op::JsonFieldU32(18,20)),
            inst(25,bytes(b"held_out_input")),
            inst(26,Op::JsonFieldBytes(12,25)),
            inst(27,bytes(&base.canonical_bytes())),
            inst(28,bytes(&candidates)),
            inst(29,Op::ConstU32(0)),
        ],jump("loop",vec![29,27,28,21,22,23,24,26])),
        block("loop",loop_state(100),vec![],
            Terminator::RepeatBounded{limit:3,body:"make".into(),
                body_args:(100..108).collect(),done:"fail".into(),done_args:vec![]}),
        block("make",loop_state(200),vec![
            inst(208,Op::JsonIndex(202,200)),
            inst(209,bytes(b"entry")),
            inst(210,Op::ConstU32(2)),
            inst(211,Op::RewriteOp(201,209,210,208)),
            inst(212,bytes(b"OCEE-PROGRAM-V1\0")),
            inst(213,Op::Concat(212,211)),
            inst(214,Op::Sha256(213)),
            inst(215,Op::HexEncode(214)),
            inst(216,bytes(b"/self/versions/")),
            inst(217,Op::Concat(216,215)),
            inst(218,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![217,211,218],next:"eval0".into(),
            next_args:vec![200,215,201,202,203,204,205,206,207]}),
        block("eval0",{
            let mut v=state(300);v.push(param(309,b));v
        },vec![],Terminator::Yield{call:WorldCall::Exec,args:vec![301,304],
            next:"status0".into(),next_args:ids(300)}),
        block("status0",{
            let mut v=state(400);v.push(param(409,b));v
        },vec![
            inst(410,bytes(b"code")),
            inst(411,Op::JsonFieldString(409,410)),
            inst(412,bytes(b"Ok")),
            inst(413,Op::EqBytes(411,412)),
        ],Terminator::Branch{condition:413,
            if_true:"score0".into(),true_args:{
                let mut v=ids(400);v.push(409);v
            },if_false:"advance".into(),false_args:advance_args(400)}),
        block("score0",{
            let mut v=state(500);v.push(param(509,b));v
        },vec![
            inst(510,bytes(b"payload")),
            inst(511,Op::JsonFieldBytes(509,510)),
            inst(512,bytes(b"U32")),
            inst(513,Op::JsonFieldU32(511,512)),
            inst(514,Op::EqU32(513,505)),
        ],Terminator::Branch{condition:514,
            if_true:"eval1".into(),true_args:ids(500),
            if_false:"advance".into(),false_args:advance_args(500)}),
        block("eval1",state(600),vec![],
            Terminator::Yield{call:WorldCall::Exec,args:vec![601,606],
                next:"status1".into(),next_args:ids(600)}),
        block("status1",{
            let mut v=state(700);v.push(param(709,b));v
        },vec![
            inst(710,bytes(b"code")),
            inst(711,Op::JsonFieldString(709,710)),
            inst(712,bytes(b"Ok")),
            inst(713,Op::EqBytes(711,712)),
        ],Terminator::Branch{condition:713,
            if_true:"score1".into(),true_args:{
                let mut v=ids(700);v.push(709);v
            },if_false:"advance".into(),false_args:advance_args(700)}),
        block("score1",{
            let mut v=state(800);v.push(param(809,b));v
        },vec![
            inst(810,bytes(b"payload")),
            inst(811,Op::JsonFieldBytes(809,810)),
            inst(812,bytes(b"U32")),
            inst(813,Op::JsonFieldU32(811,812)),
            inst(814,Op::EqU32(813,807)),
        ],Terminator::Branch{condition:814,
            if_true:"register".into(),true_args:vec![801,808],
            if_false:"advance".into(),false_args:advance_args(800)}),
        block("advance",loop_state(900),vec![
            inst(908,Op::ConstU32(1)),
            inst(909,Op::Add(900,908)),
        ],jump("loop",vec![909,901,902,903,904,905,906,907])),
        block("register",vec![param(1000,b),param(1001,b)],vec![
            inst(1002,bytes(b"/self/tools/transform-v1")),
            inst(1003,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,args:vec![1002,1000,1003],
            next:"apply".into(),next_args:vec![1000,1001]}),
        block("apply",vec![param(1100,b),param(1101,b),param(1102,b)],vec![],
            Terminator::Yield{call:WorldCall::Exec,args:vec![1100,1101],
                next:"save".into(),next_args:vec![]}),
        block("save",vec![param(1200,b)],vec![
            inst(1201,bytes(b"/self/results/transform-held-out")),
            inst(1202,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,args:vec![1201,1200,1202],
            next:"done".into(),next_args:vec![]}),
        block("done",vec![param(1300,b)],vec![
            inst(1301,Op::ConstU32(1)),
        ],Terminator::Return{value:1301}),
        block("fail",vec![],vec![
            inst(1400,Op::ConstU32(0)),
        ],Terminator::Return{value:1400}),
    ];
    let program=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{name:"main".into(),args:vec![b],
            returns:u,blocks}]};
    program.validate().expect("S5 program must be valid");
    program
}

/// Closed-vocabulary ablation: only the first operator is tried. Other state,
/// training examples and runtime infrastructure stay unchanged.
pub fn s5_one_candidate_control()->Program {
    let mut program=s5_sample_search();
    let loop_block=program.functions[0].blocks.iter_mut()
        .find(|b|b.id=="loop").expect("loop block");
    let Terminator::RepeatBounded{limit,..}=&mut loop_block.terminator
        else {panic!("expected bounded loop")};
    *limit=1;
    program.validate().expect("valid ablation");
    program
}
pub fn s5_manifest()->crate::SeedManifest {
    let program=s5_sample_search();
    crate::SeedManifest{
        manifest_version:1,kind:"S5-sample-driven-closed-vocabulary-search",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "public-input-output-example-observation",
            "preseeded-Len-XorFold-ByteAt-vocabulary",
            "generic-typed-program-node-rewriting",
            "three-candidate-bounded-search",
            "two-sample-phenotype-evaluation",
            "selected-tool-registration",
            "unlabelled-held-out-tool-application",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-transform-rule","hidden-rule-index","held-out-answer",
            "evaluation-score","private-protocol-seed"
        ],
        research_status:"CLOSED_VOCABULARY_OPERATOR_SELECTION_NOT_NOVEL_ALGORITHM_NOT_E001",
    }
}
