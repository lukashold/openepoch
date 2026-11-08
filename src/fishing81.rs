//! Installed CSV5 FishingShip and ResourceFish. Random-map ranks follow 0082D0C0.
use std::{cmp::Reverse, collections::BinaryHeap};

use crate::defs::{Def, DefId, QueueItem, A_SHIP, DOCK, NOW, SEA};
use crate::ee2_units::UnitRow;
use crate::game::{Ent, Game, NONE, O_GATHER};
use crate::map::{tile_of, Map, DIRS, N, NN, R_FISH, R_FOOD, R_NONE, T_DEEP};

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

// 0086F5B0 asks the path service for a distance before its Euclidean
// fallback. Use actual portable navigable water and the same corner rules
// as the simulation field for routes to the source-facing Dock access point.
fn water_distances(pass: &[bool], start: usize, ends: &[(usize, (f32, f32))]) -> Vec<u32> {
    let mut distances = vec![u32::MAX; NN];
    if !pass[start] { return distances; }
    let mut goals = vec![false; NN];
    let mut remaining = 0;
    for (_, (x, y)) in ends {
        let tile = tile_of(*x, *y);
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
        let (x, y) = ((tile % N) as i32, (tile / N) as i32);
        for (direction, (dx, dy)) in DIRS.into_iter().enumerate() {
            let (nx, ny) = (x + dx, y + dy);
            if !(0..N as i32).contains(&nx) || !(0..N as i32).contains(&ny) { continue; }
            let next = ny as usize * N + nx as usize;
            let diagonal = direction >= 4;
            if !pass[next] || diagonal && !(pass[y as usize * N + nx as usize] && pass[ny as usize * N + x as usize]) {
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
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() { return false; }
        let seats = if self.net { (2..=4).contains(&self.humans) && self.grouped_coop.is_none() }
            else { self.humans == 1 };
        seats && self.bench == 0 && self.map.world_dimensions() == [N, N]
    }

    pub(crate) fn configure_fishing(&mut self, network: bool) -> bool {
        if self.net != network || self.tick != 0 || self.fishing_enabled || !self.fishing_allowed() { return false; }
        place_fish(&mut self.map);
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
                routes.push((denied, super::trade30::connected(&pass, tile)));
                routes.len() - 1
            });
            if !routes[route].1[tile_of(actor.x, actor.y)] { continue; }
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

    fn fishing_pass(&self, denied: u32) -> std::borrow::Cow<'_, [bool]> {
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
        let distances = water_distances(&pass, tile_of(e.x, e.y), &candidates);
        candidates.into_iter().filter(|(_, p)| distances[tile_of(p.0, p.1)] != u32::MAX)
            .min_by_key(|(i, p)| (distances[tile_of(p.0, p.1)], *i))
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
        let food = R_FOOD as usize - 1;
        self.add_resource(e.team, food, amount);
        self.crown_gathered(e.team, food, amount);
        e.carry = 0.0;
        e.q[1] = 0;
        e.q[2] = 0;
        e.q[3] = 0;
        e.otgt = NONE;
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
}
