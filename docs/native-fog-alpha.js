const low = Object.freeze({ address: 'clamp', magnification: 'linear', minification: 'point', mip: 'point', maxAnisotropy: 1 });
const high = Object.freeze({ address: 'clamp', magnification: 'anisotropic', minification: 'anisotropic', mip: 'anisotropic', maxAnisotropy: 4 });

/** The original DX8 wrapper's stage 0 requests. The generated texture has one mip. */
export function nativeTerrainFogSampler(quality) {
  return quality >= 2 ? high : low;
}

/**
 * Original one-level fog alpha backing. Width and height count tiles.
 * Input uses the portable 0/128/255 visibility states in original raster order.
 */
export function createNativeTerrainFogAlpha({ width, height, stride = width }) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || !Number.isSafeInteger(stride) ||
      width < 1 || height < 1 || width > 16384 || height > 16384 || stride < width ||
      !Number.isSafeInteger((height - 1) * stride + width)) {
    throw new RangeError('Invalid native fog tile extent');
  }
  const size = 2 ** Math.ceil(Math.log2(Math.max(width, height)));
  const pixels = new Uint8Array(size * size).fill(255);
  const sourceLength = (height - 1) * stride + width;

  function update(visibility, { fullyVisible = false } = {}) {
    if (!(visibility instanceof Uint8Array) || visibility.length < sourceLength) {
      throw new RangeError('Incomplete native fog visibility raster');
    }
    for (let y = 1; y < height - 1; ++y) {
      const source = y * stride;
      const target = y * size;
      for (let x = 1; x < width - 1; ++x) {
        const value = visibility[source + x];
        pixels[target + x] = fullyVisible || value === 255 ? 0 : value === 128 ? 128 : 255;
      }
    }
    return pixels;
  }

  return { size, pixels, update };
}
