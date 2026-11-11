//! Native37 preparation transaction for the pinned local Valley geometry.
mod catalog;
mod forest_io;
#[cfg(feature = "ee2-valley163-forest-qa")]
mod forest_registry;
#[cfg(feature = "ee2-valley163-forest-qa")]
mod forest_replay163;
#[cfg(feature = "ee2-valley164-forest-play")]
mod forest_play164;
mod geometry;
#[cfg(feature = "ee2-authored-rect233")]
pub(crate) mod rectangular_world;
#[cfg(feature = "ee2-rect235-forest")]
pub(crate) mod rectangular_forest;
#[cfg(feature = "ee2-rect239-fresh")]
mod fresh239;
#[cfg(feature = "ee2-rect239-fresh")]
mod mobile_placement239;
#[cfg(feature = "ee2-rect267-builder-reach")]
mod builder_reach267;
#[cfg(feature = "ee2-authored-rect233")]
pub(crate) mod rectangular_save234;
#[cfg(feature = "ee2-rect261-ai-clock")]
pub(crate) mod ai_clock261;
#[cfg(feature = "ee2-authored-rect233")]
mod population234;
#[cfg(feature = "ee2-authored-rect233")]
pub(crate) mod population234_attach;
#[cfg(all(test, feature = "ee2-authored-rect233"))]
mod population234_attach_tests;
#[cfg(feature = "ee2-authored-rect233")]
mod population234_catalog;
#[cfg(feature = "ee2-authored-rect233")]
mod population234_stage;
#[cfg(feature = "ee2-authored-rect233")]
pub(crate) mod world0_resources236;
#[cfg(feature = "ee2-rect284-extractor-site")]
pub(crate) mod extractor_site284;
#[cfg(feature = "ee2-rect286-oil-visual")]
pub(crate) mod oil_visual286;
#[cfg(feature = "ee2-rect287-oil-economy")]
pub(crate) mod oil_economy287;
#[cfg(feature = "ee2-rect287-oil-economy")]
pub(crate) mod owned_ids287;
#[cfg(feature = "ee2-rect289-citizen-work")]
pub(crate) mod citizen_work289;
#[cfg(feature = "ee2-rect242-hunt")]
mod citizen_facing243;
#[cfg(feature = "ee2-rect246-deer-live")]
mod deer_live246;
#[cfg(feature = "ee2-rect247-deer-foot")]
mod deer_foot247;
#[cfg(feature = "ee2-rect249-deer-contact")]
mod deer_contact249;
#[cfg(feature = "ee2-rect250-animals")]
mod animal_wander247;
#[cfg(feature = "ee2-rect250-animals")]
mod animal_live250;
#[cfg(feature = "ee2-rect251-animal-slope")]
mod deer_slope250;
#[cfg(feature = "ee2-rect252-animal-birth")]
mod animal_birth252;
#[cfg(feature = "ee2-rect254-aircraft")]
mod aircraft252;
#[cfg(feature = "ee2-rect254-aircraft")]
mod aircraft_live254;
#[cfg(feature = "ee2-rect246-deer-live")]
mod deer_reaction245;
#[cfg(feature = "ee2-rect246-deer-live")]
mod deer_destination246;
#[cfg(feature = "ee2-rect242-hunt")]
mod hunt_order241;
#[cfg(feature = "ee2-rect244-hunt-visual")]
mod hunt_visual244;
#[cfg(feature = "ee2-rect242-hunt")]
mod wildlife_hunting240;
#[cfg(feature = "ee2-rect242-hunt")]
mod world0_hunt242;
mod hash;
mod save_wire;
mod start;
mod start_catalog;
mod source_forest_grid;
#[cfg(feature = "ee2-valley192-movement-atlas")]
pub(crate) mod atlas_runtime;
#[cfg(feature = "ee2-valley190-point-move")]
mod scenery;
#[cfg(feature = "ee2-valley190-point-move")]
mod point_search;
#[cfg(feature = "ee2-valley47-archive-qa")]
mod archived_population;
#[cfg(feature = "ee2-valley52-owned-qa")]
mod authored_owned;
#[cfg(feature = "ee2-valley55-start-qa")]
mod startup_checkpoint;
#[cfg(feature = "ee2-valley56-terrain-qa")]
mod terrain_tree;
#[cfg(feature = "ee2-valley71-neutral-qa")]
mod neutral71;
#[cfg(feature = "ee2-valley74-city-footprint")]
mod city_footprint74;
#[cfg(feature = "ee2-valley74-city-footprint")]
mod city_foundation162;
#[cfg(feature = "ee2-valley74-city-footprint")]
mod house134;
#[cfg(feature = "ee2-valley71-neutral-qa")]
pub(crate) use neutral71::{FORCE_ATTACK, RAW_ATTACK_MODIFIER};
#[cfg(any(feature = "ee2-authored-valley37-qa-commit", feature = "ee2-rect268-city-site"))]
mod source_building_forest;
#[cfg(feature = "ee2-rect269-city-minerals")]
mod city_minerals269;
#[cfg(feature = "ee2-rect270-city-actors")]
mod city_actors270;
#[cfg(feature = "ee2-rect271-build-egress")]
mod city_build_egress271;
#[cfg(feature = "ee2-rect276-clearance-resume")]
pub(crate) mod clearance_resume276;
#[cfg(feature = "ee2-rect280-farm-crop")]
pub(crate) mod farm_crop280;
#[cfg(feature = "ee2-rect278-food-resume")]
mod food_resume278;
#[cfg(feature = "ee2-rect272-builder-arrival")]
pub(crate) mod builder_arrival272;
#[cfg(feature = "ee2-rect272-builder-arrival")]
mod builder_approach272;
#[cfg(feature = "ee2-rect273-builder-waypoint")]
mod builder_waypoint273;
#[cfg(feature = "ee2-rect273-builder-waypoint")]
mod builder_waypoint_admission273;
#[cfg(any(feature = "ee2-valley204-ground-motion", feature = "ee2-rect272-builder-arrival"))]
#[path = "atlas_runtime/ground_steering.rs"]
mod ground_steering;
#[cfg(feature = "ee2-valley74-city-footprint")]
mod farm143;
#[cfg(feature = "ee2-valley74-city-footprint")]
pub(crate) mod farm_cursor147;
#[cfg(any(feature = "ee2-authored-valley37-qa-commit", feature = "ee2-rect268-city-site"))]
mod world8_placement_math;

