//! S2-P0 development fixture: actual B invocation from an S1-discovered opcode.
//! This is a researcher-provided *second initial program*, NOT a self-created
//! tool. It reads the discovery through the world and builds a P0 FOLD32 request
//! using general byte operations. Known P0 format, result-frame layout, and fixed count=3 are priors.
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};

fn byte_const(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|v|format!("{v:02x}")).collect()}
}
fn instr(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(name:&str,params:Vec<Param>,ops:Vec<Instruction>,end:Terminator)->Block {
    Block{id:name.into(),params,instructions:ops,terminator:end}
}
fn jump(next:&str,args:Vec<u32>)->Terminator {
    Terminator::Jump{target:next.into(),args}
}
pub fn s2_p0_remote_eval()->Program {
    let b=Type::Bytes;
    let n=Type::U32;
    Program{format_version:1,entry_function:"main".into(),functions:vec![
        Function{name:"main".into(),args:vec![b],returns:n,blocks:vec![
            block("entry",vec![param(0,b)],vec![
                instr(1,byte_const(b"/self/discoveries/p0-operation-candidate")),
            ],Terminator::Yield{call:WorldCall::Read,args:vec![1],
                next:"parse_discovery".into(),next_args:vec![]}),
            block("parse_discovery",vec![param(10,b)],vec![
                instr(11,byte_const(b"payload")),
                instr(12,Op::JsonFieldBytes(10,11)),
                instr(13,byte_const(b"/public/E001/job0")),
            ],Terminator::Yield{call:WorldCall::Read,args:vec![13],
                next:"job".into(),next_args:vec![12]}),
            block("job",vec![param(20,b),param(21,b)],vec![
                instr(22,byte_const(b"payload")),
                instr(23,Op::JsonFieldBytes(21,22)),
                instr(24,byte_const(b"values_le")),
                instr(25,Op::JsonFieldBytes(23,24)),
                // Initial S2 knows that this development task has exactly 3 u32 values.
                instr(26,byte_const(&[3,0])),
                instr(27,Op::Concat(26,25)), // FOLD32 body, count:u16le + values
                instr(28,Op::ConstU32(0)),
                instr(29,Op::ConstU32(3)),
                instr(30,Op::Slice(20,28,29)), // marker|kind|discovered opcode
                instr(31,byte_const(&[18,0])), // 4 request id bytes + 14 body
                instr(32,Op::Concat(30,31)),
                instr(33,Op::ConstU32(5)),
                instr(34,Op::ConstU32(4)),
                instr(35,Op::Slice(20,33,34)), // reuse observed request id
                instr(36,Op::Concat(32,35)),
                instr(37,Op::Concat(36,27)),
                instr(38,Op::XorFold(37)),
                instr(39,Op::PackByte(38)),
                instr(40,Op::Concat(37,39)),
            ],Terminator::Yield{call:WorldCall::Send,args:vec![40],
                next:"delay1".into(),next_args:vec![20,40]}),
            block("delay1",vec![param(50,b),param(51,b),param(52,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"delay2".into(),next_args:vec![50,51]}),
            block("delay2",vec![param(60,b),param(61,b),param(62,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"delay3".into(),next_args:vec![60,61]}),
            block("delay3",vec![param(70,b),param(71,b),param(72,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"receive".into(),next_args:vec![70,71]}),
            block("receive",vec![param(80,b),param(81,b),param(82,b)],vec![],
                Terminator::Yield{call:WorldCall::Receive,args:vec![],
                    next:"check".into(),next_args:vec![80,81]}),
            block("check",vec![param(90,b),param(91,b),param(92,b)],vec![
                instr(93,byte_const(b"payload")),
                instr(94,Op::JsonFieldBytes(92,93)),
                instr(95,Op::ConstU32(0)),
                instr(96,Op::JsonIndex(94,95)),
                instr(97,byte_const(b"bytes")),
                instr(98,Op::JsonFieldBytes(96,97)),
                instr(99,Op::ConstU32(9)),
                instr(100,Op::ByteAt(98,99)), // P0 ACK status
                instr(101,Op::EqU32(100,95)),
            ],Terminator::Branch{condition:101,
                if_true:"save_ack".into(),true_args:vec![91,98],
                if_false:"fail".into(),false_args:vec![]}),
            block("save_ack",vec![param(110,b),param(111,b)],vec![
                instr(112,byte_const(b"/self/results/job0/ack")),
                instr(113,Op::ConstU32(u32::MAX)),
            ],Terminator::Yield{call:WorldCall::Write,args:vec![112,111,113],
                next:"result_init".into(),next_args:vec![110]}),
            // The result opcode is also hidden. Reuse the S1-discovered empty
            // request frame as a valid P0 frame skeleton and probe 0x10..0x7f.
            block("result_init",vec![param(200,b),param(201,b)],vec![
                instr(202,Op::ConstU32(0x10)),
            ],jump("result_scan",vec![200,202])),
            block("result_scan",vec![param(210,b),param(211,n)],vec![],
                Terminator::RepeatBounded{limit:112,
                    body:"result_build".into(),body_args:vec![210,211],
                    done:"fail".into(),done_args:vec![]}),
            block("result_build",vec![param(220,b),param(221,n)],vec![
                instr(222,Op::ConstU32(2)),
                instr(223,Op::ByteAt(220,222)), // S1-discovered REMOTE_EVAL opcode
                instr(224,Op::Len(220)),
                instr(225,Op::ConstU32(1)),
                instr(226,Op::Sub(224,225)), // old checksum position
                instr(227,Op::ByteAt(220,226)),
                instr(228,Op::Xor(227,223)),
                instr(229,Op::Xor(228,221)), // XOR correction
                instr(230,Op::ReplaceByte(220,222,221)),
                instr(231,Op::ReplaceByte(230,226,229)),
            ],Terminator::Yield{call:WorldCall::Send,args:vec![231],
                next:"result_delay1".into(),next_args:vec![220,221]}),
            block("result_delay1",vec![param(240,b),param(241,n),param(242,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"result_delay2".into(),next_args:vec![240,241]}),
            block("result_delay2",vec![param(250,b),param(251,n),param(252,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"result_delay3".into(),next_args:vec![250,251]}),
            block("result_delay3",vec![param(260,b),param(261,n),param(262,b)],vec![],
                Terminator::Yield{call:WorldCall::Observe,args:vec![],
                    next:"result_receive".into(),next_args:vec![260,261]}),
            block("result_receive",vec![param(270,b),param(271,n),param(272,b)],vec![],
                Terminator::Yield{call:WorldCall::Receive,args:vec![],
                    next:"result_decode".into(),next_args:vec![270,271]}),
            block("result_decode",vec![param(280,b),param(281,n),param(282,b)],vec![
                instr(283,byte_const(b"payload")),
                instr(284,Op::JsonFieldBytes(282,283)),
                instr(285,Op::ConstU32(0)),
                instr(286,Op::JsonIndex(284,285)),
                instr(287,byte_const(b"bytes")),
                instr(288,Op::JsonFieldBytes(286,287)),
                instr(289,Op::Len(288)),
                instr(290,Op::ConstU32(15)), // P0 response with status + u32 value
                instr(291,Op::EqU32(289,290)),
            ],Terminator::Branch{condition:291,
                if_true:"result_verify".into(),true_args:vec![280,281,288],
                if_false:"result_increment".into(),false_args:vec![280,281]}),
            block("result_verify",vec![param(300,b),param(301,n),param(302,b)],vec![
                instr(303,Op::XorFold(302)), // valid P0 checksum -> 0
                instr(304,Op::ConstU32(0)),
                instr(305,Op::EqU32(303,304)),
            ],Terminator::Branch{condition:305,
                if_true:"result_verify_status".into(),true_args:vec![300,301,302],
                if_false:"result_increment".into(),false_args:vec![300,301]}),
            block("result_verify_status",vec![param(310,b),param(311,n),param(312,b)],vec![
                instr(313,Op::ConstU32(9)),
                instr(314,Op::ByteAt(312,313)), // status must be 0
                instr(315,Op::ConstU32(0)),
                instr(316,Op::EqU32(314,315)),
            ],Terminator::Branch{condition:316,
                if_true:"result_save".into(),true_args:vec![312],
                if_false:"result_increment".into(),false_args:vec![310,311]}),
            block("result_increment",vec![param(320,b),param(321,n)],vec![
                instr(322,Op::ConstU32(1)),
                instr(323,Op::Add(321,322)),
            ],jump("result_scan",vec![320,323])),
            block("result_save",vec![param(330,b)],vec![
                instr(331,Op::ConstU32(10)), // payload after 4-byte request id + status
                instr(332,Op::ConstU32(4)),
                instr(333,Op::Slice(330,331,332)),
                instr(334,byte_const(b"/self/results/job0/value_le")),
                instr(335,Op::ConstU32(u32::MAX)),
            ],Terminator::Yield{call:WorldCall::Write,args:vec![334,333,335],
                next:"success".into(),next_args:vec![]}),
            block("success",vec![param(120,b)],vec![
                instr(121,Op::ConstU32(1)),
            ],Terminator::Return{value:121}),
            block("fail",vec![],vec![
                instr(130,Op::ConstU32(0)),
            ],Terminator::Return{value:130}),
        ]}
    ]}
}
