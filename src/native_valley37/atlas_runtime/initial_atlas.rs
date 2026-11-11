//! Full original initial postorder movement atlas, with static providers.

use super::atlas_branch::{collapse, Child, Collapse, Square, DEFAULT_DENSITY, MIXED};
use super::atlas_reader::Precision53;
use super::atlas_static_leaf::{classify_static_leaf, StaticAdmission, StaticLeafProviders};

pub const ROOT_SLOT_LEVEL: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub square: Square,
    pub class: u8,
    pub density: f32,
    pub children: Option<[usize; 4]>,
}

pub struct InitialAtlas {
    pub nodes: Vec<Node>,
    pub aliases: Vec<usize>,
    pub root_dimensions: [usize; 2],
    pub root_return: u8,
}

struct Tree {
    square: Square,
    class: u8,
    outward_class: u8,
    density: f32,
    children: Option<Box<[Tree; 4]>>,
}

pub fn build(
    dimensions: [u16; 2],
    territory_enabled: bool,
    mode: Precision53,
    providers: &mut impl StaticLeafProviders,
) -> Option<InitialAtlas> {
    if dimensions.into_iter().any(|axis| !(2..=256).contains(&axis)) { return None; }
    let spans = dimensions.map(|axis| (usize::from(axis) * 2).next_power_of_two());
    let level = spans.into_iter().max()?.ilog2() as u8;
    let root_dimensions = spans.map(|span| span >> ROOT_SLOT_LEVEL);
    let root = Square { level, x: 0, y: 0 };
    let tree = construct(root, dimensions.map(|axis| axis + 1), territory_enabled, mode, providers)?;
    let mut atlas = InitialAtlas {
        nodes: Vec::new(), aliases: vec![usize::MAX; root_dimensions[0] * root_dimensions[1]],
        root_dimensions, root_return: tree.outward_class,
    };
    atlas.append(tree);
    if atlas.aliases.contains(&usize::MAX) { return None; }
    Some(atlas)
}

fn construct(
    square: Square,
    vertices: [u16; 2],
    territory_enabled: bool,
    mode: Precision53,
    providers: &mut impl StaticLeafProviders,
) -> Option<Tree> {
    if square.level == 0 {
        let origin = [square.x as i16, square.y as i16];
        let StaticAdmission::Terminal(write) = classify_static_leaf(origin, 0, vertices, territory_enabled, mode, providers)? else {
            return None;
        };
        return Some(Tree { square, class: write.class, outward_class: write.class,
                           density: write.movement_weight.unwrap_or(DEFAULT_DENSITY), children: None });
    }
    let [a, b, c, d] = square.children()?;
    let children = [construct(a, vertices, territory_enabled, mode, providers)?,
                    construct(b, vertices, territory_enabled, mode, providers)?,
                    construct(c, vertices, territory_enabled, mode, providers)?,
                    construct(d, vertices, territory_enabled, mode, providers)?];
    let summaries = children.each_ref().map(|child| Child {class: child.outward_class, density: child.density, anchor: None});
    let (class, density, children) = match collapse(square.level, summaries) {
        Collapse::RetainBranch => (MIXED, DEFAULT_DENSITY, Some(Box::new(children))),
        Collapse::Leaf {class, density, ..} => (class, density, None),
    };
    let tile = [i32::from(square.x >> 1), i32::from(square.y >> 1)];
    let veto = class != MIXED && square.level == 1 && territory_enabled
        && (0..2).all(|axis| tile[axis] < i32::from(vertices[axis]) - 1)
        && providers.special_value(tile).is_some_and(|id| id != -1);
    let outward_class = if veto { MIXED } else { class };
    Some(Tree { square, class, outward_class, density, children })
}

impl InitialAtlas {
    fn append(&mut self, tree: Tree) -> usize {
        let index = self.nodes.len();
        self.nodes.push(Node { square: tree.square, class: tree.class, density: tree.density, children: None });
        let branched = tree.children.is_some();
        let [width, height] = self.root_dimensions;
        for slot in tree.square.alias_slots(ROOT_SLOT_LEVEL, width, height, branched) {
            self.aliases[slot] = index;
        }
        if let Some(children) = tree.children {
            let children = (*children).map(|child| self.append(child));
            self.nodes[index].children = Some(children);
        }
        index
    }
}
