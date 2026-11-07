//! Military leader reserve and scheduled, source-centered battle effects.

use super::*;
use crate::leader58_defs::FIRST as MILITARY;
use crate::power42_math::store_toward_zero;
use std::collections::{BTreeMap, BTreeSet};

const MAGIC: u32 = u32::from_le_bytes(*b"LMP1");
const VERSION: u32 = 1;
const RALLY_RADIUS: f32 = 3.0;
const MANEUVER_TAG: u32 = u32::from_le_bytes(*b"FAM1");
const MANEUVER_RADIUS: f32 = 5.0;
const MANEUVER_DURATION: f32 = 45.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuraKind {
    Fervor = 1,
    Morale = 2,
    Discipline = 3,
}

impl AuraKind {
    fn for_epoch(epoch: u8) -> Self {
        match epoch { 0..=4 => Self::Fervor, 5..=9 => Self::Morale, _ => Self::Discipline }
    }

    fn radius(self) -> f32 { if self == Self::Discipline { 5.0 } else { 3.0 } }

    fn includes(self, def: DefId, epoch: u8) -> bool {
        land_rps(def, epoch) || self == Self::Fervor && def == FORTRESS
    }
}

struct Aura {
    kind: AuraKind,
    recipients: BTreeSet<u32>,
}

struct Maneuver {
    source: u32,
    started: f32,
    allies: BTreeSet<u32>,
    enemies: BTreeSet<u32>,
}

impl Maneuver {
    fn deadline(&self) -> f32 {
        store_toward_zero(f64::from(self.started) + f64::from(MANEUVER_DURATION))
    }
}

#[derive(Default)]
pub(crate) struct State {
    auras: BTreeMap<u32, Aura>,
    maneuvers: Vec<Maneuver>,
}

fn land_rps(def: DefId, epoch: u8) -> bool {
    crate::leader58_defs::branch(def).is_some() || crate::ee2_combat::land_effect_target(def, epoch)
}

pub(super) fn in_radius(center: &Ent, actor: &Ent, radius: f32) -> bool {
    let dx = f64::from(actor.x) - f64::from(center.x);
    let dy = f64::from(actor.y) - f64::from(center.y);
    let rectangle = match actor.def {
        FORTRESS => Some([1.5, 1.5]),
        MARKET => Some([1.0, 1.0]),
        DOCK => Some([1.0, 1.5]),
        _ => None,
    };
    if let Some(halves) = rectangle {
        // The live bound uses the impassable size. Native 006D9230 tests both box axes independently.
        let extent = halves.map(|half| f64::from(store_toward_zero(half - f64::from(0.01_f32))) + f64::from(radius));
        let (sin, cos) = actor.rot.sin_cos();
        let (sin, cos) = (f64::from(sin), f64::from(cos));
        return (dx * cos + dy * sin).abs() <= extent[0] && (-dx * sin + dy * cos).abs() <= extent[1];
    }
    // Source CFG5 UnitType inheritance binds ordinary live mobile actors to vertical bounds.
    let target_radius = match actor.def {
        8..=17 | 85..=86 => 0.125_f32,
        61 | 80 | 87..=89 => 0.225,
        18..=19 | 21 | 57 | 63..=64 | 68..=71 | 73 | 75 | 79 | 81 => 0.4,
        22 | 27 | 65 | 77..=78 => 0.5,
        25 | 28 => 0.55,
        67 => 0.6,
        23 | 76 => 0.7,
        43 | 74 => 0.75,
        26 | 46 | 62 | 66 | 72 => 0.9,
        20 | 24 => 1.0,
        _ => return false,
    };
    let combined = f64::from(radius) + f64::from(target_radius);
    let square = store_toward_zero(combined * combined);
    dx * dx + dy * dy < f64::from(square)
}

impl Game {
    pub(crate) fn configure_military_leader_powers(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 || !self.crown_leaders || self.powers.is_none()
            || self.military_leader_powers.is_some() { return false; }
        self.military_leader_powers = Some(State::default());
        true
    }

