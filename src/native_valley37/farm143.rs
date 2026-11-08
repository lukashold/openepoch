//! Static Mountain rejection in the original Farm placement query for Valley.

use super::{geometry::Geometry, VALLEY_SHA};
use crate::{defs::{DefId, FARM}, game::Game, hotspot_start66::{quad_from_basis, raster_quad}};

const MOUNTAIN: u8 = 3;
// 0053B03B scales Farm's 1.5 body size when the DDF omits a placement size.
const FARM_FOOTPRINT: f32 = f32::from_bits(0x4007_c3b6);
const PADDED_FOOTPRINT: f32 = (FARM_FOOTPRINT + f32::from_bits(0x3fb5_04f3)) + f32::from_bits(0x3a83_126f);

fn mountain_sample(world: &Geometry, [x, y]: [i16; 2]) -> bool {
    let stride = world.width + 1;
    let at = y as usize * stride + x as usize;
    // The pathing query chooses the positive half-cell at each integer corner.
    // Move type (1,1) preserves Mountain from either orthogonal neighbor.
    [at, at + 1, at + stride].iter().any(|&i| {
        world.move_palette[world.materials[i] as usize] == MOUNTAIN
    })
}

impl Game {
    pub(crate) fn valley_rejects_building_site(&self, def: DefId, bounds: (i32, i32, i32, i32), turn: u32) -> bool {
        if self.valley_house_reservation_overlaps(bounds) { return true; }
        if def != FARM { return false; }
        let Some(world) = self.map.source_world.as_ref().filter(|world| world.source_sha == VALLEY_SHA)
        else { return false; };
        let (tx, ty, width, height) = bounds;
        let center = [tx as f32 + width as f32 * 0.5, ty as f32 + height as f32 * 0.5];
        let (sin, cos) = Self::angle_for_turn(turn).sin_cos();
        // The source raster uses the native footprint at the current anchor.
        quad_from_basis(center, [PADDED_FOOTPRINT; 2], [cos, sin])
            .and_then(|quad| raster_quad(quad, world.width as u32, world.height as u32, true))
            .is_none_or(|raster| raster.corners.into_iter().any(|p| mountain_sample(world, p)))
    }
}
