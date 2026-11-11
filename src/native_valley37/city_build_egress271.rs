//! First-work clearance for authored rectangular City Center construction.
use super::{city_actors270::mobile_radius, city_minerals269::BoxBound};
use crate::{
    game::{BOARDED, Ent, Game, O_BUILD, O_IDLE},
    power42_math::store_toward_zero as store,
};
#[cfg(feature = "ee2-rect275-idle-brake")]
use super::ground_steering::{citizen_move_args, idle_advance, idle_brake, Precision};
#[cfg(feature = "ee2-rect275-idle-brake")]
use crate::defs::{Def, WORKER};
#[cfg(feature = "ee2-rect276-clearance-resume")]
use super::fresh239::SCOUT;

// The queue-time slot is otherwise unused for a Citizen. The tag keeps
// an in-progress stock forward scalar distinct from older portable saves.
#[cfg(feature = "ee2-rect275-idle-brake")]
const IDLE_BRAKE: f32 = 4.0;

const BODY_HALF: [f32; 3] = [f32::from_bits(0x401e_6455), f32::from_bits(0x4007_c3b6), 1024.0];
const FOOTPRINT: [f32; 2] = [f32::from_bits(0x40b5_04f3), f32::from_bits(0x409e_6455)];
const BODY_SEARCH_PADDING: f32 = f32::from_bits(0x4050_9d4e);

fn direction(angle: f64) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [store(cos), store(sin)]
}

fn turn([x, y]: [f32; 2], [c, s]: [f32; 2]) -> [f32; 2] {
    let (x, y, c, s) = (f64::from(x), f64::from(y), f64::from(c), f64::from(s));
    [store(x * c - y * s), store(y * c + x * s)]
}

fn first_radial_exit(
    [x, y]: [f32; 2],
    [actor_x, actor_y]: [f32; 2],
    radius: f32,
    mut admits: impl FnMut([f32; 2]) -> bool,
) -> Option<[f32; 2]> {
    let base = store(f64::from(FOOTPRINT[0]) * f64::from(0.55_f32));
    let upper = store(f64::from(base) + 4.0);
    let mut ring = store(f64::from(base) + f64::from(radius));
    let limit = store(f64::from(upper) + f64::from(radius));
    let ring_step = store((f64::from(radius) * 2.5).clamp(f64::from(0.3_f32), 1.0));
    let dx = store(f64::from(actor_x) - f64::from(x));
    let dy = store(f64::from(actor_y) - f64::from(y));
    let bearing = f64::from(dy).atan2(f64::from(dx));
    let bearing = store(if bearing < 0.0 { bearing + f64::from(std::f32::consts::TAU) } else { bearing });
    let first = direction(f64::from(bearing));
    let pi = f64::from(std::f32::consts::PI);

    while ring <= limit {
        // 00873A00 chooses the angular budget with the body padding in
        // place. 00873280 uses that same effective radius for the point.
        let distance = f64::from(ring) + f64::from(BODY_SEARCH_PADDING);
        let half_steps = ((distance * pi / f64::from(ring_step) + 0.5).floor() as i32 - 1).clamp(4, 16);
        let [cos, sin] = direction(pi / f64::from(half_steps));
        let (mut left, mut right) = (first, first);
        for step in 0..=half_steps {
            let count = if step == 0 || step == half_steps { 1 } else { 2 };
            for [cx, sy] in [left, right].into_iter().take(count) {
                // Native 0087331D stores the Y product before adding the
                // center. The X product stays in the x87 register.
                let px = store(f64::from(x) + distance * f64::from(cx));
                let py = store(f64::from(y) + f64::from(store(distance * f64::from(sy))));
                if admits([px, py]) { return Some([px, py]); }
            }
            left = turn(left, [cos, sin]);
            right = turn(right, [cos, -sin]);
        }
        let next = f64::from(ring) + f64::from(ring_step);
        if next > f64::from(limit) { break; }
        ring = store(next);
    }
    None
}

impl Game {
    #[cfg(feature = "ee2-rect275-idle-brake")]
    pub(crate) fn has_rect_idle_brake(actor: &Ent) -> bool {
        actor.rx == IDLE_BRAKE && Self::rect_idle_acceleration(actor).is_some()
    }

    #[cfg(feature = "ee2-rect275-idle-brake")]
    fn rect_idle_acceleration(actor: &Ent) -> Option<f32> {
        match actor.def {
            WORKER => Some(citizen_move_args(0.125).acceleration),
            #[cfg(feature = "ee2-rect276-clearance-resume")]
            SCOUT => Some(f32::from_bits(0x4119_5554)),
            _ => None,
        }
    }

    #[cfg(feature = "ee2-rect275-idle-brake")]
    pub(crate) fn clear_rect_idle_brake(actor: &mut Ent) {
        if Self::has_rect_idle_brake(actor) {
            actor.rx = -1.0;
            actor.qt = 0.0;
        }
    }

