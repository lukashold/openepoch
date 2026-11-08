//! Own256-grid pathfinder continuation state, not original EE2 map data.
//! The four-BFS-per-tick quota makes these fields decision state, not a
//! disposable graphics cache. Keep the complete inner OEU8 bytes unchanged.
use crate::{game::*,map::{self,Field}};
pub const MAGIC:u32=0x3143564e; // NVC1
pub const END:u32=0x4543564e; // NVCE
pub const HEADER:usize=48;
pub const FIELD_HEADER:usize=16;
const BORDER_FIELD_HEADER:usize=20;
pub const ACTOR:usize=8;
pub const FIELD_COUNT:usize=96;
fn u(b:&[u8],p:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(p..p+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}

/// Header-only bound, before any allocation. The caller separately verifies
/// the exact outer footer/checksum; read_navigation validates every record.
pub fn framed_len(b:&[u8])->Option<usize>{
 let field_header=match u(b,4)?{1=>FIELD_HEADER,2=>BORDER_FIELD_HEADER,_=>return None};
 if b.len()<HEADER+4||u(b,0)?!=MAGIC||u(b,12)?as usize!=FIELD_COUNT||u(b,16)?as usize!=map::NN||u(b,24)?>4||u(b,28)?as usize>MAXE||u(b,32)?>1||u(b,36)?as usize!=ACTOR||u(b,40)?as usize!=field_header||u(b,44)?as usize!=HEADER{return None;}
 let n=u(b,8)?as usize;let min=HEADER+FIELD_COUNT*field_header+u(b,28)?as usize*ACTOR+4;
 if n<min||n>min+FIELD_COUNT*map::NN||n>b.len()||u(b,n-4)?!=END{return None;}Some(n)
}

#[cfg(test)]mod tests{
 use super::*;use crate::defs::*;
 fn fixture()->Game{
  let mut g=Game::new(7,0,0,1,0);g.tick=40;
  for (slot,key)in[123u32,456].into_iter().enumerate(){g.map.fields[slot]=Field{key,denied_territories:0,ver:g.map.nav_ver,used:39+slot as u32,dir:vec![255;map::NN]};g.map.fields[slot].dir[key as usize]=8;}
  let ids:Vec<_>=(0..g.high).filter(|&i|g.ents[i].alive).take(2).collect();for(slot,&i)in ids.iter().enumerate(){g.ents[i].fslot=slot as u16;}
  g.map.budget=2;g
 }
 fn put_at(b:&mut[u8],p:usize,v:u32){b[p..p+4].copy_from_slice(&v.to_le_bytes());}
 #[test]fn full_fields_order_quota_slots_and_modern_bytes_roundtrip(){
  let mut g=fixture();let before=g.state_hash();g.save();let bytes=g.savebuf.clone();
  let mut loaded=Game::load(&bytes).unwrap();assert_eq!(loaded.state_hash(),before);loaded.save();assert!(loaded.savebuf==bytes);
  assert_eq!(loaded.map.nav_ver,g.map.nav_ver);assert_eq!(loaded.map.budget,2);assert!(!loaded.navigation_history_unknown);
  for(a,b)in loaded.map.fields.iter().zip(&g.map.fields){assert_eq!((a.key,a.ver,a.used),(b.key,b.ver,b.used));assert!(a.dir==b.dir);}
  for(a,b)in loaded.ents.iter().zip(&g.ents){if b.alive{assert_eq!(a.fslot,b.fslot);}}
 }
 #[test]fn navigation_header_directions_actor_and_tail_errors_reject_without_mutation(){
  let mut g=fixture();let mut bytes=Vec::new();g.append_navigation(&mut bytes);let before=g.state_hash();
  let second=HEADER+FIELD_HEADER+map::NN;let actors=HEADER+FIELD_COUNT*FIELD_HEADER+2*map::NN;let mut cases=Vec::new();
  for(p,v)in[(0,0),(4,2),(8,u32::MAX),(12,95),(16,1),(24,5),(28,MAXE as u32+1),(32,2),(36,7),(40,15),(44,47),(HEADER,u32::MAX),(HEADER,0x20000),(HEADER+8,41),(HEADER+12,65535),(second,123),(actors,g.h(0)^65536),(actors+4,96)]{let mut b=bytes.clone();put_at(&mut b,p,v);cases.push(b);}
  for p in[HEADER+FIELD_HEADER,HEADER+FIELD_HEADER+map::NN-1]{let mut b=bytes.clone();b[p]=9;cases.push(b);}
  for n in[0,1,4,47,48,bytes.len()-1,bytes.len()-4]{cases.push(bytes[..n].to_vec());}
  let mut b=bytes.clone();b.push(0);cases.push(b);
  for b in cases{assert!(g.read_navigation(Some(&b)).is_none());assert_eq!(g.state_hash(),before);}
 }
 #[test]fn every_decision_field_order_direction_and_slot_affects_hash(){
  let mut g=fixture();let hash=g.state_hash();
  g.map.nav_ver+=1;assert_ne!(g.state_hash(),hash);g.map.nav_ver-=1;
  g.map.budget-=1;assert_ne!(g.state_hash(),hash);g.map.budget+=1;
  g.navigation_history_unknown=true;assert_ne!(g.state_hash(),hash);g.navigation_history_unknown=false;
  g.map.fields[0].key+=1;assert_ne!(g.state_hash(),hash);g.map.fields[0].key-=1;
  g.map.fields[0].ver+=1;assert_ne!(g.state_hash(),hash);g.map.fields[0].ver-=1;
  g.map.fields[0].used-=1;assert_ne!(g.state_hash(),hash);g.map.fields[0].used+=1;
  g.map.fields[0].dir[0]=7;assert_ne!(g.state_hash(),hash);g.map.fields[0].dir[0]=255;
  g.map.fields.swap(0,1);assert_ne!(g.state_hash(),hash);g.map.fields.swap(0,1);
  let i=(0..g.high).find(|&i|g.ents[i].alive&&g.ents[i].def==TC).unwrap();let slot=g.ents[i].fslot;g.ents[i].fslot=65535;assert_ne!(g.state_hash(),hash);g.ents[i].fslot=slot;
  assert_eq!(g.state_hash(),hash);
 }
}

