//! The authored Hill objective and the installed VCKingOfTheHill timer.

use crate::defs::{Def, DefId, A_BLD, LAND, NOW};
use crate::game::Game;
use crate::ids::DEF_NONE;
use crate::map::tile_of;

pub(crate) const HILL: DefId = crate::heavy_mounted3_61::ID + 1;
pub(crate) const NATURE: u8 = 4;
pub(crate) const DEFAULT_MINUTES: i32 = 10;
const DURATIONS: [i32; 3] = [2, 5, DEFAULT_MINUTES];
pub(crate) const YAW: f32 = f32::from_bits(0xbf490fdb);
pub(crate) const COLLISION_SIDE: f32 = f32::from_bits(0x403504f3);
const TAG: u32 = u32::from_le_bytes(*b"KTH1");

pub(crate) const fn append(existing: [Def; HILL as usize]) -> [Def; HILL as usize + 1] {
    let hill = Def {
        name: "Der Hügel", epoch: 0, until: 15, dom: LAND, building: true, size: 2.0,
        hp: 1000.0, speed: 0.0, sight: 6.0, armor: A_BLD, cost: [0; 3], time: 0.0,
        pop: 0, made_at: DEF_NONE, flags: 0, w: [NOW; 2],
    };
    let mut definitions = [hill; HILL as usize + 1];
    let mut index = 0;
    while index < existing.len() {
        definitions[index] = existing[index];
        index += 1;
    }
    definitions
}

#[derive(Clone, Copy)]
pub(crate) struct Hold {
    pub(crate) actor: u32,
    pub(crate) minutes: i32,
    pub(crate) holder: u8,
    pub(crate) acquired_at: f32,
}

impl Hold {
    pub(crate) fn new(actor: u32, now: f32, minutes: i32) -> Self {
        Self { actor, minutes, holder: NATURE, acquired_at: now }
    }

    pub(crate) fn observe(&mut self, owner: u8, now: f32) -> Option<u8> {
        if owner != self.holder {
            self.holder = owner;
            self.acquired_at = now;
        }
        (owner != NATURE && f64::from(now) > self.deadline()).then_some(owner)
    }

    fn deadline(self) -> f64 {
        f64::from(self.acquired_at) + f64::from(self.minutes.wrapping_mul(60))
    }

    pub(crate) fn remaining_seconds(self, now: f32) -> i32 {
        (self.deadline() - f64::from(now)) as i32
    }

    pub(crate) fn hash(self, mut mix: impl FnMut(u32)) {
        for word in self.words() { mix(word); }
    }

    fn words(self) -> [u32; 6] {
        [TAG, 1, self.actor, self.minutes as u32, u32::from(self.holder), self.acquired_at.to_bits()]
    }

    pub(crate) fn wrap(self, core: Vec<u8>, outcome: u8) -> Vec<u8> {
        let mut result = Vec::with_capacity(core.len() + 32);
        for word in self.words() { result.extend_from_slice(&word.to_le_bytes()); }
        result.extend_from_slice(&u32::from(outcome).to_le_bytes());
        result.extend_from_slice(&core);
        result.extend_from_slice(&crate::save::checksum(&result).to_le_bytes());
        result
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<(Hold, u8)>)> {
    if !bytes.starts_with(&TAG.to_le_bytes()) { return Some((bytes, None)); }
    let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let end = bytes.len().checked_sub(4)?;
    let minutes = i32::try_from(word(12)?).ok()?;
    if end <= 28 || word(4)? != 1 || !DURATIONS.contains(&minutes)
        || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
    let holder = u8::try_from(word(16)?).ok()?;
    let acquired_at = f32::from_bits(word(20)?);
    let outcome = u8::try_from(word(24)?).ok()?;
    if holder > NATURE || !acquired_at.is_finite() || acquired_at < 0.0 || outcome > 2 { return None; }
    let hold = Hold { actor: word(8)?, minutes, holder, acquired_at };
    Some((&bytes[28..end], Some((hold, outcome))))
}

impl Game {
    pub(crate) fn configure_match_koth(
        &mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32, minutes: i32,
    ) -> bool {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return false; }

