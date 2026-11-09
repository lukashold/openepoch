//! The Spy's contamination site and standalone Toxic Sickness subeffects.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

const MAGIC: u32 = u32::from_le_bytes(*b"TOX1");
const SITE_TIME: f32 = 40.0;
const SICK_TIME: f32 = 20.0;
const RADIUS_SQUARED: f64 = 16.0;

struct Site {
    owner: u8,
    expires: f32,
    improved: bool,
    inside: BTreeSet<u32>,
}

struct Sickness {
    target: u32,
    expires: f32,
    last_hit: f32,
    improved: bool,
}

#[derive(Default)]
pub(crate) struct ToxicState {
    sites: BTreeMap<u32, Site>,
    sickness: Vec<Sickness>,
}

fn expires(now: f32, time: f32) -> f32 {
    crate::power42_math::store_toward_zero(f64::from(now) + f64::from(time))
}

impl ToxicState {
    pub(crate) fn empty(&self) -> bool { self.sites.is_empty() && self.sickness.is_empty() }

    fn words(&self, mut write: impl FnMut(u32)) {
        write(self.sites.len() as u32);
        write(self.sickness.len() as u32);
        for (&target, site) in &self.sites {
            for word in [target, u32::from(site.owner), site.expires.to_bits(),
                u32::from(site.improved), site.inside.len() as u32] { write(word); }
            for &member in &site.inside { write(member); }
        }
        for disease in &self.sickness {
            for word in [disease.target, disease.expires.to_bits(), disease.last_hit.to_bits(), u32::from(disease.improved)] {
                write(word);
            }
        }
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        if self.empty() { return; }
        write(MAGIC);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.empty() || core.is_empty() { return core; }
        let mut data = Vec::with_capacity(core.len() + 48);
        for word in [MAGIC, 1, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }

    pub(crate) fn retire(&mut self, handle: u32) {
        self.sites.remove(&handle);
        self.sickness.retain(|disease| disease.target != handle);
        for site in self.sites.values_mut() { site.inside.remove(&handle); }
    }
}

impl Game {
    pub(crate) fn contaminated(&self, index: usize) -> bool {
        self.spy_toxic.sites.get(&self.h(index)).is_some_and(|site| self.sim_elapsed() <= site.expires)
    }

    pub(crate) fn toxic_sick(&self, index: usize) -> bool {
        let handle = self.h(index);
        self.spy_toxic.sickness.iter().any(|disease| disease.target == handle && self.sim_elapsed() <= disease.expires)
    }

    pub(super) fn contaminate_building(&mut self, caster: u8, target: usize, now: f32) {
        let improved = self.has_research(caster, 160);
        self.spy_toxic.sites.insert(self.h(target), Site { owner: self.ents[target].team,
            expires: expires(now, SITE_TIME), improved, inside: BTreeSet::new() });
    }

    pub(super) fn clean_building(&mut self, target: usize) {
        self.spy_toxic.sites.remove(&self.h(target));
    }

