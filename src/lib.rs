//! OpenEpoch — an original epoch-spanning real-time strategy game.
//!
//! This crate is the whole simulation, compiled to WebAssembly with zero
//! dependencies. The page (see `docs/`) owns input, WebGL2 drawing, sound and
//! networking and talks to it through the plain C ABI below.

mod ai;
mod ids;
mod cmd;
mod defs;
mod economy;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
mod ee2_up15_economy;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_economy.rs"]
mod ee2_up15_economy;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
mod ee2_up15_attack;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_attack.rs"]
mod ee2_up15_attack;
#[cfg(feature="ee2-up15")]
mod native_attack_clock;
mod diplomacy;
mod territory;
mod placement;
#[cfg(all(feature = "ee2-reference",not(feature="ee2-up15")))]
mod ee2_profile;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
#[path="ee2_up15_profile.rs"]
mod ee2_profile;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_profile.rs"]
mod ee2_profile;
#[cfg(all(feature = "ee2-reference",not(feature="ee2-up15")))]
mod ee2_research;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
#[path="ee2_up15_research.rs"]
mod ee2_research;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_research.rs"]
mod ee2_research;
#[cfg(all(feature = "ee2-reference",not(feature="ee2-up15")))]
mod ee2_units;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
#[path="ee2_up15_units.rs"]
mod ee2_units;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_units.rs"]
mod ee2_units;
#[cfg(all(feature = "ee2-reference",not(feature="ee2-up15")))]
mod ee2_combat;
#[cfg(all(feature="ee2-up15",not(feature="ee2-roster6")))]
#[path="ee2_up15_combat.rs"]
mod ee2_combat;
#[cfg(all(feature="ee2-roster6",not(feature="ee2-civ8")))]
#[path="ee2_roster6_combat.rs"]
mod ee2_combat;
mod research;
mod game;
mod map;
mod render;
#[cfg(feature="ee2-render-meta7")]
mod render_meta7;
mod save;
#[cfg(feature="ee2-roster6")]
mod save6;
#[cfg(feature="ee2-paidqueue7")]
mod paid_queue7;
#[cfg(all(feature="ee2-paidqueue7",not(feature="ee2-civ8")))]
mod paid_queue7_main;
#[cfg(feature="ee2-civ8")]
#[path="civ8_main.rs"]
mod paid_queue7_main;
#[cfg(feature="ee2-pack8")]
mod pack8;
#[cfg(feature="ee2-pack8")]
mod pack8_defs;
#[cfg(feature="ee2-pack8")]
mod pack8_save;
#[cfg(feature="ee2-pack8")]
mod pack8_render;
#[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
mod navigation8;
#[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
mod projectiles8;
#[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
mod decision_hash8;
#[cfg(feature="ee2-civ8")]
mod match_settings8;
#[cfg(feature="ee2-roster6")]
mod roster6_registry;
#[cfg(feature="ee2-roster6")]
mod roster6_defs;
#[cfg(feature="ee2-roster6")]
mod roster6_placement;
#[cfg(all(test,feature="ee2-roster6"))]
mod roster6_queue_tests;
#[cfg(feature="ee2-reference")]
mod city_ground;
#[cfg(all(test,feature="ee2-reference"))]
mod claim_tests;

use cmd::*;
use core::ptr::addr_of_mut;
use defs::*;
use game::*;
use map::*;

static mut GAME: Option<Box<Game>> = None;
static mut LOADBUF: Vec<u8> = Vec::new();

/// Distinguishes incompatible rulesets in tests and clients.
#[no_mangle]
pub extern "C" fn ruleset() -> u32 { if cfg!(feature="ee2-up15") {2} else if cfg!(feature = "ee2-reference") { 1 } else { 0 } }

/// Public read-only diagnostics for validating imported definitions.
#[no_mangle]
pub extern "C" fn unit_stat(id: u32, field: u32) -> f32 {
    let Some(d) = DEFS.get(id as usize) else { return -1.0; };
    match field { 0 => d.hp, 1 => d.sight, 2 => d.speed, 3 => d.time,
        4 => d.w[0].dmg, 5 => d.w[0].range, 6 => d.w[0].cd,
        7 => d.w[1].dmg,
        8 => {
            #[cfg(feature = "ee2-reference")]
            { crate::ee2_profile::turn_rate(id as DefId) }
            #[cfg(not(feature = "ee2-reference"))]
            { 2.4 }
        },
        _ => -1.0 }
}

fn g() -> &'static mut Game {
    // SAFETY: wasm32-unknown-unknown is single threaded and `init` runs first.
    unsafe { (*addr_of_mut!(GAME)).as_mut().expect("init not called") }
}

/// `humans` = 2 starts a lockstep multiplayer game; `local` is the seat this client plays.
#[no_mangle]
pub extern "C" fn init(seed: u32, n_ai: u32, bench: u32, humans: u32, local: u32) {
    unsafe {
        *addr_of_mut!(GAME) = Some(Box::new(Game::new(seed, n_ai, bench, humans, local)));
        #[cfg(feature="ee2-render-meta7")]
        render_meta7::reset_session();
    }
}

#[no_mangle]
pub extern "C" fn tick() {
    g().tick();
}

/// Simulation time for visual seasons; safe before game initialization.
#[no_mangle]
pub extern "C" fn game_tick()->u32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(0,|game|game.tick) }
}

/// Completed-TC registration adapter with the native lifetime high-water radius.
#[no_mangle]
pub extern "C" fn ground_urban_at(x:f32,y:f32)->u32 {
    #[cfg(feature="ee2-reference")]
    unsafe { return (&*addr_of_mut!(GAME)).as_ref().map_or(0,|game|game.original_urban_at(x,y) as u32); }
    #[cfg(not(feature="ee2-reference"))]
    { let _=(x,y);0 }
}

/// Read-only source matchup diagnostics, independent of the active match.
#[no_mangle]
pub extern "C" fn combat_stat(attacker:u32,attacker_epoch:u32,defender:u32,defender_epoch:u32,field:u32)->f32 {
    #[cfg(feature="ee2-reference")]
    {
        let limit=if cfg!(feature="ee2-civ8"){DEFS.len()as u32}else{57};
        if attacker>=limit||defender>=limit||attacker_epoch>=15||defender_epoch>=15{return -1.0;}
        let a=ee2_combat::profile(attacker as DefId,attacker_epoch as u8);
        let d=ee2_combat::profile(defender as DefId,defender_epoch as u8);
        return match field {
            0=>ee2_combat::base_percent(a,d).map(|value|value as f32*0.01),
            1=>ee2_combat::adjustment_percent(a,d).map(|value|value as f32*0.01),
            2=>ee2_combat::multiplier(a,d),
            3=>ee2_combat::can_target(attacker as DefId,attacker_epoch as u8,defender as DefId,defender_epoch as u8).map(|value|value as u8 as f32),
            _=>None,
        }.unwrap_or(-1.0);
    }
    #[cfg(not(feature="ee2-reference"))]
    {let _=(attacker,attacker_epoch,defender,defender_epoch,field);-1.0}
}

