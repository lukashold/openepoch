//! The original Capitols clauses follow the starting city actor through capture.

use crate::{
    ai9_count::Precision53,
    defs::TC,
    ee2_sole_survivor::SoleSurvivor,
    game::{Game, NONE},
    map::tile_of,
    save::checksum,
};

const TAG: u32 = u32::from_le_bytes(*b"CPT1");
const HEADER: usize = 64;
const IDLE: f32 = -1.0;
const CAPTURE_SCORE: u32 = 100;

pub(crate) struct CapitolCapture {
    original: u8,
    previous: u8,
    capturer: u8,
    allied: bool,
}

impl CapitolCapture {
    fn notice(&self, local: u8) -> Option<u8> {
        let taking = local == self.capturer;
        let losing = local == self.previous;
        if self.original == self.capturer {
            if taking { Some(0) } else { losing.then_some(1) }
        } else if self.allied {
            if taking { Some(2) }
            else if losing { Some(3) }
            else { (local == self.original).then_some(4) }
        } else if taking { Some(5) } else { losing.then_some(6) }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Capitols {
    mode: u8,
    minutes: u8,
    sole: bool,
    pub(crate) state: SoleSurvivor,
    city: [u32; 4],
    deadline: [f32; 4],
    score: [u32; 4],
}

impl Capitols {
    fn valid_settings(mode: u32, minutes: u32, sole: u32) -> bool {
        matches!(minutes, 2 | 5 | 10) && matches!((mode, sole), (5, 0 | 1) | (6, 0))
    }

    fn claimed_seat(self, city: u32) -> Option<usize> {
        if city == NONE { return None; }
        self.city.iter().position(|&handle| handle == city)
    }

    pub(crate) fn sole_state(self) -> Option<SoleSurvivor> {
        self.sole.then_some(self.state)
    }

    fn eligible(self, presence: u8, available: u8, allied: impl Fn(u8, u8) -> bool) -> u8 {
        if self.mode == 5 { return presence; }
        (0..4).fold(presence, |admitted, seat| {
            let bit = 1 << seat;
            if available & bit != 0 && presence & bit == 0
                && (0..4).any(|other| presence & (1 << other) != 0 && allied(seat, other))
            {
                admitted | bit
            } else {
                admitted
            }
        })
    }

    fn deadline_result(&mut self, seat: usize, current: Option<u8>, now: f32) -> bool {
        let Some(owner) = current else { return false; };
        let deadline = &mut self.deadline[seat];
        if owner as usize == seat {
            *deadline = IDLE;
            return true;
        }
        if *deadline < 0.0 {
            *deadline = capitol_deadline(now, self.minutes);
        }
        now <= *deadline
    }

    pub(crate) fn hash(self, outcome: u8, mut mix: impl FnMut(u32)) {
        let scored = self.score != [0; 4];
        for word in [TAG, 1 + u32::from(scored), self.mode.into(), self.minutes.into(), u32::from(self.sole),
            self.state.eliminated.into(), self.state.deadline.to_bits(), outcome.into()] { mix(word); }
        for handle in self.city { mix(handle); }
        for deadline in self.deadline { mix(deadline.to_bits()); }
        if scored { for score in self.score { mix(score); } }
    }

    pub(crate) fn wrap(self, payload: Vec<u8>, outcome: u8) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER + 16 + payload.len() + 4);
        self.hash(outcome, |word| bytes.extend_from_slice(&word.to_le_bytes()));
        bytes.extend(payload);
        bytes.extend_from_slice(&checksum(&bytes).to_le_bytes());
        bytes
    }
}

