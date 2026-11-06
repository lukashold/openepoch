//! AircraftCarrier records from the active UP DDF and CSV5 upgrades.

use crate::defs::{Def, DefId, Wpn, AIR, A_SHIP, DOCK, NOW, SEA, W_BULLET, W_MISSILE};
use crate::ee2_units::UnitRow;
use crate::ee2_up15_attack::Attack;

pub(crate) const CARRIER: DefId = 84;
pub(crate) const SAVE_FLAG: u32 = 1 << 10;
pub(crate) const HASH_TAG: u32 = u32::from_le_bytes(*b"CAR1");
pub(crate) const TURN_RATE: f32 = 180.0 * 0.01745329238474369_f32;

const CARRIER_DEF: Def = Def {
    name: "Flugzeugträger", epoch: 10, until: 15, dom: SEA, building: false, size: 3.3,
    hp: 1200.0, speed: 1.725, sight: 12.0, armor: A_SHIP, cost: [0, 200, 0], time: 78.0,
    pop: 5, made_at: DOCK, flags: 0,
    w: [Wpn { range: 11.0, dmg: 30.0, cd: 0.5, kind: W_BULLET, targets: AIR, splash: 0.0 }, NOW],
};

pub(crate) const fn append_carrier(existing: [Def; 84]) -> [Def; 85] {
    let mut output = [CARRIER_DEF; 85];
    let mut index = 0;
    while index < existing.len() {
        output[index] = existing[index];
        index += 1;
    }
    output
}

const EARLY: UnitRow = UnitRow {
    id: CARRIER, epoch: 10, until: 13, source_epoch: 11, source_unit: "AircraftCarrier",
    source_visual: "AircraftCarrier_11", source_host: "Dock", identity_verified: true, model_verified: true,
    hp: Some(1200.0), buildtime: Some(78.0), cost9: Some([0,200,0,200,0,0,0,100,0]), pop: Some(5),
    pop_capacity: None, sight: Some(12.0), raw_size: Some([3.3, 1.0]), raw_footprint: None,
    raw_speed: Some(1.725), gather: None, farm_rate: None, carry: None, farm_carry: None, repair_rate: None,
    damage: Some(30.0), range: Some(11.0), reload: Some(0.5), garrison_slots: Some(4),
    generation: None, generation_resource: None, garrison_heal: Some(0.0),
    garrison_attack_bonus: None, garrison_size: None, fortress_garrison: false, attack_missile: None,
};
const LATE: UnitRow = UnitRow {
    epoch: 13, until: 15, source_epoch: 14, source_visual: "AircraftCarrier_14", hp: Some(1500.0),
    cost9: Some([0,200,0,200,0,0,0,0,100]), damage: Some(210.0), reload: Some(3.0),
    garrison_slots: Some(6), attack_missile: Some("GuidedMissile"), ..EARLY
};

const EARLY_ATTACK: Attack = Attack {
    id: CARRIER, epoch: 10, until: 13, present: true, missile: false, visual_kind: W_BULLET,
    min_mobile: 0.0, min_immobile: 0.0, angle: std::f32::consts::TAU, apply_time: 0.5,
    pack_time: 1.0, unpack_time: 1.0, splash: 0.0, friendly: 0.0, latent: false, auto_target: true,
    attack_on_move: false, ordnance: 0, air: false, mobile: true, projectile_speed: None,
};
const LATE_ATTACK: Attack = Attack {
    epoch: 13, until: 15, missile: true, visual_kind: W_MISSILE, projectile_speed: Some(11.0), ..EARLY_ATTACK
};

pub(crate) fn lookup(epoch: u8) -> Option<&'static UnitRow> {
    match epoch { 10..=12 => Some(&EARLY), 13..=14 => Some(&LATE), _ => None }
}

pub(crate) fn attack(epoch: u8) -> Option<&'static Attack> {
    match epoch { 10..=12 => Some(&EARLY_ATTACK), 13..=14 => Some(&LATE_ATTACK), _ => None }
}
