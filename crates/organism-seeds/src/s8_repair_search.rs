//! S8: a bounded, in-world search over *two* type-correct guard proposals
//! after an already adopted S6 tool fails on an empty input.
//!
//! The repair vocabulary [return 1, return 0] and comparison to zero are
//! researcher-seeded priors. Crucially, the first proposal is WRONG, yet
//! executable; only its measured output rejects it. The same active VM then
//! constructs a second version via generic graph editing and chooses it.
//! No host special-cases the fault or installs the corrected tool.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Instruction,Op,Param,Terminator,Type,WorldCall,Program,
};
use crate::s7_recover::s7_empty_input_recovery;
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(bytes:&[u8])->Op {Op::ConstBytes{
    hex:bytes.iter().map(|x|format!("{x:02x}")).collect()
}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}

/// Both search candidates are generated and checked by this *one* VM program.
/// The first trial always returns 1 on the discovered empty input, the second
/// returns 0. The output-value comparator is intentionally unchanged from S7.
pub fn s8_two_candidate_repair()->Program{
    let mut program=s7_empty_input_recovery();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];

    // A generic initial guard with the wrong behavior. Nothing in this block
    // contains the correct repair value; it will come from the search vocabulary.
    let original=f.blocks.iter().find(|x|x.id=="construct").unwrap();
    let ins=original.instructions.iter().find(|x|x.id==2064).unwrap();
    let Op::ConstBytes{hex}=&ins.op else{panic!("S7 guard missing")};
    let old=(0..hex.len()).step_by(2).map(|j|
        u8::from_str_radix(&hex[j..j+2],16).expect("hex")
    ).collect::<Vec<_>>();
    let mut templates:Vec<Block>=serde_json::from_slice(&old).unwrap();
    let empty=templates.iter_mut().find(|x|x.id=="empty").unwrap();
    empty.instructions[0].op=Op::ConstU32(1);
    let incorrect_template=serde_json::to_vec(&templates).unwrap();
    let candidates=serde_json::to_vec(&[
        Op::ConstU32(1),Op::ConstU32(0)
    ]).unwrap();

    let reader=f.blocks.iter_mut().find(|x|x.id=="read_broken").unwrap();
    let Terminator::Yield{next,..}=&mut reader.terminator
        else{panic!("S7 read_broken expected")};
    *next="start_search".into();

    for block in &mut f.blocks {
        let id=block.id.as_str();
        match id {
            "construct" => {
                *block=block_fn_construct(&incorrect_template,&candidates,b,u);
            },
            "validate_empty" => {
                *block=block("validate_empty",vec![
                    p(2080,b),p(2081,b),p(2082,b),p(2083,u),p(2084,b)
                ],vec![i(2085,l(&[]))],Terminator::Yield{
                    call:WorldCall::Exec,args:vec![2080,2085],
                    next:"empty_status".into(),
                    next_args:vec![2080,2081,2082,2083]
                });
            },
            "empty_status" => {
                *block=block("empty_status",vec![
                    p(2090,b),p(2091,b),p(2092,b),p(2093,u),p(2094,b)
                ],vec![
                    i(2095,l(b"code")),
                    i(2096,Op::JsonFieldString(2094,2095)),
                    i(2097,l(b"Ok")),
                    i(2098,Op::EqBytes(2096,2097)),
                ],Terminator::Branch{condition:2098,
                    if_true:"empty_value".into(),
                    true_args:vec![2090,2091,2092,2093,2094],
                    if_false:"advance_repair".into(),
                    false_args:vec![2091,2092,2093]
                });
            },
            "empty_value" => {
                *block=block("empty_value",vec![
                    p(2100,b),p(2101,b),p(2102,b),p(2103,u),p(2104,b)
                ],vec![
                    i(2105,l(b"payload")),
                    i(2106,Op::JsonFieldBytes(2104,2105)),
                    i(2107,l(b"U32")),
                    i(2108,Op::JsonFieldU32(2106,2107)),
                    i(2109,Op::ConstU32(0)),
                    i(2110,Op::EqU32(2108,2109)),
                ],Terminator::Branch{condition:2110,
                    if_true:"read_training".into(),
                    true_args:vec![2100,2101],
                    if_false:"advance_repair".into(),
                    false_args:vec![2101,2102,2103]
                });
            },
            _=>{}
        }
    }
    f.blocks.extend([
        block("start_search",vec![p(2300,b),p(2301,b)],vec![
            i(2302,l(b"payload")),
            i(2303,Op::JsonFieldBytes(2301,2302)),
            i(2304,Op::ConstU32(0)),
        ],Terminator::Jump{target:"repair_loop".into(),
            args:vec![2300,2303,2304]}),
        block("repair_loop",vec![p(2400,b),p(2401,b),p(2402,u)],
            vec![],Terminator::RepeatBounded{
                limit:2,body:"construct".into(),
                body_args:vec![2400,2401,2402],
                done:"fail".into(),done_args:vec![],
            }),
        block("advance_repair",vec![p(2500,b),p(2501,b),p(2502,u)],
            vec![
                i(2503,Op::ConstU32(1)),
                i(2504,Op::Add(2502,2503)),
            ],Terminator::Jump{target:"repair_loop".into(),
                args:vec![2500,2501,2504]}),
    ]);
    program.validate().expect("S8 seed must be a valid bounded graph");
    program
}
fn block_fn_construct(template:&[u8],candidates:&[u8],b:Type,u:Type)->Block{
    block("construct",vec![p(2060,b),p(2061,b),p(2062,u)],vec![
        i(2063,l(template)),
        i(2064,Op::PrependBlocks(2061,2063)),
        i(2065,l(candidates)),
        i(2066,Op::JsonIndex(2065,2062)),
        i(2067,l(b"empty")),
        i(2068,Op::ConstU32(1)),
        i(2069,Op::RewriteOp(2064,2067,2068,2066)),
        i(2070,l(b"OCEE-PROGRAM-V1\0")),
        i(2071,Op::Concat(2070,2069)),
        i(2072,Op::Sha256(2071)),
        i(2073,Op::HexEncode(2072)),
        i(2074,l(b"/self/versions/")),
        i(2075,Op::Concat(2074,2073)),
        i(2076,Op::ConstU32(u32::MAX)),
    ],Terminator::Yield{call:WorldCall::Write,
        args:vec![2075,2069,2076],
        next:"validate_empty".into(),
        next_args:vec![2073,2060,2061,2062]})
}

/// Ablation: one trial only, so the wrong-yet-executable first proposal can
/// never be mistaken for a successfully repaired and adopted tool.
pub fn s8_one_repair_candidate_control()->Program{
    let mut p=s8_two_candidate_repair();
    let b=p.functions[0].blocks.iter_mut().find(|x|x.id=="repair_loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut b.terminator else{unreachable!()};
    *limit=1;
    p.validate().expect("S8 single-trial ablation must validate");
    p
}
pub fn s8_manifest()->crate::SeedManifest{
    let p=s8_two_candidate_repair();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S8-bounded-post-fault-two-guard-repair-search",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-finite-grammar-tool-search",
            "S7-explicit-empty-input-probe",
            "preseeded-guard-block-structure",
            "preseeded-return-1-or-return-0-repair-vocabulary",
            "two-trial-bounded-RewriteOp-and-PrependBlocks-search",
            "in-world-EXEC-phenotype-validation-and-CAS-adoption",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "private-seed-rule","held-out-target-output","evaluator-score",
        ],
        research_status:"FINITE_PRESEEDED_REPAIR_VOCABULARY_SELECTION_NOT_NOVEL_REPAIR_NOT_M0",
    }
}
