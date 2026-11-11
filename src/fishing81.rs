//! Installed CSV5 FishingShip and ResourceFish. Random-map ranks follow 0082D0C0.
use std::{cmp::Reverse, collections::BinaryHeap};

use crate::defs::{Def, DefId, QueueItem, A_SHIP, DOCK, NOW, SEA};
use crate::ee2_units::UnitRow;
use crate::game::{Ent, Game, NONE, O_GATHER};
use crate::map::{Map, DIRS, N, NN, R_FISH, R_FOOD, R_NONE, T_DEEP};
#[cfg(feature = "ee2-authored-rect233")]
use crate::native_valley37::world0_resources236::{fish_order, FishSite};

pub(crate) const FISHING_SHIP: DefId = 108;
pub(crate) const SAVE_FLAG: u32 = 1 << 19;
pub(crate) const FISH_AMOUNT: u16 = 2000;
pub(crate) const MAX_FISHERS: usize = 6;
pub(crate) const CARRY_LIMIT: f32 = 30.0;
pub(crate) const GATHER_RANGE: f32 = 0.8;
pub(crate) const DROPOFF_RANGE: f32 = 0.7;

const DEFINITION: Def = Def {
    name: "Fischfangschiff", epoch: 0, until: 15, dom: SEA, building: false, size: 1.5,
    hp: 50.0, speed: 1.725, sight: 5.0, armor: A_SHIP, cost: [0, 30, 0], time: 32.5,
    pop: 1, made_at: DOCK, flags: 0, w: [NOW; 2],
};

pub(crate) const fn append(existing: [Def; 108]) -> [Def; 109] {
    let mut result = [DEFINITION; 109];
    let mut i = 0;
    while i < existing.len() {
        result[i] = existing[i];
        i += 1;
    }
    result
}

const BASE: UnitRow = UnitRow {
    id: FISHING_SHIP, epoch: 0, until: 5, source_epoch: 1, source_unit: "FishingShip",
    source_visual: "FishingShip", source_host: "Dock", identity_verified: true, model_verified: true,
    hp: Some(50.0), buildtime: Some(32.5), cost9: Some([0, 30, 0, 0, 20, 0, 0, 0, 0]),
    pop: Some(1), pop_capacity: None, sight: Some(5.0), raw_size: Some([0.75, 0.5]),
    raw_footprint: None, raw_speed: Some(1.725), gather: Some([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    farm_rate: Some(0.0), carry: Some(CARRY_LIMIT), farm_carry: Some(0.0), repair_rate: None,
    damage: None, range: None, reload: None, garrison_slots: None, generation: None,
    generation_resource: None, garrison_heal: None, garrison_attack_bonus: None,
    garrison_size: None, fortress_garrison: false, attack_missile: None,
};
const ROWS: [UnitRow; 3] = [
    BASE,
    UnitRow { epoch: 5, until: 10, source_epoch: 6, source_visual: "FishingShip_6", hp: Some(75.0), ..BASE },
    UnitRow { epoch: 10, until: 15, source_epoch: 11, source_visual: "FishingShip_11", hp: Some(100.0), ..BASE },
];

pub(crate) fn lookup(definition: DefId, epoch: u8) -> Option<&'static UnitRow> {
    (definition == FISHING_SHIP).then(|| ROWS.iter().find(|row| (row.epoch..row.until).contains(&epoch))).flatten()
}

fn word(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 16) & 0x7fff
}

/// The caller supplies the map-generation RNG state just before the fish phase.
/// Native counts water vertices before placement validation, including shallow water.
fn ranks(water_count: usize, state: &mut u32) -> Vec<usize> {
    let count = (water_count as f64 * f64::from(0.002_f32) + 0.5).floor() as usize;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let rank = (word(state) << 15) | word(state);
        result.push(rank as usize % water_count);
    }
    result.sort_unstable();
    result
}

fn clear_deep_footprint(map: &Map, tile: usize) -> bool {
    let (x, y) = (tile % N, tile / N);
    if x == 0 || y == 0 || x + 1 >= N || y + 1 >= N { return false; }
    [tile - N - 1, tile - N, tile - 1, tile].into_iter().all(|i| {
        map.ter[i] == T_DEEP && map.res[i] == R_NONE && !map.blocked[i]
    })
}

