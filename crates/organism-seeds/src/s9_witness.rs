//! S9: select a repair using *new public environmental witness evidence*,
//! not a correct answer embedded in the initial S8 VM comparator.
//!
//! This is still a researcher-created supervised task: a simulated authority
//! publishes two deterministic boundary observations ONLY AFTER a registered
//! tool actually faults on empty input. Observations may agree on 0 or 1,
//! be absent, or contradict one another. The same initial VM seed adopts the
//! matching candidate only when two observations agree and regression passes.
//! Both the two-candidate vocabulary and guard program skeleton remain seeded.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s8_repair_search::s8_two_candidate_repair;

fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(value:&[u8])->Op{Op::ConstBytes{
    hex:value.iter().map(|x|format!("{x:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn unknown(name:&str,reason:&[u8])->Block {
    block(name,vec![],vec![
        i(2700,l(b"/self/repair/boundary/status")),
        i(2701,l(reason)),
        i(2702,Op::ConstU32(u32::MAX))
    ],Terminator::Yield{
        call:WorldCall::Write,args:vec![2700,2701,2702],
        next:"unresolved".into(),next_args:vec![],
    })
}
/// One complete VM episode: S6 -> faulty tool -> boundary witness -> S8
/// finite program editing with an environment-derived target, or explicit
/// unresolved result when witnesses are missing or contradict one another.
pub fn s9_external_witness_repair()->Program {
    let mut program=s8_two_candidate_repair();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];

    let read=f.blocks.iter_mut().find(|x|x.id=="read_broken").unwrap();
    let Terminator::Yield{next,..}=&mut read.terminator else{unreachable!()};
    *next="read_witness".into(); // feedback from original tool READ carries its bytes

    for node in &mut f.blocks {
        match node.id.as_str() {
            "start_search"=>{
                *node=block("start_search",vec![
                    p(2300,b),p(2301,b),p(2305,u),
                ],vec![
                    i(2302,l(b"payload")),
                    i(2303,Op::JsonFieldBytes(2301,2302)),
                    i(2304,Op::ConstU32(0)),
                ],Terminator::Jump{target:"repair_loop".into(),
                    args:vec![2300,2303,2304,2305]});
            },
            "repair_loop"=>{
                *node=block("repair_loop",vec![
                    p(2400,b),p(2401,b),p(2402,u),p(2403,u),
                ],vec![],Terminator::RepeatBounded{
                    limit:2,body:"construct".into(),
                    body_args:vec![2400,2401,2402,2403],
                    done:"fail".into(),done_args:vec![],
                });
            },
            "construct"=>{
                node.params.push(p(2600,u));
                let Terminator::Yield{next_args,..}=&mut node.terminator
                    else{unreachable!()};
                next_args.push(2600);
            },
            "validate_empty"=>{
                node.params.insert(4,p(2601,u));
                let Terminator::Yield{next_args,..}=&mut node.terminator
                    else{unreachable!()};
                next_args.push(2601);
            },
            "empty_status"=>{
                node.params.insert(4,p(2602,u));
                let Terminator::Branch{true_args,false_args,..}=&mut node.terminator
                    else{unreachable!()};
                true_args.insert(4,2602);
                false_args.push(2602);
            },
            "empty_value"=>{
                node.params.insert(4,p(2603,u));
                // Removing the S8 hard-coded target-zero comparison is
                // essential: target now arrives from the external witness.
                node.instructions.retain(|x|x.id!=2109);
                let equal=node.instructions.iter_mut().find(|x|x.id==2110).unwrap();
                equal.op=Op::EqU32(2108,2603);
                let Terminator::Branch{false_args,..}=&mut node.terminator
                    else{unreachable!()};
                false_args.push(2603);
            },
            "advance_repair"=>{
                node.params.push(p(2604,u));
                let Terminator::Jump{args,..}=&mut node.terminator
                    else{unreachable!()};
                args.push(2604);
            },
            _=>{}
        }
    }
    f.blocks.extend([
        block("read_witness",vec![p(3000,b),p(3001,b)],vec![
            i(3002,l(b"/public/E001/boundary-witness")),
        ],Terminator::Yield{call:WorldCall::Read,args:vec![3002],
            next:"witness_status".into(),next_args:vec![3000,3001]}),
        block("witness_status",vec![p(3010,b),p(3011,b),p(3012,b)],vec![
            i(3013,l(b"code")),
            i(3014,Op::JsonFieldString(3012,3013)),
            i(3015,l(b"Ok")),
            i(3016,Op::EqBytes(3014,3015)),
        ],Terminator::Branch{condition:3016,
            if_true:"witness_count".into(),true_args:vec![3010,3011,3012],
            if_false:"missing_evidence".into(),false_args:vec![]}),
        block("witness_count",vec![p(3020,b),p(3021,b),p(3022,b)],vec![
            i(3023,l(b"payload")),
            i(3024,Op::JsonFieldBytes(3022,3023)),
            i(3025,l(b"count")),
            i(3026,Op::JsonFieldU32(3024,3025)),
            i(3027,Op::ConstU32(2)),
            i(3028,Op::EqU32(3026,3027)),
        ],Terminator::Branch{condition:3028,
            if_true:"witness_compare".into(),true_args:vec![3020,3021,3024],
            if_false:"missing_evidence".into(),false_args:vec![]}),
        block("witness_compare",vec![p(3030,b),p(3031,b),p(3032,b)],vec![
            i(3033,l(b"first")),
            i(3034,l(b"second")),
            i(3035,Op::JsonFieldU32(3032,3033)),
            i(3036,Op::JsonFieldU32(3032,3034)),
            i(3037,Op::EqU32(3035,3036)),
        ],Terminator::Branch{condition:3037,
            if_true:"witness_scope".into(),true_args:vec![3030,3031,3032,3035],
            if_false:"conflicting_evidence".into(),false_args:vec![]}),
        block("witness_scope",vec![p(3040,b),p(3041,b),p(3042,b),p(3043,u)],vec![
            i(3044,l(b"input")),
            i(3045,Op::JsonFieldBytes(3042,3044)),
            i(3046,l(&[])),
            i(3047,Op::EqBytes(3045,3046)),
        ],Terminator::Branch{condition:3047,
            if_true:"witness_task".into(),true_args:vec![3040,3041,3042,3043],
            if_false:"missing_evidence".into(),false_args:vec![]}),
        block("witness_task",vec![p(3050,b),p(3051,b),p(3052,b),p(3053,u)],vec![
            i(3054,l(b"task_id")),
            i(3055,Op::JsonFieldString(3052,3054)),
            i(3056,l(b"binary-byte-compose-v1")),
            i(3057,Op::EqBytes(3055,3056)),
        ],Terminator::Branch{condition:3057,
            if_true:"start_search".into(),true_args:vec![3050,3051,3053],
            if_false:"missing_evidence".into(),false_args:vec![]}),
        unknown("missing_evidence",b"missing"),
        unknown("conflicting_evidence",b"conflict"),
        block("unresolved",vec![p(3060,b)],vec![
            i(3061,Op::ConstU32(3)),
        ],Terminator::Return{value:3061}),
    ]);
    program.validate().expect("S9 program type/control-flow valid");
    program
}

/// Mechanism ablation: without consulting the external witness, the organism
/// cannot make a justified post-fault adoption. Do not silently fall back to 0.
pub fn s9_no_witness_control()->Program {
    let mut program=s9_external_witness_repair();
    let b=program.functions[0].blocks.iter_mut()
        .find(|x|x.id=="read_broken").unwrap();
    let Terminator::Yield{next,..}=&mut b.terminator else{unreachable!()};
    *next="skip_witness".into();
    // A Yield always appends one feedback argument. Keep the original hash
    // as the first argument and explicitly discard both in the skip block.
    program.functions[0].blocks.push(block("skip_witness",vec![
        p(3090,Type::Bytes),p(3091,Type::Bytes)
    ],vec![],Terminator::Jump{
        target:"missing_evidence".into(),args:vec![]
    }));
    program.validate().expect("S9 no-witness ablation must type-check");
    program
}
pub fn s9_manifest()->crate::SeedManifest {
    let program=s9_external_witness_repair();
    crate::SeedManifest {
        manifest_version:1,
        kind:"S9-post-fault-external-two-witness-repair-selection",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-compositional-search",
            "S7-explicit-empty-input-fault-probe",
            "S8-two-candidate-guard-grammar-1-and-0",
            "public-two-observation-witness-consensus-gate",
            "scope-check-for-witness-task-and-input",
            "unknown-state-if-missing-or-contradictory",
            "generic-graph-edits-with-sample-regression-and-CAS",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-boundary-answer","world-seed-mode",
            "independent-evaluator-target","private-service-parameters",
        ],
        research_status:"SIMULATED_EXTERNAL_SUPERVISION_AND_ABSTENTION_NOT_AUTONOMOUS_TRUTH_NOT_M0",
    }
}