fn capitol_deadline(now: f32, minutes: u8) -> f32 {
    let rounding = Precision53::TowardZero;
    rounding.store_f32(rounding.add(f64::from(now), f64::from(minutes) * 60.0))
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<(Capitols, u8)>)> {
    if !bytes.starts_with(&TAG.to_le_bytes()) { return Some((bytes, None)); }
    let word = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let end = bytes.len().checked_sub(4)?;
    let header = match word(4)? { 1 => HEADER, 2 => HEADER + 16, _ => return None };
    if end <= header || checksum(&bytes[..end]) != word(end)? { return None; }
    let (mode, minutes, sole) = (word(8)?, word(12)?, word(16)?);
    let (eliminated, outcome) = (u8::try_from(word(20)?).ok()?, u8::try_from(word(28)?).ok()?);
    if !Capitols::valid_settings(mode, minutes, sole) || eliminated > 15 || outcome > 2 { return None; }
    let clock = |at| {
        let value = f32::from_bits(word(at)?);
        (value == IDLE || value.is_finite() && value > 0.0).then_some(value)
    };
    let mut rule = Capitols {
        mode: mode as u8, minutes: minutes as u8, sole: sole == 1,
        state: SoleSurvivor { eliminated, deadline: clock(24)? },
        city: [NONE; 4], deadline: [IDLE; 4], score: [0; 4],
    };
    for seat in 0..4 {
        rule.city[seat] = word(32 + 4 * seat)?;
        rule.deadline[seat] = clock(48 + 4 * seat)?;
    }
    if header > HEADER {
        for seat in 0..4 { rule.score[seat] = word(HEADER + 4 * seat)?; }
        if rule.score == [0; 4] || rule.score.iter().any(|value| value % CAPTURE_SCORE != 0) { return None; }
    }
    Some((&bytes[header..end], Some((rule, outcome))))
}

