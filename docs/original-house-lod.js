import {CHOP53, NEAREST53} from './source-cloth-arithmetic.js';

const profiles = {nearest53: NEAREST53, chop53: CHOP53};
const sha256 = /^[a-f0-9]{64}$/;
const modelPath = /^local-original\/regional-houses\/models\/[a-z0-9_-]+-(?:near|middle|far)\.json$/;
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const vec3 = value => value?.length === 3 && Array.from(value).every(Number.isFinite);

export function houseLodProfile(settings) {
  if (settings === null) return null;
  const {mode, modelDetail} = settings ?? {};
  if (!Number.isInteger(modelDetail) || modelDetail < 0 || modelDetail > 2) {
    throw new RangeError('Explicit source model detail 0/1/2 required');
  }
  if (mode === 'resource' && typeof settings.rendererCap0x200 === 'boolean') {
    return Object.freeze({mode, requestedOrdinal: settings.rendererCap0x200 ? 0 : 2 - modelDetail});
  }
  if (mode !== 'range') throw new RangeError('Explicit House LOD selection mode required');
  const {maxDistance, extraCameraBias, rounding, distanceOverride = -1, forcedOrdinal = -1} = settings;
  if (!Number.isFinite(maxDistance) || maxDistance < 1 || maxDistance > 1000000
      || typeof extraCameraBias !== 'boolean' || typeof rounding !== 'string' || !Object.hasOwn(profiles, rounding)
      || !Number.isFinite(distanceOverride) || !Number.isInteger(forcedOrdinal) || forcedOrdinal > 2147483647) {
    throw new RangeError('Explicit source camera maximum distance, bias and arithmetic required');
  }
  const fpu = profiles[rounding];
  const bias = extraCameraBias ? 2 : 0;
  // CameraManager 0051EF03..0051EF97 stores this quotient in NiCamera+1F0.
  const multiplier = fpu.store32(fpu.div([6, 3, 1][modelDetail] + bias, Math.fround(maxDistance)));
  return Object.freeze({mode, multiplier, fpu, forcedOrdinal, distanceOverride: Math.fround(distanceOverride)});
}

export function nativeLodMetric(center, eye, profile) {
  if (profile.distanceOverride >= 0) return profile.distanceOverride;
  if (!vec3(center) || !vec3(eye)) return NaN;
  const {sub, mul, add, sqrt, store32} = profile.fpu;
  const x = sub(Math.fround(center[0]), Math.fround(eye[0]));
  const up = sub(Math.fround(center[1]), Math.fround(eye[1]));
  const depth = sub(Math.fround(center[2]), Math.fround(eye[2]));
  // Browser basis is [native X, native Z, -native Y]. 008EFA40 adds
  // native Z squared plus Y squared before X squared, then stores one f32.
  return store32(mul(sqrt(add(add(mul(up, up), mul(depth, depth)), mul(x, x))), profile.multiplier));
}

export function nativeRangeChild(ranges, metric) {
  const selected = ranges.findIndex(([near, far]) => near <= metric && metric < far);
  return Math.max(0, selected);
}

export function rangeChild(group, center, eye, profile) {
  // All bound House children are unskinned. 005F9333 sees a null Skin
  // Instance and bypasses the detail write, leaving 008EE490's index zero.
  if (profile.mode === 'resource') return 0;
  const ordinal = profile.forcedOrdinal;
  // Bound House ranges are in the exact contiguous, ordered form for which
  // the source 008EFAE0 interval traversal reduces to a clamped ordinal.
  if (ordinal >= 0) return Math.min(group.ranges.length - 1, ordinal);
  return nativeRangeChild(group.ranges, nativeLodMetric(center, eye, profile));
}

export function worldHouseLodCenter(local, sourceScale, instance, at, yawOffset) {
  // Same basis and yaw orientation as the production building shader. This
  // pose comes from the existing render stream, with no gameplay-state writes.
  const yaw = Math.fround(instance[at + 3] + yawOffset);
  const c = Math.cos(yaw), s = Math.sin(yaw);
  const x = local[0] * sourceScale, z = local[2] * sourceScale;
  return [Math.fround(instance[at] + x * c - z * s),
    Math.fround(instance[at + 2] + local[1] * sourceScale),
    Math.fround(instance[at + 1] + x * s + z * c)];
}

