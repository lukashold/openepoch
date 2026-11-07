//! The UP single-player Conquest modifier and its saved disband clock.

use crate::{ai9_count::Precision53, game::Game, pack8_save};

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"VCS1");
const HEADER: usize = 20;
pub(crate) const IDLE: f32 = -1.0;
const DISBAND_SECONDS: f32 = 10.0;

#[derive(Clone, Copy)]
pub(crate) struct SoleSurvivor {
    pub(crate) eliminated: u8,
    pub(crate) deadline: f32,
}

impl SoleSurvivor {
    pub(crate) fn eliminated(self, seat: usize) -> bool {
        self.eliminated & (1 << seat) != 0
    }

    pub(crate) fn active(self, seats: usize) -> u8 {
        ((1u8 << seats) - 1) & !self.eliminated
    }

    pub(crate) fn remaining_seconds(self, now: f32) -> Option<i32> {
        (self.deadline != IDLE).then(|| {
            let seconds = (f64::from(self.deadline) - f64::from(now)).max(0.0);
            (seconds + 0.5).floor() as i32
        })
    }
}

impl Game {
    pub(crate) fn configure_match_sole_survivor(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32,
    ) -> bool {
        if self.net || self.humans != 1 || self.local != 0
            || !self.configure_match_epoch_range(civs, pop, supplies, start, end)
        {
            return false;
        }
        self.sole_survivor = Some(SoleSurvivor { eliminated: 0, deadline: IDLE });
        true
    }

    pub(crate) fn sole_alliances_allowed(&self) -> bool {
        self.sole_rules().is_none_or(|rule| rule.active(self.nplayers).count_ones() != 2)
    }

    pub(crate) fn sole_rules(&self) -> Option<SoleSurvivor> {
        self.sole_survivor.or_else(|| self.regicide.filter(|rule| rule.sole).map(|rule| rule.state))
            .or_else(|| self.capitols.and_then(crate::capitols66::Capitols::sole_state))
    }

    pub(crate) fn poll_sole_disband(&mut self, rule: &mut SoleSurvivor, candidates: u8, rounding: Precision53) {
        let now = self.sim_elapsed();
        if rule.deadline == IDLE {
            rule.deadline = rounding.store_f32(rounding.add(f64::from(now), f64::from(DISBAND_SECONDS)));
        } else if now > rule.deadline {
            self.force_sole_survivor_war(candidates);
            rule.deadline = IDLE;
        }
    }

    pub(crate) fn poll_sole_outcome(&mut self, mut rule: SoleSurvivor) {
        let candidates = rule.active(self.nplayers);
        for (seat, player) in self.players[..self.nplayers].iter().enumerate() {
            if !player.alive {
                rule.eliminated |= 1 << seat;
            }
        }
        let survivors = rule.active(self.nplayers);
        let local = 1 << self.local;
        if survivors.count_ones() > 1 && crate::ee2_conquest::survivors_are_allied(self) {
            self.poll_sole_disband(&mut rule, candidates, Precision53::NearestEven);
        } else if survivors.count_ones() == 2 && survivors != candidates {
            // 005B3110 clears offers at the transition into the final pair.
            self.clear_sole_diplomacy_offers();
        }
        if survivors & local == 0 {
            self.winner = 2;
        } else if survivors == local {
            self.winner = 1;
        }
        self.sole_survivor = Some(rule);
    }

    pub(crate) fn restore_sole_survivor(&mut self, rule: SoleSurvivor, outcome: u8) -> Option<()> {
        let seats = (1u8 << self.nplayers) - 1;
        let now = self.sim_elapsed();
        if self.net || self.humans != 1 || self.territory_victory.is_some()
            || rule.eliminated & !seats != 0
            || (rule.deadline != IDLE && !(DISBAND_SECONDS..=now + DISBAND_SECONDS).contains(&rule.deadline))
            || (self.tick == 0 && (outcome != 0 || rule.eliminated != 0 || rule.deadline != IDLE))
            || rule.eliminated(0) != (outcome == 2)
            || (outcome == 1 && rule.active(self.nplayers) != 1)
            || (outcome == 0 && rule.active(self.nplayers).count_ones() < 2)
            || self.players[..self.nplayers].iter().enumerate()
                .any(|(seat, player)| rule.eliminated(seat) && player.alive)
        {
            return None;
        }
        self.sole_survivor = Some(rule);
        self.winner = outcome;
        Some(())
    }
}

