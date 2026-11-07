/// 00715930 half-axis rotation and four corner stores, using the actual f32
/// cos/sin stores supplied by00A20800. Supplying a basis avoids asserting that
/// Rust/libm reproduces the original x87 FSINCOS in every FP context.
pub fn quad_from_basis(
    center: [f32; 2],
    size: [f32; 2],
    cos_sin: [f32; 2],
) -> Option<[[f32; 2]; 4]> {
    if center
        .iter()
        .chain(size.iter())
        .chain(cos_sin.iter())
        .any(|v| !v.is_finite())
        || size.iter().any(|&v| v < 0.0)
    {
        return None;
    }
    let half = [size[0] * 0.5, size[1] * 0.5];
    let [c, s] = cos_sin;
    let a = [
        (c as f64 * half[0] as f64) as f32,
        (s as f64 * half[0] as f64) as f32,
    ];
    let b = [
        (-s as f64 * half[1] as f64) as f32,
        (c as f64 * half[1] as f64) as f32,
    ];
    let out = [[1.0, 1.0], [-1.0, 1.0], [-1.0, -1.0], [1.0, -1.0]].map(|[sa, sb]| {
        [
            (center[0] as f64 + sa * a[0] as f64 + sb * b[0] as f64) as f32,
            (center[1] as f64 + sa * a[1] as f64 + sb * b[1] as f64) as f32,
        ]
    });
    out.iter().flatten().all(|v| v.is_finite()).then_some(out)
}
/// 00A20840 stores each difference to f32, then tests strict per-axis bounds.
pub fn near_vertex(point: [f32; 2], vertex: [f32; 2]) -> bool {
    let threshold = f32::from_bits(0x3d4ccccd);
    (point[0] - vertex[0]).abs() < threshold && (point[1] - vertex[1]).abs() < threshold
}
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    pub corners: Vec<[i16; 2]>,
    pub contains_outside_corner: bool,
}
/// Integer-corner loop of007155C0 after its oriented quad is supplied.
/// Bounds use world dimensions, not corner-buffer dimensions; the endpoint
/// at exactly worldWidth/worldHeight is outside. Row-major order is retained.
pub fn raster_quad(
    quad: [[f32; 2]; 4],
    world_width: u32,
    world_height: u32,
    skip_vertices: bool,
) -> Option<Raster> {
    if world_width == 0
        || world_height == 0
        || world_width > 32767
        || world_height > 32767
        || quad.iter().flatten().any(|v| !v.is_finite())
    {
        return None;
    }
    let min_x = quad.iter().map(|v| v[0]).fold(f32::MAX, f32::min);
    let min_y = quad.iter().map(|v| v[1]).fold(f32::MAX, f32::min);
    let max_x = quad.iter().map(|v| v[0]).fold(-f32::MAX, f32::max);
    let max_y = quad.iter().map(|v| v[1]).fold(-f32::MAX, f32::max);
    let bounds = [
        min_x as f64,
        min_y as f64,
        max_x as f64 + 1.0,
        max_y as f64 + 1.0,
    ];
    if bounds
        .iter()
        .any(|&v| v < i32::MIN as f64 || v > i32::MAX as f64)
    {
        return None;
    }
    let [x0, y0, x1, y1] = bounds.map(|v| v.trunc() as i32);
    let count = (x1 as i64 - x0 as i64 + 1).checked_mul(y1 as i64 - y0 as i64 + 1)?;
    if !(0..=4_000_000).contains(&count) {
        return None;
    }
    let mut out = Raster {
        corners: Vec::new(),
        contains_outside_corner: false,
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = [x as f32, y as f32];
            if skip_vertices && quad.iter().any(|&v| near_vertex(p, v)) {
                continue;
            }
            let mut inside = true;
            for edge in 0..4 {
                let a = quad[edge];
                let b = quad[(edge + 1) % 4];
                // First edge is kept on the source x87 stack; the other three use
                // explicit Vec2 subtraction stores before00716DC0 cross product.
                let (px, py, ex, ey) = if edge == 0 {
                    (
                        p[0] as f64 - a[0] as f64,
                        p[1] as f64 - a[1] as f64,
                        b[0] as f64 - a[0] as f64,
                        b[1] as f64 - a[1] as f64,
                    )
                } else {
                    (
                        (p[0] - a[0]) as f64,
                        (p[1] - a[1]) as f64,
                        (b[0] - a[0]) as f64,
                        (b[1] - a[1]) as f64,
                    )
                };
                if px * ey - py * ex > 0.0 {
                    inside = false;
                    break;
                }
            }
            if inside {
                if x >= 0 && y >= 0 && (x as u32) < world_width && (y as u32) < world_height {
                    out.corners.push([x as i16, y as i16]);
                } else {
                    out.contains_outside_corner = true;
                }
            }
        }
    }
    Some(out)
}