pub(crate) use geometry::Geometry;
pub(crate) use save_wire::{split_save, wrap_save};
#[cfg(feature = "ee2-valley198-waypoint-save")]
mod route_save;
#[cfg(feature = "ee2-valley198-waypoint-save")]
pub(crate) use route_save::restore as restore_saved_routes;
pub(crate) use start::FixtureState;
use crate::game::Game;
use crate::map::{Map, N, T_DEEP, T_FOREST, T_GRASS, T_ROCK, T_SAND, T_SHALLOW};
use std::cell::RefCell;

const DIMENSIONS: [usize; 2] = [115, 71];
const UNASSIGNED: u8 = 31;
const VALLEY_SHA: [u8; 32] = [
    0xe8, 0x17, 0x20, 0x9c, 0xa7, 0x1a, 0xec, 0xf8, 0x85, 0x28, 0x92, 0x89, 0x66, 0x98, 0x22, 0xe5,
    0xeb, 0x79, 0x8b, 0xd1, 0x6b, 0x92, 0x2c, 0x9a, 0xef, 0x36, 0x16, 0xd0, 0x45, 0x9c, 0x4e, 0xf4,
];

#[cfg(feature = "ee2-valley190-point-move")]
impl crate::game::Game {
    pub(crate) fn valley_scenery_move(&self, index: usize, target: [f32; 2]) -> Option<Option<[f32; 2]>> {
        use crate::{defs::WORKER, map::tile_of};

        let actor = &self.ents[index];
        if actor.def != WORKER || !actor.alive || self.valley_fixture.is_none() || self.net {
            return None;
        }
        let world = self.map.source_world.as_deref()?;
        #[cfg(feature = "ee2-valley192-movement-atlas")]
        self.valley_atlas_observe_move(index, target);
        #[cfg(feature = "ee2-valley191-dynamic-move")]
        let houses = self.valley_pathing_houses();
        let blocked = |point: [f32; 2]| {
            let cell = point.map(|axis| (axis * 2.0) as i32);
            let blocked = world.scenery.iter().any(|shape| shape.overlaps_half_cell(cell));
            #[cfg(feature = "ee2-valley191-dynamic-move")]
            let blocked = blocked || houses.iter().any(|shape| shape.overlaps_half_cell(cell));
            blocked
        };
        if !blocked(target) {
            return None;
        }

        let denied = self.denied_border_territories(actor);
        let origin = [actor.x, actor.y, self.map.height(actor.x, actor.y)];
        let goal = [target[0], target[1], self.map.height(target[0], target[1])];
        let recovered = point_search::recover_point(origin, goal, false, false, |[x, y, _]| {
            let tile = tile_of(x, y);
            self.map.pass[0][tile] && self.map.permits_territory(tile, denied) && !blocked([x, y])
        });
        Some(recovered.map(|[x, y, _]| [x, y]))
    }

