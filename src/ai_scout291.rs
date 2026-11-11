//! Bounded exploration for the existing portable opponent, not a port of the
//! original strategic planner. Native evidence184/183 establishes owner-local
//! target knowledge and directed territory/movement admission; it does not
//! establish the sixteen-sector policy as the original exploration algorithm.
use super::*;

fn sector_point([width, height]: [usize; 2], sector: u32) -> [f32; 2] {
    let sector = sector % 16;
    [width as f32 * (2 * (sector % 4) + 1) as f32 / 8.0,
     height as f32 * (2 * (sector / 4) + 1) as f32 / 8.0]
}

fn best_goal(map: &Map, sector: u32, dc: usize, reachable: &[u8],
    admit: impl Fn(usize, usize, usize) -> bool) -> Option<[f32; 2]> {
    let dimensions @ [width, height] = map.world_dimensions();
    if width < 3 || height < 3 { return None; }
    let anchor = sector_point(dimensions, sector);
    let mut best = None;
    let mut distance = f32::INFINITY;
    // Search only the actual authored domain, never the padded backing store.
    // Tile-index order supplies the deterministic tie break. No actor knowledge
    // or enemy position is read and no RNG or navigation cache is changed.
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let tile = map.tile_index(x, y);
            if reachable[tile] == 255 || !map.pass[dc][tile]
                || map.terr[tile] as usize >= map.tseeds.len() || !admit(tile, x, y) { continue; }
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let delta = (point[0] - anchor[0]).powi(2) + (point[1] - anchor[1]).powi(2);
            if delta < distance {
                distance = delta;
                best = Some(point);
            }
        }
    }
    best
}

#[cfg(test)]
fn admitted_goal(map: &Map, sector: u32, dc: usize, start: usize, denied: RegionMask) -> Option<[f32; 2]> {
    best_goal(map, sector, dc, &map.scouting_component(dc, start, denied), |_, _, _| true)
}

impl Game {
    pub(super) fn scout_rectangular_opening(&mut self, army: &[usize]) {
        if self.rect_population.is_none() || self.map.world_dimensions() == [N, N] { return; }
        // Skirmish_Default.aip permits a single-unit Explore task; an initial
        // Scout must not wait for the separate eight-unit attack-base wave.
        // This remains a portable goal policy, not the full native TaskManager.
        let interval = (f64::from(self.sim_elapsed()) / 20.0) as u32;
        for &index in army {
            let actor = self.ents[index];
            if !actor.alive || actor.def != 57 || actor.order != O_IDLE { continue; }
            let sector = (interval + u32::from(actor.team) * 5 + self.seed0) % 16;
            let Some([x, y]) = self.rectangular_scout_goal(&[index], sector) else { continue; };
            if (x - actor.x).powi(2) + (y - actor.y).powi(2) > 1.0 {
                self.order_move(&[index], x, y, false);
            }
        }
    }

