//! Distinct outer envelope. The complete inner7/8 bytes, including all
//! ATK/PQP/CIV/registry adjacency, are never rearranged or heuristically parsed.
use crate::{game::*,pack8::{State,EMPTY,PACK,UNPACK,UNPACKED,UNPACKED_IDLE},pack8_defs};
pub const MAGIC:u32=0x31574b50; // PKW1
pub const END:u32=0x454b4350; // PCKE
pub const HEADER:usize=64;pub const RECORD:usize=48;
fn u(b:&[u8],at:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(at..at+4)?.try_into().ok()?))}
fn put(b:&mut Vec<u8>,v:u32){b.extend_from_slice(&v.to_le_bytes());}
fn checksum(b:&[u8])->u32{b.iter().fold(2166136261u32,|a,&v|(a^v as u32).wrapping_mul(16777619))}
pub fn split(buf:&[u8])->Option<(&[u8],Option<&[u8]>)>{
 if u(buf,0)!=Some(MAGIC){return Some((buf,None));}
 if buf.len()<HEADER+36+12||u(buf,56)!=Some(RECORD as u32)||u(buf,60)!=Some(HEADER as u32)||buf.get(24..56)?!=pack8_defs::SOURCE_HASH{return None;}
 let version=u(buf,4)?;if version!=1&&!(cfg!(all(feature="ee2-pack8",feature="ee2-civ8"))&&(version==2||version==3||(cfg!(feature="ee2-citizen9")&&version==4))){return None;}
 let core=u(buf,8)?as usize;let count=u(buf,12)?as usize;if core<36||core>64*1024*1024||count>MAXE||u(buf,20)?as usize!=crate::defs::DEFS.len(){return None;}
 let records=HEADER.checked_add(core)?;let base_end=records.checked_add(count.checked_mul(RECORD)?)?.checked_add((MAXE-count).checked_mul(4)?)?;
 let mut end=base_end;
 #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
 if version>=2{end=end.checked_add(crate::navigation8::framed_len(buf.get(end..)?)?)?;}
 #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
 if version>=3{end=end.checked_add(crate::projectiles8::framed_len(buf.get(end..)?)?)?;}
 #[cfg(feature="ee2-citizen9")]
 if version==4{end=end.checked_add(crate::citizen9::framed_len(buf.get(end..)?)?)?;}
 if end.checked_add(12)?!=buf.len()||u(buf,end)?as usize!=buf.len()||u(buf,end+4)?!=checksum(&buf[..end])||u(buf,end+8)!=Some(END){return None;}
 let inner=buf.get(HEADER..records)?;if u(inner,0)!=u(buf,16)||u(inner,0)==Some(MAGIC){return None;}
 #[cfg(feature="ee2-citizen9")] if (version==4)!=(u(inner,0)==Some(crate::citizen9::INNER_MAGIC)){return None;}
 Some((inner,Some(buf.get(records..end)?)))
}
fn words(s:State)->[u32;12]{[s.handle,s.flags as u32,s.phase as u32,s.started as u32,s.history_unknown as u32,s.start.to_bits(),s.duration.to_bits(),s.ai_until.to_bits(),s.last_goal.to_bits(),s.visual as u32,s.visual_start.to_bits(),s.visual_duration.to_bits()]}
impl Game{
 pub(crate) fn canonical_pack(&self,index:usize)->State{let state=self.pack_states[index];if state.handle==self.h(index){state}else{State::fresh(self.h(index))}}
 pub(crate) fn wrap_pack_save(&self,inner:&[u8])->Vec<u8>{
  #[cfg(feature="ee2-citizen9")] if !self.validate_source_birth(){return Vec::new();}
  let count=self.ents[..self.high].iter().filter(|e|e.alive).count();let mut b=Vec::with_capacity(HEADER+inner.len()+count*RECORD+self.free.len()*4+12);
  let version=if cfg!(feature="ee2-citizen9"){4}else if cfg!(all(feature="ee2-pack8",feature="ee2-civ8")){3}else{1};
  for v in[MAGIC,version,inner.len()as u32,count as u32,u(inner,0).unwrap(),crate::defs::DEFS.len()as u32]{put(&mut b,v);}b.extend_from_slice(&pack8_defs::SOURCE_HASH);put(&mut b,RECORD as u32);put(&mut b,HEADER as u32);b.extend_from_slice(inner);
  for i in 0..self.high{if self.ents[i].alive{for v in words(self.canonical_pack(i)){put(&mut b,v);}}}
  // The old loader reconstructs a sorted free list. Retain actual allocation
  // order here so the next real generation handle survives a modern load.
  // Dead Ent DTOs are intentionally zero in the inner codec, so retain their
  // next generation as well as the free-list order.
  for &slot in &self.free{put(&mut b,self.h(slot as usize));}
  #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
  self.append_navigation(&mut b);
  #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
  self.append_projectiles(&mut b);
  #[cfg(feature="ee2-citizen9")] if !self.append_source_birth(&mut b){return Vec::new();}
  let sum=checksum(&b);let total=(b.len()+12)as u32;put(&mut b,total);put(&mut b,sum);put(&mut b,END);b
 }
 pub(crate) fn read_pack_save(&mut self,records:Option<&[u8]>)->Option<()>{
  self.pack_states.fill(EMPTY);let now=self.pack_now();
  let Some(records)=records else{for i in 0..self.high{if self.ents[i].alive{let mut state=State::fresh(self.h(i));state.history_unknown=true;state.last_goal=now;self.pack_states[i]=state;}}
   #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]self.read_navigation(None)?;
   #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]self.read_projectiles(None)?;
   #[cfg(feature="ee2-citizen9")]self.read_source_birth(None)?;
   return Some(());};
  let count=self.ents[..self.high].iter().filter(|e|e.alive).count();let bytes=count.checked_mul(RECORD)?;
  let base=bytes.checked_add((MAXE-count)*4)?;if records.len()<base{return None;}
  #[cfg(not(all(feature="ee2-pack8",feature="ee2-civ8")))]if records.len()!=base{return None;}
  #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]let(navigation,projectiles,birth)=if records.len()>base{
   let tail=&records[base..];let n=crate::navigation8::framed_len(tail)?;let rest=&tail[n..];
   if rest.is_empty(){(Some(&tail[..n]),None,None)}else{
    let p=crate::projectiles8::framed_len(rest)?;let last=&rest[p..];
    if !cfg!(feature="ee2-citizen9")&&!last.is_empty(){return None;}
    (Some(&tail[..n]),Some(&rest[..p]),if last.is_empty(){None}else{Some(last)})
   }
  }else{(None,None,None)};
  let (records,allocator)=records[..base].split_at(bytes);let mut seen=vec![false;MAXE];let mut free=Vec::with_capacity(MAXE-count);
  for b in allocator.chunks_exact(4){let handle=u32::from_le_bytes(b.try_into().ok()?);let index=(handle&65535)as usize;if index>=MAXE||seen[index]||self.ents[index].alive||(index<self.high&&self.h(index)!=handle){return None;}seen[index]=true;self.ents[index].gen=(handle>>16)as u16;free.push(index as u16);}
  if seen.iter().enumerate().any(|(i,&v)|v==self.ents[i].alive){return None;}
  let mut previous=None;
  for b in records.chunks_exact(RECORD){
   let w:Vec<u32>=(0..12).map(|k|u(b,k*4).unwrap()).collect();let index=self.get(w[0])?;
   if previous.is_some_and(|p|index<=p)||w[1]&!10!=0||w[2]>2||w[3]>1||w[4]>1||w[9]>3{return None;}previous=Some(index);
   let f=|k|f32::from_bits(w[k]);let state=State{handle:w[0],flags:w[1]as u8,phase:w[2]as u8,started:w[3]!=0,history_unknown:w[4]!=0,start:f(5),duration:f(6),ai_until:f(7),last_goal:f(8),visual:w[9]as u8,visual_start:f(10),visual_duration:f(11)};
   if [state.start,state.duration,state.ai_until,state.last_goal,state.visual_start,state.visual_duration].iter().any(|v|!v.is_finite()||v.is_sign_negative())||state.last_goal>now||state.visual_start>now||state.start>now{return None;}
   let def=self.ents[index].def;let valid_duration=|phase:u8,value:f32|pack8_defs::ROWS.iter().any(|r|r.id==def&&r.capable&&value==if phase==PACK{r.pack}else{r.unpack});
   if state.phase==crate::pack8::NONE{if state.started||state.start!=0.0||state.duration!=0.0{return None;}}
   else if !valid_duration(state.phase,state.duration)||(state.phase==PACK&&state.flags&UNPACKED!=0)||(state.phase==UNPACK&&state.flags&UNPACKED==0)||(!state.started&&state.start!=0.0){return None;}
   if state.visual==PACK||state.visual==UNPACK{if !valid_duration(state.visual,state.visual_duration){return None;}}
   else if state.visual_duration!=0.0{return None;}
   if state.visual==UNPACKED_IDLE&&state.flags&UNPACKED==0{return None;}
   if state.started&&(state.visual!=state.phase||state.visual_duration!=state.duration){return None;}
   if !pack8_defs::ROWS.iter().any(|r|r.id==def&&r.capable)&&(state.flags!=0||state.phase!=0||state.ai_until!=0.0||state.visual!=0){return None;}
   // Delays may outlive an interrupted goal; they never need an unbounded
   // forged future wait. Source captured pack/unpack times remain class-bound.
   let maximum=pack8_defs::ROWS.iter().filter(|r|r.id==def).fold(0.0f32,|m,r|m.max(r.pack).max(r.unpack));
   if state.ai_until as f64>now as f64+maximum as f64+DT as f64{return None;}
   self.pack_states[index]=state;
  }self.free=free;
  #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]self.read_navigation(navigation)?;
  #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]self.read_projectiles(projectiles)?;
  #[cfg(feature="ee2-citizen9")]self.read_source_birth(birth)?;
  Some(())
 }
 pub(crate) fn pack_hash_words(&self)->Vec<u32>{let mut out=vec![MAGIC];for i in 0..self.high{if self.ents[i].alive{out.extend_from_slice(&words(self.canonical_pack(i)));}}out.push(self.free.len()as u32);out.extend(self.free.iter().map(|&v|self.h(v as usize)));out}
}
