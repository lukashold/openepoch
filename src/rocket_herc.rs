//! Numeric leaves of the native GuidedMissile path for ProjectileRocketHerc.
//! Positions are source world units, velocities are world units per second.

use crate::{
    defs::{DEFS, LAND, SEA, WORKER, W_MISSILE},
    game::{Ent, Game, Proj, NONE},
    map::N,
    native_rng9::NativeRng,
    power42_math::store_toward_zero,
};

#[path = "rocket_herc_aim_data.rs"]
mod aim_data;

pub(crate) const SPEED: f32 = 11.0;
const ACCELERATION: f32 = 8.0;
const RADIANS_PER_DEGREE: f32 = 0.017_453_292;
const TURN_RATE: f32 = 720.0 * RADIANS_PER_DEGREE;
const LOOKAHEAD: f32 = 2.0;
const TURN_BLEND_SECONDS: f32 = 2.0;
const MINIMUM_ALTITUDE: f32 = 0.8;
const RELAX_BEGIN: f32 = 2.5;
const RELAX_END: f32 = 1.0;
const ARRIVAL_PROJECTION: f32 = 0.2;
const LAUNCH_FORWARD_SPEED: f32 = 0.1;
const MOVING_EPSILON: f32 = 1.0e-6;

pub(crate) type Point = [f32; 3];

#[derive(Clone, Copy)]
struct AimTag {
    position: Point,
    // Columns of the source rotation, with uniform scale stripped.
    basis: [Point; 3],
}

fn model_to_world(v: Point, yaw: f32) -> Point {
    let (sine, cosine) = yaw.sin_cos();
    [-v[1] * cosine - v[0] * sine, -v[1] * sine + v[0] * cosine, v[2]]
}

fn aimed_tag(tags: &[AimTag], origin: Point, yaw: f32, missile: Point) -> Option<(f64, AimTag)> {
    const BUILDING_SCALE: f32 = 0.028_284_27;
    let mut nearest = None;
    let mut closest = f64::from(f32::MAX);
    for tag in tags {
        let basis = tag.basis.map(|axis| model_to_world(axis, yaw));
        let relative = model_to_world(scale(tag.position, f64::from(BUILDING_SCALE)), yaw);
        let position = std::array::from_fn(|i| store_toward_zero(f64::from(origin[i]) + f64::from(relative[i])));
        let delta = subtract(position, missile);
        let squared = dot(delta, delta);
        if dot(delta, basis[1]) > 0.0 && squared < closest {
            closest = f64::from(store_toward_zero(squared));
            nearest = Some((squared, AimTag { position, basis }));
        }
    }
    nearest
}

fn aim_draw(rng: &mut NativeRng) -> f32 {
    // 007F66DF / 007F69D2: the multiplier is the stored binary32 2/32767.
    const TWO_OVER_32767: f32 = f32::from_bits(0x3880_0100);
    store_toward_zero((f64::from(rng.next_word15()) * f64::from(TWO_OVER_32767) - 1.0) * 0.5)
}

fn stationary_aim(
    rng: &mut NativeRng, fire: &[AimTag], hit: &[AimTag],
    origin: Point, center: Point, yaw: f32, missile: Point,
) -> Point {
    // 007F6689 consumes the two lateral words before either tag search.
    let x = aim_draw(rng);
    let z = aim_draw(rng);
    let selected = match (aimed_tag(fire, origin, yaw, missile), aimed_tag(hit, origin, yaw, missile)) {
        (Some((fire_distance, fire)), Some((hit_distance, _))) if fire_distance < hit_distance => Some(fire),
        (_, Some((_, hit))) => Some(hit),
        (Some((_, fire)), None) => Some(fire),
        (None, None) => None,
    };
    if let Some(tag) = selected {
        return std::array::from_fn(|i| {
            let offset = store_toward_zero(f64::from(tag.basis[0][i]) * f64::from(x)
                + f64::from(tag.basis[2][i]) * f64::from(z));
            store_toward_zero(f64::from(tag.position[i]) + f64::from(offset))
        });
    }
    let y = aim_draw(rng);
    let jitter = [x, y, z];
    std::array::from_fn(|i| store_toward_zero(f64::from(jitter[i]) + f64::from(center[i])))
}

pub(crate) struct Launch {
    pub position: [f32; 2],
    pub aim: Point,
    pub flight: Flight,
}