/// Apply symmetric skirmish settings only before the first tick, single player.
#[no_mangle]
pub extern "C" fn skirmish_setup(pop_limit: u32, resources: u32) -> u32 {
    let game = g();
    if game.tick != 0 || game.net || game.bench > 0 { return 0; }
    let limit = match pop_limit { 250 | 500 | 1000 => pop_limit, _ => return 0 };
    let supplies = match resources { 0 => [400.0,400.0,200.0], 1 => [800.0,800.0,400.0], 2 if cfg!(feature="ee2-reference")=>[15000.0,15000.0,7500.0],2 => [1600.0,1600.0,800.0],3 if cfg!(feature="ee2-reference")=>[200.0,200.0,0.0],4 if cfg!(feature="ee2-reference")=>[200.0,200.0,0.0], _ => return 0 };
    for t in 0..game.nplayers {
        game.players[t].population_limit=limit;game.players[t].res=supplies;
        if cfg!(feature="ee2-reference") {
            let common=match resources{2=>10000.0,3=>100.0,4=>100.0,_=>supplies[1]};
            let stone=if resources==4{500.0}else{common};
            game.extra_resources[t]=[stone,common,supplies[2],supplies[2],supplies[2],supplies[2]];
        }
    }
    1
}

#[no_mangle]
pub extern "C" fn epoch_count() -> u32 { EPOCH_NAMES.len() as u32 }
#[no_mangle]
pub extern "C" fn epoch_name_ptr(index:u32)->*const u8 {EPOCH_NAMES.get(index as usize).map_or(core::ptr::null(),|s|s.as_ptr())}
#[no_mangle]
pub extern "C" fn epoch_name_len(index:u32)->u32 {EPOCH_NAMES.get(index as usize).map_or(0,|s|s.len() as u32)}
#[no_mangle]
pub extern "C" fn skirmish_epoch(epoch:u32)->u32 {
    let game=g();if epoch>MAX_EPOCH as u32||game.tick!=0||game.net||game.bench>0{return 0;}
    #[cfg(feature="ee2-paidqueue7")]
    if game.ents[..game.high].iter().any(|e|e.alive&&DEFS[e.def as usize].building&&e.qn>0){return 0;}
    for t in 0..game.nplayers{
        #[cfg(feature="ee2-paidqueue7")] let previous=game.players[t].epoch;
        game.players[t].epoch=epoch as u8;
        game.rebuild_modifiers(t as u8,true);
        #[cfg(feature="ee2-paidqueue7")] game.apply_paid_main(t as u8,previous);
    }
    game.build_grid();game.update_fog();1
}
#[no_mangle]
pub extern "C" fn resource_count()->u32 {economy::RESOURCE_COUNT as u32}
#[no_mangle]
pub extern "C" fn resource_stat(team:u32,slot:u32)->f32 {if team>=4||slot as usize>=economy::RESOURCE_COUNT{-1.0}else{g().resource(team as u8,slot as usize)}}
#[no_mangle]
pub extern "C" fn resource_active(team:u32,slot:u32)->u32 {if team>=4||slot as usize>=economy::RESOURCE_COUNT{0}else{economy::unlocked(slot as usize,g().players[team as usize].epoch) as u32}}
#[no_mangle]
pub extern "C" fn unit_cost_stat(team:u32,id:u32,slot:u32)->f32 {if team>=4||id as usize>=DEFS.len()||slot>=9{-1.0}else{g().unit_cost(team as u8,id as DefId)[slot as usize] as f32}}
#[no_mangle]
pub extern "C" fn resource_kind(tile:u32)->u32 {g().map.res.get(tile as usize).copied().unwrap_or(0) as u32}

#[no_mangle]
pub extern "C" fn unit_epoch(id: u32) -> u32 { DEFS.get(id as usize).map_or(u32::MAX, |d| d.epoch as u32) }

// ---- frame data -----------------------------------------------------------

#[no_mangle]
pub extern "C" fn render(alpha: f32, x0: f32, y0: f32, x1: f32, y1: f32, ghost: i32, gx: i32, gy: i32) -> u32 {
    #[cfg(feature="ee2-render-meta7")]
    unsafe { if (&*addr_of_mut!(GAME)).is_none() { render_meta7::begin_frame(); render_meta7::end_frame(); return 0; } }
    g().render(alpha, x0, y0, x1, y1, ghost, gx, gy)
}
#[no_mangle]
pub extern "C" fn render_rotated(alpha:f32,x0:f32,y0:f32,x1:f32,y1:f32,ghost:i32,gx:i32,gy:i32,turn:u32)->u32{
    #[cfg(feature="ee2-render-meta7")]
    unsafe { if (&*addr_of_mut!(GAME)).is_none() { render_meta7::begin_frame(); render_meta7::end_frame(); return 0; } }
    g().render_with_rotation(alpha,x0,y0,x1,y1,ghost,gx,gy,turn)
}

/// Optional rendering-only metadata ABI. All getters are safe before init.
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_version()->u32{1}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_stride()->u32{render_meta7::STRIDE}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_ptr()->*const render_meta7::Record{render_meta7::ptr()}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_count()->u32{render_meta7::count()}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_len()->u32{render_meta7::count()*render_meta7::STRIDE}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_session()->u32{render_meta7::session()}
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn render_meta_frame()->u32{render_meta7::frame()}
/// Liveness of an opaque full generational handle. It is not a Death event.
#[cfg(feature="ee2-render-meta7")]
#[no_mangle] pub extern "C" fn entity_alive(handle:u32)->u32{
    unsafe{(&*addr_of_mut!(GAME)).as_ref().is_some_and(|game|game.get(handle).is_some())as u32}
}

