//! OpenEpoch — an original epoch-spanning real-time strategy game.
//!
//! This crate is the whole simulation, compiled to WebAssembly with zero
//! dependencies. The page (see `docs/`) owns input, WebGL2 drawing, sound and
//! networking and talks to it through the plain C ABI below.

mod ai;
#[cfg(feature = "ee2-native13")]
mod air31;
#[cfg(feature = "ee2-native13")]
mod air31_defs;
#[cfg(feature = "ee2-native13")]
mod runway32;
#[cfg(feature = "ee2-native13")]
mod takeoff33;
#[cfg(feature = "ee2-native13")]
mod aircraft_steering;
#[cfg(feature = "ee2-native13")]
mod landing_geometry;
#[cfg(feature = "ee2-native13")]
mod landing34;
#[cfg(feature = "ee2-native13")]
mod air_audio35;
#[cfg(feature = "ee2-native13")]
mod herc_attack66;
#[cfg(feature = "ee2-native13")]
mod net_resign38;
#[cfg(feature = "ee2-native13")]
mod grouped_coop52;
#[cfg(feature="ee2-native13")]
mod initial_ceasefire;
#[cfg(feature = "ee2-native13")]
mod carrier36_defs;
#[cfg(feature = "ee2-native13")]
mod priest42_defs;
#[cfg(feature = "ee2-native13")]
mod spy46_defs;
#[cfg(feature = "ee2-native13")]
mod leader58_defs;
#[cfg(feature = "ee2-native13")]
mod wonders61;
#[cfg(feature = "ee2-native13")]
mod heavy_mounted3_61;
#[cfg(feature = "ee2-native13")]
mod koth64;
#[cfg(feature = "ee2-native13")]
mod crown_victory65;
#[cfg(feature = "ee2-native13")]
mod capitols66;
#[cfg(feature = "ee2-native13")]
mod hotspot_rule66;
#[cfg(feature = "ee2-native13")]
mod hotspot_start66;
#[cfg(feature = "ee2-native13")]
mod hotspot_terrain66;
#[cfg(feature = "ee2-native13")]
mod hotspots66;
#[cfg(feature = "ee2-native13")]
mod rocket_herc;
#[cfg(feature = "ee2-native13")]
mod wonder_open_sides61;
#[cfg(feature = "ee2-native13")]
mod diversion61_defs;
#[cfg(feature = "ee2-native13")]
mod power42;
#[cfg(feature = "ee2-native13")]
mod power42_math;
#[cfg(feature = "ee2-native13")]
mod battle_signals;
#[cfg(feature = "ee2-native13")]
mod capture_animation26;
#[cfg(feature = "ee2-native13")]
mod capture_math25;
#[cfg(feature = "ee2-native13")]
mod infantry_roster27;
#[cfg(feature = "ee2-native13")]
mod production_roster29;
#[cfg(feature = "ee2-native13")]
mod trade30_defs;
#[cfg(feature = "ee2-native13")]
mod trade30_credit;
#[cfg(feature = "ee2-native13")]
mod trade30_queue;
#[cfg(feature = "ee2-native13")]
mod trade30_quote;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_count;
#[cfg(feature = "ee2-ai9-game-bridge")]
mod ai9_game_bridge;
#[cfg(feature = "ee2-ai9-game-bridge")]
mod ai9_game_save;
#[cfg(feature = "ee2-ai9-game-bridge")]
mod ai9_rank;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_player_step;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_planner;
#[cfg(feature = "ee2-ai9-planner-probe")]
mod ai9_probe;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_activation;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_completion;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_debit;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_dispatch;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_start;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_tick;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_research_unlock;
#[cfg(feature = "ee2-ai9-planner-probe")]
pub mod ai9_tech_tree;
mod ids;
mod cmd;
mod defs;
mod economy;
mod source_clock38;
#[cfg(feature = "ee2-native13")]
mod construction_health163;
#[cfg(feature = "ee2-native13")]
mod naval_heave72;
#[cfg(feature = "ee2-native13")]
mod submarine82;
#[cfg(feature = "ee2-native13")]
mod torpedo84;
#[cfg(feature = "ee2-native13")]
mod naval_tilt73;
#[cfg(feature = "ee2-native13")]
use game::fishing81;
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
#[cfg(feature = "ee2-native13")]
mod alliance_territories;
#[cfg(feature = "ee2-native13")]
mod fixed_teams;
#[cfg(feature = "ee2-native13")]
mod timed_treaties;
#[cfg(feature = "ee2-native13")]
mod border_treaties;
#[cfg(feature = "ee2-native13")]
mod ee2_conquest;
#[cfg(feature = "ee2-native13")]
mod ee2_territory_victory;
#[cfg(feature = "ee2-native13")]
mod ee2_sole_survivor;
#[cfg(feature = "ee2-native13")]
mod ee2_regicide;
#[cfg(feature = "ee2-native13")]
mod fullmap_labels21;
#[cfg(feature = "ee2-native13")]
mod territory_name_catalog21;
#[cfg(feature = "ee2-native13")]
mod territory_names21;
#[cfg(feature = "ee2-native13")]
mod city_name_catalog22;
#[cfg(feature = "ee2-native13")]
mod city_names22;
#[cfg(feature = "ee2-native13")]
mod city_memory23;
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
#[cfg(feature = "ee2-authored-valley37")]
mod native_valley37;
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
#[cfg(feature="ee2-epoch-range13")]
mod match_epoch_range13;
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
#[cfg(feature = "ee2-native13")]
static mut AIR_MISSION_POINTS: [f32; air31::MAX_WAYPOINTS * 2] = [0.0; air31::MAX_WAYPOINTS * 2];
#[cfg(feature = "ee2-native13")]
static mut AIR_MISSION_VISUAL_ROUTES: Vec<air31::VisualRoute> = Vec::new();

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
    install_game(Game::new(seed, n_ai, bench, humans, local));
}

/// Opt-in UP startup for three or four distinct human world seats. The
/// legacy entry keeps its original clamping, including fixed multiplayer.
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn init_network_roster(seed: u32, n_ai: u32, humans: u32, local: u32) -> u32 {
    let Some(game) = Game::new_network_roster(seed, n_ai, humans, local) else { return 0; };
    install_game(game);
    1
}

/// Two controllers share the single human world player against one computer.
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn init_network_shared_owner(seed: u32, controller: u32) -> u32 {
    if controller > 1 { return 0; }
    install_game(Game::new_shared_owner_network(seed));
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn network_shared_owner() -> u32 {
    let game = g();
    u32::from(game.net && game.humans == 1)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn init_network_grouped_coop(seed: u32, ai: u32, controllers: u32, packed: u32, local: u32) -> u32 {
    let Some(game) = Game::new_network_grouped_coop(seed, ai, controllers, packed, local) else { return 0; };
    install_game(game);
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn network_grouped_coop_layout() -> u32 {
    g().grouped_coop.map_or(0, grouped_coop52::GroupedCoop::layout)
}

fn install_game(game: Game) {
    unsafe {
        *addr_of_mut!(GAME) = Some(Box::new(game));
        #[cfg(feature="ee2-render-meta7")]
        render_meta7::reset_session();
    }
}

#[no_mangle]
pub extern "C" fn tick() {
    g().tick();
}

/// Experimental solo clock. Takes an unrounded game-time delta in seconds.
/// Returns 1 after one simulation tick. Invalid input changes nothing. An
/// admitted resignation drains the command queue and returns 0 at game end.
#[no_mangle]
pub extern "C" fn tick_with_source_delta(seconds: f64) -> u32 {
    unsafe {
        (*addr_of_mut!(GAME)).as_mut().is_some_and(|game| game.tick_with_source_delta(seconds)) as u32
    }
}

/// An admitted adaptive solo slot. The old wall rate is independent of the
/// newly applied rate and effective scale at a source rate-selection boundary.
#[no_mangle]
pub extern "C" fn tick_with_adaptive_source_slot(wall_rate: u32, rate: u32, scale: f32) -> u32 {
    unsafe {
        (*addr_of_mut!(GAME)).as_mut()
            .is_some_and(|game| game.tick_with_adaptive_source_slot(wall_rate, rate, scale)) as u32
    }
}

/// A negotiated host-authoritative scheduler supplies the round boundary.
/// 0 rejects malformed input, 1 commits a world step, 2 settles a terminal round.
#[no_mangle]
pub extern "C" fn tick_with_network_source_slot(wall_rate: u32, rate: u32, scale: f32, enters_round: u32) -> u32 {
    if enters_round > 1 { return 0; }
    unsafe {
        (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
            game.tick_with_network_source_slot(wall_rate, rate, scale, enters_round != 0))
    }
}

fn adaptive_tempo() -> Option<source_clock38::AdaptiveTempo> {
    unsafe { (&*addr_of_mut!(GAME)).as_ref()?.source_clock?.adaptive }
}

#[no_mangle]
pub extern "C" fn sim_adaptive_rate() -> u32 {
    adaptive_tempo().map_or(0, |tempo| tempo.rate)
}

#[no_mangle]
pub extern "C" fn sim_adaptive_scale() -> f32 {
    adaptive_tempo().map_or(0.0, |tempo| tempo.scale)
}

#[no_mangle]
pub extern "C" fn sim_clock_active() -> u32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().is_some_and(|game| game.source_clock.is_some()) as u32 }
}

#[no_mangle]
pub extern "C" fn sim_elapsed_seconds() -> f32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(0.0, |game| game.sim_elapsed()) }
}

#[no_mangle]
pub extern "C" fn sim_step_seconds() -> f32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(0.0, |game| game.sim_step()) }
}

