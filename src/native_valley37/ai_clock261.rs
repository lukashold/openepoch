//! Seconds-based state for newly started adaptive, authored skirmishes.
use crate::{game::Game, source_clock38::store_positive_f32};

const TAG: &[u8; 4] = b"DAI1";
const NO_DEADLINE: u32 = u32::MAX;
const CROWN_PERIOD: f32 = 240.0;
const CROWN_TOLERANCE: f32 = f32::from_bits(0x3a83_126f);
pub(crate) const LEGACY_RATE: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Seat {
    crown_remaining: f32,
    wave_at: Option<f32>,
}

impl Seat {
    const START: Self = Self { crown_remaining: CROWN_PERIOD, wave_at: None };

    fn step(&mut self, seconds: f32) {
        if self.crown_remaining == 0.0 { return; }
        // 0046C540 compares the unrounded subtraction to the f32 tolerance.
        // This latch retains the first expiry for the portable wave policy.
        let remaining = f64::from(self.crown_remaining) - f64::from(seconds);
        self.crown_remaining = if remaining <= f64::from(CROWN_TOLERANCE) {
            0.0
        } else {
            store_positive_f32(remaining)
        };
    }

    fn expired(self) -> bool { self.crown_remaining == 0.0 }

    fn words(self) -> [u32; 3] {
        [self.expired() as u32, self.crown_remaining.to_bits(), self.wave_at.map_or(NO_DEADLINE, f32::to_bits)]
    }

    fn read([phase, remaining, deadline]: [u32; 3]) -> Option<Self> {
        let crown_remaining = f32::from_bits(remaining);
        match phase {
            0 if crown_remaining.is_finite() && crown_remaining >= CROWN_TOLERANCE && crown_remaining <= CROWN_PERIOD => (),
            1 if remaining == 0 => (),
            _ => return None,
        }
        let wave_at = if deadline == NO_DEADLINE { None } else {
            let seconds = f32::from_bits(deadline);
            if phase != 1 || !seconds.is_finite() || seconds < 20.0 { return None; }
            Some(seconds)
        };
        Some(Self { crown_remaining, wave_at })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct State {
    seats: [Seat; 3],
    count: usize,
}

impl State {
    pub(crate) fn fresh(game: &Game) -> Option<Self> {
        if game.tick != 0 || !Self::supports(game) { return None; }
        Some(Self { seats: [Seat::START; 3], count: game.nplayers - 1 })
    }

    fn supports(game: &Game) -> bool {
        !game.net && game.humans == 1 && game.local == 0 && game.bench == 0
            && (2..=4).contains(&game.nplayers)
            && game.players[..game.nplayers].iter().enumerate().all(|(team, player)| player.ai == (team != 0))
            && game.rect_population.as_ref().is_some_and(|population| population.animals.is_some())
    }

    pub(crate) fn step(&mut self, seconds: f32) {
        for seat in &mut self.seats[..self.count] { seat.step(seconds); }
    }

    pub(crate) fn wave_ready(&self, team: usize, now: f32) -> bool {
        let seat = self.seats[team - 1];
        seat.expired() && seat.wave_at.is_none_or(|deadline| now >= deadline)
    }

    pub(crate) fn delay_wave(&mut self, team: usize, now: f32, seconds: u32) {
        self.seats[team - 1].wave_at = Some(store_positive_f32(f64::from(now) + f64::from(seconds)));
    }

    fn words(&self) -> impl Iterator<Item = u32> + '_ {
        [u32::from_le_bytes(*TAG), self.count as u32].into_iter()
            .chain(self.seats[..self.count].iter().flat_map(|seat| seat.words()))
    }

    pub(crate) fn encode(&self) -> Vec<u8> {
        self.words().flat_map(u32::to_le_bytes).collect()
    }

    pub(crate) fn decode(bytes: &[u8], game: &Game) -> Option<Self> {
        if !Self::supports(game) || !bytes.starts_with(TAG) { return None; }
        let count = game.nplayers - 1;
        let mut words = bytes[4..].chunks_exact(4);
        if !words.remainder().is_empty() || words.len() != 1 + count * 3 { return None; }
        let mut word = || words.next().map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()));
        if word()? != count as u32 { return None; }
        let mut state = Self { seats: [Seat::START; 3], count };
        for seat in &mut state.seats[..count] { *seat = Seat::read([word()?, word()?, word()?])?; }
        if game.tick == 0 {
            if game.source_clock.is_some() || state != Self::fresh(game)? { return None; }
        } else {
            let source = game.source_clock?;
            if source.adaptive.is_none() { return None; }
            let latest = f64::from(source.elapsed) + 100.0;
            if state.seats[..count].iter().any(|seat| seat.wave_at.is_some_and(|deadline| f64::from(deadline) > latest)) {
                return None;
            }
        }
        Some(state)
    }

    pub(crate) fn hash(&self, mix: &mut impl FnMut(u32)) { self.words().for_each(mix); }
}

