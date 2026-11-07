//! CityCenter's nonmoving Building1 branch of 00860A10 and 00862220.
//! This slice ends before 0071E200, the unit collision and final policy calls.

use crate::hotspot_start66::{quad_from_basis, raster_quad, Raster};

const SIZE_PADDING: f32 = f32::from_bits(0x3fb5_04f3);
const RASTER_PADDING: f32 = f32::from_bits(0x3a83_126f);
const SMALL: f32 = f32::from_bits(0x3586_37bd);

#[derive(Clone, Copy)]
pub struct BuildingTerrainType {
    pub body_size: [f32; 2],
    pub footprint_size: [f32; 2],
    pub under_size: [f32; 2],
    pub under_offset: [f32; 2],
}

#[derive(Debug, PartialEq)]
pub struct TerrainRejection {
    pub code: u32,
    pub first_cell: Option<[i16; 2]>,
}

#[derive(Debug, PartialEq)]
pub struct BuildingTerrainProbe {
    pub footprint: Raster,
    pub foundation: Raster,
    pub average: f32,
    pub quantized: f64,
    pub min: f32,
    pub max: f32,
    pub sampled_heights: usize,
    pub rejection: Option<TerrainRejection>,
}

pub struct SourceTerrain<'a> {
    width: u32,
    height: u32,
    heights: &'a [f32],
    materials: &'a [u16],
    move_types: &'a [u8],
}

impl<'a> SourceTerrain<'a> {
    pub fn new(
        width: u32, height: u32, heights: &'a [f32], materials: &'a [u16], move_types: &'a [u8],
    ) -> Option<Self> {
        let count = (width as usize + 1).checked_mul(height as usize + 1)?;
        if !(1..=256).contains(&width) || !(1..=256).contains(&height) ||
            count != heights.len() || count != materials.len() ||
            heights.iter().any(|h| !h.is_finite()) ||
            materials.iter().any(|&id| id as usize >= move_types.len()) ||
            move_types.iter().any(|&kind| kind > 7) {
            return None;
        }
        Some(Self { width, height, heights, materials, move_types })
    }

    fn index(&self, [x, y]: [i16; 2]) -> usize {
        // 007155C0 only appends in-world points; the source corner row stride
        // has one extra column beyond the world width.
        y as usize * (self.width as usize + 1) + x as usize
    }

    fn height(&self, cell: [i16; 2]) -> f32 { self.heights[self.index(cell)] }

    fn move_type(&self, cell: [i16; 2]) -> u8 {
        // The integer point in 008624E6 reaches 007165A0(...,0,0). Its
        // bias-floor(x+0.5) and bias-floor(y+0.5) retain the same indices.
        self.move_types[self.materials[self.index(cell)] as usize]
    }

    fn raster(&self, center: [f32; 2], size: [f32; 2], yaw_basis: [f32; 2]) -> Option<Raster> {
        if size.iter().any(|&v| !v.is_finite() || v < 0.0) { return None; }
        // 715A80 stores size+sqrt(2); 7155C0 then stores the addition of
        // B5505C separately before 715930 rotates the two half-axes.
        let padded = size.map(|value| (value + SIZE_PADDING) + RASTER_PADDING);
        raster_quad(quad_from_basis(center, padded, yaw_basis)?, self.width, self.height, true)
    }

    fn height_stats(&self, corners: &[[i16; 2]]) -> (f32, f32, f32) {
        let mut sum = 0.0f64;
        let mut min = f32::from_bits(0x461c_3ff6);
        let mut max = f32::from_bits(0xc61c_3ff6);
        for &cell in corners {
            let value = self.height(cell);
            // 715B2A FADDP retains the accumulator. Only the average is
            // stored to f32 at 715B73, unlike the historic draft helper.
            sum += f64::from(value);
            min = min.min(value);
            max = max.max(value);
        }
        let average = if corners.is_empty() { 0.0 } else { (sum / corners.len() as f64) as f32 };
        (average, min, max)
    }

