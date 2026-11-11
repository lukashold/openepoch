//! Pristine territory border and level-one atlas veto inputs.
const OFFSETS: [[i32; 2]; 8] = [
    [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1], [-1, -1],
];
const VARIATION: [u8; 8] = [2, 1, 1, 0, 0, 3, 3, 2];
const SINGLE_GAP_PAIRS: [[[usize; 2]; 2]; 4] = [
    [[4, 3], [2, 3]], [[0, 1], [2, 1]], [[0, 7], [6, 7]], [[4, 5], [6, 5]],
];

pub fn descriptors(pattern: u8) -> Vec<[u8; 3]> {
    if matches!(pattern, 0 | 255) { return Vec::new(); }
    let covered = |index| pattern & (1_u8 << index) != 0;
    let mut inside = covered(0);
    let mut first = if inside && !covered(7) { Some(0) } else { None };
    let mut last = if !inside && covered(7) { Some(7) } else { None };
    let mut result = Vec::with_capacity(4);
    let mut emit = |end: usize, start: usize| {
        let gap = (start + 7 - end) % 8;
        let side = if gap == 1 { (start + 7) % 8 } else { 8 };
        // Even jump-table entries execute the ADD at the start of the case.
        let kind = gap as u8 + if start % 2 == 0 { 7 } else { 0 };
        result.push([kind, VARIATION[start], side as u8]);
    };
    for index in 0..8 {
        let next = covered(index);
        if next && !inside {
            if let Some(end) = last.take() { emit(end, index); }
            else { first = Some(index); }
        } else if !next && inside {
            last = Some(index - 1);
        }
        inside = next;
    }
    if !inside {
        emit(last.unwrap(), first.unwrap());
    }
    result
}

pub struct TerritoryBorders<'a> {
    dimensions: [usize; 2],
    ids: &'a [i32],
}

impl<'a> TerritoryBorders<'a> {
    pub fn new(dimensions: [usize; 2], ids: &'a [i32]) -> Option<Self> {
        if dimensions.iter().any(|&axis| axis == 0 || axis > i32::MAX as usize) ||
            dimensions[0].checked_mul(dimensions[1])? != ids.len()
        {
            return None;
        }
        Some(Self { dimensions, ids })
    }

    fn id(&self, [x, y]: [i32; 2]) -> i32 {
        if x < 0 || y < 0 || x as usize >= self.dimensions[0] || y as usize >= self.dimensions[1] {
            return -1;
        }
        self.ids[y as usize * self.dimensions[0] + x as usize]
    }

    pub fn query(&self, tile: [i32; 2], table: &[Vec<[u8; 3]>; 256]) -> (u8, Option<i32>) {
        let own = self.id(tile);
        if own < 1 { return (255, None); }
        let mut same = 0_u8;
        let mut unassigned = 0_u8;
        for (index, offset) in OFFSETS.into_iter().enumerate() {
            let neighbor = self.id([tile[0] + offset[0], tile[1] + offset[1]]);
            if neighbor == -1 { unassigned |= 1 << index; }
            if neighbor == own || neighbor == -1 { same |= 1 << index; }
        }
        let exposed_unknown = (!same).rotate_left(1) | (!same).rotate_right(1);
        let pattern = same & !(unassigned & exposed_unknown);
        let valid = table[usize::from(pattern)].iter().any(|&[kind, variation, _]| {
            kind != 8 || !SINGLE_GAP_PAIRS[usize::from(variation)].iter().any(|&[a, b]| {
                self.id([tile[0] + OFFSETS[a][0] + OFFSETS[b][0], tile[1] + OFFSETS[a][1] + OFFSETS[b][1]]) == own
            })
        });
        (pattern, valid.then_some(own))
    }
}

pub fn control(encoded: usize) -> [i32; 9] {
    let mut ids = [1; 9];
    let mut digits = encoded;
    for [x, y] in OFFSETS {
        ids[(y + 1) as usize * 3 + (x + 1) as usize] = match digits % 3 {
            0 => 1,
            1 => 2,
            _ => -1,
        };
        digits /= 3;
    }
    ids
}
