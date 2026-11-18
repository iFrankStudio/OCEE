//! S18: an ACTUAL active OCEE VM program that learns a P1 REMOTE_EVAL
//! opcode from public HELLO and C diagnostics, persists its discovery, then
//! invokes a real bounded C->B task and discovers RESULT for the actual value. No host oracle selects any opcode.
//!
//! IMPORTANT SEEDS: P1 delimiter/escape rules (0x7e,0x7d,^0x20), P1 inner
//! layout, checksum derived from public HELLO, BadBody selector, 112 opcode
//! candidates, fixed three-u32 FOLD32 payload, and ACK shape are prewritten.
//! Unknown: private seed, XOR key value, marker-derived frame contents,
//! hidden REMOTE_EVAL opcode, and workload values. This is NOT open grammar
//! discovery, new tool invention, or E001 confirmation.
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};

fn literal(v:&[u8])->Op {
    Op::ConstBytes{hex:v.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn a(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    end:Terminator)->Block {
    Block{id:name.into(),params,instructions,terminator:end}
}
fn jump(to:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:to.into(),args}
}
fn yield_to(call:WorldCall,args:Vec<u32>,next:&str,next_args:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args}
}
fn branch(cond:u32,yes:&str,yes_args:Vec<u32>,
    no:&str,no_args:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:yes_args,
        if_false:no.into(),false_args:no_args}
}