#[no_mangle]
pub extern "C" fn inst_ptr() -> *const f32 {
    g().inst.as_ptr()
}
#[no_mangle]
pub extern "C" fn ranges_ptr() -> *const u32 {
    g().ranges.as_ptr()
}
#[no_mangle]
pub extern "C" fn map_ptr() -> *const u8 {
    g().map.rgba.as_ptr()
}
/// Read-only tile classes for rendering; these are independent of minimap colours.
#[no_mangle]
pub extern "C" fn terrain_types_ptr() -> *const u8 {
    g().map.ter.as_ptr()
}
#[no_mangle]
pub extern "C" fn terrain_types_len() -> u32 {
    g().map.ter.len() as u32
}
/// Returns 1 once after the terrain texture changed.
#[no_mangle]
pub extern "C" fn map_dirty() -> u32 {
    let d = g().map.dirty;
    g().map.dirty = false;
    d as u32
}
#[no_mangle]
pub extern "C" fn hmap_ptr() -> *const f32 {
    g().map.hm.as_ptr()
}
#[no_mangle]
pub extern "C" fn fog_ptr() -> *const u8 {
    g().vis.as_ptr()
}
#[no_mangle]
pub extern "C" fn terr_dirty() -> u32 {
    g().terr_dirty as u32
}
#[no_mangle]
pub extern "C" fn terr_ptr() -> *const u8 {
    g().terr_tex();
    g().terr_rgba.as_ptr()
}
#[no_mangle]
pub extern "C" fn minimap() -> *const u8 {
    g().minimap();
    g().mini.as_ptr()
}
#[no_mangle]
pub extern "C" fn ui() -> *const u8 {
    g().build_ui();
    g().ui.as_ptr()
}
#[no_mangle]
pub extern "C" fn ui_len() -> u32 {
    g().ui.len() as u32
}
#[no_mangle]
pub extern "C" fn start_x() -> f32 {
    g().map.starts[g().local as usize].0 as f32
}
#[no_mangle]
pub extern "C" fn start_y() -> f32 {
    g().map.starts[g().local as usize].1 as f32
}
#[no_mangle]
pub extern "C" fn events_ptr() -> *const f32 {
    g().events.as_ptr()
}
#[no_mangle]
pub extern "C" fn events_len() -> u32 {
    g().events.len() as u32
}
#[no_mangle]
pub extern "C" fn events_clear() {
    g().events.clear();
}

// ---- save games -----------------------------------------------------------

#[no_mangle]
pub extern "C" fn save() -> *const u8 {
    g().save();
    g().savebuf.as_ptr()
}
#[no_mangle]
pub extern "C" fn save_len() -> u32 {
    g().savebuf.len() as u32
}
/// Reserve `len` bytes for a save file; the page copies the file there, then calls `load_apply`.
#[no_mangle]
pub extern "C" fn load_alloc(len: u32) -> *mut u8 {
    unsafe {
        let b = &mut *addr_of_mut!(LOADBUF);
        b.clear();
        b.resize(len as usize, 0);
        b.as_mut_ptr()
    }
}
#[no_mangle]
pub extern "C" fn load_apply() -> u32 {
    unsafe {
        match Game::load(&*addr_of_mut!(LOADBUF)) {
            Some(game) => {
                *addr_of_mut!(GAME) = Some(Box::new(game));
                #[cfg(feature="ee2-render-meta7")]
                render_meta7::reset_session();
                1
            }
            None => 0,
        }
    }
}

// ---- lockstep multiplayer -------------------------------------------------

#[no_mangle]
pub extern "C" fn outbox_ptr() -> *const u8 {
    g().outbox.as_ptr()
}
#[no_mangle]
pub extern "C" fn outbox_len() -> u32 {
    g().outbox.len() as u32
}
#[no_mangle]
pub extern "C" fn outbox_clear() {
    g().outbox.clear();
}
#[no_mangle]
pub extern "C" fn inbox_alloc(len: u32) -> *mut u8 {
    let b = &mut g().inbox;
    b.clear();
    b.resize(len as usize, 0);
    b.as_mut_ptr()
}
/// Apply the commands in the inbox for player `team`.
#[no_mangle]
pub extern "C" fn net_apply(team: u32) {
    let game=g();if team<game.nplayers as u32{game.net_apply(team as u8);}
}
#[no_mangle]
pub extern "C" fn state_hash() -> u32 {
    g().state_hash()
}

// ---- selection (purely local, never part of the simulation) ---------------

fn selected(g: &Game) -> Vec<usize> {
    (0..g.high).filter(|&i| g.ents[i].alive && g.ents[i].sel && g.ents[i].team == g.local).collect()
}
fn handles(g: &Game, ids: &[usize]) -> Vec<u32> {
    ids.iter().map(|&i| g.h(i)).collect()
}

fn clear_sel(g: &mut Game) {
    for e in g.ents[..g.high].iter_mut() {
        e.sel = false;
    }
}

/// Tell the simulation where the camera is, so clicks hit what the player sees.
#[no_mangle]
pub extern "C" fn set_eye(x: f32, height: f32, y: f32) {
    g().eye = [x, height, y];
}

/// Where a point at altitude `a` appears on the ground plane when seen from the camera.
/// A plane 3.4 tiles up is drawn well "behind" its map position under a tilted camera.
fn seen_at(g: &Game, x: f32, y: f32, a: f32) -> (f32, f32) {
    let e = g.eye;
    if e[1] <= a + 1.0 {
        return (x, y);
    }
    let k = e[1] / (e[1] - a);
    (e[0] + (x - e[0]) * k, e[2] + (y - e[2]) * k)
}

/// Apparent ground-plane position of an entity's body.
fn seen_ent(g: &Game, e: &Ent, d: &Def) -> (f32, f32) {
    let ground = g.map.height(e.x, e.y).max(0.0);
    let a = if d.building {
        if e.def == DOCK { 0.1 } else { ground + 0.2 * d.size }
    } else {
        match d.dom {
            SEA => 0.1,
            SUB => 0.0,
            AIR => 3.4 + ground * 0.6,
            _ => ground + 0.3 * d.size,
        }
    };
    seen_at(g, e.x, e.y, a)
}

/// Entity under the cursor that the player can see. `(x, y)` is a point on the terrain.
fn pick(g: &Game, x: f32, y: f32, own: bool) -> usize {
    let (x, y) = seen_at(g, x, y, g.map.height(x, y).max(0.0));
    let mut best = NOIDX;
    let mut bd = f32::MAX;
    for i in 0..g.high {
        let e = &g.ents[i];
        if !e.alive || (own && e.team != g.local) || !g.visible(i) {
            continue;
        }
        let d = &DEFS[e.def as usize];
        let (ex, ey) = seen_ent(g, e, d);
        let (dx, dy) = ((ex - x).abs(), (ey - y).abs());
        let hit = if d.building { dx.max(dy) <= Game::building_half(e) + 0.2 } else { dx * dx + dy * dy <= (d.rad() + 0.45).powi(2) };
        // units win over the building they stand next to; aircraft win over everything
        let score = dx * dx + dy * dy + if d.building { 100.0 } else if d.dom == AIR { -50.0 } else { 0.0 };
        if hit && score < bd {
            bd = score;
            best = i;
        }
    }
    best
}

