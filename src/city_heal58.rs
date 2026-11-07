//! CityHeal attachment timing and historical CityManager membership.

use std::collections::BTreeMap;
use crate::defs::{DefId, TC};
use crate::game::{Game, MAXE};

const MAGIC: u32 = u32::from_le_bytes(*b"CHL1");
const VERSION: u32 = 1;
const RADII: [f32; 5] = [7.0, 8.0, 9.0, 10.0, 11.0];
const BASE_HEAL: [f32; 5] = [1.5, 2.0, 2.5, 3.0, 3.5];

#[derive(Clone, Copy)]
struct Band {
    since: f32,
    radius: f32,
}

struct City {
    last: f32,
    bands: Vec<Band>,
}

#[derive(Default)]
pub(crate) struct State {
    cities: BTreeMap<u32, City>,
}

fn valid_recipient(def: DefId, allied: bool) -> bool {
    match def {
        61 | 86 => !allied,
        8..=28 | 43 | 46 | 57 | 62..=81 | 85 => true,
        _ => false,
    }
}

fn source_id(game: &Game, index: usize, handle: u32) -> Option<i32> {
    let actor = game.powers.as_ref()?.actor(handle)?;
    Some(((u32::from(game.ents[index].team) + 1) << 27 | actor.serial) as i32)
}

impl State {
    fn observe(&mut self, game: &Game, now: f32) {
        self.cities.retain(|&handle, _| game.get(handle).is_some_and(|i| game.city_ground_active(&game.ents[i])));
        for (index, actor) in game.ents[..game.high].iter().enumerate() {
            if !game.city_ground_active(actor) { continue; }
            let handle = game.h(index);
            self.cities.entry(handle).or_insert_with(|| City {
                last: 0.0, bands: vec![Band { since: now, radius: game.city_ground_radius(index) }],
            });
        }
    }

    fn grow(&mut self, game: &Game, now: f32) {
        for (&handle, city) in &mut self.cities {
            let Some(index) = game.get(handle) else { continue; };
            let radius = game.city_ground_radius(index);
            if city.bands.last().is_some_and(|last| radius > last.radius) {
                city.bands.push(Band { since: now, radius });
            }
        }
    }

    fn winner(&self, game: &Game, x: f32, y: f32) -> Option<u32> {
        let mut earliest = None;
        for (&handle, city) in &self.cities {
            let index = game.get(handle)?;
            let actor = &game.ents[index];
            let distance = (x - actor.x).abs() + (y - actor.y).abs();
            let band = city.bands.iter().find(|band| {
                f64::from(distance) < f64::from(band.radius) * f64::from(core::f32::consts::SQRT_2)
            });
            let Some(band) = band else { continue; };
            let rank = (band.since, source_id(game, index, handle)?);
            if earliest.is_none_or(|(other, _)| rank < other) { earliest = Some((rank, handle)); }
        }
        earliest.map(|(_, handle)| handle)
    }

