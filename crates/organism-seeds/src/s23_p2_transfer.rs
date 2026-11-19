//! S23A: in one ACTIVE, gas-metered P2 organism, turn the learned private
//! REMOTE_EVAL opcode and HELLO-derived role mapping into a content-addressed
//! executable encoder. A fresh episode may inherit only sealed program bytes
//! and the registered hash; it must test that tool on *current* C before using.
//! Stale tools fall back to the full live S22 scan, and CAS replaces the ref.
//! Researcher-supplied P2 framing AST/scan and matching criteria are priors;
//! the instance-specific tags/opcode/seed/B answer are never passed to the VM.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s22_p2_remote::s22_p2_discover_and_execute;
use crate::s23_p2_tool::s23_seeded_p2_frame_template;

fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(x:&[u8])->Op{Op::ConstBytes{hex:x.iter().map(|b|format!("{b:02x}")).collect()}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,term:Terminator)->Block {
    Block{id:name.into(),params,instructions,terminator:term}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(next:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:next.into(),args}
}
fn br(c:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:t.into(),true_args:ta,
        if_false:f.into(),false_args:fa}
}
fn ro()->Vec<Param>{vec![p(600,Type::Bytes),p(601,Type::U32)]}
fn ro_args()->Vec<u32>{vec![600,601]}
fn role_op_bytes()->Vec<Param>{
    vec![p(600,Type::Bytes),p(601,Type::U32),p(603,Type::Bytes)]
}
fn role_op_hash()->Vec<Param>{
    vec![p(600,Type::Bytes),p(601,Type::U32),p(604,Type::Bytes)]
}
fn role_op_hash_reply()->Vec<Param>{
    let mut v=role_op_hash();v.push(p(605,Type::Bytes));v
}
fn role_op_body()->Vec<Param>{
    vec![p(600,Type::Bytes),p(601,Type::U32),p(603,Type::Bytes)]
}
fn ro_reply()->Vec<Param>{
    let mut v=ro();v.push(p(603,Type::Bytes));v
}

