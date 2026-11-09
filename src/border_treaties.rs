//! Bilateral treaty restrictions for placed actors and territory traversal.

use crate::defs::{DefId, AIR, DEFS, LAND, WORKER};
use crate::diplomacy::{Diplomacy, ALLIED};
use crate::game::{Ent, Game, BOARDED, O_IDLE};
use crate::map::{tile_of, N};

const TAG: [u8; 4] = *b"BRT1";
const HEADER_BYTES: usize = 4 + 1 + 6 + 16;

#[derive(Default)]
pub(crate) struct BorderTreaties {
    exempt: u8,
    terms: [u8; 6],
    offers: [u8; 16],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defs::LAND;
    use crate::map::{tile_of, N};

    #[test]
    fn shared_flow_fields_distinguish_allowed_and_blocked_territories() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
        game.diplomacy.borders.as_mut().unwrap().set_term(0, 1, 3);
        game.towner.fill(u8::MAX);
        game.towner[1] = 1;
        game.map.pass[0].fill(true);
        game.map.terr.fill(0);
        for y in 0..N { game.map.terr[y * N + 9] = 1; }
        let index = game.spawn(WORKER, 0, 8.5, 10.5);
        let worker = game.ents[index];
        let start = tile_of(worker.x, worker.y);
        let goal = game.key_for(LAND, 10.5, 10.5);
        let mut shared_slot = u16::MAX;
        game.map.budget = 4;
        assert_eq!(game.denied_border_territories(&worker), 2);
        assert!(game.map.flow(&mut shared_slot, goal, start, 5, 0) < 8);
        assert_eq!(game.map.flow(&mut shared_slot, goal, start, 5, 2), 255);
        assert!(game.map.flow(&mut shared_slot, goal, start, 5, 0) < 8);
        let spy = Ent { def: crate::spy46_defs::SPY, ..worker };
        assert_eq!(game.denied_border_territories(&spy), 0);
        let trapped = Ent { x: 9.5, ..worker };
        assert_eq!(game.border_egress_mask(&trapped), 0);
        let cart = Ent { def: crate::trade30_defs::TRADE_CART, ..worker };
        let merchant = Ent { def: crate::trade30_defs::MERCHANT_SHIP, ..worker };
        game.diplomacy.borders.as_mut().unwrap().set_term(0, 1, 2);
        assert_eq!(game.denied_border_territories(&cart), 0);
        assert_eq!(game.denied_border_territories(&merchant), 2);
    }

    #[test]
    fn submitted_treaty_terms_survive_an_ordinary_archive_and_maintain_exemption() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        game.tick = 100;
        assert!(!game.apply_diplomacy(0, 1, 1, 16, 0.0));
        assert!(game.apply_diplomacy(0, 1, 1, 1 | (2 << 2), 2.0));
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(restored.diplomacy.borders.as_ref().unwrap().offer(1), 2);
        assert!(restored.apply_diplomacy(1, 0, 3, 0, 0.0));
        let rules = restored.diplomacy.borders.as_mut().unwrap();
        assert_eq!(rules.term(0, 1), 2);
        assert!(rules.restricts(0, 1, 2));
        assert!(!rules.restricts(0, 1, 3));
        rules.territory_count_changed(0, 1, 0);
        assert!(!rules.restricts(0, 1, 2));
        rules.territory_count_changed(0, 0, 1);
        assert!(rules.restricts(0, 1, 2));
        restored.map.budget = 4;
        let goal = restored.key_for(LAND, 60.5, 60.5);
        let mut slot = u16::MAX;
        let _ = restored.map.flow(&mut slot, goal, tile_of(59.5, 60.5), restored.tick, 1);
        assert_ne!(slot, u16::MAX);
        restored.save();
        assert_eq!(Game::load(&restored.savebuf).unwrap().state_hash(), restored.state_hash());
    }

    #[test]
    fn issued_movement_and_relocation_respect_the_current_bilateral_rule() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
        game.diplomacy.borders.as_mut().unwrap().set_term(0, 1, 3);
        game.map.pass[0].fill(true);
        game.map.terr.fill(0);
        for y in 0..N { game.map.terr[y * N + 9] = 1; }
        let enemy_city = game.ents[..game.high].iter().find(|actor| actor.alive && actor.team == 1
            && actor.def == crate::defs::TC).unwrap();
        game.map.terr[tile_of(enemy_city.x, enemy_city.y)] = 1;
        game.territory.claims.fill(crate::game::NONE);
        game.rebuild_territories();
        assert_eq!(game.towner[1], 1);
        let outside = game.spawn(WORKER, 0, 8.5, 14.5);
        let inside = game.spawn(WORKER, 0, 9.5, 10.5);
        let spy = game.spawn(crate::spy46_defs::SPY, 0, 8.5, 18.5);
        game.order_move(&[outside], 9.5, 14.5, false);
        assert_eq!(game.ents[outside].order, O_IDLE);
        game.order_move(&[outside], 12.5, 14.5, false);
        game.order_move(&[spy], 12.5, 18.5, false);
        game.reconcile_border_positions();
        assert_eq!((game.ents[inside].ox, game.ents[inside].oy), (10.5, 10.5));
        for _ in 0..100 { game.tick(); }
        assert!(game.ents[outside].x < 9.0);
        assert!(game.ents[inside].x >= 10.0);
        assert!(game.ents[spy].x >= 11.0);
    }

    #[test]
    fn movement_target_on_impassable_ground_snaps_to_open_territory() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
        game.diplomacy.borders.as_mut().unwrap().set_term(0, 1, 3);
        game.towner[1] = 1;
        game.map.terr.fill(0);
        game.map.pass[0].fill(false);
        let closed = tile_of(11.5, 10.5);
        let open = tile_of(8.5, 10.5);
        game.map.terr[closed] = 1;
        game.map.pass[0][closed] = true;
        game.map.pass[0][open] = true;
        let worker = game.spawn(WORKER, 0, 8.5, 10.5);
        assert_eq!(game.key_for(LAND, 10.5, 10.5), closed as u32);
        game.order_move(&[worker], 10.5, 10.5, false);
        assert_eq!(game.ents[worker].fkey, open as u32);
        assert_eq!((game.ents[worker].ox, game.ents[worker].oy), (8.5, 10.5));
    }
}