#[no_mangle]
pub extern "C" fn sim_render_seconds(alpha: f32) -> f32 {
    let alpha = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.0 };
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(0.0, |game| game.render_elapsed(alpha)) }
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
    #[cfg(feature = "ee2-ai9-game-bridge")]
    if game.native_research.is_some() { return 0; }
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
    let game=g();if epoch>game.match_final_epoch() as u32||game.tick!=0||game.net||game.bench>0{return 0;}
    #[cfg(feature = "ee2-ai9-game-bridge")]
    if game.native_research.is_some() { return 0; }
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
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn resource_retirement_version() -> u32 { 1 }

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

#[cfg(feature="ee2-native13")]
#[no_mangle] pub extern "C" fn render_stone_thrower_attack_elapsed(handle:u32,render_time:f32)->f32{
    unsafe{(&*addr_of_mut!(GAME)).as_ref().and_then(|game|game.stone_thrower_attack_elapsed(handle,render_time)).unwrap_or(-1.0)}
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_basis(handle: u32, render_yaw: f32) -> *const f32 {
    static mut BASIS: [f32; 9] = [0.0; 9];
    let basis = (|| {
        let game = unsafe { (&*addr_of_mut!(GAME)).as_ref()? };
        let index = game.get(handle)?;
        if !game.visible(index) { return None; }
        let (id, _, tilt_delta) = game.naval_body_pose(index)?;
        naval_tilt73::naval_surface_basis(id, tilt_delta, render_yaw)
    })();
    let Some(basis) = basis else { return core::ptr::null(); };
    let out = addr_of_mut!(BASIS);
    unsafe { out.write(basis); }
    out.cast()
}

#[cfg(feature = "ee2-native13")]
#[repr(C)]
struct NavalWakeRow {
    handle: u32,
    moving: u32,
    x: f32,
    y: f32,
    basis: [f32; 9],
}

#[cfg(feature = "ee2-native13")]
const _: [(); 52] = [(); core::mem::size_of::<NavalWakeRow>()];
#[cfg(feature = "ee2-native13")]
static mut NAVAL_WAKE_ROWS: Vec<NavalWakeRow> = Vec::new();

#[cfg(feature = "ee2-native13")]
#[repr(C)]
struct NavalWakeFleetRow {
    pose: NavalWakeRow,
    definition: u32,
    team: u32,
    epoch: u32,
}

#[cfg(feature = "ee2-native13")]
const _: [(); 64] = [(); core::mem::size_of::<NavalWakeFleetRow>()];
#[cfg(feature = "ee2-native13")]
static mut NAVAL_WAKE_FLEET_ROWS: Vec<NavalWakeFleetRow> = Vec::new();

#[cfg(feature = "ee2-native13")]
#[repr(C)]
struct NavalWakeSubmarineRow {
    fleet: NavalWakeFleetRow,
    altitude: f32,
}

#[cfg(feature = "ee2-native13")]
const _: [(); 68] = [(); core::mem::size_of::<NavalWakeSubmarineRow>()];
#[cfg(feature = "ee2-native13")]
static mut NAVAL_WAKE_SUBMARINE_ROWS: Vec<NavalWakeSubmarineRow> = Vec::new();

