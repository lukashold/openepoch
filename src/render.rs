//! Everything the browser needs to draw a frame, written into flat buffers
//! that JavaScript reads straight out of WASM memory (no per-object calls).
//!
//! Instance layout (12 floats): x, y, altitude, rotation | model, team, param, flags | size, speed, simulation time, activity.
//! Instances are bucket-sorted by model id so the page can issue one instanced
//! draw call per model; `ranges` holds (first, count) for each of the 256 ids.

use crate::defs::*;
use crate::game::*;
use crate::map::*;
use std::fmt::Write;

pub const S_TREE: f32 = if cfg!(feature="ee2-roster6"){0.0}else{100.0};
pub const S_ORE: f32 = if cfg!(feature="ee2-roster6"){2.0}else{102.0};
pub const S_BERRY: f32 = if cfg!(feature="ee2-roster6"){3.0}else{103.0};
pub const S_PROJ: f32 = if cfg!(feature="ee2-roster6"){16.0}else{103.0}; // + weapon kind (1..=6)
pub const S_FLAG: f32 = if cfg!(feature="ee2-roster6"){32.0}else{110.0};
pub const S_SCAFFOLD: f32 = if cfg!(feature="ee2-roster6"){33.0}else{111.0};
// ids from 240 are flat or camera-facing quads rather than models
pub const S_RING: f32 = if cfg!(feature="ee2-roster6"){40.0}else{240.0};
pub const S_BAR: f32 = if cfg!(feature="ee2-roster6"){41.0}else{241.0};
pub const S_BOOM: f32 = if cfg!(feature="ee2-roster6"){42.0}else{242.0};
pub const S_SHADOW: f32 = if cfg!(feature="ee2-roster6"){43.0}else{243.0};

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

pub fn qtime(item: QueueItem) -> f32 {
    if item == RESEARCH {
        EPOCH_TIME
    } else if original_research(item) {
        #[cfg(feature = "ee2-reference")]
        { crate::ee2_research::NODES[(item-ORIGINAL_RESEARCH0) as usize].time }
        #[cfg(not(feature = "ee2-reference"))]
        { 15.0 }
    } else if item >= TECH0 {
        TECHS[(item - TECH0) as usize].time
    } else {
        DEFS[item as usize].time
    }
}

// Render activities: idle, walk, woodcutting, mining, food, building, combat.
pub const ACT_IDLE: f32 = 0.0;
pub const ACT_WALK: f32 = 1.0;
pub const ACT_WOOD: f32 = 2.0;
pub const ACT_MINE: f32 = 3.0;
pub const ACT_FOOD: f32 = 4.0;
pub const ACT_BUILD: f32 = 5.0;
pub const ACT_COMBAT: f32 = 6.0;

impl Game {
    fn render_activity(&self, e: &Ent, d: &Def, speed: f32) -> (f32, f32) {
        if speed > 0.25 { return (ACT_WALK, e.rot); }
        if cfg!(feature="ee2-reference")&&d.has(F_WORKER)&&e.order==O_GATHER&&e.q[1]==1{return (ACT_IDLE,e.rot);}
        if d.has(F_WORKER) && e.order == O_GATHER {
            let (tx, ty, reach) = if let Some(j) = self.get(e.otgt) {
                let target = self.ents[j];
                if target.built < 1.0 { return (ACT_IDLE, e.rot); }
                (target.x, target.y, DEFS[target.def as usize].size * 0.5 + 0.95)
            } else {
                let t = e.otile as usize;
                if t >= NN || self.map.res[t] != e.q[0] as u8 { return (ACT_IDLE, e.rot); }
                ((t % N) as f32 + 0.5, (t / N) as f32 + 0.5, 1.45)
            };
            if (tx-e.x).abs().max((ty-e.y).abs()) <= reach {
                let activity = match e.q[0] as u8 { R_WOOD => ACT_WOOD, R_FOOD => ACT_FOOD, _ => ACT_MINE };
                return (activity, (ty-e.y).atan2(tx-e.x));
            }
        }
        if d.has(F_WORKER) && e.order == O_BUILD {
            if let Some(j) = self.get(e.otgt) {
                let b = self.ents[j];
                if b.built < 1.0 && (b.x-e.x).abs().max((b.y-e.y).abs()) <= Self::building_half(&b)+0.95 {
                    return (ACT_BUILD, (b.y-e.y).atan2(b.x-e.x));
                }
            }
        }
        if e.order != O_MOVE && d.armed() {
            if let Some(j) = self.get(e.tgt) {
                let t = self.ents[j]; let target = &DEFS[t.def as usize];
                if let Some(w) = d.weapon_vs(target.dom) {
                    if ((t.x-e.x).powi(2)+(t.y-e.y).powi(2)).sqrt()-self.entity_radius(&t) <= w.range+self.modifiers[e.team as usize][e.def as usize][6] {
                        return (ACT_COMBAT, (t.y-e.y).atan2(t.x-e.x));
                    }
                }
            }
        }
        (ACT_IDLE, e.rot)
    }

