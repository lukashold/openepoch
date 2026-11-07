//! The late Imperial Leader's timed protection of one completed building.

use super::*;
use std::collections::BTreeMap;

const IMPERIAL: DefId = crate::leader58_defs::END - 1;
const DURATION: f64 = 45.0;
const MAGIC: u32 = u32::from_le_bytes(*b"IMP1");

#[derive(Clone, Copy)]
struct Shield { owner: u8, expires: f32 }

#[derive(Default)]
pub(crate) struct State(BTreeMap<u32, Shield>);

fn building(actor: &Ent, owner: u8) -> bool {
    actor.alive && actor.hp > 0.0 && actor.built == 1.0 && actor.team == owner
        && DEFS[actor.def as usize].building
}

impl State {
    pub(crate) fn hash(&self, mut mix: impl FnMut(u32)) {
        if self.0.is_empty() { return; }
        mix(MAGIC);
        mix(self.0.len() as u32);
        for (&handle, shield) in &self.0 {
            mix(handle);
            mix(u32::from(shield.owner));
            mix(shield.expires.to_bits());
        }
    }

    pub(crate) fn wrap(&self, core: Vec<u8>, pending: bool) -> Vec<u8> {
        if self.0.is_empty() && !pending { return core; }
        let mut bytes = Vec::with_capacity(core.len() + 20 + self.0.len() * 12);
        for word in [MAGIC, 1, u32::from(pending), self.0.len() as u32] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for (&handle, shield) in &self.0 {
            for word in [handle, u32::from(shield.owner), shield.expires.to_bits()] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], State, bool)> {
    if !bytes.starts_with(&MAGIC.to_le_bytes()) { return Some((bytes, State::default(), false)); }
    let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let pending = word(8)?;
    let count = word(12)? as usize;
    let end = bytes.len().checked_sub(4)?;
    if word(4)? != 1 || pending > 1 || count > MAXE || (pending == 0 && count == 0)
        || end <= 16 + count * 12 || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
    let mut result = State::default();
    for row in 0..count {
        let at = 16 + row * 12;
        let handle = word(at)?;
        let owner = u8::try_from(word(at + 4)?).ok()?;
        let expires = f32::from_bits(word(at + 8)?);
        if !expires.is_finite() || result.0.last_key_value().is_some_and(|(&previous, _)| previous >= handle) { return None; }
        result.0.insert(handle, Shield { owner, expires });
    }
    Some((&bytes[16 + count * 12..end], result, pending != 0))
}

impl Game {
    pub(crate) fn imperialism_ready(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        self.military_leader_powers.is_some() && !self.net && self.winner == 0
            && actor.alive && actor.hp > 0.0 && actor.stuck != BOARDED && actor.def == IMPERIAL
            && self.players[actor.team as usize].epoch >= 10 && self.leader_power_reserve(index) >= 1.0
    }

    pub(crate) fn imperialism_target(&self, owner: u8, target: usize) -> bool {
        building(&self.ents[target], owner)
    }

    pub(crate) fn order_imperialism(&mut self, index: usize, target: usize) {
        if !self.imperialism_ready(index) || !self.imperialism_target(self.ents[index].team, target) { return; }
        let handle = self.h(target);
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = O_IMPERIALISM;
        actor.otgt = handle;
        actor.tgt = NONE;
        actor.stuck = 0;
    }

    pub(super) fn do_imperialism(&mut self, index: usize, actor: &mut Ent) {
        let handle = actor.otgt;
        Self::set_idle(actor);
        actor.otgt = NONE;
        if !self.imperialism_ready(index) { return; }
        let Some(target) = self.get(handle) else { return; };
        if !self.imperialism_target(actor.team, target) { return; }
        let expires = crate::power42_math::store_toward_zero(f64::from(self.sim_elapsed()) + DURATION);
        self.imperialism.0.insert(handle, Shield { owner: actor.team, expires });
        self.spend_unit_reserve(self.h(index), 1.0);
    }

    pub(crate) fn imperialism_active(&self, index: usize) -> bool {
        self.imperialism.0.get(&self.h(index)).is_some_and(|shield|
            shield.expires > self.sim_elapsed() && building(&self.ents[index], shield.owner))
    }

    pub(crate) fn imperialism_visible(&self, handle: u32) -> bool {
        self.get(handle).is_some_and(|index| self.ents[index].team == self.local && self.imperialism_active(index))
    }

    pub(crate) fn retire_imperialism(&mut self, index: usize) {
        self.imperialism.0.remove(&self.h(index));
    }

