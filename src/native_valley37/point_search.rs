/// A zero-range point Move first probes the click, then walks back toward the
/// mover one world unit at a time. The caller supplies the original-style
/// destination fit and owner-aware reachability check.
pub fn recover_point(
    origin: [f32; 3],
    target: [f32; 3],
    strict: bool,
    toward_zero: bool,
    mut fits: impl FnMut([f32; 3]) -> bool,
) -> Option<[f32; 3]> {
    if fits(target) {
        return Some(target);
    }
    if strict {
        return None;
    }

    let store = |value| store_float(value, toward_zero);
    let delta = std::array::from_fn::<_, 3, _>(|i| store(origin[i] as f64 - target[i] as f64));
    let [x, y, z] = delta.map(f64::from);
    let length = (z * z + y * y + x * x).sqrt();
    let limit = store(length) as f64 - 1.0;
    if !length.is_finite() || limit <= 1.0 {
        return None;
    }
    let mut reciprocal = 1.0 / length;
    if toward_zero && reciprocal.mul_add(length, -1.0) > 0.0 {
        reciprocal = f64::from_bits(reciprocal.to_bits() - 1);
    }
    let direction = delta.map(|axis| store(axis as f64 * reciprocal));

    let mut step = 1.0;
    while step < limit {
        let offset = direction.map(|axis| axis as f64 * step);
        // The x87 original stores the vertical offset before adding target Z.
        let candidate = [
            store(target[0] as f64 + offset[0]),
            store(target[1] as f64 + offset[1]),
            store(target[2] as f64 + store(offset[2]) as f64),
        ];
        if fits(candidate) {
            return Some(candidate);
        }
        step += 1.0;
    }
    None
}

fn store_float(value: f64, toward_zero: bool) -> f32 {
    let rounded = value as f32;
    if toward_zero && (rounded as f64).abs() > value.abs() {
        f32::from_bits(rounded.to_bits() - 1)
    } else {
        rounded
    }
}
