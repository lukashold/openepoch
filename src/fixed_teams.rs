//! Initial alliances and fixed-team diplomacy in ordinary skirmishes.

use crate::diplomacy::{ALLIED, WAR};
use crate::game::Game;

const UNUSED_SEAT: u8 = 15;
const MAX_GROUP: u8 = 10;
const SAVE_TAG: u32 = 1 << 20;
const CHECKPOINT_TAG: u32 = 1 << 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct FixedTeams([u8; 4]);

impl FixedTeams {
    fn group(self, seat: usize) -> u8 { self.0[seat] }

    fn assignments(packed: u32, players: usize, lock: bool) -> Option<Self> {
        if packed > 0xffff { return None; }
        // Values 1–11 encode original setup group IDs 0–10. Zero is unassigned.
        let choices: [u8; 4] = core::array::from_fn(|seat| ((packed >> (seat * 4)) & 15) as u8);
        if choices[..players].iter().any(|&choice| choice > MAX_GROUP + 1)
            || choices[players..].iter().any(|&choice| choice != 0)
        {
            return None;
        }
        let mut groups = choices.map(|choice| choice.checked_sub(1).unwrap_or(UNUSED_SEAT));
        if lock {
            // 00546860 assigns each remaining seat to the first empty group.
            for seat in 0..players {
                if groups[seat] == UNUSED_SEAT {
                    groups[seat] = (0..=MAX_GROUP).find(|group| !groups.contains(group))?;
                }
            }
        }
        Some(Self(groups))
    }

    pub(crate) fn visibility_word(self, visibility: u8) -> u32 {
        self.0.iter().enumerate().fold(SAVE_TAG | u32::from(visibility), |word, (seat, &group)| {
            word | (u32::from(group) << (4 + seat * 4))
        })
    }

    pub(crate) fn read_visibility_word(word: u32, players: usize) -> Option<(u8, Option<Self>)> {
        if word <= 2 { return Some((word as u8, None)); }
        let visibility = (word & 15) as u8;
        if visibility > 2 || word >> 20 != 1 { return None; }
        let groups = core::array::from_fn(|seat| ((word >> (4 + seat * 4)) & 15) as u8);
        (groups[..players].iter().all(|&group| group <= MAX_GROUP)
            && groups[players..].iter().all(|&group| group == UNUSED_SEAT))
            .then_some((visibility, Some(Self(groups))))
    }

    pub(crate) fn valid_relations(self, players: usize, relations: &[u8; 16], offers: &[u8; 16]) -> bool {
        (0..players).all(|from| (0..players).all(|to| {
            let expected = if self.group(from) == self.group(to) { ALLIED } else { WAR };
            relations[from * 4 + to] == expected && offers[from * 4 + to] == 0
        }))
    }
}

impl Game {
    pub(crate) fn supports_starting_teams(&self) -> bool {
        (if self.net { self.humans >= 2 && self.grouped_coop.is_none() } else { self.humans == 1 })
            && self.bench == 0 && self.sole_rules().is_none()
            && self.territory_victory.is_none() && self.regicide.is_none() && self.crown_victory.is_none()
            && self.koth.is_none() && self.capitols.is_none() && self.hotspots.is_none()
    }

    pub(crate) fn configure_skirmish_teams(&mut self, packed: u32, lock: bool) -> bool {
        !self.net && self.configure_starting_teams(packed, lock)
    }

    pub(crate) fn configure_network_starting_teams(&mut self, packed: u32, lock: bool) -> bool {
        self.net && self.configure_starting_teams(packed, lock)
    }

    pub(crate) fn matches_checkpoint_teams(&self, expected: u32) -> bool {
        match self.diplomacy.fixed_teams {
            None => expected == 0,
            Some(actual) => expected >> 16 == 1
                && FixedTeams::assignments(expected & (CHECKPOINT_TAG - 1), self.nplayers, true) == Some(actual),
        }
    }

    fn configure_starting_teams(&mut self, packed: u32, lock: bool) -> bool {
        if self.tick != 0 || !self.supports_starting_teams()
            || self.diplomacy.fixed_teams.is_some()
            || self.diplomacy.offers.iter().any(|&offer| offer != 0)
            || (0..self.nplayers).any(|from| (from + 1..self.nplayers).any(|to| self.allied(from as u8, to as u8)))
        {
            return false;
        }

        let Some(groups) = FixedTeams::assignments(packed, self.nplayers, lock) else { return false; };
        self.diplomacy.fixed_teams = lock.then_some(groups);

        // 00546690 establishes each initial alliance before enabling its lock.
        // 005499FA writes Allied in both directions. Zero terms use full sight.
        for from in 0..self.nplayers {
            for to in from + 1..self.nplayers {
                let allied = groups.group(from) != UNUSED_SEAT && groups.group(from) == groups.group(to);
                for index in [from * 4 + to, to * 4 + from] {
                    if allied {
                        self.diplomacy.relation[index] = ALLIED;
                        self.diplomacy.sharing[index] = 1;
                    } else if lock {
                        self.diplomacy.relation[index] = WAR;
                    }
                }
            }
        }
        self.refresh_display_fog();
        true
    }

