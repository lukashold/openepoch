//! DVP1 authenticated World0 population. This module never installs live actors.
use super::hash::sha256;
use std::{collections::HashSet, ops::Range};

#[path = "population234_catalog.rs"]
mod catalog;

const HEADER: usize = 112;
const UNIT_BYTES: usize = 64;
const FOREST_BYTES: usize = 20;
const CHILD_BYTES: usize = 16;
const ARCHIVE_BYTES: usize = 567_729;
pub const MAX_POPULATION_INPUT: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UnitKind {
    OrdinaryTree,
    ForageTree,
    Mineral,
    Fish,
    LiveAnimal,
    Carcass,
    Corpse,
    Ambient,
}

impl UnitKind {
    fn decode(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::OrdinaryTree,
            1 => Self::ForageTree,
            2 => Self::Mineral,
            3 => Self::Fish,
            4 => Self::LiveAnimal,
            5 => Self::Carcass,
            6 => Self::Corpse,
            7 => Self::Ambient,
            _ => return None,
        })
    }

    fn accepts_resource(self, resource: u8) -> bool {
        match self {
            Self::OrdinaryTree => resource == 1,
            Self::ForageTree | Self::Fish | Self::Carcass => resource == 0,
            Self::Mineral => (2..=8).contains(&resource),
            Self::LiveAnimal | Self::Corpse | Self::Ambient => resource == u8::MAX,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceUnit {
    pub packed_id: u32,
    pub source_archive: Range<usize>,
    pub type_index: u16,
    pub kind: UnitKind,
    /// Native resource enum. 255 denotes absence.
    pub resource: u8,
    pub xyz: [f32; 3],
    pub yaw: f32,
    pub hp_fraction: f32,
    pub city_id: i32,
    pub attacker_firepower: f32,
    pub flags: u32,
    pub visibility: u32,
    pub initial_stock: u32,
    pub child_range: Range<usize>,
}

impl SourceUnit {
    pub fn owner(&self) -> i32 { (self.packed_id as i32) >> 27 }

    pub fn local_id(&self) -> u32 { self.packed_id & 0x07ff_ffff }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceForest {
    /// Archive order is a separate identity namespace, independent of Unit IDs.
    pub ordinal: u32,
    pub source_offset: usize,
    pub xy_words: [u16; 2],
    pub type_index: u16,
    pub flags: u16,
    pub state: u8,
}

impl SourceForest {
    pub fn xy(&self) -> [f32; 2] { self.xy_words.map(|coordinate| f32::from(coordinate) / 128.0) }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildKind {
    Ability(u8),
    VisualState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceChild {
    pub source_offset: usize,
    pub kind: ChildKind,
    payload: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Population {
    dimensions: [u16; 2],
    types: Vec<String>,
    units: Vec<SourceUnit>,
    forest: Vec<SourceForest>,
    children: Vec<SourceChild>,
    opaque: Box<[u8]>,
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(data[at..at + 2].try_into().expect("checked record extent"))
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().expect("checked record extent"))
}

fn f32_at(data: &[u8], at: usize) -> f32 { f32::from_bits(u32_at(data, at)) }

fn decode_types(mut data: &[u8], count: usize) -> Option<Vec<String>> {
    let mut types = Vec::<String>::with_capacity(count);
    for _ in 0..count {
        let (length, rest) = data.split_at_checked(2)?;
        let (name, rest) = rest.split_at_checked(u16_at(length, 0) as usize)?;
        if name.is_empty() || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
            return None;
        }
        let name = std::str::from_utf8(name).ok()?;
        if types.last().is_some_and(|previous| previous.as_str() >= name) { return None; }
        types.push(name.to_owned());
        data = rest;
    }
    data.is_empty().then_some(types)
}

impl Population {
    pub fn dimensions(&self) -> [u16; 2] { self.dimensions }

    pub fn types(&self) -> &[String] { &self.types }

    pub fn units(&self) -> &[SourceUnit] { &self.units }

    pub fn forest(&self) -> &[SourceForest] { &self.forest }

    pub fn children(&self) -> &[SourceChild] { &self.children }

    pub(super) fn map_matches(source_sha: [u8; 32], input_sha: [u8; 32]) -> bool {
        source_sha == catalog::ARCHIVE_SHA && input_sha == catalog::PACKET_SHA
    }

    /// Validate the whole carrier and bind it to the already authenticated W8I1.
    /// A caller can stage the returned value without mutating the active game.
    pub fn decode(data: &[u8], source_sha: [u8; 32], input_sha: [u8; 32]) -> Option<Self> {
        if !Self::map_matches(source_sha, input_sha) ||
            !(HEADER..=MAX_POPULATION_INPUT).contains(&data.len()) || sha256(data) != catalog::CARRIER_SHA
        {
            return None;
        }
        Self::decode_records(data)
    }

    pub fn child_payload(&self, child_index: usize) -> Option<&[u8]> {
        self.opaque.get(self.children.get(child_index)?.payload.clone())
    }

    fn decode_records(data: &[u8]) -> Option<Self> {
        let (header, records) = data.split_at_checked(HEADER)?;
        let dimensions = [u16_at(header, 8), u16_at(header, 10)];
        let [unit_count, forest_count, type_count, child_count, type_bytes, opaque_bytes, total] =
            [12, 16, 20, 24, 28, 32, 36].map(|at| u32_at(header, at) as usize);
        if &header[..4] != b"DVP1" || u16_at(header, 4) != 1 || u16_at(header, 6) as usize != HEADER ||
            dimensions != [100, 300] || [unit_count, forest_count, type_count, child_count] != [766, 1328, 37, 1358] ||
            total != data.len() || header[40..72] != catalog::ARCHIVE_SHA || header[72..104] != catalog::PACKET_SHA ||
            header[104..].iter().any(|byte| *byte != 0)
        {
            return None;
        }
        let (type_rows, records) = records.split_at_checked(type_bytes)?;
        let (unit_rows, records) = records.split_at_checked(unit_count * UNIT_BYTES)?;
        let (forest_rows, records) = records.split_at_checked(forest_count * FOREST_BYTES)?;
        let (child_rows, opaque) = records.split_at_checked(child_count * CHILD_BYTES)?;
        if opaque.len() != opaque_bytes { return None; }
        let types = decode_types(type_rows, type_count)?;
        let mut children = Vec::with_capacity(child_count);
        let mut next_payload = 0;
        for row in child_rows.chunks_exact(CHILD_BYTES) {
            let source_offset = u32_at(row, 0) as usize;
            let start = u32_at(row, 4) as usize;
            let end = start.checked_add(u32_at(row, 8) as usize)?;
            let kind = match (row[12], row[13]) {
                (0, slot @ 0..=23) => ChildKind::Ability(slot),
                (1, 255) => ChildKind::VisualState,
                _ => return None,
            };
            if start != next_payload || end > opaque.len() || u16_at(row, 14) != 0 { return None; }
            next_payload = end;
            children.push(SourceChild { source_offset, kind, payload: start..end });
        }
        if next_payload != opaque.len() { return None; }
        let mut ids = HashSet::with_capacity(unit_count);
        let mut units = Vec::with_capacity(unit_count);
        let mut next_child = 0;
        let mut previous_unit_end = 0;
        for row in unit_rows.chunks_exact(UNIT_BYTES) {
            let start = u32_at(row, 4) as usize;
            let end = u32_at(row, 8) as usize;
            let first = u32_at(row, 56) as usize;
            let last = first.checked_add(u16_at(row, 60) as usize)?;
            let unit = SourceUnit {
                packed_id: u32_at(row, 0), source_archive: start..end, type_index: u16_at(row, 12),
                kind: UnitKind::decode(row[14])?, resource: row[15], xyz: [16, 20, 24].map(|at| f32_at(row, at)),
                yaw: f32_at(row, 28), hp_fraction: f32_at(row, 32), city_id: u32_at(row, 36) as i32,
                attacker_firepower: f32_at(row, 40), flags: u32_at(row, 44), visibility: u32_at(row, 48),
                initial_stock: u32_at(row, 52), child_range: first..last,
            };
            if !ids.insert(unit.packed_id) || unit.owner() != 0 || unit.local_id() == 0 ||
                start < previous_unit_end || start >= end || end > ARCHIVE_BYTES ||
                unit.type_index as usize >= type_count || !unit.kind.accepts_resource(unit.resource) ||
                (unit.resource == 255) != (unit.initial_stock == 0) ||
                !(0.0..f32::from(dimensions[0])).contains(&unit.xyz[0]) ||
                !(0.0..f32::from(dimensions[1])).contains(&unit.xyz[1]) || !unit.xyz[2].is_finite() ||
                !unit.yaw.is_finite() || unit.hp_fraction != 1.0 || unit.city_id != -1 ||
                !unit.attacker_firepower.is_finite() || !matches!(unit.visibility, 0 | u32::MAX) ||
                first != next_child || first == last || u16_at(row, 62) != 0
            {
                return None;
            }
            let mut previous_child_end = start;
            let mut previous_slot = None;
            let mut resource_seen = false;
            let mut visual_seen = false;
            for child in children.get(first..last)? {
                let payload_len = child.payload.len();
                let child_end = child.source_offset.checked_add(20)?.checked_add(payload_len)?;
                if child.source_offset <= previous_child_end || child_end > end || visual_seen { return None; }
                match child.kind {
                    ChildKind::Ability(slot) => {
                        if previous_slot.is_some_and(|old| slot <= old) { return None; }
                        previous_slot = Some(slot);
                        resource_seen |= slot == 8 && payload_len == 0;
                    }
                    ChildKind::VisualState => visual_seen = true,
                }
                previous_child_end = child_end;
            }
            if resource_seen != (unit.resource != 255) { return None; }
            previous_unit_end = end;
            next_child = last;
            units.push(unit);
        }
        if next_child != child_count { return None; }
        let mut forest = Vec::with_capacity(forest_count);
        let mut previous_offset = previous_unit_end;
        for (ordinal, row) in forest_rows.chunks_exact(FOREST_BYTES).enumerate() {
            let tree = SourceForest {
                ordinal: u32_at(row, 0), source_offset: u32_at(row, 4) as usize,
                xy_words: [u16_at(row, 8), u16_at(row, 10)], type_index: u16_at(row, 12),
                flags: u16_at(row, 14), state: row[16],
            };
            if tree.ordinal as usize != ordinal || tree.source_offset <= previous_offset ||
                tree.source_offset >= ARCHIVE_BYTES || tree.type_index as usize >= type_count ||
                tree.xy_words.iter().zip(dimensions).any(|(xy, limit)| u32::from(*xy) >= u32::from(limit) * 128) ||
                !matches!((tree.flags, tree.state), (0x1a0 | 0x180, 0) | (0x80, 3)) ||
                row[17..].iter().any(|byte| *byte != 0)
            {
                return None;
            }
            previous_offset = tree.source_offset;
            forest.push(tree);
        }
        Some(Self { dimensions, types, units, forest, children, opaque: opaque.into() })
    }
}
