//! Active UP Firing Ranges membership, verified after source Range writeback.
//! Source receipt SHA-256 cfb19190dc365726acfcdf7b8a41433238ad5face8469a498ce2f2fbdc744a08.

pub(crate) const RANGE_FACTORS: [(u32, f32); 4] = [
    (28, 1.2), // LandUnits_Foot_Ranged
    (29, 1.2), // LandUnits_Mounted_Ranged
    (30, 1.2), // LandUnits_Wheeled_Ranged
    (31, 1.2), // LandUnits_Mechanized_Ranged
];

// Definition, first zero-based epoch, exclusive end, visual descriptor token.
const INTERVALS: &[(u16, u8, u8, u32)] = &[
    (9, 6, 15, 28), // HeavyInfantry1
    (10, 0, 15, 28), // LightInfantry
    (11, 10, 15, 28), // HeavyInfantry2
    (12, 0, 15, 28), // LightInfantry
    (13, 6, 15, 28), // HeavyInfantry1
    (14, 0, 15, 28), // LightInfantry
    (15, 6, 15, 28), // HeavyInfantry1
    (16, 6, 15, 28), // HeavyInfantry1
    (17, 6, 15, 28), // HeavyInfantry1
    (20, 1, 15, 30), // HeavyArtillery
    (21, 1, 15, 29), // LightMounted1
    (22, 0, 15, 30), // LightArtillery1
    (23, 10, 15, 31), // Tank
    (24, 1, 15, 30), // HeavyArtillery
    (25, 10, 15, 30), // LightArtillery2
    (26, 10, 15, 31), // HeavyTank
    (27, 0, 15, 30), // LightArtillery1
    (28, 10, 15, 30), // LightArtillery2
    (43, 13, 15, 31), // LightMounted3
    (46, 10, 15, 31), // HeavyTank
    (57, 12, 15, 28), // Scout
    (62, 10, 15, 31), // UniqueUnitAmerican3
    (64, 5, 15, 29), // UniqueUnitBabylonian2
    (65, 5, 15, 30), // UniqueUnitChinese2
    (66, 10, 15, 31), // UniqueUnitChinese3
    (67, 0, 15, 29), // UniqueUnitEgyptian1
    (68, 5, 15, 29), // UniqueUnitEgyptian2
    (69, 10, 15, 29), // UniqueUnitEgyptian3
    (72, 10, 15, 31), // UniqueUnitGerman3
    (74, 10, 15, 30), // UniqueUnitInca3
    (76, 5, 15, 30), // UniqueUnitKorean2
    (77, 5, 15, 30), // UniqueUnitMayan2
    (78, 10, 15, 30), // UniqueUnitMayan3
    (crate::heavy_mounted3_61::ID, 13, 15, 31), // HeavyMounted3
];

pub(crate) fn descriptor(def: u16, epoch: u8) -> u32 {
    INTERVALS.iter().find(|&&(id, first, until, _)| id == def && first <= epoch && epoch < until)
        .map_or(0, |&(_, _, _, token)| token)
}
