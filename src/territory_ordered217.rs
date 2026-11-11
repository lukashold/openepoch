//! Ordinary claim scan from the installed territory scheduler at 0071E520.

use super::*;

struct ClaimsAtRegion {
    cities: [u32; 4],
    colonies: [u32; 4],
    fortresses: u8,
    mills: u8,
}

impl Default for ClaimsAtRegion {
    fn default() -> Self {
        Self { cities: [NONE; 4], colonies: [NONE; 4], fortresses: 0, mills: 0 }
    }
}

impl ClaimsAtRegion {
    fn retention_anchor(&self, owner: u8) -> u32 {
        let seat = usize::from(owner);
        if self.cities[seat] != NONE { self.cities[seat] } else { self.colonies[seat] }
    }
}

impl Game {
    fn ordered_claim_candidates(&self) -> [ClaimsAtRegion; REGION_CAPACITY] {
        let parthenon = self.completed_wonder_owners(crate::wonders61::PARTHENON);
        let mut result = core::array::from_fn(|_| ClaimsAtRegion::default());
        for (index, actor) in self.ents[..self.high].iter().enumerate() {
            if !Self::territory_control_ready(actor) || !self.territory_seat_active(actor.team) {
                continue;
            }
            let Some(id) = self.map.region_at(self.map.tile_of(actor.x, actor.y)) else { continue; };
            let region = &mut result[id];
            let seat = usize::from(actor.team);
            let owner_bit = 1 << seat;
            match actor.def {
                TC if region.cities[seat] == NONE => region.cities[seat] = self.h(index),
                FORTRESS => region.fortresses |= owner_bit,
                MILL | WORKSHOP if parthenon & owner_bit != 0 => region.mills |= owner_bit,
                BARRACKS if parthenon & owner_bit != 0 && region.colonies[seat] == NONE => {
                    region.colonies[seat] = self.h(index);
                }
                _ => {}
            }
        }
        for region in &mut result {
            for (seat, colony) in region.colonies.iter_mut().enumerate() {
                if region.mills & (1 << seat) == 0 { *colony = NONE; }
            }
        }
        result
    }

    fn ordered_claim_owner(&self, id: usize, claim: u32) -> Option<u8> {
        disputed_owner(claim).or_else(|| acquired_owner(claim)).or_else(|| {
            self.get(claim).map(|index| &self.ents[index])
                .filter(|actor| self.valid_territory_anchor(actor, id)).map(|actor| actor.team)
        })
    }

