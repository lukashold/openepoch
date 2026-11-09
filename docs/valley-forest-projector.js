import {chop32} from './authored-terrain-ray.js';

const f = Math.fround;

// 00625C70. The source stores direction and extent as F32, then uses
// the unrounded texture-coefficient products to construct the offsets.
export function uprightForestProjector(x, y, [lx, ly, lz], bounds, radiusScale, heightScale, {chop = false} = {}) {
  const store = chop ? chop32 : f;
  const width = Math.max(bounds[3] - bounds[0], store(bounds[4] - bounds[1]));
  const radius = store(width * radiusScale * .5);
  const magnitude = Math.sqrt(ly * ly + lx * lx);
  const length = store(((-1 / lz) * magnitude) * ((bounds[5] - bounds[2]) * heightScale));
  if (!(radius > 0 && length > 0 && magnitude > 0)) throw new RangeError('Invalid upright forest shadow projector');
  const inverse = 1 / magnitude;
  const a = store(lx * inverse), b = store(ly * inverse);
  const planes = [
    [a, b, 0, store(a * x + b * y)],
    [-a, -b, 0, store(-a * x - b * y - length)],
    [-b, a, 0, store(-b * x + a * y - radius)],
    [b, -a, 0, store(b * x - a * y - radius)],
  ];
  const across = .5 / radius, along = 1 / length;
  const ux = b * across, uy = -a * across, vx = -a * along, vy = -b * along;
  const u = [store(ux), store(uy), 0, store(ux * x + uy * y - .5)];
  const v = [store(vx), store(vy), 0, store(vx * x + vy * y - 1)];
  return {planes, u, v, x, y, radius, length};
}

export function projectedTerrainTiles(width, height, projection) {
  const {planes, x, y, radius, length} = projection;
  const [a, b] = planes[0], reachX = a * length, reachY = b * length;
  // The expanded rectangle only bounds iteration. The strict source plane
  // test decides whether to submit each complete receiver cell.
  const startX = Math.max(0, Math.floor(x + Math.min(0, reachX) - Math.abs(b) * radius) - 1);
  const startY = Math.max(0, Math.floor(y + Math.min(0, reachY) - Math.abs(a) * radius) - 1);
  const endX = Math.min(width, Math.ceil(x + Math.max(0, reachX) + Math.abs(b) * radius) + 1);
  const endY = Math.min(height, Math.ceil(y + Math.max(0, reachY) + Math.abs(a) * radius) + 1);
  const tiles = [];
  for (let iy = startY; iy < endY; iy++) {
    for (let ix = startX; ix < endX; ix++) {
      if (planes.every(([nx, ny,, d]) => nx * (ix + Number(nx > 0)) + ny * (iy + Number(ny > 0)) - d > 0)) {
        tiles.push(iy * width + ix);
      }
    }
  }
  return tiles;
}
