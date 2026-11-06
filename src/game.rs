//! The simulation: entities, orders, movement, combat, economy.
//!
//! Runs at a fixed 20 Hz, fully decoupled from rendering (which interpolates).
//! Nothing in the per-tick path allocates, so there are no GC or allocator
//! hitches no matter how many units are alive.

use crate::defs::*;
use crate::map::*;
#[cfg(feature = "ee2-native13")]
#[path = "capture25.rs"]
mod capture25;
#[cfg(feature = "ee2-native13")]
pub(crate) use capture25::split_capture_save;
#[cfg(all(test,feature="ee2-reference"))]
#[path="fortress_tests.rs"]
mod fortress_tests;
#[cfg(all(test,feature="ee2-reference"))]
#[path="ee2_combat_tests.rs"]
mod ee2_combat_tests;
#[cfg(all(test,feature="ee2-reference"))]
#[path="city_ground_tests.rs"]
mod city_ground_tests;

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
#[cfg(feature = "ee2-native13")]
pub const O_CAPTURE: u8 = 8;
/// Marker in `Ent::stuck`: the unit walked onto a transport, remove it without an explosion.
const BOARDED: u8 = 255;

pub const RESEARCH: QueueItem = crate::ids::EPOCH_JOB;
const GC: usize = 64; // grid cells per axis
const CELL: f32 = 4.0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Ent {
    pub alive: bool,
    pub def:DefId,
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
    pub q: [QueueItem; 5],
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
    pub cargo: [DefId; CARGO_MAX],
    pub cargo_hp: [u8; CARGO_MAX],
    pub ncargo: u8,
}

