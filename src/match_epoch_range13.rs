//! The fixed inclusive skirmish range. The existing match transaction still
//! initializes every active seat with the same starting epoch.

use crate::{defs::MAX_EPOCH, game::Game, pack8_save};

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"EPR1");

impl Game {
    pub(crate) fn configure_match_epoch_range(&mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32) -> bool {
        if start > end || end > MAX_EPOCH as u32 || !self.configure_match_settings(civs, pop, supplies, start) {
            return false;
        }
        self.final_epoch = end as u8;
        true
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn force_epoch_advance(&mut self) -> bool {
        use crate::{defs::original_research, game::RESEARCH};

        let owner = self.local;
        let seat = owner as usize;
        let previous = self.players[seat].epoch;
        if previous >= self.match_final_epoch() {
            return false;
        }

        let points_before_refunds = self.research_points[seat];
        for host in 0..self.high {
            if !self.ents[host].alive || self.ents[host].team != owner {
                continue;
            }
            let mut position = 0;
            while position < self.ents[host].qn as usize {
                let job = self.ents[host].q[position];
                if job == RESEARCH || original_research(job) {
                    self.cancel_paid_job(host, position);
                } else {
                    position += 1;
                }
            }
        }
        self.refund_source_research_for_epoch(owner);
        if self.research_points[seat] > points_before_refunds {
            self.research_points[seat] = self.research_points[seat].min(999_999.0);
        }

        self.reset_source_rank_for_epoch(owner);
        self.players[seat].epoch = previous + 1;
        self.players[seat].researching = false;
        self.rebuild_modifiers(owner, true);
        self.apply_paid_main(owner, previous);
        true
    }
}

fn word(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, final_epoch: u8) -> Vec<u8> {
    if payload.is_empty() || final_epoch == MAX_EPOCH {
        return payload;
    }
    let mut out = Vec::with_capacity(payload.len() + 16);
    for value in [MAGIC, final_epoch.into(), payload.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], u8)> {
    if word(buf, 0) != Some(MAGIC) {
        return Some((buf, MAX_EPOCH));
    }
    let cap = word(buf, 4)?;
    let end = 12usize.checked_add(word(buf, 8)? as usize)?;
    if cap >= MAX_EPOCH as u32 || end.checked_add(4)? != buf.len() {
        return None;
    }
    let inner_magic = word(buf, 12)?;
    let inner_valid = inner_magic == pack8_save::MAGIC;
    #[cfg(feature="ee2-ai9-game-bridge")]
    let inner_valid = inner_valid || inner_magic == crate::ai9_game_save::MAGIC;
    if !inner_valid { return None; }
    let framed = buf.get(..end)?;
    if pack8_save::checksum(framed) != word(buf, end)? {
        return None;
    }
    Some((framed.get(12..)?, cap as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::TC, game::RESEARCH};

    fn city(game: &Game) -> usize {
        game.ents[..game.high].iter().position(|unit| unit.alive && unit.def == TC && unit.team == 0).unwrap()
    }

    fn fund(game: &mut Game, epoch: u8) {
        for index in (12 * epoch as usize)..(12 * epoch as usize + 6) {
            game.researched[0][index / 32] |= 1 << (index % 32);
        }
        game.research_points[0] = 300.0;
        game.rebuild_modifiers(0, true);
    }

    #[test]
    fn actual_paid_advance_survives_load_then_stops_at_the_selected_epoch() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 1, 0, 1));
        fund(&mut game, 0);
        let tc = city(&game);
        assert!(game.train(tc, RESEARCH));
        assert_eq!(game.research_points[0], 268.0);
        for _ in 0..30 { game.tick(); }
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.match_final_epoch(), 1);
        assert_eq!(loaded.state_hash(), game.state_hash());
        let mut finished = false;
        for _ in 0..1800 {
            game.tick();
            loaded.tick();
            assert_eq!(loaded.state_hash(), game.state_hash());
            if loaded.players[0].epoch == 1 {
                finished = true;
                break;
            }
        }
        assert!(finished, "The real paid queue must complete the allowed transition");
        fund(&mut loaded, 1);
        let tc = city(&loaded);
        let before = loaded.state_hash();
        assert!(!loaded.train(tc, RESEARCH));
        assert_eq!(loaded.research_points[0], 300.0);
        assert_eq!(loaded.state_hash(), before);
        loaded.save();
        let mut restored = Game::load(&loaded.savebuf).unwrap();
        assert_eq!(restored.state_hash(), before);
        restored.save();
        assert_eq!(restored.savebuf, loaded.savebuf);
    }

    #[test]
    fn native_instant_build_completes_a_paid_epoch_at_its_first_tick() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_epoch_range(0xee10, 500, 1, 0, 1));
        fund(&mut game, 0);
        assert!(game.train(city(&game), RESEARCH));
        game.instant_build = true;
        game.tick();
        assert_eq!(game.players[0].epoch, 1);
        assert_eq!(game.players[1].epoch, 0);
        assert_eq!(game.research_points[0], 268.0);
        assert!(!game.players[0].researching);
    }

    #[test]
    fn inclusive_single_epoch_and_atomic_rejection() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let before = game.state_hash();
        for (start, end) in [(1, 0), (0, 15), (u32::MAX, u32::MAX)] {
            assert!(!game.configure_match_epoch_range(0xee10, 500, 1, start, end));
            assert_eq!(game.state_hash(), before);
        }
        assert!(game.configure_match_epoch_range(0xee10, 500, 1, 0, 0));
        assert!(!game.configure_match_epoch_range(0xee10, 500, 1, 0, 14));
        fund(&mut game, 0);
        assert!(!game.train(city(&game), RESEARCH));
        game.save();
        let (payload, cap) = split(&game.savebuf).unwrap();
        assert_eq!(cap, 0);
        assert_eq!(Game::load(payload).unwrap().match_final_epoch(), 14);
        let mut corrupt = game.savebuf.clone();
        corrupt[4] = 1;
        assert!(Game::load(&corrupt).is_none());
    }
}
