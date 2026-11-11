//! Additive joint8 fingerprint of future-read own simulator state. Keep local
//! selection, interpolation, outcome latch, FX and transport outside this hash.
//! NAV/PRJ/Pack ledgers retain their own complete ordered fingerprints.
use crate::{game::*,defs::*};
impl Game{
 pub(crate) fn hash_decisions8(&self,mut mix:impl FnMut(u32)){
  mix(0x33485348); // HSH3, canonical gameplay-field contract
  for v in[self.seed0,self.n_ai,self.humans,self.nplayers as u32,self.high as u32]{mix(v);}
  for p in &self.players[..self.nplayers]{
   for r in p.res{mix(r.to_bits());}
   for v in[p.pop,p.cap,p.population_limit,p.epoch as u32,p.researching as u32,p.alive as u32,p.ai as u32,p.ai_next_attack,p.ai_nodock as u32,p.ai_noexp as u32,p.buildings,p.tech,p.tech_busy]{mix(v);}
   for v in p.atk{mix(v.to_bits());}for v in p.dfn{mix(v.to_bits());}mix(p.gather.to_bits());mix(p.terr);
  }
  for i in 0..self.high{let e=&self.ents[i];if !e.alive{continue;}
   mix(self.h(i));mix(e.def as u32);mix(e.team as u32);
   for v in[e.x,e.y,e.vx,e.vy,e.rot,e.hp,e.cd,e.ox,e.oy,e.built,e.qt,e.rx,e.ry,e.reveal_t,e.carry]{mix(v.to_bits());}
   for v in[e.order as u32,e.otgt,e.otile,e.tgt,e.fkey,e.qn as u32,e.revealed as u32,e.stuck as u32,e.prev as u32,e.ncargo as u32]{mix(v);}
   if DEFS[e.def as usize].building{for &q in &e.q[..e.qn as usize]{mix(q);}}
   else if DEFS[e.def as usize].has(F_WORKER){for &q in &e.q[..3]{mix(q);}}
   for k in 0..e.ncargo as usize{mix(e.cargo[k]as u32);mix(e.cargo_hp[k]as u32);}
  }
  for bytes in[&self.map.ter[..],&self.map.res[..],&self.map.terr[..],&self.towner[..self.map.region_storage_len()]]{
   mix(bytes.len()as u32);for b in bytes.chunks_exact(4){mix(u32::from_le_bytes(b.try_into().unwrap()));}
  }
  mix(self.map.amt.len()as u32);for v in self.map.amt.chunks_exact(2){mix(v[0]as u32|(v[1]as u32)<<16);}
 }
}
