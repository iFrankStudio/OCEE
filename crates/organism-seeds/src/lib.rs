//! Explicitly pre-seeded S0 bootstrap. It is NOT a learning agent.
//! It observes a documented public entrypoint, replays a public sample,
//! polls for a reply and persists raw evidence. No P0/P1/P2 grammar,
 //! RemoteEval opcode or other hidden parameters are available to it.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall,
};
use serde::Serialize;

fn bytes(b:&[u8])->Op {
    Op::ConstBytes{hex:b.iter().map(|v|format!("{v:02x}")).collect()}
}
fn inst(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn arg(id:u32,ty:Type)->Param {Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:id.to_owned(),params,instructions,terminator}
}

/// Researcher-injected initial algorithm. Subsequent ticks execute it
/// entirely through the VM and ordinary world READ/SEND/RECEIVE/WRITE operations.
pub fn s0_bootstrap()->Program {
    let b=Type::Bytes;
    Program{format_version:1,entry_function:"main".into(),functions:vec![
        Function{name:"main".into(),args:vec![b],returns:Type::U32,blocks:vec![
            // Discover one documented public object; no private protocol state.
            block("entry",vec![arg(0,b)],vec![
                inst(1,bytes(b"/public/C/hello")),
            ],Terminator::Yield{call:WorldCall::Read,args:vec![1],
                next:"sample".into(),next_args:vec![]}),
            // ActionResult.payload contains JSON of HelloSample. Extract its
            // legal request as a generic JSON byte array; do NOT parse its grammar.
            block("sample",vec![arg(10,b)],vec![
                inst(11,bytes(b"payload")),
                inst(12,Op::JsonFieldBytes(10,11)),
                inst(13,bytes(b"request")),
                inst(14,Op::JsonFieldBytes(12,13)),
            ],Terminator::Yield{call:WorldCall::Send,args:vec![14],
                next:"sent".into(),next_args:vec![12]}),
            block("sent",vec![arg(20,b),arg(21,b)],vec![],
                Terminator::Jump{target:"poll".into(),args:vec![20]}),
            // The retry mechanism is entirely pre-seeded and capped at 8 polls.
            block("poll",vec![arg(30,b)],vec![],
                Terminator::RepeatBounded{limit:8,
                    body:"receive".into(),body_args:vec![30],
                    done:"timeout".into(),done_args:vec![30]}),
            block("receive",vec![arg(40,b)],vec![],
                Terminator::Yield{call:WorldCall::Receive,args:vec![],
                    next:"check".into(),next_args:vec![40]}),
            block("check",vec![arg(50,b),arg(51,b)],vec![
                inst(52,bytes(b"code")),
                inst(53,Op::JsonFieldString(51,52)),
                inst(54,bytes(b"Ok")),
                inst(55,Op::EqBytes(53,54)),
            ],Terminator::Branch{condition:55,
                if_true:"save".into(),true_args:vec![50,51],
                if_false:"poll".into(),false_args:vec![50]}),
            block("save",vec![arg(60,b),arg(61,b)],vec![
                inst(62,bytes(b"/self/observations/hello")),
                inst(63,Op::ConstU32(u32::MAX)),
            ],Terminator::Yield{call:WorldCall::Write,
                args:vec![62,60,63],next:"save_reply".into(),next_args:vec![61]}),
            block("save_reply",vec![arg(70,b),arg(71,b)],vec![
                inst(72,bytes(b"/self/observations/reply")),
                inst(73,Op::ConstU32(u32::MAX)),
            ],Terminator::Yield{call:WorldCall::Write,
                args:vec![72,70,73],next:"done".into(),next_args:vec![]}),
            block("timeout",vec![arg(90,b)],vec![
                inst(91,bytes(b"/self/observations/hello")),
                inst(92,Op::ConstU32(u32::MAX)),
            ],Terminator::Yield{call:WorldCall::Write,
                args:vec![91,90,92],next:"done".into(),next_args:vec![]}),
            block("done",vec![arg(100,b)],vec![
                inst(101,Op::ConstU32(1)),
            ],Terminator::Return{value:101}),
        ]}
    ]}
}

/// A reproducible capability/provenance declaration. The compiled content hash
/// is generated from VM code rather than copied from a human-written constant.
#[derive(Clone,Debug,Serialize)]
pub struct SeedManifest {
    pub manifest_version:u32,
    pub kind:&'static str,
    pub program_hash:String,
    pub built_in_modules:Vec<&'static str>,
    pub initial_tool_hashes:Vec<String>,
    pub known_protocol_families:Vec<String>,
    pub prohibited_knowledge:Vec<&'static str>,
    pub research_status:&'static str,
}
pub fn s0_manifest()->SeedManifest {
    SeedManifest{
        manifest_version:1,kind:"S0-bootstrap",
        program_hash:format!("sha256:{}",s0_bootstrap().content_hash()
            .expect("S0 program must validate")),
        built_in_modules:vec![
            "public-object-read", "generic-json-fields",
            "public-example-replay", "bounded-feedback-poll", "evidence-write"
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "protocol-p0-values","protocol-p1-values","protocol-p2-parser",
            "remote-eval-opcodes","hidden-seeds","evaluation-score"
        ],
        research_status:"BOOTSTRAP_ONLY_NOT_E001_S_FULL_NOT_SELF_INNOVATION",
    }
}

/// Explicitly seeded P0 XOR framing probe; not a general protocol learner.
pub mod p0_probe;

/// Development-only second-stage P0 remote invocation; explicitly seeded.
pub mod s2_p0;

/// S4 bounded structural graph repair research seed.
pub mod s4_structural;

/// S5 seed: closed-vocabulary search using public input/output examples.
pub mod s5_search;

/// S6: bounded multi-node composition search over a seeded operator grammar.
pub mod s6_compose;

/// S7: observe failing tool boundary and add typed control-flow guard.
pub mod s7_recover;

/// S8: bounded feedback-selected repair among two preset guard candidates.
pub mod s8_repair_search;

/// S9: conditional repair using post-fault public boundary witnesses.
pub mod s9_witness;

/// S10: active semantic validation even for successful tool executions.
pub mod s10_semantic;

/// S11: simulated secondary evaluator and corroboration before adoption.
pub mod s11_audit;

/// S12: source labels and content-hash binding for simulated evidence.
pub mod s12_provenance;

/// S13: observe-derived freshness challenge for bounded evidence replay detection.
pub mod s13_challenge;

/// S13: bind environment evidence to the kernel-issued exact EXEC receipt.
pub mod s13_receipt;
