//! Fixed Airport approach, descent and runway rollout for released aircraft.

use std::collections::BTreeMap;

use crate::air31_defs::{flight_profile, Flight as Profile};
use crate::aircraft_steering::{self as steering, Controls, Motion};
use crate::carrier36_defs::CARRIER;
use crate::game::{Ent, Game, MAXE};
use crate::landing_geometry::{self as geometry, Aircraft, Approach, Point, Runway};
use crate::map::N;

pub(crate) const SAVE_FLAG: u32 = 1 << 9;
const MAGIC: u32 = u32::from_le_bytes(*b"LND1");
const VERSION: u32 = 1;
const WORDS: usize = 19;
const RECORD: usize = 4 * WORDS;
const PATH_POINTS: usize = 7;
const RADIANS: f64 = f32::from_bits(0x3c8e_fa35) as f64;
const YAW_ACCEL_TIME: f64 = 0.1f32 as f64;

#[derive(Default)]
pub(crate) struct State(BTreeMap<u32, Landing>);

#[derive(Clone, Copy)]
struct Landing {
    home: u32,
    speed: f32,
    angular_rate: f32,
    phase: Phase,
}

#[derive(Clone, Copy)]
enum Phase {
    Bank { center: Approach, height: f32 },
    Runway { path: [Point; PATH_POINTS], cursor: usize, mode: u8, unreachable: bool },
    Complete,
}

// Runtime display coordinates are reflected relative to source horizontal Y.
// Do the side tests in source axes so exact collinear and zero-rate ties keep
// the source handedness.
fn source_xy([x, y]: Point) -> Point { [x, -y] }

impl Landing {
    fn start(home: u32, runway: Runway, actor: Aircraft, height: f32) -> Self {
        Self { home, speed: actor.velocity[0].hypot(actor.velocity[1]), angular_rate: 0.0,
            phase: Phase::Bank { center: geometry::approach(runway, actor), height } }
    }

    fn update(&mut self, runway: Runway, actor: Aircraft, profile: &Profile, target: f32,
              current_height: f32, deck_offset: f32, moving_deck: bool, dt: f32) -> (Motion, f32, f32) {
        let raw_turn = f64::from(profile.angle_degrees_per_second) * RADIANS;
        let max_turn = (f64::from(target) / f64::from(profile.speed) * f64::from(raw_turn as f32)) as f32;
        let angular_acceleration = (raw_turn / YAW_ACCEL_TIME) as f32;
        let margin = (f64::from(max_turn) / f64::from(angular_acceleration) * f64::from(target)) as f32;
        let control = Controls { speed: target, acceleration: profile.speed, max_turn,
            angular_acceleration, distance_margin: margin, arrival_angle: 0.0 };
        let motion = Motion { speed: self.speed, angular_rate: self.angular_rate, velocity: actor.velocity };
        if let Phase::Bank { center, .. } = self.phase {
            if !moving_deck && geometry::aligned_for_landing(center, motion.angular_rate, max_turn) {
                let path = geometry::landing_path(runway, actor, target, max_turn)
                    .expect("the source hosts have distinct runway markers");
                self.phase = Phase::Runway { path, cursor: 1, mode: 4, unreachable: false };
            }
        }
        let (motion, clearance, elevation_rate) = match &mut self.phase {
            Phase::Bank { center, height } => {
                let rate = steering::approach_turn(actor.position, actor.heading, center.point,
                    *height - current_height, center.turn_left, target, max_turn);
                let motion = steering::hold(actor.heading, motion, Controls { speed: target * 0.8, ..control }, rate, dt);
                (motion, profile.height, profile.elevation_rate)
            }
            Phase::Runway { path, cursor, mode, unreachable } => {
                let remaining = path.len() - *cursor;
                let control = Controls { speed: geometry::landing_speed(target, *mode, remaining), ..control };
                let goal = path[*cursor];
                let (allowed, next_flag) = steering::fixed_goal_gate(actor.position, actor.heading, goal, control, *unreachable);
                *unreachable = next_flag;
                let (motion, arrived) = if allowed {
                    steering::steer(actor.position, actor.heading, goal, motion, control, dt)
                } else {
                    (steering::hold(actor.heading, motion, control, 0.0, dt), false)
                };
                if arrived {
                    if remaining == 1 {
                        self.phase = Phase::Complete;
                        return (motion, 0.0, 0.0);
                    }
                    *cursor += 1;
                }
                let [x, y] = path[*cursor];
                let distance = (f64::from(x) - f64::from(actor.position[0]))
                    .hypot(f64::from(y) - f64::from(actor.position[1]));
                let (next_mode, clearance) = geometry::landing_height(profile.height, deck_offset, path.len() - *cursor, distance)
                    .expect("the landing cursor still has a target");
                *mode = next_mode;
                (motion, clearance, 2.0 * profile.elevation_rate)
            }
            Phase::Complete => return (motion, 0.0, 0.0),
        };
        self.speed = motion.speed;
        self.angular_rate = motion.angular_rate;
        (motion, clearance, elevation_rate)
    }

