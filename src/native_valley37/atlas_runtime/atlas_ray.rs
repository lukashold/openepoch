//! Walk a continuous line through the variable-size terminal atlas leaves.

use super::atlas_reader::{Precision53, quantized_floor};
use super::route_smoothing::Segment;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    pub origin_half: [u16; 2],
    pub level: u8,
    pub class: u8,
    pub density: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub start: [f32; 2],
    pub direction: [f32; 2],
    pub length: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FootCost {
    pub clearance: f32,
    pub density_weight: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub position: [f32; 2],
    pub next_position: [f32; 2],
    pub distance: f32,
    pub previous_leaf: Leaf,
    pub next_leaf: Leaf,
    pub segment_length: f32,
    pub segment_cost: f64,
}

fn round_exact(mode: Precision53, rounded: f64, residual: f64) -> f64 {
    if mode == Precision53::TowardZero && rounded != 0.0 && residual != 0.0
        && rounded.is_sign_negative() != residual.is_sign_negative()
    {
        f64::from_bits(rounded.to_bits() - 1)
    } else {
        rounded
    }
}

fn divide(mode: Precision53, numerator: f64, denominator: f64) -> f64 {
    let quotient = numerator / denominator;
    let residual = (-quotient).mul_add(denominator, numerator) * denominator.signum();
    round_exact(mode, quotient, residual)
}

fn multiply(mode: Precision53, a: f64, b: f64) -> f64 {
    let product = a * b;
    round_exact(mode, product, a.mul_add(b, -product))
}

pub fn make_ray(mode: Precision53, start: [f32; 2], end: [f32; 2]) -> Ray {
    let [dx, dy] = [0, 1].map(|axis| mode.add(f64::from(end[axis]), -f64::from(start[axis])));
    let [x, y] = [dx, dy].map(|difference| f64::from(mode.store_f32(difference)));
    let squared = mode.add(mode.mul(x, x), multiply(mode, dy, y));
    let root = squared.sqrt();
    let length = mode.store_f32(round_exact(mode, root, (-root).mul_add(root, squared)));
    let direction = if length == 0.0 {
        [0.0, 0.0]
    } else {
        let reciprocal = divide(mode, 1.0, f64::from(length));
        [x, y].map(|value| mode.store_f32(multiply(mode, value, reciprocal)))
    };
    Ray { start, direction, length }
}

/// Return the first point slightly across a terminal leaf edge. The position
/// comes from the ray origin and its absolute parameter, so errors do not
/// accumulate by adding one leaf crossing to the previous position.
pub fn advance(mode: Precision53, ray: Ray, position: [f32; 2], level: u8) -> (f32, [f32; 2]) {
    let width = f64::from(1_u32 << level) * 0.5;
    let crossings = [0, 1].map(|axis| {
        let direction = f64::from(ray.direction[axis]);
        if direction == 0.0 {
            return 100_000.0;
        }
        let current = quantized_floor(f64::from(position[axis]) / width, mode);
        let edge = f64::from(current + i32::from(direction > 0.0)) * width;
        let distance = mode.add(edge, -f64::from(ray.start[axis]));
        mode.store_f32(divide(mode, distance, direction))
    });
    let distance = mode.add(f64::from(crossings[0].min(crossings[1])), f64::from(0.001_f32));
    let x = multiply(mode, distance, f64::from(ray.direction[0]));
    let y = mode.store_f32(multiply(mode, distance, f64::from(ray.direction[1])));
    let next = [x, f64::from(y)];
    let next = [0, 1].map(|axis| mode.store_f32(mode.add(f64::from(ray.start[axis]), next[axis])));
    (mode.store_f32(distance), next)
}

fn blocked(class: u8) -> bool {
    matches!(class, 3 | 4 | 0x27..=0x29)
}

/// Cost along an ordinary FootCivilian segment with a valid local atlas.
/// The caller supplies per-transition class and territory admission.
/// Connectors and layered movement cost fields require a separate provider.
pub fn test_segment(
    mode: Precision53,
    ray: Ray,
    costs: FootCost,
    mut locate: impl FnMut([f32; 2]) -> Option<Leaf>,
    mut admits: impl FnMut(Leaf, Leaf) -> Option<bool>,
    mut observe: impl FnMut(Crossing),
) -> Option<Segment> {
    let mut leaf = locate(ray.start)?;
    let mut position = ray.start;
    let mut distance = 0.0;
    let mut cost = 0.0;
    let mut entered = !blocked(leaf.class);
    while distance < ray.length {
        let (next_distance, next_position) = advance(mode, ray, position, leaf.level);
        let next_leaf = locate(next_position)?;
        if next_leaf == leaf || next_distance <= distance {
            return Some(Segment { admitted: false, cost });
        }
        let edge = next_distance.min(ray.length);
        let length = mode.store_f32(f64::from(edge) - f64::from(distance));
        let deficit = f64::from(costs.clearance) - f64::from(leaf.density);
        let factor = if leaf.class < 0x1f && deficit > 0.0 {
            mode.store_f32(mode.add(1.0, multiply(mode, f64::from(costs.density_weight), deficit)))
        } else {
            1.0
        };
        let segment_cost = f64::from(factor) * f64::from(length);
        observe(Crossing {position, next_position, distance: next_distance, previous_leaf: leaf,
                          next_leaf, segment_length: length, segment_cost});
        cost = mode.store_f32(mode.add(f64::from(cost), segment_cost));
        if edge == ray.length {
            break;
        }
        if admits(leaf, next_leaf)? {
            entered = true;
        } else if entered {
            return Some(Segment { admitted: false, cost });
        }
        distance = edge;
        position = next_position;
        leaf = next_leaf;
    }
    Some(Segment { admitted: true, cost })
}