fn center(tile: usize) -> (f32, f32) {
    ((tile % N) as f32 + 0.5, (tile / N) as f32 + 0.5)
}

fn squared_distance(e: &Ent, point: (f32, f32)) -> f32 {
    (e.x - point.0).powi(2) + (e.y - point.1).powi(2)
}

#[cfg(feature = "ee2-authored-rect233")]
const WORLD0_FISH_REACH: f32 = f32::from_bits(0x3fec_cccd);

#[cfg(feature = "ee2-authored-rect233")]
fn world0_fish_distance(x: f32, y: f32) -> f64 {
    // The original spills XY deltas to F32 before its x87 square and sum.
    f64::from(x).powi(2) + f64::from(y).powi(2)
}

#[cfg(feature = "ee2-authored-rect233")]
fn world0_fish_turn(dx: f32, dy: f32, facing: f32) -> Option<f32> {
    if world0_fish_distance(dx, dy) <= f64::from(0.05_f32).powi(2) { return None; }
    let turn = f64::from(core::f32::consts::TAU);
    let half = f64::from(core::f32::consts::PI);
    let bearing = f64::from(dy).atan2(f64::from(dx));
    let mut difference = (f64::from(facing) - bearing) % turn;
    if difference > half { difference -= turn; }
    if difference < -half { difference += turn; }
    (difference.abs() >= 0.25).then_some(bearing as f32)
}

// 0086F5B0 asks the path service for a distance before its Euclidean
// fallback. Use actual portable navigable water and the same corner rules
// as the simulation field for routes to the source-facing Dock access point.
fn water_distances(map: &Map, pass: &[bool], start: usize, ends: &[(usize, (f32, f32))]) -> Vec<u32> {
    let mut distances = vec![u32::MAX; NN];
    if !pass[start] { return distances; }
    let mut goals = vec![false; NN];
    let mut remaining = 0;
    for (_, (x, y)) in ends {
        let tile = map.tile_of(*x, *y);
        if !goals[tile] {
            remaining += 1;
            goals[tile] = true;
        }
    }
    distances[start] = 0;
    let mut frontier = BinaryHeap::from([Reverse((0_u32, start))]);
    while let Some(Reverse((cost, tile))) = frontier.pop() {
        if cost != distances[tile] { continue; }
        if goals[tile] {
            remaining -= 1;
            if remaining == 0 { break; }
        }
        let [x, y] = map.tile_xy(tile).map(|coordinate| coordinate as i32);
        for (direction, (dx, dy)) in DIRS.into_iter().enumerate() {
            let (nx, ny) = (x + dx, y + dy);
            if !map.layout.contains(nx, ny) { continue; }
            let next = map.tile_index(nx as usize, ny as usize);
            let diagonal = direction >= 4;
            if !pass[next] || diagonal && !(pass[map.tile_index(nx as usize, y as usize)]
                && pass[map.tile_index(x as usize, ny as usize)]) {
                continue;
            }
            let next_cost = cost + if diagonal { 1414 } else { 1000 };
            if next_cost < distances[next] {
                distances[next] = next_cost;
                frontier.push(Reverse((next_cost, next)));
            }
        }
    }
    distances
}

fn place_fish(map: &mut Map) {
    // The portable terrain has its own preceding map phases. Adapt their
    // output to the source's negative-height vertex list and seed the fish
    // phase independently so a legacy map consumes no extra simulation RNG.
    let width = N + 1;
    let water: Vec<_> = map.hm.iter().enumerate().filter_map(|(i, height)| (*height < 0.0).then_some(i)).collect();
    let mut fish_state = map.seed;
    let requested = ranks(water.len(), &mut fish_state);
    let sites: Vec<_> = (0..map.ter.len()).filter(|&tile| clear_deep_footprint(map, tile)).collect();
    let mut occupied = vec![false; NN];
    for rank in requested {
        let vertex = water[rank];
        let (x, y) = (vertex % width, vertex / width);
        // The native actor-placement search has a separate contour/raster.
        // Use the corresponding nearest valid deep footprint on this map.
        let tile = sites.iter().copied().filter(|&tile| !occupied[tile])
            .min_by_key(|&tile| (x.abs_diff(tile % N).pow(2) + y.abs_diff(tile / N).pow(2), tile));
        if let Some(tile) = tile {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    occupied[(tile as isize + dy * N as isize + dx) as usize] = true;
                }
            }
            map.res[tile] = R_FISH;
            map.amt[tile] = FISH_AMOUNT;
            map.recolor(tile);
        }
    }
}

