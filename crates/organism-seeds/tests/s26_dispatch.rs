//! S26C: identical initial program bundle; active VM chooses which
//! researcher-preseeded specialist to run from public paired HELLO evidence.
//! No host per-family tool selection after charged installation.
use ocee_organism_seeds::s19_p1_tool::s19_p1_build_and_use_tool;
use ocee_organism_seeds::s23_p2_transfer::s23_p2_remote_tool_transfer;
use ocee_organism_seeds::s26_dispatch::s26_shared_bundle_router;
use ocee_organism_vm::{Program,Value};
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,WorkItem,ORGANISM_ID};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,expected_version:None,bytes,
    })).code,Code::Ok);
}
fn boot(family:Family,seed:u64)->(World,String,String,String,usize) {
    let p1=s19_p1_build_and_use_tool();
    let p2=s23_p2_remote_tool_transfer();
    let p1h=p1.content_hash().unwrap();
    let p2h=p2.content_hash().unwrap();
    let root=s26_shared_bundle_router(&p1h,&p2h);
    let rh=root.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,seed);
    // Both family specialists and the same root are charged and installed
    // in BOTH worlds, so the study cannot secretly grant one world its
    // correct specialized source code for free.
    for (h,p) in [(&p1h,&p1),(&p2h,&p2),(&rh,&root)]{
        put(&mut world,format!("/self/versions/{h}"),p.canonical_bytes());
    }
    put(&mut world,"/self/active".into(),rh.clone().into_bytes());
    let start=world.audit.len();
    (world,rh,p1h,p2h,start)
}
fn checked_result(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
    assert_eq!(w.state.service_events.iter().map(|e|e.b_gas_charged)
        .sum::<u64>(),20,"one genuine remote B computation");
    let job:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    let independent=job.values.iter().enumerate().fold(0u32,|sum,(j,&v)|
        sum.wrapping_add(v.rotate_left((j%32) as u32)));
    let key=match family {
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!(),
    };
    let observed=&w.state.objects[key].bytes;
    assert_eq!(u32::from_le_bytes(observed.as_slice().try_into().unwrap()),independent);
}
#[test]
fn one_frozen_public_entry_dispatches_to_genuinely_charged_p1_and_p2_b_tasks(){
    let mut root_hashes=std::collections::BTreeSet::new();
    let mut library_hashes=std::collections::BTreeSet::new();
    for family in [Family::P1,Family::P2] {
        let (mut w,root,p1,p2,start)=boot(family,42);
        root_hashes.insert(root.clone());
        library_hashes.insert((p1.clone(),p2.clone()));
        let expected=if family==Family::P1 {&p1}else{&p2};
        let label=if family==Family::P1 {b"P1".as_slice()}else{b"P2".as_slice()};
        let mut finished=false;
        for _ in 0..16_384 {
            let r=w.tick_autonomous(ORGANISM_ID);
            assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
                "S26C family={family:?} step={} code={:?}",
                w.state.step,r.code);
            if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
                .is_some_and(|c|c.finished) {
                finished=true;
                break;
            }
        }
        assert!(finished,"specialist must terminate with bounded resources");
        assert_eq!(w.state.organisms[&ORGANISM_ID].active_program_hash.as_deref(),
            Some(expected.as_str()),"VM must switch to the correct seeded specialist");
        assert_eq!(w.state.objects["/self/active"].version,1,
            "one live CAS, not hidden per-world host steering");
        assert_eq!(w.state.objects["/self/discoveries/framing-family"].bytes,label);
        let switches=w.audit[start..].iter().filter(|event|matches!(
            &event.action,Some(Action::Write{key,..}) if key=="/self/active"
        )).count();
        assert_eq!(switches,1,"only one autonomous active-program switch");
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
        checked_result(&w,family);
        let a_gas:u64=w.audit.iter().map(|e|
            e.result.gas_charged+e.result.vm_gas_charged).sum();
        println!("S26C family={family:?} root={root} A_gas={a_gas} ticks={} B_gas=20 active={expected}",
            w.state.step);
    }
    assert_eq!(root_hashes.len(),1,"same initial active root across worlds");
    assert_eq!(library_hashes.len(),1,"identical preinstalled programs across worlds");
}
