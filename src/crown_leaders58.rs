//! Crown leader creation and the explicit archive capability.

use super::*;

const MAGIC: [u8; 4] = *b"LDR1";
const VERSION: [u8; 4] = 1_u32.to_le_bytes();
pub(super) const HASH_TAG: u32 = u32::from_le_bytes(MAGIC);

pub(crate) fn wrap(core: Vec<u8>) -> Vec<u8> {
    if core.is_empty() { return core; }
    let mut bytes = Vec::with_capacity(core.len() + 12);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&VERSION);
    bytes.extend_from_slice(&core);
    bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
    bytes
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], bool)> {
    if !bytes.starts_with(&MAGIC) { return Some((bytes, false)); }
    let end = bytes.len().checked_sub(4)?;
    let checksum = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if end <= 8 || bytes.get(4..8)? != VERSION || crate::save::checksum(&bytes[..end]) != checksum { return None; }
    Some((&bytes[8..end], true))
}

impl Game {
    pub(crate) fn leader_power_reserve(&self, index: usize) -> f32 {
        let actor = &self.ents[index];
        if self.military_leader_powers.is_none() || !actor.alive || actor.hp <= 0.0
            || actor.stuck == BOARDED || crate::leader58_defs::branch(actor.def).is_none() {
            return -1.0;
        }
        self.powers.as_ref().and_then(|powers| powers.actor(self.h(index))).map_or(-1.0, |record| record.charge)
    }

    pub(crate) fn configure_crown_leaders(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 || self.crown_leaders
            || self.crowns.is_none() || self.powers.is_none() { return false; }
        self.crown_leaders = true;
        true
    }

    pub(super) fn award_crown_leader(&mut self, branch: usize, owner: u8) {
        let definition = crate::leader58_defs::FIRST + branch as DefId;
        let live = &self.ents[..self.high];
        if live.iter().any(|actor| actor.alive && actor.hp > 0.0 && actor.team == owner
            && (actor.def == definition || actor.cargo[..actor.ncargo as usize].contains(&definition))) { return; }
        let mut centers: Vec<_> = live.iter().copied().enumerate().filter_map(|(index, actor)| {
            if !actor.alive || actor.hp <= 0.0 || actor.built < 1.0 || actor.team != owner || actor.def != TC {
                return None;
            }
            let registered = self.powers.as_ref()?.actor(self.h(index))?;
            Some((registered.serial, actor))
        }).collect();
        if centers.is_empty() { return; }
        // SelectedUnitTypes iterates the native per-type list from its most recent insertion.
        centers.sort_unstable_by_key(|(serial, _)| core::cmp::Reverse(*serial));
        let (_, center) = centers[self.source_birth.rng.next_word15() as usize % centers.len()];
        let (x, y) = self.spawn_point(&center, &DEFS[TC as usize], &DEFS[definition as usize]);
        let actor = self.spawn(definition, owner, x, y);
        if actor != NOIDX && owner == self.local { self.ev(35.0, x, y); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaders(game: &Game, owner: u8) -> Vec<(u32, DefId)> {
        game.ents[..game.high].iter().enumerate().filter(|(_, actor)| actor.alive && actor.team == owner
            && crate::leader58_defs::branch(actor.def).is_some()).map(|(i, actor)| (game.h(i), actor.def)).collect()
    }

    #[test]
    fn crown_leader_awards_create_once_and_preserve_actor_progression_in_archive() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(!game.configure_crown_leaders());
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(!game.configure_crown_leaders());
        let stock = game.players[0].pop;
        let mut source = crate::native_rng9::NativeRng::from_state(game.source_birth.rng.state());
        game.researched[0][0] |= 0x0fff;
        game.tick = 20;
        game.tick_crowns();
        let before = leaders(&game, 0);
        assert_eq!(before.iter().map(|(_, id)| *id).collect::<Vec<_>>(), [87, 88, 89]);
        for (handle, definition) in &before {
            source.next_u32();
            assert_eq!(game.ents[game.get(*handle).unwrap()].hp, 600.0);
            assert_eq!(game.max_hp(0, *definition), 600.0);
            assert!(!game.unit_production_available(0, TC, *definition));
            let profile = crate::ee2_combat::profile(*definition, 0);
            let center = crate::ee2_combat::profile(TC, 0);
            assert_eq!(crate::ee2_combat::base_percent(profile, center), Some(100));
            assert_eq!(crate::ee2_combat::base_percent(center, profile), Some(0));
        }
        assert_eq!(game.players[0].pop, stock);
        assert_eq!(game.source_birth.rng.state(), source.state());
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert!(restored.crown_leaders);
        assert_eq!(leaders(&restored, 0), before);
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        restored.tick = 6040;
        restored.tick_crowns();
        assert_eq!(leaders(&restored, 0), before);
        assert_eq!(restored.source_birth.rng.state(), source.state());
        restored.players[0].epoch = 10;
        restored.rebuild_modifiers(0, true);
        for (_, definition) in before {
            assert_eq!(restored.max_hp(0, definition), 800.0);
            assert_eq!(crate::leader58_defs::hit_delay(definition, 10), Some(0.6));
        }
    }

    #[test]
    fn ai_crown_draw_precedes_leader_city_draw() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        game.researched[1][0] |= 0b1111;
        let mut source = crate::native_rng9::NativeRng::from_state(game.source_birth.rng.state());
        let expected = [1, 2, 3, 4, 5, 6][source.next_word15() as usize % 6];
        source.next_u32();
        game.tick = 20;
        game.tick_crowns();
        assert_eq!(game.crown_holder(0).unwrap().owner, 1);
        game.local = 1;
        assert_eq!(game.crown_power(0), Some(expected));
        assert_eq!(leaders(&game, 1).len(), 1);
        assert_eq!(game.source_birth.rng.state(), source.state());
    }

    #[test]
    fn city_draw_iterates_new_registrations_before_older_actor_slots() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        let recent = game.spawn(TC, 0, 145.5, 145.5);
        let mut draw = crate::native_rng9::NativeRng::from_state(0);
        game.source_birth.rng = draw;
        assert_eq!(draw.next_word15() % 2, 0);
        game.award_crown_leader(0, 0);
        let id = game.get(leaders(&game, 0)[0].0).unwrap();
        assert!((game.ents[id].x - game.ents[recent].x).abs() < 10.0);
        assert!((game.ents[id].y - game.ents[recent].y).abs() < 10.0);
        assert_eq!(game.source_birth.rng.state(), draw.state());
    }
}