/// Box selection. The box is a screen rectangle, which the tilted camera turns
/// into a quadrilateral on the ground: corners are passed in order around it.
#[no_mangle]
pub extern "C" fn select_quad(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, dx: f32, dy: f32, add: u32) {
    let g = g();
    if add == 0 {
        clear_sel(g);
    }
    let flat = |x: f32, y: f32| seen_at(g, x, y, g.map.height(x, y).max(0.0));
    let q = [flat(ax, ay), flat(bx, by), flat(cx, cy), flat(dx, dy)];
    let inside = |x: f32, y: f32| {
        let mut sign = 0.0f32;
        for k in 0..4 {
            let (p, n) = (q[k], q[(k + 1) % 4]);
            let c = (n.0 - p.0) * (y - p.1) - (n.1 - p.1) * (x - p.0);
            if c * sign < 0.0 {
                return false;
            }
            if c != 0.0 {
                sign = c;
            }
        }
        true
    };
    let me = g.local;
    let mut any = false;
    for pass in 0..2 {
        // prefer military units; only grab workers when the box holds nothing else
        for i in 0..g.high {
            let e = &g.ents[i];
            let d = &DEFS[e.def as usize];
            if e.alive && e.team == me && !d.building && (pass == 1 || !d.has(F_WORKER)) {
                let (sx, sy) = seen_ent(g, e, d);
                if inside(sx, sy) {
                    g.ents[i].sel = true;
                    any = true;
                }
            }
        }
        if any {
            break;
        }
    }
}

/// Click selection. `same` = also select every unit of that type inside the view rect.
#[no_mangle]
pub extern "C" fn select_point(x: f32, y: f32, add: u32, same: u32, vx0: f32, vy0: f32, vx1: f32, vy1: f32) {
    let g = g();
    let i = pick(g, x, y, true);
    if add == 0 {
        clear_sel(g);
    }
    if i == NOIDX {
        return;
    }
    if same != 0 {
        let (def, me) = (g.ents[i].def, g.local);
        for e in g.ents[..g.high].iter_mut() {
            if e.alive && e.team == me && e.def == def && e.x >= vx0 && e.x <= vx1 && e.y >= vy0 && e.y <= vy1 {
                e.sel = true;
            }
        }
    } else {
        g.ents[i].sel = !(add != 0 && g.ents[i].sel);
    }
}

#[no_mangle]
pub extern "C" fn select_none() {
    clear_sel(g());
}

#[no_mangle]
pub extern "C" fn select_idle_worker() -> u32 {
    let g = g();
    for i in 0..g.high {
        let e = g.ents[i];
        if e.alive && e.team == g.local && e.def == WORKER && e.order == O_IDLE && !e.sel {
            clear_sel(g);
            g.ents[i].sel = true;
            return i as u32;
        }
    }
    u32::MAX
}

#[no_mangle]
pub extern "C" fn ent_x(i: u32) -> f32 {
    g().ents[i as usize & (MAXE - 1)].x
}
#[no_mangle]
pub extern "C" fn ent_y(i: u32) -> f32 {
    g().ents[i as usize & (MAXE - 1)].y
}

#[no_mangle]
pub extern "C" fn group_set(n: u32) {
    let g = g();
    let hs = handles(g, &selected(g));
    g.groups[n as usize % 10] = hs;
}

/// Recall a control group; returns how many units are still alive in it.
#[no_mangle]
pub extern "C" fn group_get(n: u32) -> u32 {
    let g = g();
    let hs = g.groups[n as usize % 10].clone();
    if hs.is_empty() {
        return 0;
    }
    clear_sel(g);
    let mut c = 0;
    for h in hs {
        if let Some(i) = g.get(h) {
            g.ents[i].sel = true;
            c += 1;
        }
    }
    c
}

/// Centre of the current selection on one axis, or -1 with nothing selected.
#[no_mangle]
pub extern "C" fn sel_center(axis: u32) -> f32 {
    let g = g();
    let ids = selected(g);
    if ids.is_empty() {
        return -1.0;
    }
    let s: f32 = ids.iter().map(|&i| if axis == 0 { g.ents[i].x } else { g.ents[i].y }).sum();
    s / ids.len() as f32
}

// ---- commands: resolved here against what the player sees, then submitted --

/// Right click: attack / gather / build / board / unload / move / set rally.
#[no_mangle]
pub extern "C" fn cmd_context(x: f32, y: f32, attack_move: u32) {
    let g = g();
    let ids = selected(g);
    if ids.is_empty() {
        return;
    }
    let all = handles(g, &ids);
    if DEFS[g.ents[ids[0]].def as usize].building {
        g.submit(C_RALLY, x, y, 0, &all);
        return;
    }
    if attack_move != 0 {
        g.submit(C_AMOVE, x, y, 0, &all);
        return;
    }
    let part = |g: &Game, f: &dyn Fn(&Ent, &Def) -> bool| -> Vec<u32> { ids.iter().copied().filter(|&i| f(&g.ents[i], &DEFS[g.ents[i].def as usize])).map(|i| g.h(i)).collect() };
    let workers = part(g, &|_, d| d.has(F_WORKER));
    let others = part(g, &|_, d| !d.has(F_WORKER));
    let j = pick(g, x, y, false);
    if j != NOIDX {
        let t = g.ents[j];
        let td = &DEFS[t.def as usize];
        let th = g.h(j) as i32;
        if t.team != g.local {
            if g.hostile(g.local,t.team){if g.can_see(g.local,j){g.submit(C_ATTACK,x,y,th,&all);}else{g.submit(C_AMOVE,x,y,0,&all);}}
            else{g.submit(C_MOVE,x,y,0,&all);}return;
        }
        if td.has(F_TRANSPORT) || (td.has(F_RESEARCH|F_EXTRACT|F_DROPOFF|F_GARRISON) && t.built >= 1.0) {
            let land = part(g, &|_, d| d.dom == LAND && (!td.has(F_RESEARCH|F_EXTRACT|F_DROPOFF) || d.has(F_WORKER)));
            if !land.is_empty() {
                g.submit(C_BOARD, x, y, th, &land);
                return;
            }
        }
        if td.building && (t.built < 1.0 || t.def == FARM) && !workers.is_empty() {
            g.submit(C_WORK, x, y, th, &workers);
            if !others.is_empty() {
                g.submit(C_MOVE, x, y, 0, &others);
            }
            return;
        }
    }
    let tile = tile_of(x, y);
    if g.map.res[tile] != R_NONE && g.real_fog(g.local,tile) > 0 && !workers.is_empty() {
        if !g.citizen_can_gather(g.local,g.map.res[tile]){return;}
        g.submit(C_GATHER, x, y, tile as i32, &workers);
        if !others.is_empty() {
            g.submit(C_MOVE, x, y, 0, &others);
        }
        return;
    }
    // loaded transports sent onto land put their passengers ashore there
    let loaded = part(g, &|e, d| d.has(F_TRANSPORT) && e.ncargo > 0);
    if !loaded.is_empty() && g.map.ter[tile] >= T_SAND {
        let rest = part(g, &|e, d| !(d.has(F_TRANSPORT) && e.ncargo > 0));
        g.submit(C_UNLOAD, x, y, 0, &loaded);
        if !rest.is_empty() {
            g.submit(C_MOVE, x, y, 0, &rest);
        }
        return;
    }
    g.submit(C_MOVE, x, y, 0, &all);
}

