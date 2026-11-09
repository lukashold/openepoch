const normalScale = 1 / 32767;
const terrainBoundsFloor = Math.fround(.05);

// Unmodified terrain cells use the original shared unit lattice. A caller must
// provide an already validated rectangle and its source map height/normal words.
export function buildStockBlock({ x, y, width, height, mapWidth, heights, normalWords }) {
  const columns = width + 1;
  const vertices = new Float32Array(columns * (height + 1) * 6);
  const indices = new Uint16Array(width * height * 6);
  const spans = new Uint32Array(width * height * 3);

  for (let row = 0; row <= height; ++row) {
    for (let col = 0; col <= width; ++col) {
      const vertex = row * columns + col;
      const sample = (y + row) * mapWidth + x + col;
      const nx = normalWords[2 * sample] * normalScale;
      const ny = normalWords[2 * sample + 1] * normalScale;
      const nz = Math.sqrt(1 - (nx * nx + ny * ny));
      vertices.set([x + col, y + row, heights[sample], nx, ny, nz], vertex * 6);
    }
  }

  for (let row = 0; row < height; ++row) {
    for (let col = 0; col < width; ++col) {
      const leaf = row * width + col;
      const a = row * columns + col;
      const b = a + 1;
      const c = a + columns;
      const d = c + 1;
      indices.set([a, b, d, a, d, c], leaf * 6);
      spans.set([a | (d << 16), leaf * 6, 6], leaf * 3);
    }
  }
  return { x, y, width, height, vertices, indices, spans };
}

export function stockLeafBounds(block, leaf) {
  const { vertices, width, x, y } = block;
  const col = leaf % width;
  const row = Math.floor(leaf / width);
  const a = row * (width + 1) + col;
  const c = a + width + 1;
  const heights = [vertices[a * 6 + 2], vertices[(a + 1) * 6 + 2],
    vertices[c * 6 + 2], vertices[(c + 1) * 6 + 2]];
  return [x + col, y + row, Math.min(...heights),
    x + col + 1, y + row + 1, Math.max(terrainBoundsFloor, ...heights)];
}

export function assembleLeafIndices(block, leaves) {
  let length = 0;
  for (const leaf of leaves) length += block.spans[leaf * 3 + 2];
  const indices = new Uint16Array(length);
  let offset = 0;
  for (const leaf of leaves) {
    const first = block.spans[leaf * 3 + 1];
    const count = block.spans[leaf * 3 + 2];
    indices.set(block.indices.subarray(first, first + count), offset);
    offset += count;
  }
  return indices;
}
