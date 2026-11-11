#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FootMotion {
    pub speed: f32,
    pub yaw_rate: f32,
    pub velocity: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootStep {
    pub xy: [f32; 2],
    pub yaw: f32,
    pub motion: FootMotion,
    pub arrived: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootLimits {
    pub forward: f32,
    pub angular: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    Nearest53,
    Chop53,
}

impl Rounding {
    fn chop(self, result: f64, residual: f64) -> f64 {
        if self == Self::Chop53 && result != 0.0 && result.is_finite()
            && residual != 0.0 && result.is_sign_negative() != residual.is_sign_negative()
        {
            f64::from_bits(result.to_bits() - 1)
        } else {
            result
        }
    }

    pub(super) fn add(self, a: f64, b: f64) -> f64 {
        let sum = a + b;
        if self == Self::Nearest53 { return sum; }
        let low = sum - a;
        self.chop(sum, (a - (sum - low)) + (b - low))
    }

    pub(super) fn sub(self, a: f64, b: f64) -> f64 {
        self.add(a, -b)
    }

    pub(super) fn mul(self, a: f64, b: f64) -> f64 {
        let product = a * b;
        if self == Self::Nearest53 { return product; }
        self.chop(product, a.mul_add(b, -product))
    }

    pub(super) fn div(self, a: f64, b: f64) -> f64 {
        let quotient = a / b;
        if self == Self::Nearest53 || b == 0.0 { return quotient; }
        self.chop(quotient, (-quotient).mul_add(b, a) * b.signum())
    }

    pub(super) fn sqrt(self, value: f64) -> f64 {
        let root = value.sqrt();
        if self == Self::Nearest53 { return root; }
        self.chop(root, (-root).mul_add(root, value))
    }
}

pub(super) const DEER_SPEED: f32 = f32::from_bits(0x4004_7ae1);
pub(super) const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
pub(super) const TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const RAW_ANGULAR: f64 = 720.0 * f32::from_bits(0x3c8e_fa35) as f64;
const ARRIVAL: f32 = f32::from_bits(0x3d4c_cccd);
const ALIGNMENT: f64 = f32::from_bits(0x3f35_04f3) as f64;
const ANGLE_EPS: f32 = f32::from_bits(0x38d1_b717);

pub(super) fn store(value: f64, rounding: Rounding) -> f32 {
    let nearest = value as f32;
    if rounding == Rounding::Chop53 && f64::from(nearest).abs() > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

pub(super) fn wrap_positive(mut angle: f64, rounding: Rounding) -> f64 {
    while angle > TAU {
        angle = rounding.sub(angle, TAU);
    }
    while angle < 0.0 {
        angle = rounding.add(angle, TAU);
    }
    angle
}

pub(super) fn dot(a: [f32; 2], b: [f32; 2], rounding: Rounding) -> f64 {
    rounding.add(f64::from(a[0]) * f64::from(b[0]), f64::from(a[1]) * f64::from(b[1]))
}

fn facing(yaw: f64, rounding: Rounding) -> [f32; 2] {
    let (sin, cos) = yaw.sin_cos();
    [store(cos, rounding), store(sin, rounding)]
}

pub(super) fn angular_rate(
    current: FootMotion,
    yaw: f32,
    target_yaw: f32,
    dt: f64,
    limits: FootLimits,
    angular_acceleration: f32,
    acceleration: f32,
    slow_speed: f32,
    braking: bool,
    angle_epsilon: f32,
    rounding: Rounding,
) -> f32 {
    let FootLimits { forward, angular: max_angular } = limits;
    let mut cap = max_angular;
    if !braking {
        let speed_margin = rounding.sub(f64::from(forward), rounding.sub(f64::from(current.speed),
            dt * f64::from(acceleration))).max(0.0);
        let rate_cap = rounding.div(rounding.mul(speed_margin, f64::from(max_angular)),
            f64::from(forward) - f64::from(slow_speed));
        if rate_cap < f64::from(cap) {
            cap = store(rate_cap, rounding);
        }
    }

    let mut difference = rounding.sub(f64::from(target_yaw), f64::from(yaw));
    while difference > PI {
        difference = rounding.sub(difference, TAU);
    }
    while difference <= -PI {
        difference = rounding.add(difference, TAU);
    }
    let difference32 = store(difference, rounding);
    let old = f64::from(current.yaw_rate);
    if old * difference > 0.0 {
        let stopping_rate = rounding.div(f64::from(difference32).abs(),
            rounding.div(old.abs(), f64::from(angular_acceleration)));
        if stopping_rate < f64::from(cap) {
            cap = store(stopping_rate, rounding);
        }
    }
    if difference32.abs() < angle_epsilon {
        cap = 0.0;
    }
    let inverse_dt = rounding.div(1.0, dt);
    let desired = store(rounding.mul(f64::from(difference32), inverse_dt), rounding);
    let desired = if desired <= -cap { -cap } else { desired.min(cap) };
    let acceleration_register = rounding.mul(rounding.sub(f64::from(desired), old), inverse_dt);
    let acceleration32 = store(acceleration_register, rounding);
    let selected = if f64::from(acceleration32) * old < 0.0 {
        acceleration_register
    } else {
        let lower = -10.0 * f64::from(angular_acceleration);
        let upper = f64::from(store(10.0 * f64::from(angular_acceleration), rounding));
        f64::from(acceleration32).clamp(lower, upper)
    };
    if selected == f64::from(acceleration32) {
        desired
    } else {
        store(rounding.add(rounding.mul(selected, dt), old), rounding)
    }
}

/// The detached final-waypoint Foot branch with flat terrain, speed multiplier
/// one, no contact/formation state and ordinary Deer Move data.
pub fn deer_foot_step(
    xy: [f32; 2],
    yaw: f32,
    target: [f32; 2],
    dt: f32,
    current: FootMotion,
    rounding: Rounding,
) -> FootStep {
    steer_foot(xy, yaw, target, dt, current, rounding, |_| false)
}

pub(super) fn steer_foot(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, current: FootMotion,
    rounding: Rounding, contact_active: impl FnOnce([f32; 2]) -> bool,
) -> FootStep {
    steer_foot_at_speed(xy, yaw, target, dt, current, rounding, DEER_SPEED, contact_active)
}

pub(super) fn steer_foot_at_speed(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, current: FootMotion,
    rounding: Rounding, speed: f32, contact_active: impl FnOnce([f32; 2]) -> bool,
) -> FootStep {
    steer_foot_with_limits(xy, yaw, target, dt, current, rounding, speed, stock_foot_limits(speed, rounding), contact_active)
}

pub(super) fn stock_foot_limits(speed: f32, rounding: Rounding) -> FootLimits {
    FootLimits { forward: speed, angular: store(RAW_ANGULAR, rounding) }
}

pub(super) fn steer_foot_with_limits(
    xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, current: FootMotion,
    rounding: Rounding, speed: f32, limits: FootLimits, contact_active: impl FnOnce([f32; 2]) -> bool,
) -> FootStep {
    assert!(dt > 0.0 && dt.is_finite() && (0.0..=TAU).contains(&f64::from(yaw)));
    let FootLimits { forward, angular: max_angular } = limits;
    let dt = f64::from(dt);
    let acceleration = store(rounding.div(f64::from(speed), f64::from(0.3_f32)), rounding);
    let angular_acceleration = store(rounding.div(RAW_ANGULAR, f64::from(0.1_f32)), rounding);
    let slow_speed = store(f64::from(forward) * f64::from(0.6_f32), rounding);

    let exact_delta = [rounding.sub(f64::from(target[0]), f64::from(xy[0])),
        rounding.sub(f64::from(target[1]), f64::from(xy[1]))];
    let distance = store(rounding.sqrt(rounding.add(rounding.mul(exact_delta[0], exact_delta[0]),
        rounding.mul(exact_delta[1], exact_delta[1]))), rounding);
    let delta = exact_delta.map(|value| store(value, rounding));
    let length = store(rounding.sqrt(dot(delta, delta, rounding)), rounding);
    let direction = if length == 0.0 {
        [0.0; 2]
    } else {
        let reciprocal = rounding.div(1.0, f64::from(length));
        delta.map(|value| store(rounding.mul(f64::from(value), reciprocal), rounding))
    };

    let braking = distance < ARRIVAL;
    let target_yaw = if braking || f64::from(length) <= 0.00001 {
        yaw
    } else {
        let mut bearing = f64::from(direction[1]).atan2(f64::from(direction[0]));
        if bearing < 0.0 {
            bearing = rounding.add(bearing, TAU);
        }
        store(bearing, rounding)
    };
    let blocked = contact_active(direction);
    let yaw_rate = if blocked && !braking { current.yaw_rate } else {
        angular_rate(current, yaw, target_yaw, dt, limits,
            angular_acceleration, acceleration, slow_speed, braking, ANGLE_EPS, rounding)
    };
    let yaw_step = wrap_positive(rounding.add(f64::from(yaw), dt * f64::from(yaw_rate)), rounding);

    let (speed, velocity) = if braking {
        let speed_register = rounding.sub(f64::from(current.speed), dt * f64::from(acceleration));
        if speed_register <= 0.0 {
            (0.0, [0.0; 2])
        } else {
            let ratio = rounding.div(speed_register, f64::from(current.speed));
            (store(speed_register, rounding),
                current.velocity.map(|component| store(rounding.mul(ratio, f64::from(component)), rounding)))
        }
    } else {
        let movement = if blocked || distance < 1.0 { direction } else { facing(yaw_step, rounding) };
        let step_speed = dt * f64::from(acceleration);
        let old_speed = f64::from(current.speed);
        let speed = if dot(movement, direction, rounding) < ALIGNMENT {
            let desired = (0.25 * f64::from(forward)).min(f64::from(slow_speed));
            if desired > old_speed {
                store(rounding.add(old_speed, step_speed).min(desired), rounding)
            } else {
                let braking_step = rounding.mul(3.0 * f64::from(acceleration), dt);
                store(rounding.sub(old_speed, braking_step).max(desired), rounding)
            }
        } else {
            let turn = rounding.div(f64::from(yaw_rate.abs()), f64::from(max_angular));
            let mut desired = rounding.add(rounding.mul(rounding.sub(1.0, turn), f64::from(forward)),
                rounding.mul(turn, f64::from(slow_speed)));
            let braking_cap = if distance <= ARRIVAL {
                0.0
            } else {
                let stopping_time = rounding.div(old_speed, 3.0 * f64::from(acceleration));
                store(rounding.div(f64::from(distance), stopping_time), rounding)
            };
            desired = desired.min(f64::from(braking_cap));
            let remaining = if current.speed == 0.0 || length == 0.0 {
                length
            } else {
                let alignment = rounding.div(dot(facing(f64::from(yaw), rounding), direction, rounding),
                    f64::from(length)).clamp(0.5, 1.0);
                let alignment32 = store(alignment, rounding);
                store(f64::from(alignment32) * f64::from(length), rounding)
            };
            let inverse = rounding.div(1.0, dt);
            let target_speed = store(rounding.mul(inverse, f64::from(remaining)).min(desired), rounding);
            let wanted_acceleration = store(rounding.mul(rounding.sub(f64::from(target_speed), old_speed),
                f64::from(store(inverse, rounding))), rounding);
            if wanted_acceleration <= acceleration {
                target_speed.max(0.0)
            } else {
                store(rounding.add(old_speed, f64::from(acceleration) * dt), rounding).max(0.0)
            }
        };
        let velocity = movement.map(|component| store(f64::from(component) * f64::from(speed), rounding));
        (speed, velocity)
    };

    let arrived = speed == 0.0 && yaw_rate == 0.0 && distance <= ARRIVAL;
    let x = store(rounding.add(f64::from(xy[0]), f64::from(velocity[0]) * dt), rounding);
    let dy = store(f64::from(velocity[1]) * dt, rounding);
    let y = store(rounding.add(f64::from(xy[1]), f64::from(dy)), rounding);
    FootStep {
        xy: [x, y],
        yaw: store(yaw_step, rounding),
        motion: FootMotion { speed, yaw_rate, velocity },
        arrived,
    }
}

#[cfg(feature = "ee2-rect249-deer-contact")]
pub(super) fn maximum_yaw_rate(rounding: Rounding) -> f32 {
    store(RAW_ANGULAR, rounding)
}
