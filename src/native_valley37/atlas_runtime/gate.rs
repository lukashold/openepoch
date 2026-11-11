//! Prepared region admission for the authenticated owned Valley opening.

#[path = "region_graph.rs"]
mod region_graph;
pub use region_graph::Leaf;
use region_graph::RegionGraph;
use std::collections::{BTreeMap, BTreeSet};

const WIDTH: usize = 115;
const HEIGHT: usize = 71;
const TERRITORIES: usize = 12;
const GROUPS: usize = 7;
const SEATS: usize = 11; // Nature is source seat zero.
const UNASSIGNED: u16 = 0xfffe;
const BLOCKED: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Rounding {
    Nearest = 0,
    Chop = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TopologyIdentity {
    pub world_input: [u8; 32],
    pub owned_opening: [u8; 32],
    pub rounding: Rounding,
}

pub struct AtlasSnapshot<'a> {
    pub identity: TopologyIdentity,
    pub generation: u64,
    pub leaves: &'a [Leaf],
    pub source_territories: &'a [i32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeafShape {
    pub level: u8,
    pub class: u8,
}

impl From<Leaf> for LeafShape {
    fn from(leaf: Leaf) -> Self {
        Self {
            level: leaf.level,
            class: leaf.class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeafEdit {
    pub origin: [u16; 2],
    pub before: Option<LeafShape>,
    pub after: Option<LeafShape>,
}

pub struct AtlasDelta<'a> {
    pub identity: TopologyIdentity,
    pub generation: u64,
    pub edits: &'a [LeafEdit],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebuildError {
    Generation,
    Identity,
    InvalidAtlas,
    UnsupportedTopology,
    MissingBaseAtlas,
    InconsistentDelta,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSeat(u8);

impl SourceSeat {
    pub fn new(seat: u8) -> Option<Self> {
        (usize::from(seat) < SEATS).then_some(Self(seat))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Owner {
    Unrestricted,
    Seat(SourceSeat),
}

impl Owner {
    fn slot(self) -> usize {
        match self {
            Self::Seat(seat) => usize::from(seat.0),
            Self::Unrestricted => SEATS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MovementGroup(u8);

impl MovementGroup {
    pub fn new(group: u8) -> Option<Self> {
        (usize::from(group) < GROUPS).then_some(Self(group))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Treaty {
    #[default]
    Full,
    Civilian,
    Trade,
    Closed,
}

pub fn border_denied(
    mover: SourceSeat,
    holder: SourceSeat,
    purpose: u8,
    term: Treaty,
    exempt: bool,
    initial_ceasefire: bool,
) -> bool {
    if initial_ceasefire {
        return purpose != 3 && mover.0 != 0 && holder.0 != 0 && mover != holder;
    }
    !exempt
        && purpose != 4
        && match term {
            Treaty::Full => false,
            Treaty::Civilian => !matches!(purpose, 2 | 3),
            Treaty::Trade => purpose != 3,
            Treaty::Closed => true,
        }
}

/// One committed authoritative policy snapshot. Source IDs, not compact indices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Policy {
    owner: [SourceSeat; TERRITORIES + 1],
    treaties: [[Treaty; SEATS]; SEATS],
    exempt: [bool; SEATS],
    pub initial_ceasefire: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            owner: [SourceSeat(0); TERRITORIES + 1],
            treaties: [[Treaty::Full; SEATS]; SEATS],
            exempt: [false; SEATS],
            initial_ceasefire: false,
        }
    }
}

impl Policy {
    pub fn set_holder(&mut self, source_territory: u8, holder: SourceSeat) -> bool {
        if !(1..=TERRITORIES).contains(&usize::from(source_territory)) {
            return false;
        }
        self.owner[usize::from(source_territory)] = holder;
        true
    }

    pub fn set_treaty(&mut self, a: SourceSeat, b: SourceSeat, treaty: Treaty) -> bool {
        if a == b || a.0 == 0 || b.0 == 0 {
            return false;
        }
        let (a, b) = (usize::from(a.0), usize::from(b.0));
        self.treaties[a][b] = treaty;
        self.treaties[b][a] = treaty;
        true
    }

    pub fn set_exempt(&mut self, seat: SourceSeat, exempt: bool) {
        self.exempt[usize::from(seat.0)] = exempt;
    }

    fn denied_territories(&self, owner: Owner, group: MovementGroup) -> u16 {
        let Owner::Seat(mover) = owner else {
            return 0;
        };
        self.owner
            .iter()
            .enumerate()
            .skip(1)
            .fold(0, |mask, (territory, &holder)| {
                let denied = holder.0 > 0
                    && border_denied(
                        mover,
                        holder,
                        group.0,
                        self.treaties[usize::from(mover.0)][usize::from(holder.0)],
                        self.exempt[usize::from(mover.0)],
                        self.initial_ceasefire,
                    );
                mask | (u16::from(denied) << territory)
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Unavailable,
    Denied,
    Admitted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    Header,
    Identity,
    Truncated,
    Coverage,
    Membership,
    Topology,
    TrailingBytes,
}

#[derive(Clone, Debug)]
struct Region {
    territory: u8,
    neighbors: Vec<u16>,
}

#[derive(Debug)]
struct Labels {
    denied_territories: u16,
    component: Vec<u16>,
}

#[derive(Debug)]
struct Group {
    rows: Vec<Region>,
    cached: [Option<Labels>; SEATS + 1],
}

struct AtlasState {
    leaves: Vec<Leaf>,
    territories: Vec<i32>,
}

pub struct ValleyRegionGate {
    identity: TopologyIdentity,
    generation: u64,
    half_cell_base: Vec<u16>,
    base_group: Vec<[u16; GROUPS]>,
    groups: [Group; GROUPS],
    atlas: Option<AtlasState>,
}

fn read_u16(bytes: &mut &[u8]) -> Result<u16, DecodeError> {
    let (head, tail) = bytes.split_at_checked(2).ok_or(DecodeError::Truncated)?;
    *bytes = tail;
    Ok(u16::from_le_bytes([head[0], head[1]]))
}

impl ValleyRegionGate {
    /// Call only after authenticating the entire private blob by its pinned SHA-256.
    /// `generation` identifies the matching authoritative topology, never a game tick.
    pub fn decode_authenticated(
        mut bytes: &[u8],
        expected: TopologyIdentity,
        generation: u64,
    ) -> Result<Self, DecodeError> {
        let (header, body) = bytes.split_at_checked(76).ok_or(DecodeError::Truncated)?;
        let mut numbers = &header[4..12];
        let width = usize::from(read_u16(&mut numbers)?);
        let height = usize::from(read_u16(&mut numbers)?);
        let mode = read_u16(&mut numbers)?;
        let bases = usize::from(read_u16(&mut numbers)?);
        if &header[..4] != b"VRG1"
            || [width, height] != [WIDTH, HEIGHT]
            || mode > 1
            || bases != 430 + 2 * usize::from(mode)
        {
            return Err(DecodeError::Header);
        }
        if mode != expected.rounding as u16
            || header[12..44] != expected.world_input
            || header[44..76] != expected.owned_opening
        {
            return Err(DecodeError::Identity);
        }
        bytes = body;
        let count = 101 + usize::from(mode);
        let counts = [count, 0, count, count, count, 0, 0];
        for expected_count in counts {
            if usize::from(read_u16(&mut bytes)?) != expected_count {
                return Err(DecodeError::Topology);
            }
        }
        let mut half_cell_base = Vec::with_capacity(WIDTH * HEIGHT * 4);
        for _ in 0..WIDTH * HEIGHT * 4 {
            let id = read_u16(&mut bytes)?;
            if id != UNASSIGNED && usize::from(id) >= bases {
                return Err(DecodeError::Coverage);
            }
            half_cell_base.push(id);
        }
        let mut base_group = Vec::with_capacity(bases);
        for _ in 0..bases {
            let mut row = [UNASSIGNED; GROUPS];
            for (id, count) in row.iter_mut().zip(counts) {
                *id = read_u16(&mut bytes)?;
                if *id != UNASSIGNED && usize::from(*id) >= count {
                    return Err(DecodeError::Membership);
                }
            }
            base_group.push(row);
        }
        let mut groups = std::array::from_fn(|_| Group {
            rows: Vec::new(),
            cached: Default::default(),
        });
        for (group, count) in groups.iter_mut().zip(counts) {
            group.rows.reserve(count);
            for from in 0..count {
                let [territory, class] = read_u16(&mut bytes)?.to_le_bytes();
                let degree = usize::from(read_u16(&mut bytes)?);
                if usize::from(territory) > TERRITORIES || class != 0 || degree >= count {
                    return Err(DecodeError::Topology);
                }
                let mut neighbors = Vec::with_capacity(degree);
                for _ in 0..degree {
                    let id = read_u16(&mut bytes)?;
                    if usize::from(id) >= count
                        || usize::from(id) == from
                        || neighbors.contains(&id)
                    {
                        return Err(DecodeError::Topology);
                    }
                    neighbors.push(id);
                }
                group.rows.push(Region {
                    territory,
                    neighbors,
                });
            }
            for (from, row) in group.rows.iter().enumerate() {
                if row.neighbors.iter().any(|&to| {
                    !group.rows[usize::from(to)]
                        .neighbors
                        .contains(&(from as u16))
                }) {
                    return Err(DecodeError::Topology);
                }
            }
        }
        if !bytes.is_empty() {
            return Err(DecodeError::TrailingBytes);
        }
        Ok(Self {
            identity: expected,
            generation,
            half_cell_base,
            base_group,
            groups,
            atlas: None,
        })
    }

    /// The caller supplies the completed atlas of the authenticated map/start.
    pub fn from_atlas(atlas: &AtlasSnapshot<'_>) -> Result<Self, RebuildError> {
        let (half_cell_base, graph) = build_atlas_graph(atlas)?;
        let groups = graph.groups.map(|rows| Group {
            rows: rows
                .into_iter()
                .map(|row| Region {
                    territory: row.territory.max(0) as u8,
                    neighbors: row.adjacent,
                })
                .collect(),
            cached: Default::default(),
        });
        Ok(Self {
            identity: atlas.identity,
            generation: atlas.generation,
            half_cell_base,
            base_group: graph.group_membership,
            groups,
            atlas: Some(AtlasState {
                leaves: atlas.leaves.to_vec(),
                territories: atlas.source_territories.to_vec(),
            }),
        })
    }

    /// Replace the derived graph only after receiving the completed, authenticated
    /// atlas for the same match. A caller may supply a newer atlas after several
    /// dirty events have completed or have been joined into one source batch.
    pub fn rebuild_from_atlas(
        &mut self,
        previous_generation: u64,
        atlas: &AtlasSnapshot<'_>,
    ) -> Result<(), RebuildError> {
        self.check_successor(previous_generation, atlas.generation, atlas.identity)?;
        if self
            .atlas
            .as_ref()
            .is_some_and(|prior| prior.territories != atlas.source_territories)
        {
            return Err(RebuildError::Identity);
        }
        *self = Self::from_atlas(atlas)?;
        Ok(())
    }

    /// Commit local leaf changes supplied by the authoritative atlas producer.
    /// Territory IDs are the static map plane. Changing holders belongs to `Policy`.
    pub fn rebuild_from_delta(
        &mut self,
        previous_generation: u64,
        delta: &AtlasDelta<'_>,
    ) -> Result<(), RebuildError> {
        self.check_successor(previous_generation, delta.generation, delta.identity)?;
        let current = self.atlas.as_ref().ok_or(RebuildError::MissingBaseAtlas)?;
        let mut rows: BTreeMap<_, _> = current
            .leaves
            .iter()
            .map(|&leaf| (leaf.origin, LeafShape::from(leaf)))
            .collect();
        let mut edited = BTreeSet::new();
        for edit in delta.edits {
            if !edited.insert(edit.origin)
                || rows.get(&edit.origin).copied() != edit.before
                || edit.before == edit.after
            {
                return Err(RebuildError::InconsistentDelta);
            }
            if let Some(shape) = edit.after {
                rows.insert(edit.origin, shape);
            } else {
                rows.remove(&edit.origin);
            }
        }
        let leaves: Vec<_> = rows
            .into_iter()
            .map(|(origin, shape)| Leaf {
                origin,
                level: shape.level,
                class: shape.class,
            })
            .collect();
        let snapshot = AtlasSnapshot {
            identity: delta.identity,
            generation: delta.generation,
            leaves: &leaves,
            source_territories: &current.territories,
        };
        *self = Self::from_atlas(&snapshot)?;
        Ok(())
    }

    fn check_successor(
        &self,
        previous_generation: u64,
        next_generation: u64,
        identity: TopologyIdentity,
    ) -> Result<(), RebuildError> {
        if self.generation != previous_generation || next_generation <= self.generation {
            return Err(RebuildError::Generation);
        }
        if identity != self.identity {
            return Err(RebuildError::Identity);
        }
        Ok(())
    }

    /// Call with original-coordinate half cells after applying the source point conversion.
    pub fn admit(
        &mut self,
        current_generation: u64,
        policy: &Policy,
        owner: Owner,
        group: MovementGroup,
        from: [i32; 2],
        to: [i32; 2],
    ) -> Admission {
        if self.generation != current_generation {
            return Admission::Unavailable;
        }
        let Some(to) = self.region(group, to) else {
            return Admission::Denied;
        };
        let from = self.region(group, from);
        let denied_territories = policy.denied_territories(owner, group);
        let groups = &mut self.groups[usize::from(group.0)];
        let slot = &mut groups.cached[owner.slot()];
        if slot
            .as_ref()
            .is_none_or(|labels| labels.denied_territories != denied_territories)
        {
            *slot = Some(Labels {
                denied_territories,
                component: components(&groups.rows, denied_territories),
            });
        }
        let labels = &slot.as_ref().unwrap().component;
        let destination = labels[to];
        if destination == BLOCKED {
            return Admission::Denied;
        }
        if from.is_none_or(|id| matches!(labels[id], BLOCKED) || labels[id] == destination) {
            Admission::Admitted
        } else {
            Admission::Denied
        }
    }

    fn region(&self, group: MovementGroup, [x, y]: [i32; 2]) -> Option<usize> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        if x >= WIDTH * 2 || y >= HEIGHT * 2 {
            return None;
        }
        let base = self.half_cell_base[y * WIDTH * 2 + x];
        let &id = self
            .base_group
            .get(usize::from(base))?
            .get(usize::from(group.0))?;
        (id != UNASSIGNED).then_some(usize::from(id))
    }
}

pub(super) fn build_atlas_graph(
    atlas: &AtlasSnapshot<'_>,
) -> Result<(Vec<u16>, RegionGraph), RebuildError> {
    if atlas
        .source_territories
        .iter()
        .any(|id| !(-1..=TERRITORIES as i32).contains(id))
    {
        return Err(RebuildError::InvalidAtlas);
    }
    // The verified Valley produces only class-zero group rows. Retain the
    // full stock masks so an incompatible future input is rejected explicitly.
    let stock = [0x303, 0, 0x303, 0x303, 0x303, 0x80, 0xa0];
    let graph = RegionGraph::build_static(
        8,
        atlas.leaves,
        [WIDTH, HEIGHT],
        Some(atlas.source_territories),
        stock,
    )
    .ok_or(RebuildError::InvalidAtlas)?;
    for group in &graph.groups {
        if group.iter().enumerate().any(|(index, row)| {
            row.class != 0
                || row
                    .adjacent
                    .iter()
                    .any(|&id| !group[usize::from(id)].adjacent.contains(&(index as u16)))
        }) {
            return Err(RebuildError::UnsupportedTopology);
        }
    }
    let mut half_cell_base = vec![UNASSIGNED; WIDTH * HEIGHT * 4];
    for (leaf, &base) in atlas.leaves.iter().zip(&graph.leaves) {
        let [x, y] = leaf.origin.map(usize::from);
        let side = 1_usize << leaf.level;
        for row in y..(y + side).min(HEIGHT * 2) {
            for column in x..(x + side).min(WIDTH * 2) {
                half_cell_base[row * WIDTH * 2 + column] = base;
            }
        }
    }
    Ok((half_cell_base, graph))
}

fn components(rows: &[Region], denied_territories: u16) -> Vec<u16> {
    let mut component = vec![BLOCKED; rows.len()];
    let mut pending = Vec::new();
    let denied = |row: &Region| denied_territories & (1 << row.territory) != 0;
    for (seed, row) in rows.iter().enumerate() {
        if component[seed] != BLOCKED || denied(row) {
            continue;
        }
        let label = seed as u16;
        component[seed] = label;
        pending.push(seed);
        while let Some(from) = pending.pop() {
            for &next in &rows[from].neighbors {
                let next = usize::from(next);
                if component[next] == BLOCKED && !denied(&rows[next]) {
                    component[next] = label;
                    pending.push(next);
                }
            }
        }
    }
    component
}
