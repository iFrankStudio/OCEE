//! S26B: same initial organism bytecode/hash in P0, P1, P2.
/// No host protocol family is passed as an argument to the active VM.
use ocee_organism_seeds::s26_family::s26_public_family_router;
use ocee_organism_vm::Value;
use ocee_protocol_sim::Family;
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};

fn trial(family:Family,seed:u64)->(World,String){
    let program=s26_public_family_router();
    let h=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },family,seed);
    for (key,bytes) in [
        (format!("/self/versions/{h}"),program.canonical_bytes()),
        ("/self/active".into(),h.clone().into_bytes()),
    ] {
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    for _ in 0..64 {
        let res=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(res.code,Code::InvalidCode|Code::Denied),
            "S26B {:?} seed={} step={} got {:?}",family,seed,world.state.step,res.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|v|v.finished){break;}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|v|v.finished),"router must finish within 64 ticks");
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    (world,h)
}
#[test]
fn one_frozen_vm_public_hello_router_identifies_p1_p2_but_not_p0(){
    let mut hashes=std::collections::BTreeSet::new();
    for seed in [7,42,99]{
        for (family,label) in [
            (Family::P0,None),(Family::P1,Some(b"P1".as_slice())),
            (Family::P2,Some(b"P2".as_slice())),
        ] {
            let (world,hash)=trial(family,seed);
            hashes.insert(hash);
            let observed=world.state.objects.get("/self/discoveries/framing-family")
                .map(|o|o.bytes.as_slice());
            assert_eq!(observed,label,"seed={seed} family={family:?}");
            assert_eq!(world.state.organisms[&ORGANISM_ID].last_return,
                Some(Value::U32(u32::from(label.is_some()))));
            assert_eq!(world.state.service_events.iter().map(|x|x.b_gas_charged)
                .sum::<u64>(),0,"classification must not invoke B");
            assert!(!world.audit.iter().any(|e|matches!(&e.action,
                Some(Action::Send{..})|Some(Action::Exec{..}))),
                "classification only reads public evidence and writes to self");
            let gas:u64=world.audit.iter().map(|e|
                e.result.gas_charged+e.result.vm_gas_charged).sum();
            println!("S26B seed={seed} family={family:?} saved={label:?} A_gas={gas} ticks={} B=0",
                world.state.step);
        }
    }
    assert_eq!(hashes.len(),1,"same frozen initial VM across all families");
}
