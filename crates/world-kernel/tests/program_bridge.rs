//! P2 integration acceptance: program lifecycle + autonomous tick, not learning tests.
use ocee_organism_vm::{
    Block, Function, Instruction, Literal, Op, Param, Program, Terminator, Type, WorldCall,
};
use ocee_world_kernel::{Action, Code, Config, NodeId, World, ORGANISM_ID};

fn base_program()->Program {
    Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![Block{
                id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![Instruction{id:1,op:Op::ConstU32(7)}],
                terminator:Terminator::Return{value:1},
            }]
        }]
    }
}
fn store(world:&mut World,p:&Program) -> String {
    let hash=p.content_hash().expect("program valid");
    let r=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:p.canonical_bytes(),
    }));
    assert_eq!(r.code,Code::Ok);
    hash
}
fn select(world:&mut World,hash:&str,expected_version:Option<u64>)->Code {
    world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version,bytes:hash.as_bytes().to_vec(),
    })).code
}

#[test]
fn candidate_is_immutable_and_active_switch_can_rollback() {
    let mut w=World::new(Config::default());
    let original=base_program();
    let h0=store(&mut w,&original);
    let changed=original.mutate_literal("entry",1,Literal::U32(19)).unwrap();
    let h1=store(&mut w,&changed);
    assert_ne!(h0,h1);
    assert_eq!(select(&mut w,&h0,None),Code::Ok);
    assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(h0.as_str()));
    assert_eq!(select(&mut w,&h1,Some(0)),Code::Ok);
    assert_eq!(select(&mut w,&h0,Some(0)),Code::Conflict);
    assert_eq!(select(&mut w,&h0,Some(1)),Code::Ok);
    assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),Some(h0.as_str()));
    let reused=w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{h0}"),
        expected_version:Some(0),bytes:original.canonical_bytes(),
    }));
    assert_eq!(reused.code,Code::Conflict); // no mutation of historical program
    assert_eq!(w.state.objects[&format!("/self/versions/{h0}")].version,0);
}

#[test]
fn autonomous_tick_emits_action_then_resumes_with_feedback() {
    let p=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,
            blocks:vec![
                Block{
                    id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions:vec![Instruction{id:1,op:Op::ConstBytes{hex:"abcd".into()}}],
                    terminator:Terminator::Yield{
                        call:WorldCall::Send,args:vec![1],
                        next:"after".into(),next_args:vec![],
                    },
                },
                Block{
                    id:"after".into(),params:vec![Param{id:10,ty:Type::Bytes}],
                    instructions:vec![Instruction{id:11,op:Op::Len(10)}],
                    terminator:Terminator::Return{value:11},
                },
            ],
        }]
    };
    let mut w=World::new(Config::default());
    let hash=store(&mut w,&p);
    assert_eq!(select(&mut w,&hash,None),Code::Ok);
    let first=w.tick_autonomous(ORGANISM_ID);
    assert_eq!(first.code,Code::Ok);
    assert_eq!(first.vm_gas_charged,2); // const + YIELD
    assert_eq!(w.state.pending_deliveries.len(),1);
    assert_eq!(w.state.pending_deliveries[0].destination,NodeId::C);
    let snapshot=w.snapshot_json();
    let mut restored=World::from_snapshot(&snapshot).unwrap();
    let second=w.tick_autonomous(ORGANISM_ID);
    let second_again=restored.tick_autonomous(ORGANISM_ID);
    assert_eq!(second,second_again);
    assert_eq!(w.state_hash(),restored.state_hash());
    assert_eq!(second.vm_gas_charged,2); // LEN + RETURN
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap().finished);
    assert_eq!(w.tick_autonomous(ORGANISM_ID).code,Code::Empty);
}

#[test]
fn exec_accepts_pure_candidate_but_rejects_world_side_effects() {
    let mut w=World::new(Config::default());
    let pure=base_program();
    let hash=store(&mut w,&pure);
    let response=w.tick(Some(Action::Exec{actor:ORGANISM_ID,program_hash:hash}));
    assert_eq!(response.code,Code::Ok);
    assert_eq!(response.gas_charged,2);
    let returned:ocee_organism_vm::Value=serde_json::from_slice(&response.payload).unwrap();
    assert_eq!(returned,ocee_organism_vm::Value::U32(7));

    let side_effect=Program{format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::Bytes,
            blocks:vec![
                Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions:vec![],
                    terminator:Terminator::Yield{
                        call:WorldCall::Send,args:vec![0],
                        next:"done".into(),next_args:vec![]}},
                Block{id:"done".into(),params:vec![Param{id:1,ty:Type::Bytes}],
                    instructions:vec![],terminator:Terminator::Return{value:1}},
            ]
        }]
    };
    let hash=store(&mut w,&side_effect);
    let response=w.tick(Some(Action::Exec{actor:ORGANISM_ID,program_hash:hash}));
    assert_eq!(response.code,Code::IsolatedSideEffect);
    assert_eq!(response.gas_charged,1);
    assert!(w.state.pending_deliveries.is_empty());
}
