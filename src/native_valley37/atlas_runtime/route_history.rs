//! The sequence of admitted edits to the authenticated VAP1 opening atlas.
use super::{stationary_dimensions, BuildingEvent, BuildingPose, RuntimeAtlas, DIMENSIONS};
use crate::game::{Game, MAXE, NONE};
use std::collections::BTreeSet;

const MAX_EVENTS: usize = 256;
pub(in crate::native_valley37) const EVENT_BYTES: usize = 24;

#[derive(Default)]
pub(in crate::native_valley37) struct Journal(Vec<BuildingEvent>);

impl Journal {
    pub(super) fn has_room(&self) -> bool { self.0.len() < MAX_EVENTS }

    pub(super) fn push(&mut self, event: BuildingEvent) { self.0.push(event); }

    pub(in crate::native_valley37) fn len(&self) -> usize { self.0.len() }

    pub(in crate::native_valley37) fn encode_into(&self, output: &mut Vec<u8>) {
        for event in &self.0 {
            let words = match *event {
                BuildingEvent::Add(handle, pose) => [1, handle, u32::from(pose.def), pose.x.to_bits(), pose.y.to_bits(), pose.yaw.to_bits()],
                BuildingEvent::Remove(handle) => [2, handle, 0, 0, 0, 0],
            };
            for word in words { output.extend_from_slice(&word.to_le_bytes()); }
        }
    }

    pub(in crate::native_valley37) fn decode(bytes: &[u8], count: usize) -> Option<Self> {
        if count > MAX_EVENTS || bytes.len() != count * EVENT_BYTES { return None; }
        let mut seen = BTreeSet::new();
        let mut live = BTreeSet::new();
        let mut events = Vec::with_capacity(count);
        for row in bytes.chunks_exact(EVENT_BYTES) {
            let [kind, handle, def, x, y, yaw] = std::array::from_fn(|i| {
                u32::from_le_bytes(row[4 * i..4 * i + 4].try_into().unwrap())
            });
            if handle == NONE || (handle as u16 as usize) >= MAXE { return None; }
            let event = match kind {
                1 => {
                    let def = u16::try_from(def).ok()?;
                    let [x, y, yaw] = [x, y, yaw].map(f32::from_bits);
                    if stationary_dimensions(def).is_none() || !yaw.is_finite()
                        || !(0.0..DIMENSIONS[0] as f32).contains(&x) || !(0.0..DIMENSIONS[1] as f32).contains(&y)
                        || !seen.insert(handle) || !live.insert(handle) { return None; }
                    BuildingEvent::Add(handle, BuildingPose { def, x, y, yaw })
                }
                2 if [def, x, y, yaw] == [0; 4] && live.remove(&handle) => BuildingEvent::Remove(handle),
                _ => return None,
            };
            events.push(event);
        }
        Some(Self(events))
    }
}

impl RuntimeAtlas {
    pub(in crate::native_valley37) fn route_history_for_save(&self, game: &Game) -> Option<(&Journal, u32)> {
        (self.history.len() as u32 == self.generation && self.current(game)).then_some((&self.history, self.generation))
    }

    pub(in crate::native_valley37) fn replay_route_history(&mut self, game: &Game, history: Journal) -> Option<()> {
        if self.generation != 0 || !self.history.0.is_empty() { return None; }
        for change in history.0 { self.update(game, change)?; }
        self.current(game).then_some(())
    }
}