impl Game {
    pub(crate) fn configure_match_capitols(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32,
        mode: u32, minutes: u32, sole: u32,
    ) -> bool {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return false; }
        if self.net || self.humans != 1 || self.local != 0 || self.nplayers < 2
            || !Capitols::valid_settings(mode, minutes, sole) { return false; }
        let mut city = [NONE; 4];
        for (seat, slot) in city[..self.nplayers].iter_mut().enumerate() {
            let (x, y) = self.map.starts[seat];
            let territory = self.map.terr[tile_of(x as f32, y as f32)] as usize;
            let handle = self.territory.claims[territory];
            let Some(index) = self.get(handle) else { return false; };
            let actor = &self.ents[index];
            if actor.def != TC || actor.team as usize != seat || actor.built < 1.0 { return false; }
            *slot = handle;
        }
        if !self.configure_match_epoch_range(civs, pop, supplies, start, end) { return false; }
        self.capitols = Some(Capitols { mode: mode as u8, minutes: minutes as u8, sole: sole == 1,
            state: SoleSurvivor { eliminated: 0, deadline: IDLE }, city, deadline: [IDLE; 4], score: [0; 4] });
        true
    }

    pub(crate) fn capitol_capture_owner(&self, handle: u32, capturer: u8) -> u8 {
        let Some(seat) = self.capitols.filter(|rule| rule.mode == 6)
            .and_then(|rule| rule.claimed_seat(handle)) else { return capturer; };
        let owner = seat as u8;
        if self.players[seat].alive && self.allied(capturer, owner) { owner } else { capturer }
    }

    pub(crate) fn rebind_capitol(&mut self, old: u32, new: u32) {
        if let Some(rule) = &mut self.capitols {
            if let Some(seat) = rule.claimed_seat(old) { rule.city[seat] = new; }
        }
    }

    pub(crate) fn capitol_capture(&self, handle: u32, previous: u8, capturer: u8) -> Option<CapitolCapture> {
        let original = self.capitols?.claimed_seat(handle)? as u8;
        Some(CapitolCapture { original, previous, capturer, allied: self.allied(original, capturer) })
    }

    pub(crate) fn finish_capitol_capture(&mut self, capture: CapitolCapture, recipient: u8) {
        let rule = self.capitols.as_mut().expect("Registered Capitol");
        if capture.previous == capture.original {
            // 00871D29 checks the current owner's Capitol registry entry before transfer.
            let score = &mut rule.score[usize::from(recipient)];
            *score = score.checked_add(CAPTURE_SCORE).unwrap_or(*score);
        }
        let minutes = f32::from(rule.minutes);
        if let Some(notice) = capture.notice(self.local) {
            let seats = (capture.capturer << 4) | (capture.previous << 2) | capture.original;
            // The local interface notice survives fog and a full world sound queue.
            self.events.extend_from_slice(&[148.0 + f32::from(notice), f32::from(seats), minutes]);
        }
    }

    fn capitol_owner(&self, handle: u32) -> Option<u8> {
        self.get(handle).filter(|&index| self.ents[index].hp > 0.0)
            .map(|index| self.ents[index].team)
    }

    pub(crate) fn poll_capitol_outcome(&mut self, mut rule: Capitols) {
        let available = rule.state.active(self.nplayers);
        let presence = self.players[..self.nplayers].iter().enumerate()
            .filter(|(_, player)| player.alive).fold(0, |mask, (seat, _)| mask | (1 << seat));
        let mut survivors = rule.eligible(presence, available, |a, b| self.allied(a, b));
        let allied = (0..self.nplayers as u8).all(|seat| survivors & (1 << seat) == 0
            || (0..seat).all(|other| survivors & (1 << other) == 0 || self.allied(seat, other)));
        let mut won = false;
        if allied {
            if rule.sole && survivors.count_ones() > 1 {
                self.poll_sole_disband(&mut rule.state, available, Precision53::TowardZero);
            } else {
                won = true;
            }
        } else {
            if rule.sole && survivors.count_ones() == 2 && survivors != available {
                self.clear_sole_diplomacy_offers();
            }
            let now = self.sim_elapsed();
            for seat in 0..self.nplayers {
                if available & (1 << seat) != 0
                    && !rule.deadline_result(seat, self.capitol_owner(rule.city[seat]), now)
                {
                    survivors &= !(1 << seat);
                }
            }
        }
        rule.state.eliminated |= available & !survivors;
        for (seat, player) in self.players[..self.nplayers].iter_mut().enumerate() {
            player.alive = survivors & (1 << seat) != 0;
        }
        if survivors & (1 << self.local) == 0 { self.winner = 2; }
        else if won { self.winner = 1; }
        self.capitols = Some(rule);
    }

    pub(crate) fn capitol_status(&self, field: u32, seat: usize) -> i32 {
        let Some(rule) = self.capitols else { return if field == 0 { 0 } else { -1 }; };
        match field {
            0 => rule.mode.into(),
            1 => rule.minutes.into(),
            2 if seat < self.nplayers && rule.deadline[seat] > 0.0 =>
                Precision53::TowardZero.add(f64::from(rule.deadline[seat]), -f64::from(self.sim_elapsed())) as i32,
            3 if seat < self.nplayers => self.capitol_owner(rule.city[seat]).map_or(-1, i32::from),
            4 if seat < self.nplayers => rule.city[seat] as i32,
            5 if seat < self.nplayers => rule.score[seat] as i32,
            _ => -1,
        }
    }

    pub(crate) fn restore_capitols(&mut self, rule: Capitols, outcome: u8) -> Option<()> {
        let now = self.sim_elapsed();
        let state = rule.state;
        let seats = (1u8 << self.nplayers) - 1;
        let max_deadline = capitol_deadline(now, rule.minutes);
        if self.net || self.humans != 1 || self.nplayers < 2 || self.bench != 0
            || state.eliminated & !seats != 0
            || state.deadline != IDLE && (!rule.sole || !(10.0..=now + 10.0).contains(&state.deadline))
            || self.tick == 0 && (outcome != 0 || state.eliminated != 0 || state.deadline != IDLE || rule.score != [0; 4])
            || state.eliminated(0) != (outcome == 2)
            || outcome == 1 && rule.sole && state.active(self.nplayers) != 1
        { return None; }
        for seat in 0..4 {
            let handle = rule.city[seat];
            let deadline = rule.deadline[seat];
            if seat >= self.nplayers {
                if handle != NONE || deadline != IDLE || rule.score[seat] != 0 { return None; }
                continue;
            }
            if deadline != IDLE && (deadline > max_deadline || self.tick == 0) { return None; }
            if handle == NONE {
                if self.tick == 0 { return None; }
            } else {
                let actor = &self.ents[self.get(handle)?];
                let (x, y) = self.map.starts[seat];
                if actor.def != TC || actor.built < 1.0 || actor.x != x as f32 || actor.y != y as f32
                    || rule.city[..seat].contains(&handle) || self.tick == 0 && actor.team as usize != seat
                { return None; }
            }
            if state.eliminated(seat) && self.players[seat].alive { return None; }
        }
        self.capitols = Some(rule);
        self.winner = outcome;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skirmish(mode: u32, ai: u32) -> Game {
        let mut game = Game::new(7, ai, 0, 1, 0);
        let civs = if ai == 1 { 0xee81 } else { 0xe281 };
        assert!(game.configure_match_capitols(civs, 500, 2, 0, 0, mode, 2, 0));
        assert!(game.configure_priest_skirmish());
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        game
    }

    #[test]
    fn occupied_capitol_uses_a_strict_clock_and_restores_the_result() {
        let mut game = skirmish(5, 1);
        let rule = game.capitols.unwrap();
        let city = game.get(rule.city[1]).unwrap();
        // Occupied fixture. The capture handler's handle/recipient transfer
        // is covered separately by the ordinary completion regression.
        game.ents[city].team = 0;
        game.rebuild_territories();
        game.tick();
        let captured = game.capitols.unwrap();
        assert_eq!(game.capitol_status(3, 1), 0);
        assert_eq!(captured.city[1], rule.city[1]);
        assert_eq!(game.capitol_status(2, 1),
            (f64::from(captured.deadline[1]) - f64::from(game.sim_elapsed())) as i32);
        game.save();
        let mut loaded = Game::load(&game.savebuf).expect("active Capitol save");
        assert_eq!(loaded.state_hash(), game.state_hash());
        assert_eq!(loaded.capitols.unwrap().city, captured.city);
        for _ in 0..2500 {
            let before = game.sim_elapsed();
            game.tick();
            loaded.tick();
            if !game.players[1].alive {
                assert!(before <= captured.deadline[1] && game.sim_elapsed() > captured.deadline[1]);
                assert_eq!(game.state_hash(), loaded.state_hash());
                break;
            }
        }
        assert!(!game.players[1].alive);
        game.tick();
        loaded.tick();
        assert_eq!(game.winner, 1);
        assert_eq!(loaded.state_hash(), game.state_hash());
        game.save();
        assert_eq!(Game::load(&game.savebuf).expect("finished Capitol save").winner, 1);
    }

    #[test]
    fn losing_the_original_city_does_not_promote_a_replacement_city() {
        let mut game = skirmish(5, 1);
        let old = game.capitols.unwrap().city[0];
        let city = game.get(old).unwrap();
        game.kill(city);
        assert_eq!(game.capitols.unwrap().city[0], NONE);
        let new = game.spawn(TC, 0, 130.0, 131.0);
        assert_ne!(game.h(new), old);
        game.tick();
        assert_eq!(game.capitol_status(3, 0), -1);
        assert_eq!(game.winner, 2);
        game.save();
        assert_eq!(Game::load(&game.savebuf).expect("defeated Capitol save").winner, 2);
    }

    #[test]
    fn the_first_occupied_step_chops_the_stored_deadline_before_strict_comparison() {
        let mut rule = skirmish(5, 1).capitols.unwrap();
        assert!(rule.deadline_result(1, Some(0), 0.05));
        let nearest = 120.05f32;
        let chopped = f32::from_bits(nearest.to_bits() - 1);
        assert_eq!(rule.deadline[1], chopped);
        assert!(rule.deadline_result(1, Some(0), chopped));
        assert!(!rule.deadline_result(1, Some(0), nearest));
    }

    #[test]
    fn capitol_notice_audience_matches_the_three_original_dispatch_paths() {
        let routes = |capture: CapitolCapture| [0, 1, 2, 3].map(|local| capture.notice(local));
        assert_eq!(routes(CapitolCapture { original: 0, previous: 2, capturer: 0, allied: false }),
            [Some(0), None, Some(1), None]);
        assert_eq!(routes(CapitolCapture { original: 1, previous: 2, capturer: 0, allied: true }),
            [Some(2), Some(4), Some(3), None]);
        assert_eq!(routes(CapitolCapture { original: 1, previous: 1, capturer: 0, allied: false }),
            [Some(5), Some(6), None, None]);
    }

    #[test]
    fn an_allied_captor_returns_only_the_original_city() {
        let mut game = skirmish(6, 2);
        let city = game.capitols.unwrap().city[1];
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        for _ in 0..100 { game.tick(); }
        assert!(game.apply_diplomacy(0, 1, 1, 1, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(game.allied(0, 1));
        assert_eq!(game.capitol_capture_owner(city, 0), 1);
        assert_eq!(game.capitol_capture_owner(game.capitols.unwrap().city[2], 0), 0);
        let expansion = game.spawn(TC, 1, 135.0, 133.0);
        assert_eq!(game.capitol_capture_owner(game.h(expansion), 0), 0);
    }
}
