//! Runway acceleration and terrain-relative elevation for released aircraft.

use std::collections::BTreeMap;

use crate::air31_defs::{flight_profile, mission_aircraft};
use crate::game::{Ent, Game, MAXE, NONE};
use crate::map::{Map, N};

pub(crate) const SAVE_FLAG: u32 = 1 << 8;
const MAGIC: u32 = u32::from_le_bytes(*b"TFO1");
const FIXED_VERSION: u32 = 1;
const SOURCE_VERSION: u32 = 2;
const FIXED_RECORD: usize = 28;
const SOURCE_RECORD: usize = FIXED_RECORD + 4;
const TAG_SCALE: f64 = f32::from_bits(0x3ce7b46a) as f64;
const TAG_Z: f64 = f32::from_bits(0x3f7fdb82) as f64;
const TAKEOFF_FRACTION: f64 = f32::from_bits(0x3f666666) as f64;

#[derive(Clone, Copy)]
struct Flight {
    start: u32,
    fly_at: u32,
    source_start: Option<f32>,
    speed: f32,
    height: f32,
    previous_height: f32,
    clearance: f32,
}

#[derive(Default)]
pub(crate) struct State(BTreeMap<u32, Flight>);

pub(crate) struct Save<'a> {
    fields: &'a [u8],
    source_clock: bool,
}

impl Flight {
    fn words(self, handle: u32) -> [u32; 7] {
        [handle, self.start, self.fly_at, self.speed.to_bits(), self.height.to_bits(),
            self.previous_height.to_bits(), self.clearance.to_bits()]
    }

    fn rolling(self) -> bool { self.fly_at == NONE }

    fn source_start_word(self) -> u32 { self.source_start.map_or(NONE, f32::to_bits) }
}

fn accelerate(speed: f32, target: f32, dt: f32) -> f32 {
    // 007D6FBC stores target² / 8 as f32, then 0085E320 updates the
    // scalar MotionData speed with a single store after the x87 multiply/add.
    let acceleration = (f64::from(target) * f64::from(target) * 0.125) as f32;
    let delta = f64::from(acceleration) * f64::from(dt);
    let speed = f64::from(speed);
    let target = f64::from(target);
    if speed < target { (speed + delta).min(target) as f32 }
    else { (speed - delta).max(target) as f32 }
}

fn ground(map: &Map, x: f32, y: f32) -> f32 {
    // Own generated maps have a height field and no source cliff overrides.
    map.height(x, y).max(0.0)
}

fn terrain_ahead(map: &Map, actor: &Ent) -> f32 {
    // 007D8650 samples the current XY, 1 and 2 ahead, and two side
    // samples. Its second side uses the first side's signed sine twice.
    let (sin, cos) = f64::from(actor.rot).sin_cos();
    let (sin, cos) = (f64::from(sin as f32), f64::from(cos as f32));
    let (side_sin, side_cos) = (f64::from(actor.rot) + f64::from(std::f32::consts::FRAC_PI_4)).sin_cos();
    let (side_sin, side_cos) = (f64::from(side_sin as f32), f64::from(side_cos as f32));
    [(0.0, 0.0), (cos, sin), (cos * 2.0, sin * 2.0), (side_cos, side_sin), (side_sin, -side_sin)]
        .into_iter()
        .map(|(dx, dy)| ground(map, (f64::from(actor.x) + dx) as f32, (f64::from(actor.y) + dy) as f32))
        .fold(0.0, f32::max)
}

fn approach_height(height: f32, target: f32, rate: f32, dt: f32) -> f32 {
    // 007D89C6..007D8A7B stores the difference before testing the two
    // distance bands. The scaled step stays extended through clamping.
    let error = (target - height) as f64;
    let scale = if error.abs() > 2.5 { 7.0 } else if error.abs() > 0.5 { 3.0 } else { 1.0 };
    let step = f64::from((f64::from(dt) * f64::from(rate)) as f32) * scale;
    (f64::from(height) + error.clamp(-step, step)) as f32
}

impl State {
    pub(crate) fn clear(&mut self, handle: u32) { self.0.remove(&handle); }

    pub(crate) fn rolling(&self, handle: u32) -> bool {
        self.0.get(&handle).is_some_and(|flight| flight.rolling())
    }

