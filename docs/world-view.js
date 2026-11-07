// The browser keeps the backing texture stride while drawing only the source
// play area. Native Valley's zero-pattern cells use the a-d height diagonal.
export function authoredTerrainIndices(width, height, backing) {
  if (![width, height, backing].every(Number.isInteger) || width < 1 || height < 1 ||
      width > backing || height > backing || backing > 256) throw new RangeError('Invalid authored terrain envelope');
  const indices = new Uint32Array(6 * width * height);
  const row = backing + 1;
  for (let y = 0, i = 0; y < height; y++) {
    for (let x = 0; x < width; x++, i += 6) {
      const a = y * row + x, b = a + 1, c = a + row, d = c + 1;
      indices.set([a, c, d, a, d, b], i);
    }
  }
  return indices;
}

export function cropAuthoredMinimap(rgba, width, height, backing) {
  if (!(rgba instanceof Uint8Array || rgba instanceof Uint8ClampedArray) || rgba.length !== backing * backing * 4 ||
      ![width, height, backing].every(Number.isInteger) || width < 1 || height < 1 || width > backing || height > backing) {
    throw new RangeError('Invalid authored minimap backing texture');
  }
  const image = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) image.set(rgba.subarray(y * backing * 4, (y * backing + width) * 4), y * width * 4);
  return image;
}

export function authoredMinimapPoint(clientX, clientY, rect, width, height) {
  if (![clientX, clientY, rect.left, rect.top, rect.width, rect.height, width, height].every(Number.isFinite) ||
      rect.width <= 0 || rect.height <= 0 || width <= 0 || height <= 0) return null;
  const x = (clientX - rect.left) / rect.width;
  const y = (clientY - rect.top) / rect.height;
  return x >= 0 && x < 1 && y >= 0 && y < 1 ? [x * width, y * height] : null;
}