    fn update(&mut self, game: &mut Game) {
        let now = game.sim_elapsed();
        self.observe(game, now);
        let mut pulses = Vec::new();
        for (&handle, city) in &mut self.cities {
            let elapsed = if city.last == 0.0 { 1.0 } else { now - city.last };
            if elapsed < 1.0 { continue; }
            city.last = now;
            let Some(index) = game.get(handle) else { continue; };
            pulses.push((index, elapsed));
        }
        if pulses.is_empty() { return; }
        let mut recipients: BTreeMap<u32, Vec<(usize, f32)>> = BTreeMap::new();
        let map_range = 0.0..=crate::map::N as f32;
        for (index, unit) in game.ents[..game.high].iter().enumerate() {
            if !unit.alive || unit.hp <= 0.0 || !valid_recipient(unit.def, false)
                || !map_range.contains(&unit.x) || !map_range.contains(&unit.y)
                || game.trade_hidden(index, unit) || game.air_parked(index, unit) { continue; }
            let maximum = game.max_hp(unit.team, unit.def);
            if unit.hp >= maximum { continue; }
            if let Some(city) = self.winner(game, unit.x, unit.y) {
                recipients.entry(city).or_default().push((index, maximum));
            }
        }
        pulses.sort_unstable_by_key(|&(index, _)| index);
        for (source, elapsed) in pulses {
            let actor = game.ents[source];
            let Some(city) = self.winner(game, actor.x, actor.y) else { continue; };
            let epoch = game.players[actor.team as usize].epoch;
            let base = BASE_HEAL[(epoch / 3) as usize];
            let scale = game.crown_healing_building_scale(actor.team, TC);
            let heal = (f64::from(base) * f64::from(scale) * f64::from(elapsed)) as f32;
            for &(index, maximum) in recipients.get(&city).into_iter().flatten() {
                let unit = game.ents[index];
                if actor.team != unit.team && (!valid_recipient(unit.def, true) || !game.allied(actor.team, unit.team)) { continue; }
                game.ents[index].hp = (unit.hp + heal).min(maximum);
            }
        }
    }

    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.cities.len() as u32);
        for (&handle, city) in &self.cities {
            for word in [handle, city.last.to_bits(), city.bands.len() as u32] { write(word); }
            for band in &city.bands {
                write(band.since.to_bits());
                write(band.radius.to_bits());
            }
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        write(MAGIC);
        write(VERSION);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut result = Vec::new();
        for word in [MAGIC, VERSION, core.len() as u32] { result.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| result.extend_from_slice(&word.to_le_bytes()));
        result.extend_from_slice(&core);
        result.extend_from_slice(&crate::save::checksum(&result).to_le_bytes());
        result
    }

    pub(crate) fn read(game: &Game, bytes: &[u8]) -> Option<Self> {
        if game.net || game.bench != 0 || game.powers.is_none() || game.crowns.is_none() { return None; }
        let now = game.sim_elapsed();
        let mut words = bytes.chunks_exact(4);
        if !words.remainder().is_empty() { return None; }
        let mut next = || Some(u32::from_le_bytes(words.next()?.try_into().ok()?));
        let count = next()? as usize;
        let live_cities = game.ents[..game.high].iter().filter(|unit| game.city_ground_active(unit)).count();
        if count != live_cities { return None; }
        let mut state = Self::default();
        for _ in 0..count {
            let handle = next()?;
            let index = game.get(handle)?;
            if !game.city_ground_active(&game.ents[index]) || source_id(game, index, handle).is_none()
                || state.cities.last_key_value().is_some_and(|(&prior, _)| prior >= handle) { return None; }
            let last = f32::from_bits(next()?);
            if !(0.0..=now).contains(&last) { return None; }
            let bands_count = next()? as usize;
            if !(1..=RADII.len()).contains(&bands_count) { return None; }
            let mut bands = Vec::with_capacity(bands_count);
            for _ in 0..bands_count {
                let since = f32::from_bits(next()?);
                let radius = f32::from_bits(next()?);
                if !(0.0..=now).contains(&since) || !RADII.contains(&radius)
                    || bands.last().is_some_and(|prior: &Band| since < prior.since || radius <= prior.radius) { return None; }
                bands.push(Band { since, radius });
            }
            if bands.last()?.radius > game.city_ground_radius(index) || last != 0.0 && last < bands[0].since { return None; }
            state.cities.insert(handle, City { last, bands });
        }
        (words.len() == 0).then_some(state)
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |offset: usize| Some(u32::from_le_bytes(data.get(offset..offset + 4)?.try_into().ok()?));
    let end = data.len().checked_sub(4)?;
    let len = word(8)? as usize;
    let start = end.checked_sub(len)?;
    if word(4)? != VERSION || len == 0 || !(16..=16 + MAXE * 52).contains(&start)
        || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}

impl Game {
    pub(crate) fn configure_city_heal(&mut self) -> bool {
        if self.tick != 0 || self.net || self.bench != 0 || self.city_heal.is_some()
            || self.powers.is_none() || self.crowns.is_none() { return false; }
        let mut state = State::default();
        state.observe(self, self.sim_elapsed());
        self.city_heal = Some(state);
        true
    }

