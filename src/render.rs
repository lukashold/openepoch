//! Everything the browser needs to draw a frame, written into flat buffers
//! that JavaScript reads straight out of WASM memory (no per-object calls).
//!
//! Instance layout (8 floats): x, y, size, rotation, sprite, team, param, flags.

use crate::defs::*;
use crate::game::*;
use crate::map::*;
use std::fmt::Write;

pub const S_TREE: f32 = 100.0;
pub const S_ORE: f32 = 102.0;
pub const S_BERRY: f32 = 103.0;
pub const S_PROJ: f32 = 103.0; // + weapon kind (1..=6)
pub const S_FLAG: f32 = 110.0;
pub const S_RING: f32 = 240.0;
pub const S_BAR: f32 = 241.0;
pub const S_BOOM: f32 = 242.0;

pub const FL_SHADOW: f32 = 1.0;
pub const FL_SUBMERGED: f32 = 2.0;
pub const FL_FLASH: f32 = 4.0;
pub const FL_GHOST_OK: f32 = 8.0;
pub const FL_GHOST_BAD: f32 = 16.0;
pub const FL_NOFOG: f32 = 32.0;
pub const FL_BUILDING: f32 = 64.0;

const MAX_INST: usize = 60_000;

#[inline]
fn put(o: &mut Vec<f32>, x: f32, y: f32, size: f32, rot: f32, sprite: f32, team: f32, param: f32, flags: f32) {
    if o.len() < MAX_INST * 8 {
        o.extend_from_slice(&[x, y, size, rot, sprite, team, param, flags]);
    }
}

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, alpha: f32, x0: f32, y0: f32, x1: f32, y1: f32, ghost: i32, gx: i32, gy: i32) -> u32 {
        let mut o = std::mem::take(&mut self.inst);
        o.clear();
        let far = x1 - x0 > 110.0;

        // ---- resource tiles
        let (tx0, ty0) = ((x0 - 1.0).max(0.0) as usize, (y0 - 1.0).max(0.0) as usize);
        let (tx1, ty1) = (((x1 + 2.0) as usize).min(N), ((y1 + 2.0) as usize).min(N));
        for ty in ty0..ty1 {
            for tx in tx0..tx1 {
                let i = ty * N + tx;
                let r = self.map.res[i];
                if r == R_NONE || self.vis[i] == 0 {
                    continue;
                }
                let h = hash(tx as i32, ty as i32, 5);
                let jx = (h & 255) as f32 / 255.0 - 0.5;
                let jy = ((h >> 8) & 255) as f32 / 255.0 - 0.5;
                let (cx, cy) = (tx as f32 + 0.5, ty as f32 + 0.5);
                match r {
                    R_WOOD => put(&mut o, cx + jx * 0.4, cy + jy * 0.4, 1.5 + jx * 0.5, (h >> 16) as f32 * 0.001, S_TREE + ((h >> 20) & 1) as f32, 0.0, 0.0, 0.0),
                    R_METAL => put(&mut o, cx, cy, 1.25, 0.0, S_ORE, 0.0, 0.0, 0.0),
                    _ => put(&mut o, cx + jx * 0.2, cy + jy * 0.2, 1.1, 0.0, S_BERRY, 0.0, 0.0, 0.0),
                }
            }
        }

        let (vx0, vy0, vx1, vy1) = (x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0);
        let inview = |x: f32, y: f32| x > vx0 && x < vx1 && y > vy0 && y < vy1;

        // ---- pass 0: buildings, pass 1: surface + submerged units, pass 2: aircraft
        for pass in 0..3 {
            if pass == 2 {
                // projectiles and explosions sit between ground and air
                for p in &self.projs {
                    let (x, y) = (p.px + (p.x - p.px) * alpha, p.py + (p.y - p.py) * alpha);
                    if !inview(x, y) || self.vis[tile_of(x, y)] != 255 {
                        continue;
                    }
                    let size = [0.0, 0.7, 0.45, 0.5, 1.0, 1.0, 0.7][p.kind as usize];
                    put(&mut o, x, y, size, (p.ty - p.y).atan2(p.tx - p.x), S_PROJ + p.kind as f32, p.team as f32, 0.0, 0.0);
                }
                for f in &self.fx {
                    if inview(f.x, f.y) && self.vis[tile_of(f.x, f.y)] == 255 {
                        put(&mut o, f.x, f.y, f.size, 0.0, S_BOOM, 0.0, f.age / f.life, FL_NOFOG);
                    }
                }
            }
            for i in 0..self.high {
                let e = &self.ents[i];
                if !e.alive {
                    continue;
                }
                let d = &DEFS[e.def as usize];
                let layer = if d.building { 0 } else if d.dom == AIR { 2 } else { 1 };
                if layer != pass || !inview(e.x, e.y) || !self.visible(i) {
                    continue;
                }
                let x = e.px + (e.x - e.px) * alpha;
                let y = e.py + (e.y - e.py) * alpha;
                let rot = e.prot + angdiff(e.rot, e.prot) * alpha;
                let team = e.team as f32;
                let mut fl = if e.flash > 0.0 { FL_FLASH } else { 0.0 };
                if d.building {
                    if e.sel {
                        put(&mut o, x, y, d.size * 1.5, 0.0, S_RING, team, 0.0, FL_NOFOG);
                        if e.rx >= 0.0 {
                            put(&mut o, e.rx, e.ry, 1.3, 0.0, S_FLAG, team, 0.0, FL_NOFOG);
                        }
                    }
                    put(&mut o, x, y, d.size, 0.0, e.def as f32, team, e.built, fl + FL_BUILDING);
                } else {
                    if e.sel {
                        put(&mut o, x, y, d.size * 1.25, 0.0, S_RING, team, 0.0, FL_NOFOG);
                    }
                    if d.dom == AIR {
                        put(&mut o, x + 0.5, y + 0.9, d.size * 1.2, rot, e.def as f32, team, 0.0, FL_SHADOW);
                    }
                    if d.dom == SUB {
                        fl += FL_SUBMERGED;
                    }
                    // people are drawn larger than their collision size so they stay readable
                    let k = match d.armor {
                        A_INF => 1.9,
                        A_CAV => 1.6,
                        _ => 1.3,
                    };
                    put(&mut o, x, y, d.size * k, rot, e.def as f32, team, 0.0, fl);
                }
                if e.sel || (e.hp < d.hp && !far) {
                    let w = (d.rad() * 2.0).clamp(0.8, 3.0);
                    put(&mut o, x, y - d.rad() - 0.45, w, 0.0, S_BAR, team, (e.hp / d.hp).clamp(0.0, 1.0), FL_NOFOG);
                }
                if d.building && e.qn > 0 && e.team == 0 && !far {
                    let t = if e.q[0] == RESEARCH { EPOCH_TIME } else { DEFS[e.q[0] as usize].time };
                    put(&mut o, x, y + d.rad() + 0.3, d.size * 0.8, 0.0, S_BAR, 7.0, (e.qt / t).clamp(0.0, 1.0), FL_NOFOG);
                }
            }
        }

        if ghost >= 0 && (ghost as usize) < DEFS.len() {
            let d = &DEFS[ghost as usize];
            let ok = self.can_place(ghost as u8, gx, gy, 0) && self.afford(0, &d.cost);
            put(&mut o, gx as f32 + d.size * 0.5, gy as f32 + d.size * 0.5, d.size, 0.0, ghost as f32, 0.0, 1.0, FL_NOFOG + if ok { FL_GHOST_OK } else { FL_GHOST_BAD });
        }

        let n = (o.len() / 8) as u32;
        self.inst = o;
        n
    }

    pub fn minimap(&mut self) {
        const TEAM: [[u8; 3]; 4] = [[70, 150, 255], [240, 70, 60], [80, 220, 100], [245, 210, 70]];
        for i in 0..NN {
            let f = match self.vis[i] {
                0 => 0u32,
                255 => 256,
                _ => 130,
            };
            let o = i * 4;
            let (mut r, mut g, mut b) = (self.map.rgba[o] as u32, self.map.rgba[o + 1] as u32, self.map.rgba[o + 2] as u32);
            if self.map.res[i] == R_METAL {
                (r, g, b) = (150, 150, 165);
            }
            self.mini[o] = (r * f >> 8) as u8;
            self.mini[o + 1] = (g * f >> 8) as u8;
            self.mini[o + 2] = (b * f >> 8) as u8;
            self.mini[o + 3] = 255;
        }
        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || !self.visible(i) {
                continue;
            }
            let d = &DEFS[e.def as usize];
            let c = TEAM[e.team as usize & 3];
            let s = if d.building { d.size as i32 } else { 2 };
            let (x0, y0) = ((e.x - s as f32 * 0.5) as i32, (e.y - s as f32 * 0.5) as i32);
            for y in y0.max(0)..(y0 + s).min(N as i32) {
                for x in x0.max(0)..(x0 + s).min(N as i32) {
                    let o = (y as usize * N + x as usize) * 4;
                    self.mini[o] = c[0];
                    self.mini[o + 1] = c[1];
                    self.mini[o + 2] = c[2];
                }
            }
        }
    }

    /// HUD state as a small JSON document; the page polls it a few times a second.
    pub fn build_ui(&mut self) {
        let mut s = std::mem::take(&mut self.ui);
        s.clear();
        let p = self.players[0];
        let ep = p.epoch;
        let _ = write!(
            s,
            "{{\"res\":[{},{},{}],\"pop\":{},\"cap\":{},\"epoch\":{},\"ename\":\"{}\",\"win\":{},\"n\":{},\"tick\":{},",
            p.res[0] as i32, p.res[1] as i32, p.res[2] as i32, p.pop, p.cap, ep, EPOCH_NAMES[ep as usize], self.winner, self.sim_units, self.tick
        );
        // selection summary
        let mut first = NOIDX;
        let mut n = 0;
        let mut has_worker = false;
        let mut types: Vec<(u8, u32)> = Vec::new();
        let mut idle_workers = 0;
        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || e.team != 0 {
                continue;
            }
            if e.def == WORKER && e.order == O_IDLE {
                idle_workers += 1;
            }
            if !e.sel {
                continue;
            }
            if first == NOIDX {
                first = i;
            }
            n += 1;
            has_worker |= e.def == WORKER;
            match types.iter_mut().find(|t| t.0 == e.def) {
                Some(t) => t.1 += 1,
                None => types.push((e.def, 1)),
            }
        }
        let _ = write!(s, "\"idle\":{},\"sel\":{{\"n\":{}", idle_workers, n);
        if first != NOIDX {
            let e = self.ents[first];
            let d = &DEFS[e.def as usize];
            let _ = write!(s, ",\"name\":\"{}\",\"def\":{},\"hp\":{},\"max\":{},\"built\":{:.2},\"q\":[", d.name, e.def, e.hp.max(0.0) as i32, d.hp as i32, e.built);
            for k in 0..e.qn as usize {
                let _ = write!(s, "{}{}", if k > 0 { "," } else { "" }, e.q[k]);
            }
            let qt = if e.qn > 0 { e.qt / if e.q[0] == RESEARCH { EPOCH_TIME } else { DEFS[e.q[0] as usize].time } } else { 0.0 };
            let _ = write!(s, "],\"qp\":{:.2},\"types\":[", qt);
            for (k, t) in types.iter().take(10).enumerate() {
                let _ = write!(s, "{}[{},{},\"{}\"]", if k > 0 { "," } else { "" }, t.0, t.1, DEFS[t.0 as usize].name);
            }
            s.push(']');
            let w = &d.w[0];
            if d.armed() {
                let _ = write!(s, ",\"dmg\":{},\"rng\":{:.1}", w.dmg as i32, w.range);
            }
        }
        s.push_str("},\"act\":[");
        let mut k = 0;
        let mut act = |s: &mut String, kind: &str, id: u8, name: &str, cost: &[u16; 3], ok: bool, size: f32, tip: &str| {
            let _ = write!(s, "{}{{\"k\":\"{}\",\"id\":{},\"name\":\"{}\",\"cost\":[{},{},{}],\"ok\":{},\"size\":{},\"tip\":\"{}\"}}", if k > 0 { "," } else { "" }, kind, id, name, cost[0], cost[1], cost[2], ok, size, tip);
            k += 1;
        };
        let mut tip = String::new();
        if has_worker {
            for (id, d) in DEFS.iter().enumerate() {
                if d.building && d.epoch <= ep {
                    tip.clear();
                    let _ = write!(tip, "{} HP", d.hp as i32);
                    if id as u8 == HOUSE {
                        tip.push_str(" \u{b7} +10 population");
                    }
                    if id as u8 == FARM {
                        tip.push_str(" \u{b7} endless food, 4 workers");
                    }
                    if id as u8 == DOCK {
                        tip.push_str(" \u{b7} place on water next to the shore");
                    }
                    act(&mut s, "b", id as u8, d.name, &d.cost, self.afford(0, &d.cost), d.size, &tip);
                }
            }
        } else if first != NOIDX && DEFS[self.ents[first].def as usize].building && self.ents[first].built >= 1.0 {
            let bdef = self.ents[first].def;
            for (id, d) in DEFS.iter().enumerate() {
                if d.made_at == bdef && d.epoch <= ep && d.until > ep {
                    tip.clear();
                    let _ = write!(tip, "{} HP", d.hp as i32);
                    for w in &d.w {
                        if w.targets != 0 {
                            let _ = write!(tip, " \u{b7} {} dmg / {:.0} range vs", w.dmg as i32, w.range);
                            for (bit, nm) in [(LAND, " land"), (SEA, " sea"), (AIR, " air"), (SUB, " subs")] {
                                if w.targets & bit != 0 {
                                    tip.push_str(nm);
                                }
                            }
                        }
                    }
                    if d.has(F_STEALTH) {
                        tip.push_str(" \u{b7} stealth");
                    }
                    if d.has(F_DETECT) {
                        tip.push_str(" \u{b7} detector");
                    }
                    act(&mut s, "t", id as u8, d.name, &d.cost, self.afford(0, &d.cost) && p.pop + d.pop as u32 <= p.cap, 0.0, &tip);
                }
            }
            if bdef == TC && ep < MAX_EPOCH {
                let c = &EPOCH_COST[ep as usize + 1];
                tip.clear();
                let _ = write!(tip, "Advance to the {}", EPOCH_NAMES[ep as usize + 1]);
                act(&mut s, "e", RESEARCH, "Next Epoch", c, self.afford(0, c) && !p.researching, 0.0, &tip);
            }
        }
        s.push_str("]}");
        self.ui = s;
    }
}