#[no_mangle]
pub extern "C" fn cmd_stop() {
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_STOP, 0.0, 0.0, 0, &hs);
}

/// Explicit manual Pack/Unpack order; the usual lockstep command path owns it.
#[cfg(feature="ee2-pack8")]
#[no_mangle]
pub extern "C" fn cmd_pack(unpacked:u32)->u32{
 if unpacked>1{return 0;}let g=g();let ids:Vec<usize>=selected(g).into_iter().filter(|&i|g.pack_spec(&g.ents[i]).is_some_and(|s|s.capable)).collect();
 let hs=handles(g,&ids);if hs.is_empty(){return 0;}g.submit(C_PACK,0.0,0.0,unpacked as i32,&hs);hs.len()as u32
}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn pack_selected_count()->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(0,|game|selected(game).into_iter().filter(|&i|game.pack_spec(&game.ents[i]).is_some_and(|s|s.capable)).count()as u32)}
}

#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn pack_schema_revision()->u32{if cfg!(feature="ee2-citizen9"){4}else if cfg!(feature="ee2-civ8"){3}else{1}}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn render_pack_version()->u32{1}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn render_pack_stride()->u32{pack8_render::STRIDE}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn render_pack_ptr()->*const pack8_render::Record{pack8_render::ptr()}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn render_pack_count()->u32{pack8_render::count()}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn render_pack_len()->u32{pack8_render::count()*pack8_render::STRIDE}

/// Unload selected transports where they are.
#[no_mangle]
pub extern "C" fn cmd_unload() {
    let g = g();
    for i in selected(g) {
        if g.ents[i].ncargo > 0 {
            let (x, y, h) = (g.ents[i].x, g.ents[i].y, g.h(i));
            g.submit(C_UNLOAD, x, y, 0, &[h]);
        }
    }
}

/// Why a building cannot go here: 0 ok, 1 terrain/blocked, 2 not our territory, 3 unexplored, 4 cost.
#[no_mangle]
pub extern "C" fn place_error(def: u32, tx: i32, ty: i32) -> u32 {
    place_error_rotated(def,tx,ty,0)
}
#[no_mangle]
pub extern "C" fn place_error_rotated(def:u32,tx:i32,ty:i32,turn:u32)->u32{
    if def>=DEFS.len()as u32||turn>=placement::ROTATION_STEPS{return 1;}
    let game=g();let ids=selected(game);game.placement_error_for(game.local,def as DefId,tx,ty,turn,&ids,true)
}
#[no_mangle]
pub extern "C" fn footprint_size(def:u32,turn:u32)->u32{if def>=DEFS.len()as u32{return 0;}Game::footprint_side(def as DefId,turn)as u32}
#[no_mangle]
pub extern "C" fn dock_orientation(tx:i32,ty:i32)->u32{g().dock_auto_turn(tx,ty)}
#[no_mangle]
pub extern "C" fn placement_time_multiplier(def:u32,tx:i32,ty:i32,turn:u32)->f32{
    if def>=DEFS.len()as u32{return 0.0;}let side=Game::footprint_side(def as DefId,turn);if side==0||tx<0||ty<0||tx>N as i32-side||ty>N as i32-side{return 0.0;}let game=g();if def==TC as u32{return 1.0;}let owner=game.towner[game.map.terr[(ty+side/2)as usize*N+(tx+side/2)as usize]as usize];if owner==game.local{1.0}else if owner==255{2.0}else{3.0}
}
#[no_mangle]
pub extern "C" fn cmd_place(def:u32,tx:i32,ty:i32)->u32{cmd_place_rotated(def,tx,ty,0)}
#[no_mangle]
pub extern "C" fn cmd_place_rotated(def:u32,tx:i32,ty:i32,turn:u32)->u32{
    if place_error_rotated(def,tx,ty,turn)!=0{return 0;}let game=g();let hs=handles(game,&selected(game));let kind=if turn==0{C_PLACE}else{C_PLACE_ROTATED};game.submit(kind,tx as f32,ty as f32,(def|(turn<<if cfg!(feature="ee2-roster6"){16}else{8}))as i32,&hs);1
}

