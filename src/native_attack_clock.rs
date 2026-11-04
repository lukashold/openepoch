//! Clean implementation of native007DB4E0/007DBB20 attack round timing.
//! Per-generation state lives outside Ent/Player and is saved in an optional ATK1
//! extension. Older UP saves migrate their already-running cooldowns.
#[derive(Clone,Copy,Debug)]pub struct Clock{pub handle:u32,pub next:f32,pub flags:u8}
pub const EMPTY:Clock=Clock{handle:u32::MAX,next:0.0,flags:0};
pub const MAGIC:u32=0x314b5441;pub const END:u32=0x454b5441;
pub const INIT:u8=0x80;pub const STARTED:u8=1;pub const EXECUTED:u8=2;pub const IN_ATTACK:u8=4;
#[derive(Debug,PartialEq)]pub enum Event{None,Start,Fire,End}
pub fn step(now:f32,reload:f32,apply:f32,next:&mut f32,flags:&mut u8)->Event{
 if *flags&INIT==0{*flags=INIT;*next=0.0;}
 *flags|=IN_ATTACK;
 if *flags&STARTED==0{
  if *next<=now{*next=((now as f64)+(reload as f64))as f32;*flags=(*flags|STARTED)&!EXECUTED;return Event::Start;}
 }else if *flags&EXECUTED==0{
  // Source comparison is strict and keeps source-float arithmetic extended.
  if (*next as f64)-(now as f64)<(reload as f64)-(apply as f64){*flags|=EXECUTED;return Event::Fire;}
 }else if *next<=now{*flags&=!STARTED;return Event::End;}
 Event::None
}
pub fn stop(flags:&mut u8){if *flags&INIT!=0{*flags=(*flags&!(STARTED|IN_ATTACK))|EXECUTED;}}
#[cfg(test)]mod tests{use super::*;
 #[test]fn strict_apply_boundary(){let(mut next,mut f)=(0.0,0);assert_eq!(step(0.0,3.0,0.5,&mut next,&mut f),Event::Start);assert_eq!(step(0.5,3.0,0.5,&mut next,&mut f),Event::None);assert_eq!(step(f32::from_bits(0.5f32.to_bits()+1),3.0,0.5,&mut next,&mut f),Event::Fire);assert_eq!(step(3.0,3.0,0.5,&mut next,&mut f),Event::End);assert_eq!(step(3.0,3.0,0.5,&mut next,&mut f),Event::Start);}
 #[test]fn zero_delay_still_starts_before_shooting(){let(mut n,mut f)=(0.0,0);assert_eq!(step(1.0,2.0,0.0,&mut n,&mut f),Event::Start);assert_eq!(step(1.0,2.0,0.0,&mut n,&mut f),Event::None);assert_eq!(step(1.01,2.0,0.0,&mut n,&mut f),Event::Fire);}
 #[test]fn stop_cancels_windup_and_keeps_reload_deadline(){let(mut n,mut f)=(0.0,0);step(1.0,3.0,0.5,&mut n,&mut f);stop(&mut f);assert_eq!(n,4.0);assert_eq!(step(2.0,3.0,0.5,&mut n,&mut f),Event::None);assert_eq!(step(4.0,3.0,0.5,&mut n,&mut f),Event::Start);}
}

