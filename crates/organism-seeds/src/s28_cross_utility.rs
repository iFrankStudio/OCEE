//! S28: generic payload-size utility is VM-specialized and carried from a
//! real P1 episode into P2. It is *not* grammar invention: the one-slot
//! Len(Bytes) template, rewrite location and acceptance gate are researcher
//! priors. The active root executes and validates it before dispatching to
//! equally pre-installed protocol specialists.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s26_dispatch::s26_shared_bundle_router;

pub const UTILITY_REGISTRY:&str="/self/tools/shared-payload-len";
fn lit(raw:&[u8])->Op{
    Op::ConstBytes{hex:raw.iter().map(|b|format!("{b:02x}")).collect()}
}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(target:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:target.into(),args}
}
fn br(cond:u32,yes:&str,yes_args:Vec<u32>,no:&str,no_args:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:yes_args,
        if_false:no.into(),false_args:no_args}
}
/// A disclosed graph-template, not a discovered operation. The actual
/// reusable version is built by the *running VM*, via RewriteOp(1, Len(0)).
pub fn s28_size_template()->Program{
    let program=Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![block("entry",vec![p(0,Type::Bytes)],
                vec![i(1,Op::ConstU32(0))],
                Terminator::Return{value:1})],
        }],
    };
    program.validate().expect("small size template must typecheck");
    program
}
/// This is a single frozen active initial program for P1 and P2.
/// Both preinstalled family specialists are passed as identical initial
/// content hashes; the program reads only public /public/E001/job0 and HELLO.
pub fn s28_cross_family_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s26_shared_bundle_router(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let old=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    let hello_instructions=std::mem::replace(&mut old.instructions,
        vec![i(5000,lit(b"/public/E001/job0"))]);
    let hello_terminator=std::mem::replace(&mut old.terminator,
        y(WorldCall::Read,vec![5000],"s28_job",vec![]));
    f.blocks.push(block("s28_public_hello",vec![],
        hello_instructions,hello_terminator));
    f.blocks.extend([
        // Public job is independent of protocol family; input bytes may
        // differ by world and are extracted by the active VM, not the host.
        block("s28_job",vec![p(5001,b)],vec![
            i(5002,lit(b"payload")),
            i(5003,Op::JsonFieldBytes(5001,5002)),
            i(5004,lit(b"values_le")),
            i(5005,Op::JsonFieldBytes(5003,5004)),
            i(5006,lit(UTILITY_REGISTRY.as_bytes())),
        ],y(WorldCall::Read,vec![5006],"s28_registry",vec![5005])),
        block("s28_registry",vec![p(5100,b),p(5101,b)],vec![
            i(5102,lit(b"code")),
            i(5103,Op::JsonFieldString(5101,5102)),
            i(5104,lit(b"Ok")),
            i(5105,Op::EqBytes(5103,5104)),
        ],br(5105,"s28_reuse",vec![5100,5101],
            "s28_make",vec![5100])),
        block("s28_reuse",vec![p(5110,b),p(5111,b)],vec![
            i(5112,lit(b"payload")),
            i(5113,Op::JsonFieldBytes(5111,5112)),
        ],j("s28_exec",vec![5110,5113])),
        // VM constructs an immutable, content-addressed *executable*
        // by rewriting one researcher supplied graph slot.
        block("s28_make",vec![p(5200,b)],vec![
            i(5201,lit(&s28_size_template().canonical_bytes())),
            i(5202,lit(b"entry")),
            i(5203,Op::ConstU32(1)),
            i(5204,lit(&serde_json::to_vec(&Op::Len(0)).unwrap())),
            i(5205,Op::RewriteOp(5201,5202,5203,5204)),
            i(5206,lit(b"OCEE-PROGRAM-V1\0")),
            i(5207,Op::Concat(5206,5205)),
            i(5208,Op::Sha256(5207)),
            i(5209,Op::HexEncode(5208)),
            i(5210,lit(b"/self/versions/")),
            i(5211,Op::Concat(5210,5209)),
            i(5212,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![5211,5205,5212],
            "s28_version_written",vec![5200,5209])),
        block("s28_version_written",vec![
            p(5300,b),p(5301,b),p(5302,b)
        ],vec![
            i(5303,lit(b"code")),
            i(5304,Op::JsonFieldString(5302,5303)),
            i(5305,lit(b"Ok")),
            i(5306,Op::EqBytes(5304,5305)),
        ],br(5306,"s28_register",vec![5300,5301],
            "s28_version_conflict",vec![5300,5301,5304])),
        // The immutable version can already be present when a fresh VM
        // independently generates a byte-identical content hash. World
        // preflight ensures /self/versions/{hash} is an authenticated code
        // version; only this specific Conflict is an acceptable outcome.
        // All unrelated WRITE errors still terminate instead of being
        // reinterpreted as successful learning.
        block("s28_version_conflict",vec![
            p(5310,b),p(5311,b),p(5312,b)
        ],vec![
            i(5313,lit(b"Conflict")),
            i(5314,Op::EqBytes(5312,5313)),
        ],br(5314,"s28_register",vec![5310,5311],"failed",vec![])),
        block("s28_register",vec![p(5400,b),p(5401,b)],vec![
            i(5402,lit(UTILITY_REGISTRY.as_bytes())),
            i(5403,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![5402,5401,5403],
            "s28_registered",vec![5400,5401])),
        block("s28_registered",vec![p(5410,b),p(5411,b),p(5412,b)],vec![
            i(5413,lit(b"code")),
            i(5414,Op::JsonFieldString(5412,5413)),
            i(5415,lit(b"Ok")),
            i(5416,Op::EqBytes(5414,5415)),
        ],br(5416,"s28_exec",vec![5410,5411],
            "failed",vec![])),
        block("s28_exec",vec![p(5500,b),p(5501,b)],vec![],
            y(WorldCall::Exec,vec![5501,5500],
                "s28_exec_reply",vec![])),
        block("s28_exec_reply",vec![p(5600,b)],vec![
            i(5601,lit(b"code")),
            i(5602,Op::JsonFieldString(5600,5601)),
            i(5603,lit(b"Ok")),
            i(5604,Op::EqBytes(5602,5603)),
        ],br(5604,"s28_check_result",vec![5600],"failed",vec![])),
        block("s28_check_result",vec![p(5700,b)],vec![
            i(5701,lit(b"payload")),
            i(5702,Op::JsonFieldBytes(5700,5701)),
            i(5703,lit(b"U32")),
            i(5704,Op::JsonFieldU32(5702,5703)),
            i(5705,Op::ConstU32(12)),
            i(5706,Op::EqU32(5704,5705)),
        ],br(5706,"s28_public_hello",vec![],
            "failed",vec![])),
    ]);
    root.validate().expect("S28 shared root with functional preflight must validate");
    root
}
