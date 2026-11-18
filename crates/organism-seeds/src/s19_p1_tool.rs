//! S19: an already-running S18 organism turns learned P1 instance parameters
//! into a content-addressed executable frame encoder and reuses it for a B job.
//! No host controller selects an opcode, formats the job or registers the tool.
//!
//! This is **template specialization**, NOT invention of a new protocol
//! grammar or generic coding intelligence: the entire frame algorithm,
//! two mutation sites, functional reference and adoption policy are seeded.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s18_p1_vm::s18_p1_discover_and_invoke;

fn lit(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:name.into(),params,instructions,terminator}
}
fn yield_to(call:WorldCall,args:Vec<u32>,next:&str,next_args:Vec<u32>)->Terminator {
    Terminator::Yield{call,args,next:next.into(),next_args}
}
fn branch(cond:u32,yes:&str,yes_args:Vec<u32>,
    no:&str,no_args:Vec<u32>)->Terminator {
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:yes_args,
        if_false:no.into(),false_args:no_args}
}
fn template(bad_delimiter:bool)->Program {
    // Input contract: request_id: u32 LE followed by application body.
    // Output: framed P1 REQUEST bytes. Two slots will be rewritten in the VM:
    // register 1 = [kind=0, discovered RemoteEval opcode];
    // register 2 = [HELLO-derived XOR key].
    // No hidden generator data or private service operation is supplied.
    let b=Type::Bytes;
    let instructions=vec![
        i(1,lit(&[0,0])), // opcode slot
        i(2,lit(&[0])),   // XOR-key slot
        i(3,Op::ConstU32(0)),
        i(4,Op::ByteAt(2,3)),
        i(5,Op::Len(0)),
        i(6,Op::PackU16Be(5)),
        i(7,Op::Concat(1,6)),
        i(8,Op::Concat(7,0)),
        i(9,Op::XorFold(8)),
        i(10,Op::Xor(9,4)),
        i(11,Op::PackByte(10)),
        i(12,Op::Concat(8,11)),
        i(13,Op::ConstU32(0x7e)),
        i(14,Op::ConstU32(0x7d)),
        i(15,Op::ConstU32(0x20)),
        i(16,Op::ByteStuffEncode(12,13,14,15)),
        i(17,lit(if bad_delimiter{&[0x7d]}else{&[0x7e]})),
        i(18,Op::Concat(17,16)),
        i(19,Op::Concat(18,17)),
    ];
    let program=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{name:"main".into(),args:vec![b],returns:b,
            blocks:vec![block("entry",vec![p(0,b)],instructions,
                Terminator::Return{value:19})]}]};
    program.validate().expect("seeded P1 frame template valid");
    program
}
/// Researcher's disclosed seeded frame template, not a discovered algorithm.
pub fn s19_seeded_frame_template()->Program {template(false)}

