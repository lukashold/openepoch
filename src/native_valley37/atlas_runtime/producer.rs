//! Prepared Valley atlas plus portable local update over ordinary stationary actors.

use super::atlas_branch::{MIXED, Square};
use super::atlas_reader::Precision53;
use super::atlas_static_leaf::{SearchBox, StaticLeafProviders};
use super::dirty::{WorldRect, obstacle_event, raster_dispatch};
use super::grassfire::{self, HalfCellRect};
use super::initial_atlas::InitialAtlas;
use super::localized::LocalTree;
use super::territory_borders::{TerritoryBorders, descriptors};
use std::collections::BTreeMap;

pub const DIMENSIONS: [usize; 2] = [115, 71];

#[derive(Clone, Copy, Debug)]
pub struct StationaryActor {
    pub id: u32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub full_size: [f32; 2],
    pub height: f32,
    pub blocks: bool,
    pub transparent: bool,
}

#[derive(Clone, Copy)]
struct Shape {
    center: [f32; 3],
    axes: [[f32; 3]; 3],
    half: [f32; 3],
    actor: StationaryActor,
}

impl Shape {
    fn new(actor: StationaryActor) -> Option<Self> {
        if actor.id == 0
            || actor.position.iter().any(|v| !v.is_finite())
            || (0..2).any(|i| !(0.0..DIMENSIONS[i] as f32).contains(&actor.position[i]))
            || actor.position[2].abs() > 32.0
            || !actor.yaw.is_finite()
            || actor.full_size.iter().any(|&v| !(0.02..=64.0).contains(&v))
            || !(0.0..=32.0).contains(&actor.height)
        {
            return None;
        }
        let (sin, cos) = f64::from(actor.yaw).sin_cos();
        let (sin, cos) = (sin as f32, cos as f32);
        let height = actor.height * 0.5;
        let [x, y, z] = actor.position;
        let planar = actor
            .full_size
            .map(|v| (f64::from(v) * 0.5 - f64::from(0.01_f32)) as f32);
        Some(Self {
            center: [x, y, z + height],
            axes: [[cos, sin, 0.], [-sin, cos, 0.], [0., 0., 1.]],
            half: [planar[0], planar[1], height],
            actor,
        })
    }

    fn projected(self, mode: Precision53) -> WorldRect {
        // Native 006D98F0 stores three scaled axes and each accumulated
        // component at f32 while it enumerates the eight OBB corners.
        let scaled = std::array::from_fn::<_, 3, _>(|axis| {
            self.axes[axis]
                .map(|component| mode.store_f32(f64::from(component) * f64::from(self.half[axis])))
        });
        let mut result = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        for corner in 0..8 {
            for dim in 0..2 {
                let mut value = self.center[dim];
                for (axis, scaled) in scaled.iter().enumerate() {
                    let sign = if corner & (1 << axis) == 0 { 1.0 } else { -1.0 };
                    value =
                        mode.store_f32(mode.add(f64::from(value), f64::from(scaled[dim]) * sign));
                }
                result[dim] = result[dim].min(value);
                result[dim + 2] = result[dim + 2].max(value);
            }
        }
        result
    }

    fn hits(self, query: SearchBox) -> bool {
        // Bounded z-up stationary OBB against the source vertical half-cell
        // query. The z tests are trivial for the admitted [-32,64] shapes.
        let extent =
            std::array::from_fn::<_, 2, _>(|i| f64::from(query.max[i] - query.min[i]) * 0.5);
        let delta = std::array::from_fn::<_, 2, _>(|i| {
            f64::from(query.min[i]) - f64::from(self.center[i]) + extent[i]
        });
        let axes = self.axes.map(|axis| axis.map(f64::from));
        let half = self.half.map(f64::from);
        for i in 0..2 {
            let projected = axes[0][i].abs() * half[0] + axes[1][i].abs() * half[1];
            if delta[i].abs() > extent[i] + projected {
                return false;
            }
        }
        for i in 0..2 {
            let projected = axes[i][0].abs() * extent[0] + axes[i][1].abs() * extent[1];
            if (delta[0] * axes[i][0] + delta[1] * axes[i][1]).abs() > half[i] + projected {
                return false;
            }
        }
        true
    }

    pub fn words(self) -> [u32; 15] {
        self.center
            .into_iter()
            .chain(self.axes.into_iter().flatten())
            .chain(self.half)
            .map(f32::to_bits)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

pub fn geometry_words(actor: StationaryActor) -> Option<[u32; 15]> {
    Some(Shape::new(actor)?.words())
}

pub struct WorldPlanes<'a> {
    pub vertices: &'a [u8],
    pub cliffs: &'a [i32],
    pub territory: &'a [i32],
    pub height_half: &'a [f32],
    pub forest_half: &'a [bool],
}

impl WorldPlanes<'_> {
    fn valid(&self) -> bool {
        let [w, h] = DIMENSIONS;
        self.vertices.len() == (w + 1) * (h + 1)
            && self.cliffs.len() == w * h
            && self.territory.len() == w * h
            && self.height_half.len() == w * h * 4
            && self.forest_half.len() == w * h * 4
    }