impl Game{
 pub(crate) fn append_navigation(&self,b:&mut Vec<u8>){
  let count=self.ents[..self.high].iter().filter(|e|e.alive).count();
  let borders=self.map.fields.iter().any(|f|f.denied_territories!=0);
  let field_header=if borders{BORDER_FIELD_HEADER}else{FIELD_HEADER};
  let bytes=HEADER+FIELD_COUNT*field_header+self.map.fields.iter().map(|f|f.dir.len()).sum::<usize>()+count*ACTOR+4;
  for v in[MAGIC,1+u32::from(borders),bytes as u32,FIELD_COUNT as u32,map::NN as u32,self.map.nav_ver,self.map.budget,count as u32,self.navigation_history_unknown as u32,ACTOR as u32,field_header as u32,HEADER as u32]{put(b,v);}
  for f in &self.map.fields{
   for v in[f.key,f.ver,f.used,f.dir.len()as u32]{put(b,v);}
   if borders{put(b,f.denied_territories);}
   b.extend_from_slice(&f.dir);
  }
  for i in 0..self.high{if self.ents[i].alive{put(b,self.h(i));put(b,self.ents[i].fslot as u32);}}
  put(b,END);
 }
 pub(crate) fn read_navigation(&mut self,b:Option<&[u8]>)->Option<()>{
  let Some(b)=b else{self.navigation_history_unknown=true;return Some(());};
  if framed_len(b)?!=b.len()||u(b,28)?as usize!=self.ents[..self.high].iter().filter(|e|e.alive).count(){return None;}
  let field_header=u(b,40)?as usize;
  let borders=field_header==BORDER_FIELD_HEADER;
  #[cfg(feature="ee2-native13")]
  if borders&&self.diplomacy.borders.is_none(){return None;}
  #[cfg(not(feature="ee2-native13"))]
  if borders{return None;}
  let mut fields=Vec::with_capacity(FIELD_COUNT);let mut keys=Vec::with_capacity(FIELD_COUNT);let mut p=HEADER;
  for _ in 0..FIELD_COUNT{
   let key=u(b,p)?;let ver=u(b,p+4)?;let used=u(b,p+8)?;let n=u(b,p+12)?as usize;
   let denied_territories=if borders{u(b,p+16)?}else{0};p=p.checked_add(field_header)?;
   if used>self.tick{return None;}
   if denied_territories.checked_shr(self.map.tseeds.len()as u32).unwrap_or(0)!=0{return None;}
   if key==u32::MAX{if n!=0||ver!=0||used!=0||denied_territories!=0{return None;}}
   else{if key>>16>1||n!=map::NN||keys.contains(&(key,denied_territories)){return None;}keys.push((key,denied_territories));}
   let dir=b.get(p..p.checked_add(n)?)?;p+=n;
   if dir.iter().any(|&v|v>8&&v!=255){return None;}
   fields.push(Field{key,denied_territories,ver,used,dir:dir.to_vec()});
  }
  let mut slots=Vec::new();
  for i in 0..self.high{if self.ents[i].alive{
   if u(b,p)?!=self.h(i){return None;}let slot=u(b,p+4)?;p+=ACTOR;
   if slot!=u16::MAX as u32&&(slot as usize>=FIELD_COUNT||fields[slot as usize].dir.is_empty()){return None;}
   slots.push((i,slot as u16));
  }}
  if p.checked_add(4)?!=b.len()||u(b,p)?!=END{return None;}
  self.map.fields=fields;self.map.nav_ver=u(b,20)?;self.map.budget=u(b,24)?;
  self.navigation_history_unknown=u(b,32)?!=0;
  for(i,slot)in slots{self.ents[i].fslot=slot;}Some(())
 }
 pub(crate) fn hash_navigation(&self,mut mix:impl FnMut(u32)){
  mix(MAGIC);mix(self.map.nav_ver);mix(self.map.budget);mix(self.navigation_history_unknown as u32);
  let borders=self.map.fields.iter().any(|f|f.denied_territories!=0);
  if borders{mix(u32::from_le_bytes(*b"BRNV"));}
  mix(self.map.fields.len()as u32);
  for f in &self.map.fields{
   mix(f.key);mix(f.ver);mix(f.used);mix(f.dir.len()as u32);
   if borders{mix(f.denied_territories);}
   for b in f.dir.chunks_exact(4){mix(u32::from_le_bytes(b.try_into().unwrap()));}
  }
  for i in 0..self.high{if self.ents[i].alive{mix(self.h(i));mix(self.ents[i].fslot as u32);}}
 }
}