impl Game {
    pub(crate) fn fishing_allowed(&self) -> bool {
        #[cfg(feature = "ee2-authored-rect233")]
        if self.rect_population.is_some() {
            return !self.net && self.humans == 1 && self.bench == 0;
        }
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() { return false; }
        let seats = if self.net { (2..=4).contains(&self.humans) && self.grouped_coop.is_none() }
            else { self.humans == 1 };
        seats && self.bench == 0 && self.map.world_dimensions() == [N, N]
    }

    pub(crate) fn configure_fishing(&mut self, network: bool) -> bool {
        if self.net != network || self.tick != 0 || self.fishing_enabled || !self.fishing_allowed() { return false; }
        #[cfg(feature = "ee2-authored-rect233")]
        let authored = self.rect_population.is_some();
        #[cfg(not(feature = "ee2-authored-rect233"))]
        let authored = false;
        if !authored { place_fish(&mut self.map); }
        self.fishing_enabled = true;
        true
    }

    pub(super) fn order_fishing(&mut self, indices: &[usize], tile: usize) {
        if !self.fishing_enabled || self.map.res[tile] != R_FISH ||
            !indices.iter().any(|&index| self.ents[index].def == FISHING_SHIP) { return; }
        let mut routes = Vec::new();
        let (x, y) = center(tile);
        let key = self.key_for(SEA, x, y);
        for &index in indices {
            let actor = &self.ents[index];
            if actor.def != FISHING_SHIP || !self.map.permits_territory(tile, self.denied_border_territories(actor)) { continue; }
            let denied = self.border_egress_mask(actor);
            let route = routes.iter().position(|(mask, _)| *mask == denied).unwrap_or_else(|| {
                let pass = self.fishing_pass(denied);
                routes.push((denied, super::trade30::connected(self.map.layout, &pass, tile)));
                routes.len() - 1
            });
            if !routes[route].1[self.map.tile_of(actor.x, actor.y)] { continue; }
            let e = &mut self.ents[index];
            let returning = e.carry >= CARRY_LIMIT || e.carry >= 1.0 && e.q[1] == 1;
            e.order = O_GATHER;
            e.otile = tile as u32;
            e.otgt = NONE;
            e.tgt = NONE;
            e.q[0] = R_FISH as QueueItem;
            e.q[1] = returning as QueueItem;
            e.q[3] = 0;
            e.fkey = key;
            e.fslot = u16::MAX;
            e.stuck = 0;
        }
    }

