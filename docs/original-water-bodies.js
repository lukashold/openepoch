// Stock WaterManager geometry descriptor from 008895D0 and 0088B2D0.
const SAMPLE_SPAN = 8;
const STEP = SAMPLE_SPAN - 1;
const MIN_SENTINEL = 1e9;
const MAX_SENTINEL = -1e10;
const f32 = Math.fround;

function buildChunk(heights, sampleWidth, x0, y0, width, height, id, dryCenter) {
  const corners = new Float32Array(width * height);
  let samplesAtOrBelowSurface = 0;
  for (let y = 0; y < height; y++) {
    const start = (y0 + y) * sampleWidth + x0;
    const row = heights.subarray(start, start + width);
    corners.set(row, y * width);
    for (const sample of row) samplesAtOrBelowSurface += +(sample <= 0);
  }
  const resourceReady = samplesAtOrBelowSurface !== 0;
  const indices = [];
  let sphere = [...dryCenter, 0];
  const capacity = (width - 1) * (height - 1) * 2;
  if (resourceReady) {
    let minX = MIN_SENTINEL;
    let minY = MIN_SENTINEL;
    let maxX = MAX_SENTINEL;
    let maxY = MAX_SENTINEL;
    function triangle(a, b, c) {
      if (!(corners[a] < 0 || corners[b] < 0 || corners[c] < 0)) return;
      indices.push(a, b, c);
      for (const vertex of [a, b, c]) {
        const x = x0 + vertex % width;
        const y = y0 + Math.floor(vertex / width);
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
      }
    }
    for (let y = 0; y < height - 1; y++) for (let x = 0; x < width - 1; x++) {
      const a = y * width + x, b = a + 1, c = a + width, d = c + 1;
      triangle(a, b, d);
      triangle(a, d, c);
    }
    const dx = f32(maxX - minX);
    const dy = f32(maxY - minY);
    const radius = f32(Math.sqrt((maxY - minY) * dy + dx * dx) * 0.5);
    sphere = [f32(minX + dx * 0.5), f32(minY + dy * 0.5), 0, radius];
  }
  const triangleCount = indices.length / 3;
  const surfaceField = resourceReady ? triangleCount : capacity;
  return {
    id, x: x0, y: y0, width, height, sphere,
    samplesAtOrBelowSurface, triangleCount, surfaceField,
    resourceReady, surfaceReady: surfaceField !== 0,
    indices: resourceReady ? new Uint16Array(indices) : null,
  };
}

/** Construct all fixed partition slots from an original Z-up corner grid at water Z=0. */
export function buildOriginalWaterBodies(heights, sampleWidth, sampleHeight, {dryCenter = [0, 0, 0]} = {}) {
  if (!(heights instanceof Float32Array) || !Number.isInteger(sampleWidth) ||
      !Number.isInteger(sampleHeight) || sampleWidth < 2 || sampleHeight < 2 ||
      heights.length !== sampleWidth * sampleHeight || dryCenter?.length !== 3 ||
      !Array.from(dryCenter).every(Number.isFinite)) throw new RangeError('Invalid original water corner grid');
  const bodies = [];
  const center = Array.from(dryCenter, f32);
  for (let y = 0; y < sampleHeight - 1; y += STEP) {
    const height = Math.min(SAMPLE_SPAN, sampleHeight - y);
    for (let x = 0; x < sampleWidth - 1; x += STEP) {
      const width = Math.min(SAMPLE_SPAN, sampleWidth - x);
      bodies.push(buildChunk(heights, sampleWidth, x, y, width, height, bodies.length, center));
    }
  }
  return bodies;
}