    fn military_leader_on_map(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        actor.alive && actor.hp > 0.0 && actor.def == MILITARY && actor.stuck != BOARDED
    }

    pub(crate) fn rally_ready(&self, index: usize) -> bool {
        self.military_leader_powers.is_some() && !self.net && self.winner == 0
            && self.military_leader_on_map(index) && self.players[self.ents[index].team as usize].epoch < 5
            && self.military_leader_reserve(index) >= 1.0
    }

    pub(crate) fn fire_maneuver_ready(&self, index: usize) -> bool {
        self.military_leader_powers.is_some() && !self.net && self.winner == 0 && !self.initial_ceasefire_active()
            && (5..10).contains(&self.players[self.ents[index].team as usize].epoch)
            && self.military_leader_reserve(index) >= 1.0
    }

    pub(crate) fn military_leader_reserve(&self, index: usize) -> f32 {
        if self.ents[index].def != MILITARY { return -1.0; }
        self.leader_power_reserve(index)
    }

    pub(crate) fn order_leader_power(&mut self, index: usize, maneuver: bool) {
        let ready = if maneuver { self.fire_maneuver_ready(index) } else { self.rally_ready(index) };
        if !ready { return; }
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = if maneuver { O_LEADER_FIRE_MANEUVER } else { O_LEADER_RALLY };
        actor.otgt = NONE;
        actor.tgt = NONE;
        actor.stuck = 0;
    }

    pub(super) fn do_fire_maneuver(&mut self, index: usize, caster: &mut Ent) {
        Self::set_idle(caster);
        if !self.fire_maneuver_ready(index) { return; }
        let source = self.h(index);
        let started = self.sim_elapsed();
        self.spend_unit_reserve(source, 1.0);
        self.military_leader_powers.as_mut().unwrap().maneuvers.push(Maneuver {
            source, started, allies: BTreeSet::new(), enemies: BTreeSet::new(),
        });
    }

    pub(super) fn do_leader_rally(&mut self, index: usize, caster: &mut Ent) {
        Self::set_idle(caster);
        if !self.rally_ready(index) { return; }
        self.spend_unit_reserve(self.h(index), 1.0);
        let epoch = self.players[caster.team as usize].epoch;
        for target in 0..self.high {
            let actor = &self.ents[target];
            if actor.team != caster.team || !land_rps(actor.def, epoch) || !self.leader_effect_actor(target)
                || !in_radius(caster, actor, RALLY_RADIUS) { continue; }
            let maximum = self.max_hp(actor.team, actor.def);
            if actor.hp >= maximum { continue; }
            let healed = store_toward_zero(f64::from(actor.hp) + f64::from(maximum) * 0.5).min(maximum);
            self.attach_priest_effect(self.h(target), crate::power42::FX_RALLY_HEALED);
            if target == index { caster.hp = healed; } else { self.ents[target].hp = healed; }
        }
    }

    fn leader_effect_actor(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        actor.alive && actor.hp > 0.0 && actor.stuck != BOARDED
            && !self.trade_hidden(index, actor) && !self.air_parked(index, actor)
    }

    fn leader_aura_kind(&self, index: usize) -> Option<AuraKind> {
        if !self.military_leader_on_map(index) { return None; }
        Some(AuraKind::for_epoch(self.players[self.ents[index].team as usize].epoch))
    }

    pub(super) fn leader_area_event_due(&self, index: usize) -> bool {
        let handle = self.h(index);
        let Some(record) = self.powers.as_ref().and_then(|powers| powers.actor(handle)) else { return false; };
        let native_id = (u32::from(self.ents[index].team) + 1) << 27 | record.serial;
        crate::capture_math25::capture_event_due(self.tick, native_id, self.sim_step())
    }

