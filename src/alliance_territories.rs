//! Ordered territory terms attached to normal alliance offers.
//! Source accepts recipient terms first (0054999b), then proposer terms (005499b2).
//! OpenEpoch also revalidates both live owners before accepting a contract.

const MAGIC: &[u8; 4] = b"ALT1";
pub(crate) const MAX_IDS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllianceTerritories {
    count: u8,
    give_count: u8,
    ids: [u8; MAX_IDS],
}

impl AllianceTerritories {
    pub(crate) fn new(give: usize, ids: &[u32]) -> Option<Self> {
        if ids.is_empty() || ids.len() > MAX_IDS || give > ids.len() {
            return None;
        }
        let mut terms = Self { count: ids.len() as u8, give_count: give as u8, ids: [0; MAX_IDS] };
        let mut seen = 0u32;
        for (output, &id) in terms.ids.iter_mut().zip(ids) {
            let bit = 1u32.checked_shl(id)?;
            if seen & bit != 0 { return None; }
            seen |= bit;
            *output = id as u8;
        }
        Some(terms)
    }

    fn ids(&self) -> &[u8] { &self.ids[..usize::from(self.count)] }
    pub(crate) fn give(&self) -> &[u8] { &self.ids()[..usize::from(self.give_count)] }
    pub(crate) fn want(&self) -> &[u8] { &self.ids()[usize::from(self.give_count)..] }
}

pub(crate) type Pending = [Option<AllianceTerritories>; 16];

pub(crate) fn append(save: &mut Vec<u8>, pending: &Pending) {
    let start = save.len();
    for (slot, terms) in pending.iter().enumerate() {
        if let Some(terms) = terms {
            save.extend_from_slice(&[slot as u8, terms.give_count, terms.count]);
            save.extend_from_slice(terms.ids());
        }
    }
    if save.len() != start {
        save.extend_from_slice(&((save.len() - start) as u32).to_le_bytes());
        save.extend_from_slice(MAGIC);
    }
}

pub(crate) fn split(save: &[u8]) -> Option<(&[u8], Pending)> {
    let mut pending = [None; 16];
    let Some(framed) = save.strip_suffix(MAGIC) else { return Some((save, pending)); };
    let (body, length) = framed.split_at_checked(framed.len().checked_sub(4)?)?;
    let length = u32::from_le_bytes(length.try_into().ok()?) as usize;
    if length == 0 { return None; }
    let (base, mut rows) = body.split_at_checked(body.len().checked_sub(length)?)?;
    let mut next_slot = 0usize;
    while !rows.is_empty() {
        let [slot, give, count, rest @ ..] = rows else { return None; };
        let slot = usize::from(*slot);
        if slot < next_slot || slot >= pending.len() || usize::from(*count) > MAX_IDS { return None; }
        let (ids, next) = rest.split_at_checked(usize::from(*count))?;
        let mut words = [0u32; MAX_IDS];
        for (word, &id) in words.iter_mut().zip(ids) { *word = u32::from(id); }
        pending[slot] = Some(AllianceTerritories::new(usize::from(*give), &words[..ids.len()])?);
        rows = next;
        next_slot = slot + 1;
    }
    Some((base, pending))
}

