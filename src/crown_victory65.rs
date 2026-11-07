//! Crown victory uses possession history. The installed rules sample the
//! previous committed step before applying the current Crown award and credit.

use crate::{ai9_count::Precision53, game::{Game, Player}};

const TAG: u32 = u32::from_le_bytes(*b"CVS1");
const SEATS: usize = 4;
const KINDS: usize = 3;
const HEADER: usize = 4 * (5 + SEATS * KINDS);

#[derive(Clone, Copy)]
enum Objective {
    Count(u8),
    Time(u8),
}

impl Objective {
    fn parse(mode: u32, target: u32) -> Option<Self> {
        match (mode, target) {
            (1, 2 | 3) => Some(Self::Count(target as u8)),
            (2, 10 | 15 | 20 | 25 | 30) => Some(Self::Time(target as u8)),
            _ => None,
        }
    }

    fn mode(self) -> u32 {
        match self { Self::Count(_) => 1, Self::Time(_) => 2 }
    }

    fn target(self) -> u8 {
        match self { Self::Count(target) | Self::Time(target) => target }
    }

    fn required(self) -> i32 {
        match self {
            Self::Count(count) => i32::from(count),
            Self::Time(minutes) => i32::from(minutes) * 60,
        }
    }

    fn score(self, totals: [f32; KINDS]) -> i32 {
        match self {
            Self::Count(_) => mask(totals).count_ones() as i32,
            Self::Time(_) => totals.into_iter().fold(0.0, |sum, time| {
                Precision53::TowardZero.add(sum, f64::from(time))
            }) as i32,
        }
    }
}

fn mask(totals: [f32; KINDS]) -> u32 {
    totals.into_iter().enumerate().fold(0, |bits, (kind, time)| {
        bits | (u32::from(time > 0.0) << kind)
    })
}

#[derive(Clone, Copy)]
pub(crate) struct Rule {
    objective: Objective,
    totals: [[f32; KINDS]; SEATS],
}

impl Rule {
    fn new(objective: Objective) -> Self { Self { objective, totals: [[0.0; KINDS]; SEATS] } }

    fn outcome(self, players: &[Player], local: usize) -> u8 {
        // This solo adapter assigns each eligible seat its own fixed team.
        let live = players.iter().enumerate().filter(|(_, player)| player.alive);
        let leaders = live.clone().map(|(seat, _)| (seat, self.objective.score(self.totals[seat])));
        if let Some(highest) = leaders.clone().map(|(_, score)| score).max() {
            if highest >= self.objective.required() {
                let mut tied = leaders.filter(|&(_, score)| score == highest);
                if let (Some((seat, _)), None) = (tied.next(), tied.next()) {
                    return if seat == local { 1 } else { 2 };
                }
            }
        }
        if !players[local].alive { 2 }
        else if live.count() == 1 { 1 }
        else { 0 }
    }

    pub(crate) fn hash(self, outcome: u8, mut mix: impl FnMut(u32)) {
        for word in [TAG, 1, self.objective.mode(), u32::from(self.objective.target()), u32::from(outcome)] { mix(word); }
        for value in self.totals.into_iter().flatten() { mix(value.to_bits()); }
    }

    pub(crate) fn wrap(self, core: Vec<u8>, outcome: u8) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER + core.len() + 4);
        self.hash(outcome, |word| bytes.extend_from_slice(&word.to_le_bytes()));
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<(Rule, u8)>)> {
    if !bytes.starts_with(&TAG.to_le_bytes()) { return Some((bytes, None)); }
    let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let end = bytes.len().checked_sub(4)?;
    if end <= HEADER || word(4)? != 1 || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
    let objective = Objective::parse(word(8)?, word(12)?)?;
    let outcome = word(16)?;
    if outcome > 2 { return None; }
    let mut rule = Rule::new(objective);
    for (index, total) in rule.totals.iter_mut().flatten().enumerate() {
        *total = f32::from_bits(word(20 + 4 * index)?);
        if !total.is_finite() || total.is_sign_negative() { return None; }
    }
    Some((&bytes[HEADER..end], Some((rule, outcome as u8))))
}

