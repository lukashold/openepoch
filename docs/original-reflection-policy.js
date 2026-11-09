// Source behavior from native-reflection-camera224. Device, camera-ray and
// scene boundaries are injected so this module can run without a graphics API.
const f32 = Math.fround;
const REFLECTION_FLAG = 2;
const PIP_FLAG = 1;
const MAIN = 0;
const PIP = 1;
const FULL_VIEWPORT = Object.freeze([0, 1, 1, 0]);
const LOOK_PLANE_Z = f32(0.02);
const MIN_LOOK_DISTANCE_SQUARED = f32(1e-10);
const MIN_NORMAL_LENGTH = f32(1e-6);
const NEGATIVE_Z = [0, 0, -1];
const TARGETS = Object.freeze([
  Object.freeze({id: MAIN, width: 512, height: 512}),
  Object.freeze({id: PIP, width: 128, height: 128}),
]);

export const REFLECTION_NEAR_DEFAULTS = Object.freeze({
  nearMinimum: f32(0.1),
  maxDepthRatio: 10000,
});

const countSlot = flags => flags & PIP_FLAG;
const cameraSlot = (camera, canonicalCamera) => camera === canonicalCamera ? MAIN : PIP;

/** The resource factory returns a resource handle or null on unsupported devices. */
export function cachedReflectionTargetProbe(allocate, release) {
  let attempted = false;
  let supported = false;
  return () => {
    if (!attempted) {
      const resource = allocate(32, 32);
      supported = resource != null;
      if (supported) release(resource, 1);
      attempted = true;
    }
    return supported;
  };
}

export function reflectionClipStateAllowed(enableResult, availablePlanes) {
  return enableResult === 0 && (availablePlanes | 0) > 0;
}

/** Ceiling and requested mode are separate source fields. */
export function reflectionModeCeiling({deviceAvailable, targetSupported, shaderVersion}) {
  if (!deviceAvailable || !targetSupported) return -1;
  const major = (shaderVersion >>> 8) & 255;
  const minor = shaderVersion & 255;
  return +(major > 1 || (major === 1 && minor >= 4));
}

export function reflectionTargetMode(requestedSetting, ceiling) {
  return Math.min(requestedSetting === 2 ? 1 : 0, ceiling);
}

export function reflectionRoute(packet, canonicalCamera) {
  return {cloneSlot: countSlot(packet.flags), target: TARGETS[cameraSlot(packet.camera, canonicalCamera)]};
}

/** Matches the native comparison, including its unordered case on raw float data. */
export function reflectionSphereVisible(sphere, planes) {
  const [x, y, z, radius] = sphere;
  if (planes.length !== 6) throw new RangeError('A reflection camera requires six planes');
  for (const [nx, ny, nz, offset] of planes) {
    if ((nx * x + nz * z) + ny * y - offset <= -radius) return false;
  }
  return true;
}

/** Refresh one camera's water-body list. The other list is carried by identity. */
export function refreshReflectionBodies({bodies, camera, canonicalCamera, planes, previous}) {
  const visible = bodies.filter(body =>
    body.resourceReady && body.surfaceReady && reflectionSphereVisible(body.sphere, planes));
  const mainBodies = camera === canonicalCamera ? visible : previous?.mainBodies ?? [];
  const pipBodies = camera === canonicalCamera ? previous?.pipBodies ?? [] : visible;
  return {
    bodyCount: bodies.length,
    mainBodies,
    pipBodies,
    mainVisibleCount: mainBodies.length,
    pipVisibleCount: pipBodies.length,
  };
}

/** targetSupported() is the cached backend probe, called only at the source gate. */
export function reflectionAdmitted(packet, state) {
  const visibleCount = countSlot(packet.flags) ? state.pipVisibleCount : state.mainVisibleCount;
  return Boolean(state.bodyCount && visibleCount && state.deviceAvailable &&
    state.targetSupported() && state.qualitySetting && state.clipStateAllowed);
}

/** Owns two persistent reflected packet/camera objects, as the source slots do. */
export function createReflectionPacketCache(buildCamera) {
  const packets = [null, null];
  return source => {
    const index = countSlot(source.flags);
    const refreshed = buildCamera(source.camera);
    if (!packets[index]) packets[index] = {flags: (source.flags | REFLECTION_FLAG) >>> 0, camera: {}};
    Object.assign(packets[index].camera, {preparationWord: undefined}, refreshed);
    return packets[index];
  };
}

/** The array is newly allocated and the original packet identities/order are retained. */
export function prepareReflectionViews(sources, state, refreshClone) {
  const prefix = [];
  for (const source of sources) {
    if (!reflectionAdmitted(source, state)) continue;
    const reflected = refreshClone(source);
    if (countSlot(source.flags)) reflected.flags = (reflected.flags | PIP_FLAG) >>> 0;
    reflected.camera.preparationWord = 3;
    prefix.push(reflected);
  }
  return prefix.concat(sources);
}

/** Mirrors the +C4 argument order and preserves the supplied field references. */
export function reflectionViewArgs(camera) {
  return [camera.eye, camera.forward, camera.up, camera.right, camera.frustum, camera.viewport];
}

