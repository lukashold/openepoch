//! Authoritative source identity outside the unchanged Ent176 numeric layout.
//! CitizenM/F share the verified numeric template8. Identity survives cargo,
//! handle reuse, saves and ownership changes; the renderer never chooses it.
use crate::{game::*, defs::*, native_rng9::{NativeRng, CitizenIdentity}};

impl Game {
    /// Keep the sound event layout. Capture the identity before the actor
    /// slot is freed, using an integer exactly represented in f32.
    pub(super) fn citizen_death_cue(&self, index: usize) -> Option<f32> {
        let actor = self.ents[index];
        if actor.def != WORKER {
            return None;
        }
        let identity = self.source_identity(self.h(index))?;
        let team = actor.team as usize;
        let civilization = u32::from(*self.civilizations.get(team)?);
        let epoch = u32::from(self.players.get(team)?.epoch);
        if identity > 1 || civilization > 14 || epoch > 14 {
            return None;
        }
        Some((1024 + 2 * (epoch * 15 + civilization) + identity) as f32)
    }
}

pub const MAGIC:u32=0x31524942; // BIR1
pub const INNER_MAGIC:u32=0x3955454f; // OEU9: numeric8 plus mandatory source identity
const END:u32=0x45524942; // BIRE
const HEADER:usize=64;
const STRIDE:usize=16;
pub const NOT_CITIZEN:u32=2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot { pub handle:u32, pub identity:u32, pub cargo_females:u32 }
const EMPTY:Slot=Slot{handle:u32::MAX,identity:NOT_CITIZEN,cargo_females:0};

pub struct SourceBirth {
    pub rng:NativeRng,
    /// 0: own match seed adapter;1: reconstructed on import of an older codec.
    /// Neither value claims the complete original initialization/draw sequence.
    pub origin:u32,
    pub slots:Vec<Slot>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world()->Game {
        let mut g=Game::new(7,0,0,2,0);g.net=false;g.cheats_enabled=true;g.instant_build=true;
        for p in &mut g.players{p.res=[9000.0;3];}g
    }
    fn saved(g:&mut Game)->Vec<u8>{g.save();assert!(!g.savebuf.is_empty());g.savebuf.clone()}

    #[test]
    fn initial_explicit_males_and_normal_production_have_distinct_paths(){
        let mut g=world();assert!(g.validate_source_birth());assert_eq!(g.source_birth.rng.state(),7);
        for i in 0..g.high{if g.ents[i].alive&&g.ents[i].def==WORKER{assert_eq!(g.source_identity(g.h(i)),Some(0));}}
        let mut oracle=NativeRng::from_state(7);let mut saw=[false;2];
        for _ in 0..6 {
            assert!(g.train(0,WORKER as QueueItem));let before=g.high;let expected=(oracle.next_u32()>>16)&1;
            g.tick();assert_eq!(g.source_birth.rng.state(),oracle.state());
            let created=(before..g.high).find(|&i|g.ents[i].alive&&g.ents[i].def==WORKER).unwrap();
            assert_eq!(g.source_identity(g.h(created)),Some(expected));saw[expected as usize]=true;
            assert!(g.validate_source_birth());
        }assert!(saw.into_iter().all(|v|v));
        let bytes=saved(&mut g);let mut loaded=Game::load(&bytes).unwrap();
        assert_eq!(g.state_hash(),loaded.state_hash());assert_eq!(bytes,saved(&mut loaded));
        assert_eq!(loaded.source_birth.origin,0);
    }

    #[test]
    fn female_identity_is_cargo_state_and_partial_unload_compacts_actual_slots(){
        let mut g=world();let city=g.ents[0];let host=g.spawn(UNIVERSITY,0,city.x+10.0,city.y);
        let male=g.spawn(WORKER,0,city.x+11.0,city.y);let female=g.spawn(WORKER,0,city.x+11.2,city.y);
        g.set_citizen_identity(female,CitizenIdentity::Female);let old_female=g.h(female);
        g.order_board(&[male,female],host);g.tick();
        assert_eq!(g.ents[host].ncargo,2);assert_eq!(g.source_birth.slots[host].cargo_females,2);
        assert_eq!(g.source_identity(old_female),None);assert!(g.validate_source_birth());
        let before_rng=g.source_birth.rng.state();let bytes=saved(&mut g);let mut g=Game::load(&bytes).unwrap();
        let old_free=g.free.clone();let only=*old_free.last().unwrap();g.free=vec![only];
        // Deliberate allocator fault injection: one spawn succeeds, the second
        // fails. Restore the complete allocator before serializing this world.
        g.order_unload(&[host],0.0,0.0);
        assert_eq!(g.ents[host].ncargo,1);assert_eq!(g.source_birth.slots[host].cargo_females,1);
        assert_eq!(g.source_identity(g.h(only as usize)),Some(0));
        g.free=old_free.into_iter().filter(|&v|v!=only).collect();
        assert_eq!(g.source_birth.rng.state(),before_rng);let bytes=saved(&mut g);let mut g=Game::load(&bytes).unwrap();
        g.order_unload(&[host],0.0,0.0);assert_eq!(g.ents[host].ncargo,0);
        assert_eq!(g.source_birth.slots[host].cargo_females,0);
        assert_eq!(g.source_birth.rng.state(),before_rng);
        assert!(g.ents[..g.high].iter().enumerate().any(|(i,e)|e.alive&&e.def==WORKER&&g.source_identity(g.h(i))==Some(1)));
        assert!(g.validate_source_birth());
    }

