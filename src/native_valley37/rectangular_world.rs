//! Authenticated rectangular geometry and territory topology staging.
use super::{geometry, Geometry};
use crate::{
    game::Game,
    map::{Map, MapLayout, T_DEEP, T_FOREST, T_GRASS, T_ROCK, T_SAND, T_SHALLOW},
};
use std::cell::RefCell;

const NO_REGION: u8 = u8::MAX;
const MAX_REGIONS: usize = 64;
const WIDTH: usize = 100;
const HEIGHT: usize = 300;

struct RectangularWorld {
    geometry: Geometry,
    compact_regions: Vec<u8>,
    surface_index: Vec<Option<usize>>,
    adjacency: [u64; MAX_REGIONS],
}

impl RectangularWorld {
    fn decode(input: &[u8]) -> Option<Self> {
        let geometry = Geometry::decode(input)?;
        if geometry.input_sha != super::catalog::DEATH_VALLEY2_INPUT_SHA
            || [geometry.width, geometry.height] != [WIDTH, HEIGHT]
            || geometry.territories.len() != 43
            || geometry.starts.len() != 4
            || geometry.source_cliffs as usize != geometry.surfaces.len()
        {
            return None;
        }
        let compact_regions: Vec<u8> = geometry.territory_ids.iter().map(|&source_id| {
            geometry.territories.iter().position(|region| region.source_id == source_id)
                .map_or(NO_REGION, |region| region as u8)
        }).collect();
        let mut adjacency = [0; MAX_REGIONS];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = y * WIDTH + x;
                let region = compact_regions[index];
                if region == NO_REGION { continue; }
                let east = (x + 1 < WIDTH).then(|| compact_regions[index + 1]);
                let south = (y + 1 < HEIGHT).then(|| compact_regions[index + WIDTH]);
                for neighbor in east.into_iter().chain(south) {
                    if neighbor != NO_REGION && neighbor != region {
                        adjacency[region as usize] |= 1 << neighbor;
                        adjacency[neighbor as usize] |= 1 << region;
                    }
                }
            }
        }
        let mut surface_index = vec![None; WIDTH * HEIGHT];
        for (index, surface) in geometry.surfaces.iter().enumerate() {
            surface_index[surface.tile] = Some(index);
        }
        Some(Self { geometry, compact_regions, surface_index, adjacency })
    }

    fn height(&self, x: f32, y: f32, deformed: bool) -> f64 {
        source_height(&self.geometry, &self.surface_index, x, y, deformed)
    }

    fn map(&self, seed: u32) -> Map {
        let mut map = Map::blank_with_layout(seed, MapLayout::DEATH_VALLEY2);
        map.ter.fill(T_ROCK);
        map.terr.fill(NO_REGION);
        map.territory_neighbors = self.adjacency;
        map.blocked.fill(true);
        map.tseeds = self.geometry.territories.iter().map(|row| (row.center[0], row.center[1])).collect();
        for (slot, &([x, y], _)) in self.geometry.starts.iter().enumerate() {
            map.starts[slot] = (x as usize, y as usize);
        }
        let corner_stride = map.corner_stride();
        for (y, row) in map.hm.chunks_exact_mut(corner_stride).enumerate() {
            let first = y.min(HEIGHT) * (WIDTH + 1);
            let source = &self.geometry.heights[first..first + WIDTH + 1];
            row[..=WIDTH].copy_from_slice(source);
            row[WIDTH + 1..].fill(source[WIDTH]);
        }
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = map.tile_index(x, y);
                let material = self.geometry.materials[y * (WIDTH + 1) + x] as usize;
                map.ter[tile] = match self.geometry.move_palette[material] {
                    7 => T_DEEP,
                    6 => T_SHALLOW,
                    2 | 3 => T_ROCK,
                    4 => T_FOREST,
                    5 => T_SAND,
                    _ => T_GRASS,
                };
                map.h[tile] = self.height(x as f32 + 0.5, y as f32 + 0.5, true) as f32;
                map.terr[tile] = self.compact_regions[y * WIDTH + x];
                map.blocked[tile] = false;
                map.upd_pass(tile);
                map.recolor(tile);
            }
        }
        map.rect_surface_index.clone_from(&self.surface_index);
        map.source_world = Some(Box::new(self.geometry.clone()));
        map
    }
}

