//! Opt-in single-shot development contract. A finite environment endowment,
//! not an unlimited reward oracle or an inter-epoch fitness measure.
use super::*;

const OFFER:&str="/public/E001/transform";
const OUTCOME:&str="/self/outcomes/s29-offer-";
const DECLINE:&str="/self/declines/s29-offer-";
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
    /// Opt-in development quote fee. Absent in legacy snapshots.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub price_query_fee:Option<u64>,
    /// Opt-in earned portion of A's total spendable balance. None = legacy quota reset.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub earned_a:Option<u64>,
    /// Opt-in redundant fee total in autonomous feedback; no new information.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub fee_total_feedback:Option<bool>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct TransformSequence{
    pending:Vec<TransformTerms>,
    settled:Vec<TransformSettlement>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    max_offers:Option<u32>,
    /// None preserves the old mandatory-settlement sequence.
    #[serde(default,skip_serializing_if="Option::is_none")]
    declined:Option<Vec<TransformDecline>>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
struct TransformTerms{offer:Vec<u8>,outcome_key:String,price_a:u64}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
struct TransformSettlement{outcome_key:String,offer_version:u64,step:u64,price_a:u64}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
struct TransformDecline{outcome_key:String,offer_version:u64,step:u64}
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
            outcome_key,paid_at:None,paid_a:0,sequence:None,price_query_fee:None,earned_a:None,fee_total_feedback:None
        });
        Ok(w)
    }

    /// Bounded, trusted sequential demand; not a live second organism.
    pub fn with_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,endowment:u64)->Result<Self,Code>{
        Self::with_bounded_transform_sequence(config,tasks,endowment,None)
    }

    fn with_bounded_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,
        endowment:u64,max_offers:Option<u32>)->Result<Self,Code>{
        let max=max_offers.unwrap_or(3) as usize;
        if !(2..=max).contains(&tasks.len()){return Err(Code::BadRequest);}
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
            pending:terms,settled:vec![],max_offers,declined:None
        });
        Ok(w)
    }

    /// Trusted environment opt-in; never enabled by an organism.
    pub fn with_priced_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,
        endowment:u64,fee:u64)->Result<Self,Code>{
        if !(1..=128).contains(&fee){return Err(Code::BadRequest);}
        let mut w=Self::with_transform_sequence(config,tasks,endowment)?;
        w.state.transform_escrow.as_mut().unwrap().price_query_fee=Some(fee);
        Ok(w)
    }

    /// Trusted finite sequence opt-in, preserving the existing escrow limits.
    pub fn with_persistent_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,
        endowment:u64)->Result<Self,Code>{
        let mut w=Self::with_transform_sequence(config,tasks,endowment)?;
        w.state.transform_escrow.as_mut().unwrap().earned_a=Some(0);
        Ok(w)
    }

    /// Separate, trusted 12-offer pilot; no new funding or VM operation.
    pub fn with_long_persistent_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,
        endowment:u64)->Result<Self,Code>{
        let mut w=Self::with_bounded_transform_sequence(config,tasks,endowment,Some(12))?;
        w.state.transform_escrow.as_mut().unwrap().earned_a=Some(0);
        Ok(w)
    }

    /// Trusted finite choice pilot: permit declining nonterminal offers by a
    /// paid ordinary create-only write, without payment or replenishment.
    pub fn with_declining_transform_sequence(config:Config,tasks:Vec<(TransformTask,u64)>,
        endowment:u64)->Result<Self,Code>{
        let mut w=Self::with_persistent_transform_sequence(config,tasks,endowment)?;
        w.state.transform_escrow.as_mut().unwrap().sequence.as_mut().unwrap().declined=Some(vec![]);
        Ok(w)
    }

    /// Trusted opt-in single-offer accounting pilot; no VM enable action.
    pub fn with_persistent_transform_escrow(config:Config,task:TransformTask,
        endowment:u64,price:u64)->Result<Self,Code>{
        let mut w=Self::with_transform_escrow(config,task,endowment,price)?;
        w.state.transform_escrow.as_mut().unwrap().earned_a=Some(0);
        Ok(w)
    }

    /// A metered read of only the current, unsettled offer. Explicit task/version
    /// prevents treating an old quote as the price of a new offer.
    pub(super) fn read_transform_price(&mut self,actor:u64,suffix:&str,compact:bool)->ActionResult{
        let Some(fee)=self.state.transform_escrow.as_ref().and_then(|e|e.price_query_fee)
            else{return ActionResult::fail(Code::NotFound);};
        if actor!=ORGANISM_ID{return ActionResult::fail(Code::Denied);}
        if !(1..=128).contains(&fee){return ActionResult::fail(Code::BadRequest);}
        // Pay before any quote-dependent response, including stale requests.
        if let Err(code)=self.spend_a_gas(fee){return ActionResult::fail(code);}
        let payload=(||{
            let e=self.state.transform_escrow.as_ref()?;
            let seq=e.sequence.as_ref()?;
            let o=self.state.objects.get(OFFER)?;
            let version=(seq.settled.len()+seq.declined.as_ref().map_or(0,Vec::len)) as u64;
            let paid=seq.settled.iter().try_fold(0u64,|sum,r|sum.checked_add(r.price_a))?;
            if e.paid_at.is_some() || e.paid_a!=0 || e.price_a==0 ||
                e.price_a>e.remaining_a || e.initial_a.checked_sub(paid)!=Some(e.remaining_a) ||
                o.owner!=0 || o.version!=version || o.bytes!=e.offer{return None;}
            let task:TransformTask=serde_json::from_slice(&e.offer).ok()?;
            if suffix!=format!("{}/{}",task.task_id,version){return None;}
            if compact{
                // Same paid bound synchronous quote; no extra information.
                Some(u16::try_from(e.price_a).ok()?.to_le_bytes().to_vec())
            }else{
                serde_json::to_vec(&serde_json::json!({
                    "task_id":task.task_id,"offer_version":version,"price_a":e.price_a
                })).ok()
            }
        })();
        match payload{
            Some(bytes)=>ActionResult::ok(bytes,fee,0),
            None=>ActionResult{gas_charged:fee,..ActionResult::fail(Code::NotFound)}
        }
    }

    // Called solely after a successful ordinary create-only Write in tick_mode.
    // No validation result is exposed except a genuine resource transfer.
    pub(super) fn settle_transform_write(&mut self,actor:u64,key:&str,step:u64)->u64{
        let Some(escrow)=self.state.transform_escrow.as_ref() else{return 0;};
        if actor!=ORGANISM_ID || escrow.paid_at.is_some() || escrow.paid_a!=0 ||
            escrow.price_a==0 || escrow.price_a>escrow.remaining_a ||
            key!=escrow.outcome_key{return 0;}
        let (version,already_paid)=if let Some(seq)=&escrow.sequence{
            let max=seq.max_offers.unwrap_or(3) as usize;
            let resolved=seq.settled.len()+seq.declined.as_ref().map_or(0,Vec::len);
            if !matches!(max,3|12) || resolved>=max ||
                seq.pending.len()+resolved>=max ||
                seq.settled.iter().any(|r|r.outcome_key==key) ||
                seq.declined.as_ref().is_some_and(|rs|rs.iter().any(|r|r.outcome_key==key)){return 0;}
            let Some(sum)=seq.settled.iter().try_fold(0u64,|sum,r|sum.checked_add(r.price_a)) else{return 0;};
            (resolved as u64,sum)
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
        let earned=if let Some(old)=escrow.earned_a{
            let Some(new)=old.checked_add(price) else{return 0;};
            if old>self.state.nodes[&NodeId::A].gas_available ||
                old>already_paid || new>escrow.initial_a-remaining{return 0;}
            Some(new)
        }else{None};
        // All checks before mutating either account.
        self.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=balance;
        let e=self.state.transform_escrow.as_mut().unwrap();
        e.earned_a=earned;
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

    // Only after a successful create-only ordinary Write. A wrong/stale
    // marker is ordinary owned data, not an acknowledgement or free advance.
    pub(super) fn decline_transform_write(&mut self,actor:u64,key:&str,step:u64){
        if actor!=ORGANISM_ID{return;}
        let Some(e)=self.state.transform_escrow.as_ref() else{return;};
        let Some(seq)=e.sequence.as_ref() else{return;};
        let Some(declined)=seq.declined.as_ref() else{return;};
        let resolved=seq.settled.len()+declined.len();
        let max=seq.max_offers.unwrap_or(3) as usize;
        if max!=3 || seq.pending.is_empty() || resolved+seq.pending.len()>=max ||
            e.paid_at.is_some() || e.paid_a!=0 || e.price_a==0 || e.price_a>e.remaining_a ||
            seq.settled.iter().any(|r|r.outcome_key==e.outcome_key) ||
            declined.iter().any(|r|r.outcome_key==e.outcome_key){return;}
        let Some(suffix)=e.outcome_key.strip_prefix(OUTCOME) else{return;};
        if key!=format!("{DECLINE}{suffix}"){return;}
        let Some(offer)=self.state.objects.get(OFFER) else{return;};
        let Some(marker)=self.state.objects.get(key) else{return;};
        let version=resolved as u64;
        if offer.owner!=0 || offer.version!=version || offer.bytes!=e.offer ||
            marker.owner!=actor || marker.version!=0 || marker.bytes.as_slice()!=version.to_le_bytes().as_slice(){return;}
        let Some(paid)=seq.settled.iter().try_fold(0u64,|sum,r|sum.checked_add(r.price_a)) else{return;};
        if e.initial_a.checked_sub(paid)!=Some(e.remaining_a) ||
            e.earned_a.is_none_or(|earned|earned>paid||earned>self.state.nodes[&NodeId::A].gas_available){return;}
        let e=self.state.transform_escrow.as_mut().unwrap();
        let seq=e.sequence.as_mut().unwrap();
        seq.declined.as_mut().unwrap().push(TransformDecline{outcome_key:e.outcome_key.clone(),offer_version:version,step});
        let next=seq.pending.remove(0);
        e.offer=next.offer.clone();e.outcome_key=next.outcome_key;e.price_a=next.price_a;
        self.state.objects.insert(OFFER.into(),Object{owner:0,version:version+1,bytes:next.offer});
        self.state.version_ref_index=None;
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
    fn s29_decline_offer_contract_native(){
        let cfg=Config{seed:129,a_gas_per_epoch:500,epoch_length:64,..Config::default()};
        let mut b=task();b.task_id="b".into();
        let mut c=task();c.task_id="c".into();
        let tasks=vec![(task(),100),(b.clone(),1),(c.clone(),100)];
        let make=|enabled:bool|{
            let mut w=if enabled{World::with_declining_transform_sequence(cfg.clone(),tasks.clone(),201)}
                else{World::with_persistent_transform_sequence(cfg.clone(),tasks.clone(),201)}.unwrap();
            w.state.transform_escrow.as_mut().unwrap().price_query_fee=Some(8);
            w.tick(None);w
        };
        let decline=|id:&str,version:u64|write(ORGANISM_ID,&format!("{DECLINE}{id}"),version.to_le_bytes().to_vec(),None);
        let quote=|id:&str,version:u64|Action::Read{actor:ORGANISM_ID,key:format!("/sys/transform-price-u16/{id}/{version}")};
        let mut legacy=make(false);
        let r=legacy.tick(Some(decline("61",0)));
        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
        assert_eq!(legacy.state.objects[OFFER].version,0);
        assert!(!std::str::from_utf8(&legacy.snapshot_json()).unwrap().contains("\"declined\""));
        let mut invalid=make(true);
        let r=invalid.tick(Some(decline("61",1)));
        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
        assert_eq!(invalid.state.objects[OFFER].version,0);
        assert!(escrow(&invalid).sequence.as_ref().unwrap().declined.as_ref().unwrap().is_empty());
        let mut w=make(true);
        let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
        let mut fees=0u64;let mut credit=0u64;
        let acts=[
            write(ORGANISM_ID,KEY,receipt(),None), // a: settle
            decline("62",1),                      // b: decline
            quote("b",1),                         // stale, paid NotFound
            quote("c",2),                         // current, paid price
            write(ORGANISM_ID,"/self/outcomes/s29-offer-62",receipt(),None), // declined: no credit
            decline("63",2),                      // final: no advance
            write(ORGANISM_ID,"/self/outcomes/s29-offer-63",receipt(),None), // c: settle
            decline("62",1),                      // repeated marker: rejected
        ];
        for (i,act) in acts.into_iter().enumerate(){
            let r=w.tick(Some(act.clone()));assert_eq!(r,replay.tick(Some(act)));
            let fee=r.gas_charged+r.vm_gas_charged;fees+=fee;credit+=r.a_credited;
            assert_eq!(a(&w),500+credit-fees);
            assert_eq!(escrow(&w).remaining_a,201-credit);
            assert_eq!(escrow(&w).earned_a,Some(credit)); // initial quota covers all fees
            match i{
                0|6=>{assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,100);}
                1=>{assert_eq!(r.code,Code::Ok);assert!(fee>0);assert_eq!(r.a_credited,0);
                    assert_eq!(w.state.objects[OFFER].version,2);
                    assert_eq!(serde_json::from_slice::<TransformTask>(&w.state.objects[OFFER].bytes).unwrap(),c);}
                2=>{assert_eq!(r.code,Code::NotFound);assert_eq!(fee,8);}
                3=>{assert_eq!(r.code,Code::Ok);assert_eq!(r.payload,100u16.to_le_bytes().to_vec());assert_eq!(fee,8);}
                4|5=>{assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);assert_eq!(w.state.objects[OFFER].version,2);}
                7=>{assert_ne!(r.code,Code::Ok);assert_eq!(r.a_credited,0);}
                _=>unreachable!()
            }
        }
        let seq=escrow(&w).sequence.as_ref().unwrap();
        assert_eq!(seq.settled.iter().map(|r|r.offer_version).collect::<Vec<_>>(),vec![0,2]);
        assert_eq!(seq.declined.as_ref().unwrap().as_slice(),&[TransformDecline{outcome_key:"/self/outcomes/s29-offer-62".into(),offer_version:1,step:2}]);
        assert!(seq.pending.is_empty());assert_eq!(credit,200);assert_eq!(escrow(&w).remaining_a,1);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        println!("S29_DECLINE_CONTRACT fees={fees} credit={credit} final_A={} payer_A={} settled=2 declined=1 version=2 legacy=true stale=true final_rejected=true replay=true",a(&w),escrow(&w).remaining_a);
    }

    #[test]
    fn s29_paid_price_contract_native(){
        let config=Config{seed:129,a_gas_per_epoch:200,epoch_length:16,..Config::default()};
        let second=TransformTask{task_id:"b".into(),examples:vec![],held_out_input:vec![7;8]};
        let tasks=vec![(task(),100),(second,40)];
        let read=|suffix:&str|Action::Read{actor:ORGANISM_ID,key:format!("/sys/transform-price/{suffix}")};
        let mut legacy=World::with_transform_sequence(config.clone(),tasks.clone(),150).unwrap();
        legacy.tick(None);
        let before=a(&legacy);
        let r=legacy.tick(Some(read("a/0")));
        assert_eq!(r,ActionResult::fail(Code::NotFound));assert_eq!(a(&legacy),before);
        assert!(!std::str::from_utf8(&legacy.snapshot_json()).unwrap().contains("price_query_fee"));
        for fee in [8u64,32,96]{
            let mut w=World::with_priced_transform_sequence(config.clone(),tasks.clone(),150,fee).unwrap();
            w.tick(None);
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            // First query and settlement fit even at the largest fee. Stale
            // and current queries must be charged independently.
            let actions=[read("a/0"),write(ORGANISM_ID,KEY,receipt(),None),
                read("a/0"),read("b/1")];
            let mut results=Vec::new();
            for act in actions{
                let before=a(&w);let payer=escrow(&w).remaining_a;
                let r=w.tick(Some(act.clone()));
                assert_eq!(r,replay.tick(Some(act)));
                assert_eq!(a(&w),before-r.gas_charged-r.vm_gas_charged+r.a_credited);
                assert_eq!(escrow(&w).remaining_a,payer-r.a_credited);
                results.push(r);
            }
            let quote=|r:&ActionResult,id:&str,version:u64,price:u64|{
                assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,fee);
                assert_eq!(r.a_credited,0);
                let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                assert_eq!(v,serde_json::json!({"task_id":id,"offer_version":version,"price_a":price}));
            };
            quote(&results[0],"a",0,100);
            assert_eq!(results[1].a_credited,100);assert_eq!(results[1].gas_charged,20);
            assert_eq!(results[2].code,Code::NotFound);assert!(results[2].payload.is_empty());
            assert_eq!(results[2].gas_charged,fee);
            if fee==96{
                assert_eq!(results[3],ActionResult::fail(Code::Gas));
            }else{quote(&results[3],"b",1,40);}
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_PAID_QUOTE fee={fee} final_A={} payer_A={} last={:?} replay=true",a(&w),escrow(&w).remaining_a,results[3].code);
        }
        let mut poor=World::with_priced_transform_sequence(Config{a_gas_per_epoch:7,..config},tasks,150,8).unwrap();
        poor.tick(None);
        let r=poor.tick(Some(read("a/0")));
        assert_eq!(r,ActionResult::fail(Code::Gas));assert_eq!(a(&poor),7);
        assert_eq!(escrow(&poor).remaining_a,150);
    }


    #[test]
    fn s29_compact_quote_contract_native(){
        let config=Config{seed:129,a_gas_per_epoch:200,epoch_length:16,..Config::default()};
        let second=TransformTask{task_id:"b".into(),examples:vec![],held_out_input:vec![7;8]};
        let read=|suffix:&str|Action::Read{actor:ORGANISM_ID,key:format!("/sys/transform-price-u16/{suffix}")};
        for price in [1u64,158,159,255,256,400,4095]{
            let tasks=vec![(task(),price),(second.clone(),1)];
            let mut w=World::with_priced_transform_sequence(config.clone(),tasks,4096,8).unwrap();
            w.tick(None);
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            for (suffix,expected) in [("a/0",Some(price)),("a/1",None),("b/0",None)]{
                let before=a(&w);let payer=escrow(&w).remaining_a;
                let r=w.tick(Some(read(suffix)));
                assert_eq!(r,replay.tick(Some(read(suffix))));
                assert_eq!(r.gas_charged,8);assert_eq!(r.a_credited,0);
                assert_eq!(a(&w),before-8);assert_eq!(escrow(&w).remaining_a,payer);
                match expected{
                    Some(p)=>{assert_eq!(r.code,Code::Ok);assert_eq!(r.payload,(p as u16).to_le_bytes().to_vec());},
                    None=>{assert_eq!(r.code,Code::NotFound);assert!(r.payload.is_empty());}
                }
            }
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        }
        let tasks=vec![(task(),100),(second,40)];
        let mut legacy=World::with_transform_sequence(config.clone(),tasks.clone(),150).unwrap();
        legacy.tick(None);assert_eq!(legacy.tick(Some(read("a/0"))),ActionResult::fail(Code::NotFound));
        let mut poor=World::with_priced_transform_sequence(Config{a_gas_per_epoch:7,..config},tasks,150,8).unwrap();
        poor.tick(None);assert_eq!(poor.tick(Some(read("a/0"))),ActionResult::fail(Code::Gas));assert_eq!(a(&poor),7);
        println!("S29_COMPACT_CONTRACT seven_prices=true stale=true legacy=true gas=true replay=true");
    }


    #[test]
    fn s29_earned_resource_epoch_boundary_native(){
        // No new mechanism: directly test legacy grant reset against real
        // escrow earnings. Scripted actions isolate the accounting boundary.
        for price in [1u64,100]{
            let mut w=World::with_transform_escrow(Config{seed:129,a_gas_per_epoch:25,
                epoch_length:4,..Config::default()},task(),150,price).unwrap();
            w.tick(None);assert_eq!(a(&w),25);
            let paid=w.tick(Some(write(ORGANISM_ID,KEY,receipt(),None)));
            assert_eq!(paid.code,Code::Ok);assert_eq!(paid.gas_charged,20);
            assert_eq!(paid.a_credited,price);assert_eq!(a(&w),5+price);
            assert_eq!(escrow(&w).remaining_a,150-price);
            w.schedule_event(ScheduledEvent{due_step:4,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            let snapshot=w.snapshot_json();
            for delayed in [false,true]{
                let mut branch=World::from_snapshot(&snapshot).unwrap();
                let mut replay=World::from_snapshot(&snapshot).unwrap();
                let work="/self/epoch-boundary-work";
                let mut fee=0u64;let mut expired=0u64;let mut work_code=None;
                for step in 2..6{
                    assert_eq!(branch.state.step,step);
                    let before=a(&branch);
                    let action=if step==if delayed{4}else{3}{
                        Some(write(ORGANISM_ID,work,vec![0x37;280],None))
                    }else{None};
                    let attempted=action.is_some();
                    let r=branch.tick(action.clone());assert_eq!(r,replay.tick(action));
                    assert_eq!(r.a_credited,0);
                    assert_eq!(escrow(&branch).remaining_a,150-price);
                    if step==4{
                        expired=before;
                        assert_eq!(branch.state.nodes[&NodeId::A].gas_per_epoch,0);
                        assert_eq!(a(&branch),0);assert_eq!(r.gas_charged,0);
                    }else{assert_eq!(a(&branch),before-r.gas_charged-r.vm_gas_charged);}
                    fee+=r.gas_charged+r.vm_gas_charged;
                    if attempted{work_code=Some(r.code);}
                }
                let success=price==100 && !delayed;
                assert_eq!(work_code,Some(if success{Code::Ok}else{Code::Gas}));
                assert_eq!(branch.state.objects.contains_key(work),success);
                assert_eq!(fee,if success{20}else{0});
                assert_eq!(expired,5+price-fee);
                assert_eq!(a(&branch),0);assert_eq!(branch.state.objects[KEY].bytes,receipt());
                assert_eq!(escrow(&branch).paid_a,price);
                assert_eq!(branch.snapshot_json(),replay.snapshot_json());assert_eq!(branch.audit,replay.audit);
                println!("S29_EARNED_EPOCH price={price} delayed={delayed} initial_A=25 settlement_fee=20 credit={price} work_fee={fee} expired_A={expired} final_A=0 payer_A={} work={work_code:?} replay=true",escrow(&branch).remaining_a);
            }
        }
    }


    #[test]
    fn s29_persistent_earned_account_native(){
        for persistent in [false,true]{
        for price in [1u64,100]{
            let config=Config{seed:129,a_gas_per_epoch:25,epoch_length:4,..Config::default()};
            let mut w=if persistent{World::with_persistent_transform_escrow(config,task(),150,price)}
                else{World::with_transform_escrow(config,task(),150,price)}.unwrap();
            w.tick(None);
            let r=w.tick(Some(write(ORGANISM_ID,KEY,receipt(),None)));
            assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);assert_eq!(r.a_credited,price);
            assert_eq!(a(&w),5+price);assert_eq!(escrow(&w).earned_a,persistent.then_some(price));
            w.schedule_event(ScheduledEvent{due_step:4,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut fees=0;
            for step in 2..10{
                assert_eq!(w.state.step,step);
                let before=a(&w);
                let earned=escrow(&w).earned_a;
                let action=match step{
                    3=>Some(write(ORGANISM_ID,"/self/early-work",vec![7;280],None)),
                    4=>Some(write(ORGANISM_ID,"/self/late-work",vec![8;280],None)),
                    8=>Some(write(ORGANISM_ID,"/self/second-late-work",vec![9;280],None)),
                    _=>None
                };
                let r=w.tick(action.clone());assert_eq!(r,replay.tick(action));
                assert_eq!(r.a_credited,0);assert_eq!(escrow(&w).remaining_a,150-price);
                let fresh=if step==4||step==8{earned.unwrap_or(0)}else{before};
                assert_eq!(a(&w),fresh-r.gas_charged-r.vm_gas_charged);
                if persistent{
                    let earned=earned.unwrap();
                    let quota=fresh-earned;
                    assert_eq!(escrow(&w).earned_a,Some(earned-r.gas_charged.saturating_sub(quota)));
                }else{assert_eq!(escrow(&w).earned_a,None);}
                if [3,4,8].contains(&step){
                    let ok=price==100 && (step==3||persistent);
                    assert_eq!(r.code,if ok{Code::Ok}else{Code::Gas});
                    assert_eq!(r.gas_charged,if ok{20}else{0});
                }
                fees+=r.gas_charged;
            }
            let expected=if persistent{if price==100{45}else{1}}else{0};
            assert_eq!(a(&w),expected);assert_eq!(escrow(&w).earned_a,persistent.then_some(expected));
            assert_eq!(fees,if price==100{if persistent{60}else{20}}else{0});
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_PERSISTENT_EARNED persistent={persistent} price={price} work_fees={fees} final_A={} earned={:?} payer_A={} replay=true",a(&w),escrow(&w).earned_a,escrow(&w).remaining_a);
        }
        }
        // Arithmetic edge: initial grant clipped only when a later epoch
        // combines u64::MAX configured quota and actual earned resources.
        let mut w=World::with_persistent_transform_escrow(Config{seed:129,a_gas_per_epoch:25,
            epoch_length:4,..Config::default()},task(),150,100).unwrap();
        w.tick(None);assert_eq!(w.tick(Some(write(ORGANISM_ID,KEY,receipt(),None))).a_credited,100);
        w.schedule_event(ScheduledEvent{due_step:4,priority:0,sequence_no:0,
            kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:u64::MAX}});
        while w.state.step<=4{w.tick(None);}
        assert_eq!(a(&w),u64::MAX);assert_eq!(escrow(&w).earned_a,Some(100));
        assert_eq!(w.spend_a_gas(u64::MAX),Ok(()));assert_eq!(a(&w),0);
        assert_eq!(escrow(&w).earned_a,Some(0));
        assert_eq!(w.spend_a_gas(1),Err(Code::Gas));
        assert_eq!(escrow(&w).earned_a,Some(0));
        println!("S29_PERSISTENT_EARNED overflow_clip=true atomic_rejection=true");
    }

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


    #[test]
    fn s29_autonomous_persistent_earned_native(){
        let p=root();let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        for (id,input) in [("a",vec![44,7,2]),("b",vec![13,9,4,8,2])]{
        for price in [1u64,1000]{
        for persistent in [false,true]{
            let task=TransformTask{task_id:id.into(),examples:vec![],held_out_input:input.clone()};
            let key=format!("{OUTCOME}{:02x}",id.as_bytes()[0]);
            let config=Config{seed:129,a_gas_per_epoch:12000,epoch_length:16,..Config::default()};
            let mut w=if persistent{World::with_persistent_transform_escrow(config,task,1000,price)}
                else{World::with_transform_escrow(config,task,1000,price)}.unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            // Begin two ticks before boundary. The unchanged root reads then
            // submits, and its next autonomous action occurs at/after step32.
            while w.state.step<30{w.tick(None);}
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=1000u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut submitted_at=None;let mut worked_at=None;let mut post_fees=0u64;let mut gas=0;
            for step in 30..44{
                assert_eq!(w.state.step,step);
                if step==32{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);
                assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,1256-fees-expired);
                let e=w.audit.last().unwrap();
                if r.a_credited>0{
                    assert_eq!(r.a_credited,price);assert!(submitted_at.is_none());
                    assert_eq!(step,31);assert_eq!(r.code,Code::Ok);
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    submitted_at=Some(step);
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/earned-workspace")&&r.code==Code::Ok{
                    assert!(step>=32);assert!(worked_at.is_none());worked_at=Some(step);
                }
            }
            assert_eq!(credit,price);assert_eq!(submitted_at,Some(31));
            let receipt=&w.state.objects[&key];assert_eq!(receipt.owner,ORGANISM_ID);
            assert_eq!(receipt.version,0);assert_eq!(receipt.bytes.len(),280);
            assert_eq!(receipt.bytes[0],input.len() as u8);
            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&input).as_slice());
            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
            let success=persistent&&price==1000;
            assert_eq!(worked_at.is_some(),success);
            assert_eq!(w.state.objects.contains_key("/self/earned-workspace"),success);
            if success{
                assert_eq!(w.state.objects["/self/earned-workspace"].bytes,vec![0x37;2048]);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(1)));
                assert!(post_fees>0);assert_eq!(gas,0);
            }
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_AUTONOMOUS_PERSISTENT id={id} persistent={persistent} price={price} root={hash} root_B={} install_A={install} credit={credit} fees={fees} expired_A={expired} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} submitted_at={submitted_at:?} worked_at={worked_at:?} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
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
    fn s29_zero_grant_productive_chain_native(){
        let p=chain_root();let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [1u64,200,1000]{
        for persistent in [false,true]{
            let tasks=chain_tasks(first);
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            let mut w=if persistent{World::with_persistent_transform_sequence(config,tasks.clone(),1400)}
                else{World::with_transform_sequence(config,tasks.clone(),1400)}.unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=1400u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..54{
                assert_eq!(w.state.step,step);
                if step==32||step==48{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else{at48=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;post_credit+=r.a_credited;
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,1656-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if !persistent||first==1{assert_eq!(settlements.len(),1);assert_eq!(post_credit,0);}
            if persistent&&first==1000{
                assert_eq!(settlements.len(),3);assert_eq!(post_credit,400);
                assert_eq!(gas,0);assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert_eq!(at48,balance);
            }
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_ZERO_GRANT_CHAIN first={first} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }

    #[test]
    fn s29_productive_break_even_native(){
        let p=chain_root();let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [200u64,1000]{
        for later in [150u64,200,203,204,250]{
            let persistent=true;
            let mut tasks=chain_tasks(first);tasks[1].1=later;tasks[2].1=later;
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            let mut w=if persistent{World::with_persistent_transform_sequence(config,tasks.clone(),1600)}
                else{World::with_transform_sequence(config,tasks.clone(),1600)}.unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=1600u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..54{
                assert_eq!(w.state.step,step);
                if step==32||step==48{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else{at48=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;post_credit+=r.a_credited;
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,1856-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if first==1000||later>=200{
                assert_eq!(settlements.len(),3);assert_eq!(post_credit,2*later);
                assert_eq!(gas,0);assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(3)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert_eq!(at48,balance);
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_BREAK_EVEN first={first} later={later} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }

    #[test]
    fn s29_long_finite_production_native(){
        let mut p=chain_root();
        let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
        if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{assert_eq!(*limit,3);*limit=12;}
        else{panic!("expected bounded loop");}
        p.validate().expect("typed 12-slot root");
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [200u64,1000]{
        for later in [150u64,204,250]{
            let persistent=true;
            let templates=chain_tasks(first);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{later})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;post_credit+=r.a_credited;
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,11*later);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,later);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                if first==1000&&later==250{assert_eq!(payer,0);}
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_LONG_PRODUCTION first={first} later={later} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }

    #[test]
    fn s29_price_order_liquidity_native(){
        let mut p=chain_root();
        let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
        if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{assert_eq!(*limit,3);*limit=12;}
        else{panic!("expected bounded loop");}
        p.validate().expect("typed 12-slot root");
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [200u64,1000]{
        for tape in ["FLAT","LOW_FIRST","HIGH_FIRST","INTERLEAVED"]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_PRICE_ORDER first={first} tape={tape} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }

    #[test]
    fn s29_prefix_capital_boundary_native(){
        let mut p=chain_root();
        let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
        if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{assert_eq!(*limit,3);*limit=12;}
        else{panic!("expected bounded loop");}
        p.validate().expect("typed 12-slot root");
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for (tape,predicted) in [("FLAT",186u64),("LOW_FIRST",327),("HIGH_FIRST",186),("INTERLEAVED",222)]{
        for first in [predicted-1,predicted,predicted+1]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{post_fees+=fee;post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_PREFIX_CAPITAL first={first} tape={tape} predicted={predicted} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }


    // A paid history learner, not a predictor of unseen future opportunities.
    // 90=settled count, 91=observed cumulative post-bootstrap fees,
    // 92=observed subsequent credit, 93=maximum pre-credit deficit.
    fn reserve_history_root()->Program{
        let mut p=chain_root();let f=&mut p.functions[0];
        let i=|id,op|Instruction{id,op};
        let ctx=(90..=93).map(|id|Param{id,ty:Type::U32}).collect::<Vec<_>>();
        let mut fb=ctx.clone();fb.push(Param{id:2,ty:Type::Bytes});
        let args=vec![90,91,92,93];
        let fba=vec![90,91,92,93,2];
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.extend([i(91,Op::ConstU32(0)),i(92,Op::ConstU32(0)),i(93,Op::ConstU32(0))]);
            }else{
                b.params.splice(1..1,ctx[1..].iter().cloned());
            }
            let extend=|a:&mut Vec<u32>|{a.splice(1..1,[91,92,93]);};
            match &mut b.terminator{
                Terminator::Jump{args,..}=>extend(args),
                Terminator::Branch{true_args,false_args,..}=>{extend(true_args);extend(false_args);},
                Terminator::Yield{next_args,..}=>extend(next_args),
                Terminator::RepeatBounded{limit,body_args,done_args,..}=>{
                    *limit=12;extend(body_args);extend(done_args);
                },
                Terminator::Return{..}=>{},
                _=>panic!("unexpected chain graph")
            }
        }
        let offer=f.blocks.iter_mut().find(|b|b.id=="offer").unwrap();
        if let Terminator::Branch{if_true,..}=&mut offer.terminator{*if_true="measure_read_slot".into();}
        else{panic!("offer branch");}
        let paid=f.blocks.iter_mut().find(|b|b.id=="chain_credit").unwrap();
        if let Terminator::Branch{if_false,false_args,..}=&mut paid.terminator{
            *if_false="measure_paid_slot".into();false_args.extend([2,11]);
        }else{panic!("credit branch");}
        let done=f.blocks.iter_mut().find(|b|b.id=="failed").unwrap();
        done.instructions=vec![i(10,Op::ConstU32(12)),i(11,Op::EqU32(90,10))];
        done.terminator=Terminator::Branch{condition:11,if_true:"save_history".into(),true_args:args.clone(),
            if_false:"return_count".into(),false_args:args.clone()};
        let mut paid_params=fb.clone();paid_params.push(Param{id:3,ty:Type::U32});
        let paid_args=vec![90,91,92,93,2,3];
        let mut update_params=ctx.clone();update_params.push(Param{id:3,ty:Type::U32});
        // All quantities bounded by the finite <=4096 payer + 256 bootstrap,
        // and actual fees; no underflow in the deficit branch.
        f.blocks.extend([
            Block{id:"measure_read_slot".into(),params:fb.clone(),instructions:vec![
                i(10,Op::ConstU32(0)),i(11,Op::EqU32(90,10))],
                terminator:Terminator::Branch{condition:11,if_true:"compute".into(),true_args:fba.clone(),
                    if_false:"measure_read".into(),false_args:fba.clone()}},
            Block{id:"measure_read".into(),params:fb.clone(),instructions:vec![
                i(10,literal(b"gas_charged")),i(11,Op::JsonFieldU32(2,10)),
                i(12,literal(b"vm_gas_charged")),i(13,Op::JsonFieldU32(2,12)),
                i(14,Op::Add(11,13)),i(15,Op::Add(91,14))],
                terminator:Terminator::Jump{target:"compute".into(),args:vec![90,15,92,93,2]}},
            Block{id:"measure_paid_slot".into(),params:paid_params.clone(),instructions:vec![
                i(10,Op::ConstU32(0)),i(11,Op::EqU32(90,10))],
                terminator:Terminator::Branch{condition:11,if_true:"chain_advance".into(),true_args:args.clone(),
                    if_false:"measure_paid".into(),false_args:paid_args}},
            Block{id:"measure_paid".into(),params:paid_params,instructions:vec![
                i(10,literal(b"gas_charged")),i(11,Op::JsonFieldU32(2,10)),
                i(12,literal(b"vm_gas_charged")),i(13,Op::JsonFieldU32(2,12)),
                i(14,Op::Add(11,13)),i(15,Op::Add(91,14)),
                i(16,Op::Add(92,93)),i(17,Op::ConstU32(1)),i(18,Op::Add(16,17)),
                i(19,Op::Div(15,18)),i(20,Op::ConstU32(0)),i(21,Op::EqU32(19,20))],
                terminator:Terminator::Branch{condition:21,if_true:"keep_reserve".into(),true_args:vec![90,15,92,93,3],
                    if_false:"raise_reserve".into(),false_args:vec![90,15,92,93,3]}},
            Block{id:"keep_reserve".into(),params:update_params.clone(),instructions:vec![i(10,Op::Add(92,3))],
                terminator:Terminator::Jump{target:"chain_advance".into(),args:vec![90,91,10,93]}},
            Block{id:"raise_reserve".into(),params:update_params,instructions:vec![
                i(10,Op::Sub(91,92)),i(11,Op::Add(92,3))],
                terminator:Terminator::Jump{target:"chain_advance".into(),args:vec![90,91,11,10]}},
            Block{id:"save_history".into(),params:ctx.clone(),instructions:vec![
                i(10,Op::PackU16Le(93)),i(11,Op::PackU16Le(91)),i(12,Op::PackU16Le(92)),
                i(13,Op::Concat(10,11)),i(14,Op::Concat(13,12)),
                i(15,literal(b"/self/learned/production-reserve-le")),i(16,Op::ConstU32(u32::MAX))],
                terminator:Terminator::Yield{call:WorldCall::Write,args:vec![15,14,16],next:"history_saved".into(),next_args:args.clone()}},
            Block{id:"history_saved".into(),params:fb,instructions:vec![],terminator:Terminator::Return{value:90}},
            Block{id:"return_count".into(),params:ctx,instructions:vec![],terminator:Terminator::Return{value:90}}
        ]);
        p.validate().expect("typed reserve history root");p
    }

    #[test]
    fn s29_reserve_history_native(){
        let p=reserve_history_root();
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for tape in ["FLAT","LOW_FIRST","HIGH_FIRST","INTERLEAVED"]{
        for first in [1000u64,1500]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut observed_deficit=0u64;let mut expected_history=None;
            let mut saved_at=None;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{
                    post_fees+=fee;
                    observed_deficit=observed_deficit.max(post_fees.saturating_sub(post_credit));
                    post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/learned/production-reserve-le") && r.code==Code::Ok{
                    assert!(expected_history.is_some());assert!(saved_at.is_none());saved_at=Some(step);
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                    if slot==11{expected_history=Some([observed_deficit,post_fees,post_credit]);}
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let learned=w.state.objects.get("/self/learned/production-reserve-le");
            if let Some(object)=learned{
                let expected=expected_history.expect("complete history before save");
                assert_eq!(object.owner,ORGANISM_ID);assert_eq!(object.version,0);
                let bytes=expected.iter().flat_map(|n|u16::try_from(*n).unwrap().to_le_bytes()).collect::<Vec<_>>();
                assert_eq!(object.bytes,bytes);assert!(saved_at.is_some());
            }else{assert!(saved_at.is_none());}
            if first==1500{assert_eq!(settlements.len(),12);assert!(learned.is_some());}
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_RESERVE_HISTORY first={first} tape={tape} saved_at={saved_at:?} expected_history={expected_history:?} learned={} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",learned.is_some(),raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }

    fn reserve_total_fee_root()->Program{
        let mut p=reserve_history_root();
        for id in ["measure_read","measure_paid"]{
            let b=p.functions[0].blocks.iter_mut().find(|b|b.id==id).unwrap();
            assert_eq!(b.instructions[0].id,10);
            assert_eq!(b.instructions[4].id,14);
            b.instructions.splice(0..5,[
                Instruction{id:10,op:literal(b"f")},
                Instruction{id:14,op:Op::JsonFieldU32(2,10)}
            ]);
        }
        p.validate().expect("typed total fee history root");p
    }

    #[test]
    fn s29_reserve_fee_representation_native(){
        for mode in 0..4{
        let p=if mode<2{
            let mut p=chain_root();
            let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
            if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{*limit=12;}
            else{panic!("expected bounded loop");}
            p
        }else if mode==2{reserve_history_root()}else{reserve_total_fee_root()};
        p.validate().unwrap();
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        let first=1000u64;
        for tape in ["FLAT","LOW_FIRST","HIGH_FIRST","INTERLEAVED"]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            if mode==1||mode==3{w.state.transform_escrow.as_mut().unwrap().fee_total_feedback=Some(true);}
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut observed_deficit=0u64;let mut expected_history=None;
            let mut saved_at=None;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{
                    post_fees+=fee;
                    observed_deficit=observed_deficit.max(post_fees.saturating_sub(post_credit));
                    post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/learned/production-reserve-le") && r.code==Code::Ok{
                    assert!(expected_history.is_some());assert!(saved_at.is_none());saved_at=Some(step);
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                    if slot==11{expected_history=Some([observed_deficit,post_fees,post_credit]);}
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let learned=w.state.objects.get("/self/learned/production-reserve-le");
            if let Some(object)=learned{
                let expected=expected_history.expect("complete history before save");
                assert_eq!(object.owner,ORGANISM_ID);assert_eq!(object.version,0);
                let bytes=expected.iter().flat_map(|n|u16::try_from(*n).unwrap().to_le_bytes()).collect::<Vec<_>>();
                assert_eq!(object.bytes,bytes);assert!(saved_at.is_some());
            }else{assert!(saved_at.is_none());}
            if mode<2{assert!(learned.is_none());}
            if mode==2{assert_eq!(settlements.len(),12);assert!(learned.is_some());}
            if mode==3&&settlements.len()==12{assert!(learned.is_some());}
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_RESERVE_FEE_REP mode={mode} first={first} tape={tape} saved_at={saved_at:?} expected_history={expected_history:?} learned={} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",learned.is_some(),raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }


    // One exact sample, not a full-history estimator. No new feedback field.
    fn selective_cost_sample_root(slot:u32)->Program{
        assert!((1..12).contains(&slot));
        let mut p=chain_root();let f=&mut p.functions[0];
        let i=|id,op|Instruction{id,op};
        let ctx=(90..=92).map(|id|Param{id,ty:Type::U32}).collect::<Vec<_>>();
        let mut fb=ctx.clone();fb.push(Param{id:2,ty:Type::Bytes});
        let args=vec![90,91,92];let fba=vec![90,91,92,2];
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.extend([i(91,Op::ConstU32(0)),i(92,Op::ConstU32(0))]);
            }else{b.params.splice(1..1,ctx[1..].iter().cloned());}
            let extend=|a:&mut Vec<u32>|{a.splice(1..1,[91,92]);};
            match &mut b.terminator{
                Terminator::Jump{args,..}=>extend(args),
                Terminator::Branch{true_args,false_args,..}=>{extend(true_args);extend(false_args);},
                Terminator::Yield{next_args,..}=>extend(next_args),
                Terminator::RepeatBounded{limit,body_args,done_args,..}=>{
                    *limit=12;extend(body_args);extend(done_args);
                },
                Terminator::Return{..}=>{},
                _=>panic!("unexpected chain graph")
            }
        }
        let offer=f.blocks.iter_mut().find(|b|b.id=="offer").unwrap();
        if let Terminator::Branch{if_true,..}=&mut offer.terminator{*if_true="sample_read_slot".into();}
        else{panic!("offer branch");}
        let paid=f.blocks.iter_mut().find(|b|b.id=="chain_credit").unwrap();
        if let Terminator::Branch{if_false,false_args,..}=&mut paid.terminator{
            *if_false="sample_paid_slot".into();false_args.extend([2,11]);
        }else{panic!("credit branch");}
        let done=f.blocks.iter_mut().find(|b|b.id=="failed").unwrap();
        done.instructions=vec![i(10,Op::ConstU32(12)),i(11,Op::EqU32(90,10))];
        done.terminator=Terminator::Branch{condition:11,if_true:"save_sample".into(),true_args:args.clone(),
            if_false:"return_count".into(),false_args:args.clone()};
        let mut paid_params=fb.clone();paid_params.push(Param{id:3,ty:Type::U32});
        f.blocks.extend([
            Block{id:"sample_read_slot".into(),params:fb.clone(),instructions:vec![
                i(10,Op::ConstU32(slot)),i(11,Op::EqU32(90,10))],
                terminator:Terminator::Branch{condition:11,if_true:"sample_read".into(),true_args:fba.clone(),
                    if_false:"compute".into(),false_args:fba.clone()}},
            Block{id:"sample_read".into(),params:fb.clone(),instructions:vec![
                i(10,literal(b"gas_charged")),i(11,Op::JsonFieldU32(2,10)),
                i(12,literal(b"vm_gas_charged")),i(13,Op::JsonFieldU32(2,12)),
                i(14,Op::Add(11,13))],
                terminator:Terminator::Jump{target:"compute".into(),args:vec![90,14,92,2]}},
            Block{id:"sample_paid_slot".into(),params:paid_params.clone(),instructions:vec![
                i(10,Op::ConstU32(slot)),i(11,Op::EqU32(90,10))],
                terminator:Terminator::Branch{condition:11,if_true:"sample_paid".into(),true_args:vec![90,91,92,2,3],
                    if_false:"chain_advance".into(),false_args:args.clone()}},
            Block{id:"sample_paid".into(),params:paid_params,instructions:vec![
                i(10,literal(b"gas_charged")),i(11,Op::JsonFieldU32(2,10)),
                i(12,literal(b"vm_gas_charged")),i(13,Op::JsonFieldU32(2,12)),
                i(14,Op::Add(11,13)),i(15,Op::Add(91,14))],
                terminator:Terminator::Jump{target:"chain_advance".into(),args:vec![90,15,3]}},
            Block{id:"save_sample".into(),params:ctx.clone(),instructions:vec![
                i(10,Op::ConstU32(slot)),i(11,Op::PackU16Le(10)),i(12,Op::PackU16Le(91)),
                i(13,Op::PackU16Le(92)),i(14,Op::Concat(11,12)),i(15,Op::Concat(14,13)),
                i(16,literal(b"/self/learned/production-sample-le")),i(17,Op::ConstU32(u32::MAX))],
                terminator:Terminator::Yield{call:WorldCall::Write,args:vec![16,15,17],next:"sample_saved".into(),next_args:args.clone()}},
            Block{id:"sample_saved".into(),params:fb,instructions:vec![],terminator:Terminator::Return{value:90}},
            Block{id:"return_count".into(),params:ctx,instructions:vec![],terminator:Terminator::Return{value:90}}
        ]);
        p.validate().expect("typed selective cost sample");p
    }

    #[test]
    fn s29_selective_cost_sample_native(){
        for sample_slot in [0u32,1,11]{
        let p=if sample_slot==0{
            let mut p=chain_root();
            let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
            if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{*limit=12;}
            else{panic!("expected bounded loop");}
            p
        }else{selective_cost_sample_root(sample_slot)};
        p.validate().unwrap();
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        let first=1000u64;
        for tape in ["FLAT","LOW_FIRST","HIGH_FIRST","INTERLEAVED"]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut expected_history=None;let mut sample_read_fee=None;
            let mut saved_at=None;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{
                    post_fees+=fee;
                    post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                    if sample_slot!=0 && slot==sample_slot as usize{assert!(sample_read_fee.is_none());sample_read_fee=Some(fee);}
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/learned/production-sample-le") && r.code==Code::Ok{
                    assert!(expected_history.is_some());assert!(saved_at.is_none());saved_at=Some(step);
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                    if sample_slot!=0 && slot==sample_slot as usize{
                        expected_history=Some([sample_slot as u64,sample_read_fee.expect("sample read")+fee,r.a_credited]);
                    }
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let learned=w.state.objects.get("/self/learned/production-sample-le");
            if let Some(object)=learned{
                let expected=expected_history.expect("complete history before save");
                assert_eq!(object.owner,ORGANISM_ID);assert_eq!(object.version,0);
                let bytes=expected.iter().flat_map(|n|u16::try_from(*n).unwrap().to_le_bytes()).collect::<Vec<_>>();
                assert_eq!(object.bytes,bytes);assert!(saved_at.is_some());
            }else{assert!(saved_at.is_none());}
            if sample_slot==0{assert_eq!(settlements.len(),12);assert!(learned.is_none());}
            else if settlements.len()==12{assert!(learned.is_some());}
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_SELECTIVE_SAMPLE sample_slot={sample_slot} first={first} tape={tape} saved_at={saved_at:?} expected_history={expected_history:?} learned={} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",learned.is_some(),raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
    }


    // Once-only, in-episode use of the observed cost; no future price input.
    // The sham pays identical observation and decision arithmetic, but continues.
    fn sample_liquidity_gate_root(use_sample:bool)->Program{
        let mut p=selective_cost_sample_root(1);
        let f=&mut p.functions[0];
        let i=|id,op|Instruction{id,op};
        let ctx=(90..=92).map(|id|Param{id,ty:Type::U32}).collect::<Vec<_>>();
        let mut fb=ctx.clone();fb.push(Param{id:2,ty:Type::Bytes});
        let args=vec![90,91,92];let fba=vec![90,91,92,2];
        let sampled=f.blocks.iter_mut().find(|b|b.id=="sample_paid").unwrap();
        if let Terminator::Jump{target,..}=&mut sampled.terminator{*target="sample_gate_read".into();}
        else{panic!("sample paid jump");}
        f.blocks.extend([
            Block{id:"sample_gate_read".into(),params:ctx.clone(),
                instructions:vec![i(10,literal(b"/sys/resource-status"))],
                terminator:Terminator::Yield{call:WorldCall::Read,args:vec![10],next:"sample_gate_reply".into(),next_args:args.clone()}},
            Block{id:"sample_gate_reply".into(),params:fb.clone(),instructions:vec![
                i(10,literal(b"code")),i(11,Op::JsonFieldString(2,10)),i(12,literal(b"Ok")),i(13,Op::EqBytes(11,12))],
                terminator:Terminator::Branch{condition:13,if_true:"sample_gate_decide".into(),true_args:fba,
                    if_false:"sample_gate_stop".into(),false_args:args.clone()}},
            Block{id:"sample_gate_decide".into(),params:fb,instructions:vec![
                i(10,literal(b"payload")),i(11,Op::JsonFieldBytes(2,10)),
                i(12,literal(b"a_available")),i(13,Op::JsonFieldU32(11,12)),
                i(14,Op::Div(13,91)),i(15,Op::ConstU32(0)),i(16,Op::EqU32(14,15))],
                terminator:Terminator::Branch{condition:16,if_true:if use_sample{"sample_gate_stop".into()}else{"chain_advance".into()},
                    true_args:args.clone(),if_false:"chain_advance".into(),false_args:args.clone()}},
            Block{id:"sample_gate_stop".into(),params:ctx,instructions:vec![i(10,Op::ConstU32(1)),i(11,Op::Add(90,10))],
                terminator:Terminator::Return{value:11}}
        ]);
        p.validate().expect("typed sampled liquidity decision");p
    }

    #[test]
    fn s29_sample_liquidity_gate_native(){
        for mode in 0..4{
        let sample_slot=if mode==0{0u32}else{1};
        let p=if mode==0{
            let mut p=chain_root();
            let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
            if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{*limit=12;}
            else{panic!("expected bounded loop");}
            p
        }else if mode==1{selective_cost_sample_root(1)}else{sample_liquidity_gate_root(mode==3)};
        p.validate().unwrap();
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [250u64,350,1000]{
        for tape in ["FLAT","LOW_FIRST","HIGH_FIRST","INTERLEAVED"]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Same sum (2244A) across all 11 later offers; the mixed
            // tapes use exactly the same multiset, changing only order.
            let prices:[u64;11]=match tape{
                "FLAT"=>[204;11],
                "LOW_FIRST"=>[150,150,150,150,204,204,204,258,258,258,258],
                "HIGH_FIRST"=>[258,258,258,258,204,204,204,150,150,150,150],
                _=>[150,258,150,258,204,150,258,204,150,258,204]
            };
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut expected_history=None;let mut sample_read_fee=None;
            let mut saved_at=None;let mut gate_status=None;let mut gate_step=None;let mut after_gate_read=false;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{
                    post_fees+=fee;
                    post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                    if sample_slot!=0 && slot==sample_slot as usize{assert!(sample_read_fee.is_none());sample_read_fee=Some(fee);}
                }
                if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                    assert!(mode>=2);assert!(gate_status.is_none());assert_eq!(settlements.len(),2);
                    let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                    assert_eq!(v["a_available"].as_u64(),Some(balance));
                    gate_status=Some(balance);gate_step=Some(step);
                }
                if gate_step.is_some_and(|s|step>s)&&matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER){
                    after_gate_read=true;
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/learned/production-sample-le") && r.code==Code::Ok{
                    assert!(expected_history.is_some());assert!(saved_at.is_none());saved_at=Some(step);
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                    if sample_slot!=0 && slot==sample_slot as usize{
                        expected_history=Some([sample_slot as u64,sample_read_fee.expect("sample read")+fee,r.a_credited]);
                    }
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let learned=w.state.objects.get("/self/learned/production-sample-le");
            if let Some(object)=learned{
                let expected=expected_history.expect("complete history before save");
                assert_eq!(object.owner,ORGANISM_ID);assert_eq!(object.version,0);
                let bytes=expected.iter().flat_map(|n|u16::try_from(*n).unwrap().to_le_bytes()).collect::<Vec<_>>();
                assert_eq!(object.bytes,bytes);assert!(saved_at.is_some());
            }else{assert!(saved_at.is_none());}
            if sample_slot==0{assert!(learned.is_none());}
            else if settlements.len()==12{assert!(learned.is_some());}
            if first==1000{assert_eq!(settlements.len(),12);}
            if let (Some(status),Some(sample))=(gate_status,expected_history){
                if mode==3&&status<sample[1]{
                    assert!(!after_gate_read);assert_eq!(settlements.len(),2);
                }
                if first==1000{assert!(after_gate_read);assert!(status>=sample[1]);}
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_SAMPLE_GATE mode={mode} sample_slot={sample_slot} first={first} gate_status={gate_status:?} gate_step={gate_step:?} after_gate_read={after_gate_read} tape={tape} saved_at={saved_at:?} expected_history={expected_history:?} learned={} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",learned.is_some(),raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
        }
    }


    // Matched arithmetic, differing only in whether the divisor is the
    // measured cost, a frozen calibrated constant, or ignored for sham.
    fn sample_transfer_gate_root(mode:u32)->Program{
        assert!((1..=3).contains(&mode));
        let mut p=sample_liquidity_gate_root(mode!=1);
        let b=p.functions[0].blocks.iter_mut().find(|b|b.id=="sample_gate_decide").unwrap();
        let pos=b.instructions.iter().position(|i|i.id==14).unwrap();
        assert_eq!(b.instructions[pos].op,Op::Div(13,91));
        b.instructions.insert(pos,Instruction{id:17,op:Op::ConstU32(239)});
        if mode==2{b.instructions[pos+1].op=Op::Div(13,17);}
        p.validate().expect("matched cost transfer gate");p
    }

    #[test]
    fn s29_sample_threshold_transfer_native(){
        for mode in 0..4{
        let sample_slot=if mode==0{0u32}else{1};
        let p=if mode==0{
            let mut p=chain_root();
            let header=p.functions[0].blocks.iter_mut().find(|b|b.id=="chain_header").unwrap();
            if let Terminator::RepeatBounded{limit,..}=&mut header.terminator{*limit=12;}
            else{panic!("expected bounded loop");}
            p
        }else{sample_transfer_gate_root(mode)};
        p.validate().unwrap();
        let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
        let mut install=None;
        for first in [350u64,450,550,1000]{
        for tape in ["SHORT_SHORT","LONG_LONG","SHORT_LONG","LONG_SHORT"]{
            let persistent=true;
            let templates=chain_tasks(first);
            // Freeze the old flat price and change only input cost at
            // sampled slot1 and decision-target slot2. No future observation.
            let prices=[204u64;11];
            assert_eq!(prices.iter().sum::<u64>(),2244);
            let tasks=(0..12usize).map(|slot|{
                let mut task=templates[slot%3].0.clone();
                task.task_id=char::from(b'a'+slot as u8).to_string();
                if (slot==1&&matches!(tape,"LONG_LONG"|"LONG_SHORT")) ||
                    (slot==2&&matches!(tape,"LONG_LONG"|"SHORT_LONG")){
                    task.held_out_input=vec![255;96];
                }
                (task,if slot==0{first}else{prices[slot-1]})
            }).collect::<Vec<_>>();
            let config=Config{seed:129,epoch_length:16,a_gas_per_epoch:12000,..Config::default()};
            assert!(matches!(World::with_transform_sequence(config.clone(),tasks.clone(),3750),Err(Code::BadRequest)));
            let mut duplicate=tasks.clone();duplicate[11].0.task_id=duplicate[0].0.task_id.clone();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),duplicate,3750),Err(Code::BadRequest)));
            let mut too_long=tasks.clone();let mut extra=tasks[0].clone();extra.0.task_id="m".into();too_long.push(extra);
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),too_long,3750),Err(Code::BadRequest)));
            let required=tasks.iter().map(|(_,p)|*p).sum::<u64>();
            assert!(matches!(World::with_long_persistent_transform_sequence(config.clone(),tasks.clone(),required-1),Err(Code::BadRequest)));
            let mut w=World::with_long_persistent_transform_sequence(config,tasks.clone(),3750).unwrap();
            for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
            }
            let paid=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
            if let Some(previous)=install{assert_eq!(paid,previous);}else{install=Some(paid);}
            w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:256}});
            w.schedule_event(ScheduledEvent{due_step:32,priority:0,sequence_no:1,
                kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:0}});
            while w.state.step<30{w.tick(None);}
            assert_eq!(w.state.nodes[&NodeId::A].gas_available,256);
            assert!(w.state.organisms[&ORGANISM_ID].vm_continuation.is_none());
            let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
            let mut balance=256u64;let mut earned=0u64;let mut payer=3750u64;
            let mut fees=0u64;let mut expired=0u64;let mut credit=0u64;
            let mut post_fees=0u64;let mut post_credit=0u64;let mut gas=0;
            let mut at32=0u64;let mut at48=0u64;let mut at64=0u64;
            let mut min_post=u64::MAX;
            let mut expected_history=None;let mut sample_read_fee=None;
            let mut saved_at=None;let mut gate_status=None;let mut gate_step=None;let mut after_gate_read=false;
            let mut settlements=Vec::new();let mut reads=Vec::new();
            for step in 30..70{
                assert_eq!(w.state.step,step);
                if step==32||step==48||step==64{
                    let kept=if persistent{earned}else{0};
                    expired+=balance-kept;balance=kept;
                    if step==32{at32=kept;}else if step==48{at48=kept;}else{at64=kept;}
                }
                let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                assert_eq!(r,rr);
                let fee=r.gas_charged+r.vm_gas_charged;
                if persistent{
                    let quota=balance.checked_sub(earned).unwrap();
                    earned=earned.checked_sub(fee.saturating_sub(quota)).unwrap()+r.a_credited;
                }
                balance=balance.checked_sub(fee).unwrap()+r.a_credited;
                payer=payer.checked_sub(r.a_credited).unwrap();fees+=fee;credit+=r.a_credited;
                if step>=32{
                    post_fees+=fee;
                    post_credit+=r.a_credited;
                    min_post=min_post.min(balance);
                    assert_eq!(w.state.nodes[&NodeId::A].gas_per_epoch,0);}
                if r.code==Code::Gas{gas+=1;}
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                assert_eq!(payer,escrow.remaining_a);assert_eq!(escrow.earned_a,persistent.then_some(earned));
                assert_eq!(balance+payer,4006-fees-expired);
                let e=w.audit.last().unwrap();
                if matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER)&&r.code==Code::Ok{
                    let slot=settlements.len();
                    assert_eq!(serde_json::from_slice::<TransformTask>(&r.payload).unwrap(),tasks[slot].0);
                    reads.push((e.step,slot));
                    if sample_slot!=0 && slot==sample_slot as usize{assert!(sample_read_fee.is_none());sample_read_fee=Some(fee);}
                }
                if matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/resource-status")&&r.code==Code::Ok{
                    assert!(mode>=1);assert!(gate_status.is_none());assert_eq!(settlements.len(),2);
                    let v:serde_json::Value=serde_json::from_slice(&r.payload).unwrap();
                    assert_eq!(v["a_available"].as_u64(),Some(balance));
                    gate_status=Some(balance);gate_step=Some(step);
                }
                if gate_step.is_some_and(|s|step>s)&&matches!(&e.action,Some(Action::Read{key,..}) if key==OFFER){
                    after_gate_read=true;
                }
                if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/learned/production-sample-le") && r.code==Code::Ok{
                    assert!(expected_history.is_some());assert!(saved_at.is_none());saved_at=Some(step);
                }
                if r.a_credited>0{
                    let slot=settlements.len();let (task,price)=&tasks[slot];
                    let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                    assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                    assert!(reads.iter().any(|(s,i)|*s<step&&*i==slot));
                    assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                    let ledger=&escrow.sequence.as_ref().unwrap().settled;
                    assert_eq!(ledger.len(),slot+1);
                    assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step,price_a:*price});
                    settlements.push((step,slot,*price,balance));
                    if sample_slot!=0 && slot==sample_slot as usize{
                        expected_history=Some([sample_slot as u64,sample_read_fee.expect("sample read")+fee,r.a_credited]);
                    }
                }
            }
            assert_eq!(settlements[0].0,31);assert_eq!(settlements[0].2,first);
            assert_eq!(credit,tasks.iter().take(settlements.len()).map(|(_,p)|*p).sum::<u64>());
            for (slot,(task,_)) in tasks.iter().enumerate(){
                let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                assert_eq!(w.state.objects.contains_key(&key),slot<settlements.len());
                if slot<settlements.len(){
                    let o=&w.state.objects[&key];assert_eq!(o.owner,ORGANISM_ID);assert_eq!(o.version,0);
                    assert_eq!(o.bytes.len(),280);assert_eq!(o.bytes[0],task.held_out_input.len() as u8);
                    assert_eq!(&o.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                    assert!(o.bytes[33..].iter().all(|b|*b==0x55));
                }
            }
            if settlements.len()==12{
                assert_eq!(post_credit,2244);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(12)));
                assert!(settlements.iter().skip(1).all(|(s,_,_,_)|*s>=32));
                assert!(settlements.last().unwrap().0>=48);
                assert_eq!(at64,balance);
                let escrow=w.state.transform_escrow.as_ref().unwrap();
                assert!(escrow.sequence.as_ref().unwrap().pending.is_empty());
                assert_eq!(escrow.paid_a,prices[10]);
                assert_eq!(escrow.sequence.as_ref().unwrap().settled.len(),12);
                assert_eq!(payer,3750-first-2244);
            }
            let learned=w.state.objects.get("/self/learned/production-sample-le");
            if let Some(object)=learned{
                let expected=expected_history.expect("complete history before save");
                assert_eq!(object.owner,ORGANISM_ID);assert_eq!(object.version,0);
                let bytes=expected.iter().flat_map(|n|u16::try_from(*n).unwrap().to_le_bytes()).collect::<Vec<_>>();
                assert_eq!(object.bytes,bytes);assert!(saved_at.is_some());
            }else{assert!(saved_at.is_none());}
            if sample_slot==0{assert!(learned.is_none());}
            else if settlements.len()==12{assert!(learned.is_some());}
            if let (Some(status),Some(sample))=(gate_status,expected_history){
                let threshold=if mode==2{239}else{sample[1]};
                if mode>=2&&status<threshold{
                    assert!(!after_gate_read);assert_eq!(settlements.len(),2);
                }
            }
            let net=post_credit as i64-post_fees as i64;
            assert_eq!(balance as i64-at32 as i64,net);
            assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
            println!("S29_SAMPLE_TRANSFER mode={mode} sample_slot={sample_slot} first={first} gate_status={gate_status:?} gate_step={gate_step:?} after_gate_read={after_gate_read} tape={tape} saved_at={saved_at:?} expected_history={expected_history:?} learned={} min_post_A={min_post} net={net} persistent={persistent} root={hash} root_B={} install_A={paid} settled={} credit={credit} fees={fees} expired_A={expired} boundary32_A={at32} boundary48_A={at48} boundary64_A={at64} post_credit={post_credit} post_fees={post_fees} final_A={balance} earned={earned} payer_A={payer} gas={gas} return={:?} replay=true settlements={settlements:?}",learned.is_some(),raw.len(),settlements.len(),w.state.organisms[&ORGANISM_ID].last_return);
        }
        }
        }
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


    // Same fitting computation in both arms; only the prediction slope differs.
    fn two_sample_fit_root(use_fit:bool)->Program{
        let mut p=additive_liquidity_root(true);let f=&mut p.functions[0];
        let ys=f.blocks.iter().filter_map(|b|match &b.terminator{
            Terminator::Yield{next,..}=>Some(next.clone()),_=>None
        }).collect::<std::collections::BTreeSet<_>>();
        let u=|id|Param{id,ty:Type::U32};
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.extend([Instruction{id:95,op:Op::ConstU32(0)},Instruction{id:96,op:Op::ConstU32(1)}]);
            }else{
                let at=if ys.contains(&b.id){b.params.len()-1}else{b.params.len()};
                b.params.insert(at,u(95));b.params.insert(at+1,u(96));
            }
            match &mut b.terminator{
                Terminator::Jump{args,..}=>args.extend([95,96]),
                Terminator::Branch{true_args,false_args,..}=>{true_args.extend([95,96]);false_args.extend([95,96]);},
                Terminator::Yield{next_args,..}=>next_args.extend([95,96]),
                Terminator::RepeatBounded{body_args,done_args,..}|
                Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.extend([95,96]);done_args.extend([95,96]);},
                Terminator::Return{..}=>{}
            }
        }
        let submit=f.blocks.iter_mut().find(|b|b.id=="size_submit").unwrap();
        if let Terminator::Yield{next_args,..}=&mut submit.terminator{
            *next_args=vec![90,91,19,94,93,96];
        }else{panic!("submission must yield");}
        let m=f.blocks.iter_mut().find(|b|b.id=="liquidity_measure").unwrap();
        m.instructions.extend([Instruction{id:20,op:Op::ConstU32(1)},Instruction{id:21,op:Op::EqU32(90,20)}]);
        m.terminator=Terminator::Branch{condition:21,if_true:"fit_second".into(),
            true_args:vec![19,13,93,15,95,96,94],if_false:"chain_header".into(),
            false_args:vec![19,13,93,15,95,96]};
        let base=vec![u(90),u(91),u(93),u(94),u(95),u(96)];
        let mut fit_ctx=base.clone();fit_ctx.push(u(97));
        f.blocks.extend([
            Block{id:"fit_second".into(),params:fit_ctx.clone(),instructions:vec![
                Instruction{id:10,op:Op::Div(95,93)},Instruction{id:11,op:Op::ConstU32(0)},Instruction{id:12,op:Op::EqU32(10,11)}
            ],terminator:Terminator::Branch{condition:12,if_true:"fit_increasing".into(),true_args:vec![90,91,93,94,95,96,97],
                if_false:"chain_header".into(),false_args:vec![90,91,93,94,95,96]}},
            Block{id:"fit_increasing".into(),params:fit_ctx,instructions:vec![
                Instruction{id:10,op:Op::Sub(93,95)},Instruction{id:11,op:Op::ConstU32(2)},
                Instruction{id:12,op:Op::Div(10,11)},Instruction{id:13,op:Op::Add(97,10)},
                Instruction{id:14,op:Op::Add(13,12)},Instruction{id:15,op:Op::Div(94,14)},
                Instruction{id:16,op:Op::ConstU32(0)},Instruction{id:17,op:Op::EqU32(15,16)}
            ],terminator:Terminator::Branch{condition:17,if_true:"chain_header".into(),true_args:vec![90,91,93,94,95,96],
                if_false:"chain_header".into(),false_args:vec![90,91,93,94,95,11]}}
        ]);
        // Reuse two-block checked additive prediction, computing both candidate
        // weighted lengths before selecting the actual operands for the gate.
        for id in ["size_model_start","additive_compare"]{
            let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
            let mut prefix=vec![
                Instruction{id:30,op:Op::ConstU32(1)},Instruction{id:31,op:Op::Sub(96,30)},
                Instruction{id:32,op:Op::ConstU32(0)},Instruction{id:33,op:Op::Sub(32,31)},
                Instruction{id:34,op:Op::And(19,33)},Instruction{id:35,op:Op::And(93,33)},
                Instruction{id:36,op:Op::Add(19,34)},Instruction{id:37,op:Op::Add(93,35)}
            ];
            for i in &mut b.instructions{
                if i.id==10{i.op=Op::Add(94,if use_fit{36}else{19});}
                if i.id==11{i.op=if id=="size_model_start"{Op::Div(10,if use_fit{37}else{93})}
                    else{Op::Sub(10,if use_fit{37}else{93})};}
            }
            prefix.append(&mut b.instructions);b.instructions=prefix;
        }
        p.validate().expect("typed two-sample fitting root");p
    }

    #[test]
    fn s29_two_sample_cost_fit_native(){
        let mut observed=std::collections::BTreeSet::new();let mut paired=Vec::new();
        for tape in ["DENSE_BYTES","SPARSE_BYTES"]{
        for grant in [768u64,896,1024,1152,1280,1408,1536,1664,1792]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=two_sample_fit_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                let byte=if tape=="DENSE_BYTES"{255}else{7};
                tasks[1].0.held_out_input=vec![byte;128];tasks[2].0.held_out_input=vec![byte;255];
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
                let mut fitted=None;let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
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
                        if slot==2{
                            assert_eq!(intervals.len(),2);
                            let delta=125u64;
                            let coefficient=if intervals[1].0>=intervals[0].0+delta+delta/2{2}else{1};
                            let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                            assert_eq!(c.registers.get(&96),Some(&Value::U32(coefficient)));
                            observed.insert(coefficient);fitted=Some(coefficient);
                        }
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
                println!("S29_TWO_SAMPLE_FIT tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} fitted={fitted:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
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
                outcomes.push((expected,intervals.into_iter().take(2).collect::<Vec<_>>(),fitted,gas,balance,fees,paid));
            }
            assert_eq!(outcomes[0].1,outcomes[1].1,"identical pre-fit measurements");
            assert_eq!(outcomes[0].2,outcomes[1].2,"same actual coefficient at third offer");
            paired.push((tape,grant,outcomes[0].0,outcomes[1].0,outcomes[0].3,outcomes[1].3,outcomes[0].4,outcomes[1].4));
        }
        }
        println!("S29_FIT_RESULTS observed={observed:?} pairs={paired:?}");
        assert!(observed.contains(&1));assert!(observed.contains(&2));
    }


    #[test]
    fn s29_cost_fit_transfer_limits_native(){
        let mut observed=std::collections::BTreeSet::new();let mut paired=Vec::new();
        let mut negative=Vec::new();let mut insufficient=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE"]{
        for grant in [768u64,896,1024,1152,1280,1408,1536,1664,1792]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=two_sample_fit_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                let byte=if tape=="DENSE_TO_SPARSE"{255}else{7};
                tasks[1].0.held_out_input=vec![byte;128];tasks[2].0.held_out_input=vec![if byte==255{7}else{255};255];
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
                let mut fitted=None;let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
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
                        if slot==2{
                            assert_eq!(intervals.len(),2);
                            let delta=125u64;
                            let coefficient=if intervals[1].0>=intervals[0].0+delta+delta/2{2}else{1};
                            let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                            assert_eq!(c.registers.get(&96),Some(&Value::U32(coefficient)));
                            observed.insert(coefficient);fitted=Some(coefficient);
                        }
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
                println!("S29_FIT_TRANSFER tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} fitted={fitted:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
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
                outcomes.push((expected,intervals.into_iter().take(2).collect::<Vec<_>>(),fitted,gas,balance,fees,paid,w.state.organisms[&ORGANISM_ID].last_return.clone()));
            }
            assert_eq!(outcomes[0].1,outcomes[1].1,"identical pre-fit measurements");
            assert_eq!(outcomes[0].2,outcomes[1].2,"same actual coefficient at third offer");
            let (fixed,fit)=(&outcomes[0],&outcomes[1]);
            if tape=="DENSE_TO_SPARSE"&&fit.2==Some(2)&&fit.0==2&&fit.3==0&&fit.7==Some(Value::U32(2))&&fixed.0==3&&fixed.3==0{
                negative.push(grant);
            }
            if tape=="SPARSE_TO_DENSE"&&fit.2==Some(1)&&fit.0==2&&fit.3>0&&fit.1.len()==2&&fit.1[1].1>=fit.1[1].0+127{
                insufficient.push(grant);
            }
            paired.push((tape,grant,fixed.0,fit.0,fixed.3,fit.3,fixed.4,fit.4));
        }
        }
        println!("S29_FIT_TRANSFER_RESULTS observed={observed:?} negative={negative:?} insufficient={insufficient:?} pairs={paired:?}");
        assert!(observed.contains(&1));assert!(observed.contains(&2));
        assert!(!negative.is_empty());assert!(!insufficient.is_empty());
    }

    // Equal feature calculation, different selected feature. No new sensor.
    fn encoded_feature_root(encoded:bool)->Program{
        let mut p=additive_liquidity_root(true);let f=&mut p.functions[0];
        let compute=f.blocks.iter_mut().find(|b|b.id=="compute").unwrap();
        compute.instructions.extend([
            Instruction{id:32,op:Op::Len(11)},Instruction{id:33,op:Op::Add(32,19)},
            Instruction{id:34,op:Op::ConstU32(2)},Instruction{id:35,op:Op::Div(33,34)}
        ]);
        if let Terminator::Branch{true_args,false_args,..}=&mut compute.terminator{
            let feature=if encoded{35}else{19};
            true_args.push(feature);false_args.push(feature);
        }else{panic!("expected compute branch");}
        for id in ["size_model_start","additive_compare","size_gate","size_submit"]{
            let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
            b.params.push(Param{id:98,ty:Type::U32});
            if matches!(id,"size_model_start"|"additive_compare"){
                b.instructions.iter_mut().find(|i|i.id==10).unwrap().op=Op::Add(94,98);
            }
            match &mut b.terminator{
                Terminator::Branch{if_true,true_args,if_false,false_args,..}=>{
                    if if_true!="failed"{true_args.push(98);}
                    if if_false!="failed"{false_args.push(98);}
                },
                Terminator::Yield{next_args,..}=>{
                    assert_eq!(next_args[2],19);next_args[2]=98;
                },
                _=>panic!("unexpected feature control")
            }
        }
        p.validate().expect("typed encoded feature root");p
    }

    #[test]
    fn s29_encoded_cost_feature_native(){
        let mut checked=std::collections::BTreeSet::new();let mut paired=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE"]{
        for grant in [768u64,896,1024,1152,1280,1408,1536,1664,1792]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=encoded_feature_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                let byte=if tape=="DENSE_TO_SPARSE"{255}else{7};
                tasks[1].0.held_out_input=vec![byte;128];tasks[2].0.held_out_input=vec![if byte==255{7}else{255};255];
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
                let mut features=Vec::new();let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
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
                        let raw_len=tasks[slot].0.held_out_input.len() as u32;
                        let json_len=r.payload.len() as u32;
                        let feature=if estimate{(raw_len+json_len)/2}else{raw_len};
                        if slot>0{
                            let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                            assert_eq!(c.registers.get(&93),Some(&Value::U32(features[slot-1])));
                        }
                        assert_eq!(features.len(),slot);features.push(feature);
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                        assert_eq!(c.registers.get(&98),Some(&Value::U32(features[slot])));
                        if slot>0{checked.insert((tape,estimate));}
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_ENCODED_FEATURE tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} features={features:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
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
                outcomes.push((expected,intervals.first().copied(),gas,balance,fees,paid));
            }
            assert_eq!(outcomes[0].1,outcomes[1].1,"identical first interval");
            let (raw,encoded)=(&outcomes[0],&outcomes[1]);
            paired.push((tape,grant,raw.0,encoded.0,raw.2,encoded.2,raw.3,encoded.3,raw.4,encoded.4));
        }
        }
        println!("S29_ENCODED_FEATURE_RESULTS checked={checked:?} pairs={paired:?}");
        assert_eq!(checked.len(),4);
    }


    // Both histories and the learned feature flag are carried in the actual VM.
    fn online_feature_selection_root(select:bool)->Program{
        let mut p=encoded_feature_root(false);let f=&mut p.functions[0];
        let ys=f.blocks.iter().filter_map(|b|match &b.terminator{
            Terminator::Yield{next,..}=>Some(next.clone()),_=>None
        }).collect::<std::collections::BTreeSet<_>>();
        let u=|id|Param{id,ty:Type::U32};
        let history=[95,96,97,89];
        for b in &mut f.blocks{
            if b.id=="entry"{
                b.instructions.extend(history.iter().map(|id|Instruction{id:*id,op:Op::ConstU32(0)}));
            }else{
                let at=if ys.contains(&b.id){b.params.len()-1}else{b.params.len()};
                for (offset,id) in history.iter().enumerate(){b.params.insert(at+offset,u(*id));}
            }
            match &mut b.terminator{
                Terminator::Jump{args,..}=>args.extend(history),
                Terminator::Branch{true_args,false_args,..}=>{true_args.extend(history);false_args.extend(history);},
                Terminator::Yield{next_args,..}=>next_args.extend(history),
                Terminator::RepeatBounded{body_args,done_args,..}|
                Terminator::RepeatScoped{body_args,done_args,..}=>{body_args.extend(history);done_args.extend(history);},
                Terminator::Return{..}=>{}
            }
        }
        let compute=f.blocks.iter_mut().find(|b|b.id=="compute").unwrap();
        if let Terminator::Branch{true_args,false_args,..}=&mut compute.terminator{
            true_args.push(35);false_args.push(35);
        }else{panic!("compute branch");}
        for id in ["size_model_start","additive_compare","size_gate","size_submit"]{
            let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
            b.params.push(u(88));
            if matches!(id,"size_model_start"|"additive_compare"){
                // Compute both features with a binary selection mask.
                let mut prefix=vec![
                    Instruction{id:40,op:Op::ConstU32(0)},
                    Instruction{id:41,op:Op::Sub(40,if select{89}else{40})},
                    Instruction{id:42,op:Op::Sub(95,93)},
                    Instruction{id:43,op:Op::And(42,41)},
                    Instruction{id:44,op:Op::Add(93,43)},
                    Instruction{id:45,op:Op::Sub(88,98)},
                    Instruction{id:46,op:Op::And(45,41)},
                    Instruction{id:47,op:Op::Add(98,46)}
                ];
                for i in &mut b.instructions{
                    if i.id==10{i.op=Op::Add(94,47);}
                    if i.id==11{i.op=if id=="size_model_start"{Op::Div(10,44)}else{Op::Sub(10,44)};}
                }
                prefix.append(&mut b.instructions);b.instructions=prefix;
            }
            match &mut b.terminator{
                Terminator::Branch{if_true,true_args,if_false,false_args,..}=>{
                    if if_true!="failed"{true_args.push(88);}
                    if if_false!="failed"{false_args.push(88);}
                },
                Terminator::Yield{next_args,..}=>{
                    // submitted: count, sensor A, current raw, previous C,
                    // current encoded, previous raw, previous encoded, flag.
                    *next_args=vec![90,91,98,94,88,93,95,89];
                },
                _=>panic!("feature flow")
            }
        }
        let m=f.blocks.iter_mut().find(|b|b.id=="liquidity_measure").unwrap();
        m.instructions.extend([Instruction{id:20,op:Op::ConstU32(1)},Instruction{id:21,op:Op::EqU32(90,20)}]);
        m.terminator=Terminator::Branch{condition:21,if_true:"feature_fit".into(),
            true_args:vec![19,13,93,15,95,96,97,89,94],
            if_false:"chain_header".into(),false_args:vec![19,13,93,15,95,96,97,89]};
        // Training fixtures have increasing N and E, with deltaE >= deltaN.
        let ctx=vec![u(90),u(91),u(93),u(94),u(95),u(96),u(97),u(89),u(87)];
        f.blocks.push(Block{id:"feature_fit".into(),params:ctx,instructions:vec![
            Instruction{id:10,op:Op::Sub(93,96)},Instruction{id:11,op:Op::Sub(95,97)},
            Instruction{id:12,op:Op::Sub(11,10)},Instruction{id:13,op:Op::ConstU32(2)},
            Instruction{id:14,op:Op::Div(12,13)},Instruction{id:15,op:Op::Add(87,10)},
            Instruction{id:16,op:Op::Add(15,14)},Instruction{id:17,op:Op::ConstU32(32)},
            Instruction{id:18,op:Op::Add(16,17)},Instruction{id:19,op:Op::Div(94,18)},
            Instruction{id:20,op:Op::ConstU32(0)},Instruction{id:21,op:Op::EqU32(19,20)},
            Instruction{id:22,op:Op::ConstU32(1)}
        ],terminator:Terminator::Branch{condition:21,
            if_true:"chain_header".into(),true_args:vec![90,91,93,94,95,96,97,20],
            if_false:"chain_header".into(),false_args:vec![90,91,93,94,95,96,97,22]}});
        p.validate().expect("typed online feature selection");p
    }

    #[test]
    fn s29_online_feature_selection_native(){
        let mut checked=std::collections::BTreeSet::new();let mut choices=std::collections::BTreeSet::new();let mut paired=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE"]{
        for grant in [768u64,896,1024,1152,1280,1408,1536,1664,1792]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=online_feature_selection_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                let byte=if tape=="DENSE_TO_SPARSE"{255}else{7};
                tasks[1].0.held_out_input=vec![byte;128];tasks[2].0.held_out_input=vec![if byte==255{7}else{255};255];
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
                let mut features:Vec<(u32,u32)>=Vec::new();let mut choice=None;let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
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
                        let raw_len=tasks[slot].0.held_out_input.len() as u32;
                        let json_len=r.payload.len() as u32;
                        let feature=(raw_len,(raw_len+json_len)/2);
                        if slot>0{
                            let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                            assert_eq!(c.registers.get(&93),Some(&Value::U32(features[slot-1].0)));
                            assert_eq!(c.registers.get(&95),Some(&Value::U32(features[slot-1].1)));
                            if slot==2{
                                assert_eq!(intervals.len(),2);
                                let dn=u64::from(features[1].0-features[0].0);
                                let de=u64::from(features[1].1-features[0].1);
                                let threshold=intervals[0].0+dn+(de-dn)/2+32;
                                let selected=u32::from(intervals[1].0>=threshold);
                                assert_eq!(c.registers.get(&89),Some(&Value::U32(selected)));
                                choice=Some(selected);choices.insert(selected);
                            }
                        }
                        assert_eq!(features.len(),slot);features.push(feature);
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                        assert_eq!(c.registers.get(&98),Some(&Value::U32(features[slot].0)));
                        assert_eq!(c.registers.get(&88),Some(&Value::U32(features[slot].1)));
                        if slot>0{checked.insert((tape,estimate));}
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_ONLINE_FEATURE tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} features={features:?} choice={choice:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
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
                outcomes.push((expected,intervals.into_iter().take(2).collect::<Vec<_>>(),gas,balance,fees,paid,choice));
            }
            assert_eq!(outcomes[0].1,outcomes[1].1,"identical first two intervals");
            assert_eq!(outcomes[0].6,outcomes[1].6);
            let (raw,encoded)=(&outcomes[0],&outcomes[1]);
            paired.push((tape,grant,raw.0,encoded.0,raw.2,encoded.2,raw.3,encoded.3,raw.4,encoded.4));
        }
        }
        println!("S29_ONLINE_FEATURE_RESULTS checked={checked:?} choices={choices:?} pairs={paired:?}");
        assert_eq!(checked.len(),4);assert_eq!(choices,std::collections::BTreeSet::from([0,1]));
    }

    #[test]
    fn s29_hidden_price_decision_native(){
        let tape="DENSE_TO_SPARSE";
        let grant=1152u64;
        let mut projections=std::collections::BTreeMap::new();
        let mut deltas=Vec::new();
        for third_price in [1u64,400]{
            let mut outcomes=Vec::new();
            for estimate in [false,true]{
                let p=encoded_feature_root(estimate);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                let mut tasks=chain_tasks(1);
                tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];
                tasks[2].1=third_price;
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
                let mut prefix=Vec::new();let mut third_paid=false;
                let mut features=Vec::new();let mut sensors=Vec::new();let mut intervals=Vec::new();let mut last_sensor:Option<(u64,u64,u64)>=None;
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
                    if !third_paid{
                        let mut projected=r.clone();
                        if settlements.len()==2&&r.a_credited>0{projected.a_credited=0;third_paid=true;}
                        prefix.push((e.step,e.action.clone(),projected));
                    }
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
                        let raw_len=tasks[slot].0.held_out_input.len() as u32;
                        let json_len=r.payload.len() as u32;
                        let feature=if estimate{(raw_len+json_len)/2}else{raw_len};
                        if slot>0{
                            let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                            assert_eq!(c.registers.get(&93),Some(&Value::U32(features[slot-1])));
                        }
                        assert_eq!(features.len(),slot);features.push(feature);
                    }
                    if r.a_credited>0{
                        let slot=settlements.len();let (task,price)=&tasks[slot];
                        let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                        assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);assert_eq!(r.gas_charged,20);
                        assert!(reads.iter().any(|(step,i)|*step<e.step&&*i==slot));
                        assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                        let c=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().unwrap();
                        assert_eq!(c.registers.get(&98),Some(&Value::U32(features[slot])));
                        // No feature or price is supplied by the host.
                        settlements.push((e.step,slot,*price));
                        let ledger=&escrow.sequence.as_ref().unwrap().settled;
                        assert_eq!(ledger.len(),slot+1);
                        assert_eq!(ledger[slot],TransformSettlement{outcome_key:key,offer_version:slot as u64,step:e.step,price_a:*price});
                    }
                }
                println!("S29_HIDDEN_PRICE third_price={third_price} tape={tape} grant={grant} estimate={estimate} root={hash} root_B={} install_A={paid} settlements={settlements:?} reads={reads:?} sensors={sensors:?} intervals={intervals:?} features={features:?} credit={received} fees={fees} final_A={balance} payer_A={remaining} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                let expected=settlements.len();
                assert_eq!(expected,if estimate{3}else{2});
                assert_eq!(gas,0);
                assert_eq!(w.state.organisms[&ORGANISM_ID].last_return,Some(Value::U32(expected as u32)));
                assert_eq!(third_paid,estimate);
                if third_price==1{projections.insert(estimate,prefix);}else{
                    assert_eq!(projections.get(&estimate),Some(&prefix),"price-blind actual projection before payout");
                }
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
                outcomes.push((expected,intervals.first().copied(),gas,balance,fees,paid));
            }
            assert_eq!(outcomes[0].1,outcomes[1].1,"same first observed interval");
            let (raw,encoded)=(&outcomes[0],&outcomes[1]);
            let delta=encoded.3 as i64-raw.3 as i64;
            deltas.push((third_price,delta));
            if third_price==1{assert!(delta<0);}else{assert!(delta>0);}
        }
        println!("S29_HIDDEN_PRICE_RESULTS deltas={deltas:?} exact_projection=true");
    }


    // Selected fixture only: threshold from the previous measured 158 A
    // marginal difference; not a learned or universally valid cost estimate.
    // mode 0=stop, 1=continue, 2=paid quote. All retain the same encoded
    // liquidity policy for the first two offers and any authorized third.
    fn price_decision_root(mode:u32)->Program{
        let mut p=encoded_feature_root(true);let f=&mut p.functions[0];
        let ctx=f.blocks.iter().find(|b|b.id=="size_model_start").unwrap().params.clone();
        let args=ctx.iter().map(|p|p.id).collect::<Vec<_>>();
        let fail=vec![90,91,93,94];
        let i=|id,op|Instruction{id,op};
        let mut slot_ctx=ctx.clone();slot_ctx.push(Param{id:13,ty:Type::Bytes});
        let mut slot_args=args.clone();slot_args.push(13);
        let compute=f.blocks.iter_mut().find(|b|b.id=="compute").unwrap();
        if let Terminator::Branch{if_false,false_args,..}=&mut compute.terminator{
            assert_eq!(if_false,"size_model_start");
            // Arguments are source-block registers; params are destination IDs.
            // ENCODED supplies computed feature r35 to destination r98.
            let mut source_args=args.clone();
            assert_eq!(source_args.pop(),Some(98));source_args.push(35);
            assert_eq!(*false_args,source_args);
            *if_false="price_slot".into();false_args.push(13);
        }else{panic!("expected compute branch");}
        f.blocks.push(Block{id:"price_slot".into(),params:slot_ctx.clone(),instructions:vec![
            i(10,Op::ConstU32(2)),i(11,Op::EqU32(90,10))
        ],terminator:Terminator::Branch{condition:11,
            if_true:"price_choice".into(),true_args:slot_args.clone(),
            if_false:"size_model_start".into(),false_args:args.clone()}});
        if mode<2{
            f.blocks.push(Block{id:"price_choice".into(),params:slot_ctx,instructions:vec![],
                terminator:Terminator::Jump{target:if mode==0{"failed"}else{"size_model_start"}.into(),
                    args:if mode==0{fail}else{args}}});
        }else{
            let mut reply=slot_ctx.clone();reply.push(Param{id:2,ty:Type::Bytes});
            let mut payload_ctx=slot_ctx.clone();payload_ctx.push(Param{id:14,ty:Type::Bytes});
            let mut payload_args=slot_args.clone();payload_args.push(14);
            f.blocks.extend([
                Block{id:"price_choice".into(),params:slot_ctx.clone(),instructions:vec![
                    i(10,literal(b"/sys/transform-price/")),i(11,Op::Concat(10,13)),
                    i(12,literal(b"/2")),i(20,Op::Concat(11,12))
                ],terminator:Terminator::Yield{call:WorldCall::Read,args:vec![20],
                    next:"price_reply".into(),next_args:slot_args.clone()}},
                Block{id:"price_reply".into(),params:reply.clone(),instructions:vec![
                    i(10,literal(b"code")),i(11,Op::JsonFieldString(2,10)),
                    i(12,literal(b"Ok")),i(20,Op::EqBytes(11,12))
                ],terminator:Terminator::Branch{condition:20,if_true:"price_identity".into(),
                    true_args:{let mut a=slot_args.clone();a.push(2);a},
                    if_false:"failed".into(),false_args:fail.clone()}},
                Block{id:"price_identity".into(),params:reply,instructions:vec![
                    i(10,literal(b"payload")),i(14,Op::JsonFieldBytes(2,10)),
                    i(11,literal(b"task_id")),i(12,Op::JsonFieldString(14,11)),
                    i(20,Op::EqBytes(12,13))
                ],terminator:Terminator::Branch{condition:20,if_true:"price_version".into(),
                    true_args:payload_args.clone(),if_false:"failed".into(),false_args:fail.clone()}},
                Block{id:"price_version".into(),params:payload_ctx.clone(),instructions:vec![
                    i(10,literal(b"offer_version")),i(11,Op::JsonFieldU32(14,10)),
                    i(12,Op::EqU32(11,90))
                ],terminator:Terminator::Branch{condition:12,if_true:"price_threshold".into(),
                    true_args:payload_args,if_false:"failed".into(),false_args:fail.clone()}},
                Block{id:"price_threshold".into(),params:payload_ctx,instructions:vec![
                    i(10,literal(b"price_a")),i(11,Op::JsonFieldU32(14,10)),
                    i(12,Op::ConstU32(159)),i(20,Op::Div(11,12)),
                    i(21,Op::ConstU32(0)),i(22,Op::EqU32(20,21))
                ],terminator:Terminator::Branch{condition:22,if_true:"failed".into(),
                    true_args:fail,if_false:"size_model_start".into(),false_args:args}}
            ]);
        }
        if mode==3{
            // The synchronous, exact-key kernel READ already binds this reply
            // to the requested current task/version. Keep code guard and
            // price parsing, remove only duplicate identity/version checks.
            let b=f.blocks.iter_mut().find(|b|b.id=="price_identity").unwrap();
            b.instructions.truncate(2);
            let mut a=b.params.iter().map(|p|p.id).collect::<Vec<_>>();
            assert_eq!(a.pop(),Some(2));a.push(14);
            b.terminator=Terminator::Jump{target:"price_threshold".into(),args:a};
            f.blocks.retain(|b|b.id!="price_version");
        }
        p.validate().expect("typed price decision root");p
    }

    #[test]
    fn s29_paid_price_decision_native(){
        let mut totals=Vec::new();
        for fee in [8u64,32,96]{
            let mut sums=[0u64;3];
            for third_price in [1u64,400]{
                for mode in [0u32,1,2]{
                    let p=price_decision_root(mode);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                    let mut tasks=chain_tasks(1);
                    tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];
                    tasks[2].1=third_price;
                    let mut w=World::with_priced_transform_sequence(Config{seed:129,epoch_length:16,
                        a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400,fee).unwrap();
                    for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                        let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                    }
                    let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                    w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                        kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:1152}});
                    while w.state.step<16{w.tick(None);}
                    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                    let mut balance=1152u64;let mut payer=1400u64;let mut fees=0;let mut credit=0;
                    let mut settled=0usize;let mut quotes=0;let mut quote_ok=false;let mut gas=0;
                    let mut quote_fees=0;let mut third_attempts=0;
                    for _ in 0..15{
                        let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                        let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                        let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                        assert_eq!(r,rr);
                        let cost=r.gas_charged+r.vm_gas_charged;
                        balance=balance.checked_sub(cost).unwrap().checked_add(r.a_credited).unwrap();
                        payer=payer.checked_sub(r.a_credited).unwrap();fees+=cost;credit+=r.a_credited;
                        assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                        let escrow=w.state.transform_escrow.as_ref().unwrap();
                        assert_eq!(payer,escrow.remaining_a);assert_eq!(balance+payer,2552-fees);
                        let e=w.audit.last().unwrap();
                        if r.code==Code::Gas{gas+=1;}
                        if matches!(&e.action,Some(Action::Read{key,..}) if key.starts_with("/sys/transform-price/")){
                            assert_eq!(mode,2);assert_eq!(settled,2);quotes+=1;
                            assert!(matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/transform-price/c/2"));
                            assert_eq!(r.a_credited,0);quote_fees+=r.gas_charged;
                            if r.code==Code::Ok{
                                quote_ok=true;assert_eq!(r.gas_charged,fee);
                                assert_eq!(serde_json::from_slice::<serde_json::Value>(&r.payload).unwrap(),
                                    serde_json::json!({"task_id":"c","offer_version":2,"price_a":third_price}));
                            }
                        }
                        if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/outcomes/s29-offer-63"){
                            third_attempts+=1;
                            assert_ne!(mode,0);
                            if mode==2{assert!(quote_ok);assert_eq!(third_price,400);}
                        }
                        if r.a_credited>0{
                            let (task,price)=&tasks[settled];
                            let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                            assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);
                            assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                            let receipt=&w.state.objects[&key];
                            assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);
                            assert_eq!(receipt.bytes.len(),280);assert_eq!(receipt.bytes[0],task.held_out_input.len() as u8);
                            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                            assert_eq!(escrow.sequence.as_ref().unwrap().settled[settled],
                                TransformSettlement{outcome_key:key,offer_version:settled as u64,step:e.step,price_a:*price});
                            settled+=1;
                        }
                    }
                    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                    assert_eq!(credit,tasks.iter().take(settled).map(|(_,p)|*p).sum::<u64>());
                    assert!(settled>=2);assert!(settled<=3);
                    assert!(quotes<=1);assert!(third_attempts<=1);
                    if mode==0{assert_eq!(settled,2);assert_eq!(quotes,0);}
                    if mode==1{assert_eq!(quotes,0);}
                    if mode==2{assert_eq!(quotes,1);if third_price==1{assert_eq!(third_attempts,0);assert_eq!(settled,2);}}
                    sums[mode as usize]+=balance;
                    println!("S29_PRICE_POLICY fee={fee} third_price={third_price} mode={mode} root={hash} root_B={} install_A={install} settled={settled} quotes={quotes} quote_ok={quote_ok} quote_fees={quote_fees} third_attempts={third_attempts} credit={credit} fees={fees} final_A={balance} payer_A={payer} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                }
            }
            let delta=sums[2] as i64-sums[0].max(sums[1]) as i64;
            totals.push((fee,sums,delta));
        }
        println!("S29_PRICE_POLICY_RESULTS equal_mixture_twice_mean={totals:?}");
    }


    #[test]
    fn s29_bound_quote_elision_native(){
        let mut totals=Vec::new();
        for fee in [8u64,32,96]{
            let mut sums=[0u64;4];
            for third_price in [1u64,400]{
                for mode in [0u32,1,2,3]{
                    let p=price_decision_root(mode);let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                    let mut tasks=chain_tasks(1);
                    tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];
                    tasks[2].1=third_price;
                    let mut w=World::with_priced_transform_sequence(Config{seed:129,epoch_length:16,
                        a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400,fee).unwrap();
                    for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                        let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                    }
                    let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                    w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                        kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:1152}});
                    while w.state.step<16{w.tick(None);}
                    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                    let mut balance=1152u64;let mut payer=1400u64;let mut fees=0;let mut credit=0;
                    let mut settled=0usize;let mut quotes=0;let mut quote_ok=false;let mut gas=0;
                    let mut quote_fees=0;let mut third_attempts=0;
                    for _ in 0..15{
                        let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                        let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                        let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                        assert_eq!(r,rr);
                        let cost=r.gas_charged+r.vm_gas_charged;
                        balance=balance.checked_sub(cost).unwrap().checked_add(r.a_credited).unwrap();
                        payer=payer.checked_sub(r.a_credited).unwrap();fees+=cost;credit+=r.a_credited;
                        assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                        let escrow=w.state.transform_escrow.as_ref().unwrap();
                        assert_eq!(payer,escrow.remaining_a);assert_eq!(balance+payer,2552-fees);
                        let e=w.audit.last().unwrap();
                        if r.code==Code::Gas{gas+=1;}
                        if matches!(&e.action,Some(Action::Read{key,..}) if key.starts_with("/sys/transform-price/")){
                            assert!(mode>=2);assert_eq!(settled,2);quotes+=1;
                            assert!(matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/transform-price/c/2"));
                            assert_eq!(r.a_credited,0);quote_fees+=r.gas_charged;
                            if r.code==Code::Ok{
                                quote_ok=true;assert_eq!(r.gas_charged,fee);
                                assert_eq!(serde_json::from_slice::<serde_json::Value>(&r.payload).unwrap(),
                                    serde_json::json!({"task_id":"c","offer_version":2,"price_a":third_price}));
                            }
                        }
                        if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/outcomes/s29-offer-63"){
                            third_attempts+=1;
                            assert_ne!(mode,0);
                            if mode>=2{assert!(quote_ok);assert_eq!(third_price,400);}
                        }
                        if r.a_credited>0{
                            let (task,price)=&tasks[settled];
                            let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                            assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);
                            assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                            let receipt=&w.state.objects[&key];
                            assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);
                            assert_eq!(receipt.bytes.len(),280);assert_eq!(receipt.bytes[0],task.held_out_input.len() as u8);
                            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                            assert_eq!(escrow.sequence.as_ref().unwrap().settled[settled],
                                TransformSettlement{outcome_key:key,offer_version:settled as u64,step:e.step,price_a:*price});
                            settled+=1;
                        }
                    }
                    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                    assert_eq!(credit,tasks.iter().take(settled).map(|(_,p)|*p).sum::<u64>());
                    assert!(settled>=2);assert!(settled<=3);
                    assert!(quotes<=1);assert!(third_attempts<=1);
                    if mode==0{assert_eq!(settled,2);assert_eq!(quotes,0);}
                    if mode==1{assert_eq!(quotes,0);}
                    if mode>=2{assert_eq!(quotes,1);if third_price==1{assert_eq!(third_attempts,0);assert_eq!(settled,2);}}
                    sums[mode as usize]+=balance;
                    println!("S29_BOUND_QUOTE fee={fee} third_price={third_price} mode={mode} root={hash} root_B={} install_A={install} settled={settled} quotes={quotes} quote_ok={quote_ok} quote_fees={quote_fees} third_attempts={third_attempts} credit={credit} fees={fees} final_A={balance} payer_A={payer} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                }
            }
            let delta=sums[2] as i64-sums[0].max(sums[1]) as i64;
            let lean_delta=sums[3] as i64-sums[0].max(sums[1]) as i64;
            totals.push((fee,sums,delta,lean_delta));
        }
        println!("S29_BOUND_QUOTE_RESULTS equal_mixture_twice_mean={totals:?}");
    }


    #[test]
    fn s29_bound_quote_transfer_native(){
        let mut totals=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE","MIXED_MID"]{
        for grant in [1024u64,1152,1408]{
        for fee in [8u64,32]{
            let mut sums=[0u64;3];
            for third_price in [1u64,400]{
                for mode in [0u32,1,2]{
                    let p=match mode{0=>encoded_feature_root(false),1=>encoded_feature_root(true),_=>price_decision_root(3)};let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                    let mut tasks=chain_tasks(1);
                    match tape{
                        "DENSE_TO_SPARSE"=>{tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];},
                        "SPARSE_TO_DENSE"=>{tasks[1].0.held_out_input=vec![7;128];tasks[2].0.held_out_input=vec![255;255];},
                        _=>{tasks[1].0.held_out_input=vec![11;96];tasks[2].0.held_out_input=(0..192).map(|n|(n%2)*248+7).map(|n|n as u8).collect();}
                    }
                    tasks[2].1=third_price;
                    let mut w=World::with_priced_transform_sequence(Config{seed:129,epoch_length:16,
                        a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400,fee).unwrap();
                    for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                        let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                    }
                    let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                    w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                        kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                    while w.state.step<16{w.tick(None);}
                    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                    let mut balance=grant;let mut payer=1400u64;let mut fees=0;let mut credit=0;
                    let mut settled=0usize;let mut quotes=0;let mut quote_ok=false;let mut gas=0;
                    let mut quote_fees=0;let mut third_attempts=0;
                    for _ in 0..15{
                        let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                        let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                        let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                        assert_eq!(r,rr);
                        let cost=r.gas_charged+r.vm_gas_charged;
                        balance=balance.checked_sub(cost).unwrap().checked_add(r.a_credited).unwrap();
                        payer=payer.checked_sub(r.a_credited).unwrap();fees+=cost;credit+=r.a_credited;
                        assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                        let escrow=w.state.transform_escrow.as_ref().unwrap();
                        assert_eq!(payer,escrow.remaining_a);assert_eq!(balance+payer,grant+1400-fees);
                        let e=w.audit.last().unwrap();
                        if r.code==Code::Gas{gas+=1;}
                        if matches!(&e.action,Some(Action::Read{key,..}) if key.starts_with("/sys/transform-price/")){
                            assert!(mode>=2);assert_eq!(settled,2);quotes+=1;
                            assert!(matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/transform-price/c/2"));
                            assert_eq!(r.a_credited,0);quote_fees+=r.gas_charged;
                            if r.code==Code::Ok{
                                quote_ok=true;assert_eq!(r.gas_charged,fee);
                                assert_eq!(serde_json::from_slice::<serde_json::Value>(&r.payload).unwrap(),
                                    serde_json::json!({"task_id":"c","offer_version":2,"price_a":third_price}));
                            }
                        }
                        if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/outcomes/s29-offer-63"){
                            third_attempts+=1;
                            if mode>=2{assert!(quote_ok);assert_eq!(third_price,400);}
                        }
                        if r.a_credited>0{
                            let (task,price)=&tasks[settled];
                            let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                            assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);
                            assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                            let receipt=&w.state.objects[&key];
                            assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);
                            assert_eq!(receipt.bytes.len(),280);assert_eq!(receipt.bytes[0],task.held_out_input.len() as u8);
                            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                            assert_eq!(escrow.sequence.as_ref().unwrap().settled[settled],
                                TransformSettlement{outcome_key:key,offer_version:settled as u64,step:e.step,price_a:*price});
                            settled+=1;
                        }
                    }
                    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                    assert_eq!(credit,tasks.iter().take(settled).map(|(_,p)|*p).sum::<u64>());
                    assert!(settled<=3);
                    assert!(quotes<=1);assert!(third_attempts<=1);
                    if mode<2{assert_eq!(quotes,0);}
                    if mode==2 && third_price==1{assert_eq!(third_attempts,0);assert!(settled<=2);}
                    sums[mode as usize]+=balance;
                    println!("S29_QUOTE_TRANSFER tape={tape} grant={grant} fee={fee} third_price={third_price} mode={mode} root={hash} root_B={} install_A={install} settled={settled} quotes={quotes} quote_ok={quote_ok} quote_fees={quote_fees} third_attempts={third_attempts} credit={credit} fees={fees} final_A={balance} payer_A={payer} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                }
            }
            let delta=sums[2] as i64-sums[0].max(sums[1]) as i64;
            totals.push((tape,grant,fee,sums,delta));
        }
        }
        }
        println!("S29_QUOTE_TRANSFER_RESULTS equal_mixture_twice_mean={totals:?}");
    }



    // Move the existing encoded liquidity gate before the third-slot quote.
    // This is an ordering ablation, NOT a guarantee that the estimated
    // remaining resources cover the subsequent quote and production.
    fn liquidity_first_quote_root()->Program{
        let mut p=price_decision_root(3);let f=&mut p.functions[0];
        let compute=f.blocks.iter_mut().find(|b|b.id=="compute").unwrap();
        if let Terminator::Branch{if_false,..}=&mut compute.terminator{
            assert_eq!(if_false,"price_slot");*if_false="size_model_start".into();
        }else{panic!("expected compute branch");}
        for id in ["size_model_start","additive_compare","size_gate"]{
            let b=f.blocks.iter_mut().find(|b|b.id==id).unwrap();
            b.params.push(Param{id:99,ty:Type::Bytes});
            if let Terminator::Branch{if_false,false_args,..}=&mut b.terminator{
                if id=="size_gate"{
                    assert_eq!(if_false,"size_submit");*if_false="price_slot".into();
                }
                false_args.push(99);
            }else{panic!("expected liquidity branch");}
        }
        let slot=f.blocks.iter_mut().find(|b|b.id=="price_slot").unwrap();
        if let Terminator::Branch{if_false,..}=&mut slot.terminator{
            assert_eq!(if_false,"size_model_start");*if_false="size_submit".into();
        }else{panic!("expected slot branch");}
        let threshold=f.blocks.iter_mut().find(|b|b.id=="price_threshold").unwrap();
        if let Terminator::Branch{if_false,..}=&mut threshold.terminator{
            assert_eq!(if_false,"size_model_start");*if_false="size_submit".into();
        }else{panic!("expected threshold branch");}
        p.validate().expect("typed liquidity-first quote");p
    }

    #[test]
    fn s29_liquidity_first_quote_native(){
        let mut totals=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE","MIXED_MID"]{
        for grant in [1024u64,1152,1408]{
        for fee in [8u64,32]{
            let mut sums=[0u64;4];
            for third_price in [1u64,400]{
                for mode in [0u32,1,2,3]{
                    let p=match mode{0=>encoded_feature_root(false),1=>encoded_feature_root(true),2=>price_decision_root(3),_=>liquidity_first_quote_root()};let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                    let mut tasks=chain_tasks(1);
                    match tape{
                        "DENSE_TO_SPARSE"=>{tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];},
                        "SPARSE_TO_DENSE"=>{tasks[1].0.held_out_input=vec![7;128];tasks[2].0.held_out_input=vec![255;255];},
                        _=>{tasks[1].0.held_out_input=vec![11;96];tasks[2].0.held_out_input=(0..192).map(|n|(n%2)*248+7).map(|n|n as u8).collect();}
                    }
                    tasks[2].1=third_price;
                    let mut w=World::with_priced_transform_sequence(Config{seed:129,epoch_length:16,
                        a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400,fee).unwrap();
                    for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                        let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                    }
                    let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                    w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                        kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                    while w.state.step<16{w.tick(None);}
                    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                    let mut balance=grant;let mut payer=1400u64;let mut fees=0;let mut credit=0;
                    let mut settled=0usize;let mut quotes=0;let mut quote_ok=false;let mut gas=0;
                    let mut quote_fees=0;let mut third_attempts=0;
                    for _ in 0..15{
                        let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                        let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                        let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                        assert_eq!(r,rr);
                        let cost=r.gas_charged+r.vm_gas_charged;
                        balance=balance.checked_sub(cost).unwrap().checked_add(r.a_credited).unwrap();
                        payer=payer.checked_sub(r.a_credited).unwrap();fees+=cost;credit+=r.a_credited;
                        assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                        let escrow=w.state.transform_escrow.as_ref().unwrap();
                        assert_eq!(payer,escrow.remaining_a);assert_eq!(balance+payer,grant+1400-fees);
                        let e=w.audit.last().unwrap();
                        if r.code==Code::Gas{gas+=1;}
                        if matches!(&e.action,Some(Action::Read{key,..}) if key.starts_with("/sys/transform-price/")){
                            assert!(mode>=2);assert_eq!(settled,2);quotes+=1;
                            assert!(matches!(&e.action,Some(Action::Read{key,..}) if key=="/sys/transform-price/c/2"));
                            assert_eq!(r.a_credited,0);quote_fees+=r.gas_charged;
                            if r.code==Code::Ok{
                                quote_ok=true;assert_eq!(r.gas_charged,fee);
                                assert_eq!(serde_json::from_slice::<serde_json::Value>(&r.payload).unwrap(),
                                    serde_json::json!({"task_id":"c","offer_version":2,"price_a":third_price}));
                            }
                        }
                        if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/outcomes/s29-offer-63"){
                            third_attempts+=1;
                            if mode>=2{assert!(quote_ok);assert_eq!(third_price,400);}
                        }
                        if r.a_credited>0{
                            let (task,price)=&tasks[settled];
                            let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                            assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);
                            assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                            let receipt=&w.state.objects[&key];
                            assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);
                            assert_eq!(receipt.bytes.len(),280);assert_eq!(receipt.bytes[0],task.held_out_input.len() as u8);
                            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                            assert_eq!(escrow.sequence.as_ref().unwrap().settled[settled],
                                TransformSettlement{outcome_key:key,offer_version:settled as u64,step:e.step,price_a:*price});
                            settled+=1;
                        }
                    }
                    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                    assert_eq!(credit,tasks.iter().take(settled).map(|(_,p)|*p).sum::<u64>());
                    assert!(settled<=3);
                    assert!(quotes<=1);assert!(third_attempts<=1);
                    if mode<2{assert_eq!(quotes,0);}
                    if mode>=2 && third_price==1{assert_eq!(third_attempts,0);assert!(settled<=2);}
                    sums[mode as usize]+=balance;
                    println!("S29_LIQUIDITY_FIRST tape={tape} grant={grant} fee={fee} third_price={third_price} mode={mode} root={hash} root_B={} install_A={install} settled={settled} quotes={quotes} quote_ok={quote_ok} quote_fees={quote_fees} third_attempts={third_attempts} credit={credit} fees={fees} final_A={balance} payer_A={payer} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                }
            }
            let delta=sums[2] as i64-sums[0].max(sums[1]) as i64;
            let reordered=sums[3] as i64-sums[0].max(sums[1]) as i64;
            totals.push((tape,grant,fee,sums,delta,reordered));
        }
        }
        }
        println!("S29_LIQUIDITY_FIRST_RESULTS equal_mixture_twice_mean={totals:?}");
    }



    fn compact_liquidity_quote_root()->Program{
        let mut p=liquidity_first_quote_root();let f=&mut p.functions[0];
        let choice=f.blocks.iter_mut().find(|b|b.id=="price_choice").unwrap();
        choice.instructions[0].op=literal(b"/sys/transform-price-u16/");
        let threshold=f.blocks.iter_mut().find(|b|b.id=="price_threshold").unwrap();
        // Fixed LE u16, price>=159 iff high byte>0 or low byte>=159.
        // Reuses the paid exact-key synchronous READ and Ok guard.
        let i=|id,op|Instruction{id,op};
        threshold.instructions=vec![
            i(10,Op::ConstU32(0)),i(11,Op::ConstU32(1)),
            i(12,Op::ByteAt(14,10)),i(20,Op::ByteAt(14,11)),
            i(21,Op::ConstU32(159)),i(23,Op::Div(12,21)),
            i(24,Op::Add(20,23)),i(22,Op::EqU32(24,10))
        ];
        p.validate().expect("typed compact liquidity quote");p
    }

    #[test]
    fn s29_compact_quote_transfer_native(){
        let mut totals=Vec::new();
        for tape in ["DENSE_TO_SPARSE","SPARSE_TO_DENSE","MIXED_MID"]{
        for grant in [1024u64,1152,1408]{
        for fee in [8u64,32]{
            let mut sums=[0u64;5];
            for third_price in [1u64,400]{
                for mode in [0u32,1,2,3,4]{
                    let p=match mode{0=>encoded_feature_root(false),1=>encoded_feature_root(true),2=>price_decision_root(3),3=>liquidity_first_quote_root(),_=>compact_liquidity_quote_root()};let hash=p.content_hash().unwrap();let raw=p.canonical_bytes();
                    let mut tasks=chain_tasks(1);
                    match tape{
                        "DENSE_TO_SPARSE"=>{tasks[1].0.held_out_input=vec![255;128];tasks[2].0.held_out_input=vec![7;255];},
                        "SPARSE_TO_DENSE"=>{tasks[1].0.held_out_input=vec![7;128];tasks[2].0.held_out_input=vec![255;255];},
                        _=>{tasks[1].0.held_out_input=vec![11;96];tasks[2].0.held_out_input=(0..192).map(|n|(n%2)*248+7).map(|n|n as u8).collect();}
                    }
                    tasks[2].1=third_price;
                    let mut w=World::with_priced_transform_sequence(Config{seed:129,epoch_length:16,
                        a_gas_per_epoch:12000,..Config::default()},tasks.clone(),1400,fee).unwrap();
                    for (key,bytes) in [(format!("/self/versions/{hash}"),raw.clone()),("/self/active".into(),hash.as_bytes().to_vec())]{
                        let r=w.tick(Some(Action::Write{actor:ORGANISM_ID,key,bytes,expected_version:None}));
                        assert_eq!(r.code,Code::Ok);assert_eq!(r.a_credited,0);
                    }
                    let install=w.audit.iter().map(|e|e.result.gas_charged+e.result.vm_gas_charged).sum::<u64>();
                    w.schedule_event(ScheduledEvent{due_step:16,priority:0,sequence_no:0,
                        kind:EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:grant}});
                    while w.state.step<16{w.tick(None);}
                    let mut replay=World::from_snapshot(&w.snapshot_json()).unwrap();
                    let mut balance=grant;let mut payer=1400u64;let mut fees=0;let mut credit=0;
                    let mut settled=0usize;let mut quotes=0;let mut quote_ok=false;let mut gas=0;
                    let mut quote_fees=0;let mut third_attempts=0;
                    for _ in 0..15{
                        let finished=w.state.organisms[&ORGANISM_ID].vm_continuation.as_ref().is_some_and(|c|c.finished);
                        let r=if finished{w.tick(None)}else{w.tick_autonomous(ORGANISM_ID)};
                        let rr=if finished{replay.tick(None)}else{replay.tick_autonomous(ORGANISM_ID)};
                        assert_eq!(r,rr);
                        let cost=r.gas_charged+r.vm_gas_charged;
                        balance=balance.checked_sub(cost).unwrap().checked_add(r.a_credited).unwrap();
                        payer=payer.checked_sub(r.a_credited).unwrap();fees+=cost;credit+=r.a_credited;
                        assert_eq!(balance,w.state.nodes[&NodeId::A].gas_available);
                        let escrow=w.state.transform_escrow.as_ref().unwrap();
                        assert_eq!(payer,escrow.remaining_a);assert_eq!(balance+payer,grant+1400-fees);
                        let e=w.audit.last().unwrap();
                        if r.code==Code::Gas{gas+=1;}
                        if matches!(&e.action,Some(Action::Read{key,..}) if key.starts_with("/sys/transform-price/")||key.starts_with("/sys/transform-price-u16/")){
                            assert!(mode>=2);assert_eq!(settled,2);quotes+=1;
                            let expected_key=if mode==4{"/sys/transform-price-u16/c/2"}else{"/sys/transform-price/c/2"};
                            assert!(matches!(&e.action,Some(Action::Read{key,..}) if key==expected_key));
                            assert_eq!(r.a_credited,0);quote_fees+=r.gas_charged;
                            if r.code==Code::Ok{
                                quote_ok=true;assert_eq!(r.gas_charged,fee);
                                if mode==4{assert_eq!(r.payload,(third_price as u16).to_le_bytes().to_vec());}
                                else{assert_eq!(serde_json::from_slice::<serde_json::Value>(&r.payload).unwrap(),
                                    serde_json::json!({"task_id":"c","offer_version":2,"price_a":third_price}));}
                            }
                        }
                        if matches!(&e.action,Some(Action::Write{key,..}) if key=="/self/outcomes/s29-offer-63"){
                            third_attempts+=1;
                            if mode>=2{assert!(quote_ok);assert_eq!(third_price,400);}
                        }
                        if r.a_credited>0{
                            let (task,price)=&tasks[settled];
                            let key=format!("{OUTCOME}{:02x}",task.task_id.as_bytes()[0]);
                            assert_eq!(r.a_credited,*price);assert_eq!(r.code,Code::Ok);
                            assert!(matches!(&e.action,Some(Action::Write{key:k,expected_version:None,..}) if k==&key));
                            let receipt=&w.state.objects[&key];
                            assert_eq!(receipt.owner,ORGANISM_ID);assert_eq!(receipt.version,0);
                            assert_eq!(receipt.bytes.len(),280);assert_eq!(receipt.bytes[0],task.held_out_input.len() as u8);
                            assert_eq!(&receipt.bytes[1..33],Sha256::digest(&task.held_out_input).as_slice());
                            assert!(receipt.bytes[33..].iter().all(|b|*b==0x55));
                            assert_eq!(escrow.sequence.as_ref().unwrap().settled[settled],
                                TransformSettlement{outcome_key:key,offer_version:settled as u64,step:e.step,price_a:*price});
                            settled+=1;
                        }
                    }
                    assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
                    assert_eq!(credit,tasks.iter().take(settled).map(|(_,p)|*p).sum::<u64>());
                    assert!(settled<=3);
                    assert!(quotes<=1);assert!(third_attempts<=1);
                    if mode<2{assert_eq!(quotes,0);}
                    if mode>=2 && third_price==1{assert_eq!(third_attempts,0);assert!(settled<=2);}
                    sums[mode as usize]+=balance;
                    println!("S29_COMPACT_QUOTE tape={tape} grant={grant} fee={fee} third_price={third_price} mode={mode} root={hash} root_B={} install_A={install} settled={settled} quotes={quotes} quote_ok={quote_ok} quote_fees={quote_fees} third_attempts={third_attempts} credit={credit} fees={fees} final_A={balance} payer_A={payer} gas={gas} return={:?} replay=true",raw.len(),w.state.organisms[&ORGANISM_ID].last_return);
                }
            }
            let delta=sums[2] as i64-sums[0].max(sums[1]) as i64;
            let reordered=sums[3] as i64-sums[0].max(sums[1]) as i64;
            let compact=sums[4] as i64-sums[0].max(sums[1]) as i64;
            totals.push((tape,grant,fee,sums,delta,reordered,compact));
        }
        }
        }
        println!("S29_COMPACT_QUOTE_RESULTS equal_mixture_twice_mean={totals:?}");
    }

}
