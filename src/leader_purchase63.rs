//! The two authored Economic Leader PurchaseUnit commands.

use super::{economic_leader60::ECONOMIC_LEADER, *};
use crate::{capture_math25::CaptureTribute, economy::NATIVE_IDS};

const ARCHIVE_TAG: &[u8; 8] = b"LPR1\x01\x00\x00\x00";

pub(crate) fn wrap_order_archive(core: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(core.len() + 12);
    bytes.extend_from_slice(ARCHIVE_TAG);
    bytes.extend_from_slice(&core);
    bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
    bytes
}

pub(crate) fn split_order_archive(bytes: &[u8]) -> Option<(&[u8], bool)> {
    if !bytes.starts_with(&ARCHIVE_TAG[..4]) { return Some((bytes, false)); }
    let end = bytes.len().checked_sub(4)?;
    let expected = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if end <= ARCHIVE_TAG.len() || !bytes.starts_with(ARCHIVE_TAG)
        || crate::save::checksum(&bytes[..end]) != expected { return None; }
    Some((&bytes[ARCHIVE_TAG.len()..end], true))
}

#[derive(Clone, Copy)]
enum Mode { ForeignInvestment, Capitalism }

impl Mode {
    fn at_epoch(epoch: u8) -> Option<Self> {
        match epoch {
            5..=9 => Some(Self::ForeignInvestment),
            10..=14 => Some(Self::Capitalism),
            _ => None,
        }
    }
}

impl Game {
    pub(crate) fn leader_purchase_ready(&self, index: usize) -> bool {
        let actor = self.ents[index];
        self.economic_leader_effects.is_some() && !self.net && self.winner == 0 && !self.initial_ceasefire_active()
            && actor.alive && actor.hp > 0.0 && actor.stuck != BOARDED && actor.def == ECONOMIC_LEADER
            && Mode::at_epoch(self.players[actor.team as usize].epoch).is_some() && self.leader_power_reserve(index) >= 1.0
    }

    fn purchase_mode(&self, owner: u8, target: usize) -> Option<Mode> {
        let actor = self.ents[target];
        if !actor.alive || actor.hp <= 0.0 || actor.built != 1.0 || actor.stuck == BOARDED || actor.team == owner
            || actor.team as usize >= self.nplayers || !self.can_see(owner, target)
            || self.crown_prevents_conversion(actor.team) || self.imperialism_active(target)
            || crate::diversion61_defs::slot(actor.def).is_some() {
            return None;
        }
        let mode = Mode::at_epoch(self.players[owner as usize].epoch)?;
        let valid = match mode {
            Mode::ForeignInvestment => matches!(actor.def, BARRACKS | STABLE | MILL | WAREHOUSE | AIRFIELD | DOCK | MARKET | UNIVERSITY | HOUSE | FARM)
                && !self.allied(owner, actor.team)
                && !self.owns_completed_wonder(actor.team, crate::wonders61::SPHINX),
            Mode::Capitalism => crate::ee2_combat::land_effect_target(actor.def, self.players[actor.team as usize].epoch)
                && crate::capture_math25::conversion_target(actor.def).is_some(),
        };
        valid.then_some(mode)
    }

    fn leader_purchase_price(&self, owner: u8, definition: DefId, mode: Mode) -> [f32; 9] {
        let cost = match mode {
            Mode::ForeignInvestment => self.construction_cost(owner, definition),
            Mode::Capitalism => self.unit_cost(owner, definition),
        };
        match mode {
            Mode::ForeignInvestment => cost.map(|value| 2.0 * f32::from(value)),
            Mode::Capitalism => {
                let mut native = [0_u16; 9];
                for (index, slot) in NATIVE_IDS.into_iter().enumerate() { native[slot as usize] = cost[index]; }
                let mut price = [0.0; 9];
                price[4] = (2.0 * native.into_iter().map(f64::from).sum::<f64>()) as f32;
                price
            }
        }
    }

