//! A *researcher-side* bounded probe, given only public HELLO samples and
//! a restricted request/response oracle. This is NOT the initial S organism.
use ocee_protocol_sim::{Family, ParseError, Service, ServiceOutput, ServiceResult};

#[derive(Clone, Copy)]
enum Format { MarkerLengthXor, EscapedDelimiter }
fn xor(b:&[u8])->u8 {b.iter().fold(0u8,|v,&x|v^x)}

fn unescape(frame:&[u8])->Option<Vec<u8>> {
    if frame.len()<2 || frame.first()!=Some(&0x7e) || frame.last()!=Some(&0x7e) {return None;}
    let mut inner=vec![];
    let mut i=1;
    while i<frame.len()-1 {
        let b=frame[i];
        if b==0x7d {
            i+=1;
            inner.push(match frame.get(i).copied()? {
                0x5e=>0x7e,0x5d=>0x7d,_=>return None,
            });
        }else{inner.push(b);}
        i+=1;
    }
    Some(inner)
}
fn construct(sample:&[u8],opcode:u8,request_id:u32,body:&[u8])->Option<Vec<u8>>{
    let mut payload=request_id.to_le_bytes().to_vec();
    payload.extend_from_slice(body);
    if payload.len()>1000 {return None;}
    let format=if sample.first()==Some(&0x7e) {
        Format::EscapedDelimiter
    }else{Format::MarkerLengthXor};
    match format {
        Format::MarkerLengthXor=>{
            if sample.len()<10 {return None;}
            let mut bytes=vec![sample[0],0,opcode];
            bytes.extend_from_slice(&(payload.len() as u16).to_le_bytes());
            bytes.extend(payload);
            let check=xor(&bytes);
            bytes.push(check);
            Some(bytes)
        },
        Format::EscapedDelimiter=>{
            let source=unescape(sample)?;
            if source.len()<9 {return None;}
            // xor(inner-with-check) = seeded check key, derivable from a legal frame.
            let check_key=xor(&source);
            let mut inner=vec![0,opcode];
            inner.extend_from_slice(&(payload.len() as u16).to_be_bytes());
            inner.extend(payload);
            let check=xor(&inner)^check_key;
            inner.push(check);
            let mut wire=vec![0x7e];
            for v in inner {
                match v{
                    0x7e=>wire.extend([0x7d,0x5e]),
                    0x7d=>wire.extend([0x7d,0x5d]),
                    _=>wire.push(v),
                }
            }
            wire.push(0x7e);
            Some(wire)
        }
    }
}

/// Probe all finite opcode candidates using only oracle diagnostics.
/// MAX 112 opcode trials + 1 actual bounded eval. No hidden protocol instance
/// or seed reaches this function, but framing templates are seeded knowledge.
fn find_eval<F: FnMut(&[u8])->ServiceResult>(public_hello:&[u8],mut oracle:F)
    -> Option<(Vec<u8>,usize)> {
    let mut calls=0;
    for opcode in 0x10..=0x7f {
        let attempt=construct(public_hello,opcode,17,&[])?;
        let output=oracle(&attempt);
        calls+=1;
        if output.output!=ServiceOutput::Diagnostic(ParseError::BadBody) {continue;}
        // A malformed-but-parsed remote eval has been identified. The solver
        // can now test a valid workload, without knowing the private opcode.
        let mut body=3u16.to_le_bytes().to_vec();
        for value in [1u32,2,3] {body.extend_from_slice(&value.to_le_bytes());}
        let valid=construct(public_hello,opcode,17,&body)?;
        let result=oracle(&valid);
        calls+=1;
        if result.b_gas_charged>0 && matches!(result.output,ServiceOutput::Frame(_)) {
            return Some((valid,calls));
        }
    }
    None
}

#[test]
fn p0_p1_have_finite_restricted_opcode_discovery_paths() {
    for family in [Family::P0,Family::P1] {
        for seed in [1u64,2,42,2026] {
            let mut service=Service::new(family,seed);
            let (sample,_)=service.public_hello_sample(0);
            let mut current=0u64;
            let (candidate,calls)=find_eval(&sample,|request|{
                current+=1;
                service.handle(1,current,request,50_000)
            }).expect("finite public-feedback discovery path");
            assert!((2..=113).contains(&calls));
            let repeated=service.handle(1,current+1,&candidate,0);
            assert_eq!(repeated.b_gas_charged,0); // idempotent valid request
        }
    }
}