    pub(super) fn update_ordered_territories(&mut self, restoring: bool, advance: bool) {
        if restoring {
            // TER1 describes a checkpoint. Reconstruct ownership without running a game pulse.
            let mut owners = [TERR_NONE; REGION_CAPACITY];
            for (id, owner) in owners.iter_mut().enumerate().take(self.map.tseeds.len()) {
                *owner = self.ordered_claim_owner(id, self.territory.claims[id]).unwrap_or(TERR_NONE);
            }
            self.terr_dirty |= owners != self.towner;
            self.towner = owners;
            for seat in 0..self.nplayers {
                self.players[seat].terr = owners.iter().filter(|&&owner| usize::from(owner) == seat).count() as u32;
            }
            return;
        }

        let candidates = self.ordered_claim_candidates();
        let hill = self.koth_territory();
        let command_counts = (!advance).then(|| self.players.each_ref().map(|player| player.terr));
        // The generated map's compact IDs match its original bucket order.
        for (id, candidates) in candidates.iter().enumerate().take(self.map.tseeds.len()) {
            let prior = self.territory.claims[id];
            let previous_phase = if disputed_owner(prior).is_some() { 2 }
                else if prior == NONE || acquired_owner(prior).is_some() { 0 } else { 1 };
            if let Some((_, index)) = hill.filter(|&(territory, _)| territory == id) {
                let actor = self.ents[index];
                let (owner, anchor) = if self.valid_territory_anchor(&actor, id) {
                    (actor.team, self.h(index))
                } else {
                    (TERR_NONE, NONE)
                };
                self.territory.claims[id] = anchor;
                self.write_ordered_region_owner(id, owner, advance);
                continue;
            }

            let incumbent = if prior == NONE { TERR_NONE }
                else { self.ordered_claim_owner(id, prior).unwrap_or(self.towner[id]) };
            let retained = self.territory_seat_active(incumbent);
            if !advance && retained && (disputed_owner(prior).is_some() || acquired_owner(prior).is_some()) {
                continue;
            }
            let phase = if retained { 2 } else { 0 };
            let owner = if retained { incumbent } else { TERR_NONE };
            let anchor = if retained { candidates.retention_anchor(incumbent) } else { NONE };
            if anchor != NONE {
                // Retention is evaluated before considering any rival, regardless of seat number.
                let prior_still_usable = previous_phase == 1
                    && self.get(prior).is_some_and(|index| self.ents[index].def == TC
                        && self.ents[index].team == incumbent && Self::territory_control_ready(&self.ents[index])
                        && self.valid_territory_anchor(&self.ents[index], id));
                self.territory.claims[id] = if prior_still_usable { prior } else { anchor };
                self.write_ordered_region_owner(id, incumbent, advance);
                self.terr_dirty |= previous_phase != 1;
                continue;
            }

            self.territory.claims[id] = if retained { DISPUTED_CLAIM + u32::from(incumbent) } else { NONE };
            self.write_ordered_region_owner(id, owner, advance);
            self.terr_dirty |= previous_phase != phase;
            if !advance { continue; }
            for seat in 0..self.nplayers {
                let team = seat as u8;
                let city = candidates.cities[seat];
                let colony = candidates.colonies[seat];
                let city_qualifies = city != NONE && (self.players[seat].terr == 0
                    || candidates.fortresses & (1 << seat) != 0 || self.territory_adjacent_owned(team, id));
                if !city_qualifies && colony == NONE { continue; }
                // 0071D9B0 changes ownership but leaves the phase written by the preceding refresh.
                let marker = if phase == 2 { DISPUTED_CLAIM } else { ACQUIRED_CLAIM };
                self.territory.claims[id] = marker + u32::from(team);
                self.write_ordered_region_owner(id, team, true);
                break;
            }
        }
        // Existing negotiated swaps and direct transfers settle their whole command together.
        if let (Some(old_counts), Some(borders)) = (command_counts, &mut self.diplomacy.borders) {
            for (seat, (&old, player)) in old_counts.iter().zip(&self.players).enumerate().take(self.nplayers) {
                if player.alive && old != player.terr {
                    borders.territory_count_changed(seat, old, player.terr);
                }
            }
        }
        self.prune_alliance_territories();
    }

