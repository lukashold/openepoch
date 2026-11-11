//! Private fresh Death Valley2 setup over the authenticated source map and population.
use super::{active_game, mobile_placement239::{self, City, Citizen, Gender, Moving, Placed, Rounding,
    Stationary, StationaryKind, World}, population234::UnitKind, rectangular_world::staged_game};
use crate::{defs::{DefId, TC, WORKER}, game::{Game, Player, NOIDX, NONE}, native_rng9::{CitizenIdentity, NativeRng}};

pub(super) const SCOUT: DefId = 57;
const SOURCE_YAW: f32 = f32::from_bits(0xbf49_0fdb);
// Admission237 resolves the four immutable SPSL markers before the new actors
// are created. The coordinates are the actual first legal native results.
const START_CITIES: [[f32; 3]; 4] = [
    [91.75, 10.25, f32::from_bits(0x4041_5ec6)],
    [13.75, 14.25, f32::from_bits(0x4041_5ec6)],
    [18.75, 286.25, 1.0],
    [89.75, 290.25, 1.0],
];

#[cfg(feature = "ee2-rect267-builder-reach")]
pub(super) fn is_start_city(x: f32, y: f32, yaw: f32) -> bool {
    yaw == SOURCE_YAW && START_CITIES.iter().any(|city| x == city[0] && y == city[1])
}

struct SeatStart {
    city: City,
    citizen: Citizen,
    scout: Placed,
}

fn random_marker_order(available: usize, participating: usize, rng: &mut NativeRng) -> Option<Vec<usize>> {
    if participating > available { return None; }
    let mut markers: Vec<_> = (0..participating).collect();
    for remaining in (1..=participating).rev() {
        let index = (rng.next_word15() % remaining as u32) as usize;
        markers.swap(remaining - 1, index);
    }
    Some(markers)
}

fn source_start_plan(game: &Game, before_shuffle: u32) -> Option<(Vec<SeatStart>, NativeRng)> {
    let geometry = game.map.source_world.as_deref()?;
    let population = game.rect_population.as_ref()?.source();
    let forest = game.rect_forest.as_ref()?.placement_plane()?;
    let movement: Vec<_> = geometry.materials.iter().map(|&id| geometry.move_palette[usize::from(id)]).collect();
    let mut stationary = Vec::new();
    let mut moving = Vec::new();
    for unit in population.units() {
        let [x, y, _] = unit.xyz;
        if unit.kind == UnitKind::Mineral {
            stationary.push(Stationary { xy: [x, y], yaw: unit.yaw, kind: StationaryKind::Mineral });
        } else if unit.kind == UnitKind::LiveAnimal && matches!(unit.packed_id, 935 | 937) {
            moving.push(Moving { xyz: unit.xyz, radius: 0.225 });
        }
    }
    if stationary.len() != 43 || moving.len() != 2 { return None; }
    let mut rng = NativeRng::from_state(before_shuffle);
    let markers = random_marker_order(geometry.starts.len(), game.nplayers, &mut rng)?;
    // The authored mode-1 PlaceAllUnits activation is after the marker
    // shuffle and before the first owned City Center. Every archived live
    // animal has a positive Type+D8 gate and a null initial UnitAI.
    for unit in population.units() {
        if unit.kind == UnitKind::LiveAnimal { rng.next_u32(); }
    }
    let mut starts = Vec::with_capacity(markers.len());
    for marker in markers {
        let city = City { xyz: *START_CITIES.get(marker)?, yaw: SOURCE_YAW };
        let world = World { heights: &geometry.heights, movement_by_corner: &movement,
            cliff_patterns: &geometry.patterns, forest_100: forest, stationary: &stationary, moving: &moving };
        // These three draws are the positive installed UnitAI gates. The
        // visual and event callbacks outside the proven prefix are separate.
        rng.next_u32();
        let citizen = mobile_placement239::place_citizen(&world, city, rng.state(), Rounding::Nearest).ok()?;
        rng = NativeRng::from_state(citizen.placed.rng_after);
        rng.next_u32();
        let scout = mobile_placement239::place_scout(&world, city, &citizen, rng.state(), Rounding::Nearest).ok()?;
        rng = NativeRng::from_state(scout.rng_after);
        rng.next_u32();
        let [x, y, _] = city.xyz;
        stationary.push(Stationary { xy: [x, y], yaw: city.yaw, kind: StationaryKind::CityCenter });
        moving.push(Moving { xyz: citizen.placed.xyz, radius: 0.125 });
        moving.push(Moving { xyz: scout.xyz, radius: 0.4 });
        starts.push(SeatStart { city, citizen, scout });
    }
    Some((starts, rng))
}

