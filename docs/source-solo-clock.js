const f32 = Math.fround;
const bits = new DataView(new ArrayBuffer(8));

function precedingF64(value) {
  bits.setFloat64(0, value);
  bits.setBigUint64(0, bits.getBigUint64(0) - 1n);
  return bits.getFloat64(0);
}

function storeF32(value) {
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

function addF64(left, right) {
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
