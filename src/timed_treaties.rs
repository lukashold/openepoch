//! The negotiated duration of a bilateral alliance.

use crate::diplomacy::{Diplomacy, ALLIED};

const SAVE_TAG: [u8; 4] = *b"TMT1";
const PAIRS: usize = 6;
const OFFER_SLOTS: usize = 16;
const SAVE_BYTES: usize = 4 + 1 + PAIRS * 4 + OFFER_SLOTS;

pub(crate) struct TimedTreaties {
    pub(crate) unbreakable: bool,
    deadlines: [f32; PAIRS],
    offers: [u8; OFFER_SLOTS],
}

impl TimedTreaties {
    pub(crate) fn new(unbreakable: bool) -> Self {
        Self { unbreakable, deadlines: [0.0; PAIRS], offers: [0; OFFER_SLOTS] }
    }

    pub(crate) fn valid_minutes(minutes: u8) -> bool {
        minutes == 0 || (2..=30).contains(&minutes)
    }

    fn pair(from: usize, to: usize) -> usize {
        debug_assert_ne!(from, to);
        let high = from.max(to);
        high * (high - 1) / 2 + from.min(to)
    }

    pub(crate) fn offered_minutes(&self, slot: usize) -> u8 { self.offers[slot] }

    pub(crate) fn set_offer(&mut self, slot: usize, minutes: u8) {
        self.offers[slot] = minutes;
    }

    pub(crate) fn start(&mut self, from: usize, to: usize, minutes: u8, now: f32) {
        let deadline = if minutes == 0 { 0.0 } else { now + f32::from(minutes) * 60.0 };
        self.deadlines[Self::pair(from, to)] = deadline;
    }

    pub(crate) fn clear(&mut self, from: usize, to: usize) {
        self.deadlines[Self::pair(from, to)] = 0.0;
    }

    pub(crate) fn deadline(&self, from: usize, to: usize) -> f32 {
        self.deadlines[Self::pair(from, to)]
    }

    pub(crate) fn remaining(&self, from: usize, to: usize, now: f32) -> u32 {
        (f64::from(self.deadline(from, to)) - f64::from(now) + 0.5).max(0.0) as u32
    }

    pub(crate) fn valid(&self, diplomacy: &Diplomacy, seats: usize, now: f32) -> bool {
        for from in 0..4 {
            for to in 0..4 {
                let slot = from * 4 + to;
                let minutes = self.offers[slot];
                if !Self::valid_minutes(minutes) || (minutes != 0 &&
                    (from >= seats || to >= seats || from == to || diplomacy.offers[slot] != 2))
                {
                    return false;
                }
                if from >= to { continue; }
                let end = self.deadline(from, to);
                if end.is_sign_negative() || !end.is_finite() || (end != 0.0 &&
                    (to >= seats || diplomacy.relation[slot] != ALLIED || end <= now || end > now + 1800.0))
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::with_capacity(SAVE_BYTES + core.len());
        out.extend_from_slice(&SAVE_TAG);
        out.push(u8::from(self.unbreakable));
        for deadline in self.deadlines { out.extend_from_slice(&deadline.to_le_bytes()); }
        out.extend_from_slice(&self.offers);
        out.extend(core);
        out
    }

    pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<Self>)> {
        if !bytes.starts_with(&SAVE_TAG) { return Some((bytes, None)); }
        let (header, core) = bytes.split_at_checked(SAVE_BYTES)?;
        let unbreakable = match header[4] { 0 => false, 1 => true, _ => return None };
        let mut timed = Self::new(unbreakable);
        for (value, raw) in timed.deadlines.iter_mut().zip(header[5..5 + PAIRS * 4].chunks_exact(4)) {
            *value = f32::from_le_bytes(raw.try_into().ok()?);
        }
        timed.offers.copy_from_slice(&header[5 + PAIRS * 4..]);
        Some((core, Some(timed)))
    }

    pub(crate) fn hash(&self, mut mix: impl FnMut(u32)) {
        mix(u32::from_le_bytes(SAVE_TAG));
        mix(u32::from(self.unbreakable));
        for end in self.deadlines { mix(end.to_bits()); }
        for &minutes in &self.offers { mix(u32::from(minutes)); }
    }
}
