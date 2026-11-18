//! S15 restricted P2 *world transport* calibration, researcher-side only.
//! The solver sees the public READ of HELLO and frame/diagnostic outputs from
//! SEND -> delayed C -> RECEIVE. It never sees protocol private parameters,
//! world object maps, audit data, gas counters, or the target answer.
//! The harness audits those privileged facts only after the trial.
use ocee_protocol_sim::{Family,ParseError};
use ocee_calibration_solver::restricted_p2::{
    P2ObservedGrammar,solve_p2,solve_p2_frame_only,OracleFeedback,Diagnostic
};
use ocee_world_kernel::{
    Action,Code,Config,HelloSample,Message,NodeId,TransportDiagnostic,World,ORGANISM_ID
};

fn independent_reference(values:[u32;3])->u32 {
    values.into_iter().enumerate().fold(0u32,|acc,(i,value)|
        acc.wrapping_add(value.rotate_left(i as u32)))
}

#[test]
fn observed_p2_service_is_learnable_with_real_world_messages_and_a_quota(){
    for seed in [7u64,42] {
        let mut world=World::with_protocol(Config{
            seed,a_gas_per_epoch:12_000,..Config::default()
        },Family::P2,seed);
        let sample_result=world.tick(Some(Action::Read{
            actor:ORGANISM_ID,key:"/public/C/hello".into()
        }));
        assert_eq!(sample_result.code,Code::Ok);
        let sample:HelloSample=serde_json::from_slice(&sample_result.payload).unwrap();
        let grammar=P2ObservedGrammar::infer(&sample.request,&sample.response)
            .expect("learn P2 role tags from public world READ only");
        let values=[0x7e7d_1234,seed as u32,0xdead_beef];
        // This closure is a transport adapter, NOT a private Service oracle.
        // Each trial sends an authorized message and waits for C's reply.
        let report=solve_p2(&grammar,17,values,|wire|{
            let sent=world.tick(Some(Action::Send{
                actor:ORGANISM_ID,destination:NodeId::C,bytes:wire.to_vec()
            }));
            assert_eq!(sent.code,Code::Ok,"SEND denied or exhausted resource budget");
            for _ in 0..5 {world.tick(None);}
            let received=world.tick(Some(Action::Receive{
                actor:ORGANISM_ID,max_count:1
            }));
            assert_eq!(received.code,Code::Ok,"expected C response after bounded delay");
            let mail:Vec<Message>=serde_json::from_slice(&received.payload).unwrap();
            assert_eq!(mail.len(),1);
            if let Ok(diagnostic)=serde_json::from_slice::<TransportDiagnostic>(&mail[0].bytes){
                let public_code=match diagnostic.code {
                    ParseError::BadBody=>Diagnostic::BadBody,
                    ParseError::Unauthorized=>Diagnostic::Unauthorized,
                    _=>Diagnostic::Other,
                };
                OracleFeedback::Diagnostic(public_code)
            }else{
                OracleFeedback::Frame(mail[0].bytes.clone())
            }
        }).expect("P2 bounded learning through actual SEND/RECEIVE");
        assert_eq!(report.result,independent_reference(values));
        assert!(report.query_count<=116);
        // Privileged audit is performed OUTSIDE solve_p2 and is not returned
        // to the information-restricted reference solver.
        let b_bill:u64=world.state.service_events.iter()
            .map(|e|e.b_gas_charged).sum();
        assert_eq!(b_bill,20,"one B FOLD32 evaluation must be charged");
        let a_bill:u64=world.audit.iter().map(|event|
            event.result.gas_charged + event.result.vm_gas_charged).sum();
        assert!(a_bill<12_000,"calibration must complete within one A epoch");
        assert_eq!(world.state_hash(),
            World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
        println!("S15-P2 world transport seed={seed} oracle_queries={} world_ticks={} A_gas={} B_gas={}",
            report.query_count,world.state.step,a_bill,b_bill);
    }
}


#[test]
fn s16_diagnostic_blind_reference_solver_still_uses_metered_c_to_b_transport(){
    // Same world authorization and quotas as S15, but deliberately suppress
    // EVERY parsing/operation error label while retaining legal reply frames.
    for seed in [7u64,42] {
        let mut world=World::with_protocol(Config{
            seed,a_gas_per_epoch:12_000,..Config::default()
        },Family::P2,seed);
        let public=world.tick(Some(Action::Read{
            actor:ORGANISM_ID,key:"/public/C/hello".into()
        }));
        assert_eq!(public.code,Code::Ok);
        let pair:HelloSample=serde_json::from_slice(&public.payload).unwrap();
        let grammar=P2ObservedGrammar::infer(&pair.request,&pair.response).unwrap();
        let values=[0x7e7d_1234,seed as u32,0xdead_beef];
        let mut blinded=0usize;
        let outcome=solve_p2_frame_only(&grammar,17,values,|wire|{
            let sent=world.tick(Some(Action::Send{
                actor:ORGANISM_ID,destination:NodeId::C,bytes:wire.to_vec()
            }));
            assert_eq!(sent.code,Code::Ok);
            for _ in 0..5{world.tick(None);}
            let received=world.tick(Some(Action::Receive{
                actor:ORGANISM_ID,max_count:1
            }));
            assert_eq!(received.code,Code::Ok);
            let mail:Vec<Message>=serde_json::from_slice(&received.payload).unwrap();
            assert_eq!(mail.len(),1);
            if serde_json::from_slice::<TransportDiagnostic>(&mail[0].bytes).is_ok(){
                blinded+=1;
                // The diagnostic code is *never* passed to S16's solver.
                OracleFeedback::Diagnostic(Diagnostic::Other)
            }else{
                OracleFeedback::Frame(mail[0].bytes.clone())
            }
        }).expect("P2 still learnable from valid frames over A -> C transport");
        assert!(blinded>0);
        assert!(outcome.query_count<=226);
        assert_eq!(outcome.result,independent_reference(values));
        let a_bill:u64=world.audit.iter().map(|item|
            item.result.gas_charged+item.result.vm_gas_charged).sum();
        let b_bill:u64=world.state.service_events.iter()
            .map(|e|e.b_gas_charged).sum();
        assert!(a_bill<12_000);
        assert_eq!(b_bill,20);
        assert_eq!(world.state_hash(),
            World::from_snapshot(&world.snapshot_json()).unwrap().state_hash());
        println!("S16 P2 BLIND seed={seed} queries={} masked={} ticks={} A_gas={} B_gas={}",
            outcome.query_count,blinded,world.state.step,a_bill,b_bill);
    }
}
