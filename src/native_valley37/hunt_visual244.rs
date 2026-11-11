//! Attack1 visual state requested by the native Citizen Attack callback.
use super::{active_game, hunt_order241::CITIZEN_RELOAD, world0_hunt242::hunt_target};
use crate::game::Game;
use std::collections::BTreeMap;

const HIGH15_SCALE: f32 = f32::from_bits(0x3800_0100);
const STATE_SPAN_FRAMES: f64 = 3.0;
const ATTACK1_END: f32 = f32::from_bits(0x3fd5_5556);
const SWING_KEY: f32 = 1.25;
const SWING_CUE_BIT: u32 = 1 << 31;

#[derive(Clone, Copy)]
pub(super) struct HuntVisual {
    start: f32,
    span: f32,
    sample: f32,
}

#[derive(Clone, Copy)]
pub(super) struct HuntCursor {
    frame: u32,
    scene_time: f32,
    identity: Option<[u32; 3]>,
    elapsed: f32,
}

impl HuntCursor {
    fn observe(prior: Option<Self>, visual: Option<HuntVisual>, world: f32, scene_time: f32, frame: u32) -> (Self, bool) {
        let identity = visual.map(HuntVisual::words);
        if let Some(prior) = prior.filter(|prior| prior.frame == frame && prior.identity == identity) {
            return (prior, false);
        }
        let (elapsed, swing) = match visual {
            Some(visual) => {
                if let Some(prior) = prior.filter(|prior| {
                    scene_time >= prior.scene_time
                        && (prior.identity == identity || prior.frame.wrapping_add(1) == frame)
                }) {
                    let previous = if prior.identity == identity { prior.elapsed } else { 0.0 };
                    // 00605AB1 supplies the drawable's accepted scene-time difference.
                    let delta = (f64::from(scene_time) - f64::from(prior.scene_time)) as f32;
                    // 005FDF10 advances the key cursor at speed 1.0. The pose
                    // applies the separate 005FCC20 multiplier to the elapsed time.
                    let elapsed = (f64::from(previous) + f64::from(delta)) as f32;
                    (elapsed, previous <= SWING_KEY && SWING_KEY < elapsed)
                } else {
                    // Reconstructed objects recover the committed state's time.
                    // Audio keys already crossed before reconstruction stay silent.
                    (visual.elapsed(world), false)
                }
            }
            None => (0.0, false),
        };
        (Self { frame, scene_time, identity, elapsed }, swing)
    }
}

impl HuntVisual {
    fn start(time: f32, step: f32, high15: u32) -> Self {
        let span = (f64::from(CITIZEN_RELOAD) + f64::from(step) * STATE_SPAN_FRAMES) as f32;
        let sample = (f64::from(high15) * f64::from(HIGH15_SCALE)) as f32;
        Self { start: time, span, sample }
    }

    fn expired(self, now: f32) -> bool {
        f64::from(now) - f64::from(self.start) >= f64::from(self.span)
    }

    fn same_request(self, other: Self) -> bool {
        self.span.to_bits() == other.span.to_bits() && self.sample.to_bits() == other.sample.to_bits()
    }

    fn deadline(self) -> u32 {
        ((f64::from(self.start) + f64::from(CITIZEN_RELOAD)) as f32).to_bits()
    }

    fn phase(self, render_time: f32, intrinsic: f32) -> Option<f32> {
        if !render_time.is_finite() || !intrinsic.is_finite() || intrinsic <= 0.0 { return None; }
        Some(self.phase_at_elapsed(self.elapsed(render_time), intrinsic))
    }

    fn scale(self, intrinsic: f32) -> f32 {
        // 005FCE8B stores the Animator playback multiplier as an F32.
        (f64::from(intrinsic) / f64::from(self.span)) as f32
    }

    fn elapsed(self, time: f32) -> f32 {
        (f64::from(time) - f64::from(self.start)).max(0.0) as f32
    }

    fn phase_at_elapsed(self, elapsed: f32, intrinsic: f32) -> f32 {
        let clip_time = (f64::from(elapsed) * f64::from(self.scale(intrinsic))) as f32;
        (f64::from(clip_time) / f64::from(intrinsic)).clamp(0.0, 1.0) as f32
    }

    fn words(self) -> [u32; 3] {
        [self.start.to_bits(), self.span.to_bits(), self.sample.to_bits()]
    }