pub fn s23_p2_remote_tool_transfer()->Program{
    let mut program=s22_p2_discover_and_execute();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let s=f.blocks.iter_mut().find(|v|v.id=="s22_start").unwrap();
    s.terminator=j("s23_read_registry",vec![600]);
    let found=f.blocks.iter_mut().find(|v|v.id=="s22_remote_found").unwrap();
    // Do not mark a newly learned opcode as a successful capability before
    // the actual specialized executable passes current-C BadBody testing.
    found.terminator=j("s23_make",ro_args());
    let feedback=f.blocks.iter_mut().find(|v|v.id=="s22_feedback").unwrap();
    feedback.terminator=br(656,"s22_probe_classify",vec![600,601,655],
        "s23_route_other",vec![600,601,602,655]);
    let job=f.blocks.iter_mut().find(|v|v.id=="s22_job_send").unwrap();
    job.terminator=j("s23_job_registry",vec![600,601,676]);

    f.blocks.extend([
        block("s23_read_registry",vec![p(600,b)],vec![
            i(9000,lit(b"/self/tools/p2-eval-frame")),
        ],y(WorldCall::Read,vec![9000],"s23_read_registry_reply",vec![600])),
        block("s23_read_registry_reply",vec![p(600,b),p(603,b)],vec![
            i(9001,lit(b"code")),
            i(9002,Op::JsonFieldString(603,9001)),
            i(9003,lit(b"Ok")),
            i(9004,Op::EqBytes(9002,9003)),
        ],br(9004,"s23_inherited_exec",vec![600,603],
            "s23_fallback",vec![600])),
        block("s23_fallback",vec![p(600,b)],vec![
            i(9005,Op::ConstU32(0x10)),
        ],j("s22_remote_scan",vec![600,9005])),
        block("s23_inherited_exec",vec![p(600,b),p(603,b)],vec![
            i(9006,lit(b"payload")),
            i(9007,Op::JsonFieldBytes(603,9006)),
            i(9008,lit(b"")),
        ],y(WorldCall::Exec,vec![9007,9008],
            "s23_inherited_exec_reply",vec![600])),
        block("s23_inherited_exec_reply",vec![p(600,b),p(603,b)],vec![
            i(9010,lit(b"code")),
            i(9011,Op::JsonFieldString(603,9010)),
            i(9012,lit(b"Ok")),
            i(9013,Op::EqBytes(9011,9012)),
        ],br(9013,"s23_decode_tool_wire",vec![600,603],
            "s23_fallback",vec![600])),
        block("s23_decode_tool_wire",vec![p(600,b),p(603,b)],vec![
            i(9014,lit(b"payload")),
            i(9015,Op::JsonFieldBytes(603,9014)),
            i(9016,lit(b"Bytes")),
            i(9017,Op::JsonFieldBytes(9015,9016)),
            i(9018,Op::ConstU32(3)),
            i(9019,Op::Len(9017)),
            i(9020,Op::Sub(9019,9018)),
            i(9021,Op::Slice(9017,9018,9020)),
            i(9022,Op::TlvDecode(9021)),
            i(9023,Op::ConstU32(0)),
            i(9024,Op::JsonIndex(9022,9023)),
            i(9025,lit(b"value")),
            i(9026,Op::JsonFieldBytes(9024,9025)),
            i(9027,Op::ConstU32(2)),
            i(9028,Op::ByteAt(600,9027)),
            i(9029,Op::TlvLookup(9026,9028)),
            i(9030,Op::ByteAt(9029,9023)),
        ],y(WorldCall::Send,vec![9017],"s22_wait1",
            vec![600,9030,9031_placeholder])),
        block("s23_route_other",vec![
            p(600,b),p(601,u),p(602,u),p(603,b)
        ],vec![
            i(9031,Op::ConstU32(3)),
            i(9032,Op::EqU32(602,9031)),
        ],br(9032,"s23_compat_diag",vec![600,601,603],
            "s22_response_classify",vec![600,601,602,603])),
        block("s23_compat_diag",role_op_bytes(),vec![
            i(9033,Op::ConstU32(0)),
            i(9034,Op::ByteAt(603,9033)),
            i(9035,Op::ConstU32(0x7b)),
            i(9036,Op::EqU32(9034,9035)),
        ],br(9036,"s23_compat_code",vec![600,601,603],
            "s23_fallback",vec![600])),
        block("s23_compat_code",role_op_bytes(),vec![
            i(9037,lit(b"code")),
            i(9038,Op::JsonFieldString(603,9037)),
            i(9039,lit(b"BadBody")),
            i(9040,Op::EqBytes(9038,9039)),
        ],br(9040,"s23_pending_read",ro_args(),
            "s23_fallback",vec![600])),
        // Pending candidate exists only for the cold specialization, not
        // for an inherited program. It is never imported to fresh worlds.
        block("s23_pending_read",ro(),vec![
            i(9041,lit(b"/self/candidates/p2-eval-hash")),
        ],y(WorldCall::Read,vec![9041],"s23_pending_reply",ro_args())),
        block("s23_pending_reply",ro_reply(),vec![
            i(9042,lit(b"code")),
            i(9043,Op::JsonFieldString(603,9042)),
            i(9044,lit(b"Ok")),
            i(9045,Op::EqBytes(9043,9044)),
        ],br(9045,"s23_recheck_registry",vec![600,601,603],
            "s23_record_bridge",ro_args())),
        block("s23_record_bridge",ro(),vec![],
            y(WorldCall::Observe,vec![],"s23_record_op",ro_args())),
        block("s23_recheck_registry",role_op_bytes(),vec![
            i(9046,lit(b"payload")),
            i(9047,Op::JsonFieldBytes(603,9046)),
            i(9048,lit(b"/self/tools/p2-eval-frame")),
        ],y(WorldCall::Read,vec![9048],"s23_recheck_reply",vec![600,601,9047])),
        block("s23_recheck_reply",vec![
            p(600,b),p(601,u),p(604,b),p(603,b)
        ],vec![
            i(9049,lit(b"code")),
            i(9050,Op::JsonFieldString(603,9049)),
            i(9051,lit(b"Ok")),
            i(9052,Op::EqBytes(9050,9051)),
        ],br(9052,"s23_register_replace",vec![600,601,604],
            "s23_register_new",vec![600,601,604])),
        block("s23_register_replace",role_op_hash(),vec![
            i(9053,lit(b"/self/tools/p2-eval-frame")),
            i(9054,Op::ConstU32(0)),
        ],y(WorldCall::Write,vec![9053,604,9054],
            "s23_record_op",ro_args())),
        block("s23_register_new",role_op_hash(),vec![
            i(9055,lit(b"/self/tools/p2-eval-frame")),
            i(9056,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9055,604,9056],
            "s23_record_op",ro_args())),
        block("s23_record_op",ro_reply(),vec![
            i(9057,lit(b"code")),
            i(9058,Op::JsonFieldString(603,9057)),
            i(9059,lit(b"Ok")),
            i(9060,Op::EqBytes(9058,9059)),
        ],br(9060,"s23_record_write",ro_args(),"failed",vec![])),
        block("s23_record_write",ro(),vec![
            i(9061,Op::PackByte(601)),
            i(9062,lit(b"/self/discoveries/p2-remote-opcode")),
            i(9063,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9062,9061,9063],
            "s22_read_job",ro_args())),
        block("s23_make",ro(),vec![
            i(9064,lit(&s23_seeded_p2_frame_template().canonical_bytes())),
            i(9065,lit(b"entry")),
            i(9066,Op::ConstU32(1)),
            i(9067,Op::HexEncode(600)),
            i(9068,lit(br#"{"ConstBytes":{"hex":""#)),
            i(9069,Op::Concat(9068,9067)),
            i(9070,lit(br#""}}"#)),
            i(9071,Op::Concat(9069,9070)),
            i(9072,Op::RewriteOp(9064,9065,9066,9071)),
            i(9073,Op::PackByte(601)),
            i(9074,Op::HexEncode(9073)),
            i(9075,Op::Concat(9068,9074)),
            i(9076,Op::Concat(9075,9070)),
            i(9077,Op::ConstU32(2)),
            i(9078,Op::RewriteOp(9072,9065,9077,9076)),
            i(9079,lit(b"OCEE-PROGRAM-V1\0")),
            i(9080,Op::Concat(9079,9078)),
            i(9081,Op::Sha256(9080)),
            i(9082,Op::HexEncode(9081)),
            i(9083,lit(b"/self/versions/")),
            i(9084,Op::Concat(9083,9082)),
            i(9085,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9084,9078,9085],
            "s23_write_pending",vec![600,601,9082])),
        block("s23_write_pending",vec![
            p(600,b),p(601,u),p(604,b),p(603,b)
        ],vec![
            i(9086,lit(b"/self/candidates/p2-eval-hash")),
            i(9087,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9086,604,9087],
            "s23_cold_exec",vec![600,601,604])),
        block("s23_cold_exec",vec![
            p(600,b),p(601,u),p(604,b),p(603,b)
        ],vec![
            i(9088,lit(b"")),
        ],y(WorldCall::Exec,vec![604,9088],
            "s23_cold_exec_reply",ro_args())),
        block("s23_cold_exec_reply",ro_reply(),vec![
            i(9089,lit(b"code")),
            i(9090,Op::JsonFieldString(603,9089)),
            i(9091,lit(b"Ok")),
            i(9092,Op::EqBytes(9090,9091)),
        ],br(9092,"s23_cold_send",vec![600,601,603],"failed",vec![])),
        block("s23_cold_send",role_op_bytes(),vec![
            i(9093,lit(b"payload")),
            i(9094,Op::JsonFieldBytes(603,9093)),
            i(9095,lit(b"Bytes")),
            i(9096,Op::JsonFieldBytes(9094,9095)),
            i(9097,Op::ConstU32(3)), // compatibility probe
        ],y(WorldCall::Send,vec![9096],"s22_wait1",vec![600,601,9097])),
        block("s23_job_registry",role_op_body(),vec![
            i(9098,lit(b"/self/tools/p2-eval-frame")),
        ],y(WorldCall::Read,vec![9098],"s23_job_registry_reply",
            vec![600,601,603])),
        block("s23_job_registry_reply",vec![
            p(600,b),p(601,u),p(603,b),p(604,b)
        ],vec![
            i(9099,lit(b"code")),
            i(9100,Op::JsonFieldString(604,9099)),
            i(9101,lit(b"Ok")),
            i(9102,Op::EqBytes(9100,9101)),
        ],br(9102,"s23_job_exec",vec![600,601,603,604],
            "failed",vec![])),
        block("s23_job_exec",vec![
            p(600,b),p(601,u),p(603,b),p(604,b)
        ],vec![
            i(9103,lit(b"payload")),
            i(9104,Op::JsonFieldBytes(604,9103)),
        ],y(WorldCall::Exec,vec![9104,603],
            "s23_job_exec_reply",ro_args())),
        block("s23_job_exec_reply",ro_reply(),vec![
            i(9105,lit(b"code")),
            i(9106,Op::JsonFieldString(603,9105)),
            i(9107,lit(b"Ok")),
            i(9108,Op::EqBytes(9106,9107)),
        ],br(9108,"s23_job_send",vec![600,601,603],"failed",vec![])),
        block("s23_job_send",role_op_bytes(),vec![
            i(9109,lit(b"payload")),
            i(9110,Op::JsonFieldBytes(603,9109)),
            i(9111,lit(b"Bytes")),
            i(9112,Op::JsonFieldBytes(9110,9111)),
            i(9113,Op::ConstU32(1)), // normal ACK phase
        ],y(WorldCall::Send,vec![9112],"s22_wait1",vec![600,601,9113])),
    ]);
    // The inherited validator's tool output has an explicit opcode found
    // from its actual TLV wire. Phase 3 must be a proper u32 register value.
    let inherited=f.blocks.iter_mut()
        .find(|v|v.id=="s23_decode_tool_wire").unwrap();
    inherited.instructions.push(i(9114,Op::ConstU32(3)));
    if let Terminator::Yield{next_args,..}=&mut inherited.terminator{
        next_args[2]=9114;
    }
    program.validate().expect("S23 active executable P2 transfer graph must validate");
    program
}

pub fn s23_manifest()->crate::SeedManifest{
    let p=s23_p2_remote_tool_transfer();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S23A-active-P2-specialized-REMOTE_EVAL-executable-and-tested-reuse",
        program_hash:format!("sha256:{}",p.content_hash().unwrap()),
        built_in_modules:vec![
            "S22-seeded-P2-role-inference-and-112-opcode-search",
            "researcher-supplied-P2-generic-TLV-frame-template",
            "VM-specializes-11B-observed-role-map-and-found-REMOTE_EVAL-op",
            "content-hash-and-metered-program-version-write",
            "isolated-EXEC-and-live-C-BadBody-functional-test",
            "only-validated-tool-registered-with-CAS-version",
            "inherited-tool-EXEC-and-live-C-compatibility-test",
            "failed-inheritance-falls-back-to-S22-cold-opcode-search",
            "real-B-job-request-created-solely-by-EXEC-registered-tool",
            "S22-known-grammar-RESULT-opcode-cold-search",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["researcher-seeded-P2-five-TLV-hypothesis".into()],
        prohibited_knowledge:vec![
            "private-P2-tags","private-P2-field-order",
            "private-REMOTE_EVAL-opcode","private-RESULT-opcode",
            "protocol-seed","held-out-B-answer","privileged-B-gas",
        ],
        research_status:"S23A_P2_RESEARCHER_TEMPLATE_VM_REWRITE_AND_REUSE_NOT_GENERAL_GRAMMAR_NOT_E001",
    }
}
