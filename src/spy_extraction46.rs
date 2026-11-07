//! The Spy's Extraction marker and ordinary point-move child.

use super::*;
use crate::spy46_defs::SPY;
use std::collections::BTreeMap;

const MAGIC: u32 = u32::from_le_bytes(*b"EXT1");
const DURATION: f32 = 15.0;
const MARKER_RADIUS: f64 = 0.6_f32 as f64;
const SEARCH_STEP: f32 = 0.3;
const SHAPE_SIZE: f64 = 0.125;

#[derive(Clone, Copy)]
pub(crate) struct Marker {
    pub point: [f32; 3],
    remaining: f32,
    visual_until: f32,
    travelling: bool,
}

#[derive(Default)]
pub(crate) struct Extraction {
    pub markers: BTreeMap<u32, Marker>,
}

impl Extraction {
    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.markers.len() as u32);
        for (&handle, marker) in &self.markers {
            write(handle);
            for x in marker.point { write(x.to_bits()); }
            write(marker.remaining.to_bits());
            write(u32::from(marker.travelling));
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        if self.markers.is_empty() { return; }
        write(MAGIC);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.markers.is_empty() || core.is_empty() { return core; }
        let mut data = Vec::with_capacity(core.len() + 24 * self.markers.len() + 20);
        for word in [MAGIC, 1, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }

    pub(crate) fn retire(&mut self, handle: u32) { self.markers.remove(&handle); }
}

fn visit_ring(radius: f32, angle: f64, mut fit: impl FnMut(f64, f64) -> Option<[f32; 3]>) -> Option<[f32; 3]> {
    let radius = f64::from(radius);
    let pi = f64::from(core::f32::consts::PI);
    let steps = (((radius / f64::from(SEARCH_STEP) * pi + 0.5) as i32) - 1).max(4);
    for index in 0..=steps {
        let offset = pi * f64::from(index) / f64::from(steps);
        let directions: &[f64] = if index == 0 || index == steps { &[angle + offset] } else { &[angle + offset, angle - offset] };
        for &heading in directions {
            let (sin, cos) = heading.sin_cos();
            if let Some(point) = fit(radius * cos, radius * sin) { return Some(point); }
        }
    }
    None
}

fn follows_marker(actor: &Ent, marker: &Marker) -> bool {
    actor.order == O_MOVE && actor.ox == marker.point[0] && actor.oy == marker.point[1]
}

impl Game {
    pub(crate) fn spy_extract_ready(&self, index: usize) -> bool {
        let actor = self.ents[index];
        actor.def == SPY && actor.alive && actor.hp > 0.0 && self.winner == 0
            && (self.spy_ready(index) || self.spy_extraction.markers.contains_key(&self.h(index)))
    }

    pub(crate) fn spy_marker_at(&self, index: usize, x: f32, y: f32) -> bool {
        if !x.is_finite() || !y.is_finite() { return false; }
        self.spy_extraction.markers.get(&self.h(index)).is_some_and(|marker| {
            let dx = f64::from(x) - f64::from(marker.point[0]);
            let dy = f64::from(y) - f64::from(marker.point[1]);
            let dz = f64::from(self.map.height(x, y)) - f64::from(marker.point[2]);
            dx * dx + dy * dy + dz * dz <= MARKER_RADIUS * MARKER_RADIUS
        })
    }

    fn spy_ground_fit(&self, x: f64, y: f64) -> Option<[f32; 3]> {
        let bound = (N - 1) as f64;
        let half = SHAPE_SIZE * 0.5;
        if x - half < 0.0 || y - half < 0.0 || x + half >= bound || y + half >= bound { return None; }
        for ox in [-half, half] {
            for oy in [-half, half] {
                if !self.map.pass[0][tile_of((x + ox) as f32, (y + oy) as f32)] { return None; }
            }
        }
        let store = crate::power42_math::store_toward_zero;
        let (x, y) = (store(x), store(y));
        Some([x, y, self.map.height(x, y)])
    }

    pub(crate) fn order_spy_extraction(&mut self, index: usize, context: Option<(f32, f32)>) {
        if !self.spy_extract_ready(index) || context.is_some_and(|(x, y)| !self.spy_marker_at(index, x, y)) { return; }
        let handle = self.h(index);
        let current = self.ents[index];
        let marker = if let Some(marker) = self.spy_extraction.markers.get(&handle) {
            *marker
        } else {
            let angle = f64::from(self.source_birth.rng.next_word15()) * f64::from(0.000_191_753_45_f32);
            let radius = crate::power42_math::store_toward_zero(10.0 + SHAPE_SIZE.hypot(SHAPE_SIZE) * 0.5);
            let Some(point) = visit_ring(radius, angle, |x, y| self.spy_ground_fit(f64::from(current.x) + x, f64::from(current.y) + y)) else {
                return;
            };
            self.spend_unit_reserve(handle, 1.0);
            Marker { point, remaining: DURATION, visual_until: self.sim_elapsed() + DURATION, travelling: true }
        };
        self.order_move(&[index], marker.point[0], marker.point[1], false);
        let actor = &mut self.ents[index];
        actor.ox = marker.point[0];
        actor.oy = marker.point[1];
        self.spy_extraction.markers.insert(handle, Marker { travelling: true, ..marker });
    }

