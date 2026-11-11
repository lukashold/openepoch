//! Per-game read-only source objects before individual gameplay bindings.
use super::population234::{Population, SourceUnit};
use crate::game::Game;
use std::collections::BTreeMap;

pub(crate) struct AttachedPopulation {
    source: Population,
    units_by_source_id: BTreeMap<u32, usize>,
    resources_by_cell: BTreeMap<(u16, u16), Vec<usize>>,
    pub(super) remaining: Vec<f32>,
    #[cfg(feature = "ee2-rect242-hunt")]
    pub(super) wildlife: BTreeMap<u32, super::world0_hunt242::WildlifeState>,
    #[cfg(feature = "ee2-rect244-hunt-visual")]
    pub(super) hunt_visuals: BTreeMap<u32, super::hunt_visual244::HuntVisual>,
    #[cfg(feature = "ee2-rect244-hunt-visual")]
    pub(super) hunt_visual_cursors: Vec<Option<(u32, super::hunt_visual244::HuntCursor)>>,
    #[cfg(feature = "ee2-rect246-deer-live")]
    pub(super) deer_motion: BTreeMap<u32, super::deer_live246::DeerMotion>,
    #[cfg(feature = "ee2-rect250-animals")]
    pub(super) animals: Option<super::animal_live250::AnimalManager>,
    #[cfg(feature = "ee2-rect252-animal-birth")]
    pub(super) newborns: Vec<SourceUnit>,
}

impl AttachedPopulation {
    pub(crate) fn source(&self) -> &Population { &self.source }

    pub(super) fn index_of(&self, packed_source_id: u32) -> Option<usize> {
        self.units_by_source_id.get(&packed_source_id).copied()
    }

    pub(crate) fn unit(&self, packed_source_id: u32) -> Option<&SourceUnit> {
        if let Some(index) = self.index_of(packed_source_id) { return self.source.units().get(index); }
        #[cfg(feature = "ee2-rect252-animal-birth")]
        { return self.newborns.iter().find(|unit| unit.packed_id == packed_source_id); }
        #[cfg(not(feature = "ee2-rect252-animal-birth"))]
        None
    }

    pub(super) fn unit_count(&self) -> usize {
        let count = self.source.units().len();
        #[cfg(feature = "ee2-rect252-animal-birth")]
        let count = count + self.newborns.len();
        count
    }

    pub(super) fn unit_by_slot(&self, slot: usize) -> Option<&SourceUnit> {
        if let Some(unit) = self.source.units().get(slot) { return Some(unit); }
        #[cfg(feature = "ee2-rect252-animal-birth")]
        { return self.newborns.get(slot - self.source.units().len()); }
        #[cfg(not(feature = "ee2-rect252-animal-birth"))]
        None
    }

    #[cfg(feature = "ee2-rect252-animal-birth")]
    pub(super) fn newborn_deer(&self, id: u32, xyz: [f32; 3]) -> Option<SourceUnit> {
        let type_index = self.source.types().iter().position(|name| name == "Deer")? as u16;
        Some(SourceUnit {
            packed_id: id, source_archive: 0..0, type_index, kind: super::population234::UnitKind::LiveAnimal,
            resource: u8::MAX, xyz, yaw: super::animal_birth252::NEWBORN_YAW, hp_fraction: 1.0,
            city_id: -1, attacker_firepower: 0.0, flags: 0, visibility: 0, initial_stock: 0, child_range: 0..0,
        })
    }

    pub(crate) fn resources_at(&self, x: u16, y: u16) -> impl Iterator<Item = &SourceUnit> {
        self.resources_by_cell.get(&(x, y)).into_iter().flatten().map(|&i| &self.source.units()[i])
    }
}

impl Population {
    /// Attach only to the corresponding authenticated world. Preserve every
    /// source coordinate and stock value even when several rows share a tile.
    /// Live entity handles and gather/forest state are outside this attachment.
    pub(crate) fn attach(&self, game: &mut Game) -> bool {
        let Some(world) = game.map.source_world.as_ref() else { return false; };
        if game.rect_population.is_some() || !Self::map_matches(world.source_sha, world.input_sha) ||
            [world.width, world.height] != self.dimensions().map(usize::from) ||
            world.pending_units as usize != self.units().len() ||
            world.pending_forest as usize != self.forest().len()
        {
            return false;
        }
        let mut units_by_source_id = BTreeMap::new();
        let mut resources_by_cell = BTreeMap::<(u16, u16), Vec<usize>>::new();
        for (index, unit) in self.units().iter().enumerate() {
            units_by_source_id.insert(unit.packed_id, index);
            if unit.initial_stock != 0 {
                let [x, y, _] = unit.xyz;
                resources_by_cell.entry((x as u16, y as u16)).or_default().push(index);
            }
        }
        let remaining = self.units().iter().map(|unit| unit.initial_stock as f32).collect();
        game.rect_population = Some(AttachedPopulation {
            source: self.clone(), units_by_source_id, resources_by_cell, remaining,
            #[cfg(feature = "ee2-rect242-hunt")]
            wildlife: BTreeMap::new(),
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            hunt_visuals: BTreeMap::new(),
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            hunt_visual_cursors: vec![None; crate::game::MAXE],
            #[cfg(feature = "ee2-rect246-deer-live")]
            deer_motion: BTreeMap::new(),
            #[cfg(feature = "ee2-rect250-animals")]
            animals: None,
            #[cfg(feature = "ee2-rect252-animal-birth")]
            newborns: Vec::new(),
        });
        true
    }
}