    #[cfg(feature = "ee2-valley191-dynamic-move")]
    fn valley_pathing_houses(&self) -> Vec<scenery::Footprint> {
        use crate::defs::HOUSE;

        self.ents[..self.high].iter()
            .filter(|site| site.def == HOUSE && site.alive && site.hp > 0.0
                && self.uses_valley_house_movement(site.def) && !self.valley_house_awaiting_labor(site))
            .map(|site| scenery::Footprint::rectangle([site.x, site.y], site.rot, [1.5; 2]))
            .collect()
    }
}


fn prepare_map(world: Geometry) -> Option<Map> {
    let [width, height] = DIMENSIONS;
    if [world.width, world.height] != DIMENSIONS || world.source_sha != VALLEY_SHA || world.source_cliffs != 0 ||
        !world.surfaces.is_empty() || world.pending_units != 41 || world.pending_forest != 335 || world.territories.len() != 12 || world.forest.is_none() {
        return None;
    }
    #[cfg(feature = "ee2-valley56-terrain-qa")]
    let world = world.with_initial_terrain_tree()?;
    #[cfg(feature = "ee2-valley163-forest-qa")]
    let world = forest_replay163::bind_heights(world)?;
    #[cfg(feature = "ee2-valley190-point-move")]
    let mut world = world;
    #[cfg(feature = "ee2-valley190-point-move")]
    {
        world.scenery = archived_population::World0::decode(&world)?.scenery();
    }
    let mut map = Map::blank(0);
    map.ter.fill(T_ROCK);
    map.terr.fill(UNASSIGNED);
    map.blocked.fill(true);
    map.tseeds = world.territories.iter().map(|r| (r.center[0], r.center[1])).collect();
    for (corner_y, padded) in map.hm.chunks_exact_mut(N + 1).enumerate() {
        let row_start = corner_y.min(height) * (width + 1);
        let raw = &world.heights[row_start..row_start + width + 1];
        padded[..=width].copy_from_slice(raw);
        padded[width + 1..].fill(raw[width]);
    }
    for y in 0..height {
        for x in 0..width {
            let compact = y * width + x;
            let tile = y * N + x;
            let material = world.materials[y * (width + 1) + x] as usize;
            map.ter[tile] = match world.move_palette[material] {
                7 => T_DEEP,
                6 => T_SHALLOW,
                2 | 3 => T_ROCK,
                4 => T_FOREST,
                5 => T_SAND,
                _ => T_GRASS,
            };
            map.h[tile] = world.base_height(&world.heights, x as f32 + 0.5, y as f32 + 0.5);
            map.terr[tile] = world.territories.iter().position(|r| r.source_id == world.territory_ids[compact])
                .map_or(UNASSIGNED, |index| index as u8);
            map.blocked[tile] = false;
            map.upd_pass(tile);
            map.recolor(tile);
        }
    }
    for y in 0..height {
        for x in 0..width {
            let tile = y * N + x;
            let a = map.terr[tile];
            if a == UNASSIGNED { continue; }
            for b in [
                if x + 1 < width { map.terr[tile + 1] } else { UNASSIGNED },
                if y + 1 < height { map.terr[tile + N] } else { UNASSIGNED },
            ] {
                if b != a && b != UNASSIGNED {
                    map.territory_neighbors[a as usize] |= 1 << b;
                    map.territory_neighbors[b as usize] |= 1 << a;
                }
            }
        }
    }
    map.source_world = Some(Box::new(world));
    Some(map)
}

