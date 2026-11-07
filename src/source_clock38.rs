//! Opt-in solo simulation clock. The scheduler supplies the unrounded product
//! of wall tick interval and effective speed. Source 0059A770 stores dt as f32
//! and adds the retained binary64 product to the f32 game time. The native
//! frame wrapper selects x87 53-bit precision and round-toward-zero stores.

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"VCL3");
const HEADER_LEN: usize = 16;
const CHECKSUM_LEN: usize = 4;
// The stock adaptive limit is roughly 0.1 seconds. Leave room for its f32
// intermediate, while bounding one injected update in this experimental ABI.
const MAX_STEP: f32 = 0.125;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SourceClock {
    pub(crate) elapsed: f32,
    pub(crate) last_step: f32,
    pub(crate) previous: f32,
}

impl SourceClock {
    pub(crate) fn advance(self, seconds: f64) -> Option<Self> {
        if !(0.0..=f64::from(MAX_STEP)).contains(&seconds) || seconds == 0.0 {
            return None;
        }
        let last_step = store_positive_f32(seconds);
        if !valid_step(last_step) { return None; }
        let elapsed = store_positive_f32(add_positive_f64(f64::from(self.elapsed), seconds));
        Some(Self { elapsed, last_step, previous: self.elapsed })
    }

    pub(crate) fn valid_at(self, tick: u32, network: bool) -> bool {
        let maximum = f64::from(MAX_STEP);
        !network && tick > 0 && f64::from(self.elapsed) <= f64::from(tick) * maximum
            && f64::from(self.previous) <= f64::from(tick - 1) * maximum
    }
}

fn store_positive_f32(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest) > value { f32::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn add_positive_f64(left: f64, right: f64) -> f64 {
    let nearest = left + right;
    let virtual_right = nearest - left;
    let residual = (left - (nearest - virtual_right)) + (right - virtual_right);
    if residual < 0.0 { f64::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn valid_step(step: f32) -> bool {
    step > 0.0 && step <= MAX_STEP
}

fn valid_advance(previous: f32, elapsed: f32, last_step: f32) -> bool {
    // The positive chopped step bounds the retained f64 product in
    // [last_step, next_f32(last_step)). Test the inclusive binary64 extremes.
    let next_step = f64::from(f32::from_bits(last_step.to_bits() + 1));
    let highest_step = f64::from_bits(next_step.to_bits() - 1).min(f64::from(MAX_STEP));
    let previous = f64::from(previous);
    let lowest = store_positive_f32(add_positive_f64(previous, f64::from(last_step)));
    let highest = store_positive_f32(add_positive_f64(previous, highest_step));
    (lowest..=highest).contains(&elapsed)
}

pub(crate) fn wrap(inner: Vec<u8>, clock: Option<SourceClock>) -> Vec<u8> {
    let Some(clock) = clock else { return inner; };
    if inner.is_empty() { return inner; }
    let mut out = Vec::with_capacity(HEADER_LEN + inner.len() + CHECKSUM_LEN);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&clock.elapsed.to_le_bytes());
    out.extend_from_slice(&clock.last_step.to_le_bytes());
    out.extend_from_slice(&clock.previous.to_le_bytes());
    out.extend_from_slice(&inner);
    out.extend_from_slice(&crate::save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<SourceClock>)> {
    if !buf.starts_with(&MAGIC.to_le_bytes()) { return Some((buf, None)); }
    let elapsed = f32::from_le_bytes(buf.get(4..8)?.try_into().ok()?);
    let last_step = f32::from_le_bytes(buf.get(8..12)?.try_into().ok()?);
    if !valid_step(last_step) || !elapsed.is_finite() || elapsed < last_step {
        return None;
    }
    let previous = f32::from_le_bytes(buf.get(12..16)?.try_into().ok()?);
    if previous.is_sign_negative() || !previous.is_finite() || previous > elapsed { return None; }
    let end = buf.len().checked_sub(CHECKSUM_LEN)?;
    let stored = u32::from_le_bytes(buf.get(end..)?.try_into().ok()?);
    if !valid_advance(previous, elapsed, last_step) || stored != crate::save::checksum(buf.get(..end)?) {
        return None;
    }
    let payload = buf.get(HEADER_LEN..end)?;
    Some((payload, Some(SourceClock { elapsed, last_step, previous })))
}
