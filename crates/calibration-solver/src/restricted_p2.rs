//! S15: researcher-side, information-restricted P2 TLV calibration solver in an isolated crate.
//!
//! Only two public HELLO frames and a query-limited service oracle are inputs.
//! Does not access Protocol, Service internals, private tags, opcodes or seed.
//! The TLV envelope shape, length encoding, XOR checksum, finite opcode range,
//! known RemoteEval body, and feedback interpretation ARE researcher priors.
//! This is NOT an in-VM organism and NOT proof of general protocol learning.
#![forbid(unsafe_code)]
// These are the *only* observations allowed at the solver interface.
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Diagnostic { BadBody, Unauthorized, Other }
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum OracleFeedback { Frame(Vec<u8>), Diagnostic(Diagnostic) }
use std::collections::BTreeSet;

#[derive(Clone,Debug)]
struct Field { tag:u8, data:Vec<u8> }

fn xor(bytes:&[u8])->u8 { bytes.iter().fold(0u8,|v,&b|v^b) }

fn tlv(tag:u8, data:&[u8])->Result<Vec<u8>,&'static str> {
    let n=u16::try_from(data.len()).map_err(|_|"field too large")?;
    let mut wire=vec![tag];
    wire.extend_from_slice(&n.to_be_bytes());
    wire.extend_from_slice(data);
    Ok(wire)
}
fn fields(wire:&[u8])->Result<(u8,Vec<Field>),&'static str>{
    if wire.len()<6 || wire.len()>1024 {return Err("invalid P2 frame size");}
    let marker=wire[0];
    let outer_len=u16::from_be_bytes([wire[1],wire[2]]) as usize;
    if wire.len()!=3+outer_len {return Err("outer length mismatch");}
    if wire[3]!=0x10 {return Err("missing structural outer TLV");}
    if wire.len()<6 {return Err("short outer field");}
    let n=u16::from_be_bytes([wire[4],wire[5]]) as usize;
    if n+6!=wire.len() {return Err("inner length mismatch");}
    let mut result=Vec::new();
    let mut seen=BTreeSet::new();
    let mut rest=&wire[6..];
    while !rest.is_empty() {
        if rest.len()<3 {return Err("short inner TLV");}
        let tag=rest[0];
        let length=u16::from_be_bytes([rest[1],rest[2]]) as usize;
        if rest.len()<length+3 {return Err("bad inner TLV length");}
        if !seen.insert(tag) {return Err("duplicate inner tag");}
        result.push(Field{tag,data:rest[3..3+length].to_vec()});
        rest=&rest[3+length..];
    }
    if result.len()!=5 {return Err("not five inner fields");}
    Ok((marker,result))
}
fn value(fields:&[Field],tag:u8)->Result<&[u8],&'static str>{
    fields.iter().find(|f|f.tag==tag).map(|f|f.data.as_slice())
        .ok_or("missing inferred tag")
}
fn check_value(fields:&[Field],check_tag:u8)->bool{
    let Ok(check)=value(fields,check_tag) else{return false;};
    if check.len()!=1{return false;}
    let mut data=Vec::new();
    for f in fields {
        if f.tag!=check_tag {
            let Ok(bytes)=tlv(f.tag,&f.data) else{return false;};
            data.extend(bytes);
        }
    }
    xor(&data)==check[0]
}
fn one_tag_by_len(fields:&[Field],len:usize)->Result<u8,&'static str>{
    let matches=fields.iter().filter(|f|f.data.len()==len).collect::<Vec<_>>();
    if matches.len()!=1{return Err("ambiguous field length");}
    Ok(matches[0].tag)
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct P2ObservedGrammar {
    marker:u8,
    kind_tag:u8,
    opcode_tag:u8,
    id_tag:u8,
    body_tag:u8,
    check_tag:u8,
    order:[u8;5],
    /// Derived from public HELLO only; never used as an oracle answer.
    pub observed_hello_opcode:u8,
}
impl P2ObservedGrammar {
    /// Infer per-seed role tags and physical ordering from a public legal pair.
    /// The HELLO request has an empty body; response has a nonempty body.
    pub fn infer(hello_request:&[u8],hello_response:&[u8])
        ->Result<Self,&'static str>{
        let (marker,request)=fields(hello_request)?;
        let (response_marker,response)=fields(hello_response)?;
        if marker!=response_marker {return Err("marker changed across HELLO pair");}
        let req_tags=request.iter().map(|f|f.tag).collect::<BTreeSet<_>>();
        let resp_tags=response.iter().map(|f|f.tag).collect::<BTreeSet<_>>();
        if req_tags!=resp_tags {return Err("HELLO role tags not stable");}
        let id_tag=one_tag_by_len(&request,4)?;
        let body_tag=one_tag_by_len(&request,0)?;
        if value(&response,body_tag)?.is_empty() {
            return Err("HELLO response body is not distinguishable");
        }
        if value(&request,id_tag)?!=value(&response,id_tag)? {
            return Err("HELLO request id mismatch");
        }
        let check_candidates=request.iter()
            .filter(|f|f.data.len()==1 && check_value(&request,f.tag)
                && check_value(&response,f.tag))
            .map(|f|f.tag).collect::<Vec<_>>();
        if check_candidates.len()!=1 {return Err("ambiguous P2 checksum tag");}
        let check_tag=check_candidates[0];
        let remaining=request.iter().filter(|f|f.data.len()==1
            && f.tag!=check_tag).map(|f|f.tag).collect::<Vec<_>>();
        if remaining.len()!=2 {return Err("ambiguous kind/opcode roles");}
        let kind_candidates=remaining.iter().copied().filter(|&tag|
            value(&request,tag)==Ok(&[0][..]) &&
            value(&response,tag)==Ok(&[1][..])
        ).collect::<Vec<_>>();
        if kind_candidates.len()!=1 {return Err("ambiguous HELLO direction role");}
        let kind_tag=kind_candidates[0];
        let opcode_tag=*remaining.iter().find(|&&v|v!=kind_tag)
            .ok_or("opcode not found")?;
        if value(&request,opcode_tag)?!=value(&response,opcode_tag)? {
            return Err("HELLO opcode changed");
        }
        let observed_hello_opcode=value(&request,opcode_tag)?[0];
        let order=request.iter().map(|f|f.tag).collect::<Vec<_>>();
        let order:[u8;5]=order.try_into().map_err(|_|"invalid field count")?;
        let model=Self{marker,kind_tag,opcode_tag,id_tag,body_tag,check_tag,
            order,observed_hello_opcode};
        // The inferred model must re-create the public request without knowing
        // its private generator seed or any Rust Protocol field.
        let id=u32::from_le_bytes(value(&request,id_tag)?.try_into()
            .map_err(|_|"invalid request id")?);
        if model.request(observed_hello_opcode,id,&[])?!=hello_request {
            return Err("inferred schema cannot reproduce public HELLO request");
        }
        Ok(model)
    }

    pub fn request(&self,opcode:u8,id:u32,body:&[u8])
        ->Result<Vec<u8>,&'static str>{
        if body.len()>996{return Err("body exceeds bounded service input");}
        let mut items=[
            Field{tag:self.kind_tag,data:vec![0]},
            Field{tag:self.opcode_tag,data:vec![opcode]},
            Field{tag:self.id_tag,data:id.to_le_bytes().to_vec()},
            Field{tag:self.body_tag,data:body.to_vec()},
            Field{tag:self.check_tag,data:vec![0]},
        ];
        let mut check_data=Vec::new();
        for item in items.iter().take(4) {
            check_data.extend(tlv(item.tag,&item.data)?);
        }
        items[4].data[0]=xor(&check_data);
        let mut children=Vec::new();
        for tag in self.order {
            let field=items.iter().find(|f|f.tag==tag).ok_or("missing role")?;
            children.extend(tlv(field.tag,&field.data)?);
        }
        let outer=tlv(0x10,&children)?;
        let mut wire=vec![self.marker];
        let outer_len=u16::try_from(outer.len()).map_err(|_|"oversize outer TLV")?;
        wire.extend(outer_len.to_be_bytes());
        wire.extend(outer);
        if wire.len()>1024{return Err("frame exceeds service limit");}
        Ok(wire)
    }

    /// Returns None only for the documented service-pending status.
    pub fn result_value(&self,wire:&[u8],opcode:u8,id:u32)
        ->Result<Option<u32>,&'static str>{
        let (marker,parsed)=fields(wire)?;
        if marker!=self.marker{return Err("unexpected response marker");}
        if !check_value(&parsed,self.check_tag){return Err("invalid response checksum");}
        if value(&parsed,self.kind_tag)?!=[1] {return Err("not response");}
        if value(&parsed,self.opcode_tag)?!=[opcode] {
            return Err("unexpected response opcode");
        }
        if value(&parsed,self.id_tag)?!=id.to_le_bytes() {
            return Err("request id mismatch");
        }
        let body=value(&parsed,self.body_tag)?;
        match body {
            [1] => Ok(None),
            [0,a,b,c,d] => Ok(Some(u32::from_le_bytes([*a,*b,*c,*d]))),
            _=>Err("unexpected result response body"),
        }
    }
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct P2ProbeReport {
    pub query_count:usize,
    pub remote_opcode:u8,
    pub result_opcode:u8,
    pub result:u32,
}

