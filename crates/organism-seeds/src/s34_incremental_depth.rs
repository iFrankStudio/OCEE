//! S34: the VM constructs different-depth executable graphs incrementally.
//!
//! Unlike S33's two complete prebuilt programs, there is ONE researcher
//! provided base graph and a finite bank of typed *instruction fragments*.
//! The SAME root VM chooses the stage count 0/1/2 by public I/O feedback,
//! executes a new metered generic AppendInstructions primitive (not a host
//! shortcut), hashes, saves, validates, and migrates the resulting executable.
//! Researchers still seed fragments, 18 base operator combos, ordering,
//! max depth=2, remote task contract, and P1/P2 experts. NOT open ended.
#![forbid(unsafe_code)]
use ocee_organism_vm::{Instruction,Op,Program,Terminator};
use crate::s32_composite_b_operand::s32_composite_operands_root;
use crate::s33_adaptive_depth::DEPTH_TASK;

fn lit(input:&[u8])->Op{
    Op::ConstBytes{hex:input.iter().map(|x|format!("{x:02x}")).collect()}
}
fn ins(id:u32,op:Op)->Instruction{Instruction{id,op}}

/// Researcher-supplied typed generator vocabulary, not full-program
/// skeletons. S34 new behavior is that the organism appends actual
/// instructions to an already executable, statically checked program.
fn extension(depth:u32)->Vec<Instruction>{
    let mut stages=Vec::new();
    for level in 0..depth{
        let unary=5+2*level;
        let acc=4+2*level;
        stages.push(ins(unary,Op::Len(0)));
        stages.push(ins(unary+1,Op::Add(acc,unary)));
    }
    stages
}

pub fn s34_incremental_depth_root(p1_hash:&str,p2_hash:&str)->Program{
    let mut root=s32_composite_operands_root(p1_hash,p2_hash);
    let f=&mut root.functions[0];

    // Same root in all families/seeds. Task is test-side public-labelled
    // extension of the S33 fixture; only the two ordinary labels are read.
    let start=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
    start.instructions.iter_mut().find(|x|x.id==1).unwrap().op=
        lit(DEPTH_TASK.as_bytes());
    let bounded=f.blocks.iter_mut().find(|b|b.id=="loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut bounded.terminator
        else{panic!("S34 inherited bounded search")};
    *limit=54; // 18 base operator combinations * 3 constructed depths.

    let candidate=f.blocks.iter_mut().find(|b|b.id=="make").unwrap();
    let fragments=(0..=2u32).map(|depth|{
        serde_json::json!({
            "steps":serde_json::to_vec(&extension(depth))
                .expect("typed fragment bytes"),
            "output":4+2*depth,
        })
    }).collect::<Vec<_>>();
    // These are 3 instruction fragments, never serialized full programs.
    // The VM applies its universal, fully statically-checked editor.
    let setup=vec![
        ins(236,Op::ConstU32(18)),
        ins(237,Op::Mod(200,236)),
        ins(238,Op::Div(200,236)),
        ins(239,lit(&serde_json::to_vec(&fragments).unwrap())),
        ins(240,Op::JsonIndex(239,238)),
        ins(241,lit(b"steps")),
        ins(242,Op::JsonFieldBytes(240,241)),
        ins(243,lit(b"output")),
        ins(244,Op::JsonFieldU32(240,243)),
    ];
    candidate.instructions.splice(0..0,setup);
    for inst in &mut candidate.instructions{
        inst.op=match inst.id{
            209=>Op::Mod(237,208),
            210=>Op::Div(237,208),
            213=>Op::Div(237,212),
            // After three actual VM RewriteOp modifications, append the
            // chosen incremental extension, changing the return register.
            226=>Op::Concat(225,245),
            _=>inst.op.clone(),
        };
    }
    let insert_at=candidate.instructions.iter().position(|x|x.id==225)
        .expect("S6 hash prelude");
    candidate.instructions.insert(insert_at,
        ins(245,Op::AppendInstructions(224,218,242,244)));
    // Content-addressed WRITE must persist the SAME extended canonical
    // bytes hashed above. Using the pre-extension register 224 here
    // correctly fails the world kernel's immutable hash preflight.
    let Terminator::Yield{call:_,args,..}=&mut candidate.terminator
        else {panic!("S34 candidate must write program version")};
    assert_eq!(args.as_slice(),&[230,224,231]);
    *args=vec![230,245,231];
    root.validate().expect("S34 incremental typed construction root");
    root
}

/// Strict mechanism ablation: only the first 36 candidate trials are
/// allowed, so a task requiring two extra stages must fail after exhausting
/// all 18*2 earlier candidates, with no B work or fabricated success.
pub fn s34_at_most_one_stage(p1_hash:&str,p2_hash:&str)->Program{
    let mut p=s34_incremental_depth_root(p1_hash,p2_hash);
    let header=p.functions[0].blocks.iter_mut()
        .find(|b|b.id=="loop").unwrap();
    let Terminator::RepeatBounded{limit,..}=&mut header.terminator
        else{unreachable!()};
    *limit=36;
    p.validate().expect("S34 depth cap control");
    p
}
