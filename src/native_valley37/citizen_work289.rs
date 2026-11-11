//! Latest accepted Citizen visual tuple. Warm Animator synchronization remains
//! outside this actor-request clock; cold recovery uses simulation now-start.
use std::collections::BTreeMap;
use crate::{defs::WORKER, game::{Ent, Game, NONE, O_BUILD, O_CAPTURE, O_IDLE, O_RECT_HUNT}};

pub(super) const TAG: &[u8; 4] = b"CWV1";
const ROW_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CitizenWorkVisual {
    pub(crate) state: u32,
    pub(crate) span: f32,
    pub(crate) started: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct MotionObservation {
    forward: f32,
    order: u8,
    idle_ready: bool,
}

fn valid_tuple(state: u32, span: f32) -> bool {
    let expected = match state {
        25..=28 => 0.8_f32.to_bits(),
        0 | 2 | 3 | 7 | 19 | 20 | 22..=24 | 29 | 30 => 0,
        _ => return false,
    };
    span.to_bits() == expected
}

#[derive(Default)]
pub(crate) struct State {
    rows: BTreeMap<u32, CitizenWorkVisual>,
    // An update-slot observation, not part of the accepted native tuple.
    requested_tick: BTreeMap<u32, u32>,
}

impl State {
    fn request(&mut self, handle: u32, state: u32, span: f32, now: f32, tick: u32) {
        self.requested_tick.insert(handle, tick);
        let row = self.rows.entry(handle).or_insert(CitizenWorkVisual { state, span, started: now });
        if row.state != state || row.span.to_bits() != span.to_bits() {
            *row = CitizenWorkVisual { state, span, started: now };
        }
    }

    pub(crate) fn forget(&mut self, handle: u32) {
        self.rows.remove(&handle);
        self.requested_tick.remove(&handle);
    }

    fn words(&self) -> impl Iterator<Item = u32> + '_ {
        [u32::from_le_bytes(*TAG), self.rows.len() as u32].into_iter()
            .chain(self.rows.iter().flat_map(|(&handle, row)|
                [handle, row.state, row.span.to_bits(), row.started.to_bits()]))
    }

    pub(crate) fn section(&self) -> Option<Vec<u8>> {
        (!self.rows.is_empty()).then(|| self.words().flat_map(u32::to_le_bytes).collect())
    }

    pub(crate) fn hash(&self, mix: &mut impl FnMut(u32)) {
        if !self.rows.is_empty() { self.words().for_each(mix); }
    }
}

impl Game {
    fn citizen_work_enabled(&self, index: usize) -> bool {
        index < self.high && self.ents[index].alive && self.ents[index].def == WORKER
            && self.rect_population.is_some() && self.rect_ai_clock.is_some()
    }

    pub(crate) fn request_citizen_work(&mut self, index: usize, state: u32, span: f32) -> bool {
        if !self.citizen_work_enabled(index) || !valid_tuple(state, span) { return false; }
        let now = self.sim_elapsed();
        if !now.is_finite() || now.is_sign_negative() { return false; }
        self.rect_citizen_work.request(self.h(index), state, span, now, self.tick);
        true
    }

    pub(crate) fn citizen_work_requested_tick(&self, index: usize) -> Option<u32> {
        self.citizen_work_enabled(index).then(|| self.rect_citizen_work.requested_tick.get(&self.h(index)).copied()).flatten()
    }

    pub(crate) fn retain_citizen_work(&mut self, index: usize) -> bool {
        if !self.citizen_work_enabled(index) { return false; }
        let handle = self.h(index);
        // An old archive can have no accepted visual tuple. Suppress the
        // generic fallback without manufacturing a start time in that case.
        let retained = self.rect_citizen_work.rows.contains_key(&handle);
        self.rect_citizen_work.requested_tick.insert(handle, self.tick);
        retained
    }

    pub(crate) fn init_citizen_work_visuals(&mut self) {
        for index in 0..self.high { self.request_citizen_work(index, 0, 0.0); }
    }

