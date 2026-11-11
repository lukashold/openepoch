//! Solo diversion cohorts and source-shaped placement on the current map.

use super::*;
use crate::{diversion61_defs as native, power42_math::store_toward_zero as store};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const TAG: u32 = u32::from_le_bytes(*b"DIV1");
const DURATION: f64 = 45.0;
const WARNING: f64 = 4.0;
const TURN: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const ANGLE_STEP: f64 = f32::from_bits(0x3949_116d) as f64;
const WORD_SCALE: f64 = f32::from_bits(0x3800_0100) as f64;
const FORMATION_GAP: f64 = 0.25;
const ROW_EPSILON: f64 = f32::from_bits(0x3586_37bd) as f64;

struct Cohort {
    expiry: f32,
    members: BTreeSet<u32>,
}

#[derive(Default)]
pub(super) struct Diversions(BTreeMap<u8, Cohort>);

fn formation(count: usize, half_extent: [f32; 2]) -> Vec<[f32; 2]> {
    let [depth, width] = half_extent.map(|half| 2.0 * f64::from(half));
    let area = count as f64 * (depth + FORMATION_GAP) * (width + FORMATION_GAP);
    let budget = f64::from(store((area.sqrt() / depth).ceil() * depth));
    let mut slots = Vec::with_capacity(count);
    let mut depth_to_center = 0.0_f32;
    while slots.len() < count {
        let mut row = 1;
        let mut occupied = store(width);
        while row < count - slots.len()
            && f64::from(occupied) + width + row as f64 * FORMATION_GAP <= budget + ROW_EPSILON {
            occupied = store(f64::from(occupied) + width);
            row += 1;
        }
        if depth_to_center > 0.0 { depth_to_center = store(f64::from(depth_to_center) + depth * 0.5); }
        if row == 1 {
            slots.push([-depth_to_center, 0.0]);
        } else {
            let gap = f64::from(store((budget - f64::from(occupied)) / (row - 1) as f64));
            let mut left = -budget * 0.5;
            for _ in 0..row {
                let y = store(left + width * 0.5);
                slots.push([-depth_to_center, y]);
                left = f64::from(y) + width * 0.5 + gap;
            }
        }
        depth_to_center = store(f64::from(depth_to_center) + depth * 0.5 + FORMATION_GAP);
    }
    slots
}

fn rotate([x, y]: [f32; 2], angle: f64) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    let (sin, cos) = (f64::from(store(sin)), f64::from(store(cos)));
    let (x, y) = (f64::from(x), f64::from(y));
    [store(cos * x - sin * y), store(sin * x + cos * y)]
}

fn placement(rng: &mut crate::native_rng9::NativeRng, clicked: [f32; 2], slots: &[[f32; 2]],
    mut passable: impl FnMut([f32; 2]) -> bool) -> Option<(f32, Vec<[f32; 2]>)> {
    let mut points = Vec::with_capacity(slots.len());
    for attempt in 0..16 {
        let fraction = f64::from(attempt) / 16.0;
        let angle = f64::from(rng.next_word15()) * ANGLE_STEP;
        let radius = f64::from(store(f64::from(rng.next_word15()) * fraction * 3.0 * WORD_SCALE));
        let span = TURN * fraction;
        let yaw = f64::from(store(-span * 0.5 + span * f64::from(rng.next_word15()) * WORD_SCALE));
        let yaw = store(if yaw < 0.0 { yaw + TURN } else { yaw });
        let offset = rotate([1.0, 0.0], angle);
        let center: [f32; 2] = std::array::from_fn(|axis| store(f64::from(clicked[axis]) + radius * f64::from(offset[axis])));
        points.clear();
        for &slot in slots {
            let offset = rotate(slot, f64::from(yaw));
            let point = std::array::from_fn(|axis| store(f64::from(center[axis]) + f64::from(offset[axis])));
            if !passable(point) { break; }
            points.push(point);
        }
        if points.len() == slots.len() { return Some((yaw, points)); }
    }
    None
}

pub(crate) fn location(x: f32, y: f32) -> bool {
    let bounds = 0.0..N as f32;
    bounds.contains(&x) && bounds.contains(&y)
}

impl Game {
    pub(crate) fn configure_leader_diversion(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 { return false; }
        let Some(state) = self.military_leader_powers.as_mut() else { return false; };
        if state.diversions.is_some() { return false; }
        state.diversions = Some(Diversions::default());
        true
    }

    pub(crate) fn leader_diversion_enabled(&self) -> bool {
        !self.net && self.military_leader_powers.as_ref().is_some_and(|state| state.diversions.is_some())
    }

    pub(crate) fn leader_diversion_ready(&self, index: usize) -> bool {
        self.leader_diversion_enabled() && self.winner == 0 && !self.initial_ceasefire_active()
            && self.players[self.ents[index].team as usize].epoch >= 10
            && self.military_leader_reserve(index) >= 1.0
    }