    fn read(words: [u32; 3]) -> Option<Self> {
        let [start, span, sample] = words.map(f32::from_bits);
        let high15 = (f64::from(sample) / f64::from(HIGH15_SCALE)).round();
        if !start.is_finite() || start.is_sign_negative() || !(CITIZEN_RELOAD..=4.0).contains(&span)
            || !(0.0..=32767.0).contains(&high15)
            || ((high15 * f64::from(HIGH15_SCALE)) as f32).to_bits() != sample.to_bits()
        {
            return None;
        }
        Some(Self { start, span, sample })
    }
}

impl Game {
    pub(super) fn begin_rect_hunt_visual(&mut self, actor: usize, high15: u32) {
        let handle = self.h(actor);
        let visual = HuntVisual::start(self.sim_elapsed(), self.sim_step(), high15);
        if let Some(population) = &mut self.rect_population {
            let state = population.hunt_visuals.entry(handle).or_insert(visual);
            // 0083FF70 leaves the timestamp and observer untouched on an equal request.
            if !state.same_request(visual) { *state = visual; }
        }
    }

    pub(super) fn clear_rect_hunt_visual(&mut self, actor: usize) {
        let handle = self.h(actor);
        if let Some(population) = &mut self.rect_population { population.hunt_visuals.remove(&handle); }
    }

    fn hunt_visual_state(&self, handle: u32) -> Option<HuntVisual> {
        let actor = &self.ents[self.get(handle)?];
        hunt_target(actor)?;
        let visual = *self.rect_population.as_ref()?.hunt_visuals.get(&handle)?;
        (visual.deadline() <= actor.otile).then_some(visual)
    }

    fn hunt_visual(&self, handle: u32) -> Option<HuntVisual> {
        let visual = self.hunt_visual_state(handle)?;
        (!visual.expired(self.sim_elapsed())).then_some(visual)
    }

    pub(crate) fn rect_hunt_visual_active(&self, actor: usize) -> Option<bool> {
        self.hunt_visual_state(self.h(actor)).map(|visual| !visual.expired(self.sim_elapsed()))
    }

    pub(crate) fn prune_rect_hunt_visuals(&mut self) {
        let Some(population) = &mut self.rect_population else { return; };
        let actors = &self.ents[..self.high];
        population.hunt_visuals.retain(|&handle, visual| {
            actors.get((handle & 0xffff) as usize).is_some_and(|actor| {
                u32::from(actor.gen) == handle >> 16 && hunt_target(actor).is_some()
                    && actor.otile >= visual.deadline()
            })
        });
    }

    pub(super) fn rect_hunt_visual_rows(&self) -> impl Iterator<Item = [u32; 4]> + '_ {
        self.rect_population.iter().flat_map(|population| population.hunt_visuals.keys())
            .filter_map(|&handle| {
                let [start, span, sample] = self.hunt_visual_state(handle)?.words();
                Some([handle, start, span, sample])
            })
    }

    pub(super) fn valid_rect_hunt_visual_rows(&self, high: usize) -> bool {
        let Some(population) = &self.rect_population else { return true; };
        let now = self.sim_elapsed();
        population.hunt_visuals.iter().all(|(&handle, visual)| {
            let index = (handle & 0xffff) as usize;
            index < high && self.h(index) == handle && hunt_target(&self.ents[index]).is_some()
                && visual.start <= now && visual.deadline() <= self.ents[index].otile
        })
    }
}

pub(super) fn read_visual_rows(bytes: &[u8]) -> Option<BTreeMap<u32, HuntVisual>> {
    let (count, rows) = bytes.split_at_checked(4)?;
    let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
    if count == 0 || count > crate::game::MAXE || count.checked_mul(16)? != rows.len() { return None; }
    let mut visuals = BTreeMap::new();
    for row in rows.chunks_exact(16) {
        let word = |i: usize| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap());
        let handle = word(0);
        if visuals.last_key_value().is_some_and(|(&prev, _)| prev >= handle) { return None; }
        visuals.insert(handle, HuntVisual::read([word(1), word(2), word(3)])?);
    }
    Some(visuals)
}

#[no_mangle]
pub extern "C" fn render_rect_hunt_state_word(handle: u32, field: usize) -> u32 {
    active_game().and_then(|game| game.hunt_visual(handle))
        .and_then(|visual| visual.words().get(field).copied()).unwrap_or(u32::MAX)
}