#[derive(Default)]
struct Stage {
    input: Vec<u8>,
    start_input: Vec<u8>,
    game: Option<Game>,
    #[cfg(feature = "ee2-valley192-movement-atlas")]
    atlas_input: Vec<u8>,
    #[cfg(feature = "ee2-valley192-movement-atlas")]
    atlas_prepared: Option<atlas_runtime::Prepared>,
    #[cfg(feature = "ee2-valley47-archive-qa")]
    archive: Option<archived_population::World0>,
    #[cfg(feature = "ee2-valley52-owned-qa")]
    owned: Option<authored_owned::Owned>,
    #[cfg(feature = "ee2-valley52-owned-qa")]
    owned_input: Vec<u8>,
}

thread_local! {
    static STAGE: RefCell<Stage> = RefCell::new(Stage::default());
}

pub(crate) fn hash_game(game: &Game, mix: &mut impl FnMut(u32)) {
    #[cfg(feature = "ee2-authored-rect233")]
    if let Some(binding) = rectangular_save234::binding(game) {
        mix(u32::from_le_bytes(*rectangular_save234::MAGIC));
        game.hash_world0_resources(mix);
        #[cfg(feature = "ee2-rect246-deer-live")]
        game.hash_rect_deer(mix);
        #[cfg(feature = "ee2-rect250-animals")]
        game.hash_rect_animals(mix);
        #[cfg(feature = "ee2-rect242-hunt")]
        game.hash_world0_wildlife(mix);
        for digest in [binding.source, binding.terrain, binding.population] {
            for word in digest.chunks_exact(4) { mix(u32::from_le_bytes(word.try_into().unwrap())); }
        }
    }
    if let (Some(world), Some(fixture)) = (&game.map.source_world, &game.valley_fixture) {
        mix(save_wire::MAGIC);
        for word in world.input_sha.into_iter().chain(fixture.input_sha()) { mix(u32::from(word)); }
        let version = save_wire::version(fixture);
        if version != save_wire::VERSION { mix(version); }
        #[cfg(feature = "ee2-valley163-forest-qa")]
        if let Some(replay) = &fixture.forest_replay {
            mix(save_wire::forest_flags(fixture));
            for word in replay.encode().chunks(4) {
                let mut bits = [0; 4];
                bits[..word.len()].copy_from_slice(word);
                mix(u32::from_le_bytes(bits));
            }
        }
        #[cfg(feature = "ee2-valley198-waypoint-save")]
        if let Some(routes) = route_save::encode(game, false) {
            mix(save_wire::WAYPOINT_ROUTE_VERSION);
            for word in routes.chunks_exact(4) {
                mix(u32::from_le_bytes(word.try_into().unwrap()));
            }
        }
    }
}

