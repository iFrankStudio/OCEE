//! S20 — active VM creates TWO executable, content-addressed P1 tools:
//! REMOTE_EVAL (S19) and RESULT (S20). New worlds inherit only both sealed
//! program blobs and registry hashes. A new world uses live C feedback to
//! verify the inherited REMOTE_EVAL encoder and independently validates a
//! RESULT response before trusting the inherited RESULT encoder.
//!
//! This is a researcher-seeded P1 grammar, two template literal patches,
//! fixed FOLD32 contract and explicit fallback; NOT general tool invention,
//! unknown-family grammar induction, autonomous world migration, or E001.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s19_p1_transfer::s19_p1_transfer_aware;
use crate::s19_p1_tool::s19_seeded_frame_template;
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(v:&[u8])->Op{
    Op::ConstBytes{hex:v.iter().map(|b|format!("{b:02x}")).collect()}
}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    term:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator:term}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(target:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:target.into(),args}
}
fn br(c:u32,t:&str,ta:Vec<u32>,f:&str,fa:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:t.into(),true_args:ta,
        if_false:f.into(),false_args:fa}
}

pub fn s20_p1_dual_tool_transfer()->Program{
    let mut program=s19_p1_transfer_aware();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;

    // S19 already checks the inherited REMOTE_EVAL tool with a real C
    // BadBody response. A valid ACK now triggers a *separate* check of the
    // RESULT tool registry, not a researcher-controlled new program.
    let record=f.blocks.iter_mut().find(|x|x.id=="record").unwrap();
    record.terminator=y(WorldCall::Write,vec![291,290,292],
        "s20_result_registry_read",vec![]);
    // On cold first use, the same VM discovers the RESULT opcode with S18's
    // bounded scanning and only THEN specializes the second executable.
    let result_id=f.blocks.iter_mut().find(|x|x.id=="result_id").unwrap();
    result_id.terminator=br(517,"s20_make_result_tool",
        vec![513,511,512],"result_increment",vec![510,511,512]);

    f.blocks.extend([
        block("s20_result_registry_read",vec![p(8000,b)],vec![
            i(8001,lit(b"/self/tools/p1-result-frame")),
        ],y(WorldCall::Read,vec![8001],"s20_result_registry_answer",vec![])),
        block("s20_result_registry_answer",vec![p(8010,b)],vec![
            i(8011,lit(b"code")),
            i(8012,Op::JsonFieldString(8010,8011)),
            i(8013,lit(b"Ok")),
            i(8014,Op::EqBytes(8012,8013)),
        ],br(8014,"s20_result_tool_exec",vec![8010],
            "s20_result_fallback",vec![])),
        block("s20_result_fallback",vec![],vec![],
            // The original S18 result_read is a Yield continuation and
            // must receive one actual feedback Bytes, not a synthetic Jump.
            y(WorldCall::Observe,vec![],"result_read",vec![])),
        block("s20_result_tool_exec",vec![p(8020,b)],vec![
            i(8021,lit(b"payload")),
            i(8022,Op::JsonFieldBytes(8020,8021)),
            i(8023,lit(&[17,0,0,0])),
        ],y(WorldCall::Exec,vec![8022,8023],
            "s20_result_exec_reply",vec![])),
        block("s20_result_exec_reply",vec![p(8030,b)],vec![
            i(8031,lit(b"code")),
            i(8032,Op::JsonFieldString(8030,8031)),
            i(8033,lit(b"Ok")),
            i(8034,Op::EqBytes(8032,8033)),
        ],br(8034,"s20_result_prepare_send",vec![8030],
            "s20_result_fallback",vec![])),
        block("s20_result_prepare_send",vec![p(8040,b)],vec![
            i(8041,lit(b"payload")),
            i(8042,Op::JsonFieldBytes(8040,8041)),
            i(8043,lit(b"Bytes")),
            i(8044,Op::JsonFieldBytes(8042,8043)),
            i(8045,Op::ConstU32(1)),
            i(8046,Op::Len(8044)),
            i(8047,Op::ConstU32(2)),
            i(8048,Op::Sub(8046,8047)),
            i(8049,Op::Slice(8044,8045,8048)),
            i(8050,Op::ConstU32(0x7e)),
            i(8051,Op::ConstU32(0x7d)),
            i(8052,Op::ConstU32(0x20)),
            i(8053,Op::ByteStuffDecode(8049,8050,8051,8052)),
            i(8054,Op::ByteAt(8053,8045)), // learned opcode from own EXEC output
            i(8055,lit(b"/public/C/hello")),
        ],y(WorldCall::Read,vec![8055],"s20_result_read_hello",
            vec![8044,8054])),
        block("s20_result_read_hello",vec![
            p(8060,b),p(8061,u),p(8062,b)
        ],vec![
            i(8063,lit(b"payload")),
            i(8064,Op::JsonFieldBytes(8062,8063)),
            i(8065,lit(b"request")),
            i(8066,Op::JsonFieldBytes(8064,8065)),
            i(8067,Op::Len(8066)),
            i(8068,Op::ConstU32(2)),
            i(8069,Op::Sub(8067,8068)),
            i(8070,Op::ConstU32(1)),
            i(8071,Op::Slice(8066,8070,8069)),
            i(8072,Op::ConstU32(0x7e)),
            i(8073,Op::ConstU32(0x7d)),
            i(8074,Op::ConstU32(0x20)),
            i(8075,Op::ByteStuffDecode(8071,8072,8073,8074)),
            i(8076,Op::XorFold(8075)),
        ],y(WorldCall::Send,vec![8060],"s20_result_wait1",
            vec![8061,8076])),
        block("s20_result_wait1",vec![p(8080,u),p(8081,u),p(8082,b)],
            vec![],y(WorldCall::Observe,vec![],"s20_result_wait2",
                vec![8080,8081])),
        block("s20_result_wait2",vec![p(8090,u),p(8091,u),p(8092,b)],
            vec![],y(WorldCall::Observe,vec![],"s20_result_wait3",
                vec![8090,8091])),
        block("s20_result_wait3",vec![p(8100,u),p(8101,u),p(8102,b)],
            vec![],y(WorldCall::Observe,vec![],"s20_result_receive",
                vec![8100,8101])),
        block("s20_result_receive",vec![p(8110,u),p(8111,u),p(8112,b)],
            vec![],y(WorldCall::Receive,vec![],"s20_result_parse_response",
                vec![8110,8111])),
        block("s20_result_parse_response",vec![
            p(8120,u),p(8121,u),p(8122,b)
        ],vec![
            i(8123,lit(b"payload")),
            i(8124,Op::JsonFieldBytes(8122,8123)),
            i(8125,Op::ConstU32(0)),
            i(8126,Op::JsonIndex(8124,8125)),
            i(8127,lit(b"bytes")),
            i(8128,Op::JsonFieldBytes(8126,8127)),
            i(8129,Op::ByteAt(8128,8125)),
            i(8130,Op::ConstU32(0x7e)),
            i(8131,Op::EqU32(8129,8130)),
        ],br(8131,"s20_result_wire",vec![8120,8121,8128],
            "s20_result_fallback",vec![])),
        block("s20_result_wire",vec![
            p(8140,u),p(8141,u),p(8142,b)
        ],vec![
            i(8143,Op::Len(8142)),
            i(8144,Op::ConstU32(2)),
            i(8145,Op::Sub(8143,8144)),
            i(8146,Op::ConstU32(1)),
            i(8147,Op::Slice(8142,8146,8145)),
            i(8148,Op::ConstU32(0x7e)),
            i(8149,Op::ConstU32(0x7d)),
            i(8150,Op::ConstU32(0x20)),
            i(8151,Op::ByteStuffDecode(8147,8148,8149,8150)),
            i(8152,Op::Len(8151)),
            i(8153,Op::ConstU32(14)),
            i(8154,Op::EqU32(8152,8153)),
        ],br(8154,"s20_result_kind",vec![8140,8141,8151],
            "s20_result_fallback",vec![])),
        block("s20_result_kind",vec![p(8160,u),p(8161,u),p(8162,b)],vec![
            i(8163,Op::ConstU32(0)),
            i(8164,Op::ByteAt(8162,8163)),
            i(8165,Op::ConstU32(1)),
            i(8166,Op::EqU32(8164,8165)),
        ],br(8166,"s20_result_match_opcode",vec![8160,8161,8162],
            "s20_result_fallback",vec![])),
        block("s20_result_match_opcode",vec![
            p(8170,u),p(8171,u),p(8172,b)
        ],vec![
            i(8173,Op::ConstU32(1)),
            i(8174,Op::ByteAt(8172,8173)),
            i(8175,Op::EqU32(8174,8170)),
        ],br(8175,"s20_result_match_status",vec![8170,8171,8172],
            "s20_result_fallback",vec![])),
        block("s20_result_match_status",vec![
            p(8180,u),p(8181,u),p(8182,b)
        ],vec![
            i(8183,Op::ConstU32(8)),
            i(8184,Op::ByteAt(8182,8183)),
            i(8185,Op::ConstU32(0)),
            i(8186,Op::EqU32(8184,8185)),
        ],br(8186,"s20_result_match_checksum",vec![8180,8181,8182],
            "s20_result_fallback",vec![])),
        block("s20_result_match_checksum",vec![
            p(8190,u),p(8191,u),p(8192,b)
        ],vec![
            i(8193,Op::XorFold(8192)),
            i(8194,Op::EqU32(8193,8191)),
        ],br(8194,"s20_result_match_id",vec![8192],
            "s20_result_fallback",vec![])),
        block("s20_result_match_id",vec![p(8200,b)],vec![
            i(8201,Op::ConstU32(4)),
            i(8202,Op::Slice(8200,8201,8201)),
            i(8203,lit(&[17,0,0,0])),
            i(8204,Op::EqBytes(8202,8203)),
        ],br(8204,"result_save",vec![8200],
            "s20_result_fallback",vec![])),
        // The cold RESULT scan has proved a legal RESULT from the live C
        // service and matched request_id, checksum and success status.
        // Patch the same researcher-supplied generic encoder, but with
        // RESULT opcode instead of REMOTE_EVAL opcode.
        block("s20_make_result_tool",vec![
            p(8210,b),p(8211,u),p(8212,u)
        ],vec![
            i(8213,lit(&s19_seeded_frame_template().canonical_bytes())),
            i(8214,lit(b"entry")),
            i(8215,Op::ConstU32(1)),
            i(8216,Op::PackByte(8212)),
            i(8217,Op::HexEncode(8216)),
            i(8218,lit(br#"{"ConstBytes":{"hex":"00"#)),
            i(8219,Op::Concat(8218,8217)),
            i(8220,lit(br#""}}"#)),
            i(8221,Op::Concat(8219,8220)),
            i(8222,Op::RewriteOp(8213,8214,8215,8221)),
            i(8223,Op::PackByte(8211)),
            i(8224,Op::HexEncode(8223)),
            i(8225,lit(br#"{"ConstBytes":{"hex":""#)),
            i(8226,Op::Concat(8225,8224)),
            i(8227,Op::Concat(8226,8220)),
            i(8228,Op::ConstU32(2)),
            i(8229,Op::RewriteOp(8222,8214,8228,8227)),
            i(8230,lit(b"OCEE-PROGRAM-V1\0")),
            i(8231,Op::Concat(8230,8229)),
            i(8232,Op::Sha256(8231)),
            i(8233,Op::HexEncode(8232)),
            i(8234,lit(b"/self/versions/")),
            i(8235,Op::Concat(8234,8233)),
            i(8236,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![8235,8229,8236],
            "s20_result_tool_test",vec![8210,8233,8211,8212])),
        block("s20_result_tool_test",vec![
            p(8240,b),p(8241,b),p(8242,u),p(8243,u),p(8244,b)
        ],vec![
            i(8245,lit(&[17,0,0,0])),
        ],y(WorldCall::Exec,vec![8241,8245],
            "s20_result_tool_test_reply",vec![8240,8241,8242,8243])),
        block("s20_result_tool_test_reply",vec![
            p(8250,b),p(8251,b),p(8252,u),p(8253,u),p(8254,b)
        ],vec![
            i(8255,lit(b"code")),
            i(8256,Op::JsonFieldString(8254,8255)),
            i(8257,lit(b"Ok")),
            i(8258,Op::EqBytes(8256,8257)),
        ],br(8258,"s20_result_tool_compare",
            vec![8250,8251,8252,8253,8254],
            "result_save",vec![8250])),
        block("s20_result_tool_compare",vec![
            p(8260,b),p(8261,b),p(8262,u),p(8263,u),p(8264,b)
        ],vec![
            i(8265,lit(b"payload")),
            i(8266,Op::JsonFieldBytes(8264,8265)),
            i(8267,lit(b"Bytes")),
            i(8268,Op::JsonFieldBytes(8266,8267)),
            // Independent frame oracle executed *inside* active VM; its
            // algorithm, like the generated template, is a seeded prior.
            i(8269,lit(&[0])),
            i(8270,Op::PackByte(8263)),
            i(8271,Op::Concat(8269,8270)),
            i(8272,lit(&[17,0,0,0])),
            i(8273,Op::ConstU32(4)),
            i(8274,Op::PackU16Be(8273)),
            i(8275,Op::Concat(8271,8274)),
            i(8276,Op::Concat(8275,8272)),
            i(8277,Op::XorFold(8276)),
            i(8278,Op::Xor(8277,8262)),
            i(8279,Op::PackByte(8278)),
            i(8280,Op::Concat(8276,8279)),
            i(8281,Op::ConstU32(0x7e)),
            i(8282,Op::ConstU32(0x7d)),
            i(8283,Op::ConstU32(0x20)),
            i(8284,Op::ByteStuffEncode(8280,8281,8282,8283)),
            i(8285,lit(&[0x7e])),
            i(8286,Op::Concat(8285,8284)),
            i(8287,Op::Concat(8286,8285)),
            i(8288,Op::EqBytes(8268,8287)),
        ],br(8288,"s20_result_check_existing",vec![8260,8261],
            "result_save",vec![8260])),
        block("s20_result_check_existing",vec![p(8290,b),p(8291,b)],vec![
            i(8292,lit(b"/self/tools/p1-result-frame")),
        ],y(WorldCall::Read,vec![8292],
            "s20_result_choose_cas",vec![8290,8291])),
        block("s20_result_choose_cas",vec![
            p(8300,b),p(8301,b),p(8302,b)
        ],vec![
            i(8303,lit(b"code")),
            i(8304,Op::JsonFieldString(8302,8303)),
            i(8305,lit(b"Ok")),
            i(8306,Op::EqBytes(8304,8305)),
        ],br(8306,"s20_result_replace",vec![8300,8301],
            "s20_result_register_new",vec![8300,8301])),
        block("s20_result_replace",vec![p(8310,b),p(8311,b)],vec![
            i(8312,lit(b"/self/tools/p1-result-frame")),
            i(8313,Op::ConstU32(0)),
        ],y(WorldCall::Write,vec![8312,8311,8313],
            "s20_result_registered",vec![8310])),
        block("s20_result_register_new",vec![p(8320,b),p(8321,b)],vec![
            i(8322,lit(b"/self/tools/p1-result-frame")),
            i(8323,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![8322,8321,8323],
            "s20_result_registered",vec![8320])),
        block("s20_result_registered",vec![p(8330,b),p(8331,b)],vec![],
            j("result_save",vec![8330])),
    ]);
    program.validate().expect("S20 dual executable VM must pass static verification");
    program
}

pub fn s20_manifest()->crate::SeedManifest{
    let program=s20_p1_dual_tool_transfer();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S20-active-P1-dual-executable-tool-transfer",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S19-verified-REMOTE_EVAL-tool-generation-and-transfer",
            "seeded-P1-RESULT-frame-encoder-template",
            "result-opcode-learned-from-live-ACK-RESULT-probing",
            "VM-two-RewriteOp-and-sha256-immutable-result-program",
            "isolated-EXEC-and-independent-byte-equality-test",
            "CAS-registered-RESULT-tool",
            "inherited-RESULT-tool-EXEC-live-C-frame-validation",
            "mismatch-falls-back-to-real-RESULT-opcode-scan",
            "seeded-FOLD32-three-u32-contract-and-request-id17",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P1-escaped-XOR".into()],
        prohibited_knowledge:vec![
            "private-P1-XOR-key","private-REMOTE_EVAL-opcode",
            "private-RESULT-opcode","protocol-private-seed",
            "B-privileged-resource-ledger","actual-target-job-answer",
        ],
        research_status:"S20_DEV_SEEDED_DUAL_P1_TOOL_REUSE_NOT_P2_NOT_E001",
    }
}