// A single presentation snapshot serves both cameras, including wake tags
// just outside the current model pass. No command or simulation state changes.
#[cfg(feature = "ee2-native13")]
fn collect_naval_wakes(alpha: f32, fleet: bool, submarines: bool, mut push: impl FnMut(NavalWakeFleetRow, f32)) {
    let Some(game) = (unsafe { (&*addr_of_mut!(GAME)).as_ref() }) else { return; };
    if !(0.0..=1.0).contains(&alpha) || game.powers.is_none() { return; }
    if submarines && !game.submarine_depth_enabled { return; }
    let step = game.sim_step();
    let epsilon = f32::from_bits(0x3586_37bd);
    for (i, actor) in game.ents[..game.high].iter().enumerate() {
        if !actor.alive || actor.built < 1.0 || (!fleet && actor.def != 29) { continue; }
        if submarines != submarine82::is_submarine(actor.def) { continue; }
        let Some(player) = game.players.get(usize::from(actor.team)) else { continue; };
        let epoch = player.epoch;
        if !fleet && epoch >= 2 { continue; }
        let Some((id, altitude, tilt_delta)) = game.naval_body_pose(i) else { continue; };
        if !game.can_see(game.local, i) { continue; }
        let yaw_delta = crate::game::angdiff(actor.rot, actor.prot);
        let Some(basis) = naval_tilt73::naval_surface_basis(id, tilt_delta, actor.prot + yaw_delta * alpha) else { continue; };
        // 0085CC93–0085CD32 tests scalar motion and yaw rate. The portable
        // ship stores horizontal velocity and the two committed headings.
        let moving = actor.vx.hypot(actor.vy) >= epsilon || yaw_delta.abs() >= epsilon * step;
        let pose = NavalWakeRow { handle: game.h(i), moving: u32::from(moving),
            x: actor.px + (actor.x - actor.px) * alpha, y: actor.py + (actor.y - actor.py) * alpha, basis };
        push(NavalWakeFleetRow { pose, definition: u32::from(actor.def), team: u32::from(actor.team),
            epoch: u32::from(epoch) }, altitude);
    }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wakes(alpha: f32) -> u32 {
    let rows = unsafe { &mut *addr_of_mut!(NAVAL_WAKE_ROWS) };
    rows.clear();
    collect_naval_wakes(alpha, false, false, |row, _| rows.push(row.pose));
    rows.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wakes_ptr() -> *const u8 {
    let rows = unsafe { &*addr_of_mut!(NAVAL_WAKE_ROWS) };
    if rows.is_empty() { core::ptr::null() } else { rows.as_ptr().cast() }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wake_fleet(alpha: f32) -> u32 {
    let rows = unsafe { &mut *addr_of_mut!(NAVAL_WAKE_FLEET_ROWS) };
    rows.clear();
    collect_naval_wakes(alpha, true, false, |row, _| rows.push(row));
    rows.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wake_fleet_ptr() -> *const u8 {
    let rows = unsafe { &mut *addr_of_mut!(NAVAL_WAKE_FLEET_ROWS) };
    if rows.is_empty() { core::ptr::null() } else { rows.as_ptr().cast() }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wake_submarines(alpha: f32) -> u32 {
    let rows = unsafe { &mut *addr_of_mut!(NAVAL_WAKE_SUBMARINE_ROWS) };
    rows.clear();
    collect_naval_wakes(alpha, true, true, |fleet, altitude| rows.push(NavalWakeSubmarineRow { fleet, altitude }));
    rows.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn render_naval_wake_submarines_ptr() -> *const u8 {
    let rows = unsafe { &*addr_of_mut!(NAVAL_WAKE_SUBMARINE_ROWS) };
    if rows.is_empty() { core::ptr::null() } else { rows.as_ptr().cast() }
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
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fullmap_territory_labels_build() -> u32 {
    g().project_fullmap_territory_labels()
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fullmap_territory_labels_ptr() -> *const u32 {
    g().fullmap_territory_labels.as_ptr().cast()
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn territory_names_catalog_version() -> u32 {
    territory_name_catalog21::VERSION
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_territory_names(version: u32) -> u32 {
    g().configure_territory_names(version) as u32
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fullmap_city_labels_build() -> u32 { g().project_fullmap_city_labels() }
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fullmap_city_labels_ptr() -> *const u32 { g().fullmap_city_labels.as_ptr().cast() }
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn city_names_catalog_version() -> u32 { city_name_catalog22::VERSION }
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_city_names(version: u32) -> u32 { g().configure_city_names(version) as u32 }
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_city_memory() -> u32 { g().configure_city_memory() as u32 }
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
    let game = g();
    #[cfg(feature = "ee2-authored-valley37")]
    if let Some([x, _]) = game.valley_fixture.as_ref().and_then(|state| state.focus(game.local)) { return x; }
    game.map.starts[game.local as usize].0 as f32
}
#[no_mangle]
pub extern "C" fn start_y() -> f32 {
    let game = g();
    #[cfg(feature = "ee2-authored-valley37")]
    if let Some([_, y]) = game.valley_fixture.as_ref().and_then(|state| state.focus(game.local)) { return y; }
    game.map.starts[game.local as usize].1 as f32
}
#[no_mangle]
pub extern "C" fn events_ptr() -> *const f32 {
    g().events.as_ptr()
}
#[no_mangle]
pub extern "C" fn events_len() -> u32 {
    g().events.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_impact_prepare() -> u32 { g().prepare_guided_herc_impacts() }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_impact_ptr() -> *const f32 { g().guided_herc_impacts.as_ptr().cast() }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_impact_visible(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() { return 0; }
    let game = g();
    (game.real_fog(game.local, crate::map::tile_of(x, y)) == 255) as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_attack_prepare() -> u32 {
    let game = g();
    game.prepare_herc_attack();
    game.herc_attack.rows.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_attack_ptr() -> *const u8 { g().herc_attack.rows.as_ptr().cast() }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_attack_shot_at(index: u32) -> u32 { g().herc_attack.shot(index as usize) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_beam_count() -> u32 { g().herc_attack.beams.len() as u32 }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_beam_ptr() -> *const u8 { g().herc_attack.beams.as_ptr().cast() }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn guided_herc_beam_target_point(handle: u32, alpha: f32) -> *const f32 {
    g().sample_herc_beam_target(handle, alpha)
}

#[no_mangle]
pub extern "C" fn events_clear() {
    let game = g();
    game.events.clear();
    #[cfg(feature = "ee2-native13")]
    game.guided_herc_impacts.clear();
    #[cfg(feature = "ee2-native13")]
    game.air_audio.clear_shots();
    #[cfg(feature = "ee2-native13")]
    game.herc_attack.clear_shots();
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_version() -> u32 { 1 }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn submarine_attack_audio_version() -> u32 { 1 }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_prepare(alpha: f32) -> u32 {
    let game = g();
    game.prepare_air_audio(alpha);
    game.air_audio.current.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_current_ptr() -> *const u8 {
    g().air_audio.current.as_ptr().cast()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_transition_count() -> u32 {
    g().air_audio.transitions.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_transition_ptr() -> *const u8 {
    g().air_audio.transitions.as_ptr().cast()
}

/// Game-time seconds for the same row index in the unchanged transition ABI.
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_transition_seconds(index: u32) -> f32 {
    g().air_audio.transition_seconds(index as usize).unwrap_or(-1.0)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_shot_at(index: u32) -> u32 {
    g().air_audio.shot(index as usize)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_audio_shot_duration(index: u32) -> f32 {
    g().air_audio.shot_duration(index as usize)
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

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn save_network_resume() -> *const u8 {
    g().save_network_resume();
    g().savebuf.as_ptr()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn load_network_resume(humans: u32, local: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    load_network_resume_with_teams(humans, local, tick, rate, scale, crown_rules, 0)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn validate_network_resume(humans: u32, local: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    validate_network_resume_with_teams(humans, local, tick, rate, scale, crown_rules, 0)
}

#[cfg(feature = "ee2-native13")]
fn matching_network_resume(humans: u32, local: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32, teams: u32) -> Option<Game> {
    let bytes = unsafe { &*addr_of_mut!(LOADBUF) };
    Game::load_network_resume(bytes, humans, local, tick, rate, scale)
        .filter(|game| crown_resume_matches(game, crown_rules) && game.matches_checkpoint_teams(teams))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn load_network_resume_with_teams(humans: u32, local: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32, teams: u32) -> u32 {
    let Some(game) = matching_network_resume(humans, local, tick, rate, scale, crown_rules, teams) else { return 0; };
    unsafe { *addr_of_mut!(GAME) = Some(Box::new(game)); }
    render_meta7::reset_session();
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn validate_network_resume_with_teams(humans: u32, local: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32, teams: u32) -> u32 {
    u32::from(matching_network_resume(humans, local, tick, rate, scale, crown_rules, teams).is_some())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn save_shared_owner_resume() -> *const u8 {
    g().save_shared_owner_resume();
    g().savebuf.as_ptr()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn load_shared_owner_resume(controller: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    unsafe {
        let Some(game) = Game::load_shared_owner_resume(&*addr_of_mut!(LOADBUF), controller, tick, rate, scale) else { return 0; };
        if !crown_resume_matches(&game, crown_rules) { return 0; }
        *addr_of_mut!(GAME) = Some(Box::new(game));
    }
    render_meta7::reset_session();
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn validate_shared_owner_resume(controller: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    let bytes = unsafe { &*addr_of_mut!(LOADBUF) };
    u32::from(Game::load_shared_owner_resume(bytes, controller, tick, rate, scale)
        .is_some_and(|game| crown_resume_matches(&game, crown_rules)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn grouped_resume_layout() -> u32 { g().grouped_resume_layout().unwrap_or(0) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn save_grouped_resume() -> *const u8 {
    g().save_grouped_resume();
    g().savebuf.as_ptr()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn load_grouped_resume(layout: u32, controller: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    unsafe {
        let Some(game) = Game::load_grouped_resume(&*addr_of_mut!(LOADBUF), layout, controller, tick, rate, scale) else { return 0; };
        if !crown_resume_matches(&game, crown_rules) { return 0; }
        *addr_of_mut!(GAME) = Some(Box::new(game));
    }
    render_meta7::reset_session();
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn validate_grouped_resume(layout: u32, controller: u32, tick: u32, rate: u32, scale: f32, crown_rules: u32) -> u32 {
    let bytes = unsafe { &*addr_of_mut!(LOADBUF) };
    u32::from(Game::load_grouped_resume(bytes, layout, controller, tick, rate, scale)
        .is_some_and(|game| crown_resume_matches(&game, crown_rules)))
}

#[cfg(feature = "ee2-native13")]
fn crown_resume_matches(game: &Game, encoded: u32) -> bool {
    let rules = encoded & 255;
    let victory = (encoded >> 8) & 0x1ffff;
    let capture = (encoded >> 25) & 3;
    let sight = (encoded >> 27) & 1;
    let timed = (encoded >> 28) & 1;
    let borders = (encoded >> 29) & 1;
    let fishing = (encoded >> 30) & 1;
    (rules == 0 && victory == 0 && capture == 0 || rules == game.crown_rules() + 1)
        && victory == (game.crown_victory_config() | game.network_capitol_config())
        && capture == u32::from(game.capture_checkpoint_mode)
        && sight == u32::from(game.diplomacy.sight_terms)
        && timed == u32::from(game.diplomacy.timed.as_ref().is_some_and(|rule| rule.unbreakable))
        && (timed != 0 || game.diplomacy.timed.is_none())
        && borders == u32::from(game.diplomacy.borders.is_some())
        && fishing == u32::from(game.fishing_enabled)
        && encoded >> 31 == 0
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn network_resume_pending() -> u32 {
    u32::from(g().network_resume_tempo.is_some())
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
    let game = g();
    #[cfg(feature = "ee2-native13")]
    if game.grouped_coop.is_some() { return; }
    let limit = if game.net && game.humans != 2 { game.humans } else { game.nplayers as u32 };
    if team < limit { game.net_apply(team as u8); }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn net_apply_grouped_controller(controller: u32) -> u32 {
    let game = g();
    let Some(owner) = game.grouped_coop.and_then(|grouping| grouping.active_owner(controller)) else { return 0; };
    game.net_apply(owner);
    1
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
fn seen_ent(g: &Game, _index: usize, e: &Ent, d: &Def) -> (f32, f32) {
    let ground = g.map.height(e.x, e.y).max(0.0);
    let a = if d.building {
        if e.def == DOCK { 0.1 } else { ground + 0.2 * d.size }
    } else {
        match d.dom {
            SEA => 0.1,
            SUB => 0.0,
            AIR => {
                #[cfg(feature = "ee2-native13")]
                { g.aircraft_height(g.h(_index), 1.0, ground) }
                #[cfg(not(feature = "ee2-native13"))]
                { 3.4 + ground * 0.6 }
            }
            _ => ground + 0.3 * d.size,
        }
    };
    seen_at(g, e.x, e.y, a)
}

/// Entity under the cursor that the player can see. `(x, y)` is a point on the terrain.
fn pick(g: &Game, x: f32, y: f32, own: bool) -> usize {
    pick_where(g, x, y, |_, e| !own || e.team == g.local)
}

fn pick_where(g: &Game, x: f32, y: f32, allowed: impl Fn(usize, &Ent) -> bool) -> usize {
    let (x, y) = seen_at(g, x, y, g.map.height(x, y).max(0.0));
    let mut best = NOIDX;
    let mut bd = f32::MAX;
    for i in 0..g.high {
        let e = &g.ents[i];
        if !e.alive || !allowed(i, e) || !g.visible(i) {
            continue;
        }
        let d = &DEFS[e.def as usize];
        let (ex, ey) = seen_ent(g, i, e, d);
        let (dx, dy) = ((ex - x).abs(), (ey - y).abs());
        let hit = if d.building { Game::building_within(e, dx, dy, 0.2) } else { dx * dx + dy * dy <= (d.rad() + 0.45).powi(2) };
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
            #[cfg(feature = "ee2-native13")]
            if g.trade_hidden(i, e) || g.air_parked(i, e) { continue; }
            if e.alive && e.team == me && !d.building && (pass == 1 || !d.has(F_WORKER)) {
                let (sx, sy) = seen_ent(g, i, e, d);
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
    #[cfg(feature = "ee2-native13")]
    if !g.net {
        let clicked = pick(g, x, y, false);
        g.conversion_click = if clicked == NOIDX { NONE } else { g.h(clicked) };
    }
    let i = pick(g, x, y, true);
    if add == 0 {
        clear_sel(g);
    }
    if i == NOIDX {
        return;
    }
    if same != 0 {
        let (def, me) = (g.ents[i].def, g.local);
        for index in 0..g.high {
            #[cfg(feature = "ee2-native13")]
            if g.trade_hidden(index, &g.ents[index]) || g.air_parked(index, &g.ents[index]) { continue; }
            let e = &mut g.ents[index];
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
    let game = g();
    #[cfg(feature = "ee2-native13")]
    { game.conversion_click = NONE; }
    clear_sel(game);
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

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_capture(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_capture_checkpoint() -> u32 {
    g().configure_network_capture_checkpoint() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_animation_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_capture_animation(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn infantry_roster_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_infantry_roster(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn infantry_roster_enabled() -> u32 {
    u32::from(g().infantry_roster_enabled)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn production_roster_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_production_roster(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_late_population() -> u32 {
    u32::from(g().configure_late_population())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_generated_city_foundations() -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut() }
        .map_or(0, |game| u32::from(game.configure_generated_city_foundations()))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn trade_route_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_trade_route(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn trade_selected_count() -> u32 {
    let game = g();
    selected(game).into_iter().filter(|&i| game.trade_actor_ready(i)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn trade_wait_active(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.ents[index].team == game.local || game.visible(index)) &&
        game.trade_wait_active(handle))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_air_missions(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_runway_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_air_runway(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_takeoff_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_air_takeoff(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_takeoff_start_tick(handle: u32) -> u32 {
    let game = g();
    if game.get(handle).is_some_and(|i| game.ents[i].team == game.local || game.visible(i)) {
        game.takeoff_start_tick(handle)
    } else { NONE }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_takeoff_start_seconds(handle: u32) -> f32 {
    let game = g();
    if game.get(handle).is_some_and(|i| game.ents[i].team == game.local || game.visible(i)) {
        game.takeoff_start_seconds(handle).unwrap_or(-1.0)
    } else { -1.0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_takeoff_height(handle: u32) -> f32 {
    let game = g();
    if game.get(handle).is_some_and(|i| game.ents[i].team == game.local || game.visible(i)) {
        game.takeoff_height(handle, 1.0).unwrap_or(f32::NAN)
    } else { f32::NAN }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_landing_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_air_landing(enabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_carrier_configure_peer(enabled: u32) -> u32 {
    u32::from(enabled <= 1 && g().configure_peer_air_carrier(enabled != 0))
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_solo_command_queue(enabled: u32) -> u32 {
    let game = g();
    if enabled > 1 || game.net || game.bench != 0 || !game.outbox.is_empty() { return 0; }
    game.solo_command_queue = enabled == 1;
    1
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_resign_solo() -> u32 {
    let game = g();
    if game.net || game.bench != 0 || !game.solo_command_queue || game.winner != 0 { return 0; }
    game.submit(C_SOLO_RESIGN, 0.0, 0.0, 0, &[]);
    1
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_resignation(policy: u32) -> u32 {
    u32::from(g().configure_network_resignation(policy))
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_initial_ceasefire(mode: u32) -> u32 {
    u32::from(g().configure_initial_ceasefire(mode))
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn initial_ceasefire_seconds() -> f32 {
    g().initial_ceasefire_seconds()
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_resign_network() -> u32 {
    let game = g();
    if game.grouped_coop.is_some() || !game.network_departure_allowed(game.local) { return 0; }
    game.submit(C_NET_RESIGN, 0.0, 0.0, 0, &[]);
    1
}

#[cfg(feature="ee2-native13")]
#[no_mangle]
pub extern "C" fn network_peer_disconnected() {
    let game = g();
    if game.humans == 2 && game.grouped_coop.is_none() {
        if let Some(peer) = 1u8.checked_sub(game.local) { game.apply_network_departure(peer); }
    }
}

/// Apply the host's ordered loss record at the shared source-round boundary.
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn network_roster_peer_disconnected(seat: u32) -> u32 {
    let game = g();
    if game.humans < 3 || seat == 0 || seat >= game.humans || game.net_resignation.is_none() || game.grouped_coop.is_some() {
        return 0;
    }
    game.apply_network_departure(seat as u8);
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn network_grouped_controller_disconnected(controller: u32) -> u32 {
    u32::from(g().detach_grouped_controller(controller))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_landing_status(handle: u32, field: u32) -> u32 {
    let game = g();
    if game.get(handle).is_some_and(|i| game.visible(i)) {
        game.landing.status(handle, field)
    } else { 0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_status(handle: u32, field: u32) -> u32 {
    g().air_mission_status(handle, field)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_visual_routes() -> u32 {
    let game = g();
    let routes = unsafe { &mut *addr_of_mut!(AIR_MISSION_VISUAL_ROUTES) };
    routes.clear();
    routes.extend((0..game.high).filter(|&index| game.ents[index].alive && game.ents[index].sel)
        .filter_map(|index| game.air_mission_visual_route(game.h(index))));
    routes.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_visual_routes_ptr() -> *const air31::VisualRoute {
    unsafe { (&*core::ptr::addr_of!(AIR_MISSION_VISUAL_ROUTES)).as_ptr() }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_points_ptr() -> *mut f32 {
    addr_of_mut!(AIR_MISSION_POINTS).cast::<f32>()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn air_mission_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() { return NONE; }
    let game = g();
    if !game.air_missions_enabled { return NONE; }
    let index = pick_where(game, x, y, |i, _| game.air_valid_target(game.local, game.h(i)));
    if index == NOIDX { NONE } else { game.h(index) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_air_mission_route(count: u32, target: u32) -> u32 {
    if count as usize > air31::MAX_WAYPOINTS || (count == 0 && target == NONE) { return 0; }
    let game = g();
    if !game.air_missions_enabled || (target != NONE && !game.air_valid_target(game.local, target)) { return 0; }
    let mut payload: Vec<_> = selected(game).into_iter().map(|i| game.h(i))
        .filter(|&handle| game.air_mission_status(handle, 0) == 1).take(200).collect();
    let hosts = payload.len();
    if hosts == 0 { return 0; }
    payload.push(target);
    let points = unsafe { std::slice::from_raw_parts(air_mission_points_ptr(), count as usize * 2) };
    if !points.iter().all(|v| (0.0..N as f32).contains(v)) { return 0; }
    payload.extend(points.iter().map(|v| v.to_bits()));
    game.submit(C_AIR_MISSION, count as f32, hosts as f32, air31::SET_ROUTE, &payload);
    hosts as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_air_mission_action(action: u32) -> u32 {
    if !(air31::CANCEL as u32..=air31::AUTO as u32).contains(&action) { return 0; }
    let game = g();
    if !game.air_missions_enabled { return 0; }
    let hs: Vec<_> = selected(game).into_iter().map(|i| game.h(i)).filter(|&h| {
        let kind = game.air_mission_status(h, 0);
        match action as i32 {
            air31::CANCEL => kind == 1,
            air31::RECALL => kind == 1 && game.air_mission_status(h, 6) != 0 ||
                kind == 2 && game.air_mission_status(h, 5) != 0,
            air31::ADD => kind == 1 && game.air_mission_status(h, 3) != 0 || kind == 2,
            _ => kind == 1 && game.air_mission_status(h, 6) != 0,
        }
    }).collect();
    if hs.is_empty() { return 0; }
    game.submit(C_AIR_MISSION, 0.0, 0.0, action as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn trade_home_valid(home: u32) -> u32 {
    let game = g();
    u32::from(game.get(home).is_some_and(|i| game.trade_host_for_selection(i, NONE, &selected(game))))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn trade_endpoint_at(x: f32, y: f32, home: u32) -> u32 {
    if !x.is_finite() || !y.is_finite() { return NONE; }
    let game = g();
    if !game.trade_enabled { return NONE; }
    let selection = selected(game);
    let endpoint = pick_where(game, x, y, |index, e| {
        matches!(e.def, MARKET | DOCK) && game.trade_host_for_selection(index, home, &selection)
    });
    if endpoint == NOIDX { NONE } else { game.h(endpoint) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_trade_route(home: u32, destination: u32) -> u32 {
    let game = g();
    let ids = game.trade_candidates(game.local, home, destination, &selected(game));
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_TRADE_ROUTE, (home & 0xffff) as f32, (home >> 16) as f32, destination as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_animation_request(handle: u32) -> u32 {
    g().capture_animation_pose(handle).map_or(0, |pose| pose.request())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_animation_elapsed(handle: u32, render_time: f32) -> f32 {
    let game = g();
    let bounds = if game.source_clock.is_some() {
        game.render_elapsed(0.0)..=game.render_elapsed(1.0)
    } else {
        (game.tick as f32 * DT)..=((game.tick as f32 + 1.0) * DT)
    };
    if !bounds.contains(&render_time) { return -1.0; }
    game.capture_animation_started(handle).map_or(-1.0, |at| (render_time - at).max(0.0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_selection_count() -> u32 {
    capture_selection_count_for(0)
}

#[cfg(feature = "ee2-native13")]
fn capture_source_matches(def: DefId, mode: u32) -> bool {
    let priest = def == crate::priest42_defs::PRIEST;
    match mode { 0 => true, 1 => !priest, 2 => priest, _ => false }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_selection_count_for(mode: u32) -> u32 {
    let game = g();
    selected(game).into_iter().filter(|&index| capture_source_matches(game.ents[index].def, mode) &&
        game.capture_actor_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_target_at(x: f32, y: f32) -> u32 {
    capture_target_at_for(x, y, 0)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_target_at_for(x: f32, y: f32, mode: u32) -> u32 {
    if !x.is_finite() || !y.is_finite() || mode > 2 { return 0; }
    let game = g();
    let target = pick(game, x, y, false);
    if target == NOIDX || !game.capture_allowed(game.local, target) { return 0; }
    let selection = selected(game);
    u32::from(if selection.is_empty() { mode == 0 && DEFS[game.ents[target].def as usize].building }
        else { selection.iter().any(|&index| capture_source_matches(game.ents[index].def, mode)
            && game.capture_actor_ready(index) && game.capture_compatible(index, target)) })
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_capture_at(x: f32, y: f32) -> u32 {
    if capture_target_at(x, y) == 0 { return 0; }
    let game = g();
    let victim = pick(game, x, y, false);
    let target = game.h(victim);
    let ids: Vec<_> = selected(game).into_iter().filter(|&index|
        game.capture_actor_ready(index) && game.capture_compatible(index, victim)).collect();
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_CAPTURE, 0.0, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capture_progress(handle: u32) -> f32 {
    let game = g();
    game.get(handle).filter(|&index| game.can_see(game.local, index)).and_then(|index| game.capture_progress(index)).unwrap_or(-1.0)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn has_active_capture_glow() -> u32 {
    u32::from(g().has_active_capture_glow())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn active_capture_glow(handle: u32) -> u32 {
    u32::from(g().active_capture_glow(handle))
}

#[cfg(feature = "ee2-valley71-neutral-qa")]
#[no_mangle]
pub extern "C" fn valley71_force_attack_at(x: f32, y: f32, flags: u32) -> u32 {
    use crate::native_valley37::{FORCE_ATTACK, RAW_ATTACK_MODIFIER};
    if flags & RAW_ATTACK_MODIFIER == 0 || !x.is_finite() || !y.is_finite() { return 0; }
    let game = g();
    if !game.valid_valley_neutral_opening() { return 0; }
    let [width, height] = game.map.world_dimensions();
    if !(0.0..width as f32).contains(&x) || !(0.0..height as f32).contains(&y) { return 0; }
    let target = pick(game, x, y, false);
    if target == NOIDX || !game.can_see(game.local, target) { return 0; }
    let sources: Vec<_> = selected(game).into_iter()
        .filter(|&i| game.admits_valley_neutral_target(&game.ents[i], &game.ents[target])).collect();
    if sources.is_empty() { return 0; }
    let sources = handles(game, &sources);
    game.submit(C_ATTACK, FORCE_ATTACK, -1.0, game.h(target) as i32, &sources);
    sources.len() as u32
}

/// Right click: attack / gather / build / board / unload / move / set rally.
#[no_mangle]
pub extern "C" fn cmd_context(x: f32, y: f32, attack_move: u32) {
    submit_context_order(x, y, attack_move);
}

/// Issue a normal context order and report whether it targeted a visible hostile.
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_context_attack_reply(x: f32, y: f32) -> u32 {
    u32::from(submit_context_order(x, y, 0))
}

fn submit_context_order(x: f32, y: f32, attack_move: u32) -> bool {
    let g = g();
    #[cfg(feature = "ee2-authored-valley37")]
    if let Some(world) = &g.map.source_world {
        if !(0.0..world.width as f32).contains(&x) || !(0.0..world.height as f32).contains(&y) { return false; }
    }
    let ids = selected(g);
    #[cfg(feature = "ee2-native13")]
    let mut ids = ids;
    if ids.is_empty() {
        return false;
    }
    if DEFS[g.ents[ids[0]].def as usize].building {
        #[cfg(feature = "ee2-native13")]
        if g.air_missions_enabled { ids.retain(|&i| g.ents[i].def != AIRFIELD); }
        if ids.is_empty() { return false; }
        let all = handles(g, &ids);
        g.submit(C_RALLY, x, y, 0, &all);
        return false;
    }
    if attack_move != 0 {
        let all = handles(g, &ids);
        g.submit(C_AMOVE, x, y, 0, &all);
        return false;
    }
    #[cfg(feature = "ee2-native13")]
    if !g.net && g.powers.is_some() {
        let mut extraction = Vec::new();
        ids.retain(|&index| {
            if g.spy_marker_at(index, x, y) {
                extraction.push(g.h(index));
                false
            } else { true }
        });
        if !extraction.is_empty() { g.submit(C_EXTRACTION, x, y, 1, &extraction); }
        if ids.is_empty() { return false; }
    }
    let j = pick(g, x, y, false);
    #[cfg(feature = "ee2-native13")]
    if j != NOIDX && g.air_missions_enabled && g.air_host_slots(&g.ents[j]) != 0 && g.ents[j].team == g.local &&
        g.ents[j].built >= 1.0 {
        let mut aircraft = Vec::new();
        ids.retain(|&i| {
            if !air31_defs::mission_aircraft(g.ents[i].def) { return true; }
            aircraft.push(g.h(i));
            false
        });
        if !aircraft.is_empty() { g.submit(C_BOARD, x, y, g.h(j) as i32, &aircraft); }
        if ids.is_empty() { return false; }
    }
    #[cfg(feature = "ee2-native13")]
    if j != NOIDX && matches!(g.ents[j].def, MARKET | DOCK) {
        let target = g.h(j);
        let chosen = g.trade_automatic_choices(g.local, target, &ids);
        if !chosen.is_empty() {
            let hs: Vec<_> = chosen.iter().map(|&(index, _)| g.h(index)).collect();
            g.submit(C_TRADE_ROUTE, 65535.0, 65535.0, target as i32, &hs);
            ids.retain(|index| chosen.binary_search_by_key(index, |&(actor, _)| actor).is_err());
            if ids.is_empty() { return false; }
        }
    }
    let all = handles(g, &ids);
    let part = |g: &Game, f: &dyn Fn(&Ent, &Def) -> bool| -> Vec<u32> { ids.iter().copied().filter(|&i| f(&g.ents[i], &DEFS[g.ents[i].def as usize])).map(|i| g.h(i)).collect() };
    let workers = part(g, &|_, d| d.has(F_WORKER));
    let others = part(g, &|_, d| !d.has(F_WORKER));
    if j != NOIDX {
        let t = g.ents[j];
        let td = &DEFS[t.def as usize];
        let th = g.h(j) as i32;
        if t.team != g.local {
            if g.combat_hostile(g.local, t.team) {
                if g.can_see(g.local, j) {
                    g.submit(C_ATTACK, x, y, th, &all);
                    return true;
                }
                g.submit(C_AMOVE, x, y, 0, &all);
            } else {
                g.submit(C_MOVE, x, y, 0, &all);
            }
            return false;
        }
        if td.has(F_TRANSPORT) || t.built >= 1.0 {
            let land = part(g, &|actor, _| g.can_garrison(actor.def, t.def));
            if !land.is_empty() {
                g.submit(C_BOARD, x, y, th, &land);
                return false;
            }
        }
        if td.building && (t.built < 1.0 || t.def == FARM) && !workers.is_empty() {
            g.submit(C_WORK, x, y, th, &workers);
            if !others.is_empty() {
                g.submit(C_MOVE, x, y, 0, &others);
            }
            return false;
        }
    }
    let tile = tile_of(x, y);
    #[cfg(feature = "ee2-native13")]
    if g.fishing_enabled && g.map.res[tile] == R_FISH && g.real_fog(g.local, tile) > 0 {
        let fishers = part(g, &|actor, _| actor.def == fishing81::FISHING_SHIP);
        if !fishers.is_empty() {
            g.submit(C_GATHER, x, y, tile as i32, &fishers);
            let rest = part(g, &|actor, _| actor.def != fishing81::FISHING_SHIP);
            if !rest.is_empty() { g.submit(C_MOVE, x, y, 0, &rest); }
            return false;
        }
    }
    let kind = economy::map_resource_kind(g.map.res[tile], g.players[g.local as usize].epoch);
    if kind != R_NONE && g.real_fog(g.local, tile) > 0 && !workers.is_empty() {
        if !g.citizen_can_gather(g.local, kind) { return false; }
        g.submit(C_GATHER, x, y, tile as i32, &workers);
        if !others.is_empty() {
            g.submit(C_MOVE, x, y, 0, &others);
        }
        return false;
    }
    // loaded transports sent onto land put their passengers ashore there
    let loaded = part(g, &|e, d| d.has(F_TRANSPORT) && e.ncargo > 0);
    if !loaded.is_empty() && g.map.ter[tile] >= T_SAND {
        let rest = part(g, &|e, d| !(d.has(F_TRANSPORT) && e.ncargo > 0));
        g.submit(C_UNLOAD, x, y, 0, &loaded);
        if !rest.is_empty() {
            g.submit(C_MOVE, x, y, 0, &rest);
        }
        return false;
    }
    g.submit(C_MOVE, x, y, 0, &all);
    false
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

/// Ephemeral, per-handle render request sequence. Neither saves nor lockstep
/// hashes consume it; a rebuilt animator starts with no outgoing clip.
#[cfg(feature="ee2-pack8")]
#[no_mangle]
pub extern "C" fn pack_visual_revision(handle:u32)->u32{
    unsafe{(&*addr_of_mut!(GAME)).as_ref()
        .and_then(|game|game.get(handle).map(|index|game.canonical_pack(index).visual_revision))
        .unwrap_or(0)}
}

#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn pack_schema_revision()->u32{if cfg!(feature="ee2-citizen9"){4}else if cfg!(feature="ee2-civ8"){3}else{1}}
#[cfg(feature="ee2-pack8")]
#[no_mangle]pub extern "C" fn early_artillery_pack_version()->u32{1}
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
        #[cfg(feature = "ee2-native13")]
        if g.ents[i].def == DOCK && g.dock_garrison_count(i) > 0 {
            let (x, y, handle) = (g.ents[i].x, g.ents[i].y, g.h(i));
            g.submit(C_UNLOAD, x, y, 0, &[handle]);
            continue;
        }
        #[cfg(feature = "ee2-native13")]
        if g.air_missions_enabled && g.air_host_slots(&g.ents[i]) != 0 {
            g.submit(C_AIR_MISSION, 0.0, 0.0, air31::SCRAMBLE, &[g.h(i)]);
            continue;
        }
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
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn wonder_footprint(def: u32, turn: u32) -> u32 {
    if def >= DEFS.len() as u32 || !wonders61::is_wonder(def as DefId) { return 0; }
    let (width, height) = Game::footprint_axes(def as DefId, turn);
    width as u32 | (height as u32) << 16
}
#[no_mangle]
pub extern "C" fn dock_orientation(tx:i32,ty:i32)->u32{g().dock_auto_turn(tx,ty)}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn foreign_construction_version() -> u32 { 2 }
#[no_mangle]
pub extern "C" fn placement_time_multiplier(def:u32,tx:i32,ty:i32,turn:u32)->f32{
    if def>=DEFS.len()as u32{return 0.0;}let (width,height)=Game::footprint_axes(def as DefId,turn);if width==0||tx<0||ty<0||tx>N as i32-width||ty>N as i32-height{return 0.0;}let game=g();if def==TC as u32{return 1.0;}let owner=game.towner[game.map.terr[(ty+height/2)as usize*N+(tx+width/2)as usize]as usize];if owner==game.local{1.0}else if owner==255{2.0}else{3.0}
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

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_cancel_spy() -> u32 {
    let game = g();
    if game.net || game.powers.is_none() { return 0; }
    let Some(index) = selected(game).into_iter().find(|&index| {
        let host = &game.ents[index];
        host.def == UNIVERSITY && host.q[..host.qn as usize].contains(&(crate::spy46_defs::SPY as QueueItem))
    }) else { return 0; };
    let handle = game.h(index);
    game.submit(C_CANCEL, 0.0, 0.0, crate::spy46_defs::SPY as i32, &[handle]);
    1
}

#[no_mangle]
pub extern "C" fn cmd_delete() {
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_DELETE, 0.0, 0.0, 0, &hs);
}

/// Cheat ids: 0 enable, 1 disable, 2 loot, 3 instant build, 4 technology points,
/// 5 taxes (UP), 6 punish (UP), 7 play god (UP), 8 toggle fog (UP), 9 epoch up (UP), 10 convert (UP).
/// Results: 0 disabled/invalid, 1 applied, 2 toggle off, 3 multiplayer denied,
/// 4 at final epoch, 5 no convertible actor.
#[no_mangle]
pub extern "C" fn cheat(code: u32) -> u32 {
    let game = g();
    if game.net { return 3; }
    if code == 0 { game.cheats_enabled = true; return 1; }
    if code == 1 {
        game.cheats_enabled = false;
        if !cfg!(feature = "ee2-up15") {
            game.instant_build = false;
        }
        return 1;
    }
    if !game.cheats_enabled { return 0; }
    match code {
        2 => {for r in 0..economy::RESOURCE_COUNT{game.add_resource(game.local,r,10000.0);}1},
        3 => { game.instant_build = !game.instant_build; if game.instant_build { 1 } else { 2 } },
        4 => {
            if cfg!(feature = "ee2-reference") {
                let balance = &mut game.research_points[game.local as usize];
                let next = *balance + 50.0;
                // UP AddTechPoints compares to 999,999, but writes 99,999 on overflow.
                *balance = if cfg!(feature = "ee2-up15") && next > 999_999.0 { 99_999.0 } else { next };
            } else {
                game.tech_points = game.tech_points.saturating_add(50);
            }
            1
        },
        #[cfg(feature = "ee2-up15")]
        5 => {
            for slot in 0..economy::RESOURCE_COUNT {
                game.add_resource(game.local, slot, -100.0);
            }
            1
        },
        #[cfg(feature = "ee2-up15")]
        6 => {
            if !game.god_mode_for(game.local) {
                for actor in &mut game.ents[..game.high] {
                    if actor.alive && actor.sel && actor.team == game.local && actor.hp > 20.0 {
                        actor.hp -= 20.0;
                    }
                }
            }
            1
        },
        #[cfg(feature = "ee2-up15")]
        7 => {
            game.god_mode = !game.god_mode;
            if game.god_mode { 1 } else { 2 }
        },
        #[cfg(feature = "ee2-up15")]
        8 => {
            game.fog_inverted = !game.fog_inverted;
            game.refresh_display_fog();
            if game.display_visibility() < 2 { 1 } else { 2 }
        },
        #[cfg(feature = "ee2-native13")]
        9 => if game.force_epoch_advance() { 1 } else { 4 },
        #[cfg(feature = "ee2-native13")]
        10 => if game.convert_last_clicked() { 1 } else { 5 },
        #[cfg(feature = "ee2-native13")]
        11 => if game.recharge_selected_reserves() { 1 } else { 0 },
        _ => 0,
    }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_priest_skirmish() -> u32 {
    g().configure_priest_skirmish() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_fishing_skirmish() -> u32 {
    u32::from(g().configure_fishing(false))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_fishing() -> u32 {
    u32::from(g().configure_fishing(true))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fishing_definition() -> u32 {
    u32::from(fishing81::FISHING_SHIP)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fish_stock(tile: u32) -> u32 {
    let game = g();
    let tile = tile as usize;
    if game.map.res.get(tile) != Some(&R_FISH) || game.vis[tile] == 0 { return 0; }
    u32::from(game.map.amt[tile])
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn fishing_cargo(handle: u32) -> f32 {
    let game = g();
    game.get(handle).filter(|&index| game.ents[index].def == fishing81::FISHING_SHIP &&
        game.ents[index].team == game.local).map_or(-1.0, |index| game.ents[index].carry)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_carrier_surface_pose() -> u32 {
    g().configure_carrier_surface_pose() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_submarine_depth() -> u32 {
    u32::from(g().configure_submarine_depth())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_torpedo84() -> u32 {
    u32::from(g().configure_torpedo84())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_heavy_mounted3() -> u32 {
    u32::from(g().configure_heavy_mounted3())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn heavy_mounted3_definition() -> u32 {
    u32::from(heavy_mounted3_61::ID)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_guided_herc_rocket() -> u32 {
    u32::from(g().configure_guided_herc_rocket())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn priest_power(handle: u32) -> f32 {
    let game = g();
    let Some(index) = game.get(handle) else { return -1.0; };
    if game.ents[index].def != crate::priest42_defs::PRIEST { return -1.0; }
    game.powers.as_ref().and_then(|state| state.actor(handle)).map_or(-1.0, |actor| actor.charge)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_power(handle: u32) -> f32 {
    let game = g();
    let Some(index) = game.get(handle) else { return -1.0; };
    if game.ents[index].def != crate::spy46_defs::SPY { return -1.0; }
    game.powers.as_ref().and_then(|state| state.actor(handle)).map_or(-1.0, |actor| actor.charge)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_disguised(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.visible(index) && game.spy_disguised(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn priest_visual_state(handle: u32) -> u32 {
    let game = g();
    let now = game.sim_elapsed();
    if game.priest_bless_elapsed(handle, now).is_some() { 2 }
    else { game.priest_conversion_elapsed(handle, now).is_some() as u32 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn priest_visual_elapsed(handle: u32, render_time: f32) -> f32 {
    let game = g();
    game.priest_bless_elapsed(handle, render_time).or_else(|| game.priest_conversion_elapsed(handle, render_time)).unwrap_or(0.0)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn priest_effect_count() -> u32 {
    g().priest_effects.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn priest_effect_ptr() -> *const u32 {
    let effects = &g().priest_effects;
    if effects.is_empty() { core::ptr::null() } else { effects.as_ptr().cast() }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn bless_selection_count() -> u32 {
    let game = g();
    selected(game).iter().filter(|&&index| game.bless_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn bless_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || bless_selection_count() == 0 { return 0; }
    let game = g();
    let target = pick(game, x, y, false);
    (target != NOIDX && game.bless_target(game.local, target)) as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_bless_at(x: f32, y: f32) -> u32 {
    if bless_target_at(x, y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game, x, y, false));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.bless_ready(index)).collect();
    let hs = handles(game, &ids);
    game.submit(C_BLESS, 0.0, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_selection_count() -> u32 {
    let game = g();
    if game.net || game.initial_ceasefire_active() { return 0; }
    selected(game).iter().filter(|&&index| game.spy_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_intel_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || spy_selection_count() == 0 { return 0; }
    let game = g();
    let target = pick(game, x, y, false);
    u32::from(target != NOIDX && game.spy_intel_target(game.local, target))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_spy_intel_at(x: f32, y: f32) -> u32 {
    if spy_intel_target_at(x, y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game, x, y, false));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.spy_ready(index)).collect();
    let hs = handles(game, &ids);
    game.submit(C_GATHER_INTEL, 0.0, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_visual_state(handle: u32) -> u32 {
    let game = g();
    if game.get(handle).is_some_and(|index| game.visible(index)) { game.spy_pose_state(handle) } else { 0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_operation_selection_count(operation: u32) -> u32 {
    let game = g();
    let Some(operation) = game::spy_intel46::SpyOperation::read(operation) else { return 0; };
    if game.net || !operation.available(game, game.local)
        || operation.blocked_by_ceasefire() && game.initial_ceasefire_active() { return 0; }
    selected(game).iter().filter(|&&index| game.spy_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_operation_unlocked(operation: u32) -> u32 {
    let game = g();
    u32::from(!game.net && game.powers.is_some()
        && game::spy_intel46::SpyOperation::read(operation).is_some_and(|operation| operation.available(game, game.local)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_operation_target_at(operation: u32, x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || spy_operation_selection_count(operation) == 0 { return 0; }
    let Some(operation) = game::spy_intel46::SpyOperation::read(operation) else { return 0; };
    let game = g();
    let target = pick(game, x, y, false);
    u32::from(target != NOIDX && game.spy_operation_target(game.local, target, operation))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_spy_operation_at(operation: u32, x: f32, y: f32) -> u32 {
    if spy_operation_target_at(operation, x, y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game, x, y, false));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.spy_ready(index)).collect();
    let hs = handles(game, &ids);
    game.submit(C_GATHER_INTEL, operation as f32, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn building_sabotaged(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.visible(index) && game.sabotaged(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_toxic_status(handle: u32) -> u32 {
    let game = g();
    let Some(index) = game.get(handle).filter(|&index| game.visible(index)) else { return 0; };
    u32::from(game.contaminated(index)) | (u32::from(game.toxic_sick(index)) << 1)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_adverse_glow_present() -> u32 {
    let game = g();
    u32::from(game.spy_sabotage.active() || !game.spy_toxic.empty())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_visual_elapsed(handle: u32, render_time: f32) -> f32 {
    let game = g();
    if game.get(handle).is_some_and(|index| game.visible(index)) {
        game.spy_intel_elapsed(handle, render_time).unwrap_or(-1.0)
    } else { -1.0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_crown_intel_seconds(owner: u32) -> f32 {
    let game = g();
    if (owner as usize) < game.nplayers { game.crown_intel_seconds(game.local, owner as u8) } else { 0.0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_crown_intel_active(owner: u32) -> u32 {
    let game = g();
    u32::from((owner as usize) < game.nplayers && game.crown_intel_active(game.local, owner as u8))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_crowns(disabled: u32) -> u32 {
    u32::from(disabled <= 1 && g().configure_crowns(disabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_crowns_peer(disabled: u32) -> u32 {
    u32::from(disabled <= 1 && g().configure_crowns_peer(disabled != 0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_rules() -> u32 { g().crown_rules() }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_crown_leaders() -> u32 { u32::from(g().configure_crown_leaders()) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_military_leader_powers() -> u32 { u32::from(g().configure_military_leader_powers()) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_economic_leader_effects() -> u32 { u32::from(g().configure_economic_leader_effects()) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_mobilization_selection_count() -> u32 {
    let game = g();
    selected(game).iter().filter(|&&index| game.mobilization_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_mobilization_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || leader_mobilization_selection_count() == 0 { return 0; }
    let game = g();
    let target = pick(game,x,y,false);
    u32::from(target != NOIDX && game.mobilization_target(target))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_leader_mobilization_at(x: f32, y: f32) -> u32 {
    if leader_mobilization_target_at(x,y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game,x,y,false));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.mobilization_ready(index)).collect();
    let hs = handles(game,&ids);
    game.submit(C_LEADER_MOBILIZATION,0.0,0.0,target as i32,&hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_purchase_selection_count() -> u32 {
    let game = g();
    selected(game).into_iter().filter(|&index| game.leader_purchase_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_purchase_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || leader_purchase_selection_count() == 0 { return 0; }
    let game = g();
    let target = pick(game, x, y, false);
    u32::from(target != NOIDX && game.leader_purchase_target(game.local, target))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_leader_purchase_at(x: f32, y: f32) -> u32 {
    if leader_purchase_target_at(x, y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game, x, y, false));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.leader_purchase_ready(index)).collect();
    let hs = handles(game, &ids);
    game.submit(C_LEADER_PURCHASE, 0.0, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn imperialism_selection_count() -> u32 {
    let game = g();
    selected(game).into_iter().filter(|&index| game.imperialism_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn imperialism_target_at(x: f32, y: f32) -> u32 {
    if !x.is_finite() || !y.is_finite() || imperialism_selection_count() == 0 { return 0; }
    let game = g();
    let target = pick(game, x, y, true);
    u32::from(target != NOIDX && game.imperialism_target(game.local, target))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_imperialism_at(x: f32, y: f32) -> u32 {
    if imperialism_target_at(x, y) == 0 { return 0; }
    let game = g();
    let target = game.h(pick(game, x, y, true));
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.imperialism_ready(index)).collect();
    let hs = handles(game, &ids);
    game.submit(C_IMPERIALISM, 0.0, 0.0, target as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn imperialism_visible(handle: u32) -> u32 {
    u32::from(g().imperialism_visible(handle))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_rally_ready(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.ents[index].team == game.local && game.rally_ready(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn military_leader_reserve(handle: u32) -> f32 {
    let game = g();
    game.get(handle).filter(|&index| game.ents[index].team == game.local)
        .map_or(-1.0, |index| game.military_leader_reserve(index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_power_reserve(handle: u32) -> f32 {
    let game = g();
    game.get(handle).filter(|&index| game.ents[index].team == game.local)
        .map_or(-1.0, |index| game.leader_power_reserve(index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn imperial_construction_selection_count() -> u32 {
    let game = g();
    selected(game).into_iter().filter(|&index| game.imperial_construction_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
fn imperial_construction_pick(game: &Game, x: f32, y: f32) -> usize {
    if !x.is_finite() || !y.is_finite() { return NOIDX; }
    pick_where(game, x, y, |index, _| game.imperial_construction_target(game.local, index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn imperial_construction_target_at(x: f32, y: f32) -> u32 {
    if imperial_construction_selection_count() == 0 { return 0; }
    let game = g();
    u32::from(imperial_construction_pick(game, x, y) != NOIDX)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_imperial_construction_at(x: f32, y: f32) -> u32 {
    let game = g();
    let target = imperial_construction_pick(game, x, y);
    if target == NOIDX { return 0; }
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.imperial_construction_ready(index)).collect();
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_LEADER_CONSTRUCTION, 0.0, 0.0, game.h(target) as i32, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_leader_rally() -> u32 {
    let game = g();
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.rally_ready(index)).collect();
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_LEADER_RALLY, 0.0, 0.0, 0, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_fire_maneuver_ready(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.ents[index].team == game.local && game.fire_maneuver_ready(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_leader_fire_maneuver() -> u32 {
    let game = g();
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.fire_maneuver_ready(index)).collect();
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_LEADER_RALLY, 0.0, 0.0, 1, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_leader_diversion() -> u32 {
    let Some(game) = (unsafe { (&mut *addr_of_mut!(GAME)).as_mut() }) else { return 0; };
    u32::from(game.configure_leader_diversion())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_diversion_enabled() -> u32 {
    u32::from(g().leader_diversion_enabled())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_diversion_ready(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.ents[index].team == game.local && game.leader_diversion_ready(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_diversion_target_at(x: f32, y: f32) -> u32 {
    let game = g();
    u32::from(game::leader_diversion61::location(x, y)
        && selected(game).into_iter().any(|index| game.leader_diversion_ready(index)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_leader_diversion_at(x: f32, y: f32) -> u32 {
    if !game::leader_diversion61::location(x, y) { return 0; }
    let game = g();
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.leader_diversion_ready(index)).collect();
    if ids.is_empty() { return 0; }
    let hs = handles(game, &ids);
    game.submit(C_LEADER_DIVERSION, x, y, 0, &hs);
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_diversion_visual(handle: u32) -> u32 {
    let game = g();
    game.get(handle).map_or(0, |index| game.diversion_visual(index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn leader_maneuver_status(handle: u32) -> u32 {
    let game = g();
    game.get(handle).map_or(0, |index| game.leader_maneuver_status(index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn military_leader_status(handle: u32) -> u32 {
    let game = g();
    game.get(handle).map_or(0, |index| game.military_leader_status(index))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_city_heal() -> u32 {
    u32::from(g().configure_city_heal())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_dock_garrison() -> u32 {
    u32::from(g().configure_dock_garrison())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_rating(owner: u32, kind: u32, field: u32) -> f32 {
    let Some(rating) = g().crown_rating(owner as usize, kind as usize) else { return f32::NAN; };
    match field { 0 => rating.score, 1 => rating.threshold as f32, 2 => f32::from(rating.eligible_rank), _ => f32::NAN }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_technology(owner: u32, kind: u32, slot: u32) -> u32 {
    let game = g();
    if slot >= 4 || game.crown_rating(owner as usize, kind as usize).is_none() { return 0; }
    let epoch = u32::from(game.players[owner as usize].epoch);
    let id = (epoch * 12 + kind * 4 + slot) as usize;
    u32::from(game.has_research(owner as u8, id))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_holder(kind: u32, field: u32) -> f32 {
    let game = g();
    let Some(holder) = game.crown_holder(kind as usize) else { return 0.0; };
    match field {
        0 => f32::from(holder.owner) + 1.0,
        1 => (holder.expires - game.sim_elapsed()).max(0.0),
        2 => f32::from(holder.epoch),
        _ => f32::NAN,
    }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_choose_power(kind: u32, power: u32) -> u32 {
    let Ok(power) = u8::try_from(power) else { return 0; };
    let game = g();
    if !game.net { return u32::from(game.choose_crown_power(kind as usize, power)); }
    if !game.can_choose_crown_power(kind as usize, power) || game.network_observer(game.local) { return 0; }
    game.submit(cmd::C_CROWN_CHOOSE, kind as f32, 0.0, i32::from(power), &[]);
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_can_choose_power(kind: u32, power: u32) -> u32 {
    u32::from(u8::try_from(power).is_ok_and(|id| g().can_choose_crown_power(kind as usize, id)))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_chosen_power(kind: u32) -> i32 {
    g().crown_power(kind as usize).map_or(-1, i32::from)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_visuals_present() -> u32 {
    u32::from(g().crown_visuals_present())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_actor_visual(handle: u32, kind: u32, field: u32) -> u32 {
    g().crown_actor_visual(handle, kind as usize, field)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_actor_glow(handle: u32) -> u32 {
    g().crown_actor_glow(handle)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn market_commodity(position: u32) -> u32 {
    let game = g();
    if game.net && game.crown_rules() == 0 || game.bench != 0 { return u32::MAX; }
    let epoch = game.players[game.local as usize].epoch;
    crate::game::market57::commodities(epoch).get(position as usize).copied()
        .filter(|&id| id != u8::MAX).map_or(u32::MAX, u32::from)
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn market_quote(host: u32, resource: u32, lots: i32) -> u32 {
    let game = g();
    u8::try_from(resource).ok().and_then(|id| game.market_quote(game.local, host, id, lots))
        .map_or(0, |(gold, available)| u32::from(gold) | (u32::from(available) << 16))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_market_trade(host: u32, resource: u32, lots: i32) -> u32 {
    let game = g();
    let Ok(id) = u8::try_from(resource) else { return 0; };
    if !game.market_quote(game.local, host, id, lots).is_some_and(|(_, available)| available) { return 0; }
    game.submit(cmd::C_MARKET, f32::from(id), 0.0, lots, &[host]);
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_extraction_selection_count() -> u32 {
    let game = g();
    if game.net { return 0; }
    selected(game).iter().filter(|&&index| game.spy_extract_ready(index)).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_spy_extraction() -> u32 {
    let game = g();
    if game.net { return 0; }
    let ids: Vec<_> = selected(game).into_iter().filter(|&index| game.spy_extract_ready(index)).collect();
    let hs = handles(game, &ids);
    if !hs.is_empty() { game.submit(C_EXTRACTION, 0.0, 0.0, 0, &hs); }
    hs.len() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_extraction_count() -> u32 {
    let game = g();
    game.spy_extraction.markers.keys().filter(|&&handle| {
        game.get(handle).is_some_and(|index| game.ents[index].team == game.local)
    }).count() as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn spy_extraction_field(ordinal: u32, field: u32) -> f32 {
    let game = g();
    let Some((_, point, until)) = game.spy_marker_visual(ordinal as usize) else { return f32::NAN; };
    match field { 0..=2 => point[field as usize], 3 => until, _ => f32::NAN }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn bless_multiplier(handle: u32) -> f32 {
    let game = g();
    if game.get(handle).is_some_and(|index| game.ents[index].team == game.local) { game.blessed_attack(handle) }
    else { 1.0 }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn bless_glow_active(handle: u32) -> u32 {
    let game = g();
    u32::from(game.get(handle).is_some_and(|index| game.visible(index) && game.blessed_attack(handle) > 1.0))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn bless_preview_eligible(handle: u32) -> u32 {
    let game = g();
    u32::from(game.powers.is_some() && game.get(handle).is_some_and(|index| game.bless_target(game.local, index)))
}

#[no_mangle]
pub extern "C" fn cheat_status(field: u32) -> u32 {
    let game = g();
    match field { 0 => game.cheats_enabled as u32, 1 => game.instant_build as u32, 2 => if cfg!(feature = "ee2-reference") {game.research_points[game.local as usize] as u32} else {game.tech_points}, 3 => game.god_mode_for(game.local) as u32, _ => 0 }
}

/// Research from the full tree: use a selected host, otherwise the first suitable building.
#[no_mangle]
pub extern "C" fn cmd_research(k: u32) -> u32 {
    #[cfg(feature = "ee2-reference")]
    {
        #[cfg(feature="ee2-ai9-game-bridge")]
        let item = if ai9_rank::index(k).is_some() { k } else if k < 180 { ORIGINAL_RESEARCH0 + k } else { return 0; };
        #[cfg(not(feature="ee2-ai9-game-bridge"))]
        let item = if k < 180 { ORIGINAL_RESEARCH0 + k as QueueItem } else { return 0; };
        let game=g();
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
            let effect = n.note;
            #[cfg(feature = "ee2-native13")]
            let effect = if game.trade_enabled { game::trade30::research_note(k).unwrap_or(effect) } else { effect };
            let _=write!(s,"{}{{\"id\":{},\"name\":\"{}\",\"effect\":\"{}\",\"epoch\":{},\"branch\":{},\"host\":\"{}\",\"cost\":{},\"done\":{},\"busy\":{},\"hostBuilt\":{},\"ok\":{}}}",if k>0 {","}else{""},k,n.name,effect,n.epoch,n.branch,DEFS[n.host as usize].name,n.cost,researched,busy,host,can);
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

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_sight_terms() -> u32 {
    u32::from(g().configure_sight_terms())
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_timed_treaties(unbreakable: u32) -> u32 {
    u32::from(unbreakable <= 1 && g().configure_timed_treaties(unbreakable == 1))
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_border_treaties() -> u32 {
    u32::from(g().configure_border_treaties())
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_skirmish_teams(groups: u32, lock: u32) -> u32 {
    u32::from(lock <= 1 && g().configure_skirmish_teams(groups, lock == 1))
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_starting_teams(groups: u32, lock: u32) -> u32 {
    u32::from(lock <= 1 && g().configure_network_starting_teams(groups, lock == 1))
}
#[no_mangle]
pub extern "C" fn cmd_diplomacy(target:u32,action:u32,resource:u32,amount:f32)->u32 {
 let game=g();if !game.diplomacy_valid(game.local,target,action,resource,amount){return 0;}
 let bits=target|(action<<8)|(resource<<16);game.submit(C_DIPLOMACY,amount,0.0,bits as i32,&[]);1
}
#[cfg(feature = "ee2-native13")]
static mut ALLIANCE_TERRITORY_IDS: [u32; alliance_territories::MAX_IDS] = [NONE; alliance_territories::MAX_IDS];

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn diplomacy_territories_ptr() -> *mut u32 {
    addr_of_mut!(ALLIANCE_TERRITORY_IDS).cast::<u32>()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_diplomacy_territories(target: u32, action: u32, terms: u32, minutes: f32, give: u32, count: u32) -> u32 {
    if count as usize > alliance_territories::MAX_IDS { return 0; }
    let ids = unsafe { std::slice::from_raw_parts(diplomacy_territories_ptr(), count as usize) };
    let Some(territories) = alliance_territories::AllianceTerritories::new(give as usize, ids) else { return 0; };
    let game = g();
    if !game.diplomacy_territories_valid(game.local, target, action, terms, minutes, territories) { return 0; }
    let bits = target | (action << 8) | (terms << 16);
    game.submit(C_DIPLOMACY, minutes, give as f32, bits as i32, ids);
    1
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
#[cfg(feature = "ee2-native13")]
static mut TERRITORY_GIFT_IDS: [u32; territory::TERR_GIFT_BATCH_MAX] = [NONE; territory::TERR_GIFT_BATCH_MAX];

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn territory_gift_batch_ptr() -> *mut u32 {
    addr_of_mut!(TERRITORY_GIFT_IDS).cast::<u32>()
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn cmd_territory_gift_batch(target: u32, count: u32) -> u32 {
    if !(2..=territory::TERR_GIFT_BATCH_MAX as u32).contains(&count) {
        return 0;
    }
    let ids = unsafe { std::slice::from_raw_parts(territory_gift_batch_ptr(), count as usize) };
    let game = g();
    if !game.territory_gift_batch_valid(game.local, target, ids) {
        return 0;
    }
    let action = target | (territory::TERR_GIFT_BATCH_ACTION << 8);
    game.submit(C_TERRITORY, -1.0, -1.0, action as i32, ids);
    1
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
    #[cfg(feature="ee2-ai9-game-bridge")]
    if ai9_rank::index(item).is_some() { return submit_selected_queue_job(g(),item); }
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
#[cfg(feature="ee2-epoch-range13")]
#[no_mangle]
pub extern "C" fn configure_match_epoch_range(packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_epoch_range(packed_civs, pop, supplies, start, end) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_territory_victory(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32, percent: u32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_territory_victory(packed_civs, pop, supplies, start, end, percent) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn init_hill_world(seed: u32, n_ai: u32) -> u32 {
    if !(1..=3).contains(&n_ai) { return 0; }
    install_game(Game::new_hill_world(seed, n_ai));
    1
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_koth(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32, minutes: i32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_koth(packed_civs, pop, supplies, start, end, minutes) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn koth_definition() -> u32 { u32::from(crate::koth64::HILL) }

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn koth_status(field: u32) -> i32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(if field == 0 { 0 } else { -1 }, |game| game.koth_status(field)) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_crown_victory(mode: u32, target: u32) -> u32 {
    g().configure_network_crown_victory(mode, target) as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_crown_victory(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32, mode: u32, target: u32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_crown_victory(packed_civs, pop, supplies, start, end, mode, target) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn crown_victory_status(field: u32, seat: u32) -> i32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(if field == 0 { 0 } else { -1 }, |game|
        game.crown_victory_status(field, seat as usize)) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_capitols(minutes: u32, sole: u32) -> u32 {
    g().configure_network_capitols(minutes, sole) as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_network_capitols_roster(mode: u32, minutes: u32, sole: u32) -> u32 {
    g().configure_network_capitols_roster(mode, minutes, sole) as u32
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_capitols(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32, mode: u32, minutes: u32, sole: u32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_capitols(packed_civs, pop, supplies, start, end, mode, minutes, sole) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn capitol_status(field: u32, seat: u32) -> i32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(if field == 0 { 0 } else { -1 }, |game|
        game.capitol_status(field, seat as usize)) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_hotspots(civs: u32, pop: u32, supplies: u32, start: u32, end: u32, count: u32) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_hotspots(civs, pop, supplies, start, end, count) as u32) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn hotspot_status(field: u32, seat: u32) -> i32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(if field == 0 { 0 } else { -1 }, |game|
        game.hotspot_status(field, seat as usize)) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn hotspot_point(index: u32, axis: u32) -> f32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(f32::NAN, |game|
        game.hotspot_point(index as usize, axis as usize)) }
}

#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_sole_survivor(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_sole_survivor(packed_civs, pop, supplies, start, end) as u32) }
}
#[cfg(feature = "ee2-native13")]
#[no_mangle]
pub extern "C" fn configure_match_regicide(
    packed_civs: u32, pop: u32, supplies: u32, start: u32, end: u32, sole: u32,
) -> u32 {
    unsafe { (&mut *addr_of_mut!(GAME)).as_mut().map_or(0, |game|
        game.configure_match_regicide(packed_civs, pop, supplies, start, end, sole) as u32) }
}
#[cfg(feature="ee2-epoch-range13")]
#[no_mangle]
pub extern "C" fn match_final_epoch() -> u32 {
    unsafe { (&*addr_of_mut!(GAME)).as_ref().map_or(MAX_EPOCH as u32, |game| game.match_final_epoch() as u32) }
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
 let unknown=DEFS.len() as u32;
 assert_eq!(combat_stat(unknown,0,0,0,0),-1.0);assert_eq!(combat_stat(0,0,unknown,0,0),-1.0);
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
