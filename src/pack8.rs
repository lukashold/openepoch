//! Source Pack/Unpack transition layer. The host's existing order/navigation
//! system remains its own adapter, not an invented native generic UnitAI.
use crate::{game::*,pack8_defs};
pub const UNPACKED:u8=2;pub const MANUAL:u8=8;
pub const NONE:u8=0;pub const PACK:u8=1;pub const UNPACK:u8=2;pub const UNPACKED_IDLE:u8=3;
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct State{
 pub handle:u32,pub flags:u8,pub phase:u8,pub started:bool,pub history_unknown:bool,
 pub start:f32,pub duration:f32,pub ai_until:f32,pub last_goal:f32,
 pub visual:u8,pub visual_start:f32,pub visual_duration:f32,pub visual_revision:u32,
}
pub const EMPTY:State=State{handle:u32::MAX,flags:0,phase:0,started:false,history_unknown:false,start:0.0,duration:0.0,ai_until:0.0,last_goal:0.0,visual:0,visual_start:0.0,visual_duration:0.0,visual_revision:0};
impl State{
 pub fn fresh(handle:u32)->Self{Self{handle,..EMPTY}}

 // The animator refuses the outgoing blend if another visual request still
 // needs a scene update. Count distinct requests, including those between draws.
 fn set_visual(&mut self,visual:u8,duration:f32,now:f32){
  if (self.visual,self.visual_duration)==(visual,duration){return;}
  if visual!=NONE{self.visual_start=now;}
  self.visual=visual;
  self.visual_duration=duration;
  self.visual_revision=self.visual_revision.wrapping_add(1);
 }
}
pub fn interrupt(state:&mut State){state.phase=NONE;state.started=false;state.start=0.0;state.duration=0.0;}

