//! S22: one metered ACTIVE P2 VM learns the private TLV tags/order from
//! public paired HELLO, discovers REMOTE_EVAL via actual C BadBody feedback,
//! executes one FOLD32 B job, finds RESULT through validated live replies,
//! and stores the independently verifiable four-byte result.
//!
//! EVERY implementation strategy is researcher-seeded: known five-field TLV
//! hypothesis, XOR, 112 opcode search, BadBody diagnostic selector, fixed
//! FOLD32 workload layout and RESULT response shape. Hidden protocol tags,
//! permutation, opcode values, seed and B output are NOT supplied.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s21_p2_tlv::s21_p2_infer_public_hello_roles;
fn v(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn lit(x:&[u8])->Op{Op::ConstBytes{hex:x.iter().map(|v|format!("{v:02x}")).collect()}}
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
fn role_op()->Vec<Param>{vec![v(600,Type::Bytes),v(601,Type::U32)]}
fn role_op_args()->Vec<u32>{vec![600,601]}
fn route()->Vec<Param>{
    vec![v(600,Type::Bytes),v(601,Type::U32),v(602,Type::U32)]
}
fn route_args()->Vec<u32>{vec![600,601,602]}
fn with_bytes()->Vec<Param>{
    vec![v(600,Type::Bytes),v(601,Type::U32),v(602,Type::U32),v(603,Type::Bytes)]
}
fn byte_args()->Vec<u32>{vec![600,601,602,603]}
fn with_feedback()->Vec<Param>{
    let mut params=route();params.push(v(603,Type::Bytes));params
}
fn make(use_diagnostics:bool,enable_result:bool)->Program {
    let mut program=s21_p2_infer_public_hello_roles();
    let f=&mut program.functions[0];
    let save=f.blocks.iter_mut().find(|b|b.id=="save_roles").unwrap();
    save.terminator=y(WorldCall::Write,vec![1232,1231,1233],
        "s22_start",vec![1231]);
    f.blocks.extend([
        block("s22_start",vec![v(600,Type::Bytes),v(604,Type::Bytes)],vec![
            i(605,Op::ConstU32(0x10)),
        ],j("s22_remote_scan",vec![600,605])),
        block("s22_remote_scan",role_op(),vec![],
            Terminator::RepeatBounded{limit:112,body:"s22_probe".into(),
                body_args:role_op_args(),done:"failed".into(),done_args:vec![]}),
        block("s22_probe",role_op(),vec![
            i(606,Op::ConstU32(0)),
            i(607,lit(b"")),
        ],j("s22_construct",vec![600,601,606,607])),
        // Common P2 encoder. Its tags, original physical field order and
        // marker all originate from the current VM's 11-byte observation.
        block("s22_construct",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(602,Type::U32),
            v(603,Type::Bytes)
        ],vec![
            i(610,Op::ConstU32(0)),
            i(611,Op::ConstU32(1)),
            i(612,Op::ConstU32(2)),
            i(613,Op::ConstU32(3)),
            i(614,Op::ConstU32(4)),
            i(615,Op::ConstU32(5)),
            i(616,Op::ConstU32(6)),
            i(617,Op::ByteAt(600,610)), // public marker
            i(618,Op::ByteAt(600,611)), // role: kind tag
            i(619,Op::ByteAt(600,612)), // role: opcode tag
            i(620,Op::ByteAt(600,613)), // role: ID tag
            i(621,Op::ByteAt(600,614)), // role: body tag
            i(622,Op::ByteAt(600,615)), // role: checksum tag
            i(623,Op::Slice(600,616,615)), // observed physical order
            i(624,lit(&[0])),
            i(625,Op::TlvEncode(618,624)),
            i(626,Op::PackByte(601)),
            i(627,Op::TlvEncode(619,626)),
            i(628,lit(&[17,0,0,0])),
            i(629,Op::TlvEncode(620,628)),
            i(630,Op::TlvEncode(621,603)),
            i(631,Op::Concat(625,627)),
            i(632,Op::Concat(631,629)),
            i(633,Op::Concat(632,630)),
            i(634,Op::XorFold(633)),
            i(635,Op::PackByte(634)),
            i(636,Op::TlvEncode(622,635)),
            i(637,Op::Concat(633,636)),
            i(638,Op::TlvReorder(637,623)),
            i(639,Op::ConstU32(0x10)),
            i(640,Op::TlvEncode(639,638)),
            i(641,Op::Len(640)),
            i(642,Op::PackU16Be(641)),
            i(643,Op::PackByte(617)),
            i(644,Op::Concat(643,642)),
            i(645,Op::Concat(644,640)),
        ],y(WorldCall::Send,vec![645],"s22_wait1",route_args())),
        block("s22_wait1",with_feedback(),vec![],
            y(WorldCall::Observe,vec![],"s22_wait2",route_args())),
        block("s22_wait2",with_feedback(),vec![],
            y(WorldCall::Observe,vec![],"s22_wait3",route_args())),
        block("s22_wait3",with_feedback(),vec![],
            y(WorldCall::Observe,vec![],"s22_receive",route_args())),
        block("s22_receive",with_feedback(),vec![],
            y(WorldCall::Receive,vec![],"s22_feedback",route_args())),
        block("s22_feedback",with_feedback(),vec![
            i(650,lit(b"payload")),
            i(651,Op::JsonFieldBytes(603,650)),
            i(652,Op::ConstU32(0)),
            i(653,Op::JsonIndex(651,652)),
            i(654,lit(b"bytes")),
            i(655,Op::JsonFieldBytes(653,654)),
            i(656,Op::EqU32(602,652)), // remote opcode search?
        ],br(656,"s22_probe_classify",vec![600,601,655],
            "s22_response_classify",vec![600,601,602,655])),
        block("s22_probe_classify",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)
        ],vec![
            i(657,Op::ConstU32(0)),
            i(658,Op::ByteAt(603,657)),
            i(659,Op::ConstU32(0x7b)),
            i(660,Op::EqU32(658,659)),
        ],br(660,"s22_probe_diag",vec![600,601,603],
            "s22_remote_increment",vec![600,601])),
        block("s22_probe_diag",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)
        ],vec![
            i(661,lit(b"code")),
            i(662,Op::JsonFieldString(603,661)),
            i(663,lit(if use_diagnostics{b"BadBody"}else{b"NoSuchCode"})),
            i(664,Op::EqBytes(662,663)),
        ],br(664,"s22_remote_found",vec![600,601],
            "s22_remote_increment",vec![600,601])),
        block("s22_remote_increment",role_op(),vec![
            i(665,Op::ConstU32(1)),
            i(666,Op::Add(601,665)),
        ],j("s22_remote_scan",vec![600,666])),
        block("s22_remote_found",role_op(),vec![
            i(667,Op::PackByte(601)),
            i(668,lit(b"/self/discoveries/p2-remote-opcode")),
            i(669,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![668,667,669],
            "s22_read_job",role_op_args())),
        block("s22_read_job",vec![v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)],vec![
            i(670,lit(b"/public/E001/job0")),
        ],y(WorldCall::Read,vec![670],"s22_job_send",role_op_args())),
        block("s22_job_send",vec![v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)],vec![
            i(671,lit(b"payload")),
            i(672,Op::JsonFieldBytes(603,671)),
            i(673,lit(b"values_le")),
            i(674,Op::JsonFieldBytes(672,673)),
            i(675,lit(&[3,0])), // explicitly seeded 3*u32 workload
            i(676,Op::Concat(675,674)),
            i(677,Op::ConstU32(1)), // ACK phase
        ],j("s22_job_gate",vec![600,601,677,676])),
        // A one-shot bounded stage transition is required by the typed
        // graph verifier: otherwise reusing the same frame-builder for
        // opcode probing AND the later job forms an unbounded static
        // backedge via feedback -> remote_found -> job_send -> builder.
        // It is not a search shortcut; VM still builds every frame.
        block("s22_job_gate",vec![
            v(600,Type::Bytes),v(601,Type::U32),
            v(602,Type::U32),v(603,Type::Bytes)
        ],vec![],Terminator::RepeatBounded{
            limit:1,body:"s22_construct".into(),body_args:byte_args(),
            done:"failed".into(),done_args:vec![],
        }),
        block("s22_response_classify",with_bytes(),vec![
            i(680,Op::ConstU32(0)),
            i(681,Op::ByteAt(603,680)),
            i(682,Op::ByteAt(600,680)),
            i(683,Op::EqU32(681,682)),
        ],br(683,"s22_envelope",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_bad_response",route(),vec![
            i(684,Op::ConstU32(1)),
            i(685,Op::EqU32(602,684)),
        ],br(685,"failed",vec![],
            "s22_result_increment",role_op_args())),
        block("s22_envelope",with_bytes(),vec![
            i(686,Op::ConstU32(3)),
            i(687,Op::Len(603)),
            i(688,Op::Sub(687,686)),
            i(689,Op::PackU16Be(688)),
            i(690,Op::ConstU32(1)),
            i(691,Op::ConstU32(2)),
            i(692,Op::Slice(603,690,691)),
            i(693,Op::EqBytes(689,692)),
        ],br(693,"s22_outer",byte_args(),"s22_bad_response",route_args())),
        block("s22_outer",with_bytes(),vec![
            i(694,Op::ConstU32(3)),
            i(695,Op::Len(603)),
            i(696,Op::Sub(695,694)),
            i(697,Op::Slice(603,694,696)),
            i(698,Op::TlvDecode(697)),
            i(699,Op::ConstU32(0)),
            i(700,Op::JsonIndex(698,699)),
            i(701,lit(b"value")),
            i(702,Op::JsonFieldBytes(700,701)),
            i(703,Op::ConstU32(0x10)),
            i(704,Op::TlvEncode(703,702)),
            i(705,Op::EqBytes(704,697)), // rejects trailing outer TLV
        ],br(705,"s22_inner",vec![600,601,602,702],
            "s22_bad_response",route_args())),
        block("s22_inner",with_bytes(),vec![
            i(706,Op::ConstU32(6)),
            i(707,Op::ConstU32(5)),
            i(708,Op::Slice(600,706,707)),
            // No untrusted TLV reorder before shape validation. The total
            // predicate returns a metered Bool(false) on unexpected tags,
            // duplicate tags, bad lengths or wrong physical order, so the
            // active organism can reject this C reply and keep scanning.
            i(709,Op::TlvMatchesOrder(603,708)),
        ],br(709,"s22_check",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_check",with_bytes(),vec![
            i(711,Op::XorFold(603)),
            i(712,Op::ConstU32(5)),
            i(713,Op::ByteAt(600,712)),
            i(714,Op::ConstU32(1)),
            i(715,Op::Xor(713,714)),
            i(716,Op::EqU32(711,715)),
        ],br(716,"s22_kind",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_kind",with_bytes(),vec![
            i(717,Op::ConstU32(1)),
            i(718,Op::ByteAt(600,717)),
            i(719,Op::TlvLookup(603,718)),
            i(720,lit(&[1])),
            i(721,Op::EqBytes(719,720)),
        ],br(721,"s22_opcode",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_opcode",with_bytes(),vec![
            i(722,Op::ConstU32(2)),
            i(723,Op::ByteAt(600,722)),
            i(724,Op::TlvLookup(603,723)),
            i(725,Op::PackByte(601)),
            i(726,Op::EqBytes(724,725)),
        ],br(726,"s22_id",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_id",with_bytes(),vec![
            i(727,Op::ConstU32(3)),
            i(728,Op::ByteAt(600,727)),
            i(729,Op::TlvLookup(603,728)),
            i(730,lit(&[17,0,0,0])),
            i(731,Op::EqBytes(729,730)),
        ],br(731,"s22_body",byte_args(),
            "s22_bad_response",route_args())),
        block("s22_body",with_bytes(),vec![
            i(732,Op::ConstU32(4)),
            i(733,Op::ByteAt(600,732)),
            i(734,Op::TlvLookup(603,733)),
            i(735,Op::ConstU32(1)),
            i(736,Op::EqU32(602,735)),
        ],br(736,"s22_ack",vec![600,601,734],
            "s22_result",vec![600,601,734])),
        block("s22_ack",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)
        ],vec![
            i(737,lit(&[0])),
            i(738,Op::EqBytes(603,737)),
        ],br(738,"s22_ack_record",vec![600,601,603],
            "failed",vec![])),
        block("s22_ack_record",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)
        ],vec![
            i(739,lit(b"/self/results/p2-remote-eval-ack")),
            i(740,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![739,603,740],
            "s22_result_start",vec![600])),
        block("s22_result_start",vec![v(600,Type::Bytes),v(603,Type::Bytes)],vec![
            i(741,Op::ConstU32(0x10)),
        ],j("s22_result_scan",vec![600,741])),
        block("s22_result_scan",role_op(),vec![],
            Terminator::RepeatBounded{limit:112,body:"s22_result_probe".into(),
                body_args:role_op_args(),done:"failed".into(),done_args:vec![]}),
        block("s22_result_probe",role_op(),vec![
            i(742,Op::ConstU32(2)),
            i(743,lit(b"")),
        ],j("s22_construct",vec![600,601,742,743])),
        block("s22_result_increment",role_op(),vec![
            i(744,Op::ConstU32(1)),
            i(745,Op::Add(601,744)),
        ],j("s22_result_scan",vec![600,745])),
        block("s22_result",vec![
            v(600,Type::Bytes),v(601,Type::U32),v(603,Type::Bytes)
        ],vec![
            i(746,Op::Len(603)),
            i(747,Op::ConstU32(if enable_result{5}else{6})),
            i(748,Op::EqU32(746,747)),
        ],br(748,"s22_result_status",vec![601,603],
            "s22_result_increment",vec![600,601])),
        block("s22_result_status",vec![v(601,Type::U32),v(603,Type::Bytes)],vec![
            i(749,Op::ConstU32(0)),
            i(750,Op::ByteAt(603,749)),
            i(751,Op::EqU32(750,749)),
        ],br(751,"s22_result_found",vec![601,603],
            "failed",vec![])),
        block("s22_result_found",vec![v(601,Type::U32),v(603,Type::Bytes)],vec![
            i(752,Op::PackByte(601)),
            i(753,lit(b"/self/discoveries/p2-result-opcode")),
            i(754,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![753,752,754],"s22_result_store",vec![603])),
        block("s22_result_store",vec![v(603,Type::Bytes),v(604,Type::Bytes)],vec![
            i(755,Op::ConstU32(1)),
            i(756,Op::ConstU32(4)),
            i(757,Op::Slice(603,755,756)),
            i(758,lit(b"/self/results/p2-job0/value_le")),
            i(759,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![758,757,759],"success",vec![])),
    ]);
    program.validate().expect("S22 active P2 full-cycle VM must verify");
    program
}