fn prepare_fresh_game(seed: u32, ai: u32) -> Option<Game> {
    let mut game = staged_game(seed, ai)?;
    // The browser seed enters the first established shared checkpoint. The
    // remaining native renderer/activation callbacks are tracked separately.
    let (starts, rng) = source_start_plan(&game, seed)?;
    for (team, start) in starts.into_iter().enumerate() {
        game.players[team] = Player { res: [400.0, 400.0, 200.0], population_limit: 250,
            alive: true, ai: team != 0, ai_next_attack: 20 * 240, ..Player::default() };
        game.extra_resources[team] = [400.0, 400.0, 200.0, 200.0, 200.0, 200.0];
        game.players[team].recompute();
        game.rebuild_modifiers(team as u8, false);
        let [x, y, _] = start.city.xyz;
        let region = game.map.region_at(game.map.tile_of(x, y))?;
        if game.territory.claims[region] != NONE { return None; }
        let city = game.spawn_oriented(TC, team as u8, x, y, start.city.yaw);
        if city == NOIDX { return None; }
        game.territory.claims[region] = game.h(city);
        game.map.starts[team] = (x as usize, y as usize);
        let [x, y, _] = start.citizen.placed.xyz;
        let citizen = game.spawn_oriented(WORKER, team as u8, x, y, start.citizen.placed.yaw);
        if citizen == NOIDX { return None; }
        game.set_citizen_identity(citizen, match start.citizen.gender {
            Gender::Male => CitizenIdentity::Male,
            Gender::Female => CitizenIdentity::Female,
        });
        let [x, y, _] = start.scout.xyz;
        if game.spawn_oriented(SCOUT, team as u8, x, y, start.scout.yaw) == NOIDX { return None; }
    }
    game.source_birth.rng = rng;
    #[cfg(feature = "ee2-rect250-animals")]
    {
        let population = game.rect_population.as_mut()?;
        population.animals = super::rectangular_world::staged_animals();
        #[cfg(feature = "ee2-rect252-animal-birth")]
        {
            let cursor = population.source().units().iter().map(|unit| unit.local_id()).max()?;
            if let Some(manager) = &mut population.animals { manager.enable_births(cursor + game.high as u32); }
        }
    }
    if !game.configure_fishing(false) { return None; }
    game.rebuild_territories();
    game.build_grid();
    game.update_fog();
    Some(game)
}

#[no_mangle]
pub extern "C" fn native_rect_commit(seed: u32, ai: u32) -> u32 {
    let Some(game) = prepare_fresh_game(seed, ai) else { return 0; };
    crate::install_game(game);
    1
}

#[cfg(feature = "ee2-rect261-ai-clock")]
#[no_mangle]
pub extern "C" fn native_rect_commit_adaptive(seed: u32, ai: u32) -> u32 {
    let Some(mut game) = prepare_fresh_game(seed, ai) else { return 0; };
    let Some(clock) = super::ai_clock261::State::fresh(&game) else { return 0; };
    game.rect_ai_clock = Some(clock);
    #[cfg(feature = "ee2-rect266-city-foundation")]
    if !game.configure_rectangular_city_foundations() { return 0; }
    #[cfg(feature = "ee2-rect287-oil-economy")]
    if game.start_rect_native_ids().is_none() { return 0; }
    #[cfg(feature = "ee2-rect289-citizen-work")]
    game.init_citizen_work_visuals();
    crate::install_game(game);
    1
}

