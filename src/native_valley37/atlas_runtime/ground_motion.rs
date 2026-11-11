//! Authenticated ordinary Citizen detours using the original free-ground controller.

use super::{route_motion::RouteProgress, AtlasView};
use super::super::ground_steering::{
    citizen_move_args, free_pose, step, terminal_step, GroundClock, GroundInput, Precision,
    CITIZEN_SPEED, CITIZEN_TURN,
};
#[cfg(feature = "ee2-valley215-citizen-contact")]
use super::super::ground_steering::{self, MoveArgs};
use crate::{
    defs::{Def, BARRACKS, DEFS, F_PLANE, LAND, WORKER},
    game::{Ent, Game, BOARDED},
    map::{tile_of, N, T_ROCK, T_SAND},
};
#[cfg(feature = "ee2-valley206-citizen-clearance")]
use crate::game::{MAXE, O_IDLE};
#[cfg(feature = "ee2-valley215-citizen-contact")]
use super::contact_kernel::{self as contact, Footprint, Motion, lifecycle::{
    physical, projected, refresh_stamp, ForceState, PhysicalInput,
}};

const PRECISION: Precision = Precision::Nearest;

#[cfg(feature = "ee2-valley206-citizen-clearance")]
pub(crate) struct CitizenSeparationFrame {
    stepped: [u64; MAXE.div_ceil(64)],
    #[cfg(feature = "ee2-valley215-citizen-contact")]
    contact_checked: [u64; MAXE.div_ceil(64)],
    #[cfg(feature = "ee2-valley215-citizen-contact")]
    corrected: Vec<ContactPair>,
}

#[cfg(feature = "ee2-valley215-citizen-contact")]
struct ContactPair {
    mover: u32,
    blocker: u32,
    center: [f32; 2],
}

#[cfg(feature = "ee2-valley206-citizen-clearance")]
impl CitizenSeparationFrame {
    pub(crate) fn new() -> Self {
        Self {
            stepped: [0; MAXE.div_ceil(64)],
            #[cfg(feature = "ee2-valley215-citizen-contact")]
            contact_checked: [0; MAXE.div_ceil(64)],
            #[cfg(feature = "ee2-valley215-citizen-contact")]
            corrected: Vec::new(),
        }
    }

    pub(crate) fn record_native_step(&mut self, index: usize) {
        self.stepped[index / 64] |= 1 << (index % 64);
    }

    fn took_native_step(&self, index: usize) -> bool {
        self.stepped[index / 64] & (1 << (index % 64)) != 0
    }

    pub(crate) fn clear_pair(&self, left_index: usize, left: &Ent, right_index: usize, right: &Ent) -> bool {
        let (mover_index, mover, idle_index, idle) = match (self.took_native_step(left_index), self.took_native_step(right_index)) {
            (true, false) => (left_index, left, right_index, right),
            (false, true) => (right_index, right, left_index, left),
            _ => return false,
        };
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        {
            // C_DELETE retires the native mobile candidate immediately. The
            // portable soft-push grid still contains the body until the end
            // of this tick. Do not reintroduce that removed Citizen here.
            if self.contact_checked[mover_index / 64] & (1 << (mover_index % 64)) != 0
                && idle.hp <= 0.0 && idle.def == WORKER && idle.team == mover.team
                && idle.order == O_IDLE
            {
                return true;
            }
            let handle = |index: usize, actor: &Ent| u32::from(actor.gen) << 16 | index as u32;
            if citizen_idle_pair(mover, idle) && self.corrected.iter().any(|pair| {
                pair.mover == handle(mover_index, mover)
                    && pair.blocker == handle(idle_index, idle)
                    && pair.center == [idle.x, idle.y]
            }) {
                return true;
            }
        }
        #[cfg(not(feature = "ee2-valley215-citizen-contact"))]
        let _ = (mover_index, idle_index);
        idle.x == idle.px && idle.y == idle.py
            && citizen_pair_outside_contact(mover, idle, [mover.x, mover.y], false)
    }
}

