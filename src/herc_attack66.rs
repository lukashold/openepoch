//! Accepted state and local presentation of ordinary HERC Attack1 requests.
use std::collections::BTreeMap;

use crate::defs::{DefId, LIGHT_MOUNTED3};
use crate::game::{Ent, Game, BOARDED, MAXE};
use crate::heavy_mounted3_61::{ID as MINOTAURUS, START as FIRST_EPOCH};
use crate::save::checksum;

const TAG: [u8; 4] = *b"HAT1";
const VERSION: u32 = 1;
const RECORD_BYTES: usize = 20;

pub(crate) fn is_herc(definition: DefId) -> bool { matches!(definition, MINOTAURUS | LIGHT_MOUNTED3) }

#[derive(Clone, Copy)]
struct Request {
    definition: DefId,
    epoch: u8,
    variation: u16,
    revision: u32,
    start: f32,
    duration: f32,
    restored: bool,
}

#[repr(C)]
pub(crate) struct Row {
    handle: u32,
    definition: DefId,
    epoch: u8,
    restored: u8,
    revision: u32,
    start: f32,
    duration: f32,
    x: f32,
    y: f32,
    z: f32,
}

const _: [(); 32] = [(); core::mem::size_of::<Row>()];

#[repr(C)]
pub(crate) struct BeamStart {
    source: u32,
    target: u32,
    point: [f32; 3],
    target_offset_z: f32,
}

const _: [(); 24] = [(); core::mem::size_of::<BeamStart>()];

#[derive(Default)]
pub(crate) struct State {
    live: BTreeMap<u32, Request>,
    revision: u32,
    shots: BTreeMap<usize, u32>,
    pub(crate) beams: Vec<BeamStart>,
    pub(crate) rows: Vec<Row>,
    beam_target_sample: [f32; 3],
}

impl State {
    fn request(&mut self, handle: u32, mut request: Request) {
        // 008754B0 compares duration and raw variation, even if only the
        // default Attack1 KF exists. Mode 0 and state Attack1 are implicit.
        if self.live.get(&handle).is_some_and(|old| {
            old.definition == request.definition && old.epoch == request.epoch &&
                old.duration == request.duration && old.variation == request.variation
        }) { return; }
        self.revision = self.revision.wrapping_add(1);
        request.revision = self.revision;
        self.live.insert(handle, request);
    }

    pub(crate) fn forget(&mut self, handle: u32) { self.live.remove(&handle); }

    pub(crate) fn shot(&self, index: usize) -> u32 {
        self.shots.get(&index).copied().unwrap_or(0)
    }

    pub(crate) fn clear_shots(&mut self) {
        self.shots.clear();
        self.beams.clear();
    }
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

pub(crate) fn split_archive(bytes: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !bytes.starts_with(&TAG) { return Some((bytes, None)); }
    if word(bytes, 4)? != VERSION { return None; }
    let count = word(bytes, 8)? as usize;
    if !(1..=MAXE).contains(&count) { return None; }
    let end = bytes.len().checked_sub(4)?;
    let records_end = 12 + count * RECORD_BYTES;
    if records_end >= end || checksum(bytes.get(..end)?) != word(bytes, end)? { return None; }
    Some((bytes.get(records_end..end)?, Some(bytes.get(12..records_end)?)))
}

impl Game {
    fn herc_request_actor(&self, handle: u32, request: Request) -> Option<usize> {
        let index = self.get(handle)?;
        let actor = &self.ents[index];
        (is_herc(actor.def) && actor.def == request.definition && actor.hp > 0.0 &&
            actor.stuck != BOARDED && request.epoch >= FIRST_EPOCH &&
            self.players[actor.team as usize].epoch == request.epoch).then_some(index)
    }

    pub(crate) fn wrap_herc_attack(&self, inner: Vec<u8>) -> Vec<u8> {
        if inner.is_empty() || self.net || !self.guided_herc_rocket || self.herc_attack.live.is_empty() {
            return inner;
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TAG);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        let mut count = 0u32;
        for (&handle, &request) in &self.herc_attack.live {
            if self.herc_request_actor(handle, request).is_none() { continue; }
            // Attack1 and loop mode 0 are fixed by this archive version. The
            // source variant float is a one-to-one encoding of this word15.
            for value in [handle, u32::from(request.definition) | (u32::from(request.epoch) << 16),
                u32::from(request.variation), request.start.to_bits(), request.duration.to_bits()] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            count += 1;
        }
        if count == 0 { return inner; }
        bytes[8..12].copy_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&inner);
        bytes.extend_from_slice(&checksum(&bytes).to_le_bytes());
        bytes
    }

    pub(crate) fn restore_herc_attack(&mut self, records: Option<&[u8]>) -> Option<()> {
        let Some(records) = records else { return Some(()); };
        if self.net || !self.guided_herc_rocket { return None; }
        let mut previous = None;
        for row in records.chunks_exact(RECORD_BYTES) {
            let handle = word(row, 0)?;
            let identity = word(row, 4)?;
            let variation = word(row, 8)?;
            let start = f32::from_bits(word(row, 12)?);
            let duration = f32::from_bits(word(row, 16)?);
            if previous.is_some_and(|prior| handle <= prior) || identity >> 24 != 0 ||
                variation > 32767 || !(0.0..=self.sim_elapsed()).contains(&start) ||
                !duration.is_finite() || duration <= 0.0 { return None; }
            let request = Request { definition: identity as DefId, epoch: (identity >> 16) as u8,
                variation: variation as u16, revision: 0, start, duration, restored: true };
            self.herc_request_actor(handle, request)?;
            self.herc_attack.request(handle, request);
            previous = Some(handle);
        }
        Some(())
    }

