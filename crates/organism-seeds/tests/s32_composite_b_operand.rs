//! S32: directly consumed two-node program output in real RemoteEval.
//!
//! Source P1 actively synthesizes multi-node graph. Same frozen root and
//! specialists in P2; target independently revalidates learned graph,
//! EXECs it on target-specific new bytes and sends returned U32 as FOURTH
//! real FOLD32 operand. No host computation injected into normal worlds.
//! Paired researcher intervention changes only the final 4 bytes of the
//! generated 16-byte object, and both arms complete real B work.
use ocee_organism_seeds::{
    s32_composite_b_operand::{
        s32_composite_operands_root,s32_p1_composite_consumer,
        s32_p2_composite_consumer,REMOTE_VALUES
    },
    s30_composed_transfer::{REGISTRY,OOD_PREDICTION},
    s31_causal_remote::PREPARED_COUNT,
    s28_cross_utility::UTILITY_REGISTRY
};
use ocee_organism_vm::{Op,Program,Value};
use ocee_protocol_sim::{Family,Operation,fold32};
use ocee_world_kernel::{
    Action,Code,Config,NodeId,TransformTask,WorkItem,World,ORGANISM_ID,hidden_compose
};

fn put(w:&mut World,key:String,bytes:Vec<u8>){
    assert_eq!(w.tick(Some(Action::Write{
        actor:ORGANISM_ID,key,bytes,expected_version:None
    })).code,Code::Ok,"all initial and imported writes must be paid");
}
fn gas(w:&World)->u64{
    w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum()
}
fn bgas(w:&World)->u64{
    w.state.service_events.iter().map(|e|e.b_gas_charged).sum()
}
fn programs()->(Program,Program,Program){
    let p1=s32_p1_composite_consumer();
    let p2=s32_p2_composite_consumer();
    let root=s32_composite_operands_root(
        &p1.content_hash().unwrap(),&p2.content_hash().unwrap());
    (p1,p2,root)
}
fn boot(family:Family,seed:u64)->(World,String){
    let (p1,p2,root)=programs();
    let hash=root.content_hash().unwrap();
    let mut w=World::with_protocol(Config{
        seed,a_gas_per_epoch:100_000,..Config::default()
    },family,42);
    for program in [p1,p2,root]{
        put(&mut w,format!("/self/versions/{}",
            program.content_hash().unwrap()),program.canonical_bytes());
    }
    put(&mut w,"/self/active".into(),hash.clone().into_bytes());
    (w,hash)
}
fn run_until_switch(w:&mut World,seed:u64){
    for _ in 0..5_000 {
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "root VM trap seed={seed},step={} code={:?}",w.state.step,r.code);
        if w.state.objects["/self/active"].version==1 {break;}
    }
    assert_eq!(w.state.objects["/self/active"].version,1,
        "root must autonomously dispatch to specialist");
    assert_eq!(w.state.objects[PREPARED_COUNT].bytes,&[4,0],
        "Len of 16-byte composite work item must produce count4");
    let source:TransformTask=serde_json::from_slice(
        &w.state.objects["/public/E001/transform"].bytes).unwrap();
    let predicted:u32=serde_json::from_slice(
        &w.state.objects[OOD_PREDICTION].bytes).unwrap();
    let learned=hidden_compose(seed,&source.held_out_input);
    assert_eq!(predicted,learned,"VM must independently predict learned U32");
    let data=&w.state.objects[REMOTE_VALUES].bytes;
    assert_eq!(data.len(),16,"3 public U32s plus one synthesized U32");
    let original:WorkItem=serde_json::from_slice(
        &w.state.objects["/public/E001/job0"].bytes).unwrap();
    assert_eq!(&data[..12],original.values_le.as_slice());
    assert_eq!(u32::from_le_bytes(data[12..16].try_into().unwrap()),learned);
    assert_eq!(w.state.objects[REMOTE_VALUES].version,0);
    let composed_hash=String::from_utf8(w.state.objects[REGISTRY].bytes.clone())
        .unwrap();
    let program:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{composed_hash}")].bytes).unwrap();
    assert_eq!(program.content_hash().unwrap(),composed_hash);
    let ins=&program.functions[0].blocks[0].instructions;
    assert!(matches!(ins[1].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    assert!(matches!(ins[2].op,Op::Len(0)|Op::XorFold(0)|Op::ByteAt(0,1)));
    assert!(matches!(ins[3].op,Op::Add(2,3)|Op::Xor(2,3)));
    let last_exec=w.audit.iter().rposition(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Exec{program_hash,..})
            if program_hash==&composed_hash)).unwrap();
    let values_write=w.audit.iter().position(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{key,..})
            if key==REMOTE_VALUES)).unwrap();
    assert!(last_exec<values_write,
        "final fourth operand must originate from actual composite EXEC");
    let size_hash=String::from_utf8(w.state.objects[UTILITY_REGISTRY].bytes.clone())
        .unwrap();
    let size_exec=w.audit.iter().position(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Exec{program_hash,..})
            if program_hash==&size_hash)).unwrap();
    let count_write=w.audit.iter().position(|e|e.result.code==Code::Ok &&
        matches!(&e.action,Some(Action::Write{key,..})
            if key==PREPARED_COUNT)).unwrap();
    assert!(values_write<size_exec && size_exec<count_write,
        "assembled operand -> Len EXEC -> count WRITE must be sequential");
    assert_eq!(bgas(w),0,"do not let root see B before external control");
}
fn run_to_finish(w:&mut World){
    for _ in 0..12_000{
        let r=w.tick_autonomous(ORGANISM_ID);
        assert!(!matches!(r.code,Code::Denied|Code::InvalidCode),
            "specialist VM trap at {}={:?}",w.state.step,r.code);
        if w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
            .is_some_and(|c|c.finished){break;}
    }
    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref()
        .is_some_and(|c|c.finished),"bounded world must terminate");
    assert_eq!(w.state_hash(),
        World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
}
fn real_request(w:&World)->Vec<u8>{
    let protocol=&w.state.protocol_service.as_ref().unwrap().protocol;
    let frames=w.audit.iter().filter_map(|a|{
        if let Some(Action::Send{destination:NodeId::C,bytes,..})=&a.action{
            if let Ok(d)=protocol.decode(bytes) {
                if d.operation==Operation::RemoteEval &&
                   d.body.len()==18 &&
                   u16::from_le_bytes(d.body[..2].try_into().unwrap())==4{
                    return Some(d.body);
                }
            }
        }
        None
    }).collect::<Vec<_>>();
    assert!(!frames.is_empty(),"actual C request must contain 4 U32 operands");
    frames.last().unwrap().clone()
}
fn remote_result(w:&World,family:Family)->u32{
    let key=match family{
        Family::P1=>"/self/results/p1-job0/value_le",
        Family::P2=>"/self/results/p2-job0/value_le",
        Family::P0=>unreachable!()
    };
    u32::from_le_bytes(w.state.objects[key].bytes.as_slice().try_into().unwrap())
}
fn verify_success(w:&World,family:Family){
    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,
        Some(Value::U32(1)));
    assert_eq!(bgas(w),24,"four-operand B execution must cost 8+4*4");
    let body=real_request(w);
    assert_eq!(&body[2..],w.state.objects[REMOTE_VALUES].bytes.as_slice());
    let values=body[2..].chunks_exact(4).map(|v|
        u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>();
    assert_eq!(remote_result(w,family),fold32(&values).unwrap(),
        "independent FOLD32 from ACTUAL submitted bytes");
    let switch=w.audit.iter().find(|a|matches!(&a.action,
        Some(Action::Write{key,..}) if key=="/self/active" && a.step>=4))
        .unwrap().step;
    for path in [REMOTE_VALUES,PREPARED_COUNT]{
        assert!(w.audit.iter().any(|a|a.step>switch &&
            a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Read{key,..})
                if key==path)),"expert must READ both data and count objects");
    }
}
fn stored(w:&World)->(String,Program){
    let h=String::from_utf8(w.state.objects[REGISTRY].bytes.clone()).unwrap();
    let p:Program=serde_json::from_slice(
        &w.state.objects[&format!("/self/versions/{h}")].bytes).unwrap();
    (h,p)
}
fn import_composed(w:&mut World,h:&str,p:&Program,registry:&str)->u64{
    let pre=gas(w);
    put(w,format!("/self/versions/{h}"),p.canonical_bytes());
    put(w,registry.into(),h.as_bytes().to_vec());
    gas(w)-pre
}
fn paid_mutate(w:&mut World,mut bytes:Vec<u8>)->u64{
    assert_eq!(bytes.len(),16);
    let pre=gas(w);
    // Reverse exactly one bit of the learned fourth operand. Both arms
    // pay same metered CAS write, same 16-byte payload.
    let ret=w.tick(Some(Action::Write{actor:ORGANISM_ID,
        key:REMOTE_VALUES.into(),bytes:std::mem::take(&mut bytes),
        expected_version:Some(0)}));
    assert_eq!(ret.code,Code::Ok);
    gas(w)-pre
}
#[test]
fn synthesized_composite_operand_is_on_real_p1_p2_b_work_path(){
    let mut donors=Vec::new();
    for (src,dst) in [(1u64,19u64),(10u64,28u64)]{
        let (mut p1,root_hash)=boot(Family::P1,src);
        run_until_switch(&mut p1,src);
        let (hash,program)=stored(&p1);
        assert!(p1.audit.iter().any(|a|a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{hash}"))),
            "source P1 must actually produce a sealed graph candidate");
        let (mut p2,target_root)=boot(Family::P2,dst);
        assert_eq!(target_root,root_hash);
        let imported=import_composed(&mut p2,&hash,&program,REGISTRY);
        run_until_switch(&mut p2,dst);
        assert_eq!(stored(&p2).0,hash);
        assert_ne!(p1.state.objects[OOD_PREDICTION].bytes,
            p2.state.objects[OOD_PREDICTION].bytes,
            "target-specific unseen fourth operand should differ");
        assert!(!p2.audit.iter().any(|a|a.step>5 &&
            a.result.code==Code::Ok &&
            matches!(&a.action,Some(Action::Write{key,..})
                if key==&format!("/self/versions/{hash}"))),
            "P2 must consume P1 tool, not regenerate identical version");
        run_to_finish(&mut p1);run_to_finish(&mut p2);
        verify_success(&p1,Family::P1);
        verify_success(&p2,Family::P2);
        println!("S32 real_causal source=P1{src} target=P2{dst} root={root_hash} composed={hash} import_A={imported} P1_A={} P2_A={} P2_ticks={} B_both=24 fourth_u32={}",
            gas(&p1),gas(&p2),p2.state.step,
            u32::from_le_bytes(p2.state.objects[REMOTE_VALUES].bytes[12..].try_into().unwrap()));
        donors.push((hash,program,root_hash));
    }
    assert_ne!(donors[0].0,donors[1].0);
    assert_eq!(donors[0].2,donors[1].2);

    let (hash,program,_)=&donors[0];
    let (mut control,rc)=boot(Family::P2,19);
    let (mut tampered,rt)=boot(Family::P2,19);
    assert_eq!(rc,rt);
    assert_eq!(control.state_hash(),tampered.state_hash());
    let c=import_composed(&mut control,hash,program,REGISTRY);
    let t=import_composed(&mut tampered,hash,program,REGISTRY);
    assert_eq!(c,t);
    run_until_switch(&mut control,19);
    run_until_switch(&mut tampered,19);
    assert_eq!(control.state_hash(),tampered.state_hash());
    let good=control.state.objects[REMOTE_VALUES].bytes.clone();
    let mut bad=good.clone();bad[12]^=1;
    let charged_good=paid_mutate(&mut control,good);
    let charged_bad=paid_mutate(&mut tampered,bad.clone());
    assert_eq!(charged_good,charged_bad);
    assert_eq!(control.state.step,tampered.state.step);
    assert_eq!(control.state.nodes,tampered.state.nodes);
    assert_eq!(control.state.organisms,tampered.state.organisms);
    run_to_finish(&mut control);
    run_to_finish(&mut tampered);
    verify_success(&control,Family::P2);
    verify_success(&tampered,Family::P2);
    assert_eq!(&real_request(&control)[2..],&control.state.objects[REMOTE_VALUES].bytes);
    assert_eq!(&real_request(&tampered)[2..],bad.as_slice());
    assert_ne!(real_request(&control),real_request(&tampered),
        "different composite-derived word MUST change C wire");
    assert_ne!(remote_result(&control,Family::P2),
        remote_result(&tampered,Family::P2),
        "both successful B tasks must yield different actual answers");
    println!("S32 matched_intervention equal paid Write_A={charged_good} bytes16, B=24 both; control_A={} altered_A={} control_fold={} altered_fold={} on_wire_word_changed",
        gas(&control),gas(&tampered),
        remote_result(&control,Family::P2),
        remote_result(&tampered,Family::P2));

    // Incorrect inherited composite must be revalidated and CAS-replaced
    // before its newly selected replacement is allowed to generate a
    // *real* remote operand. No host-side selection or B answer injected.
    let (wrong_hash,wrong_program,_)=&donors[1];
    let (mut repair,repair_root)=boot(Family::P2,19);
    assert_eq!(repair_root,rc);
    import_composed(&mut repair,wrong_hash,wrong_program,REGISTRY);
    run_until_switch(&mut repair,19);
    assert_eq!(stored(&repair).0,*hash);
    assert_eq!(repair.state.objects[REGISTRY].version,1,
        "wrong inherited semantic tool must be paid-CAS-replaced");
    assert!(repair.audit.iter().any(|a|a.result.code==Code::Ok &&
        matches!(&a.action,Some(Action::Write{
            key,expected_version:Some(0),..
        }) if key==REGISTRY)));
    run_to_finish(&mut repair);
    verify_success(&repair,Family::P2);
    println!("S32 wrong_composition repaired_v1 B24 A={} ticks={} true_operand={} new_hash={}",
        gas(&repair),repair.state.step,
        u32::from_le_bytes(repair.state.objects[REMOTE_VALUES].bytes[12..].try_into().unwrap()),hash);
}
