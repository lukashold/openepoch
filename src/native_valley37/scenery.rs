//! Stationary authored scenery footprints used by the native half-cell atlas.

#[derive(Clone, Copy, Debug)]
pub(super) struct Footprint {
    center: [f64; 2],
    basis: [f64; 2],
    half: [f64; 2],
}

impl Footprint {
    pub(super) fn from_archive(name: &[u8], center: [f32; 2], yaw: f32) -> Option<Self> {
        let size = match name {
            b"DestroyedMill_12" => 0x4007_c3b6,
            b"DestroyedTemple_10" => 0x4062_4630,
            b"DESERTROCKS_02" | b"DESERTROCKS_03" | b"DESERTROCKS_04" | b"MIDDLE_SIGNPOST" => 0x3f35_04f3,
            b"ResourceDummy" => 0x3ed9_3924,
            _ => return None,
        };
        Some(Self::rectangle(center, yaw, [f32::from_bits(size); 2]))
    }

    pub(super) fn rectangle(center: [f32; 2], yaw: f32, size: [f32; 2]) -> Self {
        let half = size.map(|extent| (f64::from(extent) * 0.5 - f64::from(0.01_f32)) as f32);
        let (sine, cosine) = f64::from(yaw).sin_cos();
        Self {
            center: center.map(f64::from),
            basis: [f64::from(cosine as f32), f64::from(sine as f32)],
            half: half.map(f64::from),
        }
    }

    pub(super) fn overlaps_half_cell(self, cell: [i32; 2]) -> bool {
        let [dx, dy] = std::array::from_fn(|i| f64::from(cell[i]) * 0.5 + 0.25 - self.center[i]);
        let [cosine, sine] = self.basis;
        let [hx, hy] = self.half;
        let (c, s) = (cosine.abs(), sine.abs());
        let cell_half = 0.25;
        dx.abs() <= hx * c + hy * s + cell_half
            && dy.abs() <= hx * s + hy * c + cell_half
            && (dx * cosine + dy * sine).abs() <= hx + cell_half * (c + s)
            && (dy * cosine - dx * sine).abs() <= hy + cell_half * (c + s)
    }
}