    pub(crate) fn tick_spy_toxic(&mut self) {
        if self.spy_toxic.empty() { return; }
        let Some(powers) = &self.powers else { return; };
        let now = self.sim_elapsed();
        let (tick, step) = (self.tick, self.sim_step());
        let due = |handle, actor: &Ent| powers.actor(handle).is_some_and(|record| {
            let id = (u32::from(actor.team) + 1) << 27 | record.serial;
            crate::capture_math25::capture_event_due(tick, id, step)
        });
        let lookup = |handle: u32, actors: &[Ent]| {
            let index = (handle & 0xffff) as usize;
            actors.get(index).filter(|actor| actor.alive && actor.hp > 0.0
                && u32::from(actor.gen) == handle >> 16 && actor.stuck != BOARDED).map(|_| index)
        };
        let mut hits = Vec::new();
        self.spy_toxic.sickness.retain_mut(|disease| {
            if now > disease.expires { return false; }
            let Some(index) = lookup(disease.target, &self.ents) else { return false; };
            if due(disease.target, &self.ents[index]) {
                let elapsed = f64::from(now) - f64::from(disease.last_hit);
                disease.last_hit = now;
                let rate = if disease.improved { 8.4_f32 } else { 7.0 };
                let damage = crate::power42_math::store_toward_zero(elapsed * f64::from(rate));
                if damage > 0.0 { hits.push((index, damage)); }
            }
            true
        });
        for (index, damage) in hits {
            if !self.god_mode_for(self.ents[index].team) && self.ents[index].hp > 0.0 {
                let actor = &mut self.ents[index];
                actor.hp = crate::power42_math::store_toward_zero((f64::from(actor.hp) - f64::from(damage)).max(0.0));
                actor.flash = 0.12;
            }
        }
        let actors = &self.ents[..self.high];
        let sickness = &mut self.spy_toxic.sickness;
        self.spy_toxic.sites.retain(|&handle, site| {
            if now > site.expires { return false; }
            let Some(index) = lookup(handle, actors) else { return false; };
            let building = actors[index];
            if !due(handle, &building) { return true; }
            let mut inside = BTreeSet::new();
            for (index, worker) in actors.iter().enumerate() {
                if !worker.alive || worker.hp <= 0.0 || worker.def != WORKER || worker.team != site.owner
                    || worker.stuck == BOARDED { continue; }
                let dx = f64::from(worker.x) - f64::from(building.x);
                let dy = f64::from(worker.y) - f64::from(building.y);
                if dx * dx + dy * dy > RADIUS_SQUARED { continue; }
                let target = u32::from(worker.gen) << 16 | index as u32;
                inside.insert(target);
                if !site.inside.contains(&target) {
                    sickness.push(Sickness { target, expires: expires(now, SICK_TIME),
                        last_hit: now, improved: site.improved });
                }
            }
            site.inside = inside;
            true
        });
    }