impl BorderTreaties {
    fn pair(from: usize, to: usize) -> usize {
        debug_assert_ne!(from, to);
        let high = from.max(to);
        high * (high - 1) / 2 + from.min(to)
    }

    pub(crate) fn term(&self, from: usize, to: usize) -> u8 {
        self.terms[Self::pair(from, to)]
    }

    pub(crate) fn set_term(&mut self, from: usize, to: usize, term: u8) {
        self.terms[Self::pair(from, to)] = term;
    }

    pub(crate) fn offer(&self, slot: usize) -> u8 { self.offers[slot] }

    pub(crate) fn set_offer(&mut self, slot: usize, term: u8) {
        self.offers[slot] = term;
    }

    pub(crate) fn territory_count_changed(&mut self, seat: usize, old: u32, count: u32) {
        let bit = 1 << seat;
        if count > 0 { self.exempt &= !bit; }
        else if old > 0 { self.exempt |= bit; }
    }

    fn restricts(&self, mover: usize, owner: usize, purpose: u8) -> bool {
        if mover == owner || self.exempt & (1 << mover) != 0 || purpose == 4 {
            return false;
        }
        match self.term(mover, owner) {
            1 => !matches!(purpose, 2 | 3),
            2 => purpose != 3,
            3 => true,
            _ => false,
        }
    }

