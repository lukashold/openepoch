//! Authored owned IDs share the World0 Unit manager cursor. The portable
//! handle stays generation-tagged and continues to identify command targets.
use std::collections::{BTreeMap, BTreeSet};

use crate::{defs::CARGO_MAX, game::{Game, NONE}};

pub(super) const TAG: &[u8; 4] = b"OID1";

#[derive(Default)]
pub(crate) struct OwnedIds {
    attached: BTreeMap<u32, u32>,
    passengers: BTreeMap<(u32, usize), u32>,
}

impl OwnedIds {
    fn words(&self) -> impl Iterator<Item = u32> + '_ {
        [u32::from_le_bytes(*TAG), self.attached.len() as u32, self.passengers.len() as u32]
            .into_iter()
            .chain(self.attached.iter().flat_map(|(&handle, &local)| [handle, local]))
            .chain(self.passengers.iter().flat_map(|(&(host, slot), &local)| [host, slot as u32, local]))
    }

    pub(crate) fn section(&self) -> Vec<u8> {
        self.words().flat_map(u32::to_le_bytes).collect()
    }

    pub(crate) fn hash(&self, mix: &mut impl FnMut(u32)) {
        self.words().for_each(mix);
    }

    pub(crate) fn bind(&mut self, handle: u32, local: u32) {
        self.attached.insert(handle, local);
    }

    pub(crate) fn retire(&mut self, handle: u32, crew: u8) {
        self.attached.remove(&handle);
        for slot in 0..usize::from(crew) {
            self.passengers.remove(&(handle, slot));
        }
    }

    pub(crate) fn embark(&mut self, handle: u32, host: u32, slot: usize) {
        if let Some(&local) = self.attached.get(&handle) {
            self.passengers.insert((host, slot), local);
        }
    }

    pub(crate) fn passenger(&self, host: u32, slot: usize) -> Option<u32> {
        self.passengers.get(&(host, slot)).copied()
    }

    pub(crate) fn release(&mut self, host: u32, slot: usize, retained_slot: Option<usize>) {
        if let Some(local) = self.passengers.remove(&(host, slot)) {
            if let Some(retained_slot) = retained_slot {
                self.passengers.insert((host, retained_slot), local);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn native_rect_actor_id(handle: u32) -> u32 {
    let Some(game) = super::active_game() else { return NONE; };
    let Some(index) = game.get(handle) else { return NONE; };
    game.rect_native_id(index, game.ents[index].team).unwrap_or(NONE)
}

impl Game {
    pub(super) fn start_rect_native_ids(&mut self) -> Option<()> {
        let cursor = self.rect_native_cursor()?;
        let base = self.rect_native_base()?;
        if self.high != self.nplayers * 3 || cursor != base + self.high as u32 {
            return None;
        }
        let mut ids = OwnedIds::default();
        for (index, actor) in self.ents[..self.high].iter().enumerate() {
            if !actor.alive || usize::from(actor.team) != index / 3 { return None; }
            ids.bind(self.h(index), base + index as u32 + 1);
        }
        self.rect_native_ids = Some(ids);
        Some(())
    }

    pub(crate) fn rect_native_id(&self, index: usize, team: u8) -> Option<u32> {
        let local = self.rect_native_ids.as_ref()?.attached.get(&self.h(index))?;
        Some((u32::from(team) + 1) << 27 | local)
    }

    pub(crate) fn rect_native_transfer(&mut self, index: usize) {
        if self.rect_native_ids.is_none() { return; }
        let handle = self.h(index);
        if let Some(local) = self.note_rect_owned_allocation() {
            self.rect_native_ids.as_mut().unwrap().bind(handle, local);
        }
    }

    pub(crate) fn restore_rect_native_ids(&mut self, section: &[u8], high: usize) -> Option<()> {
        if self.rect_ai_clock.is_none() || !section.starts_with(TAG) { return None; }
        let base = self.rect_native_base()?;
        let cursor = self.rect_native_cursor()?;
        let population = self.rect_population.as_ref()?;
        let mut words = section[4..].chunks_exact(4);
        if !words.remainder().is_empty() { return None; }
        let mut word = || words.next().map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()));
        let attached = word()? as usize;
        let passengers = word()? as usize;
        if attached > high || passengers > attached * CARGO_MAX { return None; }
        let mut locals = BTreeSet::new();
        let mut insert_local = |local| (base < local && local <= cursor && population.unit(local).is_none()
            && locals.insert(local)).then_some(local);
        let mut ids = OwnedIds::default();
        let mut last = None;
        for _ in 0..attached {
            let handle = word()?;
            let local = insert_local(word()?)?;
            let index = self.get(handle)?;
            if index >= high || last.is_some_and(|previous| previous >= handle) { return None; }
            ids.bind(handle, local);
            last = Some(handle);
        }
        let mut last = None;
        for _ in 0..passengers {
            let host = word()?;
            let slot = word()? as usize;
            let local = insert_local(word()?)?;
            let index = self.get(host)?;
            let key = (host, slot);
            if index >= high || slot >= usize::from(self.ents[index].ncargo)
                || last.is_some_and(|previous| previous >= key) { return None; }
            ids.passengers.insert(key, local);
            last = Some(key);
        }
        let live = self.ents[..high].iter().filter(|actor| actor.alive);
        if word().is_some() || live.clone().count() != attached
            || live.map(|actor| usize::from(actor.ncargo)).sum::<usize>() != passengers { return None; }
        self.rect_native_ids = Some(ids);
        Some(())
    }

}
