//! Initial skirmish ceasefire selected in GameInfo, separate from negotiated diplomacy.

use crate::game::Game;

// Installed UP switch reached through 005A7D90, including extended durations.
const SECONDS: [u32; 11] = [0, 120, 300, 600, 900, 1200, 1800, 2700, 3600, 7200, 86400];
const TICKS_PER_SECOND: u32 = 20;
const SAVE_SHIFT: u32 = 12;
pub const SAVE_MASK: u32 = 0xF << SAVE_SHIFT;
pub const HASH_TAG: u32 = u32::from_le_bytes(*b"ICF1");

pub fn save_option(mode: u8) -> u32 {
    u32::from(mode) << SAVE_SHIFT
}

pub fn read_option(options: u32) -> Option<u8> {
    let mode = ((options & SAVE_MASK) >> SAVE_SHIFT) as u8;
    SECONDS.get(mode as usize).map(|_| mode)
}

impl Game {
    pub(crate) fn configure_initial_ceasefire(&mut self, mode: u32) -> bool {
        if mode >= SECONDS.len() as u32 || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() {
            return false;
        }
        self.initial_ceasefire = mode as u8;
        true
    }

    fn initial_ceasefire_end(&self) -> Option<u32> {
        let seconds = SECONDS[self.initial_ceasefire as usize];
        (seconds != 0).then_some(seconds * TICKS_PER_SECOND)
    }

    pub(crate) fn initial_ceasefire_active(&self) -> bool {
        self.initial_ceasefire_end().is_some_and(|end| self.tick <= end)
    }

    pub(crate) fn initial_ceasefire_seconds(&self) -> f32 {
        self.initial_ceasefire_end()
            .and_then(|end| end.checked_sub(self.tick))
            .map_or(-1.0, |remaining| remaining as f32 / TICKS_PER_SECOND as f32)
    }
}
