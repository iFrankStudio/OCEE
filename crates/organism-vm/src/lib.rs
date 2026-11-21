//! OCEE-0 P2: deterministic, capability-free, typed program VM.
//! The VM emits intents; it never performs a world action or selects a candidate.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PROGRAM_BYTES: usize = 64 * 1024;
pub const MAX_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 1024;
pub const MAX_BLOCKS: usize = 128;
pub const MAX_INSTRUCTIONS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type { Bool, U32, Bytes }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value { Bool(bool), U32(u32), Bytes(Vec<u8>) }
impl Value {
    fn kind(&self) -> Type {
        match self { Self::Bool(_) => Type::Bool, Self::U32(_) => Type::U32, Self::Bytes(_) => Type::Bytes }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldCall { Observe, Read, Write, Send, Receive, Exec, Probe, VerifyProbe, RenewProbe, ArmTaskFence, FenceStatus, RestartRoot, FenceStatusFor, RestartRootFor, RequestOutcomeFor, ConfirmResponseFor }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param { pub id: u32, pub ty: Type }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction { pub id: u32, pub op: Op }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    ConstU32(u32), ConstBool(bool), ConstBytes { hex: String },
    Add(u32,u32), Sub(u32,u32), Xor(u32,u32), And(u32,u32), Mod(u32,u32), Div(u32,u32),
    EqU32(u32,u32), EqBytes(u32,u32),
    Len(u32), ByteAt(u32,u32), Slice(u32,u32,u32), Concat(u32,u32),
    Sha256(u32), HexEncode(u32),
    /// Read a JSON object's named field; the field must be an array of bytes.
    JsonFieldBytes(u32,u32),
    /// Read a JSON object's named string field as UTF-8 bytes.
    JsonFieldString(u32,u32),
    /// Extract a non-negative JSON integer within the U32 range.
    JsonFieldU32(u32,u32),
    /// Return any JSON object's named member as canonical serialized JSON bytes.
    JsonFieldJson(u32,u32),
    /// Generic array indexing: emits canonical JSON bytes for the selected value.
    JsonIndex(u32,u32),
    /// Copy a byte vector while replacing a single byte at the given index.
    ReplaceByte(u32,u32,u32),
    /// Generic byte-vector XOR reduction, not protocol-specific.
    XorFold(u32),
    /// Pack the low 8 bits of U32 as a one-byte vector.
    PackByte(u32),
    /// Pack a bounded integer into two little-endian bytes.
    PackU16Le(u32),
    /// Encode/decode generic two-byte stuffing for reserved marker/escape
    /// with an XOR mask; all four operands are explicit VM values.
    ByteStuffEncode(u32,u32,u32,u32), ByteStuffDecode(u32,u32,u32,u32),
    PackU16Be(u32),
    /// Generic big-endian length-tag-value codecs, not protocol role inference.
    TlvEncode(u32,u32), TlvDecode(u32),
    /// Generic TLV tag search and physical-order permutation; all tags
    /// are organism-supplied data, not special protocol constants.
    TlvLookup(u32,u32), TlvReorder(u32,u32),
    /// Total, metered TLV shape preflight on untrusted bytes. Returns Bool
    /// rather than trapping on truncation, duplicates or unexpected tags.
    /// The expected tag order is supplied by the organism, never by the VM.
    TlvMatchesOrder(u32,u32),
    /// Generic validated graph edit: program JSON, block name, register id, new Op JSON.
    /// It changes executable structure; selection/fitness remain organism decisions.
    RewriteOp(u32,u32,u32,u32),
    /// Prepend a list of arbitrary serialized blocks; full graph revalidated.
    /// This can introduce branches/guards but cannot bypass VM static checks.
    PrependBlocks(u32,u32),
    /// Extend one existing Return block by appended typed instructions
    /// and change its return register. Input is a canonical Program,
    /// block name, JSON Vec<Instruction>, and U32 register to return.
    /// Every intermediate program must pass full static validation; no
    /// privileged action or remote service is exposed by this editor.
    AppendInstructions(u32,u32,u32,u32),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terminator {
    Return { value: u32 },
    Jump { target: String, args: Vec<u32> },
    Branch { condition: u32, if_true: String, true_args: Vec<u32>,
             if_false: String, false_args: Vec<u32> },
    /// A statically bounded loop header; the counter persists in Continuation.
    /// Only cycles passing through bounded headers are admitted by the verifier.
    RepeatBounded { limit: u32, body: String, body_args: Vec<u32>,
                    done: String, done_args: Vec<u32> },
    /// Per-scope local bound AND an aggregate bound across all changing scopes.
    /// Both counters persist across yields and snapshots.
    RepeatScoped { limit: u32, aggregate_limit: u32, scope: u32,
                   body: String, body_args: Vec<u32>,
                   done: String, done_args: Vec<u32> },
    // The target block receives next_args followed by one Bytes feedback parameter.
    Yield { call: WorldCall, args: Vec<u32>, next: String, next_args: Vec<u32> },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub params: Vec<Param>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub args: Vec<Type>,
    pub returns: Type,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub format_version: u32,
    pub entry_function: String,
    pub functions: Vec<Function>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmError {
    InvalidProgram(String), InvalidInput, Arithmetic, Range, Gas,
    OutputTooLarge, SideEffect, MissingFeedback, Finished,
}

fn bad(msg: impl Into<String>) -> VmError { VmError::InvalidProgram(msg.into()) }

fn decode_hex(hex: &str) -> Result<Vec<u8>, VmError> {
    if hex.len() % 2 != 0 || hex.len() > MAX_VALUE_BYTES * 2
        || !hex.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return Err(bad("byte literal must be bounded, even-length lowercase hex"));
    }
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i+2],16)
        .map_err(|_|bad("invalid hex"))).collect()
}
fn canonical_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    // serde_json::Value maps sort object keys unless the preserve_order feature is enabled.
    let val = serde_json::to_value(value).expect("serializable");
    serde_json::to_vec(&val).expect("serializable")
}
fn sha(bytes: &[u8]) -> String {
    let h=Sha256::digest(bytes);
    h.iter().map(|v| format!("{v:02x}")).collect::<String>()
}
impl Program {
    pub fn canonical_bytes(&self) -> Vec<u8> { canonical_bytes(self) }
    pub fn content_hash(&self) -> Result<String,VmError> {
        self.validate()?;
        let mut bytes=b"OCEE-PROGRAM-V1\0".to_vec();
        bytes.extend(self.canonical_bytes());
        Ok(sha(&bytes))
    }
    fn main(&self) -> Result<&Function,VmError> {
        self.functions.iter().find(|f| f.name==self.entry_function)
            .ok_or_else(||bad("missing entry function"))
    }
    pub fn validate(&self) -> Result<(),VmError> {
        if self.format_version != 1 {return Err(bad("unknown program format")); }
        if self.functions.len()!=1 {return Err(bad("P2 supports one function (CALL deferred)")); }
        let f=self.main()?;
        if f.args!=vec![Type::Bytes] {return Err(bad("entry expects one Bytes argument")); }
        if f.blocks.is_empty() || f.blocks.len()>MAX_BLOCKS {return Err(bad("block count")); }
        if self.canonical_bytes().len()>MAX_PROGRAM_BYTES {return Err(bad("program too large")); }
        let mut blocks=BTreeMap::new();
        for b in &f.blocks {
            if blocks.insert(b.id.as_str(),b).is_some() {return Err(bad("duplicate block id")); }
        }
        let entry=f.blocks.first().ok_or_else(||bad("missing entry block"))?;
        if entry.params.len()!=1 || entry.params[0].ty!=Type::Bytes {return Err(bad("entry block signature")); }
        let mut edges:BTreeMap<String,Vec<String>>=BTreeMap::new();
        let mut instr_count=0usize;
        for b in &f.blocks {
            instr_count+=b.instructions.len();
            if instr_count>MAX_INSTRUCTIONS {return Err(bad("instruction count")); }
            let mut defs=BTreeMap::<u32,Type>::new();
            for p in &b.params {
                if defs.insert(p.id,p.ty).is_some() {return Err(bad("duplicate parameter")); }
            }
            for inst in &b.instructions {
                if defs.contains_key(&inst.id) {return Err(bad("register redefinition")); }
                let ty=check_op(&inst.op,&defs)?;
                defs.insert(inst.id,ty);
            }
            let mut outgoing=Vec::<String>::new();
            let mut check_jump=|target:&str,args:&[u32],feedback:bool| -> Result<(),VmError> {
                let target_block=blocks.get(target).ok_or_else(||bad("unknown block target"))?;
                let n=args.len()+usize::from(feedback);
                if target_block.params.len()!=n {return Err(bad("block parameter arity")); }
                for (reg,param) in args.iter().zip(&target_block.params) {
                    if defs.get(reg)!=Some(&param.ty) {return Err(bad("block parameter type")); }
                }
                if feedback && target_block.params.last().map(|p|p.ty)!=Some(Type::Bytes) {
                    return Err(bad("feedback must be Bytes"));
                }
                outgoing.push(target.to_owned());
                Ok(())
            };
            match &b.terminator {
                Terminator::Return {value} if defs.get(value)==Some(&f.returns) => {},
                Terminator::Return {..} => return Err(bad("return type")),
                Terminator::Jump {target,args} => check_jump(target,args,false)?,
                Terminator::Branch {condition,if_true,true_args,if_false,false_args} => {
                    if defs.get(condition)!=Some(&Type::Bool) {return Err(bad("branch condition")); }
                    check_jump(if_true,true_args,false)?;
                    check_jump(if_false,false_args,false)?;
                }
                Terminator::RepeatBounded {limit,body,body_args,done,done_args} => {
                    if *limit>256 {return Err(bad("repeat limit exceeds 256"));}
                    check_jump(body,body_args,false)?;
                    check_jump(done,done_args,false)?;
                    // The bounded header removes its outgoing graph edges from
                    // the cycle check: every remaining cycle is forbidden.
                    outgoing.clear();
                }
                Terminator::RepeatScoped{limit,aggregate_limit,scope,
                    body,body_args,done,done_args}=>{
                    if *limit==0 || *limit>256 ||
                        *aggregate_limit==0 || *aggregate_limit>4096 {
                        return Err(bad("scoped repeat limit"));
                    }
                    if defs.get(scope)!=Some(&Type::U32){
                        return Err(bad("scoped repeat U32 scope"));
                    }
                    check_jump(body,body_args,false)?;
                    check_jump(done,done_args,false)?;
                    // Finite aggregate_limit bounds control flow even when
                    // an organism changes scope on every visit.
                    outgoing.clear();
                }
                Terminator::Yield {call,args,next,next_args} => {
                    check_call(*call,args,&defs)?;
                    check_jump(next,next_args,true)?;
                }
            }
            edges.insert(b.id.clone(),outgoing);
        }
        // Prevent unbounded cyclic control flow in P2; bounded loops can be added as a separate opcode.
        fn visit(n:&str,edges:&BTreeMap<String,Vec<String>>,
            pending:&mut BTreeSet<String>,done:&mut BTreeSet<String>) -> Result<(),VmError> {
            if done.contains(n) {return Ok(());}
            if !pending.insert(n.to_owned()) {return Err(bad("unbounded control-flow cycle")); }
            for to in edges.get(n).ok_or_else(||bad("missing graph block"))? {
                visit(to,edges,pending,done)?;
            }
            pending.remove(n);
            done.insert(n.to_owned());
            Ok(())
        }
        for key in edges.keys() {
            visit(key,&edges,&mut BTreeSet::new(),&mut BTreeSet::new())?;
        }
        Ok(())
    }

