//! S8 lifecycle resource accounting on development seeds, not E001 scoring.
//! Runs S7 and S8 with the original A gas quota and emits auditable JSON.
use ocee_organism_seeds::s7_recover::s7_empty_input_recovery;
use ocee_organism_seeds::s8_repair_search::{
    s8_two_candidate_repair,s8_one_repair_candidate_control
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,NodeId,World,ORGANISM_ID};
use serde_json::json;

fn measure(label:&str,seed:u64,program:Program){
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P0,seed);
    let hash=program.content_hash().unwrap();
    let initial=program.canonical_bytes();
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:initial
    })).code,Code::Ok);
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    })).code,Code::Ok);
    let mut complete=false;
    for _ in 0..4096 {
        w.tick_autonomous(ORGANISM_ID);
        if w.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {
            complete=true;
            break;
        }
    }
    assert!(complete,"{label} seed={seed} failed to finish");
    let actor=w.state.organisms.get(&ORGANISM_ID).unwrap();
    let mut gas=0u64;
    let mut vm_gas=0u64;
    let mut gas_denials=0usize;
    let mut invalid_code=0usize;
    let mut exec_calls=0usize;
    let mut writes=0usize;
    let mut candidate_writes=0usize;
    for record in &w.audit {
        gas+=record.result.gas_charged;
        vm_gas+=record.result.vm_gas_charged;
        if record.result.code==Code::Gas {gas_denials+=1;}
        if record.result.code==Code::InvalidCode {invalid_code+=1;}
        if let Some(action)=&record.action {
            match action {
                Action::Exec{actor,..} if *actor==ORGANISM_ID=>exec_calls+=1,
                Action::Write{actor,key,..} if *actor==ORGANISM_ID=>{
                    writes+=1;
                    if key.starts_with("/self/versions/"){candidate_writes+=1;}
                },
                _=>{}
            }
        }
    }
    let storage=actor.storage_used;
    let capacity=w.state.nodes[&NodeId::A].storage_capacity;
    assert!(storage<=capacity,"storage overrun");
    let repaired=w.state.objects.get("/self/tools/transform-v1")
        .is_some_and(|o|o.version==1);
    let ret=match &actor.last_return{
        Some(Value::U32(n))=>*n,
        _=>panic!("missing return for {label} seed={seed}")
    };
    if label=="S8" || label=="S7" {assert_eq!(ret,2);}
    if label=="S8-one-candidate" {assert_eq!(ret,0);}
    if label=="S8-no-fault" {assert_eq!(ret,1);}
    println!("{}",json!({
        "study":"OCEE-E001-DEV-S8-LIFECYCLE",
        "case":label,"seed":seed,"a_gas_per_epoch":12_000,
        "ticks_total_including_installation":w.state.step,
        "world_action_gas_charged":gas,
        "vm_gas_charged":vm_gas,
        "gas_charged_total":gas+vm_gas,
        "gas_denials":gas_denials,
        "invalid_code_results":invalid_code,
        "exec_calls":exec_calls,
        "writes":writes,
        "program_version_writes_including_initial":candidate_writes,
        "storage_used_bytes":storage,
        "storage_capacity_bytes":capacity,
        "final_tool_repaired":repaired,
        "last_return":ret
    }));
}
fn main(){
    for seed in [2,5,16] {
        measure("S8",seed,s8_two_candidate_repair());
    }
    measure("S7",2,s7_empty_input_recovery());
    measure("S8-one-candidate",2,s8_one_repair_candidate_control());
    measure("S8-no-fault",0,s8_two_candidate_repair());
}