#[no_mangle]
pub extern "C" fn native_rect_focus(axis: u32) -> f32 {
    if axis > 1 { return f32::NAN; }
    let Some(game) = active_game().filter(|game| game.rect_population.is_some()) else { return f32::NAN; };
    game.ents[..game.high].iter().find(|actor| actor.alive && actor.team == game.local && actor.def == TC)
        .map_or(f32::NAN, |city| if axis == 0 { city.x } else { city.y })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_valley37::rectangular_world::spatial_tests::prepare_authenticated_inputs;

    #[cfg(feature = "ee2-rect242-hunt")]
    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn normal_start_citizen_hunt_resumes_between_hits_and_deposits_corpse_food() {
        use crate::{cmd::{C_RECT_HUNT, C_STOP}, game::{O_GATHER, O_IDLE, O_RECT_HUNT}};

        prepare_authenticated_inputs();
        let mut game = prepare_fresh_game(2, 3).unwrap();
        let deer_id = 937;
        let deer = game.rect_population.as_ref().unwrap().unit(deer_id).unwrap().clone();
        let citizen = game.h(1);
        assert_eq!(game.world0_hunt_word(&deer, 13), 10.0_f32.to_bits());
        assert_eq!(game.world0_hunt_word(&deer, 16), 1);
        game.apply(1, C_RECT_HUNT, 0.0, 0.0, deer_id as i32, &[citizen]);
        assert_ne!(game.ents[1].order, O_RECT_HUNT);
        game.apply(0, C_RECT_HUNT, 0.0, 0.0, deer_id as i32, &[game.h(2)]);
        assert_ne!(game.ents[2].order, O_RECT_HUNT);
        game.apply(0, C_RECT_HUNT, 0.0, 0.0, deer_id as i32, &[citizen]);
        assert_eq!(game.ents[1].order, O_RECT_HUNT);

        let resume = |game: &mut Game| {
            game.save();
            let version = 3_u32;
            #[cfg(feature = "ee2-rect246-deer-live")]
            let version = game.deer_motion_section().map_or(version, |section|
                super::super::rectangular_save234::deer_version(&section).unwrap());
            assert_eq!(&game.savebuf[..4], b"DVS1");
            assert_eq!(&game.savebuf[4..8], &version.to_le_bytes());
            let mut loaded = Game::load(&game.savebuf).unwrap_or_else(||
                panic!("load hunt at tick {} q={:x?}", game.tick, game.ents[1].q));
            assert_eq!(loaded.state_hash(), game.state_hash());
            loaded.save();
            assert_eq!(loaded.savebuf, game.savebuf);
            loaded
        };
        let mut approaching = resume(&mut game);
        for _ in 0..250 {
            game.tick();
            approaching.tick();
            assert_eq!(approaching.state_hash(), game.state_hash());
            if game.world0_hunt_word(&deer, 13) != 10.0_f32.to_bits() { break; }
        }
        assert_eq!(game.world0_hunt_word(&deer, 13), 5.0_f32.to_bits());
        let first_hit = game.tick;
        let mut after_hit = resume(&mut game);
        let mut interrupted = Game::load(&after_hit.savebuf).unwrap();
        interrupted.apply(0, C_STOP, 0.0, 0.0, 0, &[citizen]);
        assert_eq!(interrupted.ents[1].order, O_IDLE);
        let mut cancelled = resume(&mut interrupted);
        for _ in 0..80 { cancelled.tick(); }
        assert_eq!(cancelled.world0_hunt_word(&deer, 13), 5.0_f32.to_bits());
        #[cfg(feature = "ee2-rect246-deer-live")]
        {
            for _ in 0..80 {
                game.tick();
                after_hit.tick();
                assert_eq!(after_hit.state_hash(), game.state_hash());
                if game.rect_deer_walk_word(deer_id) != u32::MAX { break; }
            }
            assert_ne!(game.rect_deer_walk_word(deer_id), u32::MAX, "Deer never accepted a flight");
            let moving = game.rect_population.as_ref().unwrap().hunt_pose(&deer);
            assert_ne!(moving.xyz, deer.xyz);
            assert_eq!([game.ents[1].ox, game.ents[1].oy], moving.xyz[..2]);
            let goal = [0, 1, 2].map(|axis| f32::from_bits(game.rect_deer_destination_word(deer_id, axis)));
            println!("Deer flight first_hit={first_hit} walk={} point={:?} goal={goal:?}", game.tick, moving.xyz);
            after_hit = resume(&mut game);
        }
        for _ in 0..if cfg!(feature = "ee2-rect246-deer-live") { 450 } else { 140 } {
            game.tick();
            after_hit.tick();
            assert_eq!(after_hit.state_hash(), game.state_hash());
            if game.world0_hunt_word(&deer, 13) == 0 { break; }
        }
        assert_eq!(game.world0_hunt_word(&deer, 13), 0);
        assert_eq!(game.world0_hunt_word(&deer, 16), 0);
        let corpse = game.world0_hunt_word(&deer, 14);
        assert_ne!(corpse, deer_id);
        assert_eq!(game.world0_hunt_word(&deer, 15), 160.0_f32.to_bits());
        assert_eq!(game.ents[1].order, O_GATHER);
        assert_eq!(game.ents[1].q[3], corpse);
        #[cfg(feature = "ee2-rect246-deer-live")]
        {
            let death = game.rect_population.as_ref().unwrap().hunt_pose(&deer);
            let target = game.world0_gather_order(&game.ents[1]).unwrap();
            assert_eq!(death.xyz[..2], target.point);
            assert_ne!(death.xyz, deer.xyz);
            assert_eq!(game.rect_deer_walk_word(deer_id), u32::MAX);
        }
        let kill = game.tick;
        let mut invalid = game.wildlife_section().unwrap();
        invalid[16..20].copy_from_slice(&161.0_f32.to_le_bytes());
        assert!(game.restore_wildlife(&invalid).is_none());
        assert_eq!(game.world0_hunt_word(&deer, 15), 160.0_f32.to_bits());

        let initial_food = game.players[0].res[0];
        let mut after_kill = resume(&mut game);
        for _ in 0..1400 {
            game.tick();
            after_kill.tick();
            assert_eq!(after_kill.state_hash(), game.state_hash());
            if game.players[0].res[0] > initial_food { break; }
        }
        assert!(game.world0_hunt_word(&deer, 15) < 160.0_f32.to_bits());
        assert!(game.players[0].res[0] > initial_food);
        println!("Hunt seed=2 prey={deer_id} corpse={corpse} first_hit={first_hit} kill={kill} deposit={} stock={} food={}",
            game.tick, f32::from_bits(game.world0_hunt_word(&deer, 15)), game.players[0].res[0]);
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn fresh_death_valley2_uses_native_seat_order_and_persists_actual_opening() {
        prepare_authenticated_inputs();
        for (seed, ai, markers) in [(1, 1, &[1, 0][..]), (2, 1, &[0, 1][..]),
            (171, 2, &[1, 0, 2][..]), (171, 3, &[0, 3, 1, 2][..])] {
            let mut game = prepare_fresh_game(seed, ai).unwrap_or_else(|| panic!("start seed {seed} with {ai} AI"));
            assert_eq!(game.high, markers.len() * 3);
            assert_eq!(game.players[..markers.len()].iter().filter(|p| p.ai).count(), ai as usize);
            assert_eq!(game.map.world_dimensions(), [100, 300]);
            assert_eq!(game.rect_population.as_ref().unwrap().source().units().len(), 766);
            assert!(!game.map.res.contains(&crate::map::R_FISH));
            for (seat, &marker) in markers.iter().enumerate() {
                let [x, y, _] = START_CITIES[marker];
                let base = seat * 3;
                let [city, citizen, scout] = [game.ents[base], game.ents[base + 1], game.ents[base + 2]];
                assert_eq!([city.def, citizen.def, scout.def], [TC, WORKER, SCOUT]);
                assert_eq!([city.team, citizen.team, scout.team], [seat as u8; 3]);
                assert_eq!([city.x, city.y, city.rot], [x, y, SOURCE_YAW]);
                let region = game.map.region_at(game.map.tile_of(x, y)).unwrap();
                assert_eq!(game.towner[region], seat as u8);
            }
            let mut expected = NativeRng::from_state(seed);
            for _ in 0..167 + 7 * markers.len() { expected.next_u32(); }
            assert_eq!(game.source_birth.rng, expected);
            game.save();
            let bytes = game.savebuf.clone();
            assert_eq!(&bytes[..8], b"DVS1\x01\0\0\0");
            let mut loaded = Game::load(&bytes).unwrap();
            assert_eq!(loaded.state_hash(), game.state_hash());
            loaded.save();
            assert_eq!(loaded.savebuf, bytes);
            for _ in 0..5 {
                game.tick();
                loaded.tick();
                assert_eq!(loaded.state_hash(), game.state_hash());
            }
        }
        assert!(prepare_fresh_game(7, 0).is_none());
        assert!(prepare_fresh_game(7, 4).is_none());
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn fresh_death_valley2_search_accepts_varied_seed_words() {
        prepare_authenticated_inputs();
        let mut seeds = NativeRng::from_state(42);
        let words: Vec<_> = [0, u32::MAX].into_iter().chain((0..96).map(|_| seeds.next_u32())).collect();
        for ai in 1..=3 {
            let game = staged_game(7, ai).unwrap();
            for &seed in &words {
                assert!(source_start_plan(&game, seed).is_some(), "seed {seed:#010x}, AI {ai}");
            }
        }
    }
}
