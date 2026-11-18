//! S15 protocol-level learnability lower-bound, NOT an autonomous OCEE VM.
//! Solver receives public HELLO pair and a metered-count oracle ONLY.
use ocee_protocol_sim::{Family,Service,ServiceOutput,ParseError};
use ocee_calibration_solver::restricted_p2::{
    P2ObservedGrammar,solve_p2,OracleFeedback,Diagnostic
};
fn visible(output:ServiceOutput)->OracleFeedback {
    match output {
        ServiceOutput::Frame(f)=>OracleFeedback::Frame(f),
        ServiceOutput::Diagnostic(ParseError::BadBody)=>
            OracleFeedback::Diagnostic(Diagnostic::BadBody),
        ServiceOutput::Diagnostic(ParseError::Unauthorized)=>
            OracleFeedback::Diagnostic(Diagnostic::Unauthorized),
        _=>OracleFeedback::Diagnostic(Diagnostic::Other),
    }
}

fn independently_fold(values:[u32;3])->u32 {
    values.into_iter().enumerate().fold(0u32,|sum,(i,value)|
        sum.wrapping_add(value.rotate_left(i as u32)))
}
#[test]
fn restricted_p2_public_samples_enable_tag_role_and_opcode_discovery(){
    // Public development seeds, never an E001 held-out/pre-registered set.
    for seed in [0u64,1,2,3,4,5,7,17,42,99,2026,65535] {
        let mut service=Service::new(Family::P2,seed);
        let (hello_req,hello_resp)=service.public_hello_sample(0);
        let grammar=P2ObservedGrammar::infer(&hello_req,&hello_resp)
            .expect("P2 public HELLO should permit bounded structural inference");
        let mut step=1u64;
        let mut audited_b_gas=0u64;
        let values=[0x7d7e_01a3,seed as u32,0xf013_7e55];
        let measured=solve_p2(&grammar,17,values,|wire|{
            let answer=service.handle(1,step,wire,50_000);
            audited_b_gas+=answer.b_gas_charged;
            step+=1; visible(answer.output)
        }).expect("restricted P2 oracle queries should find FOLD32 and RESULT");
        assert!(measured.query_count<=116,"query bound exceeded: {:?}",measured);
        assert_ne!(measured.remote_opcode,grammar.observed_hello_opcode);
        assert_ne!(measured.result_opcode,measured.remote_opcode);
        // Billing is checked by a privileged harness and never passed to the solver.
        assert_eq!(audited_b_gas,20);
        assert_eq!(measured.result,independently_fold(values));
        // Must not be able to win through unbounded repeated service charges:
        // the actual B computation is billed exactly once.
        println!("S15 P2 seed={seed} queries={} remote={:#04x} result={:#04x}",
            measured.query_count,measured.remote_opcode,measured.result_opcode);
    }
}
#[test]
fn damaged_or_unpaired_public_evidence_does_not_yield_a_schema(){
    let service=Service::new(Family::P2,42);
    let (request,response)=service.public_hello_sample(0);
    let mut truncated=request.clone();
    truncated.pop();
    assert!(P2ObservedGrammar::infer(&truncated,&response).is_err());
    let mut mismatch=response.clone();
    mismatch[0]^=0x01;
    assert!(P2ObservedGrammar::infer(&request,&mismatch).is_err());
    // Both frames are needed to distinguish kind from opcode/checksum roles.
    assert!(P2ObservedGrammar::infer(&request,&request).is_err());
}