#[derive(Clone, Copy)]
pub(crate) struct Flight {
    pub z: f32,
    pub previous_z: f32,
    pub velocity: Point,
    pub speed: f32,
    pub target_z: f32,
    pub started: f32,
}

impl Flight {
    pub(crate) fn words(self) -> [u32; 8] {
        [self.z, self.previous_z, self.velocity[0], self.velocity[1], self.velocity[2],
            self.speed, self.target_z, self.started].map(f32::to_bits)
    }

    pub(crate) fn read(words: [u32; 8], now: f32) -> Option<Self> {
        let [z, previous_z, x, y, vz, speed, target_z, started] = words.map(f32::from_bits);
        if ![z, previous_z, x, y, vz, speed, target_z, started].into_iter().all(f32::is_finite)
            || [z, previous_z, target_z].into_iter().any(|height| height.abs() > 128.0)
            || !(0.0..=SPEED + 0.001).contains(&speed) || !(0.0..=now).contains(&started)
            || (magnitude([x, y, vz]) - f64::from(speed)).abs() > 0.001 {
            return None;
        }
        Some(Self { z, previous_z, velocity: [x, y, vz], speed, target_z, started })
    }
}

fn dot(a: Point, b: Point) -> f64 {
    (0..3).map(|i| f64::from(a[i]) * f64::from(b[i])).sum()
}

fn magnitude(a: Point) -> f64 {
    dot(a, a).sqrt()
}

fn subtract(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}

fn scale(a: Point, multiplier: f64) -> Point {
    a.map(|v| (f64::from(v) * multiplier) as f32)
}

fn normalize(a: Point) -> Point {
    let length = magnitude(a);
    if length == 0.0 { a } else { scale(a, length.recip()) }
}

/// 007DC72D…007DC7E3. The default enabled setting adds the muzzle frame's
/// forward column scaled by 0.1 to the firing actor's current velocity.
pub(crate) fn launch_velocity(actor_velocity: Point, muzzle_forward: Point) -> Point {
    std::array::from_fn(|i| {
        (f64::from(actor_velocity[i]) + f64::from(muzzle_forward[i]) * f64::from(LAUNCH_FORWARD_SPEED)) as f32
    })
}

/// 007F7410. Terrain sampling is passed in at the actual lookahead XY.
pub(crate) fn steering_point(
    position: Point,
    velocity: Point,
    speed: f32,
    target: Point,
    started: f32,
    now: f32,
    ground_at: impl FnOnce(f32, f32) -> f64,
) -> Point {
    let delta = subtract(target, position);
    let distance = magnitude(delta) as f32;
    let direct = if distance > 0.0 { scale(delta, f64::from(distance).recip()) } else { delta };
    let elapsed = f64::from(now) - f64::from(started);
    let blend = speed.abs() >= MOVING_EPSILON && elapsed < f64::from(TURN_BLEND_SECONDS);
    let direction = if blend {
        let current = normalize(velocity);
        let end = f64::from(started) + f64::from(TURN_BLEND_SECONDS);
        let factor = if now <= started { 0.0 } else { (elapsed / (end - f64::from(started))) as f32 };
        normalize(std::array::from_fn(|i| {
            ((1.0 - f64::from(factor)) * f64::from(current[i]) +
                f64::from(factor) * f64::from(direct[i])) as f32
        }))
    } else {
        direct
    };
    let mut point = std::array::from_fn(|i| {
        (f64::from(position[i]) + f64::from(direction[i]) * f64::from(LOOKAHEAD)) as f32
    });
    if blend {
        let clearance = f64::from(point[2]) - ground_at(point[0], point[1]);
        if clearance < f64::from(MINIMUM_ALTITUDE) {
            let ramp = ((f64::from(distance) - f64::from(RELAX_END)) /
                f64::from(RELAX_BEGIN - RELAX_END)).clamp(0.0, 1.0);
            point[2] = (f64::from(point[2]) + (f64::from(MINIMUM_ALTITUDE) - clearance) * ramp) as f32;
        }
    }
    point
}