pub(crate) fn source_height(geometry: &Geometry, surface_index: &[Option<usize>], x: f32, y: f32, deformed: bool) -> f64 {
    if !(0.0..=WIDTH as f32).contains(&x) || !(0.0..=HEIGHT as f32).contains(&y) {
        return f64::NAN;
    }
    let index = |coordinate: f32| (((coordinate as f64 + 103079215104.0).to_bits() >> 16) as u32) as usize;
    let (col, row) = (index(x), index(y));
    let tile = (col < WIDTH && row < HEIGHT).then_some(row * WIDTH + col);
    if deformed {
        if let Some(surface) = tile.and_then(|tile| surface_index[tile]) {
            if let Some(value) = geometry.surfaces[surface].triangles.iter()
                .find_map(|face| cliff_height(*face, x, y))
            {
                return value as f64;
            }
        }
    }
    let u = x - col as f32;
    let v = y - row as f32;
    let corner = |i: usize, j: usize| geometry.heights[j.min(HEIGHT) * (WIDTH + 1) + i.min(WIDTH)];
    let (a, b, c, d) = (corner(col,row), corner(col+1,row), corner(col,row+1), corner(col+1,row+1));
    let alternate = tile.is_some_and(|tile| geometry.patterns[tile] & 0xff0000 != 0);
    let (af,bf,cf,df,uf,vf) = (a as f64,b as f64,c as f64,d as f64,u as f64,v as f64);
    if alternate {
        let sum = (u + v) as f64;
        let diagonal = (c - b) as f64;
        if sum > 1.0 { bf + (sum - 1.0) * (df - bf) + (1.0 - uf) * diagonal }
        else { bf + (1.0 - sum) * (af - bf) + vf * diagonal }
    } else {
        let diagonal = (d - a) as f64;
        if u <= v { af + uf * diagonal + (vf - uf) * (cf - af) }
        else { af + vf * diagonal + (uf - vf) * (bf - af) }
    }
}

// 006FE3C0 keeps one XY difference in the register. The other components are
// written to f32 before the face determinant and barycentric products.
fn cliff_height([a,b,c]: [[f32;3];3], x: f32, y: f32) -> Option<f32> {
    let ux = (b[0] - a[0]) as f64;
    let uy = (b[1] - a[1]) as f64;
    let vx = (c[0] - a[0]) as f64;
    let vy = c[1] as f64 - a[1] as f64;
    let dx = (x - a[0]) as f64;
    let dy = (y - a[1]) as f64;
    let det = ux * vy - uy * vx;
    if det == 0.0 { return None; }
    let sign = det.signum();
    let beta = (dx * vy - dy * vx) * sign;
    let gamma = (ux * dy - uy * dx) * sign;
    if beta < 0.0 || gamma < 0.0 || beta + gamma > det.abs() { return None; }
    let az = a[2] as f64;
    Some((az + ((c[2] as f64 - az) * gamma + (b[2] as f64 - az) * beta) / det.abs()) as f32)
}

#[cfg(feature = "ee2-rect251-animal-slope")]
pub(super) fn ground_grade_height(geometry: &Geometry, surfaces: &[Option<usize>], x: f32, y: f32) -> f64 {
    chop_height(geometry, surfaces, x.clamp(0.0, WIDTH as f32), y.clamp(0.0, HEIGHT as f32))
}

#[cfg(feature = "ee2-rect254-aircraft")]
pub(super) fn aircraft_height(geometry: &Geometry, surfaces: &[Option<usize>], [x, y]: [f32; 2]) -> f64 {
    // 0070FC10 uses the original fractional point and clamps corner indices separately.
    chop_height(geometry, surfaces, x, y)
}

