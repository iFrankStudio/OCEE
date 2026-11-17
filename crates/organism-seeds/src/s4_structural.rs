//! S4 development: bounded in-world single-node structural repair.
//!
//! The seed has a type-safe generic RewriteOp operator, an explicitly supplied
//! two-hypothesis search policy (Len vs XorFold), and a P0 framing task.
//! It does NOT contain a ready-to-run correct frame-builder program: the
//! first generated candidate is deliberately executable but wrong.
//! Failure of an output-level test triggers graph rewriting inside the VM.
//! This is NOT open-ended program synthesis or wholly new algorithm discovery.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block, Function, Instruction, Op, Param, Program, Terminator, Type,
    WorldCall,
};
use crate::p0_probe::s1_p0_tool_builder;

fn bin(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn arg(id:u32,ty:Type)->Param {Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:id.to_owned(),params,instructions,terminator}
}
fn replace(function:&mut Function,updated:Block) {
    let existing=function.blocks.iter_mut().find(|b|b.id==updated.id)
        .expect("S3 block must exist");
    *existing=updated;
}

/// A seeded search program. Its first emitted tool calculates a length instead
/// of the required checksum; only observed functional failure can trigger a
/// generic, type-validated node rewrite from Len to XorFold.
pub fn s4_p0_structural_repair()->Program {
    let mut program=s1_p0_tool_builder();
    let f=&mut program.functions[0];
    let (b,n)=(Type::Bytes,Type::U32);

    // The researcher seeds a WRONG starting structure, not the correct tool:
    // modify the emitted initial tool's instruction node 6 from XorFold to Len.
    let builder=f.blocks.iter_mut().find(|b|b.id=="tool_prepare")
        .expect("tool_prepare");
    let suffix=builder.instructions.iter_mut().find(|i|i.id==514)
        .expect("tool source fragment");
    let Op::ConstBytes{hex}= &mut suffix.op else {panic!("source fragment")};
    let chars=(0..hex.len()).step_by(2)
        .map(|i|u8::from_str_radix(&hex[i..i+2],16).expect("hex"))
        .collect::<Vec<_>>();
    let source=String::from_utf8(chars).expect("UTF-8 canonical fragment");
    assert_eq!(source.matches("\"XorFold\":5").count(),1);
    let broken=source.replace("\"XorFold\":5","\"Len\":5");
    *hex=broken.bytes().map(|x|format!("{x:02x}")).collect();
    builder.instructions.push(ins(523,Op::ConstU32(0)));
    builder.terminator=Terminator::Yield{
        call:WorldCall::Write,args:vec![521,515,522],
        next:"tool_test".into(),next_args:vec![519,500,515,523],
    };

    replace(f,block("tool_test",vec![
        arg(530,b),arg(531,b),arg(532,b),arg(533,n),arg(534,b)
    ],vec![ins(535,bin(&[0,0,0,0,1,0,9,0,0,0]))],
        Terminator::Yield{call:WorldCall::Exec,args:vec![530,535],
            next:"tool_verify".into(),next_args:vec![530,531,532,533]}));

    replace(f,block("tool_verify",vec![
        arg(540,b),arg(541,b),arg(542,b),arg(543,n),arg(544,b)
    ],vec![
        ins(545,bin(b"code")),
        ins(546,Op::JsonFieldString(544,545)),
        ins(547,bin(b"Ok")),
        ins(548,Op::EqBytes(546,547))
    ],Terminator::Branch{condition:548,
        if_true:"tool_verify_bytes".into(),true_args:vec![540,541,542,543,544],
        if_false:"tool_failed".into(),false_args:vec![]
    }));

    replace(f,block("tool_verify_bytes",vec![
        arg(700,b),arg(701,b),arg(702,b),arg(703,n),arg(704,b)
    ],vec![
        ins(705,bin(b"payload")),
        ins(706,Op::JsonFieldBytes(704,705)),
        ins(707,bin(b"Bytes")),
        ins(708,Op::JsonFieldBytes(706,707)),
        ins(709,Op::ConstU32(0)),
        ins(710,Op::ConstU32(3)),
        ins(711,Op::Slice(701,709,710)),
        ins(712,bin(&[10,0])),
        ins(713,Op::Concat(711,712)),
        ins(714,bin(&[0,0,0,0,1,0,9,0,0,0])),
        ins(715,Op::Concat(713,714)),
        ins(716,Op::XorFold(715)),
        ins(717,Op::PackByte(716)),
        ins(718,Op::Concat(715,717)),
        ins(719,Op::EqBytes(708,718)),
    ],Terminator::Branch{condition:719,
        if_true:"tool_register".into(),true_args:vec![700],
        if_false:"tool_try_rewrite".into(),false_args:vec![702,701,703]
    }));

    f.blocks.extend([
        block("tool_try_rewrite",vec![arg(800,b),arg(801,b),arg(802,n)],vec![
            ins(803,Op::ConstU32(0)),
            ins(804,Op::EqU32(802,803)),
        ],Terminator::Branch{condition:804,
            if_true:"tool_rewrite".into(),true_args:vec![800,801],
            if_false:"tool_failed".into(),false_args:vec![]
        }),
        block("tool_rewrite",vec![arg(900,b),arg(901,b)],vec![
            ins(902,bin(b"entry")),
            ins(903,Op::ConstU32(6)),
            ins(904,bin(&serde_json::to_vec(&Op::XorFold(5))
                .expect("op is serializable"))),
            ins(905,Op::RewriteOp(900,902,903,904)),
            ins(906,bin(b"OCEE-PROGRAM-V1\0")),
            ins(907,Op::Concat(906,905)),
            ins(908,Op::Sha256(907)),
            ins(909,Op::HexEncode(908)),
            ins(910,bin(b"/self/versions/")),
            ins(911,Op::Concat(910,909)),
            ins(912,Op::ConstU32(u32::MAX)),
            ins(913,Op::ConstU32(1)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![911,905,912],next:"tool_test".into(),
            next_args:vec![909,901,905,913]
        })
    ]);
    program.validate().expect("S4 repair seed must be type safe");
    program
}

/// Fair structure-search ablation: same P0 discovery, same candidate, same
/// output-level validation; only the structural rewrite branch is disabled.
pub fn s4_p0_no_rewrite_control()->Program {
    let mut p=s4_p0_structural_repair();
    let branch=p.functions[0].blocks.iter_mut()
        .find(|b|b.id=="tool_try_rewrite").expect("rewrite branch");
    branch.terminator=Terminator::Jump{target:"tool_failed".into(),args:vec![]};
    p.validate().expect("control must validate");
    p
}

pub fn s4_manifest()->crate::SeedManifest {
    let p=s4_p0_structural_repair();
    crate::SeedManifest {
        manifest_version:1,
        kind:"S4-P0-bounded-structural-repair",
        program_hash:format!("sha256:{}",p.content_hash().expect("validated seed")),
        built_in_modules:vec![
            "P0-XOR-family-prior", "bounded-opcode-discovery",
            "wrong-Len-starting-program-template",
            "generic-RewriteOp-graph-editor",
            "two-hypothesis-Len-or-XorFold-search",
            "explicit-output-phenotype-validator",
            "local-EXEC-and-world-CB-feedback",
            "tool-registration-on-functional-success"
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P0-XOR-frame-envelope".into()],
        prohibited_knowledge:vec![
            "actual-hidden-opcode","actual-protocol-seed",
            "held-out-values","evaluator-reward"
        ],
        research_status:"BOUNDED_ONE_NODE_STRUCTURAL_REPAIR_WITH_PRESEEDED_HYPOTHESES_NOT_M0",
    }
}