#[cfg(feature = "ee2-valley206-citizen-clearance")]
fn citizen_pair_outside_contact(mover: &Ent, idle: &Ent, point: [f32; 2], waypoint: bool) -> bool {
    if !citizen_idle_pair(mover, idle) {
        return false;
    }
    // 00848C70 / 00848D50 use two .125 Citizen extents, each padded by
    // .05 for the waypoint query. Leave rounding margins around both bounds.
    let bound = if waypoint { 0.36_f64 } else { 0.26_f64 };
    let dx = f64::from(idle.x) - f64::from(point[0]);
    let dy = f64::from(idle.y) - f64::from(point[1]);
    dx * dx + dy * dy > bound * bound
}

#[cfg(feature = "ee2-valley206-citizen-clearance")]
fn citizen_idle_pair(mover: &Ent, idle: &Ent) -> bool {
    mover.def == WORKER && idle.def == WORKER && mover.team == idle.team
        && mover.alive && idle.alive && mover.stuck != BOARDED && idle.stuck != BOARDED
        && mover.hp > 0.0 && idle.hp > 0.0 && idle.order == O_IDLE
        && idle.vx.abs() <= f32::MIN_POSITIVE && idle.vy.abs() <= f32::MIN_POSITIVE
}

#[derive(Clone, Copy)]
pub(crate) struct NativeMotion {
    pub(crate) forward: f32,
    pub(crate) turn: f32,
    #[cfg(feature = "ee2-valley215-citizen-contact")]
    pub(crate) force: Option<ForceState>,
}

impl NativeMotion {
    pub(crate) fn start(actor: &Ent) -> Option<Self> {
        let state = Self {
            forward: actor.vx.hypot(actor.vy),
            turn: 0.0,
            #[cfg(feature = "ee2-valley215-citizen-contact")]
            force: Some(ForceState::default()),
        };
        state.valid().then_some(state)
    }

    pub(crate) fn valid(self) -> bool {
        (0.0..=CITIZEN_SPEED + 0.00001).contains(&self.forward)
            && self.turn.is_finite()
            && self.turn.abs() <= CITIZEN_TURN
    }

    pub(crate) fn matches(self, actor: &Ent) -> bool {
        self.valid() && (actor.vx.hypot(actor.vy) - self.forward).abs() < 0.00001
    }
}

impl Game {
    pub(crate) fn valley_fallback_motion(&mut self, index: usize) {
        let handle = self.h(index);
        if let Some(route) = self.valley_routes.active.get_mut(&handle) {
            route.native = None;
        }
    }

