const CLOSE_RADIUS: f64 = f32::from_bits(0x3d4c_cccd) as f64;
const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
const TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;
pub const CITIZEN_HALF_AIM: f64 = f32::from_bits(0x3e85_1eb8) as f64;

/// Citizen heading gate after native movement, reach, and line of shot admission.
/// XY here is the original horizontal plane. The caller supplies the actor's
/// simulation yaw in radians in the same frame, with zero pointing along +X.
pub fn citizen_facing_nearest(actor: [f32; 2], target: [f32; 2], yaw: f32) -> bool {
    let dx = (f64::from(target[0]) - f64::from(actor[0])) as f32;
    let dy_unspilled = f64::from(target[1]) - f64::from(actor[1]);
    let dx = f64::from(dx);
    if dx * dx + dy_unspilled * dy_unspilled <= CLOSE_RADIUS * CLOSE_RADIUS {
        return true;
    }
    let yaw = f64::from(yaw);
    if !(-TAU..=TAU).contains(&yaw) {
        return false;
    }
    let dy = f64::from(dy_unspilled as f32);
    let mut bearing = dy.atan2(dx);
    if bearing < 0.0 {
        bearing += TAU;
    }
    let mut error = bearing - yaw;
    while error > PI {
        error -= TAU;
    }
    while error < -PI {
        error += TAU;
    }
    -CITIZEN_HALF_AIM < error && error < CITIZEN_HALF_AIM
}