    pub fn mutate_literal(&self, block_id:&str, register:u32, replacement:Literal)
        -> Result<Program,VmError> {
        self.validate()?; // reject malformed inputs before indexing the entry function
        let mut cloned=self.clone();
        let mut changed=false;
        for block in &mut cloned.functions[0].blocks {
            if block.id!=block_id {continue;}
            for inst in &mut block.instructions {
                if inst.id!=register {continue;}
                inst.op=match (&inst.op,replacement.clone()) {
                    (Op::ConstU32(_),Literal::U32(v))=>Op::ConstU32(v),
                    (Op::ConstBool(_),Literal::Bool(v))=>Op::ConstBool(v),
                    (Op::ConstBytes{..},Literal::Bytes(v))=>Op::ConstBytes{hex:v},
                    _=>return Err(bad("mutation type mismatch or non-literal")),
                };
                changed=true;
                break;
            }
        }
        if !changed {return Err(bad("unknown mutation target")); }
        cloned.validate()?;
        Ok(cloned)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal { U32(u32), Bool(bool), Bytes(String) }

fn need(defs:&BTreeMap<u32,Type>,reg:u32,ty:Type) -> Result<(),VmError> {
    if defs.get(&reg)!=Some(&ty) {return Err(bad(format!("invalid operand {reg}"))); }
    Ok(())
}
fn check_op(op:&Op,defs:&BTreeMap<u32,Type>) -> Result<Type,VmError> {
    match op {
        Op::ConstU32(_)=>Ok(Type::U32),
        Op::ConstBool(_)=>Ok(Type::Bool),
        Op::ConstBytes {hex}=>{decode_hex(hex)?;Ok(Type::Bytes)},
        Op::Add(a,b)|Op::Sub(a,b)|Op::Xor(a,b)|Op::And(a,b)|Op::Mod(a,b)|Op::Div(a,b)=>{
            need(defs,*a,Type::U32)?;need(defs,*b,Type::U32)?;Ok(Type::U32)
        },
        Op::EqU32(a,b)=>{need(defs,*a,Type::U32)?;need(defs,*b,Type::U32)?;Ok(Type::Bool)},
        Op::EqBytes(a,b)=>{need(defs,*a,Type::Bytes)?;need(defs,*b,Type::Bytes)?;Ok(Type::Bool)},
        Op::Len(a)|Op::Sha256(a)|Op::HexEncode(a)|Op::XorFold(a)=>{
            need(defs,*a,Type::Bytes)?;
            Ok(if matches!(op,Op::Len(_)|Op::XorFold(_)){Type::U32}else{Type::Bytes})
        },
        Op::TlvDecode(a)=>{need(defs,*a,Type::Bytes)?;Ok(Type::Bytes)},
        Op::TlvLookup(a,t)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*t,Type::U32)?;Ok(Type::Bytes)
        },
        Op::TlvReorder(a,order)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*order,Type::Bytes)?;Ok(Type::Bytes)
        },
        Op::TlvMatchesOrder(a,order)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*order,Type::Bytes)?;Ok(Type::Bool)
        },
        Op::TlvEncode(tag,value)=>{
            need(defs,*tag,Type::U32)?;need(defs,*value,Type::Bytes)?;
            Ok(Type::Bytes)
        },
        Op::PackByte(a)|Op::PackU16Le(a)|Op::PackU16Be(a)=>{
            need(defs,*a,Type::U32)?;Ok(Type::Bytes)
        },
        Op::ByteStuffEncode(a,m,e,mask)|Op::ByteStuffDecode(a,m,e,mask)=>{
            need(defs,*a,Type::Bytes)?;
            for reg in [m,e,mask] {need(defs,*reg,Type::U32)?;}
            Ok(Type::Bytes)
        },
        Op::ByteAt(a,i)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*i,Type::U32)?;Ok(Type::U32)
        },
        Op::Slice(a,start,len)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*start,Type::U32)?;
            need(defs,*len,Type::U32)?;Ok(Type::Bytes)
        },
        Op::AppendInstructions(program,block,steps,output)=>{
            need(defs,*program,Type::Bytes)?;
            need(defs,*block,Type::Bytes)?;
            need(defs,*steps,Type::Bytes)?;
            need(defs,*output,Type::U32)?;
            Ok(Type::Bytes)
        },
        Op::PrependBlocks(program,blocks)=>{
            need(defs,*program,Type::Bytes)?;
            need(defs,*blocks,Type::Bytes)?;
            Ok(Type::Bytes)
        },
        Op::RewriteOp(program,block,reg,op)=>{
            need(defs,*program,Type::Bytes)?;
            need(defs,*block,Type::Bytes)?;
            need(defs,*reg,Type::U32)?;
            need(defs,*op,Type::Bytes)?;
            Ok(Type::Bytes)
        },
        Op::JsonIndex(a,i)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*i,Type::U32)?;Ok(Type::Bytes)
        },
        Op::ReplaceByte(a,i,v)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*i,Type::U32)?;
            need(defs,*v,Type::U32)?;Ok(Type::Bytes)
        },
        Op::JsonFieldU32(a,b)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*b,Type::Bytes)?;Ok(Type::U32)
        },
        Op::Concat(a,b)|Op::JsonFieldBytes(a,b)|Op::JsonFieldString(a,b)|Op::JsonFieldJson(a,b)=>{
            need(defs,*a,Type::Bytes)?;need(defs,*b,Type::Bytes)?;Ok(Type::Bytes)
        },
    }
}
fn check_call(call:WorldCall,args:&[u32],defs:&BTreeMap<u32,Type>) -> Result<(),VmError> {
    let signature:&[Type]=match call {
        WorldCall::Observe|WorldCall::Receive|WorldCall::FenceStatus|WorldCall::RestartRoot=>&[],
        WorldCall::Read|WorldCall::Send|WorldCall::Probe|WorldCall::VerifyProbe|WorldCall::RenewProbe|WorldCall::ArmTaskFence|WorldCall::FenceStatusFor|WorldCall::RestartRootFor|WorldCall::RequestOutcomeFor=>&[Type::Bytes],
        WorldCall::Exec|WorldCall::ConfirmResponseFor=>&[Type::Bytes,Type::Bytes],
        WorldCall::Write=>&[Type::Bytes,Type::Bytes,Type::U32],
    };
    if args.len()!=signature.len() {return Err(bad("world call arity")); }
    for (reg,ty) in args.iter().zip(signature) {need(defs,*reg,*ty)?;}
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    pub program_hash: String,
    pub block: String,
    pub pc: usize,
    pub registers: BTreeMap<u32,Value>,
    pub awaiting: Option<AwaitFeedback>,
    #[serde(default)]
    pub loop_counters: BTreeMap<String,u32>,
    pub finished: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwaitFeedback { pub next: String, pub forwarded: Vec<Value> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SliceResult {
    Returned { output: Value, gas_used: u64 },
    Yielded { call: WorldCall, args: Vec<Value>, gas_used: u64 },
    Exhausted { gas_used: u64 },
    Faulted { error: VmError, gas_used: u64 },
}

fn get<'a>(regs:&'a BTreeMap<u32,Value>,id:u32)->Result<&'a Value,VmError>{
    regs.get(&id).ok_or(VmError::InvalidInput)
}
fn num(regs:&BTreeMap<u32,Value>,id:u32)->Result<u32,VmError>{
    if let Value::U32(v)=get(regs,id)? {Ok(*v)} else {Err(VmError::InvalidInput)}
}
fn bytes<'a>(regs:&'a BTreeMap<u32,Value>,id:u32)->Result<&'a [u8],VmError>{
    if let Value::Bytes(v)=get(regs,id)? {Ok(v)} else {Err(VmError::InvalidInput)}
}
fn bool_value(regs:&BTreeMap<u32,Value>,id:u32)->Result<bool,VmError>{
    if let Value::Bool(v)=get(regs,id)? {Ok(*v)} else {Err(VmError::InvalidInput)}
}
fn json_field(regs:&BTreeMap<u32,Value>,source:u32,key:u32)
    ->Result<serde_json::Value,VmError> {
    let data=bytes(regs,source)?;
    let name=std::str::from_utf8(bytes(regs,key)?).map_err(|_|VmError::InvalidInput)?;
    let object:serde_json::Value=serde_json::from_slice(data).map_err(|_|VmError::InvalidInput)?;
    object.as_object().and_then(|fields|fields.get(name))
        .cloned().ok_or(VmError::InvalidInput)
}