pub fn s18_p1_discover_and_invoke()->Program {
    let b=Type::Bytes;
    let u=Type::U32;
    let program=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{name:"main".into(),args:vec![b],returns:u,blocks:vec![
        block("entry",vec![a(0,b)],vec![
            i(1,literal(b"/public/C/hello")),
        ],yield_to(WorldCall::Read,vec![1],"sample",vec![])),
        block("sample",vec![a(10,b)],vec![
            i(11,literal(b"payload")),
            i(12,Op::JsonFieldBytes(10,11)),
            i(13,literal(b"request")),
            i(14,Op::JsonFieldBytes(12,13)),
            i(15,Op::ConstU32(1)),
            i(16,Op::ConstU32(2)),
            i(17,Op::Len(14)),
            i(18,Op::Sub(17,16)),
            i(19,Op::Slice(14,15,18)), // strip public P1 delimiters
            i(20,Op::ConstU32(0x7e)),
            i(21,Op::ConstU32(0x7d)),
            i(22,Op::ConstU32(0x20)),
            i(23,Op::ByteStuffDecode(19,20,21,22)),
            i(24,Op::XorFold(23)),       // key from valid public HELLO
            i(25,Op::ConstU32(0x10)),
        ],jump("scan",vec![23,24,25])),
        block("scan",vec![a(30,b),a(31,u),a(32,u)],vec![],
            Terminator::RepeatBounded{limit:112,
                body:"probe".into(),body_args:vec![30,31,32],
                done:"failure".into(),done_args:vec![]}),
        block("probe",vec![a(40,b),a(41,u),a(42,u)],vec![
            i(43,Op::ConstU32(1)),
            i(44,Op::Len(40)),
            i(45,Op::Sub(44,43)),
            i(46,Op::ReplaceByte(40,43,42)),
            i(47,Op::ConstU32(0)),
            i(48,Op::Slice(46,47,45)),
            i(49,Op::XorFold(48)),
            i(50,Op::Xor(49,41)),
            i(51,Op::PackByte(50)),
            i(52,Op::Concat(48,51)),
            i(53,Op::ConstU32(0x7e)),
            i(54,Op::ConstU32(0x7d)),
            i(55,Op::ConstU32(0x20)),
            i(56,Op::ByteStuffEncode(52,53,54,55)),
            i(57,literal(&[0x7e])),
            i(58,Op::Concat(57,56)),
            i(59,Op::Concat(58,57)),
        ],yield_to(WorldCall::Send,vec![59],"wait1",vec![40,41,42,59])),
        block("wait1",vec![a(60,b),a(61,u),a(62,u),a(63,b),a(64,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"wait2",vec![60,61,62,63])),
        block("wait2",vec![a(70,b),a(71,u),a(72,u),a(73,b),a(74,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"wait3",vec![70,71,72,73])),
        block("wait3",vec![a(80,b),a(81,u),a(82,u),a(83,b),a(84,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"receive",vec![80,81,82,83])),
        block("receive",vec![a(90,b),a(91,u),a(92,u),a(93,b),a(94,b)],vec![],
            yield_to(WorldCall::Receive,vec![],"decode",vec![90,91,92,93])),
        block("decode",vec![a(100,b),a(101,u),a(102,u),a(103,b),a(104,b)],vec![
            i(105,literal(b"payload")),
            i(106,Op::JsonFieldBytes(104,105)),
            i(107,Op::ConstU32(0)),
            i(108,Op::JsonIndex(106,107)),
            i(109,literal(b"bytes")),
            i(110,Op::JsonFieldBytes(108,109)),
            i(111,Op::ByteAt(110,107)),
            i(112,Op::ConstU32(0x7b)),
            i(113,Op::EqU32(111,112)),
        ],branch(113,"diagnostic",vec![100,101,102,103,110],
            "increment",vec![100,101,102])),
        block("diagnostic",vec![a(120,b),a(121,u),a(122,u),a(123,b),a(124,b)],vec![
            i(125,literal(b"code")),
            i(126,Op::JsonFieldString(124,125)),
            i(127,literal(b"BadBody")),
            i(128,Op::EqBytes(126,127)),
        ],branch(128,"found",vec![123,120,121,122],
            "increment",vec![120,121,122])),
        block("increment",vec![a(130,b),a(131,u),a(132,u)],vec![
            i(133,Op::ConstU32(1)),
            i(134,Op::Add(132,133)),
        ],jump("scan",vec![130,131,134])),
        block("found",vec![a(140,b),a(141,b),a(142,u),a(143,u)],vec![
            i(144,literal(b"/self/discoveries/p1-operation-candidate")),
            i(145,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![144,140,145],
            "read_job",vec![141,142,143])),
        block("read_job",vec![a(150,b),a(151,u),a(152,u),a(153,b)],vec![
            i(154,literal(b"/public/E001/job0")),
        ],yield_to(WorldCall::Read,vec![154],"build_eval",vec![150,151,152])),
        block("build_eval",vec![a(160,b),a(161,u),a(162,u),a(163,b)],vec![
            i(164,literal(b"payload")),
            i(165,Op::JsonFieldBytes(163,164)),
            i(166,literal(b"values_le")),
            i(167,Op::JsonFieldBytes(165,166)),
            i(168,literal(&[3,0])),
            i(169,Op::Concat(168,167)), // count3 + public job values
            i(170,Op::ConstU32(0)),
            i(171,Op::ConstU32(2)),
            i(172,Op::Slice(160,170,171)), // sample's kind + opcode
            i(173,Op::ConstU32(1)),
            i(174,Op::ReplaceByte(172,173,162)),
            i(175,Op::ConstU32(18)), // id4 + count2 + 3*u32
            i(176,Op::PackU16Be(175)),
            i(177,Op::Concat(174,176)),
            i(178,literal(&[17,0,0,0])), // fresh request-id
            i(179,Op::Concat(177,178)),
            i(180,Op::Concat(179,169)),
            i(181,Op::XorFold(180)),
            i(182,Op::Xor(181,161)),
            i(183,Op::PackByte(182)),
            i(184,Op::Concat(180,183)),
            i(185,Op::ConstU32(0x7e)),
            i(186,Op::ConstU32(0x7d)),
            i(187,Op::ConstU32(0x20)),
            i(188,Op::ByteStuffEncode(184,185,186,187)),
            i(189,literal(&[0x7e])),
            i(190,Op::Concat(189,188)),
            i(191,Op::Concat(190,189)),
        ],yield_to(WorldCall::Send,vec![191],"ack_wait1",vec![162])),
        block("ack_wait1",vec![a(200,u),a(201,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"ack_wait2",vec![200])),
        block("ack_wait2",vec![a(210,u),a(211,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"ack_wait3",vec![210])),
        block("ack_wait3",vec![a(220,u),a(221,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"ack_recv",vec![220])),
        block("ack_recv",vec![a(230,u),a(231,b)],vec![],
            yield_to(WorldCall::Receive,vec![],"ack_decode",vec![230])),
        block("ack_decode",vec![a(240,u),a(241,b)],vec![
            i(242,literal(b"payload")),
            i(243,Op::JsonFieldBytes(241,242)),
            i(244,Op::ConstU32(0)),
            i(245,Op::JsonIndex(243,244)),
            i(246,literal(b"bytes")),
            i(247,Op::JsonFieldBytes(245,246)),
            i(248,Op::ByteAt(247,244)),
            i(249,Op::ConstU32(0x7e)),
            i(250,Op::EqU32(248,249)),
        ],branch(250,"ack_verify",vec![247,240],"failure",vec![])),
        block("ack_verify",vec![a(260,b),a(261,u)],vec![
            i(262,Op::Len(260)),
            i(263,Op::ConstU32(2)),
            i(264,Op::Sub(262,263)),
            i(265,Op::ConstU32(1)),
            i(266,Op::Slice(260,265,264)),
            i(267,Op::ConstU32(0x7e)),
            i(268,Op::ConstU32(0x7d)),
            i(269,Op::ConstU32(0x20)),
            i(270,Op::ByteStuffDecode(266,267,268,269)),
            i(271,Op::ByteAt(270,265)),
            i(272,Op::EqU32(271,261)),
        ],branch(272,"ack_status",vec![260,270],"failure",vec![])),
        block("ack_status",vec![a(280,b),a(281,b)],vec![
            i(282,Op::ConstU32(8)),
            i(283,Op::ByteAt(281,282)),
            i(284,Op::ConstU32(0)),
            i(285,Op::EqU32(283,284)),
        ],branch(285,"record",vec![280],"failure",vec![])),
        block("record",vec![a(290,b)],vec![
            i(291,literal(b"/self/results/p1-remote-eval-ack")),
            i(292,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![291,290,292],"result_read",vec![])),
        // Stage 3: same active VM discovers an UNKNOWN RESULT opcode.
        // The work item was already accepted; test only legitimate result
        // response structure, not B gas or externally supplied answers.
        block("result_read",vec![a(320,b)],vec![
            i(321,literal(b"/public/C/hello")),
        ],yield_to(WorldCall::Read,vec![321],"result_sample",vec![])),
        block("result_sample",vec![a(330,b)],vec![
            i(331,literal(b"payload")),
            i(332,Op::JsonFieldBytes(330,331)),
            i(333,literal(b"request")),
            i(334,Op::JsonFieldBytes(332,333)),
            i(335,Op::ConstU32(1)),
            i(336,Op::ConstU32(2)),
            i(337,Op::Len(334)),
            i(338,Op::Sub(337,336)),
            i(339,Op::Slice(334,335,338)),
            i(340,Op::ConstU32(0x7e)),
            i(341,Op::ConstU32(0x7d)),
            i(342,Op::ConstU32(0x20)),
            i(343,Op::ByteStuffDecode(339,340,341,342)),
            i(344,Op::XorFold(343)),
            i(345,Op::ConstU32(0x10)),
        ],jump("result_scan",vec![343,344,345])),
        block("result_scan",vec![a(350,b),a(351,u),a(352,u)],vec![],
            Terminator::RepeatBounded{limit:112,
                body:"result_probe".into(),body_args:vec![350,351,352],
                done:"failure".into(),done_args:vec![]}),
        block("result_probe",vec![a(360,b),a(361,u),a(362,u)],vec![
            i(363,Op::ConstU32(0)),
            i(364,Op::ConstU32(4)),
            i(365,Op::Slice(360,363,364)), // kind0 + observed opcode + len4
            i(366,Op::ConstU32(1)),
            i(367,Op::ReplaceByte(365,366,362)),
            i(368,literal(&[17,0,0,0])),
            i(369,Op::Concat(367,368)),     // same request_id as REMOTE_EVAL
            i(370,Op::XorFold(369)),
            i(371,Op::Xor(370,361)),
            i(372,Op::PackByte(371)),
            i(373,Op::Concat(369,372)),
            i(374,Op::ConstU32(0x7e)),
            i(375,Op::ConstU32(0x7d)),
            i(376,Op::ConstU32(0x20)),
            i(377,Op::ByteStuffEncode(373,374,375,376)),
            i(378,literal(&[0x7e])),
            i(379,Op::Concat(378,377)),
            i(380,Op::Concat(379,378)),
        ],yield_to(WorldCall::Send,vec![380],"result_wait1",vec![360,361,362])),
        block("result_wait1",vec![a(390,b),a(391,u),a(392,u),a(393,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"result_wait2",vec![390,391,392])),
        block("result_wait2",vec![a(400,b),a(401,u),a(402,u),a(403,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"result_wait3",vec![400,401,402])),
        block("result_wait3",vec![a(410,b),a(411,u),a(412,u),a(413,b)],vec![],
            yield_to(WorldCall::Observe,vec![],"result_receive",vec![410,411,412])),
        block("result_receive",vec![a(420,b),a(421,u),a(422,u),a(423,b)],vec![],
            yield_to(WorldCall::Receive,vec![],"result_decode",vec![420,421,422])),
        block("result_decode",vec![a(430,b),a(431,u),a(432,u),a(433,b)],vec![
            i(434,literal(b"payload")),
            i(435,Op::JsonFieldBytes(433,434)),
            i(436,Op::ConstU32(0)),
            i(437,Op::JsonIndex(435,436)),
            i(438,literal(b"bytes")),
            i(439,Op::JsonFieldBytes(437,438)),
            i(440,Op::ByteAt(439,436)),
            i(441,Op::ConstU32(0x7e)),
            i(442,Op::EqU32(440,441)),
        ],branch(442,"result_wire",vec![430,431,432,439],
            "result_increment",vec![430,431,432])),
        block("result_wire",vec![a(450,b),a(451,u),a(452,u),a(453,b)],vec![
            i(454,Op::Len(453)),
            i(455,Op::ConstU32(2)),
            i(456,Op::Sub(454,455)),
            i(457,Op::ConstU32(1)),
            i(458,Op::Slice(453,457,456)),
            i(459,Op::ConstU32(0x7e)),
            i(460,Op::ConstU32(0x7d)),
            i(461,Op::ConstU32(0x20)),
            i(462,Op::ByteStuffDecode(458,459,460,461)),
            i(463,Op::Len(462)),
            i(464,Op::ConstU32(14)), // P1: header4+request_id4+status1+result4+check1
            i(465,Op::EqU32(463,464)),
        ],branch(465,"result_kind",vec![450,451,452,462],
            "result_increment",vec![450,451,452])),
        block("result_kind",vec![a(470,b),a(471,u),a(472,u),a(473,b)],vec![
            i(474,Op::ConstU32(0)),
            i(475,Op::ByteAt(473,474)),
            i(476,Op::ConstU32(1)),
            i(477,Op::EqU32(475,476)),
        ],branch(477,"result_opcode",vec![470,471,472,473],
            "result_increment",vec![470,471,472])),
        block("result_opcode",vec![a(480,b),a(481,u),a(482,u),a(483,b)],vec![
            i(484,Op::ConstU32(1)),
            i(485,Op::ByteAt(483,484)),
            i(486,Op::EqU32(485,482)),
        ],branch(486,"result_status",vec![480,481,482,483],
            "result_increment",vec![480,481,482])),
        block("result_status",vec![a(490,b),a(491,u),a(492,u),a(493,b)],vec![
            i(494,Op::ConstU32(8)),
            i(495,Op::ByteAt(493,494)),
            i(496,Op::ConstU32(0)),
            i(497,Op::EqU32(495,496)),
        ],branch(497,"result_check",vec![490,491,492,493],
            "result_increment",vec![490,491,492])),
        block("result_check",vec![a(500,b),a(501,u),a(502,u),a(503,b)],vec![
            i(504,Op::XorFold(503)),
            i(505,Op::EqU32(504,501)),
        ],branch(505,"result_id",vec![500,501,502,503],
            "result_increment",vec![500,501,502])),
        block("result_id",vec![a(510,b),a(511,u),a(512,u),a(513,b)],vec![
            i(514,Op::ConstU32(4)),
            i(515,Op::Slice(513,514,514)),
            i(516,literal(&[17,0,0,0])),
            i(517,Op::EqBytes(515,516)),
        ],branch(517,"result_save",vec![513],
            "result_increment",vec![510,511,512])),
        block("result_save",vec![a(520,b)],vec![
            i(521,Op::ConstU32(9)),
            i(522,Op::ConstU32(4)),
            i(523,Op::Slice(520,521,522)),
            i(524,literal(b"/self/results/p1-job0/value_le")),
            i(525,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![524,523,525],"success",vec![])),
        block("result_increment",vec![a(530,b),a(531,u),a(532,u)],vec![
            i(533,Op::ConstU32(1)),
            i(534,Op::Add(532,533)),
        ],jump("result_scan",vec![530,531,534])),
        block("success",vec![a(300,b)],vec![
            i(301,Op::ConstU32(1)),
        ],Terminator::Return{value:301}),
        block("failure",vec![],vec![
            i(310,Op::ConstU32(0)),
        ],Terminator::Return{value:310}),
    ]}]};
    program.validate().expect("S18 P1 in-world probe must verify");
    program
}

/// Mechanism ablation: same graph / SEND candidates / resources, but
/// disable diagnostic selection. It should not call B or save a discovery.
pub fn s18_p1_no_diagnostic_control()->Program {
    let mut program=s18_p1_discover_and_invoke();
    let diag=program.functions[0].blocks.iter_mut()
        .find(|b|b.id=="diagnostic").unwrap();
    diag.instructions.iter_mut().find(|ins|ins.id==127).unwrap().op=
        literal(b"Unrecognized");
    program.validate().expect("S18 no-diagnostic control must typecheck");
    program
}

/// Mechanism ablation: the remote B computation still occurs, but the VM
/// cannot recognize a correctly shaped RESULT frame and must not persist it.
/// This isolates result identification from earlier opcode discovery.
pub fn s18_p1_no_result_shape_control()->Program {
    let mut program=s18_p1_discover_and_invoke();
    let wire=program.functions[0].blocks.iter_mut()
        .find(|b|b.id=="result_wire").unwrap();
    wire.instructions.iter_mut().find(|ins|ins.id==464).unwrap().op=
        Op::ConstU32(15); // actual P1 RESULT frame is 14 bytes
    program.validate().expect("S18 result-shape ablation must typecheck");
    program
}

pub fn s18_manifest()->crate::SeedManifest {
    crate::SeedManifest{
        manifest_version:1,
        kind:"S18-P1-active-VM-discover-and-invoke",
        program_hash:format!("sha256:{}",s18_p1_discover_and_invoke()
            .content_hash().expect("S18 initial program valid")),
        built_in_modules:vec![
            "KNOWN-P1-delimiter-and-escape-rules",
            "KNOWN-P1-inner-frame-layout-and-XOR-checksum",
            "derive-check-key-from-public-HELLO",
            "generic-VM-byte-stuffing-and-be-u16-ops",
            "bounded-112-REMOTE_EVAL-opcode-search",
            "BadBody-diagnostic-selection",
            "fixed-three-u32-FOLD32-workload-shape",
            "P1-ACK-shape-and-success-status-comparison",
            "bounded-112-RESULT-opcode-search-by-valid-output-shape",
            "persist-and-verify-real-returned-FOLD32-u32",
            "ordinary-SEND-RECEIVE-WRITE-actions",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P1-escaped-delimiter-XOR".into()],
        prohibited_knowledge:vec![
            "hidden-REMOTE_EVAL-opcode-value","hidden-P1-XOR-key-value",
            "protocol-generator-seed","B-privileged-gas-account",
            "work-item-value-contents","RESULT-opcode-value",
            "external-evaluator-answer",
        ],
        research_status:"ACTIVE_VM_P1_SEEDED_GRAMMAR_LEARNING_NOT_NEW_GRAMMAR_NOT_E001",
    }
}