    pub(crate) fn order_leader_diversion(&mut self, index: usize, x: f32, y: f32) {
        if !self.leader_diversion_ready(index) || !location(x, y) { return; }
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = O_LEADER_DIVERSION;
        actor.ox = x;
        actor.oy = y;
        actor.tgt = NONE;
        actor.otgt = NONE;
        actor.stuck = 0;
    }

    pub(super) fn do_leader_diversion(&mut self, index: usize, actor: &mut Ent) {
        let clicked = [actor.ox, actor.oy];
        Self::set_idle(actor);
        if !self.leader_diversion_ready(index) || !location(clicked[0], clicked[1]) { return; }
        let team = actor.team;
        self.spend_unit_reserve(self.h(index), 1.0);
        if self.military_leader_powers.as_ref().unwrap().diversions.as_ref().unwrap().0.contains_key(&team) { return; }
        let Some((definition, count)) = native::cohort(self.players[team as usize].epoch) else { return; };
        if self.free.len() < count { return; }
        let mut spawned = Vec::with_capacity(count);
        for _ in 0..count { spawned.push(self.spawn(definition, team, 0.0, 0.0)); }
        let slots = formation(count, native::extent(definition).unwrap());
        let chosen = placement(&mut self.source_birth.rng, clicked, &slots, |[x, y]| {
            location(x, y) && self.map.pass[0][self.map.tile_of(x, y)]
        });
        let Some((yaw, points)) = chosen else {
            for unit in spawned { self.kill(unit); }
            return;
        };
        let mut members = BTreeSet::new();
        for (unit, [x, y]) in spawned.into_iter().zip(points) {
            let member = &mut self.ents[unit];
            member.x = x;
            member.y = y;
            member.px = x;
            member.py = y;
            member.ox = x;
            member.oy = y;
            member.rot = yaw;
            member.prot = yaw;
            members.insert(self.h(unit));
        }
        let expiry = store(f64::from(self.sim_elapsed()) + DURATION);
        self.military_leader_powers.as_mut().unwrap().diversions.as_mut().unwrap().0.insert(team, Cohort { expiry, members });
    }

    pub(super) fn tick_diversions(&mut self) {
        let now = self.sim_elapsed();
        let Some(groups) = self.military_leader_powers.as_mut().and_then(|state| state.diversions.as_mut()) else { return; };
        let expired: Vec<_> = groups.0.iter().filter_map(|(&team, cohort)|
            (cohort.members.is_empty() || now >= cohort.expiry).then_some(team)).collect();
        let retired: Vec<_> = expired.into_iter().filter_map(|team| groups.0.remove(&team)).collect();
        for group in retired {
            for handle in group.members {
                if let Some(member) = self.get(handle) { self.kill(member); }
            }
        }
    }

    pub(super) fn retire_diversion_actor(&mut self, owner: u8, handle: u32) {
        if let Some(group) = self.military_leader_powers.as_mut().and_then(|state| state.diversions.as_mut())
            .and_then(|state| state.0.get_mut(&owner)) { group.members.remove(&handle); }
    }

    pub(crate) fn diversion_visual(&self, index: usize) -> u32 {
        let actor = &self.ents[index];
        if !self.visible(index) || !native::slot(actor.def).is_some_and(|slot| slot < 3) { return 0; }
        let group = self.military_leader_powers.as_ref().and_then(|state| state.diversions.as_ref())
            .and_then(|state| state.0.get(&actor.team));
        u32::from(group.is_some_and(|cohort| f64::from(self.sim_elapsed()) < f64::from(cohort.expiry) - WARNING
            && cohort.members.contains(&self.h(index))))
    }
}

impl Diversions {
    pub(super) fn fields(&self, word: &mut impl FnMut(u32)) {
        word(TAG);
        word(self.0.len() as u32);
        for (&team, cohort) in &self.0 {
            for value in [u32::from(team), cohort.expiry.to_bits(), cohort.members.len() as u32] { word(value); }
            for &handle in &cohort.members { word(handle); }
        }
    }

    pub(super) fn read(game: &Game, count: u32, mut next: impl FnMut() -> Option<u32>) -> Option<Self> {
        if count as usize > game.nplayers { return None; }
        let mut groups = Self::default();
        for _ in 0..count {
            let team = u8::try_from(next()?).ok()?;
            if team as usize >= game.nplayers || groups.0.last_key_value().is_some_and(|(&old, _)| old >= team) { return None; }
            let expiry = f32::from_bits(next()?);
            let now = game.sim_elapsed();
            let maximum = store(f64::from(now) + DURATION);
            if !(expiry > now && expiry <= maximum) { return None; }
            let members = next()? as usize;
            if members > 13 { return None; }
            let mut group = Cohort { expiry, members: BTreeSet::new() };
            let mut definition = None;
            for _ in 0..members {
                let handle = next()?;
                let member = &game.ents[game.get(handle)?];
                let slot = native::slot(member.def)?;
                if member.team != team || !member.hp.is_finite() || member.stuck == BOARDED || member.ncargo != 0
                    || game.players[team as usize].epoch < 10 + slot as u8
                    || definition.is_some_and(|old| old != member.def)
                    || group.members.last().is_some_and(|&old| old >= handle)
                    || members > native::cohort(10 + slot as u8)?.1 { return None; }
                definition = Some(member.def);
                group.members.insert(handle);
            }
            groups.0.insert(team, group);
        }
        Some(groups)
    }

