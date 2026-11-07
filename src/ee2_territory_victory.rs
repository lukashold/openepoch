//! UP Territory Control for single-player matches with one seat per team.

use crate::{game::Game, pack8_save};

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"VCT1");
const HEADER: usize = 20;
const PERCENTAGES: [u8; 7] = [30, 40, 50, 60, 70, 80, 90];

#[derive(Clone, Copy)]
pub(crate) struct TerritoryVictory {
    pub(crate) percent: u8,
    pub(crate) eliminated: u8,
}

impl TerritoryVictory {
    pub(crate) fn eliminated(self, seat: usize) -> bool {
        self.eliminated & (1 << seat) != 0
    }
}

pub(crate) struct TerritoryStanding {
    pub(crate) total: u32,
    pub(crate) target: u32,
    pub(crate) counts: [u32; 4],
}

impl TerritoryStanding {
    pub(crate) fn winning_seat(&self) -> Option<u8> {
        let highest = self.counts.into_iter().max()?;
        if self.total == 0 || highest < self.target {
            return None;
        }
        let mut leaders = self.counts.iter().enumerate().filter(|(_, count)| **count == highest);
        let seat = leaders.next()?.0 as u8;
        leaders.next().is_none().then_some(seat)
    }
}

fn parse_percentage(value: u32) -> Option<u8> {
    let percent = u8::try_from(value).ok()?;
    PERCENTAGES.contains(&percent).then_some(percent)
}

impl Game {
    pub(crate) fn configure_match_territory_victory(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32, percentage: u32,
    ) -> bool {
        let Some(percent) = parse_percentage(percentage) else {
            return false;
        };
        if self.net || self.humans != 1 || self.local != 0
            || !self.configure_match_epoch_range(civs, pop, supplies, start, end)
        {
            return false;
        }
        self.territory_victory = Some(TerritoryVictory { percent, eliminated: 0 });
        true
    }

    pub(crate) fn territory_standing(&self, percent: u8) -> TerritoryStanding {
        let total = self.map.tseeds.len() as u32;
        // 005B5BA0 raises the percentage target above the largest fixed team
        // size. In this skirmish setup each active seat is its own team.
        let target = (total * u32::from(percent)).div_ceil(100).max(2).min(total);
        let mut counts = [0; 4];
        for &owner in &self.towner[..total as usize] {
            if (owner as usize) < self.nplayers {
                counts[owner as usize] += 1;
            }
        }
        TerritoryStanding { total, target, counts }
    }

    pub(crate) fn poll_territory_outcome(&mut self, mut rule: TerritoryVictory) {
        let candidates = ((1u8 << self.nplayers) - 1) & !rule.eliminated;
        let local = 1u8 << self.local;
        if let Some(seat) = self.territory_standing(rule.percent).winning_seat() {
            // 005B5D30 reports the winner and every other active seat as lost.
            rule.eliminated |= candidates & !(1 << seat);
            self.winner = if seat == self.local { 1 } else { 2 };
        } else {
            // 005B2230 checks the current fixed teams first. A seat eliminated
            // during this poll leaves that list on the following poll.
            for (seat, player) in self.players[..self.nplayers].iter().enumerate() {
                if !player.alive {
                    rule.eliminated |= 1 << seat;
                }
            }
            if rule.eliminated & local != 0 {
                self.winner = 2;
            } else if candidates == local {
                self.winner = 1;
            }
        }
        for (seat, player) in self.players[..self.nplayers].iter_mut().enumerate() {
            if rule.eliminated(seat) {
                player.alive = false;
            }
        }
        self.territory_victory = Some(rule);
    }

    pub(crate) fn restore_territory_victory(&mut self, rule: TerritoryVictory, outcome: u8) -> Option<()> {
        let seats = (1u8 << self.nplayers) - 1;
        if self.net || self.humans != 1 || rule.eliminated & !seats != 0
            || ((outcome != 0 || rule.eliminated != 0) && self.tick < 20)
            || rule.eliminated(0) != (outcome == 2)
            || (outcome == 1 && seats & !rule.eliminated != 1)
            || self.players[..self.nplayers].iter().enumerate()
                .any(|(seat, player)| rule.eliminated(seat) && player.alive)
        {
            return None;
        }
        self.territory_victory = Some(rule);
        self.winner = outcome;
        Some(())
    }
}

