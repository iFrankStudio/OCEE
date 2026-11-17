//! Deterministic protocol families and the bounded C -> B reference service.
//! Protocol definitions are research-side ONLY: NEVER seed them into organism memory.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const GENERATOR_VERSION: u32 = 1;
pub const MAX_PAYLOAD: usize = 1024;
const OPCODES: [Operation;4] = [
    Operation::Hello, Operation::Caps, Operation::RemoteEval, Operation::Result,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Family { P0, P1, P2 }
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Operation { Hello, Caps, RemoteEval, Result }
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameKind { Request, Response }
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ParseError {
    BadFrame, BadLength, BadEscape, BadCheck, BadField, UnknownOperation, BadBody,
    Unauthorized, ServiceBusy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Decoded {
    pub kind: FrameKind, pub operation: Operation, pub request_id: u32, pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Protocol {
    pub generator_version: u32,
    pub family: Family,
    marker: u8,
    xor_key: u8,
    opcodes: [u8;4],
    // In P2, tags are mapped to kind, opcode, request_id, body, checksum.
    tags: [u8;5],
    // P2 fields occur in a seed-determined order.
    p2_order: [usize;5],
}

fn draw(state:&mut u64)->u64 {
    // Fixed SplitMix64, deterministic across supported Rust targets.
    *state=state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z=*state;
    z=(z^(z>>30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z=(z^(z>>27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z^(z>>31)
}
fn choose(state:&mut u64,count:usize)->usize {
    (draw(state) % count as u64) as usize
}
fn shuffle<T>(state:&mut u64,a:&mut [T]) {
    for i in (1..a.len()).rev() {a.swap(i,choose(state,i+1));}
}
fn xor_bytes(bytes:&[u8])->u8 {bytes.iter().fold(0,|acc,&b|acc^b)}

impl Protocol {
    pub fn generate(family:Family,seed:u64)->Self {
        let domain=match family {Family::P0=>0x5030,Family::P1=>0x5031,Family::P2=>0x5032};
        let mut rng=seed^((GENERATOR_VERSION as u64)<<32)^domain;
        let marker=match family {
            Family::P0 => [0xa1,0xa3,0xa5,0xa7][choose(&mut rng,4)],
            Family::P1 => 0x7e,
            Family::P2 => [0xd1,0xd3,0xd5,0xd7][choose(&mut rng,4)],
        };
        let xor_key=[0x00,0x33,0x55,0xaa][choose(&mut rng,4)];
        let mut values:Vec<u8>=(0x10..=0x7f).collect();
        shuffle(&mut rng,&mut values);
        let opcodes=[values[0],values[1],values[2],values[3]];
        let mut tags=[0x21,0x22,0x23,0x24,0x25];
        shuffle(&mut rng,&mut tags);
        let mut p2_order=[0,1,2,3,4];
        shuffle(&mut rng,&mut p2_order);
        Self{generator_version:GENERATOR_VERSION,family,marker,xor_key,opcodes,tags,p2_order}
    }
    fn opcode(&self,op:Operation)->u8 {
        self.opcodes[OPCODES.iter().position(|o|*o==op).expect("known op")]
    }
    fn operation(&self,value:u8)->Result<Operation,ParseError> {
        OPCODES.iter().copied().zip(self.opcodes).find(|(_,b)|*b==value)
            .map(|(op,_)|op).ok_or(ParseError::UnknownOperation)
    }
    fn kind_byte(kind:FrameKind)->u8 {if kind==FrameKind::Request {0}else{1}}
    fn parse_kind(b:u8)->Result<FrameKind,ParseError> {
        match b {0=>Ok(FrameKind::Request),1=>Ok(FrameKind::Response),_=>Err(ParseError::BadField)}
    }
    pub fn encode(&self,frame:&Decoded)->Result<Vec<u8>,ParseError> {
        if frame.body.len()>MAX_PAYLOAD.saturating_sub(4) {return Err(ParseError::BadLength);}
        let mut result=match self.family {
            Family::P0 | Family::P1 => {
                let mut payload=frame.request_id.to_le_bytes().to_vec();
                payload.extend_from_slice(&frame.body);
                let mut v=vec![Self::kind_byte(frame.kind),self.opcode(frame.operation)];
                if self.family==Family::P0 {
                    v.splice(0..0,[self.marker]);
                    v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
                    v.extend(payload);
                    v.push(xor_bytes(&v));
                    v
                } else {
                    v.extend_from_slice(&(payload.len() as u16).to_be_bytes());
                    v.extend(payload);
                    v.push(xor_bytes(&v)^self.xor_key);
                    let mut wire=vec![0x7e];
                    for x in v {
                        match x {0x7e=>wire.extend([0x7d,0x5e]),0x7d=>wire.extend([0x7d,0x5d]),
                            _=>wire.push(x)}
                    }
                    wire.push(0x7e);
                    wire
                }
            },
            Family::P2 => {
                let values=[
                    vec![Self::kind_byte(frame.kind)],
                    vec![self.opcode(frame.operation)],
                    frame.request_id.to_le_bytes().to_vec(),
                    frame.body.clone(),
                    vec![],
                ];
                let mut parts=vec![Vec::<u8>::new();5];
                let mut check_input=vec![];
                for idx in 0..4 {
                    parts[idx]=tlv(self.tags[idx],&values[idx])?;
                    check_input.extend_from_slice(&parts[idx]);
                }
                parts[4]=tlv(self.tags[4],&[xor_bytes(&check_input)])?;
                let mut children=vec![];
                for &i in &self.p2_order {children.extend_from_slice(&parts[i]);}
                let outer=tlv(0x10,&children)?;
                let mut wire=vec![self.marker];
                wire.extend_from_slice(&(outer.len() as u16).to_be_bytes());
                wire.extend(outer);
                wire
            }
        };
        if result.len()>MAX_PAYLOAD {return Err(ParseError::BadLength);}
        Ok(std::mem::take(&mut result))
    }
    pub fn decode(&self,wire:&[u8])->Result<Decoded,ParseError> {
        if wire.len()>MAX_PAYLOAD {return Err(ParseError::BadLength);}
        let (kind,opcode,request_id,body)=match self.family {
            Family::P0 => {
                if wire.len()<10 || wire[0]!=self.marker {return Err(ParseError::BadFrame);}
                let n=u16::from_le_bytes([wire[3],wire[4]]) as usize;
                if wire.len()!=6+n {return Err(ParseError::BadLength);}
                if xor_bytes(wire)!=0 {return Err(ParseError::BadCheck);}
                if n<4 {return Err(ParseError::BadBody);}
                (wire[1],wire[2],u32::from_le_bytes(wire[5..9].try_into().unwrap()),
                    wire[9..wire.len()-1].to_vec())
            },
            Family::P1 => {
                if wire.len()<2 || wire.first()!=Some(&0x7e) || wire.last()!=Some(&0x7e) {
                    return Err(ParseError::BadFrame);
                }
                let mut inner=vec![];
                let mut i=1;
                while i<wire.len()-1 {
                    let b=wire[i];
                    if b==0x7d {
                        i+=1;
                        if i>=wire.len()-1 {return Err(ParseError::BadEscape);}
                        inner.push(match wire[i] {0x5e=>0x7e,0x5d=>0x7d,
                            _=>return Err(ParseError::BadEscape)});
                    } else if b==0x7e {return Err(ParseError::BadFrame);}
                    else {inner.push(b);}
                    i+=1;
                }
                if inner.len()<9 {return Err(ParseError::BadLength);}
                let n=u16::from_be_bytes([inner[2],inner[3]]) as usize;
                if inner.len()!=5+n {return Err(ParseError::BadLength);}
                if xor_bytes(&inner)!=self.xor_key {return Err(ParseError::BadCheck);}
                if n<4 {return Err(ParseError::BadBody);}
                (inner[0],inner[1],u32::from_le_bytes(inner[4..8].try_into().unwrap()),
                    inner[8..inner.len()-1].to_vec())
            },
            Family::P2 => {
                if wire.len()<6 || wire[0]!=self.marker {return Err(ParseError::BadFrame);}
                let len=u16::from_be_bytes([wire[1],wire[2]]) as usize;
                if wire.len()!=3+len {return Err(ParseError::BadLength);}
                let (tag,outer,remaining)=next_tlv(&wire[3..])?;
                if tag!=0x10 || !remaining.is_empty() {return Err(ParseError::BadField);}
                let mut slots:BTreeMap<u8,Vec<u8>>=BTreeMap::new();
                let mut rest=outer;
                while !rest.is_empty() {
                    let (tag,value,tail)=next_tlv(rest)?;
                    if slots.insert(tag,value.to_vec()).is_some() {return Err(ParseError::BadField);}
                    rest=tail;
                }
                if slots.len()!=5 {return Err(ParseError::BadField);}
                let mut required=Vec::new();
                for t in &self.tags {
                    required.push(slots.get(t).ok_or(ParseError::BadField)?.as_slice());
                }
                if required[0].len()!=1 || required[1].len()!=1 ||
                    required[2].len()!=4 || required[4].len()!=1 {
                    return Err(ParseError::BadField);
                }
                let mut check_input=vec![];
                for i in 0..4 {check_input.extend_from_slice(&tlv(self.tags[i],required[i])?);}
                if xor_bytes(&check_input)!=required[4][0] {return Err(ParseError::BadCheck);}
                (required[0][0],required[1][0],
                    u32::from_le_bytes(required[2].try_into().unwrap()),required[3].to_vec())
            }
        };
        Ok(Decoded{
            kind:Self::parse_kind(kind)?,
            operation:self.operation(opcode)?,
            request_id,body,
        })
    }
}
fn tlv(tag:u8,contents:&[u8])->Result<Vec<u8>,ParseError>{
    let n=u16::try_from(contents.len()).map_err(|_|ParseError::BadLength)?;
    let mut out=vec![tag];
    out.extend_from_slice(&n.to_be_bytes());
    out.extend_from_slice(contents);
    Ok(out)
}
fn next_tlv(bytes:&[u8])->Result<(u8,&[u8],&[u8]),ParseError>{
    if bytes.len()<3 {return Err(ParseError::BadLength);}
    let size=u16::from_be_bytes([bytes[1],bytes[2]]) as usize;
    if size>bytes.len()-3 {return Err(ParseError::BadLength);}
    Ok((bytes[0],&bytes[3..3+size],&bytes[3+size..]))
}

pub fn fold32(values:&[u32])->Result<u32,ParseError> {
    if values.is_empty() || values.len()>64 {return Err(ParseError::BadBody);}
    Ok(values.iter().enumerate().fold(0u32,|acc,(i,&value)|
        acc.wrapping_add(value.rotate_left((i%32) as u32))))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Job { values: Vec<u32>, result: u32, ready_step: u64 }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Service {
    pub protocol: Protocol,
    jobs: BTreeMap<(u64,u32),Job>,
    pub ready_latency_steps: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServiceOutput { Frame(Vec<u8>), Diagnostic(ParseError) }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServiceResult { pub output: ServiceOutput, pub b_gas_charged: u64 }
impl Service {
    pub fn new(family:Family,seed:u64)->Self {
        Self{protocol:Protocol::generate(family,seed),
            jobs:BTreeMap::new(),ready_latency_steps:2}
    }
    /// Public legal HELLO example frames; no REMOTE_EVAL answer or private seed.
    pub fn public_hello_sample(&self,step:u64)->(Vec<u8>,Vec<u8>) {
        let id=(step / 256) as u32;
        let req=Decoded{kind:FrameKind::Request,operation:Operation::Hello,
            request_id:id,body:vec![]};
        let resp=Decoded{kind:FrameKind::Response,operation:Operation::Hello,
            request_id:id,body:vec![0,GENERATOR_VERSION as u8]};
        (self.protocol.encode(&req).expect("small hello"),
            self.protocol.encode(&resp).expect("small hello response"))
    }

    pub fn handle(&mut self,actor:u64,now_step:u64,wire:&[u8],b_gas_available:u64)
        ->ServiceResult {
        let request=match self.protocol.decode(wire) {
            Ok(req) if req.kind==FrameKind::Request=>req,
            Ok(_)=>return self.diagnostic(ParseError::BadField),
            Err(err)=>return self.diagnostic(err),
        };
        let id=request.request_id;
        let (status,body,bill)=match request.operation {
            Operation::Hello => {
                if request.body.len()!=0 && request.body.len()!=4 {
                    return self.diagnostic(ParseError::BadBody);
                }
                (0,vec![GENERATOR_VERSION as u8],0)
            },
            Operation::Caps => {
                if !request.body.is_empty(){return self.diagnostic(ParseError::BadBody);}
                (0,vec![1,64],0) // service 1: FOLD32; maximum count 64
            },
            Operation::RemoteEval => {
                let data=&request.body;
                if data.len()<2 {return self.diagnostic(ParseError::BadBody);}
                let count=u16::from_le_bytes([data[0],data[1]]) as usize;
                if count==0 || count>64 || data.len()!=2+count*4 {
                    return self.diagnostic(ParseError::BadBody);
                }
                let values=data[2..].chunks_exact(4)
                    .map(|v|u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>();
                if let Some(old)=self.jobs.get(&(actor,id)) {
                    if old.values!=values {return self.diagnostic(ParseError::BadBody);}
                    (0,vec![],0)
                } else {
                    let gas=8+4*count as u64;
                    if b_gas_available<gas {
                        return self.diagnostic(ParseError::ServiceBusy);
                    }
                    let result=fold32(&values).expect("validated count");
                    self.jobs.insert((actor,id),Job{
                        values,result,ready_step:now_step.saturating_add(self.ready_latency_steps),
                    });
                    (0,vec![],gas)
                }
            },
            Operation::Result => {
                if !request.body.is_empty(){return self.diagnostic(ParseError::BadBody);}
                let Some(job)=self.jobs.get(&(actor,id)) else {
                    return self.diagnostic(ParseError::Unauthorized);
                };
                if now_step<job.ready_step {(1,vec![],0)}
                else {(0,job.result.to_le_bytes().to_vec(),0)}
            },
        };
        let mut reply=vec![status];
        reply.extend(body);
        let frame=Decoded{kind:FrameKind::Response,operation:request.operation,
            request_id:id,body:reply};
        match self.protocol.encode(&frame) {
            Ok(v)=>ServiceResult{output:ServiceOutput::Frame(v),b_gas_charged:bill},
            Err(e)=>self.diagnostic(e),
        }
    }
    fn diagnostic(&self,err:ParseError)->ServiceResult {
        ServiceResult{output:ServiceOutput::Diagnostic(err),b_gas_charged:0}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(proto:&Protocol,op:Operation,id:u32,body:Vec<u8>)->Vec<u8> {
        proto.encode(&Decoded{kind:FrameKind::Request,operation:op,
            request_id:id,body}).unwrap()
    }
    #[test]
    fn three_family_roundtrips_are_deterministic_and_malformed_frames_rejected() {
        for family in [Family::P0,Family::P1,Family::P2] {
            let p=Protocol::generate(family,0xdead_beef);
            assert_eq!(p,Protocol::generate(family,0xdead_beef));
            let input=Decoded{kind:FrameKind::Request,operation:Operation::Hello,
                request_id:0x7e7d_a1a3,body:vec![0x7e,0x7d,0,0xaa]};
            let raw=p.encode(&input).unwrap();
            assert_eq!(p.decode(&raw),Ok(input));
            let mut bad=raw.clone();
            bad.pop();
            assert!(p.decode(&bad).is_err());
            let mut altered=raw.clone();
            // Invalid marker/frame boundary must always be rejected.
            altered[0]=0x00;
            assert!(p.decode(&altered).is_err());
        }
    }
    #[test]
    fn bounded_fold32_service_and_idempotence() {
        for family in [Family::P0,Family::P1,Family::P2] {
            let mut service=Service::new(family,42);
            let proto=service.protocol.clone();
            let mut payload=3u16.to_le_bytes().to_vec();
            for x in [1u32,2,3] {payload.extend_from_slice(&x.to_le_bytes());}
            let eval=request(&proto,Operation::RemoteEval,8,payload.clone());
            let out=service.handle(1,10,&eval,100);
            assert_eq!(out.b_gas_charged,20);
            assert!(matches!(out.output,ServiceOutput::Frame(_)));
            let replay=service.handle(1,11,&eval,0);
            assert_eq!(replay.b_gas_charged,0);
            let poll=request(&proto,Operation::Result,8,vec![]);
            let pending=service.handle(1,11,&poll,0);
            let complete=service.handle(1,12,&poll,0);
            let ServiceOutput::Frame(v)=pending.output else {panic!("no pending frame")};
            assert_eq!(proto.decode(&v).unwrap().body,vec![1]);
            let ServiceOutput::Frame(v)=complete.output else {panic!("no result frame")};
            let data=proto.decode(&v).unwrap().body;
            assert_eq!(data[0],0);
            assert_eq!(u32::from_le_bytes(data[1..5].try_into().unwrap()),
                1u32.wrapping_add(2u32.rotate_left(1)).wrapping_add(3u32.rotate_left(2)));
            let mut changed=payload;
            changed[2]=99;
            let collision=request(&proto,Operation::RemoteEval,8,changed);
            assert_eq!(service.handle(1,13,&collision,100).output,
                ServiceOutput::Diagnostic(ParseError::BadBody));
        }
    }
}
