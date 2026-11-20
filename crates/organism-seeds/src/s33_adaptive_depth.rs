//! S33: bounded EVIDENCE-DRIVEN structural depth, not only operator choice.
//!
//! The researcher supplies two structural priors: existing S6 three-node
//! expression and a depth-extended expression adding Len(input) to the
//! previous result. The SAME root tests 18 shallow programs first, then up
//! to 18 deep programs; only public training labels select which topology
//! is actually generated, retained, inherited and used for real B work.
//! Neither the hidden seed rule, its depth flag, nor the held-out answer is
//! visible to the VM. This is finite, researcher-bounded topology selection,
//! NOT autonomous invention of arbitrary graphs or open-ended evolution.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type
};
use crate::s32_composite_b_operand::s32_composite_operands_root;

pub const DEPTH_TASK:&str="/public/E001/compose-depth";

fn lit(bytes:&[u8])->Op{
    Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}

/// Two program *shapes*. The deep one has two additional instructions
/// (Len and Add), and returns a DIFFERENT register. The VM reads a public
/// training set before selecting these, but the two shapes are researcher
/// priors, not invented from scratch.
fn shape(deep:bool)->Program{
    let mut instructions=vec![
        ins(1,Op::ConstU32(0)),
        ins(2,Op::Len(0)),
        ins(3,Op::Len(0)),
        ins(4,Op::Add(2,3)),
    ];
    if deep{
        instructions.extend([
            ins(5,Op::Len(0)),
            ins(6,Op::Add(4,5)),
        ]);
    }
    let p=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),params:vec![param(0,Type::Bytes)],
                instructions,terminator:Terminator::Return{
                    value:if deep{6}else{4}
                },
            }],
        }],
    };
    p.validate().expect("both researcher-supplied shapes must be typed");
    p
}

pub fn s33_adaptive_depth_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s32_composite_operands_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];
    let entry=f.blocks.iter_mut().find(|x|x.id=="entry").unwrap();
    entry.instructions.iter_mut().find(|x|x.id==1).unwrap().op=
        lit(DEPTH_TASK.as_bytes());
    let loop_header=f.blocks.iter_mut().find(|x|x.id=="loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut loop_header.terminator
        else {panic!("S6 bounded candidate loop missing")};
    *limit=36;

    let candidate=f.blocks.iter_mut().find(|x|x.id=="make").unwrap();
    // idx [0,35]: base_candidate=idx%18; topology=idx/18.
    // No seed-dependent branch: selection happens only after real EXEC of
    // the candidate against both examples in the active VM.
    let templates=serde_json::to_vec(&vec![
        serde_json::json!({"program":shape(false).canonical_bytes()}),
        serde_json::json!({"program":shape(true).canonical_bytes()}),
    ]).expect("finite structural grammar");
    let setup=vec![
        ins(236,Op::ConstU32(18)),
        ins(237,Op::Mod(200,236)),
        ins(238,Op::Div(200,236)),
        ins(239,lit(&templates)),
        ins(240,Op::JsonIndex(239,238)),
        ins(241,lit(b"program")),
        ins(242,Op::JsonFieldBytes(240,241)),
    ];
    candidate.instructions.splice(0..0,setup);
    for node in &mut candidate.instructions {
        node.op=match node.id{
            209=>Op::Mod(237,208),
            210=>Op::Div(237,208),
            213=>Op::Div(237,212),
            222=>Op::RewriteOp(242,218,219,214),
            _=>node.op.clone(),
        };
    }

    root.validate().expect("S33 adaptive topology selection root");
    root
}

/// Mechanism ablation: identical family experts, search body and sample
/// feedback, but only the original 18 shallow candidates are reachable.
pub fn s33_shallow_only_control(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s33_adaptive_depth_root(p1_hash,p2_hash);
    let loop_header=root.functions[0].blocks.iter_mut()
        .find(|x|x.id=="loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut loop_header.terminator
        else {unreachable!()};
    *limit=18;
    root.validate().expect("S33 depth-control root");
    root
}
