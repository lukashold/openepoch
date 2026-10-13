//! A straightforward scripted opponent: grow the economy, tech up, send waves.

use crate::defs::*;
use crate::game::*;
use crate::map::*;

impl Game {
    /// Find a free site near `(bx, by)` and start construction with a few workers.
    fn ai_build(&mut self, team: u8, def: u8, bx: i32, by: i32, builders: &[usize]) -> bool {
        let d = &DEFS[def as usize];
        if !self.afford(team, &d.cost) {
            return false;
        }
        let s = d.size as i32;
        let (r0, r1) = if def == DOCK { (4, 48) } else if def == TC { (0, 16) } else { (5, 26) };
        let start = self.rng.range(8) as i32;
        for r in r0..r1 {
            for k in (-r..=r).step_by(2) {
                let k = ((k + r + start) % (2 * r + 1)) - r;
                for &(tx, ty) in &[(bx + k, by - r), (bx + k, by + r), (bx - r, by + k), (bx + r, by + k)] {
                    // keep a one-tile lane around land buildings so bases stay walkable
                    let ok = if def == DOCK {
                        self.can_place(def, tx, ty, team)
                    } else {
                        self.can_place(def, tx, ty, team) && self.lane_free(tx - 1, ty - 1, s + 2)
                    };
                    if ok {
                        let b = self.place(def, tx, ty, team);
                        if b != NOIDX {
                            self.order_building(builders, b);
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn lane_free(&self, tx: i32, ty: i32, s: i32) -> bool {
        if tx < 1 || ty < 1 || tx + s >= N as i32 || ty + s >= N as i32 {
            return false;
        }
        for y in ty..ty + s {
            for x in tx..tx + s {
                if !self.map.pass[0][y as usize * N + x as usize] {
                    return false;
                }
            }
        }
        true
    }

    pub fn ai_step(&mut self, team: u8) {
        let ti = team as usize;
        if self.players[ti].buildings == 0 {
            return;
        }
        // ---- census
        let mut count = [0u32; 64];
        let mut tc = NOIDX;
        let mut any_b = NOIDX;
        let mut idle: Vec<usize> = Vec::new();
        let mut workers: Vec<usize> = Vec::new();
        let mut army: Vec<usize> = Vec::new();
        let mut prod: Vec<usize> = Vec::new();
        let mut finished: Vec<usize> = Vec::new();
        let mut farms: Vec<(usize, u32)> = Vec::new();
        let mut unfinished: Vec<usize> = Vec::new();
        let mut gath = [0u32; 4];
        let mut builders = 0;
        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || e.team != team {
                continue;
            }
            let d = &DEFS[e.def as usize];
            count[e.def as usize] += 1;
            if d.building {
                any_b = i;
                if e.built < 1.0 {
                    unfinished.push(i);
                } else {
                    finished.push(i);
                    match e.def {
                        TC => tc = i,
                        FARM => farms.push((i, 0)),
                        _ => {}
                    }
                    if matches!(e.def, BARRACKS | WORKSHOP | DOCK | AIRFIELD) {
                        prod.push(i);
                    }
                }
            } else if d.has(F_WORKER) {
                workers.push(i);
                match e.order {
                    O_GATHER => gath[e.q[0] as usize] += 1,
                    O_BUILD => builders += 1,
                    _ => idle.push(i),
                }
            } else {
                army.push(i);
            }
        }
        for &w in &workers {
            let e = &self.ents[w];
            if e.order == O_GATHER && e.otgt != NONE {
                if let Some(f) = farms.iter_mut().find(|f| self.h(f.0) == e.otgt) {
                    f.1 += 1;
                }
            }
        }
        let base = if tc != NOIDX { tc } else { any_b };
        let (bx, by) = (self.ents[base].x, self.ents[base].y);
        let (bxi, byi) = (bx as i32, by as i32);
        let btile = tile_of(bx, by);
        let ep = self.players[ti].epoch;
        let p = self.players[ti];

        // ---- keep construction staffed
        if builders < 2 {
            if let Some(&u) = unfinished.first() {
                let mut crew: Vec<usize> = idle.drain(..idle.len().min(2)).collect();
                if crew.is_empty() {
                    crew.extend(workers.iter().copied().filter(|&w| self.ents[w].order == O_GATHER).take(2));
                }
                self.order_building(&crew, u);
            }
        }

        // ---- workers
        let want_workers = (12 + 4 * ep as u32).min(32);
        if tc != NOIDX && (workers.len() as u32) < want_workers && self.ents[tc].qn < 2 && p.pop < p.cap {
            self.train(tc, WORKER);
        }
        let weights = [0.0, 4.0, 3.0, 1.0 + ep as f32 * 0.9];
        for &w in &idle {
            let mut kind = R_FOOD;
            let mut low = f32::MAX;
            for k in 1..4 {
                let v = gath[k] as f32 / weights[k];
                if v < low {
                    low = v;
                    kind = k as u8;
                }
            }
            gath[kind as usize] += 1;
            if kind == R_FOOD {
                if let Some(f) = farms.iter_mut().find(|f| f.1 < 4) {
                    f.1 += 1;
                    self.order_building(&[w], f.0);
                    continue;
                }
            }
            let found = self.map.nearest_res(btile, kind, 60).or_else(|| self.map.nearest_res(btile, R_WOOD, 80));
            if let Some(t) = found {
                self.order_gather(&[w], t);
            }
        }

        // ---- construction (one project at a time)
        if unfinished.is_empty() && !workers.is_empty() {
            let crew: Vec<usize> = workers.iter().copied().filter(|&w| self.ents[w].order != O_BUILD).take(3).collect();
            let c = |d: u8| count[d as usize];
            let farm_slots = farms.iter().filter(|f| f.1 < 4).count();
            let choice = if p.cap < 250 && p.cap < p.pop + 5 {
                Some(HOUSE)
            } else if c(BARRACKS) == 0 && workers.len() >= 7 {
                Some(BARRACKS)
            } else if farm_slots == 0 && c(FARM) < 3 + ep as u32 {
                Some(FARM)
            } else if ep >= 1 && c(WORKSHOP) == 0 {
                Some(WORKSHOP)
            } else if c(TOWER) < 1 + ep as u32 / 2 {
                Some(TOWER)
            } else if ep >= 1 && c(DOCK) == 0 && !p.ai_nodock {
                Some(DOCK)
            } else if ep >= 4 && c(AIRFIELD) == 0 {
                Some(AIRFIELD)
            } else if ep >= 2 && c(BARRACKS) < 2 {
                Some(BARRACKS)
            } else if ep >= 2 && c(TC) < 1 + ep as u32 / 2 && !p.ai_noexp {
                Some(TC)
            } else if ep >= 4 && c(WORKSHOP) < 2 {
                Some(WORKSHOP)
            } else {
                None
            };
            if let Some(def) = choice {
                // a new town center goes into the nearest unclaimed territory
                let mut at = (bxi, byi);
                if def == TC {
                    let mut bd = f32::MAX;
                    for (k, s) in self.map.tseeds.iter().enumerate() {
                        let dd = (s.0 - bx).powi(2) + (s.1 - by).powi(2);
                        if self.towner[k] == 255 && self.map.pass[0][tile_of(s.0, s.1)] && dd < bd {
                            bd = dd;
                            at = (s.0 as i32, s.1 as i32);
                        }
                    }
                    if bd == f32::MAX {
                        self.players[ti].ai_noexp = true;
                    }
                }
                let ok = self.ai_build(team, def, at.0, at.1, &crew);
                if !ok && self.afford(team, &DEFS[def as usize].cost) {
                    match def {
                        DOCK => self.players[ti].ai_nodock = true, // landlocked
                        TC => self.players[ti].ai_noexp = true,
                        _ => {}
                    }
                }
            }
        }

        // ---- research & army
        let wave = 8 + 5 * ep as usize;
        let saving = ep < MAX_EPOCH && !p.researching && workers.len() >= 12 && army.len() >= wave * 2 / 3;
        if saving && tc != NOIDX {
            self.train(tc, RESEARCH);
        }
        let cap_army = 24 + 12 * ep as usize;
        if !saving && army.len() < cap_army && self.players[ti].pop + 3 <= self.players[ti].cap {
            for &b in &prod {
                if self.ents[b].qn > 0 {
                    continue;
                }
                let bdef = self.ents[b].def;
                // prefer the newest unit this building can make, with a little variety
                let mut pick = 0u8;
                let mut seen = 0;
                for (k, d) in DEFS.iter().enumerate() {
                    if d.made_at == bdef && d.epoch <= ep && d.until > ep && d.armed() {
                        seen += 1;
                        if self.rng.range(seen) == 0 {
                            pick = k as u8;
                        }
                    }
                }
                if pick != 0 {
                    self.train(b, pick);
                }
            }
        }

        // ---- technologies: buy them once there is money to spare
        if !saving {
            for (k, t) in TECHS.iter().enumerate() {
                let r = self.players[ti].res;
                let rich = (0..3).all(|c| r[c] >= t.cost[c] as f32 * 1.6 + 100.0);
                if rich && t.epoch <= ep && (self.players[ti].tech | self.players[ti].tech_busy) >> k & 1 == 0 {
                    if let Some(&b) = finished.iter().find(|&&b| self.ents[b].def == t.at && self.ents[b].qn < 2) {
                        self.train(b, TECH0 + k as u8);
                        break;
                    }
                }
            }
        }

        // ---- attack waves
        if army.len() >= wave && self.tick >= self.players[ti].ai_next_attack {
            let mut best = NOIDX;
            let mut bd = f32::MAX;
            for i in 0..self.high {
                let e = &self.ents[i];
                if e.alive && e.team != team && DEFS[e.def as usize].building {
                    let dd = (e.x - bx).powi(2) + (e.y - by).powi(2);
                    if dd < bd {
                        bd = dd;
                        best = i;
                    }
                }
            }
            if best != NOIDX {
                let (tx, ty) = (self.ents[best].x, self.ents[best].y);
                self.order_move(&army, tx, ty, true);
                self.players[ti].ai_next_attack = self.tick + 20 * 100;
            }
        }
    }
}