    fn words(self, handle: u32) -> [u32; WORDS] {
        let mut words = [0; WORDS];
        words[..4].copy_from_slice(&[handle, self.home, self.speed.to_bits(), self.angular_rate.to_bits()]);
        match self.phase {
            Phase::Bank { center, height } => {
                words[4] = 3 | (u32::from(center.turn_left) << 8);
                words[5..8].copy_from_slice(&[center.point[0].to_bits(), center.point[1].to_bits(), height.to_bits()]);
            }
            Phase::Runway { path, cursor, mode, unreachable } => {
                words[4] = u32::from(mode) | (u32::from(unreachable) << 8) | ((cursor as u32) << 16);
                for (i, point) in path.into_iter().flatten().enumerate() { words[5 + i] = point.to_bits(); }
            }
            Phase::Complete => {}
        }
        words
    }
}

impl State {
    pub(crate) fn clear(&mut self, handle: u32) { self.0.remove(&handle); }

    pub(crate) fn clear_host(&mut self, home: u32) { self.0.retain(|_, row| row.home != home); }

    pub(crate) fn status(&self, handle: u32, field: u32) -> u32 {
        let Some(row) = self.0.get(&handle) else { return 0; };
        match (field, row.phase) {
            (0, Phase::Bank { .. }) => 3,
            (0, Phase::Runway { mode, .. }) => mode.into(),
            (1, Phase::Runway { cursor, .. }) => cursor as u32,
            (2, _) => row.home,
            _ => 0,
        }
    }

    pub(crate) fn complete(&self, handle: u32, home: u32) -> bool {
        self.0.get(&handle).is_some_and(|row| row.home == home && matches!(row.phase, Phase::Complete))
    }

    pub(crate) fn hash(&self, mut mix: impl FnMut(u32)) {
        mix(MAGIC);
        mix(self.0.len() as u32);
        for (&handle, &row) in &self.0 {
            for word in row.words(handle) { mix(word); }
        }
    }
}

impl Game {
    pub(crate) fn configure_peer_air_landing(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() ||
            !self.inbox.is_empty() || (enabled && !self.air_takeoff_enabled) { return false; }
        self.air_landing_enabled = enabled;
        if !enabled { self.air_carrier_enabled = false; }
        true
    }

    pub(crate) fn use_runway_landing(&self, index: usize) -> bool {
        if !self.air_landing_enabled || self.takeoff.rolling(self.h(index)) { return false; }
        let actor = &self.ents[index];
        flight_profile(actor.def, self.players[actor.team as usize].epoch).is_some() &&
            self.modifiers[actor.team as usize][actor.def as usize][2] > 0.0 &&
            self.takeoff_height(self.h(index), 1.0).is_some()
    }

