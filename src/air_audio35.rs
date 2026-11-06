//! Local aircraft audio cues. This state is outside simulation hashes and saves.

use std::collections::{BTreeMap, VecDeque};

use crate::air31_defs::mission_aircraft;
use crate::defs::DefId;
use crate::game::{Ent, Game, DT, MAXE, NONE};
use crate::map::tile_of;

const HISTORY: u32 = 16;
const WALK: u8 = 1;
const ATTACK: u8 = 2;
const DEATH: u8 = 3;
const TAKEOFF: u8 = 4;
const PARKED: u8 = 5;

#[derive(Clone, Copy)]
struct Cue {
    tick: u32,
    handle: u32,
    definition: DefId,
    team: u8,
    epoch: u8,
    kind: u8,
    at: f32,
    duration: f32,
    revision: u32,
    tile: usize,
}

impl Cue {
    fn terminal(self) -> bool { matches!(self.kind, DEATH | PARKED) }

    fn mode(self) -> u8 { u8::from(self.kind == DEATH) }
}

#[repr(C)]
pub(crate) struct Current {
    handle: u32,
    definition: u16,
    team: u8,
    epoch: u8,
    kind: u8,
    mode: u8,
    reserved: u16,
    revision: u32,
    at: f32,
    duration: f32,
    x: f32,
    y: f32,
    z: f32,
    hp_ratio: f32,
}

#[repr(C)]
pub(crate) struct Transition {
    revision: u32,
    tick: u32,
    handle: u32,
    definition: u16,
    epoch: u8,
    kind: u8,
    duration: f32,
    mode: u8,
    reserved: [u8; 3],
}

const _: [(); 40] = [(); std::mem::size_of::<Current>()];
const _: [(); 24] = [(); std::mem::size_of::<Transition>()];

#[derive(Default)]
pub(crate) struct State {
    revision: u32,
    live: BTreeMap<u32, Cue>,
    history: VecDeque<Cue>,
    shots: BTreeMap<usize, (u32, f32)>,
    pub(crate) current: Vec<Current>,
    pub(crate) transitions: Vec<Transition>,
}

impl State {
    fn queue(&mut self, mut cue: Cue) {
        self.revision = self.revision.wrapping_add(1);
        cue.revision = self.revision;
        if cue.terminal() {
            self.live.remove(&cue.handle);
        } else {
            self.live.insert(cue.handle, cue);
        }
        if self.history.len() == 2 * MAXE { self.history.pop_front(); }
        self.history.push_back(cue);
    }

    pub(crate) fn forget(&mut self, handle: u32) {
        if self.live.remove(&handle).is_some() {
            self.history.retain(|cue| cue.handle != handle || cue.terminal());
        }
    }

    pub(crate) fn prune(&mut self, tick: u32) {
        while self.history.front().is_some_and(|cue| tick.wrapping_sub(cue.tick) > HISTORY) {
            self.history.pop_front();
        }
    }

    pub(crate) fn shot(&self, index: usize) -> u32 {
        self.shots.get(&index).map_or(0, |row| row.0)
    }

    pub(crate) fn shot_duration(&self, index: usize) -> f32 {
        self.shots.get(&index).map_or(0.0, |row| row.1)
    }

    pub(crate) fn clear_shots(&mut self) {
        self.shots.clear();
    }
}

impl Game {
    fn air_audio_cue(&self, index: usize, actor: &Ent, kind: u8, at: f32, duration: f32) -> Cue {
        Cue { tick: self.tick, handle: self.h(index), definition: actor.def, team: actor.team,
            epoch: self.players[actor.team as usize].epoch, kind, at, duration,
            revision: 0, tile: tile_of(actor.x, actor.y) }
    }

    fn air_audio_visible(&self, index: usize) -> bool {
        self.ents[index].alive && mission_aircraft(self.ents[index].def) && self.can_see(self.local, index)
    }

    pub(crate) fn observe_air_motion_audio(&mut self, index: usize, actor: &Ent) {
        if !self.air_missions_enabled || !mission_aircraft(actor.def) { return; }
        let handle = self.h(index);
        if actor.hp <= 0.0 || !self.air_audio_visible(index) || !self.aircraft.is_active(handle) {
            self.air_audio.forget(handle);
            return;
        }
        let now = self.tick as f32 * DT;
        let kind = if self.takeoff_start_tick(handle) == NONE { WALK } else { TAKEOFF };
        let epoch = self.players[actor.team as usize].epoch;
        if self.air_audio.live.get(&handle).is_some_and(|cue| {
            cue.epoch == epoch && (cue.kind == kind ||
                (cue.kind == ATTACK && kind == WALK && !self.air_audio_returning(handle) && now < cue.at + cue.duration))
        }) { return; }
        let cue = self.air_audio_cue(index, actor, kind, now, 0.0);
        self.air_audio.queue(cue);
    }

