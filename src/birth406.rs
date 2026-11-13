//! Typed identity at genuine actor birth; no Source404 type provider is bound.
use crate::{defs::{DefId, WORKER}, game::Game};

/// These discriminants are the unchanged BIR1 identity words, not SourceUnit IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BirthIdentity {
    Male = 0,
    Female = 1,
    NonCitizen = 2,
}

impl BirthIdentity {
    /// Historical generic spawn is explicit Male and consumes no random draw.
    pub const fn legacy_for_definition(def: DefId) -> Self {
        if def == WORKER { Self::Male } else { Self::NonCitizen }
    }

    pub const fn matches_definition(self, def: DefId) -> bool {
        (def == WORKER) == !matches!(self, Self::NonCitizen)
    }

    #[cfg(feature = "ee2-citizen9")]
    pub const fn source_word(self) -> u32 { self as u32 }
}

#[cfg(feature = "ee2-citizen9")]
impl From<crate::native_rng9::CitizenIdentity> for BirthIdentity {
    fn from(identity: crate::native_rng9::CitizenIdentity) -> Self {
        match identity {
            crate::native_rng9::CitizenIdentity::Male => Self::Male,
            crate::native_rng9::CitizenIdentity::Female => Self::Female,
        }
    }
}

impl Game {
    /// Mandatory identity input for FIRST HP. The genuine full-handle sidecar
    /// must already contain it, even though the Ent is not alive yet.
    ///
    /// Numeric HP still delegates to the existing Legacy modifiers/rank path.
    /// This is NOT a Source404 HP calculation or a SourceName/owner binding.
    /// A later provider must use this identity without applying Legacy bonuses
    /// again to an already post-Main Source HP value.
    pub(crate) fn initial_birth_hp406(&self, _index: usize, team: u8, def: DefId,
        identity: BirthIdentity) -> f32 {
        assert!(identity.matches_definition(def), "birth identity/definition mismatch");
        #[cfg(feature = "ee2-citizen9")]
        {
            let slot = self.source_birth.slots[_index];
            assert_eq!(slot.handle, self.h(_index), "birth full-handle not registered before HP");
            assert_eq!(slot.identity, identity.source_word(), "birth identity not registered before HP");
        }
        self.max_hp(team, def)
    }
}

#[cfg(all(test, feature = "ee2-citizen9"))]
mod tests {
    use super::*;
    use crate::{defs::{TC, UNIVERSITY}, game::{Ent, NOIDX}};

    #[test]
    fn identity_is_registered_before_first_hp_and_invalid_pairs_do_not_allocate() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        let rng = game.source_birth.rng.state();
        let city = game.ents[0];
        for (def, identity) in [(WORKER, BirthIdentity::Female), (WORKER, BirthIdentity::Male),
            (UNIVERSITY, BirthIdentity::NonCitizen)] {
            // initial_birth_hp406 asserts the exact handle/identity in the real
            // sidecar before reading max_hp; this is the actual spawn path.
            let index = game.spawn_with_identity(def, 0, city.x + 12.0, city.y, identity);
            assert_ne!(index, NOIDX);
            assert_eq!(game.source_identity(game.h(index)), Some(identity.source_word()));
            assert_eq!(game.ents[index].hp.to_bits(), game.max_hp(0, def).to_bits());
        }
        let free = game.free.clone(); let high = game.high;
        let hash = game.state_hash();
        for (def, identity) in [(WORKER, BirthIdentity::NonCitizen),
            (TC, BirthIdentity::Female), (TC, BirthIdentity::Male)] {
            assert_eq!(game.spawn_with_identity(def, 0, city.x, city.y, identity), NOIDX);
            assert_eq!(game.free, free); assert_eq!(game.high, high);
            assert_eq!(game.state_hash(), hash);
        }
        let male = game.spawn(WORKER, 0, city.x + 13.0, city.y);
        assert_eq!(game.source_identity(game.h(male)), Some(0));
        assert_eq!(game.source_birth.rng.state(), rng);
        assert_eq!(core::mem::size_of::<Ent>(), 176);
        assert!(game.validate_source_birth());
    }

    #[cfg(feature = "ee2-rect287-oil-economy")]
    #[test]
    fn female_cargo_keeps_retained_native_id_and_half_hp_without_a_draw() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        game.net = false;
        let city = game.ents[0];
        let host = game.spawn(UNIVERSITY, 0, city.x + 10.0, city.y);
        let female = game.spawn_with_identity(WORKER, 0, city.x + 11.0, city.y, BirthIdentity::Female);
        game.ents[female].hp *= 0.5;
        let old_handle = game.h(female);
        let host_handle = game.h(host);
        // Explicit preexisting ID fixture; unload must reuse 702 without needing
        // a World0 allocator/provider or creating a new source identity.
        game.rect_native_ids = Some(Default::default());
        let ids = game.rect_native_ids.as_mut().unwrap();
        ids.bind(host_handle, 701); ids.bind(old_handle, 702);
        let rng = game.source_birth.rng.state();
        game.order_board(&[female], host); game.tick();
        assert_eq!(game.get(old_handle), None);
        assert_eq!(game.ents[host].cargo_hp[0], 50);
        assert_eq!(game.source_birth.slots[host].cargo_females, 1);
        assert_eq!(game.rect_native_ids.as_ref().unwrap().passenger(host_handle, 0), Some(702));
        let free = core::mem::take(&mut game.free);
        game.order_unload(&[host], 0.0, 0.0);
        assert_eq!(game.ents[host].ncargo, 1);
        assert_eq!(game.rect_native_ids.as_ref().unwrap().passenger(host_handle, 0), Some(702));
        game.free = free;
        game.order_unload(&[host], 0.0, 0.0);
        assert_eq!(game.ents[host].ncargo, 0);
        let born = (0..game.high).find(|&i| game.ents[i].alive
            && game.rect_native_id(i, 0) == Some((1 << 27) | 702)).unwrap();
        assert_eq!(game.source_identity(game.h(born)), Some(1));
        assert_ne!(game.h(born), old_handle);
        assert_eq!(game.ents[born].hp.to_bits(), (game.max_hp(0, WORKER) * 0.5).to_bits());
        assert_eq!(game.source_birth.rng.state(), rng);
        assert!(game.validate_source_birth());
    }
}
