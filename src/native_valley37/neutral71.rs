//! The first original regular citizen impact for the authenticated, open two-seat fixture.
use crate::defs::WORKER;
use crate::diplomacy::NEUTRAL;
use crate::game::{Ent, Game, O_ATTACK};

pub(crate) const RAW_ATTACK_MODIFIER: u32 = 0x400;
pub(crate) const FORCE_ATTACK: f32 = RAW_ATTACK_MODIFIER as f32;

impl Game {
    pub(super) fn set_valley_neutral_opening(&mut self) -> bool {
        let Some(fixture) = self.valley_fixture.as_mut() else { return false; };
        if fixture.query(6) != 1 || self.net || self.nplayers != 2 || self.humans != 1 ||
            self.local != 0 || !self.players[1].ai {
            return false;
        }
        fixture.neutral_opening = true;
        self.diplomacy.relation[1] = NEUTRAL;
        self.diplomacy.relation[4] = NEUTRAL;
        true
    }

    pub(crate) fn valley_neutral_opening(&self) -> bool {
        self.valley_fixture.as_ref().is_some_and(|fixture| fixture.neutral_opening)
    }

    pub(crate) fn valid_valley_neutral_opening(&self) -> bool {
        self.valley_neutral_opening() && !self.net && self.humans == 1 && self.nplayers == 2 &&
            self.local == 0 && self.relation(0, 1) == NEUTRAL && self.relation(1, 0) == NEUTRAL &&
            self.diplomacy.sharing[1] == 0 && self.diplomacy.sharing[4] == 0
    }

    pub(crate) fn close_valley_neutral_opening(&mut self) {
        if let Some(fixture) = &mut self.valley_fixture { fixture.neutral_opening = false; }
    }

    pub(crate) fn admits_valley_neutral_target(&self, attacker: &Ent, target: &Ent) -> bool {
        self.valid_valley_neutral_opening() && self.winner == 0 && !self.initial_ceasefire_active() &&
            attacker.team == 0 && target.team == 1 && attacker.def == WORKER && target.def == WORKER &&
            self.players[0].epoch == 0 && self.players[1].epoch == 0 &&
            attacker.alive && attacker.hp > 0.0 && target.alive && target.hp > 0.0 &&
            self.diplomacy.offers[1] == 0 && self.diplomacy.offers[4] == 0
    }

    pub(crate) fn valley_order_keeps_target(&self, attacker: &Ent, index: usize) -> bool {
        attacker.order == O_ATTACK && attacker.otgt == self.h(index) &&
            self.admits_valley_neutral_target(attacker, &self.ents[index])
    }

    pub(crate) fn commit_valley_regular_impact(&mut self, attacker: &Ent, index: usize) {
        if !self.valley_order_keeps_target(attacker, index) { return; }
        self.apply_valley_regular_hostility();
        self.close_valley_neutral_opening();
    }
}