    pub(super) fn tick_military_leader_auras(&mut self) {
        let Some(mut state) = self.military_leader_powers.take() else { return; };
        state.auras.retain(|&handle, aura| {
            let Some(index) = self.get(handle) else { return false; };
            if self.leader_aura_kind(index) != Some(aura.kind) { return false; }
            let owner = self.ents[index].team;
            aura.recipients.retain(|&target| self.get(target).is_some_and(|target| {
                self.ents[target].team == owner && self.leader_effect_actor(target)
            }));
            true
        });
        for index in 0..self.high {
            let Some(kind) = self.leader_aura_kind(index) else { continue; };
            let handle = self.h(index);
            if !self.leader_area_event_due(index) { continue; }
            let center = &self.ents[index];
            let epoch = self.players[center.team as usize].epoch;
            let recipients = (0..self.high).filter_map(|target| {
                let actor = &self.ents[target];
                (actor.team == center.team && self.leader_effect_actor(target) && kind.includes(actor.def, epoch)
                    && in_radius(center, actor, kind.radius())).then(|| self.h(target))
            }).collect();
            state.auras.insert(handle, Aura { kind, recipients });
        }
        let now = self.sim_elapsed();
        state.maneuvers.retain_mut(|group| {
            if now > group.deadline() { return false; }
            let Some(index) = self.get(group.source) else { return false; };
            if !self.military_leader_on_map(index) { return false; }
            if !self.leader_area_event_due(index) { return true; }
            let center = &self.ents[index];
            group.allies.clear();
            group.enemies.clear();
            for target in 0..self.high {
                let actor = &self.ents[target];
                if !self.leader_effect_actor(target) || !land_rps(actor.def, self.players[actor.team as usize].epoch)
                    || !in_radius(center, actor, MANEUVER_RADIUS) { continue; }
                if actor.team == center.team { group.allies.insert(self.h(target)); }
                else if self.hostile(center.team, actor.team) { group.enemies.insert(self.h(target)); }
            }
            true
        });
        self.military_leader_powers = Some(state);
    }

    pub(super) fn retire_military_leader_effect_actor(&mut self, handle: u32) {
        let Some(state) = &mut self.military_leader_powers else { return; };
        state.auras.remove(&handle);
        for aura in state.auras.values_mut() { aura.recipients.remove(&handle); }
        state.maneuvers.retain_mut(|group| {
            group.allies.remove(&handle);
            group.enemies.remove(&handle);
            group.source != handle
        });
    }

    fn leader_maneuvers(&self, index: usize) -> impl Iterator<Item = &Maneuver> {
        self.military_leader_powers.iter().flat_map(|state| state.maneuvers.iter()).filter(move |group| {
            self.leader_effect_actor(index) && self.get(group.source).is_some_and(|source| self.military_leader_on_map(source))
        })
    }

    pub(super) fn leader_movement_scale(&self, index: usize) -> f32 {
        let handle = self.h(index);
        self.leader_maneuvers(index).filter(|group| group.allies.contains(&handle))
            .fold(1.0, |scale, _| store_toward_zero(f64::from(scale) + 0.5))
    }

    pub(super) fn leader_immobilized(&self, index: usize) -> bool {
        let handle = self.h(index);
        self.leader_maneuvers(index).any(|group| group.enemies.contains(&handle))
    }

    pub(crate) fn leader_maneuver_status(&self, index: usize) -> u32 {
        if !self.visible(index) { return 0; }
        if self.leader_immobilized(index) { return 2; }
        u32::from(self.ents[index].team == self.local && self.leader_movement_scale(index) > 1.0)
    }

