//! S12: bind two simulated semantic reports to the exact program version
//! under examination and their declared source categories.
//!
//! This rejects mislabeled and *cross-program* replayed evidence without
//! treating string metadata as cryptographic authentication. Same-version
//! replay and coordinated privileged forgery remain known unsolved threats.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s11_audit::s11_corroborated_semantic_repair;
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(raw:&[u8])->Op{Op::ConstBytes{
    hex:raw.iter().map(|v|format!("{v:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn abstain(name:&str,reason:&[u8])->Block {
    block(name,vec![],vec![
        i(5903,l(b"/self/repair/provenance/status")),
        i(5904,l(reason)),
        i(5905,Op::ConstU32(u32::MAX)),
    ],Terminator::Yield{call:WorldCall::Write,
        args:vec![5903,5904,5905],
        next:"unresolved".into(),next_args:vec![],
    })
}
pub fn s12_program_bound_provenance()->Program{
    let mut program=s11_corroborated_semantic_repair();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];
    let primary=f.blocks.iter_mut().find(|x|x.id=="witness_task").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut primary.terminator
        else{panic!("primary task check expected")};
    *if_true="primary_identity".into();
    // Program hash, original version read-feedback, expected numeric value,
    // original report JSON (previously discarded after scope validation).
    *true_args=vec![3050,3051,3053,3052];

    let secondary=f.blocks.iter_mut().find(|x|x.id=="audit_status").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut secondary.terminator
        else{panic!("audit status expected")};
    *if_true="secondary_identity".into();
    *true_args=vec![4100,4101,4102,4103];
    f.blocks.extend([
        block("primary_identity",vec![
            p(5000,b),p(5001,b),p(5002,u),p(5003,b),
        ],vec![
            i(5004,l(b"source_id")),
            i(5005,Op::JsonFieldString(5003,5004)),
            i(5006,l(b"sim-primary/v1")),
            i(5007,Op::EqBytes(5005,5006)),
        ],Terminator::Branch{condition:5007,
            if_true:"primary_subject".into(),
            true_args:vec![5000,5001,5002,5003],
            if_false:"provenance_source".into(),false_args:vec![],
        }),
        block("primary_subject",vec![
            p(5100,b),p(5101,b),p(5102,u),p(5103,b),
        ],vec![
            i(5104,l(b"subject_program_hash")),
            i(5105,Op::JsonFieldString(5103,5104)),
            i(5106,Op::EqBytes(5105,5100)),
        ],Terminator::Branch{condition:5106,
            if_true:"audit_read".into(),true_args:vec![5100,5101,5102],
            if_false:"provenance_stale".into(),false_args:vec![],
        }),
        block("secondary_identity",vec![
            p(5200,b),p(5201,b),p(5202,u),p(5203,b),
        ],vec![
            i(5204,l(b"source_id")),
            i(5205,Op::JsonFieldString(5203,5204)),
            i(5206,l(b"sim-secondary/v1")),
            i(5207,Op::EqBytes(5205,5206)),
        ],Terminator::Branch{condition:5207,
            if_true:"secondary_subject".into(),
            true_args:vec![5200,5201,5202,5203],
            if_false:"provenance_source".into(),false_args:vec![],
        }),
        block("secondary_subject",vec![
            p(5300,b),p(5301,b),p(5302,u),p(5303,b),
        ],vec![
            i(5304,l(b"subject_program_hash")),
            i(5305,Op::JsonFieldString(5303,5304)),
            i(5306,Op::EqBytes(5305,5300)),
        ],Terminator::Branch{condition:5306,
            if_true:"audit_payload".into(),
                true_args:vec![5300,5301,5302,5303],
            if_false:"provenance_stale".into(),false_args:vec![],
        }),
        abstain("provenance_source",b"source_mismatch"),
        abstain("provenance_stale",b"stale_program"),
    ]);
    program.validate().expect("S12 bound provenance graph valid");
    program
}
/// Mechanism ablation: use S11 behavior without source/version binding.
pub fn s12_no_program_binding_control()->Program{
    s11_corroborated_semantic_repair()
}
pub fn s12_manifest()->crate::SeedManifest {
    let program=s12_program_bound_provenance();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S12-declared-source-and-program-bound-simulated-evidence",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-finite-program-synthesis",
            "S10-researcher-selected-semantic-probe",
            "S11-simulated-dual-channel-corroboration",
            "declared-source-identity-check-per-channel",
            "immutable-program-hash-evidence-binding",
            "reject-cross-program-replay-and-mislabeled-evidence",
            "preseeded-two-guard-candidate-comparison-and-CAS",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "hidden-reference-answer","secret-source-key",
            "authentic-signature-proof","world-seed",
        ],
        research_status:"DECLARED_METADATA_BINDING_NOT_AUTHENTICATION_NOT_SAME_VERSION_REPLAY_PROTECTION_NOT_M0",
    }
}
