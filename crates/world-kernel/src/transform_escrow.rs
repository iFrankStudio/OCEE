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
