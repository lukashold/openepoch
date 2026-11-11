//! Ordinary equal-range circular flee query. The caller owns map and collision authority.

const SOURCE_PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
const SOURCE_TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const SOURCE_MIN_SPACING: f64 = f32::from_bits(0x3e99_999a) as f64;
const FIXED_CONVERSION_BIAS: f64 = 103_079_215_104.0;

/// Produces the original 00873280 point stream for 00873A00's ordinary circular path.
/// The callback may replace Z with authoritative terrain height and reject the candidate.
pub fn find_circle_destination(
    origin: [f32; 3],
    direction: [f32; 2],
    raw_radius: f32,
    extent: f32,
    mut admit: impl FnMut([f32; 3]) -> Option<[f32; 3]>,
) -> Option<[f32; 3]> {
    let extent = f64::from(extent);
    let radius = (f64::from(raw_radius) + extent) as f32;
    let scalar = f64::from(radius) + extent;
    let spacing = (extent * 2.5).clamp(SOURCE_MIN_SPACING, 1.0) as f32;

    // Original positive integer conversion stores a double with this bias, then shifts its bits.
    let count_word =
        (scalar / f64::from(spacing) * SOURCE_PI + 0.5 + FIXED_CONVERSION_BIAS).to_bits();
    let mut half_turn_steps = ((count_word >> 16) as u32 as i32 - 1).max(1);
    if scalar as f32 > 0.0 {
        half_turn_steps = half_turn_steps.max(4);
    }
    let step = SOURCE_PI / f64::from(half_turn_steps);
    let (sin_step, cos_step) = step.sin_cos();
    let (sin_step, cos_step) = (f64::from(sin_step as f32), f64::from(cos_step as f32));

    let [dx, dy] = direction.map(f64::from);
    let angle = dy.atan2(dx);
    let angle = if angle < 0.0 {
        angle + SOURCE_TAU
    } else {
        angle
    };
    let (sin, cos) = f64::from(angle as f32).sin_cos();
    let mut positive = [cos as f32, sin as f32];
    let mut negative = positive;
    let origin_x = f64::from(origin[0]);
    let origin_y = f64::from(origin[1]);

    for index in 0..=half_turn_steps {
        let directions = if index == 0 || index == half_turn_steps {
            1
        } else {
            2
        };
        for ray in [positive, negative].into_iter().take(directions) {
            let [x, y] = ray.map(f64::from);
            let candidate = [
                (origin_x + x * scalar) as f32,
                (origin_y + f64::from((y * scalar) as f32)) as f32,
                origin[2],
            ];
            if let Some(accepted) = admit(candidate) {
                return Some(accepted);
            }
        }
        let [px, py] = positive.map(f64::from);
        positive = [
            (cos_step * px - sin_step * py) as f32,
            (sin_step * px + cos_step * py) as f32,
        ];
        let [nx, ny] = negative.map(f64::from);
        negative = [
            (cos_step * nx + sin_step * ny) as f32,
            (cos_step * ny - sin_step * nx) as f32,
        ];
    }
    None
}
