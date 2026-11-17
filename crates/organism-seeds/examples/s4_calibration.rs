//! E001 pre-calibration, not a confirmatory experiment.
//! Measures whether the *same* S4 seed runs within the original world budget.
//! The executable always reports both success and failure; it does NOT change
//! the world limits, skip actions, or report a skipped run as a success.
use ocee_organism_seeds::s4_structural::s4_p0_structural_repair;
use ocee_protocol_sim::Family;
use ocee_organism_vm::Program;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Record {
    label:&'static str,
    seed:u64,
    a_gas_per_epoch:u64,
    max_ticks:u64,
    completed:bool,
    elapsed_ticks:u64,
    tool_registered:bool,
    program_versions:usize,
    b_jobs_accepted:usize,
    gas_refusals:u64,
    other_failures:u64,
    vm_gas_charged:u64,
    world_action_gas_charged:u64,
    c_bandwidth_charged:u64,
    last_return:Option<String>,
}

fn calibrate(label:&'static str,gas:u64,max_ticks:u64,seed:u64)->Record {
    let cfg=Config{seed,a_gas_per_epoch:gas,..Config::default()};
    let program:Program=s4_p0_structural_repair();
    let mut world=World::with_protocol(cfg,Family::P0,seed);
    let hash=program.content_hash().expect("valid seed");
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes(),
    })).code,Code::Ok);
    assert_eq!(world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.into_bytes(),
    })).code,Code::Ok);
    let start=world.state.step;
    let mut gas_refusals=0;
    let mut other_failures=0;
    let mut vm_gas=0;
    let mut action_gas=0;
    let mut bandwidth=0;
    let mut finished=false;

    for _ in 0..max_ticks {
        let result=world.tick_autonomous(ORGANISM_ID);
        if result.code==Code::Gas {gas_refusals+=1;}
        else if !matches!(result.code,Code::Ok|Code::Empty) {other_failures+=1;}
        vm_gas+=result.vm_gas_charged;
        action_gas+=result.gas_charged;
        bandwidth+=result.bandwidth_charged;
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|continuation|continuation.finished) {
            finished=true;break;
        }
        if result.code==Code::InvalidCode {break;}
    }
    let host=&world.state.organisms[&ORGANISM_ID];
    Record {
        label,seed,a_gas_per_epoch:gas,max_ticks,
        completed:finished,
        elapsed_ticks:world.state.step-start,
        tool_registered:world.state.objects.contains_key("/self/tools/p0-eval-frame"),
        program_versions:world.state.objects.keys()
            .filter(|key|key.starts_with("/self/versions/")).count(),
        b_jobs_accepted:world.state.service_events.iter()
            .filter(|e|e.b_gas_charged>0).count(),
        gas_refusals,other_failures,vm_gas_charged:vm_gas,
        world_action_gas_charged:action_gas,c_bandwidth_charged:bandwidth,
        last_return:host.last_return.as_ref().map(|v|format!("{v:?}")),
    }
}

fn main(){
    let records=[
        calibrate("E001-default-A-gas",12_000,12_288,7),
        calibrate("development-A-gas",100_000,2_048,7),
    ];
    println!("{}",serde_json::to_string_pretty(&records).expect("JSON record"));
}
