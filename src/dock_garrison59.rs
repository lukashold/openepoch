//! Ordinary naval Dock occupancy. Trade loading uses its own source roster.

use std::collections::BTreeMap;
use crate::carrier36_defs::CARRIER;
use crate::defs::{Def, DefId, DEFS, DOCK, F_TRANSPORT};
use crate::game::{Ent, Game, MAXE, O_BOARD, O_IDLE};
use crate::map::tile_of;

const MAGIC: u32 = u32::from_le_bytes(*b"DCK1");
const VERSION: u32 = 1;
pub(crate) const SLOTS: u32 = 10;
const NAVAL_CONTACT: f32 = 1.5;

#[derive(Clone, Copy)]
struct Berth {
    host: u32,
    seats: u32,
}

#[derive(Default)]
pub(crate) struct State {
    berths: BTreeMap<u32, Berth>,
}

fn naval_family(def: DefId) -> bool {
    matches!(def, 29..=36 | 42 | 45 | 58 | 82 | 84)
}

fn seats(actor: &Ent, epoch: u8, aircraft: u32) -> u32 {
    let population = crate::economy::population(actor.def, epoch);
    let cargo = if DEFS[actor.def as usize].has(F_TRANSPORT) {
        actor.cargo[..actor.ncargo as usize].iter().map(|&definition| {
            if definition == crate::ee2_regicide::KING { 1 } else { crate::economy::population(definition, epoch) }
        }).sum()
    } else { 0 };
    population + aircraft + cargo
}

impl State {
    pub(super) fn parked(&self, index: usize, actor: &Ent) -> bool {
        naval_family(actor.def) && self.berths.contains_key(&(u32::from(actor.gen) << 16 | index as u32))
    }

