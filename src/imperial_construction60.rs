//! Immediate Imperial Leader construction effects, using the active UP family masks.

use super::*;
use crate::leader58_defs::END;

const IMPERIAL: DefId = END - 1;

fn advancement(epoch: u8, target: DefId) -> Option<f32> {
    match (epoch, target) {
        (0..=4, TC) => Some(0.5),
        (5..=9, 1 | 3..=7 | 47..=52 | 56 | 59 | 60) => Some(1.0),
        _ => None,
    }
}

impl Game {
    pub(crate) fn imperial_construction_ready(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        !self.net && self.winner == 0 && actor.def == IMPERIAL
            && self.players[actor.team as usize].epoch < 10 && self.leader_power_reserve(index) >= 1.0
    }

    fn imperial_construction_fraction(&self, owner: u8, target: usize) -> Option<f32> {
        let site = &self.ents[target];
        if !site.alive || site.hp <= 0.0 || site.team != owner || !(0.0..1.0).contains(&site.built) { return None; }
        advancement(self.players[owner as usize].epoch, site.def)
    }

    pub(crate) fn imperial_construction_target(&self, owner: u8, target: usize) -> bool {
        self.imperial_construction_fraction(owner, target).is_some()
    }

    pub(crate) fn order_leader_construction(&mut self, index: usize, target: usize) {
        if !self.imperial_construction_ready(index)
            || !self.imperial_construction_target(self.ents[index].team, target) { return; }
        let handle = self.h(target);
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = O_LEADER_CONSTRUCTION;
        actor.otgt = handle;
        actor.tgt = NONE;
        actor.stuck = 0;
    }

    pub(super) fn do_leader_construction(&mut self, index: usize, caster: &mut Ent) {
        let handle = caster.otgt;
        Self::set_idle(caster);
        caster.otgt = NONE;
        if !self.imperial_construction_ready(index) { return; }
        let Some(target) = self.get(handle) else { return; };
        let Some(fraction) = self.imperial_construction_fraction(caster.team, target) else { return; };
        self.spend_unit_reserve(self.h(index), 1.0);
        let site = self.ents[target];
        let new_progress = (site.built + fraction).min(1.0);
        let applied = new_progress - site.built;
        let maximum = self.max_hp(site.team, site.def);
        self.ents[target].built = new_progress;
        self.ents[target].hp = (site.hp + maximum * 0.9 * applied).min(maximum);
        self.sync_city_ground_entry(target);
        if new_progress == 1.0 {
            if site.def == TC { self.name_completed_city(target, self.ents[target]); }
            if site.team == self.local { self.ev(14.0, site.x, site.y); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::C_LEADER_CONSTRUCTION;

    #[test]
    fn accepted_construction_powers_require_the_authored_family_and_live_owner() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        let leader = game.spawn(IMPERIAL, 0, 140.5, 140.5);
        let city = game.spawn(TC, 0, 100.5, 100.5);
        let rival = game.spawn(TC, 1, 90.5, 90.5);
        let barracks = game.spawn(BARRACKS, 0, 105.5, 105.5);
        for index in [city, rival, barracks] { game.ents[index].built = 0.25; }
        let (handle, target, foreign, other) = (game.h(leader), game.h(city), game.h(rival), game.h(barracks));
        let order = |game: &mut Game, owner, target| {
            game.apply(owner, C_LEADER_CONSTRUCTION, 0.0, 0.0, target as i32, &[handle]);
        };
        assert!(!game.imperial_construction_ready(leader));
        game.powers.as_mut().unwrap().recharge_selected(handle);
        order(&mut game, 0, foreign);
        order(&mut game, 1, target);
        order(&mut game, 0, other);
        assert_ne!(game.ents[leader].order, O_LEADER_CONSTRUCTION);
        assert_eq!(game.leader_power_reserve(leader), 1.0);
        order(&mut game, 0, target);
        assert_eq!(game.ents[leader].order, O_LEADER_CONSTRUCTION);
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        restored.tick();
        assert_eq!(restored.ents[city].built, 0.75);
        assert!(restored.leader_power_reserve(leader) < 0.01);
        assert_eq!(restored.ents[rival].built, 0.25);
        assert_eq!(restored.ents[barracks].built, 0.25);
        restored.players[0].epoch = 5;
        restored.powers.as_mut().unwrap().recharge_selected(handle);
        order(&mut restored, 0, target);
        assert_ne!(restored.ents[leader].order, O_LEADER_CONSTRUCTION);
        order(&mut restored, 0, other);
        restored.tick();
        assert_eq!(restored.ents[barracks].built, 1.0);
        assert!(restored.leader_power_reserve(leader) < 0.01);
    }
}