pub fn split_save(buf:&[u8])->Option<(&[u8],Option<&[u8]>)>{
 #[cfg(feature="ee2-pack8")]let buf=crate::pack8_save::split(buf)?.0;
 #[cfg(feature="ee2-civ8")]let buf=crate::civ8_save::paid_prefix_view(buf)?;
 #[cfg(all(feature="ee2-paidqueue7",not(feature="ee2-civ8")))]let buf=crate::paid_queue7::strip_wrapper(buf)?.0;
 #[cfg(feature="ee2-roster6")]let buf=crate::save6::strip_if_present(buf)?;
 // Pre-extension saves can end in any resource/research float bit pattern.
 // ATKE is meaningful only beyond the complete current prefix plus a CGD tail.
 if buf.len()<36{return Some((buf,None));}
 let high=u32::from_le_bytes(buf[32..36].try_into().ok()?)as usize;
 if high>crate::game::MAXE{return None;}
 let prefix=36+core::mem::size_of::<[crate::game::Player;4]>()+
  core::mem::size_of::<crate::game::Ent>()*high+crate::map::NN*5+236+
  crate::diplomacy::DIP_BYTES+crate::territory::TERR_BYTES;
 if buf.len()<=prefix+20{return Some((buf,None));}
 if buf.len()<8||u32::from_le_bytes(buf[buf.len()-4..].try_into().ok()?)!=END{return Some((buf,None));}
 let bytes=u32::from_le_bytes(buf[buf.len()-8..buf.len()-4].try_into().ok()?)as usize;
 if bytes<20||bytes>buf.len(){return None;}let start=buf.len()-bytes;
 if start<prefix+20||u32::from_le_bytes(buf[start-4..start].try_into().ok()?)!=0x45444743{return None;}
 // Full CGD parsing also proves the attack chunk starts immediately after it.
 let(city_prefix,city)=crate::city_ground::split_save(&buf[..start])?;
 if city.is_none()||city_prefix.len()!=prefix{return None;}
 let tail=&buf[start..buf.len()-8];let count=u32::from_le_bytes(tail.get(8..12)?.try_into().ok()?)as usize;
 if count>crate::game::MAXE||bytes!=20+count*12{return None;}
 if u32::from_le_bytes(tail[..4].try_into().ok()?)!=MAGIC||u32::from_le_bytes(tail[4..8].try_into().ok()?)!=1{return None;}
 Some((&buf[..start],Some(tail)))
}
impl crate::game::Game{
 pub fn write_attack_clock(&self,b:&mut Vec<u8>){let start=b.len();b.extend_from_slice(&MAGIC.to_le_bytes());b.extend_from_slice(&1u32.to_le_bytes());let at=b.len();b.extend_from_slice(&0u32.to_le_bytes());let mut count=0u32;
  for i in 0..self.high{let c=self.attack_clock[i];if self.ents[i].alive&&c.handle==self.h(i){b.extend_from_slice(&c.handle.to_le_bytes());b.extend_from_slice(&c.next.to_le_bytes());b.extend_from_slice(&(c.flags as u32).to_le_bytes());count+=1;}}
  b[at..at+4].copy_from_slice(&count.to_le_bytes());let size=(b.len()-start+8)as u32;b.extend_from_slice(&size.to_le_bytes());b.extend_from_slice(&END.to_le_bytes());
 }
 pub fn read_attack_clock(&mut self,tail:Option<&[u8]>)->Option<()>{
  let Some(tail)=tail else{let now=self.tick as f32*crate::game::DT;for i in 0..self.high{let e=self.ents[i];if e.alive&&e.cd>0.0&&crate::ee2_up15_attack::ROWS.iter().any(|a|a.id==e.def&&a.present){if !e.cd.is_finite(){return None;}self.attack_clock[i]=Clock{handle:self.h(i),next:((now as f64)+(e.cd as f64))as f32,flags:INIT};}}return Some(());};
  let count=u32::from_le_bytes(tail[8..12].try_into().ok()?)as usize;let mut previous=None;
  for n in 0..count{let at=12+n*12;let handle=u32::from_le_bytes(tail[at..at+4].try_into().ok()?);let next=f32::from_le_bytes(tail[at+4..at+8].try_into().ok()?);let flags=u32::from_le_bytes(tail[at+8..at+12].try_into().ok()?);
   let i=self.get(handle)?;if previous.map_or(false,|p|i<=p)||!next.is_finite()||next<0.0||flags&!0x87!=0||flags&INIT as u32==0{return None;}
   if flags&STARTED as u32!=0&&flags&IN_ATTACK as u32==0{return None;}
   if !crate::ee2_up15_attack::ROWS.iter().any(|a|a.id==self.ents[i].def&&a.present){return None;}
   previous=Some(i);self.attack_clock[i]=Clock{handle,next,flags:flags as u8};
  }Some(())
 }
}
