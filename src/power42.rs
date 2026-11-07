//! Live reserve and original-ID scheduling for the gated UP Priest roster.

use crate::defs::{DefId, DEFS, AIR, F_TRANSPORT, FORTRESS, SEA, TEMPLE};
use crate::game::{Game, MAXE};
use crate::power42_math::{accumulate_recharge_factor, update_power_reserve};
use crate::priest42_defs::PRIEST;
use crate::spy46_defs::SPY;
use std::collections::BTreeMap;

const SERIAL_MASK: u32 = 0x07ff_ffff;
const MAGIC: u32 = u32::from_le_bytes(*b"PWR1");
const HEADER: usize = 24;

fn recharge_seconds(def: DefId, leaders: bool) -> Option<f32> {
    match def {
        PRIEST => Some(90.0),
        SPY => Some(60.0),
        def if leaders && crate::leader58_defs::branch(def).is_some() => Some(60.0),
        _ => None,
    }
}

#[repr(C)]
pub(crate) struct PriestEffect {
    handle: u32,
    kind: u32,
    started: f32,
}

pub(crate) const FX_CONVERT: u32 = 1;
pub(crate) const FX_BLESS: u32 = 2;
pub(crate) const FX_CONVERTED: u32 = 3;
pub(crate) const FX_RALLY_HEALED: u32 = 4;

#[derive(Clone, Copy)]
pub(crate) struct PowerActor {
    pub(crate) serial: u32,
    pub(crate) charge: f32,
}

#[derive(Default)]
pub(crate) struct PowerState {
    pub(crate) cursor: u32,
    live: BTreeMap<u32, PowerActor>,
    cargo: BTreeMap<(u32, u32), PowerActor>,
    occupied: BTreeMap<u32, Option<u32>>,
}

impl PowerState {
    pub(crate) fn register(&mut self, handle: u32) {
        self.cursor += 1;
        if self.cursor >= SERIAL_MASK { self.cursor = 0; }
        while self.occupied.contains_key(&self.cursor) {
            self.cursor = (self.cursor + 1) & SERIAL_MASK;
        }
        self.occupied.insert(self.cursor, Some(handle));
        self.live.insert(handle, PowerActor { serial: self.cursor, charge: 0.0 });
    }

    pub(crate) fn retire(&mut self, handle: u32) {
        if let Some(actor) = self.live.remove(&handle) { self.occupied.remove(&actor.serial); }
        for slot in 0..crate::defs::CARGO_MAX as u32 {
            if let Some(actor) = self.cargo.remove(&(handle, slot)) { self.occupied.remove(&actor.serial); }
        }
    }

    pub(crate) fn embark(&mut self, handle: u32, host: u32, slot: usize) {
        if let Some(actor) = self.live.remove(&handle) {
            self.occupied.insert(actor.serial, None);
            self.cargo.insert((host, slot as u32), actor);
        }
    }

    pub(crate) fn finish_unload(&mut self, host: u32, slot: usize, restored: Result<u32, usize>, cursor: u32) {
        let Some(actor) = self.cargo.remove(&(host, slot as u32)) else { return; };
        match restored {
            Ok(handle) => {
                self.retire(handle);
                self.cursor = cursor;
                self.occupied.insert(actor.serial, Some(handle));
                self.live.insert(handle, actor);
            }
            Err(remaining) => { self.cargo.insert((host, remaining as u32), actor); }
        }
    }

    pub(crate) fn actor(&self, handle: u32) -> Option<PowerActor> { self.live.get(&handle).copied() }

    pub(crate) fn live_handle(&self, serial: u32) -> Option<u32> { self.occupied.get(&serial).copied().flatten() }

    pub(crate) fn contains_serial(&self, serial: u32) -> bool { self.occupied.contains_key(&serial) }

    pub(crate) fn recharge_selected(&mut self, handle: u32) {
        if let Some(actor) = self.live.get_mut(&handle) { actor.charge = 1.0; }
    }

