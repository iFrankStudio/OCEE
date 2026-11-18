//! S14: carry an already self-generated S6 tool across fresh simulated
//! episodes, test it against the NEW world's visible examples, and only then
//! reuse it. On mismatch, resume the original finite S6 graph search.
//! This policy and the transfer mechanism are researcher-provided priors;
//! it does NOT discover a protocol, a new primitive, or open-ended evolution.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s6_compose::s6_composition_search;

fn p(id:u32,ty:Type)->Param {Param{id,ty}}
fn i(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn lit(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|v|format!("{v:02x}")).collect()}
}
fn b(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,end:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator:end}
}
fn fallback()->Terminator{Terminator::Jump{target:"restart".into(),args:vec![]}}
pub fn s14_episode_transfer()->Program {
    let mut program=s6_composition_search();
    let f=&mut program.functions[0];
    f.blocks[0].id="cold_start".into();
    let (bytes,num)=(Type::Bytes,Type::U32);
    let mut prefix=vec![
        b("entry",vec![p(3000,bytes)],vec![
            i(3001,lit(b"/self/tools/transform-v1"))
        ],Terminator::Yield{call:WorldCall::Read,args:vec![3001],
            next:"reuse_check".into(),next_args:vec![3000]}),
        b("reuse_check",vec![p(3010,bytes),p(3011,bytes)],vec![
            i(3012,lit(b"code")),i(3013,Op::JsonFieldString(3011,3012)),
            i(3014,lit(b"Ok")),i(3015,Op::EqBytes(3013,3014))
        ],Terminator::Branch{condition:3015,
            if_true:"reuse_task".into(),true_args:vec![3011],
            if_false:"cold_start".into(),false_args:vec![3010]}),
        b("reuse_task",vec![p(3020,bytes)],vec![
            i(3021,lit(b"payload")),i(3022,Op::JsonFieldBytes(3020,3021)),
            i(3023,lit(b"/public/E001/compose"))
        ],Terminator::Yield{call:WorldCall::Read,args:vec![3023],
            next:"reuse_parse".into(),next_args:vec![3022]}),
        b("reuse_parse",vec![p(3030,bytes),p(3031,bytes)],vec![
            i(3032,lit(b"payload")),i(3033,Op::JsonFieldBytes(3031,3032)),
            i(3034,lit(b"examples")),i(3035,Op::JsonFieldJson(3033,3034)),
            i(3036,Op::ConstU32(0)),i(3037,Op::ConstU32(1)),
            i(3038,Op::JsonIndex(3035,3036)),i(3039,Op::JsonIndex(3035,3037)),
            i(3040,lit(b"input")),i(3041,lit(b"output")),
            i(3042,Op::JsonFieldBytes(3038,3040)),
            i(3043,Op::JsonFieldU32(3038,3041)),
            i(3044,Op::JsonFieldBytes(3039,3040)),
            i(3045,Op::JsonFieldU32(3039,3041)),
        ],Terminator::Yield{call:WorldCall::Exec,args:vec![3030,3042],
            next:"reuse_status0".into(),next_args:vec![3030,3043,3044,3045]}),
        b("reuse_status0",vec![
            p(3050,bytes),p(3051,num),p(3052,bytes),p(3053,num),p(3054,bytes)
        ],vec![
            i(3055,lit(b"code")),i(3056,Op::JsonFieldString(3054,3055)),
            i(3057,lit(b"Ok")),i(3058,Op::EqBytes(3056,3057))
        ],Terminator::Branch{condition:3058,
            if_true:"reuse_value0".into(),true_args:vec![3050,3051,3052,3053,3054],
            if_false:"restart".into(),false_args:vec![]}),
        b("reuse_value0",vec![
            p(3060,bytes),p(3061,num),p(3062,bytes),p(3063,num),p(3064,bytes)
        ],vec![
            i(3065,lit(b"payload")),i(3066,Op::JsonFieldBytes(3064,3065)),
            i(3067,lit(b"U32")),i(3068,Op::JsonFieldU32(3066,3067)),
            i(3069,Op::EqU32(3068,3061))
        ],Terminator::Branch{condition:3069,
            if_true:"reuse_second".into(),true_args:vec![3060,3062,3063],
            if_false:"restart".into(),false_args:vec![]}),
        b("reuse_second",vec![p(3070,bytes),p(3071,bytes),p(3072,num)],vec![],
            Terminator::Yield{call:WorldCall::Exec,args:vec![3070,3071],
                next:"reuse_status1".into(),next_args:vec![3072]}),
        b("reuse_status1",vec![p(3080,num),p(3081,bytes)],vec![
            i(3082,lit(b"code")),i(3083,Op::JsonFieldString(3081,3082)),
            i(3084,lit(b"Ok")),i(3085,Op::EqBytes(3083,3084))
        ],Terminator::Branch{condition:3085,
            if_true:"reuse_value1".into(),true_args:vec![3080,3081],
            if_false:"restart".into(),false_args:vec![]}),
        b("reuse_value1",vec![p(3090,num),p(3091,bytes)],vec![
            i(3092,lit(b"payload")),i(3093,Op::JsonFieldBytes(3091,3092)),
            i(3094,lit(b"U32")),i(3095,Op::JsonFieldU32(3093,3094)),
            i(3096,Op::EqU32(3095,3090))
        ],Terminator::Branch{condition:3096,
            if_true:"reused".into(),true_args:vec![],
            if_false:"restart".into(),false_args:vec![]}),
        b("reused",vec![],vec![i(3100,Op::ConstU32(2))],
            Terminator::Return{value:3100}),
        b("restart",vec![],vec![i(3110,lit(b""))],
            Terminator::Jump{target:"cold_start".into(),args:vec![3110]}),
    ];
    // A previously installed tool occupies the same registry as the fresh
    // search would use. Rebinding is CAS(0), not silent failed overwrite.
    let register=f.blocks.iter_mut().find(|block|block.id=="register").unwrap();
    register.instructions=vec![i(1002,lit(b"/self/tools/transform-v1"))];
    register.terminator=Terminator::Yield{call:WorldCall::Read,args:vec![1002],
        next:"register_check".into(),next_args:vec![1000,1001]};
    prefix.extend([
        b("register_check",vec![p(3200,bytes),p(3201,bytes),p(3202,bytes)],vec![
            i(3203,lit(b"code")),i(3204,Op::JsonFieldString(3202,3203)),
            i(3205,lit(b"Ok")),i(3206,Op::EqBytes(3204,3205))
        ],Terminator::Branch{condition:3206,
            if_true:"register_replace".into(),true_args:vec![3200,3201],
            if_false:"register_new".into(),false_args:vec![3200,3201]}),
        b("register_new",vec![p(3210,bytes),p(3211,bytes)],vec![
            i(3212,lit(b"/self/tools/transform-v1")),
            i(3213,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,args:vec![3212,3210,3213],
            next:"apply".into(),next_args:vec![3210,3211]}),
        b("register_replace",vec![p(3220,bytes),p(3221,bytes)],vec![
            i(3222,lit(b"/self/tools/transform-v1")),
            i(3223,Op::ConstU32(0)),
        ],Terminator::Yield{call:WorldCall::Write,args:vec![3222,3220,3223],
            next:"apply".into(),next_args:vec![3220,3221]}),
    ]);
    prefix.extend(std::mem::take(&mut f.blocks));
    f.blocks=prefix;
    program.validate().expect("S14 transfer-aware initial VM program");
    program
}
pub fn s14_manifest()->crate::SeedManifest {
    let program=s14_episode_transfer();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S14-cross-episode-verified-reuse-with-fallback",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "researcher-preseeded-inherited-tool-check",
            "two-public-example-functional-comparison",
            "S6-bounded-finite-grammar-cold-start",
            "CAS-replace-incompatible-tool-reference",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "target-world-hidden-rule","target-held-out-output",
            "target-world-private-protocol-parameters","evaluation-score",
        ],
        research_status:"DEVELOPMENT_EPISODE_TOOL_REUSE_PRESEEDED_POLICY_NOT_E001_M0",
    }
}
