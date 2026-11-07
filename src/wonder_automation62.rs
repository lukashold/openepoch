//! Per-factory clocks for the authored Robotics Factory Spawn subeffects.

use super::{Ent, Game, MAXE, NONE, NOIDX, O_FACTORY_EGRESS};
use crate::{capture_math25::capture_event_due, defs::DefId, map::{tile_of, N}, save::checksum,
    wonders61::ROBOTICS_FACTORY};
use std::collections::BTreeMap;

const TAG: [u8; 4] = *b"AUT1";
const TANKS: [DefId; 2] = [23, 26];
const LATE_UNITS: [DefId; 4] = [23, 26, crate::heavy_mounted3_61::ID, crate::defs::LIGHT_MOUNTED3];
// bld_robotics_factory.nif tag_birth_01 and tag_birth_exit, in the
// portable world basis with the source model scale and visual yaw applied.
const BIRTH: [f64; 2] = [-2.9347172531, 1.8810705674];
const EXIT: [f64; 2] = [-2.9347172531, 4.5247581900];

#[derive(Clone, Copy)]
struct Clock {
    owner: u8,
    late: bool,
    last: f32,
}

#[derive(Default)]
pub(crate) struct FactoryClocks(BTreeMap<u32, Clock>);

fn spawn_due(now: f32, last: f32) -> bool {
    // 008088EF keeps the difference in x87 through the comparison.
    let elapsed = f64::from(now) - f64::from(last);
    elapsed == 0.0 || elapsed >= 60.0
}

impl FactoryClocks {
    pub(super) fn poll(&mut self, handle: u32, owner: u8, late: bool, tick: u32, now: f32, step: f32) -> bool {
        let fresh = self.0.get(&handle).is_none_or(|clock| clock.owner != owner || clock.late != late);
        // The native phase adds the packed actor subject. Use the portable
        // generational slot as the lower identity portion of that namespace.
        let phase = ((u32::from(owner) + 1) << 27) | (handle & 0x07ff_ffff);
        if !fresh && !capture_event_due(tick, phase, step) { return false; }
        let clock = self.0.entry(handle).or_insert(Clock { owner, late, last: 0.0 });
        if fresh { *clock = Clock { owner, late, last: 0.0 }; }
        if !spawn_due(now, clock.last) { return false; }
        // Native stores the poll timestamp before any host/egress check.
        clock.last = now;
        true
    }

    pub(super) fn remove(&mut self, handle: u32) { self.0.remove(&handle); }

    fn words(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().flat_map(|(&handle, clock)|
            [handle, u32::from(clock.owner) | (u32::from(clock.late) << 8), clock.last.to_bits()])
    }

    pub(super) fn hash(&self, mut mix: impl FnMut(u32)) {
        if self.0.is_empty() { return; }
        mix(u32::from_le_bytes(TAG));
        mix(self.0.len() as u32);
        self.words().for_each(mix);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if self.0.is_empty() { return core; }
        let mut bytes = Vec::with_capacity(core.len() + 12 + self.0.len() * 12);
        bytes.extend_from_slice(&TAG);
        bytes.extend_from_slice(&(self.0.len() as u32).to_le_bytes());
        for word in self.words() { bytes.extend_from_slice(&word.to_le_bytes()); }
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&checksum(&bytes).to_le_bytes());
        bytes
    }

    pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Self)> {
        let Some(tail) = bytes.strip_prefix(&TAG) else { return Some((bytes, Self::default())); };
        let count = read_word(tail)? as usize;
        if !(1..=MAXE).contains(&count) { return None; }
        let end = bytes.len().checked_sub(4)?;
        if checksum(bytes.get(..end)?) != read_word(bytes.get(end..)?)? { return None; }
        let (records, core) = bytes.get(8..end)?.split_at_checked(count * 12)?;
        if core.is_empty() { return None; }
        let mut clocks = BTreeMap::new();
        let mut previous = None;
        for record in records.chunks_exact(12) {
            let handle = read_word(record)?;
            let packed = read_word(&record[4..])?;
            let last = f32::from_bits(read_word(&record[8..])?);
            if previous.is_some_and(|old| old >= handle) || packed > 0x1ff
                || !last.is_finite() || last.is_sign_negative() { return None; }
            clocks.insert(handle, Clock { owner: packed as u8, late: packed & 0x100 != 0, last });
            previous = Some(handle);
        }
        Some((core, Self(clocks)))
    }

    pub(crate) fn valid(&self, elapsed: f32, mut is_factory: impl FnMut(u32, u8, bool) -> bool) -> bool {
        self.0.iter().all(|(&handle, clock)| clock.last <= elapsed && is_factory(handle, clock.owner, clock.late))
    }
}