    fn fields(&self, mut word: impl FnMut(u32)) {
        for value in [self.cursor, self.live.len() as u32, self.cargo.len() as u32] { word(value); }
        for (&handle, &actor) in &self.live {
            for value in [handle, actor.serial, actor.charge.to_bits()] { word(value); }
        }
        for (&(host, slot), &actor) in &self.cargo {
            for value in [host, slot, actor.serial, actor.charge.to_bits()] { word(value); }
        }
    }

    pub(crate) fn hash(&self, mut word: impl FnMut(u32)) {
        word(MAGIC);
        self.fields(word);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut data = Vec::with_capacity(HEADER + self.live.len() * 12 + self.cargo.len() * 16 + core.len() + 4);
        for word in [MAGIC, 1, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.fields(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }
}

impl Game {
    pub(crate) fn attach_priest_effect(&mut self, handle: u32, kind: u32) {
        let started = self.sim_elapsed();
        self.priest_effects.push(PriestEffect { handle, kind, started });
    }

    pub(crate) fn prune_priest_effects(&mut self) {
        if self.priest_effects.is_empty() { return; }
        let now = self.sim_previous_elapsed();
        let actors = &self.ents;
        self.priest_effects.retain(|effect| {
            let actor = actors[(effect.handle & 0xffff) as usize];
            let lifetime = if effect.kind == FX_RALLY_HEALED { 1.0 } else { 2.0 };
            actor.alive && u32::from(actor.gen) == effect.handle >> 16 && now - effect.started < lifetime
        });
    }

    pub(crate) fn power_unit_can_garrison(&self, unit: DefId, host: DefId) -> bool {
        self.powers.is_some() && (host == FORTRESS || unit == PRIEST && host == TEMPLE ||
            (DEFS[host as usize].has(F_TRANSPORT) && matches!(DEFS[host as usize].dom, SEA | AIR)))
    }

    pub(crate) fn configure_priest_skirmish(&mut self) -> bool {
        if self.net || self.bench != 0 || self.tick != 0 || self.powers.is_some() { return false; }
        let mut state = PowerState::default();
        for index in 0..self.high {
            if self.ents[index].alive { state.register(self.h(index)); }
        }
        self.powers = Some(state);
        true
    }

    pub(crate) fn tick_unit_reserve(&mut self, index: usize) {
        let actor = self.ents[index];
        let Some(seconds) = recharge_seconds(actor.def, self.military_leader_powers.is_some()) else { return; };
        if actor.hp <= 0.0 { return; }
        let handle = self.h(index);
        let Some(record) = self.powers.as_ref().and_then(|state| state.actor(handle)) else { return; };
        let id = (u32::from(actor.team) + 1) << 27 | record.serial;
        if !crate::capture_math25::capture_event_due(self.tick, id, self.sim_step()) { return; }
        let factor = self.reserve_recharge_factor(actor.team);
        let blocked = (self.priest_conversion_active(handle) || self.priest_bless_elapsed(handle, self.sim_elapsed()).is_some()) as u32;
        let updated = update_power_reserve(record.charge, seconds, factor, 0x400, blocked);
        self.powers.as_mut().expect("admitted reserve").live.get_mut(&handle).expect("live reserve owner").charge = updated;
        if actor.def == SPY { self.tick_spy_extraction(handle); }
    }

    fn reserve_recharge_factor(&self, team: u8) -> f32 {
        let mut factor = if self.civilizations[team as usize] == 13 { 0.75 } else { 1.0 };
        for research in [56, 116, 176] {
            if self.has_research(team, research) { factor = accumulate_recharge_factor(factor, 0.85); }
        }
        factor
    }

    pub(crate) fn tick_garrisoned_reserves(&mut self) {
        if self.powers.as_ref().is_none_or(|state| state.cargo.is_empty()) { return; }
        let factors: [f32; 4] = core::array::from_fn(|seat| self.reserve_recharge_factor(seat as u8));
        let step = self.sim_step();
        let military = self.military_leader_powers.is_some();
        let Some(state) = &mut self.powers else { return; };
        for (&(host, slot), reserve) in &mut state.cargo {
            let actor = self.ents[(host & 0xffff) as usize];
            let Some(seconds) = recharge_seconds(actor.cargo[slot as usize], military) else { continue; };
            if !actor.alive || actor.hp <= 0.0 { continue; }
            let id = (u32::from(actor.team) + 1) << 27 | reserve.serial;
            if crate::capture_math25::capture_event_due(self.tick, id, step) {
                reserve.charge = update_power_reserve(reserve.charge, seconds, factors[actor.team as usize], 0x400, 0);
            }
        }
    }

    pub(crate) fn spend_unit_reserve(&mut self, handle: u32, requested: f32) -> f32 {
        let record = self.powers.as_mut().unwrap().live.get_mut(&handle).expect("active special power reserve");
        let delta = requested.min(record.charge);
        record.charge = crate::power42_math::spend_conversion_reserve(record.charge, delta);
        delta
    }

    pub(crate) fn recharge_selected_reserves(&mut self) -> bool {
        if self.powers.is_none() { return false; }
        for (index, actor) in self.ents[..self.high].iter().enumerate() {
            if actor.alive && actor.sel && actor.team == self.local && recharge_seconds(actor.def, self.military_leader_powers.is_some()).is_some() {
                let handle = self.h(index);
                self.powers.as_mut().unwrap().recharge_selected(handle);
            }
        }
        true
    }

    pub(crate) fn restore_priest_power(&mut self, fields: Option<&[u8]>, military: bool) -> Option<()> {
        let Some(data) = fields else { return Some(()); };
        if self.net { return None; }
        let (cursor, live, cargo) = (word(data, 0)?, word(data, 4)?, word(data, 8)?);
        if cursor > SERIAL_MASK || live as usize != self.ents[..self.high].iter().filter(|actor| actor.alive).count()
            || cargo as usize != self.ents[..self.high].iter().filter(|actor| actor.alive).map(|actor| usize::from(actor.ncargo)).sum::<usize>() {
            return None;
        }
        let mut state = PowerState { cursor, ..Default::default() };
        let mut pos = 12;
        for _ in 0..live {
            let handle = word(data, pos)?;
            let def = self.ents[self.get(handle)?].def;
            let record = read_actor(data, pos + 4, def, military)?;
            if state.occupied.insert(record.serial, Some(handle)).is_some() || state.live.insert(handle, record).is_some() { return None; }
            pos += 12;
        }
        for _ in 0..cargo {
            let (host, slot) = (word(data, pos)?, word(data, pos + 4)?);
            let actor = self.ents[self.get(host)?];
            if slot >= u32::from(actor.ncargo) { return None; }
            let record = read_actor(data, pos + 8, actor.cargo[slot as usize], military)?;
            if state.occupied.insert(record.serial, None).is_some() || state.cargo.insert((host, slot), record).is_some() { return None; }
            pos += 16;
        }
        if pos != data.len() { return None; }
        self.powers = Some(state);
        Some(())
    }
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn read_actor(data: &[u8], at: usize, def: DefId, military: bool) -> Option<PowerActor> {
    let serial = word(data, at)?;
    let charge = f32::from_bits(word(data, at + 4)?);
    if serial > SERIAL_MASK || !(0.0..=1.0).contains(&charge) || charge.is_sign_negative() ||
        (recharge_seconds(def, military).is_none() && charge != 0.0) { return None; }
    Some(PowerActor { serial, charge })
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let (len, live, cargo) = (word(data, 8)? as usize, word(data, 16)? as usize, word(data, 20)? as usize);
    if word(data, 4)? != 1 || len == 0 || live > MAXE || cargo > MAXE * crate::defs::CARGO_MAX { return None; }
    let at = HEADER + live * 12 + cargo * 16;
    let end = at.checked_add(len)?;
    if end.checked_add(4)? != data.len() || word(data, end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[at..end], Some(&data[12..at])))
}