    fn military_auras_for(&self, actor: usize) -> impl Iterator<Item = AuraKind> + '_ {
        let target = self.h(actor);
        self.military_leader_powers.iter().flat_map(|state| state.auras.iter()).filter_map(move |(&handle, aura)| {
            if !aura.recipients.contains(&target) || !self.leader_effect_actor(actor) { return None; }
            let source = self.get(handle)?;
            (self.ents[source].team == self.ents[actor].team && self.leader_aura_kind(source) == Some(aura.kind)).then_some(aura.kind)
        })
    }

    pub(super) fn leader_attack_scale(&self, index: usize) -> f32 {
        let mut ordinary = 1.0_f32;
        for kind in self.military_auras_for(index) {
            if kind == AuraKind::Fervor { ordinary = store_toward_zero(f64::from(ordinary) + (f64::from(1.2_f32) - 1.0)); }
        }
        let art_of_war = self.art_of_war_bonus(&self.ents[index]);
        store_toward_zero(f64::from(ordinary) + f64::from(self.bless_attack_bonus(self.h(index))) + f64::from(art_of_war))
    }

    pub(super) fn leader_damage_scale(&self, index: usize, area: bool) -> f32 {
        let mut factor = 1.0_f32;
        for kind in self.military_auras_for(index) {
            let scale = match kind {
                AuraKind::Morale => 0.8_f32,
                AuraKind::Discipline if area => 0.5,
                _ => continue,
            };
            factor = store_toward_zero(f64::from(factor) + (f64::from(scale) - 1.0));
        }
        let art_of_war = self.art_of_war_bonus(&self.ents[index]);
        store_toward_zero(f64::from(factor) - f64::from(art_of_war)).max(0.0)
    }

    pub(crate) fn military_leader_status(&self, index: usize) -> u32 {
        if !self.visible(index) { return 0; }
        self.military_auras_for(index).find_map(|kind| {
            (kind == AuraKind::Fervor || self.ents[index].team == self.local).then_some(kind as u32)
        }).unwrap_or(0)
    }
}

impl State {
    fn fields(&self, mut word: impl FnMut(u32)) {
        word(self.auras.len() as u32);
        for (&source, aura) in &self.auras {
            for value in [source, aura.kind as u32, aura.recipients.len() as u32] { word(value); }
            for &target in &aura.recipients { word(target); }
        }
        if self.maneuvers.is_empty() { return; }
        word(MANEUVER_TAG);
        word(self.maneuvers.len() as u32);
        for group in &self.maneuvers {
            for value in [group.source, group.started.to_bits(), group.allies.len() as u32, group.enemies.len() as u32] { word(value); }
            for &handle in group.allies.iter().chain(&group.enemies) { word(handle); }
        }
    }

    pub(crate) fn hash(&self, mut word: impl FnMut(u32)) {
        word(MAGIC);
        word(VERSION);
        self.fields(word);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut bytes = Vec::new();
        for value in [MAGIC, VERSION, core.len() as u32] { bytes.extend_from_slice(&value.to_le_bytes()); }
        self.fields(|value| bytes.extend_from_slice(&value.to_le_bytes()));
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }

