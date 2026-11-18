//! S17: one unlabelled finite-family selector and one valid-frame feedback
//! strategy. P0/P1/P2 generator knowledge exists ONLY in the test harness.
//! The isolated reference algorithm receives neither hidden family nor B gas.
use ocee_calibration_solver::restricted_families::{
    ObservedEnvelope,solve_frame_only
};
use ocee_calibration_solver::restricted_p2::{OracleFeedback,Diagnostic};
use ocee_protocol_sim::{Family,Service,ServiceOutput};
use ocee_world_kernel::{
    Action,Code,Config,HelloSample,Message,NodeId,
    TransportDiagnostic,World,ORGANISM_ID
};

fn expected(values:[u32;3])->u32{
    values.into_iter().enumerate().fold(0u32,|acc,(i,v)|
        acc.wrapping_add(v.rotate_left(i as u32)))
}
fn blind(output:ServiceOutput)->OracleFeedback {
    match output {
        ServiceOutput::Frame(frame)=>OracleFeedback::Frame(frame),
        ServiceOutput::Diagnostic(_)=>
            OracleFeedback::Diagnostic(Diagnostic::Other),
    }
}
#[test]
fn unlabelled_public_examples_identify_all_three_predeclared_grammars(){
    for family in [Family::P0,Family::P1,Family::P2] {
        for seed in [0u64,1,7,42,99,2026] {
            let mut service=Service::new(family,seed);
            let (request,response)=service.public_hello_sample(0);
            let model=ObservedEnvelope::infer(&request,&response).unwrap();
            let label=match family {
                Family::P0=>"LengthXor",
                Family::P1=>"EscapedDelimiter",
                Family::P2=>"NestedTlv",
            };
            assert_eq!(model.hypothesis_name(),label);
            let mut b_gas=0;
            let mut step=1;
            let values=[0x7e7d_1234,seed as u32,0xdead_beef];
            let report=solve_frame_only(&model,17,values,|wire|{
                let response=service.handle(1,step,wire,50_000);
                step+=1;
                b_gas+=response.b_gas_charged;
                blind(response.output)
            }).unwrap();
            assert_eq!(report.result,expected(values));
            assert!(report.query_count<=226);
            assert_eq!(b_gas,20);
            println!("S17 {label} seed={seed} queries={} B_gas={b_gas}",
                report.query_count);
        }
    }
}
#[test]
fn unlabelled_three_family_reference_succeeds_over_metered_world_messages(){
    for family in [Family::P0,Family::P1,Family::P2] {
        for seed in [7u64,42] {
            let mut world=World::with_protocol(Config{
                seed,a_gas_per_epoch:12_000,..Config::default()
            },family,seed);
            let observed=world.tick(Some(Action::Read{
                actor:ORGANISM_ID,key:"/public/C/hello".into()
            }));
            assert_eq!(observed.code,Code::Ok);
            let samples:HelloSample=serde_json::from_slice(&observed.payload).unwrap();
            let model=ObservedEnvelope::infer(&samples.request,&samples.response).unwrap();
            let values=[0x7e7d_1234,seed as u32,0xdead_beef];
            let report=solve_frame_only(&model,17,values,|request|{
                let sent=world.tick(Some(Action::Send{
                    actor:ORGANISM_ID,destination:NodeId::C,
                    bytes:request.to_vec()
                }));
                assert_eq!(sent.code,Code::Ok);
                for _ in 0..5 {world.tick(None);}
                let received=world.tick(Some(Action::Receive{
                    actor:ORGANISM_ID,max_count:1
                }));
                assert_eq!(received.code,Code::Ok);
                let letters:Vec<Message>=serde_json::from_slice(&received.payload).unwrap();
                assert_eq!(letters.len(),1);
                if serde_json::from_slice::<TransportDiagnostic>(
                    &letters[0].bytes
                ).is_ok(){
                    OracleFeedback::Diagnostic(Diagnostic::Other)
                }else{
                    OracleFeedback::Frame(letters[0].bytes.clone())
                }
            }).expect("same reference strategy works within world quotas");
            assert_eq!(report.result,expected(values));
            let b_gas:u64=world.state.service_events.iter().map(|e|e.b_gas_charged).sum();
            let a_gas:u64=world.audit.iter().map(|e|
                e.result.gas_charged+e.result.vm_gas_charged).sum();
            let tx_bytes:usize=world.audit.iter().filter_map(|e|
                match &e.action {
                    Some(Action::Send{bytes,..})=>Some(bytes.len()),_=>None,
                }).sum();
            assert_eq!(b_gas,20);
            assert!(a_gas<12_000);
            assert_eq!(world.state_hash(),
                World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
            println!("S17 {} seed={seed} queries={} A_gas={a_gas} B_gas={b_gas} tx_bytes={tx_bytes} ticks={}",
                model.hypothesis_name(),report.query_count,world.state.step);
        }
    }
}
