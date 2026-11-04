//! Own new-match transaction. SP and lockstep peers configure the same validated
//! inputs before commands. Existing single-player legacy exports stay gated.
use crate::{defs::*,game::*};
impl Game{
 pub(crate) fn configure_match_settings(&mut self,packed:u32,pop:u32,supplies:u32,epoch:u32)->bool{
  if packed&0xffff0000!=0||![250,500,1000].contains(&pop)||supplies>4||epoch>MAX_EPOCH as u32||self.tick!=0||self.bench!=0||!self.civ_setup_open||!self.outbox.is_empty()||!self.inbox.is_empty()||self.cheats_enabled||self.instant_build{return false;}
  let civs=core::array::from_fn(|i|((packed>>(4*i))&15)as u8);
  if self.civilizations.iter().any(|&c|c!=crate::civ8_runtime::LEGACY_NEUTRAL)||civs.iter().any(|&c|c>crate::civ8_runtime::LEGACY_NEUTRAL)||civs[self.nplayers..].iter().any(|&c|c!=crate::civ8_runtime::LEGACY_NEUTRAL)||self.ents[..self.high].iter().any(|e|e.alive&&((DEFS[e.def as usize].building&&e.qn>0)||e.order!=O_IDLE)){return false;}
  // Nothing above mutates; configure_civilizations performs its own complete
  // validation before commit. No fallible operation follows the commit.
  if !self.configure_civilizations(civs){return false;}
  let stocks=match supplies{0=>[400.0,400.0,200.0],1=>[800.0,800.0,400.0],2=>[15000.0,15000.0,7500.0],_=>[200.0,200.0,0.0]};
  for t in 0..self.nplayers{
   self.players[t].population_limit=pop;self.players[t].res=stocks;
   let common=match supplies{2=>10000.0,3|4=>100.0,_=>stocks[1]};let stone=if supplies==4{500.0}else{common};self.extra_resources[t]=[stone,common,stocks[2],stocks[2],stocks[2],stocks[2]];
   let previous=self.players[t].epoch;self.players[t].epoch=epoch as u8;self.rebuild_modifiers(t as u8,true);self.apply_paid_main(t as u8,previous);
  }
  self.build_grid();self.update_fog();true
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn invalid_inputs_are_atomic_and_leave_a_valid_retry_open(){
  let mut g=Game::new(7,0,0,2,0);g.net=true;g.save();let bytes=g.savebuf.clone();let hash=g.state_hash();
  for(c,p,r,e)in[(0x1ee10,500,2,5),(0xeef0,500,2,5),(0xee10,249,2,5),(0xee10,500,5,5),(0xee10,500,2,15),(0x0e10,500,2,5)]{assert!(!g.configure_match_settings(c,p,r,e));assert!(g.civ_setup_open);assert_eq!(hash,g.state_hash());g.save();assert!(g.savebuf==bytes);}
  assert!(g.configure_match_settings(0xee10,500,2,5));assert!(!g.configure_match_settings(0xee10,500,2,5));
 }
 #[test]fn net_peers_share_symmetric_epoch_stocks_population_and_civs(){
  let mut a=Game::new(7,0,0,2,0);let mut b=Game::new(7,0,0,2,1);a.net=true;b.net=true;
  for g in[&mut a,&mut b]{assert!(g.configure_match_settings(0xee10,500,2,5));assert_eq!(g.civilizations,[0,1,14,14]);for p in&g.players[..2]{assert_eq!(p.epoch,5);assert_eq!(p.population_limit,500);assert_eq!(p.res,[15000.0,15000.0,7500.0]);}assert_eq!(g.extra_resources[0],g.extra_resources[1]);}
  assert_eq!(a.state_hash(),b.state_hash());for _ in 0..80{a.tick();b.tick();assert_eq!(a.state_hash(),b.state_hash());}
 }
 #[test]fn commands_load_tick_and_benchmark_close_the_new_match_window(){
  let mut g=Game::new(7,0,0,2,0);g.net=true;g.submit(crate::cmd::C_STOP,0.0,0.0,0,&[]);assert!(!g.configure_match_settings(0xee10,500,2,5));
  let mut g=Game::new(7,0,0,2,0);g.tick();assert!(!g.configure_match_settings(0xee10,500,2,5));
  let mut g=Game::new(7,0,1,2,0);assert!(!g.configure_match_settings(0xee10,500,2,5));
  let mut g=Game::new(7,0,0,2,0);g.save();let mut l=Game::load(&g.savebuf).unwrap();assert!(!l.configure_match_settings(0xee10,500,2,5));
 }
 #[test]fn atomic_sp_matches_the_previous_real_export_sequence_byte_for_byte(){
  crate::init(7,0,0,1,0);assert_eq!(crate::configure_civilizations(0xeee0),1);assert_eq!(crate::skirmish_setup(500,2),1);assert_eq!(crate::skirmish_epoch(5),1);let p=crate::save();let bytes=unsafe{core::slice::from_raw_parts(p,crate::save_len()as usize).to_vec()};let hash=crate::state_hash();
  crate::init(7,0,0,1,0);assert_eq!(crate::configure_match_settings(0xeee0,500,2,5),1);assert_eq!(crate::state_hash(),hash);let p=crate::save();let actual=unsafe{core::slice::from_raw_parts(p,crate::save_len()as usize)};assert!(actual==bytes);
 }
}