    fn write_ordered_region_owner(&mut self, id: usize, owner: u8, territory_pulse: bool) {
        let previous = self.towner[id];
        if previous == owner { return; }
        self.towner[id] = owner;
        self.terr_dirty = true;
        for seat in [previous, owner] {
            let index = usize::from(seat);
            if index >= self.nplayers { continue; }
            let count = self.towner.iter().filter(|&&region_owner| region_owner == seat).count() as u32;
            let prior_count = self.players[index].terr;
            self.players[index].terr = count;
            // 0071A340 releases an inactive incumbent directly. No 00548A10 exemption write.
            if territory_pulse && owner != TERR_NONE && self.territory_seat_active(seat) {
                if let Some(borders) = &mut self.diplomacy.borders {
                    borders.territory_count_changed(index, prior_count, count);
                }
            }
        }
        if owner != TERR_NONE {
            self.name_territory_acquisition(id, owner);
            if territory_pulse { self.reconcile_border_positions_for(Some(owner)); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Game {
        let mut game = Game::new(7, 3, 0, 1, 0);
        for index in 0..game.high {
            if game.ents[index].def == TC { game.kill(index); }
        }
        for player in &mut game.players {
            player.terr = 0;
            player.ai = false;
        }
        game.territory.claims.fill(NONE);
        game.towner.fill(TERR_NONE);
        game.map.territory_neighbors.fill(0);
        game
    }

    fn place(game: &mut Game, id: usize, seat: u8, def: DefId) -> usize {
        let (x, y) = game.map.tseeds[id];
        assert_eq!(game.map.terr[tile_of(x, y)] as usize, id);
        let index = game.spawn(def, seat, x, y);
        assert_ne!(index, NOIDX);
        index
    }

    fn established(game: &mut Game, id: usize, seat: u8) -> usize {
        let index = place(game, id, seat, TC);
        game.territory.claims[id] = game.h(index);
        game.towner[id] = seat;
        game.players[seat as usize].terr += 1;
        index
    }

    #[test]
    fn high_region_claims_swaps_batch_gifts_and_unassigned_tiles() {
        let mut game = scenario();
        game.map.tseeds.resize(43, (0.0, 0.0));
        for (id, x) in [(31, 80), (32, 90), (42, 100)] {
            game.map.tseeds[id] = (x as f32 + 0.5, 80.5);
            game.map.terr[80 * N + x] = id as u8;
        }
        established(&mut game, 0, 0);
        established(&mut game, 1, 1);
        established(&mut game, 31, 0);
        established(&mut game, 32, 1);
        game.map.territory_neighbors[42] = 1 << 32;
        game.map.territory_neighbors[32] = 1 << 42;
        let last = place(&mut game, 42, 1, TC);
        let unassigned = tile_of(110.5, 80.5);
        game.map.terr[unassigned] = u8::MAX;
        let no_region = game.spawn(TC, 2, 110.5, 80.5);
        assert_ne!(no_region, NOIDX);
        game.rebuild_territories();
        assert_eq!(game.towner[42], 1);
        assert_eq!(acquired_owner(game.territory.claims[42]), Some(1));
        assert_eq!(game.players[2].terr, 0);
        game.rebuild_territories();
        assert_eq!(game.territory.claims[42], game.h(last));
        assert_eq!(game.territory_owner_at(unassigned), TERR_NONE);
        game.vis.fill(255);
        game.terr_tex();
        assert_eq!(&game.terr_rgba[unassigned * 4..unassigned * 4 + 4], &[255, 0, 0, 255]);
        let mut json = String::new();
        game.append_territory_ui(&mut json);
        assert!(json.contains("\"id\":42,"));
        assert!(!json.contains("\"id\":255,"));

        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        use crate::cmd::C_TERRITORY;
        game.apply(0, C_TERRITORY, 31.0, 42.0, 1 | (1 << 8), &[]);
        assert!(game.territory.offers[1].active);
        game.apply(1, C_TERRITORY, -1.0, -1.0, 2 << 8, &[]);
        assert_eq!((game.towner[31], game.towner[42]), (1, 0));

        game.apply(1, C_TERRITORY, -1.0, -1.0, 5 << 8, &[32, 32]);
        assert_eq!(game.towner[32], 1);
        game.apply(1, C_TERRITORY, -1.0, -1.0, 5 << 8, &[32, 31]);
        assert_eq!((game.towner[31], game.towner[32]), (0, 0));
        game.apply(0, C_TERRITORY, 42.0, -1.0, 1, &[]);
        assert_eq!(game.towner[42], 1);
        assert_eq!(game.ents[last].team, 1);
    }

    #[test]
    fn nature_keeps_its_phase_across_save_and_direct_anchor_sync() {
        let mut game = scenario();
        established(&mut game, 4, 3);
        let _higher_first = place(&mut game, 0, 1, TC);
        let lower = place(&mut game, 0, 0, TC);
        game.rebuild_territories();
        assert_eq!(game.towner[0], 0);
        assert_eq!(acquired_owner(game.territory.claims[0]), Some(0));
        assert!(!game.territory_is_disputed(0));
        assert!(!game.territory_valid(0, 3, 0, 0, NONE));
        game.sync_territory_anchors();
        assert_eq!(acquired_owner(game.territory.claims[0]), Some(0));
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.state_hash(), game.state_hash());
        loaded.save();
        assert!(loaded.savebuf == game.savebuf);
        assert_eq!(acquired_owner(loaded.territory.claims[0]), Some(0));
        loaded.rebuild_territories();
        assert_eq!(loaded.territory.claims[0], loaded.h(lower));
        assert!(loaded.territory_valid(0, 3, 0, 0, NONE));
    }

    #[test]
    fn pending_or_inactive_lower_seats_yield_to_the_next_complete_claimant() {
        for inactive in [false, true] {
            let mut game = scenario();
            let low = place(&mut game, 0, 0, TC);
            place(&mut game, 0, 1, TC);
            place(&mut game, 0, 2, TC);
            if inactive { game.players[0].alive = false; } else { game.ents[low].built = 0.99; }
            game.rebuild_territories();
            assert_eq!(acquired_owner(game.territory.claims[0]), Some(1));
        }
    }

    #[test]
    fn a_second_paid_city_foundation_cannot_confirm_an_earlier_claim() {
        let mut game = scenario();
        place(&mut game, 0, 0, TC);
        game.rebuild_territories();
        assert_eq!(acquired_owner(game.territory.claims[0]), Some(0));
        let (x, y) = (16..240).step_by(4).flat_map(|y| (16..240).step_by(4).map(move |x| (x, y)))
            .find(|&(x, y)| game.placement_geometry_error(0, TC, x, y, 0) == 0).unwrap();
        let before = game.players[0].res;
        let site = game.place_with_rotation(TC, x, y, 0, 0);
        assert_ne!(site, NOIDX);
        assert_eq!(game.ents[site].built, 0.0);
        assert_ne!(game.players[0].res, before);
        assert_eq!(acquired_owner(game.territory.claims[0]), Some(0));
    }

    #[test]
    fn ordered_peer_save_requires_network_and_negotiated_dispute() {
        let mut solo = scenario();
        solo.network_ordered_territories = true;
        solo.save();
        assert!(Game::load(&solo.savebuf).is_none());

        let mut peer = Game::new(7, 0, 0, 2, 0);
        peer.network_ordered_territories = true;
        peer.save();
        assert!(Game::load(&peer.savebuf).is_none());
        peer.network_disputed_territories = true;
        peer.save();
        let mut restored = Game::load(&peer.savebuf).unwrap();
        assert!(restored.ordered_territory_rules());
        restored.save();
        assert!(restored.savebuf == peer.savebuf);
    }

    #[test]
    fn ordered_region_pass_only_cascades_toward_later_ids() {
        for reversed in [false, true] {
            let mut game = scenario();
            let [base, next, last] = if reversed { [2, 1, 0] } else { [0, 1, 2] };
            established(&mut game, base, 0);
            place(&mut game, next, 0, TC);
            place(&mut game, last, 0, TC);
            game.map.territory_neighbors[base] = 1 << next;
            game.map.territory_neighbors[next] = (1 << base) | (1 << last);
            game.map.territory_neighbors[last] = 1 << next;
            game.rebuild_territories();
            assert_eq!(acquired_owner(game.territory.claims[next]), Some(0));
            assert_eq!(game.towner[last], if reversed { TERR_NONE } else { 0 });
            game.rebuild_territories();
            assert_eq!(game.towner[last], 0);
            assert_eq!(acquired_owner(game.territory.claims[last]), reversed.then_some(0));
        }
    }

    #[test]
    fn recovered_incumbent_and_per_seat_parthenon_have_original_priority() {
        let mut game = scenario();
        game.towner[0] = 2;
        game.players[2].terr = 1;
        game.territory.claims[0] = DISPUTED_CLAIM + 2;
        place(&mut game, 0, 0, TC);
        let replacement = place(&mut game, 0, 2, TC);
        game.rebuild_territories();
        assert_eq!(game.territory.claims[0], game.h(replacement));

        let mut game = scenario();
        assert!(game.configure_priest_skirmish());
        place(&mut game, 1, 0, crate::wonders61::PARTHENON);
        let colony = place(&mut game, 0, 0, BARRACKS);
        place(&mut game, 0, 0, MILL);
        place(&mut game, 0, 1, TC);
        game.rebuild_territories();
        assert_eq!(acquired_owner(game.territory.claims[0]), Some(0));
        game.rebuild_territories();
        assert_eq!(game.territory.claims[0], game.h(colony));
    }

    #[test]
    fn valid_last_loss_flips_closed_border_while_inactive_release_does_not() {
        for (inactive, retains_dispute) in [(false, false), (false, true), (true, false)] {
            let mut game = scenario();
            assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
            let prior = established(&mut game, 0, 0);
            place(&mut game, 0, 1, TC);
            if retains_dispute {
                game.territory.claims[1] = DISPUTED_CLAIM;
                game.towner[1] = 0;
                game.players[0].terr += 1;
            }
            let borders = game.diplomacy.borders.as_mut().unwrap();
            borders.set_term(0, 1, 3);
            borders.territory_count_changed(1, 1, 0);
            game.kill(prior);
            game.players[0].alive = !inactive;
            game.rebuild_territories();
            assert_eq!(game.towner[0], 1);
            assert_eq!(game.territory_is_disputed(0), !inactive);
            assert_eq!(game.foreign_construction_is_restricted(0, 1), inactive || retains_dispute);
            assert!(game.foreign_construction_is_restricted(1, 0));
        }
    }
}
