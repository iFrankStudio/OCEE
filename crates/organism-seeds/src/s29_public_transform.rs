//! S29 developmental: active VM chooses between three generic typed
//! operators using public labelled examples, creates executable with
//! RewriteOp and proves held-out predictions across protocol families.
//! Neither operator bank, template nor active control graph is self-invented.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall};
use crate::s28_cross_utility::s28_cross_family_root;

pub const REGISTRY:&str="/self/tools/portable-transform-s29";
pub const PREDICTION:&str="/self/results/portable-transform-prediction";

fn lit(raw:&[u8])->Op{ Op::ConstBytes{hex:raw.iter().map(|b|format!("{b:02x}")).collect()} }
fn p(id:u32,ty:Type)->Param{Param{id,ty}}
fn i(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn bl(id:&str,params:Vec<Param>,instructions:Vec<Instruction>,terminator:Terminator)->Block{
    Block{id:id.into(),params,instructions,terminator}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
fn j(next:&str,args:Vec<u32>)->Terminator{Terminator::Jump{target:next.into(),args}}
fn br(c:u32,yes:&str,ya:Vec<u32>,no:&str,na:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:yes.into(),true_args:ya,
        if_false:no.into(),false_args:na}
}
fn ctx()->Vec<Param>{
    vec![p(100,Type::Bytes),p(101,Type::Bytes),p(102,Type::U32),
         p(103,Type::U32),p(104,Type::Bytes)]
}
fn ids()->Vec<u32>{vec![100,101,102,103,104]}
fn extended(extra: &[(u32,Type)])->Vec<Param>{
    let mut v=ctx();
    v.extend(extra.iter().map(|(id,ty)|p(*id,*ty)));
    v
}
fn plus(mut base:Vec<u32>,extra:u32)->Vec<u32>{base.push(extra);base}

/// Three candidate op variants are known to the researcher-supplied
/// grammar. Only the active VM chooses between them.
pub fn s29_template()->Program{
    let x=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{name:"main".into(),
            args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![bl("entry",vec![p(0,Type::Bytes)],vec![
                i(1,Op::ConstU32(0)),
                i(2,Op::ConstU32(0)),
            ],Terminator::Return{value:2})]
        }]};
    x.validate().expect("generic typed template valid");
    x
}
pub fn s29_evidence_selected_root(p1:&str,p2:&str)->Program{
    let mut root=s28_cross_family_root(p1,p2);
    let f=&mut root.functions[0];
    let b=Type::Bytes;let u=Type::U32;
    let entry=f.blocks.iter_mut().find(|b|b.id=="entry").expect("root entry");
    let previous_instructions=std::mem::replace(&mut entry.instructions,
        vec![i(6000,lit(b"/public/E001/transform"))]);
    let previous_terminator=std::mem::replace(&mut entry.terminator,
        y(WorldCall::Read,vec![6000],"s29_sample",vec![]));
    f.blocks.push(bl("s29_after",vec![],
        previous_instructions,previous_terminator));

    f.blocks.extend([
        // Public two training examples and a held-out input. No hidden
        // rule / world seed is passed to the VM.
        bl("s29_sample",vec![p(6001,b)],vec![
            i(6002,lit(b"payload")),
            i(6003,Op::JsonFieldBytes(6001,6002)),
            i(6004,lit(b"examples")),
            i(6005,Op::JsonFieldJson(6003,6004)),
            i(6006,Op::ConstU32(0)),
            i(6007,Op::ConstU32(1)),
            i(6008,Op::JsonIndex(6005,6006)),
            i(6009,Op::JsonIndex(6005,6007)),
            i(6010,lit(b"input")),
            i(6011,lit(b"output")),
            i(6012,Op::JsonFieldBytes(6008,6010)),
            i(6013,Op::JsonFieldBytes(6009,6010)),
            i(6014,Op::JsonFieldU32(6008,6011)),
            i(6015,Op::JsonFieldU32(6009,6011)),
            i(6016,lit(b"held_out_input")),
            i(6017,Op::JsonFieldBytes(6003,6016)),
            i(6018,lit(REGISTRY.as_bytes())),
        ],y(WorldCall::Read,vec![6018],"s29_registry",
            vec![6012,6013,6014,6015,6017])),
        bl("s29_registry",extended(&[(105,b)]),vec![
            i(6019,lit(b"code")),
            i(6020,Op::JsonFieldString(105,6019)),
            i(6021,lit(b"Ok")),
            i(6022,Op::EqBytes(6020,6021)),
        ],br(6022,"s29_reuse",plus(ids(),105),
            "s29_len1",ids())),
        bl("s29_reuse",extended(&[(105,b)]),vec![
            i(6023,lit(b"payload")),
            i(6024,Op::JsonFieldBytes(105,6023)),
        ],j("s29_check1",plus(ids(),6024))),
        // The search order is disclosed, but the selection uses BOTH
        // public labels. A candidate cannot pass just one colliding sample.
        bl("s29_len1",ctx(),vec![
            i(6100,Op::Len(100)),i(6101,Op::EqU32(6100,102)),
        ],br(6101,"s29_len2",ids(),"s29_xor1",ids())),
        bl("s29_len2",ctx(),vec![
            i(6102,Op::Len(101)),i(6103,Op::EqU32(6102,103)),
        ],br(6103,"s29_take_len",ids(),"s29_xor1",ids())),
        bl("s29_xor1",ctx(),vec![
            i(6110,Op::XorFold(100)),i(6111,Op::EqU32(6110,102)),
        ],br(6111,"s29_xor2",ids(),"s29_first1",ids())),
        bl("s29_xor2",ctx(),vec![
            i(6112,Op::XorFold(101)),i(6113,Op::EqU32(6112,103)),
        ],br(6113,"s29_take_xor",ids(),"s29_first1",ids())),
        bl("s29_first1",ctx(),vec![
            i(6120,Op::ConstU32(0)),i(6121,Op::ByteAt(100,6120)),
            i(6122,Op::EqU32(6121,102)),
        ],br(6122,"s29_first2",ids(),"failed",vec![])),
        bl("s29_first2",ctx(),vec![
            i(6123,Op::ConstU32(0)),i(6124,Op::ByteAt(101,6123)),
            i(6125,Op::EqU32(6124,103)),
        ],br(6125,"s29_take_first",ids(),"failed",vec![])),
        bl("s29_take_len",ctx(),vec![
            i(6130,lit(&serde_json::to_vec(&Op::Len(0)).unwrap())),
        ],j("s29_make",plus(ids(),6130))),
        bl("s29_take_xor",ctx(),vec![
            i(6131,lit(&serde_json::to_vec(&Op::XorFold(0)).unwrap())),
        ],j("s29_make",plus(ids(),6131))),
        bl("s29_take_first",ctx(),vec![
            i(6132,lit(&serde_json::to_vec(&Op::ByteAt(0,1)).unwrap())),
        ],j("s29_make",plus(ids(),6132))),
        // VM builds the executable and version hash. The host merely
        // validates and bills a real WRITE; no host-side candidate choice.
        bl("s29_make",extended(&[(105,b)]),vec![
            i(6200,lit(&s29_template().canonical_bytes())),
            i(6201,lit(b"entry")),
            i(6202,Op::ConstU32(2)),
            i(6203,Op::RewriteOp(6200,6201,6202,105)),
            i(6204,lit(b"OCEE-PROGRAM-V1\0")),
            i(6205,Op::Concat(6204,6203)),
            i(6206,Op::Sha256(6205)),
            i(6207,Op::HexEncode(6206)),
            i(6208,lit(b"/self/versions/")),
            i(6209,Op::Concat(6208,6207)),
            i(6210,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![6209,6203,6210],"s29_version_reply",
            plus(ids(),6207))),
        bl("s29_version_reply",extended(&[(105,b),(106,b)]),vec![
            i(6300,lit(b"code")),
            i(6301,Op::JsonFieldString(106,6300)),
            i(6302,lit(b"Ok")),
            i(6303,Op::EqBytes(6301,6302)),
        ],br(6303,"s29_register",plus(ids(),105),
            "s29_version_conflict",plus(plus(ids(),105),6301))),
        bl("s29_version_conflict",extended(&[(105,b),(106,b)]),vec![
            i(6310,lit(b"Conflict")),
            i(6311,Op::EqBytes(106,6310)),
        ],br(6311,"s29_register",plus(ids(),105),"failed",vec![])),
        bl("s29_register",extended(&[(105,b)]),vec![
            i(6320,lit(REGISTRY.as_bytes())),
            i(6321,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![6320,105,6321],"s29_registered",
            plus(ids(),105))),
        bl("s29_registered",extended(&[(105,b),(106,b)]),vec![
            i(6330,lit(b"code")),
            i(6331,Op::JsonFieldString(106,6330)),
            i(6332,lit(b"Ok")),
            i(6333,Op::EqBytes(6331,6332)),
        ],br(6333,"s29_check1",plus(ids(),105),"failed",vec![])),
        bl("s29_check1",extended(&[(105,b)]),vec![],
            y(WorldCall::Exec,vec![105,100],"s29_check1_reply",
                plus(ids(),105))),
        bl("s29_check1_reply",extended(&[(105,b),(106,b)]),vec![
            i(6400,lit(b"code")),
            i(6401,Op::JsonFieldString(106,6400)),
            i(6402,lit(b"Ok")),
            i(6403,Op::EqBytes(6401,6402)),
        ],br(6403,"s29_check1_ok",plus(plus(ids(),105),106),
            "failed",vec![])),
        bl("s29_check1_ok",extended(&[(105,b),(106,b)]),vec![
            i(6410,lit(b"payload")),
            i(6411,Op::JsonFieldBytes(106,6410)),
            i(6412,lit(b"U32")),
            i(6413,Op::JsonFieldU32(6411,6412)),
            i(6414,Op::EqU32(6413,102)),
        ],br(6414,"s29_check2",vec![101,103,104,105],"failed",vec![])),
        bl("s29_check2",vec![p(200,b),p(201,u),p(202,b),p(203,b)],vec![],
            y(WorldCall::Exec,vec![203,200],"s29_check2_reply",
                vec![201,202,203])),
        bl("s29_check2_reply",vec![
            p(201,u),p(202,b),p(203,b),p(204,b)
        ],vec![
            i(6500,lit(b"code")),
            i(6501,Op::JsonFieldString(204,6500)),
            i(6502,lit(b"Ok")),
            i(6503,Op::EqBytes(6501,6502)),
        ],br(6503,"s29_check2_ok",vec![201,202,203,204],
            "failed",vec![])),
        bl("s29_check2_ok",vec![
            p(201,u),p(202,b),p(203,b),p(204,b)
        ],vec![
            i(6510,lit(b"payload")),
            i(6511,Op::JsonFieldBytes(204,6510)),
            i(6512,lit(b"U32")),
            i(6513,Op::JsonFieldU32(6511,6512)),
            i(6514,Op::EqU32(6513,201)),
        ],br(6514,"s29_held",vec![202,203],"failed",vec![])),
        // The held-out answer is NOT in public data or encoded as a constant
        // in the program; it is generated by the sealed candidate.
        bl("s29_held",vec![p(202,b),p(203,b)],vec![],
            y(WorldCall::Exec,vec![203,202],"s29_held_reply",vec![])),
        bl("s29_held_reply",vec![p(6600,b)],vec![
            i(6601,lit(b"code")),
            i(6602,Op::JsonFieldString(6600,6601)),
            i(6603,lit(b"Ok")),
            i(6604,Op::EqBytes(6602,6603)),
        ],br(6604,"s29_held_ok",vec![6600],"failed",vec![])),
        bl("s29_held_ok",vec![p(6700,b)],vec![
            i(6701,lit(b"payload")),
            i(6702,Op::JsonFieldBytes(6700,6701)),
            i(6703,lit(b"U32")),
            i(6704,Op::JsonFieldJson(6702,6703)),
            i(6705,lit(PREDICTION.as_bytes())),
            i(6706,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![6705,6704,6706],
            "s29_prediction_saved",vec![])),
        bl("s29_prediction_saved",vec![p(6800,b)],vec![
            i(6801,lit(b"code")),
            i(6802,Op::JsonFieldString(6800,6801)),
            i(6803,lit(b"Ok")),
            i(6804,Op::EqBytes(6802,6803)),
        ],br(6804,"s29_after",vec![],"failed",vec![])),
    ]);
    root.validate().expect("S29 frozen VM multi-operator example selection graph");
    root
}


/// S29 development follow-up: a registered, syntactically valid inherited
/// transform may be semantically wrong for the NEW world's public labels.
/// The active VM pays a genuine first EXEC, rejects its wrong value, then
/// re-selects from the same three researcher-supplied generic operators.
/// The new program is installed with an immutable content hash and the
/// observed registry version is updated with a bounded CAS0 transaction.
/// A stale (version >=1) registry cannot be overwritten by this recovery.
/// NOTE: version objects written BEFORE a failed CAS remain as auditable
/// orphan candidates; adoption itself is never claimed in that case.
pub fn s29_feedback_repair_root(p1:&str,p2:&str)->Program{
    let mut root=s29_evidence_selected_root(p1,p2);
    let f=&mut root.functions[0];
    let first=f.blocks.iter_mut().find(|b|b.id=="s29_check1_ok")
        .expect("actual first inherited EXEC must be checked");
    // First mismatching labelled sample triggers bounded fresh selection.
    // After selecting the right generic Op, full two-label checks still run.
    first.terminator=br(6414,
        "s29_check2",vec![101,103,104,105],
        "s29_len1",ids());
    // Repaired/reselected versions may replace the known first registry
    // version, but not any silently concurrent / newer version.
    let register=f.blocks.iter_mut().find(|b|b.id=="s29_register")
        .expect("new version is paid installed before registry WRITE");
    register.instructions.iter_mut().find(|i|i.id==6321)
        .expect("old missing-only WRITE version flag").op=Op::ConstU32(0);
    let feedback=f.blocks.iter_mut().find(|b|b.id=="s29_registered")
        .expect("real write feedback");
    feedback.terminator=br(6333,"s29_repaired_check1",plus(ids(),105),
        "s29_cas_conflict",plus(plus(ids(),105),6331));
    let b=Type::Bytes;
    f.blocks.extend([
        // If the CAS0 cannot match because registry is missing, retry
        // *only once* with expected_version=None (a new object).
        // On a stale/in-use registry the fallback is Conflict too and
        // final U32(0), preserving the old registry content/version.
        bl("s29_cas_conflict",extended(&[(105,b),(106,b)]),vec![
            i(6920,lit(b"Conflict")),
            i(6921,Op::EqBytes(106,6920)),
        ],br(6921,"s29_create_missing",
            plus(ids(),105),"failed",vec![])),
        bl("s29_create_missing",extended(&[(105,b)]),vec![
            i(6922,lit(REGISTRY.as_bytes())),
            i(6923,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![6922,105,6923],
            "s29_created_reply",plus(ids(),105))),
        bl("s29_created_reply",extended(&[(105,b),(106,b)]),vec![
            i(6924,lit(b"code")),
            i(6925,Op::JsonFieldString(106,6924)),
            i(6926,lit(b"Ok")),
            i(6927,Op::EqBytes(6925,6926)),
        ],br(6927,"s29_repaired_check1",plus(ids(),105),"failed",vec![])),
        // A separate *single-pass* repaired validation graph. The initial
        // check1 may branch back into candidate selection once, but this
        // repaired check NEVER loops into selection. VM still forbids
        // unbounded control-flow cycles; we do not change that verifier.
        bl("s29_repaired_check1",extended(&[(105,b)]),vec![],
            y(WorldCall::Exec,vec![105,100],
                "s29_repaired_exec_reply",plus(ids(),105))),
        bl("s29_repaired_exec_reply",extended(&[(105,b),(106,b)]),vec![
            i(6930,lit(b"code")),
            i(6931,Op::JsonFieldString(106,6930)),
            i(6932,lit(b"Ok")),
            i(6933,Op::EqBytes(6931,6932)),
        ],br(6933,"s29_repaired_result",
            plus(plus(ids(),105),106),"failed",vec![])),
        bl("s29_repaired_result",extended(&[(105,b),(106,b)]),vec![
            i(6934,lit(b"payload")),
            i(6935,Op::JsonFieldBytes(106,6934)),
            i(6936,lit(b"U32")),
            i(6937,Op::JsonFieldU32(6935,6936)),
            i(6938,Op::EqU32(6937,102)),
        ],br(6938,"s29_check2",vec![101,103,104,105],
            "failed",vec![])),
    ]);
    root.validate().expect("S29 bounded feedback repair and CAS adoption");
    root
}


/// A first-example collision does not make an inherited tool correct:
/// its genuine second EXEC may still disagree with the public second label.
/// Recover the full two-label task through a *paid* public Read; reselect
/// within the already disclosed 3-op grammar and CAS-adopt the new version.
/// Crucially, the post-repair second EXEC follows a separate one-pass graph,
/// so no control path can restart repair indefinitely or bypass VM limits.
pub fn s29_second_label_repair_root(p1:&str,p2:&str)->Program{
    let mut root=s29_feedback_repair_root(p1,p2);
    let f=&mut root.functions[0];
    let second=f.blocks.iter_mut().find(|b|b.id=="s29_check2_ok")
        .expect("initial second labelled Exec must remain observable");
    second.terminator=br(6514,"s29_held",vec![202,203],
        "s29_second_recover_read",vec![]);
    let repaired_first=f.blocks.iter_mut()
        .find(|b|b.id=="s29_repaired_result")
        .expect("the repaired first check is already single-pass");
    repaired_first.terminator=br(6938,"s29_repaired_check2",
        vec![101,103,104,105],"failed",vec![]);
    // Exactly the same typed arithmetic and feedback validation as
    // the original second public sample, but no edge back to repair.
    let mut second_exec=f.blocks.iter()
        .find(|b|b.id=="s29_check2").unwrap().clone();
    second_exec.id="s29_repaired_check2".into();
    if let Terminator::Yield{next,..}=&mut second_exec.terminator{
        *next="s29_repaired_check2_reply".into();
    }else{panic!("second check must actually yield a paid Exec");}
    let mut second_reply=f.blocks.iter()
        .find(|b|b.id=="s29_check2_reply").unwrap().clone();
    second_reply.id="s29_repaired_check2_reply".into();
    if let Terminator::Branch{if_true,..}=&mut second_reply.terminator{
        *if_true="s29_repaired_check2_ok".into();
    }else{panic!("second check must distinguish failed Exec");}
    let mut second_ok=f.blocks.iter()
        .find(|b|b.id=="s29_check2_ok").unwrap().clone();
    second_ok.id="s29_repaired_check2_ok".into();
    second_ok.terminator=br(6514,"s29_held",vec![202,203],
        "failed",vec![]);
    let b=Type::Bytes;
    // The old second-sample success path no longer carries the first
    // sample, so recover both labels via a real charged public Read.
    let sample_instrs=f.blocks.iter()
        .find(|bl|bl.id=="s29_sample")
        .unwrap().instructions.iter()
        .filter(|inst|inst.id!=6018)
        .cloned().collect::<Vec<_>>();
    f.blocks.extend([
        bl("s29_second_recover_read",vec![],vec![
            i(7010,lit(b"/public/E001/transform"))
        ],y(WorldCall::Read,vec![7010],
            "s29_second_recover_reply",vec![])),
        bl("s29_second_recover_reply",vec![p(7011,b)],vec![
            i(7012,lit(b"code")),
            i(7013,Op::JsonFieldString(7011,7012)),
            i(7014,lit(b"Ok")),
            i(7015,Op::EqBytes(7013,7014)),
        ],br(7015,"s29_second_recover_parse",vec![7011],
            "failed",vec![])),
        bl("s29_second_recover_parse",vec![p(6001,b)],
            sample_instrs,
            j("s29_len1",vec![6012,6013,6014,6015,6017])),
        second_exec,second_reply,second_ok
    ]);
    root.validate().expect(
        "S29 second labelled feedback repair must stay statically bounded");
    root
}
