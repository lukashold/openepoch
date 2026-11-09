// Planes emitted by the original unit decal path at 00714E8A–00714F76.
// Use the original coordinate basis and the already rounded source trig pair.
export function unitTerrainDecalPlanes(input) {
  if (input.length !== 4) throw new RangeError('Expected center X, center Y, cosine and sine');
  const [x, y, cosine, sine] = Array.from(input, Math.fround);
  if (![x, y, cosine, sine].every(Number.isFinite)) {
    throw new RangeError('Decal center and orientation must be finite f32 values');
  }

  const plane = (nx, ny) => [nx, ny, 0, Math.fround((y * ny + x * nx) - 0.5)];
  return [
    plane(cosine, sine),
    plane(-cosine, -sine),
    plane(sine, -cosine),
    plane(-sine, cosine),
  ];
}
