//! S10: explicitly test an already adopted tool and read *new environmental
//! semantic feedback*, even when the tool returned Ok. A successful program
//! execution is not evidence of correct output.
//!
//! The chosen probe [] and the two-candidate guard vocabulary remain seeded.
//! The world offers a correlated simulated evaluator, not an authentic pair
//! of independent witnesses. Only agreement + input/task validation allows
//! repair, and the organism verifies its original output before deciding.
//! No direct access to the hidden reference function is granted to the VM.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s9_witness::s9_external_witness_repair;

fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(raw:&[u8])->Op{Op::ConstBytes{
    hex:raw.iter().map(|x|format!("{x:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}

/// Reuses the exact S9 consensus gate and finite S8 graph generator. S10's
/// only new behavior is to request feedback after an Ok execution as well as
/// a fault, and explicitly compare the saved tool with the public witness.
pub fn s10_active_semantic_probe()->Program{
    let mut program=s9_external_witness_repair();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];
    for node in &mut f.blocks {
        match node.id.as_str(){
            "assess_empty"=>{
                // Preserve the real first EXEC result for audit; do not use
                // S9's implicit zero-value success gate to decide correctness.
                *node=block("assess_empty",vec![p(2010,b),p(2011,b)],
                    vec![],Terminator::Jump{
                        target:"read_broken".into(),args:vec![2010,2011],
                    });
            },
            "read_witness"=>{
                let key=node.instructions.iter_mut().find(|ins|ins.id==3002).unwrap();
                key.op=l(b"/public/E001/semantic-witness");
            },
            "witness_task"=>{
                let Terminator::Branch{if_true,true_args,..}=&mut node.terminator
                    else{panic!("expected witness scope branch")};
                *if_true="semantic_verify_original".into();
                // keep [hash, saved program READ response, witness target]
                assert_eq!(true_args.len(),3);
            },
            _=>{}
        }
    }
    f.blocks.extend([
        block("semantic_verify_original",vec![
            p(3100,b),p(3101,b),p(3102,u),
        ],vec![
            i(3103,l(&[])),
        ],Terminator::Yield{call:WorldCall::Exec,
            args:vec![3100,3103],next:"semantic_status".into(),
            next_args:vec![3100,3101,3102],
        }),
        block("semantic_status",vec![
            p(3110,b),p(3111,b),p(3112,u),p(3113,b),
        ],vec![
            i(3114,l(b"code")),
            i(3115,Op::JsonFieldString(3113,3114)),
            i(3116,l(b"Ok")),
            i(3117,Op::EqBytes(3115,3116)),
        ],Terminator::Branch{condition:3117,
            if_true:"semantic_value".into(),
            true_args:vec![3110,3111,3112,3113],
            if_false:"semantic_mismatch".into(),
            false_args:vec![3110,3111,3112,3113],
        }),
        block("semantic_value",vec![
            p(3120,b),p(3121,b),p(3122,u),p(3123,b),
        ],vec![
            i(3124,l(b"payload")),
            i(3125,Op::JsonFieldBytes(3123,3124)),
            i(3126,l(b"U32")),
            i(3127,Op::JsonFieldU32(3125,3126)),
            i(3128,Op::EqU32(3127,3122)),
        ],Terminator::Branch{condition:3128,
            if_true:"no_repair".into(),true_args:vec![],
            if_false:"semantic_mismatch".into(),
            false_args:vec![3120,3121,3122,3123],
        }),
        block("semantic_mismatch",vec![
            p(3130,b),p(3131,b),p(3132,u),p(3133,b),
        ],vec![
            i(3134,l(b"/self/repair/semantic/observation")),
            i(3135,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![3134,3133,3135],
            next:"semantic_resume".into(),
            next_args:vec![3130,3131,3132],
        }),
        block("semantic_resume",vec![
            p(3140,b),p(3141,b),p(3142,u),p(3143,b),
        ],vec![
            i(3144,l(b"code")),
            i(3145,Op::JsonFieldString(3143,3144)),
            i(3146,l(b"Ok")),
            i(3147,Op::EqBytes(3145,3146)),
        ],Terminator::Branch{condition:3147,
            if_true:"start_search".into(),
            true_args:vec![3140,3141,3142],
            if_false:"fail".into(),false_args:vec![],
        }),
    ]);
    program.validate().expect("S10 semantic-probe graph must be valid");
    program
}
/// Negative control: the same initial tool-generation and active EXEC, but
/// disable reading and comparing the semantic witness after an Ok execution.
pub fn s10_no_semantic_evidence_control()->Program{
    let mut program=s10_active_semantic_probe();
    let node=program.functions[0].blocks.iter_mut()
        .find(|x|x.id=="assess_empty").expect("assessment");
    node.terminator=Terminator::Jump{
        target:"no_repair".into(),args:vec![]
    };
    program.validate().expect("S10 evidence ablation must validate");
    program
}
pub fn s10_manifest()->crate::SeedManifest{
    let p=s10_active_semantic_probe();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S10-proactive-post-adoption-semantic-probe",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-two-node-tool-search",
            "S7-human-seeded-empty-input-probe",
            "S8-human-seeded-two-guard-repair-vocabulary",
            "S9-public-correlated-witness-consensus-and-abstention",
            "proactive-semantic-check-even-when-EXEC-returns-Ok",
            "in-world-comparison-versus-environment-witness",
            "counterexample-recording-and-CAS-tool-repair",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-empty-reference-output","world-seed",
            "private-evaluator-label","held-out-scores",
        ],
        research_status:"SEEDED_ACTIVE_PROBE_AND_SIMULATED_ORACLE_NOT_GENERAL_SEMANTIC_DISCOVERY_NOT_M0",
    }
}
