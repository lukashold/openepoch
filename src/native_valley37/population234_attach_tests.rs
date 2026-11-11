use super::{geometry::Geometry, population234::Population, rectangular_world::*};
use crate::{game::Game, map::Map};


fn transfer(data: &[u8], alloc: extern "C" fn(usize) -> *mut u8) {
    let ptr = alloc(data.len());
    assert!(!ptr.is_null());
    // The allocator returns one owned writable range of exactly this length.
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()) };
}

#[test]
#[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
fn population234_attach_preserves_instances_and_world_identity() {
    let geometry = Geometry::decode((&super::private_test_fixtures::read("world.w8in"))).unwrap();
    let parsed = Population::decode((&super::private_test_fixtures::read("world0.dvp1")), geometry.source_sha, geometry.input_sha).unwrap();
    let map = Map::blank(7);
    let mut game = Game::blank(map, 7, 1, 0, 1, 0, 2, false);
    assert!(!parsed.attach(&mut game));
    let mut wrong = geometry.clone();
    wrong.input_sha[0] ^= 1;
    game.map.source_world = Some(Box::new(wrong));
    assert!(!parsed.attach(&mut game));
    assert!(game.rect_population.is_none());
    game.map.source_world = Some(Box::new(geometry));
    let before = (game.high, game.map.res.clone(), game.map.amt.clone());
    assert!(parsed.attach(&mut game));
    assert!(!parsed.attach(&mut game));
    let attached = game.rect_population.as_ref().unwrap();
    assert_eq!(attached.source(), &parsed);
    assert_eq!(attached.unit(2).unwrap().local_id(), 2);
    assert!(attached.unit(u32::MAX).is_none());
    let overlaps: Vec<_> = attached.resources_at(54, 204).collect();
    assert_eq!(overlaps.iter().map(|unit| unit.local_id()).collect::<Vec<_>>(), [924, 888, 884]);
    assert!(overlaps.iter().all(|unit| unit.initial_stock == 1277));
    assert_ne!(overlaps[0].xyz, overlaps[1].xyz);
    let mineral = parsed.units().iter().find(|unit| unit.xyz[1] >= 256.0 && unit.initial_stock > 0).unwrap();
    assert!(attached.resources_at(mineral.xyz[0] as u16, mineral.xyz[1] as u16).any(|row| row == mineral));
    assert_eq!(before, (game.high, game.map.res.clone(), game.map.amt.clone()));
}

#[test]
#[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
fn population234_stage_authenticates_both_inputs_and_is_atomic() {
    transfer((&super::private_test_fixtures::read("world0.dvp1")), native_rect_population_alloc);
    assert_eq!(native_rect_population_prepare(), 0);
    transfer((&super::private_test_fixtures::read("world.w8in")), native_rect_alloc);
    assert_eq!(native_rect_prepare(), 1);
    transfer((&super::private_test_fixtures::read("world0.dvp1")), native_rect_population_alloc);
    assert_eq!(native_rect_population_prepare(), 1);
    assert_eq!([2, 3, 4, 5].map(|field| native_rect_population_info(field)), [766, 1328, 37, 1358]);
    let mut bad = (&super::private_test_fixtures::read("world0.dvp1")).to_vec();
    bad[1024] ^= 1;
    transfer(&bad, native_rect_population_alloc);
    assert_eq!(native_rect_population_prepare(), 0);
    assert_eq!(native_rect_population_info(2), 766);
    let mut bad_map = (&super::private_test_fixtures::read("world.w8in")).to_vec();
    bad_map[1024] ^= 1;
    transfer(&bad_map, native_rect_alloc);
    assert_eq!(native_rect_prepare(), 0);
    assert_eq!(native_rect_info(0), 100);
    assert_eq!(native_rect_population_info(2), 766);
    transfer((&super::private_test_fixtures::read("world.w8in")), native_rect_alloc);
    assert_eq!(native_rect_prepare(), 1);
    assert_eq!(native_rect_population_info(2), 0);
    assert_eq!(native_rect_population_prepare(), 0);
    transfer((&super::private_test_fixtures::read("world0.dvp1")), native_rect_population_alloc);
    assert_eq!(native_rect_population_prepare(), 1);
    assert_eq!(native_rect_population_info(3), 1328);
}