    fn can_afford_purchase(&self, owner: u8, price: [f32; 9]) -> bool {
        price.into_iter().enumerate().all(|(slot, amount)| self.resource(owner, slot) >= amount)
    }

    pub(crate) fn leader_purchase_target(&self, owner: u8, target: usize) -> bool {
        self.purchase_mode(owner, target).is_some_and(|mode| {
            self.can_afford_purchase(owner, self.leader_purchase_price(owner, self.ents[target].def, mode))
        })
    }

    pub(crate) fn order_leader_purchase(&mut self, index: usize, target: usize) {
        if !self.leader_purchase_ready(index) || !self.leader_purchase_target(self.ents[index].team, target) { return; }
        let handle = self.h(target);
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = O_LEADER_PURCHASE;
        actor.otgt = handle;
        actor.tgt = NONE;
        actor.stuck = 0;
    }

    pub(super) fn do_leader_purchase(&mut self, index: usize, actor: &mut Ent) {
        let handle = actor.otgt;
        Self::set_idle(actor);
        actor.otgt = NONE;
        if !self.leader_purchase_ready(index) { return; }
        let Some(target) = self.get(handle) else { return; };
        let Some(mode) = self.purchase_mode(actor.team, target) else { return; };
        let price = self.leader_purchase_price(actor.team, self.ents[target].def, mode);
        self.spend_unit_reserve(self.h(index), 1.0);
        if !self.can_afford_purchase(actor.team, price) { return; }
        let bank: [f32; 9] = core::array::from_fn(|slot| self.resource(actor.team, slot));
        for (slot, amount) in price.into_iter().enumerate() {
            let charge = crate::civ8_bonus::cost_integer53(f64::from(amount));
            if charge > 0 { self.add_resource(actor.team, slot, -(charge as f32)); }
        }
        if !self.replace_converted_actor(target, actor.team, CaptureTribute::None) {
            for (slot, amount) in bank.into_iter().enumerate() { self.set_resource(actor.team, slot, amount); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::C_LEADER_PURCHASE;

    fn game(epoch: u8) -> (Game, usize) {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_civilizations([1, 8, 14, 14]));
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        assert!(game.configure_military_leader_powers());
        assert!(game.configure_economic_leader_effects());
        game.players[0].epoch = epoch;
        game.rebuild_modifiers(0, false);
        let leader = game.spawn(ECONOMIC_LEADER, 0, 80.0, 80.0);
        let handle = game.h(leader);
        game.powers.as_mut().unwrap().recharge_selected(handle);
        game.team_vis.fill(255);
        for slot in 0..9 { game.set_resource(0, slot, 2000.0); }
        (game, leader)
    }

    #[test]
    fn leader_purchase_investment_filters_the_authored_classes_and_debits_the_buyer() {
        let (mut game, leader) = game(5);
        let house = game.spawn(HOUSE, 1, 83.0, 80.0);
        let town = game.spawn(TC, 1, 92.0, 80.0);
        assert!(!game.leader_purchase_target(0, town));
        game.diplomacy.relation[1] = crate::diplomacy::ALLIED;
        game.diplomacy.relation[4] = crate::diplomacy::ALLIED;
        assert!(!game.leader_purchase_target(0, house));
        game.diplomacy.relation[1] = crate::diplomacy::NEUTRAL;
        game.diplomacy.relation[4] = crate::diplomacy::NEUTRAL;
        let price = game.leader_purchase_price(0, HOUSE, Mode::ForeignInvestment);
        let foreign = game.players[1].res;
        let (source, target) = (game.h(leader), game.h(house));
        let mut attacker = game.ents[leader];
        game.try_attack_round(leader, &mut attacker, &DEFS[ECONOMIC_LEADER as usize].w[0], house);
        game.ents[leader] = attacker;
        game.apply(0, C_LEADER_PURCHASE, 0.0, 0.0, target as i32, &[source]);
        assert_eq!(game.ents[leader].order, O_LEADER_PURCHASE);
        game.save();
        let mut game = Game::load(&game.savebuf).unwrap();
        game.team_vis.fill(255);
        game.tick();
        assert!(game.get(target).is_none());
        assert!(game.ents[..game.high].iter().any(|actor| actor.alive && actor.team == 0 && actor.def == HOUSE));
        for (slot, amount) in price.into_iter().enumerate() { assert_eq!(game.resource(0, slot), 2000.0 - amount); }
        assert_eq!(game.players[1].res, foreign);
        assert!(game.leader_power_reserve(leader) < 0.01);
        game.save();
        let restored = Game::load(&game.savebuf).expect("save after buying the building");
        assert_eq!(game.state_hash(), restored.state_hash());
    }

    #[test]
    fn leader_purchase_capitalism_checks_gold_again_when_the_order_runs() {
        let (mut game, leader) = game(10);
        let soldier = game.spawn(23, 1, 83.0, 80.0);
        let cost = game.leader_purchase_price(0, 23, Mode::Capitalism)[4];
        assert!(cost > 0.0);
        game.set_resource(0, 4, cost);
        let (source, target) = (game.h(leader), game.h(soldier));
        game.apply(0, C_LEADER_PURCHASE, 0.0, 0.0, target as i32, &[source]);
        assert_eq!(game.ents[leader].order, O_LEADER_PURCHASE);
        game.set_resource(0, 4, cost - 1.0);
        game.tick();
        assert!(game.get(target).is_some());
        assert_eq!(game.resource(0, 4), cost - 1.0);
        assert!(game.leader_power_reserve(leader) < 0.01);
        game.powers.as_mut().unwrap().recharge_selected(source);
        game.set_resource(0, 4, cost);
        game.team_vis.fill(255);
        game.apply(0, C_LEADER_PURCHASE, 0.0, 0.0, target as i32, &[source]);
        game.tick();
        assert!(game.get(target).is_none());
        assert_eq!(game.resource(0, 4), 0.0);
        assert_eq!(game.resource(0, 0), 2000.0);
    }

    #[test]
    fn leader_purchase_browser_fixture() {
        let Some(output) = std::env::var_os("OPEN_EPOCH_PURCHASE_BROWSER_FIXTURES") else { return; };
        let output = std::path::PathBuf::from(output);
        assert!(output.is_dir());
        for (epoch, definition, filename) in [(5, HOUSE, "investment.sav"), (10, WORKER, "capitalism.sav")] {
            let (mut game, leader) = game(epoch);
            if definition == WORKER {
                game.diplomacy.relation[1] = crate::diplomacy::NEUTRAL;
                game.diplomacy.relation[4] = crate::diplomacy::NEUTRAL;
            }
            let city = game.ents[..game.high].iter().find(|actor| actor.alive && actor.team == 0 && actor.def == TC).copied().unwrap();
            let x0 = city.x.floor() as i32;
            let y0 = city.y.floor() as i32;
            let nearby = (y0 - 9..=y0 + 9).flat_map(|y| (x0 - 9..=x0 + 9).map(move |x| (x, y)));
            let (x, y) = nearby.filter(|&(x, y)| x.abs_diff(x0) + y.abs_diff(y0) >= 7)
                .filter(|&(x, y)| (1..N as i32 - 1).contains(&x) && (1..N as i32 - 1).contains(&y))
                .filter(|&(x, y)| game.map.pass[0][y as usize * N + x as usize])
                .min_by_key(|&(x, y)| x.abs_diff(x0) + y.abs_diff(y0)).unwrap();
            let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
            game.ents[leader].x = x + 2.0;
            game.ents[leader].y = y + 1.0;
            game.ents[leader].px = x + 2.0;
            game.ents[leader].py = y + 1.0;
            game.ents[leader].ox = x + 2.0;
            game.ents[leader].oy = y + 1.0;
            let victim = game.spawn(definition, 1, x, y);
            assert!(game.leader_purchase_target(0, victim));
            game.save();
            assert!(Game::load(&game.savebuf).is_some());
            std::fs::write(output.join(filename), &game.savebuf).unwrap();
        }
    }
}