/// 0085D180. The native acceleration vector is split along the old velocity
/// and perpendicular to it. The perpendicular component has its own cap.
pub(crate) fn steer(
    position: Point,
    velocity: Point,
    point: Point,
    dt: f32,
    motion_scale: f32,
) -> (Point, f32) {
    let direction = normalize(subtract(point, position));
    let old_direction = normalize(velocity);
    let dt = f64::from(dt);
    let acceleration = f64::from(ACCELERATION * motion_scale);
    let next_speed = ((magnitude(velocity) + acceleration * dt) as f32).min(SPEED * motion_scale);
    let demanded: Point = std::array::from_fn(|i| {
        ((f64::from(direction[i]) * f64::from(next_speed) - f64::from(velocity[i])) / dt) as f32
    });
    let parallel = scale(old_direction, dot(demanded, old_direction));
    let perpendicular = subtract(demanded, parallel);
    let turn_cap = f64::from(next_speed) * f64::from(TURN_RATE * motion_scale);
    let turn = if dot(direction, old_direction) > 0.0 {
        magnitude(perpendicular).min(turn_cap)
    } else {
        turn_cap
    };
    let orthogonal_step = scale(normalize(perpendicular), turn * dt);
    let parallel_step = scale(parallel, dt);
    let updated = std::array::from_fn(|i| {
        (f64::from(velocity[i]) + f64::from(parallel_step[i]) + f64::from(orthogonal_step[i])) as f32
    });
    (scale(normalize(updated), f64::from(next_speed)), next_speed)
}

/// 0085E600. Store each updated coordinate once after velocity × frame step.
pub(crate) fn integrate(position: Point, velocity: Point, dt: f32) -> Point {
    std::array::from_fn(|i| (f64::from(position[i]) + f64::from(velocity[i]) * f64::from(dt)) as f32)
}

/// 007F7100/007F7F20 guided projected arrival for a cached position or the
/// applicable target-shape branch. Live actor sweeps are separate.
pub(crate) fn passed_point(position: Point, velocity: Point, speed: f32, target: Point) -> bool {
    if speed <= 0.0 { return false; }
    let delta = subtract(target, position);
    let limit = f64::from((1.1 * f64::from(SPEED) / f64::from(TURN_RATE)) as f32);
    dot(delta, velocity) / f64::from(speed) < f64::from(ARRIVAL_PROJECTION)
        && dot(delta, delta) < limit * limit
}

/// 007F735A…007F73AD samples raw terrain at the projectile's current XY,
/// after target arrival has been checked. Ascending or level flights survive.
fn hit_terrain(position: Point, velocity: Point, height: f64) -> bool {
    f64::from(position[2]) <= height && velocity[2] < 0.0
}

const ROCKET_CAPSULE_RADIUS: f32 = 0.35;
const HUMAN_CAPSULE_RADIUS: f32 = 0.125;
const SCOUT_CAPSULE_RADIUS: f32 = 0.4;
const CAPSULE_HALF_HEIGHT: f64 = 0.5;

fn source_mobile_capsule_radius(definition: u16, epoch: u8) -> Option<f32> {
    if definition == WORKER {
        return Some(HUMAN_CAPSULE_RADIUS);
    }
    if !matches!(definition, 9..=17 | 57) {
        return None;
    }
    let row = crate::ee2_units::lookup_class(definition, epoch)?;
    let radius = match row.source_unit {
        "Scout" => SCOUT_CAPSULE_RADIUS,
        "LightInfantry" | "HeavyInfantry1" | "HeavyInfantry2" => HUMAN_CAPSULE_RADIUS,
        _ => return None,
    };
    (row.raw_size == Some([radius, radius])).then_some(radius)
}

pub(crate) fn source_mobile_capsule_offset(definition: u16, epoch: u8) -> Option<f32> {
    source_mobile_capsule_radius(definition, epoch).map(|_| CAPSULE_HALF_HEIGHT as f32)
}

fn upright_medial_segment(center_z: f32, radius: f32) -> (f64, f64) {
    let inset = CAPSULE_HALF_HEIGHT - f64::from(radius);
    let lower = f64::from(store_toward_zero(f64::from(center_z) - inset));
    let length = f64::from(store_toward_zero(2.0 * inset));
    (lower, lower + length)
}

/// The non-indexed GuidedMissile builds an upright capsule at its actor origin.
/// The grounded human target supplies its separately registered upright capsule.
fn rocket_capsule_overlaps_target(position: Point, center: Point, radius: f32) -> bool {
    let (rocket_low, rocket_high) = upright_medial_segment(position[2], ROCKET_CAPSULE_RADIUS);
    let (target_low, target_high) = upright_medial_segment(center[2], radius);
    let dx = f64::from(position[0]) - f64::from(center[0]);
    let dy = f64::from(position[1]) - f64::from(center[1]);
    let dz = (rocket_low - target_high).max(target_low - rocket_high).max(0.0);
    let radii = f64::from(ROCKET_CAPSULE_RADIUS) + f64::from(radius);
    let limit = f64::from(store_toward_zero(radii * radii));
    dx * dx + dy * dy + dz * dz < limit
}

