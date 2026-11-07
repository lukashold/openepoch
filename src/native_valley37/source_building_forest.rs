//! Final Building-branch forest predicate 00861D80, for an actual Box1 context.
use super::source_forest_grid::SourceForestGrid;
use super::world8_placement_math::{quad_from_basis, raster_quad, Raster};

pub fn building_forest_raster(
    position: [f32; 2],
    half: [f32; 2],
    negative_yaw_basis: [f32; 2],
    width: u32,
    height: u32,
) -> Option<Raster> {
    let [c, s] = negative_yaw_basis;
    // The cached horizontal Box axes are [c,-s], [s,c]. The source stores
    // all four axis products separately, then stores their vector sum.
    let x = half[0] * c + half[1] * s;
    let y = half[0] * -s + half[1] * c;
    // 00A20800 then rotates this vector by the negative context yaw.
    let mut size = [
        (x as f64 * c as f64 - y as f64 * s as f64) as f32,
        (x as f64 * s as f64 + y as f64 * c as f64) as f32,
    ];
    for axis in &mut size {
        *axis += f32::from_bits(0x3a83_126f);
    } // B5505C
    let quad = quad_from_basis(position, size, [c, -s])?;
    raster_quad(quad, width, height, false)
}

pub fn blocked_by_forest(grid: &SourceForestGrid, raster: &Raster) -> bool {
    // At each in-bounds source integer raster point, 765150 verifies the
    // first grid's dimensions. The observed test requires plane 0 then 1.
    raster
        .corners
        .iter()
        .any(|&[x, y]| grid.blocks_building_point(x as i32, y as i32))
}
