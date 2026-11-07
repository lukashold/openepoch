//! Priest SpecialPower cast and the group-owned BlessUnits effect.

use super::*;
use crate::priest42_defs::PRIEST;
use std::collections::{BTreeMap, BTreeSet};

const CAST_TIME: f32 = 4.0;
const BLESS_TIME: f32 = 20.0;
const BLESS_RANGE: f32 = 1.0;
const RADIUS: f32 = 2.0;
const MAGIC: u32 = u32::from_le_bytes(*b"BLS1");

#[derive(Clone, Copy)]
struct Cast {
    target: u32,
    started: Option<f32>,
}

struct Group {
    owner: u8,
    center: Option<u32>,
    expires: f32,
    inside: BTreeSet<u32>,
    children: BTreeMap<u32, BlessChildren>,
}

#[derive(Default)]
struct BlessChildren {
    active: u32,
    pending: u32,
}

#[derive(Default)]
pub(crate) struct BlessState {
    casts: BTreeMap<u32, Cast>,
    groups: Vec<Group>,
}

impl BlessState {
    fn empty(&self) -> bool { self.casts.is_empty() && self.groups.is_empty() }

    fn fields(&self, mut write: impl FnMut(u32)) {
        write(self.casts.len() as u32);
        write(self.groups.len() as u32);
        for (&handle, cast) in &self.casts {
            for word in [handle, cast.target, cast.started.unwrap_or(-1.0).to_bits()] { write(word); }
        }
        for group in &self.groups {
            for word in [u32::from(group.owner), group.center.unwrap_or(NONE), group.expires.to_bits(),
                group.inside.len() as u32, group.children.len() as u32] {
                write(word);
            }
            for &member in &group.inside { write(member); }
            for (&serial, children) in &group.children {
                for word in [serial, children.active, children.pending] { write(word); }
            }
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        if self.empty() { return; }
        write(MAGIC);
        self.fields(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.empty() || core.is_empty() { return core; }
        let mut data = Vec::with_capacity(core.len() + 128);
        for word in [MAGIC, 1, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.fields(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }
}

impl Game {
    pub(crate) fn bless_ready(&self, index: usize) -> bool {
        let caster = self.ents[index];
        caster.alive && caster.hp > 0.0 && caster.def == PRIEST && self.winner == 0
            && self.powers.as_ref().and_then(|state| state.actor(self.h(index))).is_some_and(|actor| actor.charge >= 1.0)
    }

    pub(crate) fn bless_target(&self, owner: u8, index: usize) -> bool {
        let target = self.ents[index];
        target.alive && target.hp > 0.0 && target.team == owner && target.stuck != BOARDED
            && crate::ee2_combat::land_effect_target(target.def, self.players[owner as usize].epoch)
    }

    pub(crate) fn order_bless(&mut self, indices: &[usize], target: usize) {
        let center = self.ents[target];
        if !self.bless_target(center.team, target) { return; }
        let target = self.h(target);
        let key = self.key_for(LAND, center.x, center.y);
        for &index in indices {
            if !self.bless_ready(index) || self.ents[index].team != center.team { continue; }
            let handle = self.h(index);
            if self.bless.casts.get(&handle).is_some_and(|cast| cast.target == target) { continue; }
            self.bless.casts.insert(handle, Cast { target, started: None });
            self.native_stop_attack(index);
            let caster = &mut self.ents[index];
            caster.order = O_BLESS;
            caster.otgt = target;
            caster.tgt = NONE;
            caster.fkey = key;
            caster.fslot = u16::MAX;
            caster.stuck = 0;
        }
    }

    pub(crate) fn reconcile_bless_casts(&mut self) {
        if self.bless.casts.is_empty() { return; }
        let invalid: Vec<_> = self.bless.casts.iter().filter_map(|(&handle, cast)| {
            let valid = self.get(handle).is_some_and(|index| {
                let caster = self.ents[index];
                caster.hp > 0.0 && caster.order == O_BLESS && caster.otgt == cast.target
                    && self.get(cast.target).is_some_and(|target| self.bless_target(caster.team, target))
            });
            (!valid).then_some(handle)
        }).collect();
        for handle in invalid {
            self.bless.casts.remove(&handle);
            if let Some(index) = self.get(handle) {
                if self.ents[index].order == O_BLESS { Self::set_idle(&mut self.ents[index]); }
            }
        }
    }

    pub(crate) fn retire_bless_actor(&mut self, handle: u32, serial: u32) {
        self.bless.casts.remove(&handle);
        for group in &mut self.bless.groups {
            group.children.remove(&serial);
            group.inside.remove(&serial);
            if group.center == Some(serial) {
                group.center = None;
                group.inside.clear();
            }
        }
    }

    pub(super) fn do_bless(&mut self, index: usize, caster: &mut Ent, def: &Def) -> (Option<f32>, Option<(f32, f32)>) {
        let handle = self.h(index);
        let Some(target) = self.get(caster.otgt).filter(|&target| self.bless_target(caster.team, target)) else {
            self.bless.casts.remove(&handle);
            Self::set_idle(caster);
            return (None, None);
        };
        let center = self.ents[target];
        let offset = (center.x - caster.x, center.y - caster.y);
        let face = (handle != caster.otgt).then(|| offset.1.atan2(offset.0));
        let now = self.sim_elapsed();
        let Some(cast) = self.bless.casts.get_mut(&handle) else {
            Self::set_idle(caster);
            return (None, None);
        };
        if offset.0.hypot(offset.1) > BLESS_RANGE + def.rad() + DEFS[center.def as usize].rad() {
            if (self.tick + index as u32) % 8 == 0 { caster.fkey = self.key_for(LAND, center.x, center.y); }
            return (None, Some(self.steer(caster, def, center.x, center.y)));
        }
        let start_effect = cast.started.is_none();
        let started = *cast.started.get_or_insert(now);
        if start_effect { self.attach_priest_effect(handle, crate::power42::FX_BLESS); }
        if f64::from(now) - f64::from(started) <= f64::from(CAST_TIME) { return (face, None); }
        let target_serial = self.powers.as_ref().and_then(|state| state.actor(caster.otgt)).map(|record| record.serial);
        if let Some(center) = target_serial {
            self.spend_unit_reserve(handle, 1.0);
            let expires = crate::power42_math::store_toward_zero(f64::from(now) + f64::from(BLESS_TIME));
            self.bless.groups.push(Group { owner: caster.team, center: Some(center), expires,
                inside: BTreeSet::new(), children: BTreeMap::new() });
        }
        self.bless.casts.remove(&handle);
        Self::set_idle(caster);
        (face, None)
    }

    pub(crate) fn priest_bless_elapsed(&self, handle: u32, render_time: f32) -> Option<f32> {
        let actor = self.ents[self.get(handle)?];
        let cast = self.bless.casts.get(&handle)?;
        if actor.order != O_BLESS || actor.def != PRIEST || !render_time.is_finite() { return None; }
        cast.started.map(|start| (render_time - start).clamp(0.0, CAST_TIME))
    }

    pub(crate) fn tick_blessings(&mut self) {
        if self.bless.groups.is_empty() { return; }
        let Some(powers) = &self.powers else { return; };
        let (now, step, tick) = (self.sim_elapsed(), self.sim_step(), self.tick);
        let entities = &self.ents;
        let high = self.high;
        let players = &self.players;
        self.bless.groups.retain_mut(|group| {
            if now > group.expires { return false; }
            group.children.retain(|&serial, _| powers.contains_serial(serial));
            group.inside.retain(|serial| group.children.contains_key(serial));
            let due = |serial| {
                let id = (u32::from(group.owner) + 1) << 27 | serial;
                crate::capture_math25::capture_event_due(tick, id, step)
            };
            if let Some(center) = group.center {
                if !powers.contains_serial(center) {
                    group.center = None;
                    group.inside.clear();
                } else if due(center) {
                    let mut inside = BTreeSet::new();
                    if let Some(handle) = powers.live_handle(center) {
                        let center = entities[(handle & 0xffff) as usize];
                        let epoch = players[group.owner as usize].epoch;
                        for (index, actor) in entities[..high].iter().enumerate() {
                            if !actor.alive || actor.hp <= 0.0 || actor.team != group.owner ||
                                !crate::ee2_combat::land_effect_target(actor.def, epoch) ||
                                (actor.x - center.x).powi(2) + (actor.y - center.y).powi(2) > RADIUS * RADIUS { continue; }
                            let handle = u32::from(actor.gen) << 16 | index as u32;
                            if let Some(record) = powers.actor(handle) { inside.insert(record.serial); }
                        }
                    }
                    for &serial in inside.difference(&group.inside) {
                        let child = group.children.entry(serial).or_default();
                        child.pending = child.pending.saturating_add(1);
                    }
                    group.inside = inside;
                }
            }
            for (&serial, children) in &mut group.children {
                if children.pending != 0 && due(serial) {
                    children.active = children.active.saturating_add(children.pending);
                    children.pending = 0;
                }
            }
            true
        });
    }

    pub(crate) fn blessed_attack(&self, handle: u32) -> f32 {
        crate::power42_math::store_toward_zero(1.0 + f64::from(self.bless_attack_bonus(handle)))
    }

    pub(super) fn bless_attack_bonus(&self, handle: u32) -> f32 {
        let Some(actor) = self.powers.as_ref().and_then(|state| state.actor(handle)) else { return 0.0; };
        let count = self.bless.groups.iter().filter_map(|group| group.children.get(&actor.serial))
            .fold(0_u32, |total, child| total.saturating_add(child.active).min(5));
        let mut modifier = 0.0_f32;
        for _ in 0..count {
            modifier = crate::power42_math::store_toward_zero(f64::from(modifier) + (f64::from(1.2_f32) - 1.0));
        }
        modifier.min(1.0)
    }

    pub(crate) fn restore_blessings(&mut self, fields: Option<&[u8]>) -> Option<()> {
        let active = self.ents[..self.high].iter().filter(|actor| actor.alive && actor.order == O_BLESS).count();
        let Some(fields) = fields else { return (active == 0).then_some(()); };
        let powers = self.powers.as_ref()?;
        let now = self.sim_elapsed();
        let (casts, groups) = (word(fields, 0)? as usize, word(fields, 4)? as usize);
        if self.net || casts != active || casts > MAXE || groups > MAXE * 5 { return None; }
        let mut state = BlessState::default();
        let mut at = 8;
        for _ in 0..casts {
            let (handle, target) = (word(fields, at)?, word(fields, at + 4)?);
            let start = f32::from_bits(word(fields, at + 8)?);
            let started = if start == -1.0 { None } else if (0.0..=now).contains(&start) { Some(start) } else { return None; };
            let caster = self.ents[self.get(handle)?];
            if caster.def != PRIEST || caster.hp <= 0.0 || caster.order != O_BLESS || caster.otgt != target
                || !self.bless_target(caster.team, self.get(target)?) || state.casts.insert(handle, Cast { target, started }).is_some() {
                return None;
            }
            at += 12;
        }
        for _ in 0..groups {
            let owner = u8::try_from(word(fields, at)?).ok()?;
            let center = word(fields, at + 4)?;
            let expires = f32::from_bits(word(fields, at + 8)?);
            let inside_count = word(fields, at + 12)? as usize;
            let child_count = word(fields, at + 16)? as usize;
            if owner as usize >= self.nplayers || !expires.is_finite() || expires < now || f64::from(expires) > f64::from(now) + 20.0
                || inside_count > child_count || child_count > MAXE * (CARGO_MAX + 1)
                || (center != NONE && !powers.contains_serial(center)) { return None; }
            at += 20;
            let mut inside = BTreeSet::new();
            for _ in 0..inside_count {
                let serial = word(fields, at)?;
                if !inside.insert(serial) { return None; }
                at += 4;
            }
            let mut children = BTreeMap::new();
            for _ in 0..child_count {
                let (serial, active, pending) = (word(fields, at)?, word(fields, at + 4)?, word(fields, at + 8)?);
                if !powers.contains_serial(serial) || (active == 0 && pending == 0) ||
                    children.insert(serial, BlessChildren { active, pending }).is_some() { return None; }
                at += 12;
            }
            if !inside.iter().all(|serial| children.contains_key(serial)) { return None; }
            state.groups.push(Group { owner, center: (center != NONE).then_some(center), expires, inside, children });
        }
        if at != fields.len() || state.empty() { return None; }
        self.bless = state;
        Some(())
    }
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let len = word(data, 8)? as usize;
    let end = data.len().checked_sub(4)?;
    let start = end.checked_sub(len)?;
    if word(data, 4)? != 1 || len == 0 || start < 20 || word(data, end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}
