//! Isolated, event-driven movement atlas for an authenticated fresh owned Valley match.
#![allow(dead_code)]
mod atlas_branch;
mod atlas_reader;
mod atlas_static_leaf;
mod dirty;
mod gate;
mod grassfire;
mod initial_atlas;
mod localized;
mod movement_class;
mod producer;
mod territory_borders;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod atlas_ray;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod edge_metrics;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod fine_search;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod ordered_neighbors;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod route_smoothing;
#[cfg(feature = "ee2-valley196-citizen-route-shadow")]
mod live_route;
#[cfg(feature = "ee2-valley197-waypoint-motion")]
pub(crate) mod route_motion;
#[cfg(feature = "ee2-valley204-ground-motion")]
pub(crate) mod ground_motion;
#[cfg(feature = "ee2-valley215-citizen-contact")]
pub(super) mod contact_kernel;
#[cfg(feature = "ee2-valley198-waypoint-save")]
pub(super) mod route_history;

use super::{active_game, hash::sha256, STAGE};
use crate::{
    defs::{DefId, BARRACKS, DEFS, HOUSE, TEMPLE, TOWER, WORKER},
    game::{Ent, Game},
};
use atlas_branch::Square;
use atlas_reader::Precision53;
use gate::{
    Admission, AtlasSnapshot, Leaf, MovementGroup, Owner, Policy, Rounding, SourceSeat,
    TopologyIdentity, Treaty, ValleyRegionGate,
};
use initial_atlas::{InitialAtlas, Node};
use producer::{AtlasProducer, StationaryActor, StationaryEvent, WorldPlanes, DIMENSIONS};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

pub(super) const LENGTH: usize = 595_036;
const DIGEST: [u8; 32] = hex("738fd6d4585dfaf79828632be0e9b8b716a7c14510a4aa46c70254b214a72ecc");
const WORLD: [u8; 32] = hex("ba0ce30d1cba4ddb124f0378652b93cd2873fe5d33e980fe91c8c9f9bf481119");
const OPENING: [u8; 32] = hex("68ca55d581c3e855a17aad58461575f9a160a9c6bd8fa90e081cbecee05bfefc");
const IDENTITY: TopologyIdentity = TopologyIdentity {
    world_input: WORLD,
    owned_opening: OPENING,
    rounding: Rounding::Nearest,
};

const fn hex(value: &str) -> [u8; 32] {
    let input = value.as_bytes();
    let mut result = [0; 32];
    let mut index = 0;
    while index < result.len() {
        result[index] = nibble(input[2 * index]) * 16 + nibble(input[2 * index + 1]);
        index += 1;
    }
    result
}

const fn nibble(byte: u8) -> u8 {
    if byte.is_ascii_digit() {
        byte - b'0'
    } else {
        byte - b'a' + 10
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn bytes(&mut self, len: usize) -> &'a [u8] {
        let (value, remaining) = self.0.split_at(len);
        self.0 = remaining;
        value
    }
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.bytes(2).try_into().unwrap())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.bytes(4).try_into().unwrap())
    }
    fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
}

pub(super) struct Prepared {
    atlas: InitialAtlas,
    actors: Vec<StationaryActor>,
    first: Vec<u8>,
    second: Vec<u8>,
    vertices: Vec<u8>,
    cliffs: Vec<i32>,
    heights: Vec<f32>,
    forest: Vec<bool>,
}