fn read_word(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?))
}

pub(crate) fn factory_unit(definition: DefId) -> bool { LATE_UNITS.contains(&definition) }

fn map_point(factory: Ent, [x, y]: [f64; 2]) -> [f32; 2] {
    let (sin, cos) = f64::from(factory.rot).sin_cos();
    [(f64::from(factory.x) + x * cos - y * sin) as f32, (f64::from(factory.y) + x * sin + y * cos) as f32]
}

fn probe_exit([x, y]: [f32; 2], mut passable: impl FnMut(f32, f32) -> bool) -> Option<[f32; 2]> {
    let max = N as f32 - 1.0;
    let [x, y] = [x, y].map(|value| value.clamp(0.0, max - 1.0));
    if passable(x, y) { return Some([x, y]); }
    for ring in 1..=3 {
        let r = ring as f32 * 0.5;
        for ordinal in 0..=ring {
            let t = ordinal as f32 * 0.5;
            for ([px, py], in_bounds) in [
                ([x + r, y + t], x + r < max && y + t < max),
                ([x - t, y + r], x - t >= 0.0 && y + r < max),
                ([x - r, y - t], x - r >= 0.0 && y - t >= 0.0),
                ([x + t, y - r], x + t <= max && y - r >= 0.0),
            ] {
                if in_bounds && passable(px, py) { return Some([px, py]); }
            }
        }
    }
    None
}

fn terrain_rally(game: &Game, factory: Ent, [exit_x, exit_y]: [f32; 2]) -> Option<[f32; 2]> {
    if factory.rx >= 0.0 { return Some([factory.rx, factory.ry]); }
    let (dx, dy) = (exit_x - factory.x, exit_y - factory.y);
    let [width, height] = crate::wonders61::shape(ROBOTICS_FACTORY, true).unwrap();
    let distance = dx.hypot(dy);
    if distance == 0.0 { return None; }
    let radius = 0.5 * width.max(height);
    let (fx, fy) = (dx / distance * radius, dy / distance * radius);
    [(1.0, 1.0), (1.0, 0.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, 0.0), (-1.0, -1.0)]
        .into_iter().map(|(a, b)| [factory.x + a * fx - b * fy, factory.y + a * fy + b * fx])
        .find(|&[x, y]| (0.0..N as f32).contains(&x) && (0.0..N as f32).contains(&y) && game.map.height(x, y) >= 0.0)
}