/// Discover two hidden operation values via visible diagnostics, then actually
/// run a 3-u32 job and retrieve its RESULT with at most three result polls.
/// The oracle owns its world/service; it exposes no hidden configuration.
pub fn solve_p2<F>(grammar:&P2ObservedGrammar,request_id:u32,values:[u32;3],
    mut oracle:F)->Result<P2ProbeReport,&'static str>
where F:FnMut(&[u8])->OracleFeedback {
    let mut calls=0usize;
    let mut eval=None;
    let mut result=None;
    for code in 0x10..=0x7f {
        let bytes=grammar.request(code,request_id,&[])?;
        let answer=oracle(&bytes);
        calls+=1;
        match answer {
            OracleFeedback::Diagnostic(Diagnostic::BadBody)=>{
                if eval.replace(code).is_some(){return Err("multiple eval candidates");}
            },
            OracleFeedback::Diagnostic(Diagnostic::Unauthorized)=>{
                if result.replace(code).is_some(){return Err("multiple result candidates");}
            },
            _=>{},
        }
    }
    let eval=eval.ok_or("no evaluable operation")?;
    let result=result.ok_or("no RESULT operation")?;
    if eval==grammar.observed_hello_opcode || eval==result {
        return Err("invalid inferred opcode assignment");
    }
    let mut body=3u16.to_le_bytes().to_vec();
    for v in values {body.extend(v.to_le_bytes());}
    let request=grammar.request(eval,request_id,&body)?;
    let response=oracle(&request);
    calls+=1;
    if !matches!(response,OracleFeedback::Frame(_)) {
        return Err("remote eval was not actually billed and accepted");
    }
    let result_request=grammar.request(result,request_id,&[])?;
    for _ in 0..3 {
        let response=oracle(&result_request);
        calls+=1;
        if let OracleFeedback::Frame(bytes)=response {
            if let Some(value)=grammar.result_value(&bytes,result,request_id)? {
                return Ok(P2ProbeReport{
                    query_count:calls,remote_opcode:eval,
                    result_opcode:result,result:value,
                });
            }
        }else{return Err("result request not recognized");}
    }
    Err("result not ready in bounded polls")
}
