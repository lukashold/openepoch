//! The scoped atlas arithmetic uses 53-bit x87 and explicit f32 stores.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Precision53 {
    NearestEven,
    TowardZero,
}

impl Precision53 {
    pub fn add(self, a: f64, b: f64) -> f64 {
        let result = a + b;
        if self == Self::TowardZero && result != 0.0 {
            let v = result - a;
            let remainder = (a - (result - v)) + (b - v);
            if remainder != 0.0 && result.is_sign_negative() != remainder.is_sign_negative() {
                return f64::from_bits(result.to_bits() - 1);
            }
        }
        result
    }

    // Inputs on the scoped tree/raster path are f32 products or small integer
    // and power-of-two scalings, all exactly representable in binary53.
    pub fn mul(self, a: f64, b: f64) -> f64 {
        a * b
    }

    pub fn store_f32(self, value: f64) -> f32 {
        let nearest = value as f32;
        if self == Self::TowardZero && f64::from(nearest).abs() > value.abs() {
            f32::from_bits(nearest.to_bits() - 1)
        } else {
            nearest
        }
    }
}

pub fn quantized_floor(value: f64, precision: Precision53) -> i32 {
    // 00B40390 is 1.5*2^36, giving a mantissa word at 16 fractional bits.
    let biased = precision.add(value, f64::from_bits(0x4238_0000_0000_0000));
    ((biased.to_bits() as u32) as i32) >> 16
}