export function houseLodBindings(manifest) {
  if (manifest?.format !== 'ee2-house-source-lod-v1' || !manifest.models) {
    throw new Error('Unsupported original House LOD manifest');
  }
  return new Map(Object.entries(manifest.models).map(([url, row]) => {
    const levels = row.levels;
    if (!modelPath.test(url) || row.sourceFamily !== 'House' || row.sourceId !== 1
        || !sha256.test(row.sourceSha256) || !Number.isFinite(row.sourceScale) || row.sourceScale <= 0
        || !Array.isArray(levels) || ![2, 3].includes(levels.length) || levels[0]?.url !== url
        || !Array.isArray(row.groups) || !row.groups.length || row.groups.length > 2
        || !Array.isArray(row.traversal) || new Set(row.traversal).size !== row.traversal.length) {
      throw new Error('Invalid House LOD entry');
    }
    const ordered = row.traversal.filter(token => token < 0);
    if (ordered.length !== row.groups.length || ordered.some((token, index) => token !== -index - 1)) {
      throw new Error('Invalid House LOD group traversal');
    }
    for (const group of row.groups) {
      if (!vec3(group.localCenter) || !Array.isArray(group.ranges) || group.ranges.length !== levels.length
          || !Array.isArray(group.partNodes) || group.partNodes.length !== levels.length
          || !Array.isArray(group.skinInstanceLinks) || group.skinInstanceLinks.length !== levels.length
          || group.skinInstanceLinks.some(link => link !== -1)
          || group.ranges.some((range, index) => !Array.isArray(range) || range.length !== 2
            || range[0] !== (index === 0 ? 0 : group.ranges[index - 1][1])
            || !Number.isFinite(range[1]) || range[1] <= range[0])
          || group.partNodes.some(parts => !Array.isArray(parts) || !parts.length)) {
        throw new Error('Invalid House LOD source group');
      }
    }
    for (const [index, level] of levels.entries()) {
      const expected = row.traversal.flatMap(token => token < 0 ? row.groups[-token - 1].partNodes[index] : [token]);
      if (!modelPath.test(level.url) || !sha256.test(level.sha256) || !Array.isArray(level.parts)
          || !same(expected, level.parts) || level.parts.some(node => !Number.isInteger(node) || node < 0)
          || new Set(level.parts).size !== level.parts.length) {
        throw new Error('Invalid House LOD payload binding');
      }
    }
    return [url, row];
  }));
}

export function checkHouseLodModel(binding, level, model) {
  if (model?.source?.sha256 !== binding.sourceSha256 || !same(model.normalization, binding.normalization)
      || !Array.isArray(model.parts) || !same(model.parts.map(part => part.sourceNode), level.parts)) {
    throw new Error('House LOD payload identity differs');
  }
}

export async function readHouseLodModel(binding, level, base) {
  const response = await fetch(new URL(level.url, base), {cache: 'no-store'});
  if (!response.ok) throw new Error('House LOD payload unavailable: ' + level.url);
  const bytes = await response.arrayBuffer();
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  const hash = Array.from(digest, value => value.toString(16).padStart(2, '0')).join('');
  if (hash !== level.sha256) throw new Error('House LOD payload hash differs: ' + level.url);
  const model = JSON.parse(new TextDecoder().decode(bytes));
  checkHouseLodModel(binding, level, model);
  return model;
}

export function bindHouseLodParts(binding, levelParts) {
  const parts = binding.levels.map((level, index) => {
    if (level.parts.length !== levelParts[index]?.length) throw new Error('Incomplete House LOD upload');
    return new Map(level.parts.map((node, partIndex) => [node, levelParts[index][partIndex]]));
  });
  const cache = new Map();
  return (variant, instance, index, eye, profile) => {
    const selected = binding.groups.map(group => {
      const center = profile.mode === 'range' ? worldHouseLodCenter(group.localCenter, binding.sourceScale,
        instance, index * 12, variant.yawOffset) : null;
      return rangeChild(group, center, eye, profile);
    });
    const key = selected.join(',');
    if (!cache.has(key)) {
      const selectedParts = binding.traversal.flatMap(token => {
        if (token >= 0) return [parts[0].get(token)];
        const group = -token - 1, level = selected[group];
        return binding.groups[group].partNodes[level].map(node => parts[level].get(node));
      });
      cache.set(key, {...variant, parts: selectedParts, houseLodKey: key});
    }
    return cache.get(key);
  };
}
