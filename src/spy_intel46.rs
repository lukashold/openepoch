//! The Spy's Gather Intelligence cast and temporary building sight.

use super::*;
use crate::spy46_defs::SPY;
use std::collections::BTreeMap;

const MAGIC: u32 = u32::from_le_bytes(*b"INT1");
const CAST_TIME: f32 = 5.0;
const CAST_RANGE: f32 = 0.1;
const SIGHT_TIME: f32 = 30.0;
const CROWN_TIME: f32 = 300.0;
const TECH_ON_GATHER_INTEL: usize = 83; // Imperial Epoch 7, slot 4.
const TECH_LIMIT: f64 = 999_999.0;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum SpyOperation {
    Gather = 0,
    Sabotage = 1,
    Restore = 2,
    Contaminate = 3,
    Cleanup = 4,
}

impl SpyOperation {
    pub(crate) fn read(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Gather), 1 => Some(Self::Sabotage), 2 => Some(Self::Restore),
            3 => Some(Self::Contaminate), 4 => Some(Self::Cleanup), _ => None,
        }
    }

    pub(crate) fn available(self, game: &Game, owner: u8) -> bool {
        match self {
            Self::Gather => true,
            Self::Sabotage | Self::Restore => game.has_research(owner, 40),
            Self::Contaminate | Self::Cleanup => game.has_research(owner, 100),
        }
    }

    pub(crate) fn blocked_by_ceasefire(self) -> bool {
        matches!(self, Self::Gather | Self::Sabotage | Self::Contaminate)
    }
}

#[derive(Clone, Copy)]
struct Cast {
    target: u32,
    started: Option<f32>,
    operation: SpyOperation,
}

#[derive(Default)]
pub(crate) struct IntelState {
    casts: BTreeMap<u32, Cast>,
    sight: BTreeMap<(u8, u8, DefId), f32>,
    crowns: BTreeMap<(u8, u8), f32>,
    sight_changed: bool,
}

impl IntelState {
    fn empty(&self) -> bool {
        self.casts.is_empty() && self.sight.is_empty() && self.crowns.is_empty()
    }

    fn version(&self) -> u32 {
        if self.casts.values().any(|cast| cast.operation != SpyOperation::Gather) { 2 } else { 1 }
    }

