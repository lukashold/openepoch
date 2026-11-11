//! Detached static path through original atlas admission 004A9290.

use super::atlas_reader::Precision53;
use super::movement_class::half_cell_obstructed;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchBox {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

pub trait StaticLeafProviders {
    fn cliff_mask(&mut self, tile: [i32; 2]) -> Option<u16>;
    fn possible_actor(&mut self, area: SearchBox) -> bool;
    fn actor_class(&mut self, area: SearchBox) -> Option<u8>;
    fn forest_dense(&mut self, center: [f32; 2], extent: f32) -> bool;
    fn terrain_class(&mut self, center: [f32; 2], water: bool, mountains: bool) -> u8;
    fn special_value(&mut self, tile: [i32; 2]) -> Option<i32>;
    fn raw_half_weight(&mut self, corner: [i32; 2]) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassWrite {
    pub class: u8,
    pub movement_weight: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StaticAdmission {
    Terminal(ClassWrite),
    Subdivide(Option<ClassWrite>),
}

pub fn classify_static_leaf(
    origin_half: [i16; 2],
    level: u8,
    world: [u16; 2],
    special_enabled: bool,
    precision: Precision53,
    providers: &mut impl StaticLeafProviders,
) -> Option<StaticAdmission> {
    if level > 8 || origin_half.into_iter().any(|axis| axis < 0) || world.into_iter().any(|size| size < 3) {
        return None;
    }
    let origin = origin_half.map(i32::from);
    let world = world.map(i32::from);
    let tile = origin.map(|axis| axis >> 1);
    let extent = (1_u32 << level) as f32 * 0.5;
    let min = origin.map(|axis| axis as f32 * 0.5);
    let center = min.map(|axis| axis + extent * 0.5);
    let area = SearchBox { min, max: min.map(|axis| axis + extent) };
    let ring = (0..2).any(|axis| tile[axis] == 0 || tile[axis] >= world[axis] - 2);
    let class = if ring {
        0x29
    } else if half_cell_obstructed(origin, providers.cliff_mask(tile)) {
        if level != 0 { return Some(StaticAdmission::Subdivide(None)); }
        0x27
    } else {
        let possible_actor = providers.possible_actor(area);
        if possible_actor {
            if level != 0 { return Some(StaticAdmission::Subdivide(None)); }
        }
        if let Some(actor) = possible_actor.then(|| providers.actor_class(area)).flatten() {
            actor
        } else if providers.forest_dense(center, extent) {
            4
        } else {
            let class = providers.terrain_class(center, true, true);
            if class > 7 { return None; }
            class
        }
    };
    let write = write_class(class, origin, level, center, world, precision, providers);
    if !ring && level == 1 && special_enabled && providers.special_value(tile).is_some_and(|value| value != -1) {
        return Some(StaticAdmission::Subdivide(Some(write)));
    }
    Some(StaticAdmission::Terminal(write))
}

pub(crate) fn write_class(
    mut class: u8,
    origin: [i32; 2],
    level: u8,
    center: [f32; 2],
    world: [i32; 2],
    precision: Precision53,
    providers: &mut impl StaticLeafProviders,
) -> ClassWrite {
    let movement_weight = if level == 0 && (0..2).all(|axis| center[axis] < (world[axis] - 1) as f32) {
        let raw = providers.raw_half_weight(origin.map(|axis| axis + 1));
        let positive = f64::from(raw.saturating_sub(7));
        let scale = f64::from(f32::from_bits(0x3e4c_cccd));
        let weight = precision.mul(precision.mul(positive, scale), 0.5);
        let threshold = f64::from(f32::from_bits(0x3fcb_851f));
        class = match (class, weight < threshold) {
            (7, true) => 5,
            (5, false) => 7,
            _ => class,
        };
        Some(precision.store_f32(weight))
    } else {
        None
    };
    ClassWrite { class, movement_weight }
}
