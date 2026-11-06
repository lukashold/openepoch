//! Opt-in generated-skirmish city identities keyed by completed actor handle.
use crate::{city_name_catalog22::{NAME_LIMIT, POOLS, VERSION}, defs::TC,
    game::{Ent, Game, MAXE}, native_rng9::NativeRng, pack8_save};
use std::collections::BTreeMap;

pub const MAGIC: u32 = u32::from_le_bytes(*b"CTN1");
const HEADER: usize = 16;

#[derive(Default)]
pub struct CityNames {
    pub ids: BTreeMap<u32, u16>,
}

impl CityNames {
    fn allocate(&mut self, handle: u32, civ: u8, rng: &mut NativeRng) {
        if self.ids.contains_key(&handle) { return; }
        let pool = POOLS[civ as usize];
        let start = rng.next_word15() as usize % pool.len();
        // 008123D0 probes the pool cyclically and uses the initial result
        // after exhaustion. Generated maps have no authored reserved names.
        for offset in 0..pool.len() {
            let name = pool[(start + offset) % pool.len()];
            if !self.ids.values().any(|&used| used == name) {
                self.ids.insert(handle, name);
                return;
            }
        }
        self.ids.insert(handle, pool[start]);
    }
}

impl Game {
    pub(crate) fn configure_city_names(&mut self, version: u32) -> bool {
        if version != VERSION || self.tick != 0 || self.city_names.is_some()
            || !self.valid_fullmap_naming_profile() {
            return false;
        }
        self.city_names = Some(CityNames::default());
        true
    }

    pub(crate) fn name_completed_city(&mut self, index: usize, city: Ent) {
        if !self.city_ground_active(&city) { return; }
        if let Some(names) = &mut self.city_names {
            let handle = ((city.gen as u32) << 16) | index as u32;
            names.allocate(handle, self.civilizations[city.team as usize], &mut self.source_birth.rng);
        }
    }

    pub(crate) fn retire_city_name(&mut self, index: usize) {
        if self.ents[index].def != TC { return; }
        let handle = self.h(index);
        if let Some(names) = &mut self.city_names { names.ids.remove(&handle); }
    }

