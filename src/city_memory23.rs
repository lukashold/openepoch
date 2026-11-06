//! Optional local Fullmap memory for completed CityCenters in generated UP matches.
use crate::{city_name_catalog22::NAME_LIMIT, game::{Ent, Game, MAXE}, map::N, pack8_save};
use std::collections::BTreeMap;

const MAGIC: u32 = u32::from_le_bytes(*b"CSM1");
const VERSION: u32 = 1;
const HEADER: usize = 16;
const RECORD: usize = 24;
const COORDINATE_BIAS: f64 = 103_079_215_104.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CitySnapshot {
    pub x: f32,
    pub y: f32,
    angle: f32,
    owner: u8,
    pub name: u16,
}

pub(crate) type CityMemory = BTreeMap<u32, CitySnapshot>;

fn cell(value: f32) -> Option<usize> {
    let coord = ((f64::from(value) + COORDINATE_BIAS).to_bits() >> 16) as i32;
    usize::try_from(coord).ok().filter(|&coord| coord < N)
}

fn sample_tiles(x: f32, y: f32, angle: f32) -> impl Iterator<Item = usize> {
    // 0053A8D0 imports rectangle axes from SizeX/SizeY. For this city
    // 00871F30 mode 1 shrinks their half-extents by the native 0.01f.
    let size_x = 3.5 * core::f32::consts::SQRT_2;
    let size_y = 3.0 * core::f32::consts::SQRT_2;
    let extent_x = (f64::from(size_x) * 0.5 - f64::from(0.01_f32)) as f32;
    let extent_y = (f64::from(size_y) * 0.5 - f64::from(0.01_f32)) as f32;
    let (sin, cos) = f64::from(angle).sin_cos();
    let (sin, cos) = (sin as f32, cos as f32);
    let (ax, ay) = (cos * extent_x, sin * extent_x);
    let (bx, by) = (-sin * extent_y, cos * extent_y);
    // The third axis is vertical for the CityCenter box. Source corner 3
    // combines both horizontal subtractions before rounding to a float.
    let corners = [
        ((x + ax) + bx, (y + ay) + by),
        ((x + ax) - bx, (y + ay) - by),
        ((f64::from(x) - f64::from(ax) - f64::from(bx)) as f32,
         (f64::from(y) - f64::from(ay) - f64::from(by)) as f32),
        ((x - ax) + bx, (y - ay) + by),
    ];
    corners.into_iter().filter_map(|(x,y)| Some(cell(y)? * N + cell(x)?))
}

impl Game {
    pub(crate) fn configure_city_memory(&mut self) -> bool {
        if self.tick != 0 || self.bench != 0 || self.city_memory.is_some() || !self.valid_fullmap_naming_profile() {
            return false;
        }
        self.city_memory = Some(CityMemory::new());
        self.refresh_city_memory();
        true
    }

    pub(crate) fn fullmap_city_name_id(&self, handle: u32) -> u16 {
        self.city_names.as_ref().and_then(|names| names.ids.get(&handle)).copied().unwrap_or_default()
    }

    pub(crate) fn city_footprint_seen(&self, x: f32, y: f32, angle: f32) -> bool {
        self.display_visibility() == 2
            || sample_tiles(x, y, angle).any(|tile| self.real_fog(self.local, tile) == 255)
    }

    pub(crate) fn city_currently_seen(&self, index: usize, city: &Ent) -> bool {
        city.team == self.local || (self.detected(self.local,index)
            && self.city_footprint_seen(city.x, city.y, city.rot))
    }

    pub(crate) fn snapshot_still_hidden(&self, handle: u32, snapshot: &CitySnapshot) -> bool {
        !self.city_footprint_seen(snapshot.x, snapshot.y, snapshot.angle)
            && !self.get(handle).is_some_and(|index| {
                let city = &self.ents[index];
                self.city_ground_active(city) && self.city_currently_seen(index, city)
            })
    }