    pub(crate) fn observe_air_attack_audio(&mut self, index: usize, actor: &Ent, now: f32, reload: f32) {
        if !self.air_missions_enabled || !self.air_audio_visible(index) || actor.hp <= 0.0 { return; }
        let duration = (f64::from(reload) + 3.0 * f64::from(DT)) as f32;
        let cue = self.air_audio_cue(index, actor, ATTACK, now, duration);
        self.air_audio.queue(cue);
    }

    pub(crate) fn observe_air_death_audio(&mut self, index: usize) {
        if !self.air_missions_enabled || !self.air_audio_visible(index) { return; }
        let cue = self.air_audio_cue(index, &self.ents[index], DEATH, self.tick as f32 * DT, 0.0);
        self.air_audio.queue(cue);
    }

    pub(crate) fn observe_air_parking_audio(&mut self, index: usize) {
        let handle = self.h(index);
        if !self.air_audio.live.contains_key(&handle) || !self.air_audio_visible(index) { return; }
        let cue = self.air_audio_cue(index, &self.ents[index], PARKED, self.tick as f32 * DT, 0.0);
        self.air_audio.queue(cue);
    }

    pub(crate) fn mark_air_shot(&mut self, index: usize, previous_len: usize) {
        if self.events.len() != previous_len + 3 { return; }
        let Some(cue) = self.air_audio.live.get(&self.h(index)).filter(|cue| cue.kind == ATTACK) else { return; };
        let value = u32::from(cue.definition) | (u32::from(cue.epoch) << 16);
        self.air_audio.shots.insert(previous_len / 3, (value, cue.duration));
    }

    pub(crate) fn prepare_air_audio(&mut self, alpha: f32) {
        let alpha = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.0 };
        self.air_audio.prune(self.tick);
        let mut current = std::mem::take(&mut self.air_audio.current);
        let mut transitions = std::mem::take(&mut self.air_audio.transitions);
        current.clear();
        transitions.clear();
        if self.air_missions_enabled {
            for i in 0..self.high {
                let actor = &self.ents[i];
                if actor.hp <= 0.0 || !self.air_audio_visible(i) || !self.aircraft.is_active(self.h(i)) { continue; }
                let handle = self.h(i);
                let epoch = self.players[actor.team as usize].epoch;
                let cue = self.air_audio.live.get(&handle).copied().filter(|cue| cue.epoch == epoch).unwrap_or_else(|| {
                    let start = self.takeoff_start_tick(handle);
                    let (kind, at) = if start != NONE { (TAKEOFF, start) }
                    else if self.takeoff.rolling(handle) { (TAKEOFF, self.tick + 1) }
                    else { (WALK, self.tick) };
                    self.air_audio_cue(i, actor, kind, at as f32 * DT, 0.0)
                });
                let x = actor.px + (actor.x - actor.px) * alpha;
                let y = actor.py + (actor.y - actor.py) * alpha;
                let z = self.aircraft_height(handle, alpha, self.map.height(x, y).max(0.0));
                let hp_ratio = (actor.hp / self.max_hp(actor.team, actor.def)).clamp(0.0, 1.0);
                current.push(Current { handle, definition: actor.def, team: actor.team, epoch,
                    kind: cue.kind, mode: cue.mode(), reserved: 0, revision: cue.revision,
                    at: cue.at, duration: cue.duration, x, y, z, hp_ratio });
            }
            for &cue in &self.air_audio.history {
                let visible = if cue.terminal() {
                    cue.team == self.local || self.real_fog(self.local, cue.tile) == 255
                } else {
                    self.get(cue.handle).is_some_and(|i| self.air_audio_visible(i) && self.aircraft.is_active(cue.handle))
                };
                if visible {
                    transitions.push(Transition { revision: cue.revision, tick: cue.tick, handle: cue.handle,
                        definition: cue.definition, epoch: cue.epoch, kind: cue.kind,
                        duration: cue.duration, mode: cue.mode(), reserved: [0; 3] });
                }
            }
        }
        self.air_audio.current = current;
        self.air_audio.transitions = transitions;
    }
}
