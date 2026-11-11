//! Ordinary Deer Foot terrain grade at 007F26AA–007F278A and 007F27F2–007F27FC.
use super::deer_foot247::{FootMotion, Rounding, stock_foot_limits, store};
use super::deer_foot247::FootLimits;

const LOOKAHEAD: f64 = f32::from_bits(0x3a83_126f) as f64;
const THRESHOLD: f64 = 0.25;
const MIN_FACTOR: f64 = f32::from_bits(0x3e4c_cccd) as f64;

pub(super) struct FootGrade {
    pub grade: f32,
    pub limits: FootLimits,
}

/// Apply the terrain-grade limit for an animal on ordinary ground. The terrain
/// callback uses source-space XY and returns height before any F32 spill.
/// Caller owns source terrain, bridge/road routing, and other speed modifiers.
pub(super) fn foot_grade_limits(
    xy: [f32; 2],
    prior: FootMotion,
    speed: f32,
    rounding: Rounding,
    mut sample: impl FnMut([f32; 2]) -> f64,
) -> FootGrade {
    let base = stock_foot_limits(speed, rounding);
    if prior.speed <= 0.0 {
        return FootGrade { grade: 0.0, limits: base };
    }

    let start_height = f64::from(store(sample(xy), rounding));
    let probe = std::array::from_fn(|axis| {
        store(rounding.add(f64::from(xy[axis]), f64::from(prior.velocity[axis]) * LOOKAHEAD), rounding)
    });
    let height_delta = rounding.sub(sample(probe), start_height);
    let grade = rounding.div(height_delta, f64::from(prior.speed) * LOOKAHEAD);
    if grade <= THRESHOLD {
        return FootGrade { grade: store(grade, rounding), limits: base };
    }

    let blend = rounding.mul(rounding.div(THRESHOLD, grade), 1.0 - MIN_FACTOR);
    let factor = rounding.add(blend, MIN_FACTOR);
    let forward = store(rounding.mul(f64::from(base.forward), factor), rounding);
    let ratio = rounding.div(f64::from(forward), f64::from(base.forward));
    let angular = store(rounding.mul(ratio, f64::from(base.angular)), rounding);
    FootGrade { grade: store(grade, rounding), limits: FootLimits { forward, angular } }
}
