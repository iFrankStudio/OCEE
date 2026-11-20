//! S37: a paid, kernel-bound provenance check before trusting *any*
//! cached supervised probe label (whether newly minted or previously stored).
//!
//! The organism VM still selects input and performs actual Probe from S36.
//! It now submits the raw cached receipt to VerifyProbe, which compares it
//! against a PRIVATE, environment-bound issue record and current task bytes.
//! This rejects preloaded, edited, and stale caches before candidate Exec.
//!
//! No cryptographic public signature, no automatic quota re-arm on task
//! changes, no unrestricted input search. Finite researcher-defined guard.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Instruction,Op,Param,Program,Terminator,Type,WorldCall
};
use crate::s36_active_probe::s36_active_probe_root;

fn lit(data:&[u8])->Op{
    Op::ConstBytes{hex:data.iter().map(|x|format!("{x:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn block(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,term:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator:term}
}
fn s(base:u32)->Vec<Param>{
    [Type::U32,Type::Bytes,Type::Bytes,Type::Bytes,Type::U32,
     Type::Bytes,Type::U32,Type::Bytes,Type::Bytes,Type::U32]
        .iter().enumerate().map(|(idx,ty)|p(base+idx as u32,*ty)).collect()
}
fn ids(base:u32)->Vec<u32>{(base..base+10).collect()}
fn ext(base:u32,extra:&[(u32,Type)])->Vec<Param>{
    let mut x=s(base);
    x.extend(extra.iter().map(|&(id,ty)|p(id,ty)));
    x
}
fn br(cond:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:cond,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
pub fn s37_verified_probe_root(p1_hash:&str,p2_hash:&str)->Program {
    let mut root=s36_active_probe_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    // Both paths need attestation, not just a cache hit: a task may change
    // immediately after Query but before the evidence is used.
    for id in ["s36_cache_check","s36_cache_ack"]{
        let b=f.blocks.iter_mut().find(|x|x.id==id).expect("S36 gate");
        let Terminator::Branch{if_true,..}=&mut b.terminator
            else{panic!("S36 accepted-label branch missing")};
        *if_true="s37_verify_prepare".into();
    }
    f.blocks.extend([
        // Forward the normal ActionResult JSON (cached READ or fresh Probe)
        // while separately extracting only its literal payload bytes.
        block("s37_verify_prepare",ext(7000,&[(7010,Type::Bytes)]),vec![
            ins(7011,lit(b"payload")),
            ins(7012,Op::JsonFieldBytes(7010,7011)),
        ],Terminator::Yield{
            call:WorldCall::VerifyProbe,
            args:vec![7012],next:"s37_verify_result".into(),
            next_args:[ids(7000),vec![7010]].concat(),
        }),
        block("s37_verify_result",ext(7100,&[
            (7110,Type::Bytes),(7111,Type::Bytes)
        ]),vec![
            ins(7112,lit(b"code")),
            ins(7113,Op::JsonFieldString(7111,7112)),
            ins(7114,lit(b"Ok")),
            ins(7115,Op::EqBytes(7113,7114)),
        ],br(7115,"s35_probe_parse",
            [ids(7100),vec![7110]].concat(),"fail",vec![])),
    ]);
    root.validate().expect("S37 typed attested evidence gate");
    root
}