    fn fishing_pass(&self, denied: crate::map::RegionMask) -> std::borrow::Cow<'_, [bool]> {
        let pass = self.map.pass[1].as_slice();
        if denied == 0 { return pass.into(); }
        pass.iter().enumerate().map(|(tile, &water)| water && self.map.permits_territory(tile, denied))
            .collect::<Vec<_>>().into()
    }

    fn fishing_port(&self, dock: usize, team: u8) -> Option<(f32, f32)> {
        let host = &self.ents[dock];
        if host.def != DOCK || !host.alive || host.hp <= 0.0 || host.team != team || host.built < 1.0 || self.sabotaged(dock) {
            return None;
        }
        self.dock_point(dock, FISHING_SHIP)
    }

    fn nearest_fishing_port(&self, e: &Ent) -> Option<(u32, (f32, f32))> {
        let candidates: Vec<_> = (0..self.high).filter_map(|i| self.fishing_port(i, e.team).map(|p| (i, p))).collect();
        if candidates.is_empty() { return None; }
        let pass = self.fishing_pass(self.border_egress_mask(e));
        let distances = water_distances(&self.map, &pass, self.map.tile_of(e.x, e.y), &candidates);
        candidates.into_iter().filter(|(_, p)| distances[self.map.tile_of(p.0, p.1)] != u32::MAX)
            .min_by_key(|(i, p)| (distances[self.map.tile_of(p.0, p.1)], *i))
            .map(|(i, p)| (self.h(i), p))
    }

    fn fishing_steer(&mut self, e: &mut Ent, d: &Def, point: (f32, f32)) -> (f32, f32) {
        let key = self.key_for(SEA, point.0, point.1);
        if e.fkey != key {
            e.fkey = key;
            e.fslot = u16::MAX;
        }
        self.steer(e, d, point.0, point.1)
    }

    fn return_fish(&mut self, e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let now = self.sim_elapsed();
        if e.otgt == NONE && now < f32::from_bits(e.q[3]) { return; }
        let destination = self.get(e.otgt).and_then(|i| self.fishing_port(i, e.team).map(|p| (e.otgt, p)))
            .or_else(|| self.nearest_fishing_port(e));
        let Some((host, point)) = destination else {
            e.otgt = NONE;
            e.q[3] = (now + 3.0).to_bits(); // 007EF6E7: retry after the original three seconds
            return;
        };
        e.otgt = host;
        if squared_distance(e, point) > DROPOFF_RANGE * DROPOFF_RANGE {
            *want = self.fishing_steer(e, d, point);
            *moving = true;
            return;
        }
        let amount = e.carry.floor();
        #[cfg(feature = "ee2-authored-rect233")]
        let amount = if self.rect_population.is_some() { e.carry } else { amount };
        let food = R_FOOD as usize - 1;
        self.add_resource(e.team, food, amount);
        self.crown_gathered(e.team, food, amount);
        e.carry = 0.0;
        e.q[1] = 0;
        e.q[2] = 0;
        e.q[3] = 0;
        e.otgt = NONE;
        #[cfg(feature = "ee2-authored-rect233")]
        if let Some(id) = fish_order(e) {
            if self.world0_fish(id).is_none_or(|site| site.remaining == 0.0) { Self::set_idle(e); }
            return;
        }
        if self.map.res[e.otile as usize] != R_FISH { Self::set_idle(e); }
    }

    pub(super) fn do_fishing(&mut self, index: usize, e: &mut Ent, d: &Def, want: &mut (f32, f32), moving: &mut bool) {
        let tile = e.otile as usize;
        if e.q[1] == 1 {
            self.return_fish(e, d, want, moving);
            return;
        }
        if self.map.res[tile] != R_FISH || !self.map.permits_territory(tile, self.denied_border_territories(e)) {
            if e.carry >= 1.0 {
                e.q[1] = 1;
            } else {
                e.carry = 0.0;
                e.q[2] = 0;
                Self::set_idle(e);
            }
            return;
        }
        let point = center(tile);
        if squared_distance(e, point) > GATHER_RANGE * GATHER_RANGE {
            *want = self.fishing_steer(e, d, point);
            *moving = true;
            return;
        }
        let occupied = self.ents[..index].iter().filter(|boat| boat.alive && boat.def == FISHING_SHIP &&
            boat.order == O_GATHER && boat.q[1] == 0 && boat.otile == e.otile &&
            squared_distance(boat, point) <= GATHER_RANGE * GATHER_RANGE).count();
        if occupied >= MAX_FISHERS { return; }

        let before = e.carry.floor();
        let food_modifier = self.modifiers[e.team as usize][FISHING_SHIP as usize][crate::economy::gather_modifier(R_FOOD)];
        let amount = self.players[e.team as usize].gather * food_modifier * self.sim_step();
        e.carry = (e.carry + amount).min(CARRY_LIMIT).min(before + self.map.amt[tile] as f32);
        e.q[2] = R_FOOD as QueueItem;
        self.map.amt[tile] -= (e.carry.floor() - before) as u16;
        if self.map.amt[tile] == 0 {
            self.map.res[tile] = R_NONE;
            self.map.recolor(tile);
        }
        if e.carry >= CARRY_LIMIT || self.map.res[tile] == R_NONE { e.q[1] = 1; }
    }

    #[cfg(feature = "ee2-authored-rect233")]
    fn world0_fishing_approach(&self, actor: &Ent, site: FishSite) -> Option<(f32, f32)> {
        let (x, y) = site.point;
        let denied = self.denied_border_territories(actor);
        let pass = self.fishing_pass(self.border_egress_mask(actor));
        let mut candidates = Vec::new();
        for ny in (y as i32 - 1)..=(y as i32 + 1) {
            for nx in (x as i32 - 1)..=(x as i32 + 1) {
                if !self.map.layout.contains(nx, ny) { continue; }
                let tile = self.map.tile_index(nx as usize, ny as usize);
                if !pass[tile] || !self.map.permits_territory(tile, denied) { continue; }
                let point = (x.clamp(nx as f32 + 0.125, nx as f32 + 0.875),
                             y.clamp(ny as f32 + 0.125, ny as f32 + 0.875));
                let distance = (point.0 - x).hypot(point.1 - y);
                if distance <= GATHER_RANGE { candidates.push((tile, point)); }
            }
        }
        if candidates.is_empty() { return None; }
        let start = self.map.tile_of(actor.x, actor.y);
        let distances = water_distances(&self.map, &pass, start, &candidates);
        candidates.into_iter().filter(|(tile, _)| distances[*tile] != u32::MAX)
            .min_by(|(a, pa), (b, pb)| {
                let distance = |point: &(f32, f32)| (point.0 - x).hypot(point.1 - y);
                distance(pa).total_cmp(&distance(pb))
                    .then_with(|| (distances[*a], *a).cmp(&(distances[*b], *b)))
            }).map(|(_, point)| point)
    }

    #[cfg(feature = "ee2-authored-rect233")]
    pub(crate) fn can_order_world0_fish(&self, id: u32, site: FishSite, index: usize) -> bool {
        let actor = &self.ents[index];
        actor.alive && actor.hp > 0.0 && actor.def == FISHING_SHIP && actor.stuck != crate::game::BOARDED
            && fish_order(actor) != Some(id) && self.world0_fishing_approach(actor, site).is_some()
    }

    #[cfg(feature = "ee2-authored-rect233")]
    pub(crate) fn order_world0_fish(&mut self, id: u32, site: FishSite, indices: &[usize]) {
        let mut reserved = self.ents[..self.high].iter().filter(|actor| fish_order(actor) == Some(id)).count();
        let tile = self.map.tile_of(site.point.0, site.point.1);
        for &index in indices {
            if reserved >= MAX_FISHERS { break; }
            let actor = &self.ents[index];
            if actor.def != FISHING_SHIP || !actor.alive || actor.hp <= 0.0 ||
                actor.stuck == crate::game::BOARDED || fish_order(actor) == Some(id) { continue; }
            let Some((x, y)) = self.world0_fishing_approach(actor, site) else { continue; };
            let key = self.key_for(SEA, x, y);
            let actor = &mut self.ents[index];
            let returning = actor.carry >= CARRY_LIMIT || actor.carry > 0.0 && actor.q[1] == 1;
            actor.order = O_GATHER;
            actor.otile = tile as u32;
            actor.ox = x;
            actor.oy = y;
            actor.otgt = NONE;
            actor.tgt = NONE;
            actor.fkey = key;
            actor.fslot = u16::MAX;
            actor.stuck = 0;
            actor.prev = 0;
            actor.q = [u32::from(R_FISH), u32::from(returning), u32::from(R_FOOD), 0, id];
            reserved += 1;
        }
    }

    #[cfg(feature = "ee2-authored-rect233")]
    fn world0_fish_open_water(&self, actor: &Ent, point: (f32, f32), denied: crate::map::RegionMask) -> bool {
        // Native 00870CC0 submits a separate grid path query. Apply the larger
        // geometry radius where the whole local span is navigable water.
        let (x, y) = (actor.x as usize, actor.y as usize);
        let (tx, ty) = (point.0 as usize, point.1 as usize);
        (y.min(ty)..=y.max(ty)).all(|row| (x.min(tx)..=x.max(tx)).all(|col| {
            let tile = self.map.tile_index(col, row);
            self.map.pass[1][tile] && self.map.permits_territory(tile, denied)
        }))
    }

    #[cfg(feature = "ee2-authored-rect233")]
    pub(crate) fn do_world0_fishing(&mut self, actor: &mut Ent, definition: &Def,
        want: &mut (f32, f32), moving: &mut bool) -> Option<f32> {
        let Some(id) = fish_order(actor) else { Self::set_idle(actor); return None; };
        let Some(site) = self.world0_fish(id) else { Self::set_idle(actor); return None; };
        let approach = (actor.ox, actor.oy);
        let approach_tile = self.map.tile_of(approach.0, approach.1);
        let denied = self.denied_border_territories(actor);
        if actor.q[1] == 1 {
            actor.prev = 0;
            self.return_fish(actor, definition, want, moving);
            return None;
        }
        if site.remaining == 0.0 || !self.map.pass[1][approach_tile]
            || !self.map.permits_territory(approach_tile, denied) {
            actor.prev = 0;
            if actor.carry > 0.0 { actor.q[1] = 1; } else { Self::set_idle(actor); }
            return None;
        }
        let (dx, dy) = (site.point.0 - actor.x, site.point.1 - actor.y);
        let distance = world0_fish_distance(dx, dy);
        if distance > f64::from(WORLD0_FISH_REACH).powi(2)
            || (distance > f64::from(GATHER_RANGE).powi(2) && !self.world0_fish_open_water(actor, site.point, denied)) {
            actor.prev = 0;
            *want = self.fishing_steer(actor, definition, approach);
            *moving = true;
            return None;
        }
        if let Some(turn) = world0_fish_turn(dx, dy, actor.rot) {
            actor.prev = 0;
            return Some(turn);
        }
        if actor.prev == 0 {
            actor.prev = 1;
            return None;
        }
        let amount = self.sim_step();
        if !self.take_world0_fish(id, amount) {
            actor.prev = 0;
            return None;
        }
        let food_modifier = self.modifiers[actor.team as usize][FISHING_SHIP as usize]
            [crate::economy::gather_modifier(R_FOOD)];
        let factor = self.players[actor.team as usize].gather;
        let cargo = f64::from(food_modifier) * f64::from(factor) * f64::from(amount) + f64::from(actor.carry);
        actor.carry = cargo.min(f64::from(CARRY_LIMIT)) as f32;
        if cargo >= f64::from(CARRY_LIMIT) {
            actor.q[1] = 1;
            actor.prev = 0;
        }
        None
    }

}

