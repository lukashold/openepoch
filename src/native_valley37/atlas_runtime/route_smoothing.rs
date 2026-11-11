//! Ordinary ground waypoint selection from the quadtree route and segment costs.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Waypoint {
    pub position: [f32; 2],
    pub tag: u32,
    pub cumulative_cost: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub admitted: bool,
    pub cost: f32,
}

fn is_road(tag: u32) -> bool {
    matches!(tag, 0x32 | 0x33)
}

fn is_boundary(tag: u32) -> bool {
    matches!(tag & 0xffff, 0x23..=0x25)
}

/// The incoming costs are from the graph search. `segment` reports the cost of
/// a candidate shortcut, or `None` when the atlas or its policy is unavailable.
/// Road geometry is processed by the caller before this selection stage.
pub fn select(
    path: &[Waypoint],
    mut segment: impl FnMut(Waypoint, Waypoint) -> Option<Segment>,
) -> Option<Vec<usize>> {
    let last = path.len().checked_sub(1)?;
    let mut kept = Vec::with_capacity(path.len());
    let mut current = 0;
    loop {
        kept.push(current);
        if current == last {
            return Some(kept);
        }
        let next = current + 1;
        if is_road(path[next].tag) || is_boundary(path[current].tag) != is_boundary(path[next].tag) {
            current = next;
            continue;
        }

        let end = path[next..].iter().position(|point| is_road(point.tag));
        let mut high = end.map_or(last, |offset| next + offset);
        let mut low = next + 1;
        let mut furthest = next;
        while low <= high {
            let candidate = low + (high - low) / 2;
            let shortcut = segment(path[current], path[candidate])?;
            let original_cost = f64::from(path[candidate].cumulative_cost)
                - f64::from(path[current].cumulative_cost);
            if shortcut.admitted && f64::from(shortcut.cost) <= original_cost {
                furthest = candidate;
                low = candidate + 1;
            } else {
                high = candidate - 1;
            }
        }
        current = furthest;
    }
}
