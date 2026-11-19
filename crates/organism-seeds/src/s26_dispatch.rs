//! S26C: a frozen public-evidence router switches, via a charged
//! /self/active CAS, into one of two *equally preinstalled* seeded specialists.
//! All worlds receive the same root and same P1/P2 programs. This is
//! program selection under known grammar priors, not invented adaptation.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s26_family::s26_public_family_router;

fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}

/// Both supplied hash values refer only to programs preinstalled with
/// identical resource charges in every comparison world. No protocol
/// instance parameter or B result is supplied to this root program.
pub fn s26_shared_bundle_router(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s26_public_family_router();
    let f=&mut root.functions[0];
    for (name,id,hash) in [
        ("store_p1",205,p1_hash),
        ("store_p2",215,p2_hash),
    ] {
        let block=f.blocks.iter_mut().find(|b|b.id==name)
            .expect("public observation branch");
        block.instructions.push(i(id,lit(hash.as_bytes())));
        if let Terminator::Yield{next_args,..}=&mut block.terminator{
            *next_args=vec![id];
        }else{panic!("expected public observation write")}
    }
    let reply=f.blocks.iter_mut().find(|b|b.id=="write_reply")
        .expect("common write reply");
    // Carry the selected specialist's immutable hash as ordinary VM data.
    // On a successful provenance write, perform a paid CAS active switch.
    reply.params=vec![p(300,Type::Bytes),p(301,Type::Bytes)];
    reply.instructions=vec![
        i(310,lit(b"code")),
        i(311,Op::JsonFieldString(301,310)),
        i(312,lit(b"Ok")),
        i(313,Op::EqBytes(311,312)),
    ];
    reply.terminator=Terminator::Branch{
        condition:313,
        if_true:"switch_active".into(),
        true_args:vec![300],
        if_false:"failed".into(),
        false_args:vec![],
    };
    f.blocks.push(Block{
        id:"switch_active".into(),
        params:vec![p(700,Type::Bytes)],
        instructions:vec![
            i(701,lit(b"/self/active")),
            i(702,Op::ConstU32(0)),
        ],
        // A successful active replacement resets the old continuation, so
        // switch_reply is reached only if switching fails.
        terminator:Terminator::Yield{
            call:WorldCall::Write,
            args:vec![701,700,702],
            next:"switch_reply".into(),
            next_args:vec![],
        },
    });
    f.blocks.push(Block{
        id:"switch_reply".into(),
        params:vec![p(710,Type::Bytes)],
        instructions:vec![i(711,Op::ConstU32(0))],
        terminator:Terminator::Return{value:711},
    });
    root.validate().expect("S26C public routing plus paid program switch");
    root
}
