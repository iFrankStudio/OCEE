//! S23: an *executable* researcher-supplied generic P2 TLV framing template
//! with two VM-rewriteable instance parameter sites. The VM must learn the
//! actual mapping and opcode from public HELLO + C feedback before patching.
//! The template is NOT an organism-invented algorithm.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type};

fn lit(bytes:&[u8])->Op {
    Op::ConstBytes{hex:bytes.iter().map(|v|format!("{v:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}

/// Input: P2 application body bytes (the request ID 17 is a seeded
/// workload prior). Output: complete P2 request frame. Two slot locations:
/// entry register 1 = [marker,kind,opcode,id,body,check,physical-order(5)];
/// entry register 2 = [discovered operation opcode].
pub fn s23_seeded_p2_frame_template()->Program {
    let b=Type::Bytes;
    let instructions=vec![
        i(1,lit(&[0;11])),
        i(2,lit(&[0])),
        i(3,Op::ConstU32(0)),
        i(4,Op::ConstU32(1)),
        i(5,Op::ConstU32(2)),
        i(6,Op::ConstU32(3)),
        i(7,Op::ConstU32(4)),
        i(8,Op::ConstU32(5)),
        i(9,Op::ConstU32(6)),
        i(10,Op::ByteAt(1,3)),   // marker
        i(11,Op::ByteAt(1,4)),   // kind tag
        i(12,Op::ByteAt(1,5)),   // opcode tag
        i(13,Op::ByteAt(1,6)),   // id tag
        i(14,Op::ByteAt(1,7)),   // body tag
        i(15,Op::ByteAt(1,8)),   // check tag
        i(16,Op::Slice(1,9,8)), // five observed physical order tags
        i(17,lit(&[0])),
        i(18,Op::TlvEncode(11,17)),
        i(19,Op::TlvEncode(12,2)),
        i(20,lit(&[17,0,0,0])),
        i(21,Op::TlvEncode(13,20)),
        i(22,Op::TlvEncode(14,0)),
        i(23,Op::Concat(18,19)),
        i(24,Op::Concat(23,21)),
        i(25,Op::Concat(24,22)),
        i(26,Op::XorFold(25)),
        i(27,Op::PackByte(26)),
        i(28,Op::TlvEncode(15,27)),
        i(29,Op::Concat(25,28)),
        i(30,Op::TlvReorder(29,16)),
        i(31,Op::ConstU32(0x10)),
        i(32,Op::TlvEncode(31,30)),
        i(33,Op::Len(32)),
        i(34,Op::PackU16Be(33)),
        i(35,Op::PackByte(10)),
        i(36,Op::Concat(35,34)),
        i(37,Op::Concat(36,32)),
    ];
    let p=Program {
        format_version:1, entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![b],returns:b,
            blocks:vec![Block{
                id:"entry".into(),params:vec![Param{id:0,ty:b}],
                instructions,terminator:Terminator::Return{value:37}
            }]
        }]
    };
    p.validate().expect("researcher seeded P2 template must validate");
    p
}
