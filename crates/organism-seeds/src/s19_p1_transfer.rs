//! S19B: carry a VM-specialized P1 tool to a FRESH world, probe the
//! current service with malformed workload (no B charge), and reuse only
//! when the C diagnostic confirms current instance compatibility.
//! Stale tools are abandoned, a cold VM discovery is run, and a stale
//! registry is replaced with CAS(0). All policy is researcher-seeded.
//! The transfer test imports ONLY immutable program bytes + registry hash
//! using ordinary metered writes; no hidden state or continuation is copied.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s19_p1_tool::s19_p1_build_and_use_tool;

fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|v|format!("{v:02x}")).collect()}
}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn jump(to:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:to.into(),args}
}
fn yield_to(call:WorldCall,args:Vec<u32>,to:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:to.into(),next_args:carry}
}
fn branch(c:u32,yes:&str,yes_args:Vec<u32>,no:&str,no_args:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:yes.into(),true_args:yes_args,
        if_false:no.into(),false_args:no_args}
}
/// A single active VM program for fresh and inherited P1 worlds.
pub fn s19_p1_transfer_aware()->Program{
    let mut program=s19_p1_build_and_use_tool();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    f.blocks[0].id="cold_start".into();
    let mut prefix=vec![
        block("entry",vec![p(7000,b)],vec![
            i(7001,lit(b"/self/tools/p1-eval-frame")),
        ],yield_to(WorldCall::Read,vec![7001],"reuse_gate",vec![])),
        block("reuse_gate",vec![p(7010,b)],vec![
            i(7011,lit(b"code")),
            i(7012,Op::JsonFieldString(7010,7011)),
            i(7013,lit(b"Ok")),
            i(7014,Op::EqBytes(7012,7013)),
        ],branch(7014,"reuse_get_hash",vec![7010],"s19_restart",vec![])),
        block("reuse_get_hash",vec![p(7020,b)],vec![
            i(7021,lit(b"payload")),
            i(7022,Op::JsonFieldBytes(7020,7021)),
            i(7023,lit(b"/public/C/hello")),
        ],yield_to(WorldCall::Read,vec![7023],"reuse_sample",vec![7022])),
        block("reuse_sample",vec![p(7030,b),p(7031,b)],vec![
            i(7032,lit(b"payload")),
            i(7033,Op::JsonFieldBytes(7031,7032)),
            i(7034,lit(b"request")),
            i(7035,Op::JsonFieldBytes(7033,7034)),
            i(7036,Op::Len(7035)),
            i(7037,Op::ConstU32(2)),
            i(7038,Op::Sub(7036,7037)),
            i(7039,Op::ConstU32(1)),
            i(7040,Op::Slice(7035,7039,7038)),
            i(7041,Op::ConstU32(0x7e)),
            i(7042,Op::ConstU32(0x7d)),
            i(7043,Op::ConstU32(0x20)),
            i(7044,Op::ByteStuffDecode(7040,7041,7042,7043)),
            i(7045,Op::XorFold(7044)),
            i(7046,lit(&[17,0,0,0])),
        ],yield_to(WorldCall::Exec,vec![7030,7046],
            "reuse_exec_feedback",vec![7044,7045,7030])),
        block("reuse_exec_feedback",vec![
            p(7050,b),p(7051,u),p(7052,b),p(7053,b)
        ],vec![
            i(7054,lit(b"code")),
            i(7055,Op::JsonFieldString(7053,7054)),
            i(7056,lit(b"Ok")),
            i(7057,Op::EqBytes(7055,7056)),
        ],branch(7057,"reuse_send_probe",vec![7050,7051,7052,7053],
            "s19_restart",vec![])),
        block("reuse_send_probe",vec![
            p(7060,b),p(7061,u),p(7062,b),p(7063,b)
        ],vec![
            i(7064,lit(b"payload")),
            i(7065,Op::JsonFieldBytes(7063,7064)),
            i(7066,lit(b"Bytes")),
            i(7067,Op::JsonFieldBytes(7065,7066)),
            i(7068,Op::Len(7067)),
            i(7069,Op::ConstU32(2)),
            i(7070,Op::Sub(7068,7069)),
            i(7071,Op::ConstU32(1)),
            i(7072,Op::Slice(7067,7071,7070)),
            i(7073,Op::ConstU32(0x7e)),
            i(7074,Op::ConstU32(0x7d)),
            i(7075,Op::ConstU32(0x20)),
            i(7076,Op::ByteStuffDecode(7072,7073,7074,7075)),
            i(7077,Op::ByteAt(7076,7071)), // opcode from tool's actual frame
        ],yield_to(WorldCall::Send,vec![7067],"reuse_wait1",
            vec![7060,7061,7062,7077])),
        block("reuse_wait1",vec![
            p(7080,b),p(7081,u),p(7082,b),p(7083,u),p(7084,b)
        ],vec![],yield_to(WorldCall::Observe,vec![],"reuse_wait2",
            vec![7080,7081,7082,7083])),
        block("reuse_wait2",vec![
            p(7090,b),p(7091,u),p(7092,b),p(7093,u),p(7094,b)
        ],vec![],yield_to(WorldCall::Observe,vec![],"reuse_wait3",
            vec![7090,7091,7092,7093])),
        block("reuse_wait3",vec![
            p(7100,b),p(7101,u),p(7102,b),p(7103,u),p(7104,b)
        ],vec![],yield_to(WorldCall::Observe,vec![],"reuse_receive",
            vec![7100,7101,7102,7103])),
        block("reuse_receive",vec![
            p(7110,b),p(7111,u),p(7112,b),p(7113,u),p(7114,b)
        ],vec![],yield_to(WorldCall::Receive,vec![],"reuse_reply",
            vec![7110,7111,7112,7113])),
        block("reuse_reply",vec![
            p(7120,b),p(7121,u),p(7122,b),p(7123,u),p(7124,b)
        ],vec![
            i(7125,lit(b"payload")),
            i(7126,Op::JsonFieldBytes(7124,7125)),
            i(7127,Op::ConstU32(0)),
            i(7128,Op::JsonIndex(7126,7127)),
            i(7129,lit(b"bytes")),
            i(7130,Op::JsonFieldBytes(7128,7129)),
            i(7131,Op::ByteAt(7130,7127)),
            i(7132,Op::ConstU32(0x7b)), // only JSON diagnostics
            i(7133,Op::EqU32(7131,7132)),
        ],branch(7133,"reuse_diagnostic",
            vec![7120,7121,7122,7123,7130],
            "s19_restart",vec![])),
        block("reuse_diagnostic",vec![
            p(7140,b),p(7141,u),p(7142,b),p(7143,u),p(7144,b)
        ],vec![
            i(7145,lit(b"code")),
            i(7146,Op::JsonFieldString(7144,7145)),
            i(7147,lit(b"BadBody")),
            i(7148,Op::EqBytes(7146,7147)),
        ],branch(7148,"read_job",vec![7140,7141,7143,7142],
            "s19_restart",vec![])),
        block("s19_restart",vec![],vec![
            i(7150,lit(b"")),
        ],jump("cold_start",vec![7150])),
    ];
    // A stale inherited pointer may exist. Detect it with a normal read,
    // then bind the freshly learned immutable tool using a checked CAS(0).
    // There is no host-side correction of the registry state.
    let register=f.blocks.iter_mut().find(|b|b.id=="s19_register").unwrap();
    register.terminator=yield_to(WorldCall::Read,vec![6084],
        "s19_reg_check",vec![6080,6081,6082,6083]);
    prefix.extend([
        block("s19_reg_check",vec![
            p(7160,b),p(7161,b),p(7162,u),p(7163,u),p(7164,b)
        ],vec![
            i(7165,lit(b"code")),
            i(7166,Op::JsonFieldString(7164,7165)),
            i(7167,lit(b"Ok")),
            i(7168,Op::EqBytes(7166,7167)),
        ],branch(7168,"s19_reg_replace",vec![7160,7161,7162,7163],
            "s19_reg_new",vec![7160,7161,7162,7163])),
        block("s19_reg_replace",vec![
            p(7170,b),p(7171,b),p(7172,u),p(7173,u)
        ],vec![
            i(7174,lit(b"/self/tools/p1-eval-frame")),
            i(7175,Op::ConstU32(0)), // known first revision of inherited object
        ],yield_to(WorldCall::Write,vec![7174,7170,7175],
            "read_job",vec![7171,7172,7173,7170])),
        block("s19_reg_new",vec![
            p(7180,b),p(7181,b),p(7182,u),p(7183,u)
        ],vec![
            i(7184,lit(b"/self/tools/p1-eval-frame")),
            i(7185,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![7184,7180,7185],
            "read_job",vec![7181,7182,7183,7180])),
    ]);
    prefix.extend(std::mem::take(&mut f.blocks));
    f.blocks=prefix;
    program.validate().expect("S19 transfer-aware VM seed must validate");
    program
}
pub fn s19_transfer_manifest()->crate::SeedManifest {
    let program=s19_p1_transfer_aware();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S19-P1-cross-world-verified-tool-reuse",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "known-P1-registered-tool-probe-with-empty-workload",
            "BadBody-validity-and-compatibility-check",
            "inherit-only-whitelisted-program-version-and-hash",
            "fall-back-to-S19-active-discovery-and-tool-specialization",
            "CAS-replace-stale-tool-hash",
            "S18-valid-result-retrieval-and-independent-test",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P1-delimiter-XOR".into()],
        prohibited_knowledge:vec![
            "new-world-private-protocol-seed","private-REMOTE_EVAL-opcode",
            "new-world-private-checksum-key","held-out-work-item-answer",
            "world-privileged-B-gas","researcher-evaluation-score"
        ],
        research_status:"S19_RESEARCHER_SEEDED_REUSE_POLICY_NOT_NEW_GRAMMAR_NOT_E001",
    }
}
