//! The simulation: entities, orders, movement, combat, economy.
//!
//! Runs at a fixed 20 Hz, fully decoupled from rendering (which interpolates).
//! Nothing in the per-tick path allocates, so there are no GC or allocator
//! hitches no matter how many units are alive.

use crate::defs::*;
use crate::map::*;

pub const MAXE: usize = 8192;
pub const DT: f32 = 0.05;
pub const NONE: u32 = u32::MAX;
pub const NOIDX: usize = usize::MAX;

pub const O_IDLE: u8 = 0;
pub const O_MOVE: u8 = 1;
pub const O_AMOVE: u8 = 2;
pub const O_ATTACK: u8 = 3;
pub const O_GATHER: u8 = 4;
pub const O_BUILD: u8 = 5;
pub const O_BOARD: u8 = 6;
pub const O_UNLOAD: u8 = 7;
/// Marker in `Ent::stuck`: the unit walked onto a transport, remove it without an explosion.
const BOARDED: u8 = 255;

pub const RESEARCH: u8 = 255;
const GC: usize = 64; // grid cells per axis
const CELL: f32 = 4.0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Ent {
    pub alive: bool,
    pub def: u8,
    pub team: u8,
    pub gen: u16,
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    pub vx: f32,
    pub vy: f32,
    pub rot: f32,
    pub prot: f32,
    pub hp: f32,
    pub cd: f32,
    pub order: u8,
    pub ox: f32,
    pub oy: f32,
    pub otgt: u32,
    pub otile: u32,
    pub tgt: u32,
    pub fkey: u32,
    pub fslot: u16,
    pub sel: bool,
    /// Construction progress, 1.0 = finished.
    pub built: f32,
    /// Buildings: production queue. Workers: `q[0]` = resource kind being gathered.
    pub q: [u8; 5],
    pub qn: u8,
    pub qt: f32,
    pub rx: f32,
    pub ry: f32,
    /// Bit per team that currently detects this stealth unit.
    pub revealed: u8,
    pub reveal_t: f32,
    pub stuck: u8,
    pub carry: f32,
    pub flash: f32,
    /// Workers: resource kind they were gathering before being pulled off to build.
    pub prev: u8,
    /// Transports: unit type and health (percent) of each passenger.
    pub cargo: [u8; CARGO_MAX],
    pub cargo_hp: [u8; CARGO_MAX],
    pub ncargo: u8,
}

const E0: Ent = Ent {
    alive: false, def: 0, team: 0, gen: 0, x: 0.0, y: 0.0, px: 0.0, py: 0.0, vx: 0.0, vy: 0.0, rot: 0.0, prot: 0.0, hp: 0.0, cd: 0.0,
    order: O_IDLE, ox: 0.0, oy: 0.0, otgt: NONE, otile: 0, tgt: NONE, fkey: 0, fslot: u16::MAX, sel: false, built: 1.0, q: [0; 5], qn: 0,
    qt: 0.0, rx: -1.0, ry: -1.0, revealed: 0, reveal_t: 0.0, stuck: 0, carry: 0.0, flash: 0.0, prev: 0, cargo: [0; CARGO_MAX], cargo_hp: [0; CARGO_MAX], ncargo: 0,
};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Player {
    pub res: [f32; 3],
    pub pop: u32,
    pub cap: u32,
    pub epoch: u8,
    pub researching: bool,
    pub alive: bool,
    pub ai: bool,
    pub ai_next_attack: u32,
    pub ai_nodock: bool,
    pub ai_noexp: bool,
    pub buildings: u32,
    /// Bit per researched technology / per technology currently in a queue.
    pub tech: u32,
    pub tech_busy: u32,
    pub atk: [f32; 5],
    pub dfn: [f32; 5],
    pub gather: f32,
    pub terr: u32,
}

