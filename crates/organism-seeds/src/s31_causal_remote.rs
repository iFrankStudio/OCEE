//! S31: connect the ACTIVE VM-generated, content-addressed generic Len
//! executable to the actual C->B REMOTE_EVAL payload in BOTH protocol families.
//!
//! The shared root executes the generated tool on public values_le,
//! derives count = Len(bytes)/4 and packs it into a metered /self/work
//! object. Both preinstalled family specialists MUST READ that same object
//! and splice its bytes into the real REMOTE_EVAL body. They no longer embed
//! the protocol-independent constant [3,0]. A counterfactual mutation of
//! these bytes should change actual on-wire requests and B's decision.
//!
//! Researchers still preseed Len, the 4-byte word size, protocol grammar,
//! dispatch and program graph. No novel algorithm/open-ended evolution claim.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::{
    s19_p1_tool::s19_p1_build_and_use_tool,
    s23_p2_transfer::s23_p2_remote_tool_transfer,
    s30_composed_transfer::s30_composed_cross_family_root,
};

pub const PREPARED_COUNT:&str="/self/work/remote-count-le";

fn lit(data:&[u8])->Op{
    Op::ConstBytes{hex:data.iter().map(|b|format!("{b:02x}")).collect()}
}
fn p(id:u32,t:Type)->Param{Param{id,ty:t}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(next:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:next.into(),args}
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}

/// Build identical cross-family initial root. It inherits the S30 composed
/// search/reuse/repair and S28 VM-side Len specialization, but now pipes
/// the returned Len into an actual work object rather than a constant-12
/// gate. The WORD_SIZE=4 prior comes from the declared U32 workload ABI.
pub fn s31_shared_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s30_composed_cross_family_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let check=f.blocks.iter_mut().find(|b|b.id=="s28_check_result")
        .expect("S28 metered Len result");
    check.instructions=vec![
        i(5701,lit(b"payload")),
        i(5702,Op::JsonFieldBytes(5700,5701)),
        i(5703,lit(b"U32")),
        i(5704,Op::JsonFieldU32(5702,5703)),
        i(5705,Op::ConstU32(4)),
        i(5706,Op::Mod(5704,5705)),
        i(5707,Op::ConstU32(0)),
        i(5708,Op::EqU32(5706,5707)),
    ];
    check.terminator=br(5708,"s31_pack_count",vec![5704],
        "failed",vec![]);
    f.blocks.extend([
        block("s31_pack_count",vec![p(9750,u)],vec![
            i(9751,Op::ConstU32(4)),
            i(9752,Op::Div(9750,9751)),
            i(9753,Op::PackU16Le(9752)),
            i(9754,lit(PREPARED_COUNT.as_bytes())),
            i(9755,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![9754,9753,9755],
            "s31_saved_count",vec![])),
        block("s31_saved_count",vec![p(9760,b)],vec![
            i(9761,lit(b"code")),
            i(9762,Op::JsonFieldString(9760,9761)),
            i(9763,lit(b"Ok")),
            i(9764,Op::EqBytes(9762,9763)),
        ],br(9764,"s28_public_hello",vec![],"failed",vec![])),
    ]);
    root.validate().expect("S31 shared active root verifies");
    root
}

/// P1's S19 specialist now obtains the REMOTE_EVAL count from the
/// organism-written object, not a researcher literal [3,0].
pub fn s31_p1_count_consumer()->Program{
    let mut program=s19_p1_build_and_use_tool();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let job=f.blocks.iter_mut().find(|b|b.id=="s19_job").unwrap();
    job.instructions=vec![
        i(6093,lit(b"payload")),
        i(6094,Op::JsonFieldBytes(6092,6093)),
        i(6095,lit(b"values_le")),
        i(6096,Op::JsonFieldBytes(6094,6095)),
        i(6097,lit(PREPARED_COUNT.as_bytes())),
    ];
    job.terminator=y(WorldCall::Read,vec![6097],
        "s31_p1_count_status",vec![6090,6091,6096]);
    f.blocks.extend([
        block("s31_p1_count_status",vec![
            p(6090,b),p(6091,u),p(6096,b),p(6099,b)
        ],vec![
            i(9300,lit(b"code")),
            i(9301,Op::JsonFieldString(6099,9300)),
            i(9302,lit(b"Ok")),
            i(9303,Op::EqBytes(9301,9302)),
        ],br(9303,"s31_p1_count_payload",
            vec![6090,6091,6096,6099],"failure",vec![])),
        block("s31_p1_count_payload",vec![
            p(6090,b),p(6091,u),p(6096,b),p(6099,b)
        ],vec![
            i(9310,lit(b"payload")),
            i(9311,Op::JsonFieldBytes(6099,9310)),
            i(9312,lit(&[17,0,0,0])),
            i(9313,Op::Concat(9312,9311)),
            i(9314,Op::Concat(9313,6096)),
        ],y(WorldCall::Exec,vec![6090,9314],"s19_job_status",vec![6091])),
    ]);
    program.validate().expect("P1 specialist must consume S31 count");
    program
}

/// P2's S23 expert preserves all negotiated framing/role logic but
/// now splices the *same* root-produced count object into remote body.
pub fn s31_p2_count_consumer()->Program{
    let mut program=s23_p2_remote_tool_transfer();
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let job=f.blocks.iter_mut().find(|b|b.id=="s22_job_send").unwrap();
    job.instructions=vec![
        i(671,lit(b"payload")),
        i(672,Op::JsonFieldBytes(603,671)),
        i(673,lit(b"values_le")),
        i(674,Op::JsonFieldBytes(672,673)),
        i(675,lit(PREPARED_COUNT.as_bytes())),
    ];
    job.terminator=y(WorldCall::Read,vec![675],
        "s31_p2_count_status",vec![600,601,674]);
    f.blocks.extend([
        block("s31_p2_count_status",vec![
            p(600,b),p(601,u),p(674,b),p(603,b)
        ],vec![
            i(9320,lit(b"code")),
            i(9321,Op::JsonFieldString(603,9320)),
            i(9322,lit(b"Ok")),
            i(9323,Op::EqBytes(9321,9322)),
        ],br(9323,"s31_p2_count_payload",vec![600,601,674,603],
            "failed",vec![])),
        block("s31_p2_count_payload",vec![
            p(600,b),p(601,u),p(674,b),p(603,b)
        ],vec![
            i(9330,lit(b"payload")),
            i(9331,Op::JsonFieldBytes(603,9330)),
            i(9332,Op::Concat(9331,674)),
        ],j("s23_job_registry",vec![600,601,9332])),
    ]);
    program.validate().expect("P2 specialist must consume S31 count");
    program
}
