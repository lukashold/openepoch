//! Source AnimalManager Wild herd timing and ordinary Deer placement order.
use super::{animal_wander247::{draw, interval_parts, wander_point}, deer_foot247::{store, Rounding, TAU}};

pub(super) const DEER_RADIUS: f32 = 0.225;
pub(super) const NEWBORN_YAW: f32 = f32::from_bits(0x40af_eddf);
const BIRTH_PERIOD: f64 = 360.0;
const ROUNDING: Rounding = Rounding::Chop53;

pub(super) struct Birth {
    pub id: u32,
    pub xyz: [f32; 3],
    pub first_wander: f32,
}

pub(super) fn tick_birth(
    rng: &mut u32, deadline: &mut f32, population: &mut u32, cap: u32, home: [f32; 2], now: f32,
    place: impl FnOnce([f32; 2]) -> Option<(u32, [f32; 3])>,
) -> Option<Birth> {
    if now <= *deadline || *population == 0 { return None; }
    let interval = ROUNDING.div(BIRTH_PERIOD, f64::from(*population));
    let (low, sample) = interval_parts(rng, interval);
    *deadline = store(ROUNDING.add(ROUNDING.add(low, sample), f64::from(*deadline)), ROUNDING);
    if *population >= cap { return None; }
    let point = wander_point(rng, home, 4.0, ROUNDING);
    let (id, xyz) = place(point)?;
    draw(rng); // Positive Deer UnitAI activation precedes AnimalManager registration.
    let (low, sample) = interval_parts(rng, 6.0);
    let now = f64::from(now);
    let first_wander = store(ROUNDING.add(ROUNDING.add(sample, now), low), ROUNDING);
    let old = f64::from(*population);
    *population += 1;
    let remainder = ROUNDING.sub(f64::from(*deadline), now);
    let rephased = ROUNDING.div(ROUNDING.mul(remainder, old), f64::from(*population));
    *deadline = store(ROUNDING.add(now, rephased), ROUNDING);
    Some(Birth { id, xyz, first_wander })
}

pub(super) fn find_birth_site(
    home: [f32; 2], dimensions: [usize; 2], mut admit: impl FnMut([f32; 2]) -> Option<f32>,
) -> Option<[f32; 3]> {
    let radius = f64::from(DEER_RADIUS);
    let bound = (f64::hypot(dimensions[0] as f64, dimensions[1] as f64) / radius) as u32 + 1;
    for ring in 0..bound {
        let distance = f64::from(store(f64::from(ring) * radius, ROUNDING));
        let increment = if ring == 0 { 687.0 } else { store(1.0 / f64::from(ring), ROUNDING) };
        let mut angle = 0.0;
        while f64::from(angle) < TAU {
            let (sin, cos) = f64::from(angle).sin_cos();
            let [x, y] = [cos, sin].map(|component| store(ROUNDING.mul(distance, component), ROUNDING));
            let xy = [store(f64::from(home[0]) + f64::from(x), ROUNDING),
                store(f64::from(home[1]) + f64::from(y), ROUNDING)];
            if let Some(z) = admit(xy) { return Some([xy[0], xy[1], z]); }
            angle = store(f64::from(angle) + f64::from(increment), ROUNDING);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_herd68_birth_is_exact_for_tie_capacity_rejection_and_commit() {
        let home = [0x41a4_75f5, 0x4390_e0fe].map(f32::from_bits);
        for (now, count, result) in [(360.0, 1, (0x36c7_50d3, 0x43b4_0000, 1)),
            (f32::from_bits(0x43b4_0666), 7, (0x535d_2016, 0x43ce_1a73, 7)),
            (f32::from_bits(0x43b4_0666), 1, (0xc622_3fb8, 0x4435_5c95, 1))] {
            let (mut rng, mut deadline, mut population) = (919032019, 360.0, count);
            assert!(tick_birth(&mut rng, &mut deadline, &mut population, 7, home, now, |point| {
                assert!(now > 360.0 && count == 1);
                assert_eq!(point.map(f32::to_bits), [0x4197_c8c7, 0x4390_a1ff]);
                None
            }).is_none());
            assert_eq!((rng, deadline.to_bits(), population), result);
        }
        for denied in [0, 1] {
            let (mut rng, mut deadline, mut population) = (919032019, 360.0, 1);
            let mut checks = 0;
            let born = tick_birth(&mut rng, &mut deadline, &mut population, 7, home, f32::from_bits(0x43b4_0666), |point| {
                let xyz = find_birth_site(point, [100, 300], |_| { checks += 1; (checks > denied).then_some(1.0) })?;
                Some((1029, xyz))
            }).unwrap();
            assert_eq!(checks, denied + 1);
            assert_eq!(born.id, 1029);
            assert_eq!(born.xyz.map(f32::to_bits), [[0x4197_c8c7, 0x4390_a1ff, 0x3f80_0000], [0x4199_9593, 0x4390_a1ff, 0x3f80_0000]][denied]);
            assert_eq!(born.first_wander.to_bits(), 0x43b7_16a4);
            assert_eq!((rng, deadline.to_bits(), population), (0xdb12_59aa, 0x4407_afe4, 2));
        }
    }
}