impl Prepared {
    pub(super) fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != LENGTH || sha256(bytes) != DIGEST {
            return None;
        }
        // This parser executes only for the exact reviewed VAP1 content.
        let mut r = Reader(bytes);
        if r.bytes(4) != b"VAP1" || [r.u16(), r.u16(), r.u16(), r.u16()] != [115, 71, 0, 0] {
            return None;
        }
        let [nodes, aliases, actors, first, second] = std::array::from_fn(|_| r.u32() as usize);
        if [nodes, aliases, actors, first, second] != [12833, 4096, 43, 33408, 8541]
            || r.bytes(32) != WORLD
            || r.bytes(32) != OPENING
        {
            return None;
        }
        let nodes: Vec<_> = (0..nodes)
            .map(|_| {
                let [x, y] = [r.u16(), r.u16()];
                let [level, class] = r.u16().to_le_bytes();
                let _reserved = r.u16();
                let density = r.f32();
                let children: [usize; 4] = std::array::from_fn(|_| r.u32() as usize);
                let children = (children[0] != u32::MAX as usize).then_some(children);
                Node {
                    square: Square { x, y, level },
                    class,
                    density,
                    children,
                }
            })
            .collect();
        let aliases = (0..aliases).map(|_| r.u32() as usize).collect();
        let atlas = InitialAtlas {
            root_return: nodes[0].class,
            nodes,
            aliases,
            root_dimensions: [64, 64],
        };
        let actors = (0..actors)
            .map(|_| {
                let id = r.u32();
                let flags = r.u32();
                StationaryActor {
                    id,
                    position: std::array::from_fn(|_| r.f32()),
                    yaw: r.f32(),
                    full_size: std::array::from_fn(|_| r.f32()),
                    height: r.f32(),
                    blocks: flags & 1 != 0,
                    transparent: flags & 2 != 0,
                }
            })
            .collect();
        let [w, h] = DIMENSIONS;
        let vertices = r.bytes((w + 1) * (h + 1)).to_vec();
        let cliffs = (0..w * h).map(|_| r.u32() as i32).collect();
        let heights = (0..w * h * 4).map(|_| r.f32()).collect();
        let packed = r.bytes((w * h * 4).div_ceil(8));
        let forest = (0..w * h * 4)
            .map(|i| packed[i / 8] & (1 << (i % 8)) != 0)
            .collect();
        let first = r.bytes(first).to_vec();
        let second = r.bytes(second).to_vec();
        if !r.0.is_empty() {
            return None;
        }
        Some(Self {
            atlas,
            actors,
            first,
            second,
            vertices,
            cliffs,
            heights,
            forest,
        })
    }

    pub(super) fn bind(self, game: &Game) -> Option<RuntimeAtlas> {
        if !matches_opening(game) || !game.uses_valley_house_movement(HOUSE) {
            return None;
        }
        let world = game.map.source_world.as_deref()?;
        if !world
            .materials
            .iter()
            .zip(&self.vertices)
            .all(|(&id, &class)| world.move_palette[usize::from(id)] == class)
        {
            return None;
        }
        let producer = AtlasProducer::from_checkpoint(
            &self.atlas,
            self.first,
            self.second,
            Precision53::NearestEven,
            &self.actors,
        )
        .ok()?;
        let leaves = atlas_leaves(&self.atlas);
        let gate = ValleyRegionGate::from_atlas(&AtlasSnapshot {
            identity: IDENTITY,
            generation: 0,
            leaves: &leaves,
            source_territories: &world.territory_ids,
        })
        .ok()?;
        let cities = game.valley_fixture.as_ref()?.authored_city_handles()?;
        Some(RuntimeAtlas {
            producer,
            gate: RefCell::new(gate),
            atlas: self.atlas,
            vertices: self.vertices,
            cliffs: self.cliffs,
            heights: self.heights,
            forest: self.forest,
            terrain: game.map.ter.clone(),
            resources: game.map.res.clone(),
            corners: game.map.hm.clone(),
            cities,
            handles: BTreeMap::new(),
            next_actor: 0x2000_0000,
            generation: 0,
            stale: false,
            #[cfg(feature = "ee2-valley198-waypoint-save")]
            history: route_history::Journal::default(),
            last_move: Cell::new(None),
            #[cfg(feature = "ee2-valley196-citizen-route-shadow")]
            last_route: RefCell::new(None),
        })
    }
}

fn matches_opening(game: &Game) -> bool {
    game.tick == 0
        && !game.net
        && game.humans == 1
        && game.nplayers == 2
        && game
            .map
            .source_world
            .as_ref()
            .is_some_and(|w| w.input_sha == WORLD)
        && game
            .valley_fixture
            .as_ref()
            .is_some_and(|fixture| fixture.input_sha() == OPENING)
}

pub(super) struct RuntimeAtlas {
    producer: AtlasProducer,
    gate: RefCell<ValleyRegionGate>,
    atlas: InitialAtlas,
    vertices: Vec<u8>,
    cliffs: Vec<i32>,
    heights: Vec<f32>,
    forest: Vec<bool>,
    terrain: Vec<u8>,
    resources: Vec<u8>,
    corners: Vec<f32>,
    cities: [u32; 2],
    handles: BTreeMap<u32, (u32, BuildingPose)>,
    next_actor: u32,
    generation: u32,
    stale: bool,
    #[cfg(feature = "ee2-valley198-waypoint-save")]
    pub(super) history: route_history::Journal,
    last_move: Cell<Option<(u32, u32)>>,
    #[cfg(feature = "ee2-valley196-citizen-route-shadow")]
    last_route: RefCell<Option<live_route::Trace>>,
}

