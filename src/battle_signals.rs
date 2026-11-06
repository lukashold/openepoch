//! Local battle presentation for the UP ruleset.

use crate::game::Ent;
use std::collections::VecDeque;

type Point = [f32; 2];
const SEATS: usize = 4;
const INITIAL_RADIUS_SQ: f32 = 200.0;
const MAX_RADIUS_SQ: f64 = 300.0;
const PARTICIPANT_SECONDS: f32 = 180.0;
const EMPTY_SECONDS: f32 = 4.0;
const WARNING_SECONDS: f32 = 40.0;
const MARKER_SECONDS: f32 = 15.0;
const MARKER_GAP_SECONDS: f32 = 5.0;
const MARKER_LIMIT: usize = 10;

#[derive(Clone, Copy)]
pub(crate) struct Actor {
    handle: u32,
    seat: usize,
    point: Point,
}

impl Actor {
    pub(crate) fn new(index: usize, entity: &Ent) -> Self {
        Self {
            handle: (u32::from(entity.gen) << 16) | index as u32,
            seat: usize::from(entity.team),
            point: [entity.x, entity.y],
        }
    }
}

fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    dx * dx + dy * dy
}

fn resolve(entities: &[Ent], seat: usize, handle: u32) -> Option<Point> {
    let entity = entities.get((handle & 0xffff) as usize)?;
    (entity.alive && usize::from(entity.team) == seat && u32::from(entity.gen) == handle >> 16)
        .then_some([entity.x, entity.y])
}

struct Member {
    handle: u32,
    remaining: f32,
}

#[derive(Default)]
struct Participants {
    warned: bool,
    members: Vec<Member>,
}

struct Group {
    point: Point,
    radius_sq: f32,
    empty: f32,
    seats: [Participants; SEATS],
}

impl Group {
    fn new(point: Point) -> Self {
        Self { point, radius_sq: INITIAL_RADIUS_SQ, empty: EMPTY_SECONDS, seats: Default::default() }
    }

    fn update(&mut self, elapsed: f32, entities: &[Ent]) -> bool {
        let (mut x, mut y, mut count) = (0.0f32, 0.0f32, 0u32);
        for (seat, participants) in self.seats.iter_mut().enumerate() {
            participants.members.retain_mut(|member| {
                member.remaining -= elapsed;
                if member.remaining <= 0.0 { return false; }
                let Some(point) = resolve(entities, seat, member.handle) else { return false; };
                x += point[0];
                y += point[1];
                count += 1;
                true
            });
        }
        if count == 0 {
            self.empty -= elapsed;
            return self.empty > 0.0;
        }
        self.point = [(f64::from(x) / f64::from(count)) as f32, (f64::from(y) / f64::from(count)) as f32];
        // All OpenEpoch actor seats are non-neutral. The source skips neutral in this pass.
        for (seat, participants) in self.seats.iter_mut().enumerate() {
            participants.members.retain(|member| {
                let Some(point) = resolve(entities, seat, member.handle) else { return false; };
                let range = distance_sq(point, self.point);
                if range <= f64::from(self.radius_sq) { return true; }
                if range >= MAX_RADIUS_SQ { return false; }
                self.radius_sq = range as f32;
                true
            });
        }
        true
    }
}

#[repr(C)]
struct Marker {
    id: u32,
    point: Point,
    remaining: f32,
}

#[derive(Default)]
pub(crate) struct BattleSignals {
    groups: Vec<Group>,
    pending: VecDeque<Point>,
    markers: Vec<Marker>,
    warning: f32,
    marker_wait: f32,
    marker_id: u32,
    cycle_index: usize,
}

impl BattleSignals {
    fn observe(&mut self, actor: Actor, victim: bool) -> (usize, bool) {
        let mut selected = None;
        let mut membership = None;
        for (index, group) in self.groups.iter().enumerate() {
            if let Some(member) = group.seats[actor.seat].members.iter().position(|member| member.handle == actor.handle) {
                selected = Some(index);
                membership = Some(member);
                break;
            }
            // Original chooses the last spatial match, unless exact membership is found.
            if distance_sq(actor.point, group.point) < f64::from(group.radius_sq) {
                selected = Some(index);
            }
        }
        let index = selected.unwrap_or_else(|| {
            self.groups.push(Group::new(actor.point));
            self.groups.len() - 1
        });
        let group = &mut self.groups[index];
        if let Some(member) = membership {
            group.seats[actor.seat].members[member].remaining = PARTICIPANT_SECONDS;
        } else {
            if group.seats.iter().all(|seat| seat.members.is_empty()) { group.point = actor.point; }
            group.empty = EMPTY_SECONDS;
            group.seats[actor.seat].members.push(Member { handle: actor.handle, remaining: PARTICIPANT_SECONDS });
        }
        let seat = &mut group.seats[actor.seat];
        let first = victim && !seat.warned;
        seat.warned |= victim;
        (index, first)
    }

    /// Returns the independent local audio alert, which is throttled across groups.
    pub(crate) fn hit(&mut self, local: u8, victim: Actor, origin: Option<Actor>, killed: bool) -> bool {
        let (group, first) = self.observe(victim, true);
        if killed {
            self.groups[group].seats[victim.seat].members.retain(|member| member.handle != victim.handle);
        }
        let victim_point = self.groups[group].point;
        let point = origin.map_or(victim_point, |attacker| {
            let (group, _) = self.observe(attacker, false);
            self.groups[group].point
        });
        if victim.seat != usize::from(local) { return false; }
        if first { self.pending.push_back(point); }
        if self.warning > 0.0 { return false; }
        self.warning = WARNING_SECONDS;
        true
    }

