//! Active UP HeavyMounted3, admitted only in an explicitly configured solo game.
use crate::defs::{Def, DefId, Wpn, A_VEH, DEFS, FORTRESS, F_TRANSPORT, HERC_FACILITY, LAND, NOW, SEA, W_MISSILE};
use crate::ee2_units::UnitRow;
use crate::ee2_up15_attack::Attack;
use crate::game::Game;

pub(crate) const ID: DefId = crate::diversion61_defs::END;
pub(crate) const START: u8 = 13;
pub(crate) const END: u8 = 15;
pub(crate) const HASH_TAG: u32 = u32::from_le_bytes(*b"HMT1");
const SAVE_TAG: &[u8; 8] = b"HMT1\x01\x00\x00\x00";

const NAME: &str = "Schwerer HERC 'Minotaurus'";
const SOURCE: &str = "HeavyMounted3";
const MISSILE: &str = "ProjectileRocketHerc";

const ROW: UnitRow = UnitRow {
    id: ID, epoch: START, until: END, source_epoch: 14, source_unit: SOURCE, source_visual: SOURCE,
    source_host: "HercFacility", identity_verified: true, model_verified: true,
    hp: Some(562.0), buildtime: Some(75.0), cost9: Some([50, 0, 0, 0, 75, 0, 0, 0, 75]),
    pop: Some(2), pop_capacity: None, sight: Some(11.0), raw_size: Some([0.65, 0.65]),
    raw_footprint: None, raw_speed: Some(1.725), gather: None, farm_rate: None, carry: None,
    farm_carry: None, repair_rate: None, damage: Some(62.0), range: Some(10.0), reload: Some(3.0),
    garrison_slots: None, generation: None, generation_resource: None, garrison_heal: None,
    garrison_attack_bonus: None, garrison_size: Some(2), fortress_garrison: true, attack_missile: Some(MISSILE),
};

pub(crate) const ATTACK: Attack = Attack {
    id: ID, epoch: START, until: END, present: true, missile: true, visual_kind: W_MISSILE,
    min_mobile: 0.0, min_immobile: 0.0, angle: 0.52, apply_time: 0.6, pack_time: 1.0,
    unpack_time: 1.0, splash: 0.0, friendly: 0.0, latent: false, auto_target: true,
    attack_on_move: false, ordnance: 0, air: false, mobile: true, projectile_speed: Some(11.0),
};

#[cfg(feature = "ee2-pack8")]
pub(crate) const PACK: crate::pack8_defs::Spec = crate::pack8_defs::Spec {
    id: ID, from: START, until: END, capable: true, pack: 1.0, unpack: 1.0,
    unpacked_idle: false, visual: SOURCE,
};

// The unit retains its identity and weapon if ownership changes to an earlier era.
pub(crate) fn row(epoch: u8) -> Option<&'static UnitRow> { (epoch < END).then_some(&ROW) }

pub(crate) fn can_garrison(host: DefId) -> bool {
    host == FORTRESS || DEFS.get(host as usize).is_some_and(|host| host.dom == SEA && host.has(F_TRANSPORT))
}

pub(crate) const fn append(existing: [Def; ID as usize]) -> [Def; ID as usize + 1] {
    let unit = Def {
        name: NAME, epoch: START, until: END, dom: LAND, building: false, size: 0.65,
        hp: 562.0, speed: 1.725, sight: 11.0, armor: A_VEH, cost: [50, 0, 0],
        time: 75.0, pop: 2, made_at: HERC_FACILITY, flags: 0,
        w: [Wpn {range: 10.0, dmg: 62.0, cd: 3.0, kind: W_MISSILE, targets: LAND | SEA, splash: 0.0}, NOW],
    };
    let mut definitions = [unit; ID as usize + 1];
    let mut index = 0;
    while index < existing.len() {
        definitions[index] = existing[index];
        index += 1;
    }
    definitions
}

impl Game {
    pub(crate) fn configure_heavy_mounted3(&mut self) -> bool {
        if self.heavy_mounted3 || self.net || self.bench != 0 || self.tick != 0 || !self.civ_setup_open ||
            !self.outbox.is_empty() || !self.inbox.is_empty() { return false; }
        self.heavy_mounted3 = true;
        true
    }

    pub(crate) fn heavy_mounted3_offer(&self, team: u8, host: DefId) -> bool {
        self.heavy_mounted3 && host == HERC_FACILITY &&
            self.players.get(team as usize).is_some_and(|player| (START..END).contains(&player.epoch))
    }
}

pub(crate) fn wrap(core: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(core.len() + 12);
    bytes.extend_from_slice(SAVE_TAG);
    bytes.extend_from_slice(&core);
    bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
    bytes
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], bool)> {
    if !bytes.starts_with(&SAVE_TAG[..4]) { return Some((bytes, false)); }
    let end = bytes.len().checked_sub(4)?;
    let expected = u32::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    if end <= SAVE_TAG.len() || !bytes.starts_with(SAVE_TAG) ||
        crate::save::checksum(&bytes[..end]) != expected { return None; }
    Some((&bytes[SAVE_TAG.len()..end], true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defs::{BARRACKS, DOCK, TC};

    #[test]
    fn solo_offer_and_archive_require_the_source_epoch_and_host() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        game.save();
        let original = game.savebuf.clone();
        assert!(!Game::load(&original).unwrap().configure_heavy_mounted3());
        assert!(game.configure_heavy_mounted3());
        assert!(!game.configure_heavy_mounted3());
        assert!(!game.unit_production_available(0, HERC_FACILITY, ID));
        assert!(game.configure_match_epoch_range(0xee01, 500, 2, 13, 14));
        assert!(game.unit_production_available(0, HERC_FACILITY, ID));
        assert!(!game.unit_production_available(0, BARRACKS, ID));
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.state_hash(), game.state_hash());
        assert!(loaded.unit_production_available(0, HERC_FACILITY, ID));
        loaded.save();
        assert_eq!(loaded.savebuf, game.savebuf);
        let mut corrupt = game.savebuf.clone();
        let header = corrupt.windows(4).position(|part| part == b"HMT1").unwrap();
        corrupt[header + 4] ^= 1;
        assert!(Game::load(&corrupt).is_none());
    }

    #[test]
    fn native_transport_scope_and_network_archive_gate_are_explicit() {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(!game.configure_heavy_mounted3());
        game.save();
        assert!(Game::load(&wrap(game.savebuf.clone())).is_none());
        assert!(can_garrison(FORTRESS));
        assert!(DEFS.iter().enumerate().any(|(i, def)| def.dom == SEA && def.has(F_TRANSPORT) && can_garrison(i as DefId)));
        assert!(!can_garrison(DOCK));
        assert!(!can_garrison(TC));
    }
}
