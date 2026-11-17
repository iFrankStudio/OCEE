//! S1 development probe for P0's *known* XOR-framed message family.
//!
//! Seeded: generic frame-byte mutation, XOR check-delta repair, bounded
//! candidate enumeration, error-code interpretation, and persistence.
//! NOT seeded: hidden service opcode values, protocol seed, private state.
//! This is a calibrated hypothesis test of feedback selection, not open-ended
//! learning, new-program creation, or a complete E001 S organism.
use ocee_organism_vm::{
    Block, Function, Instruction, Op, Param, Program, Terminator, Type, WorldCall,
};

fn bytes(b:&[u8])->Op {
    Op::ConstBytes{hex:b.iter().map(|v|format!("{v:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn arg(id:u32,ty:Type)->Param {Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block {
    Block{id:id.to_owned(),params,instructions,terminator}
}
fn read_action()->Terminator {
    Terminator::Yield{call:WorldCall::Read,args:vec![1],
        next:"parse_public".into(),next_args:vec![]}
}
/// A bounded VM program searching unknown opcode values by observing error
/// feedback on minimally modified legal HELLO frames. It does not solve P1/P2.
pub fn s1_p0_probe()->Program {
    let b=Type::Bytes;
    let u=Type::U32;
    let forward=|target:&str,args|Terminator::Jump{target:target.into(),args};
    Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![b],returns:u,blocks:vec![
                block("entry",vec![arg(0,b)],vec![ins(1,bytes(b"/public/C/hello"))],
                    read_action()),
                block("parse_public",vec![arg(10,b)],vec![
                    ins(11,bytes(b"payload")),
                    ins(12,Op::JsonFieldBytes(10,11)),
                    ins(13,bytes(b"request")),
                    ins(14,Op::JsonFieldBytes(12,13)),
                    ins(15,Op::ConstU32(0x10)),
                ],forward("scan",vec![14,15])),
                block("scan",vec![arg(20,b),arg(21,u)],vec![],
                    Terminator::RepeatBounded{limit:112,
                        body:"modify".into(),body_args:vec![20,21],
                        done:"not_found".into(),done_args:vec![20,21]}),
                block("modify",vec![arg(30,b),arg(31,u)],vec![
                    ins(32,Op::ConstU32(2)),
                    ins(33,Op::ByteAt(30,32)), // original HELLO opcode
                    ins(34,Op::Len(30)),
                    ins(35,Op::ConstU32(1)),
                    ins(36,Op::Sub(34,35)), // final XOR checksum position
                    ins(37,Op::ByteAt(30,36)),
                    ins(38,Op::Xor(37,33)),
                    ins(39,Op::Xor(38,31)), // new XOR checksum
                    ins(40,Op::ReplaceByte(30,32,31)),
                    ins(41,Op::ReplaceByte(40,36,39)),
                ],Terminator::Yield{call:WorldCall::Send,args:vec![41],
                    next:"delay1".into(),next_args:vec![30,31,41]}),
                block("delay1",vec![arg(50,b),arg(51,u),arg(52,b),arg(53,b)],vec![],
                    Terminator::Yield{call:WorldCall::Observe,args:vec![],
                        next:"delay2".into(),next_args:vec![50,51,52]}),
                block("delay2",vec![arg(60,b),arg(61,u),arg(62,b),arg(63,b)],vec![],
                    Terminator::Yield{call:WorldCall::Observe,args:vec![],
                        next:"delay3".into(),next_args:vec![60,61,62]}),
                block("delay3",vec![arg(70,b),arg(71,u),arg(72,b),arg(73,b)],vec![],
                    Terminator::Yield{call:WorldCall::Observe,args:vec![],
                        next:"receive".into(),next_args:vec![70,71,72]}),
                block("receive",vec![arg(80,b),arg(81,u),arg(82,b),arg(83,b)],vec![],
                    Terminator::Yield{call:WorldCall::Receive,args:vec![],
                        next:"decode".into(),next_args:vec![80,81,82]}),
                block("decode",vec![arg(90,b),arg(91,u),arg(92,b),arg(93,b)],vec![
                    ins(94,bytes(b"payload")),
                    ins(95,Op::JsonFieldBytes(93,94)),
                    ins(96,Op::ConstU32(0)),
                    ins(97,Op::JsonIndex(95,96)),
                    ins(98,bytes(b"bytes")),
                    ins(99,Op::JsonFieldBytes(97,98)),
                    ins(100,Op::ByteAt(99,96)),
                    ins(101,Op::ConstU32(0x7b)), // '{' diagnostics, not frames
                    ins(102,Op::EqU32(100,101)),
                ],Terminator::Branch{condition:102,
                    if_true:"diagnostic".into(),true_args:vec![90,91,92,99],
                    if_false:"increment".into(),false_args:vec![90,91]}),
                block("diagnostic",vec![arg(110,b),arg(111,u),arg(112,b),arg(113,b)],vec![
                    ins(114,bytes(b"code")),
                    ins(115,Op::JsonFieldString(113,114)),
                    ins(116,bytes(b"BadBody")),
                    ins(117,Op::EqBytes(115,116)),
                ],Terminator::Branch{condition:117,
                    if_true:"found".into(),true_args:vec![112],
                    if_false:"increment".into(),false_args:vec![110,111]}),
                block("increment",vec![arg(120,b),arg(121,u)],vec![
                    ins(122,Op::ConstU32(1)),
                    ins(123,Op::Add(121,122)),
                ],forward("scan",vec![120,123])),
                block("found",vec![arg(130,b)],vec![
                    ins(131,bytes(b"/self/discoveries/p0-operation-candidate")),
                    ins(132,Op::ConstU32(u32::MAX)),
                ],Terminator::Yield{call:WorldCall::Write,args:vec![131,130,132],
                    next:"success".into(),next_args:vec![]}),
                block("success",vec![arg(140,b)],vec![
                    ins(141,Op::ConstU32(1)),
                ],Terminator::Return{value:141}),
                block("not_found",vec![arg(150,b),arg(151,u)],vec![
                    ins(152,Op::ConstU32(0)),
                ],Terminator::Return{value:152}),
            ],
        }]
    }
}