    pub(crate) fn update_groups(&mut self, elapsed: f32, entities: &[Ent]) {
        if self.warning >= 0.0 { self.warning -= elapsed; }
        self.groups.retain_mut(|group| group.update(elapsed, entities));
    }

    pub(crate) fn update_markers(&mut self, elapsed: f32) {
        if self.marker_wait >= 0.0 { self.marker_wait -= elapsed; }
        for marker in &mut self.markers {
            if marker.remaining > 0.0 { marker.remaining -= elapsed; }
        }
        if self.marker_wait > 0.0 { return; }
        let Some(point) = self.pending.pop_front() else { return; };
        if self.markers.len() == MARKER_LIMIT { self.markers.remove(0); }
        self.markers.push(Marker { id: self.marker_id, point, remaining: MARKER_SECONDS });
        self.marker_wait = MARKER_GAP_SECONDS;
        self.marker_id = self.marker_id.wrapping_add(1);
        self.cycle_index = self.markers.len() - 1;
    }

    fn cycle(&mut self) -> Option<&Point> {
        let marker = self.markers.get(self.cycle_index)?;
        self.cycle_index = self.cycle_index.checked_sub(1).unwrap_or(self.markers.len() - 1);
        Some(&marker.point)
    }
}

#[no_mangle]
pub extern "C" fn battle_signals_enable() {
    crate::g().battle_signals.get_or_insert_with(BattleSignals::default);
}

#[no_mangle]
pub extern "C" fn battle_signal_count() -> u32 {
    crate::g().battle_signals.as_ref().map_or(0, |state| state.markers.len() as u32)
}

#[no_mangle]
pub extern "C" fn battle_signal_ptr() -> *const u32 {
    crate::g().battle_signals.as_ref().map_or(core::ptr::null(), |state| state.markers.as_ptr().cast())
}

#[no_mangle]
pub extern "C" fn battle_signal_cycle() -> *const f32 {
    crate::g().battle_signals.as_mut().and_then(BattleSignals::cycle).map_or(core::ptr::null(), |point| point.as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(handle: u32, seat: usize, point: Point) -> Actor {
        Actor { handle, seat, point }
    }

    #[test]
    fn first_local_victim_notifies_once_and_attacker_group_supplies_location() {
        let mut battle = BattleSignals::default();
        let attacker = actor(0x10000, 1, [30.0, 40.0]);
        let victim = actor(0x10001, 0, [100.0, 100.0]);
        assert!(!battle.hit(0, attacker, None, false));
        assert!(battle.pending.is_empty());
        assert!(battle.hit(0, victim, Some(attacker), false));
        assert!(!battle.hit(0, victim, Some(attacker), false));
        assert_eq!(battle.pending.iter().copied().collect::<Vec<_>>(), [[30.0, 40.0]]);
        assert_eq!(battle.groups.len(), 2);
        battle.update_groups(39.0, &[]);
        assert!(!battle.hit(0, victim, None, false));
        battle.update_groups(1.0, &[]);
        assert!(battle.hit(0, victim, None, false));
        assert_eq!(battle.pending.len(), 2);
    }

    #[test]
    fn lethal_hit_leaves_grace_and_a_reused_handle_does_not_inherit_membership() {
        let mut battle = BattleSignals::default();
        let dead = actor(0x10000, 0, [24.0, 24.0]);
        assert!(battle.hit(0, dead, None, true));
        assert!(battle.groups[0].seats[0].members.is_empty());
        battle.update_groups(3.5, &[]);
        assert_eq!(battle.groups.len(), 1);
        battle.update_groups(0.5, &[]);
        assert!(battle.groups.is_empty());
        let alive = actor(0x20000, 0, [24.0, 24.0]);
        assert!(!battle.hit(0, alive, None, false));
        assert_eq!(battle.pending.len(), 2);
        assert_eq!(battle.groups[0].seats[0].members[0].handle, 0x20000);
    }

    #[test]
    fn spatial_join_uses_strict_radius_and_the_last_eligible_group() {
        let mut battle = BattleSignals::default();
        let first = actor(0x10000, 0, [100.0, 100.0]);
        let boundary = actor(0x10001, 1, [110.0, 110.0]);
        let overlap = actor(0x10002, 2, [105.0, 105.0]);
        assert_eq!(battle.observe(first, false), (0, false));
        assert_eq!(battle.observe(boundary, false), (1, false));
        assert_eq!(battle.observe(overlap, true), (1, true));
        assert_eq!(battle.observe(Actor { point: [108.0, 108.0], ..first }, true), (0, true));
    }

    #[test]
    fn marker_queue_keeps_ten_and_cycles_newest_first_even_after_expiry() {
        let mut battle = BattleSignals::default();
        battle.pending.extend((0..12).map(|id| [id as f32, 8.0]));
        battle.update_markers(0.0);
        assert_eq!(battle.markers.len(), 1);
        battle.update_markers(4.0);
        assert_eq!(battle.markers.len(), 1);
        battle.update_markers(1.0);
        assert_eq!(battle.markers.len(), 2);
        for _ in 2..12 { battle.update_markers(5.0); }
        assert_eq!(battle.markers.len(), 10);
        assert_eq!(battle.markers[0].id, 2);
        battle.update_markers(15.0);
        assert!(battle.markers.iter().all(|marker| marker.remaining <= 0.0));
        for id in (2..12).rev() { assert_eq!(battle.cycle(), Some(&[id as f32, 8.0])); }
        assert_eq!(battle.cycle(), Some(&[11.0, 8.0]));
    }
}