fn rocket_hits_mobile_capsule(position: Point, velocity: Point, dt: f32, center: Point, radius: f32) -> bool {
    // 007F7CEB tests the initial capsule and then subdivisions. The final
    // 007F7E52 test restores the initial capsule, so that test is redundant.
    if rocket_capsule_overlaps_target(position, center, radius) {
        return true;
    }
    let step = velocity.map(|v| store_toward_zero(f64::from(v) * f64::from(dt)));
    let count = (magnitude(step) / (2.0 * f64::from(ROCKET_CAPSULE_RADIUS))).ceil() as u32;
    if count <= 1 {
        return false;
    }
    let spacing = step.map(|v| store_toward_zero(f64::from(v) / f64::from(count)));
    (1..count).any(|sample| {
        let position = std::array::from_fn(|axis| {
            let offset = store_toward_zero(f64::from(spacing[axis]) * f64::from(sample));
            store_toward_zero(f64::from(position[axis]) + f64::from(offset))
        });
        rocket_capsule_overlaps_target(position, center, radius)
    })
}

/// 0070FC10 ordinary ground tile. The source interpolates one triangle of
/// the tile rather than using the renderer's bilinear height.
fn terrain_triangle(corners: [f32; 4], x: f32, y: f32) -> f64 {
    let [h00, h10, h01, h11] = corners.map(f64::from);
    let x = f64::from(x);
    let y = f64::from(y);
    let diagonal = f64::from(store_toward_zero(h11 - h00));
    if y < x {
        h00 + y * diagonal + (x - y) * (h10 - h00)
    } else {
        h00 + x * diagonal + (y - x) * (h01 - h00)
    }
}

fn terrain_height(heights: &[f32], x: f32, y: f32) -> f64 {
    let x = x.clamp(0.0, N as f32);
    let y = y.clamp(0.0, N as f32);
    let column = (x as usize).min(N - 1);
    let row = (y as usize).min(N - 1);
    let width = N + 1;
    let i = row * width + column;
    terrain_triangle([heights[i], heights[i + 1], heights[i + width], heights[i + width + 1]],
        x - column as f32, y - row as f32)
}

// Proof adapter for the current engine. The native flight equations above are
// independent of this map/shape binding.
const ARTEMIS: u16 = crate::defs::LIGHT_MOUNTED3;
// LightHercTurret requests tag_muzzle_01 through tag_muzzle_08, while the
// installed Artemis body has only _1 and _2. The unmatched mount list
// falls back to actor center and zero muzzle contribution (007DC9E6).
const ARTEMIS_MOUNT: (Point, Point, f32) = ([0.0; 3], [0.0; 3], 1.0);
// 00874F46 caches Attack1 at base descriptor apply .5 / reload 1.3.
// See the exact installed animation and the matrix in muzzle-proof.json.
const MINOTAURUS_MOUNT: (Point, Point, f32) = (
    [13.398475, 2.0783606, 18.36661], [0.999978, -0.0000009536178, -0.0066319164], 0.05656854,
);

pub(crate) enum GuidedStep { Continue, Impact(f32), Discard }

impl Game {
    pub(crate) fn emit_guided_herc_impact(&mut self, x: f32, y: f32, z: f32) {
        let visible = self.real_fog(self.local, crate::map::tile_of(x, y)) == 255;
        if visible && self.guided_herc_impacts.len() < 64 {
            self.guided_herc_impacts.push([x, y, z, self.sim_elapsed()]);
        }
    }

    pub(crate) fn prepare_guided_herc_impacts(&mut self) -> u32 {
        let mut impacts = core::mem::take(&mut self.guided_herc_impacts);
        impacts.retain(|&[x, y, _, _]| self.real_fog(self.local, crate::map::tile_of(x, y)) == 255);
        let count = impacts.len() as u32;
        self.guided_herc_impacts = impacts;
        count
    }

    pub(crate) fn guided_herc_ordinary_map(&self) -> bool {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() { return false; }
        true
    }

