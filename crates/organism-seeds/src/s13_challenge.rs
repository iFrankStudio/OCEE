//! S13: an organism-issued execution challenge binds simulated witness
//! objects to one *particular* probe, not merely its immutable tool version.
//! Detects unmodified same-version replay across differently timed episodes.
//! Not cryptographic origin authentication: a privileged source can forge
//! challenge and value together. The probe input and search policy are seeded.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s12_provenance::s12_program_bound_provenance;
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(raw:&[u8])->Op{Op::ConstBytes{
    hex:raw.iter().map(|v|format!("{v:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn nonce_read(label:&str,next:&str)->Block{
    block(label,vec![
        p(6200,Type::Bytes),p(6201,Type::Bytes),p(6202,Type::U32),p(6203,Type::Bytes)
    ],vec![i(6204,l(b"/self/repair/probe-nonce"))],
    Terminator::Yield{call:WorldCall::Read,args:vec![6204],
        next:next.into(),next_args:vec![6200,6201,6202,6203]})
}
fn nonce_status(label:&str,next:&str)->Block{
    block(label,vec![
        p(6300,Type::Bytes),p(6301,Type::Bytes),p(6302,Type::U32),
        p(6303,Type::Bytes),p(6304,Type::Bytes)
    ],vec![
        i(6305,l(b"code")),
        i(6306,Op::JsonFieldString(6304,6305)),
        i(6307,l(b"Ok")),
        i(6308,Op::EqBytes(6306,6307)),
    ],Terminator::Branch{condition:6308,
        if_true:next.into(),true_args:vec![6300,6301,6302,6303,6304],
        if_false:"provenance_stale".into(),false_args:vec![]})
}
fn nonce_match(label:&str,secondary:bool)->Block{
    let mut instructions=vec![
        i(6405,l(b"payload")),
        i(6406,Op::JsonFieldBytes(6404,6405)), // saved nonce object READ payload
        i(6407,l(b"probe_challenge")),
    ];
    let actual=if secondary {
        // Secondary preserved ActionResult JSON, rather than raw witness.
        instructions.extend([
            i(6408,Op::JsonFieldBytes(6403,6405)),
            i(6409,Op::JsonFieldString(6408,6407)),
        ]);
        6409
    }else {
        instructions.push(i(6409,Op::JsonFieldString(6403,6407)));
        6409
    };
    instructions.push(i(6410,Op::EqBytes(actual,6406)));
    block(label,vec![
        p(6400,Type::Bytes),p(6401,Type::Bytes),p(6402,Type::U32),
        p(6403,Type::Bytes),p(6404,Type::Bytes)
    ],instructions,Terminator::Branch{condition:6410,
        if_true:if secondary{"audit_payload".into()}else{"audit_read".into()},
        true_args:if secondary{vec![6400,6401,6402,6403]}
            else{vec![6400,6401,6402]},
        if_false:"provenance_stale".into(),false_args:vec![]})
}
pub fn s13_challenge_bound_repair()->Program{
    let mut program=s12_program_bound_provenance();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];
    for node in &mut f.blocks {
        match node.id.as_str(){
            "probe_empty"=>{
                *node=block("probe_empty",vec![
                    p(2000,b),p(2001,b)
                ],vec![],Terminator::Yield{
                    call:WorldCall::Observe,args:vec![],
                    next:"make_challenge".into(),next_args:vec![2000],
                });
            },
            "primary_subject"=>{
                let Terminator::Branch{if_true,true_args,..}=&mut node.terminator
                    else{unreachable!()};
                *if_true="nonce_read_primary".into();
                *true_args=vec![5100,5101,5102,5103];
            },
            "secondary_subject"=>{
                let Terminator::Branch{if_true,true_args,..}=&mut node.terminator
                    else{unreachable!()};
                *if_true="nonce_read_secondary".into();
                *true_args=vec![5300,5301,5302,5303];
            },
            _=>{}
        }
    }
    f.blocks.extend([
        block("make_challenge",vec![p(6000,b),p(6001,b)],vec![
            i(6002,l(b"payload")),
            i(6003,Op::JsonFieldBytes(6001,6002)),
            i(6004,Op::Sha256(6003)),
            i(6005,Op::HexEncode(6004)),
            i(6006,l(b"/self/repair/probe-nonce")),
            i(6007,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{
            call:WorldCall::Write,args:vec![6006,6005,6007],
            next:"challenge_written".into(),next_args:vec![6000],
        }),
        block("challenge_written",vec![p(6100,b),p(6101,b)],vec![
            i(6102,l(b"code")),
            i(6103,Op::JsonFieldString(6101,6102)),
            i(6104,l(b"Ok")),
            i(6105,Op::EqBytes(6103,6104)),
        ],Terminator::Branch{condition:6105,
            if_true:"challenge_exec".into(),true_args:vec![6100],
            if_false:"provenance_stale".into(),false_args:vec![],
        }),
        block("challenge_exec",vec![p(6150,b)],vec![
            i(6151,l(&[])),
        ],Terminator::Yield{
            call:WorldCall::Exec,args:vec![6150,6151],
            next:"assess_empty".into(),next_args:vec![6150],
        }),
        nonce_read("nonce_read_primary","nonce_status_primary"),
        nonce_status("nonce_status_primary","nonce_match_primary"),
        nonce_match("nonce_match_primary",false),
        nonce_read("nonce_read_secondary","nonce_status_secondary"),
        nonce_status("nonce_status_secondary","nonce_match_secondary"),
        nonce_match("nonce_match_secondary",true),
    ]);
    program.validate().expect("S13 bounded challenge graph valid");
    program
}
/// Ablation: S12 ignores the challenge, though the environment still adds
/// metadata when a nonce exists. A cross-episode stale report can be accepted.
pub fn s13_no_challenge_control()->Program{
    s12_program_bound_provenance()
}
pub fn s13_manifest()->crate::SeedManifest{
    let program=s13_challenge_bound_repair();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S13-organism-probe-challenge-replay-rejection",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S6-preseeded-finite-program-search",
            "S10-researcher-selected-empty-input-probe",
            "S11-simulated-two-source-corroboration",
            "S12-declared-source-and-version-binding",
            "observe-derived-organism-generated-per-probe-challenge",
            "require-both-simulated-reports-to-match-saved-challenge",
            "bounded-guard-search-and-CAS-on-success",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "world-reference-answer","private-authentication-key",
            "hidden-seed","external-independent-signature",
        ],
        research_status:"CHALLENGE_MATCH_BLOCKS_UNMODIFIED_REPLAY_NOT_AUTHENTICATION_NOT_M0",
    }
}
