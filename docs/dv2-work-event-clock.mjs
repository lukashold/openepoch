// frequencyRatio22050 is a diagnostic reference ratio, not decoded-buffer playback rate.
// For consumed=true use frequency / decodedSampleRate; consumed=false keeps playback rate1.
/* Source-tested presentation arithmetic only. The caller supplies scene delta,
 * model period context, key declarations and audio RNG lifetime. No world clock
 * or audio/FX/backend ownership is selected here. See native-proof.json. */
const scratch = new DataView(new ArrayBuffer(8));
const profiles = new Set(['nearest', 'towardZero']);
function checked(profile) {
  if (!profiles.has(profile)) throw new TypeError('Explicit nearest or towardZero profile required');
  return profile;
}
function parts(value) {
  if (!Number.isFinite(value)) throw new RangeError('Finite binary input required');
  scratch.setFloat64(0, value, false);
  const word = scratch.getBigUint64(0, false), exponent = Number((word >> 52n) & 2047n);
  const sign = (word >> 63n) ? -1n : 1n;
  return { n: sign * ((word & ((1n << 52n) - 1n)) | (exponent ? 1n << 52n : 0n)),
    e: exponent ? exponent - 1023 - 52 : -1074 };
}
function roundedInteger(n, shift, profile) {
  if (shift <= 0) return n << BigInt(-shift);
  const s = BigInt(shift), q = n >> s;
  if (profile === 'towardZero') return q;
  const remainder = n - (q << s), half = 1n << (s - 1n);
  return q + ((remainder > half || (remainder === half && (q & 1n))) ? 1n : 0n);
}
function rounded(n, exponent, precision, profile, negativeZero = false) {
  checked(profile);
  const negative = n < 0n || (n === 0n && negativeZero), magnitude = n < 0n ? -n : n;
  const single = precision === 24, fraction = precision - 1, bias = single ? 127 : 1023;
  const minimumNormal = 1 - bias, maximumExponent = bias, minimumSub = minimumNormal - fraction;
  const signWord = negative ? 1n << BigInt(single ? 31 : 63) : 0n;
  let payload = 0n;
  if (magnitude) {
    const length = magnitude.toString(2).length;
    let top = exponent + length - 1;
    if (top < minimumNormal) {
      payload = roundedInteger(magnitude, minimumSub - exponent, profile);
    } else {
      let significand = roundedInteger(magnitude, length - precision, profile);
      if (significand === 1n << BigInt(precision)) { significand >>= 1n; top++; }
      if (top > maximumExponent) {
        payload = profile === 'towardZero'
          ? (BigInt(2 * bias) << BigInt(fraction)) | ((1n << BigInt(fraction)) - 1n)
          : BigInt(2 * bias + 1) << BigInt(fraction);
      } else payload = (BigInt(top + bias) << BigInt(fraction)) | (significand - (1n << BigInt(fraction)));
    }
  }
  if (single) {
    scratch.setUint32(0, Number(signWord | payload), false);
    return scratch.getFloat32(0, false);
  }
  scratch.setBigUint64(0, signWord | payload, false);
  return scratch.getFloat64(0, false);
}
export function sourceF32(value, profile) {
  const p = parts(value); return rounded(p.n, p.e, 24, profile, Object.is(value, -0));
}
function add(a, b, profile) {
  const x = parts(a), y = parts(b), e = Math.min(x.e, y.e);
  return rounded((x.n << BigInt(x.e - e)) + (y.n << BigInt(y.e - e)), e, 53, profile,
    Object.is(a, -0) && Object.is(b, -0));
}
function subtract(a, b, profile) { return add(a, -b, profile); }
function multiply(a, b, profile) {
  const x = parts(a), y = parts(b);
  return rounded(x.n * y.n, x.e + y.e, 53, profile,
    (a < 0 || Object.is(a, -0)) !== (b < 0 || Object.is(b, -0)));
}
function fold(time, period, start, profile) {
  const width = sourceF32(subtract(period, start, profile), profile);
  if (!(width > 0)) throw new RangeError('Native loop width must be positive');
  // Exact dyadic subtraction is invariant throughout this common native path.
  // The fast remainder avoids one iteration per old cold cycle, without doing
  // the incorrect binary32-rounded subtraction or applying remainder early.
  if (start === 0) {
    const a = parts(time), b = parts(period), e = Math.min(a.e, b.e);
    const n = a.n << BigInt(a.e - e), d = b.n << BigInt(b.e - e);
    if (n >= 0 && d > 0 && n.toString(2).length <= 53) return rounded(n % d, e, 24, profile);
  }
  let value = time;
  for (let steps = 0; value >= period; steps++) {
    if (steps >= 1000000) throw new RangeError('Outside bounded original subtraction profile');
    const next = subtract(value, width, profile);
    if (!(next < value)) throw new RangeError('Native loop cannot make numeric progress');
    value = next;
  }
  return sourceF32(value, profile);
}
function keyType(key) { return Number(key.type); }
function scan(clock, first, last, phase, emit) {
  for (let index = 0; index < clock.keys.length; index++) {
    const key = clock.keys[index], type = keyType(key);
    if (!(first <= key.time && key.time < last) || type < 0 || type > 11) continue;
    if (key.time === 0 && [0, 1, 6, 7, 10, 11].includes(type)) continue;
    emit?.({ type, data: key.data, time: key.time, index, phase });
  }
}
export function configureTextClock(previous, { period, mode = 0, elapsed = 0, keys = [] }, profile, emit) {
  checked(profile);
  period = sourceF32(period, profile);
  if (period === -1) period = previous?.period ?? 0;
  if (period === 0) period = f32FromBits(0x7f7fffff);
  const clock = { period, mode: mode >>> 0, time: sourceF32(elapsed, profile), start: 0, speed: 1,
    keys: keys.map(key => ({ ...key, type: keyType(key), time: sourceF32(key.time, profile) })) };
  for (let index = 0; index < clock.keys.length; index++) {
    const key = clock.keys[index], type = key.type;
    if ((key.time === 0 && [0, 1, 10, 11].includes(type)) || (key.time !== 0 && [6, 7].includes(type)))
      emit?.({ type, data: key.data, time: key.time, index, phase: 'configure' });
  }
  return clock;
}
export function advanceTextClock(clock, incomingSceneDelta, profile, emit) {
  checked(profile);
  const old = clock.time, delta = sourceF32(incomingSceneDelta, profile);
  clock.time = sourceF32(add(multiply(delta, clock.speed, profile), old, profile), profile);
  if (clock.mode === 0) {
    if (clock.time >= clock.period) {
      clock.time = fold(clock.time, clock.period, clock.start, profile);
      scan(clock, old, clock.period, 'tail', emit);
      scan(clock, clock.start, clock.time, 'head', emit);
      return 1;
    }
    scan(clock, old, clock.time, 'interval', emit);
    return 0;
  }
  if (clock.mode === 1) {
    const result = clock.time >= clock.period ? 2 : 0;
    scan(clock, old, clock.time, 'interval', emit);
    return result; // Native once mode reports end; its time is not clamped.
  }
  return 0; // Native other modes advance time without dispatch.
}
export function modelTextPeriod({ span, intrinsic = 0, animatorScale24 = 1, animatorScale20 = 1, hasAnimator = true, hasAlternateAnimator = false, alternatePeriod = 0 }, profile) {
  span = sourceF32(span, profile);
  if (span !== 0) return span;
  if (!hasAnimator) return hasAlternateAnimator ? sourceF32(alternatePeriod, profile) : .5;
  return sourceF32(multiply(multiply(sourceF32(intrinsic, profile), sourceF32(animatorScale24, profile), profile),
    sourceF32(animatorScale20, profile), profile), profile);
}
export function advancePitch(seed, variation, profile) {
  checked(profile); variation = sourceF32(variation, profile); seed >>>= 0;
  if (!(variation > 0)) return { seed, consumed: false, frequency: null, frequencyRatio22050: 1 };
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  const q = (seed >>> 16) & 32767;
  let frequency = multiply(q, subtract(variation, -variation, profile), profile);
  frequency = multiply(frequency, f32FromBits(0x3f2c4559), profile);
  frequency = add(frequency, multiply(-variation, 22050, profile), profile);
  frequency = add(frequency, 22050, profile);
  if (profile === 'towardZero') frequency = Math.trunc(frequency);
  else { const floor = Math.floor(frequency), remainder = frequency - floor;
    frequency = floor + (remainder > .5 || (remainder === .5 && floor % 2 !== 0) ? 1 : 0); }
  return { seed, consumed: true, frequency, frequencyRatio22050: frequency / 22050 };
}
export function f32Bits(value) { scratch.setFloat32(0, value, false); return scratch.getUint32(0, false); }
export function f32FromBits(word) { scratch.setUint32(0, word >>> 0, false); return scratch.getFloat32(0, false); }
