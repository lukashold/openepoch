//! Everything the browser needs to draw a frame, written into flat buffers
//! that JavaScript reads straight out of WASM memory (no per-object calls).
//!
//! Instance layout (12 floats): x, y, altitude, rotation | model, team, param, flags | size, 0, 0, 0.
//! Instances are bucket-sorted by model id so the page can issue one instanced
//! draw call per model; `ranges` holds (first, count) for each of the 256 ids.

use crate::defs::*;
use crate::game::*;
use crate::map::*;
use std::fmt::Write;

pub const S_TREE: f32 = 100.0;
pub const S_ORE: f32 = 102.0;
pub const S_BERRY: f32 = 103.0;
pub const S_PROJ: f32 = 103.0; // + weapon kind (1..=6)
pub const S_FLAG: f32 = 110.0;
// ids from 240 are flat or camera-facing quads rather than models
pub const S_RING: f32 = 240.0;
pub const S_BAR: f32 = 241.0;
pub const S_BOOM: f32 = 242.0;
pub const S_SHADOW: f32 = 243.0;

pub const FL_FLASH: f32 = 4.0;
pub const FL_GHOST_OK: f32 = 8.0;
pub const FL_GHOST_BAD: f32 = 16.0;
pub const FL_NOFOG: f32 = 32.0;
pub const FL_BUILDING: f32 = 64.0;

const MAX_INST: usize = 60_000;
pub const STRIDE: usize = 12;

#[allow(clippy::too_many_arguments)]
#[inline]
fn put(o: &mut Vec<f32>, x: f32, y: f32, alt: f32, rot: f32, model: f32, team: f32, param: f32, flags: f32, size: f32) {
    if o.len() < MAX_INST * STRIDE {
        o.extend_from_slice(&[x, y, alt, rot, model, team, param, flags, size, 0.0, 0.0, 0.0]);
    }
}

pub fn qtime(item: u8) -> f32 {
    if item == RESEARCH {
        EPOCH_TIME
    } else if item >= TECH0 {
        TECHS[(item - TECH0) as usize].time
    } else {
        DEFS[item as usize].time
    }
}