impl Game {
    pub(crate) fn configure_match_crown_victory(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32, mode: u32, target: u32,
    ) -> bool {
        let Some(objective) = Objective::parse(mode, target) else { return false; };
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return false; }
        if self.net || self.humans != 1 || self.local != 0
            || !self.configure_match_epoch_range(civs, pop, supplies, start, end) { return false; }
        self.crown_victory = Some(Rule::new(objective));
        true
    }

    pub(crate) fn credit_crown_victory(&mut self, step: f32) {
        let Some(mut rule) = self.crown_victory else { return; };
        if self.winner != 0 { return; }
        let rounding = Precision53::TowardZero;
        for kind in 0..KINDS {
            if let Some(holder) = self.crown_holder(kind) {
                let total = &mut rule.totals[holder.owner as usize][kind];
                *total = rounding.store_f32(rounding.add(f64::from(*total), f64::from(step)));
            }
        }
        self.crown_victory = Some(rule);
    }

    pub(crate) fn poll_crown_victory(&mut self, rule: Rule) {
        self.winner = rule.outcome(&self.players[..self.nplayers], self.local as usize);
    }

    pub(crate) fn restore_crown_victory(&mut self, rule: Rule, outcome: u8) -> Option<()> {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return None; }
        if self.net || self.humans != 1 || self.local != 0 || self.bench != 0
            || rule.totals[self.nplayers..].iter().flatten().any(|&value| value != 0.0)
            || (self.crown_rules() != 1 && rule.totals.iter().flatten().any(|&value| value != 0.0))
            || (outcome != 0 && rule.outcome(&self.players[..self.nplayers], self.local as usize) != outcome) { return None; }
        self.crown_victory = Some(rule);
        self.winner = outcome;
        Some(())
    }

    pub(crate) fn crown_victory_status(&self, field: u32, seat: usize) -> i32 {
        let Some(rule) = self.crown_victory else { return if field == 0 { 0 } else { -1 }; };
        match field {
            0 => rule.objective.mode() as i32,
            1 => rule.objective.required(),
            2 if seat < self.nplayers => rule.objective.score(rule.totals[seat]),
            3 if seat < self.nplayers => mask(rule.totals[seat]) as i32,
            _ => -1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skirmish(mode: u32, target: u32, disabled: bool) -> Game {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let before = game.state_hash();
        assert!(!game.configure_match_crown_victory(0xee10, 250, 1, 0, 14, mode, 1));
        assert_eq!(game.state_hash(), before);
        assert!(game.configure_match_crown_victory(0xee10, 250, 1, 0, 14, mode, target));
        assert!(game.configure_crowns(disabled));
        game.players[1].ai = false;
        game
    }

    #[test]
    fn actual_crown_award_step_credits_history_and_the_next_poll_wins() {
        let mut game = skirmish(1, 2, false);
        let mut researched = [0; 2];
        for (index, node) in crate::ee2_research::NODES.iter().enumerate() {
            let Some(count) = researched.get_mut(node.branch as usize) else { continue; };
            if node.epoch != 0 || *count >= 4 { continue; }
            game.researched[0][index / 32] |= 1 << (index % 32);
            *count += 1;
        }
        assert_eq!(researched, [4, 4]);
        for _ in 0..22 {
            game.tick();
            if game.crown_victory_status(2, 0) == 2 { break; }
        }
        assert_eq!(game.crown_victory_status(2, 0), 2);
        assert_eq!(game.crown_victory_status(3, 0), 0b011);
        assert_eq!(game.winner, 0);
        game.save();
        let mut resumed = Game::load(&game.savebuf).expect("crown award archive before poll");
        assert_eq!(resumed.state_hash(), game.state_hash());
        resumed.tick();
        game.tick();
        assert_eq!(game.winner, 1);
        assert_eq!(resumed.state_hash(), game.state_hash());
        game.save();
        let finished = Game::load(&game.savebuf).expect("finished crown victory archive");
        assert_eq!(finished.winner, 1);
        assert_eq!(finished.state_hash(), game.state_hash());
    }

    #[test]
    fn an_active_crown_credits_f32_history_with_the_tick_chop_mode() {
        let mut game = skirmish(2, 10, false);
        for (index, node) in crate::ee2_research::NODES.iter().enumerate() {
            if node.epoch == 0 && node.branch == 0 {
                game.researched[0][index / 32] |= 1 << (index % 32);
            }
        }
        for _ in 0..22 {
            game.tick();
            if game.crown_holder(0).is_some() { break; }
        }
        assert_eq!(game.crown_holder(0).unwrap().owner, 0);
        game.crown_victory.as_mut().unwrap().totals[0][0] = 0.1;
        game.credit_crown_victory(0.05);
        assert_eq!(game.crown_victory.unwrap().totals[0][0].to_bits(), 0.15f32.to_bits() - 1);
    }

    #[test]
    fn native_time_tally_chops_intermediate_addition_before_truncating_seconds() {
        let native = [0x3f7fffff, 0x337fffff, 0x277fffff].map(f32::from_bits);
        assert_eq!(native.into_iter().map(f64::from).sum::<f64>() as i32, 1);
        assert_eq!(Objective::Time(10).score(native), 0);
    }

    #[test]
    fn timed_score_uses_whole_seconds_and_ties_keep_running() {
        let mut game = skirmish(2, 10, false);
        game.tick = 14_000;
        let mut rule = game.crown_victory.unwrap();
        rule.totals[0] = [299.75, 300.25, 0.0];
        rule.totals[1] = [600.875, 0.0, 0.0];
        game.crown_victory = Some(rule);
        assert_eq!(game.crown_victory_status(1, 0), 600);
        assert_eq!(game.crown_victory_status(2, 1), 600);
        game.tick();
        assert_eq!(game.winner, 0);
        game.crown_victory.as_mut().unwrap().totals[1][0] = 601.0;
        game.tick();
        assert_eq!(game.winner, 2);
    }

    #[test]
    fn disabled_crowns_are_valid_and_allow_fixed_team_fallback() {
        let mut game = skirmish(1, 3, true);
        for actor in &mut game.ents[..game.high] {
            if actor.team == 1 { actor.hp = 0.0; }
        }
        game.tick();
        assert_eq!(game.crown_rules(), 2);
        assert_eq!(game.crown_victory_status(2, 0), 0);
        assert_eq!(game.winner, 1);
        game.save();
        let restored = Game::load(&game.savebuf).expect("disabled Crown victory archive");
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}