    pub(crate) fn citizen_work_motion(&self, index:usize, actor:&Ent) -> MotionObservation {
        let native = self.valley_routes.active.get(&self.h(index)).and_then(|route| route.native);
        let braking = Self::has_rect_idle_brake(actor);
        let forward = if braking { actor.qt }
            else { native.map_or_else(|| super::ground_steering::Precision::TowardZero
                .planar_speed([actor.vx, actor.vy]), |motion| motion.forward) };
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let force_inactive = !native.and_then(|motion| motion.force).is_some_and(|force| force.stamp > 0.0);
        #[cfg(not(feature = "ee2-valley215-citizen-contact"))]
        let force_inactive = true;
        MotionObservation {
            forward,
            order: actor.order,
            // 0085E260 tests the force stamp, not the retained force vector.
            idle_ready: actor.vx == 0.0 && actor.vy == 0.0 && !braking && force_inactive
                && native.map_or(actor.rot == actor.prot, |motion| motion.turn == 0.0),
        }
    }

    pub(crate) fn finish_citizen_work_update(&mut self, index:usize, actor:&Ent, before:MotionObservation, engaged:bool) {
        if !self.citizen_work_enabled(index) || actor.hp <= 0.0 { return; }
        // Combat/capture have their existing dedicated presentation clocks.
        // Do not replay a stale economic tuple over those controllers. A later
        // work request must start anew after the intervening source state.
        if engaged || matches!(actor.order, O_RECT_HUNT | O_BUILD | O_CAPTURE) {
            self.rect_citizen_work.forget(self.h(index));
            return;
        }
        if self.citizen_work_requested_tick(index) == Some(self.tick) { return; }
        // The ordinary Move handler (minor type0) chooses a motion/idle state.
        // Deposit's minor type1 and work bodies already marked this update.
        if before.order == O_IDLE && !before.idle_ready {
            self.retain_citizen_work(index);
            return;
        }
        // 007F304F reads the raw MoveAbility descriptor. Leader/AOE movement
        // multipliers are applied separately by 007F2600, not by this selector.
        // Follow the portable core's explicit CHOP profile. The original
        // ambient rounding context remains outside the detached controls.
        let ratio = super::ground_steering::Precision::TowardZero.stored_division(
            before.forward, self.modifiers[actor.team as usize][WORKER as usize][2]);
        // 007F304C stores forward/max speed, and 007F309F compares strictly
        // against binary32 .01. Citizen has no Run state, so it uses Walk.
        let walking = before.order != O_IDLE && ratio > f32::from_bits(0x3c23_d70a);
        self.request_citizen_work(index, if walking {2} else {0}, 0.0);
    }

    pub(crate) fn citizen_work_visual(&self, handle: u32) -> Option<CitizenWorkVisual> {
        let index = self.get(handle)?;
        self.citizen_work_enabled(index).then(|| self.rect_citizen_work.rows.get(&handle).copied()).flatten()
    }

    pub(crate) fn restore_citizen_work(&mut self, bytes: &[u8], high: usize) -> Option<()> {
        let word = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at.checked_add(4)?)?.try_into().ok()?));
        if !bytes.starts_with(TAG) || self.rect_population.is_none() || self.rect_ai_clock.is_none() || high > self.ents.len() { return None; }
        let count = word(4)? as usize;
        let citizens = self.ents[..high].iter().filter(|actor| actor.alive && actor.def == WORKER).count();
        if count == 0 || count > citizens || bytes.len() != 8_usize.checked_add(count.checked_mul(ROW_BYTES)?)? { return None; }
        let (now, mut last, mut restored) = (self.sim_elapsed(), None, State::default());
        if !now.is_finite() || now.is_sign_negative() { return None; }
        for row in 0..count {
            let at = 8 + row * ROW_BYTES;
            let (handle, state, span, started) = (word(at)?, word(at + 4)?, f32::from_bits(word(at + 8)?), f32::from_bits(word(at + 12)?));
            let index = self.get(handle)?;
            if index >= high || self.ents[index].def != WORKER || last.is_some_and(|prior| handle <= prior)
                || !valid_tuple(state, span) || !started.is_finite() || started.is_sign_negative() || started > now { return None; }
            restored.rows.insert(handle, CitizenWorkVisual { state, span, started });
            last = Some(handle);
        }
        self.rect_citizen_work = restored;
        Some(())
    }
}

#[no_mangle]
pub extern "C" fn citizen_work_visual_state(handle: u32) -> u32 {
    super::active_game().and_then(|game| game.citizen_work_visual(handle)).map_or(NONE, |row| row.state)
}

#[no_mangle]
pub extern "C" fn citizen_work_visual_started(handle: u32) -> f32 {
    super::active_game().and_then(|game| game.citizen_work_visual(handle)).map_or(-1.0, |row| row.started)
}