    pub(super) fn rectangular_scout_goal(&self, army: &[usize], sector: u32) -> Option<[f32; 2]> {
        let dimensions = self.map.world_dimensions();
        if dimensions == [N, N] {
            // Preserve the established square-map command bytes exactly,
            // including integer goals rather than snapping to tile centres.
            return Some(sector_point(dimensions, sector));
        }
        for &index in army {
            let actor = self.ents[index];
            let definition = DEFS[actor.def as usize];
            if !actor.alive || definition.building { continue; }
            #[cfg(feature = "ee2-native13")]
            let denied = self.denied_border_territories(&actor);
            #[cfg(not(feature = "ee2-native13"))]
            let denied = 0;
            if definition.dom == AIR {
                let point = sector_point(dimensions, sector);
                if self.map.permits_territory(self.map.tile_of(point[0], point[1]), denied) {
                    return Some(point);
                }
            } else {
                let dc = usize::from(definition.dom != LAND);
                let component = self.map.scouting_component(dc, self.map.tile_of(actor.x, actor.y), denied);
                // Sector centres alone cannot reveal long-map edge cities.
                // Continue into genuinely unexplored, admitted cells, and try
                // the next sector when the preferred one is already known.
                // This never consults hidden enemy actors or their positions.
                for offset in 0..16 {
                    let selected = (sector + offset) % 16;
                    if let Some(point) = best_goal(&self.map, selected, dc, &component, |tile, x, y|
                        x * 4 / dimensions[0] == selected as usize % 4
                            && y * 4 / dimensions[1] == selected as usize / 4
                            && self.real_fog(actor.team, tile) == 0) {
                        return Some(point);
                    }
                }
                if let Some(point) = best_goal(&self.map, sector, dc, &component, |_, _, _| true) {
                    return Some(point);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(width: usize, height: usize) -> Map {
        let stride = width.next_power_of_two();
        let mut map = Map::blank_with_layout(17, MapLayout {width, height, stride, rows: NN / stride});
        map.tseeds.push((1.0, 1.0));
        for y in 0..height {
            for x in 0..width { let tile = map.tile_index(x, y); map.pass[0][tile] = true; }
        }
        map
    }

    #[test]
    fn rectangular_sectors_cover_both_orientations_and_keep_square_command_coordinates() {
        for (dimensions, first, last) in [([100, 300], [12.5, 37.5], [87.5, 262.5]),
                                         ([300, 100], [37.5, 12.5], [262.5, 87.5])] {
            let map = rectangle(dimensions[0], dimensions[1]);
            let start = map.tile_index(5, 5);
            assert_eq!(admitted_goal(&map, 0, 0, start, 0), Some(first));
            assert_eq!(admitted_goal(&map, 15, 0, start, 0), Some(last));
            for sector in 0..16 {
                let point = admitted_goal(&map, sector, 0, start, 0).unwrap();
                assert!(point[0] > 0.0 && point[0] < dimensions[0] as f32);
                assert!(point[1] > 0.0 && point[1] < dimensions[1] as f32);
                assert_eq!(point, sector_point(dimensions, sector));
            }
        }
        let mut game = Game::new(17, 1, 0, 1, 0);
        let before = game.rng.0;
        let expected = [32.0, 96.0, 160.0, 224.0];
        for sector in 0..16 {
            assert_eq!(game.rectangular_scout_goal(&[], sector),
                       Some([expected[(sector % 4) as usize], expected[(sector / 4) as usize]]));
        }
        assert_eq!(game.rng.0, before);
        // The read-only square path must not touch navigation cache state.
        assert_eq!(game.map.budget, 4);
        game.map.fields.clear();
        assert_eq!(game.rectangular_scout_goal(&[], 15), Some([224.0, 224.0]));
    }

    #[test]
    fn unreachable_island_and_denied_territory_are_not_selected_or_corner_cut() {
        let mut map = rectangle(100, 300);
        // An impassable full-width band separates the nominal southern sector.
        for x in 0..100 { let tile = map.tile_index(x, 200); map.pass[0][tile] = false; }
        let start = map.tile_index(5, 5);
        let goal = admitted_goal(&map, 15, 0, start, 0).unwrap();
        assert_eq!(goal, [87.5, 199.5]);
        assert!(goal[1] < 200.0);
        // A directed border restriction is part of the same connectivity graph.
        map.tseeds.push((75.0, 100.0));
        for y in 0..200 { for x in 50..100 { let tile = map.tile_index(x, y); map.terr[tile] = 1; } }
        let goal = admitted_goal(&map, 15, 0, start, 1 << 1).unwrap();
        assert_eq!(goal, [49.5, 199.5]);
        assert_eq!(map.terr[map.tile_of(goal[0], goal[1])], 0);
        // Actual flow-field diagonal admission, rather than a second flood rule.
        map.pass[0].fill(false);
        for (x,y) in [(4,4),(5,5)] { let tile = map.tile_index(x,y); map.pass[0][tile] = true; }
        assert_eq!(admitted_goal(&map, 15, 0, map.tile_index(4,4), 0), Some([4.5,4.5]));
    }

    #[test]
    fn padding_invalid_regions_and_empty_component_cannot_supply_a_goal() {
        let mut map = rectangle(100, 300);
        map.pass[0].fill(true); // Deliberately tempting padded backing tiles.
        map.terr.fill(255);
        assert_eq!(admitted_goal(&map, 15, 0, map.tile_index(5,5), 0), None);
        let tile = map.tile_index(8,280);
        map.terr[tile] = 0;
        assert_eq!(admitted_goal(&map, 15, 0, map.tile_index(5,5), 0), Some([8.5,280.5]));
        map.pass[0].fill(false);
        assert_eq!(admitted_goal(&map, 15, 0, map.tile_index(5,5), 0), None);
    }

    #[test]
    fn real_actor_query_preserves_rng_cache_and_reports_no_admitted_goal() {
        let mut game = Game::new(17, 1, 0, 1, 0);
        game.map = rectangle(100, 300);
        let soldier = game.spawn(9, 1, 5.5, 5.5);
        let rng = game.rng.0;
        let version = game.map.nav_ver;
        let budget = game.map.budget;
        let field_keys: Vec<_> = game.map.fields.iter().map(|field| field.key).collect();
        assert_eq!(game.rectangular_scout_goal(&[soldier], 15), Some([87.5, 262.5]));
        assert_eq!(game.rng.0, rng);
        assert_eq!((game.map.nav_ver, game.map.budget), (version, budget));
        assert_eq!(game.map.fields.iter().map(|field| field.key).collect::<Vec<_>>(), field_keys);
        game.map.pass[0].fill(false);
        assert_eq!(game.rectangular_scout_goal(&[soldier], 15), None);
        game.ents[soldier].alive = false;
        assert_eq!(game.rectangular_scout_goal(&[soldier], 15), None);
    }

    fn food_fixture() -> (Game, usize, u32) {
        use crate::native_valley37::rectangular_world::{spatial_tests::prepare_authenticated_inputs, staged_game};
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 1).unwrap();
        for team in 0..2 {
            game.players[team].alive = true;
            game.players[team].ai = false;
            game.players[team].population_limit = 250;
            game.players[team].res = [400.0; 3];
            game.players[team].recompute();
            game.rebuild_modifiers(team as u8, false);
        }
        let prey = 937;
        let point = game.rect_population.as_ref().unwrap().unit(prey).unwrap().xyz;
        game.spawn(TC, 0, 13.75, 14.25);
        game.spawn(TC, 1, 89.75, 290.25);
        let worker = game.spawn(WORKER, 0, point[0] - 0.5, point[1] - 0.5);
        game.build_grid();
        (game, worker, prey)
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn food_planner_requires_current_sight_and_preserves_cargo_on_real_hunt_order() {
        let (mut game, worker, prey) = food_fixture();
        assert_eq!(game.nearest_rect_ai_prey(worker, 10000.0), None);
        game.update_fog();
        let rng = game.rng.0;
        assert_eq!(game.nearest_rect_ai_prey(worker, 10000.0).map(|row| row.0), Some(prey));
        assert_eq!(game.rng.0, rng);
        // Existing cargo must remain its original resource through hunting and
        // the normal attack-to-carcass-gather transition, not become free Food.
        game.ents[worker].order = O_GATHER;
        game.ents[worker].q[0] = u32::from(R_WOOD);
        game.ents[worker].q[2] = u32::from(R_WOOD);
        game.ents[worker].carry = 7.0;
        assert!(game.ai_assign_rect_gather(0, worker, R_FOOD));
        assert_eq!(game.ents[worker].order, O_RECT_HUNT);
        assert_eq!(game.ents[worker].q[3], prey);
        assert_eq!((game.ents[worker].q[0], game.ents[worker].q[2], game.ents[worker].carry),
                   (u32::from(R_WOOD), u32::from(R_WOOD), 7.0));
        assert_eq!(game.rng.0, rng, "issuing Hunt does not preconsume its later attack RNG");
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn lone_authored_scout_explores_without_an_attack_wave_or_rng_draw() {
        let (mut game, _, _) = food_fixture();
        let scout = game.spawn(57, 0, 10.5, 14.5);
        game.update_fog();
        let rng = game.rng.0;
        let deadline = game.players[0].ai_next_attack;
        game.scout_rectangular_opening(&[scout]);
        assert_eq!(game.ents[scout].order, O_MOVE);
        assert!((1.0..100.0).contains(&game.ents[scout].ox));
        assert!((1.0..300.0).contains(&game.ents[scout].oy));
        assert_eq!(game.rng.0, rng);
        assert_eq!(game.players[0].ai_next_attack, deadline);
    }

    #[test]
    fn rectangular_exploration_reaches_unknown_edge_cells_instead_of_revisiting_centres() {
        let mut game = Game::new(17, 1, 0, 1, 0);
        game.map = rectangle(100, 300);
        let scout = game.spawn(57, 1, 5.5, 5.5);
        game.team_vis.fill(128);
        let edge = game.map.tile_index(18, 290);
        game.team_vis[NN + edge] = 0;
        assert_eq!(game.rectangular_scout_goal(&[scout], 0), Some([18.5, 290.5]));
        // A hidden but denied or disconnected tile must not attract scouting.
        game.map.terr[edge] = 255;
        assert_eq!(game.rectangular_scout_goal(&[scout], 0), Some([12.5, 37.5]));
    }
}