#[no_mangle]
pub extern "C" fn cmd_train(def: u32) -> u32 {
    if def>if cfg!(feature="ee2-roster6"){u16::MAX as u32}else{255} {return 0;}
    let g = g();
    let def = def as QueueItem;
    // spread the order over the selected buildings: shortest queue first
    let best = selected(g).into_iter().filter(|&i| g.can_train(i, def)).min_by_key(|&i| g.ents[i].qn);
    match best {
        Some(b) => {
            let h = g.h(b);
            g.submit(C_TRAIN, 0.0, 0.0, def as i32, &[h]);
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn cmd_cancel() {
    let g = g();
    if let Some(b) = selected(g).into_iter().filter(|&i| g.ents[i].qn > 0).max_by_key(|&i| g.ents[i].qn) {
        let h = g.h(b);
        g.submit(C_CANCEL, 0.0, 0.0, 0, &[h]);
    }
}

#[no_mangle]
pub extern "C" fn cmd_delete() {
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_DELETE, 0.0, 0.0, 0, &hs);
}

/// Cheat ids: 0 enable, 1 disable, 2 loot, 3 instant build toggle, 4 technology points.
/// Results: 0 disabled/invalid, 1 applied, 2 toggle off, 3 multiplayer denied.
#[no_mangle]
pub extern "C" fn cheat(code: u32) -> u32 {
    let game = g();
    if game.net { return 3; }
    if code == 0 { game.cheats_enabled = true; return 1; }
    if code == 1 { game.cheats_enabled = false; game.instant_build = false; return 1; }
    if !game.cheats_enabled { return 0; }
    match code {
        2 => {for r in 0..economy::RESOURCE_COUNT{game.add_resource(game.local,r,10000.0);}1},
        3 => { game.instant_build = !game.instant_build; if game.instant_build { 1 } else { 2 } },
        4 => {
            if cfg!(feature = "ee2-reference") { game.research_points[game.local as usize] += 50.0; }
            else { game.tech_points = game.tech_points.saturating_add(50); }
            1
        },
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn cheat_status(field: u32) -> u32 {
    let game = g();
    match field { 0 => game.cheats_enabled as u32, 1 => game.instant_build as u32, 2 => if cfg!(feature = "ee2-reference") {game.research_points[game.local as usize] as u32} else {game.tech_points}, _ => 0 }
}

/// Research from the full tree: use a selected host, otherwise the first suitable building.
#[no_mangle]
pub extern "C" fn cmd_research(k: u32) -> u32 {
    #[cfg(feature = "ee2-reference")]
    {
        if k>=180 { return 0; }
        let game=g(); let item=ORIGINAL_RESEARCH0+k as QueueItem;
        let selected=selected(game);
        let b=selected.iter().copied().chain(0..game.high).find(|&i| game.ents[i].alive && game.ents[i].team==game.local && game.can_train(i,item));
        if let Some(i)=b { let h=game.h(i);game.submit(C_TRAIN,0.0,0.0,item as i32,&[h]);return 1; }
    }
    let _=k; 0
}

#[no_mangle]
pub extern "C" fn research_stat(team: u32, field: u32, k: u32) -> f32 {
    let game=g(); if team as usize>=game.nplayers {return -1.0;}
    let t=team as usize;
    match field {
        0=>game.research_points[t], 1=>game.research_count(team as u8,game.players[t].epoch) as f32,
        2=>game.has_research(team as u8,k as usize) as u32 as f32,
        3=>if (k as usize)<DEFS.len() {game.max_hp(team as u8,k as DefId)}else{-1.0},
        _=>-1.0
    }
}

#[no_mangle]
pub extern "C" fn research_tree() -> *const u8 {
    let game=g(); let mut s=String::from("[");
    #[cfg(feature = "ee2-reference")]
    {
        use std::fmt::Write;
        for (k,n) in crate::ee2_research::NODES.iter().enumerate() {
            let researched=game.has_research(game.local,k); let busy=game.research_busy(game.local,k);
            let host=game.ents[..game.high].iter().any(|e| e.alive && e.team==game.local && e.def==n.host && e.built>=1.0);
            let can=game.ents[..game.high].iter().enumerate().any(|(i,e)| e.alive && e.team==game.local && game.can_train(i,ORIGINAL_RESEARCH0+k as QueueItem));
            let _=write!(s,"{}{{\"id\":{},\"name\":\"{}\",\"effect\":\"{}\",\"epoch\":{},\"branch\":{},\"host\":\"{}\",\"cost\":{},\"done\":{},\"busy\":{},\"hostBuilt\":{},\"ok\":{}}}",if k>0 {","}else{""},k,n.name,n.note,n.epoch,n.branch,DEFS[n.host as usize].name,n.cost,researched,busy,host,can);
        }
    }
    s.push(']');game.ui=s;game.ui.as_ptr()
}

/// Sandbox cheat for single player: grant resources.
#[no_mangle]
pub extern "C" fn grant(amount: f32) {
    let g = g();
    if !g.net {
        for r in 0..economy::RESOURCE_COUNT{g.add_resource(g.local,r,amount);}
    }
}

/// Read-only peek at any player, for tests and tooling.
/// what: 0 epoch, 1 pop, 2 cap, 3..=5 resources, 6 buildings, 7 territories, 8 technologies.
#[no_mangle]
pub extern "C" fn stat(team: u32, what: u32) -> f32 {
    let p = &g().players[team as usize & 3];
    match what {
        0 => p.epoch as f32,
        1 => p.pop as f32,
        2 => p.cap as f32,
        3..=5 => p.res[what as usize - 3],
        6 => p.buildings as f32,
        7 => p.terr as f32,
        _ => p.tech.count_ones() as f32,
    }
}

/// Start display setting; deterministic shared map config is allowed in MP.
#[no_mangle]
pub extern "C" fn skirmish_visibility(mode:u32)->u32 {let game=g();if mode>2||game.tick!=0||game.bench>0{return 0;}game.diplomacy.visibility=mode as u8;game.refresh_display_fog();1}
#[no_mangle]
pub extern "C" fn cmd_diplomacy(target:u32,action:u32,resource:u32,amount:f32)->u32 {
 let game=g();if !game.diplomacy_valid(game.local,target,action,resource,amount){return 0;}
 let bits=target|(action<<8)|(resource<<16);game.submit(C_DIPLOMACY,amount,0.0,bits as i32,&[]);1
}
#[no_mangle]
pub extern "C" fn diplomacy_stat(from:u32,to:u32,field:u32)->u32 {
 let game=g();if from>=game.nplayers as u32||to>=game.nplayers as u32{return u32::MAX;}let i=from as usize*4+to as usize;
 match field{0=>game.diplomacy.relation[i] as u32,1=>game.diplomacy.offers[i] as u32,2=>game.diplomacy.sharing[i] as u32,3=>game.diplomacy.offer_tick[i],4=>game.diplomacy.cooldown[i].saturating_sub(game.tick),5=>game.diplomacy.gifts[i],6=>game.diplomacy.offer_vision[i] as u32,_=>u32::MAX}
}

/// Territory IDs are zero based; unused give/want must be u32::MAX.
/// 0 gift, 1 swap offer, 2 accept, 3 reject, 4 withdraw.
#[no_mangle]
pub extern "C" fn cmd_territory(target:u32,action:u32,give:u32,want:u32)->u32{
 let game=g();if !game.territory_valid(game.local,target,action,give,want){return 0;}
 let enc=|v:u32|if v==u32::MAX{-1.0}else{v as f32};game.submit(C_TERRITORY,enc(give),enc(want),(target|(action<<8)) as i32,&[]);1
}
#[no_mangle]
pub extern "C" fn territory_at(x:f32,y:f32)->u32{if !x.is_finite()||!y.is_finite()||x<0.0||y<0.0||x>=N as f32||y>=N as f32{return u32::MAX;}g().map.terr[tile_of(x,y)] as u32}
#[no_mangle]
pub extern "C" fn territory_count()->u32{g().map.tseeds.len() as u32}
#[no_mangle]
pub extern "C" fn territory_owner(id:u32)->u32{let game=g();if id>=game.map.tseeds.len() as u32{u32::MAX}else{game.towner[id as usize] as u32}}
#[no_mangle]
pub extern "C" fn territory_claim_stat(team:u32,id:u32,field:u32)->f32 {
    let game=g();
    if team>=game.nplayers as u32 || id>=game.map.tseeds.len() as u32 || field>5 {return -1.0;}
    let (team,id)=(team as u8,id as usize);
    match field {0=>game.territory_claim_status(team,id) as f32,1=>game.territory_adjacent_owned(team,id) as u32 as f32,
        _=>game.territory_control_counts(team,id)[match field{2=>0,3=>1,4=>2,_=>3}] as f32}
}
#[no_mangle]
pub extern "C" fn territory_stat(id:u32,field:u32)->f32{let game=g();if id>=game.map.tseeds.len() as u32{return -1.0;}let id=id as usize;let owner=game.towner[id];let counts=game.territory_counts(id,owner);match field{0=>owner as f32,1=>game.map.tseeds[id].0,2=>game.map.tseeds[id].1,3=>counts[0] as f32,4=>counts[1] as f32,5=>counts[2] as f32,_=>-1.0}}
#[no_mangle]
pub extern "C" fn territory_offer_stat(from:u32,to:u32,field:u32)->u32{let game=g();if from>=game.nplayers as u32||to>=game.nplayers as u32{return u32::MAX;}let o=game.territory.offers[from as usize*4+to as usize];match field{0=>o.active as u32,1=>if o.active{o.give as u32}else{u32::MAX},2=>if o.active{o.want as u32}else{u32::MAX},3=>if o.active{1200-game.tick.saturating_sub(o.tick).min(1200)}else{0},4=>o.cooldown.saturating_sub(game.tick),5=>o.tick,_=>u32::MAX}}

#[cfg(all(test,feature="ee2-up15",not(feature="ee2-roster6")))]
mod ee2_up15_oracle_tests;
#[cfg(all(test,feature="ee2-roster6",not(feature="ee2-civ8")))]
mod ee2_roster6_oracle_tests;
#[cfg(all(test,feature="ee2-civ8"))]
mod ee2_civ8_oracle_tests;

/// Candidate-only read-only diagnostics for source-oracle verification.
#[cfg(feature="ee2-up15")]
#[no_mangle]
pub extern "C" fn up15_stat(team:u32,id:u32,field:u32)->f32{
    let game=g();if team as usize>=game.nplayers||id as usize>=DEFS.len(){return -1.0;}
    let epoch=game.players[team as usize].epoch;
    let Some(row)=ee2_units::lookup_class(id as DefId,epoch) else{return -1.0;};
    let d=&DEFS[id as usize];let m=game.modifiers[team as usize][id as usize];
    match field{
        0=>game.max_hp(team as u8,id as DefId),1=>game.production_time(team as u8,id as DefId),
        2=>m[5],3=>m[2],4=>d.w[0].range+m[6],5=>row.reload.unwrap_or(-1.0),
        6=>game.effective_weapon_damage(team as u8,id as DefId,d.w[0].dmg),7=>economy::population(id as DefId,epoch) as f32,
        8=>m[18],9=>game.cargo_capacity(team as u8,id as DefId) as f32,
        10=>row.generation.unwrap_or(-1.0),11=>row.farm_rate.unwrap_or(-1.0),
        12=>row.carry.unwrap_or(-1.0),13=>row.farm_carry.unwrap_or(-1.0),
        14=>ee2_up15_economy::lookup(id as DefId,epoch).and_then(|r|r.dropoff_bonus).unwrap_or(-1.0),
        20..=28=>row.gather.map(|v|v[(field-20)as usize]).unwrap_or(-1.0),_=>-1.0,
    }
}

/// Read-only source diagnostics; does not advance or mutate the simulation.
#[cfg(feature="ee2-up15")]
#[no_mangle]
pub extern "C" fn native_attack_stat(id:u32,epoch:u32,field:u32)->f32{
    if id as usize>=DEFS.len()||epoch>=15{return -1.0;}
    let Some(a)=ee2_up15_attack::lookup(id as DefId,epoch as u8) else{return -1.0;};
    match field{0=>a.present as u8 as f32,1=>a.min_mobile,2=>a.min_immobile,3=>a.angle,
        4=>a.apply_time,5=>a.pack_time,6=>a.unpack_time,7=>a.splash,8=>a.friendly,
        9=>a.missile as u8 as f32,10=>a.attack_on_move as u8 as f32,
        11=>a.auto_target as u8 as f32,12=>a.latent as u8 as f32,13=>a.ordnance as f32,
        14=>a.projectile_speed.unwrap_or(-1.0),15=>a.visual_kind as f32,
        16=>a.mobile as u8 as f32,17=>a.air as u8 as f32,_=>-1.0}
}

#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn schema_revision()->u32{if cfg!(feature="ee2-citizen9"){9}else if cfg!(feature="ee2-civ8"){8}else if cfg!(feature="ee2-paidqueue7"){7}else{6}}
#[cfg(feature="ee2-paidqueue7")]
#[no_mangle] pub extern "C" fn queue_price(handle:u32,position:u32,native_resource:u32)->f32{
    let game=g();let Some(b)=game.get(handle)else{return -1.0;};
    if position>=game.ents[b].qn as u32||native_resource>=11||!DEFS[game.ents[b].def as usize].building{return -1.0;}
    game.queue_payment(b,position as usize).price[native_resource as usize]
}
#[cfg(feature="ee2-paidqueue7")]
#[no_mangle] pub extern "C" fn queue_price_flags(handle:u32,position:u32)->u32{
    let game=g();let Some(b)=game.get(handle)else{return u32::MAX;};
    if position>=game.ents[b].qn as u32||!DEFS[game.ents[b].def as usize].building{return u32::MAX;}
    let p=game.queue_payment(b,position as usize);p.inferred as u32|((p.purchased as u32)<<1)
}
#[cfg(feature="ee2-paidqueue7")]
#[no_mangle] pub extern "C" fn queue_duration_raw(handle:u32,position:u32)->f32{
    let game=g();let Some(b)=game.get(handle)else{return -2.0;};
    if position>=game.ents[b].qn as u32||!DEFS[game.ents[b].def as usize].building{return -2.0;}
    game.queue_payment(b,position as usize).duration
}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn ranges_len()->u32{(ids::render_capacity()*2)as u32}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn render_asset_kind(id:u32)->u32{ids::render_kind(id)}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn render_asset_source_id(id:u32)->u32{ids::render_source(id)}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn queue_job_encode(kind:u32,id:u32)->u32{ids::encode_job(kind,id).unwrap_or(u32::MAX)}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn cmd_epoch()->u32{cmd_queue_job(ids::EPOCH_JOB)}

/// Selected action buttons retain the original shortest selected producer rule.
/// The full research tree separately keeps cmd_research's global host fallback.
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn cmd_queue_job(item:u32)->u32{
    if ids::decode_job(item).is_none(){return 0;}
    submit_selected_queue_job(g(),item)
}
#[cfg(feature="ee2-roster6")]
fn submit_selected_queue_job(game:&mut Game,item:QueueItem)->u32{
    let best=selected(game).into_iter().filter(|&i|game.can_train(i,item)).min_by_key(|&i|game.ents[i].qn);
    if let Some(i)=best{let h=game.h(i);game.submit(C_TRAIN,0.0,0.0,item as i32,&[h]);1}else{0}
}

#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn render_range_len()->u32{(ids::render_capacity()*2)as u32}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn queue_item_kind(item:u32)->u32{ids::decode_job(item).map_or(u32::MAX,|v|v.0)}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn queue_item_value(item:u32)->u32{ids::decode_job(item).map_or(u32::MAX,|v|v.1)}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn definition_count()->u32{DEFS.len()as u32}
#[cfg(feature="ee2-roster6")]
#[no_mangle] pub extern "C" fn definition_flags(id:u32)->u32{let Some(d)=DEFS.get(id as usize)else{return u32::MAX;};d.building as u32|((!d.building&&d.speed>0.0)as u32)<<1|(d.has(F_WORKER)as u32)<<2|(d.has(F_TRANSPORT)as u32)<<3|(d.armed()as u32)<<4}

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_economy.rs"]
mod ee2_up15_economy;

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_attack.rs"]
mod ee2_up15_attack;

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_profile.rs"]
mod ee2_profile;

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_research.rs"]
mod ee2_research;

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_units.rs"]
mod ee2_units;

#[cfg(feature="ee2-civ8")]
#[path="ee2_civ8_combat.rs"]
mod ee2_combat;

#[cfg(feature="ee2-civ8")]
mod civ8_defs;
#[cfg(feature="ee2-civ8")]
mod civ8_registry;
#[cfg(feature="ee2-civ8")]
mod civ8_gates;
#[cfg(feature="ee2-civ8")]
mod civ8_traits;
#[cfg(feature="ee2-civ8")]
mod civ8_bonus;

#[cfg(feature="ee2-civ8")] mod civ8_runtime;
#[cfg(feature="ee2-civ8")] mod civ8_save;

#[cfg(feature="ee2-civ8")]
#[no_mangle] pub extern "C" fn configure_civilizations(packed:u32)->u32{ if packed&0xffff0000!=0{return 0;}g().configure_civilizations(core::array::from_fn(|i|((packed>>(4*i))&15)as u8))as u32 }
/// Atomic fresh-match SP/MP transaction; rejects without changing any field.
#[cfg(feature="ee2-civ8")]
#[no_mangle]pub extern "C" fn configure_match_settings(packed_civs:u32,pop_limit:u32,supplies:u32,epoch:u32)->u32{
 unsafe{(&mut*addr_of_mut!(GAME)).as_mut().map_or(0,|game|game.configure_match_settings(packed_civs,pop_limit,supplies,epoch)as u32)}
}