pub(crate) fn restore_private_map(locator: &save_wire::Locator, seed: u32, n_ai: u32, humans: u32) -> Option<Game> {
    if !cfg!(any(feature = "ee2-authored-valley37-qa-commit", feature = "ee2-valley39-preview")) ||
        [seed, n_ai, humans] != [0, 1, 1] { return None; }
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let world = Geometry::decode(&stage.input)?;
        let start = decode_fixture(&stage.start_input, &world)?;
        if world.input_sha != locator.map || start.input_sha != locator.fixture { return None; }
        #[allow(unused_mut)]
        let mut game = start.bind(Game::blank(prepare_map(world)?, seed, n_ai, 0, humans, 0, 2, false))?;
        #[cfg(feature = "ee2-valley71-neutral-qa")]
        if locator.neutral_opening && !game.set_valley_neutral_opening() { return None; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if locator.native_city_footprints { game.enable_valley_start_city_masks()?; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if locator.native_house_construction { game.enable_valley_house_construction()?; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if locator.native_house_movement { game.enable_valley_house_movement()?; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if locator.native_house_assigned_clearance { game.enable_valley_house_assigned_clearance()?; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if locator.native_city_foundation { game.enable_valley_city_foundation()?; }
        #[cfg(feature = "ee2-valley163-forest-qa")]
        if let Some(bytes) = &locator.forest_replay {
            let source = game.map.source_world.as_deref()?.forest.as_ref()?;
            #[cfg(feature = "ee2-valley164-forest-play")]
            let playable = locator.forest_play;
            #[cfg(not(feature = "ee2-valley164-forest-play"))]
            let playable = false;
            #[cfg(feature = "ee2-valley166-forest-clock")]
            let timed = locator.forest_clock;
            #[cfg(not(feature = "ee2-valley166-forest-clock"))]
            let timed = false;
            let replay = forest_replay163::ForestReplay::decode(source, bytes, playable, timed)?;
            game.valley_fixture.as_mut()?.forest_replay = Some(replay);
        }
        #[cfg(feature = "ee2-valley198-waypoint-save")]
        if locator.route_resume.is_some() {
            let atlas = atlas_runtime::Prepared::decode(&stage.atlas_input)?.bind(&game)?;
            game.valley_fixture.as_mut()?.atlas = Some(atlas);
        }
        Some(game)
    })
}

fn decode_fixture(input: &[u8], _world: &Geometry) -> Option<start::Start> {
    #[cfg(feature = "ee2-valley54-menu-qa")]
    if input.starts_with(b"VOW1") { return authored_owned::authenticated_menu_fixture(input, _world); }
    start::Start::decode(input)
}

#[no_mangle]
pub extern "C" fn valley37_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.game = None;
        #[cfg(feature = "ee2-valley192-movement-atlas")]
        { stage.atlas_input.clear(); stage.atlas_prepared = None; }
        #[cfg(feature = "ee2-valley47-archive-qa")]
        { stage.archive = None; }
        #[cfg(feature = "ee2-valley52-owned-qa")]
        { stage.owned = None; stage.owned_input.clear(); }
        stage.input.clear();
        if !(84..=geometry::MAX_INPUT).contains(&length) { return std::ptr::null_mut(); }
        stage.input.resize(length, 0);
        stage.input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn valley37_prepare() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        #[cfg(feature = "ee2-valley47-archive-qa")]
        { stage.archive = None; }
        #[cfg(feature = "ee2-valley52-owned-qa")]
        { stage.owned = None; }
        stage.game = Geometry::decode(&stage.input).and_then(prepare_map).map(|map| Game::blank(map, 0, 1, 0, 1, 0, 2, false));
        u32::from(stage.game.is_some())
    })
}

#[no_mangle]
pub extern "C" fn valley37_fixture_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.start_input.clear();
        #[cfg(feature = "ee2-valley192-movement-atlas")]
        { stage.atlas_input.clear(); stage.atlas_prepared = None; }
        if !(64..=start::Start::MAX_WIRE).contains(&length) { return std::ptr::null_mut(); }
        stage.start_input.resize(length, 0);
        stage.start_input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn valley37_prepare_fixture() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        let Some(world) = stage.game.as_ref().and_then(|g| g.map.source_world.as_deref()).cloned() else { return 0; };
        let Some(start) = decode_fixture(&stage.start_input, &world) else { return 0; };
        let Some(map) = prepare_map(world) else { return 0; };
        let Some(game) = start.bind(Game::blank(map, 0, 1, 0, 1, 0, 2, false)) else { return 0; };
        #[cfg(feature = "ee2-valley47-archive-qa")]
        { stage.archive = None; }
        #[cfg(feature = "ee2-valley52-owned-qa")]
        { stage.owned = None; }
        stage.game = Some(game);
        1
    })
}

