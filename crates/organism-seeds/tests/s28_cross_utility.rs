//! S28 exploratory cross-family *functional* tool transfer with frozen
//! identical active program bundles. The generic operation and RewriteOp
//! candidate are researcher-seeded, NOT organism-invented grammar.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s28_cross_utility::{
    s28_cross_family_root,s28_size_template,UTILITY_REGISTRY
};
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    })).code,Code::Ok);
}
fn gas(w:&World)->u64{
    w.audit.iter().map(|r|r.result.gas_charged+r.result.vm_gas_charged).sum()
}
fn b_gas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn boot(family:Family,world_seed:u64,protocol_seed:u64)->(World,String){
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let root=s28_cross_family_root(&p1.content_hash().unwrap(),
        &p2.content_hash().unwrap());
    let h=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed:world_seed,a_gas_per_epoch:12_000,..Config::default()
    },family,protocol_seed);
    for p in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",p.content_hash().unwrap()),
            p.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),h.clone().into_bytes());
    (w,h)
}
fn run(w:&mut World){
    for _ in 0..16_384{
        let result=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::Denied|Code::InvalidCode),
            "S28 unexpected VM trap step={} code={:?} block={:?}",
            w.state.step,result.code,
            w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .map(|c|c.block.as_str()));
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded VM must finish");
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn stored(w:&World)->(String,Program){
    let h=String::from_utf8(w.state.objects[UTILITY_REGISTRY].bytes.clone())
        .unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{h}")].bytes).unwrap();
    assert_eq!(p.content_hash().unwrap(),h);
    (h,p)
}
fn audit_exec(w:&World,h:&str)->bool{
    let switch=w.audit.iter().position(|r|matches!(&r.action,
        Some(Action::Write{key,..}) if key=="/self/active" &&
            r.step>=4
    )).unwrap_or(w.audit.len());
    w.audit[..switch].iter().any(|r|r.result.code==Code::Ok
        && matches!(&r.action,
            Some(Action::Exec{program_hash,..}) if program_hash==h))
}
fn verify_remote(w:&World,family:Family){
    if w.state.organisms[&ORGANISM_ID].last_return!=Some(Value::U32(1)){
        let host=&w.state.organisms[&ORGANISM_ID];
        let recent=w.audit.iter().rev().take(9).map(|r|{
            let action=match &r.action{
                Some(Action::Write{key,..})=>format!("WRITE {key}"),
                Some(Action::Read{key,..})=>format!("READ {key}"),
                Some(Action::Exec{program_hash,..})=>
                    format!("EXEC {program_hash}"),
                Some(Action::Send{..})=>"SEND".into(),
                Some(Action::Receive{..})=>"RECEIVE".into(),
                Some(Action::Observe{..})=>"OBSERVE".into(),
                // S36 adds a development-only query action. Include it in
                // diagnostic formatting, even when this S28 world never
                // issues a probe. No S28 behavioral change is intended.
                Some(Action::Probe{input,..})=>format!("PROBE {} bytes",input.len()),
                Some(Action::VerifyProbe{evidence,..})=>
                    format!("VERIFY PROBE {} bytes",evidence.len()),
                Some(Action::RenewProbe{evidence,..})=>
                    format!("RENEW PROBE {} bytes",evidence.len()),
                None=>"VM-only".into(),
            };
            format!("step={} {action} code={:?} vm_gas={}",
                r.step,r.result.code,r.result.vm_gas_charged)
        }).collect::<Vec<_>>();
        panic!("S28 remote failure family={family:?} step={} A={} B={} return={:?} active={:?} block={:?} tools={:?} recent={recent:?}",
            w.state.step,gas(w),b_gas(w),host.last_return,
            host.active_program_hash,
            host.vm_continuation.as_ref().map(|c|c.block.as_str()),
            w.state.objects.keys().filter(|k|k.starts_with("/self/tools/"))
                .collect::<Vec<_>>());
    }
    assert_eq!(b_gas(w),20,"real C->B FOLD32, not a local fake");
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let expected=job.values.iter().enumerate().fold(0u32,|s,(i,&v)|
        s.wrapping_add(v.rotate_left((i%32) as u32)));
    let key=match family {
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!()
    };
    let got=u32::from_le_bytes(w.state.objects[key].bytes.as_slice()
        .try_into().unwrap());
    assert_eq!(got,expected);
}
fn import(w:&mut World,hash:&str,program:&Program,registry:&str)->u64{
    let before=gas(w);
    put(w,format!("/self/versions/{hash}"),program.canonical_bytes());
    put(w,registry.into(),hash.as_bytes().to_vec());
    gas(w)-before
}
fn generated_in_active_vm(w:&World,hash:&str)->bool{
    let key=format!("/self/versions/{hash}");
    let first_active=w.audit.iter().position(|r|matches!(&r.action,
        Some(Action::Read{key,..}) if key=="/public/E001/job0"
    )).unwrap_or(usize::MAX);
    w.audit.iter().enumerate().any(|(i,r)|i>first_active &&
        r.result.code==Code::Ok && matches!(&r.action,
            Some(Action::Write{key:k,..}) if k==&key))
}
#[test]
fn p1_generated_generic_program_executes_in_p2_and_controls_real_b_work(){
    // Source P1 root dynamically creates the tool using RewriteOp and
    // consumes its output before switching to the P1 specialist.
    let (mut donor,source_root)=boot(Family::P1,42,42);
    run(&mut donor);
    verify_remote(&donor,Family::P1);
    let (tool_hash,tool_program)=stored(&donor);
    assert_ne!(tool_hash,s28_size_template().content_hash().unwrap());
    assert!(generated_in_active_vm(&donor,&tool_hash));
    assert!(audit_exec(&donor,&tool_hash));
    assert_eq!(donor.state.objects[UTILITY_REGISTRY].version,0);

    // Same frozen initial root + both specialists in all P2 targets.
    let (mut cold,hc)=boot(Family::P2,128,42);
    let (mut carried,hd)=boot(Family::P2,128,42);
    let (mut sham,hs)=boot(Family::P2,128,42);
    let (mut poison,hp)=boot(Family::P2,128,42);
    assert_eq!(source_root,hc);
    assert_eq!((hc.clone(),hc.clone(),hc.clone()),(hd,hs,hp));
    let before_hash=cold.state_hash();
    assert_eq!(before_hash,carried.state_hash());
    assert_eq!(before_hash,sham.state_hash());
    assert_eq!(before_hash,poison.state_hash());

    let d_import=import(&mut carried,&tool_hash,&tool_program,
        UTILITY_REGISTRY);
    // Exact same copied executable and import cost, different one-character
    // registry key: tool present in storage but not discoverable by policy.
    let sham_import=import(&mut sham,&tool_hash,&tool_program,
        "/self/tools/shared-payload-lex");
    assert_eq!(d_import,sham_import);
    assert_eq!(carried.state.step,sham.state.step);
    assert_eq!(carried.state.nodes,sham.state.nodes);
    assert_eq!(carried.state.organisms,sham.state.organisms);
    assert_eq!(carried.state.objects.len(),sham.state.objects.len());
    // The dependency control uses a *valid* but wrong executable; it must
    // EXEC successfully, return 0, and fail the guarded dispatch before B.
    let wrong=s28_size_template();
    let wrong_hash=wrong.content_hash().unwrap();
    let poison_cost=import(&mut poison,&wrong_hash,&wrong,UTILITY_REGISTRY);
    assert!(poison_cost>0);

    run(&mut cold);
    run(&mut carried);
    run(&mut sham);
    run(&mut poison);
    for w in [&cold,&carried,&sham]{
        verify_remote(w,Family::P2);
        assert_eq!(w.state.objects["/self/active"].version,1);
        assert_eq!(w.state.objects["/public/E001/job0"].bytes,
            cold.state.objects["/public/E001/job0"].bytes);
    }
    for w in [&cold,&carried,&sham]{
        assert!(audit_exec(w,&tool_hash),
            "the common executable must be actually EXEC'ed before B");
    }
    assert!(generated_in_active_vm(&cold,&tool_hash),
        "no-import C must construct its own validated candidate");
    assert!(!generated_in_active_vm(&carried,&tool_hash),
        "D must reuse imported version, not recreate it");
    // The sham VM must *attempt* the same graph rewrite. Because the
    // invisible import already stored that content hash as immutable code,
    // the valid version WRITE returns Conflict, not Ok. It then registers
    // its own freshly computed hash and EXECs this already-stored version.
    let shared_key=format!("/self/versions/{tool_hash}");
    assert!(sham.audit.iter().any(|r|r.result.code==Code::Conflict
        && matches!(&r.action,
            Some(Action::Write{key,..}) if key==&shared_key)),
        "sham must independently regenerate an identical hash");
    assert!(!generated_in_active_vm(&sham,&tool_hash),
        "content address already exists; its WRITE is not a new version");
    assert_eq!(stored(&carried).0,tool_hash);
    assert_eq!(stored(&cold).0,tool_hash);
    assert!(gas(&carried)<gas(&sham),
        "same import schedule, only registry visibility differs");

    assert_eq!(poison.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)),"wrong but valid utility blocks dispatch");
    assert_eq!(b_gas(&poison),0);
    assert_eq!(poison.state.objects["/self/active"].version,0);
    assert!(audit_exec(&poison,&wrong_hash));
    println!("S28 shared_tool={} root={source_root} source=P1 target=P2",tool_hash);
    println!("S28 P2 cold_A={} imported_A={} sham_A={} poison_A={} imported_cost={} sham_cost={} poison_cost={} D_minus_sham={} cold_ticks={} D_ticks={} sham_ticks={} B_C_D_sham=20 B_poison=0",
        gas(&cold),gas(&carried),gas(&sham),gas(&poison),
        d_import,sham_import,poison_cost,
        gas(&carried) as i64-gas(&sham) as i64,
        cold.state.step,carried.state.step,sham.state.step);
}
