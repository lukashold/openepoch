const f32 = Math.fround;
const logCurve = weight => Math.log(1 + (f32(Math.E) - 1) * weight);

function angleWeight(angle, low, high) {
  if (angle <= low) return 1;
  if (angle >= high) return 0;
  return (high - angle) / (high - low);
}

// 00511BC0 projects the emitter through the primary camera before mixing.
// World coordinates follow the renderer's X, altitude, Y convention.
export function originalSpatialMix(view, x, y, altitude) {
  if (!view?.right || !view.u || !view.f || !view.eye ||
      !(view.fov > 0) || !(view.tanX > 0) || !(view.tanY > 0)) return {volume:0, pan:0};
  const dx = x - view.eye[0], dy = altitude - view.eye[1], dz = y - view.eye[2];
  const dot = axis => f32(dx * axis[0] + dy * axis[1] + dz * axis[2]);
  const right = dot(view.right), up = dot(view.u), forward = dot(view.f);
  const distanceSquared = right * right + up * up + forward * forward;
  if (!(forward > 0) || !(distanceSquared <= 2500)) return {volume:0, pan:0};

  const horizontal = f32(view.fov * .5), vertical = f32(horizontal / (view.tanX / view.tanY));
  const azimuth = f32(Math.atan2(Math.abs(right), forward));
  const elevation = f32(Math.atan2(Math.abs(up), forward));
  const radial = distanceSquared <= 900 ? 1 : f32((2500 - f32(distanceSquared)) / 1600);
  const horizontalGain = logCurve(angleWeight(azimuth, f32(.75 * horizontal), f32(1.5 * horizontal)));
  const verticalGain = logCurve(angleWeight(elevation, f32(.75 * vertical), f32((up < 0 ? 3 : 1.5) * vertical)));
  const signedPan = f32((right < 0 ? -1 : 1) * (1 - logCurve(angleWeight(azimuth, f32(f32(.3) * horizontal), f32(1.5 * horizontal)))));
  return {volume:f32(radial * horizontalGain * verticalGain), pan:2 * f32(signedPan * .5 + .5) - 1};
}

export const originalOneShotAudible = volume => volume > f32(.01);
export const originalLoopAudible = volume => volume > f32(.05);
