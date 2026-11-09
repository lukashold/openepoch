const f32 = Math.fround;
const scratch = new Float32Array(1);
const scratchBits = new Uint32Array(scratch.buffer);

export const NATIVE_REFLECTION_WARP = Object.freeze({
  lowSide: 20,
  highSide: 14,
  seed: 12345,
  seedScale: f32(0.00019175084889866412),
  phaseLimit: f32(6.2831),
  rate: 1,
  amplitude: f32(0.0035),
});

function towardZero(value) {
  scratch[0] = value;
  if (Math.abs(scratch[0]) > Math.abs(value)) scratchBits[0]--;
  return scratch[0];
}

function sumTowardZero(a, b) {
  const sum = a + b;
  const rounded = towardZero(sum);
  if (!Number.isFinite(sum) || rounded !== sum || sum === 0) return rounded;
  // Recover a discarded binary64 tail when the rounded sum lies exactly
  // on an f32 boundary. This is observable with very small opposite deltas.
  const bRounded = sum - a;
  const tail = (a - (sum - bRounded)) + (b - bRounded);
  if ((sum > 0 && tail < 0) || (sum < 0 && tail > 0)) scratchBits[0]--;
  return scratch[0];
}

function arithmetic(rounding) {
  if (rounding === 'chop') return {store: towardZero, sum: sumTowardZero};
  if (rounding === 'nearest') return {store: f32, sum: (a, b) => f32(a + b)};
  throw new RangeError('Unsupported native water rounding mode');
}

function cosineOffset(phase, store) {
  // FCOS leaves a finite operand outside its instruction range unchanged.
  const cosine = Math.abs(phase) < 2 ** 63 ? Math.cos(phase) : phase;
  const offset = store(cosine * NATIVE_REFLECTION_WARP.amplitude);
  if (store === towardZero && phase !== 0 && Math.abs(cosine) === 1) {
    // Binary64 Math.cos can round a near-extremum to ±1 even when FCOS
    // followed by the native toward-zero multiply stays below that magnitude.
    scratchBits[0]--;
    return scratch[0];
  }
  return offset;
}

/** Own one native target's elapsed time, phase array and interleaved UV offsets. */
export function createNativeReflectionWarp({requestedMode = 0, ceiling = 1, rounding = 'chop'} = {}) {
  if (ceiling !== -1 && ceiling !== 0 && ceiling !== 1) throw new RangeError('Invalid native water mode ceiling');
  const defaultArithmetic = arithmetic(rounding);
  let mode = -1;
  let side = 0;
  let elapsed = 0;
  let phases = new Float32Array();
  let offsets = new Float32Array();

  function setMode(requested, options) {
    if (requested !== 0 && requested !== 1) throw new RangeError('Invalid requested reflection mode');
    const next = Math.min(requested, ceiling);
    if (next === mode) return false;
    const {store} = options?.rounding === undefined ? defaultArithmetic : arithmetic(options.rounding);
    mode = next;
    elapsed = 0;
    side = mode === 0 ? NATIVE_REFLECTION_WARP.lowSide : NATIVE_REFLECTION_WARP.highSide;
    phases = new Float32Array(side * side);
    offsets = new Float32Array(2 * side * side);
    let seed = NATIVE_REFLECTION_WARP.seed;
    for (let i = 0; i < phases.length; i++) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      phases[i] = store(((seed >>> 16) & 0x7fff) * NATIVE_REFLECTION_WARP.seedScale);
    }
    return true;
  }

  function advance(frameDelta, {meshReady = side !== 0, rounding: override} = {}) {
    const delta = f32(frameDelta);
    if (!Number.isFinite(delta)) throw new RangeError('Invalid reflection frame delta');
    const {store, sum} = override === undefined ? defaultArithmetic : arithmetic(override);
    elapsed = sum(elapsed, delta);
    if (!meshReady || mode !== 0) return offsets;
    const phaseDelta = Math.min(delta, 1) * NATIVE_REFLECTION_WARP.rate;
    for (let row = 1; row < side - 1; row++) for (let col = 1; col < side - 1; col++) {
      const index = row * side + col;
      const phase = sum(phases[index], phaseDelta);
      phases[index] = phase > NATIVE_REFLECTION_WARP.phaseLimit ? 0 : phase;
      const offset = cosineOffset(phases[index], store);
      offsets[2 * index] = offset;
      offsets[2 * index + 1] = -offset;
    }
    return offsets;
  }

  setMode(requestedMode);
  return {
    get mode() { return mode; },
    get side() { return side; },
    get elapsed() { return elapsed; },
    get phases() { return phases; },
    get offsets() { return offsets; },
    setMode,
    advance,
  };
}
