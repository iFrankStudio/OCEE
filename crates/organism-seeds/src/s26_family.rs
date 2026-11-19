//! S26B: one frozen, metered ACTIVE VM graph recognizes disclosed P1/P2
//! framing *hypotheses* from public paired HELLO bytes alone.
//! It does NOT discover unknown grammar, solve B, or transfer a useful tool.
//! Neither Family nor protocol seed is supplied to the organism.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};

fn lit(raw:&[u8])->Op {
    Op::ConstBytes { hex: raw.iter().map(|b|format!("{b:02x}")).collect() }
}
fn p(id:u32,ty:Type)->Param {Param{id,ty}}
fn i(id:u32,op:Op)->Instruction {Instruction{id,op}}
fn b(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn jump(to:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:to.into(),args}
}
fn branch(v:u32,yes:&str,yes_args:Vec<u32>,no:&str,no_args:Vec<u32>)->Terminator{
    Terminator::Branch{condition:v,if_true:yes.into(),true_args:yes_args,
        if_false:no.into(),false_args:no_args}
}
fn yield_to(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn pair()->Vec<Param>{vec![p(100,Type::Bytes),p(101,Type::Bytes)]}
fn args()->Vec<u32>{vec![100,101]}

/// Uses two disclosed byte-format hypotheses, but never a host-provided
/// family label. Saves P1/P2 as an *observation*, not a discovered algorithm.
/// P0 and unrecognized frames return 0 without claiming a classification.
pub fn s26_public_family_router()->Program {
    let by=Type::Bytes;
    let n=Type::U32;
    let blocks=vec![
        b("entry",vec![p(0,by)],vec![
            i(1,lit(b"/public/C/hello")),
        ],yield_to(WorldCall::Read,vec![1],"sample",vec![])),
        b("sample",vec![p(10,by)],vec![
            i(11,lit(b"payload")),
            i(12,Op::JsonFieldBytes(10,11)),
            i(13,lit(b"request")),
            i(14,lit(b"response")),
            i(15,Op::JsonFieldBytes(12,13)),
            i(16,Op::JsonFieldBytes(12,14)),
            i(17,Op::Len(15)),
            i(18,Op::Len(16)),
            i(19,Op::ConstU32(0)),
            i(20,Op::EqU32(17,19)),
        ],branch(20,"failed",vec![],"nonempty_response",vec![15,16,18])),
        b("nonempty_response",vec![p(100,by),p(101,by),p(102,n)],vec![
            i(103,Op::ConstU32(0)),
            i(104,Op::EqU32(102,103)),
        ],branch(104,"failed",vec![],"classify_start",args())),
        // P1 exposes a public escaped-delimiter pair with sentinel 0x7e.
        b("classify_start",pair(),vec![
            i(105,Op::ConstU32(0)),
            i(106,Op::ByteAt(100,105)),
            i(107,Op::ConstU32(0x7e)),
            i(108,Op::EqU32(106,107)),
        ],branch(108,"p1_response_start",args(),"p2_request_size",args())),
        b("p1_response_start",pair(),vec![
            i(110,Op::ConstU32(0)),
            i(111,Op::ByteAt(101,110)),
            i(112,Op::ConstU32(0x7e)),
            i(113,Op::EqU32(111,112)),
        ],branch(113,"p1_request_end",args(),"failed",vec![])),
        b("p1_request_end",pair(),vec![
            i(120,Op::Len(100)),
            i(121,Op::ConstU32(1)),
            i(122,Op::Sub(120,121)),
            i(123,Op::ByteAt(100,122)),
            i(124,Op::ConstU32(0x7e)),
            i(125,Op::EqU32(123,124)),
        ],branch(125,"p1_response_end",args(),"failed",vec![])),
        b("p1_response_end",pair(),vec![
            i(130,Op::Len(101)),
            i(131,Op::ConstU32(1)),
            i(132,Op::Sub(130,131)),
            i(133,Op::ByteAt(101,132)),
            i(134,Op::ConstU32(0x7e)),
            i(135,Op::EqU32(133,134)),
        ],branch(135,"store_p1",vec![],"failed",vec![])),
        // P2 wraps a seed-specific marker and 2-byte length around exactly
        // one generic public TLV tag 0x10. TlvMatchesOrder is a *total*
        // metered predicate, so non-TLV P0 bytes cannot trap the VM.
        b("p2_request_size",pair(),vec![
            i(140,Op::Len(100)),
            i(141,Op::ConstU32(3)),
            i(142,Op::Div(140,141)),
            i(143,Op::ConstU32(0)),
            i(144,Op::EqU32(142,143)),
        ],branch(144,"failed",vec![],"p2_request_check",args())),
        b("p2_request_check",pair(),vec![
            i(150,Op::Len(100)),
            i(151,Op::ConstU32(3)),
            i(152,Op::Sub(150,151)),
            i(153,Op::Slice(100,151,152)),
            i(154,lit(&[0x10])),
            i(155,Op::TlvMatchesOrder(153,154)),
        ],branch(155,"p2_response_size",args(),"failed",vec![])),
        b("p2_response_size",pair(),vec![
            i(160,Op::Len(101)),
            i(161,Op::ConstU32(3)),
            i(162,Op::Div(160,161)),
            i(163,Op::ConstU32(0)),
            i(164,Op::EqU32(162,163)),
        ],branch(164,"failed",vec![],"p2_response_check",args())),
        b("p2_response_check",pair(),vec![
            i(170,Op::Len(101)),
            i(171,Op::ConstU32(3)),
            i(172,Op::Sub(170,171)),
            i(173,Op::Slice(101,171,172)),
            i(174,lit(&[0x10])),
            i(175,Op::TlvMatchesOrder(173,174)),
        ],branch(175,"store_p2",vec![],"failed",vec![])),
        b("store_p1",vec![],vec![
            i(200,lit(b"/self/discoveries/framing-family")),
            i(201,lit(b"P1")),
            i(202,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![200,201,202],"write_reply",vec![])),
        b("store_p2",vec![],vec![
            i(210,lit(b"/self/discoveries/framing-family")),
            i(211,lit(b"P2")),
            i(212,Op::ConstU32(u32::MAX)),
        ],yield_to(WorldCall::Write,vec![210,211,212],"write_reply",vec![])),
        b("write_reply",vec![p(300,by)],vec![
            i(301,lit(b"code")),
            i(302,Op::JsonFieldString(300,301)),
            i(303,lit(b"Ok")),
            i(304,Op::EqBytes(302,303)),
        ],branch(304,"success",vec![],"failed",vec![])),
        b("success",vec![],vec![i(400,Op::ConstU32(1))],
            Terminator::Return{value:400}),
        b("failed",vec![],vec![i(401,Op::ConstU32(0))],
            Terminator::Return{value:401}),
    ];
    let program=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{name:"main".into(),args:vec![by],returns:n,blocks}]};
    program.validate().expect("S26B frozen public-evidence router must typecheck");
    program
}
