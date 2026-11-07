import {ORIGINAL_CAMERA_PROFILE, originalCameraRaySegment, updateOriginalCamera} from './original-camera.js';
import {createOriginalPipCamera, ORIGINAL_PIP_CAMERA_PROFILE} from './original-pip-camera.js';
import {authoredMinimapPoint, authoredTerrainIndices, cropAuthoredMinimap} from './world-view.js';

export function activeValleyGeometry(wasm, prepared = false) {
  const info = prepared ? wasm.valley37_info : wasm.valley37_view_info;
  if (!info) return null;
  const [width, height, backing] = [0, 1, 2].map(info);
  if (width === 0 && height === 0 && backing === 0) return null;
  if (width !== 115 || height !== 71 || backing !== 256) throw new Error('Unexpected authenticated Valley dimensions');
  const corners = (width + 1) * (height + 1);
  const view = (type, field, length) => {
    const ptr = prepared ? wasm.valley37_source_ptr(field < 3 ? field : field + 2) : wasm.valley37_view_ptr(field);
    if (!ptr) throw new Error('Missing authenticated Valley terrain field');
    return new type(wasm.memory.buffer, ptr, length).slice();
  };
  const elevations = view(Float32Array, 0, corners);
  const materials = view(Uint16Array, 1, corners);
  const tints = view(Uint32Array, 2, corners);
  const patterns = view(Uint32Array, 3, width * height);
  if (patterns.some(word => (word >>> 16 & 255) !== 0)) throw new Error('Unexpected Valley terrain diagonal');
  const sourceArchiveSha256 = Array.from(view(Uint8Array, 4, 32), n => n.toString(16).padStart(2, '0')).join('');
  return {width, height, backing, elevations, materials, tints, sourceArchiveSha256};
}

export function authoredTerrainMesh(geometry) {
  const {width, height, elevations} = geometry;
  const row = width + 1;
  const vertices = new Float32Array(elevations.length * 6);
  const sample = (x, y) => elevations[y * row + x];
  for (let y = 0, at = 0; y <= height; y++) {
    for (let x = 0; x <= width; x++, at += 6) {
      const dx = sample(Math.max(0, x - 1), y) - sample(Math.min(width, x + 1), y);
      const dz = sample(x, Math.max(0, y - 1)) - sample(x, Math.min(height, y + 1));
      const norm = Math.hypot(dx, 2, dz);
      vertices.set([x, sample(x, y), y, dx / norm, 2 / norm, dz / norm], at);
    }
  }
  // The compact vertex row has width+1 elements. Minimap and fog keep their
  // independent padded simulation stride.
  return {vertices, indices: authoredTerrainIndices(width, height, width)};
}

export function initialValleyViews(wasm, viewport, seat) {
  if (wasm.valley37_view_info?.(0) !== 115) return null;
  const [x, y] = seat === undefined ? [wasm.start_x(), wasm.start_y()]
    : [wasm.valley37_view_focus(seat, 0), wasm.valley37_view_focus(seat, 1)];
  if (![x, y].every(Number.isFinite)) throw new RangeError('Invalid active Valley seat');
  const primaryPose = {x, y, heading: 0, nativeFraction: Math.fround(.2)};
  const pipState = createOriginalPipCamera({...primaryPose, nativeFraction: Math.fround(.7)});
  const groundAt = (x, y) => wasm.valley37_view_ground(x, y);
  return {focus: [x, y], primaryPose, pipState,
    primary: updateOriginalCamera(primaryPose, {}, ...viewport.primary, groundAt, ORIGINAL_CAMERA_PROFILE),
    pip: updateOriginalCamera(pipState.pose(), {}, ...viewport.pip, groundAt, ORIGINAL_PIP_CAMERA_PROFILE)};
}

export function fitAuthoredMinimap(rect, {width, height}) {
  if (![rect.left, rect.top, rect.width, rect.height].every(Number.isFinite) || rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  const scale = Math.min(rect.width / width, rect.height / height);
  const w = width * scale, h = height * scale;
  return {left: rect.left + (rect.width - w) / 2, top: rect.top + (rect.height - h) / 2, width: w, height: h};
}

export function valleyMinimapSnapshot(wasm, {width, height, backing}) {
  const rgba = new Uint8Array(wasm.memory.buffer, wasm.minimap(), backing ** 2 * 4);
  return cropAuthoredMinimap(rgba, width, height, backing);
}

export function valleyMinimapPoint(geometry, screenX, screenY, rect) {
  const image = fitAuthoredMinimap(rect, geometry);
  return image ? authoredMinimapPoint(screenX, screenY, image, geometry.width, geometry.height) : null;
}

// Intersect only the cells traversed by the normal camera ray. Each cell has
// the source a-d diagonal used by both the world geometry and the renderer.
export function pickValleyGround(geometry, view, screenX, screenY, viewportWidth, viewportHeight) {
  if (![screenX, screenY, viewportWidth, viewportHeight].every(Number.isFinite) ||
      screenX < 0 || screenY < 0 || screenX >= viewportWidth || screenY >= viewportHeight) return null;
  const {width, height, elevations} = geometry;
  const {origin: [ox, oy, oz], direction: [dx, dy, dz], near, far} =
    originalCameraRaySegment(view, screenX, screenY, viewportWidth, viewportHeight);
  const clip = (start, end, origin, direction, limit) => {
    if (direction === 0) return origin >= 0 && origin <= limit ? [start, end] : [Infinity, -Infinity];
    const a = -origin / direction, b = (limit - origin) / direction;
    return [Math.max(start, Math.min(a, b)), Math.min(end, Math.max(a, b))];
  };
  let [first, end] = clip(near, far, ox, dx, width);
  [first, end] = clip(first, end, oz, dz, height);
  if (first > end) return null;

  const stepX = Math.sign(dx), stepY = Math.sign(dz);
  const initial = (p, step, limit) => Math.min(limit - 1, Math.max(0, step < 0 ? Math.ceil(p) - 1 : Math.floor(p)));
  let x = initial(ox + dx * first, stepX, width), y = initial(oz + dz * first, stepY, height);
  const nextX = () => dx === 0 ? Infinity : (x + Number(stepX > 0) - ox) / dx;
  const nextY = () => dz === 0 ? Infinity : (y + Number(stepY > 0) - oz) / dz;
  const EPS = 1e-8;
  const row = width + 1;
  while (x >= 0 && x < width && y >= 0 && y < height && first <= end) {
    const at = y * row + x;
    const [a, b, c, d] = [at, at + 1, at + row, at + row + 1].map(index => elevations[index]);
    const tx = nextX(), ty = nextY(), stop = Math.min(tx, ty, end);
    let hit;
    for (const [gx, gy, below] of [[d - c, c - a, false], [b - a, d - b, true]]) {
      const delta = dy - gx * dx - gy * dz;
      if (Math.abs(delta) < EPS) continue;
      const distance = (a + gx * (ox - x) + gy * (oz - y) - oy) / delta;
      if (distance < first - EPS || distance > stop + EPS || (hit && distance >= hit.distance)) continue;
      const px = ox + distance * dx, py = oz + distance * dz;
      if (px < 0 || py < 0 || px >= width || py >= height) continue;
      const side = (px - x) - (py - y);
      if (below ? side < -EPS : side > EPS) continue;
      hit = {x: px, y: py, elevation: oy + distance * dy, distance};
    }
    if (hit) return hit;
    if (stop === end) break;
    if (tx <= ty) x += stepX;
    if (ty <= tx) y += stepY;
    first = stop;
  }
  return null;
}