    pub(crate) fn reconcile_spy_extraction(&mut self) {
        let ents = &self.ents;
        for (&handle, marker) in &mut self.spy_extraction.markers {
            if marker.travelling && !follows_marker(&ents[(handle & 0xffff) as usize], marker) { marker.travelling = false; }
        }
    }

    pub(crate) fn tick_spy_extraction(&mut self, handle: u32) {
        let Some(marker) = self.spy_extraction.markers.get_mut(&handle) else { return; };
        if marker.remaining > 0.0 { marker.remaining -= 1.0; }
        else { self.spy_extraction.retire(handle); }
    }

    pub(super) fn complete_spy_extraction(&mut self, index: usize, actor: &mut Ent) {
        let handle = self.h(index);
        let Some(marker) = self.spy_extraction.markers.get(&handle) else { return; };
        if !marker.travelling || !follows_marker(actor, marker) { return; }
        self.spy_extraction.retire(handle);
        let owned = self.ents[..self.high].iter().enumerate().filter(|&(other, unit)| other != index
            && unit.alive && unit.hp > 0.0 && unit.team == actor.team && unit.built >= 1.0 && unit.stuck != BOARDED);
        let Some((_, home)) = owned.clone().find(|&(_, unit)| unit.def == TC).or_else(|| owned.clone().next()) else { return; };
        let extent = if DEFS[home.def as usize].building { Self::building_half(home) } else { DEFS[home.def as usize].rad() };
        let mover = SHAPE_SIZE.hypot(SHAPE_SIZE) * 0.5;
        let mut radius = crate::power42_math::store_toward_zero(f64::from(extent) + mover + 2.0);
        let max = crate::power42_math::store_toward_zero(f64::from(extent) + mover + 3.5);
        while radius <= max {
            if let Some([x, y, _]) = visit_ring(radius, -f64::from(core::f32::consts::FRAC_PI_2),
                |x, y| self.spy_ground_fit(f64::from(home.x) + x, f64::from(home.y) + y)) {
                actor.x = x;
                actor.y = y;
                actor.px = x;
                actor.py = y;
                actor.vx = 0.0;
                actor.vy = 0.0;
                return;
            }
            radius = crate::power42_math::store_toward_zero(f64::from(radius) + f64::from(SEARCH_STEP));
        }
    }

    pub(crate) fn spy_marker_visual(&self, ordinal: usize) -> Option<(u32, [f32; 3], f32)> {
        self.spy_extraction.markers.iter().filter_map(|(&handle, marker)| {
            let index = self.get(handle)?;
            (self.ents[index].team == self.local).then_some((handle, marker.point, marker.visual_until))
        }).nth(ordinal)
    }

    pub(crate) fn restore_spy_extraction(&mut self, fields: Option<&[u8]>) -> Option<()> {
        let Some(fields) = fields else { return Some(()); };
        if self.powers.is_none() || self.net { return None; }
        let mut words = fields.chunks_exact(4);
        let mut read = || words.next().and_then(|chunk| Some(u32::from_le_bytes(chunk.try_into().ok()?)));
        let count = read()? as usize;
        if count == 0 || fields.len() != count.checked_mul(24)?.checked_add(4)? { return None; }
        let mut state = Extraction::default();
        for _ in 0..count {
            let handle = read()?;
            let index = self.get(handle)?;
            let actor = self.ents[index];
            let point = [f32::from_bits(read()?), f32::from_bits(read()?), f32::from_bits(read()?)];
            let remaining = f32::from_bits(read()?);
            let travelling = match read()? { 0 => false, 1 => true, _ => return None };
            if actor.def != SPY || actor.hp <= 0.0 || !point.iter().all(|n| n.is_finite())
                || !point[..2].iter().all(|n| (0.0..(N - 1) as f32).contains(n))
                || !(0.0..=DURATION).contains(&remaining) || remaining.fract() != 0.0 {
                return None;
            }
            let marker = Marker { point, remaining, visual_until: self.sim_elapsed() + remaining, travelling };
            if travelling && !follows_marker(&actor, &marker) || state.markers.insert(handle, marker).is_some() { return None; }
        }
        self.spy_extraction = state;
        Some(())
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let len = word(8)? as usize;
    let end = data.len().checked_sub(4)?;
    let start = end.checked_sub(len)?;
    if word(4)? != 1 || len == 0 || start < 40 || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}
