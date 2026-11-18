//! S7: detect an untrained empty-input failure of an already adopted S6 tool,
//! propose a *control-flow* guard through a generic graph-editing VM operation,
//! verify both the failing input and a public nonempty regression sample, then
//! atomically update the tool reference. No researcher changes the active seed.
//!
//! IMPORTANT PRIORS: researcher supplied the general "empty input -> zero"
//! guard algorithm, the empty probe, and the change/validation strategy.
//! This demonstrates in-world repair/rollback gates, NOT independent discovery
//! of a novel program structure or open-ended evolution.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall,
};
use crate::s6_compose::s6_composition_search;

fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(data:&[u8])->Op {
    Op::ConstBytes{hex:data.iter().map(|v|format!("{v:02x}")).collect()}
}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:id.into(),params,instructions,terminator}
}
fn guard_blocks()->Vec<Block> {
    // A generic typed control-flow guard, not a special VM host operation.
    // A newly inserted entry block branches before the unsafe old entry.
    vec![
        block("guard",vec![param(0,Type::Bytes)],vec![
            ins(1,Op::Len(0)),
            ins(2,Op::ConstU32(0)),
            ins(3,Op::EqU32(1,2)),
        ],Terminator::Branch{condition:3,
            if_true:"empty".into(),true_args:vec![],
            if_false:"entry".into(),false_args:vec![0]}),
        block("empty",vec![],vec![
            ins(1,Op::ConstU32(0)),
        ],Terminator::Return{value:1}),
    ]
}
/// S7 runs S6's entire finite search first, retaining its tool and audit.
/// It *only* attempts a repair after executing the selected tool on [] and
/// receiving a real world-visible InvalidCode result.
pub fn s7_empty_input_recovery()->Program {
    let mut p=s6_composition_search();
    let f:&mut Function=&mut p.functions[0];
    let (b,u)=(Type::Bytes,Type::U32);
    let apply=f.blocks.iter_mut().find(|bl|bl.id=="apply").unwrap();
    let Terminator::Yield{next_args,..}=&mut apply.terminator
        else {panic!("expected S6 post-selection EXEC")};
    *next_args=vec![1100]; // carry the selected content hash forward

    let save=f.blocks.iter_mut().find(|bl|bl.id=="save").unwrap();
    *save=block("save",vec![param(1200,b),param(1201,b)],vec![
        ins(1202,lit(b"/self/results/transform-held-out")),
        ins(1203,Op::ConstU32(u32::MAX)),
    ],Terminator::Yield{call:WorldCall::Write,
        args:vec![1202,1201,1203],
        next:"probe_empty".into(),next_args:vec![1200]});

    f.blocks.extend([
        block("probe_empty",vec![param(2000,b),param(2001,b)],vec![
            ins(2002,lit(&[])),
        ],Terminator::Yield{call:WorldCall::Exec,
            args:vec![2000,2002],next:"assess_empty".into(),next_args:vec![2000]}),
        block("assess_empty",vec![param(2010,b),param(2011,b)],vec![
            ins(2012,lit(b"code")),
            ins(2013,Op::JsonFieldString(2011,2012)),
            ins(2014,lit(b"Ok")),
            ins(2015,Op::EqBytes(2013,2014)),
        ],Terminator::Branch{condition:2015,
            if_true:"check_empty_value".into(),true_args:vec![2010,2011],
            if_false:"is_fault".into(),false_args:vec![2010,2011]}),
        block("check_empty_value",vec![param(2020,b),param(2021,b)],vec![
            ins(2022,lit(b"payload")),
            ins(2023,Op::JsonFieldBytes(2021,2022)),
            ins(2024,lit(b"U32")),
            ins(2025,Op::JsonFieldU32(2023,2024)),
            ins(2026,Op::ConstU32(0)),
            ins(2027,Op::EqU32(2025,2026)),
        ],Terminator::Branch{condition:2027,
            if_true:"no_repair".into(),true_args:vec![],
            if_false:"record_fault".into(),false_args:vec![2020,2021]}),
        block("is_fault",vec![param(2030,b),param(2031,b)],vec![
            ins(2032,lit(b"code")),
            ins(2033,Op::JsonFieldString(2031,2032)),
            ins(2034,lit(b"InvalidCode")),
            ins(2035,Op::EqBytes(2033,2034)),
        ],Terminator::Branch{condition:2035,
            if_true:"record_fault".into(),true_args:vec![2030,2031],
            if_false:"fail".into(),false_args:vec![]}),
        block("record_fault",vec![param(2040,b),param(2041,b)],vec![
            ins(2042,lit(b"/self/repair/empty/probe")),
            ins(2043,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![2042,2041,2043],
            next:"read_broken".into(),next_args:vec![2040]}),
        block("read_broken",vec![param(2050,b),param(2051,b)],vec![
            ins(2052,lit(b"/self/versions/")),
            ins(2053,Op::Concat(2052,2050)),
        ],Terminator::Yield{call:WorldCall::Read,args:vec![2053],
            next:"construct".into(),next_args:vec![2050]}),
        block("construct",vec![param(2060,b),param(2061,b)],vec![
            ins(2062,lit(b"payload")),
            ins(2063,Op::JsonFieldBytes(2061,2062)),
            ins(2064,lit(&serde_json::to_vec(&guard_blocks())
                .expect("guard blocks are serializable"))),
            ins(2065,Op::PrependBlocks(2063,2064)),
            ins(2066,lit(b"OCEE-PROGRAM-V1\0")),
            ins(2067,Op::Concat(2066,2065)),
            ins(2068,Op::Sha256(2067)),
            ins(2069,Op::HexEncode(2068)),
            ins(2070,lit(b"/self/versions/")),
            ins(2071,Op::Concat(2070,2069)),
            ins(2072,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![2071,2065,2072],
            next:"validate_empty".into(),next_args:vec![2069,2060]}),
        block("validate_empty",vec![param(2080,b),param(2081,b),param(2082,b)],vec![
            ins(2083,lit(&[])),
        ],Terminator::Yield{call:WorldCall::Exec,args:vec![2080,2083],
            next:"empty_status".into(),next_args:vec![2080,2081]}),
        block("empty_status",vec![param(2090,b),param(2091,b),param(2092,b)],vec![
            ins(2093,lit(b"code")),
            ins(2094,Op::JsonFieldString(2092,2093)),
            ins(2095,lit(b"Ok")),
            ins(2096,Op::EqBytes(2094,2095)),
        ],Terminator::Branch{condition:2096,
            if_true:"empty_value".into(),true_args:vec![2090,2091,2092],
            if_false:"fail".into(),false_args:vec![]}),
        block("empty_value",vec![param(2100,b),param(2101,b),param(2102,b)],vec![
            ins(2103,lit(b"payload")),
            ins(2104,Op::JsonFieldBytes(2102,2103)),
            ins(2105,lit(b"U32")),
            ins(2106,Op::JsonFieldU32(2104,2105)),
            ins(2107,Op::ConstU32(0)),
            ins(2108,Op::EqU32(2106,2107)),
        ],Terminator::Branch{condition:2108,
            if_true:"read_training".into(),true_args:vec![2100,2101],
            if_false:"fail".into(),false_args:vec![]}),
        block("read_training",vec![param(2110,b),param(2111,b)],vec![
            ins(2112,lit(b"/public/E001/compose")),
        ],Terminator::Yield{call:WorldCall::Read,args:vec![2112],
            next:"training_parse".into(),next_args:vec![2110,2111]}),
        block("training_parse",vec![param(2120,b),param(2121,b),param(2122,b)],vec![
            ins(2123,lit(b"payload")),
            ins(2124,Op::JsonFieldBytes(2122,2123)),
            ins(2125,lit(b"examples")),
            ins(2126,Op::JsonFieldJson(2124,2125)),
            ins(2127,Op::ConstU32(0)),
            ins(2128,Op::JsonIndex(2126,2127)),
            ins(2129,lit(b"input")),
            ins(2130,Op::JsonFieldBytes(2128,2129)),
            ins(2131,lit(b"output")),
            ins(2132,Op::JsonFieldU32(2128,2131)),
        ],Terminator::Yield{call:WorldCall::Exec,args:vec![2120,2130],
            next:"regression_status".into(),next_args:vec![2120,2121,2132]}),
        block("regression_status",vec![
            param(2140,b),param(2141,b),param(2142,u),param(2143,b),
        ],vec![
            ins(2144,lit(b"code")),
            ins(2145,Op::JsonFieldString(2143,2144)),
            ins(2146,lit(b"Ok")),
            ins(2147,Op::EqBytes(2145,2146)),
        ],Terminator::Branch{condition:2147,
            if_true:"regression_value".into(),
                true_args:vec![2140,2141,2142,2143],
            if_false:"fail".into(),false_args:vec![]}),
        block("regression_value",vec![
            param(2150,b),param(2151,b),param(2152,u),param(2153,b),
        ],vec![
            ins(2154,lit(b"payload")),
            ins(2155,Op::JsonFieldBytes(2153,2154)),
            ins(2156,lit(b"U32")),
            ins(2157,Op::JsonFieldU32(2155,2156)),
            ins(2158,Op::EqU32(2157,2152)),
        ],Terminator::Branch{condition:2158,
            if_true:"adopt".into(),true_args:vec![2150],
            if_false:"fail".into(),false_args:vec![]}),
        block("adopt",vec![param(2160,b)],vec![
            ins(2161,lit(b"/self/tools/transform-v1")),
            ins(2162,Op::ConstU32(0)), // CAS expected registry object version
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![2161,2160,2162],
            next:"adoption_status".into(),next_args:vec![]}),
        block("adoption_status",vec![param(2170,b)],vec![
            ins(2171,lit(b"code")),
            ins(2172,Op::JsonFieldString(2170,2171)),
            ins(2173,lit(b"Ok")),
            ins(2174,Op::EqBytes(2172,2173)),
        ],Terminator::Branch{condition:2174,
            if_true:"repaired".into(),true_args:vec![],
            if_false:"fail".into(),false_args:vec![]}),
        block("no_repair",vec![],vec![ins(2180,Op::ConstU32(1))],
            Terminator::Return{value:2180}),
        block("repaired",vec![],vec![ins(2190,Op::ConstU32(2))],
            Terminator::Return{value:2190}),
    ]);
    p.validate().expect("S7 seed must be statically valid");
    p
}
/// Ablation: observe and record the same actual failure, but disable the
/// generic graph-repair branch. Original tool must remain unchanged.
pub fn s7_no_repair_control()->Program {
    let mut p=s7_empty_input_recovery();
    let f=&mut p.functions[0];
    let after=f.blocks.iter_mut().find(|bl|bl.id=="record_fault").unwrap();
    after.terminator=Terminator::Yield{
        call:WorldCall::Write,args:vec![2042,2041,2043],
        next:"ablation_done".into(),next_args:vec![],
    };
    f.blocks.push(block("ablation_done",vec![param(2200,b)],vec![
        ins(2201,Op::ConstU32(0)),
    ],Terminator::Return{value:2201}));
    p.validate().expect("S7 ablation program must validate");
    p
}
pub fn s7_manifest()->crate::SeedManifest {
    let p=s7_empty_input_recovery();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S7-post-adoption-empty-input-guard-repair",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-closed-grammar-initial-search",
            "explicit-post-adoption-empty-input-probe",
            "world-feedback-InvalidCode-detection",
            "preseeded-empty-is-zero-guard-hypothesis",
            "generic-typed-PrependBlocks-graph-edit",
            "isolated-empty-and-public-nonempty-regression-EXEC",
            "versioned-candidate-storage-and-CAS-tool-adoption",
            "failure-evidence-persistence",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-composition-rule","private-seed-solution",
            "independent-evaluator-answer","held-out-target-output",
        ],
        research_status:"SEEDED_GUARD_HYPOTHESIS_FEEDBACK_TRIGGERED_CONTROL_FLOW_REPAIR_NOT_M0",
    }
}