    #[test]
    fn dead_full_handle_cannot_leak_identity_into_reused_slot(){
        let mut g=world();let city=g.ents[0];let i=g.spawn(WORKER,0,city.x+9.0,city.y);
        g.set_citizen_identity(i,CitizenIdentity::Female);let old=g.h(i);g.ents[i].hp=0.0;g.tick();
        assert_eq!(g.source_identity(old),None);
        let replacement=g.spawn(WORKER,0,city.x+9.0,city.y);assert_eq!(replacement,i);
        assert_ne!(g.h(i),old);assert_eq!(g.source_identity(g.h(i)),Some(0));assert!(g.validate_source_birth());
    }

    #[test]
    fn failed_birth_retains_paid_job_but_consumes_attempt_name_draw(){
        let mut g=world();assert!(g.train(0,WORKER as QueueItem));let price=g.queue_payment(0,0);
        let old_free=std::mem::take(&mut g.free);let mut oracle=NativeRng::from_state(7);
        g.tick();oracle.next_u32();assert_eq!(g.source_birth.rng.state(),oracle.state());
        assert_eq!(g.ents[0].qn,1);assert_eq!(g.queue_payment(0,0).price,price.price);
        g.free=old_free;let bytes=saved(&mut g);let mut g=Game::load(&bytes).unwrap();
        g.tick();oracle.next_u32();assert_eq!(g.source_birth.rng.state(),oracle.state());assert_eq!(g.ents[0].qn,0);
        assert!(g.validate_source_birth());
    }
}
impl SourceBirth {
    pub fn new(seed:u32)->Self { Self{rng:NativeRng::from_state(seed),origin:0,slots:vec![EMPTY;MAXE]} }
    pub fn register(&mut self,index:usize,handle:u32,worker:bool) {
        self.slots[index]=Slot{handle,identity:if worker{0}else{NOT_CITIZEN},cargo_females:0};
    }
    pub fn discard(&mut self,index:usize) {self.slots[index]=EMPTY;}
    pub fn production_identity(&mut self)->CitizenIdentity {
        self.rng.resolve_normal_unit("Citizen").citizen_identity.unwrap()
    }
    pub fn set_cargo(&mut self,host:usize,position:usize,female:bool) {
        debug_assert!(position<CARGO_MAX&&position<32);
        let bit=1_u32<<position;
        if female {self.slots[host].cargo_females|=bit;}else{self.slots[host].cargo_females&=!bit;}
    }
}

