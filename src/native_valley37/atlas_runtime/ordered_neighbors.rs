//! Four-sided terminal adjacency for an ordinary atlas leaf.

use super::atlas_ray::Leaf;

pub fn edges(source: Leaf, mut locate: impl FnMut([i32; 2]) -> Option<Leaf>) -> Option<Vec<Leaf>> {
    let [x, y] = source.origin_half.map(i32::from);
    let size = 1_i32.checked_shl(u32::from(source.level))?;
    let mut edges = Vec::with_capacity(4);
    for (start, axis) in [
        ([x - 1, y], 1),
        ([x + size, y], 1),
        ([x, y - 1], 0),
        ([x, y + size], 0),
    ] {
        let mut offset = 0;
        while offset < size {
            let mut point = start;
            point[axis] += offset;
            let leaf = locate(point)?;
            offset += 1_i32.checked_shl(u32::from(leaf.level))?;
            edges.push(leaf);
        }
    }
    Some(edges)
}