    fn valley_motion_obstacles<'a>(&'a self, index: usize, actor: &'a Ent, position: [f32; 2], _waypoint: bool) -> impl Iterator<Item = (usize, &'a Ent)> {
        let radius = self.entity_radius(actor);
        self.ents[..self.high].iter().enumerate().filter(move |&(other, e)| {
            if other == index || !e.alive || e.hp <= 0.0 || e.stuck == BOARDED {
                return false;
            }
            #[cfg(feature = "ee2-valley206-citizen-clearance")]
            if citizen_pair_outside_contact(actor, e, position, _waypoint) {
                return false;
            }
            let def = &DEFS[e.def as usize];
            !def.building
                && def.dom == LAND
                && !def.has(F_PLANE)
                && !((e.x - position[0]).powi(2) + (e.y - position[1]).powi(2)
                    > (radius + self.entity_radius(e)).powi(2))
        })
    }

    fn valley_motion_actor_clear(&self, index: usize, actor: &Ent, position: [f32; 2], waypoint: bool) -> bool {
        self.valley_motion_obstacles(index, actor, position, waypoint).next().is_none()
    }

    #[cfg(feature = "ee2-valley215-citizen-contact")]
    fn valley_contact_candidate<'a>(&'a self, index: usize, actor: &'a Ent, point: [f32; 2]) -> Option<Option<(usize, &'a Ent)>> {
        let mut candidates = self.valley_motion_obstacles(index, actor, point, false);
        let Some((other, idle)) = candidates.next() else { return Some(None); };
        let ordinary = citizen_idle_pair(actor, idle)
            && self.valley_motion_unmodified(other)
            && self.modifiers[idle.team as usize][WORKER as usize][2] == CITIZEN_SPEED;
        (ordinary && candidates.next().is_none()).then_some(Some((other, idle)))
    }

    #[cfg(feature = "ee2-valley215-citizen-contact")]
    fn valley_contact_frame(
        &self, index: usize, actor: &Ent, input: GroundInput, args: MoveArgs,
        result: &ground_steering::GroundStep, force: ForceState,
    ) -> Option<(ground_steering::FreePose, ForceState, Option<ContactPair>)> {
        let mode = contact::Precision::Nearest;
        let predicted = projected(mode, input.position, result.velocity, input.dt);
        let blocker = self.valley_contact_candidate(index, actor, predicted)?;
        if !force.active() && blocker.is_none() {
            let clock = GroundClock { now: self.sim_elapsed(), last_external_displacement: -1.0 };
            return Some((free_pose(PRECISION, input, args, result, clock), force, None));
        }
        if input.dt != 0.05 || result.reached {
            return None;
        }
        let height = |[x, y]: [f32; 2]| self.map.height(x, y);
        let flat = height(input.position);
        if !flat.is_finite() || height(predicted) != flat {
            return None;
        }
        let atlas = self.valley_atlas_view()?;
        if !self.valley_barracks_motion_clear(&atlas, actor, predicted) {
            return None;
        }
        let mut pending = [0.0; 3];
        if let Some((_, idle)) = blocker {
            let fixed = Footprint { center: [idle.x, idle.y], yaw: idle.rot };
            if height(fixed.center) != flat {
                return None;
            }
            let moved = Footprint { center: predicted, yaw: input.heading };
            let overlap = contact::overlap(mode, moved, fixed)?;
            let stationary = Motion { planar_velocity: [0.0; 2], forward: 0.0,
                turn: 0.0, type_half: [0.125; 2], pending };
            let mover = Motion { planar_velocity: result.velocity, forward: result.forward_speed,
                turn: result.turn, ..stationary };
            let [moving_force, idle_force] = contact::weighted(mode, overlap, mover, stationary);
            if idle_force != [0.0; 3] {
                return None;
            }
            pending = moving_force;
        }
        let now = self.sim_elapsed();
        let force = refresh_stamp(mode, force, input.position, [actor.vx, actor.vy], result.ray, now);
        let solved = physical(mode, PhysicalInput {
            position: input.position, projected: predicted, heading: input.heading,
            forward: result.forward_speed, turn: result.turn, max_turn: args.max_turn,
            max_speed: args.max_speed, now, dt: input.dt, pending,
        }, force);
        if height(solved.position) != flat || !solved.force.valid(now)
            || !self.valley_barracks_motion_clear(&atlas, actor, solved.position)
        {
            return None;
        }
        let final_blocker = self.valley_contact_candidate(index, actor, solved.position)?;
        if blocker.zip(final_blocker).is_some_and(|(before, after)| before.0 != after.0) {
            return None;
        }
        let pair = blocker.or(final_blocker).map(|(other, idle)| ContactPair {
            mover: self.h(index), blocker: self.h(other), center: [idle.x, idle.y],
        });
        let pose = ground_steering::FreePose {
            position: solved.position, heading: solved.heading, turn: solved.turn,
            wrote_pose: true, moving_stamp: solved.force.stamp > 0.0,
        };
        Some((pose, solved.force, pair))
    }

    fn valley_motion_slope_supported(&self, actor: &Ent, speed: f32) -> bool {
        // 007F26EE–007F2755 probes current velocity 0.001 seconds ahead
        // and reduces the effective speed only above a 0.25 ascent ratio.
        const PROBE: f64 = 0.001_f32 as f64;
        if speed == 0.0 {
            return true;
        }
        let x = (f64::from(actor.x) + f64::from(actor.vx) * PROBE) as f32;
        let y = (f64::from(actor.y) + f64::from(actor.vy) * PROBE) as f32;
        let rise = f64::from(self.map.height(x, y)) - f64::from(self.map.height(actor.x, actor.y));
        rise <= 0.25 * f64::from(speed) * PROBE
    }

    fn valley_barracks_footprint_only(&self, atlas: &AtlasView<'_>, tile: usize) -> bool {
        if self.map.pass[0][tile] {
            return true;
        }
        if !self.map.blocked[tile] || self.map.ter[tile] < T_SAND || self.map.ter[tile] == T_ROCK {
            return false;
        }
        let (tx, ty) = ((tile % N) as i32, (tile / N) as i32);
        let mut found = false;
        for (index, site) in self.ents[..self.high].iter().enumerate() {
            if !site.alive || site.hp <= 0.0 || !DEFS[site.def as usize].building {
                continue;
            }
            let (x, y, width, height) = Self::building_bounds(site);
            if !(x..x + width).contains(&tx)
                || !(y..y + height).contains(&ty)
                || !Self::building_blocks_tile(site, tx, ty)
            {
                continue;
            }
            if site.def != BARRACKS || !atlas.runtime.handles.contains_key(&self.h(index)) {
                return false;
            }
            found = true;
        }
        found
    }

    fn valley_barracks_motion_clear(
        &self,
        atlas: &AtlasView<'_>,
        actor: &Ent,
        end: [f32; 2],
    ) -> bool {
        let from = [actor.x, actor.y];
        let half = |point: [f32; 2]| point.map(|axis| (axis * 2.0).floor() as i32);
        let owner = i32::from(actor.team) + 1;
        let denied = self.border_egress_mask(actor);
        // A native substep is at most 0.115 tile. These four half-cells
        // cover either crossing order at a grid corner.
        [from, [end[0], from[1]], [from[0], end[1]], end]
            .into_iter()
            .all(|point| {
                let cell = half(point);
                let tile = tile_of(point[0], point[1]);
                atlas.leaf(cell).is_some_and(|leaf| leaf.class == 0)
                    && atlas.admit(owner, 2, half(from), cell) == 2
                    && self.map.permits_territory(tile, denied)
                    && self.valley_barracks_footprint_only(atlas, tile)
            })
    }

    fn valley_native_motion_frame(
        &self,
        index: usize,
        actor: &Ent,
        def: &Def,
        route: &RouteProgress,
        state: NativeMotion,
        dt: f32,
        #[cfg(feature = "ee2-valley215-citizen-contact")] solved_pair: &mut Option<ContactPair>,
    ) -> Option<(Ent, NativeMotion, bool, bool)> {
        if self.modifiers[actor.team as usize][WORKER as usize][2] != CITIZEN_SPEED
            || self.valley_route_generation() != Some(route.atlas_generation)
            || !(0.0 < dt && dt <= 0.1)
            || !route.valid_for_actor(self)
            || !state.matches(actor)
            || !self.valley_motion_slope_supported(actor, state.forward)
        {
            return None;
        }
        let cursor = usize::from(route.next);
        let waypoint = *route.waypoints.get(cursor)?;
        let terminal = cursor + 1 == route.waypoints.len();
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let moving_stamp = state.force.is_some_and(|force| force.stamp > 0.0);
        #[cfg(not(feature = "ee2-valley215-citizen-contact"))]
        let moving_stamp = false;
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        if terminal && state.force.is_some_and(ForceState::active) {
            return None;
        }
        let tau = f32::from_bits(0x40c90fdb);
        let heading = if (0.0..tau).contains(&actor.rot) {
            actor.rot
        } else {
            actor.rot.rem_euclid(tau)
        };
        let input = GroundInput {
            position: [actor.x, actor.y],
            waypoint,
            heading,
            moving_stamp,
            forward_speed: state.forward,
            turn: state.turn,
            dt,
        };
        let args = citizen_move_args(if terminal { 0.05 } else { 0.1 });
        if terminal || moving_stamp {
            let radius = 1.0 + f64::from(f32::from_bits(0x3db504f3));
            let close = (f64::from(actor.x) - f64::from(waypoint[0])).powi(2)
                + (f64::from(actor.y) - f64::from(waypoint[1])).powi(2)
                < radius * radius;
            if close && !self.valley_motion_actor_clear(index, actor, waypoint, true) {
                return None;
            }
        }
        let result = if terminal {
            terminal_step(PRECISION, input, args, [actor.vx, actor.vy], heading)?
        } else {
            step(PRECISION, input, args)?
        };
        let free = || free_pose(
            PRECISION,
            input,
            args,
            &result,
            GroundClock {
                now: self.sim_elapsed(),
                last_external_displacement: -1.0,
            },
        );
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let (pose, force, pair) = match state.force {
            Some(force) if !terminal => {
                let (pose, force, pair) = self.valley_contact_frame(index, actor, input, args, &result, force)?;
                (pose, Some(force), pair)
            }
            force => (free(), force, None),
        };
        #[cfg(not(feature = "ee2-valley215-citizen-contact"))]
        let pose = free();
        let atlas = self.valley_atlas_view()?;
        let half = pose.position.map(|axis| (axis * 2.0).floor() as i32);
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let crowd_clear = pair.is_some() || self.valley_motion_actor_clear(index, actor, pose.position, false);
        #[cfg(not(feature = "ee2-valley215-citizen-contact"))]
        let crowd_clear = self.valley_motion_actor_clear(index, actor, pose.position, false);
        if atlas.leaf(half)?.class != 0 || !crowd_clear {
            return None;
        }
        let mut candidate = *actor;
        [candidate.vx, candidate.vy] = result.velocity;
        self.try_move(&mut candidate, def, pose.position[0], pose.position[1]);
        if [candidate.x, candidate.y] != pose.position
            || [candidate.vx, candidate.vy] != result.velocity
        {
            if !self.valley_barracks_motion_clear(&atlas, actor, pose.position) {
                return None;
            }
            [candidate.x, candidate.y] = pose.position;
            [candidate.vx, candidate.vy] = result.velocity;
        }
        candidate.rot = pose.heading;
        candidate.stuck = 0;
        let next_state = NativeMotion {
            forward: result.forward_speed,
            turn: pose.turn,
            #[cfg(feature = "ee2-valley215-citizen-contact")]
            force,
        };
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        { *solved_pair = pair; }
        next_state
            .valid()
            .then_some((candidate, next_state, terminal, result.reached))
    }

    pub(crate) fn valley_native_motion_step(
        &mut self,
        index: usize,
        actor: &mut Ent,
        def: &Def,
        dt: f32,
        unmodified: bool,
        #[cfg(feature = "ee2-valley215-citizen-contact")] separation: &mut CitizenSeparationFrame,
    ) -> bool {
        let handle = self.h(index);
        let Some(route) = self.valley_routes.active.get(&handle) else {
            return false;
        };
        let Some(state) = route.native else {
            return false;
        };
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        let mut pair = None;
        let frame = unmodified
            .then(|| self.valley_native_motion_frame(index, actor, def, route, state, dt,
                #[cfg(feature = "ee2-valley215-citizen-contact")] &mut pair))
            .flatten();
        let Some((candidate, state, terminal, reached)) = frame else {
            self.valley_fallback_motion(index);
            return false;
        };
        *actor = candidate;
        #[cfg(feature = "ee2-valley215-citizen-contact")]
        {
            if state.force.is_some() && !terminal {
                separation.contact_checked[index / 64] |= 1 << (index % 64);
            }
            if let Some(pair) = pair {
                separation.corrected.push(pair);
            }
        }
        if terminal && reached {
            Self::set_idle(actor);
            self.clear_valley_route(index);
        } else if let Some(route) = self.valley_routes.active.get_mut(&handle) {
            route.native = Some(state);
            if reached {
                route.next += 1;
            }
        }
        true
    }
}
