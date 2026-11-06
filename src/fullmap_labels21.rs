//! Typed Fullmap territory labels projected from the current player's map view.
use crate::{game::Game, map::N, territory::TERR_NONE};

pub const CAPACITY: usize = 32;
pub const UNCLAIMED: u32 = 0;
pub const UNNAMED: u32 = 1;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TerritoryLabel {
    pub id: u32,
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub name: u32,
}

#[repr(C)]
pub struct CityLabel {
    pub handle: u32,
    pub x: f32,
    pub y: f32,
    pub name: u32,
}

const _: () = assert!(core::mem::size_of::<TerritoryLabel>() == 6 * 4);
const _: () = assert!(core::mem::size_of::<CityLabel>() == 4 * 4);

impl Game {
    pub(crate) fn project_fullmap_territory_labels(&mut self) -> u32 {
        // 0071AD20 bounds occupied cells, including each cell's far edge.
        let mut bounds = [TerritoryLabel { min_x: N as u32, min_y: N as u32, ..Default::default() }; CAPACITY];
        let mut discovered = [false; CAPACITY];
        for (tile, &id) in self.map.terr.iter().enumerate() {
            let index = id as usize;
            let area = &mut bounds[index];
            let x = (tile % N) as u32;
            let y = (tile / N) as u32;
            area.min_x = area.min_x.min(x);
            area.min_y = area.min_y.min(y);
            area.max_x = area.max_x.max(x + 1);
            area.max_y = area.max_y.max(y + 1);
            // 00727B90 accepts any matching current-or-explored cell. The
            // display layer already folds local shroud and reveal overrides in.
            discovered[index] |= self.vis[tile] != 0;
        }
        self.fullmap_territory_labels.fill(TerritoryLabel::default());
        let mut count = 0;
        for (id, mut area) in bounds.into_iter().enumerate().take(self.map.tseeds.len()) {
            if !discovered[id] { continue; }
            area.id = id as u32;
            area.name = match self.territory_names.as_ref().map_or(0, |names| names.ids[id]) {
                0 if self.towner[id] == TERR_NONE => UNCLAIMED,
                0 => UNNAMED,
                name => name.into(),
            };
            self.fullmap_territory_labels[count] = area;
            count += 1;
        }
        count as u32
    }

    pub(crate) fn project_fullmap_city_labels(&mut self) -> u32 {
        self.fullmap_city_labels.clear();
        let revealed = self.display_visibility() == 2;
        for (index, city) in self.ents[..self.high].iter().enumerate() {
            if !self.city_ground_active(city) { continue; }
            let visible = if self.city_memory.is_some() {
                self.city_currently_seen(index, city)
            } else {
                self.can_see(self.local, index) || (revealed && self.detected(self.local, index))
            };
            if !visible { continue; }
            let handle = self.h(index);
            let name = self.fullmap_city_name_id(handle);
            self.fullmap_city_labels.push(CityLabel {handle, x: city.x, y: city.y, name: name.into()});
        }
        if let Some(known) = &self.city_memory {
            for (&handle, old) in known {
                if self.snapshot_still_hidden(handle, old) {
                    self.fullmap_city_labels.push(CityLabel {handle, x:old.x, y:old.y, name:old.name.into()});
                }
            }
        }
        self.fullmap_city_labels.len() as u32
    }
}
