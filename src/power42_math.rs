//! Numeric behavior of the installed unit power reserve, isolated from world scheduling.

const MIN_RECHARGE_FACTOR: f32 = 0.05;

pub(crate) fn store_toward_zero(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

fn add_positive_toward_zero(left: f64, right: f64) -> f64 {
    let nearest = left + right;
    let right_part = nearest - left;
    let error = (left - (nearest - right_part)) + (right - right_part);
    if error < 0.0 { f64::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn reciprocal_toward_zero(value: f64) -> f64 {
    let nearest = 1.0 / value;
    if nearest.mul_add(value, -1.0) > 0.0 {
        f64::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

/// One matching effect stores the updated accumulator before the next lookup.
pub(crate) fn accumulate_recharge_factor(previous: f32, scale: f32) -> f32 {
    store_toward_zero(f64::from(previous) + (f64::from(scale) - 1.0))
}

/// Event 8 runs only when the current goal allows recharge and the simulation
/// has enabled it. The callback adds a full reciprocal quantum without dt.
pub(crate) fn update_power_reserve(current: f32, recharge_time: f32, factor: f32, flags: u32, blocks: u32) -> f32 {
    if flags & 0x400 == 0 || blocks != 0 { return current; }
    let duration = f64::from(recharge_time) * f64::from(factor.max(MIN_RECHARGE_FACTOR));
    let quantum = reciprocal_toward_zero(duration);
    store_toward_zero(add_positive_toward_zero(f64::from(current), quantum)).min(1.0)
}

/// Conversion's debiting branch limits the debit to the available reserve.
pub(crate) fn spend_conversion_reserve(current: f32, requested: f32) -> f32 {
    store_toward_zero(f64::from(current) - f64::from(current.min(requested))).max(0.0)
}

fn multiply_positive_toward_zero(left: f64, right: f64) -> f64 {
    let nearest = left * right;
    if left.mul_add(right, -nearest) < 0.0 { f64::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn divide_positive_toward_zero(numerator: f64, denominator: f64) -> f64 {
    let nearest = numerator / denominator;
    if nearest.mul_add(denominator, -numerator) > 0.0 {
        f64::from_bits(nearest.to_bits() - 1)
    } else { nearest }
}

// Own x87 evaluation of the source's lazy log-base-eight table under CW 0x0e7f.
const CONVERSION_CROWD: [f32; 32] = [
    f32::from_bits(0x3f800000), f32::from_bits(0x3f800000), f32::from_bits(0x3faaaaaa), f32::from_bits(0x3fc3a004),
    f32::from_bits(0x3fd55555), f32::from_bits(0x3fe311a5), f32::from_bits(0x3fee4aaf), f32::from_bits(0x3ff7c7cd),
    f32::from_bits(0x40000000), f32::from_bits(0x4003a004), f32::from_bits(0x4006de28), f32::from_bits(0x4009cd1b),
    f32::from_bits(0x400c7aac), f32::from_bits(0x400ef155), f32::from_bits(0x4011393b), f32::from_bits(0x401358d4),
    f32::from_bits(0x40155555), f32::from_bits(0x401732ff), f32::from_bits(0x4018f559), f32::from_bits(0x401a9f59),
    f32::from_bits(0x401c337d), f32::from_bits(0x401db3e8), f32::from_bits(0x401f2271), f32::from_bits(0x402080ae),
    f32::from_bits(0x4021d002), f32::from_bits(0x402311a5), f32::from_bits(0x402446ab), f32::from_bits(0x40257006),
    f32::from_bits(0x40268e91), f32::from_bits(0x4027a30d), f32::from_bits(0x4028ae2a), f32::from_bits(0x4029b084),
];

pub(crate) fn conversion_impact_delta(gap: i32, contributors: u32, attacker_time: f32, target_time: f32,
    player_factor: f32, target_effect: f32, attacker_effect: f32, reload: f32) -> f32 {
    let epoch_step = if gap < 0 {
        store_toward_zero(divide_positive_toward_zero(1.0, f64::from(0.9_f32)))
    } else { 0.9_f32 };
    let mut epoch_factor = 1.0;
    for _ in 0..gap.unsigned_abs() {
        epoch_factor = multiply_positive_toward_zero(epoch_factor, f64::from(epoch_step));
    }
    let resistance = store_toward_zero(multiply_positive_toward_zero(
        multiply_positive_toward_zero(epoch_factor, f64::from(attacker_time)), f64::from(target_time)));
    let crowd = CONVERSION_CROWD[contributors.min(31) as usize];
    let resistance = store_toward_zero(f64::from(resistance) * f64::from(crowd));
    let effect = store_toward_zero(f64::from(target_effect.max(0.05)) + f64::from(attacker_effect.max(0.05)) - 1.0);
    let denominator = multiply_positive_toward_zero(multiply_positive_toward_zero(f64::from(player_factor),
        f64::from(effect)), f64::from(resistance));
    store_toward_zero(divide_positive_toward_zero(f64::from(reload), denominator))
}

pub(crate) fn conversion_progress(progress: f32, delta: f32) -> f32 {
    store_toward_zero((f64::from(progress) + f64::from(delta)).min(1.0))
}