#[no_mangle]
pub extern "C" fn valley37_fixture_info(field: u32) -> u32 {
    STAGE.with(|stage| stage.borrow().game.as_ref().and_then(|g| g.valley_fixture.as_ref()).map_or(u32::MAX, |f| f.query(field)))
}

#[no_mangle]
pub extern "C" fn valley37_actor_handle(source_id: u32) -> u32 {
    STAGE.with(|stage| stage.borrow().game.as_ref().and_then(|g| g.valley_fixture.as_ref()).map_or(u32::MAX, |f| f.handle(source_id)))
}

#[no_mangle]
pub extern "C" fn valley37_actor_word(slot: usize, field: u32) -> u32 {
    STAGE.with(|stage| stage.borrow().game.as_ref().and_then(|g| g.valley_fixture.as_ref()).map_or(u32::MAX, |f| f.source_word(slot, field)))
}

#[cfg(any(feature = "ee2-authored-valley37-qa-commit", feature = "ee2-valley39-preview"))]
fn commit_staged_game() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        if stage.game.as_ref().is_none_or(|g| g.valley_fixture.is_none()) { return 0; }
        unsafe { *std::ptr::addr_of_mut!(crate::GAME) = stage.game.take().map(Box::new); }
        crate::render_meta7::reset_session();
        1
    })
}

#[cfg(feature = "ee2-authored-valley37-qa-commit")]
#[no_mangle]
pub extern "C" fn valley37_qa_commit() -> u32 { commit_staged_game() }

/// The optional private preview can commit only an authenticated authored
/// map plus one supported solo fixture. Default builds lack this entry point.
#[cfg(feature = "ee2-valley39-preview")]
#[no_mangle]
pub extern "C" fn valley39_commit(visibility: u32) -> u32 {
    if visibility > 2 { return 0; }
    let prepared = STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        let Some(game) = stage.game.as_mut() else { return false; };
        if game.valley_fixture.is_none() { return false; }
        game.diplomacy.visibility = visibility as u8;
        game.refresh_display_fog();
        true
    });
    if prepared { commit_staged_game() } else { 0 }
}

#[cfg(feature = "ee2-valley71-neutral-qa")]
#[no_mangle]
pub extern "C" fn valley71_commit(visibility: u32) -> u32 {
    commit_neutral_valley(visibility, false, false, false)
}

#[cfg(feature = "ee2-valley74-city-footprint")]
#[no_mangle]
pub extern "C" fn valley75_commit(visibility: u32) -> u32 {
    commit_neutral_valley(visibility, true, false, false)
}

#[cfg(feature = "ee2-valley74-city-footprint")]
#[no_mangle]
pub extern "C" fn valley76_commit(visibility: u32) -> u32 {
    commit_neutral_valley(visibility, true, true, false)
}

#[cfg(feature = "ee2-valley74-city-footprint")]
#[no_mangle]
pub extern "C" fn valley77_commit(visibility: u32) -> u32 {
    commit_neutral_valley(visibility, true, true, true)
}