pub(crate) const E0: Ent = Ent {
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
    pub population_limit: u32,
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
    /// Source class/upgrade snapshot at launch; modern joint8 retains it in PRJ1.
    #[cfg(feature="ee2-reference")]
    pub attacker_profile:u16,
    #[cfg(feature="ee2-up15")]
    pub friendly_ratio:f32,
    #[cfg(feature = "ee2-native13")]
    pub(crate) attack_source: u32,
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
    #[cfg(feature = "ee2-native13")]
    pub(crate) battle_signals: Option<crate::battle_signals::BattleSignals>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) capture: capture25::State,
    #[cfg(feature = "ee2-native13")]
    pub(crate) capture_enabled: bool,
    pub fx: Vec<Fx>,
    pub players: [Player; 4],
    // Kept outside Player so existing repr(C) saves retain their layout.
    pub extra_resources: [[f32;6];4],
    pub nplayers: usize,
    pub tick: u32,
    pub rng: Rng,
    #[cfg(feature="ee2-citizen9")]
    pub source_birth:crate::citizen9::SourceBirth,
    #[cfg(feature="ee2-ai9-game-bridge")]
    pub(crate) native_research:Option<crate::ai9_game_bridge::DurableResearch>,
    pub vis: Vec<u8>,
    /// Genuine per-team exploration, independent of local display options.
    pub team_vis: Vec<u8>,
    pub diplomacy: crate::diplomacy::Diplomacy,
    pub territory: crate::territory::TerritoryDiplomacy,
    #[cfg(feature="ee2-reference")]
    pub city_ground: Vec<crate::city_ground::CityBand>,
    #[cfg(feature="ee2-up15")]
    pub attack_clock:Vec<crate::native_attack_clock::Clock>,
    #[cfg(feature="ee2-paidqueue7")]
    pub queue_prices:Vec<crate::paid_queue7::Payment>,
    #[cfg(feature="ee2-pack8")]
    pub pack_states:Vec<crate::pack8::State>,
    #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
    pub navigation_history_unknown:bool,
    #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
    pub projectile_history_unknown:bool,
    pub inst: Vec<f32>,
    pub mini: Vec<u8>,
    pub ui: String,
    pub winner: u8,
    pub bench: u32,
    pub cheats_enabled: bool,
    pub instant_build: bool,
    pub god_mode: bool,
    pub fog_inverted: bool,
    pub tech_points: u32,
    pub research_points: [f32; 4],
    pub researched: [[u32; 6]; 4],
    pub modifiers: [crate::research::Modifiers; 4],
    #[cfg(feature="ee2-civ8")]
    pub civilizations:[u8;4],
    #[cfg(feature="ee2-civ8")]
    pub civ_setup_open:bool,
    #[cfg(feature = "ee2-epoch-range13")]
    pub(crate) final_epoch: u8,
    #[cfg(feature = "ee2-native13")]
    pub(crate) territory_victory: Option<crate::ee2_territory_victory::TerritoryVictory>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) sole_survivor: Option<crate::ee2_sole_survivor::SoleSurvivor>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) regicide: Option<crate::ee2_regicide::Regicide>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) territory_names: Option<crate::territory_names21::TerritoryNames>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) city_names: Option<crate::city_names22::CityNames>,
    #[cfg(feature = "ee2-native13")]
    pub(crate) city_memory: Option<crate::city_memory23::CityMemory>,
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
    #[cfg(feature = "ee2-native13")]
    pub(crate) fullmap_territory_labels: [crate::fullmap_labels21::TerritoryLabel; crate::fullmap_labels21::CAPACITY],
    #[cfg(feature = "ee2-native13")]
    pub(crate) fullmap_city_labels: Vec<crate::fullmap_labels21::CityLabel>,
    /// Sound cues for the page: (kind, x, y) triples.
    pub events: Vec<f32>,
    pub outbox: Vec<u8>,
    pub inbox: Vec<u8>,
    pub savebuf: Vec<u8>,
    pub tmp: Vec<f32>,
    pub ranges: Vec<u32>,
    /// Camera position (x, height, y) for picking; height 0 = top-down, no parallax.
    pub eye: [f32; 3],
    pub placement_cache:crate::placement::PlacementCache,
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
            #[cfg(feature = "ee2-native13")]
            battle_signals: None,
            #[cfg(feature = "ee2-native13")]
            capture: capture25::State::default(),
            #[cfg(feature = "ee2-native13")]
            capture_enabled: humans == 1 && bench == 0,
            fx: Vec::with_capacity(1024),
            players: [Player::default(); 4],
            extra_resources: [[0.0;6];4],
            nplayers,
            tick: 0,
            rng: Rng(seed.wrapping_mul(2_654_435_761) | 1),
            #[cfg(feature="ee2-citizen9")]
            source_birth:crate::citizen9::SourceBirth::new(seed),
            #[cfg(feature="ee2-ai9-game-bridge")]
            native_research:None,
            vis: vec![0; NN],
            team_vis: vec![0; 4*NN], diplomacy: crate::diplomacy::Diplomacy::default(),
            territory: crate::territory::TerritoryDiplomacy::default(),
            #[cfg(feature="ee2-reference")]
            city_ground: vec![crate::city_ground::EMPTY; MAXE],
            #[cfg(feature="ee2-up15")]
            attack_clock:vec![crate::native_attack_clock::EMPTY;MAXE],
            #[cfg(feature="ee2-paidqueue7")]
            queue_prices:Vec::new(),
            #[cfg(feature="ee2-pack8")]
            pack_states:vec![crate::pack8::EMPTY;MAXE],
            #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
            navigation_history_unknown:false,
            #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
            projectile_history_unknown:false,
            inst: Vec::with_capacity(8 * 16384),
            mini: vec![0; NN * 4],
            ui: String::with_capacity(4096),
            winner: 0,
            bench,
            cheats_enabled: false, instant_build: false, god_mode: false, fog_inverted: false, tech_points: 0,
            research_points: [0.0;4], researched: [[0;6];4], modifiers: [[[0.0;19];crate::research::DEF_COUNT];4],
            #[cfg(feature="ee2-civ8")]
            civilizations:[crate::civ8_runtime::LEGACY_NEUTRAL;4],
            #[cfg(feature="ee2-civ8")]
            civ_setup_open:true,
            #[cfg(feature = "ee2-epoch-range13")]
            final_epoch: MAX_EPOCH,
            #[cfg(feature = "ee2-native13")]
            territory_victory: None,
            #[cfg(feature = "ee2-native13")]
            sole_survivor: None,
            #[cfg(feature = "ee2-native13")]
            regicide: None,
            #[cfg(feature = "ee2-native13")]
            territory_names: None,
            #[cfg(feature = "ee2-native13")]
            city_names: None,
            #[cfg(feature = "ee2-native13")]
            city_memory: None,
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
            #[cfg(feature = "ee2-native13")]
            fullmap_territory_labels: [Default::default(); crate::fullmap_labels21::CAPACITY],
            #[cfg(feature = "ee2-native13")]
            fullmap_city_labels: Vec::new(),
            events: Vec::with_capacity(256),
            outbox: Vec::with_capacity(1024),
            inbox: Vec::new(),
            savebuf: Vec::new(),
            tmp: Vec::with_capacity(12 * 16384),
            ranges: vec![0; crate::ids::render_capacity()*2],
            eye: [0.0; 3],
            placement_cache:crate::placement::PlacementCache::default(),
        };
        for p in 0..nplayers {
            g.rebuild_modifiers(p as u8, false);
            let (sx, sy) = g.map.starts[p];
            g.players[p] = Player { res: [400.0, 400.0, 200.0], cap: 15, population_limit: 250, alive: true, ai: p >= humans as usize, ai_next_attack: 20 * 240, ..Player::default() };
            if cfg!(feature="ee2-reference"){g.extra_resources[p]=[400.0,400.0,200.0,200.0,200.0,200.0];}
            g.players[p].recompute();
            let city=g.spawn(TC, p as u8, sx as f32, sy as f32);
            // Initial territory is a scenario grant, not a new expansion claim.
            g.territory.claims[g.map.terr[tile_of(sx as f32,sy as f32)] as usize]=g.h(city);
            for k in 0..5 {
                let a = k as f32 * 1.256;
                g.spawn(WORKER, p as u8, sx as f32 + a.cos() * 3.4, sy as f32 + a.sin() * 3.4);
            }
        }
        if bench > 0 {
            g.extra_resources=[[99999.0;6];4];
            for p in g.players.iter_mut().take(nplayers) {
                p.epoch = MAX_EPOCH;
                p.ai = false;
                p.res = [99999.0; 3];
            }
            for v in g.vis.iter_mut() {
                *v = 255;
            }
            for t in 0..nplayers{g.rebuild_modifiers(t as u8,true);}
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

    pub fn spawn(&mut self, def:DefId, team: u8, x: f32, y: f32) -> usize {
        self.spawn_oriented(def,team,x,y,0.0)
    }
    pub fn spawn_oriented(&mut self,def:DefId,team:u8,x:f32,y:f32,angle:f32)->usize {
        let i = match self.free.pop() {
            Some(i) => i as usize,
            None => return NOIDX,
        };
        if i + 1 > self.high {
            self.high = i + 1;
        }
        #[cfg(feature="ee2-paidqueue7")] self.clear_queue_payments(i);
        let d = &DEFS[def as usize];
        let gen = self.ents[i].gen;
        self.ents[i] = Ent { alive: true, def, team, gen, x, y, px: x, py: y, ox: x, oy: y, hp: self.max_hp(team,def), rot:angle,prot:angle,..E0 };
        #[cfg(feature="ee2-pack8")] self.pack_reset(i);
        #[cfg(feature="ee2-citizen9")] self.source_birth.register(i,self.h(i),def==WORKER);
        #[cfg(feature="ee2-reference")]
        { self.city_ground[i]=crate::city_ground::EMPTY; self.sync_city_ground_entry(i); }
        #[cfg(feature="ee2-up15")]
        {self.attack_clock[i]=crate::native_attack_clock::EMPTY;}
        if d.building {
            self.block(i, true);
        }
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.recount_paid_players();
        i
    }

    fn block(&mut self, i: usize, on: bool) {
        let e = &self.ents[i];
        let (tx0,ty0,s)=Self::building_bounds(e);
        for ty in ty0.max(0)..(ty0+s).min(N as i32){for tx in tx0.max(0)..(tx0+s).min(N as i32){let t=ty as usize*N+tx as usize;self.map.blocked[t]=on;self.map.upd_pass(t);}}
        self.map.nav_ver += 1;
    }

    fn kill(&mut self, i: usize) {
        #[cfg(feature = "ee2-native13")]
        self.forget_capture_actor(i);
        #[cfg(feature = "ee2-native13")]
        self.retire_city_name(i);
        #[cfg(feature="ee2-citizen9")]
        let citizen_sound = {
            let cue = self.citizen_death_cue(i);
            self.source_birth.discard(i);
            cue
        };
        #[cfg(feature="ee2-pack8")] {self.pack_states[i]=crate::pack8::EMPTY;}
        #[cfg(feature="ee2-paidqueue7")] self.clear_queue_payments(i);
        let e = self.ents[i];
        let d = &DEFS[e.def as usize];
        if d.building {
            self.block(i, false);
            for &q in &e.q[..e.qn as usize] {
                if q == RESEARCH {
                    self.players[e.team as usize].researching = false;
                } else if !original_research(q) && q >= TECH0 {
                    self.players[e.team as usize].tech_busy &= !(1 << (q - TECH0));
                }
            }
        }
        if d.building || e.stuck != BOARDED {
            if self.fx.len() < 1000 {
                self.fx.push(Fx { x: e.x, y: e.y, size: d.size * if d.building { 1.6 } else { 1.3 }, age: 0.0, life: 0.6 });
            }
            let sound = if d.building { 11.0 } else { 10.0 };
            #[cfg(feature="ee2-citizen9")]
            let sound = citizen_sound.unwrap_or(sound);
            self.ev(sound, e.x, e.y);
        }
        let s = &mut self.ents[i];
        s.alive = false;
        s.sel = false;
        s.gen = s.gen.wrapping_add(1);
        #[cfg(feature="ee2-reference")]
        { self.city_ground[i]=crate::city_ground::EMPTY; }
        #[cfg(feature="ee2-up15")]
        {self.attack_clock[i]=crate::native_attack_clock::EMPTY;}
        self.free.push(i as u16);
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.recount_paid_players();
    }

    /// Detection is separate from terrain sight: even a revealed-map start
    /// does not expose undetected stealth units.
    pub fn detected(&self, team:u8, j:usize)->bool {
        let t=&self.ents[j];if t.team==team||!DEFS[t.def as usize].has(F_STEALTH)||t.reveal_t>0.0||t.revealed>>team&1==1{return true;}
        (0..self.nplayers).any(|p|self.shares_to(p as u8,team)&&(t.team==p as u8||t.revealed>>p&1==1))
    }
    #[inline]
    pub fn can_see(&self, team:u8, j:usize)->bool {
        let t=&self.ents[j];t.team==team||(self.real_fog(team,tile_of(t.x,t.y))==255&&self.detected(team,j))
    }
    /// Display visibility deliberately differs from combat visibility.
    pub fn visible(&self,j:usize)->bool {
        let t=&self.ents[j];if t.team==self.local{return true;}
        let need=if DEFS[t.def as usize].building{128}else{255};
        (if self.display_visibility()<2{self.real_fog(self.local,tile_of(t.x,t.y))}else{255})>=need&&self.detected(self.local,j)
    }

    // ------------------------------------------------------------------ economy

    pub fn afford(&self, team: u8, c: &[u16]) -> bool {
        c.iter().enumerate().all(|(r,&v)|self.resource(team,r)>=v as f32)
    }
    pub fn pay(&mut self, team: u8, c: &[u16], sign: f32) {
        for (r,&v) in c.iter().enumerate(){self.add_resource(team,r,-(v as f32)*sign);}
    }

    /// `tx,ty` is the top-left tile of the footprint.
    pub fn can_place(&self,def:DefId,tx:i32,ty:i32,team:u8)->bool{self.placement_geometry_error(team,def,tx,ty,0)==0}
    pub fn place(&mut self,def:DefId,tx:i32,ty:i32,team:u8)->usize{self.place_with_rotation(def,tx,ty,team,0)}

    /// Would `train(b, def)` succeed right now?
    pub(crate) fn unit_production_available(&self, team: u8, host: DefId, unit: DefId) -> bool {
        let def = &DEFS[unit as usize];
        let epoch = self.players[team as usize].epoch;
        #[cfg(feature = "ee2-ai9-game-bridge")]
        if host == MILL && self.source_light_artillery_production(team, unit) {
            return true;
        }
        def.made_at == host && def.epoch <= epoch && def.until > epoch
    }

    pub fn can_train(&self, b: usize, def:QueueItem) -> bool {
        #[cfg(feature="ee2-ai9-game-bridge")]
        if crate::ai9_rank::index(def).is_some() {
            let host = &self.ents[b];
            return host.alive && host.hp > 0.0 && host.built >= 1.0
                && self.can_source_research(host.team, host.def, def as usize);
        }
        #[cfg(feature="ee2-civ8")]
        if (def as usize)<DEFS.len()&&!self.definition_civ_available(self.ents[b].team,def as DefId){return false;}
        // The installed UP profile exposes native research, never the separate
        // independent game's technologies. Their wide wire namespace is kept
        // distinct but must not become newly accessible through a crafted job.
        #[cfg(feature="ee2-roster6")]
        if crate::ids::decode_job(def).is_none_or(|(kind,_)|kind==2){return false;}
        let e = &self.ents[b];
        let p = &self.players[e.team as usize];
        #[cfg(feature="ee2-ai9-game-bridge")]
        if original_research(def) {
            let id = (def - ORIGINAL_RESEARCH0) as usize;
            if self.owns_source_research(e.team, id) {
                return e.alive && e.built >= 1.0 && self.can_source_research(e.team, e.def, id);
            }
        }
        if !DEFS[e.def as usize].building || e.built < 1.0 || e.qn >= 5 {
            return false;
        }
        if def == RESEARCH {
            e.def == TC && self.can_advance(e.team)
        } else if original_research(def) {
            #[cfg(feature = "ee2-reference")]
            { self.can_original_research(b,def) }
            #[cfg(not(feature = "ee2-reference"))]
            { false }
        } else if def >= TECH0 {
            let k = (def - TECH0) as usize;
            k < TECHS.len() && TECHS[k].at == e.def && TECHS[k].epoch <= p.epoch && (p.tech | p.tech_busy) >> k & 1 == 0 && self.afford(e.team, &TECHS[k].cost)
        } else {
            (def as usize) < DEFS.len() && self.unit_production_available(e.team, e.def, def as DefId) && self.afford(e.team, &self.unit_cost(e.team,def as DefId)) && p.pop + crate::economy::population(def as DefId,p.epoch) <= p.cap
        }
    }

    pub fn train(&mut self, b: usize, def:QueueItem) -> bool {
        if !self.can_train(b, def) {
            return false;
        }
        let team = self.ents[b].team;
        #[cfg(feature="ee2-ai9-game-bridge")]
        if crate::ai9_rank::index(def).is_some() {
            return self.request_source_research(team, def as usize);
        }
        #[cfg(feature="ee2-ai9-game-bridge")]
        if original_research(def) {
            let id = (def - ORIGINAL_RESEARCH0) as usize;
            if self.owns_source_research(team, id) {
                return self.request_source_research(team, id);
            }
        }
        #[cfg(feature="ee2-paidqueue7")]
        let paid=self.quote_paid_job(b,self.ents[b].qn as usize,def,false);
        if def == RESEARCH {
            self.advance_payment(team,1.0);
            self.players[team as usize].researching = true;
        } else if original_research(def) {
            #[cfg(feature = "ee2-reference")]
            { self.research_points[team as usize] -= crate::ee2_research::NODES[(def-ORIGINAL_RESEARCH0) as usize].cost; }
        } else if def >= TECH0 {
            self.pay(team, &TECHS[(def - TECH0) as usize].cost, 1.0);
            self.players[team as usize].tech_busy |= 1 << (def - TECH0);
        } else {
            self.pay(team, &self.unit_cost(team,def as DefId), 1.0);
        }
        #[cfg(feature="ee2-paidqueue7")] self.record_queue_payment(paid);
        let e = &mut self.ents[b];
        e.q[e.qn as usize] = def;
        e.qn += 1;
        #[cfg(feature="ee2-paidqueue7")] self.recount_paid_players();
        true
    }

    pub fn cancel(&mut self, b: usize) {
        #[cfg(feature="ee2-paidqueue7")]
        {if self.ents[b].qn>0{self.cancel_paid_job(b,self.ents[b].qn as usize-1);}return;}
        #[cfg(not(feature="ee2-paidqueue7"))]
        self.cancel_legacy(b);
    }
    #[cfg(not(feature="ee2-paidqueue7"))]
    fn cancel_legacy(&mut self,b:usize){
        let e = self.ents[b];
        if e.qn == 0 {
            return;
        }
        let item = e.q[e.qn as usize - 1];
        if item == RESEARCH {
            self.advance_payment(e.team,-1.0);
            self.players[e.team as usize].researching = false;
        } else if original_research(item) {
            #[cfg(feature = "ee2-reference")]
            { self.research_points[e.team as usize] += crate::ee2_research::NODES[(item-ORIGINAL_RESEARCH0) as usize].cost; }
        } else if item >= TECH0 {
            self.pay(e.team, &TECHS[(item - TECH0) as usize].cost, -1.0);
            self.players[e.team as usize].tech_busy &= !(1 << (item - TECH0));
        } else {
            self.pay(e.team, &self.unit_cost(e.team,item as DefId), -1.0);
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
            #[cfg(feature="ee2-pack8")] crate::pack8::interrupt(&mut self.pack_states[i]);
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
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
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
            if !self.hostile(self.ents[i].team,t.team)||!self.can_see(self.ents[i].team,tj){continue;}
            let d = &DEFS[self.ents[i].def as usize];
            if d.building {
                continue;
            }
            if d.weapon_vs(td.dom).is_none() || !self.rps_allows(&self.ents[i],&t) {
                if !d.has(F_WORKER) {
                    self.order_move(&[i], t.x, t.y, false);
                }
                continue;
            }
            let key = self.key_for(d.dom, t.x, t.y);
            #[cfg(feature="ee2-pack8")] self.pack_interrupt(i);
            let e = &mut self.ents[i];
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.order = O_ATTACK;
            e.otgt = h;
            e.tgt = h;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    pub fn order_gather(&mut self, ids: &[usize], tile: usize) {
        let kind = self.map.res[tile];
        if kind==R_NONE{return;}
        let ids:Vec<usize>=ids.iter().copied().filter(|&i|self.citizen_can_gather(self.ents[i].team,kind)).collect();
        let (cx, cy) = ((tile % N) as f32 + 0.5, (tile / N) as f32 + 0.5);
        let key = self.key_for(LAND, cx, cy);
        for &i in &ids {
            let e = &mut self.ents[i];
            if !DEFS[e.def as usize].has(F_WORKER) {
                continue;
            }
            e.order = O_GATHER;
            if cfg!(feature="ee2-reference")&&e.carry>0.0&&e.q[0]!=kind as QueueItem {if e.q[2]==0{e.q[2]=e.q[0];}e.q[1]=1;}
            e.otile = tile as u32;
            e.otgt = NONE;
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
            e.q[0] = kind as QueueItem;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    /// Send workers to a building: construct it if unfinished, farm it if it is a farm.
    pub fn order_building(&mut self, ids: &[usize], bj: usize) {
        let b = self.ents[bj];
        let h = self.h(bj);
        let(tx,ty,side)=Self::building_bounds(&b);self.prepare_build_routes(tx,ty,side);
        let assignments:Vec<(usize,u32)>=ids.iter().copied().filter_map(|i|{let e=*self.ents.get(i)?;if !e.alive||e.team!=b.team||!DEFS[e.def as usize].has(F_WORKER){return None;}let route=self.placement_cache.approach[tile_of(e.x,e.y)];
            let route=if route==NONE&&e.x>=tx as f32&&e.x<(tx+side)as f32&&e.y>=ty as f32&&e.y<(ty+side)as f32{self.map.snap(e.x,e.y,0)as u32}else{route};(route!=NONE).then_some((i,route))}).collect();
        for(i,key)in assignments{let e=&mut self.ents[i];e.prev=match e.order{O_GATHER if e.otgt==NONE=>e.q[0] as u8,O_BUILD=>e.prev,_=>0};e.order=if b.built<1.0{O_BUILD}else{O_GATHER};e.q[0]=R_FOOD as QueueItem;e.otgt=h;e.tgt=NONE;e.fkey=key;e.fslot=u16::MAX;e.stuck=0;}
    }

    /// Land units walk to a transport ship and climb aboard.
    pub fn order_board(&mut self, ids: &[usize], tj: usize) {
        let t = self.ents[tj];
        let research = DEFS[t.def as usize].has(F_RESEARCH|F_EXTRACT|F_DROPOFF);
        if (!DEFS[t.def as usize].has(F_TRANSPORT|F_GARRISON) && !research) || t.built < 1.0 {
            return;
        }
        let h = self.h(tj);
        let key = self.key_for(LAND, t.x, t.y);
        let mut first = None;
        for &i in ids {
            if t.def==FORTRESS && self.fortress_slot_cost(t.team,self.ents[i].def).is_none() {continue;}
            let e = &mut self.ents[i];
            let d = &DEFS[e.def as usize];
            if d.building || d.dom != LAND || e.team != t.team || (research && !d.has(F_WORKER)) {
                continue;
            }
            e.order = O_BOARD;
            e.otgt = h;
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
            e.fkey = key;
            e.stuck = 0;
            first.get_or_insert((e.x, e.y));
        }
        // an idle ship comes to the beach to pick them up
        if !research && !DEFS[t.def as usize].building {
            if let (Some((x, y)), O_IDLE) = (first, t.order) { self.order_move(&[tj], x, y, false); }
        }
    }

    /// Transports sail to the shore nearest `(x, y)` and put everyone ashore.
    pub fn order_unload(&mut self, ids: &[usize], x: f32, y: f32) {
        let key = self.key_for(SEA, x.clamp(1.0, N as f32 - 1.0), y.clamp(1.0, N as f32 - 1.0));
        let t = (key & 0xffff) as usize;
        for &i in ids {
            if DEFS[self.ents[i].def as usize].has(F_RESEARCH|F_EXTRACT|F_DROPOFF|F_GARRISON) {
                let mut e=self.ents[i]; self.unload(i,&mut e); self.ents[i]=e;
                continue;
            }
            let e = &mut self.ents[i];
            if !DEFS[e.def as usize].has(F_TRANSPORT) {
                continue;
            }
            e.order = O_UNLOAD;
            e.ox = (t % N) as f32 + 0.5;
            e.oy = (t / N) as f32 + 0.5;
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
            e.otgt = NONE;
            e.fkey = key;
            e.stuck = 0;
        }
    }

    fn unload(&mut self, _host:usize, e: &mut Ent) {
        let lt = self.map.snap(e.x, e.y, 0);
        let (lx, ly) = ((lt % N) as f32 + 0.5, (lt / N) as f32 + 0.5);
        let exit_radius_squared=if DEFS[e.def as usize].building {20.0_f32.max((Self::building_half(e)+1.0).powi(2))}else{20.0};
        if (lx - e.x).powi(2) + (ly - e.y).powi(2) > exit_radius_squared {
            return; // no beach within reach: keep everyone aboard
        }
        let old_count=e.ncargo as usize;let mut remaining=0;
        #[cfg(feature="ee2-citizen9")] let source_mask=self.source_birth.slots[_host].cargo_females;
        #[cfg(feature="ee2-citizen9")] let mut remaining_source_mask=0_u32;
        for k in 0..old_count {
            let (a, r) = (k as f32 * 2.4, 0.45 * (k as f32).sqrt());
            let (mut x, mut y) = (lx + a.cos() * r, ly + a.sin() * r);
            if !self.map.pass[0][tile_of(x, y)] { x = lx; y = ly; }
            let u = self.spawn(e.cargo[k], e.team, x, y);
            if u != NOIDX {
                self.ents[u].hp = self.max_hp(e.team,e.cargo[k]) * (e.cargo_hp[k] as f32 / 100.0).min(1.0);
                #[cfg(feature="ee2-citizen9")] if e.cargo[k]==WORKER&&source_mask>>k&1!=0{self.set_citizen_identity(u,crate::native_rng9::CitizenIdentity::Female);}
            }else {
                e.cargo[remaining]=e.cargo[k];e.cargo_hp[remaining]=e.cargo_hp[k];
                #[cfg(feature="ee2-citizen9")] if source_mask>>k&1!=0{remaining_source_mask|=1<<remaining;}
                remaining+=1;
            }
        }
        e.cargo[remaining..old_count].fill(0);e.cargo_hp[remaining..old_count].fill(0);e.ncargo=remaining as u8;
        #[cfg(feature="ee2-citizen9")] {self.source_birth.slots[_host].cargo_females=remaining_source_mask;}
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
        let td=&DEFS[t.def as usize];
        if t.built < 1.0 || (!td.has(F_TRANSPORT|F_GARRISON) && !td.has(F_RESEARCH|F_EXTRACT|F_DROPOFF)) || (td.has(F_RESEARCH|F_EXTRACT|F_DROPOFF) && !d.has(F_WORKER)) { Self::set_idle(e); return; }
        let slots=if t.def==FORTRESS {match self.fortress_slot_cost(t.team,e.def){Some(v)=>v,None=>{Self::set_idle(e);return;}}}else{1};
        if if td.building{(t.x-e.x).abs().max((t.y-e.y).abs())<=Self::building_half(&t)+0.95}else{(t.x-e.x).powi(2)+(t.y-e.y).powi(2)<2.8*2.8}{
            if (t.ncargo as usize)<CARGO_MAX && self.cargo_slots_used(&t)+slots<=self.cargo_capacity(t.team,t.def) {
                let maxhp=self.max_hp(e.team,e.def);
                #[cfg(feature="ee2-citizen9")] self.source_birth.set_cargo(j,t.ncargo as usize,e.def==WORKER&&self.source_birth.slots[i].identity==1);
                let tr = &mut self.ents[j];
                tr.cargo[tr.ncargo as usize] = e.def;
                tr.cargo_hp[tr.ncargo as usize] = (e.hp / maxhp * 100.0).clamp(1.0, 100.0) as u8;
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
        #[cfg(feature = "ee2-epoch-range13")]
        if self.final_epoch != MAX_EPOCH {
            mix(crate::match_epoch_range13::MAGIC);
            mix(self.final_epoch.into());
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.territory_victory {
            mix(crate::ee2_territory_victory::MAGIC);
            mix(rule.percent.into());
            mix(rule.eliminated.into());
            mix(self.winner.into());
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.sole_survivor {
            mix(crate::ee2_sole_survivor::MAGIC);
            mix(rule.deadline.to_bits());
            mix(rule.eliminated.into());
            mix(self.winner.into());
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.regicide {
            mix(crate::ee2_regicide::MAGIC);
            mix(rule.state.eliminated.into());
            mix(rule.state.deadline.to_bits());
            mix(rule.sole.into());
            mix(self.winner.into());
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(names) = &self.territory_names {
            mix(crate::territory_names21::MAGIC);
            mix(crate::territory_name_catalog21::VERSION);
            for id in names.ids { mix(id.into()); }
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(names) = &self.city_names {
            mix(crate::city_names22::MAGIC);
            mix(crate::city_name_catalog22::VERSION);
            mix(names.ids.len() as u32);
            for (&handle, &id) in &names.ids { mix(handle); mix(id.into()); }
        }
        #[cfg(feature = "ee2-native13")]
        self.capture.hash(&mut mix);
        #[cfg(feature="ee2-civ8")]
        if self.civilizations.iter().any(|&c|c!=crate::civ8_runtime::LEGACY_NEUTRAL){
            mix(0x38564943);for c in self.civilizations{mix(c as u32);}
        }
        mix(self.cheats_enabled as u32);
        mix(self.instant_build as u32);
        mix(self.tech_points);
        if self.god_mode { mix(u32::from_le_bytes(*b"GOD1")); }
        if self.fog_inverted { mix(u32::from_le_bytes(*b"FOG1")); }
        mix(self.diplomacy.visibility as u32);
        for a in [&self.diplomacy.relation,&self.diplomacy.offers,&self.diplomacy.offer_vision,&self.diplomacy.sharing]{for &v in a{mix(v as u32);}}
        for a in [&self.diplomacy.offer_tick,&self.diplomacy.cooldown,&self.diplomacy.gifts,&self.diplomacy.neutral_since,&self.diplomacy.war_tick]{for &v in a{mix(v);}}
        for h in self.territory.claims{mix(h);}
        for o in self.territory.offers{mix(o.active as u32|(o.give as u32)<<8|(o.want as u32)<<16);mix(o.tick);mix(o.cooldown);}
        for e in &self.territory.log{mix(e.tick);mix(e.from as u32|(e.to as u32)<<8|(e.kind as u32)<<16|(e.reason as u32)<<24);mix(e.give);mix(e.want);}
        for chunk in self.team_vis.chunks_exact(4){mix(u32::from_le_bytes(chunk.try_into().unwrap()));}
        for e in &self.diplomacy.log{mix(e.tick);mix(e.from as u32|(e.to as u32)<<8|(e.kind as u32)<<16|(e.reason as u32)<<24);mix(e.value);mix(e.resource);}

        for t in 0..self.nplayers {
            mix(self.research_points[t].to_bits());
            for b in self.researched[t] { mix(b); }
            if cfg!(feature="ee2-reference"){for value in self.extra_resources[t]{mix(value.to_bits());}}
        }
        for p in &self.players[..self.nplayers] {
            for r in p.res {
                mix(r.to_bits());
            }
            mix(p.epoch as u32 | p.tech << 8);
            mix(p.population_limit);
        }
        for e in &self.ents[..self.high] {
            if e.alive {
                if cfg!(feature="ee2-roster6"){mix(e.def as u32);mix(e.team as u32 |(e.order as u32)<<8|(e.ncargo as u32)<<16);}else{mix(e.def as u32 | (e.team as u32) << 8 | (e.order as u32) << 16 | (e.ncargo as u32) << 24);}
                mix(e.x.to_bits());
                mix(e.y.to_bits());
                mix(e.hp.to_bits());
                mix(e.built.to_bits());
                if DEFS[e.def as usize].building{mix(e.rot.to_bits());}
                mix(e.carry.to_bits());
                mix(e.qt.to_bits());

                mix(e.qn as u32);
                if DEFS[e.def as usize].building {for q in &e.q[..e.qn as usize] {mix(*q as u32);}}
                for k in 0..e.ncargo as usize {if cfg!(feature="ee2-roster6"){mix(e.cargo[k]as u32);mix(e.cargo_hp[k]as u32);}else{mix(e.cargo[k] as u32 | (e.cargo_hp[k] as u32)<<8);}}
            }
        }
        #[cfg(feature="ee2-reference")]
        {
            mix(crate::city_ground::CITY_MAGIC);
            for i in 0..self.high {
                let radius=self.city_ground_radius(i);
                if radius>0.0 {mix(self.h(i));mix(radius.to_bits());}
            }
        }
        #[cfg(feature="ee2-up15")]
        {mix(crate::native_attack_clock::MAGIC);for i in 0..self.high{let c=self.attack_clock[i];if self.ents[i].alive&&c.handle==self.h(i){mix(c.handle);mix(c.next.to_bits());mix(c.flags as u32);}}}
        #[cfg(feature="ee2-paidqueue7")]
        {mix(crate::paid_queue7::MAGIC);for p in self.canonical_queue_payments(){mix(p.handle);mix(p.position);mix(p.job);mix(p.inferred as u32|((p.purchased as u32)<<1));for v in p.price{mix(v.to_bits());}mix(p.duration.to_bits());}}
        #[cfg(feature="ee2-pack8")]
        {for word in self.pack_hash_words(){mix(word);}}
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.hash_navigation(&mut mix);
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.hash_projectiles(&mut mix);
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.hash_decisions8(&mut mix);
        #[cfg(feature="ee2-citizen9")] for word in self.source_birth_hash_words(){mix(word);}
        #[cfg(feature="ee2-ai9-game-bridge")]
        if let Some(state) = &self.native_research { crate::ai9_game_save::hash(state, &mut mix); }
        h
    }

    pub fn order_stop(&mut self, ids: &[usize]) {
        for &i in ids {
            #[cfg(feature="ee2-pack8")] self.pack_interrupt(i);
            self.native_stop_attack(i);
            let e = &mut self.ents[i];
            e.order = O_IDLE;
            e.ox = e.x;
            e.oy = e.y;
            #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
        }
    }

    // ------------------------------------------------------------------ tick

    pub fn tick(&mut self) {
        // Reset resource work slots once, before either old or newly trained
        // citizens run. Resetting inside a building update lets later IDs bypass it.
        for e in &mut self.ents[..self.high]{if e.alive&&e.def==FARM{e.carry=0.0;}}
        self.tick += 1;
        #[cfg(feature = "ee2-native13")]
        if let Some(signals) = &mut self.battle_signals { signals.update_groups(DT, &self.ents); }
        if self.bench==0{self.update_diplomacy();}
        #[cfg(feature = "ee2-native13")]
        self.tick_capture_state();
        self.map.budget = 4;
        self.build_grid();
        if self.bench==0{self.update_territory_diplomacy();}
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
        let mut deaths=false;let mut city_death=false;
        for i in 0..self.high {
            if self.ents[i].alive && self.ents[i].hp <= 0.0 {
                deaths=true;city_death|=self.ents[i].def==TC;self.kill(i);
            }
        }
        if city_death{self.rebuild_territories();self.void_landless_alliances();}
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.recount_paid_players();
        if deaths&&self.bench==0{self.prune_territory_offers();}
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
            #[cfg(feature="ee2-ai9-game-bridge")]
            self.tick_source_research();
            for t in 0..self.nplayers {
                #[cfg(feature = "ee2-native13")]
                if self.eliminated_by_victory(t) {
                    continue;
                }
                if self.players[t].ai && (self.tick + t as u32 * 7) % 24 == 0 {
                    self.ai_step(t as u8);
                }
            }
            let poll_result = self.tick % 20 == 0;
            #[cfg(feature = "ee2-native13")]
            let poll_result = poll_result
                || ((self.sole_survivor.is_some() || self.regicide.is_some()) && self.winner == 0);
            if poll_result && (self.winner == 0 || cfg!(all(feature="ee2-pack8",feature="ee2-civ8"))) {
                self.refresh_match_alive();
                if self.winner == 0 {
                    self.poll_match_outcome();
                }
            }
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(signals) = &mut self.battle_signals { signals.update_markers(DT); }
        #[cfg(feature = "ee2-native13")]
        self.reconcile_capture_orders();
    }

    #[cfg(feature = "ee2-native13")]
    fn eliminated_by_victory(&self, seat: usize) -> bool {
        self.territory_victory.is_some_and(|rule| rule.eliminated(seat))
            || self.sole_survivor.is_some_and(|rule| rule.eliminated(seat))
            || self.regicide.is_some_and(|rule| rule.state.eliminated(seat))
    }

    pub(crate) fn refresh_match_alive(&mut self) {
        #[cfg(feature = "ee2-native13")]
        if !self.net && self.bench == 0 {
            let mut living = crate::ee2_conquest::living_seats(self);
            for (seat, alive) in living.iter_mut().enumerate() {
                *alive &= !self.eliminated_by_victory(seat);
            }
            for (player, alive) in self.players[..self.nplayers].iter_mut().zip(living) {
                player.alive = alive;
            }
            return;
        }
        for player in &mut self.players[..self.nplayers] {
            player.alive = player.buildings > 0;
        }
    }

    fn match_has_winner(&self, local: usize) -> bool {
        #[cfg(feature = "ee2-native13")]
        if !self.net && self.bench == 0 {
            return crate::ee2_conquest::survivors_are_allied(self);
        }
        (0..self.nplayers).all(|p| p == local || !self.players[p].alive)
    }

    fn poll_match_outcome(&mut self) {
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.regicide {
            self.poll_regicide_outcome(rule);
            return;
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.sole_survivor {
            self.poll_sole_outcome(rule);
            return;
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(rule) = self.territory_victory {
            self.poll_territory_outcome(rule);
            return;
        }
        let me = self.local as usize;
        if !self.players[me].alive {
            self.winner = 2;
        } else if self.match_has_winner(me) {
            self.winner = 1;
        }
    }

    /// Counting-sort every live entity into a uniform grid; also recount population.
    pub fn build_grid(&mut self) {
        #[cfg(feature="ee2-reference")]
        self.sync_city_ground();
        let g = &mut self.grid;
        for c in g.start.iter_mut() {
            *c = 0;
        }
        let mut pop = [0u32; 4];
        let mut cap = [0u32; 4];
        let mut bld = [0u32; 4];
        let mut n = 0;
        for e in &self.ents[..self.high] {
            if !e.alive {
                continue;
            }
            n += 1;
            g.start[cell_of(e.x, e.y) + 1] += 1;
            let d = &DEFS[e.def as usize];
            let t = e.team as usize;
            pop[t] += crate::economy::population(e.def,self.players[t].epoch);
            for &c in &e.cargo[..e.ncargo as usize] {
                pop[t] += crate::economy::population(c,self.players[t].epoch);
            }
            if d.building {
                bld[t] += 1;
                if e.built >= 1.0 {
                    cap[t] += self.modifiers[t][e.def as usize][18].round().max(0.0) as u32;
                }
                for &q in &e.q[..e.qn as usize] {
                    if (q as usize) < DEFS.len() {
                        pop[t] += crate::economy::population(q as DefId,self.players[t].epoch);
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
            self.players[t].cap = cap[t].min(self.players[t].population_limit);
            self.players[t].buildings = bld[t];
        }
        self.rebuild_territories();
    }

    #[inline]
    fn rps_allows(&self,attacker:&Ent,defender:&Ent)->bool {
        #[cfg(feature="ee2-reference")]
        {return crate::ee2_combat::can_target(attacker.def,self.players[attacker.team as usize].epoch,defender.def,self.players[defender.team as usize].epoch).unwrap_or(true);}
        #[cfg(not(feature="ee2-reference"))]
        {let _=(attacker,defender);true}
    }

    fn native_attack_min(&self,e:&Ent,t:&Ent)->f32 {
        #[cfg(feature="ee2-up15")] {
            if let Some(a)=crate::ee2_up15_attack::lookup(e.def,self.players[e.team as usize].epoch){
                let mobile=crate::ee2_up15_attack::lookup(t.def,self.players[t.team as usize].epoch).map_or(!DEFS[t.def as usize].building,|a|a.mobile);
                return crate::ee2_up15_attack::min_range(a,mobile);
            }
        }
        let _=(e,t);0.0
    }
    fn native_too_close(&self,e:&Ent,t:&Ent,edge_distance:f32)->bool {
        let minimum=self.native_attack_min(e,t);minimum>0.0&&edge_distance<minimum
    }
    /// Nearest hostile that `e` can see and shoot within `r`.
    fn acquire(&self, e: &Ent, d: &Def, r: f32) -> u32 {
        let mask = d.all_targets();
        let m = r + 5.0;
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
                    if !self.hostile(e.team,t.team) {
                        continue;
                    }
                    let td = &DEFS[t.def as usize];
                    if mask & td.dom == 0 {
                        continue;
                    }
                    if !self.rps_allows(e,t){continue;}
                    let (dx, dy) = (t.x - e.x, t.y - e.y);
                    let dist = (dx * dx + dy * dy).sqrt() - self.entity_radius(t);
                    if dist > r || self.native_too_close(e,t,dist) {
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

    fn try_attack_round(&mut self,index:usize,e:&mut Ent,w:&Wpn,tj:usize){
        #[cfg(feature="ee2-pack8")] if self.pack_wait_for_attack(index,e){return;}
        #[cfg(feature="ee2-up15")]
        if let Some(a)=crate::ee2_up15_attack::lookup(e.def,self.players[e.team as usize].epoch){
            let now=self.tick as f32*DT;
            let reload=crate::ee2_units::lookup_class(e.def,self.players[e.team as usize].epoch).and_then(|r|r.reload).unwrap_or(w.cd);
            let handle=((e.gen as u32)<<16)|index as u32;let c=&mut self.attack_clock[index];
            if c.handle!=handle{*c=crate::native_attack_clock::Clock{handle,next:0.0,flags:0};}
            let event=crate::native_attack_clock::step(now,reload,a.apply_time,&mut c.next,&mut c.flags);
            let remaining=(c.next-now).max(0.0);e.cd=remaining;
            if event==crate::native_attack_clock::Event::Fire{self.fire(index,e,w,tj);e.cd=remaining;}
            return;
        }
        self.fire(index,e,w,tj);
    }
    pub(crate) fn native_stop_attack(&mut self,index:usize){
        #[cfg(feature="ee2-up15")]crate::native_attack_clock::stop(&mut self.attack_clock[index].flags);
        #[cfg(not(feature="ee2-up15"))]let _=index;
    }
    fn native_round_locked(&self,index:usize,e:&Ent)->bool{
        #[cfg(feature="ee2-up15")]{let c=self.attack_clock[index];return c.handle==((e.gen as u32)<<16|index as u32)&&c.flags&(crate::native_attack_clock::STARTED|crate::native_attack_clock::IN_ATTACK)==(crate::native_attack_clock::STARTED|crate::native_attack_clock::IN_ATTACK);}
        #[cfg(not(feature="ee2-up15"))]{let _=(index,e);false}
    }
    fn fire(&mut self, _source_index: usize, e: &mut Ent, w: &Wpn, tj: usize) {
        if !self.hostile(e.team,self.ents[tj].team)||!self.can_see(e.team,tj)||!self.rps_allows(e,&self.ents[tj]){e.tgt=NONE;return;}
        #[cfg(feature="ee2-reference")]
        let attacker_profile=crate::ee2_combat::profile(e.def,self.players[e.team as usize].epoch);
        let mut w=*w;
        #[cfg(feature="ee2-up15")]
        let attack=crate::ee2_up15_attack::lookup(e.def,self.players[e.team as usize].epoch);
        #[cfg(feature="ee2-up15")]
        if let Some(a)=attack {w.kind=a.visual_kind;w.splash=a.splash;}
        // Both Cannonball and TankShell use our existing shell projectile.
        #[cfg(feature="ee2-reference")]
        if e.def==FORTRESS {
            if let Some(row)=crate::ee2_units::lookup_class(FORTRESS,self.players[e.team as usize].epoch) {
                if matches!(row.attack_missile,Some("Cannonball"|"TankShell")) {w.kind=W_SHELL;}
                if let Some(reload)=row.reload {w.cd=reload;}
            }
        }
        #[cfg(feature="ee2-up15")]
        if let Some(row)=crate::ee2_units::lookup_class(e.def,self.players[e.team as usize].epoch){
            if let Some(reload)=row.reload {w.cd=reload;}
        }
        e.cd = if cfg!(feature="ee2-up15")||e.def==FORTRESS {w.cd} else {w.cd * (0.9 + 0.2 * self.rng.f())};
        let d = &DEFS[e.def as usize];
        if d.has(F_STEALTH) {
            e.reveal_t = 3.0;
        }
        let dmg = self.scaled_weapon_damage(e.team,e.def,w.dmg,self.players[e.team as usize].atk[class_of(d)]) * (1.0+self.fortress_attack_bonus(e));
        let sound = if w.kind == W_MELEE { 7.0 } else { w.kind as f32 };
        #[cfg(feature="ee2-up15")]
        let sound = match attack {
            // LightArtillery1_01 Attack1 has its own launch sound at ApplyDamageTime.
            Some(row) if row.id == 22 && row.epoch == 0 && row.until == 2 => 26.0,
            _ => sound,
        };
        self.ev(sound, e.x, e.y);
        let direct={
            #[cfg(feature="ee2-up15")] {attack.map(|a|!a.missile).unwrap_or(w.kind==W_MELEE)}
            #[cfg(not(feature="ee2-up15"))] {w.kind==W_MELEE}
        };
        if direct || self.projs.len() >= 4000 {
            #[cfg(feature="ee2-up15")]
            self.native_attack_impact(e.team,self.h(tj),self.ents[tj].x,self.ents[tj].y,dmg,w.kind,w.targets,w.splash,attack.map_or(0.0,|a|a.friendly),attacker_profile,
                #[cfg(feature = "ee2-native13")]
                Some(crate::battle_signals::Actor::new(_source_index, e)));
            #[cfg(all(feature="ee2-reference",not(feature="ee2-up15")))]
            self.damage_from(tj,dmg,w.kind,attacker_profile);
            #[cfg(not(feature="ee2-reference"))]
            self.damage(tj, dmg, w.kind);
            return;
        }
        let t = &self.ents[tj];
        let d0 = ((t.x - e.x).powi(2) + (t.y - e.y).powi(2)).sqrt().max(0.1);
        const SPEED: [f32; 7] = [0.0, 14.0, 34.0, 15.0, 6.5, 13.0, 7.0];
        self.projs.push(Proj { x: e.x, y: e.y, px: e.x, py: e.y, tx: t.x, ty: t.y, tgt: self.h(tj), speed: {
            #[cfg(feature="ee2-up15")] {attack.and_then(|a|a.projectile_speed).filter(|v|*v>0.0).unwrap_or(SPEED[w.kind as usize])}
            #[cfg(not(feature="ee2-up15"))] {SPEED[w.kind as usize]}
        }, dmg, kind: w.kind, team: e.team, splash: w.splash, targets: w.targets,
            #[cfg(feature="ee2-reference")]
            attacker_profile,
            #[cfg(feature="ee2-up15")]
            friendly_ratio:attack.map_or(0.0,|a|a.friendly),
            #[cfg(feature = "ee2-native13")]
            attack_source: self.h(_source_index),
            d0 });
    }

    #[inline]
    fn damage(&mut self, j: usize, dmg: f32, kind: u8) {
        let (team, td) = (self.ents[j].team, &DEFS[self.ents[j].def as usize]);
        let m = MULT[kind as usize][td.armor as usize] * self.players[team as usize].dfn[class_of(td)];
        let protected = self.god_mode_for(team);
        let t = &mut self.ents[j];
        if !protected { t.hp -= dmg * m; }
        t.flash = 0.12;
        if team == self.local && m > 0.0 && self.tick % 8 == 0 {
            let (x, y) = (t.x, t.y);
            self.ev(15.0, x, y);
        }
    }

    #[cfg(feature="ee2-reference")]
    #[inline]
    fn damage_from(&mut self,j:usize,dmg:f32,kind:u8,attacker_profile:u16){
        let (team,def)=(self.ents[j].team,self.ents[j].def);
        let td=&DEFS[def as usize];
        let native=crate::ee2_combat::multiplier_from_profile(attacker_profile,def,self.players[team as usize].epoch);
        let m=native.unwrap_or(MULT[kind as usize][td.armor as usize])*self.players[team as usize].dfn[class_of(td)];
        let protected = self.god_mode_for(team);
        let t=&mut self.ents[j];if !protected {t.hp-=dmg*m;}t.flash=0.12;
        let legacy_warning = {
            #[cfg(feature = "ee2-native13")]
            { self.battle_signals.is_none() }
            #[cfg(not(feature = "ee2-native13"))]
            { true }
        };
        if legacy_warning&&team==self.local&&m>0.0&&self.tick%8==0 {let(x,y)=(t.x,t.y);self.ev(15.0,x,y);}
    }

    #[cfg(feature="ee2-up15")]
    fn native_attack_impact(&mut self,team:u8,primary:u32,x:f32,y:f32,damage:f32,kind:u8,targets:u8,radius:f32,friendly:f32,profile:u16,
        #[cfg(feature = "ee2-native13")] origin: Option<crate::battle_signals::Actor>){
        let hit = |game: &mut Self, j: usize, points: f32| {
            #[cfg(feature = "ee2-native13")]
            let can_signal = game.ents[j].hp > 0.0 && game.ents[j].def != FARM;
            game.damage_from(j, points, kind, profile);
            #[cfg(feature = "ee2-native13")]
            if can_signal {
                if let Some(signals) = &mut game.battle_signals {
                    let target = &game.ents[j];
                    let actor = crate::battle_signals::Actor::new(j, target);
                    if signals.hit(game.local, actor, origin, target.hp <= 0.0) {
                        game.ev(15.0, target.x, target.y);
                    }
                }
            }
        };
        if radius<=0.0 {if let Some(j)=self.get(primary){if self.hostile(team,self.ents[j].team){hit(self,j,damage);}}return;}
        for j in 0..self.high{
            let t=self.ents[j];if !t.alive||targets&DEFS[t.def as usize].dom==0{continue;}
            let ratio=if self.hostile(team,t.team){1.0}else if team==t.team{friendly}else{0.0};if ratio<=0.0{continue;}
            let distance=((t.x-x).powi(2)+(t.y-y).powi(2)).sqrt();
            if distance<=radius{let falloff=crate::ee2_combat::splash_factor(distance,radius,self.h(j)==primary);hit(self,j,damage*falloff*ratio);}
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
                    if self.hostile(p.team,self.ents[j].team)&&self.can_see(p.team,j){p.tx = self.ents[j].x;
                    p.ty = self.ents[j].y;}
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
            #[cfg(feature="ee2-up15")]
            self.native_attack_impact(p.team,p.tgt,p.tx,p.ty,p.dmg,p.kind,p.targets,p.splash,p.friendly_ratio,p.attacker_profile,
                #[cfg(feature = "ee2-native13")]
                self.get(p.attack_source).filter(|&index| self.ents[index].team == p.team)
                    .map(|index| crate::battle_signals::Actor::new(index, &self.ents[index])));
            #[cfg(not(feature="ee2-up15"))]
            if p.splash > 0.0 {
                let m = p.splash + 5.0;
                let (cx0, cy0) = (((p.tx - m) / CELL).max(0.0) as usize, ((p.ty - m) / CELL).max(0.0) as usize);
                let (cx1, cy1) = ((((p.tx + m) / CELL) as usize).min(GC - 1), (((p.ty + m) / CELL) as usize).min(GC - 1));
                for cy in cy0..=cy1 {
                    for cx in cx0..=cx1 {
                        let c = cy * GC + cx;
                        for q in self.grid.start[c]..self.grid.start[c + 1] {
                            let j = self.grid.items[q as usize] as usize;
                            let t = &self.ents[j];
                            let td = &DEFS[t.def as usize];
                            if !self.hostile(p.team,t.team) || td.dom & p.targets == 0 || !t.alive {
                                continue;
                            }
                            let center_distance=((t.x-p.tx).powi(2)+(t.y-p.ty).powi(2)).sqrt();
                            #[cfg(feature="ee2-reference")]
                            let d=center_distance;
                            #[cfg(not(feature="ee2-reference"))]
                            let d=center_distance-self.entity_radius(t);
                            if d <= p.splash {
                                #[cfg(feature="ee2-reference")]
                                let f=crate::ee2_combat::splash_factor(d,p.splash,self.h(j)==p.tgt);
                                #[cfg(not(feature="ee2-reference"))]
                                let f = 1.0 - 0.5 * (d / p.splash).clamp(0.0, 1.0);
                                #[cfg(feature="ee2-reference")]
                                self.damage_from(j,p.dmg*f,p.kind,p.attacker_profile);
                                #[cfg(not(feature="ee2-reference"))]
                                self.damage(j, p.dmg * f, p.kind);
                            }
                        }
                    }
                }
            } else if let Some(j) = self.get(p.tgt) {
                if self.hostile(p.team,self.ents[j].team){
                    #[cfg(feature="ee2-reference")]
                    self.damage_from(j,p.dmg,p.kind,p.attacker_profile);
                    #[cfg(not(feature="ee2-reference"))]
                    self.damage(j,p.dmg,p.kind);
                }
            }
            if p.kind >= W_SHELL && self.fx.len() < 1000 {
                self.fx.push(Fx { x: p.tx, y: p.ty, size: (p.splash * 2.0).max(0.7), age: 0.0, life: 0.2 + p.splash * 0.14 });
            }
        }
    }

    /// Direction to walk toward `(gx, gy)`: shared flow field when far, straight line when close.
    pub(crate) fn steer(&mut self, e: &mut Ent, d: &Def, gx: f32, gy: f32) -> (f32, f32) {
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

    pub(crate) fn set_idle(e: &mut Ent) {
        e.order = O_IDLE;
        e.ox = e.x;
        e.oy = e.y;
        e.stuck = 0;
    }

    pub(crate) fn instant_build_for(&self, owner: u8) -> bool {
        !self.net && self.instant_build && owner == self.local
            && (cfg!(feature = "ee2-up15") || self.cheats_enabled)
    }

    pub(crate) fn god_mode_for(&self, owner: u8) -> bool {
        cfg!(feature = "ee2-up15") && !self.net && self.cheats_enabled && self.god_mode && owner == self.local
    }

    fn update_building(&mut self, i: usize, e: &mut Ent, d: &Def) {
        let instant = self.instant_build_for(e.team);
        if e.built < 1.0 && instant {
            e.built = 1.0;
            e.hp = self.max_hp(e.team,e.def);
            #[cfg(feature = "ee2-native13")]
            self.name_completed_city(i, *e);
        }
        #[cfg(feature="ee2-reference")]
        self.observe_city_ground(i,*e);
        if e.built < 1.0 { return; }
        if e.def==FORTRESS && e.ncargo>0 {
            let rate=self.fortress_heal_rate(e.team);
            let before=e.carry;let after=before+DT;
            // Existing saves store cargo health in whole percent. A shared
            // healing clock accumulates native HP/sec without losing fractional
            // work every tick; display/storage precision remains one percent.
            for k in 0..e.ncargo as usize {
                let hp=self.max_hp(e.team,e.cargo[k]).max(0.01);
                let added=((after*rate*100.0/hp).floor()-(before*rate*100.0/hp).floor()).max(0.0) as u32;
                e.cargo_hp[k]=(e.cargo_hp[k] as u32+added).min(100) as u8;
            }
            e.carry=after;
        }else if e.def==FORTRESS {e.carry=0.0;}
        if d.has(F_RESEARCH) {
            // Ghidra 007e7cf0: accumulate per building, floor, then credit the owner's bank.
            e.carry += (if cfg!(feature="ee2-up15"){self.native_generation(e.def,e.team,9)}else{UNIVERSITY_RATE}) * (e.ncargo as usize).min(self.cargo_capacity(e.team,e.def)) as f32 * DT * self.modifiers[e.team as usize][e.def as usize][7];
            let points=e.carry.floor();
            if points>=1.0 {self.research_points[e.team as usize]+=points;e.carry-=points;}
        }
        if d.has(F_EXTRACT){
            let kind=Self::extractor_kind(e.def);
            // Gifts do not unlock a recipient's future or obsolete resource.
            // Keep every crew member; inactive deposits do not accumulate work.
            if self.resource_unlocked(e.team,kind){
                e.carry+=(if cfg!(feature="ee2-up15"){self.native_generation(e.def,e.team,kind-1)}else{0.75})*(e.ncargo as usize).min(self.cargo_capacity(e.team,e.def)) as f32*DT*self.modifiers[e.team as usize][e.def as usize][crate::economy::gather_modifier(kind)];
                let whole=e.carry.floor();if whole>=1.0{self.add_resource(e.team,kind as usize-1,whole);e.carry-=whole;}
            }else{e.carry=0.0;}
        }
        #[cfg(feature="ee2-paidqueue7")] let paid_ready=self.purchase_front_job(i);
        #[cfg(not(feature="ee2-paidqueue7"))] let paid_ready=true;
        if e.qn > 0 && paid_ready {
            let item = e.q[0];
            #[cfg(feature="ee2-paidqueue7")] let(job_limit,job_start)=self.begin_paid_job(i);
            let team = e.team as usize;
            let mut done = false;
            if item == RESEARCH {
                e.qt += DT;
                if (cfg!(feature = "ee2-up15") && instant) || e.qt >= {
                    #[cfg(feature="ee2-paidqueue7")] {job_limit}
                    #[cfg(not(feature="ee2-paidqueue7"))] {EPOCH_TIME}
                } {
                    #[cfg(feature="ee2-paidqueue7")] let previous_epoch=self.players[team].epoch;
                    #[cfg(feature="ee2-ai9-game-bridge")]
                    self.reset_source_rank_for_epoch(e.team);
                    self.players[team].epoch += 1;
                    self.players[team].researching = false;
                    let old=self.max_hp(e.team,e.def);self.rebuild_modifiers(e.team,true);e.hp*=self.max_hp(e.team,e.def)/old.max(0.01);
                    #[cfg(feature="ee2-paidqueue7")] self.apply_paid_main(team as u8,previous_epoch);
                    // Main may reprice/remove this city's pending jobs while
                    // update_building holds its active order in a local copy.
                    #[cfg(feature="ee2-paidqueue7")]
                    {e.q=self.ents[i].q;e.qn=self.ents[i].qn;}
                    done = true;
                }
            } else if original_research(item) {
                #[cfg(feature = "ee2-reference")]
                {
                    let k=(item-ORIGINAL_RESEARCH0) as usize;
                    e.qt += DT;
                    if (cfg!(feature = "ee2-up15") && instant) || e.qt + 0.00001 >= {
                        #[cfg(feature="ee2-paidqueue7")] {job_limit}
                        #[cfg(not(feature="ee2-paidqueue7"))] {crate::ee2_research::NODES[k].time}
                    } {
                        self.researched[team][k/32] |= 1 << (k%32);
                        let old=self.max_hp(e.team,e.def);
                        self.rebuild_modifiers(e.team,true);
                        e.hp *= self.max_hp(e.team,e.def)/old;
                        done=true;
                    }
                }
            } else if item >= TECH0 {
                e.qt += DT;
                let k = item - TECH0;
                if !self.net && self.cheats_enabled && e.team == self.local {
                    let used = self.tech_points.min((TECHS[k as usize].time - e.qt).max(0.0).ceil() as u32);
                    e.qt += used as f32;
                    self.tech_points -= used;
                }
                if e.qt >= TECHS[k as usize].time {
                    let p = &mut self.players[team];
                    p.tech |= 1 << k;
                    p.tech_busy &= !(1 << k);
                    p.recompute();
                    done = true;
                }
            } else {
                let ud = &DEFS[item as usize];
                #[cfg(feature="ee2-paidqueue7")] let time=job_limit;
                #[cfg(not(feature="ee2-paidqueue7"))] let time=self.production_time(e.team,item as DefId);
                if instant { e.qt = time; }
                if e.qt < time {
                    #[cfg(feature="ee2-paidqueue7")] {e.qt=(e.qt+if job_start{0.009999999776482582_f32}else{DT}).min(time);}
                    #[cfg(not(feature="ee2-paidqueue7"))] {e.qt+=DT;}
                }
                // population was already reserved when queued; cap can still drop if houses die
                if e.qt >= time && (self.players[team].pop <= self.players[team].cap || self.bench > 0) {
                    let (sx, sy) = self.spawn_point(e, d, ud);
                    #[cfg(feature="ee2-citizen9")] let source_identity=if item==WORKER as QueueItem{Some(self.source_birth.production_identity())}else{None};
                    let u = self.spawn(item as DefId, e.team, sx, sy);
                    #[cfg(feature="ee2-citizen9")] if u!=NOIDX{if let Some(identity)=source_identity{self.set_citizen_identity(u,identity);}}
                    if u != NOIDX && e.rx >= 0.0 {
                        let rt = tile_of(e.rx, e.ry);
                        if ud.has(F_WORKER) && self.map.res[rt] != R_NONE {
                            self.order_gather(&[u], rt);
                        } else {
                            self.order_move(&[u], e.rx, e.ry, false);
                        }
                    }
                    #[cfg(not(feature="ee2-citizen9"))] {done = true;}
                    #[cfg(feature="ee2-citizen9")] {done = u!=NOIDX;}
                }
            }
            if done {
                if e.team == self.local {
                    self.ev(if original_research(item) || item >= TECH0 { 13.0 } else { 12.0 }, e.x, e.y);
                }
                #[cfg(feature="ee2-paidqueue7")] self.remove_queue_payment(((e.gen as u32)<<16)|i as u32,0);
                e.q.copy_within(1.., 0);
                e.qn -= 1;
                e.qt = 0.0;
            }
        }
        if d.armed() {
            let mut tj = NOIDX;
            if let Some(j) = self.get(e.tgt) {
                if self.hostile(e.team,self.ents[j].team)&&self.can_see(e.team, j)&&self.rps_allows(e,&self.ents[j]) {
                    tj = j;
                }
            }
            if tj == NOIDX && (self.tick + i as u32) % 8 == 0 {
                e.tgt = self.acquire(e, d, d.max_range()+self.modifiers[e.team as usize][e.def as usize][6]);
                tj = self.get(e.tgt).unwrap_or(NOIDX);
            }
            if tj != NOIDX {
                let t = self.ents[tj];
                let td = &DEFS[t.def as usize];
                let dist = ((t.x - e.x).powi(2) + (t.y - e.y).powi(2)).sqrt() - self.entity_radius(&t);
                match d.weapon_vs(td.dom) {
                    Some(w) if (dist <= w.range+self.modifiers[e.team as usize][e.def as usize][6] && !self.native_too_close(e,&t,dist))||self.native_round_locked(i,e) => {
                        if cfg!(feature="ee2-up15")||e.cd <= 0.0 {
                            let w = *w;
                            self.try_attack_round(i,e, &w, tj);
                        }
                    }
                    _ => {self.native_stop_attack(i);e.tgt = NONE;},
                }
            }else{self.native_stop_attack(i);}
        }
    }

    fn spawn_point(&self, b: &Ent, _bd: &Def, ud: &Def) -> (f32, f32) {
        if ud.dom == AIR {
            return (b.x, b.y);
        }
        let pass = &self.map.pass[if ud.dom == LAND { 0 } else { 1 }];
        let (cx, cy) = (b.x.floor() as i32, b.y.floor() as i32);
        let (wx, wy) = if b.rx >= 0.0 { (b.rx, b.ry) } else if b.def==DOCK{(b.x+b.rot.sin()*50.0,b.y-b.rot.cos()*50.0)}else{(b.x-b.rot.sin()*50.0,b.y+b.rot.cos()*50.0)};
        let r0 = Self::building_half(b) as i32 + 1;
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

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn regicide_spawn_point(&self, city: &Ent) -> Option<(f32, f32)> {
        let king = &DEFS[crate::ee2_regicide::KING as usize];
        let point = self.spawn_point(city, &DEFS[TC as usize], king);
        self.map.pass[0][tile_of(point.0, point.1)].then_some(point)
    }

    fn update_ent(&mut self, i: usize) {
        let mut e = self.ents[i];
        #[cfg(feature="ee2-pack8")] self.pack_tick_actor(i,&e);
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
                Some(j) if self.hostile(e.team,self.ents[j].team)&&self.can_see(e.team, j)&&self.rps_allows(&e,&self.ents[j]) => {
                    e.tgt = e.otgt;
                    if (self.tick + i as u32) % 32 == 0 {
                        e.fkey = self.key_for(d.dom, self.ents[j].x, self.ents[j].y);
                    }
                }
                _ => {
                    Self::set_idle(&mut e);
                    #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
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
                Some(j) if self.hostile(e.team,self.ents[j].team)&&self.can_see(e.team, j)&&self.rps_allows(&e,&self.ents[j]) => tj = j,
                _ => {self.native_stop_attack(i);e.tgt = NONE;},
            }
        }
        if tj == NOIDX && stagger && d.armed() && e.order <= O_AMOVE {
            let range=d.max_range()+self.modifiers[e.team as usize][e.def as usize][6];
            let r = if e.order == O_MOVE { range } else { self.modifiers[e.team as usize][e.def as usize][5].max(range + 1.0) };
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
                let edge_distance=dist-self.entity_radius(&t);
                let too_close=self.native_too_close(&e,&t,edge_distance);
                let round_locked=self.native_round_locked(i,&e);
                if (edge_distance <= w.range+self.modifiers[e.team as usize][e.def as usize][6] && !too_close)||round_locked {
                    face = Some(ang);
                    engaged = e.order != O_MOVE;
                    goal = (t.x, t.y);
                    let aligned = self.native_round_locked(i,&e)||!plane || w.kind == W_BOMB || angdiff(ang, e.rot).abs() < 0.5;
                    let allowed_on_move={
                        #[cfg(feature="ee2-up15")] {e.order!=O_MOVE||crate::ee2_up15_attack::lookup(e.def,self.players[e.team as usize].epoch).map_or(false,|a|a.attack_on_move)}
                        #[cfg(not(feature="ee2-up15"))] {true}
                    };
                    if (cfg!(feature="ee2-up15")||e.cd<=0.0)&&aligned&&allowed_on_move{
                        self.try_attack_round(i,&mut e,&w,tj);
                    }else if !allowed_on_move{self.native_stop_attack(i);}
                } else if e.order == O_MOVE {
                    self.native_stop_attack(i);e.tgt = NONE;
                } else {
                    self.native_stop_attack(i);
                    let leash = if plane { 30.0 } else { 15.0 };
                    if e.order == O_IDLE && (e.x - e.ox).powi(2) + (e.y - e.oy).powi(2) > leash * leash {
                        // chased too far from our post: walk back
                        #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
                        e.order = O_MOVE;
                        e.fkey = self.key_for(d.dom, e.ox, e.oy);
                    } else {
                        engaged = true;
                        moving = true;
                        goal = (t.x, t.y);
                        want = if too_close {(-dx/dist.max(1e-4),-dy/dist.max(1e-4))} else if e.order == O_ATTACK { self.steer(&mut e, d, t.x, t.y) } else { (dx / dist.max(1e-4), dy / dist.max(1e-4)) };
                    }
                }
            } else {
                #[cfg(feature="ee2-up15")]
            crate::native_attack_clock::stop(&mut self.attack_clock[i].flags);
            e.tgt = NONE;
            }
        }

        // ---- follow orders when not fighting
        if tj==NOIDX{self.native_stop_attack(i);}
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
                        self.unload(i,&mut e);
                        Self::set_idle(&mut e);
                    } else {
                        let (ox, oy) = (e.ox, e.oy);
                        want = self.steer(&mut e, d, ox, oy);
                        moving = true;
                    }
                }
                O_BOARD => self.do_board(i, &mut e, d, &mut want, &mut moving),
                O_GATHER => self.do_gather(i,&mut e, d, &mut want, &mut moving),
                O_BUILD => self.do_build(&mut e, d, &mut want, &mut moving),
                #[cfg(feature = "ee2-native13")]
                O_CAPTURE => {
                    let (direction, movement) = self.do_capture(i, &mut e, d);
                    face = direction;
                    if let Some(velocity) = movement { want = velocity; moving = true; }
                }
                _ => {}
            }
        }

        #[cfg(feature="ee2-pack8")]
        if moving&&self.pack_wait_for_move(i,&e,want.0,want.1){moving=false;want=(0.0,0.0);}
        let speed=self.modifiers[e.team as usize][e.def as usize][2];
        // ---- integrate
        if plane {
            #[cfg(feature = "ee2-reference")]
            let angular_speed = {
                #[cfg(feature="ee2-up15")] {crate::ee2_profile::turn_rate_at(e.def,self.players[e.team as usize].epoch)}
                #[cfg(not(feature="ee2-up15"))] {crate::ee2_profile::turn_rate(e.def)}
            };
            #[cfg(not(feature = "ee2-reference"))]
            let angular_speed = 2.4;
            let rate = angular_speed * DT;
            let (dx, dy) = (goal.0 - e.x, goal.1 - e.y);
            let dist = (dx * dx + dy * dy).sqrt();
            let diff = angdiff(dy.atan2(dx), e.rot);
            let rturn = speed / angular_speed.max(0.001);
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
            e.vx = e.rot.cos() * speed;
            e.vy = e.rot.sin() * speed;
            e.x = (e.x + e.vx * DT).clamp(0.6, N as f32 - 0.6);
            e.y = (e.y + e.vy * DT).clamp(0.6, N as f32 - 0.6);
        } else {
            let (tvx, tvy) = if moving { (want.0 * speed, want.1 * speed) } else { (0.0, 0.0) };
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
                if moved < speed * DT * 0.25 && near {
                    e.stuck += 1;
                    if e.stuck > 24 {
                        Self::set_idle(&mut e); // goal is crowded or walled off: close enough
                    }
                } else {
                    e.stuck = 0;
                }
            }
        }
        #[cfg(feature="ee2-pack8")] self.pack_finish_actor(i,&e,engaged||e.order!=O_IDLE,moving);
        self.ents[i] = e;
    }

    fn do_gather(&mut self, index:usize,e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let kind=e.q[0] as u8;
        if cfg!(feature="ee2-reference")&&e.carry>0.0&&e.q[2]>0&&e.q[2]!=kind as QueueItem {e.q[1]=1;}
        if cfg!(feature="ee2-reference")&&e.q[1]==1{self.return_cargo(e,d,want,moving);return;}
        if !self.citizen_can_gather(e.team,kind){e.carry=0.0;e.q[2]=0;Self::set_idle(e);return;}
        let (cx, cy, reach, farm) = if e.otgt != NONE {
            match self.get(e.otgt) {
                Some(j) if self.ents[j].built >= 1.0 && self.ents[j].team==e.team && self.ents[j].def==FARM => (self.ents[j].x, self.ents[j].y, Self::building_half(&self.ents[j]) + 0.95, j),
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
        let p = self.players[e.team as usize];
        let resource_mod=self.modifiers[e.team as usize][e.def as usize][crate::economy::gather_modifier(kind)];
        #[cfg(not(feature="ee2-up15"))]
        let rate=if cfg!(feature="ee2-reference"){if farm!=NOIDX{4.37}else{0.85}}else{0.55*(1.0+0.1*p.epoch as f32)};
        #[cfg(feature="ee2-up15")]
        let rate=crate::ee2_units::lookup_class(e.def,p.epoch).map(|r|if farm!=NOIDX{r.farm_rate.unwrap_or(0.0)}else{r.gather.and_then(|v|v.get(kind as usize-1).copied()).unwrap_or(0.0)}).unwrap_or(0.0);
        let amount = rate * p.gather * resource_mod * DT;
        if cfg!(feature="ee2-reference"){
            if farm==NOIDX {
                let limit=if kind==R_WOOD{3}else{6};
                let occupying=self.ents[..index].iter().filter(|w|w.alive&&w.def==WORKER&&w.order==O_GATHER&&w.q[1]==0&&w.otgt==NONE&&w.otile==e.otile&&(w.x-cx).abs().max((w.y-cy).abs())<=reach).count();
                if occupying>=limit{return;}
            }
            if farm!=NOIDX {
                if self.ents[farm].carry>=1.0{return;}self.ents[farm].carry+=1.0;
            }
            let carry_limit={
                #[cfg(feature="ee2-up15")] {crate::ee2_units::lookup_class(e.def,p.epoch).and_then(|r|if farm!=NOIDX{r.farm_carry}else{r.carry}).unwrap_or(0.0)}
                #[cfg(not(feature="ee2-up15"))] {15.0}
            };
            let previous=e.carry.floor();e.carry=(e.carry+amount).min(carry_limit);e.q[2]=kind as QueueItem;
            if farm==NOIDX {
                let t=e.otile as usize;let removed=(e.carry.floor()-previous) as u16;
                self.map.amt[t]=self.map.amt[t].saturating_sub(removed);
                if self.map.amt[t]==0 {self.map.res[t]=R_NONE;if self.map.ter[t]==T_FOREST{self.map.ter[t]=T_GRASS;}self.map.recolor(t);e.q[1]=1;}
            }
            if e.carry>=carry_limit {e.q[1]=1;}
            return;
        }
        if farm != NOIDX {
            // a farm supports four workers; extras wait their turn
            if self.ents[farm].carry < 4.0 {
                self.ents[farm].carry += 1.0;
                self.add_resource(e.team,0,amount);
            }
            return;
        }
        self.add_resource(e.team,kind as usize-1,amount);
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
        if self.ents[j].team!=e.team{Self::set_idle(e);e.otgt=NONE;return;}
        let b = self.ents[j];
        let (bx,by,side)=Self::building_bounds(&b);
        if b.built<1.0&&e.x>=bx as f32&&e.x<(bx+side)as f32&&e.y>=by as f32&&e.y<(by+side)as f32{
            let goal=(e.fkey&0xffff)as usize;
            *want=self.steer(e,d,(goal%N)as f32+0.5,(goal/N)as f32+0.5);*moving=true;return;
        }
        if b.built >= 1.0 {
            let farm_staffed=cfg!(feature="ee2-reference")&&self.ents[..self.high].iter().any(|w|w.alive&&w.def==WORKER&&w.order==O_GATHER&&w.otgt==e.otgt);
            if b.def == FARM&&!farm_staffed {
                e.order = O_GATHER;
                e.q[0] = R_FOOD as QueueItem;
            } else if e.prev != R_NONE {
                // job done: go back to the resource we were working before
                let t = e.otile as usize;
                e.order = O_GATHER;
                e.otgt = NONE;
                e.q[0] = e.prev as QueueItem;
                e.fkey = self.key_for(LAND, (t % N) as f32 + 0.5, (t / N) as f32 + 0.5);
            } else {
                Self::set_idle(e);
            }
            return;
        }
        if (b.x - e.x).abs().max((b.y - e.y).abs()) > Self::building_half(&b) + 0.95 {
            let goal=(e.fkey&0xffff)as usize;
            *want = self.steer(e, d, (goal%N)as f32+0.5, (goal/N)as f32+0.5);
            *moving = true;
            return;
        }
        let maxhp=self.max_hp(e.team,b.def);
        let step = DT / (self.production_time(e.team,b.def)*self.construction_multiplier(&b));
        let t = &mut self.ents[j];
        t.built = (t.built + step).min(1.0);
        t.hp = (t.hp + maxhp * 0.9 * step).min(maxhp);
        #[cfg(feature = "ee2-native13")]
        let city_completed = t.def == TC && t.built >= 1.0;
        if t.built >= 1.0 && t.team == self.local {
            self.ev(14.0, b.x, b.y);
        }
        #[cfg(feature="ee2-reference")]
        self.sync_city_ground_entry(j);
        #[cfg(feature = "ee2-native13")]
        if city_completed { self.name_completed_city(j, self.ents[j]); }
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
        for v in self.team_vis.iter_mut() {
            if *v == 255 {
                *v = 128;
            }
        }
        for e in &self.ents[..self.high] {
            if !e.alive {
                continue;
            }
            let r = self.modifiers[e.team as usize][e.def as usize][5];
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
                for v in &mut self.team_vis[e.team as usize*NN+row+x0..=e.team as usize*NN+row+x1] {
                    *v = 255;
                }
            }
        }
        // Shared exploration is remembered; current unit sight remains a
        // live treaty right and disappears when the agreement ends.
        for donor in 0..self.nplayers{for viewer in 0..self.nplayers{if self.shares_to(donor as u8,viewer as u8){for t in 0..NN{if self.team_vis[donor*NN+t]>0{let v=&mut self.team_vis[viewer*NN+t];*v=(*v).max(128);}}}}}
        self.refresh_display_fog();
        #[cfg(feature = "ee2-native13")]
        self.refresh_city_memory();
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
                let mut def = 0 as DefId;
                for _ in 0..40 {
                    let k = WORKER as u32 + 1 + self.rng.range(DEFS.len() as u32 - WORKER as u32 - 1);
                    let dd = &DEFS[k as usize];
                    let dom = if dd.dom == SUB { SEA } else { dd.dom };
                    if dom == want && dd.epoch >= 4 {
                        def = k as DefId;
                        break;
                    }
                }
                if def == 0 {
                    continue;
                }
                let u = self.spawn(def as DefId, team as u8, x, y);
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

#[cfg(test)]
mod diplomacy_combat_tests {
    use super::*;
    fn projectile(g:&Game,target:usize,splash:f32)->Proj {
        let t=g.ents[target];Proj{x:t.x,y:t.y,px:t.x,py:t.y,tx:t.x,ty:t.y,tgt:g.h(target),speed:14.0,dmg:10.0,kind:W_ARROW,team:0,splash,targets:LAND|SEA|SUB|AIR,
            #[cfg(feature="ee2-reference")]
            attacker_profile:crate::ee2_combat::UNKNOWN_PROFILE,
            #[cfg(feature="ee2-up15")]
            friendly_ratio:0.0,
            #[cfg(feature = "ee2-native13")]
            attack_source: NONE,
            d0:1.0}
    }
    #[test]
    fn treaty_stops_direct_and_splash_projectiles_already_in_flight(){
        let mut g=Game::new(7,1,0,2,0);
        let partner=g.spawn(WORKER,1,100.0,100.0);let opponent=g.spawn(WORKER,2,100.5,100.0);
        g.build_grid();g.update_fog();let hp=g.ents[partner].hp;let hp_other=g.ents[opponent].hp;
        g.projs.push(projectile(&g,partner,0.0));g.projs.push(projectile(&g,partner,2.0));
        assert!(g.apply_diplomacy(0,1,0,0,0.0));assert!(g.apply_diplomacy(1,0,3,0,0.0));
        g.update_projs();assert!(g.projs.is_empty());assert_eq!(g.ents[partner].hp,hp);assert!(g.ents[opponent].hp<hp_other);
    }
    #[test]
    fn revealed_display_does_not_expose_undetected_stealth(){
        let mut g=Game::new(7,0,0,2,0);let def=DEFS.iter().position(|d|d.has(F_STEALTH)).unwrap() as u8;
        let j=g.spawn(def as DefId,1,120.0,120.0);g.diplomacy.visibility=2;g.refresh_display_fog();
        assert!(!g.visible(j));g.ents[j].revealed=1;assert!(g.visible(j));assert!(!g.can_see(0,j));
    }
    #[test]
    fn free_alliance_is_not_a_shared_victory(){
        let mut g=Game::new(7,0,0,2,0);g.diplomacy.relation[1]=crate::diplomacy::ALLIED;g.diplomacy.relation[4]=crate::diplomacy::ALLIED;
        for _ in 0..20{g.tick();}assert_eq!(g.winner,0);
        for e in &mut g.ents[..g.high]{if e.alive&&e.team==1&&DEFS[e.def as usize].building{e.hp=0.0;}}
        for _ in 0..40{g.tick();}assert_eq!(g.winner,1);
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn native_battle_uses_the_projectile_owner_at_launch_for_identity() {
        crate::init(7, 0, 0, 2, 0);
        crate::battle_signals::battle_signals_enable();
        let g = crate::g();
        g.team_vis.fill(255);
        let shooter = g.spawn(10, 1, 100.5, 100.5);
        let victim = g.spawn(WORKER, 0, 125.5, 100.5);
        let mut actor = g.ents[shooter];
        g.fire(shooter, &mut actor, &DEFS[10].w[0], victim);
        assert_eq!(g.projs.len(), 1);
        assert_eq!(g.projs[0].attack_source, g.h(shooter));
        g.ents[shooter].team = 0;
        for _ in 0..100 {
            g.update_projs();
            if g.projs.is_empty() { break; }
        }
        assert!(g.projs.is_empty());
        g.battle_signals.as_mut().unwrap().update_markers(DT);
        let point = crate::battle_signals::battle_signal_cycle();
        assert!(!point.is_null());
        assert_eq!(unsafe { [*point, *point.add(1)] }, [125.5, 100.5]);
    }
}

#[cfg(test)]
mod territory_cargo_tests{
 use super::*;
 #[test]fn unload_at_entity_limit_preserves_unspawned_passengers(){
  let mut g=Game::new(7,0,0,2,0);let city=g.ents[0];let mut e=city;e.ncargo=3;e.cargo[..3].fill(WORKER);e.cargo_hp[..3].copy_from_slice(&[30,60,90]);
  g.free.clear();g.unload(0,&mut e);assert_eq!(e.ncargo,3);assert_eq!(e.cargo_hp[..3],[30,60,90]);
  let slot=g.high as u16;g.free.push(slot);g.unload(0,&mut e);assert_eq!(e.ncargo,2);assert_eq!(e.cargo_hp[..2],[60,90]);assert!(g.ents[slot as usize].alive);assert!((g.ents[slot as usize].hp/g.max_hp(0,WORKER)-0.3).abs()<0.0001);
 }
 #[test]fn transferred_cargo_keeps_people_but_respects_recipient_work_limit(){
  let mut g=Game::new(7,0,0,2,0);let (x,y)=g.map.starts[0];let u=g.spawn(UNIVERSITY,0,x as f32+7.0,y as f32);g.ents[u].ncargo=3;g.ents[u].cargo[..3].fill(WORKER);g.ents[u].cargo_hp[..3].fill(100);let id=g.map.terr[tile_of(x as f32,y as f32)];g.modifiers[1][UNIVERSITY as usize][11]=1.0;
  assert!(g.apply_territory(0,1,0,id as u32,NONE));assert_eq!(g.ents[u].ncargo,3);assert_eq!(g.cargo_capacity(1,UNIVERSITY),1);let before=g.research_points[1];let mut e=g.ents[u];
  for _ in 0..1000{g.update_building(u,&mut e,&DEFS[UNIVERSITY as usize]);}g.ents[u]=e;let made=g.research_points[1]-before;let max=(1000.0*DT*UNIVERSITY_RATE*g.modifiers[1][UNIVERSITY as usize][7]).ceil();assert!(made<=max);assert!(made>0.0);assert_eq!(g.ents[u].ncargo,3);
 }
 #[cfg(feature="ee2-reference")]
 #[test]fn transferred_extractor_does_not_work_above_recipient_limit(){
  let mut g=Game::new(7,0,0,2,0);for p in &mut g.players[..2]{p.epoch=MAX_EPOCH;}for t in 0..2{g.rebuild_modifiers(t,false);}let (x,y)=g.map.starts[0];let u=g.spawn(OIL_RIG,0,x as f32+7.0,y as f32);g.ents[u].ncargo=3;g.ents[u].cargo[..3].fill(WORKER);g.ents[u].cargo_hp[..3].fill(100);let id=g.map.terr[tile_of(x as f32,y as f32)];g.modifiers[1][OIL_RIG as usize][11]=1.0;
  assert!(g.apply_territory(0,1,0,id as u32,NONE));let before=g.resource(1,R_OIL as usize-1);let mut e=g.ents[u];for _ in 0..100{g.update_building(u,&mut e,&DEFS[OIL_RIG as usize]);}let made=g.resource(1,R_OIL as usize-1)-before;let max=(100.0*DT*0.75*g.modifiers[1][OIL_RIG as usize][crate::economy::gather_modifier(R_OIL)]).ceil();assert!(made<=max);assert!(made>0.0);assert_eq!(e.ncargo,3);
 }
}

#[cfg(test)]
mod territory_construction_tests{
 use super::*;
 #[test]fn gifted_construction_keeps_progress_and_health_fraction_and_changes_build_rights(){
  let mut g=Game::new(7,0,0,2,0);let id=g.map.terr[tile_of(g.ents[0].x,g.ents[0].y)];let base=g.ents[0];let house=g.spawn(HOUSE,0,base.x+7.0,base.y);g.ents[house].built=0.4;g.ents[house].hp=g.max_hp(0,HOUSE)*0.37;g.modifiers[1][HOUSE as usize][0]=1000.0;
  let mut at=None;for y in 4..N as i32-8{for x in 4..N as i32-8{if g.map.terr[tile_of(x as f32+1.0,y as f32+1.0)]==id&&g.can_place(HOUSE,x,y,0){at=Some((x,y));break;}}if at.is_some(){break;}}
  let(x,y)=at.unwrap();assert!(!g.can_place(HOUSE,x,y,1));assert!(g.apply_territory(0,1,0,id as u32,NONE));assert!(!g.can_place(HOUSE,x,y,0));assert!(g.can_place(HOUSE,x,y,1));assert_eq!(g.ents[house].built,0.4);assert!((g.ents[house].hp-370.0).abs()<0.001);
 }
}

#[cfg(all(test,feature="ee2-reference"))]
mod territory_resource_epoch_tests{
 use super::*;
 #[test]fn gifted_extractors_wait_for_recipient_epoch_without_losing_crew(){
  for def in [OIL_RIG,URANIUM_MINE]{let mut g=Game::new(7,0,0,2,0);g.players[0].epoch=MAX_EPOCH;g.rebuild_modifiers(0,false);let (x,y)=g.map.starts[0];let u=g.spawn(def,0,x as f32+7.0,y as f32);g.ents[u].ncargo=3;g.ents[u].cargo[..3].fill(WORKER);g.ents[u].cargo_hp[..3].fill(100);let id=g.map.terr[tile_of(x as f32,y as f32)];let kind=Game::extractor_kind(def);let bank=g.resource(1,kind as usize-1);
   assert!(g.apply_territory(0,1,0,id as u32,NONE));let mut e=g.ents[u];for _ in 0..1000{g.update_building(u,&mut e,&DEFS[def as usize]);}assert_eq!(g.resource(1,kind as usize-1),bank);assert_eq!(e.carry,0.0);assert_eq!(e.ncargo,3);
   g.players[1].epoch=crate::economy::REVEAL_EPOCH[kind as usize-1];g.rebuild_modifiers(1,false);for _ in 0..1000{g.update_building(u,&mut e,&DEFS[def as usize]);}assert!(g.resource(1,kind as usize-1)>bank);assert_eq!(e.ncargo,3);
  }
 }
 #[test]fn gifted_worker_discards_expired_cargo_like_normal_return_cargo(){
  let mut g=Game::new(7,0,0,2,0);g.players[0].epoch=6;g.players[1].epoch=9;for t in 0..2{g.rebuild_modifiers(t,false);}let id=g.map.terr[tile_of(g.ents[0].x,g.ents[0].y)];let workers:Vec<usize>=g.ents[..g.high].iter().enumerate().filter(|(_,e)|e.alive&&e.team==0&&e.def==WORKER).map(|(i,_)|i).collect();let iron=workers[0];let wood=workers[1];g.ents[iron].carry=12.0;g.ents[iron].q[1]=1;g.ents[iron].q[2]=R_METAL as QueueItem;g.ents[wood].carry=9.0;g.ents[wood].q[1]=1;g.ents[wood].q[2]=R_WOOD as QueueItem;let bank=g.resource(1,R_METAL as usize-1);
  assert!(g.apply_territory(0,1,0,id as u32,NONE));assert_eq!(g.ents[iron].carry,0.0);assert_eq!(g.ents[iron].q[1],0);assert_eq!(g.ents[iron].q[2],0);assert_eq!(g.resource(1,R_METAL as usize-1),bank);assert_eq!(g.ents[wood].carry,9.0);assert_eq!(g.ents[wood].q[2],R_WOOD as QueueItem);
 }
}

#[cfg(all(test,feature="ee2-up15"))]
#[path="native_attack_tests.rs"]
mod native_attack_tests;
#[cfg(all(test,feature="ee2-roster6"))]
#[path="roster6_classic_tests.rs"]
mod roster6_classic_tests;
#[cfg(all(test,feature="ee2-paidqueue7"))]
#[path="paid_queue7_tests.rs"]
mod paid_queue7_tests;
#[cfg(all(test,feature="ee2-pack8"))]
#[path="pack8_tests.rs"]
mod pack8_tests;
#[cfg(all(test,feature="ee2-pack8",feature="ee2-civ8"))]
#[path="projectiles8_tests.rs"]
mod projectiles8_tests;

