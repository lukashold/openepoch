//! Numeric Crown descriptors for the current UP roster.

use crate::defs::DefId;

#[derive(Clone, Copy)]
pub(super) enum Stat {
    Hp, Attack, Speed, Range, Sight, Capacity, Population, ProductionCost, ProductionTime, UnitCost,
}

pub(super) struct Modifier {
    pub stat: Stat,
    pub units: &'static [DefId],
    pub scale: f32,
}

pub(super) struct Power {
    pub first_epoch: u8,
    pub last_epoch: u8,
    pub modifiers: &'static [Modifier],
}

pub(super) fn get(kind: usize, index: u8) -> Option<&'static Power> {
    Some(match (kind, index) {
        (0, 0) => &Power { // AirSuperiorityDoctrine
            first_epoch: 11, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[37, 39, 44], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[37, 39, 44], scale: 1.2 },
            ],
        },
        (0, 1) => &Power { // AmphibiousWarfareDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[29, 42], scale: 1.5 },
                Modifier { stat: Stat::Speed, units: &[29, 42], scale: 1.5 },
            ],
        },
        (0, 2) => &Power { // FireSupportDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[20, 22, 24, 27, 65, 74, 76, 77, 78], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[20, 22, 24, 27, 65, 74, 76, 77, 78], scale: 1.2 },
            ],
        },
        (0, 3) => &Power { // InfantryDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[9, 10, 11, 12, 13, 14, 15, 16, 17], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[9, 10, 11, 12, 13, 14, 15, 16, 17], scale: 1.2 },
            ],
        },
        (0, 4) => &Power { // MobileWarfareDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[18, 19, 21, 23, 26, 43, 46, 62, 63, 64, 66, 67, 68, 69, 70, 71, 72, 73, 75, 79], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[18, 19, 21, 23, 26, 43, 46, 62, 63, 64, 66, 67, 68, 69, 70, 71, 72, 73, 75, 79], scale: 1.2 },
            ],
        },
        (0, 5) => &Power { // SeaLanesDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[29, 30, 31, 32, 33, 34, 35, 36, 45, 58, 84], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[29, 30, 31, 32, 33, 34, 35, 36, 45, 58, 84], scale: 1.2 },
            ],
        },
        (0, 6) => &Power { // StaticDefenseDoctrine
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[7, 56, 59, 60], scale: 1.2 },
                Modifier { stat: Stat::Range, units: &[7, 56, 59, 60], scale: 1.2 },
                Modifier { stat: Stat::Sight, units: &[], scale: 2.0 },
            ],
        },
        (0, 7) => &Power { // StrategicBombingDoctrine
            first_epoch: 11, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[38, 40], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[38, 40], scale: 1.2 },
            ],
        },
        (0, 8) => &Power { // VerticalEnvelopmentDoctrine
            first_epoch: 11, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Hp, units: &[], scale: 1.5 },
                Modifier { stat: Stat::Capacity, units: &[], scale: 1.25 },
                Modifier { stat: Stat::Attack, units: &[83], scale: 1.2 },
                Modifier { stat: Stat::Hp, units: &[83], scale: 1.2 },
                Modifier { stat: Stat::Attack, units: &[41], scale: 1.2 },
                Modifier { stat: Stat::Hp, units: &[41], scale: 1.2 },
            ],
        },
        (1, 0) => &Power { // AirSuperiority
            first_epoch: 11, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::ProductionTime, units: &[6], scale: 0.8 },
                Modifier { stat: Stat::ProductionCost, units: &[6], scale: 0.8 },
            ],
        },
        (1, 1) => &Power { // Farming
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 2) => &Power { // FoodProcessing
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 3) => &Power { // Immigration
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 4) => &Power { // IncomeTax
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 5) => &Power { // Lumbering
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 6) => &Power { // MarketStability
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 7) => &Power { // Quarrying
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 8) => &Power { // Retooling
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (1, 9) => &Power { // RuleTheWaves
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::ProductionCost, units: &[5], scale: 0.8 },
                Modifier { stat: Stat::ProductionTime, units: &[5], scale: 0.8 },
            ],
        },
        (1, 10) => &Power { // WarEconomy
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::ProductionCost, units: &[3, 4, 48, 49], scale: 0.9 },
                Modifier { stat: Stat::ProductionTime, units: &[3, 4, 48, 49], scale: 0.9 },
            ],
        },
        (2, 0) => &Power { // Centralization
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (2, 1) => &Power { // Drills
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Speed, units: &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 43, 46, 57, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 85, 86], scale: 1.2 },
            ],
        },
        (2, 2) => &Power { // Expansionism
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Population, units: &[0], scale: 1.3 },
            ],
        },
        (2, 3) => &Power { // Exploration
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Sight, units: &[29, 30, 31, 32, 33, 34, 35, 36, 42, 45, 58, 82, 84], scale: 1.25 },
                Modifier { stat: Stat::Speed, units: &[29, 30, 31, 32, 33, 34, 35, 36, 42, 45, 58, 82, 84], scale: 1.25 },
            ],
        },
        (2, 4) => &Power { // Fanaticism
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (2, 5) => &Power { // FiringRanges
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (2, 6) => &Power { // Missionaries
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::UnitCost, units: &[85], scale: 0.75 },
                Modifier { stat: Stat::Speed, units: &[85], scale: 1.25 },
            ],
        },
        (2, 7) => &Power { // Piracy
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (2, 8) => &Power { // SocializedMedicine
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        (2, 9) => &Power { // SuperSpies
            first_epoch: 1, last_epoch: 15, modifiers: &[
                Modifier { stat: Stat::Speed, units: &[57, 61, 86], scale: 1.25 },
                Modifier { stat: Stat::Hp, units: &[57, 61, 86], scale: 1.25 },
            ],
        },
        (2, 10) => &Power { // ThinkTanks
            first_epoch: 1, last_epoch: 15, modifiers: &[
            ],
        },
        _ => return None,
    })
}
