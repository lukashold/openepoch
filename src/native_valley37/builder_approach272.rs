//! Bridge the source City's terminal Build approach to the portable mover.
use super::builder_arrival272::{heading_ready, in_work_band, target_heading};
use super::ground_steering::{
    citizen_move_args, free_pose, terminal_step, GroundClock, GroundInput, GroundStep, Precision,
};
#[cfg(feature = "ee2-rect274-builder-brake")]
use super::ground_steering::direct_terminal_step;
use crate::{
    defs::{Def, WORKER},
    game::{Ent, Game, NONE},
    map::RegionMask,
    power42_math::store_toward_zero,
};

// Citizens cannot receive a rally point. Keep their terminal angular rate
// in the existing float pair, separate from the negative startup-exit key.
const TURN_MARKER: f32 = 0.0;
const TURN_LIMIT: f32 = f32::from_bits(0x4149_0fda);
#[cfg(feature = "ee2-rect273-builder-waypoint")]
const WAYPOINT_MARKER: f32 = 1.0;
#[cfg(feature = "ee2-rect273-builder-waypoint")]
const WAYPOINT_TERMINAL_MARKER: f32 = 2.0;
#[cfg(feature = "ee2-rect274-builder-brake")]
const DIRECT_TERMINAL_MARKER: f32 = 3.0;

impl Game {
    pub(crate) fn has_rect_city_alignment(worker: &Ent) -> bool {
        worker.def == WORKER && worker.rx == TURN_MARKER
    }

    #[cfg(feature = "ee2-rect273-builder-waypoint")]
    pub(crate) fn has_rect_city_waypoint(worker: &Ent) -> bool {
        worker.def == WORKER
            && (matches!(worker.rx, WAYPOINT_MARKER | WAYPOINT_TERMINAL_MARKER) || Self::has_rect_city_direct_brake(worker))
    }

    #[cfg(feature = "ee2-rect273-builder-waypoint")]
    fn has_rect_city_direct_brake(worker: &Ent) -> bool {
        #[cfg(feature = "ee2-rect274-builder-brake")]
        { worker.def == WORKER && worker.rx == DIRECT_TERMINAL_MARKER }
        #[cfg(not(feature = "ee2-rect274-builder-brake"))]
        { let _ = worker; false }
    }

    fn has_rect_city_approach(worker: &Ent) -> bool {
        let alignment = Self::has_rect_city_alignment(worker);
        #[cfg(feature = "ee2-rect273-builder-waypoint")]
        { alignment || Self::has_rect_city_waypoint(worker) }
        #[cfg(not(feature = "ee2-rect273-builder-waypoint"))]
        { alignment }
    }

    pub(crate) fn clear_rect_city_approach(worker: &mut Ent) {
        if Self::has_rect_city_approach(worker) {
            #[cfg(feature = "ee2-rect274-builder-brake")]
            if Self::has_rect_city_direct_brake(worker) { worker.qt = 0.0; }
            worker.rx = -1.0;
            worker.ry = -1.0;
        }
    }

