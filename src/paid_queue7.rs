//! Prices captured when jobs are paid. Original 008167F0 copies eleven saved
//! resource floats; 006E38E0 refunds the nine natural-resource components.
//! Older saves cannot reveal their original payments and are marked inferred.
use crate::{defs::*, game::*};

pub const MAGIC:u32=0x31505150; // PQP1
pub const END:u32=0x45505150; // PQPE
pub const SAVE_MAGIC:u32=0x3755454f; // OEU7
const RECORD:usize=64;
const VERSION:u32=2;
const META:u32=0x31533652;
const META_END:u32=0x45533652;
pub const NATIVE_IDS:[usize;9]=[0,1,5,2,3,4,6,7,8];

///006E8461..8482 with a binary53, nearest evaluation context: division is
/// followed by double0.5, double103079215104, then __allshr16 on its bits.
/// This is intentionally not an f32 round/floor replacement. The original
/// callback's temporal x87 control-word context remains a separate source gap.
pub(crate) fn partial_purchase_integer53(gold:f32,rate:f32)->i32{
    let biased=(gold as f64/rate as f64+0.5_f64)+103079215104.0_f64;
    ((biased.to_bits()as i64)>>16)as i32
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Payment {
    pub handle:u32,
    pub position:u32,
    pub job:QueueItem,
    pub inferred:bool,
    pub purchased:bool,
    /// Native order: Food,Wood,Stone,Gold,Tin,Iron,Saltpeter,Oil,Uranium,Tech,Pop.
    pub price:[f32;11],
    /// Native TimeToProduce starts at-1 and is latched only when work begins.
    pub duration:f32,
}

fn u32v(b:&[u8],o:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(o..o+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}

impl Game {
    fn quoted_job_duration(&self,team:u8,job:QueueItem)->f32{
        if job==RESEARCH{EPOCH_TIME}
        else if original_research(job){crate::ee2_research::NODES[(job-ORIGINAL_RESEARCH0)as usize].time}
        else if(job as usize)<DEFS.len(){self.production_time(team,job as DefId)}else{0.0}
    }
    pub(crate) fn begin_paid_job(&mut self,b:usize)->(f32,bool){
        let mut p=self.queue_payment(b,0);
        if self.ents[b].qt>0.0&&p.duration>=0.0{return(p.duration,false);}
        p.duration=self.quoted_job_duration(self.ents[b].team,p.job);
        self.record_queue_payment(p);
        // OEU5/OEU6 jobs retain their old first-step continuation policy. New
        // captured orders use the source's literal0.01 startup increment.
        (p.duration,!p.inferred)
    }
    pub(crate) fn queue_time_limit(&self,b:usize,position:usize)->f32{
        let p=self.queue_payment(b,position);
        if p.duration>=0.0{p.duration}else{self.quoted_job_duration(self.ents[b].team,p.job)}
    }
    /// Financial commands can change reserved population between normal grid
    /// rebuilds. Recount only derived player fields, never the spatial grid.
    pub(crate) fn recount_paid_players(&mut self){
        let mut pop=[0_u32;4];let mut cap=[0_u32;4];let mut buildings=[0_u32;4];
        for e in &self.ents[..self.high]{
            if !e.alive{continue;}let t=e.team as usize;let epoch=self.players[t].epoch;let d=&DEFS[e.def as usize];
            pop[t]+=crate::economy::population(e.def,epoch);
            for &def in &e.cargo[..e.ncargo as usize]{pop[t]+=crate::economy::population(def,epoch);}
            if d.building{
                buildings[t]+=1;
                if e.built>=1.0{cap[t]+=self.modifiers[t][e.def as usize][18].round().max(0.0)as u32;}
                for &job in &e.q[..e.qn as usize]{if(job as usize)<DEFS.len(){pop[t]+=crate::economy::population(job as DefId,epoch);}}
            }
        }
        for t in 0..4{self.players[t].pop=pop[t];self.players[t].cap=cap[t].min(self.players[t].population_limit);self.players[t].buildings=buildings[t];}
    }
    pub(crate) fn quote_paid_job(&self,b:usize,position:usize,job:QueueItem,inferred:bool)->Payment {
        let e=self.ents[b];let mut price=[0.0;11];
        if job==RESEARCH {
            price[9]=crate::ee2_research::ADVANCE_COST[(self.players[e.team as usize].epoch as usize+1).min(MAX_EPOCH as usize)];
        }else if original_research(job){
            price[9]=crate::ee2_research::NODES[(job-ORIGINAL_RESEARCH0)as usize].cost;
        }else if (job as usize)<DEFS.len(){
            let cost=self.unit_cost(e.team,job as DefId);
            for r in 0..9{price[NATIVE_IDS[r]]=cost[r]as f32;}
            price[10]=crate::economy::population(job as DefId,self.players[e.team as usize].epoch)as f32;
        }
        let duration=if inferred&&position==0&&e.qt>0.0{self.quoted_job_duration(e.team,job)}else{-1.0};
        Payment{handle:self.h(b),position:position as u32,job,inferred,purchased:true,price,duration}
    }
    pub(crate) fn queue_payment(&self,b:usize,position:usize)->Payment {
        let handle=self.h(b);let job=self.ents[b].q[position];
        self.queue_prices.iter().find(|p|p.handle==handle&&p.position==position as u32&&p.job==job)
            .copied().unwrap_or_else(||self.quote_paid_job(b,position,job,true))
    }
    pub(crate) fn record_queue_payment(&mut self,payment:Payment){
        self.queue_prices.retain(|p|!(p.handle==payment.handle&&p.position==payment.position));
        self.queue_prices.push(payment);
    }
    pub(crate) fn remove_queue_payment(&mut self,handle:u32,position:usize){
        self.queue_prices.retain(|p|!(p.handle==handle&&p.position==position as u32));
        for p in &mut self.queue_prices{if p.handle==handle&&p.position>position as u32{p.position-=1;}}
    }
    pub(crate) fn clear_queue_payments(&mut self,index:usize){
        self.queue_prices.retain(|p|p.handle&0xffff!=index as u32);
    }
    pub(crate) fn purchase_front_job(&mut self,b:usize)->bool{
        if self.ents[b].qn==0{return true;}
        let old=self.queue_payment(b,0);
        if old.purchased{return true;}
        // Repricing may leave a cyclic producer's pending job unpaid. It must
        // acquire its new price before any progress is made at the front.
        if old.job as usize>=DEFS.len(){return false;}
        let team=self.ents[b].team;let cost=self.unit_cost(team,old.job as DefId);
        if !self.afford(team,&cost){return false;}
        let payment=self.quote_paid_job(b,0,old.job,false);
        self.pay(team,&cost,1.0);self.record_queue_payment(payment);true
    }
    fn discard_pending_job(&mut self,b:usize,position:usize){
        self.remove_queue_payment(self.h(b),position);
        let e=&mut self.ents[b];e.q.copy_within(position+1..e.qn as usize,position);
        e.qn-=1;e.q[e.qn as usize]=0;
    }
    pub(crate) fn change_reprice_gold53(&mut self,team:u8,delta:f64){
        // Native operations retain the extended/binary53 product through the
        // balance add/subtract and store only the final account value as f32.
        // Gold is core slot4, hence extra_resources slot1.
        let value=(self.resource(team,4)as f64+delta).clamp(0.0,999999.0)as f32;
        self.extra_resources[team as usize][1]=value;
    }

    /// Source006E8070: the active order is a separate return from845DB0 and is
    /// deliberately absent from its pending vector. Current producers have no
    /// cycle command; unaffordable pending orders use the normal delete branch.
    pub(crate) fn reprice_paid_pending(&mut self,team:u8,def:DefId){
        let quoted=self.quote_type_price(team,def);
        if quoted[..9].iter().map(|&v|v as f64).sum::<f64>()<1.0{return;}
        let mut changed=false;
        for b in 0..self.high{
            let e=self.ents[b];if !e.alive||e.team!=team||!DEFS[e.def as usize].building{continue;}
            let pending:Vec<usize>=(1..e.qn as usize).filter(|&p|e.q[p]==def as QueueItem&&self.queue_payment(b,p).purchased).collect();
            if pending.is_empty(){continue;}
            changed=true;
            let mut budget=[0.0_f32;9];let mut extra=[0.0_f32;9];
            for &p in &pending{let paid=self.queue_payment(b,p);for r in 0..9{budget[r]+=paid.price[r];}}
            // The existing smaller engine has no manual market transactions or
            // crown modifiers. Its initial quote is the proved native110/90.
            for r in 0..9{
                if budget[r]>=1.0&&quoted[r]<1.0{
                    let rate=if r==3{100.0}else{90.0};
                    self.change_reprice_gold53(team,rate as f64*0.009999999776482582_f64*budget[r]as f64);
                    budget[r]=0.0;
                }
                if budget[r]<1.0&&quoted[r]>=1.0{
                    let raw_rate=(if r==3{100.0}else{110.0})*0.009999999776482582_f64;
                    let rate=raw_rate as f32;let mut amount=pending.len()as f32*quoted[r];
                    let mut charge=raw_rate*amount as f64;
                    let gold=self.resource(team,4);
                    if (gold as f64)<charge{amount=partial_purchase_integer53(gold,rate)as f32;charge=rate as f64*amount as f64;}
                    if amount>0.0{self.change_reprice_gold53(team,-charge);extra[r]=amount;budget[r]+=amount;}
                }
            }
            // Mirror the native repeated subtraction, including its uncapped
            // affordable count. This is not replaced with a different policy.
            let mut affordable=0_usize;
            while(0..9).all(|r|budget[r]>=quoted[r]){
                affordable+=1;for r in 0..9{budget[r]-=quoted[r];extra[r]-=quoted[r];}
            }
            let remove=pending.len().saturating_sub(affordable);
            for &p in pending.iter().rev().take(remove){self.discard_pending_job(b,p);}
            for p in 1..self.ents[b].qn as usize{
                if self.ents[b].q[p]==def as QueueItem{
                    let mut old=self.queue_payment(b,p);
                    if old.purchased{old.price=quoted;self.record_queue_payment(old);}
                }
            }
            for r in 0..9{
                if extra[r]>0.0{
                    budget[r]-=extra[r];
                    let quote=if r==3{-1.0_f64}else{90.0_f64};
                    self.change_reprice_gold53(team,quote*0.009999999776482582_f64*extra[r]as f64);
                }
                if budget[r]>0.0{
                    let core=NATIVE_IDS.iter().position(|&n|n==r).unwrap();self.add_resource(team,core,budget[r]);
                }
            }
        }
        if changed{self.recount_paid_players();}
    }
    fn quote_type_price(&self,team:u8,def:DefId)->[f32;11]{
        let mut p=[0.0;11];let cost=self.unit_cost(team,def);
        for r in 0..9{p[NATIVE_IDS[r]]=cost[r]as f32;}
        p[10]=crate::economy::population(def,self.players[team as usize].epoch)as f32;p
    }
    pub(crate) fn apply_paid_main(&mut self,team:u8,previous_epoch:u8){
        let epoch=self.players[team as usize].epoch;
        if epoch<=previous_epoch{return;}
        for a in crate::paid_queue7_main::ACTIONS{
            if a.epoch<=previous_epoch||a.epoch>epoch{continue;}
            match a.kind{
                crate::paid_queue7_main::Kind::Reprice=>self.reprice_paid_pending(team,a.def),
                crate::paid_queue7_main::Kind::Obsolete=>self.retire_paid_production(team,previous_epoch),
            }
        }
        self.recount_paid_players();
    }
    pub(crate) fn canonical_queue_payments(&self)->Vec<Payment>{
        let mut out=Vec::new();
        for b in 0..self.high{let e=self.ents[b];if e.alive&&DEFS[e.def as usize].building{
            for position in 0..e.qn as usize{out.push(self.queue_payment(b,position));}
        }}
        out
    }
    /// Unlike the old cancellation adapter, this never requotes a captured job.
    pub(crate) fn cancel_paid_job(&mut self,b:usize,position:usize)->bool {
        let e=self.ents[b];if !e.alive||position>=e.qn as usize{return false;}
        let item=e.q[position];let paid=self.queue_payment(b,position);
        if paid.purchased{
            for r in 0..9{self.add_resource(e.team,r,paid.price[NATIVE_IDS[r]]);}
            self.research_points[e.team as usize]+=paid.price[9];
        }
        if item==RESEARCH{self.players[e.team as usize].researching=false;}
        // Independent prototype tech jobs are forbidden in this profile.
        self.remove_queue_payment(paid.handle,position);
        let q=&mut self.ents[b];q.q.copy_within(position+1..q.qn as usize,position);
        q.qn-=1;q.q[q.qn as usize]=0;
        if position==0{q.qt=0.0;}
        self.recount_paid_players();
        true
    }
    pub(crate) fn retire_paid_production(&mut self,team:u8,previous_epoch:u8){
        let epoch=self.players[team as usize].epoch;
        if epoch<=previous_epoch{return;}
        for b in 0..self.high {
            if !self.ents[b].alive||self.ents[b].team!=team||!DEFS[self.ents[b].def as usize].building{continue;}
            let mut position=0;
            while position<self.ents[b].qn as usize {
                let item=self.ents[b].q[position];
                let obsolete=(57..DEFS.len()as u32).contains(&item)&&
                    previous_epoch<DEFS[item as usize].until&&epoch>=DEFS[item as usize].until;
                if obsolete{self.cancel_paid_job(b,position);}else{position+=1;}
            }
        }
        self.recount_paid_players();
    }
    pub(crate) fn write_paid_queue(&self,b:&mut Vec<u8>){
        let start=b.len();let records=self.canonical_queue_payments();
        for v in[MAGIC,VERSION,records.len()as u32]{put(b,v);}
        for p in records{
            for v in[p.handle,p.position,p.job,p.inferred as u32|((p.purchased as u32)<<1)]{put(b,v);}
            for f in p.price{put(b,f.to_bits());}
            put(b,p.duration.to_bits());
        }
        put(b,(b.len()+8-start)as u32);put(b,END);
    }
    pub(crate) fn read_paid_queue(&mut self,chunk:Option<&[u8]>)->Option<()> {
        let expected=self.canonical_queue_payments();
        let Some(b)=chunk else{
            self.queue_prices=expected.into_iter().map(|mut p|{p.inferred=true;p}).collect();
            return Some(());
        };
        if u32v(b,0)?!=MAGIC||u32v(b,4)?!=VERSION{return None;}
        let count=u32v(b,8)? as usize;
        if count!=expected.len()||b.len()!=20usize.checked_add(count.checked_mul(RECORD)?)?{return None;}
        let mut records=Vec::with_capacity(count);
        for (i,want) in expected.iter().enumerate(){
            let at=12+i*RECORD;let handle=u32v(b,at)?;let position=u32v(b,at+4)?;let job=u32v(b,at+8)?;let flag=u32v(b,at+12)?;
            if handle!=want.handle||position!=want.position||job!=want.job||flag>3{return None;}
            let mut price=[0.0;11];
            for(j,f)in price.iter_mut().enumerate(){let bits=u32v(b,at+16+j*4)?;*f=f32::from_bits(bits);if !f.is_finite()||bits>>31!=0||*f>65535.0{return None;}}
            let duration=f32::from_bits(u32v(b,at+60)?);
            if !duration.is_finite()||(duration!= -1.0&&(duration<0.0||duration>1000000.0)){return None;}
            // This FIFO profile has no queue reorder/suspend command. A job
            // can acquire its duration only after it reaches the front.
            if position>0&&duration!= -1.0{return None;}
            let host=self.get(handle)?;
            if position==0&&self.ents[host].qt>0.0&&(duration<0.0||flag&2==0){return None;}
            if position==0&&self.ents[host].qt==0.0&&duration>0.0{return None;}
            if (job as usize)<DEFS.len(){
                let host=self.get(handle)?;let d=&DEFS[job as usize];
                if d.building||d.made_at!=self.ents[host].def||price[9]!=0.0||price[10]>255.0||price[10].fract()!=0.0{return None;}
                if !(0..=MAX_EPOCH).any(|epoch|price[10]==crate::economy::population(job as DefId,epoch)as f32){return None;}
            }
            else{
                if flag&2==0||price[..9].iter().any(|&v|v!=0.0)||price[10]!=0.0{return None;}
                if original_research(job){if price[9]!=crate::ee2_research::NODES[(job-ORIGINAL_RESEARCH0)as usize].cost{return None;}}
                else if job==RESEARCH{if price[9]<=0.0||!crate::ee2_research::ADVANCE_COST.contains(&price[9]){return None;}}
                else{return None;}
            }
            records.push(Payment{handle,position,job,inferred:flag&1!=0,purchased:flag&2!=0,price,duration});
        }
        self.queue_prices=records;Some(())
    }
}

pub fn append_descriptor(b:&mut Vec<u8>){
    for v in[META,7,2]{put(b,v);}
    b.extend_from_slice(&crate::roster6_registry::FINGERPRINT);
    for v in[52,META_END]{put(b,v);}
}

pub fn strip_wrapper(input:&[u8])->Option<(&[u8],Option<&[u8]>)>{
    if u32v(input,0)?!=SAVE_MAGIC{return Some((input,None));}
    let descriptor=input.len().checked_sub(52)?;let d=input.get(descriptor..)?;
    if u32v(d,0)?!=META||u32v(d,4)?!=7||u32v(d,8)?!=2||d.get(12..44)?!=crate::roster6_registry::FINGERPRINT||u32v(d,44)?!=52||u32v(d,48)?!=META_END{return None;}
    if u32v(input,12)? as usize!=DEFS.len(){return None;}
    let payload=input.get(..descriptor)?;
    if u32v(payload,payload.len().checked_sub(4)?)?!=END{return None;}
    let length=u32v(payload,payload.len().checked_sub(8)?)? as usize;
    let start=payload.len().checked_sub(length)?;let chunk=payload.get(start..)?;
    if length<20||u32v(chunk,0)?!=MAGIC||u32v(chunk,4)?!=VERSION{return None;}
    let count=u32v(chunk,8)? as usize;
    if count>MAXE*5||length!=20usize.checked_add(count.checked_mul(RECORD)?)?{return None;}
    let prefix=payload.get(..start)?;
    if prefix.len()<4||u32v(prefix,prefix.len()-4)?!=crate::native_attack_clock::END{return None;}
    Some((prefix,Some(chunk)))
}

/// Convert only the checked wrapper to OEU6 for the existing explicit entity
/// normalizer. The immutable ledger is then validated against the loaded world.
pub fn split_input(input:&[u8])->Option<(Vec<u8>,Option<Vec<u8>>)> {
    let (prefix,chunk)=strip_wrapper(input)?;
    let Some(chunk)=chunk else{return Some((prefix.to_vec(),None));};
    let mut normalized=prefix.to_vec();
    normalized.get_mut(..4)?.copy_from_slice(&crate::save6::MAGIC6.to_le_bytes());
    crate::save6::append_descriptor(&mut normalized);
    Some((normalized,Some(chunk.to_vec())))
}
