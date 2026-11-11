//! Localized static update of an initialized atlas with no dynamic anchors or links.

use super::atlas_branch::{collapse, Child, Collapse, Square, DEFAULT_DENSITY, MIXED};
use super::atlas_reader::Precision53;
use super::atlas_static_leaf::{
    classify_static_leaf, write_class, StaticAdmission, StaticLeafProviders,
};
use super::dirty::WorldRect;
use super::initial_atlas::{InitialAtlas, Node, ROOT_SLOT_LEVEL};

#[derive(Clone)]
pub struct LocalTree {
    square: Square,
    class: u8,
    density: f32,
    children: Option<Box<[Self; 4]>>,
}

struct LocalUpdate<'a, P> {
    dirty: WorldRect,
    vertices: [u16; 2],
    territory: bool,
    precision: Precision53,
    providers: &'a mut P,
    classifications: usize,
}

impl LocalTree {
    pub fn from_initial(initial: &InitialAtlas, index: usize) -> Self {
        let row = initial.nodes[index];
        Self {
            square: row.square,
            class: row.class,
            density: row.density,
            children: row
                .children
                .map(|children| Box::new(children.map(|id| Self::from_initial(initial, id)))),
        }
    }

    pub fn update(
        &mut self,
        dirty: WorldRect,
        dimensions: [u16; 2],
        territory: bool,
        precision: Precision53,
        providers: &mut impl StaticLeafProviders,
    ) -> usize {
        let mut update = LocalUpdate {
            dirty,
            vertices: dimensions.map(|axis| axis + 1),
            territory,
            precision,
            providers,
            classifications: 0,
        };
        self.apply(self.class, &mut update);
        update.classifications
    }

    fn apply(
        &mut self,
        mut inherited: u8,
        update: &mut LocalUpdate<'_, impl StaticLeafProviders>,
    ) -> u8 {
        let [x, y] = [self.square.x, self.square.y].map(|axis| f32::from(axis) * 0.5);
        let extent = (1_u32 << self.square.level) as f32 * 0.5;
        let overlaps = update.dirty[0].max(x) < update.dirty[2].min(x + extent)
            && update.dirty[1].max(y) < update.dirty[3].min(y + extent);
        let origin = [self.square.x, self.square.y].map(i32::from);
        if !overlaps {
            if self.children.is_some() || self.class != MIXED {
                return self.class;
            }
            if !matches!(inherited, 3 | 4 | 0x23..=0x29) {
                let write = write_class(
                    inherited,
                    origin,
                    self.square.level,
                    self.square.center(),
                    update.vertices.map(i32::from),
                    update.precision,
                    update.providers,
                );
                self.class = write.class;
                if let Some(density) = write.movement_weight {
                    self.density = density;
                }
                return self.class;
            }
        }
        if self.square.level == 0 && self.children.is_none() {
            update.classifications += 1;
            let Some(StaticAdmission::Terminal(write)) = classify_static_leaf(
                origin.map(|axis| axis as i16),
                0,
                update.vertices,
                update.territory,
                update.precision,
                update.providers,
            ) else {
                panic!("Invalid static finest leaf in localized fixture")
            };
            self.class = write.class;
            if let Some(density) = write.movement_weight {
                self.density = density;
            }
            return self.class;
        }
        let children = self.children.get_or_insert_with(|| {
            if self.class != MIXED {
                inherited = self.class;
            }
            Box::new(self.square.children().unwrap().map(|square| Self {
                square,
                class: MIXED,
                density: DEFAULT_DENSITY,
                children: None,
            }))
        });
        let mut returned = [Child {
            class: MIXED,
            density: DEFAULT_DENSITY,
            anchor: None,
        }; 4];
        for (out, child) in returned.iter_mut().zip(children.iter_mut()) {
            out.class = child.apply(inherited, update);
            out.density = child.density;
        }
        match collapse(self.square.level, returned) {
            Collapse::RetainBranch => self.class = MIXED,
            Collapse::Leaf { class, density, .. } => {
                self.class = class;
                self.density = density;
                self.children = None;
            }
        }
        let tile = origin.map(|axis| axis >> 1);
        let veto = self.class != MIXED
            && self.square.level == 1
            && update.territory
            && (0..2).all(|axis| tile[axis] < i32::from(update.vertices[axis]) - 1)
            && update
                .providers
                .special_value(tile)
                .is_some_and(|id| id != -1);
        if veto {
            MIXED
        } else {
            self.class
        }
    }

    pub fn snapshot(&self, dimensions: [usize; 2]) -> InitialAtlas {
        let mut snapshot = InitialAtlas {
            nodes: Vec::new(),
            aliases: vec![usize::MAX; dimensions[0] * dimensions[1]],
            root_dimensions: dimensions,
            root_return: self.class,
        };
        self.append(&mut snapshot);
        assert!(!snapshot.aliases.contains(&usize::MAX));
        snapshot
    }

    fn append(&self, output: &mut InitialAtlas) -> usize {
        let index = output.nodes.len();
        output.nodes.push(Node {
            square: self.square,
            class: self.class,
            density: self.density,
            children: None,
        });
        let [width, height] = output.root_dimensions;
        for slot in self
            .square
            .alias_slots(ROOT_SLOT_LEVEL, width, height, self.children.is_some())
        {
            output.aliases[slot] = index;
        }
        if let Some(children) = &self.children {
            output.nodes[index].children =
                Some(children.each_ref().map(|child| child.append(output)));
        }
        index
    }
}