    /// 00715C50 mode zero plans one 0070F1A0 call per retained corner.
    /// The caller supplies the body dimensions. The setter's derived effects
    /// and the later 00864890 actor creation are separate from this projection.
    pub fn flatten_writes(
        &self, position: [f32; 2], body_size: [f32; 2], yaw_basis: [f32; 2], step: f32,
    ) -> Option<(Raster, f32)> {
        let raster = self.raster(position, body_size, yaw_basis)?;
        let (mean, _, _) = self.height_stats(&raster.corners);
        // 715D6B stores the quantized result before passing the scalar to
        // every setter. Acceptance's analogous return stays wide.
        let height = quantized_height(mean, step)? as f32;
        Some((raster, height))
    }

    pub fn probe(
        &self, position: [f32; 2], ty: BuildingTerrainType, negative_yaw_basis: [f32; 2], step: f32,
    ) -> Option<BuildingTerrainProbe> {
        if ty.under_size.iter().any(|&v| !v.is_finite() || v < 0.0) ||
            ty.under_offset.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let [c, s] = negative_yaw_basis;
        let yaw_basis = [c, -s];
        let footprint = self.raster(position, ty.footprint_size, yaw_basis)?;
        let use_under = square_length(ty.under_size) >= SMALL;
        let foundation_size = if use_under { ty.under_size } else { ty.body_size };
        let center = if use_under {
            let [x, y] = ty.under_offset.map(|v| -f64::from(v));
            // 871F30 mode2 computes the offset with 8E6970's two f32 stores.
            // 860A83/860A89 separately add the anchor when updating BoxBound.
            let ox = (y * f64::from(s) + x * f64::from(c)) as f32;
            let oy = (x * -f64::from(s) + y * f64::from(c)) as f32;
            [position[0] + ox, position[1] + oy]
        } else { position };
        let foundation = self.raster(center, foundation_size, yaw_basis)?;
        let sampled_heights = foundation.corners.len();
        let (average, min, max) = self.height_stats(&foundation.corners);
        let quantized = quantized_height(average, step)?;
        let rejection = if footprint.contains_outside_corner {
            Some(TerrainRejection { code: 2, first_cell: None })
        } else if f64::from(max) - quantized > 0.5 || quantized - f64::from(min) > 0.5 {
            // Original C6A8B0=1.0 multiplied by B4165C=0.5, strict compare.
            Some(TerrainRejection { code: 4, first_cell: None })
        } else if let Some(cell) = footprint.corners.iter().copied().find(|&cell| self.height(cell) < 0.0) {
            Some(TerrainRejection { code: 2, first_cell: Some(cell) })
        } else {
            footprint.corners.iter().copied().find(|&cell| self.move_type(cell) != 0)
                .map(|cell| TerrainRejection { code: 2, first_cell: Some(cell) })
        };
        Some(BuildingTerrainProbe { footprint, foundation, average, quantized, min, max, sampled_heights, rejection })
    }
}

fn square_length([x, y]: [f32; 2]) -> f32 {
    (f64::from(x) * f64::from(x) + f64::from(y) * f64::from(y)) as f32
}

fn quantized_height(average: f32, step: f32) -> Option<f64> {
    if !average.is_finite() || !step.is_finite() { return None; }
    let average = f64::from(average);
    if step.abs() < SMALL { return Some(average); }
    let step = f64::from(step);
    let quotient = (average / step).trunc();
    if !(i32::MIN as f64..=i32::MAX as f64).contains(&quotient) { return None; }
    let mut index = quotient as i32;
    let residual = average - quotient * step;
    if residual > 0.0 && residual >= step * 0.5 {
        index = index.checked_add(1)?;
    } else if residual < 0.0 && residual <= -step * 0.5 {
        index = index.checked_sub(1)?;
    }
    // 71618A/7161BE return x87 ST0; 862431 and 862443 compare that value
    // without a fresh f32 store.
    Some(f64::from(index) * step)
}