#[derive(Clone, Copy, PartialEq)]
struct BuildingPose {
    def: DefId,
    x: f32,
    y: f32,
    yaw: f32,
}

impl From<&Ent> for BuildingPose {
    fn from(site: &Ent) -> Self {
        Self {
            def: site.def,
            x: site.x,
            y: site.y,
            yaw: site.rot,
        }
    }
}

#[derive(Clone, Copy)]
enum BuildingEvent {
    Add(u32, BuildingPose),
    Remove(u32),
}

fn stationary_dimensions(def: DefId) -> Option<(f32, f32)> {
    match def {
        HOUSE => Some((1.5, 1.0)),
        BARRACKS => Some((2.0, 1.0)),
        TEMPLE => Some((2.5, 1.0)),
        TOWER => Some((1.0, 4.0)),
        _ => None,
    }
}

fn atlas_leaves(atlas: &InitialAtlas) -> Vec<Leaf> {
    atlas
        .nodes
        .iter()
        .filter(|node| node.children.is_none())
        .map(|node| Leaf {
            origin: [node.square.x, node.square.y],
            level: node.square.level,
            class: node.class,
        })
        .collect()
}

impl RuntimeAtlas {
    fn unchanged_world(&self, game: &Game) -> bool {
        !self.stale
            && self.terrain == game.map.ter
            && self.resources == game.map.res
            && self.corners == game.map.hm
            && game.players[..2].iter().all(|seat| seat.epoch == 0)
            && !game
                .valley_forest_play()
                .is_some_and(|forest| forest.promoted.iter().flatten().any(|row| row.wood == 0.0))
    }

    fn building_change(&self, game: &Game) -> Result<Option<BuildingEvent>, ()> {
        let mut result = None;
        let mut cities = 0;
        let mut known = 0;
        for (index, site) in game.ents[..game.high].iter().enumerate() {
            if !site.alive
                || site.hp <= 0.0
                || site.built == 0.0
                || !DEFS[site.def as usize].building
            {
                continue;
            }
            let handle = game.h(index);
            if self.cities.contains(&handle) {
                cities += 1;
                continue;
            }
            let pose = BuildingPose::from(site);
            if let Some((_, previous)) = self.handles.get(&handle) {
                if *previous != pose {
                    return Err(());
                }
                known += 1;
            } else if result.replace(BuildingEvent::Add(handle, pose)).is_some() {
                return Err(());
            }
        }
        if cities != 2 || self.handles.len() - known > 1 {
            return Err(());
        }
        if self.handles.len() != known {
            let handle = self
                .handles
                .keys()
                .find(|&&handle| game.get(handle).is_none())
                .copied()
                .ok_or(())?;
            if result.replace(BuildingEvent::Remove(handle)).is_some() {
                return Err(());
            }
        }
        Ok(result)
    }

    fn current(&self, game: &Game) -> bool {
        self.unchanged_world(game) && matches!(self.building_change(game), Ok(None))
    }

    fn update(&mut self, game: &Game, change: BuildingEvent) -> Option<()> {
        if !self.unchanged_world(game) {
            return None;
        }
        #[cfg(feature = "ee2-valley198-waypoint-save")]
        if !self.history.has_room() { return None; }
        let event = match change {
            BuildingEvent::Add(_, site) => {
                let (size, height) = stationary_dimensions(site.def)?;
                StationaryEvent::Add(StationaryActor {
                    id: self.next_actor,
                    position: [site.x, site.y, game.map.height(site.x, site.y)],
                    yaw: site.yaw,
                    full_size: [size; 2],
                    height,
                    blocks: true,
                    transparent: false,
                })
            }
            BuildingEvent::Remove(handle) => StationaryEvent::Remove(self.handles.get(&handle)?.0),
        };
        let generation = self.generation.checked_add(1)?;
        let territory = &game.map.source_world.as_ref()?.territory_ids;
        let world = WorldPlanes {
            vertices: &self.vertices,
            cliffs: &self.cliffs,
            territory,
            height_half: &self.heights,
            forest_half: &self.forest,
        };
        let update = self.producer.apply(&world, event).ok()?;
        let leaves = atlas_leaves(&update.atlas);
        self.gate
            .get_mut()
            .rebuild_from_atlas(
                u64::from(self.generation),
                &AtlasSnapshot {
                    identity: IDENTITY,
                    generation: u64::from(generation),
                    leaves: &leaves,
                    source_territories: territory,
                },
            )
            .ok()?;
        match change {
            BuildingEvent::Add(handle, pose) => {
                self.handles.insert(handle, (self.next_actor, pose));
                self.next_actor = self.next_actor.checked_add(1)?;
            }
            BuildingEvent::Remove(handle) => {
                self.handles.remove(&handle);
            }
        }
        self.atlas = update.atlas;
        self.generation = generation;
        #[cfg(feature = "ee2-valley198-waypoint-save")]
        self.history.push(change);
        Some(())
    }