        let [width, height] = self.map.world_dimensions();
        let (x, y) = (width as f32 * 0.5, height as f32 * 0.5);
        if self.humans != 1 || self.local != 0 || !DURATIONS.contains(&minutes)
            || !self.koth_placement_available(x, y)
            || !self.map.connected(self.nplayers, true)
            || !self.configure_match_epoch_range(civs, pop, supplies, start, end) {
            return false;
        }
        self.spawn_koth(x, y, minutes);
        true
    }

    fn koth_placement_available(&self, x: f32, y: f32) -> bool {
        let [width, height] = self.map.world_dimensions();
        self.tick == 0 && !self.net && self.koth.is_none() && !self.free.is_empty()
            && (2.0..=width as f32 - 2.0).contains(&x) && (2.0..=height as f32 - 2.0).contains(&y)
    }

    /// Seed the objective only from an already resolved, authenticated map point.
    fn spawn_koth(&mut self, x: f32, y: f32, minutes: i32) -> u32 {
        let index = self.spawn_oriented(HILL, NATURE, x, y, YAW);
        let handle = self.h(index);
        self.koth = Some(Hold::new(handle, self.sim_elapsed(), minutes));
        self.rebuild_territories();
        self.build_grid();
        handle
    }

    #[cfg(test)]
    pub(crate) fn place_koth(&mut self, x: f32, y: f32) -> Option<u32> {
        self.koth_placement_available(x, y).then(|| self.spawn_koth(x, y, DEFAULT_MINUTES))
    }

    pub(crate) fn koth_status(&self, field: u32) -> i32 {
        let Some(rule) = self.koth else { return if field == 0 { 0 } else { -1 }; };
        match field {
            0 => 1,
            1 => rule.actor as i32,
            2 => i32::from(rule.holder),
            3 => rule.remaining_seconds(self.sim_elapsed()),
            4 => rule.minutes,
            _ => -1,
        }
    }

    pub(crate) fn koth_territory(&self) -> Option<(usize, usize)> {
        let index = self.get(self.koth?.actor)?;
        let actor = &self.ents[index];
        Some((self.map.terr[tile_of(actor.x, actor.y)] as usize, index))
    }

    pub(crate) fn restore_koth(&mut self, rule: Hold, outcome: u8) -> Option<()> {
        if self.net || self.bench != 0 || self.humans != 1 || self.local != 0 || rule.acquired_at > self.sim_elapsed()
            || (rule.holder != NATURE && rule.holder as usize >= self.nplayers) { return None; }
        let index = self.get(rule.actor)?;
        let actor = &self.ents[index];
        let [width, height] = self.map.world_dimensions();
        if actor.def != HILL || actor.team != rule.holder || actor.hp <= 0.0 || actor.built != 1.0
            || [actor.x, actor.y] != [width as f32 * 0.5, height as f32 * 0.5]
            || actor.rot.to_bits() != YAW.to_bits() || actor.qn != 0 || actor.ncargo != 0 { return None; }
        if outcome != 0 && (rule.holder == NATURE || (outcome == 1) != (rule.holder == self.local)
            || f64::from(self.sim_elapsed()) <= rule.deadline()) { return None; }
        self.koth = Some(rule);
        self.winner = outcome;
        Some(())
    }

    pub(crate) fn koth_owner_replaced(&mut self, old_handle: u32, new_handle: u32, owner: u8) {
        let now = self.sim_elapsed();
        if let Some(rule) = &mut self.koth {
            if rule.actor == old_handle {
                rule.actor = new_handle;
                rule.observe(owner, now);
            }
        }
    }

