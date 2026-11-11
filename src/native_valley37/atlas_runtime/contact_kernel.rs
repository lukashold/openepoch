//! Detached ordinary same-owner Citizen footprint and contact force.

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

    fn div(self, a: f64, b: f64) -> f64 {
        let nearest = a / b;
        let residual = (-nearest).mul_add(b, a);
        self.round(nearest, if b.is_sign_negative() { -residual } else { residual })
    }

    fn sqrt(self, value: f64) -> f64 {
        let nearest = value.sqrt();
        self.round(nearest, (-nearest).mul_add(nearest, value))
    }

    fn squared(self, [x, y]: [f32; 2]) -> f64 {
        let [x, y] = [f64::from(x), f64::from(y)];
        self.add(x * x, y * y)
    }

    fn rotate(self, [x, y]: [f32; 2], yaw: f64) -> [f32; 2] {
        let (sin, cos) = yaw.sin_cos();
        let [sin, cos, x, y] = [self.store(sin), self.store(cos), x, y].map(f64::from);
        [self.store(self.add(cos * x, -sin * y)), self.store(self.add(cos * y, sin * x))]
    }

    fn normal(self, vector: [f32; 2], length: f64) -> [f32; 2] {
        let inverse = self.div(1.0, length);
        vector.map(|axis| self.store(self.mul(f64::from(axis), inverse)))
    }
}

#[derive(Clone, Copy)]
pub struct Footprint {
    pub center: [f32; 2],
    pub yaw: f32,
}

#[derive(Clone, Copy)]
pub struct Overlap {
    pub hit: bool,
    pub depth: f32,
    pub push: [f32; 2],
}

const CITIZEN_RADIUS: f64 = 0.125;
const COINCIDENT: f64 = 0.01;
const MIN_RESOLVED_SQUARED: f64 = 1e-5;

fn support(mode: Precision, body: Footprint, direction: [f32; 2], side: f64) -> [f32; 2] {
    let seed: [f32; 2] = std::array::from_fn(|axis| {
        mode.store(mode.add(f64::from(body.center[axis]), side * 4.0 * CITIZEN_RADIUS * f64::from(direction[axis])))
    });
    let delta = std::array::from_fn(|axis| mode.store(f64::from(seed[axis]) - f64::from(body.center[axis])));
    let local = mode.rotate(delta, -f64::from(body.yaw)).map(|axis| mode.store(f64::from(axis) / CITIZEN_RADIUS));
    let direction = mode.normal(local, mode.sqrt(mode.squared(local)));
    let delta = mode.rotate(direction.map(|axis| mode.store(f64::from(axis) * CITIZEN_RADIUS)), f64::from(body.yaw));
    std::array::from_fn(|axis| mode.store(mode.add(f64::from(body.center[axis]), f64::from(delta[axis]))))
}

/// Reproduce the two stock circular footprints from 00848D50.
pub fn overlap(mode: Precision, mover: Footprint, other: Footprint) -> Option<Overlap> {
    for body in [mover, other] {
        if !body.center.into_iter().all(|axis| (0.0..=256.0).contains(&axis))
            || !(-std::f32::consts::TAU..=std::f32::consts::TAU).contains(&body.yaw)
        {
            return None;
        }
    }
    let delta = std::array::from_fn(|axis| mode.store(mode.add(f64::from(mover.center[axis]), -f64::from(other.center[axis]))));
    let distance = mode.sqrt(mode.squared(delta));
    if distance < COINCIDENT {
        return Some(Overlap { hit: true, depth: CITIZEN_RADIUS as f32, push: [0.0, CITIZEN_RADIUS as f32] });
    }
    let direction = mode.normal(delta, distance);
    let left = support(mode, mover, direction, -1.0);
    let right = support(mode, other, direction, 1.0);
    let left_delta = std::array::from_fn(|axis| mode.store(f64::from(left[axis]) - f64::from(mover.center[axis])));
    let [right_x, right_y] = std::array::from_fn(|axis| f64::from(right[axis]) - f64::from(other.center[axis]));
    let left_extent = mode.sqrt(mode.squared(left_delta));
    let right_extent = mode.sqrt(mode.add(mode.mul(right_x, right_x), mode.mul(right_y, right_y)));
    let raw_depth = mode.add(mode.add(left_extent, right_extent), -f64::from(mode.store(distance)));
    let depth = mode.store(raw_depth);
    let hit = raw_depth > 0.0;
    let push = if hit { direction.map(|axis| mode.store(f64::from(axis) * f64::from(depth))) } else { [0.0; 2] };
    Some(Overlap { hit, depth, push })
}