#[no_mangle]
pub extern "C" fn render_rect_hunt_phase(handle: u32, render_time: f32, intrinsic_clip_seconds: f32) -> f32 {
    active_game().and_then(|game| game.hunt_visual(handle))
        .and_then(|visual| visual.phase(render_time, intrinsic_clip_seconds)).unwrap_or(f32::NAN)
}

#[no_mangle]
pub extern "C" fn render_rect_hunt_visual_step(handle: u32, world_time: f32, scene_time: f32,
    intrinsic_clip_seconds: f32, frame: u32, commit: u32) -> u32
{
    if active_game().is_none() || commit > 1 || intrinsic_clip_seconds != ATTACK1_END
        || !world_time.is_finite() || !scene_time.is_finite() || scene_time < 0.0
    {
        return u32::MAX;
    }
    let game = crate::g();
    let Some(index) = game.get(handle) else { return u32::MAX; };
    if game.ents[index].def != crate::defs::WORKER { return u32::MAX; }
    let visual = game.hunt_visual(handle);
    let Some(population) = &mut game.rect_population else { return u32::MAX; };
    // Browser render passes hold views into WASM memory. This fixed slot must
    // not allocate or grow that memory while those views are in use.
    let cursor_slot = &mut population.hunt_visual_cursors[index];
    let previous = cursor_slot.filter(|(seen, _)| *seen == handle).map(|(_, cursor)| cursor);
    let (cursor, swing) = HuntCursor::observe(previous, visual, world_time, scene_time, frame);
    if commit == 1 {
        *cursor_slot = Some((handle, cursor));
    }
    visual.map_or(u32::MAX, |visual| {
        let phase = visual.phase_at_elapsed(cursor.elapsed, intrinsic_clip_seconds);
        phase.to_bits() | if swing && commit == 1 { SWING_CUE_BIT } else { 0 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_nearest53_start_payload_and_scale() {
        let state = HuntVisual::start(0.0, 0.125, (1013904223_u32 >> 16) & 0x7fff);
        assert_eq!(state.words(), [0, 0x4058_0000, 0x3ef1_b9e3]);
        assert_eq!(state.scale(f32::from_bits(0x3fd5_5556)).to_bits(), 0x3efc_d6eb);
        assert_eq!(state.phase(0.0, 1.6666667), Some(0.0));
        assert_eq!(state.phase(4.0, 1.6666667), Some(1.0));
        assert!(state.same_request(HuntVisual { start: 2.0, ..state }));
        assert!(!state.same_request(HuntVisual { sample: 0.0, ..state }));
        assert_eq!(HuntVisual::read(state.words()).unwrap().words(), state.words());
        assert!(HuntVisual::read([0, 0x4058_0000, 0x3ef1_b9e4]).is_none());
    }

    #[test]
    fn visual_key_has_unscaled_upper_exclusive_cursor_and_no_cold_replay() {
        let visual = HuntVisual::start(4.0, 0.05, 10);
        let (idle, _) = HuntCursor::observe(None, None, 3.0, 0.0, 40);
        let (boundary, swing) = HuntCursor::observe(Some(idle), Some(visual), 5.25, 1.25, 41);
        assert_eq!(boundary.elapsed, SWING_KEY);
        assert!(!swing);
        let (reentry, swing) = HuntCursor::observe(Some(boundary), Some(visual), 5.25, 1.25, 41);
        assert_eq!(reentry.elapsed, SWING_KEY);
        assert!(!swing);
        let adjacent = f32::from_bits(SWING_KEY.to_bits() + 1);
        let (crossing, swing) = HuntCursor::observe(Some(boundary), Some(visual), 5.3, adjacent, 42);
        assert_eq!(crossing.elapsed, adjacent);
        assert!(swing);
        assert!(visual.phase_at_elapsed(crossing.elapsed, ATTACK1_END) < 0.4);
        assert!(!HuntCursor::observe(Some(crossing), Some(visual), 5.4, 1.4, 43).1);
        assert!(!HuntCursor::observe(None, Some(visual), 5.4, 0.1, 50).1);
        assert!(HuntCursor::observe(Some(boundary), Some(visual), 5.4, 1.4, 50).1);
        let changed = HuntVisual::start(5.3, 0.05, 11);
        assert!(!HuntCursor::observe(Some(boundary), Some(changed), 5.4, 1.4, 50).1);
    }
}
