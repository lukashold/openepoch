//! Opt-in simulation clock. The scheduler supplies the unrounded product
//! of wall tick interval and effective speed. Source 0059A770 stores dt as f32
//! and adds the retained binary64 product to the f32 game time. The native
//! frame wrapper selects x87 53-bit precision and round-toward-zero stores.

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"VCL3");
pub(crate) const ADAPTIVE_MAGIC: u32 = u32::from_le_bytes(*b"VCA1");
pub(crate) const NETWORK_MAGIC: u32 = u32::from_le_bytes(*b"VCN1");
const HEADER_LEN: usize = 16;
const ADAPTIVE_HEADER_LEN: usize = 28;
const CHECKSUM_LEN: usize = 4;
// The stock adaptive limit is roughly 0.1 seconds. Leave room for its f32
// intermediate, while bounding one injected update in this experimental ABI.
const MAX_STEP: f32 = 0.125;
const MAX_ADAPTIVE_STEP: f64 = f64::from_bits(0x3fd4_7ae1_4ccc_cccc);
const MIN_RATE: u32 = 5;
const MAX_RATE: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AdaptiveTempo {
    pub(crate) wall_rate: u32,
    pub(crate) rate: u32,
    pub(crate) scale: f32,
}

impl AdaptiveTempo {
    fn checked(wall_rate: u32, rate: u32, scale: f32) -> Option<Self> {
        if !(MIN_RATE..=MAX_RATE).contains(&wall_rate) || !(MIN_RATE..=MAX_RATE).contains(&rate) {
            return None;
        }
        let cap = store_positive_f32(f64::from(rate) * f64::from(0.1f32)).min(1.6f32);
        if !(0.05f32..=cap).contains(&scale) { return None; }
        Some(Self { wall_rate, rate, scale })
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn network_resume(rate: u32, scale: f32) -> Option<Self> {
        Self::checked(rate, rate, scale)
    }

    fn delta(self) -> f64 {
        let rate = f64::from(self.wall_rate);
        let nearest = 1.0 / rate;
        let (high, low) = product_parts(nearest, rate);
        let product = high + low;
        let interval = if (product - 1.0) + sum_error(high, low, product) > 0.0 {
            f64::from_bits(nearest.to_bits() - 1)
        } else { nearest };
        let (high, low) = product_parts(interval, f64::from(self.scale));
        add_positive_f64(high, low)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SourceClock {
    pub(crate) elapsed: f32,
    pub(crate) last_step: f32,
    pub(crate) previous: f32,
    pub(crate) adaptive: Option<AdaptiveTempo>,
    pub(crate) network: bool,
}

impl SourceClock {
    pub(crate) fn advance(self, seconds: f64) -> Option<Self> {
        if self.adaptive.is_some() { return None; }
        self.advance_inner(seconds, None)
    }

    pub(crate) fn advance_adaptive(self, tick: u32, wall_rate: u32, rate: u32, scale: f32) -> Option<Self> {
        if self.network { return None; }
        let tempo = AdaptiveTempo::checked(wall_rate, rate, scale)?;
        if let Some(previous) = self.adaptive {
            if wall_rate != previous.rate || (tick % MAX_RATE != 0 &&
                (rate != previous.rate || scale.to_bits() != previous.scale.to_bits())) { return None; }
        } else if tick != 0 || wall_rate != rate { return None; }
        self.advance_inner(tempo.delta(), Some(tempo))
    }

    pub(crate) fn advance_network(self, tick: u32, wall_rate: u32, rate: u32, scale: f32, enters_round: bool) -> Option<Self> {
        let tempo = AdaptiveTempo::checked(wall_rate, rate, scale)?;
        if let Some(previous) = self.adaptive {
            if !self.network || wall_rate != previous.rate || (!enters_round &&
                (rate != previous.rate || scale.to_bits() != previous.scale.to_bits())) {
                return None;
            }
        } else if tick != 0 || wall_rate != MAX_RATE || rate != wall_rate {
            return None;
        }
        let mut advanced = self.advance_inner(tempo.delta(), Some(tempo))?;
        advanced.network = true;
        Some(advanced)
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn advance_network_resume(self, tempo: AdaptiveTempo) -> Option<Self> {
        if !self.network { return None; }
        self.advance_inner(tempo.delta(), Some(tempo))
    }

    pub(crate) fn magic(self) -> u32 {
        if self.network { NETWORK_MAGIC }
        else if self.adaptive.is_some() { ADAPTIVE_MAGIC }
        else { MAGIC }
    }

    fn advance_inner(self, seconds: f64, adaptive: Option<AdaptiveTempo>) -> Option<Self> {
        let maximum = if adaptive.is_some() { MAX_ADAPTIVE_STEP } else { f64::from(MAX_STEP) };
        if !(0.0..=maximum).contains(&seconds) || seconds == 0.0 {
            return None;
        }
        let last_step = store_positive_f32(seconds);
        if last_step == 0.0 { return None; }
        let elapsed = store_positive_f32(add_positive_f64(f64::from(self.elapsed), seconds));
        Some(Self { elapsed, last_step, previous: self.elapsed, adaptive, network: self.network })
    }

    pub(crate) fn valid_at(self, tick: u32, network: bool) -> bool {
        let maximum = if self.adaptive.is_some() { MAX_ADAPTIVE_STEP } else { f64::from(MAX_STEP) };
        self.network == network && tick > 0 && f64::from(self.elapsed) <= f64::from(tick) * maximum
            && f64::from(self.previous) <= f64::from(tick - 1) * maximum
            && self.adaptive.is_none_or(|tempo| {
                if self.network && tick == 1 { return tempo.wall_rate == MAX_RATE && tempo.rate == MAX_RATE; }
                if tempo.wall_rate == tempo.rate { return true; }
                if self.network { tick > 1 } else { tick > 1 && (tick - 1) % MAX_RATE == 0 }
            })
    }
}

fn store_positive_f32(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest) > value { f32::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn add_positive_f64(left: f64, right: f64) -> f64 {
    let nearest = left + right;
    if sum_error(left, right, nearest) < 0.0 { f64::from_bits(nearest.to_bits() - 1) } else { nearest }
}

fn sum_error(left: f64, right: f64, nearest: f64) -> f64 {
    let virtual_right = nearest - left;
    (left - (nearest - virtual_right)) + (right - virtual_right)
}

fn product_parts(left: f64, right: f64) -> (f64, f64) {
    let expanded = 134217729.0 * left;
    let high = expanded - (expanded - left);
    (high * right, (left - high) * right)
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
    let header = if clock.adaptive.is_some() { ADAPTIVE_HEADER_LEN } else { HEADER_LEN };
    let mut out = Vec::with_capacity(header + inner.len() + CHECKSUM_LEN);
    out.extend_from_slice(&clock.magic().to_le_bytes());
    out.extend_from_slice(&clock.elapsed.to_le_bytes());
    out.extend_from_slice(&clock.last_step.to_le_bytes());
    out.extend_from_slice(&clock.previous.to_le_bytes());
    if let Some(tempo) = clock.adaptive {
        out.extend_from_slice(&tempo.wall_rate.to_le_bytes());
        out.extend_from_slice(&tempo.rate.to_le_bytes());
        out.extend_from_slice(&tempo.scale.to_le_bytes());
    }
    out.extend_from_slice(&inner);
    out.extend_from_slice(&crate::save::checksum(&out).to_le_bytes());
    out
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<SourceClock>)> {
    let network = buf.starts_with(&NETWORK_MAGIC.to_le_bytes());
    let adaptive_archive = network || buf.starts_with(&ADAPTIVE_MAGIC.to_le_bytes());
    if !adaptive_archive && !buf.starts_with(&MAGIC.to_le_bytes()) { return Some((buf, None)); }
    let elapsed = f32::from_le_bytes(buf.get(4..8)?.try_into().ok()?);
    let last_step = f32::from_le_bytes(buf.get(8..12)?.try_into().ok()?);
    if !last_step.is_finite() || last_step <= 0.0 || !elapsed.is_finite() || elapsed < last_step {
        return None;
    }
    let previous = f32::from_le_bytes(buf.get(12..16)?.try_into().ok()?);
    if previous.is_sign_negative() || !previous.is_finite() || previous > elapsed { return None; }
    let end = buf.len().checked_sub(CHECKSUM_LEN)?;
    let stored = u32::from_le_bytes(buf.get(end..)?.try_into().ok()?);
    if stored != crate::save::checksum(buf.get(..end)?) { return None; }
    let (header, adaptive) = if adaptive_archive {
        let word = |at| Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?));
        let tempo = AdaptiveTempo::checked(word(16)?, word(20)?, f32::from_bits(word(24)?))?;
        let source = SourceClock { elapsed: previous, ..SourceClock::default() };
        let next = source.advance_inner(tempo.delta(), Some(tempo))?;
        if next.elapsed != elapsed || next.last_step != last_step { return None; }
        (ADAPTIVE_HEADER_LEN, Some(tempo))
    } else {
        if !valid_step(last_step) || !valid_advance(previous, elapsed, last_step) { return None; }
        (HEADER_LEN, None)
    };
    let payload = buf.get(header..end)?;
    Some((payload, Some(SourceClock { elapsed, last_step, previous, adaptive, network })))
}
