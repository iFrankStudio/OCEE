//! S17: finite grammar-family inference and *frame-only* remote utilization.
//!
//! The reference algorithm does not receive Family/Protocol/Service, the seed,
//! hidden opcodes or privileged B billing. It sees only a public HELLO pair and
//! the same delayed authorized frame-or-erased-diagnostic interface as S16.
//! The finite P0/P1/P2 grammar hypotheses and FOLD32 request/reply shapes are
//! explicit RESEARCHER PRIORS. This is NOT a general protocol inventor and is
//! not runnable OCEE organism code.
#![forbid(unsafe_code)]
use crate::restricted_p2::{
    Diagnostic,OracleFeedback,P2ObservedGrammar,
};

fn xor(bytes:&[u8])->u8 {bytes.iter().fold(0u8,|v,&b|v^b)}

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
enum LinearFlavor { LengthXor, EscapedDelimiter }

#[derive(Clone,Debug)]
struct LinearGrammar {
    flavor:LinearFlavor,
    marker:u8,
    xor_key:u8,
    hello_opcode:u8,
}
#[derive(Clone,Debug)]
struct LinearDecoded {kind:u8,opcode:u8,id:u32,body:Vec<u8>}

fn unescape(wire:&[u8])->Result<Vec<u8>,&'static str>{
    if wire.len()<2 || wire[0]!=0x7e || wire[wire.len()-1]!=0x7e {
        return Err("missing P1 frame boundaries");
    }
    let mut out=Vec::new();
    let mut i=1usize;
    while i<wire.len()-1 {
        match wire[i] {
            0x7e=>return Err("unescaped delimiter"),
            0x7d=>{
                i+=1;
                if i>=wire.len()-1 {return Err("incomplete escape");}
                out.push(match wire[i] {
                    0x5e=>0x7e,0x5d=>0x7d,
                    _=>return Err("unrecognized escape"),
                });
            },
            v=>out.push(v),
        }
        i+=1;
    }
    Ok(out)
}
impl LinearGrammar {
    fn decode(&self,wire:&[u8])->Result<LinearDecoded,&'static str>{
        if wire.len()>1024{return Err("wire too long");}
        let (kind,opcode,id,body)=match self.flavor {
            LinearFlavor::LengthXor=>{
                if wire.len()<10 || wire[0]!=self.marker {
                    return Err("bad P0 marker");
                }
                let n=u16::from_le_bytes([wire[3],wire[4]]) as usize;
                if n<4 || wire.len()!=6+n {return Err("bad P0 payload len");}
                if xor(wire)!=0 {return Err("bad P0 check");}
                let id=u32::from_le_bytes(wire[5..9].try_into().unwrap());
                (wire[1],wire[2],id,wire[9..wire.len()-1].to_vec())
            },
            LinearFlavor::EscapedDelimiter=>{
                let inner=unescape(wire)?;
                if inner.len()<9 {return Err("P1 short inner");}
                let n=u16::from_be_bytes([inner[2],inner[3]]) as usize;
                if n<4 || inner.len()!=5+n {return Err("bad P1 payload len");}
                if xor(&inner)!=self.xor_key {return Err("bad P1 check");}
                let id=u32::from_le_bytes(inner[4..8].try_into().unwrap());
                (inner[0],inner[1],id,inner[8..inner.len()-1].to_vec())
            },
        };
        if kind>1{return Err("bad frame direction");}
        Ok(LinearDecoded{kind,opcode,id,body})
    }
    fn infer(flavor:LinearFlavor,request:&[u8],response:&[u8])
        ->Result<Self,&'static str>{
        if request.is_empty(){return Err("empty public frame");}
        let xor_key=if flavor==LinearFlavor::EscapedDelimiter {
            xor(&unescape(request)?)
        } else {0};
        let blank=Self{flavor,marker:request[0],xor_key,hello_opcode:0};
        let req=blank.decode(request)?;
        let resp=blank.decode(response)?;
        if req.kind!=0 || resp.kind!=1 || req.id!=resp.id
            || req.opcode!=resp.opcode || !req.body.is_empty()
            || resp.body.is_empty()
        {
            return Err("not a matched public HELLO pair");
        }
        let learned=Self{hello_opcode:req.opcode,..blank};
        if learned.request(learned.hello_opcode,req.id,&[])?!=request {
            return Err("HELLO re-encoding failed");
        }
        Ok(learned)
    }

    fn request(&self,opcode:u8,id:u32,body:&[u8])
        ->Result<Vec<u8>,&'static str>{
        let len=body.len().checked_add(4).ok_or("too long")?;
        let n=u16::try_from(len).map_err(|_|"too long")?;
        let mut payload=id.to_le_bytes().to_vec();
        payload.extend_from_slice(body);
        let wire=match self.flavor {
            LinearFlavor::LengthXor=>{
                let mut frame=vec![self.marker,0,opcode];
                frame.extend_from_slice(&n.to_le_bytes());
                frame.extend(payload);
                frame.push(xor(&frame));
                frame
            },
            LinearFlavor::EscapedDelimiter=>{
                let mut inner=vec![0,opcode];
                inner.extend_from_slice(&n.to_be_bytes());
                inner.extend(payload);
                inner.push(xor(&inner)^self.xor_key);
                let mut frame=vec![0x7e];
                for byte in inner {
                    match byte {
                        0x7e=>frame.extend_from_slice(&[0x7d,0x5e]),
                        0x7d=>frame.extend_from_slice(&[0x7d,0x5d]),
                        _=>frame.push(byte),
                    }
                }
                frame.push(0x7e);
                frame
            },
        };
        if wire.len()>1024 {return Err("frame exceeds allowed bytes");}
        Ok(wire)
    }
    fn response_body(&self,wire:&[u8],opcode:u8,id:u32)
        ->Result<Vec<u8>,&'static str> {
        let result=self.decode(wire)?;
        if result.kind!=1 || result.opcode!=opcode || result.id!=id {
            return Err("wrong response identity");
        }
        Ok(result.body)
    }
}

