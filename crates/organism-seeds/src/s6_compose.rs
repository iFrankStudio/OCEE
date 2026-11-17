//! S6: finite *two-node composition* search derived from S5's generic
//! sample comparator and in-world tool adoption machinery.
//!
//! The researcher seeds a finite grammar (three unary primitives and two
//! binary combinators); the agent must enumerate 18 operator triples and
//! validate outputs against environmental examples. Not open-ended synthesis.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall
};
use crate::s5_search::s5_sample_search;

fn lit(data:&[u8])->Op {
    Op::ConstBytes{hex:data.iter().map(|b|format!("{b:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn candidate_skeleton()->Program{
    Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),params:vec![param(0,Type::Bytes)],
                instructions:vec![
                    ins(1,Op::ConstU32(0)),
                    ins(2,Op::Len(0)),
                    ins(3,Op::Len(0)),
                    ins(4,Op::Add(2,3)),
                ],
                terminator:Terminator::Return{value:4},
            }],
        }]
    }
}
pub fn s6_composition_search()->Program {
    let mut p=s5_sample_search();
    let f=&mut p.functions[0];
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    entry.instructions[0].op=lit(b"/public/E001/compose");

    let parse=f.blocks.iter_mut().find(|b|b.id=="parse").unwrap();
    parse.instructions.iter_mut().find(|i|i.id==27).unwrap().op=
        lit(&candidate_skeleton().canonical_bytes());

    let bounded=f.blocks.iter_mut().find(|b|b.id=="loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut bounded.terminator
        else{panic!("bounded search loop")};
    *limit=18;

    let make=f.blocks.iter_mut().find(|b|b.id=="make").unwrap();
    make.params=vec![
        param(200,Type::U32),param(201,Type::Bytes),param(202,Type::Bytes),
        param(203,Type::Bytes),param(204,Type::U32),
        param(205,Type::Bytes),param(206,Type::U32),
        param(207,Type::Bytes),
    ];
    let binaries=serde_json::to_vec(&[
        Op::Add(2,3),Op::Xor(2,3)
    ]).expect("ops serialize");
    make.instructions=vec![
        ins(208,Op::ConstU32(3)),
        ins(209,Op::Mod(200,208)),
        ins(210,Op::Div(200,208)),
        ins(211,Op::Mod(210,208)),
        ins(212,Op::ConstU32(9)),
        ins(213,Op::Div(200,212)),
        ins(214,Op::JsonIndex(202,209)),
        ins(215,Op::JsonIndex(202,211)),
        ins(216,lit(&binaries)),
        ins(217,Op::JsonIndex(216,213)),
        ins(218,lit(b"entry")),
        ins(219,Op::ConstU32(2)),
        ins(220,Op::ConstU32(3)),
        ins(221,Op::ConstU32(4)),
        ins(222,Op::RewriteOp(201,218,219,214)),
        ins(223,Op::RewriteOp(222,218,220,215)),
        ins(224,Op::RewriteOp(223,218,221,217)),
        ins(225,lit(b"OCEE-PROGRAM-V1\0")),
        ins(226,Op::Concat(225,224)),
        ins(227,Op::Sha256(226)),
        ins(228,Op::HexEncode(227)),
        ins(229,lit(b"/self/versions/")),
        ins(230,Op::Concat(229,228)),
        ins(231,Op::ConstU32(u32::MAX)),
    ];
    make.terminator=Terminator::Yield{call:WorldCall::Write,
        args:vec![230,224,231],next:"eval0".into(),
        next_args:vec![200,228,201,202,203,204,205,206,207]};

    p.validate().expect("S6 program must statically validate");
    p
}
/// Ablation: identical search/feedback, but the second unary node is fixed
/// to constant zero. This removes genuine binary-input composition.
pub fn s6_single_operand_control()->Program {
    let mut p=s6_composition_search();
    let block=p.functions[0].blocks.iter_mut().find(|b|b.id=="make").unwrap();
    let op=block.instructions.iter_mut().find(|i|i.id==215).unwrap();
    op.op=lit(&serde_json::to_vec(&Op::ConstU32(0)).expect("op JSON"));
    p.validate().expect("S6 control must validate");
    p
}
pub fn s6_manifest()->crate::SeedManifest {
    let p=s6_composition_search();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S6-two-node-composition-finite-grammar",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "environment-two-example-observation",
            "preseeded-three-unary-operator-grammar",
            "preseeded-Add-and-Xor-binary-operator-grammar",
            "eighteen-candidate-cartesian-search",
            "three-generic-RewriteOp-steps-per-candidate",
            "functional-comparison-and-held-out-reuse",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-composition-rule","selected-operator-triple",
            "held-out-output","evaluator-score",
        ],
        research_status:"BOUNDED_MULTI_NODE_COMPOSITION_FROM_PRESEEDED_GRAMMAR_NOT_E001_M0",
    }
}