#[cfg(feature = "ee2-rect251-animal-slope")]
fn chop_height(geometry: &Geometry, surfaces: &[Option<usize>], x: f32, y: f32) -> f64 {
    use super::deer_foot247::{store, Rounding};
    let rounding = Rounding::Chop53;
    let index = |coordinate: f32| ((rounding.add(f64::from(coordinate), 103079215104.0).to_bits() >> 16) as u32) as i32;
    let (col, row) = (index(x), index(y));
    let tile = (0..WIDTH as i32).contains(&col).then_some(col as usize)
        .zip((0..HEIGHT as i32).contains(&row).then_some(row as usize)).map(|(x, y)| y * WIDTH + x);
    if let Some(surface) = tile.and_then(|tile| surfaces[tile]) {
        if let Some(height) = geometry.surfaces[surface].triangles.iter()
            .find_map(|face| grade_cliff_height(*face, x, y)) {
            return f64::from(height);
        }
    }
    let u = f64::from(store(rounding.sub(f64::from(x), col as f64), rounding));
    let v = f64::from(store(rounding.sub(f64::from(y), row as f64), rounding));
    let corner = |i: i32, j: i32| f64::from(
        geometry.heights[j.clamp(0, HEIGHT as i32) as usize * (WIDTH + 1) + i.clamp(0, WIDTH as i32) as usize]);
    let [a, b, c, d] = [corner(col, row), corner(col + 1, row), corner(col, row + 1), corner(col + 1, row + 1)];
    let alternate = tile.is_some_and(|tile| geometry.patterns[tile] & 0xff0000 != 0);
    if alternate {
        let sum = rounding.add(u, v);
        let sum32 = f64::from(store(sum, rounding));
        let diagonal = f64::from(store(rounding.sub(c, b), rounding));
        let (edge, cross) = if sum > 1.0 {
            (rounding.mul(rounding.sub(sum32, 1.0), rounding.sub(d, b)), rounding.mul(rounding.sub(1.0, u), diagonal))
        } else {
            (rounding.mul(rounding.sub(1.0, sum32), rounding.sub(a, b)), rounding.mul(v, diagonal))
        };
        return rounding.add(rounding.add(edge, cross), b);
    }
    let diagonal = f64::from(store(rounding.sub(d, a), rounding));
    let (side, fraction, diagonal_fraction) = if u > v { (b, rounding.sub(u, v), v) }
        else { (c, rounding.sub(v, u), u) };
    let edge = rounding.mul(rounding.sub(side, a), fraction);
    rounding.add(rounding.add(edge, rounding.mul(diagonal_fraction, diagonal)), a)
}

#[cfg(feature = "ee2-rect251-animal-slope")]
fn grade_cliff_height([a, b, c]: [[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    use super::deer_foot247::{store, Rounding};
    let rounding = Rounding::Chop53;
    let diff = |a: f32, b: f32| rounding.sub(f64::from(a), f64::from(b));
    let spill = |a, b| f64::from(store(diff(a, b), rounding));
    let dx = spill(x, a[0]);
    let dy = spill(y, a[1]);
    let ux = spill(b[0], a[0]);
    let uy = spill(b[1], a[1]);
    let vx = spill(c[0], a[0]);
    let wide_vy = diff(c[1], a[1]);
    let vy = f64::from(store(wide_vy, rounding));
    let det = rounding.sub(rounding.mul(ux, wide_vy), vx * uy);
    if det == 0.0 { return None; }
    let sign = det.signum();
    let beta = sign * rounding.sub(dx * vy, dy * vx);
    let gamma = sign * rounding.sub(ux * dy, uy * dx);
    if beta < 0.0 || gamma < 0.0 || rounding.add(beta, gamma) > det.abs() { return None; }
    let height = rounding.add(rounding.mul(diff(c[2], a[2]), gamma), rounding.mul(diff(b[2], a[2]), beta));
    Some(store(rounding.add(rounding.div(height, det.abs()), f64::from(a[2])), rounding))
}

#[derive(Default)]
struct Stage {
    input: Vec<u8>,
    world: Option<RectangularWorld>,
    population: super::population234_stage::PendingPopulation,
    #[cfg(feature = "ee2-rect250-animals")]
    animal_input: Vec<u8>,
    #[cfg(feature = "ee2-rect250-animals")]
    animals: Option<super::animal_live250::AnimalManager>,
}

thread_local! {
    static STAGE: RefCell<Stage> = RefCell::new(Stage::default());
}

/// Build an uncommitted authenticated map and its immutable World0 catalog.
pub(crate) fn staged_game(seed: u32, ai: u32) -> Option<Game> {
    if !(1..=3).contains(&ai) { return None; }
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let world = stage.world.as_ref()?;
        let mut game = Game::blank(world.map(seed), seed, ai, 0, 1, 0, ai as usize + 1, false);
        if !stage.population.ready()?.attach(&mut game) { return None; }
        #[cfg(feature = "ee2-rect235-forest")]
        game.attach_rect_forest()?;
        Some(game)
    })
}

