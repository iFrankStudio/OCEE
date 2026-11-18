//! S11: a second simulated semantic audit is required before adopting a
//! post-fault or post-semantic-mismatch repair.
//!
//! The two reports use separate world reference implementations, but share
//! researcher control, seed, and publication privilege. No cryptographic
//! authentication or true source independence is claimed. This is a bounded
//! corroboration/abstention mechanism, not general epistemic reliability.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s10_semantic::s10_active_semantic_probe;

fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(raw:&[u8])->Op {Op::ConstBytes{
    hex:raw.iter().map(|v|format!("{v:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn abstain(name:&str,reason:&[u8])->Block{
    block(name,vec![],vec![
        i(5900,lit(b"/self/repair/audit/status")),
        i(5901,lit(reason)),
        i(5902,Op::ConstU32(u32::MAX)),
    ],Terminator::Yield{call:WorldCall::Write,
        args:vec![5900,5901,5902],
        next:"unresolved".into(),next_args:vec![],
    })
}
pub fn s11_corroborated_semantic_repair()->Program {
    let mut program=s10_active_semantic_probe();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];
    let witness=f.blocks.iter_mut().find(|x|x.id=="witness_task")
        .expect("S10 witness scope");
    let Terminator::Branch{if_true,true_args,..}=&mut witness.terminator
        else{panic!("expected S10 scope check")};
    *if_true="audit_read".into();
    assert_eq!(true_args.len(),3);

    f.blocks.extend([
        block("audit_read",vec![p(4000,b),p(4001,b),p(4002,u)],vec![
            i(4003,lit(b"/public/E001/semantic-audit")),
        ],Terminator::Yield{call:WorldCall::Read,args:vec![4003],
            next:"audit_status".into(),next_args:vec![4000,4001,4002]}),
        block("audit_status",vec![p(4100,b),p(4101,b),p(4102,u),p(4103,b)],vec![
            i(4104,lit(b"code")),
            i(4105,Op::JsonFieldString(4103,4104)),
            i(4106,lit(b"Ok")),
            i(4107,Op::EqBytes(4105,4106)),
        ],Terminator::Branch{condition:4107,
            if_true:"audit_payload".into(),true_args:vec![4100,4101,4102,4103],
            if_false:"audit_missing".into(),false_args:vec![],
        }),
        block("audit_payload",vec![p(4200,b),p(4201,b),p(4202,u),p(4203,b)],vec![
            i(4204,lit(b"payload")),
            i(4205,Op::JsonFieldBytes(4203,4204)),
            i(4206,lit(b"count")),
            i(4207,Op::JsonFieldU32(4205,4206)),
            i(4208,Op::ConstU32(1)),
            i(4209,Op::EqU32(4207,4208)),
        ],Terminator::Branch{condition:4209,
            if_true:"audit_value".into(),true_args:vec![4200,4201,4202,4205],
            if_false:"audit_missing".into(),false_args:vec![],
        }),
        block("audit_value",vec![p(4300,b),p(4301,b),p(4302,u),p(4303,b)],vec![
            i(4304,lit(b"first")),
            i(4305,Op::JsonFieldU32(4303,4304)),
            i(4306,Op::EqU32(4305,4302)),
        ],Terminator::Branch{condition:4306,
            if_true:"audit_task".into(),true_args:vec![4300,4301,4302,4303],
            if_false:"audit_mismatch".into(),false_args:vec![],
        }),
        block("audit_task",vec![p(4400,b),p(4401,b),p(4402,u),p(4403,b)],vec![
            i(4404,lit(b"task_id")),
            i(4405,Op::JsonFieldString(4403,4404)),
            i(4406,lit(b"binary-byte-compose-v1")),
            i(4407,Op::EqBytes(4405,4406)),
        ],Terminator::Branch{condition:4407,
            if_true:"audit_scope".into(),true_args:vec![4400,4401,4402,4403],
            if_false:"audit_missing".into(),false_args:vec![],
        }),
        block("audit_scope",vec![p(4500,b),p(4501,b),p(4502,u),p(4503,b)],vec![
            i(4504,lit(b"input")),
            i(4505,Op::JsonFieldBytes(4503,4504)),
            i(4506,lit(&[])),
            i(4507,Op::EqBytes(4505,4506)),
        ],Terminator::Branch{condition:4507,
            if_true:"semantic_verify_original".into(),
                true_args:vec![4500,4501,4502],
            if_false:"audit_missing".into(),false_args:vec![],
        }),
        abstain("audit_mismatch",b"mismatch"),
        abstain("audit_missing",b"missing"),
    ]);
    program.validate().expect("S11 bounded graph valid");
    program
}
/// Control: the second audit is ignored even if available, forcing
/// abstention without a corroborating second reference observation.
pub fn s11_no_secondary_audit_control()->Program{
    let mut program=s11_corroborated_semantic_repair();
    let witness=program.functions[0].blocks.iter_mut()
        .find(|x|x.id=="witness_task").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut witness.terminator
        else{panic!("scope branch missing")};
    *if_true="audit_missing".into();
    true_args.clear();
    program.validate().expect("S11 ablation graph valid");
    program
}
pub fn s11_manifest()->crate::SeedManifest{
    let program=s11_corroborated_semantic_repair();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S11-two-simulated-semantic-source-corroboration",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-finite-program-grammar",
            "S10-human-seeded-empty-input-active-probe",
            "S10-two-correlated-labels-first-source",
            "separate-public-secondary-simulated-audit-source",
            "task-input-and-value-cross-checks",
            "explicit-abstention-on-source-mismatch",
            "same-preseeded-two-guard-repair-vocabulary-and-CAS",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-semantic-answer","secondary-source-private-code",
            "world-seed","holdout-evaluator-answer",
        ],
        research_status:"SYNTHETIC_DUAL_SOURCE_CHECK_NOT_AUTHENTICATED_NOT_GENERAL_TRUTH_NOT_M0",
    }
}
