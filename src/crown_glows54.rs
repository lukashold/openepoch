//! Crown visual descriptor bindings for the current UP roster.

use crate::defs::DefId;

pub(super) enum Selector { Units(&'static [DefId]), Convertible, FiringRange }

pub(super) struct Visual {
    pub row: u32,
    pub selector: Selector,
    pub city: bool,
    pub enemy: bool,
    pub owner_only: bool,
    pub glow: u32,
}

pub(super) fn rows(branch: usize, power: u8) -> &'static [Visual] {
    match (branch, power) {
        (0, 0) => &[
            Visual { row: 1, selector: Selector::Units(&[37, 39, 44]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 1) => &[
            Visual { row: 2, selector: Selector::Units(&[29, 42]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 2) => &[
            Visual { row: 3, selector: Selector::Units(&[20, 22, 24, 27, 65, 74, 76, 77, 78]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 3) => &[
            Visual { row: 4, selector: Selector::Units(&[9, 10, 11, 12, 13, 14, 15, 16, 17]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 4) => &[
            Visual { row: 5, selector: Selector::Units(&[18, 19, 21, 23, 26, 43, 46, 62, 63, 64, 66, 67, 68, 69, 70, 71, 72, 73, 75, 79]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 5) => &[
            Visual { row: 6, selector: Selector::Units(&[29, 30, 31, 32, 33, 34, 35, 36, 45, 58, 84]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 6) => &[
            Visual { row: 7, selector: Selector::Units(&[7, 56, 59, 60]),
                city: false, enemy: false, owner_only: true, glow: 0 },
        ],
        (0, 7) => &[
            Visual { row: 8, selector: Selector::Units(&[38, 40]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (0, 8) => &[
            Visual { row: 9, selector: Selector::Units(&[]),
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 10, selector: Selector::Units(&[83]),
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 11, selector: Selector::Units(&[41]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 0) => &[
            Visual { row: 12, selector: Selector::Units(&[6]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 1) => &[
            Visual { row: 13, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 2) => &[
            Visual { row: 14, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 3) => &[
            Visual { row: 15, selector: Selector::Units(&[0]),
                city: false, enemy: false, owner_only: true, glow: 0 },
        ],
        (1, 4) => &[
            Visual { row: 16, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 5) => &[
            Visual { row: 17, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 6) => &[
            Visual { row: 18, selector: Selector::Units(&[50]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 7) => &[
            Visual { row: 19, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 8) => &[
            Visual { row: 20, selector: Selector::Units(&[8]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 9) => &[
            Visual { row: 21, selector: Selector::Units(&[5]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (1, 10) => &[
            Visual { row: 22, selector: Selector::Units(&[3, 4, 48, 49]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 0) => &[
            Visual { row: 23, selector: Selector::Units(&[0, 1, 3, 4, 5, 6, 7, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 59, 60]),
                city: true, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 1) => &[
            Visual { row: 24, selector: Selector::Units(&[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 43, 46, 57, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 85, 86]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 2) => &[
            Visual { row: 25, selector: Selector::Units(&[0]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 3) => &[
            Visual { row: 26, selector: Selector::Units(&[29, 30, 31, 32, 33, 34, 35, 36, 42, 45, 58, 82, 84]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 4) => &[
            Visual { row: 27, selector: Selector::Convertible,
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 5) => &[
            Visual { row: 28, selector: Selector::FiringRange,
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 29, selector: Selector::FiringRange,
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 30, selector: Selector::FiringRange,
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 31, selector: Selector::FiringRange,
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 6) => &[
            Visual { row: 32, selector: Selector::Units(&[85]),
                city: false, enemy: false, owner_only: true, glow: 1 },
            Visual { row: 33, selector: Selector::Units(&[51]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 7) => &[
            Visual { row: 34, selector: Selector::Units(&[29, 30, 31, 32, 33, 34, 35, 36, 42, 45, 58, 82, 84]),
                city: false, enemy: true, owner_only: false, glow: 0 },
        ],
        (2, 8) => &[
            Visual { row: 36, selector: Selector::Units(&[0, 5, 6, 56]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 9) => &[
            Visual { row: 37, selector: Selector::Units(&[57, 61, 86]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        (2, 10) => &[
            Visual { row: 38, selector: Selector::Units(&[47, 51]),
                city: false, enemy: false, owner_only: true, glow: 1 },
        ],
        _ => &[],
    }
}
