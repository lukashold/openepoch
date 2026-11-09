const f32 = Math.fround;
const stride = 6;

function classify(vertices, at, plane) {
  const [x, y, z, distance] = plane;
  return (z * vertices[at + 2] + y * vertices[at + 1]) + x * vertices[at] - distance >= 0;
}

function planeDistance(vertices, at, plane) {
  const [x, y, z, distance] = plane;
  return (y * vertices[at + 1] + x * vertices[at]) + z * vertices[at + 2] - distance;
}

function crossing(a, at, b, bt, plane) {
  const da = planeDistance(a, at, plane);
  const db = planeDistance(b, bt, plane);
  const t = da / (da - db);
  const s = 1 - t;
  const normalS = f32(s);
  const point = [
    f32(f32(a[at] * s) + b[bt] * t),
    f32(f32(a[at + 1] * s) + f32(b[bt + 1] * t)),
    f32(f32(a[at + 2] * s) + f32(b[bt + 2] * t)),
    f32(f32(a[at + 3] * normalS) + b[bt + 3] * t),
    f32(f32(a[at + 4] * normalS) + f32(b[bt + 4] * t)),
    f32(f32(a[at + 5] * normalS) + f32(b[bt + 5] * t)),
  ];
  const [nx, ny, nz] = point.slice(3);
  const length = Math.sqrt((nx * nx + ny * ny) + nz * nz);
  if (length > 0) {
    const inverse = 1 / length;
    point[3] = f32(nx * inverse);
    point[4] = f32(ny * inverse);
    point[5] = f32(nz * inverse);
  }
  return point;
}

// One call covers all selected index spans of a single terrain render block.
// The output identities depend on source vertex IDs, never on equal coordinates.
export function clipTerrainDecal(vertices, indices, planes) {
  const crossingIds = new Map();
  const generated = [];
  const outputIds = new Map();
  const outputVertices = [];
  const outputIndices = [];

  function vertex(id) {
    return id >= 0 ? [vertices, id * stride] : [generated[~id], 0];
  }

  function crossingId(first, second, planeIndex) {
    const a = Math.min(first, second);
    const b = Math.max(first, second);
    const key = `${a}:${b}:${planeIndex}`;
    let id = crossingIds.get(key);
    if (id !== undefined) return id;
    const [av, at] = vertex(a);
    const [bv, bt] = vertex(b);
    id = ~generated.length;
    generated.push(crossing(av, at, bv, bt, planes[planeIndex]));
    crossingIds.set(key, id);
    return id;
  }

  function outputIndex(id) {
    let out = outputIds.get(id);
    if (out !== undefined) return out;
    const [values, start] = vertex(id);
    out = outputIds.size;
    outputIds.set(id, out);
    for (let i = 0; i < stride; ++i) outputVertices.push(values[start + i]);
    return out;
  }

  function clipPolygon(polygon, planeIndex) {
    const plane = planes[planeIndex];
    const next = [];
    let last = polygon.at(-1);
    let lastInside = classify(...vertex(last), plane);
    for (const current of polygon) {
      const inside = classify(...vertex(current), plane);
      if (lastInside !== inside) next.push(crossingId(last, current, planeIndex));
      if (inside) next.push(current);
      last = current;
      lastInside = inside;
    }
    return next;
  }

  for (let i = 0; i < indices.length; i += 3) {
    let polygon = [indices[i], indices[i + 1], indices[i + 2]];
    for (let plane = 0; plane < planes.length && polygon.length; ++plane) {
      polygon = clipPolygon(polygon, plane);
    }
    for (let fan = 2; fan < polygon.length; ++fan) {
      outputIndices.push(outputIndex(polygon[0]), outputIndex(polygon[fan - 1]), outputIndex(polygon[fan]));
    }
  }
  return { vertices: Float32Array.from(outputVertices), indices: Uint16Array.from(outputIndices) };
}