    pub(crate) fn fixed_team(&self, seat: usize) -> Option<u8> {
        self.diplomacy.fixed_teams.map(|teams| teams.group(seat))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::NONE;

    fn match_with_teams(locked: bool) -> Game {
        let mut game = Game::new(7, 2, 0, 1, 0);
        // Seats 0 and 1 share original group 3. Seat 2 has no assignment.
        assert!(game.configure_skirmish_teams(0x44, locked));
        game
    }

    #[test]
    fn starts_mutual_alliance_full_sight_and_applies_lock_to_normal_commands() {
        let mut game = match_with_teams(true);
        assert_eq!(game.fixed_team(0), Some(3));
        assert_eq!(game.fixed_team(1), Some(3));
        assert_eq!(game.fixed_team(2), Some(0));
        assert!(game.allied(0, 1) && game.shares_to(0, 1) && game.shares_to(1, 0));
        assert!(game.hostile(0, 2) && game.hostile(2, 0));
        assert!(!game.apply_diplomacy(0, 1, 2, 0, 0.0));
        assert!(!game.apply_diplomacy(0, 2, 0, 0, 0.0));
        assert!(!game.apply_diplomacy(0, 1, 7, 0, 0.0));
        let wood = game.resource(1, 1);
        assert!(game.apply_diplomacy(0, 1, 5, 1, 1.0));
        assert_eq!(game.resource(1, 1), wood + 1.0);
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(), loaded.state_hash());
        assert_eq!(loaded.fixed_team(2), Some(0));
        assert!(!loaded.apply_diplomacy(0, 1, 2, 0, 0.0));
        loaded.save();
        assert_eq!(game.savebuf, loaded.savebuf);
    }

    #[test]
    fn unlocked_starting_alliance_can_be_changed() {
        let mut game = match_with_teams(false);
        assert_eq!(game.fixed_team(0), None);
        assert!(game.allied(0, 1));
        assert!(game.apply_diplomacy(0, 1, 2, 0, 0.0));
        assert!(game.hostile(0, 1));
    }

    #[test]
    fn giving_away_the_last_territory_preserves_fixed_relations_and_archive() {
        let mut game = match_with_teams(true);
        let region = game.towner.iter().position(|&owner| owner == 0).unwrap() as u32;
        assert!(game.apply_territory(0, 1, 0, region, NONE));
        assert_eq!(game.players[0].terr, 0);
        assert!(game.allied(0, 1));
        game.save();
        let loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(), loaded.state_hash());
    }

    #[test]
    fn invalid_rosters_and_archive_lock_words_are_rejected() {
        let mut game = Game::new(7, 2, 0, 1, 0);
        let old_hash = game.state_hash();
        assert!(!game.configure_skirmish_teams(0xc, true));
        assert!(!game.configure_skirmish_teams(0x1000, true));
        assert_eq!(game.state_hash(), old_hash);
        assert!(FixedTeams::read_visibility_word(SAVE_TAG | 1, 3).is_none());
        assert!(FixedTeams::read_visibility_word(SAVE_TAG | 0x000f_0003, 3).is_none());
        assert!(FixedTeams::read_visibility_word(SAVE_TAG | 0x000f_0002, 3).is_some());
    }

    #[test]
    fn distinct_network_seats_keep_fixed_teams_through_authenticated_resume() {
        let mut peers = core::array::from_fn::<_, 3, _>(|seat| Game::new_network_roster(7, 1, 3, seat as u32).unwrap());
        for game in &mut peers {
            assert!(game.configure_match_epoch_range(0xe281, 500, 2, 0, 14));
            assert!(game.configure_network_resignation(1));
            assert!(!game.configure_skirmish_teams(0x44, true));
            assert!(game.configure_network_starting_teams(0x44, true));
            assert!(game.allied(0, 1) && game.hostile(0, 2) && game.hostile(0, 3));
            assert!(!game.apply_diplomacy(1, 0, 2, 0, 0.0));
            assert!(game.apply_diplomacy(1, 0, 5, 1, 10.0));
            assert_eq!(game.tick_with_network_source_slot(30, 30, 1.5, true), 1);
            game.save_network_resume();
            assert!(!game.savebuf.is_empty());
        }
        let archive = &peers[0].savebuf;
        for (seat, game) in peers.iter().enumerate() {
            assert!(archive == &game.savebuf, "canonical archive differs by seat");
            let loaded = Game::load_network_resume(archive, 3, seat as u32, 1, 30, 1.5).unwrap();
            assert_eq!(loaded.state_hash(), game.state_hash());
            assert!(loaded.matches_checkpoint_teams(CHECKPOINT_TAG | 0x44));
            assert!(!loaded.matches_checkpoint_teams(CHECKPOINT_TAG | 0x42));
            assert!(!loaded.matches_checkpoint_teams(0));
        }
        assert!(Game::load(archive).is_none());
    }
}
