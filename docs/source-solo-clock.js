const f32 = Math.fround;
const bits = new DataView(new ArrayBuffer(8));
const MIN_RATE = 5, MAX_RATE = 30;
const CPU_FRACTION = 0.75, PREVIOUS_CPU_WEIGHT = f32(0.9);

export function precedingF64(value) {
  bits.setFloat64(0, value);
  bits.setBigUint64(0, bits.getBigUint64(0) - 1n);
  return bits.getFloat64(0);
}

export function storeF32(value) {
  const nearest = f32(value);
  if (nearest <= value) return nearest;
  bits.setFloat32(0, nearest);
  bits.setUint32(0, bits.getUint32(0) - 1);
  return bits.getFloat32(0);
}

function sumError(left, right, nearest) {
  const virtualRight = nearest - left;
  return (left - (nearest - virtualRight)) + (right - virtualRight);
}

export function addF64(left, right) {
  const nearest = left + right;
  return sumError(left, right, nearest) < 0 ? precedingF64(nearest) : nearest;
}

// The right operand is an f32 or a small integral rate. Splitting the f64
// left operand makes both products exact in binary64 before they are added.
function productParts(left, right) {
  const expanded = 134217729 * left;
  const high = expanded - (expanded - left);
  return [high * right, (left - high) * right];
}

export function sourceWallInterval(rate) {
  const nearest = 1 / rate;
  const [high, low] = productParts(nearest, rate);
  const product = high + low;
  return (product - 1) + sumError(high, low, product) > 0 ? precedingF64(nearest) : nearest;
}

// 0059AC50 uses an f32 wall accumulator and a strict comparison with the
// reciprocal of the tick rate. The browser adapter uses the source startup
// rate. Native CPU and render sampling for adaptive rates is a separate input.
export const SOURCE_SOLO_INTERVAL = sourceWallInterval(30);

export function sourceStepDelta(scale, interval = SOURCE_SOLO_INTERVAL) {
  return addF64(...productParts(interval, f32(scale)));
}

export function selectAdaptiveTempo(currentRate, cpuSeconds, renderFps, requested) {
  let rate = cpuSeconds === 0 ? MIN_RATE : Math.max(MIN_RATE, Math.min(MAX_RATE, Math.trunc(CPU_FRACTION / cpuSeconds)));
  if (rate > MIN_RATE) {
    // For an integer rate in range, truncation of the x87 quotient is the
    // mathematical floor. Correct an IEEE-nearest division at an integer edge.
    const [high, low] = productParts(cpuSeconds, rate);
    const product = high + low;
    if ((product - CPU_FRACTION) + sumError(high, low, product) > 0) rate--;
  }
  if (cpuSeconds < sourceStepDelta(CPU_FRACTION, sourceWallInterval(currentRate))) {
    rate = Math.max(currentRate, Math.min(rate, Math.ceil(renderFps)));
  }
  return {rate, scale: Math.min(f32(requested), storeF32(rate * f32(0.1)))};
}

export function sampleSourceRenderFps(state, seconds) {
  if (!Number.isFinite(seconds) || seconds < 0) return;
  state.count = storeF32(addF64(state.count, 1));
  const retained = addF64(state.remainder, storeF32(seconds));
  state.remainder = storeF32(retained);
  if (retained < 1) return;
  state.remainder = storeF32(addF64(state.remainder, -1));
  state.fps = state.count;
  state.count = 0;
}

export function createAdaptiveSourceClock(tick, rate, scale) {
  return {tick, rate, scale, cpuSeconds:0, remainder:0};
}

export function filterSourceCpu(previous, sample) {
  const oldPart = sourceStepDelta(PREVIOUS_CPU_WEIGHT, previous);
  const newPart = sourceStepDelta(1 - PREVIOUS_CPU_WEIGHT, sample);
  return addF64(oldPart, newPart);
}

export function advanceAdaptiveSourceFrame(state, wallSeconds, paused, renderFps, callbacks) {
  if (!Number.isFinite(wallSeconds) || wallSeconds < 0) wallSeconds = 0;
  state.remainder = storeF32(addF64(state.remainder, storeF32(wallSeconds)));
  let steps = 0, gameSeconds = 0;
  for (;;) {
    const oldRate = state.rate;
    const interval = sourceWallInterval(oldRate);
    if (state.remainder <= interval) break;
    state.remainder = storeF32(addF64(state.remainder, -interval));
    if (paused) continue;
    const {rate, scale} = state.tick % MAX_RATE === 0
      ? selectAdaptiveTempo(oldRate, state.cpuSeconds, renderFps, callbacks.requested()) : state;
    callbacks.beforeAdmit();
    const before = callbacks.nowSeconds();
    if (!callbacks.admit(oldRate, rate, scale)) {
      state.remainder = 0;
      break;
    }
    const after = callbacks.nowSeconds();
    const sample = after >= before ? addF64(after, -before) : 0;
    state.cpuSeconds = filterSourceCpu(state.cpuSeconds, sample);
    state.tick++;
    state.rate = rate;
    state.scale = scale;
    steps++;
    gameSeconds = storeF32(addF64(gameSeconds, sourceStepDelta(scale, interval)));
  }
  return {steps, gameSeconds};
}

export function advanceSourceSoloFrame(remainder, wallSeconds, paused, speedForStep, tick) {
  if (!Number.isFinite(wallSeconds) || wallSeconds < 0) wallSeconds = 0;
  let acc = storeF32(addF64(storeF32(remainder), storeF32(wallSeconds)));
  let steps = 0, gameSeconds = 0;
  while (acc > SOURCE_SOLO_INTERVAL) {
    acc = storeF32(addF64(acc, -SOURCE_SOLO_INTERVAL));
    if (paused) continue;
    const delta = sourceStepDelta(speedForStep());
    if (!tick(delta)) return { remainder: 0, steps, gameSeconds };
    steps++;
    gameSeconds = storeF32(addF64(gameSeconds, delta));
  }
  return { remainder: acc, steps, gameSeconds };
}

export function sourceRenderSeconds(wasm, alpha) {
  if (wasm.sim_clock_active?.() !== 1) return null;
  return wasm.sim_render_seconds(alpha);
}
