//! Ordinary no-hover Airplane Move with stock Hawk/Vulture limits and a direct path.
use super::deer_foot247::{FootLimits, FootMotion, Rounding, TAU, angular_rate, store, wrap_positive};

const FORWARD: f32 = f32::from_bits(0x405c_cccd);
const BASE_ANGULAR: f64 = 90.0 * f32::from_bits(0x3c8e_fa35) as f64;
const TAKEOFF_THRESHOLD: f32 = f32::from_bits(0x3f66_6666);
const SLOW_FORWARD_RATIO: f32 = f32::from_bits(0x3f4c_cccd);
const ARRIVAL_DISTANCE: f32 = f32::from_bits(0x3d4c_cccd);
const DIAGONAL: f64 = f32::from_bits(0x3f49_0fdb) as f64;
const CRUISE_AGL: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AirPhase {
    Takeoff = 1,
    Travel = 2,
    Circle = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircleStart {
    pub anchor: [f32; 3],
    pub turn_left: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirplaneState {
    pub position: [f32; 3],
    pub yaw: f32,
    pub motion: FootMotion,
    pub phase: AirPhase,
    pub takeoff_agl: f32,
    pub circle: Option<CircleStart>,
    pub(super) unreachable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirplaneMove {
    pub target: [f32; 3],
    pub direct_path_ready: bool,
    pub arrived: bool,
}

impl AirplaneMove {
    pub fn direct(target: [f32; 3]) -> Self {
        assert!(target.into_iter().all(f32::is_finite));
        Self { target, direct_path_ready: false, arrived: false }
    }

    pub(super) fn clamp_path(&mut self, map_max_xy: [f32; 2]) {
        for (v, limit) in self.target[..2].iter_mut().zip(map_max_xy) {
            *v = v.clamp(0.0, limit);
        }
        self.direct_path_ready = true;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlightFrame {
    pub dt: f32,
    pub now: f32,
    pub packed_actor_id: u32,
    /// Maximum anchor coordinates from original terrain corner counts minus one.
    pub map_max_xy: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightStep {
    pub phase_before_commit: AirPhase,
    pub arrived: bool,
}

fn stock_limits(rounding: Rounding) -> (FootLimits, f32) {
    let angular = store(BASE_ANGULAR, rounding);
    let angular_acceleration = store(rounding.div(BASE_ANGULAR, f64::from(0.1_f32)), rounding);
    (FootLimits { forward: FORWARD, angular }, angular_acceleration)
}

/// Starts the original phase-0 actor branch. The terrain callback returns the
/// widened value before the source's optional zero clamp or 32-bit spill.
pub fn start_airplane(
    position: [f32; 3],
    yaw: f32,
    rounding: Rounding,
    mut terrain_height: impl FnMut([f32; 2]) -> f64,
) -> AirplaneState {
    assert!(position.into_iter().all(f32::is_finite) && (0.0..=TAU).contains(&f64::from(yaw)));
    let ground = terrain_height([position[0], position[1]]).max(0.0);
    AirplaneState {
        position, yaw, motion: FootMotion::default(), phase: AirPhase::Takeoff,
        takeoff_agl: store(rounding.sub(f64::from(position[2]), ground), rounding),
        circle: None, unreachable: false,
    }
}

fn facing(yaw: f64, rounding: Rounding) -> [f32; 2] {
    let cos = yaw.cos();
    let sin = yaw.sin();
    if rounding == Rounding::Chop53 && yaw != 0.0 {
        // FSINCOS can retain the tiny deficit from one before FSTP. A
        // nearest f64 libm result of exactly one loses that direction.
        let spill = |value: f64, lost_deficit: bool| {
            let scalar = store(value, rounding);
            if lost_deficit && scalar != 0.0 && f64::from(scalar) == value {
                f32::from_bits(scalar.to_bits() - 1)
            } else { scalar }
        };
        [spill(cos, cos.abs() == 1.0), spill(sin, sin.abs() == 1.0 || sin == yaw)]
    } else {
        [store(cos, rounding), store(sin, rounding)]
    }
}

fn travel_motion(
    state: &AirplaneState, target: [f32; 3], dt: f64, rounding: Rounding,
) -> (f32, f32, bool) {
    let [dx, dy] = [0, 1].map(|axis| f64::from(store(rounding.sub(
        f64::from(target[axis]), f64::from(state.position[axis])), rounding)));
    let length = rounding.sqrt(rounding.add(rounding.mul(dx, dx), rounding.mul(dy, dy)));
    let bearing = if length <= f64::from(ARRIVAL_DISTANCE) {
        state.yaw
    } else {
        let inverse = rounding.div(1.0, length);
        let [nx, ny] = [dx, dy].map(|v| f64::from(store(rounding.mul(v, inverse), rounding)));
        store(wrap_positive(ny.atan2(nx), rounding), rounding)
    };

    let (limits, angular_acceleration) = stock_limits(rounding);
    let forward = f64::from(limits.forward);
    let slow_forward = store(rounding.mul(f64::from(SLOW_FORWARD_RATIO), forward), rounding);
    let yaw_rate = angular_rate(state.motion, state.yaw, bearing, dt, limits,
        angular_acceleration, FORWARD, slow_forward, true, -1.0, rounding);
    let fraction = rounding.div(f64::from(yaw_rate).abs(), f64::from(limits.angular));
    let wanted = rounding.add(rounding.mul(rounding.sub(1.0, fraction), forward),
        rounding.mul(fraction, f64::from(slow_forward)));
    let old_speed = f64::from(state.motion.speed);
    let delta = rounding.mul(dt, forward);
    let speed = store(if wanted < old_speed {
        rounding.sub(old_speed, delta).max(0.0)
    } else {
        rounding.add(old_speed, delta).min(wanted)
    }, rounding);
    let arrival_radius = rounding.add(f64::from(ARRIVAL_DISTANCE), rounding.mul(dt, f64::from(speed)));
    (speed, yaw_rate, f64::from(store(length, rounding)) <= arrival_radius)
}

// 007D7093–007D728D. A point inside either turning circle is temporarily
// unreachable. The latched half-unit margin keeps the approach from oscillating.
fn unreachable_waypoint(state: &mut AirplaneState, target: [f32; 3], rounding: Rounding) -> bool {
    let (limits, _) = stock_limits(rounding);
    let slow = store(rounding.mul(f64::from(SLOW_FORWARD_RATIO), f64::from(FORWARD)), rounding);
    let base_radius = store(rounding.div(f64::from(slow), f64::from(limits.angular)), rounding);
    let radius = if state.unreachable { store(rounding.add(f64::from(base_radius), 0.5), rounding) }
        else { base_radius };
    let radius = f64::from(radius);
    let [x, y] = [f64::from(state.position[0]), f64::from(state.position[1])];
    let [gx, gy] = [f64::from(target[0]), f64::from(target[1])];
    let dx = f64::from(store(rounding.sub(gx, x), rounding));
    let dy = f64::from(store(rounding.sub(gy, y), rounding));
    let sq = |v| rounding.mul(v, v);
    let distance2 = rounding.add(sq(dy), sq(dx));
    if rounding.mul(sq(radius), 4.0) <= distance2 {
        return false;
    }
    let [cos, sin] = facing(f64::from(state.yaw), rounding).map(f64::from);
    let left_x = rounding.add(x, f64::from(store(rounding.mul(-sin, radius), rounding)));
    let left_y = f64::from(store(rounding.add(y, rounding.mul(cos, radius)), rounding));
    let right_x = f64::from(store(rounding.add(x, f64::from(store(rounding.mul(sin, radius), rounding))), rounding));
    let right_y = f64::from(store(rounding.add(y, rounding.mul(-cos, radius)), rounding));
    let inner = f64::from(store(rounding.sub(radius, 0.1), rounding));
    let limit2 = f64::from(store(sq(inner), rounding));
    let contains = |cx, cy| rounding.add(sq(rounding.sub(cy, gy)), sq(rounding.sub(cx, gx))) < limit2;
    state.unreachable = contains(left_x, left_y) || contains(right_x, right_y);
    state.unreachable
}

fn step_altitude(
    position: [f32; 3], yaw: f32, direction: [f32; 2], agl: f32, vertical_rate: f64,
    dt: f64, rounding: Rounding, terrain_height: &mut impl FnMut([f32; 2]) -> f64,
) -> f32 {
    let [cx, sy] = direction.map(f64::from);
    let [diagonal_x, diagonal_y] = facing(rounding.add(f64::from(yaw), DIAGONAL), rounding).map(f64::from);
    let mut highest = f32::NEG_INFINITY;
    for [dx, dy] in [[0.0, 0.0], [cx, sy], [cx * 2.0, sy * 2.0],
        [diagonal_x, diagonal_y], [diagonal_y, -diagonal_y]]
    {
        let query = [store(rounding.add(f64::from(position[0]), dx), rounding),
            store(rounding.add(f64::from(position[1]), dy), rounding)];
        let ground = store(terrain_height(query), rounding).max(0.0);
        highest = highest.max(store(rounding.add(f64::from(ground), f64::from(agl)), rounding));
    }
    let old_z = f64::from(position[2]);
    let delta = rounding.sub(f64::from(highest), old_z);
    let magnitude = store(delta, rounding).abs();
    let multiplier = if magnitude > 2.5 { 7.0 } else if magnitude > 0.5 { 3.0 } else { 1.0 };
    let base_cap = f64::from(store(rounding.mul(dt, vertical_rate), rounding));
    let cap = rounding.mul(base_cap, multiplier);
    let offset = store(delta.clamp(-cap, cap), rounding);
    store(rounding.add(old_z, f64::from(offset)), rounding)
}

/// Advance task update and event-1 pose commit through the arrival frame.
/// An ongoing phase-3 loiter uses a separate controller and has no task step.
pub fn step_airplane(
    state: &mut AirplaneState,
    task: &mut AirplaneMove,
    frame: FlightFrame,
    rounding: Rounding,
    mut terrain_height: impl FnMut([f32; 2]) -> f64,
) -> Option<FlightStep> {
    if task.arrived || state.phase == AirPhase::Circle {
        return None;
    }
    assert!(frame.dt > 0.0 && frame.dt.is_finite() && frame.now >= 0.0 && frame.now.is_finite());
    assert!(frame.map_max_xy.into_iter().all(|end| end >= 0.0 && end.is_finite()));
    let dt = f64::from(frame.dt);
    let old = state.position;
    let forward = f64::from(FORWARD);
    let (speed, yaw_rate, arrived) = if state.phase == AirPhase::Takeoff {
        unreachable_waypoint(state, task.target, rounding);
        let acceleration = store(rounding.div(forward, rounding.div(f64::from(CRUISE_AGL),
            rounding.mul(forward, 0.5))), rounding);
        let old_speed = f64::from(state.motion.speed);
        if old_speed >= rounding.mul(f64::from(TAKEOFF_THRESHOLD), forward) {
            state.phase = AirPhase::Travel;
        }
        let speed = rounding.add(old_speed, rounding.mul(dt, f64::from(acceleration))).min(forward);
        (store(speed, rounding), 0.0, false)
    } else {
        if !task.direct_path_ready { task.clamp_path(frame.map_max_xy); }
        if unreachable_waypoint(state, task.target, rounding) {
            let (_, angular_acceleration) = stock_limits(rounding);
            let speed = move_scalar(state.motion.speed, FORWARD, rounding.mul(dt, forward), rounding);
            let yaw_rate = move_scalar(state.motion.yaw_rate, 0.0, rounding.mul(dt, f64::from(angular_acceleration)), rounding);
            (speed, yaw_rate, false)
        } else {
            travel_motion(state, task.target, dt, rounding)
        }
    };
    let phase_before_commit = state.phase;
    let (agl, rate) = if state.phase == AirPhase::Takeoff { (state.takeoff_agl, 2.0) }
        else { (CRUISE_AGL, 1.0) };
    commit_airplane(state, speed, yaw_rate, dt, agl, rate, rounding, &mut terrain_height);
    if arrived {
        task.arrived = true;
        state.phase = AirPhase::Circle;
        let turn_left = ((frame.packed_actor_id & 0x07ff_ffff).wrapping_add(frame.now as u32) & 1) == 0;
        state.circle = Some(CircleStart { anchor: [
            old[0].clamp(0.0, frame.map_max_xy[0]), old[1].clamp(0.0, frame.map_max_xy[1]),
            state.position[2],
        ], turn_left });
    }
    Some(FlightStep { phase_before_commit, arrived })
}

fn commit_airplane(
    state: &mut AirplaneState, speed: f32, yaw_rate: f32, dt: f64, agl: f32, rate: f64,
    rounding: Rounding, terrain_height: &mut impl FnMut([f32; 2]) -> f64,
) {
    let old = state.position;
    let direction = facing(f64::from(state.yaw), rounding);
    let velocity = direction.map(|v| store(rounding.mul(f64::from(v), f64::from(speed)), rounding));
    let z = step_altitude(old, state.yaw, direction, agl, rate, dt, rounding, terrain_height);
    let yaw_delta = rounding.mul(dt, f64::from(yaw_rate));
    state.yaw = store(wrap_positive(rounding.add(f64::from(state.yaw), yaw_delta), rounding), rounding);
    state.motion = FootMotion { speed, yaw_rate, velocity };
    state.position = [
        store(rounding.add(f64::from(old[0]), f64::from(store(rounding.mul(f64::from(velocity[0]), dt), rounding))), rounding),
        store(rounding.add(f64::from(old[1]), f64::from(store(rounding.mul(f64::from(velocity[1]), dt), rounding))), rounding),
        z,
    ];
}

fn angle_between(position: [f32; 3], anchor: [f32; 3], front: [f32; 3], rounding: Rounding) -> f32 {
    let a = [0, 1, 2].map(|i| rounding.sub(f64::from(anchor[i]), f64::from(position[i])));
    let b = [0, 1, 2].map(|i| rounding.sub(f64::from(front[i]), f64::from(position[i])));
    let [bx, by, bz] = [b[0], f64::from(store(b[1], rounding)), f64::from(store(b[2], rounding))];
    let projection = store(rounding.add(rounding.add(rounding.mul(bx, a[0]), rounding.mul(bz, a[2])),
        rounding.mul(by, a[1])), rounding);
    if projection == 0.0 {
        return DIAGONAL as f32 * 2.0;
    }
    let len_b = rounding.add(rounding.add(rounding.mul(by, by), rounding.mul(bz, bz)), rounding.mul(bx, bx));
    let len_a = rounding.add(rounding.add(rounding.mul(a[0], a[0]), rounding.mul(a[2], a[2])), rounding.mul(a[1], a[1]));
    let scale = rounding.sqrt(rounding.mul(len_a, len_b));
    let ratio = store(rounding.div(f64::from(projection), scale), rounding);
    store(f64::from(ratio.clamp(-1.0, 1.0)).acos(), rounding)
}

fn move_scalar(old: f32, target: f32, max_delta: f64, rounding: Rounding) -> f32 {
    if old == target { return old; }
    let old_wide = f64::from(old);
    let target_wide = f64::from(target);
    store(if old > target {
        rounding.sub(old_wide, max_delta).max(target_wide)
    } else {
        rounding.add(old_wide, max_delta).min(target_wide)
    }, rounding)
}

/// Run the stock phase-3 orbit event. UnitAI link ownership is external.
pub fn step_airplane_circle(
    state: &mut AirplaneState, dt: f32, rounding: Rounding,
    mut terrain_height: impl FnMut([f32; 2]) -> f64,
) -> bool {
    if state.phase != AirPhase::Circle { return false; }
    assert!(dt > 0.0 && dt.is_finite());
    let CircleStart { anchor, turn_left } = state.circle.expect("phase 3 requires a native orbit anchor");
    let dt = f64::from(dt);
    let (limits, angular_acceleration) = stock_limits(rounding);
    let orbit_speed = store(rounding.mul(f64::from(SLOW_FORWARD_RATIO), f64::from(limits.forward)), rounding);
    let radius = f64::from(store(rounding.div(rounding.mul(f64::from(SLOW_FORWARD_RATIO), f64::from(limits.forward)),
        f64::from(limits.angular)), rounding));
    let position = state.position;
    let [dx, dy] = [0, 1].map(|i| rounding.sub(f64::from(anchor[i]), f64::from(position[i])));
    let distance = f64::from(store(rounding.sqrt(rounding.add(rounding.mul(dx, dx), rounding.mul(dy, dy))), rounding));
    let [cos, sin] = facing(f64::from(state.yaw), rounding);
    let front = [store(rounding.add(f64::from(position[0]), f64::from(cos)), rounding),
        store(rounding.add(f64::from(position[1]), f64::from(sin)), rounding), position[2]];
    let angle = f64::from(angle_between(position, anchor, front, rounding));
    let turn = angle >= DIAGONAL || distance >= rounding.sub(radius, 1.0) && distance <= rounding.add(radius, 1.0);
    let desired_rate = if !turn { 0.0 } else if turn_left { limits.angular } else { -limits.angular };
    let speed = move_scalar(state.motion.speed, orbit_speed, rounding.mul(dt, f64::from(FORWARD)), rounding);
    let yaw_rate = move_scalar(state.motion.yaw_rate, desired_rate, rounding.mul(dt, f64::from(angular_acceleration)), rounding);
    commit_airplane(state, speed, yaw_rate, dt, CRUISE_AGL, 1.0, rounding, &mut terrain_height);
    true
}

/// Install a new direct point path while already circling. Call only after the
/// actual UnitAI/manager admission gate sees an empty root link.
pub fn restart_airplane(
    state: &mut AirplaneState, mut task: AirplaneMove, map_max_xy: [f32; 2],
) -> Option<AirplaneMove> {
    if state.phase != AirPhase::Circle || task.arrived || task.direct_path_ready { return None; }
    assert!(map_max_xy.into_iter().all(|end| end >= 0.0 && end.is_finite()));
    state.phase = AirPhase::Travel;
    task.clamp_path(map_max_xy);
    Some(task)
}
