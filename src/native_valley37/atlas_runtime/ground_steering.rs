//! Detached ordinary ground waypoint steering and terminal braking.

#[derive(Clone, Copy)]
pub enum Precision {
    Nearest,
    TowardZero,
}

impl Precision {
    fn round(self, nearest: f64, residual: f64) -> f64 {
        if matches!(self, Self::TowardZero)
            && nearest != 0.0
            && residual != 0.0
            && nearest.is_sign_negative() != residual.is_sign_negative()
        {
            f64::from_bits(nearest.to_bits() - 1)
        } else {
            nearest
        }
    }

    fn store(self, value: f64) -> f32 {
        let nearest = value as f32;
        if matches!(self, Self::TowardZero) && f64::from(nearest).abs() > value.abs() {
            f32::from_bits(nearest.to_bits() - 1)
        } else {
            nearest
        }
    }

    fn add(self, a: f64, b: f64) -> f64 {
        let nearest = a + b;
        let part_b = nearest - a;
        self.round(nearest, (a - (nearest - part_b)) + (b - part_b))
    }

    fn mul(self, a: f64, b: f64) -> f64 {
        let nearest = a * b;
        self.round(nearest, a.mul_add(b, -nearest))
    }

    fn div(self, numerator: f64, denominator: f64) -> f64 {
        let nearest = numerator / denominator;
        let remainder = (-nearest).mul_add(denominator, numerator);
        self.round(
            nearest,
            if denominator.is_sign_negative() {
                -remainder
            } else {
                remainder
            },
        )
    }

    #[cfg(feature = "ee2-rect289-citizen-work")]
    pub(super) fn stored_division(self, numerator: f32, denominator: f32) -> f32 {
        self.store(self.div(f64::from(numerator), f64::from(denominator)))
    }

    fn sqrt(self, value: f64) -> f64 {
        let nearest = value.sqrt();
        self.round(nearest, (-nearest).mul_add(nearest, value))
    }

    fn dot(self, a: [f32; 2], b: [f32; 2]) -> f64 {
        let [x, y] = std::array::from_fn(|i| f64::from(a[i]) * f64::from(b[i]));
        self.add(x, y)
    }

    #[cfg(feature = "ee2-rect274-builder-brake")]
    pub(super) fn planar_speed(self, velocity: [f32; 2]) -> f32 {
        self.store(self.sqrt(self.dot(velocity, velocity)))
    }
}

const PI: f64 = f32::from_bits(0x40490fdb) as f64;
const ALIGNMENT_COS: f64 = f32::from_bits(0x3f3504f3) as f64;
const MIN_TURN: f64 = f32::from_bits(0x38d1b717) as f64;

#[derive(Clone, Copy)]
pub struct MoveArgs {
    pub radius: f32,
    pub max_speed: f32,
    pub acceleration: f32,
    pub max_turn: f32,
    pub turn_acceleration: f32,
    pub turn_speed: f32,
}

pub(super) const CITIZEN_SPEED: f32 = f32::from_bits(0x3f93_3333);
pub(super) const CITIZEN_TURN: f32 = f32::from_bits(0x4149_0fdb);

pub(super) fn citizen_move_args(radius: f32) -> MoveArgs {
    MoveArgs {
        radius,
        max_speed: CITIZEN_SPEED,
        acceleration: f32::from_bits(0x4075_5554),
        max_turn: CITIZEN_TURN,
        turn_acceleration: 720.0,
        turn_speed: 0.6 * CITIZEN_SPEED,
    }
}

#[cfg(feature = "ee2-rect275-idle-brake")]
pub(super) fn idle_advance(
    mode: Precision, position: [f32; 2], velocity: [f32; 2], dt: f32,
) -> [f32; 2] {
    let [dx, dy] = velocity.map(|component| f64::from(component) * f64::from(dt));
    // 0085E7F3 rounds the Y displacement before adding the origin.
    [mode.store(mode.add(f64::from(position[0]), dx)),
        mode.store(mode.add(f64::from(position[1]), f64::from(mode.store(dy))))]
}

#[cfg(feature = "ee2-rect275-idle-brake")]
pub(super) fn idle_brake(
    mode: Precision, velocity: [f32; 2], forward: f32, acceleration: f32, dt: f32,
) -> ([f32; 2], f32) {
    if forward <= 0.0 {
        return (velocity, forward);
    }
    // 0085E4B0 keeps the new scalar in the x87 register while scaling
    // the old velocity components. The f32 scalar store happens last.
    let next = mode.add(f64::from(forward), -f64::from(acceleration) * f64::from(dt));
    if next <= 0.0 {
        return ([0.0; 2], 0.0);
    }
    let ratio = mode.div(next, f64::from(forward));
    (velocity.map(|component| mode.store(mode.mul(ratio, f64::from(component)))), mode.store(next))
}

