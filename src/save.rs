//! Save games.
//!
//! The map is regenerated from its seed, then everything that can change is
//! written on top: players, entities, harvested tiles and explored fog.
//! Projectiles and explosions in flight are dropped, which nobody will miss.

use crate::defs::*;
use crate::game::*;
use crate::map::*;
use core::mem::size_of;

const MAGIC: u32 = 0x3250_454f; // "OEP2"

/// View a slice of plain `#[repr(C)]` values as bytes.
fn raw<T: Copy>(s: &[T]) -> &[u8] {
    // SAFETY: only used with Ent / Player / u16, which are plain old data; the
    // bytes are written out and read back by the same build (sizes are checked).
    unsafe { core::slice::from_raw_parts(s.as_ptr() as *const u8, core::mem::size_of_val(s)) }
}

fn fill<T: Copy>(dst: &mut [T], src: &[u8]) {
    assert_eq!(core::mem::size_of_val(dst), src.len());
    // SAFETY: as above; `src` holds exactly the bytes `raw` produced for this type.
    unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr() as *mut u8, src.len()) }
}

impl Game {
    pub fn save(&mut self) {
        let mut b = std::mem::take(&mut self.savebuf);
        b.clear();
        let head = [MAGIC, size_of::<Ent>() as u32, size_of::<Player>() as u32, DEFS.len() as u32, self.seed0, self.n_ai, self.tick, self.rng.0, self.high as u32];
        for v in head {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(raw(&self.players));
        b.extend_from_slice(raw(&self.ents[..self.high]));
        b.extend_from_slice(&self.map.ter);
        b.extend_from_slice(&self.map.res);
        b.extend_from_slice(raw(&self.map.amt));
        b.extend_from_slice(&self.vis);
        self.savebuf = b;
    }

    pub fn load(buf: &[u8]) -> Option<Game> {
        const H: usize = 9 * 4;
        if buf.len() < H {
            return None;
        }
        let u = |k: usize| u32::from_le_bytes([buf[k * 4], buf[k * 4 + 1], buf[k * 4 + 2], buf[k * 4 + 3]]);
        let high = u(8) as usize;
        let (psz, esz) = (size_of::<[Player; 4]>(), size_of::<Ent>() * high);
        // a save from another build has a different layout: refuse it rather than misread it
        if u(0) != MAGIC || u(1) as usize != size_of::<Ent>() || u(2) as usize != size_of::<Player>() || u(3) as usize != DEFS.len() || high > MAXE || buf.len() != H + psz + esz + NN * 5 {
            return None;
        }
        let mut g = Game::new(u(4), u(5), 0, 1, 0);
        g.tick = u(6);
        g.rng = Rng(u(7));
        let mut o = H;
        let mut take = |n: usize| {
            o += n;
            &buf[o - n..o]
        };
        fill(&mut g.players, take(psz));
        // wipe what `new` spawned, then drop the saved world in
        for i in 0..NN {
            g.map.blocked[i] = false;
        }
        for e in g.ents.iter_mut() {
            e.alive = false;
        }
        fill(&mut g.ents[..high], take(esz));
        g.map.ter.copy_from_slice(take(NN));
        g.map.res.copy_from_slice(take(NN));
        fill(&mut g.map.amt, take(NN * 2));
        g.vis.copy_from_slice(take(NN));
        g.high = high;
        g.free = (0..MAXE).rev().filter(|&i| !g.ents[i].alive).map(|i| i as u16).collect();
        for i in 0..high {
            g.ents[i].fslot = u16::MAX; // flow-field cache starts empty
            if g.ents[i].alive && DEFS[g.ents[i].def as usize].building {
                let (x, y, s) = (g.ents[i].x, g.ents[i].y, DEFS[g.ents[i].def as usize].size);
                let (tx0, ty0) = ((x - s * 0.5).round() as usize, (y - s * 0.5).round() as usize);
                for ty in ty0..ty0 + s as usize {
                    for tx in tx0..tx0 + s as usize {
                        g.map.blocked[ty * N + tx] = true;
                    }
                }
            }
        }
        for i in 0..NN {
            g.map.upd_pass(i);
            g.map.recolor(i);
        }
        g.map.nav_ver += 1;
        g.build_grid();
        g.terr_dirty = true;
        Some(g)
    }
}