    fn flat_half(&self, [x, y]: [usize; 2]) -> bool {
        let [w, _] = DIMENSIONS;
        let tile = [x / 2, y / 2];
        let corner = tile[1] * (w + 1) + tile[0];
        self.height_half[y * 2 * w + x] == 1.0
            && self.cliffs[tile[1] * w + tile[0]] == -1
            && [0, 1, w + 1, w + 2]
                .into_iter()
                .all(|offset| self.vertices[corner + offset] == 0)
    }
}

#[derive(Clone, Copy)]
pub enum StationaryEvent {
    Add(StationaryActor),
    Remove(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidCheckpoint,
    InvalidInput,
    ActorState,
    UnsupportedTerrain,
    AmbiguousActorClass,
}

pub struct Update {
    pub dirty: WorldRect,
    pub classifications: usize,
    pub atlas: InitialAtlas,
}

pub struct AtlasProducer {
    tree: LocalTree,
    aliases: [usize; 2],
    first: Vec<u8>,
    second: Vec<u8>,
    mode: Precision53,
    actors: BTreeMap<u32, Shape>,
}

fn valid_checkpoint(atlas: &InitialAtlas) -> bool {
    let [width, height] = atlas.root_dimensions;
    if [width, height] != [64, 64]
        || atlas.nodes.is_empty()
        || atlas.nodes.len() > 87381
        || atlas.aliases.len() != width * height
        || atlas.root_return != atlas.nodes[0].class
    {
        return false;
    }
    let mut aliases = vec![usize::MAX; width * height];
    let mut visited = vec![false; atlas.nodes.len()];
    let mut pending = vec![(
        0,
        Square {
            level: 8,
            x: 0,
            y: 0,
        },
    )];
    while let Some((index, square)) = pending.pop() {
        let Some(node) = atlas.nodes.get(index) else {
            return false;
        };
        if visited[index] || node.square != square || !(0.0..=6.0).contains(&node.density) {
            return false;
        }
        visited[index] = true;
        for slot in square.alias_slots(2, width, height, node.children.is_some()) {
            aliases[slot] = index;
        }
        if let Some(children) = node.children {
            let Some(squares) = square.children() else {
                return false;
            };
            if node.class != MIXED {
                return false;
            }
            pending.extend(children.into_iter().zip(squares).rev());
        } else if !matches!(node.class, 0..=7 | 0x27..=0x29) {
            return false;
        }
    }
    visited.iter().all(|v| *v) && !aliases.contains(&usize::MAX) && aliases == atlas.aliases
}

fn rect([x0, y0, x1, y1]: [i32; 4]) -> HalfCellRect {
    HalfCellRect {
        min: [x0, y0],
        max: [x1, y1],
    }
}

fn leaf_hits<'a>(
    actors: &'a BTreeMap<u32, Shape>,
    area: SearchBox,
) -> impl Iterator<Item = &'a Shape> {
    actors
        .values()
        .filter(move |actor| actor.actor.blocks && actor.hits(area))
}

struct Local<'a> {
    world: &'a WorldPlanes<'a>,
    first: &'a [u8],
    actors: &'a BTreeMap<u32, Shape>,
    veto: &'a [Option<i32>],
    mixed: bool,
    invalid: bool,
}

impl StaticLeafProviders for Local<'_> {
    fn cliff_mask(&mut self, tile: [i32; 2]) -> Option<u16> {
        let raw = self.world.cliffs[tile[1] as usize * DIMENSIONS[0] + tile[0] as usize];
        u16::try_from(raw).ok()
    }
    fn possible_actor(&mut self, area: SearchBox) -> bool {
        leaf_hits(self.actors, area).next().is_some()
    }
    fn actor_class(&mut self, area: SearchBox) -> Option<u8> {
        let mut hits = leaf_hits(self.actors, area);
        let transparent = hits.next()?.actor.transparent;
        self.mixed |= hits.any(|actor| actor.actor.transparent != transparent);
        Some(if transparent { 0x27 } else { 0x28 })
    }
    fn forest_dense(&mut self, [x, y]: [f32; 2], extent: f32) -> bool {
        self.invalid |= extent != 0.5;
        let at = (y * 2.0) as usize * DIMENSIONS[0] * 2 + (x * 2.0) as usize;
        self.world.forest_half[at]
    }
    fn terrain_class(&mut self, [x, y]: [f32; 2], _water: bool, _mountains: bool) -> u8 {
        self.invalid |= !self
            .world
            .flat_half([(x * 2.0) as usize, (y * 2.0) as usize]);
        0
    }
    fn special_value(&mut self, [x, y]: [i32; 2]) -> Option<i32> {
        self.veto[y as usize * DIMENSIONS[0] + x as usize]
    }
    fn raw_half_weight(&mut self, [x, y]: [i32; 2]) -> u8 {
        self.first[y as usize * (DIMENSIONS[0] * 2 + 2) + x as usize]
    }
}