fn execute(op:&Op,regs:&BTreeMap<u32,Value>)->Result<Value,VmError>{
    Ok(match op {
        Op::ConstU32(v)=>Value::U32(*v),
        Op::ConstBool(v)=>Value::Bool(*v),
        Op::ConstBytes{hex}=>Value::Bytes(decode_hex(hex)?),
        Op::Add(a,b)=>Value::U32(num(regs,*a)?.wrapping_add(num(regs,*b)?)),
        Op::Sub(a,b)=>Value::U32(num(regs,*a)?.wrapping_sub(num(regs,*b)?)),
        Op::Xor(a,b)=>Value::U32(num(regs,*a)? ^ num(regs,*b)?),
        Op::And(a,b)=>Value::U32(num(regs,*a)? & num(regs,*b)?),
        Op::Mod(a,b)=>{
            let d=num(regs,*b)?;
            if d==0 {return Err(VmError::Arithmetic);}
            Value::U32(num(regs,*a)?%d)
        },
        Op::Div(a,b)=>{
            let d=num(regs,*b)?;
            if d==0 {return Err(VmError::Arithmetic);}
            Value::U32(num(regs,*a)?/d)
        },
        Op::EqU32(a,b)=>Value::Bool(num(regs,*a)?==num(regs,*b)?),
        Op::EqBytes(a,b)=>Value::Bool(bytes(regs,*a)?==bytes(regs,*b)?),
        Op::Len(a)=>Value::U32(bytes(regs,*a)?.len() as u32),
        Op::ByteAt(a,i)=>Value::U32(*bytes(regs,*a)?.get(num(regs,*i)? as usize)
            .ok_or(VmError::Range)? as u32),
        Op::Slice(a,start,len)=>{
            let raw=bytes(regs,*a)?;
            let s=num(regs,*start)? as usize;
            let l=num(regs,*len)? as usize;
            let end=s.checked_add(l).ok_or(VmError::Range)?;
            Value::Bytes(raw.get(s..end).ok_or(VmError::Range)?.to_vec())
        },
        Op::Concat(a,b)=>{
            let x=bytes(regs,*a)?;
            let y=bytes(regs,*b)?;
            if x.len().saturating_add(y.len())>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            let mut out=x.to_vec();
            out.extend_from_slice(y);
            Value::Bytes(out)
        },
        Op::XorFold(a)=>Value::U32(bytes(regs,*a)?.iter().fold(0u32,|acc,&v|acc ^ u32::from(v))),
        Op::PackU16Le(a)=>{
            let value=num(regs,*a)?;
            if value>u16::MAX as u32 {return Err(VmError::Range);}
            Value::Bytes((value as u16).to_le_bytes().to_vec())
        },
        Op::PackU16Be(a)=>{
            let value=num(regs,*a)?;
            if value>u16::MAX as u32 {return Err(VmError::Range);}
            Value::Bytes((value as u16).to_be_bytes().to_vec())
        },
        Op::TlvEncode(tag,payload)=>{
            let t=num(regs,*tag)?;
            let value=bytes(regs,*payload)?;
            if t>255 || value.len()>u16::MAX as usize ||
                value.len().saturating_add(3)>MAX_VALUE_BYTES {
                return Err(VmError::Range);
            }
            let mut out=Vec::with_capacity(value.len()+3);
            out.push(t as u8);
            out.extend_from_slice(&(value.len() as u16).to_be_bytes());
            out.extend_from_slice(value);
            Value::Bytes(out)
        },
        Op::TlvLookup(source,tag)=>{
            let raw=bytes(regs,*source)?;
            let wanted=num(regs,*tag)?;
            if wanted>255 {return Err(VmError::Range);}
            let mut at=0usize;
            let mut answer=None;
            while at<raw.len(){
                if raw.len()-at<3 {return Err(VmError::InvalidInput);}
                let n=u16::from_be_bytes([raw[at+1],raw[at+2]]) as usize;
                let end=(at+3).checked_add(n).ok_or(VmError::Range)?;
                if end>raw.len() {return Err(VmError::InvalidInput);}
                if raw[at]==wanted as u8 {
                    if answer.is_some(){return Err(VmError::InvalidInput);}
                    answer=Some(raw[at+3..end].to_vec());
                }
                at=end;
            }
            Value::Bytes(answer.ok_or(VmError::InvalidInput)?)
        },
        Op::TlvMatchesOrder(source,order)=>{
            let raw=bytes(regs,*source)?;
            let tags=bytes(regs,*order)?;
            let mut at=0usize;
            let mut n=0usize;
            let mut seen=[false;256];
            let mut valid=tags.len()<=256;
            while valid && at<raw.len(){
                if raw.len()-at<3 || n>=tags.len(){valid=false;break;}
                let tag=raw[at];
                if tag!=tags[n] || seen[usize::from(tag)] {
                    valid=false;
                    break;
                }
                seen[usize::from(tag)]=true;
                let len=u16::from_be_bytes([raw[at+1],raw[at+2]]) as usize;
                if len>raw.len()-(at+3){valid=false;break;}
                at+=3+len;
                n+=1;
            }
            Value::Bool(valid && at==raw.len() && n==tags.len())
        },
        Op::TlvReorder(source,order)=>{
            let raw=bytes(regs,*source)?;
            let desired=bytes(regs,*order)?;
            if desired.len()>256 {return Err(VmError::InvalidInput);}
            let mut at=0usize;
            let mut fields=std::collections::BTreeMap::<u8,Vec<u8>>::new();
            while at<raw.len(){
                if raw.len()-at<3 {return Err(VmError::InvalidInput);}
                let n=u16::from_be_bytes([raw[at+1],raw[at+2]]) as usize;
                let end=(at+3).checked_add(n).ok_or(VmError::Range)?;
                if end>raw.len() {return Err(VmError::InvalidInput);}
                if fields.insert(raw[at],raw[at..end].to_vec()).is_some(){
                    return Err(VmError::InvalidInput);
                }
                at=end;
            }
            if fields.len()!=desired.len(){return Err(VmError::InvalidInput);}
            let mut result=Vec::with_capacity(raw.len());
            for tag in desired {
                let part=fields.remove(tag).ok_or(VmError::InvalidInput)?;
                result.extend(part);
            }
            if result.len()>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            Value::Bytes(result)
        },
        Op::TlvDecode(source)=>{
            let wire=bytes(regs,*source)?;
            let mut at=0usize;
            let mut fields=Vec::<serde_json::Value>::new();
            while at<wire.len(){
                if wire.len()-at<3 {return Err(VmError::InvalidInput);}
                let length=u16::from_be_bytes([wire[at+1],wire[at+2]]) as usize;
                let start=at+3;
                let end=start.checked_add(length).ok_or(VmError::Range)?;
                if end>wire.len() || fields.len()>=256 {
                    return Err(VmError::InvalidInput);
                }
                fields.push(serde_json::json!({
                    "tag":wire[at] as u32, "value":wire[start..end]
                }));
                at=end;
            }
            let encoded=canonical_bytes(&fields);
            if encoded.len()>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            Value::Bytes(encoded)
        },
        Op::ByteStuffEncode(a,marker,escape,mask)=>{
            let m=num(regs,*marker)?;
            let e=num(regs,*escape)?;
            let x=num(regs,*mask)?;
            if m>255 || e>255 || x>255 || m==e || x==0 {
                return Err(VmError::Range);
            }
            let mut out=Vec::new();
            for &b in bytes(regs,*a)? {
                if b==m as u8 || b==e as u8 {
                    out.extend_from_slice(&[e as u8,b^(x as u8)]);
                } else {out.push(b);}
                if out.len()>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            }
            Value::Bytes(out)
        },
        Op::ByteStuffDecode(a,marker,escape,mask)=>{
            let m=num(regs,*marker)?;
            let e=num(regs,*escape)?;
            let x=num(regs,*mask)?;
            if m>255 || e>255 || x>255 || m==e || x==0 {
                return Err(VmError::Range);
            }
            let input=bytes(regs,*a)?;
            let mut out=Vec::with_capacity(input.len());
            let mut i=0;
            while i<input.len(){
                let b=input[i];
                if b==m as u8{return Err(VmError::InvalidInput);}
                if b==e as u8 {
                    i+=1;
                    let next=*input.get(i).ok_or(VmError::InvalidInput)?^(x as u8);
                    if next!=m as u8 && next!=e as u8 {
                        return Err(VmError::InvalidInput);
                    }
                    out.push(next);
                } else {out.push(b);}
                i+=1;
            }
            Value::Bytes(out)
        },
        Op::PackByte(a)=>{
            let value=num(regs,*a)?;
            if value>255 {return Err(VmError::Range);}
            Value::Bytes(vec![value as u8])
        },
        Op::AppendInstructions(source,name,steps,output)=>{
            let original=bytes(regs,*source)?;
            let block_id=std::str::from_utf8(bytes(regs,*name)?)
                .map_err(|_|VmError::InvalidInput)?;
            let additions:Vec<Instruction>=serde_json::from_slice(bytes(regs,*steps)?)
                .map_err(|_|VmError::InvalidInput)?;
            let final_register=num(regs,*output)?;
            let mut program:Program=serde_json::from_slice(original)
                .map_err(|_|VmError::InvalidInput)?;
            if program.canonical_bytes()!=original || program.validate().is_err(){
                return Err(VmError::InvalidInput);
            }
            let block=program.functions[0].blocks.iter_mut()
                .find(|b|b.id==block_id).ok_or(VmError::InvalidInput)?;
            if !matches!(block.terminator,Terminator::Return{..}){
                return Err(VmError::InvalidInput);
            }
            block.instructions.extend(additions);
            block.terminator=Terminator::Return{value:final_register};
            // Full verification also rejects duplicate register IDs,
            // invalid operands, excessive length, and broken types.
            program.validate().map_err(|_|VmError::InvalidInput)?;
            Value::Bytes(program.canonical_bytes())
        },
        Op::PrependBlocks(source,new_blocks)=>{
            let original=bytes(regs,*source)?;
            let additions:Vec<Block>=serde_json::from_slice(bytes(regs,*new_blocks)?)
                .map_err(|_|VmError::InvalidInput)?;
            if additions.is_empty(){return Err(VmError::InvalidInput);}
            let mut program:Program=serde_json::from_slice(original)
                .map_err(|_|VmError::InvalidInput)?;
            if program.canonical_bytes()!=original || program.validate().is_err(){
                return Err(VmError::InvalidInput);
            }
            program.functions[0].blocks.splice(0..0,additions);
            program.validate().map_err(|_|VmError::InvalidInput)?;
            Value::Bytes(program.canonical_bytes())
        },
        Op::RewriteOp(source,name,reg,replacement)=>{
            let original=bytes(regs,*source)?;
            let block_id=std::str::from_utf8(bytes(regs,*name)?)
                .map_err(|_|VmError::InvalidInput)?;
            let reg=num(regs,*reg)?;
            let next_op:Op=serde_json::from_slice(bytes(regs,*replacement)?)
                .map_err(|_|VmError::InvalidInput)?;
            let mut program:Program=serde_json::from_slice(original)
                .map_err(|_|VmError::InvalidInput)?;
            if program.canonical_bytes()!=original || program.validate().is_err(){
                return Err(VmError::InvalidInput);
            }
            let block=program.functions[0].blocks.iter_mut()
                .find(|b|b.id==block_id).ok_or(VmError::InvalidInput)?;
            let instruction=block.instructions.iter_mut().find(|i|i.id==reg)
                .ok_or(VmError::InvalidInput)?;
            instruction.op=next_op;
            program.validate().map_err(|_|VmError::InvalidInput)?;
            Value::Bytes(program.canonical_bytes())
        },
        Op::JsonIndex(source,index)=>{
            let raw=bytes(regs,*source)?;
            let data:serde_json::Value=serde_json::from_slice(raw)
                .map_err(|_|VmError::InvalidInput)?;
            let index=num(regs,*index)? as usize;
            let item=data.as_array().and_then(|list|list.get(index))
                .ok_or(VmError::InvalidInput)?;
            Value::Bytes(canonical_bytes(item))
        },
        Op::ReplaceByte(a,index,byte)=>{
            let mut data=bytes(regs,*a)?.to_vec();
            let index=num(regs,*index)? as usize;
            let value=num(regs,*byte)?;
            if value>255 {return Err(VmError::Range);}
            *data.get_mut(index).ok_or(VmError::Range)?=value as u8;
            Value::Bytes(data)
        },
        Op::JsonFieldBytes(source,field)=>{
            let value=json_field(regs,*source,*field)?;
            let serde_json::Value::Array(items)=value else {return Err(VmError::InvalidInput);};
            if items.len()>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            let result=items.into_iter().map(|v|{
                v.as_u64().filter(|n|*n<=255).map(|n|n as u8)
                    .ok_or(VmError::InvalidInput)
            }).collect::<Result<Vec<u8>,VmError>>()?;
            Value::Bytes(result)
        },
        Op::JsonFieldJson(source,field)=>{
            let value=json_field(regs,*source,*field)?;
            Value::Bytes(canonical_bytes(&value))
        },
        Op::JsonFieldU32(source,field)=>{
            let value=json_field(regs,*source,*field)?;
            let n=value.as_u64().ok_or(VmError::InvalidInput)?;
            Value::U32(u32::try_from(n).map_err(|_|VmError::Range)?)
        },
        Op::JsonFieldString(source,field)=>{
            let value=json_field(regs,*source,*field)?;
            let serde_json::Value::String(text)=value else {return Err(VmError::InvalidInput);};
            if text.len()>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            Value::Bytes(text.into_bytes())
        },
        Op::Sha256(a)=>Value::Bytes(Sha256::digest(bytes(regs,*a)?).to_vec()),
        Op::HexEncode(a)=>{
            let input=bytes(regs,*a)?;
            if input.len().saturating_mul(2)>MAX_VALUE_BYTES {return Err(VmError::OutputTooLarge);}
            let mut out=Vec::with_capacity(input.len()*2);
            for b in input {out.extend_from_slice(format!("{b:02x}").as_bytes());}
            Value::Bytes(out)
        },
    })
}
fn gas_cost(op:&Op,regs:&BTreeMap<u32,Value>)->Result<u64,VmError>{
    let len = match op {
        Op::ConstBytes{hex} => hex.len()/2,
        Op::Concat(a,b) => bytes(regs,*a)?.len().saturating_add(bytes(regs,*b)?.len()),
        Op::Slice(_,_,len)=>num(regs,*len)? as usize,
        Op::Sha256(a)=>bytes(regs,*a)?.len(),
        Op::HexEncode(a)=>bytes(regs,*a)?.len().saturating_mul(2),
        Op::XorFold(a)=>bytes(regs,*a)?.len(),
        Op::JsonIndex(a,_)|Op::ReplaceByte(a,_,_)=>bytes(regs,*a)?.len(),
        Op::TlvDecode(a)=>bytes(regs,*a)?.len().saturating_mul(8),
        Op::TlvLookup(a,_)=>bytes(regs,*a)?.len().saturating_mul(2),
        Op::TlvReorder(a,b)|Op::TlvMatchesOrder(a,b)=>
            bytes(regs,*a)?.len().saturating_mul(2)
                .saturating_add(bytes(regs,*b)?.len()),
        Op::TlvEncode(_,a)=>bytes(regs,*a)?.len().saturating_add(3),
        Op::ByteStuffEncode(a,_,_,_)=>bytes(regs,*a)?.len().saturating_mul(2),
        Op::ByteStuffDecode(a,_,_,_)=>bytes(regs,*a)?.len(),
        Op::AppendInstructions(a,b,c,_)=>bytes(regs,*a)?.len().saturating_mul(3)
            .saturating_add(bytes(regs,*b)?.len())
            .saturating_add(bytes(regs,*c)?.len()),
        Op::PrependBlocks(a,b)=>bytes(regs,*a)?.len().saturating_mul(3)
            .saturating_add(bytes(regs,*b)?.len()),
        Op::RewriteOp(a,b,_,d)=>bytes(regs,*a)?.len().saturating_mul(3)
            .saturating_add(bytes(regs,*b)?.len())
            .saturating_add(bytes(regs,*d)?.len()),
        Op::JsonFieldBytes(a,b)|Op::JsonFieldString(a,b)|Op::JsonFieldU32(a,b)|Op::JsonFieldJson(a,b)=>
            bytes(regs,*a)?.len().saturating_add(bytes(regs,*b)?.len()),
        _=>0,
    };
    Ok((if matches!(op,Op::Sha256(_)){10}else{1}) + (len as u64).saturating_add(15)/16)
}
impl Continuation {
    pub fn start(program:&Program,input:Vec<u8>)->Result<Self,VmError> {
        program.validate()?;
        if input.len()>MAX_VALUE_BYTES {return Err(VmError::InvalidInput);}
        let entry=&program.main()?.blocks[0];
        let mut registers=BTreeMap::new();
        registers.insert(entry.params[0].id,Value::Bytes(input));
        Ok(Self{program_hash:program.content_hash()?,block:entry.id.clone(),
            pc:0,registers,awaiting:None,loop_counters:BTreeMap::new(),finished:false})
    }