fn word(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

pub(crate) fn wrap(payload: Vec<u8>, game: &Game) -> Vec<u8> {
    let Some(rule) = game.territory_victory else {
        return payload;
    };
    if payload.is_empty() {
        return payload;
    }
    let mut out = Vec::with_capacity(HEADER + payload.len() + 4);
    for value in [MAGIC, rule.percent.into(), rule.eliminated.into(), game.rule_save_outcome().into(), payload.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out.extend_from_slice(&pack8_save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<(TerritoryVictory, u8)>)> {
    if word(buf, 0) != Some(MAGIC) {
        return Some((buf, None));
    }
    let percent = parse_percentage(word(buf, 4)?)?;
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
    Some((buf.get(HEADER..end)?, Some((TerritoryVictory { percent, eliminated }, outcome))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::{TC, WORKER}, game::{NONE, NOIDX}, map::{tile_of, N}};

    fn match_for(percent: u32) -> Game {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_territory_victory(0xee10, 1000, 2, 0, 4, percent));
        game.players[1].ai = false;
        game
    }

    fn step(game: &mut Game, count: u32) {
        for _ in 0..count {
            game.tick();
        }
    }

    fn owned_town(game: &Game, seat: u8) -> usize {
        game.ents[..game.high].iter().position(|unit| unit.alive && unit.def == TC && unit.team == seat).unwrap()
    }

    fn territory(game: &Game, index: usize) -> u32 {
        let actor = &game.ents[index];
        u32::from(game.map.terr[tile_of(actor.x, actor.y)])
    }

    fn place_next_town(game: &mut Game, builders: &[usize]) -> usize {
        let worker = game.ents[builders[0]];
        for radius in 1..N as i32 {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs().max(dy.abs()) != radius { continue; }
                    let tx = worker.x as i32 + dx;
                    let ty = worker.y as i32 + dy;
                    if !game.can_place(TC, tx, ty, 0) { continue; }
                    let mid = Game::footprint_side(TC, 0) as f32 * 0.5;
                    let region = game.map.terr[tile_of(tx as f32 + mid, ty as f32 + mid)] as usize;
                    if !game.territory_adjacent_owned(0, region) { continue; }
                    let town = game.begin_construction(0, TC, tx, ty, 0, builders, false);
                    if town != NOIDX { return town; }
                }
            }
        }
        panic!("Expected a legal expansion site");
    }

    fn complete_claim(game: &mut Game, town: usize) -> u32 {
        let region = territory(game, town);
        for _ in 0..400 {
            step(game, 20);
            if game.towner[region as usize] == 0 {
                assert!(game.ents[town].built >= 1.0);
                return region;
            }
        }
        panic!("The ordinary paid town did not complete its claim");
    }

    #[test]
    fn paid_town_and_atomic_exchange_feed_the_same_ownership_count() {
        let mut game = match_for(50);
        let human_old = territory(&game, owned_town(&game, 0));
        let rival_old = territory(&game, owned_town(&game, 1));
        let builders: Vec<_> = game.ents[..game.high].iter().enumerate()
            .filter(|(_, actor)| actor.alive && actor.def == WORKER && actor.team == 0)
            .map(|(index, _)| index).collect();
        let wood = game.resource(0, 1);
        let bill = game.unit_cost(0, TC)[1];
        let town = place_next_town(&mut game, &builders);
        let claimed = territory(&game, town);
        assert_ne!(claimed, human_old);
        assert_ne!(claimed, rival_old);
        assert_eq!(game.resource(0, 1), wood - f32::from(bill));
        assert_eq!(game.territory_standing(50).counts, [1, 1, 0, 0]);
        assert!(game.ents[town].built < 1.0);
        assert_eq!(complete_claim(&mut game, town), claimed);
        assert_eq!(game.towner[claimed as usize], 0);
        assert!(game.ents[town].built >= 1.0);
        assert_eq!(game.territory_standing(50).counts, [2, 1, 0, 0]);
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        step(&mut game, 100);
        assert!(game.apply_diplomacy(0, 1, 1, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(game.allied(0, 1));
        step(&mut game, 20);
        assert_eq!(game.winner, 0);
        assert!(game.apply_territory(0, 1, 1, human_old, rival_old));
        assert_eq!(game.towner[human_old as usize], 0);
        assert!(game.apply_territory(1, 0, 2, NONE, NONE));
        assert_eq!([game.towner[human_old as usize], game.towner[rival_old as usize]], [1, 0]);
        assert_eq!(game.territory_standing(50).counts, [2, 1, 0, 0]);
        assert!(game.apply_territory(0, 1, 0, claimed, NONE));
        assert_eq!(game.towner[claimed as usize], 1);
        assert_eq!(game.territory_standing(50).counts, [1, 2, 0, 0]);
        assert_eq!(game.winner, 0);
        game.save();
        let mut resumed = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(), resumed.state_hash());
        resumed.save();
        assert_eq!(game.savebuf, resumed.savebuf);
        for _ in 0..100 {
            game.tick();
            resumed.tick();
            assert_eq!(game.state_hash(), resumed.state_hash());
        }
    }

    #[test]
    fn fallback_waits_for_the_next_active_seat_poll_and_resumes_exactly() {
        let mut game = match_for(90);
        for actor in &mut game.ents[..game.high] {
            if actor.team == 1 { actor.hp = 0.0; }
        }
        step(&mut game, 20);
        assert_eq!(game.winner, 0);
        assert!(game.territory_victory.unwrap().eliminated(1));
        assert!(game.territory_standing(90).winning_seat().is_none());
        game.save();
        let mut resumed = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(), resumed.state_hash());
        step(&mut game, 20);
        step(&mut resumed, 20);
        assert_eq!(game.winner, 1);
        assert_eq!(game.state_hash(), resumed.state_hash());
        game.save();
        resumed.save();
        assert_eq!(game.savebuf, resumed.savebuf);
        let final_match = Game::load(&game.savebuf).unwrap();
        assert_eq!(final_match.winner, 1);
    }

    #[test]
    fn equal_territory_leaders_do_not_win() {
        let game = match_for(30);
        let total = game.map.tseeds.len() as u32;
        for percent in PERCENTAGES {
            assert_eq!(game.territory_standing(percent).target,
                ((f64::from(total * u32::from(percent)) * 0.01).ceil() as u32).max(2).min(total));
        }
        let tied = TerritoryStanding { total: 10, target: 3, counts: [3, 3, 2, 0] };
        assert_eq!(tied.winning_seat(), None);
        let leading = TerritoryStanding { counts: [3, 4, 2, 0], ..tied };
        assert_eq!(leading.winning_seat(), Some(1));
    }

    #[test]
    fn the_required_paid_claim_wins_while_the_rival_still_has_a_town() {
        let mut game = match_for(30);
        let builders: Vec<_> = game.ents[..game.high].iter().enumerate()
            .filter(|(_, actor)| actor.alive && actor.def == WORKER && actor.team == 0)
            .map(|(index, _)| index).collect();
        let rival_town = owned_town(&game, 1);
        let target = game.territory_standing(30).target;
        for count in 2..=target {
            let town = place_next_town(&mut game, &builders);
            complete_claim(&mut game, town);
            assert_eq!(game.territory_standing(30).counts[0], count);
            if count < target { assert_eq!(game.winner, 0); }
        }
        step(&mut game, 20);
        assert!(game.ents[rival_town].alive);
        assert_eq!(game.winner, 1);
        assert!(game.territory_victory.unwrap().eliminated(1));
        game.save();
        assert_eq!(Game::load(&game.savebuf).unwrap().winner, 1);
    }
}