    pub(crate) fn restore_city_names(&mut self, names: CityNames) -> Option<()> {
        if !self.valid_fullmap_naming_profile() { return None; }
        for &handle in names.ids.keys() {
            let index = self.get(handle)?;
            if index >= self.high || !self.city_ground_active(&self.ents[index]) { return None; }
        }
        self.city_names = Some(names);
        Some(())
    }
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, names: Option<&CityNames>) -> Vec<u8> {
    let Some(names) = names.filter(|_| !payload.is_empty()) else { return payload; };
    let mut out = Vec::with_capacity(HEADER + names.ids.len() * 8 + payload.len() + 4);
    for value in [MAGIC, VERSION, payload.len() as u32, names.ids.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for (&handle, &name) in &names.ids {
        out.extend_from_slice(&handle.to_le_bytes());
        out.extend_from_slice(&u32::from(name).to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<CityNames>)> {
    if word(bytes, 0) != Some(MAGIC) { return Some((bytes, None)); }
    let count = word(bytes, 12)? as usize;
    if count > MAXE { return None; }
    let start = HEADER + count * 8;
    let end = start.checked_add(word(bytes, 8)? as usize)?;
    if word(bytes, 4)? != VERSION || end.checked_add(4)? != bytes.len()
        || pack8_save::checksum(bytes.get(..end)?) != word(bytes, end)? {
        return None;
    }
    let mut names = CityNames::default();
    let mut previous = None;
    for index in 0..count {
        let offset = HEADER + index * 8;
        let handle = word(bytes, offset)?;
        let id = u16::try_from(word(bytes, offset + 4)?).ok()?;
        if previous.is_some_and(|last| handle <= last) || !(1..NAME_LIMIT).contains(&id) { return None; }
        names.ids.insert(handle, id);
        previous = Some(handle);
    }
    Some((bytes.get(start..end)?, Some(names)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cmd::C_DELETE, defs::WORKER, game::NOIDX, map::{tile_of, N}, territory::TERR_NONE};

    fn solo() -> Game {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 2, 0, 14));
        assert!(game.configure_city_names(VERSION));
        game.players[1].ai = false;
        game
    }

    #[test]
    fn actual_worker_completion_registers_once_and_delete_retires_the_handle() {
        let mut game = solo();
        let worker = game.ents[..game.high].iter().position(|e| e.team == 0 && e.def == WORKER).unwrap();
        game.add_resource(0, 1, 2_000.0);
        game.add_resource(0, 3, 2_000.0);
        game.diplomacy.visibility = 2;
        game.refresh_display_fog();
        let side = Game::footprint_side(TC, 0);
        let mut site = None;
        'tiles: for y in (1..N as i32 - side).step_by(2) {
            for x in (1..N as i32 - side).step_by(2) {
                let at = (y + side / 2) as usize * N + (x + side / 2) as usize;
                let territory = game.map.terr[at] as usize;
                if game.towner[territory] == TERR_NONE && game.territory_adjacent_owned(0, territory)
                    && game.placement_error_for(0, TC, x, y, 0, &[worker], true) == 0 {
                    site = Some((x, y));
                    break 'tiles;
                }
            }
        }
        let (x, y) = site.expect("generated site for a paid expansion");
        let city = game.begin_construction(0, TC, x, y, 0, &[worker], true);
        assert_ne!(city, NOIDX);
        let handle = game.h(city);
        let mut expected_rng = game.source_birth.rng;
        assert!(!game.city_names.as_ref().unwrap().ids.contains_key(&handle));
        game.ents[worker].x = x as f32 - 0.5;
        game.ents[worker].y = y as f32 + side as f32 * 0.5;
        game.ents[city].built = 0.9999;
        game.tick();
        assert_eq!(game.ents[city].built, 1.0);
        let expected = POOLS[0][expected_rng.next_word15() as usize % POOLS[0].len()];
        assert_eq!(game.city_names.as_ref().unwrap().ids.get(&handle), Some(&expected));
        assert_eq!(game.source_birth.rng, expected_rng);
        game.apply(0, C_DELETE, 0.0, 0.0, 0, &[handle]);
        assert!(!game.city_names.as_ref().unwrap().ids.contains_key(&handle));
        game.save();
        let restored = Game::load(&game.savebuf).unwrap();
        assert!(restored.city_names.as_ref().unwrap().ids.is_empty());
    }

    #[test]
    fn current_city_labels_do_not_use_explored_only_tiles() {
        let mut game = solo();
        let enemy = game.ents[..game.high].iter().position(|e| e.team == 1 && e.def == TC).unwrap();
        let tile = tile_of(game.ents[enemy].x, game.ents[enemy].y);
        let handle = game.h(enemy);
        game.team_vis.fill(0);
        game.team_vis[tile] = 128;
        let hash = game.state_hash();
        assert_eq!(game.project_fullmap_city_labels(), 1);
        assert!(game.fullmap_city_labels.iter().all(|city| city.handle != handle));
        assert_eq!(game.state_hash(), hash);
        game.team_vis[tile] = 255;
        assert_eq!(game.project_fullmap_city_labels(), 2);
        assert!(game.fullmap_city_labels.iter().any(|city| city.handle == handle));
    }

    #[test]
    fn city_pool_exhaustion_reuses_only_the_original_rolled_slot() {
        let mut names = CityNames::default();
        let pool = POOLS[11];
        for (index, &id) in pool.iter().enumerate() { names.ids.insert(index as u32, id); }
        let mut rng = NativeRng::from_state(71);
        let mut expected_rng = rng;
        let expected = pool[expected_rng.next_word15() as usize % pool.len()];
        names.allocate(999, 11, &mut rng);
        assert_eq!(names.ids[&999], expected);
        names.allocate(999, 11, &mut rng);
        assert_eq!(rng, expected_rng);
    }

    #[test]
    fn save_rejects_an_unresolved_actor_handle() {
        let mut game = solo();
        game.city_names.as_mut().unwrap().ids.insert(u32::MAX, POOLS[0][0]);
        game.save();
        assert!(Game::load(&game.savebuf).is_none());
    }
}