    pub(crate) fn read(game: &Game, payload: &[u8]) -> Option<Self> {
        if game.net || game.bench != 0 || !game.crown_leaders || game.powers.is_none() || game.crowns.is_none() { return None; }
        let mut words = payload.chunks_exact(4);
        if !words.remainder().is_empty() { return None; }
        let mut next = || Some(u32::from_le_bytes(words.next()?.try_into().ok()?));
        let count = next()? as usize;
        if count > game.high { return None; }
        let mut result = Self::default();
        for _ in 0..count {
            let handle = next()?;
            let source = game.get(handle)?;
            if game.ents[source].def != MILITARY { return None; }
            let kind = match next()? { 1 => AuraKind::Fervor, 2 => AuraKind::Morale, 3 => AuraKind::Discipline, _ => return None };
            let current = AuraKind::for_epoch(game.players[game.ents[source].team as usize].epoch);
            if kind as u32 > current as u32 || result.auras.last_key_value().is_some_and(|(&old, _)| old >= handle) { return None; }
            let owner = game.ents[source].team;
            let count = next()? as usize;
            if count > game.high { return None; }
            let mut recipients = BTreeSet::new();
            for _ in 0..count {
                let handle = next()?;
                let index = game.get(handle)?;
                let actor = &game.ents[index];
                if actor.team != owner || kind == current && !kind.includes(actor.def, game.players[owner as usize].epoch)
                    || recipients.last().is_some_and(|&old| old >= handle) { return None; }
                recipients.insert(handle);
            }
            result.auras.insert(handle, Aura { kind, recipients });
        }
        if let Some(tag) = next() {
            let count = next()? as usize;
            if tag != MANEUVER_TAG || !(1..=MAXE).contains(&count) { return None; }
            for _ in 0..count {
                let source = next()?;
                let actor = &game.ents[game.get(source)?];
                if actor.def != MILITARY || game.players[actor.team as usize].epoch < 5 { return None; }
                let started = f32::from_bits(next()?);
                let now = game.sim_elapsed();
                if !(0.0..=now).contains(&started) || result.maneuvers.last().is_some_and(|last| started < last.started) { return None; }
                let ally_count = next()? as usize;
                let enemy_count = next()? as usize;
                let mut group = Maneuver { source, started, allies: BTreeSet::new(), enemies: BTreeSet::new() };
                if now > group.deadline() || ally_count > game.high || enemy_count > game.high - ally_count { return None; }
                for (members, count, same_owner) in [(&mut group.allies, ally_count, true), (&mut group.enemies, enemy_count, false)] {
                    for _ in 0..count {
                        let handle = next()?;
                        let member = &game.ents[game.get(handle)?];
                        if (member.team == actor.team) != same_owner || members.last().is_some_and(|&last| handle <= last) { return None; }
                        members.insert(handle);
                    }
                }
                result.maneuvers.push(group);
            }
        }
        (words.len() == 0).then_some(result)
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !bytes.starts_with(&MAGIC.to_le_bytes()) { return Some((bytes, None)); }
    let word = |offset: usize| Some(u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?));
    let end = bytes.len().checked_sub(4)?;
    let start = end.checked_sub(word(8)? as usize)?;
    if word(4)? != VERSION || start < 16 || start == end || word(end)? != crate::save::checksum(&bytes[..end]) { return None; }
    Some((&bytes[start..end], Some(&bytes[12..start])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_LEADER_RALLY, C_STOP};

    fn earned() -> (Game, usize) {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(!game.configure_military_leader_powers());
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        assert!(!game.configure_military_leader_powers());
        game.researched[0][0] |= 0b1111;
        game.tick = 20;
        game.tick_crowns();
        let leader = game.ents[..game.high].iter().position(|e| e.alive && e.team == 0 && e.def == MILITARY).unwrap();
        game.ents[leader].x = 100.0;
        game.ents[leader].y = 100.0;
        (game, leader)
    }

    fn next_phase(game: &mut Game, leader: usize) {
        let serial = game.powers.as_ref().unwrap().actor(game.h(leader)).unwrap().serial;
        let id = (u32::from(game.ents[leader].team) + 1) << 27 | serial;
        while !crate::capture_math25::capture_event_due(game.tick, id, game.sim_step()) { game.tick += 1; }
    }

    #[test]
    fn earned_rally_recharges_and_heals_only_matching_owned_living_targets() {
        let (mut game, leader) = earned();
        let handle = game.h(leader);
        let infantry = game.spawn(9, 0, 103.0, 100.0);
        let outside = game.spawn(9, 0, 103.125, 100.0);
        let other = game.spawn(9, 1, 101.0, 100.0);
        let fortress = game.spawn(FORTRESS, 0, 100.5, 100.0);
        let dead = game.spawn(9, 0, 101.0, 100.5);
        for target in [leader, infantry, outside, other, fortress] { game.ents[target].hp = 10.0; }
        game.ents[dead].hp = 0.0;
        assert!(!game.rally_ready(leader));
        for _ in 0..60 {
            game.tick += 1;
            next_phase(&mut game, leader);
            game.tick_unit_reserve(leader);
        }
        // Original recharge uses rounded-down f32 stores, so a final pulse may be needed.
        if !game.rally_ready(leader) {
            game.tick += 1;
            next_phase(&mut game, leader);
            game.tick_unit_reserve(leader);
        }
        assert!(game.rally_ready(leader));
        game.apply(1, C_LEADER_RALLY, 0.0, 0.0, 0, &[handle]);
        game.apply(0, C_LEADER_RALLY, 1.0, 0.0, 0, &[handle]);
        assert_ne!(game.ents[leader].order, O_LEADER_RALLY);
        game.apply(0, C_LEADER_RALLY, 0.0, 0.0, 0, &[handle]);
        assert_eq!(game.ents[leader].order, O_LEADER_RALLY);
        game.apply(0, C_STOP, 0.0, 0.0, 0, &[handle]);
        assert!(game.rally_ready(leader));
        assert_eq!(game.ents[infantry].hp, 10.0);
        game.apply(0, C_LEADER_RALLY, 0.0, 0.0, 0, &[handle]);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        assert_eq!(restored.state_hash(), game.state_hash());
        let maximum = game.max_hp(0, 9);
        for state in [&mut game, &mut restored] { state.update_ent(leader); }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.ents[leader].hp, 310.0);
        assert_eq!(game.ents[infantry].hp, (10.0 + maximum * 0.5).min(maximum));
        for target in [outside, other, fortress] { assert_eq!(game.ents[target].hp, 10.0); }
        assert_eq!(game.ents[dead].hp, 0.0);
        assert_eq!(game.powers.as_ref().unwrap().actor(handle).unwrap().charge, 0.0);
        assert_eq!(game.ents[leader].order, O_IDLE);
        game.powers.as_mut().unwrap().recharge_selected(handle);
        game.players[0].epoch = 5;
        assert!(!game.rally_ready(leader));
    }

