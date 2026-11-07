//! Surface Z displacement from the original NavalMove oscillator 007F3310.

const DURATIONS: [f32; 6] = [0.45, 0.55, 0.7, 0.55, 0.3, 0.45];
const AMPLITUDES: [f32; 6] = [0.7, 0.9, 1.0, 0.6, 0.3, 0.5];
const TOTAL_DURATION: f64 = 3.0;
const PHASE_RATE: f64 = f64::from_bits(0x3fc1_1111_1111_1111);
// 007F3524..3532 stores f32(.2f * .9f) with x87 round-toward-zero.
const SURFACE_RATE: f64 = f32::from_bits(0x3e38_51eb) as f64;
const SURFACE_AMPLITUDE: f64 = 0.05f32 as f64;

fn trunc_rounding(value: f64, error: f64) -> f64 {
    if value != 0.0 && (value.is_sign_positive() && error < 0.0 ||
        value.is_sign_negative() && error > 0.0) {
        f64::from_bits(value.to_bits() - 1)
    } else {
        value
    }
}

pub(crate) fn add(left: f64, right: f64) -> f64 {
    let value = left + right;
    let virtual_right = value - left;
    let error = (left - (value - virtual_right)) + (right - virtual_right);
    trunc_rounding(value, error)
}

fn mul(left: f64, right: f64) -> f64 {
    let value = left * right;
    trunc_rounding(value, left.mul_add(right, -value))
}

fn div(numerator: f64, denominator: f64) -> f64 {
    let value = numerator / denominator;
    trunc_rounding(value, (-value).mul_add(denominator, numerator))
}

pub(crate) fn store(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

/// NavalMove applies this to the zero surface depth after its movement step.
/// The caller supplies the last committed simulation time, shared by all views.
pub(crate) fn surface_heave(packed_actor_id: u32, elapsed: f32) -> f32 {
    let actor_phase = f64::from((packed_actor_id & 0x07ff_ffff) % 13);
    let phase = mul(actor_phase * TOTAL_DURATION, PHASE_RATE);
    let shifted = add(phase, f64::from(elapsed) * SURFACE_RATE);
    let phase_at_store = f64::from(store(shifted));
    let cycle = div(shifted, TOTAL_DURATION).floor();
    let mut remainder = add(phase_at_store, -cycle * TOTAL_DURATION);
    let mut lobe = DURATIONS.len() - 1;
    for (index, duration) in DURATIONS[..lobe].iter().enumerate() {
        let duration = f64::from(*duration);
        if remainder < duration {
            lobe = index;
            break;
        }
        remainder = add(remainder, -duration);
    }
    let progress = div(remainder, f64::from(DURATIONS[lobe])) * 4.0;
    let curve = if progress < 2.0 {
        let offset = add(1.0, -progress);
        add(1.0, -mul(offset, offset))
    } else {
        let offset = add(progress, -3.0);
        add(mul(offset, offset), -1.0)
    };
    store(mul(mul(curve, f64::from(AMPLITUDES[lobe])), SURFACE_AMPLITUDE)) + 0.0
}

impl crate::game::Game {
    pub(crate) fn naval_surface_pose(&self, index: usize) -> Option<(u32, f32)> {
        self.naval_surface_pose_at(index, self.sim_elapsed())
    }

    pub(crate) fn naval_surface_pose_at(&self, index: usize, elapsed: f32) -> Option<(u32, f32)> {
        let actor = &self.ents[index];
        // The hull keeps its NavalMove class after its training line retires.
        // WarGalley, Destroyer, Battleship and FightingSail retain zero-depth
        // surface NavalMove through their active source upgrades.
        let carrier = actor.def == crate::carrier36_defs::CARRIER && self.carrier_surface_pose_enabled;
        if !(matches!(actor.def, 29 | 32 | 33 | 35 | 58) || carrier) ||
            actor.hp <= 0.0 || !self.production_roster_enabled { return None; }
        let serial = self.powers.as_ref()?.actor(self.h(index))?.serial;
        let id = (u32::from(actor.team) + 1) << 27 | serial;
        Some((id, surface_heave(id, elapsed)))
    }
}