#[cfg(feature="ee2-civ8")]
#[no_mangle] pub extern "C" fn civilization_count()->u32{crate::civ8_runtime::COUNT}

#[cfg(feature="ee2-civ8")]
#[no_mangle] pub extern "C" fn civilization_region(civ:u32)->u32{crate::civ8_gates::REGION_BY_CIV.get(civ as usize).map(|&r|r as u32).unwrap_or(u32::MAX)}

#[cfg(feature="ee2-civ8")]
#[no_mangle] pub extern "C" fn player_civilization(team:u32)->u32{g().civilizations.get(team as usize).map(|&c|c as u32).unwrap_or(u32::MAX)}

/// 1 means an imported historical save did not store its pathfinder fields.
#[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
#[no_mangle]pub extern "C" fn navigation_history_unknown()->u32{unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(1,|g|g.navigation_history_unknown as u32)}}
#[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
#[no_mangle]pub extern "C" fn projectile_history_unknown()->u32{unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(1,|g|g.projectile_history_unknown as u32)}}

#[cfg(feature="ee2-civ8")]
#[no_mangle] pub extern "C" fn definition_ttciv(def:u32)->u32{if def as usize>=DEFS.len(){return u32::MAX;}crate::civ8_runtime::source_ttciv(def as DefId)as u32}

