//! Follows independently admitted ordinary Valley routes with the portable locomotion integrator.

use super::active_game;
use crate::{
    defs::WORKER,
    game::{Ent, Game, BOARDED, NONE, O_MOVE},
};
use std::collections::BTreeMap;

pub(crate) const MAX_ROUTES: usize = 64;
pub(crate) const MAX_POINTS: usize = 1025;
const INTERMEDIATE_CONTACT_SQ: f32 = 0.1 * 0.1;

#[derive(Default)]
pub(crate) struct RouteMotionState {
    pub(crate) enabled: bool,
    pub(crate) active: BTreeMap<u32, RouteProgress>,
}

pub(crate) struct RouteProgress {
    pub(crate) actor: u32,
    pub(crate) issued_tick: u32,
    pub(crate) final_goal: [f32; 2],
    pub(crate) waypoints: Vec<[f32; 2]>,
    pub(crate) next: u16,
    pub(crate) atlas_generation: u32,
    #[cfg(feature = "ee2-valley204-ground-motion")]
    pub(crate) native: Option<super::ground_motion::NativeMotion>,
}

impl RouteProgress {
    fn matches(&self, ent: &Ent, local: u8) -> bool {
        ent.alive
            && u32::from(ent.gen) == self.actor >> 16
            && ent.hp > 0.0
            && ent.def == WORKER
            && ent.team == local
            && ent.stuck != BOARDED
            && ent.order == O_MOVE
            && ent.otgt == NONE
            && [ent.ox.to_bits(), ent.oy.to_bits()] == self.final_goal.map(f32::to_bits)
    }

    pub(crate) fn valid_for_actor(&self, game: &Game) -> bool {
        game.get(self.actor)
            .is_some_and(|index| self.matches(&game.ents[index], game.local))
    }
}

impl Game {
    pub(crate) fn valley_route_generation(&self) -> Option<u32> {
        if self.initial_ceasefire_active()
            || self.diplomacy.borders.as_ref().is_some_and(|rules| rules.term(0, 1) != 0)
        {
            return None;
        }
        self.valley_atlas_view().map(|view| view.runtime.generation)
    }

    pub(crate) fn install_valley_route(&mut self, progress: RouteProgress) {
        self.prune_valley_routes();
        if self.valley_routes.active.len() < MAX_ROUTES && progress.valid_for_actor(self) {
            self.valley_routes.active.insert(progress.actor, progress);
        }
    }

    pub(crate) fn prune_valley_routes(&mut self) {
        if self.valley_routes.active.is_empty() {
            return;
        }
        let generation = self.valley_route_generation();
        let ents = &self.ents;
        let local = self.local;
        self.valley_routes.active.retain(|&handle, route| {
            generation == Some(route.atlas_generation)
                && ents.get((handle & 0xffff) as usize).is_some_and(|ent| route.matches(ent, local))
        });
    }

    pub(crate) fn clear_valley_route(&mut self, index: usize) {
        self.valley_routes.active.remove(&self.h(index));
    }

    pub(crate) fn valley_route_leg(&mut self, index: usize, ent: &Ent) -> Option<([f32; 2], bool)> {
        let handle = self.h(index);
        let route = self.valley_routes.active.get(&handle)?;
        if !route.matches(ent, self.local) || self.valley_route_generation() != Some(route.atlas_generation) {
            self.clear_valley_route(index);
            return None;
        }
        let route = self.valley_routes.active.get_mut(&handle)?;
        let next = usize::from(route.next);
        let position = route.waypoints[next];
        let distance_sq = (position[0] - ent.x).powi(2) + (position[1] - ent.y).powi(2);
        // Original early advancement needs actual actor contact. Use the
        // portable integrator's tight free-space contact for these legs.
        if next + 1 < route.waypoints.len() && distance_sq < INTERMEDIATE_CONTACT_SQ {
            route.next += 1;
        }
        let next = usize::from(route.next);
        Some((route.waypoints[next], next + 1 == route.waypoints.len()))
    }
}

#[no_mangle]
pub extern "C" fn valley197_enable_motion() -> u32 {
    if active_game().is_none() {
        return 0;
    }
    let game = crate::g();
    if game.tick != 0 || game.valley_route_generation() != Some(0) {
        return 0;
    }
    game.valley_routes.enabled = true;
    1
}

#[no_mangle]
pub extern "C" fn valley197_route_state(actor: u32, field: u32) -> u32 {
    let route = active_game().and_then(|game| game.valley_routes.active.get(&actor));
    route.and_then(|route| match field {
        0 => Some(1),
        1 => Some(u32::from(route.next)),
        2 => Some(route.waypoints.len() as u32),
        3 => Some(route.atlas_generation),
        4 => Some(route.issued_tick),
        #[cfg(feature = "ee2-valley204-ground-motion")]
        5 => Some(u32::from(route.native.is_some())),
        #[cfg(feature = "ee2-valley204-ground-motion")]
        6 => route.native.map(|state| state.forward.to_bits()),
        #[cfg(feature = "ee2-valley204-ground-motion")]
        7 => route.native.map(|state| state.turn.to_bits()),
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        8..=14 => route.native.and_then(|state| state.force).map(|force| force.words()[(field - 8) as usize]),
        _ => None,
    })
    .unwrap_or(if field == 0 { 0 } else { u32::MAX })
}