    pub(super) fn tick_imperialism(&mut self) {
        let now = self.sim_elapsed();
        self.imperialism.0.retain(|&handle, shield| {
            let actor = &self.ents[(handle & 0xffff) as usize];
            shield.expires > now && u32::from(actor.gen) == handle >> 16 && building(actor, shield.owner)
        });
    }

    pub(crate) fn restore_imperialism(&mut self, state: State) -> Option<()> {
        if !state.0.is_empty() && (self.net || self.military_leader_powers.is_none()) { return None; }
        let now = self.sim_elapsed();
        for (&handle, shield) in &state.0 {
            let target = self.get(handle)?;
            if !building(&self.ents[target], shield.owner) || shield.expires <= now
                || f64::from(shield.expires) > f64::from(now) + DURATION { return None; }
        }
        self.imperialism = state;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::C_IMPERIALISM;

    #[test]
    fn imperialism_browser_fixture() {
        let Some(file) = std::env::var_os("OPEN_EPOCH_IMPERIALISM_BROWSER_FIXTURE") else { return; };
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_civilizations([1, 8, 14, 14]));
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        game.players[0].epoch = 10;
        game.rebuild_modifiers(0, false);
        let city = *game.ents[..game.high].iter().find(|actor| actor.alive && actor.team == 0 && actor.def == TC).unwrap();
        let leader = game.spawn(IMPERIAL, 0, city.x + 4.5, city.y - 4.5);
        let handle = game.h(leader);
        game.powers.as_mut().unwrap().recharge_selected(handle);
        game.save();
        assert!(Game::load(&game.savebuf).is_some());
        std::fs::write(file, &game.savebuf).unwrap();
    }

    #[test]
    fn imperialism_protects_the_selected_building_until_its_deadline() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_civilizations([1, 8, 14, 14]));
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        assert!(game.configure_economic_leader_effects());
        game.players[0].epoch = 10;
        game.players[1].epoch = 5;
        game.rebuild_modifiers(0, false);
        game.rebuild_modifiers(1, false);
        game.diplomacy.relation[1] = crate::diplomacy::WAR;
        game.diplomacy.relation[4] = crate::diplomacy::WAR;
        game.team_vis.fill(255);
        for slot in 0..9 { game.set_resource(1, slot, 2000.0); }

        let leader = game.spawn(IMPERIAL, 0, 82.0, 82.0);
        let house = game.spawn(HOUSE, 0, 80.0, 80.0);
        let nearby = game.spawn(HOUSE, 0, 77.0, 80.0);
        let rival = game.spawn(HOUSE, 1, 92.0, 92.0);
        let foundation = game.spawn(HOUSE, 0, 86.0, 86.0);
        game.ents[foundation].built = 0.5;
        let source = game.h(leader);
        let target = game.h(house);
        game.powers.as_mut().unwrap().recharge_selected(source);
        assert!(!game.imperialism_target(0, rival));
        assert!(!game.imperialism_target(0, foundation));
        assert!(!game.imperialism_target(0, leader));
        assert!(game.leader_purchase_target(1, house));
        game.apply(0, C_IMPERIALISM, 0.0, 0.0, target as i32, &[source]);
        assert_eq!(game.ents[leader].order, O_IMPERIALISM);
        game.save();
        let mut game = Game::load(&game.savebuf).expect("pending Imperialism survives load");
        game.tick();
        let now = game.sim_elapsed();
        assert!(game.imperialism_visible(target));
        assert!(game.leader_power_reserve(leader) < 0.01);
        game.team_vis.fill(255);
        assert!(!game.leader_purchase_target(1, house));
        assert!(game.combat_hostile(1, 0));
        let (shielded_hp, neighbor_hp) = (game.ents[house].hp, game.ents[nearby].hp);
        let profile = crate::ee2_combat::UNKNOWN_PROFILE;
        game.native_attack_impact(1, target, 80.0, 80.0, 40.0, 3, 7, 0.0, 0.0, profile, None);
        game.native_attack_impact(1, target, 80.0, 80.0, 40.0, 3, 7, 4.0, 0.0, profile, None);
        assert_eq!(game.ents[house].hp, shielded_hp);
        assert!(game.ents[nearby].hp < neighbor_hp);
        game.save();
        let mut game = Game::load(&game.savebuf).expect("active Imperialism survives load");
        assert!(game.imperialism_visible(target));
        game.local = 1;
        assert!(!game.imperialism_visible(target));
        game.local = 0;
        while game.sim_elapsed() < now + 45.0 { game.tick += 1; }
        game.tick_imperialism();
        assert!(!game.imperialism_active(house));
        game.native_attack_impact(1, target, 80.0, 80.0, 40.0, 3, 7, 0.0, 0.0, profile, None);
        assert!(game.ents[house].hp < shielded_hp);
    }
}