#[derive(Clone, Copy)]
pub struct GroundInput {
    pub position: [f32; 2],
    pub waypoint: [f32; 2],
    pub heading: f32,
    pub moving_stamp: bool,
    pub forward_speed: f32,
    pub turn: f32,
    pub dt: f32,
}

#[derive(Debug)]
pub struct GroundStep {
    pub ray: [f32; 2],
    pub effective_distance: f32,
    pub velocity: [f32; 2],
    pub forward_speed: f32,
    pub turn: f32,
    pub reached: bool,
}

fn heading_vector(mode: Precision, angle: f64) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    // Near a cardinal heading, nearest-even f64 can hide the fractional
    // reduction that the x87 truncate-to-f32 store still observes.
    let store = |value: f64, below: bool| {
        let result = mode.store(value);
        if matches!(mode, Precision::TowardZero) && below && f64::from(result) == value {
            f32::from_bits(result.to_bits() - 1)
        } else {
            result
        }
    };
    [
        store(cos, cos.abs() == 1.0 && angle != 0.0),
        store(sin, sin.abs() == 1.0 || sin == angle && angle != 0.0),
    ]
}

fn signed_heading(mode: Precision, difference: f64) -> f64 {
    if difference > PI {
        mode.add(difference, -2.0 * PI)
    } else if difference <= -PI {
        mode.add(difference, 2.0 * PI)
    } else {
        difference
    }
}

fn ray_heading(mode: Precision, ray: [f32; 2]) -> f32 {
    let [x, y] = ray.map(f64::from);
    let angle = y.atan2(x);
    mode.store(if angle < 0.0 {
        mode.add(angle, 2.0 * PI)
    } else {
        angle
    })
}

fn turn_for_heading(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    wanted: f32,
    braking: bool,
    angular_tolerance: f64,
) -> f32 {
    let dt = f64::from(input.dt);
    let previous = f64::from(input.turn);
    let difference = signed_heading(mode, f64::from(wanted) - f64::from(input.heading));
    let saved_difference = f64::from(mode.store(difference));
    let turn_accel = f64::from(args.turn_acceleration);
    let mut limit = f64::from(args.max_turn);
    let max_speed = f64::from(args.max_speed);
    let turn_speed = f64::from(args.turn_speed);
    if !braking && turn_speed < max_speed {
        let slowed = mode.add(
            f64::from(input.forward_speed),
            -mode.mul(dt, f64::from(args.acceleration)),
        );
        let margin = mode.add(max_speed, -slowed).max(0.0);
        let scaled = mode.div(mode.mul(margin, limit), max_speed - turn_speed);
        limit = f64::from(mode.store(limit.min(scaled)));
    }
    if previous * difference > 0.0 {
        let speed = mode.div(saved_difference.abs(), mode.div(previous.abs(), turn_accel));
        limit = f64::from(mode.store(speed.min(limit)));
    }
    if saved_difference.abs() < angular_tolerance {
        limit = 0.0;
    }
    let inv_dt = mode.div(1.0, dt);
    let desired = f64::from(mode.store(mode.mul(saved_difference, inv_dt)));
    let goal = if desired <= -limit {
        -limit
    } else {
        desired.min(limit)
    };
    let raw_accel = mode.mul(goal - previous, inv_dt);
    let saved_accel = f64::from(mode.store(raw_accel));
    let chosen = if saved_accel * previous < 0.0 {
        raw_accel
    } else {
        saved_accel.clamp(-10.0 * turn_accel, f64::from(mode.store(10.0 * turn_accel)))
    };
    mode.store(if chosen == saved_accel {
        goal
    } else {
        mode.add(previous, mode.mul(chosen, dt))
    })
}

