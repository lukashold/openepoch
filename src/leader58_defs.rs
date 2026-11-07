//! Earned Crown leaders from the installed UP progression.

use crate::defs::{Def, DefId, Wpn, A_INF, LAND, NOW, W_MELEE};
use crate::ee2_units::UnitRow;

#[path = "leader_stats58.rs"]
mod stats;

pub(crate) const FIRST: DefId = 87;
pub(crate) const END: DefId = FIRST + 3;
const FAMILIES: [&str; 3] = ["LeaderMilitary", "LeaderEconomic", "LeaderImperial"];
const NAMES: [&str; 3] = ["Militärführer", "Wirtschaftsführer", "Imperialer Anführer"];
const VISUALS: [[&str; 3]; 3] = [
    ["LeaderMilitary", "LeaderMilitary_6", "LeaderMilitary_11"],
    ["LeaderEconomic", "LeaderEconomic_6", "LeaderEconomic_11"],
    ["LeaderImperial", "LeaderImperial_6", "LeaderImperial_11"],
];

pub(crate) fn branch(definition: DefId) -> Option<usize> {
    (FIRST..END).contains(&definition).then(|| usize::from(definition - FIRST))
}

pub(crate) fn hit_delay(definition: DefId, epoch: u8) -> Option<f32> {
    stats::STATS[branch(definition)?].get(epoch as usize).map(|row| row.hit_delay)
}

const fn definition(kind: usize) -> Def {
    let entry = &stats::STATS[kind][0];
    Def {
        name: NAMES[kind], epoch: 0, until: 15, dom: LAND, building: false, size: 0.45,
        hp: entry.hp, speed: entry.movement, sight: entry.sight, armor: A_INF,
        cost: [0; 3], time: 0.0, pop: 0, made_at: DefId::MAX, flags: 0,
        w: [Wpn { range: entry.range, dmg: entry.damage, cd: entry.reload,
            kind: W_MELEE, targets: LAND, splash: 0.0 }, NOW],
    }
}

pub(crate) const fn append(existing: [Def; 87]) -> [Def; 90] {
    let mut rows = [definition(0); 90];
    let mut index = 0;
    while index < existing.len() {
        rows[index] = existing[index];
        index += 1;
    }
    rows[index + 1] = definition(1);
    rows[index + 2] = definition(2);
    rows
}

const fn row(kind: usize, epoch: usize) -> UnitRow {
    let entry = &stats::STATS[kind][epoch];
    UnitRow {
        id: FIRST + kind as DefId, epoch: epoch as u8, until: (epoch + 1) as u8,
        source_epoch: (epoch + 1) as u8, source_unit: FAMILIES[kind], source_visual: VISUALS[kind][epoch / 5],
        source_host: "", identity_verified: true, model_verified: false,
        hp: Some(entry.hp), buildtime: Some(0.0), cost9: Some([0; 9]), pop: Some(0), pop_capacity: None,
        sight: Some(entry.sight), raw_size: Some([0.225, 0.225]), raw_footprint: None,
        raw_speed: Some(entry.movement), gather: None, farm_rate: None, carry: None, farm_carry: None,
        repair_rate: None, damage: Some(entry.damage), range: Some(entry.range), reload: Some(entry.reload),
        garrison_slots: None, generation: None, generation_resource: None, garrison_heal: None,
        garrison_attack_bonus: None, garrison_size: Some(1), fortress_garrison: true, attack_missile: None,
    }
}

const ROWS: [[UnitRow; 15]; 3] = {
    let mut rows = [[row(0, 0); 15]; 3];
    let mut kind = 0;
    while kind < rows.len() {
        let mut epoch = 0;
        while epoch < rows[kind].len() {
            rows[kind][epoch] = row(kind, epoch);
            epoch += 1;
        }
        kind += 1;
    }
    rows
};

pub(crate) fn lookup(definition: DefId, epoch: u8) -> Option<&'static UnitRow> {
    ROWS[branch(definition)?].get(epoch as usize)
}