impl Game {
    /// Is every tile of this footprint explored by the local player?
    pub fn explored(&self, def: u8, tx: i32, ty: i32) -> bool {
        let s = DEFS[def as usize].size as i32;
        (ty..ty + s).all(|y| (tx..tx + s).all(|x| x >= 0 && y >= 0 && x < N as i32 && y < N as i32 && self.vis[y as usize * N + x as usize] > 0))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, alpha: f32, x0: f32, y0: f32, x1: f32, y1: f32, ghost: i32, gx: i32, gy: i32) -> u32 {
        let mut o = std::mem::take(&mut self.tmp);
        o.clear();
        let far = x1 - x0 > 130.0;
        let me = self.local;

        // ---- resource tiles
        let (tx0, ty0) = ((x0 - 1.0).max(0.0) as usize, (y0 - 1.0).max(0.0) as usize);
        let (tx1, ty1) = (((x1 + 2.0).max(0.0) as usize).min(N), ((y1 + 2.0).max(0.0) as usize).min(N));
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
                let (cx, cy) = (tx as f32 + 0.5 + jx * 0.4, ty as f32 + 0.5 + jy * 0.4);
                let alt = self.map.height(cx, cy).max(0.0);
                let rot = (h >> 16) as f32 * 0.001;
                match r {
                    R_WOOD => put(&mut o, cx, cy, alt, rot, S_TREE + ((h >> 20) & 1) as f32, 0.0, 0.0, 0.0, 1.0 + jx * 0.35),
                    R_METAL => put(&mut o, cx, cy, alt, rot, S_ORE, 0.0, 0.0, 0.0, 1.0),
                    _ => put(&mut o, cx, cy, alt, rot, S_BERRY, 0.0, 0.0, 0.0, 1.0),
                }
            }
        }

        let (vx0, vy0, vx1, vy1) = (x0 - 4.0, y0 - 4.0, x1 + 4.0, y1 + 4.0);
        let inview = |x: f32, y: f32| x > vx0 && x < vx1 && y > vy0 && y < vy1;

        for p in &self.projs {
            let (x, y) = (p.px + (p.x - p.px) * alpha, p.py + (p.y - p.py) * alpha);
            if !inview(x, y) || self.vis[tile_of(x, y)] != 255 {
                continue;
            }
            // 0 at launch, 1 at impact
            let t = (1.0 - ((p.tx - x).powi(2) + (p.ty - y).powi(2)).sqrt() / p.d0).clamp(0.0, 1.0);
            let ground = self.map.height(x, y).max(0.0);
            let alt = match p.kind {
                W_TORP => -0.12,
                W_BOMB => ground + 3.4 * (1.0 - t),
                W_SHELL => ground + 0.5 + (t * core::f32::consts::PI).sin() * (0.6 + p.d0 * 0.16),
                W_MISSILE => ground + 0.6 + (t * core::f32::consts::PI).sin() * (0.4 + p.d0 * 0.1),
                _ => ground + 0.55,
            };
            put(&mut o, x, y, alt, (p.ty - p.y).atan2(p.tx - p.x), S_PROJ + p.kind as f32, p.team as f32, 0.0, 0.0, 1.0);
        }
        for f in &self.fx {
            if inview(f.x, f.y) && self.vis[tile_of(f.x, f.y)] == 255 {
                put(&mut o, f.x, f.y, self.map.height(f.x, f.y).max(0.0) + 0.5, 0.0, S_BOOM, 0.0, f.age / f.life, FL_NOFOG, f.size * 1.3);
            }
        }

        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || !inview(e.x, e.y) || !self.visible(i) {
                continue;
            }
            let d = &DEFS[e.def as usize];
            let x = e.px + (e.x - e.px) * alpha;
            let y = e.py + (e.y - e.py) * alpha;
            let rot = e.prot + angdiff(e.rot, e.prot) * alpha;
            let team = e.team as f32;
            let fl = if e.flash > 0.0 { FL_FLASH } else { 0.0 };
            let ground = self.map.height(x, y).max(0.0);
            let (alt, top) = if d.building {
                let a = if e.def == DOCK { 0.0 } else { ground };
                if e.sel {
                    put(&mut o, x, y, a + 0.12, 0.0, S_RING, team, 0.0, FL_NOFOG, d.size * 1.5);
                    if e.rx >= 0.0 {
                        put(&mut o, e.rx, e.ry, self.map.height(e.rx, e.ry).max(0.0), 0.0, S_FLAG, team, 0.0, FL_NOFOG, 1.0);
                    }
                }
                put(&mut o, x, y, a, 0.0, e.def as f32, team, e.built, fl + FL_BUILDING, d.size);
                (a, d.size * 0.6 + 0.7)
            } else {
                let a = match d.dom {
                    SEA => 0.0,
                    SUB => -0.34,
                    AIR => 3.4 + ground * 0.6,
                    _ => ground,
                };
                if e.sel {
                    put(&mut o, x, y, a.max(0.0) + 0.1, 0.0, S_RING, team, 0.0, FL_NOFOG, d.size * 1.3);
                }
                if d.dom == AIR {
                    put(&mut o, x, y, ground + 0.08, 0.0, S_SHADOW, 0.0, 0.0, 0.0, d.size * 1.1);
                }
                // people are drawn larger than their collision size so they stay readable
                let k = match d.armor {
                    A_INF => 1.5,
                    A_CAV => 1.3,
                    _ => 1.15,
                };
                put(&mut o, x, y, a, rot, e.def as f32, team, 0.0, fl, d.size * k);
                (a.max(0.0), d.size * 0.5 + 0.6)
            };
            if e.sel || (e.hp < d.hp && !far) {
                let w = (d.rad() * 2.0).clamp(0.8, 3.0);
                put(&mut o, x, y, alt + top, 0.0, S_BAR, team, (e.hp / d.hp).clamp(0.0, 1.0), FL_NOFOG, w);
            }
            if d.building && e.qn > 0 && e.team == me && !far {
                put(&mut o, x, y, alt + top + 0.28, 0.0, S_BAR, 7.0, (e.qt / qtime(e.q[0])).clamp(0.0, 1.0), FL_NOFOG, d.size * 0.8);
            }
        }

        if ghost >= 0 && (ghost as usize) < DEFS.len() {
            let d = &DEFS[ghost as usize];
            let ok = self.can_place(ghost as u8, gx, gy, me) && self.explored(ghost as u8, gx, gy) && self.afford(me, &d.cost);
            let (cx, cy) = (gx as f32 + d.size * 0.5, gy as f32 + d.size * 0.5);
            let a = if ghost as u8 == DOCK { 0.0 } else { self.map.height(cx, cy).max(0.0) };
            put(&mut o, cx, cy, a, 0.0, ghost as f32, me as f32, 1.0, FL_NOFOG + if ok { FL_GHOST_OK } else { FL_GHOST_BAD }, d.size);
        }

        // ---- counting sort by model id
        let n = o.len() / STRIDE;
        let mut count = [0u32; 257];
        for k in 0..n {
            count[(o[k * STRIDE + 4] as usize & 255) + 1] += 1;
        }
        for m in 0..256 {
            self.ranges[m * 2] = count[m];
            self.ranges[m * 2 + 1] = count[m + 1];
            count[m + 1] += count[m];
        }
        self.inst.clear();
        self.inst.resize(o.len(), 0.0);
        for k in 0..n {
            let m = o[k * STRIDE + 4] as usize & 255;
            let dst = count[m] as usize * STRIDE;
            count[m] += 1;
            self.inst[dst..dst + STRIDE].copy_from_slice(&o[k * STRIDE..(k + 1) * STRIDE]);
        }
        self.tmp = o;
        n as u32
    }

    /// Territory id and owner per tile, as an RGBA texture for the terrain shader.
    pub fn terr_tex(&mut self) {
        for i in 0..NN {
            let t = self.map.terr[i];
            let owner = self.towner[t as usize];
            self.terr_rgba[i * 4] = t;
            self.terr_rgba[i * 4 + 1] = if owner == 255 { 0 } else { owner + 1 };
            self.terr_rgba[i * 4 + 3] = 255;
        }
        self.terr_dirty = false;
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
            // owned territories get a wash of their owner's colour, borders a stronger one
            let t = self.map.terr[i];
            let owner = self.towner[t as usize];
            if owner != 255 {
                let c = TEAM[owner as usize & 3];
                let edge = (i % N + 1 < N && self.map.terr[i + 1] != t) || (i + N < NN && self.map.terr[i + N] != t);
                let k = if edge { 150 } else { 28 };
                r = (r * (256 - k) + c[0] as u32 * k) >> 8;
                g = (g * (256 - k) + c[1] as u32 * k) >> 8;
                b = (b * (256 - k) + c[2] as u32 * k) >> 8;
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
        let me = self.local;
        let p = self.players[me as usize];
        let ep = p.epoch;
        let _ = write!(
            s,
            "{{\"res\":[{},{},{}],\"pop\":{},\"cap\":{},\"epoch\":{},\"ename\":\"{}\",\"win\":{},\"n\":{},\"tick\":{},\"terr\":{},\"local\":{},\"net\":{},",
            p.res[0] as i32, p.res[1] as i32, p.res[2] as i32, p.pop, p.cap, ep, EPOCH_NAMES[ep as usize], self.winner, self.sim_units, self.tick, p.terr, me, self.net
        );
        // selection summary
        let mut first = NOIDX;
        let mut n = 0;
        let mut has_worker = false;
        let mut cargo = 0;
        let mut transports = 0;
        let mut types: Vec<(u8, u32)> = Vec::new();
        let mut idle_workers = 0;
        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || e.team != me {
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
            if DEFS[e.def as usize].has(F_TRANSPORT) {
                transports += 1;
                cargo += e.ncargo as u32;
            }
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
            let qt = if e.qn > 0 { e.qt / qtime(e.q[0]) } else { 0.0 };
            let _ = write!(s, "],\"qp\":{:.2},\"types\":[", qt);
            for (k, t) in types.iter().take(10).enumerate() {
                let _ = write!(s, "{}[{},{},\"{}\"]", if k > 0 { "," } else { "" }, t.0, t.1, DEFS[t.0 as usize].name);
            }
            s.push(']');
            if d.armed() {
                let _ = write!(s, ",\"dmg\":{},\"rng\":{:.1}", (d.w[0].dmg * p.atk[class_of(d)]) as i32, d.w[0].range);
            }
            if transports > 0 {
                let _ = write!(s, ",\"cargo\":{},\"cargomax\":{}", cargo, transports * CARGO_MAX);
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
                    tip.push_str(match id as u8 {
                        TC => " \u{b7} claims a free territory \u{b7} +15 population",
                        HOUSE => " \u{b7} +10 population",
                        FARM => " \u{b7} endless food, 4 workers",
                        DOCK => " \u{b7} place on water next to the shore",
                        _ => "",
                    });
                    if id as u8 != TC {
                        tip.push_str(" \u{b7} only inside your territory");
                    }
                    act(&mut s, "b", id as u8, d.name, &d.cost, self.afford(me, &d.cost), d.size, &tip);
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
                    if d.has(F_TRANSPORT) {
                        tip.push_str(" \u{b7} carries 10 land units: right-click it to board, right-click a shore to land");
                    }
                    act(&mut s, "t", id as u8, d.name, &d.cost, self.afford(me, &d.cost) && p.pop + d.pop as u32 <= p.cap, 0.0, &tip);
                }
            }
            for (id, t) in TECHS.iter().enumerate() {
                if t.at == bdef && t.epoch <= ep && p.tech >> id & 1 == 0 {
                    let busy = p.tech_busy >> id & 1 == 1;
                    act(&mut s, "r", TECH0 + id as u8, t.name, &t.cost, self.afford(me, &t.cost) && !busy, 0.0, if busy { "Research in progress" } else { t.desc });
                }
            }
            if bdef == TC && ep < MAX_EPOCH {
                let c = &EPOCH_COST[ep as usize + 1];
                tip.clear();
                let _ = write!(tip, "Advance to the {}", EPOCH_NAMES[ep as usize + 1]);
                act(&mut s, "e", RESEARCH, "Next Epoch", c, self.afford(me, c) && !p.researching, 0.0, &tip);
            }
        } else if cargo > 0 {
            act(&mut s, "u", 0, "Unload", &[0, 0, 0], true, 0.0, "Put everyone ashore at the nearest beach");
        }
        s.push_str("],\"techs\":[");
        let mut first_t = true;
        for (id, t) in TECHS.iter().enumerate() {
            if p.tech >> id & 1 == 1 {
                let _ = write!(s, "{}\"{}\"", if first_t { "" } else { "," }, t.name);
                first_t = false;
            }
        }
        s.push_str("]}");
        self.ui = s;
    }
}
