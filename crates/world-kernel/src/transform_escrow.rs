//! Opt-in single-shot development contract. A finite environment endowment,
//! not an unlimited reward oracle or an inter-epoch fitness measure.
use super::*;

const OFFER:&str="/public/E001/transform";
const OUTCOME:&str="/self/outcomes/s29-offer-";
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct TransformEscrow{
    pub initial_a:u64,
    pub remaining_a:u64,
    pub price_a:u64,
    pub offer:Vec<u8>,
    pub outcome_key:String,
    pub paid_at:Option<u64>,
    pub paid_a:u64,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub sequence:Option<TransformSequence>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct TransformSequence{
    pending:Vec<TransformTerms>,
    settled:Vec<TransformSettlement>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
struct TransformTerms{offer:Vec<u8>,outcome_key:String,price_a:u64}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
struct TransformSettlement{outcome_key:String,offer_version:u64,step:u64,price_a:u64}
impl World{
    /// Trusted environment construction only; no corresponding VM call.
    /// The explicitly allocated payer endowment is independent of epoch grants.
    pub fn with_transform_escrow(config:Config,task:TransformTask,endowment:u64,price:u64)->Result<Self,Code>{
        let id=task.task_id.as_bytes();
        if id.len()!=1 || !id[0].is_ascii_lowercase() ||
            task.held_out_input.is_empty() || task.held_out_input.len()>255 ||
            !task.examples.is_empty() || endowment>4096 || price==0 || price>endowment{
            return Err(Code::BadRequest);
        }
        let outcome_key=format!("{OUTCOME}{:02x}",id[0]);
        let offer=serde_json::to_vec(&task).map_err(|_|Code::BadRequest)?;
        let mut w=Self::new(config);
        w.state.objects.insert(OFFER.into(),Object{owner:0,version:0,bytes:offer.clone()});
        w.state.transform_escrow=Some(TransformEscrow{
            initial_a:endowment,remaining_a:endowment,price_a:price,offer,
            outcome_key,paid_at:None,paid_a:0,sequence:None
        });
        Ok(w)
    }

    /// Bounded, trusted sequential demand; not a live second organism.
    pub fn with_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,endowment:u64)->Result<Self,Code>{
        if !(2..=3).contains(&tasks.len()){return Err(Code::BadRequest);}
        let mut keys=std::collections::BTreeSet::new();
        let mut total=0u64;
        let mut terms=Vec::new();
        // Reuse exactly the single-offer validator, not a weaker path.
        for (task,price) in &tasks{
            let checked=Self::with_transform_escrow(config.clone(),task.clone(),endowment,*price)?;
            let e=checked.state.transform_escrow.unwrap();
            if !keys.insert(e.outcome_key.clone()){return Err(Code::BadRequest);}
            total=total.checked_add(*price).ok_or(Code::BadRequest)?;
            terms.push(TransformTerms{offer:e.offer,outcome_key:e.outcome_key,price_a:*price});
        }
        if total>endowment{return Err(Code::BadRequest);}
        let mut w=Self::with_transform_escrow(config,tasks[0].0.clone(),endowment,tasks[0].1)?;
        terms.remove(0);
        w.state.transform_escrow.as_mut().unwrap().sequence=Some(TransformSequence{
            pending:terms,settled:vec![]
        });
        Ok(w)
    }

    // Called solely after a successful ordinary create-only Write in tick_mode.
    // No validation result is exposed except a genuine resource transfer.
    pub(super) fn settle_transform_write(&mut self,actor:u64,key:&str,step:u64)->u64{
        let Some(escrow)=self.state.transform_escrow.as_ref() else{return 0;};
        if actor!=ORGANISM_ID || escrow.paid_at.is_some() || escrow.paid_a!=0 ||
            escrow.price_a==0 || escrow.price_a>escrow.remaining_a ||
            key!=escrow.outcome_key{return 0;}
        let (version,already_paid)=if let Some(seq)=&escrow.sequence{
            if seq.settled.len()>2 || seq.pending.len()+seq.settled.len()>2 ||
                seq.settled.iter().any(|r|r.outcome_key==key){return 0;}
            let Some(sum)=seq.settled.iter().try_fold(0u64,|sum,r|sum.checked_add(r.price_a)) else{return 0;};
            (seq.settled.len() as u64,sum)
        }else{(0,0)};
        if escrow.initial_a.checked_sub(already_paid)!=Some(escrow.remaining_a){return 0;}
        let Some(offer)=self.state.objects.get(OFFER) else{return 0;};
        if offer.owner!=0 || offer.version!=version || offer.bytes!=escrow.offer{return 0;}
        let Ok(task)=serde_json::from_slice::<TransformTask>(&escrow.offer) else{return 0;};
        let Some(receipt)=self.state.objects.get(key) else{return 0;};
        if receipt.owner!=actor || receipt.version!=0 || receipt.bytes.len()!=280{return 0;}
        let digest=Sha256::digest(&task.held_out_input);
        let answer=hidden_transform(self.state.config.seed,&task.held_out_input);
        if u32::from(receipt.bytes[0])!=answer ||
            &receipt.bytes[1..33]!=digest.as_slice() ||
            !receipt.bytes[33..].iter().all(|b|*b==0x55){return 0;}
        let price=escrow.price_a;
        let Some(balance)=self.state.nodes[&NodeId::A].gas_available.checked_add(price) else{return 0;};
        let Some(remaining)=escrow.remaining_a.checked_sub(price) else{return 0;};
        // Both checked before mutating either account.
        self.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=balance;
        let e=self.state.transform_escrow.as_mut().unwrap();
        e.remaining_a=remaining;e.paid_at=Some(step);e.paid_a=price;
        let next=if let Some(seq)=e.sequence.as_mut(){
            seq.settled.push(TransformSettlement{outcome_key:key.into(),offer_version:version,step,price_a:price});
            if seq.pending.is_empty(){None}else{Some(seq.pending.remove(0))}
        }else{None};
        if let Some(next)=next{
            e.offer=next.offer.clone();e.outcome_key=next.outcome_key;e.price_a=next.price_a;
            e.paid_at=None;e.paid_a=0;
            self.state.objects.insert(OFFER.into(),Object{owner:0,version:version+1,bytes:next.offer});
            self.state.version_ref_index=None;
        }
        price
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    const KEY:&str="/self/outcomes/s29-offer-61";
    fn task()->TransformTask{
        TransformTask{task_id:"a".into(),examples:vec![],held_out_input:vec![44,7,2]}
    }
    fn receipt()->Vec<u8>{
        let mut b=vec![3]; // seed129 => length; independently known expected output
        b.extend_from_slice(Sha256::digest(&task().held_out_input).as_slice());
        b.extend_from_slice(&[0x55;247]);b
    }
    fn world(gas:u64,storage:u64)->World{
        World::with_transform_escrow(Config{seed:129,a_gas_per_epoch:gas,
            a_storage_bytes:storage,epoch_length:16,..Config::default()},task(),150,100).unwrap()
    }
    fn write(actor:u64,key:&str,bytes:Vec<u8>,version:Option<u64>)->Action{
        Action::Write{actor,key:key.into(),bytes,expected_version:version}
    }
    fn a(w:&World)->u64{w.state.nodes[&NodeId::A].gas_available}
    fn escrow(w:&World)->&TransformEscrow{w.state.transform_escrow.as_ref().unwrap()}
    #[test]
    fn finite_escrow_settles_real_spendable_a_and_replays(){
        let mut w=world(25,1024);w.tick(None);
        let initial=w.snapshot_json();let mut replay=World::from_snapshot(&initial).unwrap();
        let before=a(&w)+escrow(&w).remaining_a;
        let submit=write(ORGANISM_ID,KEY,receipt(),None);
        let r=w.tick(Some(submit.clone()));
        assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);assert_eq!(r.a_credited,100);
        assert_eq!(r,replay.tick(Some(submit)));
        assert_eq!(a(&w),105);assert_eq!(escrow(&w).remaining_a,50);
        assert_eq!(a(&w)+escrow(&w).remaining_a,before-r.gas_charged);
        assert_eq!(escrow(&w).paid_at,Some(1));assert_eq!(escrow(&w).paid_a,100);
        assert_eq!(w.state.objects[KEY].version,0);
        let actions=[
            Action::Read{actor:ORGANISM_ID,key:"/sys/resource-status".into()},
            write(ORGANISM_ID,"/self/use-earned",vec![7;96],None),
            write(ORGANISM_ID,KEY,receipt(),None),
            write(ORGANISM_ID,KEY,receipt(),Some(0))
        ];
        for (i,action) in actions.into_iter().enumerate(){
            let balance=a(&w);let result=w.tick(Some(action.clone()));
            assert_eq!(result,replay.tick(Some(action)));assert_eq!(result.a_credited,0);
            assert_eq!(a(&w),balance-result.gas_charged);
            assert_eq!(escrow(&w).remaining_a,50);
            if i==0{
                assert_eq!(result.code,Code::Ok);
                let value:serde_json::Value=serde_json::from_slice(&result.payload).unwrap();
                assert_eq!(value["a_available"].as_u64(),Some(a(&w)));
            }else if i==2{assert_eq!(result.code,Code::Conflict);}
            else{assert_eq!(result.code,Code::Ok);}
            if i==1{assert!(result.gas_charged>5);}
        }
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        assert_eq!(w.audit.iter().map(|e|e.result.a_credited).sum::<u64>(),100);
        while w.state.step<17{w.tick(None);replay.tick(None);}
        assert_eq!(a(&w),25);assert_eq!(escrow(&w).remaining_a,50);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        println!("S29_ESCROW_SETTLED credit=100 fee=20 remaining=50 post_credit_A=105 replay=true epoch_reset_A=25");
    }

    #[test]
    fn finite_escrow_no_credit_on_invalid_or_failed_submission(){
        for mode in ["wrong_answer","wrong_digest","wrong_padding","wrong_path","stale_offer","gas","quota","actor","overflow"]{
            let mut w=match mode{
                "gas"=>world(19,1024),"quota"=>world(25,279),
                "overflow"=>world(u64::MAX,1024),_=>world(25,1024)
            };
            w.tick(None);
            let mut bytes=receipt();let mut key=KEY;let mut actor=ORGANISM_ID;
            match mode{
                "wrong_answer"=>bytes[0]^=1,"wrong_digest"=>bytes[1]^=1,
                "wrong_padding"=>bytes[33]^=1,"wrong_path"=>key="/self/unrelated",
                "stale_offer"=>w.state.objects.get_mut(OFFER).unwrap().version=1,
                "actor"=>actor=99,_=>{}
            }
            let balance=a(&w);let r=w.tick(Some(write(actor,key,bytes,None)));
            assert_eq!(r.a_credited,0,"{mode}");
            assert_eq!(a(&w),balance-r.gas_charged);
            assert_eq!(escrow(&w).remaining_a,150);assert_eq!(escrow(&w).paid_at,None);
            assert_eq!(r.code,match mode{"gas"=>Code::Gas,"quota"=>Code::Quota,"actor"=>Code::Denied,_=>Code::Ok});
            if matches!(mode,"gas"|"quota"|"actor"){assert!(!w.state.objects.contains_key(key));}
        }
        let mut w=World::new(Config{seed:129,..Config::default()});
        let r=w.tick(Some(write(ORGANISM_ID,KEY,receipt(),None)));
        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
        assert!(w.state.transform_escrow.is_none());
        assert!(!serde_json::to_string(&r).unwrap().contains("a_credited"));
        assert!(!serde_json::to_string(&w.state).unwrap().contains("transform_escrow"));
        let mut w=world(25,1024);
        while w.state.step<17{w.tick(None);}
        assert_eq!(escrow(&w).remaining_a,150);
        let r=w.tick(Some(write(ORGANISM_ID,KEY,receipt(),None)));
        assert_eq!(r.a_credited,100);assert_eq!(escrow(&w).paid_at,Some(17));
        println!("S29_ESCROW_NEGATIVE wrong_or_failed=9 disabled=true unspent_epoch_persistent=true");
    }
}


#[cfg(test)]
mod autonomous_pilot{
    use super::*;
    use ocee_organism_vm::{Block,Function,Instruction,Op,Param,Program,Terminator,Type,Value,WorldCall};
    fn literal(bytes:&[u8])->Op{
        Op::ConstBytes{hex:bytes.iter().map(|b|format!("{b:02x}")).collect::<String>()}
    }
    fn root()->Program{
        let i=|id,op|Instruction{id,op};
        let feedback=||vec![Param{id:2,ty:Type::Bytes}];
        let blocks=vec![
            Block{id:"entry".into(),params:vec![Param{id:0,ty:Type::Bytes}],
                instructions:vec![i(10,literal(OFFER.as_bytes()))],
                terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],next:"offer".into(),next_args:vec![]}},
            Block{id:"offer".into(),params:feedback(),instructions:vec![
                i(10,literal(b"code")),i(11,Op::JsonFieldString(2,10)),i(12,literal(b"Ok")),i(13,Op::EqBytes(11,12))],
                terminator:Terminator::Branch{condition:13,if_true:"compute".into(),true_args:vec![2],
                    if_false:"failed".into(),false_args:vec![]}},
            Block{id:"compute".into(),params:feedback(),instructions:vec![
                i(10,literal(b"payload")),i(11,Op::JsonFieldBytes(2,10)),
                i(12,literal(b"task_id")),i(13,Op::JsonFieldString(11,12)),i(14,Op::HexEncode(13)),
                i(15,literal(OUTCOME.as_bytes())),i(16,Op::Concat(15,14)),
                i(17,literal(b"held_out_input")),i(18,Op::JsonFieldBytes(11,17)),
                i(19,Op::Len(18)),i(20,Op::PackByte(19)),i(21,Op::Sha256(18)),
                i(22,Op::Concat(20,21)),i(23,literal(&[0x55;247])),i(24,Op::Concat(22,23)),
                i(25,Op::ConstU32(u32::MAX))],
                terminator:Terminator::Yield{call:WorldCall::Write,args:vec![16,24,25],next:"submitted".into(),next_args:vec![]}},
            Block{id:"submitted".into(),params:feedback(),instructions:vec![
                i(10,literal(b"code")),i(11,Op::JsonFieldString(2,10)),i(12,literal(b"Ok")),i(13,Op::EqBytes(11,12))],
                terminator:Terminator::Branch{condition:13,if_true:"workspace".into(),true_args:vec![],
                    if_false:"failed".into(),false_args:vec![]}},
            Block{id:"workspace".into(),params:vec![],instructions:vec![
                i(10,literal(b"/self/earned-workspace")),i(11,literal(&[0x37;2048])),i(12,Op::ConstU32(u32::MAX))],
                terminator:Terminator::Yield{call:WorldCall::Write,args:vec![10,11,12],next:"worked".into(),next_args:vec![]}},
            Block{id:"worked".into(),params:feedback(),instructions:vec![
                i(10,literal(b"code")),i(11,Op::JsonFieldString(2,10)),i(12,literal(b"Ok")),i(13,Op::EqBytes(11,12))],
                terminator:Terminator::Branch{condition:13,if_true:"done".into(),true_args:vec![],if_false:"failed".into(),false_args:vec![]}},
            Block{id:"done".into(),params:vec![],instructions:vec![i(10,Op::ConstU32(1))],terminator:Terminator::Return{value:10}},
            Block{id:"failed".into(),params:vec![],instructions:vec![i(10,Op::ConstU32(0))],terminator:Terminator::Return{value:10}}
        ];
        let p=Program{format_version:1,entry_function:"main".into(),functions:vec![Function{
            name:"main".into(),args:vec![Type::Bytes],returns:Type::U32,blocks}]};
        p.validate().expect("typed earn-spend root");p
    }
    #[test]
    fn finite_escrow_autonomous_earn_spend(){
        let p=root();let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        for (id,input) in [("a",vec![44,7,2]),("b",vec![13,9,4,8,2])]{
            let task=TransformTask{task_id:id.into(),examples:vec![],held_out_input:input.clone()};
            let key=format!("{OUTCOME}{:02x}",id.as_bytes()[0]);
            let mut setup_cost=None;
            for price in [1,1000]{
                let config=Config{seed:129,a_gas_per_epoch:12_000,epoch_length:16,..Config::default()};
                let mut w=World::with_transform_escrow(config,task.clone(),1000,price).unwrap();
                for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                    let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                    assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                }
                let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                if let Some(previous)=setup_cost{assert_eq!(paid,previous);}else{setup_cost=Some(paid);}
                w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                    kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
                while w.state.step<16{w.tick(None);}
                assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
                let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                let mut balance=256u64;let mut remaining=1000u64;let mut fees=0;let mut after_fees=0;
                let mut received=0;let mut submitted_at=None;let mut workspace_ok=false;let mut gas=0;
                for _ in 0..12{
                    let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                    let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                    let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                    assert_eq!(r,rr);
                    let fee=r.gas_charged+r.vm_gas_charged;
                    if submitted_at.is_some(){after_fees+=fee;}
                    balance=balance.checked_sub(fee).expect("fees before credit").checked_add(r.a_credited).unwrap();
                    remaining=remaining.checked_sub(r.a_credited).unwrap();fees+=fee;received+=r.a_credited;
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                    assert_eq!(remaining,w.state.transform_escrow.as_ref().unwrap().remaining_a);
                    assert_eq!(balance+remaining,1256-fees);
                    let e=w.audit.last().unwrap();
                    if r.code==Code::Gas{gas+=1;}
                    if r.a_credited>0{
                        assert_eq!(r.a_credited,price);assert_eq!(r.code,Code::Ok);
                        assert_eq!(r.gas_charged,20);assert!(submitted_at.is_none());
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        submitted_at=Some(e.step);
                    }
                    if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/earned-workspace")&&r.code==Code::Ok{
                        assert!(submitted_at.is_some());workspace_ok=true;
                    }
                }
                println!("S29_AUTONOMOUS_ESCROW id={id} price={price} root={hash} root_B={} install_A={paid} submitted_at={submitted_at:?} received={received} fees={fees} after_fees={after_fees} final_A={balance} escrow_A={remaining} workspace={workspace_ok} gas={gas} return={:?} storage_B={} replay=true",
                    raw.len(),w.state.organisms[&ORGANISM_ID].last_return,w.state.organisms[&ORGANISM_ID].storage_used);
                assert_eq!(received,price);assert!(submitted_at.is_some());
                let receipt=&w.state.objects[&key];
                assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);assert_eq!(receipt.bytes.len(),280);
                assert_eq!(receipt.bytes[0],input.len() as u8);
                assert_eq!(&receipt.bytes[1..33],Sha256::digest(&input).as_slice());
                assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                assert_eq!(w.state.transform_escrow.as_ref().unwrap().paid_at,submitted_at);
                assert_eq!(w.state.transform_escrow.as_ref().unwrap().paid_a,price);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                assert_eq!(w.audit.iter().filter(|e|matches!(&e.action,Some(Action::Write{key:k,..}) if k==&key)).count(),1);
                if price==1000{
                    assert!(workspace_ok);assert_eq!(w.state.objects["/self/earned-workspace"].bytes,vec![0x37;2048]);
                    assert_eq!(w.state.objects["/self/earned-workspace"].version,0);
                    assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
                    assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap().finished);
                    assert_eq!(gas,0);assert!(fees>257);
                }else{
                    assert!(!workspace_ok);assert!(!w.state.objects.contains_key("/self/earned-workspace"));assert!(gas>0);
                }
            }
        }
    }

    fn chain_root()->Program{
        let mut p=root();let f=&mut p.functions[0];
        f.blocks.retain(|b|!matches!(b.id.as_str(),"workspace"|"worked"|"done"));
        let ctx=vec![Param{id:90,ty:Type::U32}];
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions=vec![Instruction{id:90,op:Op::ConstU32(0)}];
                b.terminator=Terminator::Jump{target:"chain_header".into(),args:vec![90]};
            }else{
                b.params.insert(0,ctx[0].clone());
                match &mut b.terminator{
                    Terminator::Branch{true_args,false_args,..}=>{true_args.insert(0,90);false_args.insert(0,90);},
                    Terminator::Yield{next_args,..}=>next_args.insert(0,90),
                    Terminator::Return{value}=>{*value=90;b.instructions.clear();},
                    _=>panic!("unexpected root structure")
                }
            }
        }
        let b=f.blocks.iter_mut().find(|b|b.id=="submitted").unwrap();
        b.terminator=Terminator::Branch{condition:13,if_true:"chain_credit".into(),true_args:vec![90,2],
            if_false:"failed".into(),false_args:vec![90]};
        // Missing credit is omitted by ActionResult serialization, so use
        // the actual JSON object field via a total string/bytes test? The
        // contracted paid sequence emits a positive U32 field on every Ok
        // receipt. For no-credit Ok, use a generic field extraction guard
        // in the existing VM; see the explicit failure path below.
        let mut feedback=ctx.clone();feedback.push(Param{id:2,ty:Type::Bytes});
        f.blocks.extend([
            Block{id:"chain_header".into(),params:ctx.clone(),instructions:vec![],
                terminator:Terminator::RepeatBounded{limit:3,body:"chain_read".into(),body_args:vec![90],
                    done:"failed".into(),done_args:vec![90]}},
            Block{id:"chain_read".into(),params:ctx.clone(),instructions:vec![Instruction{id:10,op:literal(OFFER.as_bytes())}],
                terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],next:"offer".into(),next_args:vec![90]}},
            Block{id:"chain_credit".into(),params:feedback,instructions:vec![
                Instruction{id:10,op:literal(b"a_credited")},Instruction{id:11,op:Op::JsonFieldU32(2,10)},
                Instruction{id:12,op:Op::ConstU32(0)},Instruction{id:13,op:Op::EqU32(11,12)}
            ],terminator:Terminator::Branch{condition:13,if_true:"failed".into(),true_args:vec![90],
                if_false:"chain_advance".into(),false_args:vec![90]}},
            Block{id:"chain_advance".into(),params:ctx,instructions:vec![
                Instruction{id:10,op:Op::ConstU32(1)},Instruction{id:11,op:Op::Add(90,10)}
            ],terminator:Terminator::Jump{target:"chain_header".into(),args:vec![11]}}
        ]);
        p.validate().expect("bounded sequential productive root");p
    }
    fn chain_tasks(first:u64)->Vec<(TransformTask,u64)>{
        [("a",vec![44,7,2],first),("b",vec![13,9,4,8,2],200),
            ("c",vec![27,2,1,4,8,6],200)].into_iter().map(|(id,input,price)|
                (TransformTask{task_id:id.into(),examples:vec![],held_out_input:input},price)).collect()
    }
    #[test]
    fn finite_escrow_productive_chain_native(){
        let p=chain_root();let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [1,1000]{
            let tasks=chain_tasks(first);
            let mut w=World::with_transform_sequence(Config{seed:129,epoch_length:16,
                a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(v)=install{assert_eq!(paid,v);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            while w.state.step<16{w.tick(None);}
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut remaining=1400u64;let mut fees=0u64;
            let mut received=0u64;let mut settlements=Vec::new();let mut reads=Vec::new();let mut gas=0;
            for _ in 0..12{
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                balance=balance.checked_sub(fee).unwrap().checked_add(r.a_credited).unwrap();
                remaining=remaining.checked_sub(r.a_credited).unwrap();
                received+=r.a_credited;fees+=fee;
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(remaining,escrow.remaining_a);assert_eq!(balance+remaining,1656-fees);
                let e=w.audit.last().unwrap();
                if r.code==Code::Gas{gas+=1;}
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    settlements.push((e.step,slot,*price));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    let public=&w.state.objects[OFFER];
                    let next=(slot+1).min(2);
                    assert_eq!(public.version,next as u64);
                    assert_eq!(serde_json::from_slice::<TransformTask>(&public.bytes).unwrap(),tasks[next].0);
                }
            }
            println!("S29_PRODUCTIVE_CHAIN first={first} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            let expected=if first==1000{3}else{1};
            assert_eq!(settlements.len(),expected);
            assert_eq!(received,if first==1000{1400}else{1});
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<expected);
                if slot<expected{
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if first==1000{
                assert_eq!(gas,0);assert!(fees>257);assert_eq!(remaining,0);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
            }else{assert!(gas>0);assert_eq!(remaining,1399);}
        }
    }
    #[test]
    fn finite_escrow_productive_chain_negative(){
        let cfg=Config{seed:129,a_gas_per_epoch:256,epoch_length:16,..Config::default()};
        let mut duplicate=chain_tasks(1000);duplicate[1].0.task_id="a".into();
        assert!(matches!(World::with_transform_sequence(cfg.clone(),duplicate,1400),Err(Code::BadRequest)));
        assert!(matches!(World::with_transform_sequence(cfg.clone(),chain_tasks(1000),1399),Err(Code::BadRequest)));
        let tasks=chain_tasks(1000);
        let write=|task:&TransformTask,wrong:bool|{
            let mut bytes=vec![task.held_out_input.len() as u8^u8::from(wrong)];
            bytes.extend_from_slice(Sha256::digest(&task.held_out_input).as_slice());bytes.extend_from_slice(&[0x55;247]);
            Action::Write{actor:ORGANISM_ID,key:format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]),bytes,expected_version:None}
        };
        let mut bad=World::with_transform_sequence(cfg.clone(),tasks.clone(),1400).unwrap();
        assert_eq!(bad.tick(Some(write(&tasks[0].0,true))).a_credited,0);
        assert_eq!(bad.state.objects[OFFER].version,0);
        assert_eq!(bad.state.transform_escrow.as_ref().unwrap().remaining_a,1400);
        let mut w=World::with_transform_sequence(cfg,tasks.clone(),1400).unwrap();
        assert_eq!(w.tick(Some(write(&tasks[0].0,false))).a_credited,1000);
        let r=w.tick(Some(write(&tasks[0].0,false)));
        assert_eq!(r.code,Code::Conflict);assert_eq!(r.a_credited,0);
        assert_eq!(w.state.objects[OFFER].version,1);
        while w.state.step<17{w.tick(None);}
        assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
        assert_eq!(w.state.transform_escrow.as_ref().unwrap().remaining_a,400);
        assert_eq!(w.tick(Some(write(&tasks[1].0,false))).a_credited,200);
        assert_eq!(w.tick(Some(write(&tasks[2].0,false))).a_credited,200);
        assert_eq!(w.state.transform_escrow.as_ref().unwrap().remaining_a,0);
        assert_eq!(w.state.objects[OFFER].version,2);
        println!("S29_CHAIN_NEGATIVE duplicate=true overcommitted=true wrong_no_advance=true stale_no_credit=true epoch_preserved=true");
    }

    fn liquidity_root(estimate:bool)->Program{
        let mut p=chain_root();let f=&mut p.functions[0];
        let yields=f.blocks.iter().filter_map(|b|match &b.terminator{
            Terminator::Yield{next,..}=>Some(next.clone()),_=>None
        }).collect::<std::collections::BTreeSet<_>>();
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.push(Instruction{id:91,op:Op::ConstU32(0)});
            }else{
                let at=if yields.contains(&b.id){b.params.len()-1}else{b.params.len()};
                b.params.insert(at,Param{id:91,ty:Type::U32});
            }
            match &mut b.terminator{
                Terminator::Jump{args,..}=>args.push(91),
                Terminator::Branch{true_args,false_args,..}=>{true_args.push(91);false_args.push(91);},
                Terminator::Yield{next_args,..}=>next_args.push(91),
                Terminator::RepeatBounded{body_args,done_args,..}|
                Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.push(91);done_args.push(91);},
                Terminator::Return{..}=>{}
            }
        }
        let ctx=vec![Param{id:90,ty:Type::U32},Param{id:91,ty:Type::U32}];
        let mut initial_reply=ctx.clone();initial_reply.push(Param{id:2,ty:Type::Bytes});
        let entry=f.blocks.iter_mut().find(|b|b.id=="entry").unwrap();
        entry.instructions.push(Instruction{id:10,op:literal(b"/sys/resource-status")});
        entry.terminator=Terminator::Yield{call:WorldCall::Read,args:vec![10],next:"liquidity_initial".into(),next_args:vec![90,91]};
        let credit=f.blocks.iter_mut().find(|b|b.id=="chain_credit").unwrap();
        credit.terminator=Terminator::Branch{condition:13,if_true:"failed".into(),true_args:vec![90,91],
            if_false:"liquidity_read".into(),false_args:vec![90,91,11]};
        // The old increment is replaced by a measured-interval decision.
        f.blocks.retain(|b|b.id!="chain_advance");
        let mut cost_ctx=ctx.clone();cost_ctx.push(Param{id:92,ty:Type::U32});
        let mut cost_reply=cost_ctx.clone();cost_reply.push(Param{id:2,ty:Type::Bytes});
        let measured=|id:u32|vec![
            Instruction{id:10,op:literal(b"payload")},Instruction{id:11,op:Op::JsonFieldBytes(2,10)},
            Instruction{id:12,op:literal(b"a_available")},Instruction{id,op:Op::JsonFieldU32(11,12)}
        ];
        let mut measure=measured(13);
        measure.extend([
            Instruction{id:14,op:Op::Add(91,92)},Instruction{id:15,op:Op::Sub(14,13)},
            Instruction{id:16,op:Op::ConstU32(0)},Instruction{id:17,op:Op::EqU32(15,16)},
            Instruction{id:18,op:Op::ConstU32(1)},Instruction{id:19,op:Op::Add(90,18)}
        ]);
        let mut compare_ctx=ctx.clone();compare_ctx.push(Param{id:92,ty:Type::U32});
        f.blocks.extend([
            Block{id:"liquidity_initial".into(),params:initial_reply,instructions:measured(13),
                terminator:Terminator::Jump{target:"chain_header".into(),args:vec![90,13]}},
            Block{id:"liquidity_read".into(),params:cost_ctx,instructions:vec![Instruction{id:10,op:literal(b"/sys/resource-status")}],
                terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],next:"liquidity_measure".into(),next_args:vec![90,91,92]}},
            Block{id:"liquidity_measure".into(),params:cost_reply,instructions:measure,
                terminator:Terminator::Branch{condition:17,if_true:"failed".into(),true_args:vec![19,13],
                    if_false:"liquidity_compare".into(),false_args:vec![19,13,15]}},
            Block{id:"liquidity_compare".into(),params:compare_ctx,instructions:vec![
                Instruction{id:10,op:Op::Div(91,92)},Instruction{id:11,op:Op::ConstU32(0)},
                Instruction{id:12,op:Op::EqU32(10,11)}
            ],terminator:if estimate{Terminator::Branch{condition:12,if_true:"failed".into(),true_args:vec![90,91],
                if_false:"chain_header".into(),false_args:vec![90,91]}}
                else{Terminator::Jump{target:"chain_header".into(),args:vec![90,91]}}}
        ]);
        p.validate().expect("typed measured liquidity");p
    }
    #[test]
    fn s29_measured_liquidity_native(){
        for first in [1,1000]{
            let mut totals=Vec::new();
            for estimate in [false,true]{
                let p=liquidity_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let tasks=chain_tasks(first);
                let mut w=World::with_transform_sequence(Config{seed:129,epoch_length:16,
                    a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400).unwrap();
                for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                    let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                    assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                }
                let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                    kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:384}});
                while w.state.step<16{w.tick(None);}
                let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                let mut balance=384u64;let mut remaining=1400u64;let mut fees=0u64;
                let mut received=0u64;let mut settlements=Vec::new();let mut reads=Vec::new();let mut gas=0;
                let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
                for _ in 0..12{
                    let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                    let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                    let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                    assert_eq!(r,rr);
                    let fee=r.gas_charged+r.vm_gas_charged;
                    balance=balance.checked_sub(fee).unwrap().checked_add(r.a_credited).unwrap();
                    remaining=remaining.checked_sub(r.a_credited).unwrap();
                    received+=r.a_credited;fees+=fee;
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                    let escrow=w.state.transform_escrow.as_ref().unwrap();
                    assert_eq!(remaining,escrow.remaining_a);assert_eq!(balance+remaining,1784-fees);
                    let e=w.audit.last().unwrap();
                    if r.code==Code::Gas{gas+=1;}
                    if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                        assert_eq!(r.gas_charged,2);
                        let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                        assert_eq!(v["a_available"].as_u64(),Some(balance));
                        assert_eq!(v["capacity"].as_u64(),Some(w.state.nodes[&NodeId::A].storage_capacity));
                        assert_eq!(v["used"].as_u64(),Some(w.state.organisms[&ORGANISM_ID].storage_used));
                        if let Some((old,credit,spent))=last_sensor{
                            let cost=old+(received-credit)-balance;
                            assert_eq!(cost,fees-spent);assert!(cost>0);
                            intervals.push((cost,balance,balance/cost));
                        }
                        last_sensor=Some((balance,received,fees));sensors.push((e.step,balance));
                    }
                    if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                        let slot=settlements.len();
                        assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                        reads.push((e.step,slot));
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_LIQUIDITY first={first} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                let expected=if first==1000{3}else{1};
                assert_eq!(settlements.len(),expected);assert_eq!(sensors.len(),expected+1);
                assert_eq!(received,if first==1000{1400}else{1});
                for (slot,(task,_)) in tasks.iter().enumerate(){
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(w.state.objects.contains_key(&key),slot<expected);
                    if slot<expected{
                        let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                        assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                        assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                        assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                    }
                }
                if first==1000{
                    assert_eq!(gas,0);assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
                    assert!(intervals.iter().all(|x|x.2>0));
                }else{
                    assert_eq!(intervals[0].2,0);
                    if estimate{
                        assert_eq!(gas,0);assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
                        assert_eq!(reads.len(),1);
                    }else{assert!(gas>0);}
                }
                totals.push((balance,fees,paid));
            }
            if first==1{assert!(totals[1].0>totals[0].0);}
        }
    }
    #[test]
    fn s29_measured_liquidity_limits_native(){
        let mut false_affordable=Vec::new();let mut false_unaffordable=Vec::new();
        for tape in ["INCREASE","DECREASE"]{
        for grant in [384u64,512,640,768,896,1024,1280]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=liquidity_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                if tape=="INCREASE"{tasks[1].0.held_out_input=vec![7;255];}
                else{tasks[0].0.held_out_input=vec![7;128];}
                let mut w=World::with_transform_sequence(Config{seed:129,epoch_length:16,
                    a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400).unwrap();
                for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                    let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                    assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                }
                let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                    kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                while w.state.step<16{w.tick(None);}
                let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                let mut balance=grant;let mut remaining=1400u64;let mut fees=0u64;
                let mut received=0u64;let mut settlements=Vec::new();let mut reads=Vec::new();let mut gas=0;
                let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
                for _ in 0..12{
                    let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                    let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                    let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                    assert_eq!(r,rr);
                    let fee=r.gas_charged+r.vm_gas_charged;
                    balance=balance.checked_sub(fee).unwrap().checked_add(r.a_credited).unwrap();
                    remaining=remaining.checked_sub(r.a_credited).unwrap();
                    received+=r.a_credited;fees+=fee;
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                    let escrow=w.state.transform_escrow.as_ref().unwrap();
                    assert_eq!(remaining,escrow.remaining_a);assert_eq!(balance+remaining,grant+1400-fees);
                    let e=w.audit.last().unwrap();
                    if r.code==Code::Gas{gas+=1;}
                    if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                        assert_eq!(r.gas_charged,2);
                        let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                        assert_eq!(v["a_available"].as_u64(),Some(balance));
                        assert_eq!(v["capacity"].as_u64(),Some(w.state.nodes[&NodeId::A].storage_capacity));
                        assert_eq!(v["used"].as_u64(),Some(w.state.organisms[&ORGANISM_ID].storage_used));
                        if let Some((old,credit,spent))=last_sensor{
                            let cost=old+(received-credit)-balance;
                            assert_eq!(cost,fees-spent);assert!(cost>0);
                            intervals.push((cost,balance,balance/cost));
                        }
                        last_sensor=Some((balance,received,fees));sensors.push((e.step,balance));
                    }
                    if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                        let slot=settlements.len();
                        assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                        reads.push((e.step,slot));
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_LIQUIDITY_LIMIT tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                let expected=settlements.len();
                assert!(expected<=3);
                assert_eq!(received,tasks.iter().take(expected).map(|(_,p)|*p).sum::<u64>());
                assert!(sensors.len()<=expected+1);
                assert_eq!(intervals.len(),sensors.len().saturating_sub(1));
                for (slot,(task,_)) in tasks.iter().enumerate(){
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(w.state.objects.contains_key(&key),slot<expected);
                    if slot<expected{
                        let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                        assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                        assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                        assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                    }
                }
                let first_interval=intervals.first().copied();
                let read_second=reads.iter().any(|(_,slot)|*slot==1);
                let normal_one=w.state.organisms[&ORGANISM_ID].last_return==Some(Value::U32(1));
                outcomes.push((expected,first_interval,read_second,gas,normal_one));
            }
            let (count_c,interval_c,_,_,_)=outcomes[0];
            let (count_e,interval_e,read_e,gas_e,normal_e)=outcomes[1];
            assert_eq!(interval_c,interval_e,"identical pre-decision measurement");
            if let Some((cost,available,q))=interval_e{
                if tape=="INCREASE"&&q>=1&&count_e==1&&read_e&&gas_e>0{
                    false_affordable.push((grant,cost,available));
                }
                if tape=="DECREASE"&&q==0&&count_e==1&&normal_e&&gas_e==0&&count_c>=2{
                    false_unaffordable.push((grant,cost,available));
                }
            }
        }
        }
        println!("S29_LIQUIDITY_COUNTEREXAMPLES false_affordable={false_affordable:?} false_unaffordable={false_unaffordable:?}");
        assert!(!false_affordable.is_empty());
        assert!(!false_unaffordable.is_empty());
    }


    // Paid next-offer inspection in both arms; finite 9-bit multiplication
    // implements the exact ratio without adding a VM primitive.
    fn size_conditioned_root(scale:bool)->Program{
        let mut p=liquidity_root(false);let f=&mut p.functions[0];
        let ys=f.blocks.iter().filter_map(|b|match &b.terminator{
            Terminator::Yield{next,..}=>Some(next.clone()),_=>None
        }).collect::<std::collections::BTreeSet<_>>();
        let u=|id|Param{id,ty:Type::U32};
        let bts=|id|Param{id,ty:Type::Bytes};
        let ctx=vec![u(90),u(91),u(93),u(94)];
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.extend([Instruction{id:93,op:Op::ConstU32(0)},Instruction{id:94,op:Op::ConstU32(0)}]);
            }else{
                let at=if ys.contains(&b.id){b.params.len()-1}else{b.params.len()};
                b.params.insert(at,u(93));b.params.insert(at+1,u(94));
            }
            match &mut b.terminator{
                Terminator::Jump{args,..}=>args.extend([93,94]),
                Terminator::Branch{true_args,false_args,..}=>{true_args.extend([93,94]);false_args.extend([93,94]);},
                Terminator::Yield{next_args,..}=>next_args.extend([93,94]),
                Terminator::RepeatBounded{body_args,done_args,..}|
                Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.extend([93,94]);done_args.extend([93,94]);},
                Terminator::Return{..}=>{}
            }
        }
        let m=f.blocks.iter_mut().find(|b|b.id=="liquidity_measure").unwrap();
        m.terminator=Terminator::Branch{condition:17,if_true:"failed".into(),true_args:vec![19,13,93,15],
            if_false:"chain_header".into(),false_args:vec![19,13,93,15]};
        f.blocks.retain(|b|b.id!="liquidity_compare");
        let compute=f.blocks.iter_mut().find(|b|b.id=="compute").unwrap();
        assert_eq!(compute.params.iter().map(|p|p.id).collect::<Vec<_>>(),vec![90,2,91,93,94]);
        let tail=compute.instructions.split_off(compute.instructions.iter().position(|i|i.id==20).unwrap());
        compute.instructions.extend([Instruction{id:30,op:Op::ConstU32(0)},Instruction{id:31,op:Op::EqU32(90,30)}]);
        let args=vec![90,91,93,94,16,18,19];
        let mut offer_ctx=ctx.clone();offer_ctx.extend([bts(16),bts(18),u(19)]);
        compute.terminator=Terminator::Branch{condition:31,if_true:"size_submit".into(),
            true_args:args.clone(),if_false:"size_model_start".into(),false_args:args.clone()};
        let mut mul_ctx=offer_ctx.clone();mul_ctx.extend([u(95),u(96),u(97)]);
        let mut mul_args=args.clone();mul_args.extend([95,96,97]);
        f.blocks.extend([
            Block{id:"size_submit".into(),params:offer_ctx.clone(),instructions:tail,
                terminator:Terminator::Yield{call:WorldCall::Write,args:vec![16,24,25],next:"submitted".into(),next_args:vec![90,91,19,94]}},
            Block{id:"size_model_start".into(),params:offer_ctx.clone(),instructions:vec![
                Instruction{id:10,op:Op::ConstU32(128)},Instruction{id:11,op:Op::Add(19,10)},
                Instruction{id:12,op:Op::ConstU32(0)}
            ],terminator:Terminator::Jump{target:"size_mul_header".into(),
                args:{let mut a=args.clone();a.extend([12,94,11]);a}}},
            Block{id:"size_mul_header".into(),params:mul_ctx.clone(),instructions:vec![],
                terminator:Terminator::RepeatScoped{limit:9,aggregate_limit:18,scope:90,
                    body:"size_mul_step".into(),body_args:mul_args.clone(),done:"size_compare".into(),done_args:mul_args.clone()}},
            Block{id:"size_mul_step".into(),params:mul_ctx.clone(),instructions:vec![
                Instruction{id:10,op:Op::ConstU32(2)},Instruction{id:11,op:Op::Mod(97,10)},
                Instruction{id:12,op:Op::ConstU32(0)},Instruction{id:13,op:Op::Sub(12,11)},
                Instruction{id:14,op:Op::And(96,13)},Instruction{id:15,op:Op::Add(95,14)},
                Instruction{id:20,op:Op::Add(96,96)},Instruction{id:21,op:Op::Div(97,10)}
            ],terminator:Terminator::Jump{target:"size_mul_header".into(),
                args:{let mut a=args.clone();a.extend([15,20,21]);a}}},
            Block{id:"size_compare".into(),params:mul_ctx,instructions:vec![
                Instruction{id:10,op:Op::ConstU32(128)},Instruction{id:11,op:Op::Add(93,10)},
                Instruction{id:12,op:Op::Div(95,11)},Instruction{id:13,op:Op::ConstU32(0)},
                Instruction{id:14,op:Op::EqU32(if scale{12}else{94},13)}
            ],terminator:Terminator::Branch{condition:14,if_true:"failed".into(),true_args:vec![90,91,93,94],
                if_false:"size_gate".into(),false_args:{let mut a=args.clone();a.push(if scale{12}else{94});a}}},
            Block{id:"size_gate".into(),params:{let mut c=offer_ctx;c.push(u(92));c},instructions:vec![
                Instruction{id:10,op:Op::Div(91,92)},Instruction{id:11,op:Op::ConstU32(0)},
                Instruction{id:12,op:Op::EqU32(10,11)}
            ],terminator:Terminator::Branch{condition:12,if_true:"failed".into(),true_args:vec![90,91,93,94],
                if_false:"size_submit".into(),false_args:args}}
        ]);
        p.validate().expect("typed size conditioned liquidity");p
    }

    #[test]
    fn s29_size_conditioned_liquidity_native(){
        let mut avoided=Vec::new();let mut recovered=Vec::new();let mut lost=Vec::new();
        for tape in ["INCREASE","DECREASE"]{
        for grant in [384u64,512,640,768,896,1024,1280]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=size_conditioned_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                if tape=="INCREASE"{tasks[1].0.held_out_input=vec![7;255];}
                else{tasks[0].0.held_out_input=vec![7;128];}
                let mut w=World::with_transform_sequence(Config{seed:129,epoch_length:16,
                    a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400).unwrap();
                for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                    let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                    assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                }
                let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                    kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                while w.state.step<16{w.tick(None);}
                let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                let mut balance=grant;let mut remaining=1400u64;let mut fees=0u64;
                let mut received=0u64;let mut settlements=Vec::new();let mut reads=Vec::new();let mut gas=0;
                let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
                for _ in 0..15{
                    let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                    let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                    let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                    assert_eq!(r,rr);
                    let fee=r.gas_charged+r.vm_gas_charged;
                    balance=balance.checked_sub(fee).unwrap().checked_add(r.a_credited).unwrap();
                    remaining=remaining.checked_sub(r.a_credited).unwrap();
                    received+=r.a_credited;fees+=fee;
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                    let escrow=w.state.transform_escrow.as_ref().unwrap();
                    assert_eq!(remaining,escrow.remaining_a);assert_eq!(balance+remaining,grant+1400-fees);
                    let e=w.audit.last().unwrap();
                    if r.code==Code::Gas{gas+=1;}
                    if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                        assert_eq!(r.gas_charged,2);
                        let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                        assert_eq!(v["a_available"].as_u64(),Some(balance));
                        assert_eq!(v["capacity"].as_u64(),Some(w.state.nodes[&NodeId::A].storage_capacity));
                        assert_eq!(v["used"].as_u64(),Some(w.state.organisms[&ORGANISM_ID].storage_used));
                        if let Some((old,credit,spent))=last_sensor{
                            let cost=old+(received-credit)-balance;
                            assert_eq!(cost,fees-spent);assert!(cost>0);
                            intervals.push((cost,balance,balance/cost));
                        }
                        last_sensor=Some((balance,received,fees));sensors.push((e.step,balance));
                    }
                    if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                        let slot=settlements.len();
                        assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                        reads.push((e.step,slot));
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_SIZE_LIQUIDITY tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                let expected=settlements.len();
                assert!(expected<=3);
                assert_eq!(received,tasks.iter().take(expected).map(|(_,p)|*p).sum::<u64>());
                assert!(sensors.len()<=expected+1);
                assert_eq!(intervals.len(),sensors.len().saturating_sub(1));
                for (slot,(task,_)) in tasks.iter().enumerate(){
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(w.state.objects.contains_key(&key),slot<expected);
                    if slot<expected{
                        let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                        assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                        assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                        assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                    }
                }
                let first_interval=intervals.first().copied();
                let read_second=reads.iter().any(|(_,slot)|*slot==1);
                let normal_one=w.state.organisms[&ORGANISM_ID].last_return==Some(Value::U32(1));
                outcomes.push((expected,first_interval,read_second,gas,normal_one));
            }
            let (count_c,interval_c,read_c,gas_c,_)=outcomes[0];
            let (count_e,interval_e,_,gas_e,normal_e)=outcomes[1];
            assert_eq!(interval_c,interval_e,"identical first-task measurement");
            if tape=="INCREASE"&&count_c==1&&read_c&&gas_c>0&&count_e==1&&normal_e&&gas_e==0{
                avoided.push(grant);
            }
            if tape=="DECREASE"&&count_e>count_c&&gas_e==0{recovered.push(grant);}
            if count_e<count_c{lost.push((tape,grant,count_c,count_e));}
        }
        }
        println!("S29_SIZE_COUNTEREXAMPLES avoided={avoided:?} recovered={recovered:?} lost={lost:?}");
        assert!(!avoided.is_empty());
        assert!(!recovered.is_empty());
    }


    // Same paid inspection, much cheaper additive size prior. All quantities
    // bounded by the fixture; division by L is safe after a paid nonempty task.
    fn additive_liquidity_root(add:bool)->Program{
        let mut p=size_conditioned_root(false);let f=&mut p.functions[0];
        f.blocks.retain(|b|!matches!(b.id.as_str(),"size_mul_header"|"size_mul_step"|"size_compare"));
        let start=f.blocks.iter_mut().find(|b|b.id=="size_model_start").unwrap();
        let args=vec![90,91,93,94,16,18,19];
        let ctx=start.params.clone();
        assert_eq!(ctx.iter().map(|p|p.id).collect::<Vec<_>>(),args);
        start.instructions=vec![
            Instruction{id:10,op:Op::Add(94,19)},
            Instruction{id:11,op:Op::Div(10,93)},
            Instruction{id:12,op:Op::ConstU32(0)},
            Instruction{id:13,op:Op::EqU32(11,12)}
        ];
        start.terminator=Terminator::Branch{condition:13,if_true:"failed".into(),
            true_args:vec![90,91,93,94],if_false:"additive_compare".into(),false_args:args.clone()};
        f.blocks.push(Block{id:"additive_compare".into(),params:ctx,instructions:vec![
            Instruction{id:10,op:Op::Add(94,19)},Instruction{id:11,op:Op::Sub(10,93)},
            Instruction{id:12,op:Op::ConstU32(0)},Instruction{id:13,op:Op::EqU32(11,12)}
        ],terminator:Terminator::Branch{condition:13,if_true:"failed".into(),true_args:vec![90,91,93,94],
            if_false:"size_gate".into(),false_args:{let mut a=args;a.push(if add{11}else{94});a}}});
        p.validate().expect("typed bounded additive liquidity");p
    }

    #[test]
    fn s29_additive_liquidity_native(){
        let mut avoided=Vec::new();let mut recovered=Vec::new();let mut lost=Vec::new();
        for tape in ["INCREASE","DECREASE"]{
        for grant in [384u64,512,640,768,896,1024,1280]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=additive_liquidity_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                if tape=="INCREASE"{tasks[1].0.held_out_input=vec![7;255];}
                else{tasks[0].0.held_out_input=vec![7;128];}
                let mut w=World::with_transform_sequence(Config{seed:129,epoch_length:16,
                    a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400).unwrap();
                for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                    let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                    assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                }
                let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                    kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                while w.state.step<16{w.tick(None);}
                let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                let mut balance=grant;let mut remaining=1400u64;let mut fees=0u64;
                let mut received=0u64;let mut settlements=Vec::new();let mut reads=Vec::new();let mut gas=0;
                let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
                for _ in 0..15{
                    let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                    let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                    let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                    assert_eq!(r,rr);
                    let fee=r.gas_charged+r.vm_gas_charged;
                    balance=balance.checked_sub(fee).unwrap().checked_add(r.a_credited).unwrap();
                    remaining=remaining.checked_sub(r.a_credited).unwrap();
                    received+=r.a_credited;fees+=fee;
                    assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                    let escrow=w.state.transform_escrow.as_ref().unwrap();
                    assert_eq!(remaining,escrow.remaining_a);assert_eq!(balance+remaining,grant+1400-fees);
                    let e=w.audit.last().unwrap();
                    if r.code==Code::Gas{gas+=1;}
                    if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                        assert_eq!(r.gas_charged,2);
                        let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                        assert_eq!(v["a_available"].as_u64(),Some(balance));
                        assert_eq!(v["capacity"].as_u64(),Some(w.state.nodes[&NodeId::A].storage_capacity));
                        assert_eq!(v["used"].as_u64(),Some(w.state.organisms[&ORGANISM_ID].storage_used));
                        if let Some((old,credit,spent))=last_sensor{
                            let cost=old+(received-credit)-balance;
                            assert_eq!(cost,fees-spent);assert!(cost>0);
                            intervals.push((cost,balance,balance/cost));
                        }
                        last_sensor=Some((balance,received,fees));sensors.push((e.step,balance));
                    }
                    if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                        let slot=settlements.len();
                        assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                        reads.push((e.step,slot));
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_ADDITIVE_LIQUIDITY tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                let expected=settlements.len();
                assert!(expected<=3);
                assert_eq!(received,tasks.iter().take(expected).map(|(_,p)|*p).sum::<u64>());
                assert!(sensors.len()<=expected+1);
                assert_eq!(intervals.len(),sensors.len().saturating_sub(1));
                for (slot,(task,_)) in tasks.iter().enumerate(){
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(w.state.objects.contains_key(&key),slot<expected);
                    if slot<expected{
                        let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                        assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                        assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                        assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                    }
                }
                let first_interval=intervals.first().copied();
                let read_second=reads.iter().any(|(_,slot)|*slot==1);
                let normal_one=w.state.organisms[&ORGANISM_ID].last_return==Some(Value::U32(1));
                outcomes.push((expected,first_interval,read_second,gas,normal_one));
            }
            let (count_c,interval_c,read_c,gas_c,_)=outcomes[0];
            let (count_e,interval_e,_,gas_e,normal_e)=outcomes[1];
            assert_eq!(interval_c,interval_e,"identical first-task measurement");
            if tape=="INCREASE"&&count_c==1&&read_c&&gas_c>0&&count_e==1&&normal_e&&gas_e==0{
                avoided.push(grant);
            }
            if tape=="DECREASE"&&count_e>count_c&&gas_e==0{recovered.push(grant);}
            if count_e<count_c{lost.push((tape,grant,count_c,count_e));}
        }
        }
        println!("S29_ADDITIVE_COUNTEREXAMPLES avoided={avoided:?} recovered={recovered:?} lost={lost:?}");
        assert!(!avoided.is_empty());
        assert!(!recovered.is_empty());
    }

}