    pub(crate) fn configure_guided_herc_rocket(&mut self) -> bool {
        if self.guided_herc_rocket || self.net || self.bench != 0 || self.tick != 0
            || !self.guided_herc_ordinary_map() || !self.civ_setup_open
            || !self.outbox.is_empty() || !self.inbox.is_empty() {
            return false;
        }
        self.guided_herc_rocket = true;
        true
    }

    fn herc_rocket_target_z(&self, actor: &Ent) -> f32 {
        let def = &DEFS[actor.def as usize];
        let ground = self.map.height(actor.x, actor.y).max(0.0);
        if def.dom == SEA { return 0.25; }
        // 007F6620 -> 00840440 aims at the live mobile spatial center.
        // The verified human profiles center the capsule half a UnitHeight above the base.
        if source_mobile_capsule_radius(actor.def, self.players[actor.team as usize].epoch).is_some() {
            return store_toward_zero(f64::from(ground) + CAPSULE_HALF_HEIGHT);
        }
        ground + def.size.clamp(0.25, 1.5)
    }

    pub(crate) fn launch_guided_herc_rocket(&mut self, source: &Ent, target: &Ent, weapon: u8)
        -> Option<Launch>
    {
        if !self.guided_herc_rocket || weapon != W_MISSILE || DEFS[target.def as usize].dom & (LAND | SEA) == 0 {
            return None;
        }
        let (offset, forward, model_scale) = match source.def {
            crate::heavy_mounted3_61::ID => MINOTAURUS_MOUNT,
            ARTEMIS if self.players[source.team as usize].epoch >= 13 => ARTEMIS_MOUNT,
            _ => return None,
        };
        let (sine, cosine) = source.rot.sin_cos();
        let rotate = |v: Point| [v[0] * cosine - v[1] * sine, v[0] * sine + v[1] * cosine, v[2]];
        let offset = rotate(scale(offset, f64::from(model_scale)));
        let x = source.x + offset[0];
        let y = source.y + offset[1];
        if !(0.0..=N as f32).contains(&x) || !(0.0..=N as f32).contains(&y) {
            return None;
        }
        let z = self.map.height(source.x, source.y).max(0.0) + offset[2];
        let velocity = launch_velocity([source.vx, source.vy, 0.0], rotate(forward));
        if source.def == crate::heavy_mounted3_61::ID {
            // 007DCC24 selects the one numbered Minotaurus cache slot.
            self.source_birth.rng.next_word15();
        }
        let center = [target.x, target.y, self.herc_rocket_target_z(target)];
        let aim = if let Some((fire, hit)) = aim_data::tags(target.def, self.players[target.team as usize].epoch)
            .filter(|_| target.built >= 1.0) {
            let origin = [target.x, target.y, self.map.height(target.x, target.y).max(0.0)];
            stationary_aim(&mut self.source_birth.rng, fire, hit, origin, center, target.rot, [x, y, z])
        } else {
            center
        };
        let flight = Flight {
            z, previous_z: z, velocity, speed: magnitude(velocity) as f32,
            target_z: aim[2], started: self.sim_elapsed(),
        };
        Some(Launch { position: [x, y], aim, flight })
    }