    pub(crate) fn valid_rect_city_approach(&self, worker: &Ent) -> bool {
        if !Self::has_rect_city_approach(worker) { return true; }
        if !self.generated_city_foundations || self.rect_ai_clock.is_none()
            || !self.supports_rectangular_city_foundations() || worker.ry.abs() > TURN_LIMIT
        {
            return false;
        }
        #[cfg(feature = "ee2-rect274-builder-brake")]
        if Self::has_rect_city_direct_brake(worker) && worker.qt < 0.0 { return false; }
        #[cfg(feature = "ee2-rect273-builder-waypoint")]
        if Self::has_rect_city_waypoint(worker) && worker.order == crate::game::O_BUILD {
            let [w, h] = self.map.world_dimensions().map(|axis| axis as f32);
            if !(0.0..w).contains(&worker.ox) || !(0.0..h).contains(&worker.oy)
                || worker.fkey as usize != self.map.tile_of(worker.ox, worker.oy)
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn rect_city_uses_work_band(&self, worker: &Ent, site: &Ent) -> bool {
        worker.def == WORKER && self.rect_native_city_collision(site)
    }

    pub(crate) fn rect_city_in_work_band(&self, worker: &Ent, site: &Ent) -> bool {
        self.rect_city_uses_work_band(worker, site)
            && in_work_band([worker.x, worker.y], [site.x, site.y], site.rot)
    }

    pub(crate) fn rect_city_work_point(site: &Ent, [x, y]: [f32; 2]) -> Option<[f32; 2]> {
        // Store the flow tile in the existing Build key. Derive the exact
        // sub-tile point again on load and keep a deterministic seed order.
        const OFFSETS: [[f32; 2]; 9] = [
            [0.0, 0.0], [0.25, 0.0], [-0.25, 0.0], [0.0, 0.25], [0.0, -0.25],
            [0.25, 0.25], [-0.25, 0.25], [0.25, -0.25], [-0.25, -0.25],
        ];
        OFFSETS.into_iter().map(|[dx, dy]| [x + dx, y + dy])
            .find(|&point| in_work_band(point, [site.x, site.y], site.rot))
    }

    fn rect_city_build_waypoint(
        &mut self, worker: &mut Ent, site: &Ent, denied: RegionMask,
    ) -> Option<[f32; 2]> {
        let tile = (worker.fkey & 0xffff) as usize;
        let point = if self.map.pass[0][tile] && self.map.permits_territory(tile, denied) {
            Self::rect_city_work_point(site, self.map.tile_center(tile))
        } else {
            None
        };
        if point.is_some() && self.tick & 31 != 0 { return point; }

        self.prepare_rect_city_work_routes(site, denied);
        let from = self.map.tile_of(worker.x, worker.y);
        let mut key = self.placement_cache.approach[from];
        if key == NONE && !self.map.pass[0][from] {
            // A paid reservation can cover an assigned worker's old tile.
            // The normal integrator permits egress from such a tile.
            key = self.placement_cache.queue[..self.placement_cache.seed_count]
                .iter().copied().min_by(|&a, &b| {
                    let distance = |key| {
                        let [x, y] = self.map.tile_center(key as usize);
                        (x - worker.x).powi(2) + (y - worker.y).powi(2)
                    };
                    distance(a).total_cmp(&distance(b))
                }).unwrap_or(NONE);
        }
        if key == NONE { return None; }
        if key != worker.fkey { Self::assign_build_key(worker, key); }
        Self::rect_city_work_point(site, self.map.tile_center(key as usize))
    }

    pub(crate) fn rect_city_arrived_for_work(
        &mut self, worker: &mut Ent, definition: &Def, site: &Ent,
        want: &mut (f32, f32), moving: &mut bool,
        #[cfg(feature = "ee2-rect273-builder-waypoint")] integrated: &mut bool,
    ) -> bool {
        let position = [worker.x, worker.y];
        let target = [site.x, site.y];
        let in_band = in_work_band(position, target, site.rot);
        #[cfg(feature = "ee2-rect273-builder-waypoint")]
        if self.rect_city_follow_workpoint(worker, definition, site, in_band, want, moving, integrated) {
            return false;
        }
        if !in_band {
            Self::clear_rect_city_approach(worker);
            let denied = self.border_egress_mask(worker);
            if let Some([x, y]) = self.rect_city_build_waypoint(worker, site, denied) {
                *want = self.steer(worker, definition, x, y);
                *moving = *want != (0.0, 0.0);
            }
            return false;
        }

        // The original child Move keeps the same waypoint once its target
        // geometry already admits it. End translation on the current pulse,
        // then face the site. Residual portable damping is not native motion.
        if worker.vx != 0.0 || worker.vy != 0.0 {
            worker.vx = 0.0;
            worker.vy = 0.0;
            return false;
        }
        let turning = Self::has_rect_city_alignment(worker);
        if !turning && heading_ready(position, worker.rot, target) {
            return true;
        }
        let rate = if turning { worker.ry } else { 0.0 };
        if let Some(step) = self.rect_city_terminal_step(worker, definition, position, target, rate, 0.0) {
            if step.reached {
                Self::clear_rect_city_approach(worker);
            } else {
                worker.rx = TURN_MARKER;
                worker.ry = step.turn;
            }
        }
        // A finished child Move returns to the parent Build on its next update.
        false
    }

    fn rect_city_terminal_step(
        &self, worker: &mut Ent, definition: &Def, waypoint: [f32; 2], site: [f32; 2], rate: f32,
        forward_speed: f32,
    ) -> Option<GroundStep> {
        let position = [worker.x, worker.y];
        let velocity = [worker.vx, worker.vy];
        let tau = f64::from(std::f32::consts::TAU);
        let heading = store_toward_zero(f64::from(worker.rot).rem_euclid(tau));
        let heading = if f64::from(heading) >= tau { 0.0 } else { heading };
        let input = GroundInput {
            position, waypoint, heading, moving_stamp: false, forward_speed,
            turn: rate, dt: self.sim_step(),
        };
        let aim = target_heading(waypoint, site);
        let mut args = citizen_move_args(0.05);
        args.max_turn = TURN_LIMIT;
        let mode = Precision::TowardZero;
        #[cfg(feature = "ee2-rect274-builder-brake")]
        let steer_terminal = if Self::has_rect_city_direct_brake(worker) { direct_terminal_step } else { terminal_step };
        #[cfg(not(feature = "ee2-rect274-builder-brake"))]
        let steer_terminal = terminal_step;
        let step = steer_terminal(mode, input, args, velocity, aim)?;
        let pose = free_pose(mode, input, args, &step, GroundClock {
            now: self.sim_elapsed(), last_external_displacement: -1.0,
        });
        worker.rot = pose.heading;
        [worker.vx, worker.vy] = step.velocity;
        if pose.position != position {
            self.try_move(worker, definition, pose.position[0], pose.position[1]);
        }
        Some(step)
    }

    #[cfg(feature = "ee2-rect273-builder-waypoint")]
    fn rect_city_follow_workpoint(
        &mut self, worker: &mut Ent, definition: &Def, site: &Ent, in_band: bool,
        want: &mut (f32, f32), moving: &mut bool, integrated: &mut bool,
    ) -> bool {
        if Self::has_rect_city_alignment(worker) { return false; }
        if Self::has_rect_city_waypoint(worker) {
            let tile = (worker.fkey & 0xffff) as usize;
            let denied = self.border_egress_mask(worker);
            let direct = Self::has_rect_city_direct_brake(worker);
            if !self.map.permits_territory(tile, denied)
                || (!direct && (!self.map.pass[0][tile]
                    || (self.tick & 31 == 0 && !self.rect_city_can_follow_workpoint(worker, site))))
            {
                Self::clear_rect_city_approach(worker);
            }
        }
        let position = [worker.x, worker.y];
        let target = [site.x, site.y];
        if !Self::has_rect_city_waypoint(worker) {
            if in_band && worker.vx == 0.0 && worker.vy == 0.0 && heading_ready(position, worker.rot, target) {
                return false;
            }
            let Some([x, y]) = self.rect_city_source_workpoint(worker, site, in_band) else { return false; };
            let direct = [x, y] == position;
            #[cfg(feature = "ee2-rect274-builder-brake")]
            let direct_brake = direct && (worker.vx != 0.0 || worker.vy != 0.0);
            #[cfg(not(feature = "ee2-rect274-builder-brake"))]
            let direct_brake = false;
            if direct && !direct_brake { return false; }
            let tile = self.map.tile_of(x, y);
            Self::assign_build_key(worker, tile as u32);
            worker.ox = x;
            worker.oy = y;
            worker.rx = WAYPOINT_MARKER;
            worker.ry = 0.0;
            #[cfg(feature = "ee2-rect274-builder-brake")]
            if direct_brake {
                // The original direct child keeps its starting point even
                // when it arrives with velocity. Carry the native scalar
                // separately from the velocity components after the bridge.
                worker.rx = DIRECT_TERMINAL_MARKER;
                worker.qt = Precision::TowardZero.planar_speed([worker.vx, worker.vy]);
            }
        }

        let waypoint = [worker.ox, worker.oy];
        let remaining = (worker.x - waypoint[0]).hypot(worker.y - waypoint[1]);
        // Let the final-leg controller decelerate on the clear local tail.
        // Keep the flow field in charge while it still needs to route around
        // obstructions.
        if worker.rx == WAYPOINT_MARKER && remaining > 0.05
            && (remaining > 1.0 || !self.rect_city_workpoint_has_clear_tail(worker, site))
        {
            *want = self.steer(worker, definition, waypoint[0], waypoint[1]);
            *moving = *want != (0.0, 0.0);
            return true;
        }

        let direct = Self::has_rect_city_direct_brake(worker);
        if !direct { worker.rx = WAYPOINT_TERMINAL_MARKER; }
        let rate = worker.ry;
        let forward_speed = if direct { worker.qt } else { worker.vx.hypot(worker.vy) };
        if let Some(step) = self.rect_city_terminal_step(worker, definition, waypoint, target, rate, forward_speed) {
            worker.ry = step.turn;
            #[cfg(feature = "ee2-rect274-builder-brake")]
            if direct {
                let velocity = [worker.vx, worker.vy];
                worker.qt = if velocity == step.velocity {
                    step.forward_speed
                } else {
                    Precision::TowardZero.planar_speed(velocity)
                };
            }
            *integrated = true;
            if step.reached { Self::clear_rect_city_approach(worker); }
        }
        true
    }
}