    fn leaf(&self, x: i32, y: i32) -> Option<AtlasLeaf> {
        let [w, h] = DIMENSIONS;
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        if x >= 2 * w || y >= 2 * h {
            return None;
        }
        let mut node = &self.atlas.nodes[self.atlas.aliases[(y >> 2) * 64 + (x >> 2)]];
        while let Some(children) = node.children {
            let half = 1_usize << (node.square.level - 1);
            let child = usize::from(x >= usize::from(node.square.x) + half)
                + 2 * usize::from(y >= usize::from(node.square.y) + half);
            node = &self.atlas.nodes[children[child]];
        }
        Some(AtlasLeaf {
            origin_half: [node.square.x, node.square.y],
            level: node.square.level,
            class: node.class,
            density: node.density,
        })
    }

    fn cell(&self, x: i32, y: i32, field: u32) -> Option<u32> {
        let leaf = self.leaf(x, y)?;
        let [w, _] = DIMENSIONS;
        match field {
            0 => Some(u32::from(leaf.class)),
            1 => Some(leaf.density.to_bits()),
            2 => Some(u32::from(
                self.producer.grids().0[(y as usize + 1) * (2 * w + 2) + x as usize + 1],
            )),
            3 => Some(u32::from(leaf.level)),
            4 | 5 => Some(u32::from(leaf.origin_half[(field - 4) as usize])),
            _ => None,
        }
    }
}

pub(crate) struct AtlasLeaf {
    pub(crate) origin_half: [u16; 2],
    pub(crate) level: u8,
    pub(crate) class: u8,
    pub(crate) density: f32,
}

pub(crate) struct AtlasView<'a> {
    runtime: &'a RuntimeAtlas,
    policy: Policy,
}

impl AtlasView<'_> {
    pub(crate) fn cell(&self, x: i32, y: i32, field: u32) -> Option<u32> {
        self.runtime.cell(x, y, field)
    }
    pub(crate) fn leaf(&self, half: [i32; 2]) -> Option<AtlasLeaf> {
        self.runtime.leaf(half[0], half[1])
    }
    pub(crate) fn admit(&self, source_owner: i32, group: u32, from: [i32; 2], to: [i32; 2]) -> u32 {
        let owner = match source_owner {
            -1 => Owner::Unrestricted,
            0..=2 => Owner::Seat(SourceSeat::new(source_owner as u8).unwrap()),
            _ => return 0,
        };
        let Some(group) = u8::try_from(group).ok().and_then(MovementGroup::new) else {
            return 0;
        };
        match self.runtime.gate.borrow_mut().admit(
            u64::from(self.runtime.generation),
            &self.policy,
            owner,
            group,
            from,
            to,
        ) {
            Admission::Unavailable => 0,
            Admission::Denied => 1,
            Admission::Admitted => 2,
        }
    }
}