    fn words(&self, version: u32, mut write: impl FnMut(u32)) {
        for len in [self.casts.len(), self.sight.len(), self.crowns.len()] { write(len as u32); }
        for (&handle, cast) in &self.casts {
            for word in [handle, cast.target, cast.started.unwrap_or(-1.0).to_bits()] { write(word); }
            if version == 2 { write(cast.operation as u32); }
        }
        for (&(observer, owner, def), &expires) in &self.sight {
            let key = u32::from(def) << 16 | u32::from(owner) << 8 | u32::from(observer);
            write(key);
            write(expires.to_bits());
        }
        for (&(observer, owner), &expires) in &self.crowns {
            write(u32::from(owner) << 8 | u32::from(observer));
            write(expires.to_bits());
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        if self.empty() { return; }
        write(MAGIC);
        let version = self.version();
        if version != 1 { write(version); }
        self.words(version, write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.empty() || core.is_empty() { return core; }
        let mut data = Vec::with_capacity(core.len() + 32);
        let version = self.version();
        for word in [MAGIC, version, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(version, |word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }
}

impl Game {
    pub(crate) fn spy_ready(&self, index: usize) -> bool {
        let actor = self.ents[index];
        actor.alive && actor.hp > 0.0 && actor.def == SPY && self.winner == 0
            && self.powers.as_ref().and_then(|state| state.actor(self.h(index))).is_some_and(|actor| actor.charge >= 1.0)
    }

    pub(crate) fn spy_intel_target(&self, owner: u8, index: usize) -> bool {
        self.spy_operation_target(owner, index, SpyOperation::Gather)
    }

    pub(crate) fn spy_operation_target(&self, owner: u8, index: usize, operation: SpyOperation) -> bool {
        let target = self.ents[index];
        if !target.alive || target.hp <= 0.0 || target.built < 1.0
            || target.team as usize >= self.nplayers || !DEFS[target.def as usize].building
            || operation.blocked_by_ceasefire() && self.initial_ceasefire_active() {
            return false;
        }
        match operation {
            SpyOperation::Gather => target.team != owner,
            SpyOperation::Sabotage => target.team != owner && !self.sabotaged(index),
            SpyOperation::Restore => self.allied(owner, target.team) && self.sabotaged(index),
            SpyOperation::Contaminate => matches!(target.def, TC | WAREHOUSE | crate::granary211::ID)
                && target.team != owner && !self.contaminated(index),
            SpyOperation::Cleanup => matches!(target.def, TC | WAREHOUSE | crate::granary211::ID)
                && self.allied(owner, target.team) && self.contaminated(index),
        }
    }

    pub(crate) fn order_spy_operation(&mut self, indices: &[usize], target: usize, operation: SpyOperation) {
        let building = self.ents[target];
        let target = self.h(target);
        let key = self.key_for(LAND, building.x, building.y);
        for &index in indices {
            if !self.spy_ready(index) || !operation.available(self, self.ents[index].team) { continue; }
            let handle = self.h(index);
            if self.spy_intel.casts.get(&handle).is_some_and(|cast| cast.target == target && cast.operation == operation) { continue; }
            self.spy_intel.casts.insert(handle, Cast { target, started: None, operation });
            self.native_stop_attack(index);
            let actor = &mut self.ents[index];
            actor.order = O_GATHER_INTEL;
            actor.otgt = target;
            actor.tgt = NONE;
            actor.fkey = key;
            actor.fslot = u16::MAX;
            actor.stuck = 0;
        }
    }

    fn valid_spy_cast(&self, handle: u32, target: u32, operation: SpyOperation) -> bool {
        self.get(handle).is_some_and(|index| {
            let actor = self.ents[index];
            self.spy_ready(index) && actor.order == O_GATHER_INTEL && actor.otgt == target
                && operation.available(self, actor.team)
                && self.get(target).is_some_and(|target| self.spy_operation_target(actor.team, target, operation))
        })
    }

    pub(crate) fn reconcile_spy_intel(&mut self) {
        let invalid: Vec<_> = self.spy_intel.casts.iter().filter_map(|(&handle, cast)| {
            (!self.valid_spy_cast(handle, cast.target, cast.operation)).then_some(handle)
        }).collect();
        for handle in invalid {
            self.spy_intel.casts.remove(&handle);
            if let Some(index) = self.get(handle) {
                if self.ents[index].order == O_GATHER_INTEL { Self::set_idle(&mut self.ents[index]); }
            }
        }
    }

    pub(super) fn do_spy_intel(&mut self, index: usize, actor: &mut Ent, def: &Def) -> (Option<f32>, Option<(f32, f32)>) {
        let handle = self.h(index);
        let Some(operation) = self.spy_intel.casts.get(&handle).map(|cast| cast.operation) else {
            Self::set_idle(actor);
            return (None, None);
        };
        let Some(target) = self.get(actor.otgt).filter(|&target| self.spy_operation_target(actor.team, target, operation)) else {
            self.spy_intel.casts.remove(&handle);
            Self::set_idle(actor);
            return (None, None);
        };
        let building = self.ents[target];
        let offset = (building.x - actor.x, building.y - actor.y);
        let now = self.sim_elapsed();
        let cast = self.spy_intel.casts.get_mut(&handle).expect("admitted spy cast");
        let half = Self::building_half(&building);
        let edge = (offset.0.abs() - half).max(0.0).hypot((offset.1.abs() - half).max(0.0));
        if cast.started.is_none() && edge > CAST_RANGE + def.rad() {
            return (None, Some(self.steer(actor, def, building.x, building.y)));
        }
        let face = Some(offset.1.atan2(offset.0));
        let started = *cast.started.get_or_insert(now);
        if f64::from(now) - f64::from(started) <= f64::from(CAST_TIME) { return (face, None); }

        self.spend_unit_reserve(handle, 1.0);
        self.spy_intel.casts.remove(&handle);
        match operation {
            SpyOperation::Gather => self.complete_spy_intel(actor.team, building, now),
            SpyOperation::Sabotage => self.sabotage_building(actor.team, target, now),
            SpyOperation::Restore => self.restore_building(target),
            SpyOperation::Contaminate => self.contaminate_building(actor.team, target, now),
            SpyOperation::Cleanup => self.clean_building(target),
        }
        Self::set_idle(actor);
        (face, None)
    }

    fn complete_spy_intel(&mut self, team: u8, building: Ent, now: f32) {
        let end = |duration| crate::power42_math::store_toward_zero(f64::from(now) + f64::from(duration));
        self.spy_intel.sight.insert((team, building.team, building.def), end(SIGHT_TIME));
        self.spy_intel.sight_changed = true;
        if building.def == TC {
            self.spy_intel.crowns.insert((team, building.team), (f64::from(now) + f64::from(CROWN_TIME)).round() as f32);
        }
        if self.has_research(team, TECH_ON_GATHER_INTEL) {
            let owner = building.team as usize;
            let observer = team as usize;
            let stock = f64::from(self.research_points[owner]);
            let amount = stock.min(3.0);
            if amount > 0.0 {
                let clamp = |balance: f64| crate::power42_math::store_toward_zero(balance.clamp(0.0, TECH_LIMIT));
                self.research_points[owner] = clamp(stock - amount);
                self.research_points[observer] = clamp(f64::from(self.research_points[observer]) + amount);
            }
        }
    }

    pub(crate) fn spy_intel_elapsed(&self, handle: u32, render_time: f32) -> Option<f32> {
        let cast = self.spy_intel.casts.get(&handle)?;
        if !render_time.is_finite() || !self.valid_spy_cast(handle, cast.target, cast.operation) { return None; }
        cast.started.map(|start| (render_time - start).clamp(0.0, CAST_TIME))
    }

    pub(crate) fn spy_pose_state(&self, handle: u32) -> u32 {
        self.spy_intel_elapsed(handle, self.sim_elapsed()).map_or(0, |_| self.spy_intel.casts[&handle].operation as u32 + 1)
    }

    pub(crate) fn crown_intel_seconds(&self, observer: u8, owner: u8) -> f32 {
        self.spy_intel.crowns.get(&(observer, owner)).map_or(0.0, |&expires| (expires - self.sim_elapsed().round()).max(0.0))
    }

    pub(crate) fn crown_intel_active(&self, observer: u8, owner: u8) -> bool {
        self.spy_intel.crowns.get(&(observer, owner)).is_some_and(|&expires| self.sim_elapsed().round() <= expires)
    }

    pub(crate) fn tick_spy_intel(&mut self) -> bool {
        let now = self.sim_elapsed();
        let count = self.spy_intel.sight.len();
        self.spy_intel.sight.retain(|_, &mut expires| now <= expires);
        self.spy_intel.crowns.retain(|_, &mut expires| now.round() <= expires);
        std::mem::take(&mut self.spy_intel.sight_changed) || self.spy_intel.sight.len() != count
    }

    pub(crate) fn paint_spy_intel(&mut self) {
        for &(observer, owner, def) in self.spy_intel.sight.keys() {
            for actor in &self.ents[..self.high] {
                if actor.alive && actor.hp > 0.0 && actor.built >= 1.0 && actor.team == owner && actor.def == def {
                    let range = self.modifiers[owner as usize][def as usize][5];
                    Self::paint_unit_sight(&mut self.team_vis, observer, actor, range, self.map.world_dimensions(), self.map.layout.stride);
                }
            }
        }
    }

    pub(crate) fn restore_spy_intel(&mut self, fields: Option<(u32, &[u8])>) -> Option<()> {
        let orders = self.ents[..self.high].iter().filter(|actor| actor.alive && actor.order == O_GATHER_INTEL).count();
        let Some((version, fields)) = fields else { return (orders == 0).then_some(()); };
        if self.powers.is_none() || self.net { return None; }
        let mut words = fields.chunks_exact(4);
        let mut read = || words.next().and_then(|chunk| Some(u32::from_le_bytes(chunk.try_into().ok()?)));
        let counts = [read()? as usize, read()? as usize, read()? as usize];
        let expected = counts[0].checked_mul((version + 2) as usize)?.checked_add(counts[1].checked_add(counts[2])?.checked_mul(2)?)?;
        if counts[0] != orders || fields.len() != expected.checked_mul(4)?.checked_add(12)? { return None; }
        let now = self.sim_elapsed();
        let mut state = IntelState::default();
        for _ in 0..counts[0] {
            let (handle, target, start) = (read()?, read()?, f32::from_bits(read()?));
            let operation = if version == 2 { SpyOperation::read(read()?)? } else { SpyOperation::Gather };
            let started = if start == -1.0 { None } else if (0.0..=now).contains(&start) { Some(start) } else { return None; };
            if !self.valid_spy_cast(handle, target, operation)
                || state.casts.insert(handle, Cast { target, started, operation }).is_some() {
                return None;
            }
        }
        for (index, count) in counts[1..].iter().enumerate() {
            let (start, duration) = [(now, SIGHT_TIME), (now.round(), CROWN_TIME)][index];
            for _ in 0..*count {
                let (key, expires) = (read()?, f32::from_bits(read()?));
                let (observer, owner, def) = (key as u8, (key >> 8) as u8, (key >> 16) as DefId);
                if observer == owner || (observer.max(owner) as usize) >= self.nplayers
                    || !(start..=start + duration).contains(&expires) { return None; }
                if index == 0 {
                    if !DEFS.get(def as usize).is_some_and(|d| d.building)
                        || state.sight.insert((observer, owner, def), expires).is_some() { return None; }
                } else if def != 0 || expires.fract() != 0.0 || state.crowns.insert((observer, owner), expires).is_some() {
                    return None;
                }
            }
        }
        self.spy_intel = state;
        Some(())
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<(u32, &[u8])>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let len = word(8)? as usize;
    let end = data.len().checked_sub(4)?;
    let start = end.checked_sub(len)?;
    let version = word(4)?;
    if !matches!(version, 1 | 2) || len == 0 || start < 24 || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some((version, &data[12..start]))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_GATHER_INTEL, C_STOP};

    #[test]
    fn intel_cast_archive_type_filter_and_expiry() {
        let mut g = Game::new(11, 2, 0, 1, 0);
        assert!(g.configure_priest_skirmish());
        let spy = g.spawn(SPY, 0, 40.0, 40.0);
        let target = g.spawn(TC, 1, 42.0, 40.0);
        let same_type = g.spawn(TC, 1, 70.0, 40.0);
        let other_type = g.spawn(HOUSE, 1, 100.0, 40.0);
        let other_owner = g.spawn(TC, 2, 130.0, 40.0);
        let unfinished = g.spawn(TC, 1, 160.0, 40.0);
        g.ents[unfinished].built = 0.4;
        let spy_handle = g.h(spy);
        let target_handle = g.h(target);
        let issue = |g: &mut Game, target| g.apply(0, C_GATHER_INTEL, 0.0, 0.0, target as i32, &[spy_handle]);
        let cast = |g: &mut Game| {
            let mut actor = g.ents[spy];
            g.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
            g.ents[spy] = actor;
        };
        let charge = |g: &Game| g.powers.as_ref().unwrap().actor(spy_handle).unwrap().charge;
        let fog = |g: &Game, observer, index: usize| {
            let unit = &g.ents[index];
            g.real_fog(observer, tile_of(unit.x, unit.y))
        };
        assert!(g.configure_initial_ceasefire(1));
        g.powers.as_mut().unwrap().recharge_selected(spy_handle);
        issue(&mut g, target_handle);
        assert_eq!(g.ents[spy].order, O_IDLE);
        assert!(g.configure_initial_ceasefire(0));
        let unfinished_handle = g.h(unfinished);
        issue(&mut g, unfinished_handle);
        assert_eq!(g.ents[spy].order, O_IDLE);
        issue(&mut g, target_handle);
        cast(&mut g);
        assert_eq!(g.spy_intel_elapsed(spy_handle, 0.0), Some(0.0));
        g.apply(0, C_STOP, 0.0, 0.0, 0, &[spy_handle]);
        assert_eq!(g.spy_intel_elapsed(spy_handle, 0.0), None);
        assert_eq!(charge(&g), 1.0);
        g.researched[0][TECH_ON_GATHER_INTEL / 32] |= 1 << (TECH_ON_GATHER_INTEL % 32);
        g.research_points[0] = 1.0;
        g.research_points[1] = 2.25;
        issue(&mut g, target_handle);
        cast(&mut g);
        g.tick = 100;
        cast(&mut g);
        assert_eq!(charge(&g), 1.0);
        assert_eq!(g.spy_intel_elapsed(spy_handle, 5.0), Some(5.0));
        g.save();
        let mut g = Game::load(&g.savebuf).expect("cast restored at exact five-second boundary");
        g.tick = 101;
        cast(&mut g);
        assert_eq!(charge(&g), 0.0);
        assert_eq!(g.research_points[0], 3.25);
        assert_eq!(g.research_points[1], 0.0);
        assert_eq!(g.spy_intel_elapsed(spy_handle, 5.05), None);
        assert!((g.crown_intel_seconds(0, 1) - 300.0).abs() < 0.001);
        assert_eq!(g.crown_intel_seconds(2, 1), 0.0);
        assert!(g.tick_spy_intel());
        g.team_vis.fill(0);
        g.update_fog();
        assert_eq!(fog(&g, 0, same_type), 255);
        for index in [other_type, other_owner, unfinished] { assert_eq!(fog(&g, 0, index), 0); }
        assert_eq!(fog(&g, 2, same_type), 0);
        g.save();
        let mut corrupt = g.savebuf.clone();
        let marker = corrupt.windows(4).position(|word| word == b"INT1").unwrap();
        corrupt[marker + 24] ^= 0x80;
        assert!(Game::load(&corrupt).is_none());
        let mut g = Game::load(&g.savebuf).expect("active sight and crown intel restored");
        assert_eq!(fog(&g, 0, same_type), 255);
        g.tick = 702;
        assert!(g.tick_spy_intel());
        g.update_fog();
        assert_eq!(fog(&g, 0, same_type), 128);
        assert!(g.crown_intel_seconds(0, 1) > 0.0);
        g.tick = 6109;
        g.tick_spy_intel();
        assert_eq!(g.crown_intel_seconds(0, 1), 0.0);
        assert!(g.crown_intel_active(0, 1));
        g.tick = 6111;
        g.tick_spy_intel();
        assert!(!g.crown_intel_active(0, 1));
        assert!(g.spy_intel.empty());
    }
}
