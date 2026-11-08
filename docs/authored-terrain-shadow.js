import {chop32, createTerrainRay} from './authored-terrain-ray.js';
import {authoredTerrainNormals} from './authored-terrain-normals.js';

export const SHADOW_RESOLUTION = 128;
export const SHADOW_BLOCK_SPAN = 25;
const HEIGHT_OFFSET = Math.fround(.01);
const COORDINATE_BIAS = 103079215104;
const rawCoordinate = new DataView(new ArrayBuffer(8));

function gridIndex(value) {
  rawCoordinate.setFloat64(0, COORDINATE_BIAS + value, true);
  return rawCoordinate.getInt32(2, true);
}

// The outermost blocks keep their full 25-unit span. 0070FC10 clamps the
// individual corner reads and preserves the source sample's XY coordinates.
function terrainHeight(elevations, width, height, x, y, store) {
  const i = gridIndex(x), j = gridIndex(y);
  const u = store(x - i), v = store(y - j);
  const at = (ix, iy) => elevations[Math.min(height, Math.max(0, iy)) * (width + 1) + Math.min(width, Math.max(0, ix))];
  const a = at(i, j), diagonal = store(at(i + 1, j + 1) - a);
  return u <= v ? a + u * diagonal + (v - u) * (at(i, j + 1) - a)
    : a + v * diagonal + (u - v) * (at(i + 1, j) - a);
}

function sampleNormal(geometry, normals, originX, originY, x, y, store, target) {
  const {width, height, elevations} = geometry;
  const right = Math.min(width, originX + SHADOW_BLOCK_SPAN);
  const bottom = Math.min(height, originY + SHADOW_BLOCK_SPAN);
  const index = (ix, iy) => Math.min(bottom, Math.max(originY, iy)) * (width + 1) + Math.min(right, Math.max(originX, ix));
  const ix = originX + gridIndex(x), iy = originY + gridIndex(y);
  const u = store(x - Math.floor(x)), v = store(y - Math.floor(y));
  const a = index(ix, iy), d = index(ix + 1, iy + 1);
  const c = u > v ? index(ix + 1, iy) : index(ix, iy + 1);
  const diagonalWeight = Math.min(u, v), sideWeight = store(Math.abs(u - v));
  const component = (point, k) => elevations[point] < Math.fround(.05) ? Number(k === 1) : normals[point * 3 + k];
  for (let k = 0; k < 3; k++) {
    const first = component(a, k);
    const diagonal = store(store(component(d, k) - first) * diagonalWeight);
    const side = store(store(component(c, k) - first) * sideWeight);
    target[k] = store(store(first + diagonal) + side);
  }
  const [nx, nz, ny] = target;
  const length = Math.sqrt(nx * nx + ny * ny + nz * nz);
  if (length !== 0) {
    const inverse = 1 / length;
    for (let k = 0; k < 3; k++) target[k] = store(target[k] * inverse);
  }
}

function visibilityByte(occluded, x, y) {
  const limit = SHADOW_RESOLUTION - 1;
  let weight = 0;
  for (let dy = -1; dy <= 1; dy++) {
    const row = Math.max(0, Math.min(limit, y + dy)) * SHADOW_RESOLUTION;
    for (let dx = -1; dx <= 1; dx++) {
      weight += occluded[row + Math.max(0, Math.min(limit, x + dx))] * (dx === 0 ? 2 : 1) * (dy === 0 ? 2 : 1);
    }
  }
  return Math.trunc((1 - weight / 16) * 255);
}

export function createTerrainShadow(geometry, {chop = false} = {}) {
  const {width, height, elevations} = geometry;
  const ray = createTerrainRay(width, height, elevations, {chop});
  const store = chop ? chop32 : Math.fround;
  const positions = new Float32Array(SHADOW_RESOLUTION);
  const step = store(SHADOW_BLOCK_SPAN / (SHADOW_RESOLUTION - 1));
  for (let i = 1; i < positions.length; i++) positions[i] = store(positions[i - 1] + step);
  function block(x, y, direction, onSample) {
    // direction is the installed world's light vector in source Z-up order.
    const towardsSun = direction.map(value => store(-value * 1024));
    const start = [0, 0, 0], end = [0, 0, 0];
    const occluded = new Uint8Array(SHADOW_RESOLUTION ** 2);
    for (let j = 0, index = 0; j < positions.length; j++) {
      start[1] = store(y + positions[j]);
      end[1] = store(start[1] + towardsSun[1]);
      for (let i = 0; i < positions.length; i++, index++) {
        start[0] = store(x + positions[i]);
        start[2] = store(Math.max(0, terrainHeight(elevations, width, height, start[0], start[1], store)) + HEIGHT_OFFSET);
        end[0] = store(start[0] + towardsSun[0]);
        end[2] = store(start[2] + towardsSun[2]);
        occluded[index] = ray.firstLeaf(start, end) >= 0 ? 1 : 0;
        if (onSample) onSample(index, start, end);
      }
    }
    return occluded;
  }
  return {
    blocksX: Math.ceil(width / SHADOW_BLOCK_SPAN),
    blocksY: Math.ceil(height / SHADOW_BLOCK_SPAN),
    block,
    lightmapBlock(x, y, direction, normals) {
      if (!(normals instanceof Float32Array) || normals.length !== elevations.length * 3) {
        throw new RangeError('Invalid authored lightmap normal field');
      }
      const occluded = block(x, y, direction);
      const light = new Uint8Array(occluded.length);
      const normal = [0, 0, 0];
      for (let j = 0, index = 0; j < positions.length; j++) {
        for (let i = 0; i < positions.length; i++, index++) {
          sampleNormal(geometry, normals, x, y, positions[i], positions[j], store, normal);
          const dot = normal[2] * -direction[1] + normal[1] * -direction[2] + normal[0] * -direction[0];
          light[index] = Math.max(0, Math.trunc(dot * visibilityByte(occluded, i, j)));
        }
      }
      return {occluded, light};
    },
  };
}

export function buildAuthoredTerrainLightmap(geometry, direction) {
  const normals = authoredTerrainNormals(geometry);
  const shadow = createTerrainShadow(geometry);
  const layers = shadow.blocksX * shadow.blocksY, samples = SHADOW_RESOLUTION ** 2;
  const data = new Uint8Array(layers * samples);
  for (let y = 0, layer = 0; y < shadow.blocksY; y++) {
    for (let x = 0; x < shadow.blocksX; x++, layer++) {
      const block = shadow.lightmapBlock(x * SHADOW_BLOCK_SPAN, y * SHADOW_BLOCK_SPAN, direction, normals);
      data.set(block.light, layer * samples);
    }
  }
  return {data, layers, blocksX: shadow.blocksX};
}
