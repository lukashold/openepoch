use super::{resolve_fresh, Precision};

#[derive(Clone, Copy)]
pub struct ForceState {
    pub retained: [f32; 3],
    pub last: f32,
    pub stamp: f32,
    pub origin: [f32; 2],
}

impl Default for ForceState {
    fn default() -> Self {
        Self { retained: [0.0; 3], last: -1.0, stamp: 0.0, origin: [0.0; 2] }
    }
}

impl ForceState {
    pub fn active(self) -> bool {
        self.stamp > 0.0 || self.retained != [0.0; 3]
    }

    pub fn words(self) -> [u32; 7] {
        [self.retained[0], self.retained[1], self.retained[2], self.last, self.stamp,
            self.origin[0], self.origin[1]].map(f32::to_bits)
    }

    pub fn from_words(words: [u32; 7]) -> Option<Self> {
        let [x, y, z, last, stamp, ox, oy] = words.map(f32::from_bits);
        let force = Self { retained: [x, y, z], last, stamp, origin: [ox, oy] };
        force.valid(f32::INFINITY).then_some(force)
    }

    pub fn valid(self, now: f32) -> bool {
        let [x, y, z] = self.retained;
        let [ox, oy] = self.origin;
        let has_origin = (0.0..115.0).contains(&ox) && (0.0..71.0).contains(&oy);
        let phase = if self.stamp > 0.0 {
            self.last.is_finite() && self.stamp.is_finite()
                && self.last > 0.0 && self.last <= now && self.stamp <= now && has_origin
        } else if self.stamp == -1.0 {
            self.last == -1.0 && has_origin
        } else {
            self.words() == Self::default().words()
        };
        phase && x.hypot(y) <= 0.25001 && z.to_bits() == 0
    }
}

pub struct PhysicalInput {
    pub position: [f32; 2],
    pub projected: [f32; 2],
    pub heading: f32,
    pub forward: f32,
    pub turn: f32,
    pub max_turn: f32,
    pub max_speed: f32,
    pub now: f32,
    pub dt: f32,
    pub pending: [f32; 3],
}

pub struct PhysicalResult {
    pub position: [f32; 2],
    pub heading: f32,
    pub turn: f32,
    pub force: ForceState,
}

/// 0085C585 before the regular steering calculation.
pub fn refresh_stamp(mode: Precision, mut state: ForceState, position: [f32; 2], velocity: [f32; 2], ray: [f32; 2], now: f32) -> ForceState {
    let [rx, ry] = ray.map(f64::from);
    let [vx, vy] = velocity.map(f64::from);
    let [dx, dy] = std::array::from_fn(|axis| f64::from(position[axis]) - f64::from(state.origin[axis]));
    let progress = mode.add(mode.mul(dx, rx), mode.mul(dy, ry));
    let forward = mode.add(vx * rx, vy * ry);
    if state.stamp > 0.0 && (progress > 0.003 || forward < 0.0) {
        state.stamp = now;
        state.origin = position;
    }
    state
}

pub fn projected(mode: Precision, origin: [f32; 2], velocity: [f32; 2], dt: f32) -> [f32; 2] {
    std::array::from_fn(|axis| mode.store(mode.add(f64::from(origin[axis]), f64::from(velocity[axis]) * f64::from(dt))))
}

const RETAIN: f64 = 0.8_f32 as f64;
const TURN_SCALE: f64 = 0.6_f32 as f64;
const PI: f64 = f32::from_bits(0x40490fdb) as f64;
const TAU: f64 = f32::from_bits(0x40c90fdb) as f64;

fn blend(mode: Precision, previous: ForceState, pending: [f32; 3]) -> [f32; 3] {
    if previous.retained[..2] == [0.0; 2] {
        return pending;
    }
    if previous.last < 0.0 {
        return [0.0; 3];
    }
    std::array::from_fn(|axis| {
        let remaining = mode.store(mode.mul(f64::from(previous.retained[axis]), RETAIN));
        let arrival = mode.mul(f64::from(pending[axis]), 1.0 - RETAIN);
        let arrival = if axis == 0 { arrival } else { f64::from(mode.store(arrival)) };
        mode.store(mode.add(f64::from(remaining), arrival))
    })
}

fn rotate_from_contact(mode: Precision, input: &PhysicalInput, position: [f32; 2]) -> f64 {
    let delta: [f32; 2] = std::array::from_fn(|axis| {
        mode.store(f64::from(position[axis]) - f64::from(input.position[axis]))
    });
    let [x, y] = delta.map(f64::from);
    let direction = y.atan2(x);
    let mut diff = mode.add(direction, -f64::from(input.heading));
    if diff > PI {
        diff = mode.add(diff, -TAU);
    }
    if diff < -PI {
        diff = mode.add(diff, TAU);
    }
    let limit = mode.store(mode.mul(mode.mul(f64::from(input.dt), TURN_SCALE), f64::from(input.max_turn)));
    f64::from(mode.store(diff.clamp(-f64::from(limit), f64::from(limit))))
}

pub fn physical(mode: Precision, input: PhysicalInput, mut force: ForceState) -> PhysicalResult {
    force.retained = blend(mode, force, input.pending);
    let pose = resolve_fresh(mode, input.position, input.projected, force.retained, input.dt, input.max_speed);
    if pose.stamp {
        if force.stamp <= 0.0 {
            force.stamp = input.now;
            force.origin = input.position;
        }
        force.last = input.now;
    }
    let active = force.stamp > 0.0 && input.forward != 0.0;
    let turn = if active { 0.0 } else { input.turn };
    let delta = if active {
        rotate_from_contact(mode, &input, pose.point)
    } else {
        mode.mul(f64::from(turn), f64::from(input.dt))
    };
    let mut angle = mode.add(f64::from(input.heading), delta);
    if angle >= TAU {
        angle = mode.add(angle, -TAU);
    }
    if angle < 0.0 {
        angle = mode.add(angle, TAU);
    }
    if force.stamp > 0.0 && f64::from(input.now) - 1.0 > f64::from(force.last) {
        force.stamp = -1.0;
        force.last = -1.0;
    }
    PhysicalResult { position: pose.point, heading: mode.store(angle), turn, force }
}