#[cfg(all(test, feature = "ee2-authored-rect233"))]
mod authored_tests {
    use super::*;
    use crate::{cmd::C_RECT_GATHER, defs::{TC, WORKER}, game::{E0, NOIDX},
        native_valley37::rectangular_world::{spatial_tests::prepare_authenticated_inputs, staged_game}};

    #[test]
    fn authored_fish_heading_is_strict_at_quarter_radian_with_near_bypass() {
        assert_eq!(world0_fish_turn(1.0, 0.0, f32::from_bits(0x3e7f_ffff)), None);
        assert_eq!(world0_fish_turn(1.0, 0.0, 0.25), Some(0.0));
        assert_eq!(world0_fish_turn(1.0, 0.0, -0.25), Some(0.0));
        assert_eq!(world0_fish_turn(0.05, 0.0, core::f32::consts::PI), None);
        assert_eq!(world0_fish_turn(f32::from_bits(0x3d4c_ccce), 0.0, 0.25), Some(0.0));
        assert_eq!(world0_fish_turn(0.0, -1.0, -core::f32::consts::FRAC_PI_2), None);
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn authored_fish_paid_ship_fractional_stock_and_delivery_survive_checkpoint() {
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 1).unwrap();
        for seat in 0..2 {
            let player = &mut game.players[seat];
            player.alive = true;
            player.ai = seat != 0;
            player.ai_next_attack = 4800;
            player.population_limit = 250;
            player.res = [1200.0; 3];
            player.recompute();
            game.rebuild_modifiers(seat as u8, false);
        }
        game.extra_resources[0] = [1200.0; 6];
        game.spawn(TC, 0, 18.0, 121.0);
        game.spawn(TC, 1, 88.45, 10.05);
        game.spawn(WORKER, 0, 16.0, 123.0);
        assert!(game.configure_fishing(false));
        assert!(!game.configure_fishing(false));
        assert!(!game.map.res.contains(&R_FISH));
        let schools: Vec<_> = game.rect_population.as_ref().unwrap().source().units().iter()
            .filter_map(|unit| game.world0_fish(unit.packed_id).map(|site| (unit.packed_id, site)))
            .collect();
        assert_eq!(schools.len(), 66);
        assert!(schools.iter().all(|(_, site)| site.remaining == 2000.0));

        let mut ready = None;
        'sites: for y in 124..134 {
            for x in 4..15 {
                for turn in [0, 2, 4, 6] {
                    if game.placement_geometry_error(0, DOCK, x, y, turn) != 0 { continue; }
                    let dock = game.place_with_rotation(DOCK, x, y, 0, turn);
                    if dock == NOIDX { continue; }
                    game.ents[dock].built = 1.0;
                    game.ents[dock].hp = game.max_hp(0, DOCK);
                    let Some((sx, sy)) = game.dock_point(dock, FISHING_SHIP) else { game.kill(dock); continue; };
                    let template = Ent { alive: true, def: FISHING_SHIP, x: sx, y: sy, hp: 50.0, ..E0 };
                    if let Some(&(id, site)) = schools.iter().filter(|(_, site)| squared_distance(&template, site.point) < 16.0)
                        .find(|(_, site)| game.world0_fishing_approach(&template, *site).is_some()) {
                        ready = Some((dock, id, site));
                        break 'sites;
                    }
                    game.kill(dock);
                }
            }
        }
        let (dock, id, site) = ready.expect("real coastal Dock placement and navigable original fish");
        game.build_grid();
        game.update_fog();
        let gold = game.resource(0, 4);
        let wood = game.resource(0, 1);
        assert!(game.train(dock, QueueItem::from(FISHING_SHIP)));
        for _ in 0..700 {
            game.tick();
            if game.ents[..game.high].iter().any(|actor| actor.alive && actor.def == FISHING_SHIP) { break; }
        }
        let ship = game.ents[..game.high].iter().position(|actor| actor.alive && actor.def == FISHING_SHIP).unwrap();
        assert_eq!(game.resource(0, 1), wood - 30.0);
        assert_eq!(game.resource(0, 4), gold - 20.0);
        game.update_fog();
        assert!(game.world0_fish_visible(0, id).is_some());
        game.save();
        if std::env::var_os("EE2_WRITE_FISH_CHECKPOINT").is_some() {
            std::fs::write("../private-fish-opening.dvs1", &game.savebuf).unwrap();
        }
        let handle = game.h(ship);
        let bank = game.resource(0, 0);
        game.apply(0, C_RECT_GATHER, 0.0, 0.0, id as i32, &[handle]);
        assert_eq!(fish_order(&game.ents[ship]), Some(id));
        assert_eq!(game.ents[ship].otile as usize, game.map.tile_of(site.point.0, site.point.1));
        for _ in 0..400 {
            game.tick();
            if game.ents[ship].carry > 0.0 { break; }
        }
        assert_eq!(game.ents[ship].carry.to_bits(), 0x3d4c_cccd);
        assert_eq!(game.world0_fish(id).unwrap().remaining.to_bits(), 0x44f9_fe66);
        let boat = game.ents[ship];
        let (dx, dy) = (site.point.0 - boat.x, site.point.1 - boat.y);
        let first_distance = world0_fish_distance(dx, dy).sqrt();
        assert!(first_distance > f64::from(GATHER_RANGE) && first_distance <= f64::from(WORLD0_FISH_REACH));
        assert_eq!(world0_fish_turn(dx, dy, boat.rot), None);
        println!("first native-range gathering tick={} distance={first_distance} facing={}", game.tick, boat.rot);
        assert!(schools.iter().filter(|(key, _)| *key != id)
            .all(|(key, _)| game.world0_fish(*key).unwrap().remaining == 2000.0));
        game.save();
        let checkpoint = game.savebuf.clone();
        assert_eq!(&checkpoint[..8], b"DVS1\x02\0\0\0");
        let mut resumed = Game::load(&checkpoint).unwrap();
        assert_eq!(resumed.state_hash(), game.state_hash());
        resumed.save();
        assert_eq!(resumed.savebuf, checkpoint);
        if std::env::var_os("EE2_WRITE_FISH_CHECKPOINT").is_some() {
            std::fs::write("../private-fish-checkpoint.dvs1", &checkpoint).unwrap();
        }
        for _ in 0..1800 {
            resumed.tick();
            if resumed.resource(0, 0) > bank { break; }
        }
        assert_eq!(resumed.resource(0, 0), bank + 30.0);
        assert_eq!(resumed.ents[ship].carry, 0.0);
        assert!(!resumed.map.res.contains(&R_FISH));
        println!("native authored fish id={id} point={:?} dock={:?} deposit tick={} stock={:08x}",
            site.point, (resumed.ents[dock].x, resumed.ents[dock].y), resumed.tick, resumed.world0_fish(id).unwrap().remaining.to_bits());
    }
}