    pub(crate) fn refresh_city_memory(&mut self) {
        let Some(mut known) = self.city_memory.take() else { return; };
        // Prune on current corner sight, then capture from visible actors.
        // Hidden ownership, name and removal never change a captured record.
        known.retain(|_, old| !self.city_footprint_seen(old.x, old.y, old.angle));
        for (index, city) in self.ents[..self.high].iter().enumerate() {
            if city.team == self.local || !self.city_ground_active(city)
                || !self.city_currently_seen(index, city) { continue; }
            let handle = self.h(index);
            known.insert(handle, CitySnapshot {x:city.x, y:city.y, angle:city.rot,
                owner:city.team, name:self.fullmap_city_name_id(handle)});
        }
        self.city_memory = Some(known);
    }

    pub(crate) fn restore_city_memory(&mut self, memory: CityMemory) -> Option<()> {
        if self.bench != 0 || !self.valid_fullmap_naming_profile()
            || memory.values().any(|old| old.owner == self.local || old.owner as usize >= self.nplayers) { return None; }
        self.city_memory = Some(memory);
        Some(())
    }
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, memory: Option<&CityMemory>) -> Vec<u8> {
    let Some(memory) = memory.filter(|_| !payload.is_empty()) else { return payload; };
    let mut out = Vec::with_capacity(HEADER + memory.len() * RECORD + payload.len() + 4);
    for value in [MAGIC, VERSION, payload.len() as u32, memory.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for (&handle, old) in memory {
        for value in [handle, old.owner.into(), old.name.into(), old.x.to_bits(), old.y.to_bits(), old.angle.to_bits()] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<CityMemory>)> {
    if word(bytes, 0) != Some(MAGIC) { return Some((bytes, None)); }
    let count = word(bytes, 12)? as usize;
    let start = HEADER.checked_add(count.checked_mul(RECORD)?)?;
    let end = start.checked_add(word(bytes, 8)? as usize)?;
    if word(bytes, 4)? != VERSION || end.checked_add(4)? != bytes.len()
        || pack8_save::checksum(bytes.get(..end)?) != word(bytes, end)? { return None; }
    let mut memory = CityMemory::new();
    let mut previous = None;
    for index in 0..count {
        let offset = HEADER + index * RECORD;
        let handle = word(bytes, offset)?;
        let owner = u8::try_from(word(bytes, offset + 4)?).ok()?;
        let name = u16::try_from(word(bytes, offset + 8)?).ok()?;
        let x = f32::from_bits(word(bytes, offset + 12)?);
        let y = f32::from_bits(word(bytes, offset + 16)?);
        let angle = f32::from_bits(word(bytes, offset + 20)?);
        if previous.is_some_and(|last| handle <= last) || (handle & 0xffff) as usize >= MAXE
            || name >= NAME_LIMIT || !(0.0..N as f32).contains(&x) || !(0.0..N as f32).contains(&y)
            || !angle.is_finite() { return None; }
        memory.insert(handle, CitySnapshot {x, y, angle, owner, name});
        previous = Some(handle);
    }
    Some((bytes.get(start..end)?, Some(memory)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{city_name_catalog22::{POOLS, VERSION as NAMES}, cmd::C_DELETE,
        defs::{TC, WORKER}, game::NONE, map::tile_of};

    #[test]
    fn hidden_city_keeps_seen_name_and_owner_then_updates_on_corner_sight() {
        let mut game = Game::new(7, 2, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xe210, 500, 2, 0, 14));
        assert!(game.configure_city_names(NAMES));
        assert!(game.configure_city_memory());
        let index = game.ents[..game.high].iter().position(|actor| actor.team == 1 && actor.def == TC).unwrap();
        let city = game.ents[index];
        let handle = game.h(index);
        let corner = sample_tiles(city.x, city.y, city.rot).next().unwrap();
        let center = tile_of(city.x, city.y);
        assert_ne!(corner, center);
        let names = POOLS[0];
        game.city_names.as_mut().unwrap().ids.insert(handle, names[0]);
        game.team_vis.fill(0);
        game.team_vis[center] = 255;
        game.refresh_city_memory();
        assert!(!game.city_memory.as_ref().unwrap().contains_key(&handle));
        game.team_vis[corner] = 255;
        game.refresh_city_memory();
        let seen = game.city_memory.as_ref().unwrap()[&handle];
        assert_eq!(seen.owner, 1);
        assert_eq!(seen.name, names[0]);

        game.team_vis[corner] = 128;
        assert!(game.apply_territory(1, 2, 0, game.map.terr[center] as u32, NONE));
        assert_eq!(game.ents[index].team, 2);
        game.city_names.as_mut().unwrap().ids.insert(handle, names[1]);
        game.refresh_city_memory();
        assert_eq!(game.city_memory.as_ref().unwrap()[&handle], seen);
        game.project_fullmap_city_labels();
        let row = game.fullmap_city_labels.iter().find(|row| row.handle == handle).unwrap();
        assert_eq!(row.name, u32::from(seen.name));
        game.team_vis[corner] = 255;
        game.refresh_city_memory();
        assert_eq!(game.city_memory.as_ref().unwrap()[&handle].owner, 2);
        assert_eq!(game.city_memory.as_ref().unwrap()[&handle].name, names[1]);
        game.project_fullmap_city_labels();
        let mut rows = game.fullmap_city_labels.iter().filter(|row| row.handle == handle);
        assert_eq!(rows.next().unwrap().name, u32::from(names[1]));
        assert!(rows.next().is_none());
    }

    #[test]
    fn hidden_destruction_survives_save_until_the_original_footprint_is_seen() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 2, 0, 14));
        assert!(game.configure_city_names(NAMES));
        assert!(game.configure_city_memory());
        game.players[1].ai = false;
        let index = game.ents[..game.high].iter().position(|actor| actor.team == 1 && actor.def == TC).unwrap();
        let city = game.ents[index];
        let handle = game.h(index);
        let corner = sample_tiles(city.x, city.y, city.rot).next().unwrap();
        game.city_names.as_mut().unwrap().ids.insert(handle, POOLS[0][0]);
        game.team_vis.fill(0);
        game.team_vis[corner] = 255;
        game.refresh_city_memory();
        let seen = game.city_memory.as_ref().unwrap()[&handle];
        game.team_vis[corner] = 128;
        game.refresh_display_fog();
        game.refresh_city_memory();
        game.apply(1, C_DELETE, 0.0, 0.0, 0, &[handle]);
        game.tick();
        assert!(game.get(handle).is_none());
        assert!(!game.city_names.as_ref().unwrap().ids.contains_key(&handle));
        let state = game.state_hash();
        game.save();
        let bytes = game.savebuf.clone();
        let mut loaded = Game::load(&bytes).unwrap();
        loaded.save();
        assert_eq!(loaded.savebuf, bytes);
        assert_eq!(loaded.state_hash(), state);
        assert_eq!(loaded.city_memory.as_ref().unwrap()[&handle], seen);
        loaded.project_fullmap_city_labels();
        assert!(loaded.fullmap_city_labels.iter().any(|row| row.handle == handle && row.name == u32::from(seen.name)));
        assert_eq!(loaded.state_hash(), state);
        loaded.team_vis[corner] = 255;
        loaded.refresh_city_memory();
        assert!(!loaded.city_memory.as_ref().unwrap().contains_key(&handle));
        loaded.project_fullmap_city_labels();
        assert!(loaded.fullmap_city_labels.iter().all(|row| row.handle != handle));
        let mut bad = bytes;
        bad[20] ^= 4;
        assert!(Game::load(&bad).is_none());
    }

    #[test]
    fn normal_fog_update_keeps_a_city_after_the_observer_dies() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 2, 0, 14));
        assert!(game.configure_city_memory());
        game.players[1].ai = false;
        let index = game.ents[..game.high].iter().position(|actor| actor.team == 1 && actor.def == TC).unwrap();
        let city = game.ents[index];
        let handle = game.h(index);
        assert!(!game.city_currently_seen(index, &city));
        assert!(!game.city_memory.as_ref().unwrap().contains_key(&handle));
        let observer = game.spawn(WORKER, 0, city.x + 3.0, city.y);
        game.update_fog();
        assert!(game.city_currently_seen(index, &city));
        let snapshot = game.city_memory.as_ref().unwrap()[&handle];
        game.apply(0, C_DELETE, 0.0, 0.0, 0, &[game.h(observer)]);
        for _ in 0..4 { game.tick(); }
        assert!(!game.city_currently_seen(index, &city));
        assert_eq!(game.city_memory.as_ref().unwrap()[&handle], snapshot);
    }
}
