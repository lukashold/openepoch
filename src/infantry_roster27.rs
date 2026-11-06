//! The Barracks retains its three source production families across epochs.
//! Existing role IDs remain the saved/wire definitions, including their aliases.

use crate::defs::DefId;
use crate::game::Game;

pub(crate) const SAVE_FLAG: u32 = 1 << 3;

impl Game {
    pub(crate) fn configure_peer_infantry_roster(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() || !self.inbox.is_empty() {
            return false;
        }
        self.infantry_roster_enabled = enabled;
        true
    }

    /// None preserves the normal production rule for another profile or unit.
    pub(crate) fn source_infantry_offer(&self, team: u8, unit: DefId) -> Option<bool> {
        if !self.infantry_roster_enabled { return None; }
        let epochs = match unit {
            9 | 10 => 0..2,
            11 => 1..15,
            12 => 2..4,
            13 => 2..6,
            14 => 4..15,
            15 => 6..10,
            16 => 10..12,
            17 => 12..15,
            _ => return None,
        };
        Some(epochs.contains(&self.players[team as usize].epoch))
    }
}
