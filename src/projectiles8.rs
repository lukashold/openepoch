//! Own projectile continuation, separate from navigation and presentation FX.
//! Preserve launch-time damage/profile/target and vector order, including a
//! target handle whose actor has since died or whose slot has been recycled.
use crate::{game::*,defs::*,map::N};
pub const MAGIC:u32=0x314a5250; // PRJ1
pub const END:u32=0x454a5250; // PRJE
pub const HEADER:usize=32;pub const RECORD:usize=64;pub const MAX:usize=4000;
#[cfg(feature = "ee2-native13")]
const GUIDED: usize = RECORD + 4;
fn u(b:&[u8],p:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(p..p+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}
fn record_length(version: u32) -> Option<usize> {
    match version {
        1 => Some(RECORD),
        2 if cfg!(feature = "ee2-native13") => Some(RECORD + 4),
        #[cfg(feature = "ee2-native13")]
        3 | 4 => Some(GUIDED + 4 + 8 * 4),
        _ => None,
    }
}
pub fn framed_len(b:&[u8])->Option<usize>{
 let version=u(b,4)?;
 let record=record_length(version)?;
 let allowed_flags=if version==4{3}else{1};
 if b.len()<HEADER+4||u(b,0)?!=MAGIC||u(b,12)?as usize>MAX||u(b,16)?as usize!=record||u(b,20)?>allowed_flags||u(b,24)?as usize!=HEADER||u(b,28)?as usize!=MAX{return None;}
 let size=HEADER.checked_add((u(b,12)?as usize).checked_mul(record)?)?.checked_add(4)?;
 if u(b,8)?as usize!=size||size>b.len()||u(b,size-4)?!=END{return None;}Some(size)
}
fn words(p:&Proj)->[u32;16]{[p.x.to_bits(),p.y.to_bits(),p.px.to_bits(),p.py.to_bits(),p.tx.to_bits(),p.ty.to_bits(),p.tgt,p.speed.to_bits(),p.dmg.to_bits(),p.kind as u32,p.team as u32,p.splash.to_bits(),p.targets as u32,p.attacker_profile as u32,p.friendly_ratio.to_bits(),p.d0.to_bits()]}
#[cfg(feature = "ee2-native13")]
fn flight_kind(p: &Proj) -> u32 {
    if p.guided.is_none() { 0 } else if p.kind == W_TORP { 2 } else { 1 }
}
impl Game{
 pub(crate) fn append_projectiles(&self,b:&mut Vec<u8>){
  let source=cfg!(feature="ee2-native13")&&self.source_clock.is_some();
  let version=if source{2}else{1};
  #[cfg(feature = "ee2-native13")]
  let version = if self.torpedo84_enabled { 4 } else if self.guided_herc_rocket { 3 } else { version };
  let flags = u32::from(self.projectile_history_unknown);
  #[cfg(feature = "ee2-native13")]
  let flags = flags | if version == 4 { u32::from(self.guided_herc_rocket) << 1 } else { 0 };
  let record=record_length(version).expect("supported projectile archive version");
  let size=HEADER+self.projs.len()*record+4;
  for v in[MAGIC,version,size as u32,self.projs.len()as u32,record as u32,flags,HEADER as u32,MAX as u32]{put(b,v);}
  for p in &self.projs{
   for v in words(p){put(b,v);}
   #[cfg(feature="ee2-native13")]
   if version >= 2 {put(b,p.attack_source);}
   #[cfg(feature = "ee2-native13")]
   if version >= 3 {
    put(b,flight_kind(p));
    for word in p.guided.map_or([0;8], crate::rocket_herc::Flight::words) {put(b,word);}
   }
  }
  put(b,END);
 }
 pub(crate) fn read_projectiles(&mut self,b:Option<&[u8]>)->Option<()>{
  let Some(b)=b else{self.projs.clear();self.projectile_history_unknown=true;return Some(());};
  if framed_len(b)?!=b.len(){return None;}
  let version=u(b,4)?;
  if version==2&&self.source_clock.is_none(){return None;}
  #[cfg(feature = "ee2-native13")]
  if version==3&&(self.net||!self.guided_herc_ordinary_map()){return None;}
  #[cfg(feature = "ee2-native13")]
  // The outer loader validates this capability once the actor ledger is restored.
  if version==4&&!self.submarine_depth_enabled{return None;}
  let flags = u(b,20)?;
  #[cfg(feature = "ee2-native13")]
  let herc = version == 3 || (version == 4 && flags & 2 != 0);
  let record=record_length(version)?;
  let bounds=self.map.simulation_dimensions().map(|side|side as f32);
  let mut projs=Vec::with_capacity(u(b,12)?as usize);
  for r in b[HEADER..b.len()-4].chunks_exact(record){
   let w:Vec<u32>=(0..16).map(|k|u(r,k*4).unwrap()).collect();let f=|k|f32::from_bits(w[k]);
   #[cfg(feature="ee2-native13")]
   let aim_margin = if version >= 3 && u(r,GUIDED)? == 1 { 8.0 } else { 0.0 };
   #[cfg(not(feature="ee2-native13"))]
   let aim_margin = 0.0;
   if [0,1,2,3,4,5,7,8,11,14,15].iter().any(|&i|!f(i).is_finite())
    || [0,1,2,3].iter().any(|&i|f(i)<0.0||f(i)>bounds[i%2])
    || [4,5].iter().any(|&i|f(i) < -aim_margin||f(i)>bounds[i%2]+aim_margin)
    || [7,8,11,14].iter().any(|&i|f(i)<0.0)||f(15)<=0.0{return None;}
   let profile=w[13];
   let allowed=profile==crate::ee2_combat::UNKNOWN_PROFILE as u32||(profile as usize)<crate::ee2_combat::PROFILES.len();
   #[cfg(feature="ee2-native13")]
   let allowed=allowed||(self.air_missions_enabled&&profile==u32::from(crate::ee2_combat::profile(crate::air31_defs::FIGHTER_BOMBER,10)));
   #[cfg(feature="ee2-native13")]
   let allowed = allowed || (self.heavy_mounted3 && w[9] == u32::from(W_MISSILE)
    && profile == u32::from(crate::ee2_combat::profile(crate::heavy_mounted3_61::ID, crate::heavy_mounted3_61::START)));
   if (w[6]!=NONE&&(w[6]&65535)as usize>=MAXE)||w[9]>W_BOMB as u32||w[10]as usize>=self.nplayers||w[12]&!15!=0||!allowed{return None;}
   #[cfg(feature="ee2-native13")]
   let attack_source=if version>=2{u(r,RECORD)?}else{NONE};
   #[cfg(feature="ee2-native13")]
   if attack_source!=NONE&&(attack_source&65535)as usize>=MAXE{return None;}
   #[cfg(feature = "ee2-native13")]
   let guided = if version >= 3 {
    let mut flight = [0;8];
    for (index, word) in flight.iter_mut().enumerate() { *word = u(r, GUIDED + 4 + 4 * index)?; }
    match u(r,GUIDED)? {
     0 if flight == [0;8] => None,
     1 if herc && w[9] == u32::from(W_MISSILE) && f(7) == crate::rocket_herc::SPEED && attack_source != NONE => {
      let artemis = (13..15).any(|epoch| profile == u32::from(crate::ee2_combat::profile(43, epoch)));
      let minotaurus = self.heavy_mounted3 && profile == u32::from(crate::ee2_combat::profile(
        crate::heavy_mounted3_61::ID, crate::heavy_mounted3_61::START));
      if !artemis && !minotaurus { return None; }
      Some(crate::rocket_herc::Flight::read(flight, self.sim_elapsed())?)
     },
     2 if version == 4 && w[9] == u32::from(W_TORP) && f(7) == crate::torpedo84::SPEED && attack_source != NONE => {
      if !crate::torpedo84::valid_profile(profile as u16) || flight[6] != crate::torpedo84::WATERLINE.to_bits() { return None; }
      Some(crate::rocket_herc::Flight::read_with_speed(flight, self.sim_elapsed(), crate::torpedo84::SPEED, true)?)
     },
     _ => return None,
    }
   } else { None };
   projs.push(Proj{x:f(0),y:f(1),px:f(2),py:f(3),tx:f(4),ty:f(5),tgt:w[6],speed:f(7),dmg:f(8),kind:w[9]as u8,team:w[10]as u8,splash:f(11),targets:w[12]as u8,attacker_profile:w[13]as u16,friendly_ratio:f(14),d0:f(15),
    #[cfg(feature = "ee2-native13")]
    attack_source,
    #[cfg(feature = "ee2-native13")]
    guided,
   });
  }
  #[cfg(feature = "ee2-native13")]
  { self.guided_herc_rocket = herc; self.torpedo84_enabled = version == 4; }
  self.projs=projs;self.projectile_history_unknown=flags&1!=0;Some(())
 }
 pub(crate) fn hash_projectiles(&self,mut mix:impl FnMut(u32)){
  mix(MAGIC);mix(self.projectile_history_unknown as u32);mix(self.projs.len()as u32);
  #[cfg(feature = "ee2-native13")]
  if self.torpedo84_enabled { mix(4); mix(u32::from(self.guided_herc_rocket)); }
  else if self.guided_herc_rocket { mix(3); }
  for p in &self.projs{
   for v in words(p){mix(v);}
   #[cfg(feature="ee2-native13")]
   if self.source_clock.is_some() || self.guided_herc_rocket || self.torpedo84_enabled {mix(p.attack_source);}
   #[cfg(feature = "ee2-native13")]
   if self.guided_herc_rocket || self.torpedo84_enabled {
    mix(flight_kind(p));
    if let Some(flight) = p.guided { for word in flight.words() {mix(word);} }
   }
  }
 }
}