/// S24B: researcher-proposed bounded recovery policy for an invalid C ACK.
/// Do NOT infer B execution from a malformed ACK. Instead, query RESULT via
/// the same active VM and accept ONLY a fully validated positive result.
/// If C never executed the task, the bounded result search ends in failure.
/// A test injects malformed TLV only after the real C service built its ACK.
pub fn s24_p2_retry_result_after_bad_ack()->Program {
    let mut program=s22_p2_discover_and_execute();
    let f=&mut program.functions[0];
    let invalid=f.blocks.iter_mut().find(|b|b.id=="s22_bad_response").unwrap();
    invalid.terminator=br(685,"s24_ack_recovery",vec![600],
        "s22_result_increment",vec![600,601]);
    f.blocks.push(block("s24_ack_recovery",vec![v(600,Type::Bytes)],vec![],
        // s22_result_start is a WorldCall::Write continuation; supply a
        // real paid Observe feedback rather than forging that extra Bytes.
        y(WorldCall::Observe,vec![],"s22_result_start",vec![600])));
    program.validate().expect("S24 bounded malformed-ACK result recovery must typecheck");
    program
}
pub fn s24_recovery_manifest()->crate::SeedManifest{
    let program=s24_p2_retry_result_after_bad_ack();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S24B-P2-researcher-seeded-bounded-ACK-result-recovery",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S22-public-HELLO-P2-role-learning",
            "S22-researcher-seeded-112-REMOTE_EVAL-and-RESULT-scans",
            "generic-TlvMatchesOrder-returning-ordinary-bool",
            "malformed-ACK-branches-to-metered-bounded-RESULT-discovery",
            "RESULT-only-accepted-on-full-frame-and-ID-validation",
            "no-private-B-ledger-or-oracle-input",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["researcher-seeded-P2-five-TLV-XOR".into()],
        prohibited_knowledge:vec![
            "private-P2-tags","private-P2-field-permutation",
            "hidden-REMOTE_EVAL-opcode","hidden-RESULT-opcode",
            "protocol-seed","B-privileged-work-status","B-gas","FOLD32-answer",
        ],
        research_status:"S24B_SEEDED_RECOVERY_POLICY_NOT_SELF_INVENTED_NOT_E001",
    }
}