pub(crate) fn hash(pending: &Pending, mix: &mut impl FnMut(u32)) {
    for (slot, terms) in pending.iter().enumerate() {
        if let Some(terms) = terms {
            mix(u32::from_le_bytes(*MAGIC));
            mix(slot as u32 | u32::from(terms.give_count) << 8 | u32::from(terms.count) << 16);
            for &id in terms.ids() { mix(u32::from(id)); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cmd::C_DIPLOMACY, defs::{FORTRESS, TC}, diplomacy::{ALLIED, NEUTRAL}, game::{Game, NONE}};

    fn settle(game: &mut Game, team: u8) -> u32 {
        let (id, &(x, y)) = game.map.tseeds.iter().enumerate()
            .find(|&(id, _)| game.towner[id] == u8::MAX).unwrap();
        game.spawn(TC, team, x, y);
        game.spawn(FORTRESS, team, x + 2.0, y);
        game.build_grid();
        assert_eq!(game.towner[id], team);
        id as u32
    }

    fn setup(capitol: bool) -> (Game, [u32; 2], [u32; 4]) {
        let mut game = Game::new(7, 2, 0, 1, 0);
        if capitol { assert!(game.configure_match_capitols(0xee81, 500, 2, 0, 0, 5, 2, 0)); }
        game.players[1].ai = false;
        game.players[2].ai = false;
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(true) && game.configure_border_treaties());
        let home = core::array::from_fn(|team| game.towner.iter().position(|&owner| owner == team as u8).unwrap() as u32);
        let ids = [settle(&mut game, 0), settle(&mut game, 0), settle(&mut game, 1), settle(&mut game, 1)];
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        game.tick = 100;
        (game, home, ids)
    }

    #[test]
    fn alliance_territories_requires_exact_acceptance_and_roundtrips_pending() {
        let (mut game, _, ids) = setup(false);
        let offered = AllianceTerritories::new(2, &ids).unwrap();
        assert!(!game.apply_diplomacy_territories(0, 1, 1, 9, 2.0, AllianceTerritories::new(1, &ids).unwrap()));
        let base = game.state_hash();
        game.apply(0, C_DIPLOMACY, 2.0, 2.0, 1 | (1 << 8) | (9 << 16), &ids);
        assert_eq!(game.diplomacy.alliance_territories[1], Some(offered));
        assert_ne!(game.state_hash(), base);
        assert_eq!(game.relation(0, 1), NEUTRAL);
        assert!(!game.apply_diplomacy(1, 0, 3, 0, 0.0));
        let mut reordered = ids;
        reordered.swap(2, 3);
        assert!(!game.apply_diplomacy_territories(1, 0, 3, 9, 2.0, AllianceTerritories::new(2, &reordered).unwrap()));
        assert!(!game.apply_diplomacy_territories(1, 0, 3, 5, 2.0, offered));
        assert!(!game.apply_diplomacy_territories(1, 0, 3, 9, 3.0, offered));
        game.save();
        assert!(game.savebuf.ends_with(MAGIC));
        let mut restored = Game::load(&game.savebuf).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        let mut malformed = game.savebuf.clone();
        let duplicate = malformed.len() - 9;
        malformed[duplicate] = malformed[duplicate - 1];
        assert!(Game::load(&malformed).is_none());
        assert!(restored.apply_diplomacy_territories(1, 0, 3, 9, 2.0, offered));
        assert_eq!(ids.map(|id| restored.towner[id as usize]), [1, 1, 0, 0]);
        assert!(restored.allied(0, 1));
        assert_eq!(restored.relation(1, 0), ALLIED);
        assert_eq!(restored.diplomacy.borders.as_ref().unwrap().term(0, 1), 2);
        assert!(restored.diplomacy.timed.as_ref().unwrap().deadline(0, 1) > 0.0);
        assert_eq!(restored.diplomacy.alliance_territories, [None; 16]);
        restored.save();
        assert!(!restored.savebuf.ends_with(MAGIC));
    }

    #[test]
    fn alliance_territories_stale_either_leg_keeps_other_regions() {
        for stale_leg in [0, 2] {
            let (mut game, _, ids) = setup(false);
            let terms = AllianceTerritories::new(2, &ids).unwrap();
            assert!(game.apply_diplomacy_territories(0, 1, 1, 0, 0.0, terms));
            let donor = game.towner[ids[stale_leg] as usize];
            assert!(game.apply_territory(donor, 2, 0, ids[stale_leg], NONE));
            assert_eq!(game.diplomacy.alliance_territories[1], None);
            let owners = ids.map(|id| game.towner[id as usize]);
            assert!(!game.apply_diplomacy_territories(1, 0, 3, 0, 0.0, terms));
            assert!(!game.apply_diplomacy(1, 0, 3, 0, 0.0));
            assert_eq!(ids.map(|id| game.towner[id as usize]), owners);
            assert_eq!(game.relation(0, 1), NEUTRAL);
        }
    }

    #[test]
    fn alliance_territories_protects_capitol_on_both_legs() {
        let (mut game, home, mut ids) = setup(true);
        assert!(game.apply_diplomacy_territories(0, 1, 1, 0, 0.0, AllianceTerritories::new(2, &ids).unwrap()));
        assert!(game.apply_diplomacy(1, 0, 4, 0, 0.0));
        game.tick += 100;
        let prior = game.state_hash();
        for (leg, capitol) in [(0, home[0]), (2, home[1])] {
            let expansion = core::mem::replace(&mut ids[leg], capitol);
            assert!(!game.apply_diplomacy_territories(0, 1, 1, 0, 0.0, AllianceTerritories::new(2, &ids).unwrap()));
            assert_eq!(game.state_hash(), prior);
            ids[leg] = expansion;
        }
    }
}