impl Game {
    pub(crate) fn valley_atlas_view(&self) -> Option<AtlasView<'_>> {
        let runtime = self.valley_fixture.as_ref()?.atlas.as_ref()?;
        if !runtime.current(self) {
            return None;
        }
        let world = self.map.source_world.as_ref()?;
        let mut policy = Policy::default();
        for compact in 0..world.territories.len() {
            let owner = match self.towner[compact] {
                255 => 0,
                seat @ 0..=1 => seat + 1,
                _ => return None,
            };
            let territory = u8::try_from(world.source_territory(compact as u8)?).ok()?;
            if !policy.set_holder(territory, SourceSeat::new(owner)?) {
                return None;
            }
        }
        policy.initial_ceasefire = self.initial_ceasefire_active();
        if let Some(rules) = &self.diplomacy.borders {
            let [a, b] = [1, 2].map(|id| SourceSeat::new(id).unwrap());
            let term = [
                Treaty::Full,
                Treaty::Civilian,
                Treaty::Trade,
                Treaty::Closed,
            ]
            .get(usize::from(rules.term(0, 1)))
            .copied()?;
            policy.set_treaty(a, b, term);
            policy.set_exempt(a, rules.atlas_exempt(0));
            policy.set_exempt(b, rules.atlas_exempt(1));
        }
        Some(AtlasView { runtime, policy })
    }

    pub(crate) fn sync_valley_atlas(&mut self) {
        let Some(mut runtime) = self
            .valley_fixture
            .as_mut()
            .and_then(|fixture| fixture.atlas.take())
        else {
            return;
        };
        if !runtime.stale {
            runtime.stale = match runtime.building_change(self) {
                Ok(None) => false,
                Ok(Some(change)) => runtime.update(self, change).is_none(),
                Err(()) => true,
            };
        }
        self.valley_fixture.as_mut().unwrap().atlas = Some(runtime);
    }

    pub(super) fn valley_atlas_observe_move(&self, index: usize, target: [f32; 2]) {
        let actor = &self.ents[index];
        if actor.def != WORKER || actor.team > 1 || !target.into_iter().all(f32::is_finite) {
            return;
        }
        let Some(view) = self.valley_atlas_view() else {
            return;
        };
        let half = |point: [f32; 2]| point.map(|value| (value * 2.0).floor() as i32);
        let result = view.admit(
            i32::from(actor.team) + 1,
            2,
            half([actor.x, actor.y]),
            half(target),
        );
        view.runtime.last_move.set(Some((self.tick, result)));
    }
}

#[no_mangle]
pub extern "C" fn valley192_atlas_alloc(length: usize) -> *mut u8 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.atlas_prepared = None;
        stage.atlas_input.clear();
        if length != LENGTH || !stage.game.as_ref().is_some_and(matches_opening) {
            return std::ptr::null_mut();
        }
        stage.atlas_input.resize(length, 0);
        stage.atlas_input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn valley192_atlas_prepare() -> u32 {
    STAGE.with(|stage| {
        let mut stage = stage.borrow_mut();
        stage.atlas_prepared = stage
            .game
            .as_ref()
            .filter(|game| matches_opening(game))
            .and_then(|_| Prepared::decode(&stage.atlas_input));
        if !cfg!(feature = "ee2-valley198-waypoint-save") || stage.atlas_prepared.is_none() {
            stage.atlas_input.clear();
        }
        u32::from(stage.atlas_prepared.is_some())
    })
}

#[no_mangle]
pub extern "C" fn valley192_atlas_status() -> u32 {
    let Some(game) = active_game() else {
        return 0;
    };
    let Some(runtime) = game
        .valley_fixture
        .as_ref()
        .and_then(|fixture| fixture.atlas.as_ref())
    else {
        return 0;
    };
    if runtime.current(game) {
        1
    } else {
        2
    }
}

#[no_mangle]
pub extern "C" fn valley192_atlas_generation() -> u32 {
    active_game()
        .and_then(Game::valley_atlas_view)
        .map_or(u32::MAX, |view| view.runtime.generation)
}

#[no_mangle]
pub extern "C" fn valley192_atlas_cell(x: i32, y: i32, field: u32) -> u32 {
    active_game()
        .and_then(Game::valley_atlas_view)
        .and_then(|view| view.cell(x, y, field))
        .unwrap_or(u32::MAX)
}

#[no_mangle]
pub extern "C" fn valley192_atlas_admit(
    owner: i32,
    group: u32,
    fx: i32,
    fy: i32,
    tx: i32,
    ty: i32,
) -> u32 {
    active_game()
        .and_then(Game::valley_atlas_view)
        .map_or(0, |view| view.admit(owner, group, [fx, fy], [tx, ty]))
}

#[no_mangle]
pub extern "C" fn valley192_atlas_last_move(field: u32) -> u32 {
    active_game()
        .and_then(Game::valley_atlas_view)
        .and_then(|view| view.runtime.last_move.get())
        .map_or(u32::MAX, |(tick, result)| match field {
            0 => tick,
            1 => result,
            _ => u32::MAX,
        })
}
