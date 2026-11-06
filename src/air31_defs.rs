//! Source rows for the three late-game aircraft mission families.

use crate::defs::{Def, DefId, Wpn, AIR, AIRFIELD, A_AIR, F_PLANE, LAND, NOW, SEA, SUB, W_BOMB, W_MISSILE};
use crate::ee2_units::UnitRow;
use crate::ee2_up15_attack::Attack;

pub(crate) const FIGHTER_BOMBER: DefId = 83;

pub(crate) const FIGHTER_BOMBER_DEF: Def = Def {
    name: "Jagdbomber", epoch: 10, until: 15, dom: AIR, building: false, size: 2.0,
    hp: 400.0, speed: 5.75, sight: 9.0, armor: A_AIR, cost: [0, 60, 0], time: 58.5,
    pop: 2, made_at: AIRFIELD, flags: F_PLANE,
    w: [Wpn { range: 1.0, dmg: 70.0, cd: 1.0, kind: W_BOMB, targets: LAND | SEA | SUB, splash: 1.0 }, NOW],
};

pub(crate) const fn append_fighter_bomber(existing: [Def; 83]) -> [Def; 84] {
    let mut output = [FIGHTER_BOMBER_DEF; 84];
    let mut index = 0;
    while index < existing.len() {
        output[index] = existing[index];
        index += 1;
    }
    output
}

pub(crate) fn mission_aircraft(id: DefId) -> bool {
    matches!(id, 37 | 38 | 39 | 40 | 44 | FIGHTER_BOMBER)
}

#[derive(Clone, Copy)]
pub(crate) struct Flight {
    pub speed: f32,
    pub angle_degrees_per_second: f32,
    pub height: f32,
    pub elevation_rate: f32,
}

const fn flight(speed: f32, angle_degrees_per_second: f32, height: f32) -> Flight {
    Flight { speed, angle_degrees_per_second, height, elevation_rate: 1.0 }
}

const FIGHTER_BOMBER_FLIGHT: [Flight; 5] = [
    flight(5.75, 42.0, 12.0),
    flight(5.98, 45.0, 13.0),
    flight(6.21, 49.0, 14.0),
    flight(6.44, 52.0, 15.0),
    flight(6.67, 56.0, 16.0),
];

// Active UP CSV5 selects these AircraftMove rows through the per-epoch
// UpgradeAbilities …EpochNAttack declarations, including their move fields.
const FIGHTER_FLIGHT: [Flight; 5] = [
    flight(5.75, 57.0, 13.0),
    flight(6.325, 59.0, 14.0),
    flight(6.9, 61.0, 15.0),
    flight(7.475, 62.0, 16.0),
    flight(8.05, 64.0, 17.0),
];

const BOMBER_FLIGHT: [Flight; 5] = [
    flight(5.52, 30.0, 10.0),
    flight(5.865, 32.0, 11.0),
    flight(6.21, 34.0, 12.0),
    flight(6.555, 36.0, 13.0),
    flight(6.9, 39.0, 14.0),
];

const BASE: UnitRow = UnitRow {
    id: FIGHTER_BOMBER, epoch: 10, until: 11, source_epoch: 11, source_unit: "FighterBomber",
    source_visual: "FighterBomber_11", source_host: "Airport", identity_verified: true, model_verified: true,
    hp: Some(400.0), buildtime: Some(58.5), cost9: Some([0,60,0,0,120,0,0,90,0]), pop: Some(2),
    pop_capacity: None, sight: Some(9.0), raw_size: Some([1.0, 1.0]), raw_footprint: None, raw_speed: Some(5.75),
    gather: None, farm_rate: None, carry: None, farm_carry: None, repair_rate: None, damage: Some(70.0),
    range: Some(1.0), reload: Some(1.0), garrison_slots: None, generation: None, generation_resource: None,
    garrison_heal: None, garrison_attack_bonus: None, garrison_size: None, fortress_garrison: false,
    attack_missile: Some("Bomb02"),
};