/** Run synchronous backend work in source order. Unbind even if scene begin fails. */
export function captureReflectionViews(sources, state, {canonicalCamera, restoreCamera, refreshClone, backend}) {
  const captured = [];
  if (!state.deviceAvailable || !state.targetSupported()) return captured;
  for (const source of sources) {
    if (!reflectionAdmitted(source, state)) continue;
    const reflected = refreshClone(source);
    const {target} = reflectionRoute(source, canonicalCamera);
    try {
      backend.bindTarget(target);
      if (!backend.beginScene(0)) continue;
      try {
        backend.prepareCamera(reflected.camera, 7);
        backend.setView(...reflectionViewArgs(reflected.camera));
        backend.drawScene(reflected, target);
        captured.push({packet: reflected, target});
      } finally {
        try {
          backend.setView(...reflectionViewArgs(restoreCamera));
        } finally {
          backend.endScene();
        }
      }
    } finally {
      backend.bindTarget(null, target);
    }
  }
  return captured;
}

const finiteF32 = value => typeof value === 'number' && Number.isFinite(f32(value));
const vector3 = value => value?.length === 3 && Array.from(value).every(finiteF32);
const vectorCopy = value => Array.from(value, f32);
const length3 = ([x, y, z]) => Math.sqrt((x * x + y * y) + z * z);
const cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
].map(f32);
function normalizeCross([x, y, z], length) {
  const inverse = 1 / f32(length);
  const stored = f32(inverse);
  return [f32(x * stored), f32(y * stored), f32(z * inverse)];
}

function lookAt(eye, target, source) {
  const difference = eye.map((value, index) => value - target[index]);
  const [x, y, z] = difference.map(f32);
  const squared = (difference[2] * z + y * y) + x * x;
  if (squared >= MIN_LOOK_DISTANCE_SQUARED) {
    const length = Math.sqrt(squared);
    const forward = length > MIN_NORMAL_LENGTH
      ? [x, y, z].map(value => -f32(value * (1 / length))) : [0, 0, 0];
    const lateral = cross(forward, NEGATIVE_Z);
    const lateralLength = length3(lateral);
    if (lateralLength > MIN_NORMAL_LENGTH) {
      const right = normalizeCross(lateral, lateralLength);
      const upward = cross(right, forward);
      const upwardLength = length3(upward);
      if (upwardLength > MIN_NORMAL_LENGTH) return {forward, up: normalizeCross(upward, upwardLength), right};
    }
  }
  // Failed original look-at retains the rotation copied from the direct camera.
  return {forward: vectorCopy(source.forward), up: vectorCopy(source.up), right: vectorCopy(source.right)};
}

/** Native coordinates, Z up. source.eye is local and world eye for a parent-free camera. */
export function cloneReflectionCamera(source, {
  screenWidth, screenHeight, screenRay,
  nearMinimum = REFLECTION_NEAR_DEFAULTS.nearMinimum,
  maxDepthRatio = REFLECTION_NEAR_DEFAULTS.maxDepthRatio,
}) {
  const {eye, forward, up, right, frustum} = source;
  const {left, right: rightEdge, top, bottom, near, far, orthographic} = frustum;
  if (![eye, forward, up, right].every(vector3) ||
      ![left, rightEdge, top, bottom, near, far, nearMinimum, maxDepthRatio].every(finiteF32) ||
      f32(maxDepthRatio) <= 0 || !Number.isInteger(screenWidth) || screenWidth <= 0 ||
      !Number.isInteger(screenHeight) || screenHeight <= 0 || typeof screenRay !== 'function') {
    throw new RangeError('Invalid reflection clone inputs');
  }
  const nearResult = Math.max(f32(near), f32(f32(far) / f32(maxDepthRatio)), f32(nearMinimum));
  const projection = {
    left: f32(left), right: f32(rightEdge), top: f32(top), bottom: f32(bottom),
    near: nearResult, far: f32(far), orthographic: Boolean(orthographic),
  };
  const pixel = [Math.trunc(f32(screenWidth * 0.5)), Math.trunc(f32(screenHeight * 0.5))];
  const initialCamera = {
    eye: vectorCopy(eye), forward: vectorCopy(forward), up: vectorCopy(up), right: vectorCopy(right),
    frustum: projection, viewport: FULL_VIEWPORT,
  };
  const ray = screenRay(initialCamera, ...pixel);
  if (!vector3(ray?.origin) || !vector3(ray?.direction) || f32(ray.direction[2]) === 0) {
    throw new RangeError('Reflection center ray does not meet the look plane');
  }
  const origin = vectorCopy(ray.origin);
  const direction = vectorCopy(ray.direction);
  const distance = -(1 / direction[2]) * (origin[2] - LOOK_PLANE_Z);
  const target = origin.map((value, axis) => {
    const delta = distance * direction[axis];
    return f32(value + (axis === 0 ? delta : f32(delta)));
  });
  if (!target.every(Number.isFinite)) throw new RangeError('Nonfinite reflection look point');
  const reflectedEye = [initialCamera.eye[0], initialCamera.eye[1], -initialCamera.eye[2]];
  const orientation = lookAt(reflectedEye, target, initialCamera);
  return {
    ...initialCamera, ...orientation, eye: reflectedEye, target, pixel,
    preparationWord: source.preparationWord,
  };
}
