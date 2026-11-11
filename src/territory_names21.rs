//! Opt-in solo territory name identities, assigned at ownership transitions.
//!
//! The pool search follows 0071D9B0, 00720080 and 0071FDE0. OpenEpoch's
//! ownership rounds and shared RNG schedule are the current game projection.
use crate::{game::Game, native_rng9::NativeRng, pack8_save, territory::TERR_NONE};
use crate::territory_name_catalog21::{NAME_LIMIT, POOLS, VERSION};

pub const MAGIC: u32 = u32::from_le_bytes(*b"TNM1");
const REGION_START: [usize; 5] = [0, 5, 8, 11, 14];
const CAPACITY: usize = crate::map::LEGACY_REGION_CAPACITY;
const HEADER: usize = 12 + CAPACITY * 2;

#[derive(Default)]
pub struct TerritoryNames {
    pub ids: [u16; CAPACITY],
}

impl TerritoryNames {
    fn search_region(&self, start: usize, region: usize, rng: &mut NativeRng) -> Option<u16> {
        let first = REGION_START[region];
        let count = REGION_START[region + 1] - first;
        for step in 0..count {
            let civilization = first + (start - first + step) % count;
            let pool = POOLS[civilization];
            let first_name = rng.next_word15() as usize % pool.len();
            for offset in 0..pool.len() {
                let name = pool[(first_name + offset) % pool.len()];
                if !self.ids.contains(&name) {
                    return Some(name);
                }
            }
        }
        None
    }

    fn assign(&mut self, territory: usize, civilization: u8, rng: &mut NativeRng) {
        if self.ids[territory] != 0 { return; }
        let civilization = civilization as usize;
        let region = crate::civ8_gates::REGION_BY_CIV[civilization] as usize;
        let mut selected = self.search_region(civilization, region, rng);
        for step in 1..4 {
            if selected.is_some() { break; }
            let other = (region + step) % 4;
            let first = REGION_START[other];
            let count = REGION_START[other + 1] - first;
            let start = first + rng.next_word15() as usize % count;
            selected = self.search_region(start, other, rng);
        }
        self.ids[territory] = selected.unwrap_or_default();
    }
}

impl Game {
    pub(crate) fn valid_fullmap_naming_profile(&self) -> bool {
        self.humans == 1 && !self.net && self.map.tseeds.len() <= CAPACITY
            && self.civilizations[..self.nplayers].iter().all(|&civ| (civ as usize) < POOLS.len())
    }

    pub(crate) fn configure_territory_names(&mut self, version: u32) -> bool {
        if version != VERSION || self.tick != 0 || self.territory_names.is_some()
            || !self.valid_fullmap_naming_profile() {
            return false;
        }
        self.territory_names = Some(TerritoryNames::default());
        true
    }

    pub(crate) fn name_territory_owner_changes(&mut self, owners: &[u8]) {
        let Some(names) = &mut self.territory_names else { return; };
        for (id, &owner) in owners.iter().enumerate().take(self.map.tseeds.len()) {
            if owner != TERR_NONE && owner != self.towner[id] {
                names.assign(id, self.civilizations[owner as usize], &mut self.source_birth.rng);
            }
        }
    }

    #[cfg(feature = "ee2-ordered-claims217")]
    pub(crate) fn name_territory_acquisition(&mut self, id: usize, owner: u8) {
        if let Some(names) = &mut self.territory_names {
            names.assign(id, self.civilizations[owner as usize], &mut self.source_birth.rng);
        }
    }

