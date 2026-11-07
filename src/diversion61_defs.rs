//! Original diversion cohorts are fixed source unit types selected at birth.

use crate::defs::{Def, DefId, Wpn, A_VEH, LAND, NOW, W_MELEE, W_SHELL};
use crate::ee2_units::UnitRow;

pub(crate) const FIRST: DefId = crate::wonders61::END;
pub(crate) const END: DefId = FIRST + 4;
const COUNT: usize = END as usize;
const NAMES: [&str; 4] = ["MG-Panzer", "Sturmpanzer", "Hauptkampfpanzer", "Schwerer HERC 'Minotaurus'"];
const SOURCES: [&str; 4] = ["DiversionUnit_Epoch11", "DiversionUnit_Epoch12", "DiversionUnit_Epoch13", "DiversionUnit_Epoch14"];
const HP: [f32; 4] = [225.0, 275.0, 300.0, 400.0];
const DAMAGE: [f32; 4] = [20.0, 40.0, 50.0, 150.0];
const RANGE: [f32; 4] = [9.0, 12.0, 8.0, 6.0];
const POPULATION: [usize; 4] = [9, 12, 13, 13];

pub(crate) fn slot(definition: DefId) -> Option<usize> {
    (FIRST..END).contains(&definition).then(|| usize::from(definition - FIRST))
}

pub(crate) fn cohort(epoch: u8) -> Option<(DefId, usize)> {
    let index = usize::from(epoch.checked_sub(10)?.min(3));
    Some((FIRST + index as DefId, POPULATION[index]))
}

const fn shape(index: usize) -> [f32; 2] {
    if index == 3 { [0.65, 0.65] } else { [0.9, 0.7] }
}

pub(crate) fn extent(definition: DefId) -> Option<[f32; 2]> {
    slot(definition).map(shape)
}

pub(crate) fn attack(definition: DefId) -> Option<&'static crate::ee2_up15_attack::Attack> {
    slot(definition).map(|index| &ATTACKS[index])
}

const fn definition(index: usize) -> Def {
    Def {
        name: NAMES[index], epoch: (10 + index) as u8, until: 15, dom: LAND, building: false,
        size: shape(index)[0] * 2.0, hp: HP[index] * 4.0, speed: 1.725,
        sight: if index == 3 { 6.0 } else { 9.0 }, armor: A_VEH, cost: [0; 3],
        time: 0.0, pop: 0, made_at: DefId::MAX, flags: 0,
        w: [Wpn { range: RANGE[index], dmg: DAMAGE[index], cd: if index == 3 { 1.3 } else { 1.5 },
            kind: if index == 3 { W_MELEE } else { W_SHELL }, targets: LAND, splash: 0.0 }, NOW],
    }
}

pub(crate) const fn append(existing: [Def; FIRST as usize]) -> [Def; COUNT] {
    let mut result = [definition(0); COUNT];
    let mut index = 0;
    while index < existing.len() {
        result[index] = existing[index];
        index += 1;
    }
    while index < result.len() {
        result[index] = definition(index - existing.len());
        index += 1;
    }
    result
}

const fn row(index: usize) -> UnitRow {
    let def = definition(index);
    UnitRow {
        id: FIRST + index as DefId, epoch: 10 + index as u8, until: 15, source_epoch: 11 + index as u8,
        source_unit: SOURCES[index], source_visual: SOURCES[index], source_host: "", identity_verified: true,
        model_verified: false, hp: Some(def.hp), buildtime: Some(0.0), cost9: Some([0; 9]), pop: Some(0),
        pop_capacity: None, sight: Some(def.sight), raw_size: Some(shape(index)), raw_footprint: None,
        raw_speed: Some(def.speed), gather: None, farm_rate: None, carry: None, farm_carry: None,
        repair_rate: None, damage: Some(def.w[0].dmg), range: Some(def.w[0].range), reload: Some(def.w[0].cd),
        garrison_slots: None, generation: None, generation_resource: None, garrison_heal: None,
        garrison_attack_bonus: None, garrison_size: None, fortress_garrison: false,
        attack_missile: if index == 3 { None } else { Some("TankShell") },
    }
}

const ROWS: [UnitRow; 4] = [row(0), row(1), row(2), row(3)];
pub(crate) fn lookup(definition: DefId) -> Option<&'static UnitRow> {
    slot(definition).map(|index| &ROWS[index])
}

const fn attack_row(index: usize) -> crate::ee2_up15_attack::Attack {
    let late = index == 3;
    crate::ee2_up15_attack::Attack {
        id: FIRST + index as DefId, epoch: 10 + index as u8, until: 15, present: true,
        missile: !late, visual_kind: if late { W_MELEE } else { W_SHELL }, min_mobile: 0.0,
        min_immobile: 0.0, angle: 0.52, apply_time: if late { 0.6 } else { 0.5 },
        pack_time: 1.0, unpack_time: 1.0, splash: 0.0, friendly: 0.0, latent: false,
        auto_target: false, attack_on_move: false, ordnance: 0, air: false, mobile: true,
        projectile_speed: if late { None } else { Some(30.0) },
    }
}

const ATTACKS: [crate::ee2_up15_attack::Attack; 4] = [attack_row(0), attack_row(1), attack_row(2), attack_row(3)];
