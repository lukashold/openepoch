//! Stock AnimalManager single-animal wander timing and point generation.
use super::deer_foot247::{store, Rounding};

const LOWER_FACTOR: f64 = f32::from_bits(0x3f73_3333) as f64;
const UPPER_FACTOR: f64 = f32::from_bits(0x3f86_6666) as f64;
const HIGH15_RECIPROCAL: f64 = f32::from_bits(0x3800_0100) as f64;
const ANGLE_STEP: f64 = f32::from_bits(0x3949_116d) as f64;

pub(super) fn draw(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 16) & 0x7fff
}

pub(super) fn interval_parts(state: &mut u32, interval: f64) -> (f64, f64) {
    let low = interval * LOWER_FACTOR;
    let high = interval * UPPER_FACTOR;
    let sample = f64::from(draw(state)) * (high - low) * HIGH15_RECIPROCAL;
    (low, sample)
}

pub(super) fn wander_point(state: &mut u32, anchor: [f32; 2], radius: f32, rounding: Rounding) -> [f32; 2] {
    let distance = store(f64::from(draw(state)) * f64::from(radius) * HIGH15_RECIPROCAL, rounding);
    let angle = f64::from(draw(state)) * ANGLE_STEP;
    let (sin, cos) = angle.sin_cos();
    let axis = |home, trig| {
        let step = store(f64::from(distance) * f64::from(store(trig, rounding)), rounding);
        store(f64::from(home) + f64::from(step), rounding)
    };
    [axis(anchor[0], cos), axis(anchor[1], sin)]
}

/// Process one stock animal row in caller-owned hash traversal order.
/// The callback admits the point using terrain and the real Move task queue. It runs before
/// the deadline draw, so it shares the source RNG if an admitted task needs it.
pub fn update_due(
    state: &mut u32,
    deadline: &mut f32,
    now: f32,
    busy: bool,
    anchor: [f32; 2],
    radius: f32,
    interval: f32,
    rounding: Rounding,
    propose_move: impl FnOnce([f32; 2], &mut u32),
) -> bool {
    if !(now >= *deadline) {
        return false;
    }
    if !busy {
        let point = wander_point(state, anchor, radius, rounding);
        propose_move(point, state);
    }
    let (low, sample) = interval_parts(state, f64::from(interval));
    *deadline = store((sample + low) + f64::from(*deadline), rounding);
    true
}