fn forward_speed(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    alignment: f64,
    distance: f32,
    turn: f32,
    anchor_distance: Option<f32>,
) -> f32 {
    let previous = f64::from(input.forward_speed);
    let dt = f64::from(input.dt);
    let max_speed = f64::from(args.max_speed);
    let accel = f64::from(args.acceleration);
    let turning_speed = f64::from(args.turn_speed);
    if alignment < ALIGNMENT_COS {
        let target = (max_speed * if input.moving_stamp { 0.5 } else { 0.25 }).min(turning_speed);
        let step = mode.mul(dt, accel);
        return mode.store(if previous < target {
            mode.add(previous, step).min(target)
        } else {
            mode.add(previous, -mode.mul(3.0, step)).max(target)
        });
    }

    let turn_ratio = mode.div(f64::from(turn.abs()), f64::from(args.max_turn));
    let mut mixed = mode.add(
        mode.mul(mode.add(1.0, -turn_ratio), max_speed),
        mode.mul(turn_ratio, turning_speed),
    );
    if let Some(anchor) = anchor_distance.filter(|_| previous > 0.0) {
        let brake_factor = mode.div(previous, 3.0 * accel);
        let limit = mode.store(mode.div(f64::from(anchor), brake_factor));
        mixed = mixed.min(f64::from(limit));
    }
    let inv_dt = mode.div(1.0, dt);
    let target = f64::from(mode.store(mixed.min(mode.mul(f64::from(distance), inv_dt))));
    let acceleration =
        f64::from(mode.store(mode.mul(target - previous, f64::from(mode.store(inv_dt)))));
    if acceleration <= accel {
        mode.store(target)
    } else {
        mode.store(mode.add(previous, mode.mul(accel, dt)).max(0.0))
    }
}

fn valid_input(input: GroundInput, args: MoveArgs) -> bool {
    (0.0..2.0 * PI).contains(&f64::from(input.heading))
        && input.forward_speed.is_finite()
        && input.forward_speed >= 0.0
        && input.turn.is_finite()
        && input.turn.abs() <= args.max_turn
        && [
            input.dt,
            args.radius,
            args.max_speed,
            args.acceleration,
            args.max_turn,
            args.turn_acceleration,
            args.turn_speed,
        ]
        .into_iter()
        .all(|x| x.is_finite() && x > 0.0)
}

fn measure(mode: Precision, input: GroundInput) -> ([f32; 2], f32) {
    let delta: [f32; 2] = std::array::from_fn(|i| {
        mode.store(f64::from(input.waypoint[i]) - f64::from(input.position[i]))
    });
    let distance = mode.store(mode.sqrt(mode.dot(delta, delta)));
    if distance == 0.0 {
        return (delta, distance);
    }
    let inverse = mode.div(1.0, f64::from(distance));
    let ray = delta.map(|axis| mode.store(mode.mul(f64::from(axis), inverse)));
    (ray, distance)
}

fn unsigned_heading(mode: Precision, angle: f64) -> f64 {
    if angle < 0.0 {
        mode.add(angle, 2.0 * PI)
    } else if angle >= 2.0 * PI {
        mode.add(angle, -2.0 * PI)
    } else {
        angle
    }
}

fn approaching(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    ray: [f32; 2],
    distance: f32,
    anchor_distance: Option<f32>,
    angular_tolerance: f64,
) -> GroundStep {
    let heading = f64::from(input.heading);
    let projected = mode
        .div(
            mode.dot(heading_vector(mode, heading), ray),
            f64::from(distance),
        )
        .clamp(0.5, 1.0);
    let effective_distance =
        mode.store(mode.mul(f64::from(mode.store(projected)), f64::from(distance)));

    let turn = if input.moving_stamp {
        input.turn
    } else {
        turn_for_heading(mode, input, args, ray_heading(mode, ray), false, angular_tolerance)
    };
    let direction = if input.moving_stamp || anchor_distance.is_some_and(|value| value < 1.0) {
        ray
    } else {
        let angle = mode.add(heading, mode.mul(f64::from(turn), f64::from(input.dt)));
        heading_vector(mode, unsigned_heading(mode, angle))
    };
    let forward_speed = forward_speed(
        mode,
        input,
        args,
        mode.dot(direction, ray),
        effective_distance,
        turn,
        anchor_distance,
    );
    let velocity = direction.map(|axis| mode.store(f64::from(axis) * f64::from(forward_speed)));
    let distance = f64::from(effective_distance);
    let radius = f64::from(args.radius.max(0.05));
    let reached = anchor_distance.is_none()
        && (distance <= mode.add(radius, f64::from(input.dt) * f64::from(forward_speed))
            || distance <= mode.div(f64::from(args.max_speed), f64::from(args.max_turn))
                && mode.dot(ray, velocity) <= 0.0);
    GroundStep {
        ray,
        effective_distance,
        velocity,
        forward_speed,
        turn,
        reached,
    }
}