fn word(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, game: &Game) -> Vec<u8> {
    let Some(rule) = game.sole_survivor else {
        return payload;
    };
    if payload.is_empty() {
        return payload;
    }
    let mut out = Vec::with_capacity(HEADER + payload.len() + 4);
    for value in [MAGIC, rule.deadline.to_bits(), rule.eliminated.into(), game.rule_save_outcome().into(), payload.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<(SoleSurvivor, u8)>)> {
    if word(buf, 0) != Some(MAGIC) {
        return Some((buf, None));
    }
    let deadline = f32::from_bits(word(buf, 4)?);
    let eliminated = u8::try_from(word(buf, 8)?).ok()?;
    let outcome = u8::try_from(word(buf, 12)?).ok().filter(|&value| value <= 2)?;
    let end = HEADER.checked_add(word(buf, 16)? as usize)?;
    if end.checked_add(4)? != buf.len() || pack8_save::checksum(buf.get(..end)?) != word(buf, end)? {
        return None;
    }
    match word(buf, HEADER)? {
        pack8_save::MAGIC | crate::match_epoch_range13::MAGIC | crate::ai9_game_save::MAGIC => {}
        _ => return None,
    }
    Some((buf.get(HEADER..end)?, Some((SoleSurvivor { deadline, eliminated }, outcome))))
}

#[cfg(test)]
mod tests {
    use crate::game::DT;
    use super::*;
    use crate::defs::WORKER;

    fn match_for(opponents: u32) -> Game {
        let mut game = Game::new(7, opponents, 0, 1, 0);
        let civs = if opponents == 1 { 0xee10 } else { 0xe210 };
        assert!(game.configure_match_sole_survivor(civs, 1000, 2, 0, 4));
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        game
    }

    fn peace(game: &mut Game, a: u8, b: u8) {
        assert!(game.apply_diplomacy(a, b.into(), 0, 0, 0.0));
        assert!(game.apply_diplomacy(b, a.into(), 3, 0, 0.0));
        for _ in 0..100 { game.tick(); }
    }

    fn ally(game: &mut Game, a: u8, b: u8) {
        peace(game, a, b);
        assert!(game.apply_diplomacy(a, b.into(), 1, 1, 0.0));
        assert!(game.apply_diplomacy(b, a.into(), 3, 0, 0.0));
        assert!(game.shares_to(a, b) && game.shares_to(b, a));
    }

    fn eliminate(game: &mut Game, seat: u8) {
        for actor in &mut game.ents[..game.high] {
            if actor.team == seat { actor.hp = 0.0; }
        }
        game.tick();
    }

    #[test]
    fn source_clock_disbands_only_after_ten_actual_seconds_and_restores_mid_deadline() {
        let mut game = match_for(2);
        assert!(game.tick_with_source_delta(0.125));
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        for _ in 0..40 { assert!(game.tick_with_source_delta(0.125)); }
        assert!(game.apply_diplomacy(0, 1, 1, 1, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        for actor in &mut game.ents[..game.high] {
            if actor.team == 2 { actor.hp = 0.0; }
        }
        assert!(game.tick_with_source_delta(0.125));
        let deadline = game.sole_survivor.unwrap().deadline;
        assert_eq!(deadline, game.sim_elapsed() + 10.0);
        for _ in 0..24 { assert!(game.tick_with_source_delta(0.125)); }
        assert_eq!(game.sole_survivor.unwrap().remaining_seconds(game.sim_elapsed()), Some(7));
        game.save();
        let mut copy = Game::load(&game.savebuf).expect("source disband deadline saves");
        copy.save();
        assert_eq!(copy.savebuf, game.savebuf);
        while game.sim_elapsed() < deadline {
            assert!(game.tick_with_source_delta(0.125));
            assert!(copy.tick_with_source_delta(0.125));
            assert!(game.allied(0, 1));
            assert_eq!(copy.state_hash(), game.state_hash());
        }
        assert_eq!(game.sim_elapsed(), deadline);
        assert!(game.tick_with_source_delta(0.03125));
        assert!(copy.tick_with_source_delta(0.03125));
        assert_eq!(copy.state_hash(), game.state_hash());
        assert!(game.hostile(0, 1) && game.hostile(1, 0));
        assert_eq!(game.sole_survivor.unwrap().deadline, IDLE);
    }

    #[test]
    fn allied_final_pair_waits_for_the_strict_deadline_then_breaks_both_directions() {
        let mut game = match_for(2);
        ally(&mut game, 0, 1);
        assert!(game.sole_alliances_allowed());
        assert_eq!(game.sole_survivor.unwrap().deadline, IDLE);
        while game.tick % 20 != 19 { game.tick(); }
        eliminate(&mut game, 2);
        let armed = game.sole_survivor.unwrap();
        assert!(armed.eliminated(2));
        assert!(!game.sole_alliances_allowed());
        assert_eq!(armed.deadline, game.tick as f32 * DT + 10.0);
        assert_eq!(armed.remaining_seconds(game.sim_elapsed()), Some(10));
        assert_eq!(game.winner, 0);
        for _ in 0..37 { game.tick(); }
        assert_eq!(armed.remaining_seconds(game.sim_elapsed()), Some(8));
        game.save();
        let mut resumed = Game::load(&game.savebuf).unwrap();
        resumed.save();
        assert_eq!(game.savebuf, resumed.savebuf);
        while (game.tick + 1) as f32 * DT <= armed.deadline {
            game.tick();
            resumed.tick();
            assert!(game.allied(0, 1));
            assert_eq!(game.state_hash(), resumed.state_hash());
        }
        assert_eq!(game.tick as f32 * DT, armed.deadline);
        game.tick();
        resumed.tick();
        assert_eq!(game.state_hash(), resumed.state_hash());
        assert_eq!(game.sole_survivor.unwrap().deadline, IDLE);
        assert!(game.hostile(0, 1) && game.hostile(1, 0));
        assert!(!game.shares_to(0, 1) && !game.shares_to(1, 0));
        assert_eq!(game.winner, 0);
        let rogue = game.spawn(WORKER, 2, 100.5, 100.5);
        game.tick();
        assert!(game.ents[rogue].alive && !game.players[2].alive);
        eliminate(&mut game, 1);
        assert_eq!(game.winner, 1);
        game.save();
        assert_eq!(Game::load(&game.savebuf).unwrap().winner, 1);
    }

    #[test]
    fn transition_to_two_unallied_players_cancels_a_pending_alliance() {
        let mut game = match_for(2);
        peace(&mut game, 0, 1);
        assert!(game.apply_diplomacy(0, 1, 1, 0, 0.0));
        assert_ne!(game.diplomacy.offers[1], 0);
        eliminate(&mut game, 2);
        assert_eq!(game.diplomacy.offers[1], 0);
        assert_eq!(game.sole_survivor.unwrap().deadline, IDLE);
        assert!(!game.diplomacy_valid(1, 0, 3, 0, 0.0));
        for _ in 0..100 { game.tick(); }
        assert!(!game.diplomacy_valid(0, 1, 1, 0, 0.0));
        assert_eq!(game.winner, 0);
    }

    #[test]
    fn initial_duel_allows_peace_but_declines_a_new_alliance() {
        let mut game = match_for(1);
        assert!(!game.sole_alliances_allowed());
        peace(&mut game, 0, 1);
        assert!(!game.apply_diplomacy(0, 1, 1, 0, 0.0));
        eliminate(&mut game, 0);
        assert_eq!(game.winner, 2);
        game.save();
        assert_eq!(Game::load(&game.savebuf).unwrap().winner, 2);
    }

    #[test]
    fn a_broken_coalition_leaves_the_original_deadline_latched() {
        let mut game = match_for(2);
        ally(&mut game, 0, 1);
        ally(&mut game, 0, 2);
        ally(&mut game, 1, 2);
        game.tick();
        let armed = game.sole_survivor.unwrap().deadline;
        assert_ne!(armed, IDLE);
        assert!(game.apply_diplomacy(0, 2, 2, 0, 0.0));
        while game.tick as f32 * DT <= armed { game.tick(); }
        assert_eq!(game.sole_survivor.unwrap().deadline, armed);
        assert!(game.allied(0, 1));
        assert!(game.allied(1, 2));
        assert_eq!(game.winner, 0);
    }
}