    #[test]
    fn passive_membership_is_phased_and_archived_and_changes_on_epoch_band() {
        let (mut game, leader) = earned();
        let infantry = game.spawn(9, 0, 102.0, 100.0);
        let outside = game.spawn(9, 0, 105.0, 100.0);
        let fortress = game.spawn(FORTRESS, 0, 101.0, 100.0);
        let citizen = game.spawn(WORKER, 0, 101.0, 100.0);
        let outpost = game.spawn(TOWER, 0, 101.0, 100.0);
        let foe = game.spawn(9, 1, 101.0, 100.0);
        assert_eq!(game.leader_attack_scale(infantry), 1.0);
        next_phase(&mut game, leader);
        game.tick_military_leader_auras();
        assert_eq!(game.leader_attack_scale(infantry), 1.2);
        assert_eq!(game.leader_attack_scale(fortress), 1.2);
        assert_eq!(game.leader_attack_scale(outside), 1.0);
        assert_eq!(game.leader_attack_scale(citizen), 1.2);
        assert_eq!(game.leader_attack_scale(outpost), 1.0);
        assert_eq!(game.leader_attack_scale(foe), 1.0);
        game.ents[infantry].x = 106.0;
        game.tick += 1;
        game.tick_military_leader_auras();
        assert_eq!(game.leader_attack_scale(infantry), 1.2);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        assert_eq!(restored.leader_attack_scale(infantry), 1.2);
        next_phase(&mut game, leader);
        game.tick_military_leader_auras();
        assert_eq!(game.leader_attack_scale(infantry), 1.0);
        game.ents[infantry].x = 102.0;
        game.players[0].epoch = 5;
        game.tick_military_leader_auras();
        assert_eq!(game.leader_attack_scale(fortress), 1.0);
        assert_eq!(game.leader_damage_scale(infantry, false), 0.8);
        assert_eq!(game.leader_damage_scale(infantry, true), 0.8);
        assert_eq!(game.leader_damage_scale(outside, true), 1.0);
        game.players[0].epoch = 10;
        game.tick_military_leader_auras();
        assert_eq!(game.leader_damage_scale(infantry, false), 1.0);
        assert_eq!(game.leader_damage_scale(outside, true), 0.5);
        game.ents[leader].hp = 0.0;
        assert_eq!(game.leader_damage_scale(outside, true), 1.0);
    }