fn program_with_template(broken:bool)->Program {
    let mut program=s18_p1_discover_and_invoke();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let found=f.blocks.iter_mut().find(|block|block.id=="found").unwrap();
    found.terminator=yield_to(WorldCall::Write,vec![144,140,145],
        "s19_make",vec![141,142,143]);
    let read_job=f.blocks.iter_mut().find(|block|block.id=="read_job").unwrap();
    read_job.params=vec![p(150,b),p(151,u),p(152,u),p(153,b),p(159,b)];
    read_job.terminator=yield_to(WorldCall::Read,vec![154],
        "s19_job",vec![153,152]);
    // Remove the original hardcoded request construction: only the
    // newly stored executable may form the actual RemoteEval request.
    f.blocks.retain(|block|block.id!="build_eval");
    f.blocks.extend([
        block("s19_make",vec![p(6000,b),p(6001,u),p(6002,u),p(6003,b)],vec![
            i(6004,lit(&template(broken).canonical_bytes())),
            i(6005,lit(b"entry")),
            i(6006,Op::ConstU32(1)),
            i(6007,Op::PackByte(6002)),
            i(6008,Op::HexEncode(6007)),
            i(6009,lit(br#"{"ConstBytes":{"hex":"00"#)),
            i(6010,Op::Concat(6009,6008)),
            i(6011,lit(br#""}}"#)),
            i(6012,Op::Concat(6010,6011)),
            i(6013,Op::RewriteOp(6004,6005,6006,6012)),
            i(6014,Op::PackByte(6001)),
            i(6015,Op::HexEncode(6014)),
            i(6016,lit(br#"{"ConstBytes":{"hex":""#)),
            i(6017,Op::Concat(6016,6015)),
            i(6018,Op::Concat(6017,6011)),
            i(6019,Op::ConstU32(2)),
            i(6020,Op::RewriteOp(6013,6005,6019,6018)),
            i(6021,lit(b"OCEE-PROGRAM-V1\0")),
            i(6022,Op::Concat(6021,6020)),
            i(6023,Op::Sha256(6022)),
            i(6024,Op::HexEncode(6023)),
            i(6025,lit(b"/self/versions/")),
            i(6026,Op::Concat(6025,6024)),
            i(6027,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![6026,6020,6027],
            "s19_test",vec![6024,6000,6001,6002])),
        block("s19_test",vec![p(6030,b),p(6031,b),p(6032,u),p(6033,u),p(6034,b)],vec![
            i(6035,lit(&[17,0,0,0,3,0,1,0,0,0,2,0,0,0,3,0,0,0])),
        ],yield_to(WorldCall::Exec,vec![6030,6035],
            "s19_test_status",vec![6030,6031,6032,6033])),
        block("s19_test_status",vec![
            p(6040,b),p(6041,b),p(6042,u),p(6043,u),p(6044,b)
        ],vec![
            i(6045,lit(b"code")),
            i(6046,Op::JsonFieldString(6044,6045)),
            i(6047,lit(b"Ok")),
            i(6048,Op::EqBytes(6046,6047)),
        ],branch(6048,"s19_test_value",vec![6040,6041,6042,6043,6044],
            "failure",vec![])),
        block("s19_test_value",vec![
            p(6050,b),p(6051,b),p(6052,u),p(6053,u),p(6054,b)
        ],vec![
            i(6055,lit(b"payload")),
            i(6056,Op::JsonFieldBytes(6054,6055)),
            i(6057,lit(b"Bytes")),
            i(6058,Op::JsonFieldBytes(6056,6057)),
            // Independent reference construction for a public-free test input;
            // it does not read the candidate's AST to choose an answer.
            i(6059,lit(&[0])),
            i(6060,Op::PackByte(6053)),
            i(6061,Op::Concat(6059,6060)),
            i(6062,lit(&[17,0,0,0,3,0,1,0,0,0,2,0,0,0,3,0,0,0])),
            i(6063,Op::Len(6062)),
            i(6064,Op::PackU16Be(6063)),
            i(6065,Op::Concat(6061,6064)),
            i(6066,Op::Concat(6065,6062)),
            i(6067,Op::XorFold(6066)),
            i(6068,Op::Xor(6067,6052)),
            i(6069,Op::PackByte(6068)),
            i(6070,Op::Concat(6066,6069)),
            i(6071,Op::ConstU32(0x7e)),
            i(6072,Op::ConstU32(0x7d)),
            i(6073,Op::ConstU32(0x20)),
            i(6074,Op::ByteStuffEncode(6070,6071,6072,6073)),
            i(6075,lit(&[0x7e])),
            i(6076,Op::Concat(6075,6074)),
            i(6077,Op::Concat(6076,6075)),
            i(6078,Op::EqBytes(6058,6077)),
        ],branch(6078,"s19_register",vec![6050,6051,6052,6053],
            "failure",vec![])),
        block("s19_register",vec![
            p(6080,b),p(6081,b),p(6082,u),p(6083,u)
        ],vec![
            i(6084,lit(b"/self/tools/p1-eval-frame")),
            i(6085,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![6084,6080,6085],
            "read_job",vec![6081,6082,6083,6080])),
        block("s19_job",vec![p(6090,b),p(6091,u),p(6092,b)],vec![
            i(6093,lit(b"payload")),
            i(6094,Op::JsonFieldBytes(6092,6093)),
            i(6095,lit(b"values_le")),
            i(6096,Op::JsonFieldBytes(6094,6095)),
            i(6097,lit(&[17,0,0,0,3,0])),
            i(6098,Op::Concat(6097,6096)),
        ],yield_to(WorldCall::Exec,vec![6090,6098],
            "s19_job_status",vec![6091])),
        block("s19_job_status",vec![p(6100,u),p(6101,b)],vec![
            i(6102,lit(b"code")),
            i(6103,Op::JsonFieldString(6101,6102)),
            i(6104,lit(b"Ok")),
            i(6105,Op::EqBytes(6103,6104)),
        ],branch(6105,"s19_job_send",vec![6100,6101],
            "failure",vec![])),
        block("s19_job_send",vec![p(6110,u),p(6111,b)],vec![
            i(6112,lit(b"payload")),
            i(6113,Op::JsonFieldBytes(6111,6112)),
            i(6114,lit(b"Bytes")),
            i(6115,Op::JsonFieldBytes(6113,6114)),
        ],yield_to(WorldCall::Send,vec![6115],"ack_wait1",vec![6110])),
    ]);
    program.validate().expect("S19 active tool specialization must verify");
    program
}

/// The program is installed ONCE; later tool creation is a VM-side decision.
pub fn s19_p1_build_and_use_tool()->Program {program_with_template(false)}

/// Negative mechanism control: isolated EXEC succeeds with an incorrectly
/// delimited but otherwise type-correct frame; independent semantic comparison
/// must decline tool registration and B execution.
pub fn s19_p1_broken_template_control()->Program {program_with_template(true)}

pub fn s19_manifest()->crate::SeedManifest {
    let program=s19_p1_build_and_use_tool();
    crate::SeedManifest{
        manifest_version:1,kind:"S19-P1-active-VM-template-specialization",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S18-active-P1-known-grammar-and-opcode-scanner",
            "seeded-generic-P1-executable-frame-template",
            "VM-RewriteOp-for-two-instance-parameter-slots",
            "VM-computed-content-hash-and-immutable-tool-version",
            "isolated-EXEC-self-test",
            "independent-expected-frame-comparison",
            "self-tool-hash-registration",
            "real-job-requires-EXEC-of-registered-program",
            "S18-real-C-ACK-and-RESULT-scan",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P1-delimiter-XOR".into()],
        prohibited_knowledge:vec![
            "private-P1-XOR-value","private-REMOTE_EVAL-opcode-value",
            "private-RESULT-opcode-value","protocol-seed","held-out-job-value",
            "B-gas-ledger","evaluation-oracle",
        ],
        research_status:"ACTIVE_VM_SEEDED_TEMPLATE_SPECIALIZATION_NOT_ALGORITHM_INVENTION_NOT_E001",
    }
}