impl Game {
    pub(super) fn tick_factory_automation(&mut self) {
        if self.net || self.bench != 0 || self.winner != 0 || self.powers.is_none() || self.wonder_sites.is_empty() { return; }
        let factories: Vec<_> = self.wonder_sites.iter().copied().filter(|&index| {
            let actor = self.ents[index];
            actor.def == ROBOTICS_FACTORY && actor.alive && actor.hp > 0.0 && actor.built >= 1.0
                && self.players[actor.team as usize].epoch >= 10
        }).collect();
        let (now, step) = (self.sim_elapsed(), self.sim_step());
        for index in factories {
            let factory = self.ents[index];
            let handle = self.h(index);
            let late = self.players[factory.team as usize].epoch >= 14;
            if !self.factory_clocks.poll(handle, factory.team, late, self.tick, now, step) { continue; }
            let units: &[DefId] = if late { &LATE_UNITS } else { &TANKS };
            let rng = &mut self.source_birth.rng;
            rng.next_word15(); // Authored minCount=maxCount=1 still consumes a draw.
            let definition = units[rng.next_word15() as usize % units.len()];
            rng.next_word15(); // The generic class is unchanged by the sampled civilization.
            let [x, y] = map_point(factory, BIRTH);
            let Some([exit_x, exit_y]) = probe_exit(map_point(factory, EXIT), |x, y| {
                (0.0..N as f32).contains(&x) && (0.0..N as f32).contains(&y) && self.map.pass[0][tile_of(x, y)]
            }) else { continue; };
            let rally = terrain_rally(self, factory, [exit_x, exit_y]);
            if late { self.heavy_mounted3 = true; }
            let yaw = (exit_y - factory.y).atan2(exit_x - factory.x).rem_euclid(core::f32::consts::TAU);
            let unit = self.spawn_oriented(definition, factory.team, x, y, yaw);
            if unit == NOIDX { continue; }
            let actor = &mut self.ents[unit];
            actor.order = O_FACTORY_EGRESS;
            actor.ox = exit_x;
            actor.oy = exit_y;
            actor.otgt = handle;
            if let Some([rx, ry]) = rally { actor.rx = rx; actor.ry = ry; }
        }
    }

