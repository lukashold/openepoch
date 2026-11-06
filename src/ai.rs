//! A straightforward scripted opponent: grow the economy, tech up, send waves.

use crate::defs::*;
use crate::game::*;
use crate::map::*;

impl Game {
    /// Find a free site near `(bx, by)` and start construction with a few workers.
    fn ai_build(&mut self, team: u8, def:DefId, bx: i32, by: i32, builders: &[usize]) -> bool {
        let d = &DEFS[def as usize];
        if d.epoch>self.players[team as usize].epoch{return false;}
        if !self.afford(team, &self.unit_cost(team,def)) {
            return false;
        }
        let s = d.size as i32;
        let wanted_territory=self.map.terr[tile_of(bx as f32,by as f32)];
        let (r0, r1) = if def == DOCK { (4, 48) } else if def == TC { (0, 16) } else { (5, 26) };
        let start = self.rng.range(8) as i32;
        for r in r0..r1 {
            for k in (-r..=r).step_by(2) {
                let k = ((k + r + start) % (2 * r + 1)) - r;
                for &(tx, ty) in &[(bx + k, by - r), (bx + k, by + r), (bx - r, by + k), (bx + r, by + k)] {
                    if cfg!(feature="ee2-reference") && matches!(def,TC|FORTRESS) && self.map.terr[tile_of(tx as f32+s as f32*0.5,ty as f32+s as f32*0.5)]!=wanted_territory {continue;}
                    // keep a one-tile lane around land buildings so bases stay walkable
                    let ok = if def == DOCK {
                        self.can_place(def, tx, ty, team)
                    } else {
                        self.can_place(def, tx, ty, team) && self.lane_free(tx - 1, ty - 1, s + 2)
                    };
                    if ok {
                        let turn=if def==DOCK{self.dock_auto_turn_for(team,tx,ty)}else{0};
                        let b = self.begin_construction(team,def,tx,ty,turn,builders,false);
                        if b != NOIDX {
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
        #[cfg(feature="ee2-civ8")]
        let mut count = [0u32; DEFS.len()];
        #[cfg(not(feature="ee2-civ8"))]
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
        let mut gath = [0u32; 10];
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
                    if matches!(e.def, BARRACKS | WORKSHOP | DOCK | AIRFIELD | STABLE | MILL | HERC_FACILITY) {
                        prod.push(i);
                    }
                }
            } else if d.has(F_WORKER) {
                workers.push(i);
                match e.order {
                    O_GATHER => gath[e.q[0] as usize] += 1,
                    O_BUILD => builders += 1,
                    O_BOARD => {},
                    _ => idle.push(i),
                }
            } else {
                #[cfg(feature="ee2-native13")]
                if e.def == crate::ee2_regicide::KING { continue; }
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
        if cfg!(feature = "ee2-reference") && workers.len()>8 {
            for &b in &finished {
                if self.ents[b].def != UNIVERSITY {continue;}
                let slots=self.cargo_capacity(team,UNIVERSITY).saturating_sub(self.ents[b].ncargo as usize);
                let available=workers.iter().filter(|&&w|!matches!(self.ents[w].order,O_BUILD|O_BOARD)).count();
                let crew:Vec<usize>=workers.iter().copied().filter(|&w| !matches!(self.ents[w].order,O_BUILD|O_BOARD)).take(slots.min(3).min(available.saturating_sub(12))).collect();
                self.order_board(&crew,b);
            }
            idle.retain(|&w| self.ents[w].order!=O_BOARD);
        }
        if cfg!(feature="ee2-reference")&&workers.len()>12{
            for &b in &finished{
                if !DEFS[self.ents[b].def as usize].has(F_EXTRACT){continue;}
                let slots=self.cargo_capacity(team,self.ents[b].def).saturating_sub(self.ents[b].ncargo as usize);
                let available=workers.iter().filter(|&&w|!matches!(self.ents[w].order,O_BUILD|O_BOARD)).count();
                let crew:Vec<usize>=workers.iter().copied().filter(|&w|!matches!(self.ents[w].order,O_BUILD|O_BOARD)).take(slots.min(6).min(available.saturating_sub(12))).collect();
                self.order_board(&crew,b);
            }
            idle.retain(|&w|self.ents[w].order!=O_BOARD);
        }

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
        let want_workers = ((if cfg!(feature="ee2-reference"){18}else{12}) + 6 * ep as u32).min((p.population_limit / 6).clamp(24, 60));
        if tc != NOIDX && (workers.len() as u32) < want_workers && self.ents[tc].qn < 2 && p.pop < p.cap {
            self.train(tc, WORKER as QueueItem);
        }
        let weights = [0.0,4.0,3.0,1.0+ep as f32*0.9,3.0,2.5,2.5,2.5,0.0,0.0];
        for &w in &idle {
            let mut kind = R_FOOD;
            let mut low = f32::MAX;
            for k in 1..=crate::economy::RESOURCE_COUNT {
                if !self.citizen_can_gather(team,k as u8){continue;}
                let v = gath[k] as f32 / weights[k] * (1.0+self.resource(team,k-1)/800.0);
                if v < low {
                    low = v;
                    kind = k as u8;
                }
            }
            gath[kind as usize] += 1;
            if kind == R_FOOD {
                if let Some(f) = farms.iter_mut().find(|f| f.1 < if cfg!(feature="ee2-reference"){1}else{4}) {
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
            let crew: Vec<usize> = workers.iter().copied().filter(|&w| !matches!(self.ents[w].order,O_BUILD|O_BOARD)).take(3).collect();
            let c = |d: DefId| count[d as usize];
            let farm_slots = farms.iter().filter(|f| f.1 < if cfg!(feature="ee2-reference"){1}else{4}).count();
            let pending_fort=if cfg!(feature="ee2-reference") {finished.iter().copied().find(|&b| {
                let e=self.ents[b];let id=self.map.terr[tile_of(e.x,e.y)] as usize;
                e.def==TC && self.towner[id]==255 && !self.territory_adjacent_owned(team,id)
                    && self.territory_control_counts(team,id)[1]==0
            })}else{None};
            let choice = if p.cap < p.population_limit && p.cap < p.pop + 10 {
                Some(HOUSE)
            } else if pending_fort.is_some() {
                Some(FORTRESS)
            } else if cfg!(feature="ee2-reference")&&ep>=9&&c(OIL_RIG)==0&&workers.len()>=18{
                Some(OIL_RIG)
            } else if cfg!(feature="ee2-reference")&&ep>=12&&c(URANIUM_MINE)==0&&workers.len()>=18{
                Some(URANIUM_MINE)
            } else if c(BARRACKS) == 0 && workers.len() >= 7 {
                Some(BARRACKS)
            } else if cfg!(feature = "ee2-reference") && c(UNIVERSITY) < 1 + ep as u32 / 3 && workers.len() >= 15 {
                Some(UNIVERSITY)
            } else if cfg!(feature = "ee2-reference") && c(UNIVERSITY)>0 && c(WAREHOUSE)==0 {
                Some(WAREHOUSE)
            } else if cfg!(feature = "ee2-reference") && c(UNIVERSITY)>0 && c(MILL)==0 {
                Some(MILL)
            } else if cfg!(feature = "ee2-reference") && ep>=DEFS[STABLE as usize].epoch && c(STABLE)==0 {
                Some(STABLE)
            } else if cfg!(feature = "ee2-reference") && ep>=DEFS[TEMPLE as usize].epoch && c(TEMPLE)==0 {
                Some(TEMPLE)
            } else if cfg!(feature = "ee2-reference") && ep>=DEFS[MARKET as usize].epoch && c(MARKET)==0 {
                Some(MARKET)
            } else if ep>=DEFS[FARM as usize].epoch&&farm_slots == 0 && c(FARM) < if cfg!(feature="ee2-reference"){(workers.len() as u32/4+1).min(8)}else{3+ep as u32} && (!cfg!(feature="ee2-reference")||p.res[0]<250.0) {
                Some(FARM)
            } else if ep >= 1 && c(WORKSHOP) == 0 {
                Some(WORKSHOP)
            } else if c(TOWER) < 1 + ep as u32 / 2 {
                Some(TOWER)
            } else if cfg!(feature="ee2-reference")&&workers.len()>=14&&c(FORTRESS)==0 {
                Some(FORTRESS)
            } else if ep >= 1 && c(DOCK) == 0 && !p.ai_nodock {
                Some(DOCK)
            } else if ep >= 4 && c(AIRFIELD) == 0 {
                Some(AIRFIELD)
            } else if cfg!(feature="ee2-reference")&&ep>=DEFS[HERC_FACILITY as usize].epoch&&c(HERC_FACILITY)==0{
                Some(HERC_FACILITY)
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
                if def==FORTRESS {if let Some(b)=pending_fort {at=(self.ents[b].x as i32,self.ents[b].y as i32);}}
                if def == TC {
                    let mut bd = f32::MAX;
                    let mut best_adjacent=false;
                    for (k, s) in self.map.tseeds.iter().enumerate() {
                        let dd = (s.0 - bx).powi(2) + (s.1 - by).powi(2);
                        let adjacent=cfg!(feature="ee2-reference")&&self.territory_adjacent_owned(team,k);
                        let occupied=self.ents[..self.high].iter().any(|e|e.alive&&e.hp>0.0&&e.def==TC&&self.map.terr[tile_of(e.x,e.y)] as usize==k);
                        if self.towner[k] == 255 && !occupied && self.map.pass[0][tile_of(s.0, s.1)] && ((adjacent&&!best_adjacent)||adjacent==best_adjacent&&dd<bd) {
                            bd = dd;
                            best_adjacent=adjacent;
                            at = (s.0 as i32, s.1 as i32);
                        }
                    }
                    if bd == f32::MAX {
                        self.players[ti].ai_noexp = true;
                    }
                }
                let ok = self.ai_build(team, def, at.0, at.1, &crew);
                if !ok && self.afford(team, &self.unit_cost(team,def)) {
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
        let saving = ep < self.match_final_epoch() && !p.researching && workers.len() >= 12 && (cfg!(feature="ee2-reference")||army.len() >= wave * 2 / 3) && (!cfg!(feature = "ee2-reference") || self.can_advance(team));
        if saving && tc != NOIDX {
            self.train(tc, RESEARCH);
        }
        let cap_army = (24 + 20 * ep as usize).min(p.population_limit as usize / 2);
        if !saving && army.len() < cap_army && self.players[ti].pop + 3 <= self.players[ti].cap && (!cfg!(feature="ee2-reference")||workers.len()>=14) {
            for &b in &prod {
                if self.ents[b].qn > 0 {
                    continue;
                }
                let bdef = self.ents[b].def;
                // prefer the newest unit this building can make, with a little variety
                let mut pick = 0 as DefId;
                let mut seen = 0;
                for (k, d) in DEFS.iter().enumerate() {
                    let available = d.made_at == bdef && d.epoch <= ep && d.until > ep;
                    #[cfg(feature="ee2-native13")]
                    let available = if bdef == BARRACKS && self.infantry_roster_enabled {
                        self.unit_production_available(team, bdef, k as DefId)
                    } else { available };
                    if available && d.armed() {
                        // Own scripted AI: sample only jobs that this producer
                        // can legally buy. Rejected civilization/price/population
                        // choices must not consume its reservoir RNG draw.
                        #[cfg(feature="ee2-civ8")]
                        if !self.can_train(b,k as QueueItem) {continue;}
                        seen += 1;
                        if self.rng.range(seen) == 0 {
                            pick = k as DefId;
                        }
                    }
                }
                if pick != 0 {
                    self.train(b, pick as QueueItem);
                }
            }
        }

        // ---- technologies: buy them once there is money to spare
        #[cfg(not(feature = "ee2-reference"))]
        if !saving {
            for (k, t) in TECHS.iter().enumerate() {
                let r = self.players[ti].res;
                let rich = (0..3).all(|c| r[c] >= t.cost[c] as f32 * 1.6 + 100.0);
                if rich && t.epoch <= ep && (self.players[ti].tech | self.players[ti].tech_busy) >> k & 1 == 0 {
                    if let Some(&b) = finished.iter().find(|&&b| self.ents[b].def == t.at && self.ents[b].qn < 2) {
                        self.train(b, TECH0 + k as QueueItem);
                        break;
                    }
                }
            }
        }
        #[cfg(feature = "ee2-reference")]
        if !p.researching {
            for (k,n) in crate::ee2_research::NODES.iter().enumerate() {
                if n.epoch!=ep || self.has_research(team,k) {continue;}
                if self.research_count(team,ep)>=6 {break;}
                if let Some(&b)=finished.iter().find(|&&b| self.ents[b].def==n.host && self.ents[b].qn<2 && self.can_train(b,ORIGINAL_RESEARCH0+k as QueueItem)) {
                    self.train(b,ORIGINAL_RESEARCH0+k as QueueItem);break;
                }
            }
        }

        // ---- attack waves
        if army.len() >= wave && self.tick >= self.players[ti].ai_next_attack {
            let mut best = NOIDX;
            let mut bd = f32::MAX;
            for i in 0..self.high {
                let e = &self.ents[i];
                if e.alive && self.hostile(team,e.team) && self.can_see(team,i) && DEFS[e.def as usize].building {
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
            } else if (0..self.nplayers).any(|p|self.players[p].alive&&self.hostile(team,p as u8)) {
                // Scout deterministic map sectors, never hidden enemy entity
                // coordinates. Real sight then permits a targeted attack wave.
                let k=(self.tick/400+team as u32*5+self.seed0)%16;
                self.order_move(&army,32.0+(k%4) as f32*64.0,32.0+(k/4) as f32*64.0,true);
                self.players[ti].ai_next_attack=self.tick+400;
            }
        }
    }
}

#[cfg(all(test,feature="ee2-civ8"))]
mod civ8_ai_tests {
    use super::*;

    // A controlled own-AI fixture: unfinished construction suppresses the
    // unrelated building-site RNG; workers have active construction orders.
    fn fixture(civ:u8,epoch:u8,host:DefId)->(Game,usize) {
        let mut g=Game::new(7,1,0,1,0);
        assert!(g.configure_civilizations([14,civ,14,14]));
        for i in 0..g.high {if g.ents[i].team==1 {g.ents[i].alive=false;}}
        g.players[1].epoch=epoch;g.rebuild_modifiers(1,false);
        g.players[1].buildings=2;g.players[1].population_limit=1000;
        g.players[1].pop=14;g.players[1].cap=1000;g.players[1].researching=true;
        for slot in 0..9 {let old=g.resource(1,slot);g.add_resource(1,slot,60000.0-old);}
        let b=g.spawn(host,1,80.5,80.5);g.ents[b].built=1.0;
        let site=g.spawn(HOUSE,1,90.5,90.5);g.ents[site].built=0.5;
        for i in 0..14 {let w=g.spawn(WORKER,1,84.0+i as f32*0.1,84.0);g.ents[w].order=O_BUILD;}
        (g,b)
    }

    #[test]
    fn census_accepts_every_actual_eighty_definition() {
        let (mut g,_)=fixture(9,14,STABLE);
        for def in 0..DEFS.len() {let i=g.spawn(def as DefId,1,120.5+(def%8)as f32*4.0,120.5+(def/8)as f32*4.0);g.ents[i].built=1.0;}
        // Includes legal upper-census IDs64..79, surviving obsolete units and
        // buildings. Previously the first such actor indexed beyond count64.
        g.ai_step(1);
        assert_eq!(SAVE_DEFINITION_COUNT,80);
        for def in 64..80 {assert!(g.ents[..g.high].iter().any(|e|e.alive&&e.team==1&&e.def as usize==def));}
    }

    #[test]
    fn fourteen_civilizations_sample_only_legal_jobs_before_rng() {
        for civ in 0..14 {for epoch in [0,5,10,14] {for host in [BARRACKS,STABLE,MILL] {
            let (mut g,b)=fixture(civ,epoch,host);
            #[cfg(feature="ee2-native13")]
            { g.infantry_roster_enabled = false; }
            let mut expected_rng=Rng(g.rng.0);let mut pick=0 as DefId;let mut seen=0;
            for (k,d) in DEFS.iter().enumerate() {
                if d.made_at==host&&d.epoch<=epoch&&d.until>epoch&&d.armed()&&g.can_train(b,k as QueueItem) {
                    seen+=1;if expected_rng.range(seen)==0 {pick=k as DefId;}
                }
            }
            g.ai_step(1);
            assert_eq!(g.rng.0,expected_rng.0,"civ{civ} epoch{epoch} host{host}");
            if pick==0 {assert_eq!(g.ents[b].qn,0);} else {
                assert_eq!(g.ents[b].qn,1);assert_eq!(g.ents[b].q[0],pick as QueueItem);
                assert!(g.definition_civ_available(1,pick));
            }
        }}}
    }

    #[test]
    #[cfg(feature="ee2-native13")]
    fn source_barracks_ai_samples_the_three_legal_families_in_every_epoch() {
        let heavy = [9, 9, 13, 13, 13, 13, 15, 15, 15, 15, 16, 16, 17, 17, 17];
        let light = [10, 10, 12, 12, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14];
        for epoch in 0..15 {
            let (mut g, b) = fixture(0, epoch as u8, BARRACKS);
            // The controlled AI fixture has no City Center and suppresses
            // construction. Give it population room after spawn recounts.
            g.players[1].cap = 1000;
            let offers: Vec<_> = (9..=17).filter(|&id| g.can_train(b, id)).collect();
            let mut expected = vec![heavy[epoch], light[epoch]];
            if epoch > 0 { expected.push(11); }
            expected.sort_unstable();
            assert_eq!(offers, expected);

            let mut rng = Rng(g.rng.0);
            let mut pick = 0;
            for (index, job) in expected.into_iter().enumerate() {
                if rng.range(index as u32 + 1) == 0 { pick = job; }
            }
            g.ai_step(1);
            assert_eq!(g.rng.0, rng.0);
            assert_eq!(g.ents[b].qn, 1);
            assert_eq!(g.ents[b].q[0], pick);
            assert_eq!(g.queue_payment(b, 0).job, pick);
        }
    }

    #[test]
    fn unaffordable_candidates_do_not_consume_rng() {
        let (mut g,b)=fixture(5,5,MILL);
        for slot in 0..9 {let old=g.resource(1,slot);g.add_resource(1,slot,-old);}
        let before=g.rng.0;g.ai_step(1);
        assert_eq!(g.ents[b].qn,0);assert_eq!(g.rng.0,before);
    }

    #[test]
    #[cfg(feature="ee2-pack8")]
    fn sourceproof_live_load_gap_is_exactly_unsaved_navigation_fields() {
        let mut g=Game::new(7,0,0,1,0);
        let tc=g.ents[..g.high].iter().position(|e|e.alive&&e.team==0&&e.def==TC).unwrap();
        let (x,y)=(g.ents[tc].x,g.ents[tc].y);
        let goals:Vec<_>=(0..crate::map::NN).filter(|&t|g.map.pass[0][t]&&((t%N)as f32-x).abs()<30.0&&((t/N)as f32-y).abs()<30.0&&((t%N)as f32-x).abs()>15.0).step_by(71).take(8).collect();
        assert_eq!(goals.len(),8);
        for (k,t) in goals.into_iter().enumerate(){let w=g.spawn(WORKER,0,x+5.0+k as f32*0.2,y+5.0);g.order_move(&[w],(t%N)as f32+0.5,(t/N)as f32+0.5,false);}
        for _ in 0..40{g.tick();}
        g.save();let mut repaired=Game::load(&g.savebuf).unwrap();
        let raw=&g.savebuf;let u=|p|u32::from_le_bytes(raw[p..p+4].try_into().unwrap())as usize;
        let nav=64+u(8)+u(12)*48+(MAXE-u(12))*4;
        let mut old=raw[..nav].to_vec();old[4..8].copy_from_slice(&1u32.to_le_bytes());
        let sum=old.iter().fold(2166136261u32,|a,&v|(a^v as u32).wrapping_mul(16777619));let length=(old.len()+12)as u32;
        for word in[length,sum,crate::pack8_save::END]{old.extend_from_slice(&word.to_le_bytes());}
        // Source9's identity-bearing OEU9 marker cannot be stripped into an old
        // version1 envelope. Native8 still runs the original cache-gap proof.
        #[cfg(feature="ee2-citizen9")]{assert!(Game::load(&old).is_none());return;}
        let mut loaded=Game::load(&old).unwrap();
        assert!(g.map.fields.iter().filter(|f|!f.dir.is_empty()).count()>4);
        assert!(loaded.map.fields.iter().all(|f|f.dir.is_empty()));
        assert!(loaded.navigation_history_unknown);assert!(!repaired.navigation_history_unknown);
        assert_eq!(g.state_hash(),repaired.state_hash());
        g.tick();loaded.tick();repaired.tick();
        assert_ne!(g.state_hash(),loaded.state_hash(),"empty cache changes real next-tick movement");
        assert!(g.ents.iter().zip(&loaded.ents).any(|(a,b)|a.alive&&(a.x.to_bits()!=b.x.to_bits()||a.y.to_bits()!=b.y.to_bits())),"actual positions differ, independently of the history marker");
        assert_eq!(g.state_hash(),repaired.state_hash(),"modern envelope restores exactly the navigation decision state");
    }
}