    fn used(&self, host: u32) -> u32 {
        self.berths.values().filter(|berth| berth.host == host).map(|berth| berth.seats).sum()
    }

    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.berths.len() as u32);
        for (&actor, berth) in &self.berths {
            for word in [actor, berth.host, berth.seats] { write(word); }
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        write(MAGIC);
        write(VERSION);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut data = Vec::new();
        for word in [MAGIC, VERSION, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }

    pub(crate) fn read(game: &Game, data: &[u8]) -> Option<Self> {
        if game.net || game.bench != 0 { return None; }
        let mut words = data.chunks_exact(4);
        if !words.remainder().is_empty() { return None; }
        let mut next = || Some(u32::from_le_bytes(words.next()?.try_into().ok()?));
        let count = next()? as usize;
        if count > game.high { return None; }
        let mut state = Self::default();
        for _ in 0..count {
            let actor = next()?;
            let host = next()?;
            let occupied = next()?;
            let index = game.get(actor)?;
            let unit = &game.ents[index];
            let dock = &game.ents[game.get(host)?];
            let aircraft = game.dock_nested_aircraft(index)?;
            if !naval_family(unit.def) || unit.hp <= 0.0 || unit.order != O_IDLE || unit.sel
                || game.trade_hidden(index, unit) || !valid_dock(dock, unit.team)
                || occupied > SLOTS || !((0..15).any(|epoch| seats(unit, epoch, aircraft) == occupied))
                || state.used(host) + occupied > SLOTS
                || state.berths.last_key_value().is_some_and(|(&previous, _)| previous >= actor) { return None; }
            state.berths.insert(actor, Berth { host, seats: occupied });
        }
        (words.len() == 0).then_some(state)
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |offset: usize| Some(u32::from_le_bytes(data.get(offset..offset + 4)?.try_into().ok()?));
    let end = data.len().checked_sub(4)?;
    let len = word(8)? as usize;
    let start = end.checked_sub(len)?;
    if word(4)? != VERSION || len == 0 || !(16..=16 + MAXE * 12).contains(&start)
        || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}

fn valid_dock(dock: &Ent, owner: u8) -> bool {
    dock.def == DOCK && dock.built >= 1.0 && dock.hp > 0.0 && dock.team == owner
}

impl Game {
    pub(crate) fn configure_dock_garrison(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 || self.powers.is_none() || self.dock_garrison.is_some() { return false; }
        self.dock_garrison = Some(State::default());
        true
    }

    pub(crate) fn can_dock_naval(&self, definition: DefId) -> bool {
        self.dock_garrison.is_some() && naval_family(definition)
    }

    pub(crate) fn dock_parked(&self, index: usize, actor: &Ent) -> bool {
        self.dock_garrison.as_ref().is_some_and(|state| state.parked(index, actor))
    }

    pub(crate) fn dock_garrison_count(&self, index: usize) -> usize {
        let handle = self.h(index);
        self.dock_garrison.as_ref().map_or(0, |state| state.berths.values().filter(|slot| slot.host == handle).count())
    }

    pub(crate) fn dock_garrison_used(&self, index: usize) -> u32 {
        self.dock_garrison.as_ref().map_or(0, |state| state.used(self.h(index)))
    }

    fn dock_point(&self, index: usize, definition: DefId) -> Option<(f32, f32)> {
        let mut dock = self.ents[index];
        dock.rx = -1.0;
        let point = self.spawn_point(&dock, &DEFS[DOCK as usize], &DEFS[definition as usize]);
        self.map.pass[1][tile_of(point.0, point.1)].then_some(point)
    }

    fn dock_nested_aircraft(&self, index: usize) -> Option<u32> {
        if self.ents[index].def == CARRIER { self.air_cargo_for_dock(self.h(index)) } else { Some(0) }
    }

    pub(super) fn order_dock_board(&mut self, ids: &[usize], dock: usize) {
        let host = self.ents[dock];
        let handle = self.h(dock);
        if !valid_dock(&host, host.team) || self.sabotaged(dock) { return; }
        let Some((x, y)) = self.dock_point(dock, 29) else { return; };
        let connected = super::trade30::connected(&self.map.pass[1], tile_of(x, y));
        for &index in ids {
            let actor = self.ents[index];
            if actor.team != host.team || actor.hp <= 0.0 || !self.can_dock_naval(actor.def)
                || self.trade_hidden(index, &actor) || !connected[tile_of(actor.x, actor.y)] { continue; }
            let Some(aircraft) = self.dock_nested_aircraft(index) else { continue; };
            let cost = seats(&actor, self.players[actor.team as usize].epoch, aircraft);
            if self.dock_garrison.as_ref().unwrap().used(handle) + cost > SLOTS { continue; }
            self.trade_interrupt(index);
            self.native_stop_attack(index);
            let key = self.key_for(DEFS[actor.def as usize].dom, x, y);
            let actor = &mut self.ents[index];
            actor.order = O_BOARD;
            actor.otgt = handle;
            actor.ox = x;
            actor.oy = y;
            actor.fkey = key;
            actor.tgt = super::NONE;
            actor.stuck = 0;
        }
    }

    pub(super) fn do_dock_board(&mut self, index: usize, actor: &mut Ent, definition: &Def,
        dock: usize, want: &mut (f32, f32), moving: &mut bool) {
        if !self.can_dock_naval(actor.def) || !valid_dock(&self.ents[dock], actor.team) || self.sabotaged(dock) {
            Self::set_idle(actor);
            return;
        }
        let Some((x, y)) = self.dock_point(dock, actor.def) else { Self::set_idle(actor); return; };
        if (actor.x - x).powi(2) + (actor.y - y).powi(2) > NAVAL_CONTACT * NAVAL_CONTACT {
            *want = self.steer(actor, definition, x, y);
            *moving = true;
            return;
        }
        let handle = self.h(index);
        let host = self.h(dock);
        let Some(aircraft) = self.dock_nested_aircraft(index) else { Self::set_idle(actor); return; };
        let cost = seats(actor, self.players[actor.team as usize].epoch, aircraft);
        let state = self.dock_garrison.as_mut().unwrap();
        if state.used(host) + cost <= SLOTS {
            state.berths.insert(handle, Berth { host, seats: cost });
            actor.sel = false;
            actor.vx = 0.0;
            actor.vy = 0.0;
        }
        Self::set_idle(actor);
        actor.otgt = super::NONE;
    }

    pub(super) fn unload_dock(&mut self, index: usize, forced: bool) {
        let Some(state) = &self.dock_garrison else { return; };
        if !forced && self.sabotaged(index) { return; }
        let host = self.h(index);
        let occupants: Vec<_> = state.berths.iter().filter_map(|(&actor, berth)| (berth.host == host).then_some(actor)).collect();
        for handle in occupants {
            self.release_dock_vessel(handle, index, forced);
        }
    }

    fn release_dock_vessel(&mut self, handle: u32, host: usize, forced: bool) {
        let Some(actor) = self.get(handle) else {
            self.dock_garrison.as_mut().unwrap().berths.remove(&handle);
            return;
        };
        let point = self.dock_point(host, self.ents[actor].def);
        if point.is_none() && !forced { return; }
        self.dock_garrison.as_mut().unwrap().berths.remove(&handle);
        let vessel = &mut self.ents[actor];
        if let Some((x, y)) = point {
            vessel.x = x;
            vessel.y = y;
            vessel.px = x;
            vessel.py = y;
            Self::set_idle(vessel);
        } else {
            vessel.hp = 0.0;
        }
    }

    pub(crate) fn reconcile_dock_owners(&mut self) {
        let Some(state) = &self.dock_garrison else { return; };
        let changed: Vec<_> = state.berths.iter().filter_map(|(&actor, berth)| {
            let unit = self.get(actor)?;
            let dock = self.get(berth.host)?;
            (self.ents[unit].team != self.ents[dock].team).then_some((actor, dock))
        }).collect();
        for (actor, dock) in changed { self.release_dock_vessel(actor, dock, true); }
    }

    pub(crate) fn retire_dock_garrison(&mut self, index: usize) {
        if self.dock_garrison.is_none() { return; }
        if self.ents[index].def == DOCK { self.unload_dock(index, true); }
        let handle = self.h(index);
        self.dock_garrison.as_mut().unwrap().berths.remove(&handle);
    }

    pub(super) fn tick_dock_garrison(&mut self) {
        let Some(state) = &self.dock_garrison else { return; };
        let mut repair = Vec::new();
        for (&actor, berth) in &state.berths {
            let Some(index) = self.get(actor) else { continue; };
            let Some(host) = self.get(berth.host) else { continue; };
            let dock = self.ents[host];
            let vessel = self.ents[index];
            if !valid_dock(&dock, vessel.team) || vessel.hp <= 0.0 || self.sabotaged(host) { continue; }
            let Some(serial) = self.powers.as_ref().and_then(|powers| powers.actor(berth.host)) else { continue; };
            let id = (u32::from(dock.team) + 1) << 27 | serial.serial;
            if crate::capture_math25::capture_event_due(self.tick, id, self.sim_step()) {
                let maximum = self.max_hp(vessel.team, vessel.def);
                let amount = 5.0 * self.crown_healing_building_scale(dock.team, DOCK);
                repair.push((index, (vessel.hp + amount).min(maximum)));
            }
        }
        for (index, hp) in repair { self.ents[index].hp = hp; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_BOARD, C_MOVE, C_UNLOAD};
    use crate::map::{NN, T_SHALLOW};

    fn setup() -> (Game, usize) {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_dock_garrison());
        game.map.ter.fill(T_SHALLOW);
        for index in 0..NN { game.map.upd_pass(index); }
        let dock = game.spawn(DOCK, 0, 75.5, 118.5);
        for slot in 0..9 { game.add_resource(0, slot, 1000.0); }
        game.build_grid();
        game.update_fog();
        (game, dock)
    }

    fn until(game: &mut Game, limit: u32, expect: impl Fn(&Game) -> bool) {
        for _ in 0..limit {
            if expect(game) { return; }
            game.tick();
        }
        assert!(expect(game), "condition failed at tick {}", game.tick);
    }

    fn board(game: &mut Game, dock: usize, occupants: &[usize]) {
        let handle = game.h(dock);
        let actors: Vec<_> = occupants.iter().map(|&actor| game.h(actor)).collect();
        game.apply(0, C_BOARD, 0.0, 0.0, handle as i32, &actors);
    }

    #[test]
    fn paid_warship_boards_heals_and_unloads_the_same_actor_after_archive() {
        let (mut game, dock) = setup();
        let hull = 29;
        let stock = |game: &Game| std::array::from_fn::<_, 9, _>(|i| game.resource(0, i));
        let before = stock(&game);
        let costs = game.unit_cost(0, hull);
        assert!(game.train(dock, hull as _));
        assert_eq!(stock(&game), std::array::from_fn(|slot| before[slot] - f32::from(costs[slot])));
        let vessel = |g: &Game| g.ents[..g.high].iter().position(|actor| actor.alive && actor.def == hull && actor.team == 0);
        until(&mut game, 1800, |game| vessel(game).is_some());
        let ship = vessel(&game).unwrap();
        let handle = game.h(ship);
        let point = game.dock_point(dock, hull).unwrap();
        game.ents[ship].x = point.0 - 5.0;
        game.ents[ship].hp = 30.0;
        board(&mut game, dock, &[ship]);
        assert_eq!(game.ents[ship].order, O_BOARD);
        until(&mut game, 900, |g| g.dock_parked(ship, &g.ents[ship]));
        assert!(!game.visible(ship));
        assert!(!game.can_see(0, ship));
        assert_eq!(game.dock_garrison_used(dock), crate::economy::population(hull, 0));
        let hp = game.ents[ship].hp;
        game.apply(0, C_MOVE, 85.0, 120.0, 0, &[handle]);
        assert_eq!(game.ents[ship].order, O_IDLE);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        for _ in 0..20 {
            restored.tick();
            game.tick();
            assert_eq!(restored.state_hash(), game.state_hash());
        }
        assert_eq!(game.ents[ship].hp, hp + 5.0);
        restored.apply(0, C_UNLOAD, 0.0, 0.0, 0, &[game.h(dock)]);
        assert_eq!(restored.get(handle), Some(ship));
        assert!(restored.visible(ship));
        assert_eq!(restored.ents[ship].hp, hp + 5.0);
        assert_eq!(restored.dock_garrison_count(dock), 0);
        assert!(restored.map.pass[1][tile_of(restored.ents[ship].x, restored.ents[ship].y)]);
    }

    #[test]
    fn population_capacity_and_invalid_or_sabotaged_commands_are_consistent() {
        let (mut game, dock) = setup();
        let hull = 29;
        let point = game.dock_point(dock, hull).unwrap();
        let own: Vec<_> = (0..6).map(|_| game.spawn(hull, 0, point.0, point.1)).collect();
        let citizen = game.spawn(8, 0, point.0, point.1);
        let fake = game.h(own[0]) ^ 0x0010_0000;
        let host = game.h(dock);
        game.apply(0, C_BOARD, 0.0, 0.0, host as i32, &[fake, game.h(citizen)]);
        assert_eq!(game.ents[citizen].order, O_IDLE);
        assert_eq!(game.ents[own[0]].order, O_IDLE);
        game.sabotage_building(0, dock, game.sim_elapsed());
        board(&mut game, dock, &own);
        assert!(own.iter().all(|&i| game.ents[i].order == O_IDLE));
        game.restore_building(dock);
        board(&mut game, dock, &own);
        for _ in 0..10 { game.tick(); }
        let cost = crate::economy::population(hull, 0);
        assert_eq!(game.dock_garrison_count(dock), (SLOTS / cost) as usize);
        assert_eq!(game.dock_garrison_used(dock), SLOTS / cost * cost);
        let active: Vec<_> = own.into_iter().filter(|&i| game.dock_parked(i, &game.ents[i])).collect();
        let hp = 20.0;
        for &i in &active { game.ents[i].hp = hp; }
        game.sabotage_building(0, dock, game.sim_elapsed());
        game.apply(0, C_UNLOAD, 0.0, 0.0, 0, &[host]);
        for _ in 0..20 { game.tick(); }
        assert!(active.iter().all(|&i| game.dock_parked(i, &game.ents[i]) && game.ents[i].hp == hp));

        let bytes = game.dock_garrison.as_ref().unwrap().wrap(vec![1]);
        let (_, payload) = split(&bytes).unwrap();
        let mut corrupt = payload.unwrap().to_vec();
        corrupt[12..16].copy_from_slice(&11u32.to_le_bytes());
        assert!(State::read(&game, &corrupt).is_none());
        game.restore_building(dock);
        game.apply(0, crate::cmd::C_DELETE, 0.0, 0.0, 0, &[host]);
        game.save();
        assert!(Game::load(&game.savebuf).is_some());
    }

    #[test]
    fn parked_carrier_preserves_nested_plane_and_seat_cost() {
        let (mut game, dock) = setup();
        game.players[0].epoch = 10;
        game.rebuild_modifiers(0, false);
        let (x, y) = game.dock_point(dock, CARRIER).unwrap();
        let carrier = game.spawn(CARRIER, 0, x, y);
        let plane = game.spawn(37, 0, x, y);
        game.air_order_land(&[plane], carrier);
        until(&mut game, 2500, |g| g.air_parked(plane, &g.ents[plane]));
        let carrier_handle = game.h(carrier);
        let plane_handle = game.h(plane);
        board(&mut game, dock, &[carrier]);
        until(&mut game, 20, |g| g.dock_parked(carrier, &g.ents[carrier]));
        assert_eq!(game.dock_garrison_used(dock), 6);
        assert!(game.air_parked(plane, &game.ents[plane]));
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        for _ in 0..20 { restored.tick(); }
        assert_eq!(restored.get(carrier_handle), Some(carrier));
        assert_eq!(restored.get(plane_handle), Some(plane));
        assert!(restored.air_parked(plane, &restored.ents[plane]));
        restored.apply(0, C_UNLOAD, 0.0, 0.0, 0, &[game.h(dock)]);
        assert!(restored.visible(carrier));
        assert_eq!(restored.air_mission_status(carrier_handle, 1), 1);
    }

    #[test]
    fn trade_loader_stays_outside_ordinary_repair_roster() {
        let (mut game, dock) = setup();
        let other = game.spawn(DOCK, 0, 92.5, 118.5);
        let ship = game.spawn(82, 0, 82.5, 120.5);
        let handle = game.h(ship);
        let home = game.h(dock);
        game.ents[ship].hp = 10.0;
        game.apply(0, crate::cmd::C_TRADE_ROUTE, (home & 0xffff) as f32, (home >> 16) as f32,
            game.h(other) as i32, &[handle]);
        until(&mut game, 600, |g| g.trade_hidden(ship, &g.ents[ship]));
        assert!(!game.dock_parked(ship, &game.ents[ship]));
        assert_eq!(game.dock_garrison_count(other), 0);
        for _ in 0..20 { game.tick(); }
        assert!(game.trade_hidden(ship, &game.ents[ship]));
        assert_eq!(game.ents[ship].hp, 10.0);
    }
}
