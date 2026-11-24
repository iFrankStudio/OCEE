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
}
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
            outcome_key,paid_at:None,paid_a:0
        });
        Ok(w)
    }

    // Called solely after a successful ordinary create-only Write in tick_mode.
    // No validation result is exposed except a genuine resource transfer.
    pub(super) fn settle_transform_write(&mut self,actor:u64,key:&str,step:u64)->u64{
        let Some(escrow)=self.state.transform_escrow.as_ref() else{return 0;};
        if actor!=ORGANISM_ID || escrow.paid_at.is_some() || escrow.paid_a!=0 ||
            escrow.price_a==0 || escrow.price_a>escrow.remaining_a ||
            escrow.remaining_a!=escrow.initial_a || key!=escrow.outcome_key{return 0;}
        let Some(offer)=self.state.objects.get(OFFER) else{return 0;};
        if offer.owner!=0 || offer.version!=0 || offer.bytes!=escrow.offer{return 0;}
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
}
