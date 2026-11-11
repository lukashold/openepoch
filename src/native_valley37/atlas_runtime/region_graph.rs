//! Build the static region membership and ordered neighbor lists from atlas leaves.

use std::collections::BTreeSet;

pub const UNADMITTED: u16 = 0xfffe;

#[derive(Clone, Copy)]
pub struct Leaf {
    pub origin: [u16; 2],
    pub level: u8,
    pub class: u8,
}

#[derive(Debug)]
pub struct Region {
    pub class: u8,
    pub territory: i32,
    pub adjacent: Vec<u16>,
}

pub struct RegionGraph {
    pub base: Vec<Region>,
    pub groups: [Vec<Region>; 7],
    pub leaves: Vec<u16>,
    pub group_membership: Vec<[u16; 7]>,
}

impl RegionGraph {
    /// Stock terrain and territory regions with no dynamic connector sidecars.
    pub fn build_static(
        level: u8, leaves: &[Leaf], dimensions: [usize; 2], territory: Option<&[i32]>, masks: [u32; 7],
    ) -> Option<Self> {
        let side = 1_usize.checked_shl(u32::from(level))?;
        if side > 512 || leaves.len() > side * side
            || dimensions.into_iter().any(|axis| !(2..=256).contains(&axis))
            || leaves.iter().any(|leaf| !matches!(leaf.class, 0..=7 | 0x27..=0x29))
            || masks.iter().any(|mask| mask & 0x18 != 0)
            || territory.is_some_and(|ids| ids.len() != dimensions[0] * dimensions[1]) {
            return None;
        }
        let neighbors = leaf_neighbors(side, leaves)?;
        let keys: Vec<_> = leaves.iter().map(|leaf| {
            let origin = leaf.origin.map(usize::from);
            let [x, y] = origin.map(|axis| (2 * axis + (1 << leaf.level)) / 4);
            let territory = territory.filter(|_| x < dimensions[0] && y < dimensions[1])
                .map_or(-1, |ids| ids[y * dimensions[0] + x]);
            (origin.map(|axis| axis >> 5), leaf.class, territory)
        }).collect();
        let mut membership = vec![u16::MAX; leaves.len()];
        let mut base = Vec::new();
        for (index, &key) in keys.iter().enumerate() {
            if membership[index] != u16::MAX { continue; }
            let id = region_id(base.len())?;
            fill(index, id, &mut membership, &neighbors, |other| keys[other] == key);
            base.push(Region { class: key.1, territory: key.2, adjacent: Vec::new() });
        }
        let base_pairs = connected_regions(&neighbors, &membership);
        bind_edges(&mut base, base_pairs);

        let base_neighbors = base.iter().map(|region| {
            region.adjacent.iter().copied().map(usize::from).collect::<Vec<_>>()
        }).collect::<Vec<_>>();
        let mut group_membership = vec![[UNADMITTED; 7]; base.len()];
        let mut groups = std::array::from_fn(|_| Vec::new());
        for (group, regions) in groups.iter_mut().enumerate() {
            let admits = |index: usize| {
                let class = base[index].class;
                class < 8 && masks[group] & (1 << class) != 0
            };
            let mut membership: Vec<_> = (0..base.len()).map(|index| {
                if admits(index) { u16::MAX } else { UNADMITTED }
            }).collect();
            for (index, region) in base.iter().enumerate() {
                if membership[index] != u16::MAX { continue; }
                let id = region_id(regions.len())?;
                fill(index, id, &mut membership, &base_neighbors, |other| base[other].territory == region.territory);
                regions.push(Region { class: region.class, territory: region.territory, adjacent: Vec::new() });
            }
            let pairs = connected_regions(&base_neighbors, &membership);
            for (larger, smaller) in pairs {
                regions[usize::from(larger)].adjacent.push(smaller);
                regions[usize::from(smaller)].adjacent.push(larger);
            }
            for region in regions { region.adjacent.reverse(); }
            for (region, id) in group_membership.iter_mut().zip(membership) { region[group] = id; }
        }
        Some(Self { base, groups, leaves: membership, group_membership })
    }
}

fn region_id(index: usize) -> Option<u16> {
    (index < usize::from(UNADMITTED)).then_some(index as u16)
}

fn leaf_neighbors(side: usize, leaves: &[Leaf]) -> Option<Vec<Vec<usize>>> {
    let mut grid = vec![usize::MAX; side * side];
    for (index, leaf) in leaves.iter().enumerate() {
        let [x, y] = leaf.origin.map(usize::from);
        let span = 1_usize.checked_shl(u32::from(leaf.level))?;
        if x + span > side || y + span > side { return None; }
        for row in grid.chunks_exact_mut(side).skip(y).take(span) {
            let cells = &mut row[x..x + span];
            if cells.iter().any(|&cell| cell != usize::MAX) { return None; }
            cells.fill(index);
        }
    }
    if grid.contains(&usize::MAX) { return None; }
    let mut pairs = BTreeSet::new();
    for (index, &cell) in grid.iter().enumerate() {
        for other in [
            (index % side + 1 < side).then_some(index + 1),
            (index + side < grid.len()).then_some(index + side),
        ].into_iter().flatten().map(|other| grid[other]) {
            if cell != other { pairs.insert((cell.min(other), cell.max(other))); }
        }
    }
    let mut neighbors = vec![Vec::new(); leaves.len()];
    for (a, b) in pairs {
        neighbors[a].push(b);
        neighbors[b].push(a);
    }
    Some(neighbors)
}

fn fill(index: usize, id: u16, membership: &mut [u16], neighbors: &[Vec<usize>], admits: impl Fn(usize) -> bool) {
    let mut queue = vec![index];
    membership[index] = id;
    while let Some(current) = queue.pop() {
        for &other in &neighbors[current] {
            if membership[other] == u16::MAX && admits(other) {
                membership[other] = id;
                queue.push(other);
            }
        }
    }
}

fn connected_regions(neighbors: &[Vec<usize>], membership: &[u16]) -> BTreeSet<(u16, u16)> {
    let mut pairs = BTreeSet::new();
    for (index, adjacent) in neighbors.iter().enumerate() {
        let a = membership[index];
        if a >= UNADMITTED { continue; }
        for &other in adjacent {
            let b = membership[other];
            if b < UNADMITTED && a != b { pairs.insert((a.max(b), a.min(b))); }
        }
    }
    pairs
}

fn blocked(class: u8) -> bool {
    matches!(class, 3 | 4 | 0x27..=0x29)
}

fn bind_edges(regions: &mut [Region], pairs: BTreeSet<(u16, u16)>) {
    let mut pairs = pairs.into_iter().peekable();
    for larger in 0..regions.len() {
        let mut directed = Vec::new();
        while pairs.peek().is_some_and(|pair| usize::from(pair.0) == larger) {
            let (a, b) = pairs.next().unwrap();
            let smaller = usize::from(b);
            match (blocked(regions[larger].class), blocked(regions[smaller].class)) {
                (true, false) => directed.push(b),
                (false, true) => regions[smaller].adjacent.push(a),
                _ => {
                    regions[larger].adjacent.push(b);
                    regions[smaller].adjacent.push(a);
                }
            }
        }
        regions[larger].adjacent.extend(directed);
    }
    for region in regions { region.adjacent.reverse(); }
}