    pub(crate) fn advance_guided_herc_rocket(&self, p: &mut Proj, mut flight: Flight, dt: f32) -> GuidedStep {
        let target_index = self.get(p.tgt).filter(|&index| {
            let target = &self.ents[index];
            self.combat_hostile(p.team, target.team) && self.can_see(p.team, index)
        });
        if let Some(index) = target_index.filter(|&index| DEFS[self.ents[index].def as usize].speed > 0.0) {
            let target = &self.ents[index];
            p.tx = target.x;
            p.ty = target.y;
            flight.target_z = self.herc_rocket_target_z(target);
        }
        let position = [p.x, p.y, flight.z];
        let target = [p.tx, p.ty, flight.target_z];
        let passed = passed_point(position, flight.velocity, flight.speed, target);

        // Current engine target volumes use a circular XY footprint and a
        // terrain-anchored height proxy. Preserve the source's projected
        // cached-point retirement separately from that live swept check.
        let mut stationary_hit = false;
        let hits_target = target_index.is_some_and(|index| {
            let actor = &self.ents[index];
            let def = &DEFS[actor.def as usize];
            if def.speed == 0.0 {
                stationary_hit = passed;
                return passed;
            }
            if let Some(radius) = source_mobile_capsule_radius(actor.def, self.players[actor.team as usize].epoch) {
                return rocket_hits_mobile_capsule(position, flight.velocity, dt, target, radius);
            }
            let radius = self.entity_radius(actor).max(0.125);
            let horizontal = [flight.velocity[0] * dt, flight.velocity[1] * dt];
            let squared = horizontal[0].powi(2) + horizontal[1].powi(2);
            let t = if squared == 0.0 { 0.0 } else {
                (((p.tx - p.x) * horizontal[0] + (p.ty - p.y) * horizontal[1]) / squared).clamp(0.0, 1.0)
            };
            let delta = integrate(position, flight.velocity, dt * t);
            (delta[0] - p.tx).powi(2) + (delta[1] - p.ty).powi(2) <= radius.powi(2)
                && (delta[2] - flight.target_z).abs() <= def.size.clamp(0.25, 1.5)
        });
        if hits_target {
            if stationary_hit {
                p.x = store_toward_zero(f64::from(p.x) + f64::from(p.tx)) * 0.5;
                p.y = store_toward_zero(f64::from(p.y) + f64::from(p.ty)) * 0.5;
                return GuidedStep::Impact(store_toward_zero(f64::from(flight.z) + f64::from(flight.target_z)) * 0.5);
            }
            return GuidedStep::Impact(flight.z);
        }
        if target_index.is_none() && passed {
            p.tgt = NONE;
            return GuidedStep::Impact(flight.z);
        }
        let ground = terrain_height(&self.map.hm, p.x, p.y);
        if hit_terrain(position, flight.velocity, ground) {
            p.tgt = NONE;
            return GuidedStep::Impact(store_toward_zero(ground));
        }

        let point = steering_point(position, flight.velocity, flight.speed, target,
            flight.started, self.sim_elapsed(), |x, y| terrain_height(&self.map.hm, x, y).max(0.0));
        let (velocity, speed) = steer(position, flight.velocity, point, dt, 1.0);
        let next = integrate(position, velocity, dt);
        if !(0.0..N as f32).contains(&next[0]) || !(0.0..N as f32).contains(&next[1]) {
            p.tgt = NONE;
            return GuidedStep::Discard;
        }
        p.x = next[0];
        p.y = next[1];
        p.guided = Some(Flight {
            z: next[2], previous_z: flight.z, velocity, speed, ..flight
        });
        GuidedStep::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guided_impact_outbox_respects_combat_fog_and_stays_out_of_archives() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_guided_herc_rocket());
        let (x, y) = (100.0, 100.0);
        let tile = crate::map::tile_of(x, y);
        game.diplomacy.visibility = 2;
        game.team_vis[tile] = 128;
        game.emit_guided_herc_impact(x, y, 1.5);
        assert_eq!(game.prepare_guided_herc_impacts(), 0);
        game.team_vis[tile] = 255;
        game.save();
        let hash = game.state_hash();
        let baseline = game.savebuf.clone();
        for _ in 0..66 { game.emit_guided_herc_impact(x, y, 1.5); }
        assert_eq!(game.prepare_guided_herc_impacts(), 64);
        assert_eq!(game.guided_herc_impacts[0], [x, y, 1.5, 0.0]);
        assert_eq!(game.state_hash(), hash);
        game.save();
        assert_eq!(game.savebuf, baseline);
        let mut restored = Game::load(&baseline).expect("guided archive");
        assert_eq!(restored.prepare_guided_herc_impacts(), 0);
        game.team_vis[tile] = 128;
        assert_eq!(game.prepare_guided_herc_impacts(), 0);
    }

    #[test]
    fn mobile_capsule_compares_both_radii_and_skips_next_endpoint() {
        let origin = [0.0, 0.0, 0.5];
        let velocity = [11.0, 0.0, 0.0];
        let hits = |x, z, dt| rocket_hits_mobile_capsule(origin, velocity, dt, [x, 0.0, z], HUMAN_CAPSULE_RADIUS);
        assert!(hits(0.4, 0.5, 0.05));
        assert!(!hits(0.65, 0.5, 0.05));
        assert!(!hits(1.75, 0.5, 0.16));
        assert!(hits(1.15, 0.5, 0.16));
        assert!(!hits(0.0, 1.50002, 0.05));
        let tangent = store_toward_zero(f64::from(ROCKET_CAPSULE_RADIUS) + f64::from(HUMAN_CAPSULE_RADIUS));
        assert!(!rocket_hits_mobile_capsule(origin, [0.0; 3], 0.05, [tangent, 0.0, 0.5], HUMAN_CAPSULE_RADIUS));
    }

    #[test]
    fn scout_and_infantry_use_their_own_source_capsules() {
        assert_eq!(source_mobile_capsule_radius(57, 14), Some(0.4));
        for definition in [11, 14, 17] {
            assert_eq!(source_mobile_capsule_radius(definition, 14), Some(0.125));
        }
        for definition in [0, 7, 18, 43, 58, 86, 106] {
            assert_eq!(source_mobile_capsule_radius(definition, 14), None);
        }
        let rocket = [0.0, 0.0, 0.5];
        let target = [0.65, 0.0, 0.5];
        assert!(!rocket_capsule_overlaps_target(rocket, target, HUMAN_CAPSULE_RADIUS));
        assert!(rocket_capsule_overlaps_target(rocket, target, SCOUT_CAPSULE_RADIUS));
    }

    #[test]
    fn paid_minotaurus_reloads_while_attacking_each_new_ground_profile() {
        use crate::defs::{HERC_FACILITY, TC};
        use crate::map::{tile_of, T_SAND};
        let minotaurus = crate::heavy_mounted3_61::ID;
        for definition in [57, 17, 11, 14] {
            let mut game = Game::new(7, 0, 0, 1, 0);
            assert!(game.configure_heavy_mounted3());
            assert!(game.configure_guided_herc_rocket());
            assert!(game.configure_match_epoch_range(0xee01, 500, 2, 14, 14));
            let home = *game.ents[..game.high].iter().find(|actor| actor.alive && actor.team == 0 && actor.def == TC).unwrap();
            let host = game.spawn(HERC_FACILITY, 0, home.x + 6.0, home.y);
            let before: [f32; 9] = std::array::from_fn(|slot| game.resource(0, slot));
            assert!(game.train(host, minotaurus.into()));
            assert_eq!(std::array::from_fn(|slot| before[slot] - game.resource(0, slot)),
                [50.0, 0.0, 0.0, 0.0, 75.0, 0.0, 0.0, 0.0, 75.0]);
            let source = (0..2000).find_map(|_| {
                game.tick();
                game.ents[..game.high].iter().position(|actor| actor.alive && actor.team == 0 && actor.def == minotaurus)
            }).expect("paid factory trained a Minotaurus");
            let actor = game.ents[source];
            let (tx, ty) = [(4.0, 0.0), (-4.0, 0.0), (0.0, 4.0), (0.0, -4.0)].into_iter()
                .map(|(dx, dy)| (actor.x + dx, actor.y + dy))
                .find(|&(x, y)| game.map.ter[tile_of(x, y)] >= T_SAND
                    && game.map.ter[tile_of(x, y + 2.0)] >= T_SAND)
                .expect("ground target site near the trained unit");
            let target = game.spawn(definition, 1, tx, ty);
            let target_handle = game.h(target);
            let source_handle = game.h(source);
            let hp = game.ents[target].hp;
            game.order_move(&[target], tx, ty + 2.0, false);
            game.order_attack(&[source], target);
            let flight = (0..180).find_map(|_| {
                game.tick();
                game.projs.iter().find(|p| p.attack_source == source_handle && p.tgt == target_handle)
                    .and_then(|p| p.guided.map(|flight| (p.tx, p.ty, flight)))
            }).expect("ordinary attack admitted a guided shot at this profile");
            let moved = &game.ents[target];
            assert!((moved.y - ty).abs() > 0.02, "profile {definition} was walking");
            let z = game.map.height(moved.x, moved.y).max(0.0);
            assert_eq!((flight.0, flight.1, flight.2.target_z),
                (moved.x, moved.y, store_toward_zero(f64::from(z) + 0.5)));
            game.save();
            let mut loaded = Game::load(&game.savebuf).expect("in-flight archive reopens");
            loaded.save();
            assert_eq!(loaded.savebuf, game.savebuf);
            for _ in 0..80 {
                game.tick();
                loaded.tick();
                assert_eq!(game.state_hash(), loaded.state_hash());
            }
            assert!(game.get(target_handle).is_none_or(|index| game.ents[index].hp < hp),
                "profile {definition} received real missile damage");
        }
    }

    #[test]
    fn guided_launch_curves_and_caps_speed() {
        let mut position = [0.0, 0.0, 0.35];
        let mut velocity = launch_velocity([0.0; 3], [1.0, 0.0, 0.0]);
        assert_eq!(velocity, [0.1, 0.0, 0.0]);
        let mut speed = 0.1;
        for tick in 0..40 {
            let now = tick as f32 * 0.05;
            let target = [10.0, 3.0, 0.4];
            let point = steering_point(position, velocity, speed, target, 0.0, now, |_, _| 0.0);
            let old_speed = speed;
            (velocity, speed) = steer(position, velocity, point, 0.05, 1.0);
            assert!((speed - (old_speed + 0.4).min(11.0)).abs() < 0.00001);
            assert!((magnitude(velocity) - f64::from(speed)).abs() < 0.00001);
            position = integrate(position, velocity, 0.05);
            if tick == 0 {
                assert!(velocity[2] > 0.0);
                assert!(velocity[1].abs() < 0.00001);
            }
        }
        assert_eq!(speed, 11.0);
        assert!(position[1] > 0.1);
    }

    #[test]
    fn archived_vector_validates_finite_speed_and_causal_clock() {
        let state = Flight {
            z: 1.0, previous_z: 0.8, velocity: [3.0, 4.0, 0.0],
            speed: 5.0, target_z: 0.25, started: 100.0,
        };
        let words = state.words();
        assert_eq!(Flight::read(words, 100.0).unwrap().words(), words);
        assert!(Flight::read(words, 99.9).is_none());
        for i in 0..words.len() {
            let mut invalid = words;
            invalid[i] = f32::NAN.to_bits();
            assert!(Flight::read(invalid, 100.0).is_none());
        }
        let mut invalid = words;
        invalid[5] = 6.0_f32.to_bits();
        assert!(Flight::read(invalid, 100.0).is_none());
    }

    #[test]
    fn stationary_target_uses_two_lateral_words_or_three_fallback_words() {
        let seed = 0x1234_5678;
        let mut tagged = NativeRng::from_state(seed);
        let mut fallback = tagged;
        let mut oracle = tagged;
        let expected = [aim_draw(&mut oracle), aim_draw(&mut oracle)];
        let state_two = oracle.state();
        let third = aim_draw(&mut oracle);
        let state_three = oracle.state();
        let target = AimTag { position: [0.0; 3], basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] };
        let origin = [10.0, 10.0, 2.0];
        let center = [10.0, 10.0, 3.0];
        let aim = stationary_aim(&mut tagged, &[], &[target], origin, center, 0.0, [20.0, 10.0, 3.0]);
        assert_eq!(tagged.state(), state_two);
        assert_eq!(aim, [10.0, store_toward_zero(10.0 + f64::from(expected[0])),
            store_toward_zero(2.0 + f64::from(expected[1]))]);
        let aim = stationary_aim(&mut fallback, &[], &[target], origin, center, 0.0, [0.0, 10.0, 3.0]);
        assert_eq!(fallback.state(), state_three);
        assert_eq!(aim, [store_toward_zero(10.0 + f64::from(expected[0])),
            store_toward_zero(10.0 + f64::from(third)), store_toward_zero(3.0 + f64::from(expected[1]))]);
    }

    #[test]
    fn terrain_floor_checks_current_altitude_inclusively_and_downward_motion_strictly() {
        assert!(hit_terrain([0.0, 0.0, -0.25], [0.0, 0.0, -1.0], -0.25));
        assert!(hit_terrain([0.0, 0.0, -0.5], [0.0, 0.0, -1.0], -0.25));
        assert!(!hit_terrain([0.0, 0.0, -0.24], [0.0, 0.0, -1.0], -0.25));
        assert!(!hit_terrain([0.0, 0.0, -0.5], [0.0, 0.0, 0.0], -0.25));
        assert!(!hit_terrain([0.0, 0.0, -0.5], [0.0, 0.0, 1.0], -0.25));
    }

    #[test]
    fn ordinary_height_interpolates_the_two_source_triangles() {
        let corners = [0.0, 4.0, 2.0, 8.0];
        assert_eq!(terrain_triangle(corners, 0.75, 0.25), 4.0);
        assert_eq!(terrain_triangle(corners, 0.25, 0.75), 3.0);
        assert_eq!(terrain_triangle(corners, 0.5, 0.5), 4.0);
        assert_eq!(terrain_triangle([-2.0, -1.0, 1.0, 2.0], 0.125, 0.125), -1.5);
    }
}