#[cfg(feature = "ee2-valley71-neutral-qa")]
fn commit_neutral_valley(visibility: u32, _current_solo: bool, _assigned_clearance: bool, _city_foundation: bool) -> u32 {
    if visibility > 2 { return 0; }
    let prepared = STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        #[cfg(feature = "ee2-valley192-movement-atlas")]
        let atlas = stage.atlas_prepared.take();
        let Some(game) = stage.game.as_mut() else { return false; };
        if !game.set_valley_neutral_opening() { return false; }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if game.enable_valley_start_city_masks().is_none() {
            stage.game = None;
            return false;
        }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if _current_solo && (!configure_current_solo(game) ||
            game.enable_valley_house_construction().and_then(|()| game.enable_valley_house_movement()).is_none()) {
            stage.game = None;
            return false;
        }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if _assigned_clearance && game.enable_valley_house_assigned_clearance().is_none() {
            stage.game = None;
            return false;
        }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if _city_foundation && game.enable_valley_city_foundation().is_none() {
            stage.game = None;
            return false;
        }
        #[cfg(feature = "ee2-valley192-movement-atlas")]
        if _city_foundation {
            if let Some(prepared) = atlas {
                let runtime = prepared.bind(game);
                game.valley_fixture.as_mut().unwrap().atlas = runtime;
            }
        }
        true
    });
    if prepared { valley39_commit(visibility) } else { 0 }
}

#[cfg(feature = "ee2-valley74-city-footprint")]
fn configure_current_solo(game: &mut Game) -> bool {
    if !game.configure_priest_skirmish() || !game.configure_dock_garrison() ||
        !game.configure_crowns(false) || !game.configure_city_heal() || !game.configure_crown_leaders() ||
        !game.configure_military_leader_powers() || !game.configure_leader_diversion() ||
        !game.configure_economic_leader_effects() {
        return false;
    }
    // The authenticated fixture has already installed its two civilizations.
    // This private tick-zero transaction admits the same late roster as solo Skirmish.
    game.heavy_mounted3 = true;
    true
}

#[cfg(feature = "ee2-valley71-neutral-qa")]
#[no_mangle]
pub extern "C" fn valley71_neutral_opening() -> u32 {
    u32::from(active_game().is_some_and(Game::valid_valley_neutral_opening))
}

#[no_mangle]
pub extern "C" fn valley37_info(field: u32) -> u32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(game) = &stage.game else { return 0; };
        let [width, height] = game.map.world_dimensions();
        match field {
            0 => width as u32,
            1 => height as u32,
            2 => N as u32,
            3 => game.high as u32,
            4 => game.tick,
            _ => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn valley37_height(x: f32, y: f32) -> f32 {
    STAGE.with(|stage| stage.borrow().game.as_ref().map_or(f32::NAN, |game| game.map.height(x, y)))
}

#[no_mangle]
pub extern "C" fn valley37_tile(x: u32, y: u32) -> u32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(game) = &stage.game else { return u32::MAX; };
        if x >= N as u32 || y >= N as u32 { return u32::MAX; }
        let map = &game.map;
        let i = y as usize * N + x as usize;
        (map.ter[i] as u32) | ((map.terr[i] as u32) << 8) | ((map.res[i] as u32) << 16) |
            ((map.pass[0][i] as u32) << 24) | ((map.pass[1][i] as u32) << 25) | ((map.blocked[i] as u32) << 26)
    })
}

#[no_mangle]
pub extern "C" fn valley37_hmap_ptr() -> *const f32 {
    STAGE.with(|stage| stage.borrow().game.as_ref().map_or(std::ptr::null(), |game| game.map.hm.as_ptr()))
}

#[no_mangle]
pub extern "C" fn valley37_source_ptr(field: u32) -> *const u8 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some(world) = stage.game.as_ref().and_then(|g| g.map.source_world.as_deref()) else { return std::ptr::null(); };
        match field {
            0 => world.heights.as_ptr().cast(),
            1 => world.materials.as_ptr().cast(),
            2 => world.tints.as_ptr().cast(),
            3 => world.territory_ids.as_ptr().cast(),
            4 => world.input_sha.as_ptr(),
            5 => world.patterns.as_ptr().cast(),
            6 => world.source_sha.as_ptr(),
            _ => std::ptr::null(),
        }
    })
}

#[no_mangle]
pub extern "C" fn valley37_build_check(def: u32, x: i32, y: i32, turn: u32) -> u32 {
    if def >= crate::defs::DEFS.len() as u32 { return u32::MAX; }
    STAGE.with(|stage| stage.borrow().game.as_ref().map_or(u32::MAX, |game| {
        game.placement_geometry_error(0, def as crate::defs::DefId, x, y, turn)
    }))
}