    pub(crate) fn hash(&self, source_clock: bool, mut mix: impl FnMut(u32)) {
        mix(MAGIC);
        mix(self.0.len() as u32);
        for (&handle, &flight) in &self.0 {
            for word in flight.words(handle) { mix(word); }
            if source_clock { mix(flight.source_start_word()); }
        }
    }
}

impl Game {
    pub(crate) fn configure_peer_air_takeoff(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() ||
            !self.inbox.is_empty() || (enabled && !self.air_runway_enabled) {
            return false;
        }
        self.set_air_takeoff_enabled(enabled);
        true
    }

    pub(crate) fn set_air_takeoff_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.air_landing_enabled = false;
            self.air_carrier_enabled = false;
        }
        if self.air_takeoff_enabled != enabled {
            self.air_takeoff_enabled = enabled;
            for team in 0..self.nplayers {
                self.rebuild_modifiers(team as u8, false);
            }
        }
    }

    pub(crate) fn begin_takeoff(&mut self, index: usize, airport: &Ent, deck_height: Option<f32>) {
        if !self.air_takeoff_enabled { return; }
        let handle = self.h(index);
        // 008741E0 rounds the scaled Z before adding the Airport's world Z.
        let height = if let Some(height) = deck_height { height }
        else if airport.def == crate::carrier36_defs::CARRIER {
            self.carrier_deck_offset(airport, 0)
        } else { ground(&self.map, airport.x, airport.y) + (TAG_Z * TAG_SCALE) as f32 };
        let actor = &mut self.ents[index];
        let clearance = height - ground(&self.map, actor.x, actor.y);
        (actor.vx, actor.vy) = (0.0, 0.0);
        self.takeoff.0.insert(handle, Flight {
            start: self.tick + 1, fly_at: NONE, source_start: None, speed: 0.0, height,
            previous_height: height, clearance,
        });
    }

    pub(crate) fn advance_takeoff(&mut self, index: usize, actor: &mut Ent) -> bool {
        let handle = self.h(index);
        let dt = self.sim_step();
        let source_now = self.source_clock.map(|clock| clock.elapsed);
        let Some(flight) = self.takeoff.0.get_mut(&handle) else { return false; };
        if self.tick == flight.start { flight.source_start = source_now; }
        let target = self.modifiers[actor.team as usize][actor.def as usize][2];
        if flight.rolling() && target > 0.0 && f64::from(flight.speed) > f64::from(target) * TAKEOFF_FRACTION {
            // 007D6D80 requested Takeoff at entry, before changing mode.
            flight.fly_at = self.tick + 1;
        }
        let rolling = flight.rolling();
        let epoch = self.players[actor.team as usize].epoch;
        let profile = flight_profile(actor.def, epoch);
        let rate = profile.map_or(1.0, |row| row.elevation_rate);
        let (clearance, rate) = if rolling { (flight.clearance, 2.0 * rate) }
            else { (profile.map_or(flight.clearance, |row| row.height), rate) };
        flight.previous_height = flight.height;
        flight.height = approach_height(flight.height, terrain_ahead(&self.map, actor) + clearance, rate, dt);
        if rolling {
            flight.speed = accelerate(flight.speed, target, dt);
            let (sin, cos) = f64::from(actor.rot).sin_cos();
            actor.vx = cos as f32 * flight.speed;
            actor.vy = sin as f32 * flight.speed;
            // 0085E6C0 stores the Y displacement before adding world Y.
            let x = (f64::from(actor.x) + f64::from(actor.vx) * f64::from(dt)) as f32;
            let y = actor.y + actor.vy * dt;
            actor.x = x.clamp(0.0, N as f32 - 1.0);
            actor.y = y.clamp(0.0, N as f32 - 1.0);
        }
        rolling
    }

    pub(crate) fn takeoff_start_tick(&self, handle: u32) -> u32 {
        self.takeoff.0.get(&handle)
            .filter(|flight| flight.start <= self.tick && self.tick < flight.fly_at)
            .map_or(NONE, |flight| flight.start)
    }

    pub(crate) fn takeoff_start_seconds(&self, handle: u32) -> Option<f32> {
        let start = self.takeoff_start_tick(handle);
        if start == NONE { return None; }
        if self.source_clock.is_some() { self.takeoff.0.get(&handle)?.source_start }
        else { Some(start as f32 * crate::game::DT) }
    }

    pub(crate) fn aircraft_height(&self, handle: u32, alpha: f32, ground: f32) -> f32 {
        self.takeoff_height(handle, alpha).unwrap_or(3.4 + ground * 0.6)
    }

    pub(crate) fn takeoff_height(&self, handle: u32, alpha: f32) -> Option<f32> {
        self.takeoff.0.get(&handle).map(|flight| {
            if alpha == 1.0 { flight.height }
            else { flight.previous_height + (flight.height - flight.previous_height) * alpha }
        })
    }

    pub(crate) fn follow_air_height(&mut self, handle: u32, actor: &Ent, clearance: f32, rate: f32) {
        let dt = self.sim_step();
        if let Some(flight) = self.takeoff.0.get_mut(&handle) {
            flight.previous_height = flight.height;
            flight.height = approach_height(flight.height, terrain_ahead(&self.map, actor) + clearance, rate, dt);
        }
    }

    pub(crate) fn wrap_takeoff_save(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() || self.takeoff.0.is_empty() { return core; }
        let source_clock = self.source_clock.is_some();
        let (version, record) = if source_clock { (SOURCE_VERSION, SOURCE_RECORD) } else { (FIXED_VERSION, FIXED_RECORD) };
        let length = 20 + record * self.takeoff.0.len();
        let mut out = Vec::with_capacity(length + core.len());
        for word in [MAGIC, version, length as u32, self.takeoff.0.len() as u32] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for (&handle, &flight) in &self.takeoff.0 {
            for word in flight.words(handle) { out.extend_from_slice(&word.to_le_bytes()); }
            if source_clock { out.extend_from_slice(&flight.source_start_word().to_le_bytes()); }
        }
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out.extend(core);
        out
    }

    pub(crate) fn restore_takeoff_save(&mut self, save: Option<Save<'_>>) -> Option<()> {
        let Some(save) = save else { return Some(()); };
        if !self.air_takeoff_enabled || save.source_clock != self.source_clock.is_some() { return None; }
        let record = if save.source_clock { SOURCE_RECORD } else { FIXED_RECORD };
        let elapsed = self.sim_elapsed();
        let mut last = None;
        for row in save.fields.chunks_exact(record) {
            let read = |i: usize| u32::from_le_bytes(row[4 * i..4 * i + 4].try_into().unwrap());
            let handle = read(0);
            let index = self.get(handle)?;
            let actor = &self.ents[index];
            let source_start = if save.source_clock && read(7) != NONE { Some(f32::from_bits(read(7))) } else { None };
            let flight = Flight {
                start: read(1), fly_at: read(2), source_start, speed: f32::from_bits(read(3)),
                height: f32::from_bits(read(4)), previous_height: f32::from_bits(read(5)),
                clearance: f32::from_bits(read(6)),
            };
            if last.is_some_and(|previous| handle <= previous) || actor.hp <= 0.0 ||
                !mission_aircraft(actor.def) || !self.aircraft.is_active(handle) ||
                flight.start == 0 || flight.start > self.tick.saturating_add(1) ||
                (!flight.rolling() && (flight.fly_at <= flight.start || flight.fly_at > self.tick.saturating_add(1))) ||
                ![flight.speed, flight.height, flight.previous_height, flight.clearance].into_iter().all(f32::is_finite) ||
                flight.speed < 0.0 {
                return None;
            }
            if save.source_clock && ((flight.start <= self.tick) != source_start.is_some() ||
                source_start.is_some_and(|at| !at.is_finite() || at <= 0.0 || at > elapsed)) {
                return None;
            }
            last = Some(handle);
            self.takeoff.0.insert(handle, flight);
        }
        Some(())
    }
}

pub(crate) fn split_save(buf: &[u8]) -> Option<(&[u8], Option<Save<'_>>)> {
    let word = |start| Some(u32::from_le_bytes(buf.get(start..start + 4)?.try_into().ok()?));
    if word(0) != Some(MAGIC) { return Some((buf, None)); }
    let length = word(8)? as usize;
    let count = word(12)? as usize;
    let (source_clock, record) = match word(4)? {
        FIXED_VERSION => (false, FIXED_RECORD),
        SOURCE_VERSION => (true, SOURCE_RECORD),
        _ => return None,
    };
    if count == 0 || count > MAXE || length != 20 + record * count || length >= buf.len() {
        return None;
    }
    let end = length - 4;
    if word(end)? != crate::pack8_save::checksum(&buf[..end]) { return None; }
    Some((&buf[length..], Some(Save { fields: &buf[16..end], source_clock })))
}
