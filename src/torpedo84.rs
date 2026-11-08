//! Portable Torpedo type 5 movement and the native naval-target point check.
use crate::defs::{DEFS, SEA, SUB, W_TORP};
use crate::game::{Ent, Game, Proj, NONE};
use crate::map::N;
use crate::naval_heave72::{add, div, mul, store};
use crate::rocket_herc::{Flight, GuidedStep, Launch, Point};

pub(crate) const SPEED: f32 = 3.0;
pub(crate) const WATERLINE: f32 = f32::from_bits(0xbca3_d70a);
const ACCELERATION: f64 = 8.0;
const TURN_RATE: f64 = f32::from_bits(0x4149_0fda) as f64;
const ARRIVAL_PROJECTION: f64 = 0.2f32 as f64;

fn length_squared([x, y, z]: Point) -> f64 {
    let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
    add(add(x * x, y * y), z * z)
}

fn length(vector: Point) -> f64 {
    let squared = length_squared(vector);
    let root = squared.sqrt();
    if root.mul_add(root, -squared) > 0.0 { f64::from_bits(root.to_bits() - 1) } else { root }
}

fn dot(left: Point, right: Point) -> f64 {
    let term = |i: usize| f64::from(left[i]) * f64::from(right[i]);
    add(add(term(2), term(1)), term(0))
}

fn difference(left: Point, right: Point) -> Point {
    std::array::from_fn(|i| store(add(f64::from(left[i]), -f64::from(right[i]))))
}

fn scale(point: Point, factor: f64) -> Point {
    point.map(|value| store(mul(f64::from(value), factor)))
}

fn normalize(point: Point, magnitude: f64) -> Point {
    if magnitude > 0.0 { scale(point, div(1.0, magnitude)) } else { point }
}

fn lookahead(position: Point, aim: Point) -> Point {
    let delta = difference(aim, position);
    // 007F7516 stores the length before the reciprocal. Each component is
    // then stored for the two-unit lookahead, before adding the position.
    let direction = normalize(delta, f64::from(store(length(delta))));
    let ahead = scale(direction, 2.0);
    std::array::from_fn(|i| store(add(f64::from(position[i]), f64::from(ahead[i]))))
}

fn steer(position: Point, velocity: Point, aim: Point, dt: f32) -> (Point, f32) {
    let delta = difference(lookahead(position, aim), position);
    let desired = normalize(delta, length(delta));
    let old_speed = length(velocity);
    let old_direction = normalize(velocity, old_speed);
    let next_speed = add(old_speed, f64::from(dt) * ACCELERATION).min(f64::from(SPEED));
    let next_speed = f64::from(store(next_speed));
    let demanded = std::array::from_fn(|i| store(add(
        f64::from(desired[i]) * next_speed, -f64::from(velocity[i]),
    )));
    // 0085D292 writes the frame delta to the divisor slot used at 0085D2B1.
    // The parallel projection's scratch slot is overwritten by the turn step.
    let demanded = scale(demanded, div(1.0, f64::from(dt)));
    let parallel = scale(old_direction, dot(demanded, old_direction));
    let perpendicular = difference(demanded, parallel);
    let magnitude = length(perpendicular);
    let cap = next_speed * TURN_RATE;
    let turn = if dot(desired, old_direction) > 0.0 {
        let stored_cap = store(cap);
        if store(magnitude) > stored_cap { f64::from(stored_cap) } else { magnitude }
    } else {
        cap
    };
    let turn_step = scale(scale(normalize(perpendicular, magnitude), turn), f64::from(dt));
    let updated = std::array::from_fn(|i| store(add(f64::from(velocity[i]), f64::from(turn_step[i]))));
    (scale(normalize(updated, length(updated)), next_speed), next_speed as f32)
}

fn integrate(position: Point, velocity: Point, dt: f32) -> Point {
    // 0085E600 leaves X's product in the FPU. Y and Z use the two
    // intermediate f32 slots before the final position stores.
    std::array::from_fn(|i| {
        let step = f64::from(velocity[i]) * f64::from(dt);
        let step = if i == 0 { step } else { f64::from(store(step)) };
        store(add(f64::from(position[i]), step))
    })
}

fn arrived(position: Point, velocity: Point, speed: f32, aim: Point) -> bool {
    if speed <= 0.0 { return false; }
    let delta = difference(aim, position);
    let radius = f64::from(store(mul(div(f64::from(SPEED), TURN_RATE), 1.1)));
    div(dot(delta, velocity), f64::from(speed)) < ARRIVAL_PROJECTION
        && length_squared(delta) < radius * radius
}

pub(crate) fn valid_profile(profile: u16) -> bool {
    profile != crate::ee2_combat::UNKNOWN_PROFILE &&
        [34, 36, 45].into_iter().any(|definition| (10..15).any(|epoch|
        crate::ee2_combat::profile(definition, epoch) == profile))
}

impl Game {
    pub(crate) fn torpedo84_allowed(&self) -> bool {
        self.submarine_depth_enabled && self.submarine_depth_allowed()
    }

    pub(crate) fn configure_torpedo84(&mut self) -> bool {
        if self.tick != 0 || self.torpedo84_enabled || !self.torpedo84_allowed() { return false; }
        self.torpedo84_enabled = true;
        true
    }