#[no_mangle]
pub extern "C" fn native_rect_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.input.clear();
        if !(84..=geometry::MAX_INPUT).contains(&length) { return std::ptr::null_mut(); }
        stage.input.resize(length, 0);
        stage.input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn native_rect_prepare() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        let Some(world) = RectangularWorld::decode(&stage.input) else { return 0; };
        stage.world = Some(world);
        stage.population.reset();
        #[cfg(feature = "ee2-rect250-animals")]
        { stage.animals = None; stage.animal_input.clear(); }
        1
    })
}

#[cfg(feature = "ee2-rect250-animals")]
pub(super) fn staged_animals() -> Option<super::animal_live250::AnimalManager> {
    STAGE.with(|stage| stage.borrow().animals.clone())
}

#[cfg(feature = "ee2-rect250-animals")]
#[no_mangle]
pub extern "C" fn native_rect_animal_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.animal_input.clear();
        stage.animals = None;
        if length != super::animal_live250::INPUT_LENGTH { return std::ptr::null_mut(); }
        stage.animal_input.resize(length, 0);
        stage.animal_input.as_mut_ptr()
    })
}

#[cfg(feature = "ee2-rect250-animals")]
#[no_mangle]
pub extern "C" fn native_rect_animal_prepare() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.animals = stage.population.ready().and_then(|population|
            super::animal_live250::AnimalManager::decode(&stage.animal_input, population));
        u32::from(stage.animals.is_some())
    })
}

#[no_mangle]
pub extern "C" fn native_rect_info(field: u32) -> u32 {
    STAGE.with(|stage| stage.borrow().world.as_ref().map_or(0, |world| {
        let geometry = &world.geometry;
        match field {
            0 => geometry.width as u32,
            1 => geometry.height as u32,
            2 => geometry.territories.len() as u32,
            3 => geometry.starts.len() as u32,
            4 => geometry.source_cliffs,
            5 => geometry.pending_units,
            6 => geometry.pending_forest,
            _ => 0,
        }
    }))
}

#[no_mangle]
pub extern "C" fn native_rect_ptr(field: u32) -> *const u8 {
    STAGE.with(|stage| stage.borrow().world.as_ref().map_or(std::ptr::null(), |world| {
        let geometry = &world.geometry;
        match field {
            0 => geometry.heights.as_ptr().cast(),
            1 => geometry.materials.as_ptr().cast(),
            2 => geometry.tints.as_ptr().cast(),
            3 => geometry.patterns.as_ptr().cast(),
            4 => geometry.source_sha.as_ptr(),
            5 => geometry.territory_ids.as_ptr().cast(),
            6 => world.compact_regions.as_ptr(),
            _ => std::ptr::null(),
        }
    }))
}

