//! Original Oil Rig temporary animation task and latest actor visual request.
use std::{cell::RefCell, collections::BTreeMap};

use crate::defs::OIL_RIG;
use crate::game::{Ent, Game, NONE};

pub(super) const TAG: &[u8; 4] = b"OIV1";
const IDLE: u32 = 0;
const GARRISONED: u32 = 32;
const PENDING: u32 = 0x100;
const ROW_BYTES: usize = 12;
const AUDIO_FIELDS: usize = 6;

thread_local! {
    static AUDIO_ROWS: RefCell<Vec<[u32; AUDIO_FIELDS]>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
pub(crate) struct OilVisual {
    pub(crate) state: u32,
    pub(crate) started: f32,
    pending: bool,
}

#[no_mangle]
pub extern "C" fn valley_oil_visual_state(handle: u32) -> u32 {
    super::active_game().and_then(|game| game.rect_oil_visual(handle)).map_or(NONE, |row| row.state)
}

#[no_mangle]
pub extern "C" fn valley_oil_visual_started(handle: u32) -> f32 {
    super::active_game().and_then(|game| game.rect_oil_visual(handle)).map_or(-1.0, |row| row.started)
}

#[no_mangle]
pub extern "C" fn valley_oil_audio_count() -> u32 {
    AUDIO_ROWS.with(|rows| {
        let mut rows = rows.borrow_mut();
        rows.clear();
        let Some(game) = super::active_game() else { return 0; };
        for (index, actor) in game.ents[..game.high].iter().enumerate() {
            if !game.rect_oil_visual_enabled(actor) { continue; }
            let handle = game.h(index);
            let durable = game.rect_oil_visuals.0.get(&handle);
            let visual = durable.copied().unwrap_or_else(|| OilVisual::initial(actor.ncargo != 0, game.sim_elapsed()));
            if visual.state != GARRISONED { continue; }
            rows.push([
                handle, actor.x.to_bits(), actor.y.to_bits(),
                game.map.height(actor.x, actor.y).max(0.0).to_bits(),
                visual.started.to_bits(), u32::from(durable.is_some()),
            ]);
        }
        rows.len() as u32
    })
}

#[no_mangle]
pub extern "C" fn valley_oil_audio_word(index: u32, field: u32) -> u32 {
    AUDIO_ROWS.with(|rows| {
        rows.borrow().get(index as usize).and_then(|row| row.get(field as usize)).copied().unwrap_or(NONE)
    })
}

impl OilVisual {
    fn initial(staffed: bool, now: f32) -> Self {
        Self { state: if staffed { GARRISONED } else { IDLE }, started: now, pending: false }
    }

    fn word(self) -> u32 {
        self.state | if self.pending { PENDING } else { 0 }
    }

    fn update(&mut self, now: f32) {
        if self.pending {
            self.state = if self.state == IDLE { GARRISONED } else { IDLE };
            self.started = now;
            self.pending = false;
        }
    }
}

#[derive(Default)]
pub(crate) struct OilVisuals(BTreeMap<u32, OilVisual>);

impl OilVisuals {
    fn row(&mut self, handle: u32, staffed: bool, now: f32) -> &mut OilVisual {
        self.0.entry(handle).or_insert_with(|| OilVisual::initial(staffed, now))
    }

    pub(crate) fn forget(&mut self, handle: u32) {
        self.0.remove(&handle);
    }

    pub(crate) fn section(&self) -> Option<Vec<u8>> {
        if self.0.is_empty() { return None; }
        let mut bytes = Vec::with_capacity(8 + ROW_BYTES * self.0.len());
        bytes.extend_from_slice(TAG);
        bytes.extend_from_slice(&(self.0.len() as u32).to_le_bytes());
        for (&handle, row) in &self.0 {
            for word in [handle, row.word(), row.started.to_bits()] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
        Some(bytes)
    }

    pub(crate) fn hash(&self, mix: &mut impl FnMut(u32)) {
        if self.0.is_empty() { return; }
        mix(u32::from_le_bytes(*TAG));
        mix(self.0.len() as u32);
        for (&handle, row) in &self.0 {
            mix(handle);
            mix(row.word());
            mix(row.started.to_bits());
        }
    }
}

impl Game {
    fn rect_oil_visual_enabled(&self, actor: &Ent) -> bool {
        actor.alive && actor.def == OIL_RIG && actor.built >= 1.0
            && self.rect_ai_clock.is_some() && super::rectangular_save234::is_rectangular(self)
    }

    pub(crate) fn rect_oil_boarded(&mut self, index: usize, previous_crew: u8) {
        if !self.rect_oil_visual_enabled(&self.ents[index]) { return; }
        let handle = self.h(index);
        let now = self.sim_elapsed();
        let row = self.rect_oil_visuals.row(handle, previous_crew != 0, now);
        // A duplicate is refused while the working task is present, even
        // when its final-unload completion is waiting for the Rig update.
        if row.state == IDLE { row.pending = true; }
    }

    pub(crate) fn rect_oil_unloaded(&mut self, index: usize, actor: &mut Ent, previous_crew: usize) {
        if previous_crew == 0 || actor.ncargo != 0 || !self.rect_oil_visual_enabled(actor) { return; }
        actor.carry = 0.0;
        let handle = self.h(index);
        let now = self.sim_elapsed();
        let row = self.rect_oil_visuals.row(handle, true, now);
        // Cancel a queued start before it ever presents. A mounted request
        // instead returns to Idle when the host processes its finished task.
        row.pending = row.state == GARRISONED;
    }

    pub(crate) fn update_rect_oil_visual(&mut self, index: usize, actor: &Ent) {
        if !self.rect_oil_visual_enabled(actor) { return; }
        let handle = self.h(index);
        let now = self.sim_elapsed();
        self.rect_oil_visuals.row(handle, actor.ncargo != 0, now).update(now);
    }

    pub(crate) fn rect_oil_visual(&self, handle: u32) -> Option<OilVisual> {
        let actor = &self.ents[self.get(handle)?];
        if !self.rect_oil_visual_enabled(actor) { return None; }
        Some(self.rect_oil_visuals.0.get(&handle).copied()
            .unwrap_or_else(|| OilVisual::initial(actor.ncargo != 0, self.sim_elapsed())))
    }

    pub(crate) fn restore_rect_oil_visuals(&mut self, bytes: &[u8], high: usize) -> Option<()> {
        let word = |row: &[u8], at: usize| Some(u32::from_le_bytes(row.get(at..at + 4)?.try_into().ok()?));
        if !bytes.starts_with(TAG) { return None; }
        let count = word(bytes, 4)? as usize;
        let records = bytes.get(8..)?;
        if count == 0 || count > high || records.len() != count.checked_mul(ROW_BYTES)? { return None; }
        let mut visuals = BTreeMap::new();
        let mut last = None;
        let now = self.sim_elapsed();
        for row in records.chunks_exact(ROW_BYTES) {
            let handle = word(row, 0)?;
            if last.is_some_and(|previous| handle <= previous) { return None; }
            let index = self.get(handle)?;
            if index >= high { return None; }
            let actor = &self.ents[index];
            let flags = word(row, 4)?;
            let state = flags & !PENDING;
            let pending = flags & PENDING != 0;
            let started = f32::from_bits(word(row, 8)?);
            if !self.rect_oil_visual_enabled(actor) || !matches!(state, IDLE | GARRISONED)
                || !started.is_finite() || started.is_sign_negative() || started > now
                || actor.ncargo == 0 && (state == IDLE && pending || state == GARRISONED && !pending)
            {
                return None;
            }
            visuals.insert(handle, OilVisual { state, started, pending });
            last = Some(handle);
        }
        self.rect_oil_visuals = OilVisuals(visuals);
        Some(())
    }
}