impl AtlasProducer {
    /// Caller authenticates the complete checkpoint and world planes first.
    pub fn from_checkpoint(
        initial: &InitialAtlas,
        first: Vec<u8>,
        second: Vec<u8>,
        mode: Precision53,
        actors: &[StationaryActor],
    ) -> Result<Self, Error> {
        let [w, h] = DIMENSIONS;
        if !valid_checkpoint(initial)
            || first.len() != (2 * w + 2) * (2 * h + 2)
            || second.len() != (w + 2) * (h + 2)
        {
            return Err(Error::InvalidCheckpoint);
        }
        let mut shapes = BTreeMap::new();
        for &actor in actors {
            let shape = Shape::new(actor).ok_or(Error::InvalidInput)?;
            if shapes.insert(actor.id, shape).is_some() {
                return Err(Error::ActorState);
            }
        }
        Ok(Self {
            tree: LocalTree::from_initial(initial, 0),
            aliases: initial.root_dimensions,
            first,
            second,
            mode,
            actors: shapes,
        })
    }

    pub fn grids(&self) -> (&[u8], &[u8]) {
        (&self.first, &self.second)
    }

    /// Apply a native ordinary obstruction transition using current actor and
    /// terrain input. Failed validation leaves both rasters and tree untouched.
    pub fn apply(
        &mut self,
        world: &WorldPlanes<'_>,
        event: StationaryEvent,
    ) -> Result<Update, Error> {
        if !world.valid() {
            return Err(Error::InvalidInput);
        }
        let [w, h] = DIMENSIONS;
        let mut actors = self.actors.clone();
        let shape = match event {
            StationaryEvent::Add(actor) => {
                let shape = Shape::new(actor).ok_or(Error::InvalidInput)?;
                if actors.insert(actor.id, shape).is_some() {
                    return Err(Error::ActorState);
                }
                shape
            }
            StationaryEvent::Remove(id) => actors.remove(&id).ok_or(Error::ActorState)?,
        };
        if !shape.actor.blocks {
            return Err(Error::InvalidInput);
        }
        let size = [w as u16, h as u16];
        let bounds = obstacle_event(shape.projected(self.mode), size).ok_or(Error::InvalidInput)?;
        if (0..2).any(|axis| bounds[axis] > bounds[axis + 2]) {
            return Err(Error::InvalidInput);
        }
        let dispatch = raster_dispatch(bounds, self.mode).ok_or(Error::InvalidInput)?;
        let half_rect = rect(dispatch.half_cells);
        let region =
            grassfire::update_region([2 * w, 2 * h], half_rect, 12).ok_or(Error::InvalidInput)?;
        if (region.min[1]..region.max[1])
            .any(|y| (region.min[0]..region.max[0]).any(|x| !world.flat_half([x, y])))
        {
            return Err(Error::UnsupportedTerrain);
        }
        let mut first = self.first.clone();
        grassfire::update(
            &mut first,
            [2 * w, 2 * h],
            half_rect,
            12,
            self.mode,
            |[x, y]| {
                let min = [x as f32 * 0.5, y as f32 * 0.5];
                let area = SearchBox {
                    min,
                    max: min.map(|v| v + 0.5),
                };
                (x == 0
                    || y == 0
                    || x == 2 * w - 1
                    || y == 2 * h - 1
                    || leaf_hits(&actors, area).next().is_some()
                    || world.forest_half[y * 2 * w + x])
                    .then_some(0.0)
            },
        )
        .ok_or(Error::InvalidInput)?;
        let mut second = self.second.clone();
        grassfire::update(
            &mut second,
            [w, h],
            rect(dispatch.whole_tiles),
            127,
            self.mode,
            |[x, y]| (world.vertices[y * (w + 1) + x] == 7).then_some(0.0),
        )
        .ok_or(Error::InvalidInput)?;
        let borders =
            TerritoryBorders::new(DIMENSIONS, world.territory).ok_or(Error::InvalidInput)?;
        let table = std::array::from_fn(|index| descriptors(index as u8));
        let veto: Vec<_> = (0..w * h)
            .map(|index| {
                borders
                    .query([(index % w) as i32, (index / w) as i32], &table)
                    .1
            })
            .collect();
        let mut local = Local {
            world,
            first: &first,
            actors: &actors,
            veto: &veto,
            mixed: false,
            invalid: false,
        };
        let mut tree = self.tree.clone();
        let classifications = tree.update(dispatch.tree_bounds, size, true, self.mode, &mut local);
        if local.invalid {
            return Err(Error::UnsupportedTerrain);
        }
        if local.mixed {
            return Err(Error::AmbiguousActorClass);
        }
        let atlas = tree.snapshot(self.aliases);
        self.tree = tree;
        self.first = first;
        self.second = second;
        self.actors = actors;
        Ok(Update {
            dirty: bounds,
            classifications,
            atlas,
        })
    }
}