    #[cfg(feature = "ee2-rect275-idle-brake")]
    pub(crate) fn valid_rect_idle_brake(&self, actor: &Ent) -> bool {
        !Self::has_rect_idle_brake(actor) || (self.supports_rectangular_city_foundations()
            && actor.qt.is_finite() && actor.qt > 0.0 && (actor.vx != 0.0 || actor.vy != 0.0))
    }

    #[cfg(feature = "ee2-rect275-idle-brake")]
    pub(crate) fn rect_idle_brake_step(&self, actor: &mut Ent, definition: &Def, dt: f32) -> bool {
        if actor.order != O_IDLE || self.rect_ai_clock.is_none()
            || !self.supports_rectangular_city_foundations()
        {
            return false;
        }
        let Some(acceleration) = Self::rect_idle_acceleration(actor) else { return false; };
        let original = [actor.vx, actor.vy];
        if original == [0.0; 2] {
            Self::clear_rect_idle_brake(actor);
            return true;
        }

        let mode = Precision::TowardZero;
        let forward = if Self::has_rect_idle_brake(actor) { actor.qt } else { mode.planar_speed(original) };
        let [x, y] = idle_advance(mode, [actor.x, actor.y], original, dt);
        // In 007F3140 the motion advances before the separate no-input brake.
        self.try_move(actor, definition, x, y);
        let velocity = [actor.vx, actor.vy];
        let forward = if velocity == original { forward } else { mode.planar_speed(velocity) };
        let (velocity, forward) = idle_brake(mode, velocity, forward, acceleration, dt);
        [actor.vx, actor.vy] = velocity;
        if forward > 0.0 && velocity != [0.0; 2] {
            actor.rx = IDLE_BRAKE;
            actor.qt = forward;
        } else {
            Self::clear_rect_idle_brake(actor);
        }
        true
    }

    pub(crate) fn rect_city_defers_displacement(&self, site: &Ent, actor: &Ent) -> bool {
        self.rect_native_city_collision(site) && mobile_radius(actor).is_some()
    }

    pub(crate) fn rect_city_first_work_clear(&mut self, site: &Ent, handle: u32) -> bool {
        if !self.rect_native_city_collision(site) || !self.generated_city_awaiting_labor(site) {
            return true;
        }

        // 007E0D10 queries the intended building body at half scale. It
        // distinguishes a movable bystander from a hard obstruction before
        // 007E1D60 can enter either the labor or the egress branch.
        let center = [site.x, site.y, 0.0];
        let body = BoxBound::from_half_extents(center, BODY_HALF, site.rot);
        let mut movable = false;
        for actor in &self.ents[..self.high] {
            let Some(radius) = mobile_radius(actor) else { continue; };
            if !self.rect_work_overlaps(body, actor, radius) { continue; }
            if !self.rect_can_clear_actor(site, actor) { return false; }
            if actor.order != O_BUILD || actor.otgt != handle { movable = true; }
        }
        if !movable { return true; }

        // The separate 007E3410 search visits the enlarged footprint, not
        // just the actors that triggered the intended-body query.
        let footprint_half = FOOTPRINT.map(|size| store(f64::from(size) * f64::from(1.1_f32) * 0.5));
        let footprint = BoxBound::from_half_extents(center, [footprint_half[0], footprint_half[1], 1024.0], site.rot);
        for i in 0..self.high {
            let actor = self.ents[i];
            let Some(radius) = mobile_radius(&actor) else { continue; };
            if !self.rect_work_overlaps(footprint, &actor, radius) || !self.rect_can_clear_actor(site, &actor) {
                continue;
            }
            // Suspend the supported stationary jobs without replacing
            // the actor's cargo or the underlying work assignment.
            #[cfg(feature = "ee2-rect276-clearance-resume")]
            let suspended = self.rect_clearance_can_suspend(&actor, handle);
            #[cfg(not(feature = "ee2-rect276-clearance-resume"))]
            let suspended = false;
            if !(actor.order == O_IDLE || suspended) || actor.rot != actor.prot {
                continue;
            }
            #[cfg(feature = "ee2-rect275-idle-brake")]
            let translation_active = if Self::rect_idle_acceleration(&actor).is_some() {
                actor.vx != 0.0 || actor.vy != 0.0
            } else {
                actor.x != actor.px || actor.y != actor.py
            };
            #[cfg(not(feature = "ee2-rect275-idle-brake"))]
            let translation_active = actor.x != actor.px || actor.y != actor.py;
            if translation_active {
                continue;
            }
            if let Some([x, y]) = self.rect_city_exit(site, &actor, radius, body) {
                #[cfg(feature = "ee2-rect276-clearance-resume")]
                if suspended {
                    self.rect_clearance_suspend(i, x, y);
                    continue;
                }
                self.order_move(&[i], x, y, false);
            }
        }
        false
    }

    fn rect_can_clear_actor(&self, site: &Ent, actor: &Ent) -> bool {
        actor.team == site.team || self.rect_mobile_ally_of_site(actor.team, site.team)
    }