    pub fn resume(&mut self,program:&Program,feedback:Option<Vec<u8>>,limit:u64)
        ->Result<SliceResult,VmError> {
        program.validate()?;
        if self.program_hash!=program.content_hash()? {return Err(VmError::InvalidInput);}
        if self.finished {return Err(VmError::Finished);}
        let f=program.main()?;
        if let Some(awaiting)=self.awaiting.clone() {
            let response=feedback.ok_or(VmError::MissingFeedback)?;
            if response.len()>MAX_VALUE_BYTES {return Err(VmError::InvalidInput);}
            let mut incoming=awaiting.forwarded;
            incoming.push(Value::Bytes(response));
            self.enter(f,&awaiting.next,incoming)?;
            self.awaiting=None;
        } else if feedback.is_some() {return Err(VmError::InvalidInput);}
        let mut gas: u64=0;
        loop {
            let block=f.blocks.iter().find(|b|b.id==self.block).ok_or(VmError::InvalidInput)?;
            if self.pc<block.instructions.len() {
                let inst=&block.instructions[self.pc];
                let fee=gas_cost(&inst.op,&self.registers)?;
                if gas.saturating_add(fee)>limit {return Ok(SliceResult::Exhausted{gas_used:gas});}
                let result=execute(&inst.op,&self.registers);
                gas+=fee;
                let result=match result {
                    Ok(value)=>value,
                    Err(error)=>return Ok(SliceResult::Faulted{error,gas_used:gas}),
                };
                if let Value::Bytes(v)=&result {
                    if v.len()>MAX_VALUE_BYTES {
                        return Ok(SliceResult::Faulted{error:VmError::OutputTooLarge,gas_used:gas});
                    }
                }
                self.registers.insert(inst.id,result);
                self.pc+=1;
                continue;
            }
            if gas>=limit {return Ok(SliceResult::Exhausted{gas_used:gas});}
            // Every block terminator has a fixed cost of one gas.
            gas+=1;
            match &block.terminator {
                Terminator::Return {value}=>{
                    let output=get(&self.registers,*value)?.clone();
                    if let Value::Bytes(v)=&output {
                        if v.len()>MAX_OUTPUT_BYTES {
                            return Ok(SliceResult::Faulted{error:VmError::OutputTooLarge,gas_used:gas});
                        }
                    }
                    self.finished=true;
                    return Ok(SliceResult::Returned{output,gas_used:gas});
                },
                Terminator::Jump{target,args}=>{
                    let vals=values(&self.registers,args)?;
                    self.enter(f,target,vals)?;
                },
                Terminator::Branch{condition,if_true,true_args,if_false,false_args}=>{
                    let (target,args)=if bool_value(&self.registers,*condition)? {
                        (if_true,true_args)
                    } else {(if_false,false_args)};
                    let vals=values(&self.registers,args)?;
                    self.enter(f,target,vals)?;
                },
                Terminator::RepeatBounded{limit,body,body_args,done,done_args}=>{
                    let count=self.loop_counters.entry(block.id.clone()).or_insert(0);
                    let (target,args)=if *count<*limit {
                        *count+=1;
                        (body,body_args)
                    }else{(done,done_args)};
                    let vals=values(&self.registers,args)?;
                    self.enter(f,target,vals)?;
                },
                Terminator::RepeatScoped{
                    limit,aggregate_limit,scope,body,body_args,done,done_args
                }=>{
                    let scope_value=num(&self.registers,*scope)?;
                    let local_key=format!("{}:{scope_value}",block.id);
                    let global=self.loop_counters.get(&block.id).copied().unwrap_or(0);
                    let local=self.loop_counters.get(&local_key).copied().unwrap_or(0);
                    let (target,args)=if global<*aggregate_limit && local<*limit{
                        self.loop_counters.insert(block.id.clone(),global+1);
                        self.loop_counters.insert(local_key,local+1);
                        (body,body_args)
                    }else{(done,done_args)};
                    let vals=values(&self.registers,args)?;
                    self.enter(f,target,vals)?;
                },
                Terminator::Yield{call,args,next,next_args}=>{
                    let provided=values(&self.registers,args)?;
                    let forwarded=values(&self.registers,next_args)?;
                    self.awaiting=Some(AwaitFeedback{next:next.clone(),forwarded});
                    return Ok(SliceResult::Yielded{call:*call,args:provided,gas_used:gas});
                },
            }
        }
    }
    fn enter(&mut self,f:&Function,target:&str,values:Vec<Value>)->Result<(),VmError> {
        let b=f.blocks.iter().find(|b|b.id==target).ok_or(VmError::InvalidInput)?;
        if b.params.len()!=values.len() ||
            !b.params.iter().zip(&values).all(|(p,v)|p.ty==v.kind()) {
            return Err(VmError::InvalidInput);
        }
        self.block=target.to_owned();
        self.pc=0;
        self.registers=b.params.iter().zip(values)
            .map(|(p,v)|(p.id,v)).collect();
        Ok(())
    }
}
fn values(regs:&BTreeMap<u32,Value>,ids:&[u32])->Result<Vec<Value>,VmError>{
    ids.iter().map(|id|get(regs,*id).cloned()).collect()
}

