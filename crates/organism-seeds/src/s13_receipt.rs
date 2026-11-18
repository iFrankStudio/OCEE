//! S13: reject *same-program, same-input* stale reports by binding both
//! simulated witnesses to the kernel's receipt for THIS individual's probe.
//!
//! The existing S12 program/source binding remains intact. The S13 organism
//! persists the EXEC result (including the kernel-issued exec_step), then
//! checks every report's observed_after_step against that particular receipt.
//! This prevents a prior genuine report being recycled for a newer EXEC of
//! identical program and input. It does not authenticate the publishers,
//! prevent receipt forgery by a privileged world attacker, or detect an
//! identical-step collusion. Both validation rules and [] probe are seeded.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s12_provenance::s12_program_bound_provenance;
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn l(raw:&[u8])->Op{Op::ConstBytes{
    hex:raw.iter().map(|v|format!("{v:02x}")).collect()
}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn receipt_read(name:&str,params:Vec<Param>,next:&str,args:Vec<u32>)->Block{
    block(name,params,vec![
        i(6000,l(b"/self/repair/probe/receipt")),
    ],Terminator::Yield{call:WorldCall::Read,args:vec![6000],
        next:next.into(),next_args:args})
}
fn compare_receipt(name:&str,params:Vec<Param>,report_param:u32,
    read_result_param:u32,continuation:&str,next_args:Vec<u32>)->Block{
    block(name,params,vec![
        i(6100,l(b"payload")),
        i(6101,Op::JsonFieldBytes(read_result_param,6100)),
        i(6102,l(b"exec_step")),
        i(6103,Op::JsonFieldU32(6101,6102)),
        i(6104,l(b"observed_after_step")),
        i(6105,Op::JsonFieldU32(report_param,6104)),
        i(6106,Op::EqU32(6103,6105)),
    ],Terminator::Branch{condition:6106,
        if_true:continuation.into(),true_args:next_args,
        if_false:"provenance_replay".into(),false_args:vec![],
    })
}
pub fn s13_execution_receipt_binding()->Program{
    let mut program=s12_program_bound_provenance();
    let (b,u)=(Type::Bytes,Type::U32);
    let f=&mut program.functions[0];

    // Existing first active EXEC returns two block arguments:
    // program hash and the serialized ActionResult. A normal WorldCall::Write
    // stores this specific feedback before either witness is consulted.
    let first=f.blocks.iter_mut().find(|x|x.id=="assess_empty").unwrap();
    *first=block("assess_empty",vec![p(2010,b),p(2011,b)],vec![
        i(2012,l(b"/self/repair/probe/receipt")),
        i(2013,Op::ConstU32(u32::MAX)),
    ],Terminator::Yield{call:WorldCall::Write,
        args:vec![2012,2011,2013],
        next:"receipt_written".into(),next_args:vec![2010]});

    let primary=f.blocks.iter_mut().find(|x|x.id=="primary_subject").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut primary.terminator
        else{panic!("S12 primary subject expected branch")};
    *if_true="primary_receipt_read".into();
    *true_args=vec![5100,5101,5102,5103];

    let secondary=f.blocks.iter_mut().find(|x|x.id=="secondary_subject").unwrap();
    let Terminator::Branch{if_true,true_args,..}=&mut secondary.terminator
        else{panic!("S12 secondary subject expected branch")};
    *if_true="secondary_receipt_read".into();
    *true_args=vec![5300,5301,5302,5303];

    f.blocks.extend([
        block("receipt_written",vec![p(6200,b),p(6201,b)],vec![
            i(6202,l(b"code")),
            i(6203,Op::JsonFieldString(6201,6202)),
            i(6204,l(b"Ok")),
            i(6205,Op::EqBytes(6203,6204)),
        ],Terminator::Branch{condition:6205,
            if_true:"read_broken".into(),true_args:vec![6200,6201],
            if_false:"fail".into(),false_args:vec![],
        }),
        receipt_read("primary_receipt_read",
            vec![p(6300,b),p(6301,b),p(6302,u),p(6303,b)],
            "primary_receipt_compare",vec![6300,6301,6302,6303]),
        compare_receipt("primary_receipt_compare",
            vec![p(6400,b),p(6401,b),p(6402,u),p(6403,b),p(6404,b)],
            6403,6404,"audit_read",vec![6400,6401,6402]),
        receipt_read("secondary_receipt_read",
            vec![p(6500,b),p(6501,b),p(6502,u),p(6503,b)],
            "secondary_receipt_compare",vec![6500,6501,6502,6503]),
        // Secondary report is a READ ActionResult; unpack its payload before
        // testing observed_after_step, unlike primary which already carries
        // the unwrapped evidence document from witness_task.
        block("secondary_receipt_compare",vec![
            p(6600,b),p(6601,b),p(6602,u),p(6603,b),p(6604,b),
        ],vec![
            i(6605,l(b"payload")),
            i(6606,Op::JsonFieldBytes(6603,6605)),
            i(6607,Op::JsonFieldBytes(6604,6605)),
            i(6608,l(b"exec_step")),
            i(6609,Op::JsonFieldU32(6607,6608)),
            i(6610,l(b"observed_after_step")),
            i(6611,Op::JsonFieldU32(6606,6610)),
            i(6612,Op::EqU32(6609,6611)),
        ],Terminator::Branch{condition:6612,
            if_true:"audit_payload".into(),
                true_args:vec![6600,6601,6602,6603],
            if_false:"provenance_replay".into(),false_args:vec![],
        }),
        block("provenance_replay",vec![],vec![
            i(6700,l(b"/self/repair/provenance/status")),
            i(6701,l(b"replayed_execution")),
            i(6702,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![6700,6701,6702],
            next:"unresolved".into(),next_args:vec![],
        }),
    ]);
    program.validate().expect("S13 receipt-bound control graph must validate");
    program
}
/// Mechanism ablation: correct source and program identity are checked, but
/// the particular EXEC receipt is not; authentic earlier reports are reused.
pub fn s13_without_execution_receipt()->Program{
    s12_program_bound_provenance()
}
pub fn s13_manifest()->crate::SeedManifest{
    let p=s13_execution_receipt_binding();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S13-actual-execution-receipt-bound-simulated-evidence",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "S12-program-version-and-declared-source-metadata-binding",
            "kernel-issued-execution-step-in-ActionResult-feedback",
            "organism-saved-actual-tool-probe-receipt",
            "two-channel-execution-step-comparison",
            "same-program-same-input-prior-execution-replay-abstention",
            "preseeded-empty-input-probe-and-two-candidate-repair",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec![],
        prohibited_knowledge:vec![
            "world-seed","hidden-function-correct-answer",
            "cryptographic-source-authenticity","shared-world-private-state"
        ],
        research_status:"ACTUAL_EXECUTION_FRESHNESS_CHECK_NOT_AUTHENTICATED_REPORT_NOT_M0",
    }
}
