//! Source-shaped fixed airplane steering after the runway roll.

use crate::landing_geometry::Point;

const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
const TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const SMALL_DISTANCE: f64 = f32::from_bits(0x3d4c_cccd) as f64;
const TURNING_SPEED_FACTOR: f32 = f32::from_bits(0x3f19_999a);
const CIRCLE_FACTOR: f32 = f32::from_bits(0x3f4c_cccd);
const ARRIVAL_BUFFER: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub speed: f32,
    pub angular_rate: f32,
    pub velocity: Point,
}

#[derive(Clone, Copy)]
pub struct Controls {
    pub speed: f32,
    pub acceleration: f32,
    pub max_turn: f32,
    pub angular_acceleration: f32,
    pub distance_margin: f32,
    pub arrival_angle: f32,
}

pub fn wrap_error(mut error: f64) -> f64 {
    while error > PI { error -= TAU; }
    while error <= -PI { error += TAU; }
    error
}

fn angular_rate(error: f64, rate: f32, maximum: f32, acceleration: f32, dt: f32) -> f32 {
    let rounded = error as f32;
    let previous = f64::from(rate);
    let inv_dt = 1.0 / f64::from(dt);
    let accel = f64::from(acceleration);
    let mut limit = maximum;
    // 0085D601..651 reduces the target turn when the previous angular
    // motion is already headed toward the requested bearing.
    if previous * error > 0.0 {
        let braking = f64::from(rounded.abs()) / (previous.abs() / accel);
        if braking < f64::from(limit) { limit = braking as f32; }
    }
    let desired = ((f64::from(rounded) * inv_dt) as f32).clamp(-limit, limit);
    let proposed = (f64::from(desired) - previous) * inv_dt;
    let stored = proposed as f32;
    // 0085D6D4 stores the acceleration used by the sign test and the
    // clamp comparisons. Reversing/decelerating skips the acceleration cap.
    let chosen = if f64::from(stored) * previous < 0.0 {
        proposed
    } else if f64::from(stored) <= -accel * 10.0 {
        -accel * 10.0
    } else {
        f64::from(stored.min((accel * 10.0) as f32))
    };
    if chosen == f64::from(stored) { desired }
    else { (chosen * f64::from(dt) + previous) as f32 }
}

fn source_velocity(heading: f32, speed: f32) -> Point {
    let (sin, cos) = f64::from(heading).sin_cos();
    [cos as f32 * speed, sin as f32 * speed]
}

/// 0085CEC0 for a waypoint admitted by the original reachability gate.
pub fn steer(position: Point, heading: f32, destination: Point, motion: Motion,
             control: Controls, dt: f32) -> (Motion, bool) {
    debug_assert!(control.max_turn > 0.0 && control.angular_acceleration > 0.0 && dt > 0.0);
    let dx = f64::from(destination[0] - position[0]);
    let dy = f64::from(destination[1] - position[1]);
    let distance = (dx * dx + dy * dy).sqrt();
    let bearing = if distance > SMALL_DISTANCE {
        let (nx, ny) = ((dx / distance) as f32, (dy / distance) as f32);
        let angle = f64::from(ny).atan2(f64::from(nx));
        (if angle < 0.0 { angle + TAU } else { angle }) as f32
    } else { heading };
    let error = wrap_error(f64::from(bearing) - f64::from(heading));
    let rate = angular_rate(error, motion.angular_rate, control.max_turn, control.angular_acceleration, dt);

    let ratio = f64::from(rate.abs()) / f64::from(control.max_turn);
    let slow_speed = control.speed * TURNING_SPEED_FACTOR;
    let target = (1.0 - ratio) * f64::from(control.speed) + ratio * f64::from(slow_speed);
    let speed = f64::from(motion.speed);
    let delta = f64::from(dt) * f64::from(control.acceleration);
    let speed = if target < speed {
        // The decreasing branch floors at zero and can cross the desired
        // speed. The increasing branch clamps overshoot to the target.
        (speed - delta).max(0.0) as f32
    } else {
        (speed + delta).min(target) as f32
    };
    let velocity = source_velocity(heading, speed);
    let radius = f64::from(control.distance_margin.max(SMALL_DISTANCE as f32));
    let arrived = f64::from(distance as f32) <= radius + f64::from(dt) * f64::from(speed) &&
        (control.arrival_angle <= 0.0 || (error as f32).abs() <= control.arrival_angle);
    (Motion { speed, angular_rate: rate, velocity }, arrived)
}

