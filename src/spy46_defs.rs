//! Spy training from the active UP CSV5 and the Spy/Human declarations.

use crate::defs::{Def, DefId, A_INF, LAND, NOW, UNIVERSITY};
use crate::ee2_units::UnitRow;

pub(crate) const SPY: DefId = 86;

const DEFINITION: Def = Def {
    name: "Spion", epoch: 0, until: 15, dom: LAND, building: false, size: 0.25,
    hp: 75.0, speed: 1.875, sight: 7.0, armor: A_INF, cost: [150, 0, 0], time: 45.5,
    pop: 1, made_at: UNIVERSITY, flags: 0, w: [NOW; 2],
};

pub(crate) const fn append(existing: [Def; 86]) -> [Def; 87] {
    let mut rows = [DEFINITION; 87];
    let mut index = 0;
    while index < existing.len() {
        rows[index] = existing[index];
        index += 1;
    }
    rows
}

const EARLY: UnitRow = UnitRow {
    id: SPY, epoch: 0, until: 3, source_epoch: 1, source_unit: "Spy",
    source_visual: "Spy", source_host: "University", identity_verified: true, model_verified: false,
    hp: Some(75.0), buildtime: Some(45.5), cost9: Some([150, 0, 0, 0, 100, 0, 0, 0, 0]),
    pop: Some(1), pop_capacity: None, sight: Some(7.0), raw_size: Some([0.125, 0.125]),
    raw_footprint: None, raw_speed: Some(1.875), gather: None, farm_rate: None, carry: None,
    farm_carry: None, repair_rate: None, damage: None, range: None, reload: None,
    garrison_slots: None, generation: None, generation_resource: None, garrison_heal: None,
    garrison_attack_bonus: None, garrison_size: Some(1), fortress_garrison: true, attack_missile: None,
};

const ROWS: [UnitRow; 5] = [
    EARLY,
    UnitRow { epoch: 3, until: 6, source_epoch: 4, source_visual: "Spy_4", hp: Some(100.0), sight: Some(8.0), ..EARLY },
    UnitRow { epoch: 6, until: 9, source_epoch: 7, source_visual: "Spy_7", hp: Some(125.0), sight: Some(9.0), ..EARLY },
    UnitRow { epoch: 9, until: 12, source_epoch: 10, source_visual: "Spy_10", hp: Some(150.0), sight: Some(10.0), ..EARLY },
    UnitRow { epoch: 12, until: 15, source_epoch: 13, source_visual: "Spy_13", hp: Some(175.0), sight: Some(11.0), ..EARLY },
];

pub(crate) fn lookup(epoch: u8) -> Option<&'static UnitRow> {
    ROWS.get(usize::from(epoch / 3))
}
