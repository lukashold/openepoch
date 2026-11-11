//! Full and incremental transforms for the original half-cell movement raster.

use super::atlas_reader::{quantized_floor, Precision53};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HalfCellRect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateRegion {
    pub min: [usize; 2],
    pub max: [usize; 2],
}

fn seed_byte(seed: Option<f32>, maximum: u8, precision: Precision53) -> Option<u8> {
    let Some(seed) = seed else { return Some(maximum.wrapping_mul(5)); };
    if !seed.is_finite() { return None; }
    let scaled = precision.mul(f64::from(seed), 5.0);
    let biased = if scaled > 0.0 { precision.add(scaled, 7.0) } else { scaled };
    Some(quantized_floor(precision.add(biased, 0.5), precision) as u8)
}

fn padded_shape([width, height]: [usize; 2]) -> Option<(usize, usize)> {
    let stride = width.checked_add(2)?;
    let length = stride.checked_mul(height.checked_add(2)?)?;
    (length <= isize::MAX as usize).then_some((stride, length))
}

fn sweep(grid: &mut [u8], stride: usize, region: UpdateRegion) {
    let [x0, y0] = region.min;
    let [x1, y1] = region.max;
    for reverse in [false, true] {
        let direction = if reverse { -1 } else { 1 };
        let row = stride as isize * direction;
        for y in y0..y1 {
            for x in x0..x1 {
                let (x, y) = if reverse { (x0 + x1 - 1 - x, y0 + y1 - 1 - y) } else { (x, y) };
                let cell = ((y + 1) * stride + x + 1) as isize;
                let at = |offset| (cell + offset) as usize;
                let straight = grid[at(-direction)].min(grid[at(-row)]).wrapping_add(5);
                let diagonal = grid[at(-row - direction)].min(grid[at(-row + direction)]).wrapping_add(7);
                grid[at(0)] = grid[at(0)].min(straight).min(diagonal);
            }
        }
    }
}

pub fn raster(
    [width, height]: [usize; 2],
    maximum: u8,
    border: u8,
    seeds: &[Option<f32>],
    precision: Precision53,
) -> Option<Vec<u8>> {
    if seeds.len() != width.checked_mul(height)? { return None; }
    let (stride, length) = padded_shape([width, height])?;
    let mut grid = vec![border; length];
    for y in 0..height {
        for x in 0..width {
            grid[(y + 1) * stride + x + 1] = seed_byte(seeds[y * width + x], maximum, precision)?;
        }
    }
    sweep(&mut grid, stride, UpdateRegion { min: [0, 0], max: [width, height] });
    Some(grid)
}

pub(crate) fn update_region([width, height]: [usize; 2], dirty: HalfCellRect, maximum: u8) -> Option<UpdateRegion> {
    let dimensions = [i32::try_from(width).ok()?, i32::try_from(height).ok()?];
    let radius = i32::from(maximum);
    let mut region = UpdateRegion { min: [0, 0], max: [0, 0] };
    for (axis, dimension) in dimensions.into_iter().enumerate() {
        if dimension == 0 || dirty.min[axis] > dirty.max[axis] { return None; }
        let min = dirty.min[axis].checked_sub(radius)?.max(0);
        // The original stores dimension−1 as an exclusive upper limit.
        let max = dirty.max[axis].checked_add(radius)?.min(dimension - 1);
        if min > max { return None; }
        region.min[axis] = min as usize;
        region.max[axis] = max as usize;
    }
    Some(region)
}

/// Update one dirty half-cell rectangle, preserving every exterior and padding byte.
/// The seed provider supplies finite values independently of this raster. Validation
/// happens before the stored field changes.
pub fn update(
    grid: &mut [u8],
    dimensions: [usize; 2],
    dirty: HalfCellRect,
    maximum: u8,
    precision: Precision53,
    mut seed: impl FnMut([usize; 2]) -> Option<f32>,
) -> Option<UpdateRegion> {
    let (stride, length) = padded_shape(dimensions)?;
    if grid.len() != length { return None; }
    let region = update_region(dimensions, dirty, maximum)?;
    let [x0, y0] = region.min;
    let [x1, y1] = region.max;
    let width = x1 - x0;
    if width == 0 || y0 == y1 { return Some(region); }
    let mut seeds = Vec::with_capacity(width * (y1 - y0));
    for y in y0..y1 {
        for x in x0..x1 {
            seeds.push(seed_byte(seed([x, y]), maximum, precision)?);
        }
    }
    for (y, row) in (y0..y1).zip(seeds.chunks_exact(width)) {
        let start = (y + 1) * stride + x0 + 1;
        grid[start..start + width].copy_from_slice(row);
    }
    sweep(grid, stride, region);
    Some(region)
}
