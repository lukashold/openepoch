//! Fresh ordinary ground edge length and the native route heuristic.

use super::atlas_reader::Precision53;

#[derive(Clone, Copy)]
pub struct Profile {
    pub clearance: f32,
    pub density_weight: f32,
    pub heuristic_divisor: f32,
    pub cross_weight: Option<f32>,
}

fn difference(mode: Precision53, from: [f32; 2], to: [f32; 2]) -> [f64; 2] {
    [0, 1].map(|i| mode.add(f64::from(from[i]), -f64::from(to[i])))
}

/// The caller checks class, ownership and connector admission first.
pub fn ordinary_edge_cost(
    mode: Precision53,
    from: [f32; 2],
    to: [f32; 2],
    density: f32,
    profile: Profile,
) -> f32 {
    let [x, y] = difference(mode, from, to).map(|value| f64::from(mode.store_f32(value).abs()));
    let longer = x.max(y);
    let shorter = x.min(y);
    let distance = if longer == 0.0 {
        0.001_f32
    } else {
        let diagonal = shorter * shorter * f64::from_bits(0x3fda7ef9db22d0e5) / longer;
        mode.store_f32(mode.add(longer, diagonal))
    };
    let deficit = mode.add(f64::from(profile.clearance), -f64::from(density));
    let factor = if deficit > 0.0 {
        mode.store_f32(mode.add(1.0, deficit * f64::from(profile.density_weight)))
    } else {
        1.0
    };
    mode.store_f32(f64::from(distance) * f64::from(factor))
}

pub fn heuristic(
    mode: Precision53,
    origin: [f32; 2],
    from: [f32; 2],
    goal: [f32; 2],
    profile: Profile,
) -> f32 {
    let delta = difference(mode, from, goal);
    let [x, y] = delta.map(|value| f64::from(mode.store_f32(value).abs()));
    let manhattan = mode.add(x, y);
    let cost = if let Some(cross_weight) = profile.cross_weight {
        let [ox, oy] = difference(mode, origin, goal);
        let cross = mode.add(oy * delta[0], -(ox * delta[1]));
        let cross = f64::from(mode.store_f32(cross).abs()) * f64::from(cross_weight);
        mode.add(f64::from(mode.store_f32(manhattan)), cross)
    } else {
        manhattan
    };
    mode.store_f32(cost / f64::from(profile.heuristic_divisor))
}
