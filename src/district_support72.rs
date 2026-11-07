//! Saved support-row demand and the per-player HotSpots manager deadline.

use crate::hotspot_rule66::{add_chop, mul_chop};

const OLD_TAG: u32 = u32::from_le_bytes(*b"HSD1");
const TAG: u32 = u32::from_le_bytes(*b"HSD2");
const FAILURE_TAG: u32 = u32::from_le_bytes(*b"HSD3");
const POINT_TAG: [u8; 4] = *b"HSP2";
const SEATS: usize = 4;
const SLOTS: usize = 4 * SEATS;
pub(crate) const ROWS: usize = 3;

#[derive(Clone)]
pub(super) struct SupportPlan {
    demand: [u8; ROWS * SLOTS],
    deadline: [f32; SEATS],
    failed_placement: [u32; ROWS * SLOTS],
}

impl Default for SupportPlan {
    fn default() -> Self {
        Self { demand: [0; ROWS * SLOTS], deadline: [0.0; SEATS], failed_placement: [0; ROWS * SLOTS] }
    }
}

impl SupportPlan {
    pub(super) fn armed(&self, seat: usize) -> bool {
        self.deadline[seat] > 0.0
    }

    pub(super) fn count(&mut self, point: usize, seat: usize, row: usize, previous: f32, draw: impl FnOnce() -> u32) -> u8 {
        let refresh = self.armed(seat) && previous >= self.deadline[seat];
        let count = &mut self.demand[row * SLOTS + point * SEATS + seat];
        if *count == 0 || refresh {
            *count = 1 + (draw() % 2) as u8;
        }
        *count
    }

    pub(super) fn finish_pass(&mut self, seat: usize, previous: f32, draw: impl FnOnce() -> u32) {
        if previous >= self.deadline[seat] {
            self.deadline[seat] = refreshed_deadline(previous, draw());
        }
    }

    pub(super) fn placement(&mut self, point: usize, seat: usize, row: usize, found: bool) {
        let failures = &mut self.failed_placement[row * SLOTS + point * SEATS + seat];
        *failures = if found { 0 } else { failures.wrapping_add(1) };
    }

    pub(super) fn valid_for(&self, points: usize, active_ai: impl Fn(usize) -> bool) -> bool {
        self.deadline.iter().enumerate().all(|(seat, &time)| time == 0.0 || active_ai(seat))
            && self.demand.iter().enumerate().all(|(slot, &value)| {
                let seat = slot % SEATS;
                let armed = self.armed(seat);
                (self.failed_placement[slot] == 0 || value != 0 && armed)
                    && (value == 0 || (slot % SLOTS / SEATS < points && active_ai(seat)
                        && (slot < SLOTS || armed)))
            })
    }

    fn tag(&self) -> Option<u32> {
        if self.failed_placement.iter().any(|&count| count != 0) { Some(FAILURE_TAG) }
        else if self.deadline.iter().any(|&time| time > 0.0) { Some(TAG) }
        else if self.demand.iter().any(|&value| value != 0) { Some(OLD_TAG) }
        else { None }
    }

    fn samples(&self, tag: u32) -> &[u8] {
        if tag == OLD_TAG { &self.demand[..SLOTS] } else { &self.demand }
    }

    pub(super) fn hash(&self, mut mix: impl FnMut(u32)) {
        let Some(tag) = self.tag() else { return; };
        mix(tag);
        for row in self.samples(tag).chunks_exact(4) {
            mix(u32::from_le_bytes(row.try_into().unwrap()));
        }
        if tag != OLD_TAG {
            for time in self.deadline { mix(time.to_bits()); }
        }
        if tag == FAILURE_TAG {
            for count in self.failed_placement { mix(count); }
        }
    }

    pub(super) fn wrap(&self, points: Vec<u8>) -> Vec<u8> {
        let Some(tag) = self.tag() else { return points; };
        let times = if tag == OLD_TAG { 0 } else { self.deadline.len() * 4 };
        let failures = if tag == FAILURE_TAG { self.failed_placement.len() * 4 } else { 0 };
        let mut bytes = Vec::with_capacity(12 + self.samples(tag).len() + times + failures + points.len());
        bytes.extend_from_slice(&tag.to_le_bytes());
        let version: u32 = match tag { OLD_TAG => 1, TAG => 2, _ => 3 };
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(self.samples(tag));
        if tag != OLD_TAG {
            for time in self.deadline { bytes.extend_from_slice(&time.to_le_bytes()); }
        }
        if tag == FAILURE_TAG {
            for count in self.failed_placement { bytes.extend_from_slice(&count.to_le_bytes()); }
        }
        bytes.extend_from_slice(&points);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }

