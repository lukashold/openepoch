//! Original event-8 clock and extraction account arithmetic.
use crate::{defs::OIL_RIG, economy::gather_modifier, game::{source_math as native, Ent, Game}, map::R_OIL};

pub(crate) fn event_due(tick: u32, packed_id: u32, step: f32) -> bool {
    if !step.is_finite() || step <= 0.0 {
        return false;
    }
    let period = native::div(1.0, f64::from(step)).ceil();
    period <= f64::from(i32::MAX) && tick.wrapping_add(packed_id) as i32 % period as i32 == 0
}

pub(crate) fn output(progress: &mut f32, occupants: u8, rate: f32, modifier: f32) -> f32 {
    // 007E7D42 spills the first product, then 007E7D78 retains the
    // second sum for floor while also storing the chopped f32 progress.
    let staffed_rate = native::f(native::mul(f64::from(occupants), f64::from(rate)));
    let amount = native::mul(staffed_rate, f64::from(modifier.max(0.05)));
    let sum = native::add(f64::from(*progress), amount);
    *progress = native::f(sum) as f32;
    let whole = sum.floor();
    if whole < 1.0 {
        return 0.0;
    }
    *progress = native::f(native::sub(f64::from(*progress), native::f(whole))) as f32;
    native::f(whole) as f32
}

pub(crate) fn credit(balance: f32, amount: f64) -> f32 {
    native::f(native::add(f64::from(balance), amount).clamp(0.0, 999_999.0)) as f32
}

impl Game {
    pub(crate) fn update_rect_oil_economy(&mut self, index: usize, actor: &mut Ent, step: f32) -> bool {
        if actor.def != OIL_RIG { return false; }
        let Some(id) = self.rect_native_id(index, actor.team) else { return false; };
        if actor.hp > 0.0 && actor.ncargo != 0 && !self.sabotaged(index) && event_due(self.tick, id, step) {
            let modifier = self.modifiers[actor.team as usize][OIL_RIG as usize][gather_modifier(R_OIL)];
            #[cfg(feature = "ee2-rect288-oil-research")]
            let rate = self.oil_research_rate(actor.team);
            #[cfg(feature = "ee2-rect288-oil-research")]
            let modifier = native::f(native::add(f64::from(modifier), native::sub(
                f64::from(self.crown_generation_scale(actor.team, OIL_RIG, R_OIL - 1)), 1.0,
            ))) as f32;
            #[cfg(not(feature = "ee2-rect288-oil-research"))]
            let rate = 0.75;
            let whole = output(&mut actor.carry, actor.ncargo, rate, modifier);
            if whole > 0.0 {
                let slot = usize::from(R_OIL) - 1;
                let balance = credit(self.resource(actor.team, slot), f64::from(whole));
                self.set_resource(actor.team, slot, balance);
                self.crown_gathered(actor.team, slot, whole);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_normal_and_speed_profiles_preserve_actor_phase() {
        let vectors = [
            (0x3cbf258b, 0x08000004, 1, &[17, 60, 103][..], 0x40000000, 0x3e800000),
            (0x3d088888, 0x08000004, 1, &[23, 54, 85, 116], 0x40400000, 0),
            (0x3d317e4a, 0x08000004, 6, &[12, 36, 60, 84, 108], 0x41b00000, 0x3f000000),
            (0x3d5a740d, 0x08000004, 6, &[16, 35, 54, 73, 92, 111], 0x41d80000, 0),
            (0x3d088889, 0x08000004, 1, &[18, 48, 78, 108], 0x40400000, 0),
            (0x3d088888, 0x08080004, 1, &[7, 38, 69, 100], 0x40400000, 0),
        ];
        for (step, packed, crew, expected, bank_word, progress_word) in vectors {
            let (mut balance, mut progress) = (0.0, 0.0);
            let mut ticks = Vec::new();
            for tick in 1..=126 {
                if event_due(tick, packed, f32::from_bits(step)) {
                    ticks.push(tick);
                    balance = credit(balance, f64::from(output(&mut progress, crew, 0.75, 1.0)));
                }
            }
            assert_eq!(ticks, expected);
            assert_eq!((balance.to_bits(), progress.to_bits()), (bank_word, progress_word));
        }
    }

    #[test]
    fn adaptive_rate_switch_uses_admitted_delta_and_uninterrupted_tick() {
        let mut progress = 0.0;
        let ticks: Vec<_> = (1..=99).filter(|&tick| {
            let delta = f32::from_bits(match tick {
                1..=29 => 0x3dcccccc,
                30 => 0x3ea3d70a,
                _ => 0x3d5a740d,
            });
            event_due(tick, 0x08000004, delta)
        }).collect();
        assert_eq!(ticks, [11, 22, 35, 54, 73, 92]);
        let balance = ticks.into_iter().fold(0.0, |balance, _| {
            credit(balance, f64::from(output(&mut progress, 6, 0.75, 1.0)))
        });
        assert_eq!((balance, progress), (27.0, 0.0));
    }

    #[test]
    fn extraction_preserves_fraction_and_clamps_bank_independently() {
        let mut progress = 0.0;
        assert_eq!(output(&mut progress, 0, 0.75, 1.0), 0.0);
        assert_eq!(output(&mut progress, 6, 0.75, 1.0), 4.0);
        assert_eq!(progress, 0.5);
        assert_eq!(credit(999_998.5, 4.0), 999_999.0);
        assert_eq!(output(&mut progress, 6, 0.75, 1.0), 5.0);
        assert_eq!(progress, 0.0);
        assert!(!event_due(0, 0, 0.0));
    }

    #[test]
    fn original_chop_matches_native_ledger_words() {
        let vectors = [
            (1, 0x3f800000, 0x3e7fffff, &[(0x3f7fffff,0),(0x3f3ffffe,0x3f800000)][..]),
            (1, 0x3fe00000, 0, &[(0x3ea00000,0x3f800000),(0x3f200000,0x40000000),
                (0x3f700000,0x40400000),(0x3e800000,0x40a00000)]),
            (6, 0x3f999998, 0, &[(0x3eccccb0,0x40a00000),(0x3f4cccb0,0x41200000),
                (0x3e4ccc20,0x41800000),(0x3f199960,0x41a80000)]),
            (6, 0x00000000, 0, &[(0x3e666666,0),(0x3ee66666,0),(0x3f2ccccc,0),
                (0x3f666665,0),(0x3dfffff0,0x3f800000),(0x3eb3332f,0x3f800000)]),
        ];
        for (crew, modifier, initial, rows) in vectors {
            let mut progress = f32::from_bits(initial);
            let mut balance = 0.0;
            for &expected in rows {
                balance = credit(balance, f64::from(output(&mut progress, crew, 0.75, f32::from_bits(modifier))));
                assert_eq!((progress.to_bits(),balance.to_bits()), expected);
            }
        }
    }
}
