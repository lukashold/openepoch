//! The installed Crown selectors projected onto the current UP unit IDs.

pub(super) const MOBILE: u16 = 1 << 0;
pub(super) const BUILDING: u16 = 1 << 1;
pub(super) const MILITARY: u16 = 1 << 2;
pub(super) const MILITARY_BUILDING: u16 = 1 << 3;
pub(super) const ECONOMIC_BUILDING: u16 = 1 << 4;
pub(super) const HUMAN: u16 = 1 << 5;
pub(super) const ROAD: u16 = 1 << 6;
pub(super) const CULTURAL: u16 = 1 << 7;
pub(super) const WONDER: u16 = 1 << 8;
pub(super) const NON_BUILDING: u16 = 1 << 9;

pub(super) const MASKS: [u16; 90] = [
    2, 530, 16, 10, 10, 10, 10, 522, 545, 549,
    549, 549, 549, 549, 549, 549, 549, 549, 517, 517,
    517, 517, 517, 517, 517, 513, 517, 517, 513, 513,
    513, 513, 513, 513, 513, 513, 513, 513, 513, 513,
    513, 513, 513, 517, 513, 513, 517, 18, 10, 10,
    18, 18, 18, 2, 2, 2, 522, 545, 513, 522,
    522, 545, 517, 517, 517, 517, 517, 517, 517, 517,
    517, 517, 517, 517, 517, 517, 517, 517, 517, 517,
    513, 513, 513, 513, 513, 673, 673, 513, 513, 513,
];