/// Explicit isolated EXEC: no world I/O, including READ/SEND/WRITE.
/// It runs only finite pure programs. Persistent state remains with the caller.
pub fn run_isolated(program:&Program,input:Vec<u8>,limit:u64)
    ->Result<(Value,u64),VmError> {
    let mut cont=Continuation::start(program,input)?;
    let mut used=0;
    match cont.resume(program,None,limit)? {
        SliceResult::Returned{output,gas_used}=>{used+=gas_used;Ok((output,used))},
        SliceResult::Yielded{..}=>Err(VmError::SideEffect),
        SliceResult::Exhausted{..}=>Err(VmError::Gas),
        SliceResult::Faulted{error,..}=>Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(instructions:Vec<Instruction>,term:Terminator,returns:Type)->Program {
        Program{format_version:1,entry_function:"main".into(),functions:vec![
            Function{name:"main".into(),args:vec![Type::Bytes],returns,
                blocks:vec![Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions,terminator:term}]}]}
    }
    #[test]
    fn pure_vm_runs_and_hash_is_stable() {
        let program=p(vec![Instruction{id:1,op:Op::Len(0)}],
            Terminator::Return{value:1},Type::U32);
        let h=program.content_hash().unwrap();
        let decoded:Program=serde_json::from_slice(&program.canonical_bytes()).unwrap();
        assert_eq!(h,decoded.content_hash().unwrap());
        assert_eq!(run_isolated(&program,vec![1,2,3,4],100),
            Ok((Value::U32(4),2)));
    }

    #[test]
    fn append_instructions_extends_typed_graph_and_rejects_invalid_extensions(){
        let literal=|data:&[u8]|Op::ConstBytes{
            hex:data.iter().map(|b|format!("{b:02x}")).collect()
        };
        let base=p(vec![Instruction{id:1,op:Op::Len(0)}],
            Terminator::Return{value:1},Type::U32);
        let stage=vec![
            Instruction{id:2,op:Op::Len(0)},
            Instruction{id:3,op:Op::Add(1,2)},
        ];
        let editor=|raw:Vec<u8>,items:Vec<Instruction>,output:u32|{
            p(vec![
                Instruction{id:1,op:literal(&raw)},
                Instruction{id:2,op:literal(b"entry")},
                Instruction{id:3,op:literal(&serde_json::to_vec(&items).unwrap())},
                Instruction{id:4,op:Op::ConstU32(output)},
                Instruction{id:5,op:Op::AppendInstructions(1,2,3,4)},
            ],Terminator::Return{value:5},Type::Bytes)
        };
        let original=base.canonical_bytes();
        let edited=run_isolated(&editor(original.clone(),stage.clone(),3),vec![],4000)
            .unwrap().0;
        let Value::Bytes(raw)=edited else {panic!("new graph must be bytes")};
        let program:Program=serde_json::from_slice(&raw).unwrap();
        assert_eq!(program.canonical_bytes(),raw);
        assert_eq!(program.functions[0].blocks[0].instructions.len(),3);
        assert_eq!(run_isolated(&program,vec![1,2,3],200).unwrap().0,Value::U32(6));
        assert_ne!(base.content_hash().unwrap(),program.content_hash().unwrap());

        // Type safety: inputs, duplicate registers and declared return
        // must still satisfy the complete verifier.
        let wrong=vec![Instruction{id:2,op:Op::Add(0,1)}];
        assert_eq!(run_isolated(&editor(original.clone(),wrong,2),vec![],4000),
            Err(VmError::InvalidInput));
        let duplicate=vec![Instruction{id:1,op:Op::ConstU32(99)}];
        assert_eq!(run_isolated(&editor(original.clone(),duplicate,1),vec![],4000),
            Err(VmError::InvalidInput));
        assert_eq!(run_isolated(&editor(original.clone(),stage.clone(),99),vec![],4000),
            Err(VmError::InvalidInput));
        let no_op=run_isolated(&editor(original,vec![],1),vec![],4000).unwrap().0;
        assert_eq!(no_op,Value::Bytes(base.canonical_bytes()));
    }

    #[test]
    fn mutation_creates_a_new_valid_version() {
        let base=p(vec![Instruction{id:1,op:Op::ConstU32(4)}],
            Terminator::Return{value:1},Type::U32);
        let next=base.mutate_literal("entry",1,Literal::U32(8)).unwrap();
        assert_ne!(base.content_hash().unwrap(),next.content_hash().unwrap());
        assert_eq!(run_isolated(&next,vec![],100).unwrap().0,Value::U32(8));
        assert!(base.mutate_literal("entry",1,Literal::Bytes("00".into())).is_err());
    }
    #[test]
    fn type_error_and_unbounded_cycle_rejected() {
        let invalid=p(vec![Instruction{id:1,op:Op::Add(0,0)}],
            Terminator::Return{value:1},Type::U32);
        assert!(invalid.validate().is_err());
        let cyclic=p(vec![],Terminator::Jump{target:"entry".into(),args:vec![0]},Type::Bytes);
        assert!(cyclic.validate().is_err());
    }
    #[test]
    fn gas_precheck_and_bounds_are_enforced() {
        let program=p(vec![Instruction{id:1,op:Op::ConstBytes{hex:"ab".repeat(64)}}],
            Terminator::Return{value:1},Type::Bytes);
        assert_eq!(run_isolated(&program,vec![],3),Err(VmError::Gas));
        assert_eq!(run_isolated(&program,vec![],100).unwrap().0,
            Value::Bytes(vec![0xab;64]));
    }
    #[test]
    fn yield_retains_continuation_and_uses_next_tick_feedback() {
        let program=Program{format_version:1,entry_function:"main".into(),
            functions:vec![Function{name:"main".into(),args:vec![Type::Bytes],
            returns:Type::U32,blocks:vec![
                Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions:vec![],terminator:Terminator::Yield{
                        call:WorldCall::Send,args:vec![0],next:"after".into(),next_args:vec![]}},
                Block{id:"after".into(),params:vec![Param{id:10,ty:Type::Bytes}],
                    instructions:vec![Instruction{id:11,op:Op::Len(10)}],
                    terminator:Terminator::Return{value:11}},
            ]}]};
        let mut cont=Continuation::start(&program,vec![4]).unwrap();
        assert_eq!(cont.resume(&program,None,10).unwrap(),
            SliceResult::Yielded{call:WorldCall::Send,args:vec![Value::Bytes(vec![4])],gas_used:1});
        assert_eq!(cont.resume(&program,Some(vec![1,2,3]),10).unwrap(),
            SliceResult::Returned{output:Value::U32(3),gas_used:2});
        assert_eq!(cont.resume(&program,None,10),Err(VmError::Finished));
    }
    #[test]
    fn isolated_exec_cannot_call_world() {
        let program=Program{format_version:1,entry_function:"main".into(),
            functions:vec![Function{name:"main".into(),args:vec![Type::Bytes],
            returns:Type::Bytes,blocks:vec![
                Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![],terminator:Terminator::Yield{
                    call:WorldCall::Send,args:vec![0],next:"after".into(),next_args:vec![]}},
                Block{id:"after".into(),params:vec![Param{id:1,ty:Type::Bytes}],
                instructions:vec![],terminator:Terminator::Return{value:1}}
            ]}]};
        assert_eq!(run_isolated(&program,vec![1],100),Err(VmError::SideEffect));
    }
    #[test]
    fn bounded_loop_uses_persistent_counter_and_resumable_gas_slices() {
        let program=Program{format_version:1,entry_function:"main".into(),
            functions:vec![Function{name:"main".into(),args:vec![Type::Bytes],
                returns:Type::U32,blocks:vec![
                Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions:vec![Instruction{id:1,op:Op::ConstU32(0)}],
                    terminator:Terminator::Jump{target:"header".into(),args:vec![1]}},
                Block{id:"header".into(),params:vec![Param{id:10,ty:Type::U32}],
                    instructions:vec![],
                    terminator:Terminator::RepeatBounded{limit:3,
                        body:"body".into(),body_args:vec![10],
                        done:"done".into(),done_args:vec![10]}},
                Block{id:"body".into(),params:vec![Param{id:20,ty:Type::U32}],
                    instructions:vec![Instruction{id:21,op:Op::ConstU32(1)},
                        Instruction{id:22,op:Op::Add(20,21)}],
                    terminator:Terminator::Jump{target:"header".into(),args:vec![22]}},
                Block{id:"done".into(),params:vec![Param{id:30,ty:Type::U32}],
                    instructions:vec![],terminator:Terminator::Return{value:30}},
            ]}]
        };
        program.validate().expect("bounded loop must validate");
        let mut cont=Continuation::start(&program,vec![]).unwrap();
        let mut total_gas=0;
        let output=loop {
            match cont.resume(&program,None,4).unwrap() {
                SliceResult::Exhausted{gas_used}=>{
                    assert!(gas_used>0);
                    total_gas+=gas_used;
                },
                SliceResult::Returned{output,gas_used}=>{
                    total_gas+=gas_used;
                    break output;
                },
                other=>panic!("unexpected: {other:?}"),
            }
        };
        assert_eq!(output,Value::U32(3));
        assert!(total_gas>8);
        assert_eq!(cont.loop_counters.get("header"),Some(&3));
        let json=serde_json::to_vec(&cont).unwrap();
        let restored:Continuation=serde_json::from_slice(&json).unwrap();
        assert_eq!(restored,cont);
    }

    #[test]
    fn generic_block_insertion_guards_faults_but_rejects_unbounded_cycles(){
        fn literal(data:&[u8])->Op {
            Op::ConstBytes{hex:data.iter().map(|v|format!("{v:02x}")).collect()}
        }
        let base=p(vec![
            Instruction{id:1,op:Op::ConstU32(0)},
            Instruction{id:2,op:Op::ByteAt(0,1)},
        ],Terminator::Return{value:2},Type::U32);
        assert_eq!(run_isolated(&base,vec![],500),Err(VmError::Range));
        let guard=Block{
            id:"guard".into(),params:vec![Param{id:0,ty:Type::Bytes}],
            instructions:vec![
                Instruction{id:10,op:Op::Len(0)},
                Instruction{id:11,op:Op::ConstU32(0)},
                Instruction{id:12,op:Op::EqU32(10,11)},
            ],
            terminator:Terminator::Branch{condition:12,
                if_true:"empty".into(),true_args:vec![],
                if_false:"entry".into(),false_args:vec![0]},
        };
        let empty=Block{id:"empty".into(),params:vec![],
            instructions:vec![Instruction{id:20,op:Op::ConstU32(0)}],
            terminator:Terminator::Return{value:20}};
        let editor=|blocks:Vec<Block>|p(vec![
            Instruction{id:1,op:literal(&base.canonical_bytes())},
            Instruction{id:2,op:literal(&serde_json::to_vec(&blocks).unwrap())},
            Instruction{id:3,op:Op::PrependBlocks(1,2)},
        ],Terminator::Return{value:3},Type::Bytes);

        let (out,_gas)=run_isolated(&editor(vec![guard,empty]),vec![],2000).unwrap();
        let Value::Bytes(raw)=out else {panic!("expected canonical program")};
        let patched:Program=serde_json::from_slice(&raw).unwrap();
        assert_eq!(patched.canonical_bytes(),raw);
        assert_ne!(patched.content_hash().unwrap(),base.content_hash().unwrap());
        assert_eq!(run_isolated(&patched,vec![],500).unwrap().0,Value::U32(0));
        assert_eq!(run_isolated(&patched,vec![5],500).unwrap().0,Value::U32(5));

        let bad=Block{id:"guard".into(),
            params:vec![Param{id:0,ty:Type::Bytes}],
            instructions:vec![],
            terminator:Terminator::Jump{target:"guard".into(),args:vec![0]}};
        assert_eq!(run_isolated(&editor(vec![bad]),vec![],2000),
            Err(VmError::InvalidInput));
    }

    #[test]
    fn generic_graph_rewrite_changes_opcode_structure_and_revalidates_types() {
        fn hex(data:&[u8])->String {
            data.iter().map(|b|format!("{b:02x}")).collect()
        }
        let original=p(vec![Instruction{id:1,op:Op::Len(0)}],
            Terminator::Return{value:1},Type::U32);
        let src=original.canonical_bytes();
        let mutator=p(vec![
            Instruction{id:1,op:Op::ConstBytes{hex:hex(&src)}},
            Instruction{id:2,op:Op::ConstBytes{hex:hex(b"entry")}},
            Instruction{id:3,op:Op::ConstU32(1)},
            Instruction{id:4,op:Op::ConstBytes{
                hex:hex(&serde_json::to_vec(&Op::XorFold(0)).unwrap())}},
            Instruction{id:5,op:Op::RewriteOp(1,2,3,4)}
        ],Terminator::Return{value:5},Type::Bytes);
        let output=run_isolated(&mutator,vec![],2000).unwrap().0;
        let Value::Bytes(candidate)=output else {panic!("expected bytes")};
        let changed:Program=serde_json::from_slice(&candidate).unwrap();
        assert_eq!(changed.canonical_bytes(),candidate);
        assert_ne!(original.content_hash().unwrap(),changed.content_hash().unwrap());
        assert_eq!(run_isolated(&original,vec![1,2,4],100).unwrap().0,
            Value::U32(3));
        assert_eq!(run_isolated(&changed,vec![1,2,4],100).unwrap().0,
            Value::U32(7));
        let bad=p(vec![
            Instruction{id:1,op:Op::ConstBytes{hex:hex(&src)}},
            Instruction{id:2,op:Op::ConstBytes{hex:hex(b"entry")}},
            Instruction{id:3,op:Op::ConstU32(1)},
            Instruction{id:4,op:Op::ConstBytes{
                hex:hex(&serde_json::to_vec(&Op::Sha256(0)).unwrap())}},
            Instruction{id:5,op:Op::RewriteOp(1,2,3,4)}
        ],Terminator::Return{value:5},Type::Bytes);
        assert_eq!(run_isolated(&bad,vec![],2000),
            Err(VmError::InvalidInput)); // incompatible return type, reject
    }


    #[test]
    fn byte_stuffing_handles_reserved_bytes_and_rejects_malformed_escape() {
        let settings=vec![
            Instruction{id:1,op:Op::ConstU32(0x7e)},
            Instruction{id:2,op:Op::ConstU32(0x7d)},
            Instruction{id:3,op:Op::ConstU32(0x20)}
        ];
        let mut encoding=settings.clone();
        encoding.push(Instruction{id:4,op:Op::ByteStuffEncode(0,1,2,3)});
        let mut decoding=settings;
        decoding.push(Instruction{id:4,op:Op::ByteStuffDecode(0,1,2,3)});
        let input=vec![0x7e,0x7d,0,0x5e,0x5d,0xff];
        let encoded=run_isolated(&p(encoding,Terminator::Return{value:4},
            Type::Bytes),input.clone(),100).unwrap();
        assert_eq!(encoded.0,Value::Bytes(
            vec![0x7d,0x5e,0x7d,0x5d,0,0x5e,0x5d,0xff]));
        assert!(encoded.1>0,"byte stuffing must consume VM gas");
        let decoder=p(decoding,Terminator::Return{value:4},Type::Bytes);
        let Value::Bytes(data)=encoded.0 else {panic!("expected bytes")};
        assert_eq!(run_isolated(&decoder,data,100).unwrap().0,
            Value::Bytes(input));
        assert_eq!(run_isolated(&decoder,vec![0x7d],100),
            Err(VmError::InvalidInput));
        assert_eq!(run_isolated(&decoder,vec![0x7d,0x7f],100),
            Err(VmError::InvalidInput));
        let big_endian=p(vec![
            Instruction{id:1,op:Op::ConstU32(0x1234)},
            Instruction{id:2,op:Op::PackU16Be(1)}
        ],Terminator::Return{value:2},Type::Bytes);
        assert_eq!(run_isolated(&big_endian,vec![],100).unwrap().0,
            Value::Bytes(vec![0x12,0x34]));
    }

    #[test]
    fn generic_tlv_codec_is_metered_and_rejects_truncated_or_oversized_fields(){
        let encoder=p(vec![
            Instruction{id:1,op:Op::ConstU32(0xa9)},
            Instruction{id:2,op:Op::TlvEncode(1,0)}
        ],Terminator::Return{value:2},Type::Bytes);
        let input=vec![0x00,0x7e,0x99];
        let (wire,gas)=run_isolated(&encoder,input.clone(),100).unwrap();
        assert!(gas>1);
        assert_eq!(wire,Value::Bytes(vec![0xa9,0,3,0,0x7e,0x99]));
        let decoder=p(vec![Instruction{id:1,op:Op::TlvDecode(0)}],
            Terminator::Return{value:1},Type::Bytes);
        let Value::Bytes(wire)=wire else {panic!("bytes required")};
        let decoded=run_isolated(&decoder,wire,100).unwrap();
        assert!(decoded.1>1);
        let Value::Bytes(json)=decoded.0 else {panic!("json bytes required")};
        let fields:serde_json::Value=serde_json::from_slice(&json).unwrap();
        assert_eq!(fields[0]["tag"],0xa9);
        assert_eq!(fields[0]["value"],serde_json::json!([0,0x7e,0x99]));
        for invalid in [vec![0x20],vec![0x20,0,2,0x12],
            vec![0x20,0,0,0x21,0,1]] {
            assert_eq!(run_isolated(&decoder,invalid,100),
                Err(VmError::InvalidInput));
        }
        let bad_tag=p(vec![
            Instruction{id:1,op:Op::ConstU32(256)},
            Instruction{id:2,op:Op::TlvEncode(1,0)}
        ],Terminator::Return{value:2},Type::Bytes);
        assert_eq!(run_isolated(&bad_tag,input,100),Err(VmError::Range));
    }

    #[test]
    fn tlv_lookup_and_reorder_are_generic_metered_and_reject_ambiguous_tags(){
        let tlvs=vec![0x44,0,1,0x05,0x33,0,2,0xa1,0xa2,0x22,0,0];
        let lookup=p(vec![
            Instruction{id:1,op:Op::ConstU32(0x33)},
            Instruction{id:2,op:Op::TlvLookup(0,1)},
        ],Terminator::Return{value:2},Type::Bytes);
        assert_eq!(run_isolated(&lookup,tlvs.clone(),100).unwrap().0,
            Value::Bytes(vec![0xa1,0xa2]));
        let reorder=p(vec![
            Instruction{id:1,op:Op::ConstBytes{hex:"223344".into()}},
            Instruction{id:2,op:Op::TlvReorder(0,1)},
        ],Terminator::Return{value:2},Type::Bytes);
        let (got,gas)=run_isolated(&reorder,tlvs.clone(),100).unwrap();
        assert!(gas>1);
        assert_eq!(got,Value::Bytes(vec![
            0x22,0,0,0x33,0,2,0xa1,0xa2,0x44,0,1,0x05
        ]));
        assert_eq!(run_isolated(&lookup,
            [tlvs.clone(),vec![0x33,0,0]].concat(),100),
            Err(VmError::InvalidInput));
        assert_eq!(run_isolated(&reorder,
            [tlvs.clone(),vec![0x33,0,0]].concat(),100),
            Err(VmError::InvalidInput));
        assert_eq!(run_isolated(&reorder,vec![0x33,0,1],100),
            Err(VmError::InvalidInput));
    }

    #[test]
    fn tlv_shape_preflight_is_total_metered_and_does_not_accept_duplicate_or_extra_tags(){
        let verifier=p(vec![
            Instruction{id:1,op:Op::ConstBytes{hex:"213141".into()}},
            Instruction{id:2,op:Op::TlvMatchesOrder(0,1)}
        ],Terminator::Return{value:2},Type::Bool);
        let good=vec![0x21,0,1,0x01,0x31,0,0,0x41,0,2,0x03,0x04];
        assert_eq!(run_isolated(&verifier,good.clone(),100).unwrap().0,
            Value::Bool(true));
        for bad in [
            vec![0x21], // truncated
            vec![0x21,0,4,0x00], // overflow length
            [good.clone(),vec![0x7f,0,0]].concat(), // extra valid TLV
            vec![0x21,0,1,1,0x21,0,0,0x41,0,0], // duplicate tag
            vec![0x31,0,0,0x21,0,1,1,0x41,0,2,3,4], // wrong order
        ] {
            let (result,gas)=run_isolated(&verifier,bad.clone(),100).unwrap();
            assert_eq!(result,Value::Bool(false),"bad={bad:?}");
            assert!(gas>1);
        }
        // Preflight never exposes a protocol-specific tag map and never
        // turns malformed bytes into an unchecked VM InvalidInput fault.
    }

    #[test]
    fn scoped_wait_budget_is_per_candidate_but_has_hard_global_cap(){
        fn run(global:u32)->(u32,BTreeMap<String,u32>){
            let u=Type::U32;
            let program=Program{
                format_version:1,entry_function:"main".into(),
                functions:vec![Function{
                    name:"main".into(),args:vec![Type::Bytes],returns:u,
                    blocks:vec![
                        Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                            instructions:vec![Instruction{id:1,op:Op::ConstU32(0)}],
                            terminator:Terminator::Jump{target:"head".into(),args:vec![1]}},
                        Block{id:"head".into(),params:vec![Param{id:10,ty:u}],
                            instructions:vec![],terminator:Terminator::RepeatScoped{
                                scope:10,limit:2,aggregate_limit:global,
                                body:"poll".into(),body_args:vec![10],
                                done:"advance".into(),done_args:vec![10]}},
                        Block{id:"poll".into(),params:vec![Param{id:20,ty:u}],
                            instructions:vec![],terminator:Terminator::Yield{
                                call:WorldCall::Observe,args:vec![],
                                next:"again".into(),next_args:vec![20]}},
                        Block{id:"again".into(),params:vec![Param{id:30,ty:u},
                            Param{id:31,ty:Type::Bytes}],instructions:vec![],
                            terminator:Terminator::Jump{target:"head".into(),args:vec![30]}},
                        Block{id:"advance".into(),params:vec![Param{id:40,ty:u}],
                            instructions:vec![
                                Instruction{id:41,op:Op::ConstU32(1)},
                                Instruction{id:42,op:Op::Add(40,41)},
                                Instruction{id:43,op:Op::ConstU32(2)},
                                Instruction{id:44,op:Op::EqU32(42,43)}],
                            terminator:Terminator::Branch{condition:44,
                                if_true:"done".into(),true_args:vec![42],
                                if_false:"head".into(),false_args:vec![42]}},
                        Block{id:"done".into(),params:vec![Param{id:50,ty:u}],
                            instructions:vec![],terminator:Terminator::Return{value:50}},
                    ]}]
            };
            program.validate().unwrap();
            let mut state=Continuation::start(&program,vec![]).unwrap();
            let mut feedback=None;
            let mut ticks=0;
            loop{
                match state.resume(&program,feedback.take(),300).unwrap(){
                    SliceResult::Yielded{call:WorldCall::Observe,..}=>{
                        ticks+=1;
                        let serialized=serde_json::to_vec(&state).unwrap();
                        state=serde_json::from_slice(&serialized).unwrap();
                        feedback=Some(vec![]);
                    },
                    SliceResult::Returned{output:Value::U32(2),..}=>break,
                    result=>panic!("unexpected scoped VM result {result:?}"),
                }
                assert!(ticks<6);
            }
            (ticks,state.loop_counters)
        }
        let (ticks,counts)=run(4);
        assert_eq!(ticks,4);
        assert_eq!(counts["head"],4);
        assert_eq!(counts["head:0"],2);
        assert_eq!(counts["head:1"],2);
        let (ticks,counts)=run(3);
        assert_eq!(ticks,3);
        assert_eq!(counts["head"],3);
        assert_eq!(counts["head:0"],2);
        assert_eq!(counts["head:1"],1);
    }
}