    pub(crate) fn advance_landing(&mut self, index: usize, actor: &mut Ent) -> bool {
        if !self.air_landing_enabled { return false; }
        let handle = self.h(index);
        let Some(home) = self.air_landing_home(handle).filter(|_| self.use_runway_landing(index)) else {
            self.landing.clear(handle);
            return false;
        };
        let host_index = self.get(home).expect("returning plane has a live host");
        let host = &self.ents[host_index];
        let carrier = host.def == CARRIER;
        let (markers, deck_offset) = if carrier {
            // Actor updates run in slot order. Sample the deck clock of the
            // same host pose whose XY and heading the returning plane sees.
            let elapsed = if host_index < index { self.sim_elapsed() } else { self.sim_previous_elapsed() };
            self.carrier_deck(host_index, elapsed).map_or_else(
                || (self.air_host_markers(host), self.carrier_deck_offset(host, 1)),
                |deck| (deck.markers(), deck.landing_clearance()),
            )
        } else { (self.air_host_markers(host), 0.0) };
        let [first, second] = markers.map(source_xy);
        let moving_deck = carrier && (host.vx != 0.0 || host.vy != 0.0 || host.rot != host.prot);
        let runway = Runway { first, second };
        let pose = Aircraft { position: source_xy([actor.x, actor.y]), velocity: source_xy([actor.vx, actor.vy]),
            heading: (-actor.rot).rem_euclid(std::f32::consts::TAU), turn_rate: 0.0 };
        let height = self.takeoff_height(handle, 1.0).expect("landing requires tracked altitude");
        let dt = self.sim_step();
        let flight = self.landing.0.entry(handle).or_insert_with(|| Landing::start(home, runway, pose, height));
        if flight.home != home { *flight = Landing::start(home, runway, pose, height); }
        let profile = flight_profile(actor.def, self.players[actor.team as usize].epoch).unwrap();
        let speed = self.modifiers[actor.team as usize][actor.def as usize][2];
        let pose = Aircraft { turn_rate: flight.angular_rate, ..pose };
        let (motion, clearance, rate) = flight.update(runway, pose, profile, speed, height, deck_offset, moving_deck, dt);
        if matches!(flight.phase, Phase::Complete) { return true; }
        self.follow_air_height(handle, actor, clearance, rate);
        let (position, heading) = steering::integrate(pose.position, pose.heading, motion, dt);
        let [x, y] = source_xy(position);
        let [width, height] = self.map.simulation_dimensions().map(|side| side as f32);
        actor.x = x.clamp(0.0, width - 1.0);
        actor.y = y.clamp(0.0, height - 1.0);
        actor.rot = -heading;
        [actor.vx, actor.vy] = source_xy(motion.velocity);
        true
    }

    pub(crate) fn wrap_landing_save(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() || self.landing.0.is_empty() { return core; }
        let length = 20 + RECORD * self.landing.0.len();
        let mut out = Vec::with_capacity(length + core.len());
        for word in [MAGIC, VERSION, length as u32, self.landing.0.len() as u32] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for (&handle, &row) in &self.landing.0 {
            for word in row.words(handle) { out.extend_from_slice(&word.to_le_bytes()); }
        }
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out.extend(core);
        out
    }

    pub(crate) fn restore_landing_save(&mut self, fields: Option<&[u8]>) -> Option<()> {
        let Some(fields) = fields else { return Some(()); };
        if !self.air_landing_enabled { return None; }
        let mut previous = None;
        for row in fields.chunks_exact(RECORD) {
            let word = |i: usize| u32::from_le_bytes(row[4 * i..4 * i + 4].try_into().unwrap());
            let float = |i| f32::from_bits(word(i));
            let (handle, home, flags) = (word(0), word(1), word(4));
            let index = self.get(handle)?;
            let mode = flags as u8;
            let bit = flags & (1 << 8) != 0;
            let cursor = (flags >> 16) as usize;
            if previous.is_some_and(|last| handle <= last) || self.air_landing_home(handle) != Some(home) ||
                !self.use_runway_landing(index) || flags & 0x0000_fe00 != 0 ||
                !float(2).is_finite() || !(0.0..=128.0).contains(&float(2)) ||
                !float(3).is_finite() || float(3).abs() > 128.0 { return None; }
            let phase = match mode {
                3 if cursor == 0 && (5..8).all(|i| float(i).is_finite()) && (8..WORDS).all(|i| word(i) == 0) => {
                    Phase::Bank { center: Approach { point: [float(5), float(6)], turn_left: bit }, height: float(7) }
                }
                4 | 5 if (1..PATH_POINTS).contains(&cursor) && (mode == 5) == (cursor == PATH_POINTS - 1) &&
                    (5..WORDS).all(|i| float(i).is_finite() && float(i).abs() < 3.0 * N as f32) => {
                    Phase::Runway { path: std::array::from_fn(|i| [float(5 + i * 2), float(6 + i * 2)]),
                        cursor, mode, unreachable: bit }
                }
                _ => return None,
            };
            self.landing.0.insert(handle, Landing { home, speed: float(2), angular_rate: float(3), phase });
            previous = Some(handle);
        }
        Some(())
    }
}

pub(crate) fn split_save(buf: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    let word = |start| Some(u32::from_le_bytes(buf.get(start..start + 4)?.try_into().ok()?));
    if word(0) != Some(MAGIC) { return Some((buf, None)); }
    let length = word(8)? as usize;
    let count = word(12)? as usize;
    if word(4)? != VERSION || count == 0 || count > MAXE || length != 20 + RECORD * count || length >= buf.len() {
        return None;
    }
    let end = length - 4;
    if word(end)? != crate::pack8_save::checksum(&buf[..end]) { return None; }
    Some((&buf[length..], Some(&buf[16..end])))
}