    pub(crate) fn restore_spy_toxic(&mut self, fields: Option<&[u8]>) -> Option<()> {
        let Some(fields) = fields else { return Some(()); };
        if self.powers.is_none() || self.net || fields.len() % 4 != 0 { return None; }
        let mut words = fields.chunks_exact(4);
        fn word(words: &mut std::slice::ChunksExact<'_, u8>) -> Option<u32> {
            Some(u32::from_le_bytes(words.next()?.try_into().ok()?))
        }
        fn flag(words: &mut std::slice::ChunksExact<'_, u8>) -> Option<bool> {
            match word(words)? { 0 => Some(false), 1 => Some(true), _ => None }
        }
        let (sites, sick) = (word(&mut words)? as usize, word(&mut words)? as usize);
        if sites == 0 && sick == 0 || sites > self.high || sites.checked_mul(5)?.checked_add(sick.checked_mul(4)?)? > words.len() {
            return None;
        }
        let now = self.sim_elapsed();
        let mut state = ToxicState::default();
        for _ in 0..sites {
            let handle = word(&mut words)?;
            let target = self.ents[self.get(handle)?];
            let owner = u8::try_from(word(&mut words)?).ok()?;
            let deadline = f32::from_bits(word(&mut words)?);
            let improved = flag(&mut words)?;
            let count = word(&mut words)? as usize;
            if !matches!(target.def, TC | WAREHOUSE | crate::granary211::ID) || target.hp <= 0.0 || target.built < 1.0
                || owner as usize >= self.nplayers || !(now..=now + SITE_TIME).contains(&deadline)
                || count > self.high || count > words.len() { return None; }
            let mut inside = BTreeSet::new();
            for _ in 0..count {
                let member = word(&mut words)?;
                let worker = self.ents[self.get(member)?];
                if worker.def != WORKER || worker.hp <= 0.0 || !inside.insert(member) { return None; }
            }
            if state.sites.insert(handle, Site { owner, expires: deadline, improved, inside }).is_some() { return None; }
        }
        for _ in 0..sick {
            let target = word(&mut words)?;
            let worker = self.ents[self.get(target)?];
            let deadline = f32::from_bits(word(&mut words)?);
            let last_hit = f32::from_bits(word(&mut words)?);
            let improved = flag(&mut words)?;
            if worker.def != WORKER || worker.hp <= 0.0 || !(now..=now + SICK_TIME).contains(&deadline)
                || !(0.0..=now).contains(&last_hit) { return None; }
            state.sickness.push(Sickness { target, expires: deadline, last_hit, improved });
        }
        if words.len() != 0 { return None; }
        self.spy_toxic = state;
        Some(())
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let len = word(8)? as usize;
    let end = data.len().checked_sub(4)?;
    let start = end.checked_sub(len)?;
    if word(4)? != 1 || len == 0 || start < 36 || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::C_GATHER_INTEL;
    use crate::spy46_defs::SPY;

    #[test]
    fn contamination_spreads_only_to_owner_citizens_and_cleanup_keeps_sickness() {
        let mut g = Game::new(7, 1, 0, 1, 0);
        assert!(g.configure_priest_skirmish());
        let spy = g.spawn(SPY, 0, 40.0, 40.0);
        let rival = g.spawn(WAREHOUSE, 1, 42.0, 40.0);
        let adjacent = g.spawn(WORKER, 1, 43.0, 41.0);
        let outside = g.spawn(WORKER, 1, 47.0, 40.0);
        let foreign = g.spawn(WORKER, 0, 43.0, 40.0);
        let house = g.spawn(HOUSE, 1, 40.0, 43.0);
        let handle = g.h(spy);
        let site = g.h(rival);
        g.powers.as_mut().unwrap().recharge_selected(handle);
        g.apply(0, C_GATHER_INTEL, 3.0, 0.0, site as i32, &[handle]);
        assert_eq!(g.ents[spy].order, O_IDLE);
        g.researched[0][3] |= 1 << 4;
        g.apply(0, C_GATHER_INTEL, 3.0, 0.0, g.h(house) as i32, &[handle]);
        assert_eq!(g.ents[spy].order, O_IDLE);
        g.ents[spy].x = g.ents[rival].x - Game::building_half(&g.ents[rival]) - 0.05;
        g.apply(0, C_GATHER_INTEL, 3.0, 0.0, site as i32, &[handle]);
        assert_eq!(g.ents[spy].order, O_GATHER_INTEL);
        let mut actor = g.ents[spy];
        g.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
        g.ents[spy] = actor;
        assert_eq!(g.spy_pose_state(handle), 4);
        g.tick = 105;
        g.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
        g.ents[spy] = actor;
        assert!(g.contaminated(rival));
        let hp = g.ents[adjacent].hp;
        let next = |g: &Game, index: usize, first: u32| {
            let id = (u32::from(g.ents[index].team) + 1) << 27
                | g.powers.as_ref().unwrap().actor(g.h(index)).unwrap().serial;
            (first..first + 20).find(|&tick| crate::capture_math25::capture_event_due(tick, id, 0.05)).unwrap()
        };
        g.tick = next(&g, rival, 105);
        let exposed_at = g.sim_elapsed();
        let expiry_tick = g.tick + 400;
        g.tick_spy_toxic();
        assert!(g.toxic_sick(adjacent));
        assert!(!g.toxic_sick(outside));
        assert!(!g.toxic_sick(foreign));
        g.save();
        let mut g = Game::load(&g.savebuf).expect("restore parent site and standalone illness");
        g.tick = next(&g, adjacent, g.tick + 1);
        g.tick_spy_toxic();
        let expected = hp - (g.sim_elapsed() - exposed_at) * 7.0;
        assert!((g.ents[adjacent].hp - expected).abs() < 0.001);
        let cured_site = g.spawn(WAREHOUSE, 0, 39.0, 41.0);
        g.contaminate_building(1, cured_site, g.sim_elapsed());
        g.ents[spy].x = g.ents[cured_site].x;
        g.ents[spy].y = g.ents[cured_site].y - Game::building_half(&g.ents[cured_site]) - 0.05;
        g.powers.as_mut().unwrap().recharge_selected(handle);
        g.apply(0, C_GATHER_INTEL, 4.0, 0.0, g.h(cured_site) as i32, &[handle]);
        let mut actor = g.ents[spy];
        g.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
        g.ents[spy] = actor;
        assert_eq!(g.spy_pose_state(handle), 5);
        g.tick += 105;
        g.do_spy_intel(spy, &mut actor, &DEFS[SPY as usize]);
        assert!(!g.contaminated(cured_site));
        g.clean_building(rival);
        assert!(!g.contaminated(rival));
        assert!(g.toxic_sick(adjacent));
        g.tick = expiry_tick + 1;
        g.tick_spy_toxic();
        assert!(!g.toxic_sick(adjacent));
    }
}
