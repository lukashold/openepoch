//! UnitForest cell planes from 00819310, 00821780 and 0081E0C0.
//! Only the authored map registration path is projected here.

#[derive(Clone, Debug)]
pub struct SourceForestGrid {
    width: usize,
    height: usize,
    stride: usize,
    planes: [Vec<u32>; 2],
}

impl SourceForestGrid {
    pub fn new(width: usize, height: usize) -> Option<Self> {
        if !(1..=5000).contains(&width) || !(1..=5000).contains(&height) {
            return None;
        }
        let stride = width / 32 + 1;
        let planes = [vec![0; stride * height], vec![0; stride * height]];
        Some(Self {
            width,
            height,
            stride,
            planes,
        })
    }

    /// Extents are the effective Type+68/+6C full footprint sizes. Forest
    /// record coordinates are unsigned u16 at source scale 1/128.
    pub fn register(&mut self, xy: [u16; 2], size: [f32; 2], flags: u16, state: u8) -> Option<()> {
        if state > 3
            || size
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=8192.0).contains(v))
        {
            return None;
        }
        if state == 1 || state == 2 {
            return Some(()); // dropped by 0081C570 before registration
        }
        let xy = xy.map(|v| v as f64 / 128.0);
        let half = size.map(|v| (v * 0.5) as f64);
        let [x0, y0] = std::array::from_fn(|a| floor_source((xy[a] - half[a]) as f32));
        let [x1, y1] = std::array::from_fn(|a| floor_source((xy[a] + half[a]) as f32));
        let xs = x0.max(0)..=x1.min(self.width as i32 - 1);
        let ys = y0.max(0)..=y1.min(self.height as i32 - 1);
        for (plane, flag) in self.planes.iter_mut().zip([0x20, 0x100]) {
            if flags & flag == 0 {
                continue;
            }
            for x in xs.clone() {
                let x = x as usize;
                let mask = 0x8000_0000u32 >> (x % 32);
                for y in ys.clone() {
                    plane[y as usize * self.stride + x / 32] |= mask;
                }
            }
        }
        Some(())
    }

    pub fn plane(&self, index: usize) -> Option<&[u32]> {
        Some(self.planes.get(index)?.as_slice())
    }

    pub fn dimensions(&self) -> [u32; 2] {
        [self.width as u32, self.height as u32]
    }

    /// 00778020 reads the plane registered by flag 0x100.
    pub fn blocks_source_cell(&self, x: i32, y: i32) -> bool {
        self.cell(1, x, y)
    }

    /// 00861D80 requires both flags at the same raster point.
    pub fn blocks_building_point(&self, x: i32, y: i32) -> bool {
        self.cell(0, x, y) && self.cell(1, x, y)
    }

    pub fn point_flags(&self, x: i32, y: i32) -> u32 {
        u32::from(self.cell(0, x, y)) | (u32::from(self.blocks_source_cell(x, y)) << 1)
    }

    fn cell(&self, plane: usize, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return false;
        }
        let (x, y) = (x as usize, y as usize);
        self.planes[plane][y * self.stride + x / 32] & (0x8000_0000 >> (x % 32)) != 0
    }
}

pub fn floor_source(value: f32) -> i32 {
    // 81E0C0 first stores the translated bounds in F32, then uses the
    // F64 bias and signed shift. Values just below an integer can round up.
    (((value as f64 + 103_079_215_104.0).to_bits() as i64) >> 16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_msb_clip_and_exact_width_padding() {
        let mut grid = SourceForestGrid::new(32, 3).unwrap();
        grid.register([31 * 128 + 64, 0], [1.0, 1.0], 0x20, 0)
            .unwrap();
        grid.register([0, 128], [1.0, 2.0], 0x100, 0).unwrap();
        assert_eq!(grid.plane(0).unwrap(), [1, 0, 0, 0, 0, 0]);
        assert_eq!(
            grid.plane(1).unwrap(),
            [0x8000_0000, 0, 0x8000_0000, 0, 0x8000_0000, 0]
        );
        assert!(grid.blocks_source_cell(0, 2));
        assert!(!grid.blocks_building_point(0, 2));
        assert!(!grid.blocks_source_cell(31, 0));
        assert!(!grid.blocks_source_cell(32, 0));
        assert!(!grid.blocks_source_cell(-1, 0));
    }

    #[test]
    fn serializer_drops_states_1_and_2_independently_of_flags() {
        let mut grid = SourceForestGrid::new(4, 4).unwrap();
        for state in [1, 2] {
            grid.register([128, 128], [2.0, 2.0], 0x120, state).unwrap();
        }
        assert_eq!(grid.plane(0).unwrap(), [0; 4]);
        assert_eq!(grid.plane(1).unwrap(), [0; 4]);
        assert_eq!(grid.register([128, 128], [f32::NAN, 2.0], 0x120, 0), None);
        grid.register([128, 128], [2.0, 2.0], 0x120, 3).unwrap();
        assert!(grid.blocks_building_point(0, 0));
        assert_eq!(
            grid.plane(0).unwrap(),
            [0xe000_0000, 0xe000_0000, 0xe000_0000, 0]
        );
        assert_eq!(grid.plane(1).unwrap(), grid.plane(0).unwrap());
    }

    #[test]
    fn biased_near_integer_conversion_is_not_floor() {
        let just_below = f32::from_bits(3.0f32.to_bits() - 1);
        assert_eq!(floor_source(just_below), 3);
        assert_eq!(just_below.floor(), 2.0);
        assert_eq!(floor_source(-0.125), -1);
    }
}