    pub(crate) fn valid(&self, diplomacy: &Diplomacy, territory_counts: &[u32]) -> bool {
        let seats = territory_counts.len();
        if self.exempt >> seats != 0 || territory_counts.iter().enumerate().any(|(seat, &count)|
            count > 0 && self.exempt & (1 << seat) != 0)
        {
            return false;
        }
        for from in 0..4 {
            for to in 0..4 {
                let offer = self.offer(from * 4 + to);
                if offer > 3 || (offer != 0 && (from >= seats || to >= seats || from == to ||
                    diplomacy.offers[from * 4 + to] != 2))
                {
                    return false;
                }
                if from < to {
                    let term = self.term(from, to);
                    if term > 3 || (term != 0 && (to >= seats || diplomacy.relation[from * 4 + to] != ALLIED)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + core.len());
        out.extend_from_slice(&TAG);
        out.push(self.exempt);
        out.extend_from_slice(&self.terms);
        out.extend_from_slice(&self.offers);
        out.extend(core);
        out
    }

    pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<Self>)> {
        if !bytes.starts_with(&TAG) { return Some((bytes, None)); }
        let (header, core) = bytes.split_at_checked(HEADER_BYTES)?;
        Some((core, Some(Self {
            exempt: header[4],
            terms: header[5..11].try_into().ok()?,
            offers: header[11..].try_into().ok()?,
        })))
    }

    pub(crate) fn hash(&self, mut mix: impl FnMut(u32)) {
        mix(u32::from_le_bytes(TAG));
        mix(u32::from(self.exempt));
        for term in self.terms.into_iter().chain(self.offers) { mix(u32::from(term)); }
    }
}

fn purpose(def: DefId) -> u8 {
    match def {
        WORKER => 2,
        crate::trade30_defs::TRADE_CART => 3,
        crate::spy46_defs::SPY => 4,
        _ => 0,
    }
}

fn relocation(start: [i32; 2], width: i32, height: i32, mut allowed: impl FnMut([i32; 2]) -> bool) -> Option<[i32; 2]> {
    let (x, y) = (start[0].clamp(0, width - 1), start[1].clamp(0, height - 1));
    if allowed([x, y]) { return Some([x, y]); }
    for radius in 1..width.max(height) {
        for offset in 0..=radius {
            for (point, in_bounds) in [
                ([x + radius, y + offset], x + radius < width && y + offset < height),
                ([x - offset, y + radius], x >= offset && y + radius < height),
                ([x - radius, y - offset], x >= radius && y >= offset),
                ([x + offset, y - radius], x + offset <= width && y >= radius),
            ] {
                if in_bounds && allowed(point) { return Some(point); }
            }
        }
    }
    None
}

impl Game {
    pub(crate) fn border_egress_mask(&self, actor: &Ent) -> u32 {
        let denied = self.denied_border_territories(actor);
        if denied == 0 { return 0; }
        let tile = crate::map::tile_of(actor.x, actor.y);
        let here = 1u32.checked_shl(u32::from(self.map.terr[tile])).unwrap_or(0);
        denied & !here
    }

    pub(crate) fn reconcile_border_positions(&mut self) {
        self.reconcile_border_positions_for(None);
    }

    pub(crate) fn reconcile_border_positions_for(&mut self, new_owner: Option<u8>) {
        if self.diplomacy.borders.as_ref().is_none_or(|rules| rules.terms.iter().all(|&term| term == 0)) {
            return;
        }
        let [width, height] = self.map.world_dimensions().map(|side| side as i32);
        for index in 0..self.high {
            let actor = self.ents[index];
            let def = &DEFS[actor.def as usize];
            if !actor.alive || actor.hp <= 0.0 || actor.stuck == BOARDED || actor.order != O_IDLE
                || def.building || self.trade_hidden(index, &actor) || self.air_parked(index, &actor)
            {
                continue;
            }
            let here = tile_of(actor.x, actor.y);
            if new_owner.is_some_and(|seat| {
                seat != actor.team && self.towner.get(usize::from(self.map.terr[here])) != Some(&seat)
            }) {
                continue;
            }
            if self.map.permits_territory(here, self.denied_border_territories(&actor)) {
                continue;
            }
            let denied = self.denied_border_territories_for(actor.team, 7);
            let pass = &self.map.pass[usize::from(def.dom != LAND)];
            let destination = relocation([actor.x as i32, actor.y as i32], width, height, |[x, y]| {
                if x >= width { return false; }
                let tile = y as usize * N + x as usize;
                (def.dom == AIR || pass[tile]) && self.map.permits_territory(tile, denied)
            });
            if let Some([x, y]) = destination {
                self.order_move(&[index], x as f32 + 0.5, y as f32 + 0.5, false);
            }
        }
    }

    pub(crate) fn foreign_construction_is_restricted(&self, builder: u8, owner: u8) -> bool {
        self.diplomacy.borders.as_ref().map_or(true, |rules|
            rules.restricts(builder as usize, owner as usize, 2))
    }

    pub(crate) fn denied_border_territories(&self, actor: &Ent) -> u32 {
        self.denied_border_territories_for(actor.team, purpose(actor.def))
    }

    fn denied_border_territories_for(&self, mover: u8, purpose: u8) -> u32 {
        let Some(rules) = &self.diplomacy.borders else { return 0; };
        let mover = mover as usize;
        if mover >= self.nplayers || rules.terms.iter().all(|&term| term == 0) { return 0; }
        self.towner.iter().enumerate().fold(0, |mask, (territory, &owner)| {
            let owner = owner as usize;
            let denied = owner < self.nplayers && rules.restricts(mover, owner, purpose);
            mask | (u32::from(denied) << territory)
        })
    }
}
