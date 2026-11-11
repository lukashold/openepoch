//! Foot contact state around the native final-waypoint steering and commit.
use super::deer_foot247::{dot, maximum_yaw_rate, steer_foot_with_limits, stock_foot_limits, store, wrap_positive,
    FootLimits, FootMotion, FootStep, Rounding, DEER_SPEED, PI, TAU};

const MINIMUM_AXIS_NUDGE: f32 = f32::from_bits(0x3b44_9ba6);
const PROGRESS_THRESHOLD: f64 = 0.003;
const CONTACT_YAW_FACTOR: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FootContact {
    pub last_block: f32,
    pub anchor_time: f32,
    pub anchor: [f32; 2],
}

impl Default for FootContact {
    fn default() -> Self {
        Self { last_block: -1.0, anchor_time: -1.0, anchor: [0.0; 2] }
    }
}

impl FootContact {
    pub fn words(self) -> [f32; 4] {
        [self.last_block, self.anchor_time, self.anchor[0], self.anchor[1]]
    }

    pub fn read(words: [f32; 4], now: f32) -> Option<Self> {
        let [last_block, anchor_time, x, y] = words;
        let valid_time = |time: f32| time == -1.0 || (0.0..=now).contains(&time);
        if !valid_time(last_block) || !valid_time(anchor_time) || (last_block < 0.0) != (anchor_time < 0.0)
            || !x.is_finite() || !y.is_finite() { return None; }
        Some(Self { last_block, anchor_time, anchor: [x, y] })
    }

    fn note_progress(&mut self, xy: [f32; 2], velocity: [f32; 2], direction: [f32; 2], now: f32, rounding: Rounding) -> bool {
        if self.anchor_time <= 0.0 { return false; }
        let dx = rounding.sub(f64::from(xy[0]), f64::from(self.anchor[0]));
        let dy = rounding.sub(f64::from(xy[1]), f64::from(self.anchor[1]));
        let along = rounding.add(rounding.mul(dx, f64::from(direction[0])), rounding.mul(dy, f64::from(direction[1])));
        if along > PROGRESS_THRESHOLD || dot(velocity, direction, rounding) < 0.0 {
            self.anchor_time = now;
            self.anchor = xy;
        }
        true
    }
}

pub(super) fn advance(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, now: f32, motion: FootMotion,
    contact: &mut FootContact, rounding: Rounding, passable: impl FnMut([f32; 2]) -> bool,
) -> FootStep {
    advance_at_speed(xy, yaw, target, dt, now, motion, contact, rounding, DEER_SPEED, passable)
}

pub(super) fn advance_at_speed(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, now: f32, motion: FootMotion,
    contact: &mut FootContact, rounding: Rounding, speed: f32, passable: impl FnMut([f32; 2]) -> bool,
) -> FootStep {
    advance_with_limits(xy, yaw, target, dt, now, motion, contact, rounding, speed, stock_foot_limits(speed, rounding), passable)
}

pub(super) fn advance_with_limits(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, now: f32, motion: FootMotion,
    contact: &mut FootContact, rounding: Rounding, speed: f32, limits: FootLimits, mut passable: impl FnMut([f32; 2]) -> bool,
) -> FootStep {
    let mut step = steer_foot_with_limits(xy, yaw, target, dt, motion, rounding, speed, limits,
        |direction| contact.note_progress(xy, motion.velocity, direction, now, rounding));
    if step.xy != xy && !passable(step.xy) {
        if step.motion.velocity != [0.0; 2] {
            if contact.anchor_time <= 0.0 {
                contact.anchor_time = now;
                contact.anchor = xy;
            }
            contact.last_block = now;
            step.xy = slide(xy, step.xy, rounding, &mut passable);
        } else {
            step.xy = xy;
        }
    }
    if contact.anchor_time > 0.0 && step.motion.speed != 0.0 {
        step.motion.yaw_rate = 0.0;
        step.yaw = contact_yaw(xy, step.xy, yaw, dt, rounding);
    }
    if contact.anchor_time > 0.0 && rounding.sub(f64::from(now), 1.0) > f64::from(contact.last_block) {
        contact.last_block = -1.0;
        contact.anchor_time = -1.0;
    }
    step
}

fn slide(xy: [f32; 2], next: [f32; 2], rounding: Rounding, passable: &mut impl FnMut([f32; 2]) -> bool) -> [f32; 2] {
    let minimum = f64::from(MINIMUM_AXIS_NUDGE);
    let nudge = |axis: usize| {
        let delta = rounding.sub(f64::from(next[axis]), f64::from(xy[axis]));
        if delta < 0.0 { delta.min(-minimum) } else { delta.max(minimum) }
    };
    let x = nudge(0);
    let y = f64::from(store(nudge(1), rounding));
    let axes = [
        [store(rounding.add(f64::from(xy[0]), x), rounding), xy[1]],
        [xy[0], store(rounding.add(f64::from(xy[1]), y), rounding)],
    ];
    let first = usize::from(y.abs() > f64::from(store(x, rounding)).abs());
    for axis in [first, 1 - first] {
        if passable(axes[axis]) { return axes[axis]; }
    }
    xy
}

fn contact_yaw(from: [f32; 2], to: [f32; 2], yaw: f32, dt: f32, rounding: Rounding) -> f32 {
    let delta = [0, 1].map(|axis| store(rounding.sub(f64::from(to[axis]), f64::from(from[axis])), rounding));
    let bearing = f64::from(delta[1]).atan2(f64::from(delta[0]));
    let mut difference = rounding.sub(bearing, f64::from(yaw));
    while difference > PI { difference = rounding.sub(difference, TAU); }
    while difference <= -PI { difference = rounding.add(difference, TAU); }
    let limit = store(rounding.mul(f64::from(dt) * f64::from(CONTACT_YAW_FACTOR),
        f64::from(maximum_yaw_rate(rounding))), rounding);
    let turn = store(difference, rounding).clamp(-limit, limit);
    store(wrap_positive(rounding.add(f64::from(yaw), f64::from(turn)), rounding), rounding)
}