    pub(super) fn contains_actor(&self, owner: u8, handle: u32) -> bool {
        self.0.get(&owner).is_some_and(|group| group.members.contains(&handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_DELETE, C_LEADER_DIVERSION, C_MOVE};

    fn fixture() -> (Game, usize, [f32; 2]) {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        assert!(game.configure_leader_diversion());
        game.players[0].epoch = 10;
        game.rebuild_modifiers(0, false);
        let leader = game.spawn(crate::leader58_defs::FIRST, 0, 100.0, 100.0);
        let handle = game.h(leader);
        game.powers.as_mut().unwrap().recharge_selected(handle);
        (game, leader, [108.0, 108.0])
    }

    #[test]
    fn diversion_moves_expires_and_restores() {
        let (mut game, leader, [x, y]) = fixture();
        game.map.pass[0].fill(true);
        let wonder = game.spawn_oriented(crate::wonders61::PARTHENON, 0, 40.5, 40.0, Game::angle_for_turn(2));
        assert_ne!(wonder, NOIDX);
        game.block(wonder, true);
        let wonder_handle = game.h(wonder);
        game.apply(0, C_LEADER_DIVERSION, x, y, 0, &[game.h(leader)]);
        game.tick();
        let members: Vec<_> = game.ents.iter().enumerate().filter_map(|(i, actor)| {
            (actor.alive && native::slot(actor.def).is_some()).then_some(game.h(i))
        }).collect();
        assert_eq!(members.len(), 9);
        let first = members[0];
        game.apply(0, C_DELETE, 0.0, 0.0, 0, &[first]);
        game.apply(0, C_MOVE, x + 7.0, y, 0, &[first]);
        for _ in 0..20 { game.tick(); }
        assert!(game.get(first).is_some());
        game.save();
        let saved = game.savebuf.clone();
        let mut loaded = Game::load(&saved).unwrap();
        assert_eq!(game.state_hash(), loaded.state_hash());
        assert_eq!(loaded.ents[loaded.get(wonder_handle).unwrap()].def, crate::wonders61::PARTHENON);
        for _ in 0..910 { loaded.tick(); }
        assert!(members.into_iter().all(|handle| loaded.get(handle).is_none()));
    }

    #[test]
    fn diversion_rejected_placement_advances_all_shared_draws() {
        let (mut game, leader, [x, y]) = fixture();
        game.map.pass[0].fill(false);
        let mut expected = game.source_birth.rng;
        for _ in 0..48 { expected.next_word15(); }
        game.apply(0, C_LEADER_DIVERSION, x, y, 0, &[game.h(leader)]);
        game.tick();
        assert_eq!(game.source_birth.rng, expected);
        assert!(game.ents.iter().all(|actor| !actor.alive || native::slot(actor.def).is_none()));
        assert_eq!(game.military_leader_reserve(leader), 0.0);
    }

    #[test]
    fn diversion_attack_emits_a_projectile_but_changes_no_target_health() {
        let (mut game, leader, [x, y]) = fixture();
        game.map.pass[0].fill(true);
        game.apply(0, C_LEADER_DIVERSION, x, y, 0, &[game.h(leader)]);
        game.tick();
        let attacker = game.ents.iter().position(|actor| actor.alive && native::slot(actor.def).is_some()).unwrap();
        let mut actor = game.ents[attacker];
        game.nplayers = 2;
        game.players[1].epoch = 10;
        game.players[1].alive = true;
        game.rebuild_modifiers(1, false);
        game.diplomacy.relation[1] = crate::diplomacy::WAR;
        game.team_vis.fill(255);
        let target = game.spawn(WORKER, 1, actor.x + 2.0, actor.y);
        let health = game.ents[target].hp;
        let weapon = &DEFS[actor.def as usize].w[0];
        game.fire(attacker, &mut actor, weapon, target);
        assert_eq!(game.projs.len(), 1);
        assert_eq!(game.projs[0].dmg, 0.0);
        game.kill(attacker);
        for _ in 0..10 { game.update_projs(); }
        assert!(game.projs.is_empty());
        assert_eq!(game.ents[target].hp, health);
        assert_eq!(game.ents[target].flash, 0.0);
    }
}