impl Player {
    /// Rebuild the multipliers from the technology bits.
    pub fn recompute(&mut self) {
        self.atk = [1.0; 5];
        self.dfn = [1.0; 5];
        self.gather = 1.0;
        for (k, t) in TECHS.iter().enumerate() {
            if self.tech >> k & 1 == 1 {
                match t.kind {
                    T_ATK => self.atk[t.class] *= t.amount,
                    T_DEF => self.dfn[t.class] *= t.amount,
                    _ => self.gather *= t.amount,
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Proj {
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    pub tx: f32,
    pub ty: f32,
    pub tgt: u32,
    pub speed: f32,
    pub dmg: f32,
    pub kind: u8,
    pub team: u8,
    pub splash: f32,
    pub targets: u8,
    /// Distance at launch, so the renderer can draw an arc.
    pub d0: f32,
}

#[derive(Clone, Copy)]
pub struct Fx {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub age: f32,
    pub life: f32,
}

pub struct Grid {
    pub start: Vec<u32>,
    pub items: Vec<u16>,
    cursor: Vec<u32>,
}

#[inline]
fn cell_of(x: f32, y: f32) -> usize {
    let cx = ((x / CELL) as i32).clamp(0, GC as i32 - 1) as usize;
    let cy = ((y / CELL) as i32).clamp(0, GC as i32 - 1) as usize;
    cy * GC + cx
}

#[inline]
pub fn angdiff(a: f32, b: f32) -> f32 {
    let mut d = a - b;
    while d > core::f32::consts::PI {
        d -= core::f32::consts::TAU;
    }
    while d < -core::f32::consts::PI {
        d += core::f32::consts::TAU;
    }
    d
}

pub struct Game {
    pub map: Map,
    pub ents: Vec<Ent>,
    pub free: Vec<u16>,
    pub high: usize,
    pub grid: Grid,
    pub projs: Vec<Proj>,
    pub fx: Vec<Fx>,
    pub players: [Player; 4],
    pub nplayers: usize,
    pub tick: u32,
    pub rng: Rng,
    pub vis: Vec<u8>,
    pub inst: Vec<f32>,
    pub mini: Vec<u8>,
    pub ui: String,
    pub winner: u8,
    pub bench: u32,
    pub groups: Vec<Vec<u32>>,
    pub push: Vec<(f32, f32)>,
    pub scratch: Vec<usize>,
    pub sim_units: u32,
    // ---- not part of the deterministic simulation state ----
    /// Which player this client controls and sees through.
    pub local: u8,
    /// Lockstep multiplayer: commands go to `outbox` instead of being applied at once.
    pub net: bool,
    pub seed0: u32,
    pub n_ai: u32,
    #[allow(dead_code)]
    pub humans: u32,
    pub towner: [u8; 32],
    pub terr_rgba: Vec<u8>,
    pub terr_dirty: bool,
    /// Sound cues for the page: (kind, x, y) triples.
    pub events: Vec<f32>,
    pub outbox: Vec<u8>,
    pub inbox: Vec<u8>,
    pub savebuf: Vec<u8>,
    pub tmp: Vec<f32>,
    pub ranges: Vec<u32>,
    /// Camera position (x, height, y) for picking; height 0 = top-down, no parallax.
    pub eye: [f32; 3],
}

impl Game {
    pub fn new(seed: u32, n_ai: u32, bench: u32, humans: u32, local: u32) -> Game {
        let humans = humans.clamp(1, 2);
        let nplayers = ((humans + n_ai) as usize).clamp(2, 4);
        let map = Map::new(seed.max(1), nplayers);
        let mut g = Game {
            map,
            ents: vec![E0; MAXE],
            free: (0..MAXE as u16).rev().collect(),
            high: 0,
            grid: Grid { start: vec![0; GC * GC + 1], items: Vec::with_capacity(MAXE), cursor: vec![0; GC * GC] },
            projs: Vec::with_capacity(4096),
            fx: Vec::with_capacity(1024),
            players: [Player::default(); 4],
            nplayers,
            tick: 0,
            rng: Rng(seed.wrapping_mul(2_654_435_761) | 1),
            vis: vec![0; NN],
            inst: Vec::with_capacity(8 * 16384),
            mini: vec![0; NN * 4],
            ui: String::with_capacity(4096),
            winner: 0,
            bench,
            groups: vec![Vec::new(); 10],
            push: vec![(0.0, 0.0); MAXE],
            scratch: Vec::with_capacity(256),
            sim_units: 0,
            local: (local as usize).min(nplayers - 1) as u8,
            net: humans > 1,
            seed0: seed,
            n_ai,
            humans,
            towner: [255; 32],
            terr_rgba: vec![0; NN * 4],
            terr_dirty: true,
            events: Vec::with_capacity(256),
            outbox: Vec::with_capacity(1024),
            inbox: Vec::new(),
            savebuf: Vec::new(),
            tmp: Vec::with_capacity(12 * 16384),
            ranges: vec![0; 512],
            eye: [0.0; 3],
        };
        for p in 0..nplayers {
            let (sx, sy) = g.map.starts[p];
            g.players[p] = Player { res: [400.0, 400.0, 200.0], cap: 15, alive: true, ai: p >= humans as usize, ai_next_attack: 20 * 240, ..Player::default() };
            g.players[p].recompute();
            g.spawn(TC, p as u8, sx as f32, sy as f32);
            for k in 0..5 {
                let a = k as f32 * 1.256;
                g.spawn(WORKER, p as u8, sx as f32 + a.cos() * 3.4, sy as f32 + a.sin() * 3.4);
            }
        }
        if bench > 0 {
            for p in g.players.iter_mut().take(nplayers) {
                p.epoch = MAX_EPOCH;
                p.ai = false;
                p.res = [99999.0; 3];
            }
            for v in g.vis.iter_mut() {
                *v = 255;
            }
        }
        g.build_grid();
        g.update_fog();
        g
    }

    // ------------------------------------------------------------------ entities

    #[inline]
    pub fn h(&self, i: usize) -> u32 {
        ((self.ents[i].gen as u32) << 16) | i as u32
    }
    #[inline]
    pub fn get(&self, h: u32) -> Option<usize> {
        if h == NONE {
            return None;
        }
        let i = (h & 0xffff) as usize;
        // handles can come off the network, so never index blindly
        let e = self.ents.get(i)?;
        if e.alive && e.gen as u32 == h >> 16 { Some(i) } else { None }
    }

    pub fn spawn(&mut self, def: u8, team: u8, x: f32, y: f32) -> usize {
        let i = match self.free.pop() {
            Some(i) => i as usize,
            None => return NOIDX,
        };
        if i + 1 > self.high {
            self.high = i + 1;
        }
        let d = &DEFS[def as usize];
        let gen = self.ents[i].gen;
        self.ents[i] = Ent { alive: true, def, team, gen, x, y, px: x, py: y, ox: x, oy: y, hp: d.hp, ..E0 };
        if d.building {
            self.block(i, true);
        }
        i
    }

    fn block(&mut self, i: usize, on: bool) {
        let e = &self.ents[i];
        let s = DEFS[e.def as usize].size as usize;
        let tx0 = (e.x - s as f32 * 0.5).round() as usize;
        let ty0 = (e.y - s as f32 * 0.5).round() as usize;
        for ty in ty0..ty0 + s {
            for tx in tx0..tx0 + s {
                let t = ty * N + tx;
                self.map.blocked[t] = on;
                self.map.upd_pass(t);
            }
        }
        self.map.nav_ver += 1;
    }

    fn kill(&mut self, i: usize) {
        let e = self.ents[i];
        let d = &DEFS[e.def as usize];
        if d.building {
            self.block(i, false);
            for &q in &e.q[..e.qn as usize] {
                if q == RESEARCH {
                    self.players[e.team as usize].researching = false;
                } else if q >= TECH0 {
                    self.players[e.team as usize].tech_busy &= !(1 << (q - TECH0));
                }
            }
        }
        if d.building || e.stuck != BOARDED {
            if self.fx.len() < 1000 {
                self.fx.push(Fx { x: e.x, y: e.y, size: d.size * if d.building { 1.6 } else { 1.3 }, age: 0.0, life: 0.6 });
            }
            self.ev(if d.building { 11.0 } else { 10.0 }, e.x, e.y);
        }
        let s = &mut self.ents[i];
        s.alive = false;
        s.sel = false;
        s.gen = s.gen.wrapping_add(1);
        self.free.push(i as u16);
    }

    #[inline]
    pub fn can_see(&self, team: u8, j: usize) -> bool {
        let t = &self.ents[j];
        if !DEFS[t.def as usize].has(F_STEALTH) {
            return true;
        }
        t.team == team || t.reveal_t > 0.0 || (t.revealed >> team) & 1 == 1
    }

    /// Is entity `j` currently visible to the human player (fog + stealth)?
    pub fn visible(&self, j: usize) -> bool {
        let t = &self.ents[j];
        if t.team == self.local {
            return true;
        }
        let v = self.vis[tile_of(t.x, t.y)];
        let need = if DEFS[t.def as usize].building { 128 } else { 255 };
        v >= need && self.can_see(self.local, j)
    }

    // ------------------------------------------------------------------ economy

    pub fn afford(&self, team: u8, c: &[u16; 3]) -> bool {
        let r = &self.players[team as usize].res;
        r[0] >= c[0] as f32 && r[1] >= c[1] as f32 && r[2] >= c[2] as f32
    }
    pub fn pay(&mut self, team: u8, c: &[u16; 3], sign: f32) {
        let r = &mut self.players[team as usize].res;
        for k in 0..3 {
            r[k] -= c[k] as f32 * sign;
        }
    }

    /// `tx,ty` is the top-left tile of the footprint.
    pub fn can_place(&self, def: u8, tx: i32, ty: i32, team: u8) -> bool {
        let s = DEFS[def as usize].size as i32;
        if tx < 2 || ty < 2 || tx + s > N as i32 - 2 || ty + s > N as i32 - 2 {
            return false;
        }
        // territories: a town center claims a free one, everything else needs our own
        let owner = self.towner[self.map.terr[(ty + s / 2) as usize * N + (tx + s / 2) as usize] as usize];
        if owner != team && !(def == TC && owner == 255) {
            return false;
        }
        for y in ty..ty + s {
            for x in tx..tx + s {
                let t = y as usize * N + x as usize;
                if self.map.blocked[t] || self.map.res[t] != R_NONE {
                    return false;
                }
                let ok = if def == DOCK { self.map.ter[t] <= T_SHALLOW } else { self.map.pass[0][t] };
                if !ok {
                    return false;
                }
            }
        }
        if def == DOCK {
            // must touch walkable shore so workers can reach it
            for k in -1..=s {
                for &(x, y) in &[(tx + k, ty - 1), (tx + k, ty + s), (tx - 1, ty + k), (tx + s, ty + k)] {
                    if self.map.pass[0][y as usize * N + x as usize] {
                        return true;
                    }
                }
            }
            return false;
        }
        true
    }

    pub fn place(&mut self, def: u8, tx: i32, ty: i32, team: u8) -> usize {
        if def as usize >= DEFS.len() {
            return NOIDX;
        }
        let d = &DEFS[def as usize];
        if !d.building || d.epoch > self.players[team as usize].epoch || !self.afford(team, &d.cost) || !self.can_place(def, tx, ty, team) {
            return NOIDX;
        }
        let i = self.spawn(def, team, tx as f32 + d.size * 0.5, ty as f32 + d.size * 0.5);
        if i != NOIDX {
            self.pay(team, &d.cost, 1.0);
            self.ents[i].built = 0.0;
            self.ents[i].hp = d.hp * 0.1;
        }
        i
    }

    /// Would `train(b, def)` succeed right now?
    pub fn can_train(&self, b: usize, def: u8) -> bool {
        let e = &self.ents[b];
        let p = &self.players[e.team as usize];
        if !DEFS[e.def as usize].building || e.built < 1.0 || e.qn >= 5 {
            return false;
        }
        if def == RESEARCH {
            e.def == TC && !p.researching && p.epoch < MAX_EPOCH && self.afford(e.team, &EPOCH_COST[p.epoch as usize + 1])
        } else if def >= TECH0 {
            let k = (def - TECH0) as usize;
            k < TECHS.len() && TECHS[k].at == e.def && TECHS[k].epoch <= p.epoch && (p.tech | p.tech_busy) >> k & 1 == 0 && self.afford(e.team, &TECHS[k].cost)
        } else {
            (def as usize) < DEFS.len() && DEFS[def as usize].made_at == e.def && DEFS[def as usize].epoch <= p.epoch && self.afford(e.team, &DEFS[def as usize].cost)
        }
    }

    pub fn train(&mut self, b: usize, def: u8) -> bool {
        if !self.can_train(b, def) {
            return false;
        }
        let team = self.ents[b].team;
        if def == RESEARCH {
            let ep = self.players[team as usize].epoch as usize;
            self.pay(team, &EPOCH_COST[ep + 1], 1.0);
            self.players[team as usize].researching = true;
        } else if def >= TECH0 {
            self.pay(team, &TECHS[(def - TECH0) as usize].cost, 1.0);
            self.players[team as usize].tech_busy |= 1 << (def - TECH0);
        } else {
            self.pay(team, &DEFS[def as usize].cost, 1.0);
        }
        let e = &mut self.ents[b];
        e.q[e.qn as usize] = def;
        e.qn += 1;
        true
    }

    pub fn cancel(&mut self, b: usize) {
        let e = self.ents[b];
        if e.qn == 0 {
            return;
        }
        let item = e.q[e.qn as usize - 1];
        if item == RESEARCH {
            let ep = self.players[e.team as usize].epoch as usize;
            self.pay(e.team, &EPOCH_COST[ep + 1], -1.0);
            self.players[e.team as usize].researching = false;
        } else if item >= TECH0 {
            self.pay(e.team, &TECHS[(item - TECH0) as usize].cost, -1.0);
            self.players[e.team as usize].tech_busy &= !(1 << (item - TECH0));
        } else {
            self.pay(e.team, &DEFS[item as usize].cost, -1.0);
        }
        let e = &mut self.ents[b];
        e.qn -= 1;
        if e.qn == 0 {
            e.qt = 0.0;
        }
    }

    // ------------------------------------------------------------------ orders

    pub fn key_for(&self, dom: u8, x: f32, y: f32) -> u32 {
        if dom == AIR {
            return 0;
        }
        let dc = if dom == LAND { 0 } else { 1 };
        self.map.snap(x, y, dc) as u32 | ((dc as u32) << 16)
    }

    pub fn order_move(&mut self, ids: &[usize], x: f32, y: f32, attack: bool) {
        let x = x.clamp(1.0, N as f32 - 1.0);
        let y = y.clamp(1.0, N as f32 - 1.0);
        let kl = self.key_for(LAND, x, y);
        let ks = self.key_for(SEA, x, y);
        let mut k = 0.0f32;
        for &i in ids {
            let e = &mut self.ents[i];
            let d = &DEFS[e.def as usize];
            if d.building || !e.alive {
                continue;
            }
            // sunflower-spiral formation slots around the goal
            let r = 0.85 * k.sqrt() * d.size.max(1.0);
            let a = k * 2.399_96;
            let (mut tx, mut ty) = (x + r * a.cos(), y + r * a.sin());
            let key = if d.dom == LAND { kl } else { ks };
            if d.dom != AIR {
                let dc = if d.dom == LAND { 0 } else { 1 };
                if !self.map.pass[dc][tile_of(tx, ty)] {
                    let t = (key & 0xffff) as usize;
                    tx = (t % N) as f32 + 0.5;
                    ty = (t / N) as f32 + 0.5;
                }
            }
            e.order = if attack { O_AMOVE } else { O_MOVE };
            e.ox = tx;
            e.oy = ty;
            e.tgt = NONE;
            e.otgt = NONE;
            e.fkey = key;
            e.stuck = 0;
            k += 1.0;
        }
    }

    pub fn order_attack(&mut self, ids: &[usize], tj: usize) {
        let t = self.ents[tj];
        let td = &DEFS[t.def as usize];
        let h = self.h(tj);
        for &i in ids {
            let d = &DEFS[self.ents[i].def as usize];
            if d.building {
                continue;
            }
            if d.weapon_vs(td.dom).is_none() {
                if !d.has(F_WORKER) {
                    self.order_move(&[i], t.x, t.y, false);
                }
                continue;
            }
            let key = self.key_for(d.dom, t.x, t.y);
            let e = &mut self.ents[i];
            e.order = O_ATTACK;
            e.otgt = h;
            e.tgt = h;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    pub fn order_gather(&mut self, ids: &[usize], tile: usize) {
        let kind = self.map.res[tile];
        let (cx, cy) = ((tile % N) as f32 + 0.5, (tile / N) as f32 + 0.5);
        let key = self.key_for(LAND, cx, cy);
        for &i in ids {
            let e = &mut self.ents[i];
            if !DEFS[e.def as usize].has(F_WORKER) {
                continue;
            }
            e.order = O_GATHER;
            e.otile = tile as u32;
            e.otgt = NONE;
            e.tgt = NONE;
            e.q[0] = kind;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    /// Send workers to a building: construct it if unfinished, farm it if it is a farm.
    pub fn order_building(&mut self, ids: &[usize], bj: usize) {
        let b = self.ents[bj];
        let h = self.h(bj);
        let key = self.key_for(LAND, b.x, b.y);
        for &i in ids {
            let e = &mut self.ents[i];
            if !DEFS[e.def as usize].has(F_WORKER) {
                continue;
            }
            e.prev = match e.order {
                O_GATHER if e.otgt == NONE => e.q[0],
                O_BUILD => e.prev,
                _ => 0,
            };
            e.order = if b.built < 1.0 { O_BUILD } else { O_GATHER };
            e.q[0] = R_FOOD;
            e.otgt = h;
            e.tgt = NONE;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    /// Land units walk to a transport ship and climb aboard.
    pub fn order_board(&mut self, ids: &[usize], tj: usize) {
        let t = self.ents[tj];
        if !DEFS[t.def as usize].has(F_TRANSPORT) {
            return;
        }
        let h = self.h(tj);
        let key = self.key_for(LAND, t.x, t.y);
        let mut first = None;
        for &i in ids {
            let e = &mut self.ents[i];
            let d = &DEFS[e.def as usize];
            if d.building || d.dom != LAND || e.team != t.team {
                continue;
            }
            e.order = O_BOARD;
            e.otgt = h;
            e.tgt = NONE;
            e.fkey = key;
            e.stuck = 0;
            first.get_or_insert((e.x, e.y));
        }
        // an idle ship comes to the beach to pick them up
        if let (Some((x, y)), O_IDLE) = (first, t.order) {
            self.order_move(&[tj], x, y, false);
        }
    }

    /// Transports sail to the shore nearest `(x, y)` and put everyone ashore.
    pub fn order_unload(&mut self, ids: &[usize], x: f32, y: f32) {
        let key = self.key_for(SEA, x.clamp(1.0, N as f32 - 1.0), y.clamp(1.0, N as f32 - 1.0));
        let t = (key & 0xffff) as usize;
        for &i in ids {
            let e = &mut self.ents[i];
            if !DEFS[e.def as usize].has(F_TRANSPORT) {
                continue;
            }
            e.order = O_UNLOAD;
            e.ox = (t % N) as f32 + 0.5;
            e.oy = (t / N) as f32 + 0.5;
            e.tgt = NONE;
            e.otgt = NONE;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    fn unload(&mut self, e: &mut Ent) {
        let lt = self.map.snap(e.x, e.y, 0);
        let (lx, ly) = ((lt % N) as f32 + 0.5, (lt / N) as f32 + 0.5);
        if (lx - e.x).powi(2) + (ly - e.y).powi(2) > 20.0 {
            return; // no beach within reach: keep everyone aboard
        }
        for k in 0..e.ncargo as usize {
            let (a, r) = (k as f32 * 2.4, 0.45 * (k as f32).sqrt());
            let (mut x, mut y) = (lx + a.cos() * r, ly + a.sin() * r);
            if !self.map.pass[0][tile_of(x, y)] {
                x = lx;
                y = ly;
            }
            let u = self.spawn(e.cargo[k], e.team, x, y);
            if u != NOIDX {
                self.ents[u].hp = DEFS[e.cargo[k] as usize].hp * (e.cargo_hp[k] as f32 / 100.0).min(1.0);
            }
        }
        e.ncargo = 0;
    }

    fn do_board(&mut self, i: usize, e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let j = match self.get(e.otgt) {
            Some(j) if self.ents[j].team == e.team => j,
            _ => {
                Self::set_idle(e);
                return;
            }
        };
        let t = self.ents[j];
        if (t.x - e.x).powi(2) + (t.y - e.y).powi(2) < 2.8 * 2.8 {
            if (t.ncargo as usize) < CARGO_MAX {
                let tr = &mut self.ents[j];
                tr.cargo[tr.ncargo as usize] = e.def;
                tr.cargo_hp[tr.ncargo as usize] = (e.hp / d.hp * 100.0).clamp(1.0, 100.0) as u8;
                tr.ncargo += 1;
                e.hp = 0.0;
                e.stuck = BOARDED;
            } else {
                Self::set_idle(e);
            }
            return;
        }
        if (self.tick + i as u32) % 32 == 0 {
            e.fkey = self.key_for(LAND, t.x, t.y);
        }
        *want = self.steer(e, d, t.x, t.y);
        *moving = true;
    }

    /// Sound cue for the page; only things the local player can currently see.
    pub fn ev(&mut self, kind: f32, x: f32, y: f32) {
        if self.events.len() < 3 * 64 && self.vis[tile_of(x, y)] == 255 {
            self.events.extend_from_slice(&[kind, x, y]);
        }
    }

    /// Fingerprint of the simulation state, compared between multiplayer peers to catch desyncs.
    pub fn state_hash(&self) -> u32 {
        let mut h = 2_166_136_261u32;
        let mut mix = |v: u32| h = (h ^ v).wrapping_mul(16_777_619);
        mix(self.tick);
        mix(self.rng.0);
        for p in &self.players[..self.nplayers] {
            for r in p.res {
                mix(r.to_bits());
            }
            mix(p.epoch as u32 | p.tech << 8);
        }
        for e in &self.ents[..self.high] {
            if e.alive {
                mix(e.def as u32 | (e.team as u32) << 8 | (e.order as u32) << 16 | (e.ncargo as u32) << 24);
                mix(e.x.to_bits());
                mix(e.y.to_bits());
                mix(e.hp.to_bits());
            }
        }
        h
    }

    pub fn order_stop(&mut self, ids: &[usize]) {
        for &i in ids {
            let e = &mut self.ents[i];
            e.order = O_IDLE;
            e.ox = e.x;
            e.oy = e.y;
            e.tgt = NONE;
        }
    }

    // ------------------------------------------------------------------ tick

    pub fn tick(&mut self) {
        self.tick += 1;
        self.map.budget = 4;
        self.build_grid();
        for i in 0..self.high {
            if self.ents[i].alive {
                self.update_ent(i);
            }
        }
        self.separate();
        self.update_projs();
        let mut k = 0;
        while k < self.fx.len() {
            self.fx[k].age += DT;
            if self.fx[k].age >= self.fx[k].life {
                self.fx.swap_remove(k);
            } else {
                k += 1;
            }
        }
        for i in 0..self.high {
            if self.ents[i].alive && self.ents[i].hp <= 0.0 {
                self.kill(i);
            }
        }
        while self.high > 0 && !self.ents[self.high - 1].alive {
            self.high -= 1;
        }
        if self.tick % 4 == 0 && self.bench == 0 {
            self.update_fog();
        }
        if self.bench > 0 {
            if self.tick % 5 == 0 {
                self.bench_step();
            }
        } else {
            for t in 0..self.nplayers {
                if self.players[t].ai && (self.tick + t as u32 * 7) % 24 == 0 {
                    self.ai_step(t as u8);
                }
            }
            if self.tick % 20 == 0 && self.winner == 0 {
                for p in 0..self.nplayers {
                    self.players[p].alive = self.players[p].buildings > 0;
                }
                let me = self.local as usize;
                if !self.players[me].alive {
                    self.winner = 2;
                } else if (0..self.nplayers).all(|p| p == me || !self.players[p].alive) {
                    self.winner = 1;
                }
            }
        }
    }

    /// Counting-sort every live entity into a uniform grid; also recount population.
    pub fn build_grid(&mut self) {
        let g = &mut self.grid;
        for c in g.start.iter_mut() {
            *c = 0;
        }
        let mut pop = [0u32; 4];
        let mut cap = [0u32; 4];
        let mut bld = [0u32; 4];
        let mut own = [255u8; 32];
        let mut n = 0;
        for e in &self.ents[..self.high] {
            if !e.alive {
                continue;
            }
            n += 1;
            g.start[cell_of(e.x, e.y) + 1] += 1;
            let d = &DEFS[e.def as usize];
            let t = e.team as usize;
            pop[t] += d.pop as u32;
            for &c in &e.cargo[..e.ncargo as usize] {
                pop[t] += DEFS[c as usize].pop as u32;
            }
            if d.building {
                bld[t] += 1;
                if e.def == TC {
                    let k = self.map.terr[tile_of(e.x, e.y)] as usize;
                    if own[k] == 255 {
                        own[k] = e.team;
                    }
                }
                if e.built >= 1.0 {
                    cap[t] += match e.def {
                        TC => 15,
                        HOUSE => 10,
                        _ => 0,
                    };
                }
                for &q in &e.q[..e.qn as usize] {
                    if q < TECH0 {
                        pop[t] += DEFS[q as usize].pop as u32;
                    }
                }
            }
        }
        self.sim_units = n;
        for c in 0..GC * GC {
            g.start[c + 1] += g.start[c];
            g.cursor[c] = g.start[c];
        }
        g.items.clear();
        g.items.resize(n as usize, 0);
        for (i, e) in self.ents[..self.high].iter().enumerate() {
            if e.alive {
                let c = cell_of(e.x, e.y);
                g.items[g.cursor[c] as usize] = i as u16;
                g.cursor[c] += 1;
            }
        }
        for t in 0..4 {
            self.players[t].pop = pop[t];
            self.players[t].cap = cap[t].min(250);
            self.players[t].buildings = bld[t];
            self.players[t].terr = own.iter().filter(|&&o| o as usize == t).count() as u32;
        }
        if own != self.towner {
            self.towner = own;
            self.terr_dirty = true;
        }
    }

    /// Nearest hostile that `e` can see and shoot within `r`.
    fn acquire(&self, e: &Ent, d: &Def, r: f32) -> u32 {
        let mask = d.all_targets();
        let m = r + 2.5;
        let (cx0, cy0) = (((e.x - m) / CELL).max(0.0) as usize, ((e.y - m) / CELL).max(0.0) as usize);
        let (cx1, cy1) = ((((e.x + m) / CELL) as usize).min(GC - 1), (((e.y + m) / CELL) as usize).min(GC - 1));
        let mut best = NOIDX;
        let mut bs = f32::MAX;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * GC + cx;
                for k in self.grid.start[c]..self.grid.start[c + 1] {
                    let j = self.grid.items[k as usize] as usize;
                    let t = &self.ents[j];
                    if t.team == e.team {
                        continue;
                    }
                    let td = &DEFS[t.def as usize];
                    if mask & td.dom == 0 {
                        continue;
                    }
                    let (dx, dy) = (t.x - e.x, t.y - e.y);
                    let dist = (dx * dx + dy * dy).sqrt() - td.rad();
                    if dist > r {
                        continue;
                    }
                    // shoot things that shoot back first
                    let score = dist + if td.armed() { 0.0 } else if td.building { 6.0 } else { 3.0 };
                    if score < bs && self.can_see(e.team, j) {
                        bs = score;
                        best = j;
                    }
                }
            }
        }
        if best == NOIDX { NONE } else { self.h(best) }
    }

    /// Which teams have a detector close enough to spot stealth unit `e`.
    fn detect_mask(&self, e: &Ent) -> u8 {
        const R: f32 = 6.5;
        let (cx0, cy0) = (((e.x - R) / CELL).max(0.0) as usize, ((e.y - R) / CELL).max(0.0) as usize);
        let (cx1, cy1) = ((((e.x + R) / CELL) as usize).min(GC - 1), (((e.y + R) / CELL) as usize).min(GC - 1));
        let mut mask = 0u8;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * GC + cx;
                for k in self.grid.start[c]..self.grid.start[c + 1] {
                    let t = &self.ents[self.grid.items[k as usize] as usize];
                    if t.team != e.team && DEFS[t.def as usize].has(F_DETECT) && t.built >= 1.0 {
                        let (dx, dy) = (t.x - e.x, t.y - e.y);
                        if dx * dx + dy * dy < R * R {
                            mask |= 1 << t.team;
                        }
                    }
                }
            }
        }
        mask
    }

    fn fire(&mut self, e: &mut Ent, w: &Wpn, tj: usize) {
        e.cd = w.cd * (0.9 + 0.2 * self.rng.f());
        let d = &DEFS[e.def as usize];
        if d.has(F_STEALTH) {
            e.reveal_t = 3.0;
        }
        let dmg = w.dmg * self.players[e.team as usize].atk[class_of(d)];
        self.ev(if w.kind == W_MELEE { 7.0 } else { w.kind as f32 }, e.x, e.y);
        if w.kind == W_MELEE || self.projs.len() >= 4000 {
            self.damage(tj, dmg, w.kind);
            return;
        }
        let t = &self.ents[tj];
        let d0 = ((t.x - e.x).powi(2) + (t.y - e.y).powi(2)).sqrt().max(0.1);
        const SPEED: [f32; 7] = [0.0, 14.0, 34.0, 15.0, 6.5, 13.0, 7.0];
        self.projs.push(Proj { x: e.x, y: e.y, px: e.x, py: e.y, tx: t.x, ty: t.y, tgt: self.h(tj), speed: SPEED[w.kind as usize], dmg, kind: w.kind, team: e.team, splash: w.splash, targets: w.targets, d0 });
    }

    #[inline]
    fn damage(&mut self, j: usize, dmg: f32, kind: u8) {
        let (team, td) = (self.ents[j].team, &DEFS[self.ents[j].def as usize]);
        let m = MULT[kind as usize][td.armor as usize] * self.players[team as usize].dfn[class_of(td)];
        let t = &mut self.ents[j];
        t.hp -= dmg * m;
        t.flash = 0.12;
        if team == self.local && m > 0.0 && self.tick % 8 == 0 {
            let (x, y) = (t.x, t.y);
            self.ev(15.0, x, y);
        }
    }

    fn update_projs(&mut self) {
        let mut k = 0;
        while k < self.projs.len() {
            let mut p = self.projs[k];
            p.px = p.x;
            p.py = p.y;
            if p.kind != W_SHELL && p.kind != W_BOMB {
                if let Some(j) = self.get(p.tgt) {
                    p.tx = self.ents[j].x;
                    p.ty = self.ents[j].y;
                }
            }
            let (dx, dy) = (p.tx - p.x, p.ty - p.y);
            let dist = (dx * dx + dy * dy).sqrt();
            let step = p.speed * DT;
            if dist > step {
                p.x += dx / dist * step;
                p.y += dy / dist * step;
                self.projs[k] = p;
                k += 1;
                continue;
            }
            self.projs.swap_remove(k);
            if p.splash > 0.0 {
                let m = p.splash + 2.5;
                let (cx0, cy0) = (((p.tx - m) / CELL).max(0.0) as usize, ((p.ty - m) / CELL).max(0.0) as usize);
                let (cx1, cy1) = ((((p.tx + m) / CELL) as usize).min(GC - 1), (((p.ty + m) / CELL) as usize).min(GC - 1));
                for cy in cy0..=cy1 {
                    for cx in cx0..=cx1 {
                        let c = cy * GC + cx;
                        for q in self.grid.start[c]..self.grid.start[c + 1] {
                            let j = self.grid.items[q as usize] as usize;
                            let t = &self.ents[j];
                            let td = &DEFS[t.def as usize];
                            if t.team == p.team || td.dom & p.targets == 0 || !t.alive {
                                continue;
                            }
                            let d = ((t.x - p.tx).powi(2) + (t.y - p.ty).powi(2)).sqrt() - td.rad();
                            if d <= p.splash {
                                let f = 1.0 - 0.5 * (d / p.splash).clamp(0.0, 1.0);
                                self.damage(j, p.dmg * f, p.kind);
                            }
                        }
                    }
                }
            } else if let Some(j) = self.get(p.tgt) {
                self.damage(j, p.dmg, p.kind);
            }
            if p.kind >= W_SHELL && self.fx.len() < 1000 {
                self.fx.push(Fx { x: p.tx, y: p.ty, size: (p.splash * 2.0).max(0.7), age: 0.0, life: 0.2 + p.splash * 0.14 });
            }
        }
    }

    /// Direction to walk toward `(gx, gy)`: shared flow field when far, straight line when close.
    fn steer(&mut self, e: &mut Ent, d: &Def, gx: f32, gy: f32) -> (f32, f32) {
        let (dx, dy) = (gx - e.x, gy - e.y);
        let dist = (dx * dx + dy * dy).sqrt().max(1e-4);
        let direct = (dx / dist, dy / dist);
        if d.dom == AIR || dist < 1.5 {
            return direct;
        }
        let dc = if d.dom == LAND { 0 } else { 1 };
        let tile = tile_of(e.x, e.y);
        if !self.map.pass[dc][tile] {
            return direct;
        }
        // inside the formation disc around the field's goal: walk straight to our slot
        let gt = (e.fkey & 0xffff) as usize;
        let (fx, fy) = ((gt % N) as f32 + 0.5, (gt / N) as f32 + 0.5);
        let off = ((gx - fx).powi(2) + (gy - fy).powi(2)).sqrt();
        if off > 0.8 && off < 30.0 && ((e.x - fx).powi(2) + (e.y - fy).powi(2)).sqrt() < off + 2.5 {
            return direct;
        }
        let k = self.map.flow(&mut e.fslot, e.fkey, tile, self.tick) as usize;
        if k < 8 {
            let l = if k >= 4 { core::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
            (DIRS[k].0 as f32 * l, DIRS[k].1 as f32 * l)
        } else {
            direct
        }
    }

    fn try_move(&self, e: &mut Ent, d: &Def, nx: f32, ny: f32) {
        let nx = nx.clamp(0.6, N as f32 - 0.6);
        let ny = ny.clamp(0.6, N as f32 - 0.6);
        if d.dom == AIR {
            e.x = nx;
            e.y = ny;
            return;
        }
        let pass = &self.map.pass[if d.dom == LAND { 0 } else { 1 }];
        if !pass[tile_of(e.x, e.y)] {
            // standing somewhere illegal (e.g. a building was placed on us): walk out freely
            e.x = nx;
            e.y = ny;
            return;
        }
        if pass[tile_of(nx, e.y)] {
            e.x = nx;
        } else {
            e.vx = 0.0;
        }
        if pass[tile_of(e.x, ny)] {
            e.y = ny;
        } else {
            e.vy = 0.0;
        }
    }

    fn set_idle(e: &mut Ent) {
        e.order = O_IDLE;
        e.ox = e.x;
        e.oy = e.y;
        e.stuck = 0;
    }

    fn update_building(&mut self, i: usize, e: &mut Ent, d: &Def) {
        if e.def == FARM {
            e.carry = 0.0; // worker-slot counter, refilled by gatherers each tick
        }
        if e.built < 1.0 {
            return;
        }
        if e.qn > 0 {
            let item = e.q[0];
            let team = e.team as usize;
            let mut done = false;
            if item == RESEARCH {
                e.qt += DT;
                if e.qt >= EPOCH_TIME {
                    self.players[team].epoch += 1;
                    self.players[team].researching = false;
                    done = true;
                }
            } else if item >= TECH0 {
                e.qt += DT;
                let k = item - TECH0;
                if e.qt >= TECHS[k as usize].time {
                    let p = &mut self.players[team];
                    p.tech |= 1 << k;
                    p.tech_busy &= !(1 << k);
                    p.recompute();
                    done = true;
                }
            } else {
                let ud = &DEFS[item as usize];
                if e.qt < ud.time {
                    e.qt += DT;
                }
                // population was already reserved when queued; cap can still drop if houses die
                if e.qt >= ud.time && (self.players[team].pop <= self.players[team].cap || self.bench > 0) {
                    let (sx, sy) = self.spawn_point(e, d, ud);
                    let u = self.spawn(item, e.team, sx, sy);
                    if u != NOIDX && e.rx >= 0.0 {
                        let rt = tile_of(e.rx, e.ry);
                        if ud.has(F_WORKER) && self.map.res[rt] != R_NONE {
                            self.order_gather(&[u], rt);
                        } else {
                            self.order_move(&[u], e.rx, e.ry, false);
                        }
                    }
                    done = true;
                }
            }
            if done {
                if e.team == self.local {
                    self.ev(if item >= TECH0 { 13.0 } else { 12.0 }, e.x, e.y);
                }
                e.q.copy_within(1.., 0);
                e.qn -= 1;
                e.qt = 0.0;
            }
        }
        if d.armed() {
            let mut tj = NOIDX;
            if let Some(j) = self.get(e.tgt) {
                if self.can_see(e.team, j) {
                    tj = j;
                }
            }
            if tj == NOIDX && (self.tick + i as u32) % 8 == 0 {
                e.tgt = self.acquire(e, d, d.max_range());
                tj = self.get(e.tgt).unwrap_or(NOIDX);
            }
            if tj != NOIDX {
                let t = self.ents[tj];
                let td = &DEFS[t.def as usize];
                let dist = ((t.x - e.x).powi(2) + (t.y - e.y).powi(2)).sqrt() - td.rad();
                match d.weapon_vs(td.dom) {
                    Some(w) if dist <= w.range => {
                        if e.cd <= 0.0 {
                            let w = *w;
                            self.fire(e, &w, tj);
                        }
                    }
                    _ => e.tgt = NONE,
                }
            }
        }
    }

    fn spawn_point(&self, b: &Ent, bd: &Def, ud: &Def) -> (f32, f32) {
        if ud.dom == AIR {
            return (b.x, b.y);
        }
        let pass = &self.map.pass[if ud.dom == LAND { 0 } else { 1 }];
        let (cx, cy) = (b.x.floor() as i32, b.y.floor() as i32);
        let (wx, wy) = if b.rx >= 0.0 { (b.rx, b.ry) } else { (b.x, b.y + 50.0) };
        let r0 = (bd.size * 0.5) as i32 + 1;
        for r in r0..r0 + 6 {
            let mut best = None;
            let mut bdist = f32::MAX;
            for k in -r..=r {
                for &(x, y) in &[(cx + k, cy - r), (cx + k, cy + r), (cx - r, cy + k), (cx + r, cy + k)] {
                    if x < 1 || y < 1 || x >= N as i32 - 1 || y >= N as i32 - 1 || !pass[y as usize * N + x as usize] {
                        continue;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let dd = (fx - wx).powi(2) + (fy - wy).powi(2);
                    if dd < bdist {
                        bdist = dd;
                        best = Some((fx, fy));
                    }
                }
            }
            if let Some(p) = best {
                return p;
            }
        }
        (b.x, b.y)
    }

    fn update_ent(&mut self, i: usize) {
        let mut e = self.ents[i];
        let d = &DEFS[e.def as usize];
        e.px = e.x;
        e.py = e.y;
        e.prot = e.rot;
        if e.cd > 0.0 {
            e.cd -= DT;
        }
        if e.reveal_t > 0.0 {
            e.reveal_t -= DT;
        }
        if e.flash > 0.0 {
            e.flash -= DT;
        }
        if d.building {
            self.update_building(i, &mut e, d);
            self.ents[i] = e;
            return;
        }
        let stagger = (self.tick + i as u32) % 8 == 0;
        if d.has(F_STEALTH) && stagger {
            e.revealed = self.detect_mask(&e);
        }
        let plane = d.has(F_PLANE);

        // ---- resolve the current target
        if e.order == O_ATTACK {
            match self.get(e.otgt) {
                Some(j) if self.can_see(e.team, j) => {
                    e.tgt = e.otgt;
                    if (self.tick + i as u32) % 32 == 0 {
                        e.fkey = self.key_for(d.dom, self.ents[j].x, self.ents[j].y);
                    }
                }
                _ => {
                    Self::set_idle(&mut e);
                    e.tgt = NONE;
                    if plane {
                        e.ox = e.x + e.rot.cos() * 4.0;
                        e.oy = e.y + e.rot.sin() * 4.0;
                    }
                }
            }
        }
        let mut tj = NOIDX;
        if e.tgt != NONE {
            match self.get(e.tgt) {
                Some(j) if self.can_see(e.team, j) => tj = j,
                _ => e.tgt = NONE,
            }
        }
        if tj == NOIDX && stagger && d.armed() && e.order <= O_AMOVE {
            let r = if e.order == O_MOVE { d.max_range() } else { d.sight.max(d.max_range() + 1.0) };
            e.tgt = self.acquire(&e, d, r);
            tj = self.get(e.tgt).unwrap_or(NOIDX);
        }

        // ---- combat
        let mut want = (0.0f32, 0.0f32);
        let mut moving = false;
        let mut face = None;
        let mut engaged = false;
        let mut goal = (e.ox, e.oy);
        if tj != NOIDX {
            let t = self.ents[tj];
            let td = &DEFS[t.def as usize];
            if let Some(w) = d.weapon_vs(td.dom) {
                let w = *w;
                let (dx, dy) = (t.x - e.x, t.y - e.y);
                let dist = (dx * dx + dy * dy).sqrt();
                let ang = dy.atan2(dx);
                if dist - td.rad() <= w.range {
                    face = Some(ang);
                    engaged = e.order != O_MOVE;
                    goal = (t.x, t.y);
                    let aligned = !plane || w.kind == W_BOMB || angdiff(ang, e.rot).abs() < 0.5;
                    if e.cd <= 0.0 && aligned {
                        self.fire(&mut e, &w, tj);
                    }
                } else if e.order == O_MOVE {
                    e.tgt = NONE;
                } else {
                    let leash = if plane { 30.0 } else { 15.0 };
                    if e.order == O_IDLE && (e.x - e.ox).powi(2) + (e.y - e.oy).powi(2) > leash * leash {
                        // chased too far from our post: walk back
                        e.tgt = NONE;
                        e.order = O_MOVE;
                        e.fkey = self.key_for(d.dom, e.ox, e.oy);
                    } else {
                        engaged = true;
                        moving = true;
                        goal = (t.x, t.y);
                        want = if e.order == O_ATTACK { self.steer(&mut e, d, t.x, t.y) } else { (dx / dist.max(1e-4), dy / dist.max(1e-4)) };
                    }
                }
            } else {
                e.tgt = NONE;
            }
        }

        // ---- follow orders when not fighting
        if !engaged {
            match e.order {
                O_MOVE | O_AMOVE => {
                    let dist = ((e.ox - e.x).powi(2) + (e.oy - e.y).powi(2)).sqrt();
                    if plane {
                        if dist < 2.5 {
                            e.order = O_IDLE;
                            e.stuck = 0;
                        }
                    } else if dist < 0.25 {
                        Self::set_idle(&mut e);
                    } else {
                        let (ox, oy) = (e.ox, e.oy);
                        want = self.steer(&mut e, d, ox, oy);
                        moving = true;
                    }
                }
                O_UNLOAD => {
                    let dist = ((e.ox - e.x).powi(2) + (e.oy - e.y).powi(2)).sqrt();
                    if dist < 1.3 || e.stuck > 20 {
                        self.unload(&mut e);
                        Self::set_idle(&mut e);
                    } else {
                        let (ox, oy) = (e.ox, e.oy);
                        want = self.steer(&mut e, d, ox, oy);
                        moving = true;
                    }
                }
                O_BOARD => self.do_board(i, &mut e, d, &mut want, &mut moving),
                O_GATHER => self.do_gather(&mut e, d, &mut want, &mut moving),
                O_BUILD => self.do_build(&mut e, d, &mut want, &mut moving),
                _ => {}
            }
        }

        // ---- integrate
        if plane {
            let rate = 2.4 * DT;
            let (dx, dy) = (goal.0 - e.x, goal.1 - e.y);
            let dist = (dx * dx + dy * dy).sqrt();
            let diff = angdiff(dy.atan2(dx), e.rot);
            let rturn = d.speed / 2.4;
            if !engaged && e.order == O_IDLE && dist < 4.0 {
                e.rot += rate * 0.6; // loiter in a lazy circle
            } else {
                // target inside our turning circle: fly straight to open distance first
                if e.stuck == 0 && dist < rturn * 1.1 && diff.abs() > 0.9 {
                    e.stuck = 1;
                }
                if e.stuck == 1 && dist > rturn * 2.6 {
                    e.stuck = 0;
                }
                if e.stuck == 0 {
                    e.rot += diff.clamp(-rate, rate);
                }
            }
            e.rot = angdiff(e.rot, 0.0);
            e.vx = e.rot.cos() * d.speed;
            e.vy = e.rot.sin() * d.speed;
            e.x = (e.x + e.vx * DT).clamp(0.6, N as f32 - 0.6);
            e.y = (e.y + e.vy * DT).clamp(0.6, N as f32 - 0.6);
        } else {
            let (tvx, tvy) = if moving { (want.0 * d.speed, want.1 * d.speed) } else { (0.0, 0.0) };
            e.vx += (tvx - e.vx) * 0.4;
            e.vy += (tvy - e.vy) * 0.4;
            if e.vx != 0.0 || e.vy != 0.0 {
                let (nx, ny) = (e.x + e.vx * DT, e.y + e.vy * DT);
                self.try_move(&mut e, d, nx, ny);
            }
            match face {
                Some(a) if !moving => e.rot += angdiff(a, e.rot) * 0.35,
                _ => {
                    if e.vx * e.vx + e.vy * e.vy > 0.04 {
                        e.rot += angdiff(e.vy.atan2(e.vx), e.rot) * 0.3;
                    }
                }
            }
            if moving && (e.order == O_MOVE || e.order == O_AMOVE || e.order == O_UNLOAD) && !engaged {
                let moved = ((e.x - e.px).powi(2) + (e.y - e.py).powi(2)).sqrt();
                let near = (e.ox - e.x).powi(2) + (e.oy - e.y).powi(2) < 36.0;
                if moved < d.speed * DT * 0.25 && near {
                    e.stuck += 1;
                    if e.stuck > 24 {
                        Self::set_idle(&mut e); // goal is crowded or walled off: close enough
                    }
                } else {
                    e.stuck = 0;
                }
            }
        }
        self.ents[i] = e;
    }

    fn do_gather(&mut self, e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let kind = e.q[0];
        let (cx, cy, reach, farm) = if e.otgt != NONE {
            match self.get(e.otgt) {
                Some(j) if self.ents[j].built >= 1.0 => (self.ents[j].x, self.ents[j].y, DEFS[self.ents[j].def as usize].size * 0.5 + 0.95, j),
                _ => {
                    Self::set_idle(e);
                    return;
                }
            }
        } else {
            let mut t = e.otile as usize;
            if self.map.res[t] != kind {
                match self.map.nearest_res(t, kind, 14) {
                    Some(nt) => {
                        t = nt;
                        e.otile = nt as u32;
                        e.fkey = self.key_for(LAND, (nt % N) as f32 + 0.5, (nt / N) as f32 + 0.5);
                    }
                    None => {
                        Self::set_idle(e);
                        return;
                    }
                }
            }
            ((t % N) as f32 + 0.5, (t / N) as f32 + 0.5, 1.45, NOIDX)
        };
        if (cx - e.x).abs().max((cy - e.y).abs()) > reach {
            *want = self.steer(e, d, cx, cy);
            *moving = true;
            return;
        }
        let p = &mut self.players[e.team as usize];
        let amount = 0.55 * (1.0 + 0.1 * p.epoch as f32) * p.gather * DT;
        if farm != NOIDX {
            // a farm supports four workers; extras wait their turn
            if self.ents[farm].carry < 4.0 {
                self.ents[farm].carry += 1.0;
                p.res[0] += amount;
            }
            return;
        }
        p.res[kind as usize - 1] += amount;
        e.carry += amount;
        if e.carry >= 1.0 {
            e.carry -= 1.0;
            let t = e.otile as usize;
            self.map.amt[t] = self.map.amt[t].saturating_sub(1);
            if self.map.amt[t] == 0 {
                self.map.res[t] = R_NONE;
                if self.map.ter[t] == T_FOREST {
                    self.map.ter[t] = T_GRASS;
                }
                self.map.recolor(t);
            }
        }
    }

    fn do_build(&mut self, e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let j = match self.get(e.otgt) {
            Some(j) => j,
            None => {
                Self::set_idle(e);
                return;
            }
        };
        let b = self.ents[j];
        let bd = &DEFS[b.def as usize];
        if b.built >= 1.0 {
            if b.def == FARM {
                e.order = O_GATHER;
                e.q[0] = R_FOOD;
            } else if e.prev != R_NONE {
                // job done: go back to the resource we were working before
                let t = e.otile as usize;
                e.order = O_GATHER;
                e.otgt = NONE;
                e.q[0] = e.prev;
                e.fkey = self.key_for(LAND, (t % N) as f32 + 0.5, (t / N) as f32 + 0.5);
            } else {
                Self::set_idle(e);
            }
            return;
        }
        if (b.x - e.x).abs().max((b.y - e.y).abs()) > bd.size * 0.5 + 0.95 {
            *want = self.steer(e, d, b.x, b.y);
            *moving = true;
            return;
        }
        let step = DT / bd.time;
        let t = &mut self.ents[j];
        t.built = (t.built + step).min(1.0);
        t.hp = (t.hp + bd.hp * 0.9 * step).min(bd.hp);
        if t.built >= 1.0 && t.team == self.local {
            self.ev(14.0, b.x, b.y);
        }
    }

    /// Soft collision: overlapping units in the same layer push each other apart.
    fn separate(&mut self) {
        for i in 0..self.high {
            let e = &self.ents[i];
            self.push[i] = (0.0, 0.0);
            if !e.alive {
                continue;
            }
            let d = &DEFS[e.def as usize];
            if d.building || d.has(F_PLANE) {
                continue;
            }
            let r = d.rad();
            let layer = if d.dom == AIR { AIR } else { !AIR };
            let m = r + 1.3;
            let (cx0, cy0) = (((e.x - m) / CELL).max(0.0) as usize, ((e.y - m) / CELL).max(0.0) as usize);
            let (cx1, cy1) = ((((e.x + m) / CELL) as usize).min(GC - 1), (((e.y + m) / CELL) as usize).min(GC - 1));
            let (mut px, mut py) = (0.0f32, 0.0f32);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    let c = cy * GC + cx;
                    for k in self.grid.start[c]..self.grid.start[c + 1] {
                        let j = self.grid.items[k as usize] as usize;
                        if j == i {
                            continue;
                        }
                        let o = &self.ents[j];
                        let od = &DEFS[o.def as usize];
                        if od.building || od.has(F_PLANE) || od.dom & layer == 0 {
                            continue;
                        }
                        // grid positions are from the start of the tick, which is fine for a soft push
                        let (dx, dy) = (e.x - o.x, e.y - o.y);
                        let min = r + od.rad();
                        let d2 = dx * dx + dy * dy;
                        if d2 < min * min {
                            if d2 > 1e-6 {
                                let dist = d2.sqrt();
                                let f = (min - dist) / dist * 0.5;
                                px += dx * f;
                                py += dy * f;
                            } else {
                                // exactly stacked: split deterministically
                                let a = (i as f32) * 2.4;
                                px += a.cos() * 0.05;
                                py += a.sin() * 0.05;
                            }
                        }
                    }
                }
            }
            self.push[i] = (px, py);
        }
        for i in 0..self.high {
            let (px, py) = self.push[i];
            if px == 0.0 && py == 0.0 {
                continue;
            }
            let mut e = self.ents[i];
            let d = &DEFS[e.def as usize];
            let l = (px * px + py * py).sqrt();
            let s = (0.6f32).min(0.14 / l); // cap the correction per tick so crowds flow instead of popping
            let (ovx, ovy) = (e.vx, e.vy);
            let (nx, ny) = (e.x + px * s, e.y + py * s);
            self.try_move(&mut e, d, nx, ny);
            e.vx = ovx;
            e.vy = ovy;
            self.ents[i] = e;
        }
    }

    pub fn update_fog(&mut self) {
        if self.bench > 0 {
            return;
        }
        for v in self.vis.iter_mut() {
            if *v == 255 {
                *v = 128;
            }
        }
        for e in &self.ents[..self.high] {
            if !e.alive || e.team != self.local {
                continue;
            }
            let r = DEFS[e.def as usize].sight;
            let (cx, cy) = (e.x as i32, e.y as i32);
            let ri = r as i32;
            for dy in -ri..=ri {
                let y = cy + dy;
                if y < 0 || y >= N as i32 {
                    continue;
                }
                let w = ((r * r - (dy * dy) as f32).max(0.0)).sqrt() as i32;
                let x0 = (cx - w).max(0) as usize;
                let x1 = (cx + w).min(N as i32 - 1) as usize;
                let row = y as usize * N;
                for v in &mut self.vis[row + x0..=row + x1] {
                    *v = 255;
                }
            }
        }
    }

    /// Stress test: keep two huge mixed armies topped up and marching at each other.
    fn bench_step(&mut self) {
        let target = self.bench as usize;
        let mut count = [0usize; 2];
        for e in &self.ents[..self.high] {
            if e.alive && !DEFS[e.def as usize].building && (e.team as usize) < 2 && !DEFS[e.def as usize].has(F_WORKER) {
                count[e.team as usize] += 1;
            }
        }
        let c = N as f32 * 0.5;
        for team in 0..2usize {
            let (hx, ex) = if team == 0 { (c - 34.0, c + 34.0) } else { (c + 34.0, c - 34.0) };
            let mut spawned = 0;
            let mut tries = 0;
            self.scratch.clear();
            while count[team] + spawned < target && spawned < 60 && tries < 400 {
                tries += 1;
                let x = hx + (self.rng.f() - 0.5) * 50.0;
                let y = c + (self.rng.f() - 0.5) * 110.0;
                let t = tile_of(x, y);
                let roll = self.rng.f();
                let want = if roll < 0.12 { AIR } else if self.map.pass[0][t] { LAND } else if self.map.pass[1][t] { SEA } else { 0 };
                if want == 0 {
                    continue;
                }
                // pick a random late-game unit of the matching domain
                let mut def = 0u8;
                for _ in 0..40 {
                    let k = WORKER as u32 + 1 + self.rng.range(DEFS.len() as u32 - WORKER as u32 - 1);
                    let dd = &DEFS[k as usize];
                    let dom = if dd.dom == SUB { SEA } else { dd.dom };
                    if dom == want && dd.epoch >= 4 {
                        def = k as u8;
                        break;
                    }
                }
                if def == 0 {
                    continue;
                }
                let u = self.spawn(def, team as u8, x, y);
                if u == NOIDX {
                    break;
                }
                self.scratch.push(u);
                spawned += 1;
            }
            if !self.scratch.is_empty() {
                let ids = std::mem::take(&mut self.scratch);
                // several rally points so the front line is wide rather than one blob
                for (k, chunk) in ids.chunks(12).enumerate() {
                    let y = c + ((k as f32 * 0.37 + self.rng.f()) % 1.0 - 0.5) * 90.0;
                    self.order_move(chunk, ex, y, true);
                }
                self.scratch = ids;
            }
        }
    }
}
