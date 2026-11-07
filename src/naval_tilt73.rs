//! NavalMove 007F3970 and 008E6480/64C0/6500/67E0.
use crate::naval_heave72::{add, store};

const HALF_PI: f64 = f32::from_bits(0x3fc9_0fdb) as f64;
const TURN: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const X_GAIN: f64 = 0.2f32 as f64;
const Y_GAIN: f64 = f32::from_bits(0x3f68_ba2e) as f64;

type Matrix = [f32; 9];

fn sin_cos(angle: f32) -> (f32, f32) {
    let (sin, cos) = f64::from(angle).sin_cos();
    (store(sin), store(cos))
}

fn product(left: Matrix, right: Matrix) -> Matrix {
    // The x87 matrix product stores each cell separately. The first two
    // terms are not always added in column order.
    const ORDER: [[usize; 3]; 9] = [
        [1, 0, 2], [2, 0, 1], [2, 0, 1],
        [2, 1, 0], [2, 1, 0], [2, 1, 0],
        [2, 1, 0], [0, 2, 1], [0, 2, 1],
    ];
    std::array::from_fn(|cell| {
        let (row, col) = (cell / 3, cell % 3);
        let term = |k| f64::from(left[row * 3 + k]) * f64::from(right[k * 3 + col]);
        let [a, b, c] = ORDER[cell];
        store(add(add(term(a), term(b)), term(c)))
    })
}

/// Column-major basis for the converter's X/Z/-Y model axes.
pub(crate) fn naval_surface_basis(id: u32, height: f32, render_yaw: f32) -> Option<Matrix> {
    if !(f64::from(render_yaw).abs() <= TURN) { return None; }
    // Browser headings reflect native Y. 00841100 normalizes native heading
    // before 007F3970 builds the signed yaw input.
    let heading = -f64::from(render_yaw);
    let heading = if heading < 0.0 { heading + TURN }
        else if heading >= TURN { heading - TURN } else { heading };
    let yaw = store(-(f64::from(store(heading)) + HALF_PI));
    let height = f64::from(height);
    let x = store(if id & 1 == 0 { height } else { -height } * X_GAIN);
    let y = store(if id & 2 == 0 { height } else { -height } * Y_GAIN);
    let (sx, cx) = sin_cos(x);
    let (sy, cy) = sin_cos(y);
    let (sz, cz) = sin_cos(yaw);
    let x = [1.0, 0.0, 0.0, 0.0, cx, sx, 0.0, -sx, cx];
    let y = [cy, 0.0, -sy, 0.0, 1.0, 0.0, sy, 0.0, cy];
    let z = [cz, sz, 0.0, -sz, cz, 0.0, 0.0, 0.0, 1.0];
    let m = product(product(z, y), x);
    Some([m[0], m[6], -m[3], m[2], m[8], -m[5], -m[1], -m[7], m[4]])
}