/// A finite, nonterminal ordinary command with no external look-ahead or contact.
pub fn step(mode: Precision, input: GroundInput, args: MoveArgs) -> Option<GroundStep> {
    if !valid_input(input, args) {
        return None;
    }
    let (ray, distance) = measure(mode, input);
    (distance.is_finite() && distance > 1e-5)
        .then(|| approaching(mode, input, args, ray, distance, None, MIN_TURN))
}

/// Terminal Move for which the waypoint is also the original anchor.
pub fn terminal_step(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    incoming_velocity: [f32; 2],
    terminal_heading: f32,
) -> Option<GroundStep> {
    terminal_step_with_bounds(mode, input, args, incoming_velocity, terminal_heading, MIN_TURN, false)
}

#[cfg(feature = "ee2-rect274-builder-brake")]
pub(super) fn direct_terminal_step(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    incoming_velocity: [f32; 2],
    terminal_heading: f32,
) -> Option<GroundStep> {
    // Build's actor-target Move supplies a terminal facing tolerance of
    // .5 * .15 in source precision. 85C4C0 enters the braking branch
    // strictly inside the terminal radius and clears translation at equality.
    let facing_tolerance = f64::from(f32::from_bits(0x3d99_9999));
    terminal_step_with_bounds(mode, input, args, incoming_velocity, terminal_heading, facing_tolerance, true)
}

fn terminal_step_with_bounds(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    incoming_velocity: [f32; 2],
    terminal_heading: f32,
    angular_tolerance: f64,
    stop_at_edge: bool,
) -> Option<GroundStep> {
    if !valid_input(input, args)
        || !incoming_velocity.into_iter().all(f32::is_finite)
        || !(0.0..2.0 * PI).contains(&f64::from(terminal_heading))
    {
        return None;
    }
    let (ray, distance) = measure(mode, input);
    let [x, y] =
        std::array::from_fn(|i| f64::from(input.waypoint[i]) - f64::from(input.position[i]));
    let anchor = mode.store(mode.sqrt(mode.add(mode.mul(x, x), mode.mul(y, y))));
    if !anchor.is_finite() || !distance.is_finite() {
        return None;
    }
    let radius = args.radius.max(0.05);
    if anchor > radius {
        return Some(approaching(mode, input, args, ray, distance, Some(anchor), angular_tolerance));
    }
    // At the exact edge the source applies the waypoint bearing, while
    // readiness still compares the actor to the requested final heading.
    let at_edge = anchor == radius && stop_at_edge;
    let heading = if at_edge { ray_heading(mode, ray) } else { terminal_heading };
    let turn = turn_for_heading(mode, input, args, heading, !at_edge, angular_tolerance);
    let previous = f64::from(input.forward_speed);
    let braking = if at_edge {
        0.0
    } else {
        mode.add(previous, -mode.mul(f64::from(input.dt), f64::from(args.acceleration)))
    };
    let (forward_speed, velocity) = if braking > 0.0 {
        let scale = mode.div(braking, previous);
        (
            mode.store(braking),
            incoming_velocity.map(|axis| mode.store(mode.mul(f64::from(axis), scale))),
        )
    } else {
        (0.0, [0.0; 2])
    };
    let faces_final = !at_edge || f64::from(mode.store(signed_heading(
        mode, f64::from(terminal_heading) - f64::from(input.heading),
    ))).abs() <= angular_tolerance;
    Some(GroundStep {
        ray,
        effective_distance: distance,
        velocity,
        forward_speed,
        turn,
        reached: forward_speed == 0.0 && turn == 0.0 && faces_final,
    })
}

#[derive(Debug)]
pub struct FreePose {
    pub position: [f32; 2],
    pub heading: f32,
    pub turn: f32,
    pub moving_stamp: bool,
    pub wrote_pose: bool,
}

#[derive(Clone, Copy)]
pub struct GroundClock {
    pub now: f32,
    pub last_external_displacement: f32,
}