    pub(super) fn advance_factory_egress(&mut self, index: usize, mut actor: Ent, step: f32) {
        let dx = actor.ox - actor.x;
        let dy = actor.oy - actor.y;
        let distance = dx.hypot(dy);
        let speed = self.modifiers[actor.team as usize][actor.def as usize][2];
        if distance <= speed * step {
            let rally = (actor.rx >= 0.0).then_some([actor.rx, actor.ry]);
            actor.x = actor.ox;
            actor.y = actor.oy;
            actor.vx = 0.0;
            actor.vy = 0.0;
            actor.otgt = NONE;
            actor.rx = -1.0;
            actor.ry = -1.0;
            Self::set_idle(&mut actor);
            self.ents[index] = actor;
            if let Some([x, y]) = rally { self.order_move(&[index], x, y, false); }
        } else {
            actor.vx = dx / distance * speed;
            actor.vy = dy / distance * speed;
            actor.x += actor.vx * step;
            actor.y += actor.vy * step;
            self.ents[index] = actor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_first_update_and_late_replacement_use_new_deadlines() {
        let mut clocks = FactoryClocks::default();
        assert!(!clocks.poll(7, 0, false, 800, 40.0, 0.05));
        let step = (1..100).find(|&tick| capture_event_due(tick, (1 << 27) | 7, 0.05)).unwrap();
        assert!(!clocks.poll(7, 0, false, step, f32::from_bits(60.0_f32.to_bits() - 1), 0.05));
        assert!(clocks.poll(7, 0, false, step, 60.0, 0.05));
        assert!(!clocks.poll(7, 0, false, step + 1, 120.0, 0.05));
        assert!(clocks.poll(7, 0, true, step + 1, 119.0, 0.05));
        assert!(!clocks.poll(7, 0, true, step, 178.0, 0.05));
        assert!(clocks.poll(7, 0, true, step, 179.0, 0.05));
    }

    #[test]
    fn automation_clocks_preserve_canonical_order_and_legacy_core() {
        let core = b"prior game archive".to_vec();
        let mut clocks = FactoryClocks::default();
        assert_eq!(clocks.wrap(core.clone()), core);
        assert!(clocks.poll(0x20003, 1, true, 500, 70.0, 0.05));
        assert!(clocks.poll(0x10004, 0, false, 700, 80.0, 0.05));
        let mut encoded = clocks.wrap(core.clone());
        let (loaded_core, loaded) = FactoryClocks::split(&encoded).unwrap();
        assert_eq!(loaded_core, core);
        assert!(loaded.valid(80.0, |handle, owner, late| matches!((handle, owner, late), (0x20003, 1, true) | (0x10004, 0, false))));
        assert!(!loaded.valid(79.0, |_, _, _| true));
        assert_eq!(loaded.wrap(core), encoded);
        encoded[10] ^= 1;
        assert!(FactoryClocks::split(&encoded).is_none());
    }

    #[test]
    fn automation_birth_egress_and_expiry_persist_through_the_game_archive() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        game.players[0].epoch = 14;
        game.rebuild_modifiers(0, false);
        game.tick = 1500;
        let factory = game.spawn_oriented(ROBOTICS_FACTORY, 0, 130.5, 130.5, Game::angle_for_turn(2));
        game.ents[factory].rx = 117.0;
        game.ents[factory].ry = 127.0;
        game.map.pass[0].fill(true);
        let from = game.high;
        let (rng, expected) = (0..100).find_map(|seed| {
            let rng = crate::native_rng9::NativeRng::from_state(seed);
            let mut expected = rng;
            expected.next_word15();
            let heavy = expected.next_word15() as usize % LATE_UNITS.len() == 2;
            expected.next_word15();
            heavy.then_some((rng, expected))
        }).unwrap();
        game.source_birth.rng = rng;
        let bank = game.players[0].res;
        game.tick_factory_automation();
        assert_eq!(game.source_birth.rng, expected);
        assert_eq!(game.players[0].res, bank);
        assert_eq!(game.high, from + 1);
        let actor = game.ents[from];
        assert_eq!(actor.def, crate::heavy_mounted3_61::ID);
        assert_eq!(actor.order, O_FACTORY_EGRESS);
        assert_eq!([actor.x, actor.y], map_point(game.ents[factory], BIRTH));
        assert_eq!([actor.ox, actor.oy], map_point(game.ents[factory], EXIT));
        let old = game.state_hash();
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        assert_eq!(loaded.state_hash(), old);
        loaded.ents[factory].rx = 118.0;
        for _ in 0..100 {
            if loaded.ents[from].order != O_FACTORY_EGRESS { break; }
            loaded.advance_factory_egress(from, loaded.ents[from], 0.05);
        }
        assert_eq!(loaded.ents[from].order, super::super::O_MOVE);
        assert_eq!([loaded.ents[from].x, loaded.ents[from].y], [actor.ox, actor.oy]);
        assert_eq!([loaded.ents[from].ox, loaded.ents[from].oy], [117.0, 127.0]);
        loaded.kill(factory);
        assert!(loaded.factory_clocks.0.is_empty());
    }

    #[test]
    fn automation_exit_checks_center_and_ring_order_before_declining() {
        let mut probes = Vec::new();
        assert_eq!(probe_exit([30.0, 40.0], |x, y| {
            probes.push([x, y]);
            x == 29.5 && y == 39.5
        }), Some([29.5, 39.5]));
        assert_eq!(probes, [[30.0, 40.0], [30.5, 40.0], [30.0, 40.5], [29.5, 40.0],
            [30.0, 39.5], [30.5, 40.5], [29.5, 40.5], [29.5, 39.5]]);
        assert_eq!(probe_exit([30.0, 40.0], |_, _| false), None);
        let mut edge = Vec::new();
        assert_eq!(probe_exit([N as f32 + 1.0, -4.0], |x, y| {
            edge.push([x, y]);
            edge.len() == 3
        }), Some([N as f32 - 2.0, 0.5]));
        assert_eq!(edge, [[N as f32 - 2.0, 0.0], [N as f32 - 1.5, 0.0], [N as f32 - 2.0, 0.5]]);
    }
}