    #[test]
    fn fire_maneuver_phases_movement_and_retires_after_the_strict_deadline() {
        let (mut game, leader) = earned();
        game.players[0].epoch = 5;
        game.rebuild_modifiers(0, true);
        let handle = game.h(leader);
        let infantry = game.spawn(14, 0, 103.0, 100.0);
        let enemy = game.spawn(14, 1, 103.0, 101.0);
        let outside = game.spawn(14, 1, 106.0, 100.0);
        game.powers.as_mut().unwrap().recharge_selected(handle);
        game.initial_ceasefire = 1;
        assert!(!game.fire_maneuver_ready(leader));
        game.apply(0, C_LEADER_RALLY, 0.0, 0.0, 1, &[handle]);
        assert_eq!(game.ents[leader].order, O_IDLE);
        game.initial_ceasefire = 0;
        assert!(game.fire_maneuver_ready(leader));
        game.apply(0, C_LEADER_RALLY, 0.0, 0.0, 1, &[handle]);
        assert_eq!(game.ents[leader].order, O_LEADER_FIRE_MANEUVER);
        game.save();
        let mut game = Game::load(&game.savebuf).unwrap();
        game.update_ent(leader);
        assert!(!game.fire_maneuver_ready(leader));
        assert_eq!(game.leader_movement_scale(infantry), 1.0);
        assert!(!game.leader_immobilized(enemy));
        game.order_move(&[enemy], 110.0, 110.0, false);
        assert_eq!(game.ents[enemy].order, O_MOVE);
        next_phase(&mut game, leader);
        game.tick_military_leader_auras();
        assert_eq!(game.leader_movement_scale(infantry), 1.5);
        assert!(game.leader_immobilized(enemy));
        assert!(!game.leader_immobilized(outside));
        let position = (game.ents[enemy].x, game.ents[enemy].y);
        game.update_ent(enemy);
        assert_eq!(game.ents[enemy].order, O_IDLE);
        assert_eq!((game.ents[enemy].x, game.ents[enemy].y), position);
        game.order_move(&[enemy], 110.0, 110.0, false);
        assert_eq!(game.ents[enemy].order, O_IDLE);
        game.save();
        let mut game = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.leader_movement_scale(infantry), 1.5);
        assert!(game.leader_immobilized(enemy));
        game.powers.as_mut().unwrap().recharge_selected(handle);
        game.apply(0, C_LEADER_RALLY, 0.0, 0.0, 1, &[handle]);
        game.update_ent(leader);
        game.tick_military_leader_auras();
        assert_eq!(game.leader_movement_scale(infantry), 2.0);
        assert_eq!(game.military_leader_powers.as_ref().unwrap().maneuvers.len(), 2);
        let deadline = game.military_leader_powers.as_ref().unwrap().maneuvers[0].deadline();
        while game.sim_elapsed() < deadline { game.tick += 1; }
        assert_eq!(game.sim_elapsed(), deadline);
        game.tick_military_leader_auras();
        assert_eq!(game.leader_movement_scale(infantry), 2.0);
        game.tick += 1;
        game.tick_military_leader_auras();
        assert_eq!(game.leader_movement_scale(infantry), 1.5);
        game.kill(leader);
        assert_eq!(game.leader_movement_scale(infantry), 1.0);
        assert!(!game.leader_immobilized(enemy));
    }

    #[test]
    fn native_radius_includes_mobile_footprints_and_the_fortress_box_axes() {
        let mut center = E0;
        center.x = 100.0;
        center.y = 100.0;
        let mut infantry = center;
        infantry.def = 9;
        infantry.x = f32::from_bits(103.125_f32.to_bits() - 1);
        assert!(in_radius(&center, &infantry, 3.0));
        infantry.x = 103.125;
        assert!(!in_radius(&center, &infantry, 3.0));
        let mut fortress = center;
        fortress.def = FORTRESS;
        fortress.x = 104.4;
        fortress.y = 104.4;
        assert!(in_radius(&center, &fortress, 3.0));
        fortress.x = 104.5;
        assert!(!in_radius(&center, &fortress, 3.0));
    }
}
