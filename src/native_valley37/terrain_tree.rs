//! Initial bounds for the authenticated, cliff-free authored map.
use super::{active_game, Geometry, STAGE};

const NO_CHILD: i32 = i32::MAX;

#[derive(Clone, Copy, Debug)]
struct Bounds {
    low: f32,
    high: f32,
}

impl Bounds {
    fn include(&mut self, next: Self) {
        if next.low < self.low { self.low = next.low; }
        if next.high > self.high { self.high = next.high; }
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    children: [i32; 4],
    bounds: Bounds,
}

#[derive(Clone, Debug)]
pub(super) struct Tree {
    nodes: Vec<Node>,
    size: usize,
    root: i32,
}

// 70CF00 derives x,y by dividing by the cell width but samples heights
// from a separate row stride one corner wider.
fn cell_bounds(world: &Geometry, x: usize, y: usize) -> Bounds {
    let stride = world.width + 1;
    let row = y * stride + x;
    let corners = [row, row + 1, row + stride, row + stride + 1];
    let height = world.heights[corners[0]];
    let mut bounds = Bounds {low: height, high: height};
    for at in corners.into_iter().skip(1) {
        let height = world.heights[at];
        bounds.include(Bounds {low: height, high: height});
    }
    bounds
}

impl Tree {
    fn build(&mut self, world: &Geometry, x: usize, y: usize, size: usize) -> (i32, Bounds) {
        if size == 1 {
            return (!((y * world.width + x) as i32), cell_bounds(world, x, y));
        }
        // 6EFC60 allocates the parent before visiting valid children in
        // source order. Missing siblings retain the native sentinel.
        let index = self.nodes.len();
        let placeholder = Bounds {low: 0.0, high: 0.0};
        self.nodes.push(Node {children: [NO_CHILD; 4], bounds: placeholder});
        let half = size / 2;
        let (first, mut bounds) = self.build(world, x, y, half);
        let mut children = [first, NO_CHILD, NO_CHILD, NO_CHILD];
        for (slot, (cx, cy)) in [(x + half, y), (x, y + half), (x + half, y + half)].into_iter().enumerate() {
            if cx >= world.width || cy >= world.height { continue; }
            let (child, extent) = self.build(world, cx, cy, half);
            children[slot + 1] = child;
            bounds.include(extent);
        }
        self.nodes[index] = Node {children, bounds};
        (index as i32, bounds)
    }
}

impl Geometry {
    pub(super) fn with_initial_terrain_tree(mut self) -> Option<Self> {
        if self.source_cliffs != 0 || !self.surfaces.is_empty() { return None; }
        let size = self.width.max(self.height).next_power_of_two();
        let mut tree = Tree {nodes: Vec::new(), size, root: NO_CHILD};
        tree.root = tree.build(&self, 0, 0, size).0;
        self.initial_terrain_tree = Some(tree);
        Some(self)
    }
}

fn word(world: &Geometry, record: u32, index: usize, field: usize) -> Option<u32> {
    let tree = world.initial_terrain_tree.as_ref()?;
    let cell_count = world.width * world.height;
    match record {
        0 => [tree.root as u32, tree.nodes.len() as u32, cell_count as u32, tree.size as u32].get(field).copied(),
        1 if index < cell_count => {
            let [x, y] = [index % world.width, index / world.width];
            let bounds = cell_bounds(world, x, y);
            [x as f32, y as f32, bounds.low, (x + 1) as f32, (y + 1) as f32, bounds.high].get(field).map(|f| f.to_bits())
        }
        2 => {
            let node = tree.nodes.get(index)?;
            if field < 4 { return Some(node.children[field] as u32); }
            [node.bounds.low, node.bounds.high].get(field - 4).map(|f| f.to_bits())
        }
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn valley56_initial_terrain_word(prepared: u32, record: u32, index: usize, field: usize) -> u32 {
    let query = |game: &crate::game::Game| word(game.map.source_world.as_deref()?, record, index, field);
    match prepared {
        0 => active_game().and_then(query),
        1 => STAGE.with(|stage| stage.borrow().game.as_ref().and_then(query)),
        _ => None,
    }.unwrap_or(u32::MAX)
}

// 5A3290 counts the two fresh ordinary owners and divides the live match's
// GameOptions+3C maximum. 5E3FA0 bounds the uncapped signed result at 9999.
// The QA preview's own settings do not prove the original live maximum.
#[no_mangle]
pub extern "C" fn valley56_source_seat_maximum(global_maximum: u32) -> u32 {
    if global_maximum == 0 || global_maximum > i32::MAX as u32 ||
        super::startup_checkpoint::active_authored_test().is_none() { return u32::MAX; }
    (global_maximum / 2).min(9999)
}