    pub(crate) fn restore_territory_names(&mut self, names: TerritoryNames) -> Option<()> {
        if !self.valid_fullmap_naming_profile() || names.ids[self.map.tseeds.len()..].iter().any(|&id| id != 0) {
            return None;
        }
        self.territory_names = Some(names);
        Some(())
    }
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, names: Option<&TerritoryNames>) -> Vec<u8> {
    let Some(names) = names.filter(|_| !payload.is_empty()) else { return payload; };
    let mut out = Vec::with_capacity(payload.len() + HEADER + 4);
    for value in [MAGIC, VERSION, payload.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for id in names.ids { out.extend_from_slice(&id.to_le_bytes()); }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<TerritoryNames>)> {
    if word(bytes, 0) != Some(MAGIC) { return Some((bytes, None)); }
    let end = HEADER.checked_add(word(bytes, 8)? as usize)?;
    if word(bytes, 4)? != VERSION || end.checked_add(4)? != bytes.len()
        || pack8_save::checksum(bytes.get(..end)?) != word(bytes, end)? {
        return None;
    }
    let mut names = TerritoryNames::default();
    for (index, pair) in bytes.get(12..HEADER)?.chunks_exact(2).enumerate() {
        let id = u16::from_le_bytes(pair.try_into().ok()?);
        if id != 0 && (!(2..NAME_LIMIT).contains(&id) || names.ids[..index].contains(&id)) {
            return None;
        }
        names.ids[index] = id;
    }
    Some((bytes.get(HEADER..end)?, Some(names)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::TC, map::tile_of};

    fn solo() -> Game {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 2, 0, 14));
        assert!(game.configure_territory_names(VERSION));
        game
    }

    #[test]
    fn ownership_and_removal_preserve_the_first_name() {
        let mut game = solo();
        let id = (0..game.map.tseeds.len()).find(|&id|
            game.towner[id] == TERR_NONE && game.territory_adjacent_owned(0, id)).unwrap();
        let (x, y) = game.map.tseeds[id];
        let city = game.spawn(TC, 0, x, y);
        game.ents[city].built = 0.0;
        let mut expected_rng = game.source_birth.rng;
        game.build_grid();
        assert_eq!(game.source_birth.rng, expected_rng);
        assert_eq!(game.territory_names.as_ref().unwrap().ids[id], 0);

        let expected = POOLS[0][expected_rng.next_word15() as usize % POOLS[0].len()];
        game.ents[city].built = 1.0;
        game.build_grid();
        assert_eq!(game.towner[id], 0);
        assert_eq!(game.territory_names.as_ref().unwrap().ids[id], expected);
        assert_eq!(game.source_birth.rng, expected_rng);
        if game.ordered_territory_rules() { game.rebuild_territories(); }
        assert!(game.apply_territory(0, 1, 0, id as u32, u32::MAX));
        assert_eq!(game.towner[id], 1);
        assert_eq!(game.territory_names.as_ref().unwrap().ids[id], expected);
        assert_eq!(game.source_birth.rng, expected_rng);

        game.ents[city].hp = 0.0;
        game.build_grid();
        assert_eq!(game.towner[id], 1);
        assert!(game.territory_is_disputed(id));
        assert_eq!(game.territory_names.as_ref().unwrap().ids[id], expected);
        assert_eq!(game.source_birth.rng, expected_rng);
    }

    #[test]
    fn named_territories_swap_and_restore_without_consuming_a_draw() {
        let mut game = solo();
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        let a = game.towner.iter().position(|&owner| owner == 0).unwrap();
        let b = game.towner.iter().position(|&owner| owner == 1).unwrap();
        let names = game.territory_names.as_mut().unwrap();
        names.ids[a] = POOLS[0][0];
        names.ids[b] = POOLS[1][0];
        let expected = names.ids;
        let expected_rng = game.source_birth.rng;
        assert!(game.apply_territory(0, 1, 1, a as u32, b as u32));
        assert!(game.apply_territory(1, 0, 2, u32::MAX, u32::MAX));
        assert_eq!((game.towner[a], game.towner[b]), (1, 0));
        assert_eq!(game.territory_names.as_ref().unwrap().ids, expected);
        assert_eq!(game.source_birth.rng, expected_rng);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.territory_names.as_ref().unwrap().ids, expected);
        assert_eq!(restored.source_birth.rng, expected_rng);
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
    }

    #[test]
    fn explored_cell_reveals_the_full_territory_bounds() {
        let mut game = solo();
        game.vis.fill(0);
        let (x, y) = game.map.starts[0];
        let cell = tile_of(x as f32, y as f32);
        game.vis[cell] = 128;
        let id = game.map.terr[cell];
        let hash = game.state_hash();
        assert_eq!(game.project_fullmap_territory_labels(), 1);
        let label = game.fullmap_territory_labels[0];
        assert_eq!(label.id, id.into());
        assert!(label.min_x <= x as u32 && label.max_x > x as u32);
        assert!(label.min_y <= y as u32 && label.max_y > y as u32);
        assert_eq!(game.state_hash(), hash);
        game.vis.fill(0);
        assert_eq!(game.project_fullmap_territory_labels(), 0);
        assert_eq!(game.fullmap_territory_labels[0].max_x, 0);
    }

    #[test]
    fn multiplayer_and_invalid_named_saves_stay_gated() {
        let mut network = Game::new(7, 0, 0, 2, 0);
        assert!(network.configure_match_epoch_range(0xee10, 500, 2, 0, 14));
        assert!(!network.configure_territory_names(VERSION));
        network.save();
        assert_ne!(word(&network.savebuf, 0), Some(MAGIC));
        assert!(Game::load(&wrap(network.savebuf, Some(&TerritoryNames::default()))).is_none());

        let mut game = solo();
        let name = POOLS[0][0];
        game.territory_names.as_mut().unwrap().ids[..2].fill(name);
        game.save();
        assert!(Game::load(&game.savebuf).is_none());
    }

    #[test]
    fn exhausted_civilization_pool_rotates_to_next_civilization() {
        let mut names = TerritoryNames::default();
        let inca = POOLS[11];
        names.ids[..inca.len()].copy_from_slice(inca);
        let mut rng = NativeRng::from_state(17);
        let mut expected_rng = rng;
        expected_rng.next_word15();
        let aztec = POOLS[12];
        let expected = aztec[expected_rng.next_word15() as usize % aztec.len()];
        names.assign(inca.len(), 11, &mut rng);
        assert_eq!(names.ids[inca.len()], expected);
        assert_eq!(rng, expected_rng);
    }
}
