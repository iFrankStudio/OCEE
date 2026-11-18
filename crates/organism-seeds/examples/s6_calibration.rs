//! S6 feasibility pilot. This only reports observations; no confirmatory seeds.
//! Runs the UNMODIFIED S6 program at both original (12k) and development (100k)
//! A-gas budgets. Failed/unfinished attempts are retained, not discarded.
use ocee_organism_seeds::s6_compose::s6_composition_search;
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};
use serde::Serialize;

#[derive(Serialize)]
struct Record {
    seed:u64,
    a_gas_per_epoch:u64,
    max_ticks:u64,
    elapsed_ticks:u64,
    completed:bool,
    success:bool,
    program_versions:usize,
    exec_world_actions:usize,
    gas_refusals:u64,
    other_failures:u64,
    vm_gas:u64,
    world_action_gas:u64,
    initial_setup_gas:u64,
    tool_registered:bool,
    result_persisted:bool,
    last_return:Option<String>,
}
fn one(seed:u64,limit:u64,cap:u64)->Record{
    let program:Program=s6_composition_search();
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:limit,..Config::default()
    },Family::P0,seed);
    let first=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    }));
    assert_eq!(first.code,Code::Ok);
    let second=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.into_bytes()
    }));
    assert_eq!(second.code,Code::Ok);
    let initial_setup_gas=first.gas_charged+second.gas_charged;
    let first_step=world.state.step;
    let mut gas_refusals=0;
    let mut other_failures=0;
    let mut vm_gas=0;
    let mut world_action_gas=0;
    let mut completed=false;
    for _ in 0..cap {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::Gas {gas_refusals+=1;}
        else if !matches!(result.code,Code::Ok|Code::Empty){other_failures+=1;}
        vm_gas+=result.vm_gas_charged;
        world_action_gas+=result.gas_charged;
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|v|v.finished){
            completed=true;break;
        }
        if matches!(result.code,Code::InvalidCode|Code::Denied){break;}
    }
    let organism=&world.state.organisms[&ORGANISM_ID];
    Record{
        seed,a_gas_per_epoch:limit,max_ticks:cap,
        elapsed_ticks:world.state.step-first_step,
        completed,
        success:completed&&organism.last_return==Some(Value::U32(1)),
        program_versions:world.state.objects.keys()
            .filter(|key|key.starts_with("/self/versions/")).count(),
        exec_world_actions:world.audit.iter().filter(|r|matches!(r.action,Some(Action::Exec{..}))).count(),
        gas_refusals,other_failures,vm_gas,world_action_gas,initial_setup_gas,
        tool_registered:world.state.objects.contains_key("/self/tools/transform-v1"),
        result_persisted:world.state.objects.contains_key("/self/results/transform-held-out"),
        last_return:organism.last_return.as_ref().map(|x|format!("{x:?}")),
    }
}
fn main() {
    let mut records=Vec::new();
    // Same public development seeds under both resource configurations.
    for seed in [0,7,16]{
        records.push(one(seed,12_000,6_144));
        records.push(one(seed,100_000,1_536));
    }
    println!("{}",serde_json::to_string_pretty(&records).unwrap());
}