const FIGHTER_BOMBER_ROWS: [UnitRow; 5] = [
    BASE,
    UnitRow { epoch: 11, until: 12, source_epoch: 12, source_visual: "FighterBomber_12", hp: Some(470.0),
        cost9: Some([0,65,0,0,125,0,0,100,0]), damage: Some(80.0), raw_speed: Some(5.98), ..BASE },
    UnitRow { epoch: 12, until: 13, source_epoch: 13, source_visual: "FighterBomber_13", hp: Some(540.0),
        cost9: Some([0,70,0,0,130,0,0,100,0]), damage: Some(90.0), raw_speed: Some(6.21), range: Some(4.0),
        sight: Some(10.0), attack_missile: Some("GuidedMissile"), ..BASE },
    UnitRow { epoch: 13, until: 14, source_epoch: 14, source_visual: "FighterBomber_14", hp: Some(610.0),
        cost9: Some([0,75,0,0,135,0,0,0,105]), damage: Some(100.0), raw_speed: Some(6.44), range: Some(5.0),
        sight: Some(10.0), attack_missile: Some("GuidedMissile"), ..BASE },
    UnitRow { epoch: 14, until: 15, source_epoch: 15, source_visual: "FighterBomber_15", hp: Some(680.0),
        cost9: Some([0,80,0,0,140,0,0,0,110]), damage: Some(110.0), raw_speed: Some(6.67), range: Some(6.0),
        sight: Some(10.0), attack_missile: Some("GuidedMissile"), reload: Some(0.3), ..BASE },
];

const ATTACK: Attack = Attack {
    id: FIGHTER_BOMBER, epoch: 10, until: 11, present: true, missile: true, visual_kind: W_BOMB,
    min_mobile: 0.0, min_immobile: 0.0, angle: std::f32::consts::FRAC_PI_3,
    apply_time: 0.0, pack_time: 1.0, unpack_time: 1.0, splash: 1.0, friendly: 0.0,
    latent: false, auto_target: true, attack_on_move: true, ordnance: 14, air: true, mobile: true,
    projectile_speed: Some(1.0),
};
const GUIDED: Attack = Attack { visual_kind: W_MISSILE, projectile_speed: Some(11.0), ..ATTACK };
const FIGHTER_BOMBER_ATTACK: [Attack; 5] = [
    ATTACK,
    Attack { epoch: 11, until: 12, splash: 1.25, ordnance: 16, ..ATTACK },
    Attack { epoch: 12, until: 13, splash: 1.5, ordnance: 18, ..GUIDED },
    Attack { epoch: 13, until: 14, splash: 1.75, ordnance: 20, ..GUIDED },
    Attack { epoch: 14, until: 15, splash: 2.0, ordnance: 22, ..GUIDED },
];

fn row_index(epoch: u8) -> Option<usize> { epoch.checked_sub(10).map(usize::from) }

pub(crate) fn flight_for_fighter_bomber(epoch: u8) -> Option<&'static Flight> {
    FIGHTER_BOMBER_FLIGHT.get(row_index(epoch)?)
}

pub(crate) fn flight_profile(definition: DefId, epoch: u8) -> Option<&'static Flight> {
    let rows = match definition {
        37 | 39 | 44 => &FIGHTER_FLIGHT,
        38 | 40 => &BOMBER_FLIGHT,
        FIGHTER_BOMBER => &FIGHTER_BOMBER_FLIGHT,
        _ => return None,
    };
    rows.get(row_index(epoch)?)
}

pub(crate) fn fighter_bomber(epoch: u8) -> Option<&'static UnitRow> {
    FIGHTER_BOMBER_ROWS.get(row_index(epoch)?)
}

pub(crate) fn fighter_bomber_attack(epoch: u8) -> Option<&'static Attack> {
    FIGHTER_BOMBER_ATTACK.get(row_index(epoch)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aircraft_profile_inherits_zero_friendly_ratio_and_finite_ordnance() {
        assert_eq!(FIGHTER_BOMBER_DEF.made_at, AIRFIELD);
        assert!(fighter_bomber(9).is_none() && fighter_bomber_attack(15).is_none());
        for epoch in 10..15 {
            let row = fighter_bomber(epoch).unwrap();
            let attack = fighter_bomber_attack(epoch).unwrap();
            let flight = flight_for_fighter_bomber(epoch).unwrap();
            assert_eq!(row.source_epoch, epoch + 1);
            assert_eq!(row.raw_speed, Some(flight.speed));
            assert_eq!(row.buildtime, Some(58.5));
            assert_eq!(row.pop, Some(2));
            assert_eq!(attack.ordnance, 14 + 2 * u16::from(epoch - 10));
            assert_eq!(attack.friendly, 0.0);
            assert!(attack.attack_on_move);
        }
    }
}
