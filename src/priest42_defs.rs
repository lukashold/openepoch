//! Priest production from the active UP CSV5 and Priest/Human declarations.

use crate::defs::{Def, DefId, A_INF, LAND, NOW, TEMPLE};
use crate::ee2_units::UnitRow;

pub(crate) const PRIEST: DefId = 85;

const DEFINITION: Def = Def {
    name: "Priester", epoch: 0, until: 15, dom: LAND, building: false, size: 0.25,
    hp: 200.0, speed: 1.15, sight: 5.0, armor: A_INF, cost: [35, 0, 0], time: 45.5,
    pop: 1, made_at: TEMPLE, flags: 0, w: [NOW; 2],
};

pub(crate) const fn append(existing: [Def; 85]) -> [Def; 86] {
    let mut rows = [DEFINITION; 86];
    let mut index = 0;
    while index < existing.len() {
        rows[index] = existing[index];
        index += 1;
    }
    rows
}

const EARLY: UnitRow = UnitRow {
    id: PRIEST, epoch: 0, until: 3, source_epoch: 1, source_unit: "Priest",
    source_visual: "Priest", source_host: "Temple", identity_verified: true, model_verified: true,
    hp: Some(200.0), buildtime: Some(45.5), cost9: Some([35, 0, 0, 0, 65, 0, 0, 0, 0]),
    pop: Some(1), pop_capacity: None, sight: Some(5.0), raw_size: Some([0.125, 0.125]),
    raw_footprint: None, raw_speed: Some(1.15), gather: None, farm_rate: None, carry: None,
    farm_carry: None, repair_rate: None, damage: None, range: None, reload: None,
    garrison_slots: None, generation: None, generation_resource: None, garrison_heal: None,
    garrison_attack_bonus: None, garrison_size: Some(1), fortress_garrison: true, attack_missile: None,
};

const ROWS: [UnitRow; 5] = [
    EARLY,
    UnitRow { epoch: 3, until: 6, source_epoch: 4, source_visual: "Priest_4", hp: Some(260.0),
        cost9: Some([38, 0, 0, 0, 70, 0, 0, 0, 0]), ..EARLY },
    UnitRow { epoch: 6, until: 9, source_epoch: 7, source_visual: "Priest_7", hp: Some(315.0),
        cost9: Some([41, 0, 0, 0, 75, 0, 0, 0, 0]), ..EARLY },
    UnitRow { epoch: 9, until: 12, source_epoch: 10, source_visual: "Priest_10", hp: Some(375.0),
        cost9: Some([44, 0, 0, 0, 80, 0, 0, 0, 0]), ..EARLY },
    UnitRow { epoch: 12, until: 15, source_epoch: 13, source_visual: "Priest_13", hp: Some(570.0),
        cost9: Some([47, 0, 0, 0, 85, 0, 0, 0, 0]), ..EARLY },
];

pub(crate) fn lookup(epoch: u8) -> Option<&'static UnitRow> {
    ROWS.get(usize::from(epoch / 3))
}
