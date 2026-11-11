// Installed UP15 UnitForest rotation for unsigned 1/128 source coordinates.
const octaves = [0x3c6ef35f, 0x47502932, 0xd1ccf6e9];
const cornerScale = Math.fround(2 / 32767);
const byteScale = Math.fround(1 / 255);
const tau = Math.fround(Math.PI * 2);
const sourceBias = 103079215104;

function corner(seed, x, y) {
  let word = (seed + (((y << 9) + x) << 7)) >>> 0;
  const rounds = (x + y) % 4 + 3;
  for (let i = 0; i < rounds; ++i) word = (Math.imul(word, 1664525) + 1013904223) >>> 0;
  return Math.fround(((word >>> 16) & 32767) * cornerScale - 1);
}

function octave(seed, sx, sy) {
  const x = Math.floor(sx + sourceBias - sourceBias);
  const y = Math.floor(sy + sourceBias - sourceBias);
  const fx = sx - x, fy = sy - y;
  const a = corner(seed, x, y), b = corner(seed, x + 1, y);
  const c = corner(seed, x, y + 1), d = corner(seed, x + 1, y + 1);
  const low = Math.fround((b - a) * fx + a);
  const high = (d - c) * fx + c;
  return (high - low) * fy + low;
}

export function sourceForestYawByte(xWord, yWord) {
  if (![xWord, yWord].every(word => Number.isInteger(word) && word >= 0 && word <= 65535)) {
    throw new RangeError('Invalid UnitForest coordinates');
  }
  let sum = 0;
  for (let i = 0; i < octaves.length; ++i) {
    const scale = (1 << i) / 128;
    sum = Math.fround(sum + octave(octaves[i], xWord * scale, yWord * scale) / (1 << i));
  }
  return Math.trunc(sum * 255) & 255;
}

export function sourceForestYaw(xWord, yWord) {
  return Math.fround(sourceForestYawByte(xWord, yWord) * byteScale * tau);
}