#[derive(Clone, Copy)]
pub struct Motion {
    pub planar_velocity: [f32; 2],
    pub forward: f32,
    pub turn: f32,
    pub type_half: [f32; 2],
    pub pending: [f32; 3],
}

impl Motion {
    fn metric(self, mode: Precision) -> f64 {
        mode.sqrt(mode.squared(self.type_half)) * 0.5
    }

    fn moving(self) -> bool {
        self.planar_velocity != [0.0; 2] || self.turn != 0.0
    }
}

/// 0085D760 weights for two same-owner kind-two footprints without group activity.
pub fn weighted(mode: Precision, hit: Overlap, mover: Motion, other: Motion) -> [[f32; 3]; 2] {
    if !hit.hit {
        return [mover.pending, other.pending];
    }
    let a = mover.metric(mode);
    let b = other.metric(mode);
    let share = mode.div(f64::from(mode.store(a)), mode.add(a, f64::from(mode.store(b))));
    let cutoff = f64::from(0.6_f32);
    let factors = if !mover.moving() && !other.moving() {
        [mode.store(mode.add(1.0, -share)), mode.store(share)]
    } else if share > cutoff {
        [0.0, 1.0]
    } else if share < 1.0 - cutoff || mover.forward >= other.forward {
        [1.0, 0.0]
    } else {
        [0.0, 1.0]
    };
    let [x, y] = hit.push;
    let push = [x, y, 0.0];
    std::array::from_fn(|actor| {
        let before = [mover.pending, other.pending][actor];
        let scalar = if actor == 0 { factors[0] } else { -factors[1] };
        if scalar == 0.0 {
            return before;
        }
        std::array::from_fn(|axis| {
            let change = mode.store(f64::from(push[axis]) * f64::from(scalar));
            mode.store(mode.add(f64::from(before[axis]), f64::from(change)))
        })
    })
}

#[derive(Clone, Copy)]
pub struct ResolvedPose {
    pub point: [f32; 2],
    pub stamp: bool,
}

/// Fresh planar 0085E979 branch with no prior accumulation or support override.
pub fn resolve_fresh(mode: Precision, origin: [f32; 2], projected: [f32; 2], external: [f32; 3], dt: f32, speed: f32) -> ResolvedPose {
    let [x, y, z] = external.map(f64::from);
    let magnitude = mode.add(mode.add(x * x, y * y), z * z);
    if magnitude <= MIN_RESOLVED_SQUARED {
        return ResolvedPose { point: projected, stamp: false };
    }
    let point: [f32; 2] = std::array::from_fn(|axis| {
        mode.store(mode.add(f64::from(projected[axis]), f64::from(external[axis])))
    });
    let delta = std::array::from_fn(|axis| mode.store(mode.add(f64::from(point[axis]), -f64::from(origin[axis]))));
    let squared = f64::from(mode.store(mode.squared(delta)));
    let minimum = f64::from(dt) * f64::from(0.003_f32);
    let maximum = mode.store(mode.mul(f64::from(dt) * f64::from(1.2_f32), f64::from(speed)));
    let cap = if squared < minimum * minimum {
        mode.store(minimum)
    } else if squared > f64::from(maximum) * f64::from(maximum) {
        maximum
    } else {
        return ResolvedPose { point, stamp: true };
    };
    let length = mode.sqrt(squared);
    let direction = if length > 0.0 { mode.normal(delta, length) } else { delta };
    let point = std::array::from_fn(|axis| {
        mode.store(mode.add(f64::from(origin[axis]), f64::from(direction[axis]) * f64::from(cap)))
    });
    ResolvedPose { point, stamp: true }
}

#[path = "lifecycle.rs"]
pub mod lifecycle;