/// Physical integration in the detached flat, unobstructed XY branch.
pub fn free_pose(
    mode: Precision,
    input: GroundInput,
    args: MoveArgs,
    step: &GroundStep,
    clock: GroundClock,
) -> FreePose {
    let dt = f64::from(input.dt);
    let [dx, dy] = step.velocity.map(|value| mode.mul(dt, f64::from(value)));
    let position = [
        mode.store(mode.add(f64::from(input.position[0]), dx)),
        mode.store(mode.add(f64::from(input.position[1]), f64::from(mode.store(dy)))),
    ];
    let heading = f64::from(input.heading);
    let turn_with_displacement = input.moving_stamp && step.forward_speed != 0.0;
    let rotation = if turn_with_displacement {
        let [dx, dy] = std::array::from_fn(|i| {
            f64::from(mode.store(f64::from(position[i]) - f64::from(input.position[i])))
        });
        let angle = dy.atan2(dx);
        let wanted = if angle < 0.0 {
            mode.add(angle, 2.0 * PI)
        } else {
            angle
        };
        let limit = f64::from(
            mode.store(mode.mul(mode.mul(dt, f64::from(0.6_f32)), f64::from(args.max_turn))),
        );
        f64::from(mode.store(signed_heading(mode, mode.add(wanted, -heading)))).clamp(-limit, limit)
    } else {
        mode.mul(dt, f64::from(step.turn))
    };
    let wrote_pose = position != input.position || step.turn != 0.0 || input.moving_stamp;
    FreePose {
        position,
        heading: mode.store(unsigned_heading(mode, mode.add(heading, rotation))),
        turn: if turn_with_displacement {
            0.0
        } else {
            step.turn
        },
        moving_stamp: input.moving_stamp
            && mode.add(f64::from(clock.now), -1.0) <= f64::from(clock.last_external_displacement),
        wrote_pose,
    }
}

#[cfg(all(test, feature = "ee2-rect274-builder-brake"))]
mod direct_terminal_tests {
    use super::*;

    #[test]
    fn builder_brake_reconstructs_speed_once_and_honors_the_strict_facing_limit() {
        let mode = Precision::TowardZero;
        let incoming = [f32::from_bits(0xbf0b_8693), f32::from_bits(0x3ed8_a651)];
        assert_eq!(mode.planar_speed(incoming).to_bits(), 0x3f30_a3d6);

        let input = GroundInput {
            position: [0.0; 2], waypoint: [0.0; 2], heading: 0.0, moving_stamp: false,
            forward_speed: 0.0, turn: 0.0, dt: f32::from_bits(0x3d08_8888),
        };
        let args = citizen_move_args(0.05);
        let tolerance: u32 = 0x3d99_9999;
        let steer = |heading| direct_terminal_step(mode, input, args, [0.0; 2], f32::from_bits(heading)).unwrap();
        assert!(steer(tolerance - 1).reached);
        assert!(!steer(tolerance).reached);
    }

    #[test]
    fn builder_brake_distinguishes_both_sides_of_the_exact_terminal_radius() {
        let mode = Precision::TowardZero;
        let args = citizen_move_args(0.05);
        for (distance, speed, reached) in [
            (0x3d4c_cccc, 0x3ebe_93e9, false),
            (0x3d4c_cccd, 0x0000_0000, true),
            (0x3d4c_ccce, 0x3f20_b60b, false),
        ] {
            let input = GroundInput {
                position: [0.0; 2], waypoint: [f32::from_bits(distance), 0.0],
                heading: 0.0, moving_stamp: false, forward_speed: 0.5,
                turn: 0.0, dt: f32::from_bits(0x3d08_8888),
            };
            let step = direct_terminal_step(mode, input, args, [0.5, 0.0], 0.0).unwrap();
            assert_eq!(step.forward_speed.to_bits(), speed);
            assert_eq!(step.reached, reached);
        }
        let edge = GroundInput {
            position: [0.0; 2], waypoint: [0.05, 0.0], heading: 0.0, moving_stamp: false,
            forward_speed: 0.5, turn: 0.0, dt: f32::from_bits(0x3d08_8888),
        };
        let faces_waypoint = direct_terminal_step(mode, edge, args, [0.5, 0.0], 0.2).unwrap();
        assert_eq!(faces_waypoint.forward_speed, 0.0);
        assert_eq!(faces_waypoint.turn, 0.0);
        assert!(!faces_waypoint.reached);
        let rotates = direct_terminal_step(mode, GroundInput { heading: 0.2, ..edge }, args, [0.5, 0.0], 0.0).unwrap();
        assert_eq!(rotates.turn.to_bits(), 0xc0c0_0000);
        let residual = direct_terminal_step(mode, GroundInput { turn: 0.5, ..edge }, args, [0.5, 0.0], 0.0).unwrap();
        assert_eq!(residual.turn.to_bits(), 0x2480_0000);
        assert!(!residual.reached);
    }
}
