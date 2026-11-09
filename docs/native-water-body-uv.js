const f32 = Math.fround;
const DEFAULT_SCALE = f32(0.07);
const JITTER_AMPLITUDE = f32(0.03);

/** Source native X/Y coordinates, with native Z as the height axis. */
export function createOriginalWaterBodyUV({uScale = DEFAULT_SCALE, vScale = DEFAULT_SCALE, rounding = 'chop'} = {}) {
  uScale = f32(uScale);
  vScale = f32(vScale);
  if (!Number.isFinite(uScale) || !Number.isFinite(vScale) ||
      (rounding !== 'nearest' && rounding !== 'chop')) throw new RangeError('Invalid original water UV configuration');
  const value = new Float32Array(1);
  const bits = new Uint32Array(value.buffer);
  const store = rounding === 'nearest' ? f32 : number => {
    value[0] = number;
    if (Math.abs(value[0]) > Math.abs(number)) bits[0]--;
    return value[0];
  };
  return (x, y) => {
    x = f32(x);
    y = f32(y);
    const jitter = Math.cos((x + y) * 0.25) * JITTER_AMPLITUDE;
    const baseV = store(y * vScale);
    return [store(x * uScale + jitter), store(baseV - jitter)];
  };
}

export const originalWaterBodyUV = createOriginalWaterBodyUV();
