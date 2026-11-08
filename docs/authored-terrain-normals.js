// Ordinary source cells share their top-left to bottom-right diagonal.
// The source stores signed 16-bit horizontal normals at grid corners.
const f = Math.fround;
const PI = f(Math.PI);
const SCALE = 32767;
const NEIGHBORS = [[0, -1], [1, 0], [0, 1], [-1, 0], [-1, -1], [1, -1], [1, 1], [-1, 1]];

function triangle(nx, ny) {
  const length = Math.sqrt(nx * nx + ny * ny + 1);
  return [f(nx / length), f(ny / length), f(1 / length)];
}

function cornerWeight(above, below, corner, axis) {
  if (corner === 0 || corner === 2) return f(f(f(above[axis] + below[axis]) * PI) * .25);
  return f(f((corner === 1 ? above : below)[axis] * PI) * .5);
}

function cornerAt(x, y, cellX, cellY) {
  if (y === cellY) return x === cellX ? 0 : 1;
  return x === cellX ? 3 : 2;
}

function packedComponent(value) {
  return Math.floor(value * SCALE + .5 + 1 / 131072);
}

export function authoredTerrainNormals({width, height, elevations}) {
  const stride = width + 1;
  const above = new Array(width * height), below = new Array(width * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = y * stride + x, cell = y * width + x;
      const a = elevations[i], b = elevations[i + 1], c = elevations[i + stride], d = elevations[i + stride + 1];
      above[cell] = triangle(-f(b - a), f(f(b - a) - f(d - a)));
      below[cell] = triangle(f(f(c - a) - f(d - a)), -f(c - a));
    }
  }
  const decoded = new Float32Array(elevations.length * 3);
  for (let y = 0; y <= height; y++) {
    for (let x = 0; x <= width; x++) {
      // The last cell to write each shared corner fixes the accumulation order.
      const cellX = Math.min(x, width - 1), cellY = Math.min(y, height - 1), cell = cellY * width + cellX;
      const corner = cornerAt(x, y, cellX, cellY);
      const normal = [0, 1, 2].map(axis => cornerWeight(above[cell], below[cell], corner, axis));
      for (const [dx, dy] of NEIGHBORS) {
        const i = cellX + dx, j = cellY + dy;
        if (i < 0 || j < 0 || i >= width || j >= height || x < i || y < j || x > i + 1 || y > j + 1) continue;
        const n = j * width + i, c = cornerAt(x, y, i, j);
        for (let k = 0; k < 3; k++) normal[k] = f(normal[k] + cornerWeight(above[n], below[n], c, k));
      }
      const [nx, ny, nz] = normal;
      const length = Math.sqrt(nx * nx + nz * nz + ny * ny);
      const sx = packedComponent(corner === 0 ? nx / length : f(nx / length)) * (1 / SCALE);
      const sy = packedComponent(f(ny / length)) * (1 / SCALE);
      const at = (y * stride + x) * 3;
      decoded[at] = sx;
      decoded[at + 1] = Math.sqrt(1 - (sx * sx + sy * sy));
      decoded[at + 2] = sy;
    }
  }
  return decoded;
}