    pub(crate) fn launch_native_torpedo(&self, index: usize, source: &Ent, target: &Ent, weapon: u8) -> Option<Launch> {
        if !self.torpedo84_enabled || weapon != W_TORP || !crate::submarine82::is_submarine(source.def)
            || self.players[source.team as usize].epoch < 10 || DEFS[target.def as usize].dom & (SEA | SUB) == 0 {
            return None;
        }
        let (id, z, tilt) = self.naval_body_pose(index)?;
        let basis = crate::naval_tilt73::naval_surface_basis(id, tilt, source.rot)?;
        // Portable launch origin. The hull's forward column supplies the
        // original 0.1 launch term while the private muzzle socket is separate.
        let forward = [-basis[6], -basis[8], -basis[7]];
        let velocity = [source.vx, source.vy, 0.0];
        let velocity = std::array::from_fn(|i| store(add(f64::from(velocity[i]), f64::from(forward[i]) * f64::from(0.1f32))));
        Some(Launch { position: [source.x, source.y], aim: [target.x, target.y, WATERLINE],
            flight: Flight { z, previous_z: z, velocity, speed: store(length(velocity)),
                target_z: WATERLINE, started: self.sim_elapsed() } })
    }

    pub(crate) fn advance_native_torpedo(&self, p: &mut Proj, mut flight: Flight, dt: f32) -> GuidedStep {
        let target = self.get(p.tgt).filter(|&index| {
            let actor = &self.ents[index];
            actor.hp > 0.0 && DEFS[actor.def as usize].dom & (SEA | SUB) != 0 &&
                self.combat_hostile(p.team, actor.team) && self.can_see(p.team, index)
        });
        if let Some(index) = target {
            let actor = &self.ents[index];
            if DEFS[actor.def as usize].speed > 0.0 { p.tx = actor.x; p.ty = actor.y; }
        }
        let aim = [p.tx, p.ty, flight.target_z];
        let position = [p.x, p.y, WATERLINE];
        if arrived(position, flight.velocity, flight.speed, aim) {
            if target.is_some() {
                // 007F7B52 on the admitted live naval branch.
                p.x = store(add(f64::from(p.x), f64::from(p.tx)) * 0.5);
                p.y = store(add(f64::from(p.y), f64::from(p.ty)) * 0.5);
            } else {
                p.tgt = NONE;
            }
            return GuidedStep::Impact(WATERLINE);
        }
        let (velocity, speed) = steer(position, flight.velocity, aim, dt);
        let next = integrate(position, velocity, dt);
        if !(0.0..N as f32).contains(&next[0]) || !(0.0..N as f32).contains(&next[1]) {
            p.tgt = NONE;
            return GuidedStep::Discard;
        }
        p.x = next[0];
        p.y = next[1];
        flight.previous_z = flight.z;
        flight.z = next[2];
        flight.velocity = velocity;
        flight.speed = speed;
        p.guided = Some(flight);
        GuidedStep::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naval_arrival_boundaries_are_strict() {
        let position = [0.0, 0.0, WATERLINE];
        let velocity = [1.0, 0.0, 0.0];
        let projection = ARRIVAL_PROJECTION as f32;
        let radius = store(mul(div(f64::from(SPEED), TURN_RATE), 1.1));
        let below = |value: f32| f32::from_bits(value.to_bits() - 1);
        assert!(!arrived(position, velocity, 1.0, [projection, 0.0, WATERLINE]));
        assert!(arrived(position, velocity, 1.0, [below(projection), 0.0, WATERLINE]));
        assert!(!arrived(position, velocity, 1.0, [0.0, radius, WATERLINE]));
        assert!(arrived(position, velocity, 1.0, [0.0, below(radius), WATERLINE]));
        assert!(!arrived(position, [0.0; 3], 0.0, position));
    }

    #[test]
    fn live_target_midpoint_and_typed_archive_checks() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_submarine_depth());
        assert!(game.configure_torpedo84());
        let source = game.spawn(34, 0, 95.0, 100.0);
        let target = game.spawn(42, 1, 100.125, 100.0);
        game.team_vis[crate::map::tile_of(100.125, 100.0)] = 255;
        let flight = Flight { z: WATERLINE, previous_z: WATERLINE, velocity: [1.0, 0.0, 0.0],
            speed: 1.0, target_z: WATERLINE, started: 0.0 };
        let mut torpedo = Proj { x: 100.0, y: 100.0, px: 100.0, py: 100.0, tx: 100.125, ty: 100.0,
            tgt: game.h(target), speed: SPEED, dmg: 165.0, kind: W_TORP, team: 0, splash: 0.0, targets: SEA | SUB,
            attacker_profile: crate::ee2_combat::profile(34, 10), friendly_ratio: 0.0, attack_source: game.h(source),
            guided: Some(flight), d0: 5.0 };
        game.projs.push(torpedo);
        let mut archive = Vec::new();
        game.append_projectiles(&mut archive);
        let hash = game.state_hash();
        for (offset, value) in [(20, 4), (32 + 28, 2.0f32.to_bits()),
            (32 + 52, u32::from(crate::ee2_combat::UNKNOWN_PROFILE)), (32 + 68, 3), (32 + 96, 0)] {
            let mut forged = archive.clone();
            forged[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(game.read_projectiles(Some(&forged)).is_none());
            assert_eq!(game.state_hash(), hash);
        }
        assert!(game.read_projectiles(Some(&archive)).is_some());
        assert_eq!(game.state_hash(), hash);
        assert!(matches!(game.advance_native_torpedo(&mut torpedo, flight, 0.05), GuidedStep::Impact(z) if z == WATERLINE));
        assert_eq!([torpedo.x, torpedo.y], [100.0625, 100.0]);
        assert_eq!(torpedo.tgt, game.h(target));
    }
}