    pub(super) fn split(bytes: &[u8]) -> Option<(&[u8], Self)> {
        let mut plan = Self::default();
        let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
        let (version, samples) = match word(0) {
            Some(OLD_TAG) => (1, SLOTS),
            Some(TAG) => (2, ROWS * SLOTS),
            Some(FAILURE_TAG) => (3, ROWS * SLOTS),
            _ => return Some((bytes, plan)),
        };
        if word(4)? != version { return None; }
        let mut header = 8 + samples;
        plan.demand[..samples].copy_from_slice(bytes.get(8..header)?);
        if plan.demand.iter().any(|&value| value > 2) { return None; }
        if version >= 2 {
            for time in &mut plan.deadline {
                let bits = word(header)?;
                *time = f32::from_bits(bits);
                if bits != 0 && (!time.is_finite() || *time < 30.0) { return None; }
                header += 4;
            }
        }
        if version == 3 {
            for count in &mut plan.failed_placement {
                *count = word(header)?;
                header += 4;
            }
        }
        if plan.tag()? != word(0)? { return None; }
        let end = bytes.len().checked_sub(4)?;
        let inner = bytes.get(header..end)?;
        if !inner.starts_with(&POINT_TAG) || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
        Some((inner, plan))
    }
}

fn refreshed_deadline(previous: f32, word: u32) -> f32 {
    // 004E33F6..004E342B: 53-bit, chopped operations, then one f32 store.
    let span = 270.0 * f64::from(word);
    let delay = mul_chop(span, f64::from(f32::from_bits(0x3800_0100)));
    let exact = add_chop(add_chop(delay, f64::from(previous)), 30.0);
    let nearest = exact as f32;
    if f64::from(nearest) > exact { f32::from_bits(nearest.to_bits() - 1) } else { nearest }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_deadline_vectors() {
        assert_eq!(refreshed_deadline(0.0, 0), 30.0);
        assert_eq!(refreshed_deadline(0.0, 0x7fff).to_bits(), 0x4395_ffff);
        assert_eq!(refreshed_deadline(1.0, 1795628641 >> 16 & 0x7fff).to_bits(), 0x4380_6243);
        assert_eq!(refreshed_deadline(120.15, 2252622528 >> 16 & 0x7fff).to_bits(), 0x4323_5df0);
    }

    #[test]
    fn pending_pass_retains_rows_and_early_visits_do_not_draw() {
        let mut state = SupportPlan::default();
        assert_eq!(state.count(0, 1, 0, 0.0, || 1), 2);
        state.finish_pass(1, 0.0, || 0);
        let one_before = f32::from_bits(30f32.to_bits() - 1);
        assert_eq!(state.count(0, 1, 0, one_before, || panic!("early row draw")), 2);
        state.finish_pass(1, one_before, || panic!("early manager draw"));
        state.finish_pass(1, 30.0, || 0);
        assert_eq!(state.count(0, 1, 0, 30.0, || panic!("pending row must be retained")), 2);
        assert_eq!(state.count(0, 1, 1, 30.0, || 0), 1);
        assert_eq!(state.count(0, 1, 0, 60.0, || 0), 1);
        state.finish_pass(1, 60.0, || 0);
        assert_eq!(state.deadline[1], 90.0);
    }

    #[test]
    fn failed_placement_has_an_exact_optional_row_archive() {
        let mut state = SupportPlan::default();
        let points = b"HSP2-canonical-inner".to_vec();
        state.count(0, 1, 0, 0.0, || 1);
        state.finish_pass(1, 0.0, || 0);
        let before = state.wrap(points.clone());
        assert!(before.starts_with(b"HSD2"));
        state.placement(0, 1, 0, false);
        state.placement(0, 1, 0, false);
        let archived = state.wrap(points.clone());
        assert!(archived.starts_with(b"HSD3"));
        let (inner, mut restored) = SupportPlan::split(&archived).unwrap();
        assert_eq!(inner, points);
        assert_eq!(restored.failed_placement[1], 2);
        assert!(restored.valid_for(3, |seat| seat == 1));
        assert!(!restored.valid_for(3, |seat| seat == 0));
        assert_eq!(restored.wrap(points.clone()), archived);
        restored.placement(0, 1, 0, true);
        assert_eq!(restored.wrap(points), before);
    }
}