#[no_mangle]
pub extern "C" fn citizen_work_visual_span(handle: u32) -> f32 {
    super::active_game().and_then(|game| game.citizen_work_visual(handle)).map_or(-1.0, |row| row.span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_valley37::{fresh239, rectangular_world};

    fn fixture(adaptive: bool) -> Game {
        rectangular_world::spatial_tests::prepare_authenticated_inputs();
        let input = crate::native_valley37::private_test_fixtures::read("animal-manager.anm");
        let ptr = rectangular_world::native_rect_animal_alloc(input.len());
        assert!(!ptr.is_null());
        unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), ptr, input.len()); }
        assert_eq!(rectangular_world::native_rect_animal_prepare(), 1);
        assert_eq!(if adaptive { fresh239::native_rect_commit_adaptive(171, 1) }
            else { fresh239::native_rect_commit(171, 1) }, 1);
        let game = crate::g();
        game.save();
        Game::load(&game.savebuf).unwrap()
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_request_coalesces_and_ephemeral_touches_do_not_change_tuple() {
        let mut game = fixture(true);
        let worker = 1;
        assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().state, 0);
        assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().started, 0.0);
        assert!(!game.request_citizen_work(0, 23, 0.0));
        assert!(!game.request_citizen_work(usize::MAX, 23, 0.0));
        let rng = (game.rng.0, game.source_birth.rng.state());
        game.tick = 20;
        assert!(game.request_citizen_work(worker, 23, 0.0));
        let first = game.citizen_work_visual(game.h(worker)).unwrap();
        game.tick = 30;
        let before = game.rect_citizen_work.section().unwrap();
        let hash = game.state_hash();
        assert!(game.request_citizen_work(worker, 23, 0.0));
        assert_eq!(game.citizen_work_requested_tick(worker), Some(30));
        assert_eq!(game.citizen_work_visual(game.h(worker)), Some(first));
        assert_eq!(game.rect_citizen_work.section().unwrap(), before);
        assert_eq!(game.state_hash(), hash);
        game.tick = 31;
        let hash = game.state_hash();
        assert!(game.retain_citizen_work(worker));
        assert_eq!(game.citizen_work_requested_tick(worker), Some(31));
        assert_eq!(game.rect_citizen_work.section().unwrap(), before);
        assert_eq!(game.state_hash(), hash);
        assert!(!game.retain_citizen_work(0));
        assert!(!game.request_citizen_work(worker, 23, 0.8));
        assert!(!game.request_citizen_work(worker, 21, 0.8));
        game.tick = 40;
        assert!(game.request_citizen_work(worker, 28, 0.8));
        let changed = game.citizen_work_visual(game.h(worker)).unwrap();
        assert_eq!((changed.state, changed.span, changed.started), (28, 0.8, game.sim_elapsed()));
        assert_eq!((game.rng.0, game.source_birth.rng.state()), rng);
        let handle = game.h(worker);
        crate::install_game(game);
        assert_eq!(citizen_work_visual_state(handle), 28);
        assert_eq!(citizen_work_visual_started(handle), changed.started);
        assert_eq!(citizen_work_visual_span(handle), 0.8);
        assert_eq!(citizen_work_visual_state(handle ^ 0x10000), NONE);
        assert_eq!(citizen_work_visual_started(handle ^ 0x10000), -1.0);
        assert_eq!(citizen_work_visual_span(0), -1.0);
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_optional_archive_preserves_empty_old_identity_modes() {
        for owned in [false, true] {
            let mut game = fixture(true);
            game.rect_citizen_work = State::default();
            if !owned { game.rect_native_ids = None; }
            game.save();
            let old = game.savebuf.clone();
            assert_eq!(u32::from_le_bytes(old[4..8].try_into().unwrap()), if owned {20} else {13});
            let mut cold = Game::load(&old).unwrap();
            cold.save();
            assert_eq!(cold.savebuf, old);
            assert_eq!(cold.citizen_work_visual(cold.h(1)), None);
            assert!(!cold.retain_citizen_work(1));
            assert_eq!(cold.citizen_work_requested_tick(1), Some(cold.tick));
            let mut moving = cold.ents[1];
            moving.order = crate::game::O_MOVE;
            moving.vx = 1.0;
            let before = cold.citizen_work_motion(1, &moving);
            cold.finish_citizen_work_update(1, &moving, before, false);
            assert_eq!(cold.citizen_work_visual(cold.h(1)), None);
            cold.save();
            assert_eq!(cold.savebuf, old, "no-request retention must not invent an old archive timestamp");
            assert!(cold.request_citizen_work(1, 23, 0.0));
            cold.save();
            let current = cold.savebuf.clone();
            assert_eq!(u32::from_le_bytes(current[4..8].try_into().unwrap()), 21);
            let mut restored = Game::load(&current).unwrap();
            assert_eq!(restored.rect_native_ids.is_some(), owned);
            assert_eq!(restored.citizen_work_visual(restored.h(1)), cold.citizen_work_visual(cold.h(1)));
            assert_eq!(restored.citizen_work_requested_tick(1), None);
            assert_eq!(restored.state_hash(), cold.state_hash());
            restored.save();
            assert_eq!(restored.savebuf, current);
        }
        let mut nonadaptive = fixture(false);
        nonadaptive.save();
        let old = nonadaptive.savebuf.clone();
        assert!(u32::from_le_bytes(old[4..8].try_into().unwrap()) <= 12);
        assert!(!nonadaptive.request_citizen_work(1, 23, 0.0));
        assert!(nonadaptive.rect_citizen_work.section().is_none());
        nonadaptive.save();
        assert_eq!(nonadaptive.savebuf, old);
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_motion_uses_source_cutoff_and_preintegration_stop_context() {
        let mut game = fixture(true);
        let worker = 1;
        game.modifiers[0][WORKER as usize][2] = 2.0;
        let mut actor = game.ents[worker];
        actor.order = crate::game::O_MOVE;
        // Native motion301 controls use max2: .02/max is exactly the .01
        // cutoff; the next representable forward speed selects Walk.
        for (forward, expected) in [(0.02_f32, 0), (f32::from_bits(0.02_f32.to_bits()+1), 2),
            (-0.02_f32, 0), (-0.0_f32, 0)] {
            game.tick += 1;
            game.finish_citizen_work_update(worker, &actor,
                MotionObservation {forward, order:crate::game::O_MOVE, idle_ready:false}, false);
            assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().state, expected);
        }
        game.tick += 1;
        game.modifiers[0][WORKER as usize][2] = f32::from_bits(0x3f90_0000);
        game.finish_citizen_work_update(worker, &actor, MotionObservation {
            forward:f32::from_bits(0x3c38_51ec), order:crate::game::O_MOVE, idle_ready:false,
        }, false);
        assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().state, 0,
            "native CHOP boundary must not use the nearest quotient one ULP above .01");
        game.tick += 1;
        game.request_citizen_work(worker, 30, 0.0);
        let carrying = game.citizen_work_visual(game.h(worker)).unwrap();
        game.tick += 1;
        actor.order = O_IDLE;
        actor.vx = 0.1;
        let before_brake = game.citizen_work_motion(worker, &actor);
        actor.vx = 0.0; // the final brake step has now integrated
        actor.rot = actor.prot;
        game.finish_citizen_work_update(worker, &actor, before_brake, false);
        assert_eq!(game.citizen_work_visual(game.h(worker)), Some(carrying));
        game.tick += 1;
        let stationary = game.citizen_work_motion(worker, &actor);
        game.finish_citizen_work_update(worker, &actor, stationary, false);
        assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().state, 0);
        game.tick += 1;
        actor.order = crate::game::O_MOVE;
        actor.vx = 0.1;
        let arriving = game.citizen_work_motion(worker, &actor);
        actor.order = O_IDLE;
        actor.vx = 0.0;
        game.finish_citizen_work_update(worker, &actor, arriving, false);
        assert_eq!(game.citizen_work_visual(game.h(worker)).unwrap().state, 2,
            "arrival does not replace the Move request selected before integration");
    }

    #[cfg(feature = "ee2-valley215-citizen-contact")]
    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_native_observation_reads_yaw_and_force_stamp_not_compatibility_proxies() {
        use crate::native_valley37::atlas_runtime::{ground_motion::NativeMotion,
            route_motion::RouteProgress, contact_kernel::lifecycle::ForceState};
        let mut game = fixture(true);
        let mut actor = game.ents[1];
        actor.order = O_IDLE;
        actor.vx = 0.0;
        actor.vy = 0.0;
        actor.rot = actor.prot + 0.1;
        let handle = game.h(1);
        // Supplied motion records isolate the original field selector. This
        // is not a claim about the portable route lifetime after Stop.
        for (turn, stamp, retained, expected) in [(0.1, 0.0, 0.0, false),
            (0.0, 0.0, 0.0, true), (0.0, 1.0, 0.0, false), (0.0, -1.0, 0.1, true)] {
            game.valley_routes.active.insert(handle, RouteProgress {
                actor:handle, issued_tick:game.tick, final_goal:[actor.x, actor.y],
                waypoints:vec![[actor.x, actor.y];3], next:1, atlas_generation:0,
                native:Some(NativeMotion {forward:0.3, turn, force:Some(ForceState {
                    retained:[retained,0.0,0.0], last:-1.0, stamp, origin:[actor.x,actor.y],
                })}),
            });
            let observed = game.citizen_work_motion(1, &actor);
            assert_eq!(observed.forward, 0.3);
            assert_eq!(observed.idle_ready, expected);
        }
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_malformed_rows_reject_without_mutating_the_prior_map() {
        let mut game = fixture(true);
        game.tick = 20;
        assert!(game.request_citizen_work(1, 23, 0.0));
        assert!(game.request_citizen_work(4, 30, 0.0));
        let section = game.rect_citizen_work.section().unwrap();
        let mut invalid = Vec::new();
        for (at, value) in [(8, game.h(1) ^ 0x10000), (8, 0), (12, 21), (12, 31),
            (16, 0.8_f32.to_bits()), (16, (-0.0_f32).to_bits()), (16, f32::NAN.to_bits()),
            (20, (game.sim_elapsed() + 1.0).to_bits()), (20, (-0.0_f32).to_bits()),
            (20, f32::INFINITY.to_bits()), (24, game.h(1)), (4, 0), (4, 3)] {
            let mut bytes = section.clone();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            invalid.push(bytes);
        }
        invalid.push(section[..section.len() - 1].to_vec());
        let mut trailing = section.clone(); trailing.extend_from_slice(&0_u32.to_le_bytes()); invalid.push(trailing);
        game.save();
        let archived = game.savebuf.clone();
        let table = 172 + u32::from_le_bytes(archived[8..12].try_into().unwrap()) as usize;
        let start = table + 44 + (0..10).map(|i|
            u32::from_le_bytes(archived[table + i * 4..table + i * 4 + 4].try_into().unwrap()) as usize).sum::<usize>();
        let hash = game.state_hash();
        for bytes in invalid {
            assert!(game.restore_citizen_work(&bytes, game.high).is_none());
            assert_eq!(game.rect_citizen_work.section(), Some(section.clone()));
            assert_eq!(game.state_hash(), hash);
            let mut wire = archived[..start].to_vec();
            wire[table + 40..table + 44].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
            wire.extend_from_slice(&bytes);
            let digest = crate::native_valley37::hash::sha256(&wire);
            wire.extend_from_slice(&digest);
            assert!(Game::load(&wire).is_none(), "valid-digest malformed CWV must reject transactionally");
        }
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn citizen_work_death_and_normal_boarding_retire_generation_rows() {
        let mut game = fixture(true);
        let handle = game.h(1);
        assert!(game.request_citizen_work(1, 24, 0.0));
        game.kill(1);
        assert_eq!(game.citizen_work_visual(handle), None);
        assert!(!game.rect_citizen_work.rows.contains_key(&handle));
        assert!(!game.rect_citizen_work.requested_tick.contains_key(&handle));
        let citizen = game.spawn(WORKER, 0, 91.0, 12.0);
        assert_eq!(citizen, 1);
        let new_handle = game.h(citizen);
        assert_ne!(new_handle, handle);
        assert!(game.request_citizen_work(citizen, 23, 0.0));
        assert_eq!(game.citizen_work_visual(handle), None);
        let host = game.spawn(crate::defs::UNIVERSITY, 0, 91.0, 12.0);
        game.apply(0, crate::cmd::C_BOARD, 0.0, 0.0, game.h(host) as i32, &[new_handle]);
        for _ in 0..60 {
            assert!(game.tick_with_adaptive_source_slot(30, 30, 1.0));
            if game.get(new_handle).is_none() { break; }
        }
        assert!(game.get(new_handle).is_none());
        assert_eq!(game.ents[host].ncargo, 1);
        assert!(!game.rect_citizen_work.rows.contains_key(&new_handle));
        assert!(!game.rect_citizen_work.requested_tick.contains_key(&new_handle));
    }
}
