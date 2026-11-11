//! Ordinary City construction arrival for a FootCitizen in the native XY plane.

const HALF_X: f64 = f32::from_bits(0x401e_6455) as f64;
const HALF_Y: f64 = f32::from_bits(0x4007_c3b6) as f64;
// 00870740 stores the ordinary .3 request plus the mover's .125 radius
// and .2 + .05 + .05 allowance. 008714F0 adds this to the selected edge.
const OUTER_MARGIN: f64 = f32::from_bits(0x3f39_9999) as f64;
const INNER_MARGIN: f64 = f32::from_bits(0x3c23_d70a) as f64;
const NEAR_FACING_BYPASS: f64 = f32::from_bits(0x3d4c_cccd) as f64;
const HALF_CONE: f64 = 0.25;
const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
const TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;

fn store_chop(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

fn add_chop(left: f64, right: f64) -> f64 {
    let nearest = left + right;
    let right_part = nearest - left;
    let residual = (left - (nearest - right_part)) + (right - right_part);
    if (nearest > 0.0 && residual < 0.0) || (nearest < 0.0 && residual > 0.0) {
        f64::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

fn difference(from: [f32; 2], to: [f32; 2]) -> [f64; 2] {
    [0, 1].map(|axis| add_chop(f64::from(to[axis]), -f64::from(from[axis])))
}

fn finite_pair(xy: [f32; 2]) -> bool {
    xy.into_iter().all(f32::is_finite)
}

/// Footprint stage of 007FE060 → 008714F0. The ordinary Build call uses
/// path mode zero, whose subsequent path gate admits the shape result.
pub(crate) fn in_work_band(worker: [f32; 2], site: [f32; 2], site_yaw: f32) -> bool {
    if !finite_pair(worker) || !finite_pair(site) || !site_yaw.is_finite() {
        return false;
    }
    let [dx, dy] = difference(site, worker).map(|delta| f64::from(store_chop(delta)));
    let (sin, cos) = (-f64::from(site_yaw)).sin_cos();
    let sin = f64::from(store_chop(sin));
    let cos = f64::from(store_chop(cos));
    let local_x = store_chop(add_chop(cos * dx, -sin * dy));
    let local_y = store_chop(add_chop(sin * dx, cos * dy));

    // 00873E90 selects the dominant local side using the body proportions.
    // Its ratio is stored to f32. Exact ties use X.
    let use_y = local_x == 0.0
        || f64::from(store_chop(f64::from(local_y) / f64::from(local_x))).abs() > HALF_Y / HALF_X;
    let (half, distance) = if use_y { (HALF_Y, local_y.abs()) } else { (HALF_X, local_x.abs()) };
    let inner = store_chop(add_chop(half, INNER_MARGIN));
    let outer = store_chop(add_chop(half, OUTER_MARGIN));
    (inner..=outer).contains(&distance)
}

fn bearing(dx: f32, dy: f32) -> f64 {
    let angle = f64::from(dy).atan2(f64::from(dx));
    // 00A1DD10 adds the stored source 2π word to a negative FPATAN result.
    if angle < 0.0 {
        add_chop(angle, TAU)
    } else {
        angle
    }
}

/// Desired facing toward the center, stored as CHOP float32 for the caller.
pub(crate) fn target_heading(worker: [f32; 2], site: [f32; 2]) -> f32 {
    if !finite_pair(worker) || !finite_pair(site) {
        return f32::NAN;
    }
    let [dx, dy] = difference(worker, site).map(store_chop);
    store_chop(bearing(dx, dy))
}

/// Facing stage after native work-band admission. The caller owns stopping
/// and the motion lifecycle. The native default cone is open at both edges.
pub(crate) fn heading_ready(worker: [f32; 2], worker_yaw: f32, site: [f32; 2]) -> bool {
    if !finite_pair(worker) || !finite_pair(site) || !worker_yaw.is_finite() {
        return false;
    }
    let [dx, dy] = difference(worker, site);
    let dx = store_chop(dx);
    // 007FE139 spills Y for FPATAN but keeps the wider value for distance.
    let wide_dx = f64::from(dx);
    if add_chop(wide_dx * wide_dx, dy * dy) <= NEAR_FACING_BYPASS * NEAR_FACING_BYPASS {
        return true;
    }
    let mut error = add_chop(bearing(dx, store_chop(dy)), -f64::from(worker_yaw));
    // Bound the normalization for a finite angle from a noncanonical save.
    if error.abs() > 2.0 * TAU {
        error %= TAU;
    }
    while error > PI {
        error = add_chop(error, -TAU);
    }
    while error <= -PI {
        error = add_chop(error, TAU);
    }
    error.abs() < HALF_CONE
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: [f32; 2] = [72.0, 24.0];
    const NEAR: [f32; 2] = [f32::from_bits(0x4295_69a4), f32::from_bits(0x41c3_076f)];

    #[test]
    fn stock_chop_arrival_matches_measured_and_boundary_cases() {
        // CHOP53 rows from the identical installed DX8/DX9 traces in Arrival272.
        let cases = [
            ("measured-near", [0x429569a4, 0x41c3076f], 0x00000000, 0x401ecfa4, true, false),
            ("measured-far", [0x429fc0b6, 0x41b30397], 0x00000000, 0x4034cf4b, false, false),
            ("near-faced", [0x429569a4, 0x41c3076f], 0x00000000, 0x4051f55a, true, true),
            ("near-faced-quarter", [0x429569a4, 0x41c3076f], 0x3fc90fdb, 0x4051f55a, true, true),
            ("inside-city", [0x42900ccd, 0x41c00000], 0x00000000, 0x00000000, false, false),
            ("x-below-inner", [0x4294f5c3, 0x41c00000], 0x00000000, 0x40490fdb, false, false),
            ("y-below-inner", [0x42900000, 0x41d10a3d], 0x00000000, 0xbfc90fdb, false, false),
            ("x-above-inner", [0x4294fae1, 0x41c00000], 0x00000000, 0x40490fdb, true, true),
            ("y-above-inner", [0x42900000, 0x41d10e56], 0x00000000, 0xbfc90fdb, true, true),
            ("x-below-outer", [0x42965c29, 0x41c00000], 0x00000000, 0x40490fdb, true, true),
            ("y-below-outer", [0x42900000, 0x41d6c28f], 0x00000000, 0xbfc90fdb, true, true),
            ("x-above-outer", [0x42966666, 0x41c00000], 0x00000000, 0x40490fdb, false, false),
            ("y-above-outer", [0x42900000, 0x41d6cccd], 0x00000000, 0xbfc90fdb, false, false),
            ("diagonal-0", [0x4295199a, 0x41d0cccd], 0x00000000, 0xc01cf886, true, true),
            ("diagonal-1", [0x42943333, 0x41d40000], 0x00000000, 0xc0113ec3, true, true),
            ("diagonal-2", [0x4294e666, 0x41d13333], 0x00000000, 0xc01af6e4, true, true),
            ("diagonal-3", [0x42950000, 0x41d73333], 0x00000000, 0xc0121079, false, false),
            ("yaw-within", [0x42960000, 0x41c00000], 0x00000000, 0x4058ff79, true, true),
            ("yaw-outside", [0x42960000, 0x41c00000], 0x00000000, 0x4059203d, true, false),
            ("yaw-wrapped", [0x42960000, 0x41c00000], 0x00000000, 0xc039203d, true, true),
            ("yaw-exact-positive", [0x428a0000, 0x41c00000], 0x00000000, 0x3e800000, true, false),
            ("yaw-exact-negative", [0x428a0000, 0x41c00000], 0x00000000, 0xbe800000, true, false),
            ("yaw-inner-positive", [0x428a0000, 0x41c00000], 0x00000000, 0x3e7fffff, true, true),
            ("yaw-inner-negative", [0x428a0000, 0x41c00000], 0x00000000, 0xbe7fffff, true, true),
        ];
        for (label, worker, site_yaw, worker_yaw, geometry, arrives) in cases {
            let worker = worker.map(f32::from_bits);
            let shape = in_work_band(worker, CITY, f32::from_bits(site_yaw));
            assert_eq!(shape, geometry, "{label} footprint");
            assert_eq!(
                shape && heading_ready(worker, f32::from_bits(worker_yaw), CITY),
                arrives,
                "{label} arrival"
            );
        }
    }

    #[test]
    fn target_heading_uses_positive_source_bearing_and_chop_store() {
        let yaw = target_heading(NEAR, CITY);
        assert_eq!(yaw.to_bits(), 0x4051_f548);
        assert!(heading_ready(NEAR, yaw, CITY));
        assert_eq!(target_heading([75.0, 24.0], CITY).to_bits(), 0x4049_0fda);
        assert!(heading_ready([72.025, 24.0], 0.0, CITY));
        assert!(!in_work_band([72.025, 24.0], CITY, 0.0));
    }
}
