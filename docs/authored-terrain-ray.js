// Source terrain segment broadphase and ordinary triangle leaf intersection.
const f32 = Math.fround;
const DET_EPSILON = f32(1e-6);
const float = new Float32Array(1);
const word = new Uint32Array(float.buffer);

export function chop32(number) {
  float[0] = number;
  if (Math.abs(float[0]) > Math.abs(number)) word[0]--;
  return float[0];
}

function cellRay(x, y, corners, start, direction, store) {
  const [a, b, c, d] = corners;
  const [vx, vy, vz] = direction;
  for (let upper = 0; upper < 2; upper++) {
    // The source shares the a–d diagonal and tests the triangle below it first.
    const dx = upper ? b - a : d - c;
    const dy = upper ? d - b : c - a;
    const denominator = vz - dx * vx - dy * vy;
    if (Math.abs(store(denominator)) < DET_EPSILON) continue;
    const distance = (a + dx * (start[0] - x) + dy * (start[1] - y) - start[2]) / denominator;
    if (!(store(distance) > 0)) continue;
    const u = start[0] + vx * distance - x, v = start[1] + vy * distance - y;
    if (u >= 0 && v >= 0 && u <= 1 && v <= 1 && (upper ? u >= v : v >= u)) return true;
  }
  return false;
}

function boxSegment(start, end, box) {
  let near = 0, far = 1;
  for (let axis = 0; axis < 3; axis++) {
    const a = start[axis], b = end[axis];
    const low = box[axis], high = box[axis + 3];
    if (a === b) {
      if (a < low || a > high) return false;
    } else {
      const x = (low - a) / (b - a), y = (high - a) / (b - a);
      near = Math.max(near, Math.min(x, y));
      far = Math.min(far, Math.max(x, y));
      if (near > far) return false;
    }
  }
  return true;
}

export function createTerrainRay(width, height, elevations, {chop = false} = {}) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 || width > 1024 || height > 1024 ||
      !(elevations instanceof Float32Array) || elevations.length !== (width + 1) * (height + 1) ||
      !elevations.every(Number.isFinite)) throw new RangeError('Invalid base terrain ray heightfield');
  const stride = width + 1;
  const store = chop ? chop32 : f32;
  let branches = 0, leaves = 0;
  function build(x, y, span) {
    if (span === 1) {
      const offset = y * stride + x;
      const corners = [elevations[offset], elevations[offset + 1], elevations[offset + stride], elevations[offset + stride + 1]];
      leaves++;
      return {x, y, id: y * width + x, corners, box: [x, y, Math.min(...corners), x + 1, y + 1, Math.max(...corners)]};
    }
    branches++;
    const half = span / 2;
    const children = [[x, y], [x + half, y], [x, y + half], [x + half, y + half]]
      .filter(([cx, cy]) => cx < width && cy < height).map(([cx, cy]) => build(cx, cy, half));
    const low = Math.min(...children.map(child => child.box[2]));
    const high = Math.max(...children.map(child => child.box[5]));
    return {children, box: [x, y, low, x + span, y + span, high]};
  }
  const span = 2 ** Math.ceil(Math.log2(Math.max(width, height)));
  const root = build(0, 0, span);
  function visit(node, start, end, direction, onLeaf) {
    if (!boxSegment(start, end, node.box)) return -1;
    if (node.children) {
      for (const child of node.children) {
        const first = visit(child, start, end, direction, onLeaf);
        if (first !== -1) return first;
      }
      return -1;
    }
    if (onLeaf) onLeaf(node.id);
    // The segment bounds select leaves. The leaf's own positive ray parameter
    // can extend beyond the end, as in the installed engine.
    return cellRay(node.x, node.y, node.corners, start, direction, store) ? node.id : -1;
  }
  return {
    branches, leaves, span,
    firstLeaf(start, end, onLeaf) {
      const direction = [store(end[0] - start[0]), store(end[1] - start[1]), store(end[2] - start[2])];
      return visit(root, start, end, direction, onLeaf);
    },
  };
}