/// Mechanism ablation: retain frame generation, probing, message delays and
/// memory capacity while disabling the use of observed diagnostic labels.
/// This is a development control, not an A/B/C/D E001 confirmatory group.
pub fn s1_p0_no_feedback_control()->Program {
    let mut program=s1_p0_probe();
    let diagnostic=program.functions[0].blocks.iter_mut()
        .find(|b|b.id=="diagnostic").expect("diagnostic block");
    let instruction=diagnostic.instructions.iter_mut()
        .find(|i|i.id==115).expect("feedback selector");
    instruction.op=bytes(b"UnknownOperation");
    program.validate().expect("control must typecheck");
    program
}

pub fn s1_manifest()->crate::SeedManifest {
    crate::SeedManifest{
        manifest_version:1,kind:"S1-P0-development-probe",
        program_hash:format!("sha256:{}",s1_p0_probe().content_hash()
            .expect("probe program must validate")),
        built_in_modules:vec![
            "public-hello-read", "generic-json-feedback",
            "KNOWN-P0-XOR-byte-delta-repair", "bounded-opcode-enumeration",
            "BAD_BODY-feedback-selector", "fixed-latency-poll",
            "save-discovered-frame"
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["P0-XOR-frame-envelope".into()],
        prohibited_knowledge:vec![
            "hidden-opcode-values","private-generator-seed",
            "remote-eval-success-reward","evaluator-score"
        ],
        research_status:"DEVELOPMENT_ABLATION_NOT_E001_FULL_NOT_NEW_TOOL",
    }
}

    
/// This specializes an *explicitly seeded* general P0 frame algorithm using
/// an opcode actually discovered through world interaction. It is NOT invention
/// of the algorithm. No researcher switches its active program mid-run.
pub fn s1_p0_tool_builder()->Program {
    let mut program=s1_p0_probe();
    let f=&mut program.functions[0];
    let found=f.blocks.iter_mut().find(|b|b.id=="found")
        .expect("S1 discovery block");
    found.terminator=Terminator::Yield{
        call:WorldCall::Write,args:vec![131,130,132],
        next:"tool_prepare".into(),next_args:vec![130],
    };
    let b=Type::Bytes;
    let u=Type::U32;
    f.blocks.extend([
        block("tool_prepare",vec![arg(500,b),arg(501,b)],vec![
            ins(502,Op::ConstU32(0)),
            ins(503,Op::ByteAt(500,502)),
            ins(504,Op::PackByte(503)),
            ins(505,bytes(&[0])),
            ins(506,Op::ConstU32(2)),
            ins(507,Op::ByteAt(500,506)),
            ins(508,Op::PackByte(507)),
            ins(509,Op::Concat(504,505)),
            ins(510,Op::Concat(509,508)),
            ins(511,Op::HexEncode(510)),
            ins(512,Op::ConstBytes{hex:"7b22656e7472795f66756e6374696f6e223a226d61696e222c22666f726d61745f76657273696f6e223a312c2266756e6374696f6e73223a5b7b2261726773223a5b224279746573225d2c22626c6f636b73223a5b7b226964223a22656e747279222c22696e737472756374696f6e73223a5b7b226964223a312c226f70223a7b22436f6e73744279746573223a7b22686578223a22".into()}),
            ins(513,Op::Concat(512,511)),
            ins(514,Op::ConstBytes{hex:"227d7d7d2c7b226964223a322c226f70223a7b224c656e223a307d7d2c7b226964223a332c226f70223a7b225061636b5531364c65223a327d7d2c7b226964223a342c226f70223a7b22436f6e636174223a5b312c335d7d7d2c7b226964223a352c226f70223a7b22436f6e636174223a5b342c305d7d7d2c7b226964223a362c226f70223a7b22586f72466f6c64223a357d7d2c7b226964223a372c226f70223a7b225061636b42797465223a367d7d2c7b226964223a382c226f70223a7b22436f6e636174223a5b352c375d7d7d5d2c22706172616d73223a5b7b226964223a302c227479223a224279746573227d5d2c227465726d696e61746f72223a7b2252657475726e223a7b2276616c7565223a387d7d7d5d2c226e616d65223a226d61696e222c2272657475726e73223a224279746573227d5d7d".into()}),
            ins(515,Op::Concat(513,514)),
            ins(516,bytes(b"OCEE-PROGRAM-V1\\0")),
            ins(517,Op::Concat(516,515)),
            ins(518,Op::Sha256(517)),
            ins(519,Op::HexEncode(518)),
            ins(520,bytes(b"/self/versions/")),
            ins(521,Op::Concat(520,519)),
            ins(522,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![521,515,522],next:"tool_test".into(),next_args:vec![519]}),
        block("tool_test",vec![arg(530,b),arg(531,b)],vec![
            ins(532,bytes(&[0,0,0,0,1,0,9,0,0,0])),
        ],Terminator::Yield{call:WorldCall::Exec,
            args:vec![530,532],next:"tool_verify".into(),next_args:vec![530]}),
        block("tool_verify",vec![arg(540,b),arg(541,b)],vec![
            ins(542,bytes(b"code")),
            ins(543,Op::JsonFieldString(541,542)),
            ins(544,bytes(b"Ok")),
            ins(545,Op::EqBytes(543,544)),
        ],Terminator::Branch{condition:545,
            if_true:"tool_register".into(),true_args:vec![540],
            if_false:"tool_failed".into(),false_args:vec![]}),
        block("tool_register",vec![arg(550,b)],vec![
            ins(551,bytes(b"/self/tools/p0-eval-frame")),
            ins(552,Op::ConstU32(u32::MAX)),
        ],Terminator::Yield{call:WorldCall::Write,
            args:vec![551,550,552],next:"tool_done".into(),next_args:vec![]}),
        block("tool_done",vec![arg(560,b)],vec![
            ins(561,Op::ConstU32(1)),
        ],Terminator::Return{value:561}),
        block("tool_failed",vec![],vec![
            ins(570,Op::ConstU32(0)),
        ],Terminator::Return{value:570}),
    ]);
    program.validate().expect("S1 tool specialist must be type-safe");
    program
}

pub fn s1_p0_tool_builder_no_feedback()->Program {
    let mut program=s1_p0_tool_builder();
    let diagnostic=program.functions[0].blocks.iter_mut()
        .find(|b|b.id=="diagnostic").expect("selector block");
    diagnostic.instructions.iter_mut().find(|i|i.id==115)
        .expect("diagnostic field").op=bytes(b"UnknownOperation");
    program.validate().expect("ablation program must validate");
    program
}