    fn rect_work_overlaps(&self, bound: BoxBound, actor: &Ent, radius: f32) -> bool {
        actor.alive && actor.hp > 0.0 && actor.stuck != BOARDED
            && bound.overlaps_upright_capsule(
                [actor.x, actor.y, self.map.height(actor.x, actor.y) + 0.5], radius, 0.5,
            )
    }

    fn rect_city_exit(&self, site: &Ent, actor: &Ent, radius: f32, body: BoxBound) -> Option<[f32; 2]> {
        let denied = self.border_egress_mask(actor);
        let [width, height] = self.map.world_dimensions();
        first_radial_exit([site.x, site.y], [actor.x, actor.y], radius, |[x, y]| {
            if !(1.0..=width as f32 - 1.0).contains(&x) || !(1.0..=height as f32 - 1.0).contains(&y) {
                return false;
            }
            let tile = self.map.tile_of(x, y);
            self.map.pass[0][tile] && self.map.permits_territory(tile, denied)
                && !body.overlaps_upright_capsule([x, y, self.map.height(x, y) + 0.5], radius, 0.5)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::first_radial_exit;

    #[cfg(feature = "ee2-rect275-idle-brake")]
    #[test]
    fn source_chop_idle_advances_before_braking() {
        use super::{citizen_move_args, idle_advance, idle_brake, Precision};
        let mode = Precision::TowardZero;
        let dt = f32::from_bits(0x3d08_8888);
        let acceleration = citizen_move_args(0.125).acceleration;
        let origin = [f32::from_bits(0x4291_59f6), f32::from_bits(0x41c4_7d26)];
        let mut xy = origin;
        let (mut velocity, mut forward) = ([0.5, 0.0], 0.5);
        for word in [0x3ebe_93e9, 0x3e7a_4fa5, 0x3dee_eef1, 0] {
            xy = idle_advance(mode, xy, velocity, dt);
            (velocity, forward) = idle_brake(mode, velocity, forward, acceleration, dt);
            assert_eq!([velocity[0].to_bits(), forward.to_bits()], [word; 2]);
        }
        assert_eq!(xy.map(f32::to_bits), [0x4291_6f00, 0x41c4_7d26]);

        let tiny = f32::from_bits(1);
        let velocity = [-tiny; 2];
        assert_eq!(idle_advance(mode, origin, velocity, dt).map(f32::to_bits),
            [0x4291_59f5, 0x41c4_7d26]);
        assert_eq!(idle_brake(mode, velocity, tiny, acceleration, dt), ([0.0; 2], 0.0));
        assert_eq!(idle_brake(mode, velocity, 0.0, acceleration, dt), (velocity, 0.0));
    }

    #[cfg(feature = "ee2-rect276-clearance-resume")]
    #[test]
    fn scout_uses_its_original_idle_deceleration() {
        use super::{idle_advance, idle_brake, Game, Precision, SCOUT};
        let scout = crate::game::Ent { def: SCOUT, ..crate::game::E0 };
        let mode = Precision::TowardZero;
        let dt = f32::from_bits(0x3d08_8888);
        let acceleration = Game::rect_idle_acceleration(&scout).unwrap();
        let mut xy = [f32::from_bits(0x4291_59f6), f32::from_bits(0x41c4_7d26)];
        let (mut velocity, mut forward) = ([0.5, 0.0], 0.5);
        for (x, speed) in [(0x4291_627e, 0x3e38_e392), (0x4291_6592, 0)] {
            xy = idle_advance(mode, xy, velocity, dt);
            (velocity, forward) = idle_brake(mode, velocity, forward, acceleration, dt);
            assert_eq!([xy[0].to_bits(), velocity[0].to_bits(), forward.to_bits()], [x, speed, speed]);
        }
    }

    #[test]
    fn source_chop_radial_points_after_first_rejection() {
        let mut proposals = Vec::new();
        let exit = first_radial_exit(
            [72.0, 24.0],
            [f32::from_bits(0x4291_59f6), f32::from_bits(0x41c4_7d26)],
            0.125,
            |point| {
                proposals.push(point.map(f32::to_bits));
                proposals.len() == 2
            },
        );
        assert_eq!(proposals, [[0x4299_feb3, 0x41e1_3302], [0x4298_2f04, 0x41e8_5c69]]);
        assert_eq!(exit.unwrap().map(f32::to_bits), proposals[1]);
    }

    #[test]
    fn source_chop_radial_points_wrap_negative_bearing() {
        let mut proposals = Vec::new();
        let exit = first_radial_exit(
            [72.0, 24.0],
            [f32::from_bits(0x4291_59f6), f32::from_bits(0x41bb_82da)],
            0.125,
            |point| {
                proposals.push(point.map(f32::to_bits));
                proposals.len() == 2
            },
        );
        assert_eq!(proposals, [[0x4299_feb2, 0x419e_ccfd], [0x429b_6c0d, 0x41a7_3d01]]);
        assert_eq!(exit.unwrap().map(f32::to_bits), proposals[1]);
    }
}
