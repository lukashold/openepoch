const f32 = Math.fround;

// Source 00889A50 and 00A4D610, on finite native coordinates.
export function admitsReflectionActorWater(localEye, sphere, visibleBodies) {
  const radius = sphere[3];
  if (radius > 10) return true;
  const delta = localEye.map((value, axis) => f32(sphere[axis] - value));
  const [dx, dy, dz] = delta;
  const length = Math.sqrt((dx*dx + dy*dy) + dz*dz);
  const [x, y, z] = length > 0 ? delta.map(value => f32(value * (1/length))) : delta;
  if (z === 0) return false;

  const time = (-1/z) * localEye[2];
  const hitX = f32(localEye[0] + time*x), hitY = f32(localEye[1] + f32(time*y));
  return visibleBodies.some(({sphere:[bodyX, bodyY, , bodyRadius]}) => {
    const offsetX = bodyX - hitX, offsetY = bodyY - hitY;
    return Math.sqrt(offsetY*offsetY + offsetX*offsetX) <= bodyRadius + radius;
  });
}