fn u(b:&[u8],at:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(at..at+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}

pub fn current_outer(b:&[u8])->bool{u(b,0)==Some(crate::pack8_save::MAGIC)&&u(b,4)==Some(4)}
pub fn mark_inner(b:&mut [u8])->bool{
    let Some(at)=b.len().checked_sub(52)else{return false;};
    if u(b,0)!=Some(crate::civ8_save::MAGIC)||u(b,at)!=Some(0x31533652)||u(b,at+4)!=Some(8)||u(b,at+8)!=Some(1)||b.get(at+12..at+44)!=Some(&crate::civ8_registry::FINGERPRINT[..]){return false;}
    b[..4].copy_from_slice(&INNER_MAGIC.to_le_bytes());
    b[at+4..at+8].copy_from_slice(&9_u32.to_le_bytes());
    b[at+12..at+44].copy_from_slice(&crate::citizen9_spec::FINGERPRINT);true
}
pub fn normalize_inner(b:&[u8],current:bool)->Option<Vec<u8>>{
    let magic=u(b,0)?;
    if !current {return if magic==INNER_MAGIC{None}else{Some(b.to_vec())};}
    let at=b.len().checked_sub(52)?;
    if magic!=INNER_MAGIC||u(b,at)?!=0x31533652||u(b,at+4)?!=9||u(b,at+8)?!=1||b.get(at+12..at+44)?!=crate::citizen9_spec::FINGERPRINT||u(b,at+44)?!=52||u(b,at+48)?!=0x45533652{return None;}
    let mut normalized=b.to_vec();
    normalized[..4].copy_from_slice(&crate::civ8_save::MAGIC.to_le_bytes());
    normalized[at+4..at+8].copy_from_slice(&8_u32.to_le_bytes());
    normalized[at+12..at+44].copy_from_slice(&crate::civ8_registry::FINGERPRINT);
    Some(normalized)
}

pub fn framed_len(b:&[u8])->Option<usize>{
    if u(b,0)?!=MAGIC||u(b,4)?!=1||u(b,16)?!=STRIDE as u32||u(b,24)?>1||u(b,28)?!=HEADER as u32{return None;}
    if b.get(32..64)?!=crate::citizen9_spec::FINGERPRINT{return None;}
    let count=u(b,12)? as usize;if count>MAXE{return None;}
    let length=HEADER.checked_add(count.checked_mul(STRIDE)?)?.checked_add(4)?;
    if u(b,8)? as usize!=length||u(b,length.checked_sub(4)?)?!=END{return None;}
    Some(length)
}

impl Game {
    pub(crate) fn source_identity(&self,handle:u32)->Option<u32>{
        let i=self.get(handle)?;let s=self.source_birth.slots[i];
        if s.handle!=handle{return None;}Some(s.identity)
    }
    pub(crate) fn set_citizen_identity(&mut self,index:usize,identity:CitizenIdentity){
        debug_assert!(self.ents[index].alive&&self.ents[index].def==WORKER);
        let handle=self.h(index);let s=&mut self.source_birth.slots[index];
        debug_assert_eq!(s.handle,handle);s.identity=identity as u32;
    }
    pub(crate) fn validate_source_birth(&self)->bool{
        if self.source_birth.origin>1||self.source_birth.slots.len()!=MAXE||CARGO_MAX>32{return false;}
        for i in 0..self.high {
            let e=&self.ents[i];if !e.alive{continue;}let s=self.source_birth.slots[i];
            if s.handle!=self.h(i)||(if e.def==WORKER{s.identity>1}else{s.identity!=NOT_CITIZEN}){return false;}
            let allowed=e.cargo[..e.ncargo as usize].iter().enumerate().fold(0_u32,|m,(k,&id)|m|if id==WORKER{1<<k}else{0});
            if s.cargo_females&!allowed!=0{return false;}
        }true
    }
    pub(crate) fn append_source_birth(&self,b:&mut Vec<u8>)->bool{
        if !self.validate_source_birth(){return false;}
        let count=self.ents[..self.high].iter().filter(|e|e.alive).count();
        let total=HEADER+count*STRIDE+4;
        for v in[MAGIC,1,total as u32,count as u32,STRIDE as u32,self.source_birth.rng.state(),self.source_birth.origin,HEADER as u32]{put(b,v);}
        b.extend_from_slice(&crate::citizen9_spec::FINGERPRINT);
        for i in 0..self.high{if self.ents[i].alive{let s=self.source_birth.slots[i];for v in[s.handle,s.identity,s.cargo_females,0]{put(b,v);}}}
        put(b,END);true
    }
    pub(crate) fn read_source_birth(&mut self,chunk:Option<&[u8]>)->Option<()>{
        let mut parsed=SourceBirth::new(self.seed0);
        let Some(b)=chunk else{
            parsed.origin=1;
            // Old own8 could only create the explicit numeric CitizenM type.
            // Reconstruct that known identity without inventing old RNG draws.
            for i in 0..self.high{if self.ents[i].alive{parsed.register(i,self.h(i),self.ents[i].def==WORKER);}}
            self.source_birth=parsed;return Some(());
        };
        if framed_len(b)?!=b.len(){return None;}
        let count=self.ents[..self.high].iter().filter(|e|e.alive).count();if u(b,12)? as usize!=count{return None;}
        parsed.rng=NativeRng::from_state(u(b,20)?);parsed.origin=u(b,24)?;
        let mut at=HEADER;
        for i in 0..self.high {
            let e=&self.ents[i];if !e.alive{continue;}
            let handle=u(b,at)?;let identity=u(b,at+4)?;let cargo_females=u(b,at+8)?;
            if handle!=self.h(i)||u(b,at+12)?!=0||(if e.def==WORKER{identity>1}else{identity!=NOT_CITIZEN}){return None;}
            let allowed=e.cargo[..e.ncargo as usize].iter().enumerate().fold(0_u32,|m,(k,&id)|m|if id==WORKER{1<<k}else{0});
            if cargo_females&!allowed!=0{return None;}
            parsed.slots[i]=Slot{handle,identity,cargo_females};at+=STRIDE;
        }
        self.source_birth=parsed;Some(())
    }
    pub(crate) fn source_birth_hash_words(&self)->Vec<u32>{
        let mut out=vec![MAGIC,1,self.source_birth.rng.state(),self.source_birth.origin];
        out.extend(crate::citizen9_spec::FINGERPRINT.chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())));
        for i in 0..self.high{if self.ents[i].alive{let s=self.source_birth.slots[i];out.extend([s.handle,s.identity,s.cargo_females]);}}
        out
    }
}