#[no_mangle]
pub extern "C" fn native_rect_adjacent(region: usize) -> u64 {
    STAGE.with(|stage| stage.borrow().world.as_ref().and_then(|world|
        world.adjacency.get(region).copied()).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn native_rect_start(slot: usize, field: usize) -> f32 {
    STAGE.with(|stage| {
        let stage = stage.borrow();
        let Some((point, priority)) = stage.world.as_ref()
            .and_then(|world| world.geometry.starts.get(slot)) else { return f32::NAN; };
        match field {
            0 | 1 => point[field],
            2 => *priority as f32,
            _ => f32::NAN,
        }
    })
}

#[no_mangle]
pub extern "C" fn native_rect_height(x: f32, y: f32, deformed: u32) -> f32 {
    STAGE.with(|stage| stage.borrow().world.as_ref().map_or(f32::NAN, |world| {
        world.height(x, y, deformed != 0) as f32
    }))
}

#[cfg(feature = "ee2-rect251-animal-slope")]
#[no_mangle]
pub extern "C" fn native_rect_grade_height(x: f32, y: f32) -> f64 {
    if !x.is_finite() || !y.is_finite() { return f64::NAN; }
    super::active_game().filter(|game| game.rect_animal_grade_active())
        .map_or(f64::NAN, |game| game.rect_grade_height([x, y]))
}

#[no_mangle]
pub extern "C" fn native_rect_ray_start_height(x: f32, y: f32) -> f32 {
    STAGE.with(|stage| stage.borrow().world.as_ref().map_or(f32::NAN, |world| {
        let height = world.height(x, y, true);
        if height.is_nan() { f32::NAN } else { (height.max(0.0) + 0.01_f32 as f64) as f32 }
    }))
}

#[cfg(test)]
pub(crate) mod spatial_tests {
    use super::*;
    use crate::{cmd, defs::{HOUSE, TC, WORKER}, game::O_MOVE, map::R_WOOD};

    pub(crate) fn prepare_authenticated_inputs() {
        let terrain = crate::native_valley37::private_test_fixtures::read("world.w8in");
        let population = crate::native_valley37::private_test_fixtures::read("world0.dvp1");
        STAGE.with(|stage| {
            let stage = &mut *stage.borrow_mut();
            let world = RectangularWorld::decode(&terrain).expect("private Death Valley2 W8IN");
            stage.population.reset();
            let buffer = stage.population.alloc(population.len());
            assert!(!buffer.is_null());
            // The ABI destination is the stage-owned buffer just allocated to this exact length.
            unsafe { std::ptr::copy_nonoverlapping(population.as_ptr(), buffer, population.len()); }
            assert!(stage.population.prepare(&world.geometry));
            stage.world = Some(world);
        });
    }

    #[cfg(feature = "ee2-rect270-city-actors")]
    #[test] #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn overlapping_reservations_survive_removal_and_cold_reload() {
        use crate::game::NOIDX;

        prepare_authenticated_inputs();
        let mut game = staged_game(171, 1).unwrap();
        for (seat, player) in game.players[..game.nplayers].iter_mut().enumerate() {
            player.alive = true;
            player.ai = seat != 0;
        }
        let first = game.spawn(HOUSE, 0, 78.0, 24.0);
        let second = game.spawn(HOUSE, 1, 79.0, 24.0);
        assert_ne!(first, NOIDX);
        assert_ne!(second, NOIDX);
        let cleared = game.map.tile_index(77, 23);
        let shared = game.map.tile_index(78, 23);
        assert!(game.map.blocked[cleared] && game.map.blocked[shared]);
        game.kill(first);
        assert!(!game.map.blocked[cleared] && game.map.blocked[shared]);
        assert!(game.map.pass[0][cleared] && !game.map.pass[0][shared]);

        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.map.blocked, game.map.blocked);
        assert_eq!(loaded.map.pass[0], game.map.pass[0]);
        loaded.kill(second);
        assert!(!loaded.map.blocked[shared] && loaded.map.pass[0][shared]);
    }

    #[test] #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn authentic_regions_and_world0_survive_high_y_checkpoint() {
        use super::super::{hash::sha256, rectangular_save234 as save};
        use crate::game::{NONE, NOIDX};
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 3).expect("authentic rectangular Game");
        for seat in 0..game.nplayers {
            let player = &mut game.players[seat];
            player.alive = true;
            player.ai = seat != 0;
            player.ai_next_attack = 4800;
            player.population_limit = 250;
            player.res = [400.0; 3];
            player.recompute();
            game.rebuild_modifiers(seat as u8, false);
        }
        let points = game.map.source_world.as_ref().unwrap().starts.clone();
        for (seat, (point, _)) in points.into_iter().enumerate() {
            let tile = game.map.tile_of(point[0], point[1]);
            let region = game.map.region_at(tile).expect("start has a territory");
            let city = game.spawn(TC, seat as u8, point[0], point[1]);
            assert_ne!(city, NOIDX);
            game.territory.claims[region] = game.h(city);
        }
        let region = 42;
        let (x, y) = game.map.tseeds[region];
        let region_tile = game.map.tile_of(x, y);
        let [x, y] = if game.map.region_at(region_tile) == Some(region) { [x, y] } else {
            let tile = game.map.terr.iter().position(|&id| usize::from(id) == region).unwrap();
            game.map.tile_xy(tile).map(|axis| axis as f32 + 0.5)
        };
        if game.territory.claims[region] == NONE {
            let city = game.spawn(TC, 0, x, y);
            assert_ne!(city, NOIDX);
            game.territory.claims[region] = game.h(city);
        }
        game.build_grid();
        game.update_fog();
        game.save();
        let wire = game.savebuf.clone();
        assert!(wire.starts_with(save::MAGIC));
        let (_, locator) = save::split(&wire).unwrap();
        let locator = locator.unwrap();
        assert_eq!(locator.claims[10], game.territory.claims[42]);
        assert_ne!(locator.claims[10], NONE);
        let mut resumed = Game::load(&wire).expect("DVS1 restore");
        assert_eq!(resumed.state_hash(), game.state_hash());
        assert_eq!(resumed.map.territory_neighbors, game.map.territory_neighbors);
        assert_eq!(resumed.towner, game.towner);
        assert_eq!(resumed.rect_population.as_ref().unwrap().source().units().len(), 766);
        resumed.save();
        assert!(resumed.savebuf == wire);
        if std::env::var_os("EE2_WRITE_RECT_CHECKPOINT").is_some() {
            std::fs::write("../private-host-checkpoint.dvs1", &wire).expect("private WASM crossload fixture");
        }
        for _ in 0..8 {
            game.tick();
            resumed.tick();
            assert_eq!(resumed.state_hash(), game.state_hash());
        }

        let resigned = |mut bytes: Vec<u8>| {
            let end = bytes.len() - 32;
            let digest = sha256(&bytes[..end]);
            bytes[end..].copy_from_slice(&digest);
            bytes
        };
        let mut wrong_population = wire.clone();
        wrong_population[96] ^= 1;
        assert!(Game::load(&resigned(wrong_population)).is_none());
        let mut wrong_claim = wire.clone();
        wrong_claim[168..172].copy_from_slice(&0xffff_0000u32.to_le_bytes());
        assert!(Game::load(&resigned(wrong_claim)).is_none());
        STAGE.with(|stage| stage.borrow_mut().population.reset());
        assert!(Game::load(&wire).is_none());
    }

    #[test] #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn authentic_south_supports_live_spatial_queries_and_orders() {
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 1).expect("authentic rectangular Game");
        assert_eq!(game.map.world_dimensions(), [100, 300]);
        assert_eq!(game.map.backing_dimensions(), [128, 512]);
        assert_eq!(game.map.hm.len(), 129 * 513);
        assert_eq!(game.map.ter.len(), 65_536);
        assert_eq!(game.map.tseeds.len(), 43);
        assert_eq!(game.map.starts[2..], [(18, 286), (89, 289)]);
        assert_eq!(game.map.tile_of(18.760_166, 286.549_2), 36_626);
        assert_eq!(game.map.tile_of(89.859_96, 289.962_83), 37_081);
        let mut unassigned = 0;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = game.map.tile_index(x, y);
                assert_eq!(game.map.tile_xy(tile), [x, y]);
                unassigned += usize::from(game.map.terr[tile] == NO_REGION);
                let world_height = game.map.height(x as f32 + 0.5, y as f32 + 0.5);
                assert_eq!(world_height.to_bits(), game.map.h[tile].to_bits());
            }
        }
        assert_eq!(unassigned, 8020);
        assert!(!game.map.pass[0][game.map.tile_index(WIDTH, 266)]);
        assert!(!game.map.pass[1][game.map.tile_index(70, HEIGHT)]);
        assert!(game.map.height(89.859_96, 289.962_83).is_finite());

        assert_eq!(game.map.territory_neighbors.iter().map(|row| row.count_ones()).sum::<u32>(), 150);
        assert_ne!(game.map.territory_neighbors[42], 0);
        assert!(game.map.territory_neighbors.iter().any(|&row| row >> 32 != 0));
        for seat in 0..2 {
            game.players[seat].alive = true;
            game.players[seat].population_limit = 250;
            game.players[seat].res = [400.0; 3];
            game.rebuild_modifiers(seat as u8, false);
        }
        let (x, y) = (264..288).find_map(|y| (3..88).find(|&x| {
            (0..7).all(|dy| (0..9).all(|dx| game.map.pass[0][game.map.tile_index(x + dx, y + dy)]))
        }).map(|x| (x, y))).expect("southern navigable rectangle");
        let point = |dx: usize, dy: usize| (x as f32 + dx as f32 + 0.5, y as f32 + dy as f32 + 0.5);
        let start = point(1, 1);
        let goal = point(7, 1);
        let second = game.map.starts[3];
        game.spawn(TC, 1, second.0 as f32, second.1 as f32);
        let worker = game.spawn(WORKER, 0, start.0, start.1);
        game.update_fog();
        let start_tile = game.map.tile_of(start.0, start.1);
        assert_eq!(game.vis[start_tile], 255);
        assert_ne!(game.vis[crate::map::tile_of(start.0, start.1)], 255);
        let goal_tile = game.map.tile_of(goal.0, goal.1);
        assert_eq!(game.key_for(crate::defs::LAND, goal.0, goal.1), goal_tile as u32);
        let mut slot = u16::MAX;
        assert_eq!(game.map.flow(&mut slot, goal_tile as u32, start_tile, 2, 0), 0);
        assert_eq!(game.map.flow(&mut slot, goal_tile as u32, goal_tile, 2, 0), 8);
        let deposit = game.map.tile_index(x + 7, y + 2);
        game.map.res[deposit] = R_WOOD;
        game.map.amt[deposit] = 100;
        assert_eq!(game.map.nearest_res(goal_tile, R_WOOD, 3), Some(deposit));
        let handle = game.h(worker);
        game.apply(0, cmd::C_MOVE, goal.0, goal.1, 0, &[handle]);
        assert_eq!(game.ents[worker].order, O_MOVE);
        for _ in 0..90 { game.tick(); }
        assert!(game.ents[worker].y > 255.0);
        assert!((game.ents[worker].x - goal.0).abs() < 1.0, "actual worker at {:?}", (game.ents[worker].x, game.ents[worker].y));
        assert!(game.render(1.0, x as f32, y as f32, x as f32 + 9.0, y as f32 + 7.0, -1, 0, 0) >= 2);
        game.minimap();
        assert!(game.mini[goal_tile * 4..goal_tile * 4 + 3].iter().any(|&channel| channel != 0));
        assert!(game.footprint_known(0, HOUSE, x as i32 + 3, y as i32 + 3, 0));
        game.prepare_build_routes((x as i32 + 3, y as i32 + 3, 2, 2));
    }
}

#[no_mangle]
pub extern "C" fn native_rect_population_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| stage.borrow_mut().population.alloc(length))
}

#[no_mangle]
pub extern "C" fn native_rect_population_prepare() -> u32 {
    STAGE.with(|stage| {
        let Stage { world, population, .. } = &mut *stage.borrow_mut();
        let Some(world) = world else { return 0; };
        u32::from(population.prepare(&world.geometry))
    })
}

#[no_mangle]
pub extern "C" fn native_rect_population_info(field: u32) -> u32 {
    STAGE.with(|stage| stage.borrow().population.ready().map_or(0, |p| match field {
        0 => u32::from(p.dimensions()[0]),
        1 => u32::from(p.dimensions()[1]),
        2 => p.units().len() as u32,
        3 => p.forest().len() as u32,
        4 => p.types().len() as u32,
        5 => p.children().len() as u32,
        _ => 0,
    }))
}
