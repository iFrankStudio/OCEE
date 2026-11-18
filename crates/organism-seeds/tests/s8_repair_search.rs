//! S8 bounded in-world repair candidate selection. No E001/M0 claims.
//! Tests deliberate *wrong but executable* first candidate, then a second
//! candidate chosen by real phenotype feedback; no researcher stage handoff.
use ocee_organism_seeds::s7_recover::s7_empty_input_recovery;
use ocee_organism_seeds::s8_repair_search::{
    s8_two_candidate_repair,s8_one_repair_candidate_control,s8_manifest
};
use ocee_organism_vm::{Program,Value,run_isolated};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{
    Action,ActionResult,Code,Config,World,ORGANISM_ID,hidden_compose
};

fn run(seed:u64,gas:u64,program:Program)->World{
    program.validate().expect("initial program must validate");
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:gas,..Config::default()
    },Family::P0,seed);
    let r=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
        expected_version:None,bytes:program.canonical_bytes()
    }));
    assert_eq!(r.code,Code::Ok,"initial S8 program write failed");
    let r=world.tick(Some(Action::Write{
        actor:ORGANISM_ID,key:"/self/active".into(),
        expected_version:None,bytes:hash.as_bytes().to_vec()
    }));
    assert_eq!(r.code,Code::Ok,"initial activation failed");

    for _ in 0..4096 {
        let r=world.tick_autonomous(ORGANISM_ID);
        if r.code==Code::InvalidCode {
            assert!(matches!(world.audit.last().and_then(|e|e.action.as_ref()),
                Some(Action::Exec{..})),
                "only an isolated tool EXEC may fail in a healthy organism");
        }
        assert_ne!(r.code,Code::Denied,"permission error at step {}",world.state.step);
        if world.state.organisms[&ORGANISM_ID].vm_continuation
            .as_ref().is_some_and(|c|c.finished) {return world;}
    }
    panic!("S8 did not terminate within 4096 ticks");
}
fn current_hash(w:&World)->String{
    String::from_utf8(w.state.objects["/self/tools/transform-v1"].bytes.clone()).unwrap()
}
fn version(w:&World,h:&str)->Program{
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{h}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),h);
    p
}
fn trial_versions(w:&World)->Vec<Program>{
    w.state.objects.iter().filter_map(|(key,obj)|{
        if !key.starts_with("/self/versions/"){return None;}
        let p:Program=serde_json::from_slice(&obj.bytes).ok()?;
        if p.functions[0].blocks.len()==3 {Some(p)}else{None}
    }).collect()
}
#[test]
fn bounded_two_candidate_search_rejects_wrong_guard_and_adopts_valid_guard(){
    let seed_program=s8_two_candidate_repair();
    let seed_hash=seed_program.content_hash().unwrap();
    assert_eq!(s8_manifest().program_hash,format!("sha256:{seed_hash}"));
    assert!(s8_manifest().research_status.contains("NOT_M0"));
    // Original A resource limit, rather than enlarged development limit.
    for seed in [2u64,5,16] {
        let w=run(seed,12_000,seed_program.clone());
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(2)),"two-candidate search failed, seed {seed}");
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash,
            Some(seed_hash.clone()),"no researcher stage switch");
        let current=current_hash(&w);
        assert_eq!(w.state.objects["/self/tools/transform-v1"].version,1);
        let proposals=trial_versions(&w);
        assert_eq!(proposals.len(),2,"must materialize two distinct candidate ASTs");
        assert!(proposals.iter().any(|p|
            run_isolated(p,vec![],2000).unwrap().0==Value::U32(1)),
            "first candidate must be executable but semantically wrong");
        assert!(proposals.iter().any(|p|
            run_isolated(p,vec![],2000).unwrap().0==Value::U32(0)),
            "second candidate must compute the expected empty value");
        let adopted=version(&w,&current);
        assert_eq!(run_isolated(&adopted,vec![],2000).unwrap().0,Value::U32(0));
        let first_hash=w.audit.iter().filter_map(|entry|{
            if let Some(Action::Write{key,bytes,..})=&entry.action {
                if key=="/self/tools/transform-v1"{
                    return Some(String::from_utf8(bytes.clone()).unwrap());
                }
            }
            None
        }).next().unwrap();
        assert_ne!(current,first_hash);
        assert!(w.audit.iter().any(|entry|matches!(
            &entry.action,Some(Action::Exec{program_hash,input,..})
                if input.is_empty() && program_hash==&first_hash
                && entry.result.code==Code::InvalidCode
        )));
        let observed:ActionResult=serde_json::from_slice(
            &w.state.objects["/self/repair/empty/probe"].bytes).unwrap();
        assert_eq!(observed.code,Code::InvalidCode);
        let unseen:[&[u8];3]=[&[99,0,13],&[1,2,3,4,5,6],&[200,7,41,17,9]];
        for input in unseen {
            assert_eq!(run_isolated(&adopted,input.to_vec(),2000).unwrap().0,
                Value::U32(hidden_compose(seed,input)));
        }
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }
}
#[test]
fn disabling_second_repair_candidate_leaves_original_broken_tool(){
    let w=run(2,12_000,s8_one_repair_candidate_control());
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert_eq!(w.state.objects["/self/tools/transform-v1"].version,0);
    assert_eq!(trial_versions(&w).len(),1);
    assert_eq!(run_isolated(&trial_versions(&w)[0],vec![],2000).unwrap().0,
        Value::U32(1));
    assert_eq!(w.state.objects.contains_key("/self/repair/empty/probe"),true);
}
#[test]
fn existing_correct_tool_and_s7_single_guard_work_under_original_resource_limit(){
    let no_fault=run(0,12_000,s8_two_candidate_repair());
    assert_eq!(no_fault.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(no_fault.state.objects["/self/tools/transform-v1"].version,0);
    assert!(trial_versions(&no_fault).is_empty());

    // S7 feasibility: same initial S7 program, standard 12k A gas/epoch.
    let legacy=run(2,12_000,s7_empty_input_recovery());
    assert_eq!(legacy.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(2)),"S7 failed at E001 original A budget");
}
