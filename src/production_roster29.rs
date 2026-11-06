//! Production windows from the active UP tech tree and Main upgrade sets.
//! Keep the established definition aliases for saved jobs and wire commands.

use crate::defs::DefId;
use crate::game::Game;

pub(crate) const SAVE_FLAG: u32 = 1 << 4;

impl Game {
    pub(crate) fn configure_peer_production_roster(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() || !self.inbox.is_empty() {
            return false;
        }
        self.production_roster_enabled = enabled;
        true
    }

    /// None leaves unrelated definitions and older match modes alone.
    pub(crate) fn source_production_offer(&self, team: u8, unit: DefId) -> Option<bool> {
        if !self.production_roster_enabled { return None; }
        let epochs = match unit {
            // Stable. The mounted nodes are retired in Main11.
            18 => 1..3,
            19 => 3..10,
            21 => 1..10,
            23 => 10..15,
            26 => 10..14,
            46 => 14..15,
            // Mill. Keep a single offer for LightArtillery1 across all epochs.
            20 => 1..9,
            22 => 0..15,
            24 => 9..15,
            25 => 10..14,
            27 => return Some(false),
            28 => 14..15,
            // Dock. WarGalley retires in Main6, sailing warships in Main11.
            29 => 0..5,
            30 | 31 | 58 => 5..10,
            32 => 10..15,
            33 | 34 => 10..12,
            35 => 12..15,
            36 => 12..14,
            42 => 5..15,
            45 => 14..15,
            // Airport.
            37 => 10..12,
            38 => 10..13,
            39 => 12..14,
            40 => 13..15,
            44 => 14..15,
            _ => return None,
        };
        Some(epochs.contains(&self.players[team as usize].epoch))
    }
}