    pub(crate) fn poll_koth_outcome(&mut self, mut rule: Hold) {
        let Some(index) = self.get(rule.actor) else { return; };
        if let Some(owner) = rule.observe(self.ents[index].team, self.sim_elapsed()) {
            // The solo setup currently allocates one player per fixed team.
            self.winner = if owner == self.local { 1 } else { 2 };
        }
        self.koth = Some(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hill_uses_the_authored_actor_without_creating_an_extra_player() {
        let mut game = Game::new_hill_world(7, 3);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        let hill = game.place_koth(128.0, 128.0).unwrap();
        let index = game.get(hill).unwrap();
        assert_eq!(game.ents[index].def, HILL);
        assert_eq!(game.ents[index].team, NATURE);
        assert_eq!(game.ents[index].hp, 1000.0);
        assert_eq!(game.ents[index].rot.to_bits(), 0xbf490fdb);
        let (territory, _) = game.koth_territory().unwrap();
        assert_eq!(game.territory_claim_status(0, territory), 5);
        assert_eq!(game.towner[territory], crate::territory::TERR_NONE);
        assert!(!game.construction_available(0, HILL));
        let blocked = (126..130).flat_map(|y| (126..130).map(move |x| (x, y)))
            .filter(|&(x, y)| game.map.blocked[y * crate::map::N + x]).count();
        assert_eq!(blocked, 12);
        assert_eq!(game.nplayers, 4);
        assert!(game.place_koth(129.0, 129.0).is_none());
        for _ in 0..32 { game.tick(); }
        game.diplomacy.visibility = 2;
        game.refresh_display_fog();
        game.render(1.0, 118.0, 118.0, 138.0, 138.0, -1, 0, 0);
        game.minimap();
        let actor = game.ents[index];
        assert_eq!((actor.x, actor.y, actor.hp), (128.0, 128.0, 1000.0));
        assert_eq!(game.relation(0, NATURE), crate::diplomacy::NEUTRAL);
        game.save();
        let mut restored = Game::load(&game.savebuf).expect("Nature Hill archive");
        assert_eq!(game.state_hash(), restored.state_hash());
        restored.save();
        assert_eq!(game.savebuf, restored.savebuf);
    }

    #[test]
    fn default_hill_timer_only_awards_after_strict_deadline() {
        let mut rule = Hold::new(42, 0.0, DEFAULT_MINUTES);
        assert_eq!(rule.remaining_seconds(0.0), 600);
        assert_eq!(rule.observe(NATURE, 800.0), None);
        assert_eq!(rule.observe(0, 801.0), None);
        assert_eq!(rule.observe(0, 1401.0), None);
        assert_eq!(rule.observe(0, 1401.125), Some(0));
        assert_eq!(rule.observe(1, 1401.25), None);
    }

    #[test]
    fn fresh_match_uses_the_generated_center_and_only_accepts_authored_durations() {
        for minutes in DURATIONS {
            let mut game = Game::new_hill_world(minutes as u32 - 1, 1);
            let before = game.state_hash();
            assert!(!game.configure_match_koth(0xee10, 250, 0, 0, 14, 3));
            assert_eq!(game.state_hash(), before);
            assert!(game.configure_match_koth(0xee10, 250, 0, 0, 14, minutes));
            let [width, height] = game.map.world_dimensions();
            let actor = game.ents[game.get(game.koth.unwrap().actor).unwrap()];
            assert_eq!([actor.x, actor.y], [width as f32 / 2.0, height as f32 / 2.0]);
            let center = tile_of(actor.x, actor.y);
            assert!(game.map.ter[center] >= crate::map::T_SAND && game.map.ter[center] < crate::map::T_ROCK);
            assert_eq!(game.koth_status(3), minutes * 60);
            assert_eq!(game.koth_status(4), minutes);
            game.save();
            let restored = Game::load(&game.savebuf).expect("Hill setup archive");
            assert_eq!(restored.koth_status(4), minutes);
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.map.hm, game.map.hm);
        }
    }
}
