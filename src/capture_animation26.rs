//! Presentation request resolved from the source shared RNG and UnitModel states.

use crate::{defs::DefId, native_rng9::NativeRng};

const WORD_SCALE: f32 = f32::from_bits(0x38000100);
const STATE_ONE_LIMIT: f32 = 0.6;
const HEAVY1_EPOCHS: [u8; 8] = [0, 2, 4, 6, 8, 10, 12, 14];
const HEAVY2_EPOCHS: [u8; 5] = [1, 4, 7, 10, 13];
const SECOND_STATE_CONTEXT: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Pose {
    state: u8,
    context: u8,
    variation: u16,
    pub(crate) tick: u32,
}

fn contexts(def: DefId) -> (&'static [u8], u8) {
    match def {
        9 | 13 | 15 | 16 | 17 => (&HEAVY1_EPOCHS, 1),
        11 => (&HEAVY2_EPOCHS, 9),
        10 | 12 | 14 => (&HEAVY1_EPOCHS, 14),
        _ => (&[], 0),
    }
}

pub(crate) fn context(def: DefId, epoch: u8) -> Option<u8> {
    let (epochs, first) = contexts(def);
    epochs.iter().rposition(|&start| start <= epoch).map(|index| first + index as u8)
}

fn selected_state(word: u32) -> u8 {
    // Native compares the x87 product before an f32 store.
    if f64::from(word) * f64::from(WORD_SCALE) < f64::from(STATE_ONE_LIMIT) { 1 } else { 2 }
}

impl Pose {
    pub(crate) fn apply(current: &mut Option<Self>, rng: &mut NativeRng, context: Option<u8>, tick: u32) {
        let state = selected_state(rng.next_word15());
        let variation = rng.next_word15() as u16;
        let Some(context) = context else { return; };
        if (state == 2 && context != SECOND_STATE_CONTEXT)
            || current.is_some_and(|pose| pose.state == state && pose.context == context) {
            return;
        }
        *current = Some(Self { state, context, variation, tick });
    }

    pub(crate) fn matches_context(self, context: Option<u8>) -> bool { context == Some(self.context) }

    pub(crate) fn request(self) -> u32 { u32::from(self.variation) << 2 | u32::from(self.state) }

    pub(crate) fn packed(self) -> u32 { u32::from(self.context) << 17 | self.request() }

    pub(crate) fn restore(packed: u32, tick: u32, def: DefId) -> Option<Self> {
        let context = u8::try_from(packed >> 17).ok()?;
        let state = (packed & 3) as u8;
        let (epochs, first) = contexts(def);
        if !(first..first + epochs.len() as u8).contains(&context) || tick == 0
            || (state != 1 && !(state == 2 && context == SECOND_STATE_CONTEXT)) { return None; }
        Some(Self { state, context, variation: (packed >> 2 & 0x7fff) as u16, tick })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_state_threshold_and_random_variation_match_every_range() {
        for word in 0..=32767 {
            assert_eq!(selected_state(word), if word <= 19660 { 1 } else { 2 });
        }
        let mut rng = NativeRng::from_state(1590824933);
        let mut pose = None;
        Pose::apply(&mut pose, &mut rng, context(9, 0), 7);
        assert_eq!(rng.state(), 1791685471);
        assert_eq!(pose.unwrap().request(), 27338 << 2 | 1);
    }

    #[test]
    fn missing_and_repeated_states_consume_draws_without_restarting_pose() {
        let mut pose = None;
        let maceman = context(9, 0);
        Pose::apply(&mut pose, &mut NativeRng::from_state(12345), maceman, 7);
        let first = pose;
        let mut absent = NativeRng::from_state(3080130533);
        Pose::apply(&mut pose, &mut absent, maceman, 9);
        assert_eq!(absent.state(), 3503813471);
        assert_eq!(pose, first);
        Pose::apply(&mut pose, &mut NativeRng::from_state(0), maceman, 11);
        assert_eq!(pose, first);
        let rifleman = context(9, 10);
        Pose::apply(&mut pose, &mut NativeRng::from_state(3080130533), rifleman, 13);
        let pose = pose.unwrap();
        assert_eq!(pose.request(), 20695 << 2 | 2);
        assert_eq!(pose.tick, 13);
        assert_eq!(Pose::restore(pose.packed(), pose.tick, 9), Some(pose));
        assert_eq!(Pose::restore(pose.packed(), pose.tick, 10), None);
        assert_eq!(Pose::restore(pose.packed(), 0, 9), None);
    }
}