impl Game{
 pub(crate) fn pack_now(&self)->f32{self.sim_elapsed()}
 pub(crate) fn pack_reset(&mut self,index:usize){self.pack_states[index]=State::fresh(self.h(index));}
 pub(crate) fn pack_spec(&self,e:&Ent)->Option<pack8_defs::Spec>{pack8_defs::lookup(e.def,self.players.get(e.team as usize)?.epoch)}
 fn pack_sync(&mut self,index:usize,e:&Ent){let handle=(e.gen as u32)<<16|index as u32;if self.pack_states[index].handle!=handle{self.pack_states[index]=State::fresh(handle);}}
 ///00840220 changes logical flags immediately. Acceptance is supplied by the
 ///actual host order lifecycle; successful own allocation is the normal domain.
 pub(crate) fn pack_enqueue(&mut self,index:usize,e:&Ent,unpacked:bool,accepted:bool)->bool{
  self.pack_sync(index,e);let spec=self.pack_spec(e);let capable=spec.is_some_and(|s|s.capable);let now=self.pack_now();let state=&mut self.pack_states[index];
  let required=capable&&((state.flags&UNPACKED!=0)!=unpacked);
  if required&&accepted{let spec=spec.unwrap();state.phase=if unpacked{UNPACK}else{PACK};state.started=false;state.start=0.0;state.duration=if unpacked{spec.unpack}else{spec.pack};state.ai_until=((now as f64)+(state.duration as f64))as f32;}
  if unpacked{if capable{state.flags|=UNPACKED;}}else{state.flags&=!UNPACKED;}
  required
 }
 ///00815FF0: activation/resume stores clock only while the old stamp==0.
 pub(crate) fn pack_activate(&mut self,index:usize){let now=self.pack_now();let state=&mut self.pack_states[index];if state.phase==NONE{return;}if state.start==0.0{state.start=now;}state.started=true;
  //008754B0: identical packets preserve their visual start stamp.
  state.set_visual(state.phase,state.duration,now);
 }
 pub(crate) fn pack_tick_actor(&mut self,index:usize,e:&Ent){
  self.pack_sync(index,e);if self.pack_states[index].phase!=NONE&&!self.pack_states[index].started{self.pack_activate(index);}
  let now=self.pack_now();let state=&mut self.pack_states[index];
  if state.phase!=NONE&&state.started&&(now as f64)>(state.start as f64)+(state.duration as f64){state.phase=NONE;state.started=false;state.start=0.0;state.duration=0.0;}
 }
 pub(crate) fn pack_busy(&self,index:usize)->bool{let state=self.pack_states[index];state.phase!=NONE||self.pack_now()<state.ai_until}
 ///A real new accepted order may replace its visual goal. Native logical flags
 ///and the separate AI/attack deadlines are not rolled back by replacement.
 pub(crate) fn pack_interrupt(&mut self,index:usize){interrupt(&mut self.pack_states[index]);}
 pub(crate) fn order_pack(&mut self,indices:&[usize],unpacked:bool){
  for &index in indices{let mut e=self.ents[index];if !e.alive||!self.pack_spec(&e).is_some_and(|s|s.capable){continue;}
   self.pack_interrupt(index);Self::set_idle(&mut e);self.native_stop_attack(index);e.tgt=crate::game::NONE;
   self.pack_enqueue(index,&e,unpacked,true);
   if unpacked{self.pack_states[index].flags|=MANUAL;}else{self.pack_states[index].flags&=!MANUAL;}
   self.pack_activate(index);self.pack_states[index].last_goal=self.pack_now();self.ents[index]=e;
  }
 }
 ///0080A950's explicit move-command mode1 requests manual Pack before
 ///appending movement whenever bit3 is set. C_MOVE/C_AMOVE are this host's
 ///declared direct-move adapter; AI waypoint steering still uses007F2810.
 pub(crate) fn pack_direct_move(&mut self,indices:&[usize]){
  for &index in indices{let e=self.ents[index];if !e.alive||self.pack_states[index].flags&MANUAL==0{continue;}
   let required=self.pack_enqueue(index,&e,false,true);self.pack_states[index].flags&=!MANUAL;if required{self.pack_activate(index);}
  }
 }
 pub(crate) fn pack_wait_for_attack(&mut self,index:usize,e:&mut Ent)->bool{
  self.pack_sync(index,e);if self.pack_busy(index){return true;}
  let required=self.pack_enqueue(index,e,true,true);
  if required{self.pack_activate(index);let duration=self.pack_states[index].duration;let now=self.pack_now();let clock=&mut self.attack_clock[index];let handle=(e.gen as u32)<<16|index as u32;
   if clock.handle!=handle{*clock=crate::native_attack_clock::Clock{handle,next:0.0,flags:0};}
   clock.flags=(clock.flags|crate::native_attack_clock::INIT)&!crate::native_attack_clock::STARTED;clock.next=((now as f64)+(duration as f64))as f32;e.cd=(clock.next-now).max(0.0);return true;
  }false
 }
 pub(crate) fn pack_wait_for_move(&mut self,index:usize,e:&Ent,dx:f32,dy:f32)->bool{
  self.pack_sync(index,e);if self.pack_busy(index){return true;}
  if self.pack_states[index].flags&UNPACKED!=0&&dx*dx+dy*dy>0.0{if self.pack_enqueue(index,e,false,true){self.pack_activate(index);}return true;}false
 }
 pub(crate) fn pack_finish_actor(&mut self,index:usize,e:&Ent,active_goal:bool,moving:bool){
  let now=self.pack_now();let spec=self.pack_spec(e);let state=&mut self.pack_states[index];
  if active_goal||state.phase!=NONE{state.last_goal=now;}
  if state.phase!=NONE{return;}
  if active_goal||moving{state.set_visual(NONE,0.0,now);return;}
  if now<state.ai_until{return;}
  if state.flags&UNPACKED==0{state.set_visual(NONE,0.0,now);return;}
  if spec.is_some_and(|s|s.unpacked_idle){state.set_visual(UNPACKED_IDLE,0.0,now);return;}
  if state.flags&MANUAL==0&&(now as f64)-1.0>(state.last_goal as f64){if self.pack_enqueue(index,e,false,true){self.pack_activate(index);}}
 }
}
