//! Own projectile continuation, separate from navigation and presentation FX.
//! Preserve launch-time damage/profile/target and vector order, including a
//! target handle whose actor has since died or whose slot has been recycled.
use crate::{game::*,defs::*,map::N};
pub const MAGIC:u32=0x314a5250; // PRJ1
pub const END:u32=0x454a5250; // PRJE
pub const HEADER:usize=32;pub const RECORD:usize=64;pub const MAX:usize=4000;
fn u(b:&[u8],p:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(p..p+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}
fn record_length(version: u32) -> Option<usize> {
    match version {
        1 => Some(RECORD),
        2 if cfg!(feature = "ee2-native13") => Some(RECORD + 4),
        _ => None,
    }
}
pub fn framed_len(b:&[u8])->Option<usize>{
 let record=record_length(u(b,4)?)?;
 if b.len()<HEADER+4||u(b,0)?!=MAGIC||u(b,12)?as usize>MAX||u(b,16)?as usize!=record||u(b,20)?>1||u(b,24)?as usize!=HEADER||u(b,28)?as usize!=MAX{return None;}
 let size=HEADER.checked_add((u(b,12)?as usize).checked_mul(record)?)?.checked_add(4)?;
 if u(b,8)?as usize!=size||size>b.len()||u(b,size-4)?!=END{return None;}Some(size)
}
fn words(p:&Proj)->[u32;16]{[p.x.to_bits(),p.y.to_bits(),p.px.to_bits(),p.py.to_bits(),p.tx.to_bits(),p.ty.to_bits(),p.tgt,p.speed.to_bits(),p.dmg.to_bits(),p.kind as u32,p.team as u32,p.splash.to_bits(),p.targets as u32,p.attacker_profile as u32,p.friendly_ratio.to_bits(),p.d0.to_bits()]}
impl Game{
 pub(crate) fn append_projectiles(&self,b:&mut Vec<u8>){
  let source=cfg!(feature="ee2-native13")&&self.source_clock.is_some();
  let version=if source{2}else{1};
  let record=if source{RECORD+4}else{RECORD};
  let size=HEADER+self.projs.len()*record+4;
  for v in[MAGIC,version,size as u32,self.projs.len()as u32,record as u32,self.projectile_history_unknown as u32,HEADER as u32,MAX as u32]{put(b,v);}
  for p in &self.projs{
   for v in words(p){put(b,v);}
   #[cfg(feature="ee2-native13")]
   if source{put(b,p.attack_source);}
  }
  put(b,END);
 }
 pub(crate) fn read_projectiles(&mut self,b:Option<&[u8]>)->Option<()>{
  let Some(b)=b else{self.projs.clear();self.projectile_history_unknown=true;return Some(());};
  if framed_len(b)?!=b.len(){return None;}
  let version=u(b,4)?;
  if version==2&&self.source_clock.is_none(){return None;}
  let record=record_length(version)?;
  let mut projs=Vec::with_capacity(u(b,12)?as usize);
  for r in b[HEADER..b.len()-4].chunks_exact(record){
   let w:Vec<u32>=(0..16).map(|k|u(r,k*4).unwrap()).collect();let f=|k|f32::from_bits(w[k]);
   if [0,1,2,3,4,5,7,8,11,14,15].iter().any(|&i|!f(i).is_finite())||[0,1,2,3,4,5].iter().any(|&i|f(i)<0.0||f(i)>N as f32)||[7,8,11,14].iter().any(|&i|f(i)<0.0)||f(15)<=0.0{return None;}
   let profile=w[13];
   let allowed=profile==crate::ee2_combat::UNKNOWN_PROFILE as u32||(profile as usize)<crate::ee2_combat::PROFILES.len();
   #[cfg(feature="ee2-native13")]
   let allowed=allowed||(self.air_missions_enabled&&profile==u32::from(crate::ee2_combat::profile(crate::air31_defs::FIGHTER_BOMBER,10)));
   if (w[6]!=NONE&&(w[6]&65535)as usize>=MAXE)||w[9]>W_BOMB as u32||w[10]as usize>=self.nplayers||w[12]&!15!=0||!allowed{return None;}
   #[cfg(feature="ee2-native13")]
   let attack_source=if version==2{u(r,RECORD)?}else{NONE};
   #[cfg(feature="ee2-native13")]
   if attack_source!=NONE&&(attack_source&65535)as usize>=MAXE{return None;}
   projs.push(Proj{x:f(0),y:f(1),px:f(2),py:f(3),tx:f(4),ty:f(5),tgt:w[6],speed:f(7),dmg:f(8),kind:w[9]as u8,team:w[10]as u8,splash:f(11),targets:w[12]as u8,attacker_profile:w[13]as u16,friendly_ratio:f(14),d0:f(15),
    #[cfg(feature = "ee2-native13")]
    attack_source,
   });
  }
  self.projs=projs;self.projectile_history_unknown=u(b,20)?!=0;Some(())
 }
 pub(crate) fn hash_projectiles(&self,mut mix:impl FnMut(u32)){
  mix(MAGIC);mix(self.projectile_history_unknown as u32);mix(self.projs.len()as u32);
  for p in &self.projs{
   for v in words(p){mix(v);}
   #[cfg(feature="ee2-native13")]
   if self.source_clock.is_some(){mix(p.attack_source);}
  }
 }
}