fn active_game() -> Option<&'static Game> {
    unsafe { &*std::ptr::addr_of!(crate::GAME) }.as_deref()
}

#[no_mangle]
pub extern "C" fn valley37_view_info(field: u32) -> u32 {
    let Some(game) = active_game() else { return 0; };
    if game.map.layout.profile() != 0 { return 0; }
    let Some(world) = game.map.source_world.as_deref() else { return 0; };
    match field {
        0 => world.width as u32,
        1 => world.height as u32,
        2 => N as u32,
        3 => game.valley_fixture.as_ref().map_or(0, |fixture| 1 + 2 * fixture.query(0) + fixture.query(1)),
        4 => game.valley_fixture.as_ref().map_or(0, |fixture| fixture.query(6)),
        5 => game.valley_fixture.as_ref().map_or(0, |fixture| fixture.query(5)),
        6 => game.valley_fixture.as_ref().map_or(0, |fixture| fixture.query(3)),
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn valley37_view_ptr(field: u32) -> *const u8 {
    let Some(world) = active_game().filter(|game| game.map.layout.profile() == 0)
        .and_then(|game| game.map.source_world.as_deref()) else { return std::ptr::null(); };
    match field {
        0 => world.heights.as_ptr().cast(),
        1 => world.materials.as_ptr().cast(),
        2 => world.tints.as_ptr().cast(),
        3 => world.patterns.as_ptr().cast(),
        4 => world.source_sha.as_ptr(),
        _ => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn valley37_view_ground(x: f32, y: f32) -> f32 {
    active_game().filter(|game| game.map.source_world.is_some() && x.is_finite() && y.is_finite())
        .map_or(f32::NAN, |game| game.map.height(x, y))
}

#[no_mangle]
pub extern "C" fn valley37_view_focus(team: u32, axis: usize) -> f32 {
    if team > 1 { return f32::NAN; }
    active_game().and_then(|game| game.valley_fixture.as_ref()?.focus(team as u8)?.get(axis).copied())
        .unwrap_or(f32::NAN)
}

#[cfg(feature = "ee2-authored-valley37-qa-commit")]
fn active_forest() -> Option<&'static source_forest_grid::SourceForestGrid> {
    active_game()?.map.source_world.as_deref()?.forest.as_ref()
}

#[cfg(feature = "ee2-authored-valley37-qa-commit")]
#[no_mangle]
pub extern "C" fn valley37_forest_flags(x: i32, y: i32) -> u32 {
    active_forest().map_or(0, |forest| forest.point_flags(x, y))
}

#[cfg(feature = "ee2-authored-valley37-qa-commit")]
#[no_mangle]
pub extern "C" fn valley37_forest_word(plane: usize, word: usize) -> u32 {
    active_forest().and_then(|forest| forest.plane(plane)).and_then(|words| words.get(word)).copied().unwrap_or(u32::MAX)
}

#[cfg(feature = "ee2-authored-valley37-qa-commit")]
#[no_mangle]
pub extern "C" fn valley37_forest_building(x: f32, y: f32, hx: f32, hy: f32, c: f32, s: f32) -> u32 {
    let Some(forest) = active_forest() else { return u32::MAX; };
    let [width, height] = forest.dimensions();
    if !(0.0..width as f32).contains(&x) || !(0.0..height as f32).contains(&y) ||
        ![hx, hy].into_iter().all(|v| (0.0..=64.0).contains(&v)) ||
        ![c, s].into_iter().all(|v| (-1.0..=1.0).contains(&v)) || (c * c + s * s - 1.0).abs() > 0.000001 {
        return u32::MAX;
    }
    let Some(raster) = source_building_forest::building_forest_raster([x, y], [hx, hy], [c, s], width, height)
        else { return u32::MAX; };
    u32::from(source_building_forest::blocked_by_forest(forest, &raster))
}

// Optional integration fixture reader is absent from production builds.
#[cfg(test)]
pub(crate) mod private_test_fixtures;
