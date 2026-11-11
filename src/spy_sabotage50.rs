//! Native effect kind 62, attached to a completed building by the Spy.

use super::*;
use std::collections::BTreeMap;

const MAGIC: u32 = u32::from_le_bytes(*b"SAB1");

#[derive(Clone, Copy)]
struct Disabled {
    since: f32,
    improved: bool,
    crowned: bool,
}

impl Disabled {
    fn active(self, now: f32) -> bool {
        let base = if self.improved { 144.0 } else { 120.0 };
        let duration = if self.crowned { base * 1.25 } else { base };
        now <= crate::power42_math::store_toward_zero(f64::from(self.since) + duration)
    }
}

#[derive(Default)]
pub(crate) struct Sabotage {
    targets: BTreeMap<u32, Disabled>,
}

impl Sabotage {
    pub(crate) fn active(&self) -> bool { !self.targets.is_empty() }

    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.targets.len() as u32);
        for (&handle, disabled) in &self.targets {
            write(handle);
            write(disabled.since.to_bits());
            write(u32::from(disabled.improved) | (u32::from(disabled.crowned) << 1));
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        if self.targets.is_empty() { return; }
        write(MAGIC);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.targets.is_empty() || core.is_empty() { return core; }
        let mut data = Vec::with_capacity(core.len() + 12 * self.targets.len() + 20);
        for word in [MAGIC, 1, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }

    pub(crate) fn retire(&mut self, handle: u32) { self.targets.remove(&handle); }
}

impl Game {
    pub(crate) fn sabotaged(&self, index: usize) -> bool {
        self.spy_sabotage.targets.get(&self.h(index)).is_some_and(|effect| effect.active(self.sim_elapsed()))
    }

    pub(super) fn sabotage_building(&mut self, owner: u8, target: usize, since: f32) {
        let improved = self.has_research(owner, 160); // Military Epoch 14, slot 1.
        let crowned = self.crown_super_spies(owner);
        self.spy_sabotage.targets.insert(self.h(target), Disabled { since, improved, crowned });
    }

    pub(super) fn restore_building(&mut self, target: usize) {
        self.spy_sabotage.retire(self.h(target));
    }

    pub(crate) fn tick_spy_sabotage(&mut self) {
        let now = self.sim_elapsed();
        self.spy_sabotage.targets.retain(|_, disabled| disabled.active(now));
    }

    pub(crate) fn restore_spy_sabotage(&mut self, fields: Option<&[u8]>) -> Option<()> {
        let Some(fields) = fields else { return Some(()); };
        if self.powers.is_none() || self.net { return None; }
        let mut words = fields.chunks_exact(4);
        let mut read = || words.next().and_then(|chunk| Some(u32::from_le_bytes(chunk.try_into().ok()?)));
        let count = read()? as usize;
        if count == 0 || fields.len() != count.checked_mul(12)?.checked_add(4)? { return None; }
        let now = self.sim_elapsed();
        let mut state = Sabotage::default();
        for _ in 0..count {
            let handle = read()?;
            let target = self.ents[self.get(handle)?];
            let since = f32::from_bits(read()?);
            let flags = read()?;
            if flags > 3 || flags & 2 != 0 && self.crowns.is_none() { return None; }
            let effect = Disabled { since, improved: flags & 1 != 0, crowned: flags & 2 != 0 };
            if target.hp <= 0.0 || target.built < 1.0 || !DEFS[target.def as usize].building
                || !(0.0..=now).contains(&since) || !effect.active(now)
                || state.targets.insert(handle, effect).is_some() {
                return None;
            }
        }
        self.spy_sabotage = state;
        Some(())
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let len = word(8)? as usize;
    let end = data.len().checked_sub(4)?;
    let start = end.checked_sub(len)?;
    if word(4)? != 1 || len == 0 || start < 28 || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::C_GATHER_INTEL;
    use crate::spy46_defs::SPY;

    fn run_cast(game: &mut Game, spy: usize) {
        let mut actor = game.ents[spy];
        game.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
        game.ents[spy] = actor;
    }

    #[test]
    fn native_duration_production_and_restore_cast_archive() {
        let mut g = Game::new(7, 1, 0, 1, 0);
        assert!(g.configure_priest_skirmish());
        let spy = g.spawn(SPY, 0, 40.0, 40.0);
        let rival = g.spawn(TC, 1, 42.0, 40.0);
        let ally = g.spawn(TC, 0, 40.0, 42.0);
        let spy_handle = g.h(spy);
        let rival_handle = g.h(rival);
        let ally_handle = g.h(ally);
        let issue = |g: &mut Game, command, target| g.apply(0, C_GATHER_INTEL, command, 0.0, target as i32, &[spy_handle]);
        let recharge = |g: &mut Game| g.powers.as_mut().unwrap().recharge_selected(spy_handle);
        recharge(&mut g);
        issue(&mut g, 1.0, rival_handle);
        assert_eq!(g.ents[spy].order, O_IDLE, "the epoch alone never unlocks the ability");
        g.researched[0][1] |= 1 << 8;
        issue(&mut g, 2.0, ally_handle);
        assert_eq!(g.ents[spy].order, O_IDLE, "restore requires a live sabotage subeffect");
        assert!(g.train(rival, WORKER as QueueItem));
        issue(&mut g, 1.0, rival_handle);
        run_cast(&mut g, spy);
        assert_eq!(g.spy_pose_state(spy_handle), 2);
        g.save();
        let mut g = Game::load(&g.savebuf).expect("restore the typed in-progress sabotage cast");
        g.tick = 100;
        run_cast(&mut g, spy);
        assert!(!g.sabotaged(rival), "five seconds at equality do not finish the cast");
        g.tick = 105;
        run_cast(&mut g, spy);
        assert!(g.sabotaged(rival));
        assert!(!g.can_train(rival, WORKER as QueueItem));
        let qt = g.ents[rival].qt;
        for _ in 0..4 { g.tick(); }
        assert_eq!(g.ents[rival].qt, qt);
        assert_eq!(g.ents[rival].qn, 1, "existing paid order remains queued");
        recharge(&mut g);
        issue(&mut g, 1.0, rival_handle);
        assert_eq!(g.ents[spy].order, O_IDLE, "a second sabotage is rejected during the active effect");
        g.sabotage_building(1, ally, g.sim_elapsed());
        issue(&mut g, 2.0, ally_handle);
        run_cast(&mut g, spy);
        assert_eq!(g.spy_pose_state(spy_handle), 3);
        g.save();
        let mut g = Game::load(&g.savebuf).expect("restore active sabotage before validating the pending repair cast");
        g.tick += 105;
        run_cast(&mut g, spy);
        assert!(!g.sabotaged(ally));
        assert!(g.sabotaged(rival));
        g.tick = 2505;
        assert!(g.sabotaged(rival));
        g.tick = 2506;
        g.tick_spy_sabotage();
        assert!(!g.sabotaged(rival));
        g.update_ent(rival, #[cfg(feature = "ee2-valley206-citizen-clearance")] &mut CitizenSeparationFrame::new());
        assert!(g.ents[rival].qt > qt);
        g.researched[0][5] |= 1;
        g.tick = 2600;
        g.sabotage_building(0, rival, g.sim_elapsed());
        g.tick = 5480;
        assert!(g.sabotaged(rival));
        g.tick = 5481;
        assert!(!g.sabotaged(rival));
    }
}
