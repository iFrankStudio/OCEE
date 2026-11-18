//! S21A: test-only privileged protocol comparisons never reach VM inputs.
//! One ACTIVE VM reads only /public/C/hello and infers P2 tags/order.
use ocee_organism_seeds::s21_p2_tlv::{
    s21_p2_infer_public_hello_roles,s21_p2_request_only_ablation,s21_manifest
};
use ocee_organism_vm::Value;
use ocee_protocol_sim::{Family,Decoded,FrameKind,Operation};
use ocee_world_kernel::{Action,Code,Config,World,ORGANISM_ID};
use std::collections::{BTreeMap,BTreeSet};

fn run(seed:u64,ablate:bool)->World{
    let program=if ablate{s21_p2_request_only_ablation()}
        else{s21_p2_infer_public_hello_roles()};
    let hash=program.content_hash().unwrap();
    let mut world=World::with_protocol(Config{
        seed,a_gas_per_epoch:12_000,..Config::default()
    },Family::P2,seed);
    for (key,bytes) in [
        (format!("/self/versions/{hash}"),program.canonical_bytes()),
        ("/self/active".to_owned(),hash.into_bytes()),
    ]{
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key,expected_version:None,bytes,
        })).code,Code::Ok);
    }
    for _ in 0..256{
        let result=world.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(result.code,Code::InvalidCode|Code::Denied),
            "seed={seed} VM failure at {} = {:?}",world.state.step,result.code);
        if world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break}
    }
    assert!(world.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished));
    assert_eq!(world.state_hash(),
        World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
    world
}
fn fields(wire:&[u8])->Vec<(u8,Vec<u8>)>{
    assert_eq!(wire.len(),3+u16::from_be_bytes([wire[1],wire[2]]) as usize);
    assert_eq!(wire[3],0x10);
    let inner_len=u16::from_be_bytes([wire[4],wire[5]]) as usize;
    assert_eq!(wire.len(),6+inner_len);
    let mut bytes=&wire[6..];
    let mut rows=Vec::new();
    while !bytes.is_empty(){
        let tag=bytes[0];
        let len=u16::from_be_bytes([bytes[1],bytes[2]]) as usize;
        assert!(bytes.len()>=3+len);
        rows.push((tag,bytes[3..3+len].to_vec()));
        bytes=&bytes[3+len..];
    }
    rows
}
#[test]
fn active_vm_derives_all_five_role_tags_and_order_from_public_p2_pair(){
    assert_eq!(s21_manifest().program_hash,
        format!("sha256:{}",s21_p2_infer_public_hello_roles()
            .content_hash().unwrap()));
    let mut orders=BTreeSet::<Vec<u8>>::new();
    let mut role_sets=BTreeSet::<Vec<u8>>::new();
    for seed in [7u64,42,99,272,335,1001] {
        let w=run(seed,false);
        assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
            Some(Value::U32(1)),"seed={seed}");
        let map=&w.state.objects["/self/discoveries/p2-tlv-roles"].bytes;
        assert_eq!(map.len(),11);
        let (marker,kind,op,id,body,check)=
            (map[0],map[1],map[2],map[3],map[4],map[5]);
        assert_eq!(map[6..].iter().copied().collect::<BTreeSet<_>>(),
            [kind,op,id,body,check].into_iter().collect());
        let service=w.state.protocol_service.as_ref().unwrap();
        let (request,response)=service.public_hello_sample(0);
        assert_eq!(request[0],marker);
        let req=fields(&request);
        let resp=fields(&response);
        assert_eq!(req.iter().map(|(tag,_)|*tag).collect::<Vec<_>>(),
            map[6..].to_vec(),"preserve physical field order");
        let req=BTreeMap::<u8,Vec<u8>>::from_iter(req);
        let resp=BTreeMap::<u8,Vec<u8>>::from_iter(resp);
        assert_eq!(req[&kind],vec![0]);
        assert_eq!(resp[&kind],vec![1]);
        assert_eq!(req[&op],resp[&op]);
        assert_eq!(req[&id],vec![0,0,0,0]);
        assert_eq!(resp[&id],vec![0,0,0,0]);
        assert_eq!(req[&body],Vec::<u8>::new());
        assert_eq!(resp[&body],vec![0,1]);
        assert_eq!(req[&check].len(),1);
        let private_test_only=service.protocol.encode(&Decoded{
            kind:FrameKind::Request,operation:Operation::RemoteEval,
            request_id:0x1234_5678,body:vec![3,0,9,8,7,6],
        }).unwrap();
        let later=BTreeMap::<u8,Vec<u8>>::from_iter(fields(&private_test_only));
        assert_eq!(later[&kind],vec![0]);
        assert_eq!(later[&id],0x1234_5678u32.to_le_bytes());
        assert_eq!(later[&body],vec![3,0,9,8,7,6]);
        assert_ne!(later[&op],req[&op],"role is opcode, not checksum");
        assert_ne!(later[&check],req[&check],"new body changes check");
        assert_eq!(w.state.service_events.len(),0,
            "tag learning must not secretly use C/B runtime oracle");
        let vm_gas:u64=w.audit.iter().map(|a|a.result.vm_gas_charged).sum();
        assert!(vm_gas>0);
        let total:u64=w.audit.iter().map(|a|
            a.result.vm_gas_charged+a.result.gas_charged).sum();
        println!("S21A P2 seed={seed} roles={:02x?} order={:02x?} ticks={} VM_gas={vm_gas} A_gas={total} B_gas=0",
            &map[1..6],&map[6..],w.state.step);
        orders.insert(map[6..].to_vec());
        role_sets.insert(map[1..6].to_vec());
    }
    assert!(orders.len()>=3,"seed-dependent TLV orders must actually differ");
    assert!(role_sets.len()>=3,"must infer different hidden tag mappings");
}
#[test]
fn missing_public_response_cannot_infer_roles(){
    let w=run(42,true);
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(0)));
    assert!(!w.state.objects.contains_key("/self/discoveries/p2-tlv-roles"));
    assert!(w.state.service_events.is_empty());
}
