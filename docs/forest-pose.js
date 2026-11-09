const f32 = Math.fround;
const invByte = f32(1 / 255);
const turn = f32(2 * Math.PI);
const wobbleSize = f32(0.005);
const fallCoefficient = f32(f32(Math.PI) / 9);
const view = new DataView(new ArrayBuffer(4));
const absent = 0xffffffff;

export function forestClockWord(word) {
  if (word === absent) return NaN;
  view.setUint32(0, word, true);
  return view.getFloat32(0, true);
}

export function forestPose(phase, variation, now, chop, exhaustion) {
  const fallen = phase === 2;
  const elapsed = f32(now - (fallen ? exhaustion : chop));
  const yaw = f32(variation * invByte * turn);
  if (fallen) {
    if (!(elapsed >= 0 && elapsed <= 3)) return {yaw, stump: true, trunk: false};
    return {
      yaw, stump: true, trunk: true,
      tilt: f32(elapsed * elapsed * fallCoefficient),
      sink: f32(elapsed * 2 / 3),
    };
  }
  if (phase !== 1 || !(elapsed >= 0 && elapsed <= 0.5)) return null;
  const progress = elapsed * 2;
  const sine = f32(Math.sin(progress * turn));
  return {yaw, trunk: true, tilt: f32((1 - progress) * wobbleSize * sine), sink: 0};
}

// Native third-axis times first-axis basis, in the mesh converter's x,z,-y
// coordinates and in column order for WebGL.
export function forestModelBasis(yaw, tilt) {
  const c = f32(Math.cos(yaw)), s = f32(Math.sin(yaw));
  const p = f32(Math.cos(tilt)), q = f32(Math.sin(tilt));
  return Float32Array.of(c, 0, s, s * q, p, -c * q, -s * p, q, c * p);
}
