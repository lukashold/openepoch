//! Source SpecialPower reveal windows driven by staggered nearby scans.

use super::*;
use crate::spy46_defs::SPY;
use std::collections::BTreeMap;

const DISGUISE_TIME: f32 = 10.0;

#[derive(Default)]
pub(crate) struct SpyDisguise {
    deadline: BTreeMap<u32, f32>,
    last_scan: BTreeMap<u32, f32>,
}

impl SpyDisguise {
    fn hidden(&self, handle: u32, now: f32) -> bool {
        let deadline = self.deadline.get(&handle).copied().unwrap_or(0.0);
        now > deadline || f64::from(now) < f64::from(deadline) - f64::from(DISGUISE_TIME)
    }

    pub(crate) fn retire(&mut self, handle: u32) {
        self.deadline.remove(&handle);
        self.last_scan.remove(&handle);
    }
}

fn scan_due(tick: u32, id: u32, now: f32, last: f32, step: f32, interval: f32) -> bool {
    let period = (f64::from(interval) / f64::from(step)).ceil() as i32;
    period > 0 && ((tick.wrapping_add(id) as i32) % period == 0
        || f64::from(now) - f64::from(interval) >= f64::from(last))
}

impl Game {
    pub(crate) fn spy_disguised(&self, index: usize) -> bool {
        self.ents[index].def == SPY && self.powers.is_some()
            && self.spy_disguise.hidden(self.h(index), self.sim_elapsed())
    }

    pub(crate) fn tick_spy_disguise(&mut self) {
        let Some(powers) = &self.powers else { return; };
        let available = |actor: &Ent| actor.alive && actor.hp > 0.0 && actor.built >= 1.0 && actor.stuck != BOARDED;
        let now = self.sim_elapsed();
        let step = self.sim_step();
        let actors = &self.ents[..self.high];
        if !actors.iter().any(|actor| actor.def == SPY && available(actor)) { return; }
        for (index, detector) in actors.iter().enumerate() {
            if !matches!(detector.def, SPY | TOWER) || !available(detector) { continue; }
            let handle = self.h(index);
            let Some(record) = powers.actor(handle) else { continue; };
            let actor_id = (u32::from(detector.team) + 1) << 27 | record.serial;
            // Neither of these detector families has the ICBM interception modifier.
            let interval = 1.5;
            let last = self.spy_disguise.last_scan.get(&handle).copied().unwrap_or(0.0);
            if !scan_due(self.tick, actor_id, now, last, step, interval) { continue; }
            self.spy_disguise.last_scan.insert(handle, now);
            let radius = f64::from(self.modifiers[detector.team as usize][detector.def as usize][5]);
            if radius <= 0.0 { continue; }
            let z = self.map.height(detector.x, detector.y);
            for (spy, actor) in actors.iter().enumerate() {
                if actor.def != SPY || !available(actor) || actor.team == detector.team { continue; }
                let x = f64::from(actor.x) - f64::from(detector.x);
                let y = f64::from(actor.y) - f64::from(detector.y);
                let dz = f64::from(self.map.height(actor.x, actor.y)) - f64::from(z);
                if x * x + y * y + dz * dz <= radius * radius {
                    let expires = crate::power42_math::store_toward_zero(f64::from(now) + f64::from(DISGUISE_TIME));
                    let target = ((u32::from(actor.gen)) << 16) | spy as u32;
                    self.spy_disguise.deadline.insert(target, expires);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_scan_phase_repeat_and_unsaved_reveal_window() {
        assert!(!scan_due(1, 0, 0.05, 0.0, 0.05, 1.5));
        assert!(scan_due(29, 1, 1.45, 0.0, 0.05, 1.5));
        assert!(scan_due(29, 2, 1.5, 0.0, 0.05, 1.5));
        assert!(!scan_due(29, 2, 1.49, 0.0, 0.05, 1.5));
        let mut g = Game::new(7, 1, 0, 1, 0);
        assert!(g.configure_priest_skirmish());
        g.map.hm.fill(0.0);
        let spy = g.spawn(SPY, 0, 96.0, 96.0);
        let own = g.spawn(TOWER, 0, 92.0, 96.0);
        let bystander = g.spawn(WORKER, 1, 96.0, 99.0);
        assert!(!g.spy_disguised(spy));
        g.tick = 30;
        g.tick_spy_disguise();
        assert!(g.spy_disguised(spy));
        assert!(g.detected(0, spy));
        assert!(!g.detected(1, spy));
        let radius = g.modifiers[1][TOWER as usize][5];
        let detector = g.spawn(TOWER, 1, 96.0 + radius, 96.0);
        g.tick_spy_disguise();
        assert!(!g.spy_disguised(spy), "the detector is exactly on the inclusive LOS boundary");
        assert!(g.detected(1, spy));
        g.tick = 60;
        g.tick_spy_disguise();
        g.kill(detector);
        g.kill(bystander);
        g.kill(own);
        g.tick = 255;
        assert!(!g.spy_disguised(spy), "repeat detection refreshed the deadline");
        g.save();
        let loaded = Game::load(&g.savebuf).expect("the original does not serialize the reveal deadline");
        assert!(loaded.spy_disguised(spy));
        g.tick = 260;
        assert!(!g.spy_disguised(spy));
        g.tick = 261;
        assert!(g.spy_disguised(spy));
    }
}
