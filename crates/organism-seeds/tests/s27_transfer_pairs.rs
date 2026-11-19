//! S27 exploratory paired *real-world* transfer control using one identical
//! S26C initial bundle, not two different host-selected specialist programs.
//!
//! P2->P2 is a SAME-FAMILY positive control; P1->P2 carries a correctly
//! namespaced, valid P1 tool and is a CROSS-FAMILY negative control.
//! Neither outcome establishes novel syntax invention or E001 H3.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s26_dispatch::s26_shared_bundle_router;
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

const TARGET_SEED:u64=128;
const PROTOCOL_SEED:u64=42;

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,expected_version:None,bytes
    })).code,Code::Ok,"all initial / import WRITE costs are real");
}
fn gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn bytes(w:&World)->u64{
    w.audit.iter().map(|e|e.result.bandwidth_charged).sum()
}
fn boot(family:Family,seed:u64,protocol_seed:u64)->(World,String,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let h1=p1.content_hash().unwrap();
    let h2=p2.content_hash().unwrap();
    let root=s26_shared_bundle_router(&h1,&h2);
    let h=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,protocol_seed);
    let baseline=w.state_hash(); // independent of subsequent inheritance
    for (hash,program) in [(&h1,&p1),(&h2,&p2),(&h,&root)]{
        put(&mut w,format!("/self/versions/{hash}"),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),h.clone().into_bytes());
    (w,h,baseline)
}
fn finish(w:&mut World){
    for _ in 0..16_384{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "S27 VM fault step={} code={:?} block={:?}",
            w.state.step,r.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .map(|v|v.block.as_str()));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished),"bounded target must finish");
    assert_eq!(w.state_hash(),World::from_snapshot(&w.snapshot_json())
        .unwrap().state_hash(),"exact deterministic snapshot");
}
fn verify_b(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)),"no false success");
    assert_eq!(b_gas(w),20,"exactly one real B evaluation");
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let key=match family {
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!(),
    };
    let got=u32::from_le_bytes(w.state.objects[key].bytes.as_slice().try_into().unwrap());
    let expected=job.values.iter().enumerate().fold(0u32,|sum,(i,&value)|
        sum.wrapping_add(value.rotate_left((i%32) as u32)));
    assert_eq!(got,expected,"external FOLD32 oracle only after completion");
}
fn own_tool(w:&World,path:&str)->(String,Program){
    let hash=String::from_utf8(w.state.objects[path].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{hash}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),hash);
    (hash,p)
}
fn import(w:&mut World,hash:&str,program:&Program,registry:&str)->u64{
    assert!(!w.state.objects.contains_key(registry));
    let prev=gas(w);
    put(w,format!("/self/versions/{hash}"),program.canonical_bytes());
    put(w,registry.into(),hash.as_bytes().to_vec());
    gas(w)-prev
}
fn exec_seen(w:&World,hash:&str)->bool{
    w.audit.iter().any(|e|matches!(
        &e.action,Some(Action::Exec{program_hash,..}) if program_hash==hash
    ))
}
fn csv_record(pair:&str,arm:&str,success:bool,w:&World,tool_used:bool,seed_hash:&str)->String{
    format!("{pair},{arm},P1,P2,{},{},{},{},{},{},{}\n",
        u8::from(success),gas(w),b_gas(w),bytes(w),w.state.step,
        u8::from(tool_used),seed_hash)
}
#[test]
fn matched_bundle_paired_transfer_has_real_positive_and_negative_controls(){
    // Real source episodes: root autonomously selects its preinstalled
    // specialist, which creates the exported tool via VM RewriteOp/WRITE.
    let (mut source_p1,root_p1,_)=boot(Family::P1,42,42);
    let (mut source_p2,root_p2,_)=boot(Family::P2,42,42);
    assert_eq!(root_p1,root_p2,"identical sources have identical seed bundle");
    finish(&mut source_p1);
    finish(&mut source_p2);
    verify_b(&source_p1,Family::P1);
    verify_b(&source_p2,Family::P2);
    let (p1_hash,p1_program)=own_tool(&source_p1,"/self/tools/p1-eval-frame");
    let (p2_hash,p2_program)=own_tool(&source_p2,"/self/tools/p2-eval-frame");
    assert!(!source_p1.state.objects.contains_key("/self/tools/p2-eval-frame"));
    assert_ne!(p1_hash,p2_hash);

    // Same exact root and experts, same target seed/protocol/job, same
    // initial permissions; ONLY imported VM-written tools differ.
    let (mut cold,root_c,seed_c)=boot(Family::P2,TARGET_SEED,PROTOCOL_SEED);
    let (mut foreign,root_f,seed_f)=boot(Family::P2,TARGET_SEED,PROTOCOL_SEED);
    let (mut same,root_s,seed_s)=boot(Family::P2,TARGET_SEED,PROTOCOL_SEED);
    assert_eq!(root_c,root_p1);
    assert_eq!(root_c,root_f);
    assert_eq!(root_c,root_s);
    assert_eq!(seed_c,seed_f);
    assert_eq!(seed_c,seed_s);
    let foreign_cost=import(&mut foreign,&p1_hash,&p1_program,
        "/self/tools/p1-eval-frame");
    let same_cost=import(&mut same,&p2_hash,&p2_program,
        "/self/tools/p2-eval-frame");
    assert!(foreign_cost>0 && same_cost>0);

    // The target VM is not host-selected after boot and all groups have
    // been charged the complete three-program initial bundle.
    finish(&mut cold);
    finish(&mut foreign);
    finish(&mut same);
    for w in [&cold,&foreign,&same] {
        verify_b(w,Family::P2);
        assert_eq!(w.state.objects["/self/active"].version,1);
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),
            Some(s23_p2_remote_tool_transfer().content_hash().unwrap().as_str()));
        assert_eq!(w.state.objects["/public/E001/job0"].bytes,
            cold.state.objects["/public/E001/job0"].bytes);
    }

    // Source P1 knowledge stored under its correct semantic registry is
    // not silently reinterpreted as a P2 executable. It yields no gain.
    assert!(!exec_seen(&foreign,&p1_hash));
    assert_eq!(own_tool(&foreign,"/self/tools/p1-eval-frame").0,p1_hash);
    assert_eq!(own_tool(&foreign,"/self/tools/p2-eval-frame").0,
        own_tool(&cold,"/self/tools/p2-eval-frame").0);
    assert!(gas(&foreign)>gas(&cold),
        "paid but unused P1 history must not be reported as positive transfer");

    // Positive control: the P2-generated executable really is EXEC'ed
    // and reused on different job bytes. Remove that tool and the paired
    // counterfactual falls back to the cold full discovery path.
    assert_ne!(source_p2.state.objects["/public/E001/job0"].bytes,
        same.state.objects["/public/E001/job0"].bytes);
    assert!(exec_seen(&same,&p2_hash));
    assert_eq!(own_tool(&same,"/self/tools/p2-eval-frame").0,p2_hash);
    assert!(!same.state.objects.contains_key("/self/candidates/p2-eval-hash"));
    assert!(gas(&same)<gas(&cold),
        "compatible tool must save metered A gas even after paid import");
    assert!(same.state.step<cold.state.step);
    assert!(cold.state.objects.contains_key("/self/candidates/p2-eval-hash"),
        "without inheritance the VM must learn and test a cold candidate");
    assert_eq!(cold.state.objects["/self/tools/p2-eval-frame"].version,0);
    // The cold VM may independently generate an *identical content hash*
    // under the unchanged P2 protocol seed; hash equality alone is not
    // evidence of inheritance. The candidate and audit trajectory distinguish it.

    let csv=format!("pair_id,arm,source_family,target_family,success,a_gas,b_gas,bytes_sent,ticks,causal_tool_used,seed_hash\n{}{}",
        csv_record("s27-p1-to-p2-world128","C",true,&cold,false,&seed_c),
        csv_record("s27-p1-to-p2-world128","D",true,&foreign,
            exec_seen(&foreign,&p1_hash),&seed_f));
    if let Ok(path)=std::env::var("OCEE_S27_CSV"){
        let path=std::path::Path::new(&path);
        if let Some(parent)=path.parent(){
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path,csv.as_bytes()).unwrap();
    }
    println!("S27 root={root_c} sourceP1={p1_hash} sourceP2={p2_hash}");
    println!("S27 p1_to_p2 cold_A={} D_A={} import_A={} delta_A={} exec_donor={} target_B=20",
        gas(&cold),gas(&foreign),foreign_cost,
        gas(&foreign) as i64-gas(&cold) as i64,exec_seen(&foreign,&p1_hash));
    println!("S27 p2_to_p2 cold_A={} D_A={} import_A={} saved_A={} cold_ticks={} D_ticks={} causal_exec={} B_each=20",
        gas(&cold),gas(&same),same_cost,gas(&cold)-gas(&same),
        cold.state.step,same.state.step,exec_seen(&same,&p2_hash));
    println!("S27 measured_cross_family_csv:\n{csv}");
}