// The portable combined planner retains its nominal 24/20-second cadence and
// seat stagger. The original engine has independent strategy and production clocks.
pub(crate) fn planner_due(team: usize, before: f32, now: f32) -> bool {
    let cycles = |seconds: f32| ((f64::from(seconds) * f64::from(LEGACY_RATE) + (team * 7) as f64) / 24.0).floor();
    cycles(before) < cycles(now)
}

#[no_mangle]
pub extern "C" fn native_rect_ai_clock_word(team: usize, field: u32) -> u32 {
    let state = super::active_game().and_then(|game| game.rect_ai_clock.as_ref());
    let seat = state.filter(|state| (1..=state.count).contains(&team)).map(|state| state.seats[team - 1]);
    match (field, seat) {
        (0, _) => seat.is_some() as u32,
        (1, Some(seat)) => seat.crown_remaining.to_bits(),
        (2, Some(seat)) => seat.expired() as u32,
        (3, Some(seat)) => seat.wave_at.map_or(NO_DEADLINE, f32::to_bits),
        _ => u32::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_first_expiry_matches_original_chop_countdown() {
        for (step_bits, expiry, last_remaining) in [(0x3d4c_cccc, 4800, 0x3d17_1654), (0x3d08_8888, 7199, 0x3ccd_4fb0)] {
            let mut seat = Seat::START;
            for _ in 1..expiry { seat.step(f32::from_bits(step_bits)); }
            assert_eq!(seat.crown_remaining.to_bits(), last_remaining);
            assert!(!seat.expired());
            seat.step(f32::from_bits(step_bits));
            assert_eq!(seat.words(), [1, 0, NO_DEADLINE]);
            seat.step(f32::from_bits(step_bits));
            assert_eq!(seat.words(), [1, 0, NO_DEADLINE]);
        }
    }

    #[test]
    fn adaptive_planner_admits_the_crossing_once_for_each_seat() {
        for team in 1..=3 {
            let mut admissions = Vec::new();
            for tick in 1..=200 {
                if planner_due(team, (tick - 1) as f32 / 32.0, tick as f32 / 32.0) {
                    admissions.push(tick);
                }
            }
            let expected: Vec<_> = (1..200).filter(|tick| (tick + team * 7) % 24 == 0)
                .map(|tick| (tick * 32).div_ceil(20)).take_while(|&tick| tick <= 200).collect();
            assert_eq!(admissions, expected);
        }
    }

    #[test]
    fn wave_cooldown_uses_seconds_and_rejects_contradictory_archive_words() {
        let mut state = State { seats: [Seat::START; 3], count: 2 };
        state.seats[0].crown_remaining = 0.0;
        assert!(state.wave_ready(1, 239.9375));
        assert!(!state.wave_ready(2, 300.0));
        state.delay_wave(1, 239.9375, 100);
        assert!(!state.wave_ready(1, 339.937));
        assert!(state.wave_ready(1, 339.9375));
        assert_eq!(Seat::read(state.seats[0].words()), Some(state.seats[0]));
        assert!(Seat::read([0, CROWN_PERIOD.to_bits(), 339.9375_f32.to_bits()]).is_none());
        assert!(Seat::read([1, 0, f32::INFINITY.to_bits()]).is_none());
        assert!(Seat::read([1, (-0.0_f32).to_bits(), NO_DEADLINE]).is_none());
    }
}