    pub(crate) fn retire_city_heal(&mut self, index: usize) {
        let handle = self.h(index);
        if let Some(state) = &mut self.city_heal { state.cities.remove(&handle); }
    }

    pub(super) fn tick_city_heal(&mut self) {
        let Some(mut state) = self.city_heal.take() else { return; };
        state.update(self);
        self.city_heal = Some(state);
    }

    pub(super) fn tick_city_heal_growth(&mut self) {
        let Some(mut state) = self.city_heal.take() else { return; };
        state.grow(self, self.sim_elapsed());
        self.city_heal = Some(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diplomacy::ALLIED;

    fn setup(players: u32, enabled: bool) -> Game {
        let mut game = Game::new(7, players - 1, 0, 1, 0);
        for player in &mut game.players { player.ai = false; }
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crowns(false));
        if enabled { assert!(game.configure_city_heal()); }
        game
    }

    fn move_to(game: &mut Game, index: usize, x: f32, y: f32) {
        let actor = &mut game.ents[index];
        actor.x = x;
        actor.y = y;
        actor.px = x;
        actor.py = y;
        actor.ox = x;
        actor.oy = y;
    }

    fn city(game: &Game, team: u8) -> usize {
        game.ents[..game.high].iter().position(|unit| unit.alive && unit.def == TC && unit.team == team).unwrap()
    }

    #[test]
    fn own_and_ally_city_masks_use_strict_current_location() {
        let mut game = setup(2, true);
        game.diplomacy.relation[1] = ALLIED;
        game.diplomacy.relation[4] = ALLIED;
        let source = city(&game, 0);
        let rival = city(&game, 1);
        move_to(&mut game, source, 0.0, 16.0);
        move_to(&mut game, rival, 140.0, 140.0);
        let own_spy = game.spawn(86, 0, 1.0, 16.0);
        let ally_spy = game.spawn(86, 1, 2.0, 16.0);
        let ally_scout = game.spawn(57, 1, 3.0, 16.0);
        let ally_special = game.spawn(61, 1, 4.0, 16.0);
        let edge = 7.0 * core::f32::consts::SQRT_2;
        let inside = game.spawn(8, 0, f32::from_bits(edge.to_bits() - 1), 16.0);
        let outside = game.spawn(8, 0, f32::from_bits(edge.to_bits() + 1), 16.0);
        let ship = game.spawn(82, 0, 5.0, 16.0);
        for i in [own_spy, ally_spy, ally_scout, ally_special, inside, outside, ship] { game.ents[i].hp = 10.0; }
        game.tick = 1;
        game.tick_city_heal();
        for i in [own_spy, ally_scout, inside] { assert_eq!(game.ents[i].hp, 11.5, "eligible {}", game.ents[i].def); }
        for i in [ally_spy, ally_special, outside, ship] { assert_eq!(game.ents[i].hp, 10.0, "ineligible {}", game.ents[i].def); }
        move_to(&mut game, outside, 6.0, 16.0);
        game.tick = 30;
        game.tick_city_heal();
        assert_eq!(game.ents[outside].hp, 10.0 + heal_at(1.5, 1.0, game.sim_elapsed() - 0.05));
    }

    fn heal_at(base: f32, scale: f32, elapsed: f32) -> f32 {
        (f64::from(base) * f64::from(scale) * f64::from(elapsed)) as f32
    }

    #[test]
    fn independent_pulses_replay_after_archive_and_legacy_stays_exact() {
        let mut old = setup(1, false);
        old.save();
        let archive = old.savebuf.clone();
        let mut old = Game::load(&archive).unwrap();
        assert!(old.city_heal.is_none());
        old.save();
        assert_eq!(old.savebuf, archive);
        old.tick();
        assert!(!old.configure_city_heal());

        let mut game = setup(1, true);
        let first = city(&game, 0);
        move_to(&mut game, first, 100.0, 100.0);
        let second = game.spawn(TC, 0, 104.0, 100.0);
        let citizen = game.spawn(8, 0, 101.0, 101.0);
        game.ents[citizen].hp = 10.0;
        assert!(game.tick_with_source_delta(0.05));
        assert_eq!(game.ents[citizen].hp, 13.0, "two attached source effects share one winning city");
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        loaded.save();
        assert_eq!(loaded.savebuf, game.savebuf);
        for delta in [0.049, 0.0625, 0.08].into_iter().cycle().take(22) {
            assert!(game.tick_with_source_delta(delta));
            assert!(loaded.tick_with_source_delta(delta));
            assert_eq!(loaded.state_hash(), game.state_hash());
        }
        assert!(game.ents[citizen].hp > 16.0);
        let retired = game.h(second);
        game.ents[second].hp = 0.0;
        assert!(game.tick_with_source_delta(0.05));
        assert!(!game.city_heal.as_ref().unwrap().cities.contains_key(&retired));
        let restored = game.spawn(TC, 0, 104.0, 100.0);
        let before = game.ents[citizen].hp;
        assert!(game.tick_with_source_delta(0.05));
        assert_ne!(game.h(restored), retired);
        assert_eq!(game.ents[citizen].hp, before + 1.5, "new generation's first pulse uses one second");
    }

    #[test]
    fn current_power_and_rate_recompute_between_pulses_and_bad_archive_rejected() {
        let mut game = setup(1, true);
        let city = city(&game, 0);
        let position = (game.ents[city].x, game.ents[city].y);
        let citizen = game.spawn(8, 0, position.0 + 1.0, position.1);
        let outer = game.spawn(8, 0, position.0 + 10.5, position.1);
        game.ents[citizen].hp = 10.0;
        game.ents[outer].hp = 10.0;
        game.tick = 1;
        game.tick_city_heal();
        assert_eq!(game.ents[citizen].hp, 11.5);
        game.players[0].epoch = 3;
        for tech in 44..48 { game.researched[0][tech / 32] |= 1 << (tech % 32); }
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(2, 8));
        game.tick = 21;
        game.tick_city_heal();
        assert_eq!(game.ents[citizen].hp, 11.5 + heal_at(2.0, 1.33, 1.0));
        assert_eq!(game.ents[outer].hp, 10.0, "new radius becomes visible after this pulse");
        game.tick_city_heal_growth();
        game.tick = 6020;
        game.tick_city_heal();
        assert!(game.ents[outer].hp > 10.0);
        game.ents[citizen].hp = 10.0;
        let last = game.sim_elapsed();
        game.tick = 6040;
        game.tick_crowns();
        assert_eq!(game.crown_power(2), None);
        game.tick_city_heal();
        assert_eq!(game.ents[citizen].hp, 10.0 + heal_at(2.0, 1.0, game.sim_elapsed() - last));
        assert!(game.choose_crown_power(2, 8));
        let before = game.ents[citizen].hp;
        game.tick = 6061;
        game.tick_city_heal();
        assert!(game.ents[citizen].hp > before + 2.66);

        let packed = game.city_heal.as_ref().unwrap().wrap(vec![0]);
        let (_, payload) = split(&packed).unwrap();
        let mut corrupt = payload.unwrap().to_vec();
        assert!(State::read(&game, &corrupt).is_some());
        corrupt[8..12].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(State::read(&game, &corrupt).is_none());
        let mut corrupt = packed;
        corrupt[4] = 2;
        assert!(split(&corrupt).is_none());
    }

    #[test]
    fn immediate_city_delete_retires_history_before_next_tick_or_save() {
        let mut game = setup(1, true);
        let handle = game.h(city(&game, 0));
        assert!(game.city_heal.as_ref().unwrap().cities.contains_key(&handle));
        game.apply(0, crate::cmd::C_DELETE, 0.0, 0.0, 0, &[handle]);
        assert!(!game.city_heal.as_ref().unwrap().cities.contains_key(&handle));
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        loaded.save();
        assert_eq!(loaded.savebuf, game.savebuf);
    }
}