#[cfg(all(test,feature="ee2-civ8"))] mod civ8_tests;

#[cfg(all(test,feature="ee2-civ8"))]
#[test]fn combat_diagnostics_cover_eighty_source_ids_without_mutating_the_match(){
 init(7,1,0,1,0);let before=state_hash();let game=g();game.save();let bytes=game.savebuf.clone();
 assert!(combat_stat(79,0,8,0,0)>0.0);assert!(combat_stat(65,5,0,5,0)>0.0);
 assert!(combat_stat(12,5,79,0,3)>=0.0);
 for id in 0..80{for field in 0..4{assert!(combat_stat(id,5,79,5,field).is_finite());}}
 assert_eq!(combat_stat(80,0,0,0,0),-1.0);assert_eq!(combat_stat(0,0,80,0,0),-1.0);
 assert_eq!(state_hash(),before);let game=g();game.save();assert!(game.savebuf==bytes);
}

#[cfg(feature="ee2-citizen9")]mod native_rng9;
#[cfg(feature="ee2-citizen9")]mod citizen9;
#[cfg(feature="ee2-citizen9")]mod citizen9_spec;
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn source_identity_version()->u32{1}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn actor_source_identity(handle:u32)->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().and_then(|game|game.source_identity(handle)).unwrap_or(u32::MAX)}
}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn cargo_source_identity(handle:u32,position:u32)->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().and_then(|game|{
  let i=game.get(handle)?;let e=game.ents[i];if position>=e.ncargo as u32{return None;}
  if e.cargo[position as usize]!=WORKER{return Some(citizen9::NOT_CITIZEN);}
  Some((game.source_birth.slots[i].cargo_females>>position)&1)
 }).unwrap_or(u32::MAX)}
}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn source_rng_state()->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(0,|game|game.source_birth.rng.state())}
}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn source_rng_origin()->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(u32::MAX,|game|game.source_birth.origin)}
}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn selected_actor_handle()->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().and_then(|game|selected(game).first().map(|&i|game.h(i))).unwrap_or(u32::MAX)}
}
#[cfg(feature="ee2-citizen9")]
#[no_mangle]pub extern "C" fn source_rng_history_unknown()->u32{
 unsafe{(&*addr_of_mut!(GAME)).as_ref().map_or(1,|game|u32::from(game.source_birth.origin==1))}
}
