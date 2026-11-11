//! Staging buffer and authenticated immutable World0 carrier.
use super::{geometry::Geometry, population234::{Population, MAX_POPULATION_INPUT}};

#[derive(Default)]
pub(super) struct PendingPopulation {
    input: Vec<u8>,
    ready: Option<Population>,
}

impl PendingPopulation {
    pub(super) fn alloc(&mut self, length: usize) -> *mut u8 {
        self.input.clear();
        if !(112..=MAX_POPULATION_INPUT).contains(&length) { return std::ptr::null_mut(); }
        self.input.resize(length, 0);
        self.input.as_mut_ptr()
    }

    pub(super) fn prepare(&mut self, geometry: &Geometry) -> bool {
        let Some(parsed) = Population::decode(&self.input, geometry.source_sha, geometry.input_sha) else {
            return false;
        };
        if parsed.dimensions().map(usize::from) != [geometry.width, geometry.height] ||
            parsed.units().len() != geometry.pending_units as usize ||
            parsed.forest().len() != geometry.pending_forest as usize
        {
            return false;
        }
        self.ready = Some(parsed);
        true
    }

    pub(super) fn ready(&self) -> Option<&Population> { self.ready.as_ref() }

    pub(super) fn reset(&mut self) {
        self.input.clear();
        self.ready = None;
    }
}