pub fn s22_p2_discover_and_execute()->Program{make(true,true)}
pub fn s22_p2_no_badbody_selection()->Program{make(false,true)}
pub fn s22_p2_no_result_shape()->Program{make(true,false)}
pub fn s22_manifest()->crate::SeedManifest{
    let program=s22_p2_discover_and_execute();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S22-active-P2-public-TLV-hidden-opcode-B-FOLD32",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S21A-seeded-five-TLV-public-HELLO-role-inference",
            "generic-TlvEncode-TlvReorder-TlvLookup-XorFold",
            "bounded-112-REMOTE_EVAL-opcode-scan",
            "researcher-seeded-BadBody-remote-opcode-selector",
            "fixed-three-u32-FOLD32-task-shape",
            "P2-response-marker-envelope-physical-order-and-xor-check",
            "live-ACK-status-direction-opcode-request-id-validation",
            "bounded-112-RESULT-opcode-scan-and-u32-extraction",
            "all-A-VM-instructions-and-actions-metered",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["seeded-P2-single-outer-five-inner-TLV-XOR".into()],
        prohibited_knowledge:vec![
            "hidden-P2-role-tags","hidden-P2-permutation",
            "hidden-REMOTE_EVAL-opcode","hidden-RESULT-opcode",
            "generator-seed","B-resource-ledger","held-out-FOLD32-answer",
        ],
        research_status:"S22_DEV_VM_P2_FULL_B_CYCLE_KNOWN_TLV_HYPOTHESIS_NOT_E001",
    }
}
