//! UP capture descriptors and per-impact arithmetic for the civ8 roster.
//! The simulation supplies target legality, active effects, garrison occupancy,
//! actor phases and the normal attack/transfer lifecycle.

use crate::ids::DefId;

pub(crate) const CITY_CAPTURE_RESISTANCE: f32 = 1.05;
pub(crate) const CAPTURE_COMPLETE: f32 = 0.99;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CaptureAttacker {
    pub range: f32,
    pub reload_seconds: f32,
    pub impact_seconds: f32,
    pub time_modifier: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CaptureTribute {
    None,
    Natural(f32),
    Gold(f32),
    Tech(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CaptureTarget {
    pub time_modifier: f32,
    pub garrison_time_per_unit: f32,
    pub garrison_damage_per_unit: f32,
    pub damage_rate: f32,
    pub has_garrison: bool,
    pub tribute: CaptureTribute,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CaptureFactors {
    /// Current epoch of the attacker minus current epoch of the target owner.
    pub epoch_gap: i32,
    pub contributors: u32,
    pub garrison_count: u8,
    pub player_factor: f32,
    pub target_effect: f32,
    pub attacker_effect: f32,
}

/// ConvertAttack declarations of HeavyInfantry1, HeavyInfantry2 and
/// LightInfantry. The current roster's linked epoch upgrades retain these
/// fields. Research modifiers, if present, are supplied by the caller.
pub(crate) fn capture_attacker(def_id: DefId) -> Option<CaptureAttacker> {
    if def_id == crate::priest42_defs::PRIEST {
        return Some(CaptureAttacker { range: 8.0, reload_seconds: 2.0, impact_seconds: 0.5, time_modifier: 5.0 });
    }
    let range = match def_id {
        9 | 13 | 15..=17 => 0.2,
        10..=12 | 14 => 2.0,
        _ => return None,
    };
    Some(CaptureAttacker {
        range,
        reload_seconds: 4.0,
        impact_seconds: 1.0,
        time_modifier: 3.0,
    })
}

/// The active Convertable descriptors with capture=false. The empty TradeCart
/// descriptor is normalized to one; ordinary Human/Mounted/Artillery uses three.
pub(crate) fn conversion_target(def_id: DefId) -> Option<CaptureTarget> {
    let time_modifier = match def_id {
        crate::trade30_defs::TRADE_CART => 1.0,
        8..=28 | 46 | 57 | 61..=79 | crate::priest42_defs::PRIEST => 3.0,
        _ => return None,
    };
    Some(CaptureTarget { time_modifier, garrison_time_per_unit: 1.0, garrison_damage_per_unit: 0.0,
        damage_rate: 0.0, has_garrison: false, tribute: CaptureTribute::None })
}

/// Effective Convertable capture=1 rows in the active registry.
/// Ordinary mobile Convertable rows lack capture=1. Linked DDF epoch upgrades
/// do not replace these descriptors or change whether a Garrison slot exists.
pub(crate) fn capture_target(def_id: DefId) -> Option<CaptureTarget> {
    use CaptureTribute::{Gold, Natural, Tech};

    // 007E42F0 normalizes a zero garrison-time coefficient to one.
    let mut target = CaptureTarget {
        time_modifier: 200.0,
        garrison_time_per_unit: 1.0,
        garrison_damage_per_unit: 0.0,
        damage_rate: 0.0,
        has_garrison: false,
        tribute: CaptureTribute::None,
    };
    match def_id {
        0 => {
            target.time_modifier = 300.0;
            target.has_garrison = true;
            target.tribute = Natural(0.1);
        }
        1 => target.time_modifier = 100.0,
        3 | 4 | 6 | 7 | 48 | 49 | 55 | 59 | 60 => {}
        53 | 54 | 56 => target.has_garrison = true,
        5 | 47 | 50..=52 => {
            if def_id != 5 {
                target.time_modifier = 100.0;
            }
            target.garrison_time_per_unit = 0.1;
            target.garrison_damage_per_unit = 1.0;
            target.damage_rate = 10.0;
            target.has_garrison = matches!(def_id, 47 | 51 | 52);
            target.tribute = match def_id {
                47 | 51 => Tech(0.1),
                52 => Natural(0.05),
                _ => Gold(0.1),
            };
        }
        definition if crate::wonders61::is_wonder(definition) => target.time_modifier = 300.0,
        _ => return None,
    }
    Some(target)
}

/// The final native acquire gate assumes the target has passed the shared
/// attack/actor filters and its descriptor is present.
pub(crate) fn can_join_capture(
    target_hp: f32,
    contributors: u32,
    registered_owner: Option<u8>,
    attacker_owner: u8,
) -> bool {
    target_hp > 0.0 && (contributors == 0 || registered_owner == Some(attacker_owner))
}

/// 007DE030's capture branch with its explicit f32 store boundaries. Binary53
/// intermediates reproduce the checked game-state vectors. The native x87
/// control word and rounding ties for arbitrary inputs remain unverified.
pub(crate) fn capture_delta(
    attacker: CaptureAttacker,
    target: CaptureTarget,
    factors: CaptureFactors,
) -> f32 {
    let epoch_factor = if factors.epoch_gap < 0 {
        (1.0_f64 / 0.9_f32 as f64) as f32
    } else {
        0.9_f32
    };
    let mut epoch_multiplier = 1.0_f64;
    for _ in 0..factors.epoch_gap.unsigned_abs() {
        epoch_multiplier *= epoch_factor as f64;
    }
    let base = (epoch_multiplier * attacker.time_modifier as f64 * target.time_modifier as f64) as f32;
    let count = factors.contributors.clamp(1, 99) as f64;
    let crowd = (1.0 + count.log2() / 3.0) as f32;
    let crowded = base * crowd;
    let garrison = if target.has_garrison {
        factors.garrison_count as f64 * target.garrison_time_per_unit as f64
    } else {
        1.0
    };
    let resistance = (crowded as f64 + garrison) as f32;
    let effect = (factors.target_effect.max(0.05) as f64
        + factors.attacker_effect.max(0.05) as f64
        - 1.0) as f32;
    (attacker.reload_seconds as f64 / (factors.player_factor as f64 * effect as f64 * resistance as f64)) as f32
}

pub(crate) fn capture_counter_damage(target: CaptureTarget, garrison_count: u8) -> f32 {
    if target.has_garrison {
        target.garrison_damage_per_unit * garrison_count as f32
    } else {
        0.0
    }
}

pub(crate) fn apply_capture_delta(progress: f32, delta: f32) -> (f32, bool) {
    let sum = progress + delta;
    let progress = if sum > 1.0 { 1.0 } else { sum };
    (progress, progress >= CAPTURE_COMPLETE)
}

/// One eligible Event 8 with zero contributors, as dispatched by 008416C0.
pub(crate) fn decay_capture_progress(progress: f32) -> f32 {
    (progress - 0.1).max(0.0)
}

/// The common native Actor update staggers Event 8 by its packed actor ID.
/// Pass the native phase ID or the simulation's explicitly chosen projection.
pub(crate) fn capture_event_due(native_step: u32, packed_actor_id: u32, sim_delta: f32) -> bool {
    if sim_delta <= 0.0 || !sim_delta.is_finite() {
        return false;
    }
    let interval = (1.0_f64 / sim_delta as f64).ceil();
    interval <= i32::MAX as f64
        && (native_step.wrapping_add(packed_actor_id) as i32) % interval as i32 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn factors() -> CaptureFactors {
        CaptureFactors {
            epoch_gap: 0,
            contributors: 1,
            garrison_count: 0,
            player_factor: 1.0,
            target_effect: CITY_CAPTURE_RESISTANCE,
            attacker_effect: 1.0,
        }
    }

    fn city_delta(factors: CaptureFactors) -> u32 {
        capture_delta(capture_attacker(9).unwrap(), capture_target(0).unwrap(), factors).to_bits()
    }

    #[test]
    fn source_attacker_and_target_coverage() {
        let attackers: Vec<_> = (0..=80).filter(|&id| capture_attacker(id).is_some()).collect();
        assert_eq!(attackers, (9..=17).collect::<Vec<_>>());
        assert_eq!(capture_attacker(13).unwrap().range, 0.2);
        assert_eq!(capture_attacker(14).unwrap().range, 2.0);
        assert_eq!(capture_attacker(14).unwrap().impact_seconds, 1.0);
        let targets: Vec<_> = (0..=80).filter_map(|id| capture_target(id).map(|target| (id, target))).collect();
        assert_eq!(
            targets.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            [0, 1, 3, 4, 5, 6, 7, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 59, 60],
        );
        assert_eq!(
            targets.iter().filter_map(|&(id, target)| target.has_garrison.then_some(id)).collect::<Vec<_>>(),
            [0, 47, 51, 52, 53, 54, 56],
        );
        assert_eq!(capture_target(0).unwrap().tribute, CaptureTribute::Natural(0.1));
        assert_eq!(capture_target(52).unwrap().tribute, CaptureTribute::Natural(0.05));
        assert_eq!(capture_target(47).unwrap().tribute, CaptureTribute::Tech(0.1));
        assert_eq!(capture_target(5).unwrap().tribute, CaptureTribute::Gold(0.1));
        assert_eq!(capture_counter_damage(capture_target(5).unwrap(), 1), 0.0);
        assert_eq!(capture_counter_damage(capture_target(47).unwrap(), 2), 2.0);
        assert_eq!(capture_counter_damage(capture_target(0).unwrap(), 5), 0.0);
    }

    #[test]
    fn source_derived_impact_vectors_preserve_native_store_boundaries() {
        assert_eq!(city_delta(CaptureFactors { target_effect: 1.0, ..factors() }), 0x3b91_a2b4);
        assert_eq!(city_delta(factors()), 0x3b8a_b356);
        assert_eq!(city_delta(CaptureFactors { garrison_count: 1, ..factors() }), 0x3b8a_8bee);
        assert_eq!(city_delta(CaptureFactors { contributors: 8, ..factors() }), 0x3b0a_b356);
        assert_eq!(city_delta(CaptureFactors { epoch_gap: 1, ..factors() }), 0x3b9a_1c99);
        assert_eq!(city_delta(CaptureFactors { epoch_gap: -1, ..factors() }), 0x3b79_a934);
        assert_eq!(city_delta(CaptureFactors { player_factor: 1.2, ..factors() }), 0x3b67_2ae5);
        assert_eq!(
            capture_delta(capture_attacker(9).unwrap(), capture_target(1).unwrap(),
                          CaptureFactors { target_effect: 1.0, ..factors() }).to_bits(),
            0x3c59_ba42,
        );
        assert_eq!(
            city_delta(CaptureFactors { contributors: 99, ..factors() }),
            city_delta(CaptureFactors { contributors: 100, ..factors() }),
        );
    }

    #[test]
    fn completion_release_and_decay_boundaries() {
        let mut progress = 0.0;
        let delta = f32::from_bits(city_delta(factors()));
        for _ in 0..233 {
            let (value, complete) = apply_capture_delta(progress, delta);
            assert!(!complete);
            progress = value;
        }
        let (value, complete) = apply_capture_delta(progress, delta);
        assert!(complete);
        assert_eq!(value.to_bits(), 0x3f7d_8ff7);
        assert_eq!(apply_capture_delta(0.99, 0.0), (0.99, true));
        assert_eq!(apply_capture_delta(0.99, 0.4), (1.0, true));
        assert!(!apply_capture_delta(0.5, f32::NAN).1);
        assert_eq!(decay_capture_progress(0.04), 0.0);
        assert_eq!(decay_capture_progress(0.5), 0.4);
        assert!(can_join_capture(0.001, 0, None, 1));
        assert!(can_join_capture(1.0, 1, Some(1), 1));
        assert!(!can_join_capture(1.0, 1, Some(1), 2));
        assert!(!can_join_capture(0.0, 0, None, 1));
    }

    #[test]
    fn event_eight_uses_actor_phase_and_the_source_ceil() {
        let phase = |actor| (0..=100).filter(|&step| capture_event_due(step, actor, 0.05)).collect::<Vec<_>>();
        assert_eq!(phase(1), [19, 39, 59, 79, 99]);
        assert_eq!(phase(2), [18, 38, 58, 78, 98]);
        assert!(capture_event_due(4, 0, 0.3));
        assert!(!capture_event_due(3, 0, 0.3));
        assert!(!capture_event_due(20, 0, 0.0));
        assert!(capture_event_due(1, 0x7fff_ffff, 0.5));
    }
}