    /// Is every tile of this footprint explored by the local player?
    pub fn explored(&self, def:DefId, tx: i32, ty: i32) -> bool {
        self.footprint_explored(self.local,def,tx,ty)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, alpha: f32, x0: f32, y0: f32, x1: f32, y1: f32, ghost: i32, gx: i32, gy: i32) -> u32 {
        self.render_with_rotation(alpha,x0,y0,x1,y1,ghost,gx,gy,0)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_rotation(&mut self,alpha:f32,x0:f32,y0:f32,x1:f32,y1:f32,ghost:i32,gx:i32,gy:i32,turn:u32)->u32{
        #[cfg(feature="ee2-render-meta7")]
        crate::render_meta7::begin_frame();
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
                if r == R_NONE || self.vis[i] == 0 || self.map.blocked[i] {
                    continue;
                }
                let h = hash(tx as i32, ty as i32, 5);
                let jx = (h & 255) as f32 / 255.0 - 0.5;
                let jy = ((h >> 8) & 255) as f32 / 255.0 - 0.5;
                let (cx, cy) = (tx as f32 + 0.5 + jx * 0.4, ty as f32 + 0.5 + jy * 0.4);
                let alt = self.map.height(cx, cy).max(0.0);
                let rot = (h >> 16) as f32 * 0.001;
                let hidden=cfg!(feature="ee2-reference")&&self.players[self.local as usize].epoch<crate::economy::REVEAL_EPOCH[r as usize-1];
                if hidden{put(&mut o,cx,cy,alt,rot,crate::ids::resource_render(110),0.0,0.0,0.0,1.0);continue;}
                match r {
                    R_WOOD => put(&mut o, cx, cy, alt, rot, S_TREE + ((h >> 20) & 1) as f32, 0.0, 0.0, 0.0, 1.0 + jx * 0.35),
                    R_METAL => put(&mut o, cx, cy, alt, rot, S_ORE, 0.0, 0.0, 0.0, 1.0),
                    R_STONE..=R_URANIUM => put(&mut o,cx,cy,alt,rot,crate::ids::resource_render(104+(r-R_STONE)as u32),0.0,0.0,0.0,1.0),
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
            let rot = if d.building{e.rot}else{e.prot + angdiff(e.rot, e.prot) * alpha};
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
                let before=o.len();put(&mut o, x, y, a, rot, crate::ids::unit_render(e.def), team, e.built, fl + FL_BUILDING, d.size);
                if o.len()>before {o[before+10]=(self.tick as f32+alpha)*DT;o[before+11]=if d.has(F_EXTRACT)&&e.ncargo>0&&self.resource_unlocked(e.team,Self::extractor_kind(e.def)) {ACT_MINE as f32}else{ACT_IDLE as f32};}
                #[cfg(feature="ee2-render-meta7")]
                if o.len()>before {crate::render_meta7::actor(before/STRIDE,self.h(i),e,self.max_hp(e.team,e.def));}
                #[cfg(feature="ee2-pack8")]
                if o.len()>before {crate::pack8_render::actor(before/STRIDE,self.canonical_pack(i),self.pack_spec(e).is_some_and(|s|s.capable));}
                if e.built < 1.0 && !far {
                    put(&mut o, x, y, a, rot, S_SCAFFOLD, team, e.built, fl, d.size);
                }
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
                if d.dom == AIR || d.dom == LAND {
                    put(&mut o, x, y, ground + 0.08, 0.0, S_SHADOW, 0.0, 0.0, 0.0, d.size * 1.1);
                }
                // people are drawn larger than their collision size so they stay readable
                let k = match d.armor {
                    A_INF => 1.5,
                    A_CAV => 1.3,
                    _ => 1.15,
                };
                let speed = ((e.x-e.px).powi(2)+(e.y-e.py).powi(2)).sqrt() / DT;
                let (activity, facing) = self.render_activity(e, d, speed);
                let pose_rot = if (8..=17).contains(&e.def) && activity >= ACT_WOOD { facing } else { rot };
                let before = o.len();
                put(&mut o, x, y, a, pose_rot, crate::ids::unit_render(e.def), team, 0.0, fl, d.size * k);
                if o.len() > before {
                    // Stable visual animation phase; independent of movement and simulation state.
                    o[before + 6] = (i as f32 * 0.75487766 + e.gen as f32 * 0.38196602).fract();
                    o[before + 9] = speed;
                    o[before + 10] = (self.tick as f32 + alpha) * DT;
                    o[before + 11] = activity;
                    #[cfg(feature="ee2-render-meta7")]
                    crate::render_meta7::actor(before/STRIDE,self.h(i),e,self.max_hp(e.team,e.def));
                    #[cfg(feature="ee2-pack8")]
                    crate::pack8_render::actor(before/STRIDE,self.canonical_pack(i),self.pack_spec(e).is_some_and(|s|s.capable));
                }
                (a.max(0.0), d.size * 0.5 + 0.6)
            };
            let maxhp=self.max_hp(e.team,e.def);
            if e.sel || (e.hp < maxhp && !far) {
                let w = (d.rad() * 2.0).clamp(0.8, 3.0);
                put(&mut o, x, y, alt + top, 0.0, S_BAR, team, (e.hp / maxhp).clamp(0.0, 1.0), FL_NOFOG, w);
            }
            if d.building && e.qn > 0 && e.team == me && !far {
                put(&mut o, x, y, alt + top + 0.28, 0.0, S_BAR, 7.0, (e.qt / qtime(e.q[0])).clamp(0.0, 1.0), FL_NOFOG, d.size * 0.8);
            }
        }

        if ghost>=0&&(ghost as usize)<DEFS.len()&&turn<crate::placement::ROTATION_STEPS{
            let d=&DEFS[ghost as usize];let side=Self::footprint_side(ghost as DefId,turn);
            if side>0{let ids:Vec<usize>=(0..self.high).filter(|&i|self.ents[i].alive&&self.ents[i].sel&&self.ents[i].team==me).collect();let ok=self.placement_error_for(me,ghost as DefId,gx,gy,turn,&ids,true)==0;
                let(cx,cy)=(gx as f32+side as f32*0.5,gy as f32+side as f32*0.5);let a=if ghost as DefId==DOCK{0.0}else{self.map.height(cx,cy).max(0.0)};
                put(&mut o,cx,cy,a,Self::angle_for_turn(turn),crate::ids::unit_render(ghost as DefId),me as f32,1.0,FL_NOFOG+if ok{FL_GHOST_OK}else{FL_GHOST_BAD},d.size);
            }
        }

        // ---- counting sort by model id
        let n = o.len() / STRIDE;
        let capacity=crate::ids::render_capacity();let mut count = vec![0u32;capacity+1];
        for k in 0..n {
            count[o[k * STRIDE + 4] as usize + 1] += 1;
        }
        for m in 0..capacity {
            self.ranges[m * 2] = count[m];
            self.ranges[m * 2 + 1] = count[m + 1];
            count[m + 1] += count[m];
        }
        self.inst.clear();
        self.inst.resize(o.len(), 0.0);
        #[cfg(feature="ee2-render-meta7")]
        crate::render_meta7::prepare_sort(n);
        for k in 0..n {
            let m = o[k * STRIDE + 4] as usize;
            let dst = count[m] as usize * STRIDE;
            count[m] += 1;
            self.inst[dst..dst + STRIDE].copy_from_slice(&o[k * STRIDE..(k + 1) * STRIDE]);
            #[cfg(feature="ee2-render-meta7")]
            crate::render_meta7::sorted_row(k,dst/STRIDE,&o[k*STRIDE..(k+1)*STRIDE]);
        }
        self.tmp = o;
        #[cfg(feature="ee2-render-meta7")]
        crate::render_meta7::end_frame();
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
            match self.map.res[i] {R_METAL=>(r,g,b)=(150,150,165),R_STONE=>(r,g,b)=(180,180,170),R_GOLD=>(r,g,b)=(230,195,55),R_TIN=>(r,g,b)=(160,180,190),R_SALTPETER=>(r,g,b)=(210,195,170),R_OIL=>(r,g,b)=(70,60,52),R_URANIUM=>(r,g,b)=(135,180,60),_=>{}}
            let kind=self.map.res[i];if cfg!(feature="ee2-reference")&&kind>0&&self.players[self.local as usize].epoch<crate::economy::REVEAL_EPOCH[kind as usize-1]{(r,g,b)=(150,150,150);}
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
        s.push_str("{\"res\":[");
        for r in 0..crate::economy::RESOURCE_COUNT {let _=write!(s,"{}{}",if r>0 {","}else{""},self.resource(me,r).floor() as i32);}
        s.push_str("],\"resourceActive\":[");
        for r in 0..crate::economy::RESOURCE_COUNT {let _=write!(s,"{}{}",if r>0 {","}else{""},crate::economy::unlocked(r,ep));}
        s.push_str("],");
        let _ = write!(
            s,
            "\"pop\":{},\"cap\":{},\"epoch\":{},\"ename\":\"{}\",\"win\":{},\"n\":{},\"tick\":{},\"terr\":{},\"local\":{},\"net\":{},",
            p.pop, p.cap, ep, EPOCH_NAMES[ep as usize], self.winner, self.sim_units, self.tick, p.terr, me, self.net
        );
        // selection summary
        let mut first = NOIDX;
        let mut n = 0;
        let mut has_worker = false;
        let mut cargo = 0;
        let mut cargo_units = 0;
        let mut cargo_max = 0;
        let mut types: Vec<(DefId, u32)> = Vec::new();
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
            if DEFS[e.def as usize].has(F_TRANSPORT|F_RESEARCH|F_EXTRACT|F_DROPOFF|F_GARRISON) {
                cargo_max += self.cargo_capacity(e.team,e.def);
                cargo += self.cargo_slots_used(e) as u32;
                cargo_units += e.ncargo as u32;
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
            let speed = ((e.x-e.px).powi(2)+(e.y-e.py).powi(2)).sqrt()/DT;
            let activity = if d.building { ACT_IDLE } else { self.render_activity(&e,d,speed).0 };
            let _ = write!(s, ",\"activity\":{}", activity as u32);
            if cfg!(feature="ee2-reference")&&d.has(F_WORKER){let _=write!(s,",\"carrying\":{:.2},\"carriedResource\":{},\"returning\":{}",e.carry,e.q[2],e.q[1]==1);}
            let _ = write!(s, ",\"name\":\"{}\",\"def\":{},\"hp\":{},\"max\":{},\"built\":{:.2},\"q\":[", d.name, e.def, e.hp.max(0.0).round() as i32, self.max_hp(e.team,e.def).round() as i32, e.built);
            for k in 0..e.qn as usize {
                let _ = write!(s, "{}{}", if k > 0 { "," } else { "" }, e.q[k]);
            }
            #[cfg(not(feature="ee2-paidqueue7"))]
            let qt = if e.qn > 0 { e.qt / if (e.q[0] as usize) < DEFS.len() {self.production_time(e.team,e.q[0] as DefId)} else {qtime(e.q[0])} } else { 0.0 };
            #[cfg(feature="ee2-paidqueue7")]
            let qt=if e.qn>0{let limit=self.queue_time_limit(first,0);if limit>0.0{e.qt/limit}else{1.0}}else{0.0};
            let _ = write!(s, "],\"qp\":{:.2},\"types\":[", qt);
            for (k, t) in types.iter().take(10).enumerate() {
                let _ = write!(s, "{}[{},{},\"{}\"]", if k > 0 { "," } else { "" }, t.0, t.1, DEFS[t.0 as usize].name);
            }
            s.push(']');
            if d.armed() {
                let _ = write!(s, ",\"dmg\":{},\"rng\":{:.1}", (self.scaled_weapon_damage(me,e.def,d.w[0].dmg,p.atk[class_of(d)])) as i32, d.w[0].range+self.modifiers[me as usize][e.def as usize][6]);
            }
            if cargo_max > 0 {
                let _ = write!(s, ",\"cargo\":{},\"cargomax\":{}", cargo, cargo_max);
                if e.def==FORTRESS {let _=write!(s,",\"cargoUnits\":{},\"garrisonHealRate\":{},\"garrisonAttackBonus\":{:.1},\"garrisonSlotRule\":\"native-population\"",cargo_units,self.fortress_heal_rate(e.team),self.fortress_attack_bonus(&e)*100.0);}
            }
        }
        s.push_str("},\"act\":[");
        let mut k = 0;
        let mut act = |s: &mut String, kind: &str, id:QueueItem, name: &str, cost: &[u16], ok: bool, size: f32, tip: &str| {
            #[cfg(feature = "ee2-reference")]
            let tp = if kind == "e" {crate::ee2_research::ADVANCE_COST[ep as usize+1]} else if original_research(id as QueueItem) {crate::ee2_research::NODES[(id as QueueItem-ORIGINAL_RESEARCH0) as usize].cost} else {0.0};
            #[cfg(not(feature = "ee2-reference"))]
            let tp=0.0;
            let _ = write!(s, "{}{{\"k\":\"{}\",\"id\":{},\"name\":\"{}\",\"cost\":[", if k > 0 { "," } else { "" }, kind, id, name);
            for r in 0..crate::economy::RESOURCE_COUNT {let _=write!(s,"{}{}",if r>0 {","}else{""},cost.get(r).copied().unwrap_or(0));}
            let _=write!(s,"],\"tp\":{},\"ok\":{},\"size\":{},\"tip\":\"{}\"}}",tp,ok,size,tip);
            k += 1;
        };
        let mut tip = String::new();
        if has_worker {
            for (id, d) in DEFS.iter().enumerate() {
                if d.building && d.epoch <= ep {
                    tip.clear();
                    let _ = write!(tip, "{} HP", self.max_hp(me,id as DefId) as i32);
                    tip.push_str(match id as DefId {
                        TC if cfg!(feature="ee2-reference") => " · angrenzendes freies Gebiet: fertiges Stadtzentrum · entferntes Gebiet: zusätzlich fertige Festung",
                        TC => " \u{b7} claims a free territory \u{b7} +15 population",
                        HOUSE => " \u{b7} +10 population",
                        FARM => " \u{b7} endless food, 4 workers",
                        DOCK => " \u{b7} place on water next to the shore",
                        UNIVERSITY => " · Bürger mit Rechtsklick einquartieren · 0,03 Technologiepunkte je Bürger und Sekunde",
                        FORTRESS => " · sichert entfernte Gebietsansprüche · Besatzung mit Rechtsklick · 10 Plätze · 3 Prozent Angriff je belegtem Platz",
                        _ => "",
                    });
                    if id as DefId != TC {
                        tip.push_str(if cfg!(feature="ee2-reference") {" · neutrales Land: doppelte Bauzeit"} else {" \u{b7} only inside your territory"});
                    }
                    let cost=self.unit_cost(me,id as DefId);
                    act(&mut s, "b", id as QueueItem, d.name, &cost, self.afford(me, &cost), d.size, &tip);
                }
            }
        } else if first != NOIDX && DEFS[self.ents[first].def as usize].building && self.ents[first].built >= 1.0 {
            let bdef = self.ents[first].def;
            for (id, d) in DEFS.iter().enumerate() {
                if d.made_at == bdef && d.epoch <= ep && d.until > ep && {
                    #[cfg(feature="ee2-civ8")]{self.definition_civ_available(self.local,id as DefId)}
                    #[cfg(not(feature="ee2-civ8"))]{true}
                } {
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
                    let cost=self.unit_cost(me,id as DefId);
                    act(&mut s, "t", id as QueueItem, d.name, &cost, self.can_train(first,id as QueueItem), 0.0, &tip);
                }
            }
            #[cfg(not(feature = "ee2-reference"))]
            for (id, t) in TECHS.iter().enumerate() {
                if t.at == bdef && t.epoch <= ep && p.tech >> id & 1 == 0 {
                    let busy = p.tech_busy >> id & 1 == 1;
                    act(&mut s, "r", TECH0 + id as DefId, t.name, &t.cost, self.afford(me, &t.cost) && !busy, 0.0, if busy { "Research in progress" } else { t.desc });
                }
            }
            #[cfg(feature = "ee2-reference")]
            for (id,t) in crate::ee2_research::NODES.iter().enumerate() {
                if t.host == bdef && t.epoch == ep && !self.has_research(me,id) {
                    let item=ORIGINAL_RESEARCH0+id as QueueItem;
                    let busy=self.research_busy(me,id);
                    act(&mut s,"r",item,t.name,&[0,0,0],self.can_train(first,item),0.0,if busy {"Forschung läuft"} else {"15 Sekunden Forschung · zählt für den Epochenwechsel"});
                }
            }
            if bdef == TC && ep < self.match_final_epoch() {
                #[cfg(not(feature = "ee2-reference"))]
                let c = &EPOCH_COST[ep as usize + 1];
                #[cfg(feature = "ee2-reference")]
                let c=&[0,0,0];
                tip.clear();
                let _ = write!(tip, "Nächste Epoche: {}", EPOCH_NAMES[ep as usize + 1]);
                if cfg!(feature = "ee2-reference") {let _=write!(tip," · {}/6 Technologien der aktuellen Epoche erforscht",self.research_count(me,ep));}
                act(&mut s, "e", RESEARCH, "Next Epoch", c, self.can_advance(me), 0.0, &tip);
            }
            if DEFS[bdef as usize].has(F_RESEARCH|F_EXTRACT|F_DROPOFF|F_GARRISON) && cargo > 0 { act(&mut s,"u",0,"Besatzung ausquartieren",&[0,0,0],true,0.0,"Die einquartierten Einheiten verlassen das Gebäude"); }
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
        let rate:f32=self.ents[..self.high].iter().filter(|e| e.alive && e.team==me && e.built>=1.0 && DEFS[e.def as usize].has(F_RESEARCH)).map(|e| UNIVERSITY_RATE*(e.ncargo as usize).min(self.cargo_capacity(e.team,e.def)) as f32*self.modifiers[me as usize][e.def as usize][7]).sum();
        let _=write!(s,"],\"research\":{{\"points\":{:.2},\"rate\":{:.2},\"count\":{},\"total\":{}}}}}",self.research_points[me as usize],rate,self.research_count(me,ep),self.researched[me as usize].iter().map(|v|v.count_ones()).sum::<u32>());
        s.pop();self.append_diplomacy_ui(&mut s);s.push('}');
        self.ui = s;
    }
}