#[derive(Clone,Debug)]
pub enum ObservedEnvelope {
    LengthXor(LinearGrammar),
    EscapedDelimiter(LinearGrammar),
    NestedTlv(P2ObservedGrammar),
}
impl ObservedEnvelope {
    /// Identify one and only one predeclared framing hypothesis. The caller
    /// must NOT reveal which protocol family generated these public bytes.
    pub fn infer(request:&[u8],response:&[u8])->Result<Self,&'static str>{
        let mut candidates=Vec::new();
        if let Ok(known)=LinearGrammar::infer(
            LinearFlavor::LengthXor,request,response
        ){
            candidates.push(Self::LengthXor(known));
        }
        if let Ok(known)=LinearGrammar::infer(
            LinearFlavor::EscapedDelimiter,request,response
        ){
            candidates.push(Self::EscapedDelimiter(known));
        }
        if let Ok(known)=P2ObservedGrammar::infer(request,response){
            candidates.push(Self::NestedTlv(known));
        }
        if candidates.len()!=1 {return Err("zero or ambiguous frame hypotheses");}
        Ok(candidates.remove(0))
    }

    pub fn hypothesis_name(&self)->&'static str{
        match self {
            Self::LengthXor(_)=>"LengthXor",
            Self::EscapedDelimiter(_)=>"EscapedDelimiter",
            Self::NestedTlv(_)=>"NestedTlv",
        }
    }
    fn request(&self,op:u8,id:u32,body:&[u8])
        ->Result<Vec<u8>,&'static str>{
        match self {
            Self::LengthXor(m)|Self::EscapedDelimiter(m)=>m.request(op,id,body),
            Self::NestedTlv(m)=>m.request(op,id,body),
        }
    }
    fn accepted_eval(&self,wire:&[u8],op:u8,id:u32)->bool{
        match self {
            Self::LengthXor(m)|Self::EscapedDelimiter(m)=>
                m.response_body(wire,op,id).is_ok_and(|v|v==[0]),
            Self::NestedTlv(m)=>m.accepted_eval(wire,op,id),
        }
    }
    fn result_value(&self,wire:&[u8],op:u8,id:u32)
        ->Result<Option<u32>,&'static str>{
        match self {
            Self::LengthXor(m)|Self::EscapedDelimiter(m)=>{
                let body=m.response_body(wire,op,id)?;
                match body.as_slice() {
                    [1]=>Ok(None),
                    [0,a,b,c,d]=>Ok(Some(u32::from_le_bytes([*a,*b,*c,*d]))),
                    _=>Err("not a successful FOLD32 RESULT frame"),
                }
            },
            Self::NestedTlv(m)=>m.result_value(wire,op,id),
        }
    }
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct FamilyProbeReport {
    pub query_count:usize,
    pub eval_opcode:u8,
    pub result_opcode:u8,
    pub result:u32,
}

/// S17 finite-family frame-only probe. Errors are ignored, including if
/// their diagnostic labels have been deliberately erased. This is the same
/// high-level strategy across all three different inferred framing grammars.
pub fn solve_frame_only<F>(model:&ObservedEnvelope,request_id:u32,
    values:[u32;3],mut oracle:F)->Result<FamilyProbeReport,&'static str>
where F:FnMut(&[u8])->OracleFeedback {
    let mut calls=0usize;
    let mut body=3u16.to_le_bytes().to_vec();
    for value in values {body.extend_from_slice(&value.to_le_bytes());}

    let mut eval=None;
    for opcode in 0x10..=0x7f {
        let request=model.request(opcode,request_id,&body)?;
        let observed=oracle(&request);
        calls+=1;
        if let OracleFeedback::Frame(wire)=observed {
            if model.accepted_eval(&wire,opcode,request_id) {
                eval=Some(opcode);
                break;
            }
        }
    }
    let eval=eval.ok_or("no accepted REMOTE_EVAL frame")?;
    for opcode in 0x10..=0x7f {
        if opcode==eval {continue;}
        let request=model.request(opcode,request_id,&[])?;
        for attempt in 0..3 {
            let observed=oracle(&request);
            calls+=1;
            match observed {
                OracleFeedback::Frame(wire)=>match model.result_value(
                    &wire,opcode,request_id
                ){
                    Ok(Some(result))=>return Ok(FamilyProbeReport{
                        query_count:calls,eval_opcode:eval,
                        result_opcode:opcode,result,
                    }),
                    Ok(None) if attempt<2=>continue,
                    _=>break,
                },
                OracleFeedback::Diagnostic(_)=>break,
            }
        }
    }
    Err("no completed result frame")
}