/// 0085E320 for mode 3 or a goal temporarily inside the turn circle.
pub fn hold(heading: f32, motion: Motion, control: Controls, desired_turn: f32, dt: f32) -> Motion {
    let step = |current: f32, target: f32, acceleration: f32| {
        if current == target { return current; }
        let (current, target) = (f64::from(current), f64::from(target));
        let delta = f64::from(dt) * f64::from(acceleration);
        if current < target { (current + delta).min(target) as f32 }
        else { (current - delta).max(target) as f32 }
    };
    let speed = step(motion.speed, control.speed, control.acceleration);
    let angular_rate = step(motion.angular_rate, desired_turn, control.angular_acceleration);
    Motion { speed, angular_rate, velocity: source_velocity(heading, speed) }
}

/// 007D8AE0 mode 3 chooses the angular target before applying 0085E320.
pub fn approach_turn(position: Point, heading: f32, center: Point, center_dz: f32,
                     turn_left: bool, effective_speed: f32, effective_turn: f32) -> f32 {
    let (sin, cos) = f64::from(heading).sin_cos();
    let forward = [position[0] + cos as f32, position[1] + sin as f32];
    let ux = f64::from(forward[0] - position[0]);
    let uy = f64::from(forward[1] - position[1]);
    let vx = f64::from(center[0]) - f64::from(position[0]);
    let vy = f64::from(center[1]) - f64::from(position[1]);
    let vz = f64::from(center_dz);
    let dot = (ux * vx + uy * vy) as f32;
    let angle = if dot == 0.0 {
        std::f32::consts::FRAC_PI_2
    } else {
        let length = ((ux * ux + uy * uy) * (vx * vx + vy * vy + vz * vz)).sqrt();
        let cosine = (f64::from(dot) / length) as f32;
        f64::from(cosine.clamp(-1.0, 1.0)).acos() as f32
    };
    let radius = (f64::from(CIRCLE_FACTOR) * f64::from(effective_speed) / f64::from(effective_turn)) as f32;
    let distance = (vx * vx + vy * vy).sqrt() as f32;
    let radius = f64::from(radius);
    if angle.abs() >= std::f32::consts::FRAC_PI_4 ||
        (radius - 1.0..=radius + 1.0).contains(&f64::from(distance)) {
        if turn_left { effective_turn } else { -effective_turn }
    } else { 0.0 }
}

/// Turn-circle gate for an authored fixed-host route whose goal has zero range.
/// Returns whether this tick steers and the updated hysteresis flag.
pub fn fixed_goal_gate(position: Point, heading: f32, goal: Point, control: Controls,
                       was_unreachable: bool) -> (bool, bool) {
    let mut radius = (control.speed * CIRCLE_FACTOR) / control.max_turn;
    if was_unreachable { radius += 0.5; }
    let (dx, dy) = (f64::from(goal[0] - position[0]), f64::from(goal[1] - position[1]));
    let r = f64::from(radius);
    if dx * dx + dy * dy >= 4.0 * r * r { return (true, was_unreachable); }
    let margin = -f64::from(control.distance_margin).max(ARRIVAL_BUFFER) as f32;
    let inner = f64::from(margin + radius);
    let circle = f64::from((inner * inner) as f32);
    let (sin, cos) = f64::from(heading).sin_cos();
    let (sin, cos) = (sin as f32, cos as f32);
    let left = [
        f64::from(position[0]) + f64::from(-sin * radius),
        f64::from((f64::from(position[1]) + f64::from(cos) * r) as f32),
    ];
    let right = [
        f64::from(position[0] + sin * radius),
        f64::from((f64::from(position[1]) - f64::from(cos) * r) as f32),
    ];
    let blocked = [left, right].into_iter().any(|[x, y]| {
        let (dx, dy) = (x - f64::from(goal[0]), y - f64::from(goal[1]));
        dx * dx + dy * dy < circle
    });
    (!blocked, blocked)
}

pub fn integrate(position: Point, heading: f32, motion: Motion, dt: f32) -> (Point, f32) {
    let mut yaw = f64::from(heading) + f64::from(dt) * f64::from(motion.angular_rate);
    while yaw >= TAU { yaw -= TAU; }
    while yaw < 0.0 { yaw += TAU; }
    let yaw = yaw as f32;
    let yaw = if f64::from(yaw) >= TAU { 0.0 } else { yaw };
    let x = (f64::from(position[0]) + f64::from(dt) * f64::from(motion.velocity[0])) as f32;
    let y = position[1] + dt * motion.velocity[1];
    ([x, y], yaw)
}
