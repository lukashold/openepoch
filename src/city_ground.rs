//! Source-backed CityManager radius rule for the completed-TC registration adapter.
//!
//! Native 008153F0 stores an unscaled radius, 00814040 only increases it, and
//! 00815610 destroys the record. A boolean urban query needs the maximum radius
//! over this TC's life, not the native timestamp/owner tie-breaking history.
use crate::defs::TC;
use crate::game::{Ent, Game, MAXE, NONE};

pub const CITY_MAGIC: u32 = 0x3144_4743; // CGD1
pub const CITY_END: u32 = 0x4544_4743; // CGDE
const VERSION: u32 = 1;
const HEADER: usize = 12;
const FOOTER: usize = 8;

#[derive(Clone, Copy)]
pub struct CityBand { pub handle: u32, pub radius: f32 }
pub const EMPTY: CityBand = CityBand { handle: NONE, radius: 0.0 };

fn word(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

/// Recognize an explicit footer before legacy trailer inference. Neither old
/// entity layouts nor an arbitrary large legacy trailer are interpreted as CGD.
pub fn split_save(b: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    // All accepted pre-CGD layouts end no later than the full ECO1/DIP1/TER1
    // payload. Legacy final stock/research/tech-point bits may equal CGDE;
    // interpreting those as a footer would reject an otherwise valid old save.
    if b.len()<36 { return Some((b,None)); }
    let high=word(b,32)? as usize;
    if high>MAXE { return None; }
    let legacy_end=36+core::mem::size_of::<[crate::game::Player;4]>()+
        core::mem::size_of::<Ent>()*high+crate::map::NN*5+236+
        crate::diplomacy::DIP_BYTES+crate::territory::TERR_BYTES;
    if b.len()<=legacy_end { return Some((b,None)); }
    if b.len() < FOOTER || word(b, b.len()-4)? != CITY_END { return Some((b, None)); }
    let size = word(b, b.len()-8)? as usize;
    if size < HEADER+FOOTER || size > HEADER+FOOTER+MAXE*8 { return None; }
    let start = b.len().checked_sub(size)?;
    if start!=legacy_end { return None; }
    let c = &b[start..];
    if word(c,0)? != CITY_MAGIC || word(c,4)? != VERSION { return None; }
    let count = word(c,8)? as usize;
    if count > MAXE || size != HEADER+FOOTER+count*8 { return None; }
    Some((&b[..start], Some(&c[..size-FOOTER])))
}

impl Game {
    pub(crate) fn city_ground_active(&self, e: &Ent) -> bool {
        e.alive && e.def == TC && e.built >= 1.0 && e.hp > 0.0 && (e.team as usize) < self.nplayers
    }
    fn city_ground_profile(&self, e: &Ent) -> f32 {
        7.0 + (self.players[e.team as usize].epoch.min(14)/3) as f32
    }
    pub(crate) fn observe_city_ground(&mut self, i: usize, e: Ent) {
        if !self.city_ground_active(&e) { self.city_ground[i] = EMPTY; return; }
        let handle = ((e.gen as u32)<<16) | i as u32;
        let radius = self.city_ground_profile(&e);
        let old = self.city_ground[i];
        self.city_ground[i] = CityBand { handle, radius: if old.handle == handle {old.radius.max(radius)} else {radius} };
    }
    pub(crate) fn sync_city_ground_entry(&mut self, i: usize) { self.observe_city_ground(i, self.ents[i]); }
    pub(crate) fn sync_city_ground(&mut self) {
        for i in 0..self.high { self.sync_city_ground_entry(i); }
    }
    /// Pure fallback to the present source profile also keeps controlled fixture
    /// edits well-defined. Lifetime history itself is written only by sim hooks.
    pub(crate) fn city_ground_radius(&self, i: usize) -> f32 {
        let e = &self.ents[i];
        if !self.city_ground_active(e) { return 0.0; }
        let c = self.city_ground[i];
        let base = self.city_ground_profile(e);
        if c.handle == self.h(i) { c.radius.max(base) } else { base }
    }
    pub fn original_urban_at(&self, x: f32, y: f32) -> bool {
        if !x.is_finite() || !y.is_finite() { return false; }
        self.ents[..self.high].iter().enumerate().any(|(i,city)| {
            let radius = self.city_ground_radius(i);
            if radius == 0.0 { return false; }
            let distance = (x-city.x).abs()+(y-city.y).abs();
            // 00814340 rounds the L1 sum to float before an x87 comparison.
            (distance as f64) < (radius as f64)*(1.4142135381698608_f32 as f64)
        })
    }
    pub(crate) fn write_city_ground(&self, b: &mut Vec<u8>) {
        let start = b.len();
        let count = self.ents[..self.high].iter().filter(|e| self.city_ground_active(e)).count();
        for v in [CITY_MAGIC, VERSION, count as u32] { b.extend_from_slice(&v.to_le_bytes()); }
        for i in 0..self.high {
            let radius = self.city_ground_radius(i);
            if radius == 0.0 { continue; }
            b.extend_from_slice(&self.h(i).to_le_bytes());
            b.extend_from_slice(&radius.to_le_bytes());
        }
        let size = b.len()-start+FOOTER;
        b.extend_from_slice(&(size as u32).to_le_bytes());
        b.extend_from_slice(&CITY_END.to_le_bytes());
    }
    pub(crate) fn read_city_ground(&mut self, c: Option<&[u8]>) -> Option<()> {
        self.city_ground.fill(EMPTY);
        let Some(c) = c else { self.sync_city_ground(); return Some(()); };
        let count = word(c,8)? as usize;
        if count > self.high || c.len() != HEADER+count*8 { return None; }
        let mut previous = None;
        for k in 0..count {
            let handle = word(c,HEADER+k*8)?;
            let i = self.get(handle)?;
            if i >= self.high || previous.is_some_and(|p| i <= p) || !self.city_ground_active(&self.ents[i]) { return None; }
            let radius = f32::from_bits(word(c,HEADER+k*8+4)?);
            // This adapter currently imports only BASE city_center.ddf profiles;
            // accepting another number would invent unsupported upgrade state.
            if ![7.0,8.0,9.0,10.0,11.0].contains(&radius) || radius < self.city_ground_profile(&self.ents[i]) { return None; }
            self.city_ground[i] = CityBand {handle,radius};
            previous = Some(i);
        }
        if self.ents[..self.high].iter().filter(|e| self.city_ground_active(e)).count() != count { return None; }
        Some(())
    }
}