    pub(crate) fn observe_herc_attack(&mut self, index: usize, actor: &Ent, start: f32, reload: f32) {
        if !self.guided_herc_rocket || !is_herc(actor.def) { return; }
        let epoch = self.players[actor.team as usize].epoch;
        if epoch < FIRST_EPOCH { return; }
        let duration = crate::power42_math::store_toward_zero(f64::from(reload) + 3.0 * f64::from(self.sim_step()));
        let handle = self.h(index);
        // 007DD666 advances the shared game stream before 008754B0
        // decides if the visual-state request is a repeat.
        let variation = self.source_birth.rng.next_word15() as u16;
        self.herc_attack.request(handle, Request { definition: actor.def, epoch, variation,
            revision: 0, start, duration, restored: false });
    }

    pub(crate) fn mark_herc_shot(&mut self, index: usize, previous_len: usize) {
        if self.events.len() != previous_len + 3 { return; }
        let Some(request) = self.herc_attack.live.get(&self.h(index)) else { return; };
        // The legacy fallback already exists. An unauthorised new row does
        // not disclose a unit definition, handle or location.
        let value = if self.can_see(self.local, index) { u32::from(request.definition) } else { u32::MAX };
        self.herc_attack.shots.insert(previous_len / 3, value);
    }

    fn herc_beam_target_offset(&self, target: usize) -> Option<f32> {
        let actor = &self.ents[target];
        if actor.hp <= 0.0 || actor.built < 1.0 || !self.can_see(self.local, target) { return None; }
        // 00871F30 stores half the cached source UnitHeight in Unit+7C.
        match actor.def {
            crate::defs::TC => Some(0.5),
            crate::defs::TOWER => Some(2.0),
            def @ (22 | 24 | 28) if self.source_production_offer(actor.team, def) == Some(true) => {
                let row = crate::ee2_units::lookup_class(def, self.players[actor.team as usize].epoch)?;
                // The ordinary Artillery ancestry supplies UnitHeight 1. This
                // is the visual midpoint, independently of missile hit geometry.
                matches!((def, row.source_unit),
                    (22, "LightArtillery1") | (24, "HeavyArtillery") | (28, "LightArtillery2"))
                    .then_some(0.5)
            }
            def => crate::rocket_herc::source_mobile_capsule_offset(def, self.players[actor.team as usize].epoch),
        }
    }

    pub(crate) fn queue_herc_beam(&mut self, index: usize, source: &Ent, target: usize) {
        if source.def != MINOTAURUS || self.herc_attack.beams.len() >= 64 || !self.can_see(self.local, index) { return; }
        let Some(target_offset_z) = self.herc_beam_target_offset(target) else { return; };
        let end = &self.ents[target];
        let point = [end.x, end.y, self.map.height(end.x, end.y).max(0.0) + target_offset_z];
        self.herc_attack.beams.push(BeamStart { source: self.h(index), target: self.h(target), point, target_offset_z });
    }

    pub(crate) fn sample_herc_beam_target(&mut self, handle: u32, alpha: f32) -> *const f32 {
        if !self.guided_herc_rocket || !(0.0..=1.0).contains(&alpha) { return core::ptr::null(); }
        let Some(index) = self.get(handle) else { return core::ptr::null(); };
        let Some(offset) = self.herc_beam_target_offset(index) else { return core::ptr::null(); };
        let actor = &self.ents[index];
        // Same interpolation and grounded root as the admitted model draw.
        let x = actor.px + (actor.x - actor.px) * alpha;
        let y = actor.py + (actor.y - actor.py) * alpha;
        self.herc_attack.beam_target_sample = [x, y, self.map.height(x, y).max(0.0) + offset];
        self.herc_attack.beam_target_sample.as_ptr()
    }

    pub(crate) fn prepare_herc_attack(&mut self) {
        let mut rows = std::mem::take(&mut self.herc_attack.rows);
        rows.clear();
        for (&handle, &request) in &self.herc_attack.live {
            let Some(index) = self.herc_request_actor(handle, request) else { continue; };
            let actor = &self.ents[index];
            if !self.can_see(self.local, index) { continue; }
            let (x, y) = (actor.x, actor.y);
            // 00868244–00868268 use the source UnitHeight midpoint (0.5).
            let z = self.map.height(x, y).max(0.0) + 0.5;
            rows.push(Row { handle, definition: request.definition, epoch: request.epoch, restored: u8::from(request.restored),
                revision: request.revision, start: request.start, duration: request.duration, x, y, z });
        }
        self.herc_attack.rows = rows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_request_keeps_original_stamp_and_raw_variant_change_restarts() {
        let mut state = State::default();
        let request = Request { definition: MINOTAURUS, epoch: FIRST_EPOCH, variation: 15,
            revision: 0, start: 10.0, duration: 3.15, restored: false };
        state.request(17, request);
        state.request(17, Request { start: 11.0, ..request });
        assert_eq!(state.live[&17].start, 10.0);
        state.request(17, Request { variation: 16, start: 12.0, ..request });
        assert_eq!(state.live[&17].start, 12.0);
        assert_eq!(state.live[&17].revision, 2);
        state.forget(17);
        assert!(state.live.is_empty());
    }
}
