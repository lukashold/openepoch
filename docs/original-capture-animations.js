import {loadAnimationPayload} from './original-animation-payload.js?v=citizen9';
import {regionalModelEntries} from './original-regional-models.js?v=regional5';

const STATES = [null, 'CaptureAttack1', 'CaptureAttack2'];
const ART = /^(?:\.\/)?local-original\/capture-animations\/(?:[a-z0-9_.-]+\/)*[a-z0-9_.-]+$/i;
const SHA = /^[a-f0-9]{64}$/;
const clipKey = name => `native-capture:${name}`;
const privateAsset = url => typeof url === 'string' && ART.test(url) && !url.split('/').includes('..');
const integerRange = (value, min, max) => Number.isInteger(value) && value >= min && value <= max;
const integerList = (values, min, max) => Array.isArray(values) && values.length > 0 &&
  values.every(value => integerRange(value, min, max));

function captureStates(states) {
  const entries = Object.entries(states || {});
  if (!entries.length) throw new Error('Missing capture animation states');
  for (const [name, state] of entries) {
    if (!STATES.includes(name) || !Array.isArray(state?.clips) || !state.clips.length ||
        state.clips.some(clip => typeof clip !== 'string' || !/^[a-z0-9_.-]+\.kf$/.test(clip)) ||
        !Array.isArray(state.weights) || state.weights.length !== state.clips.length || state.weights[0] !== 1 ||
        state.weights.some(weight => !Number.isFinite(weight) || weight <= 0)) throw new Error('Invalid capture animation state');
  }
  return entries;
}

export function captureAnimationBindings(manifest) {
  if (manifest?.format !== 'ee2-native-capture-animations-v1' || !Array.isArray(manifest.bindings)) {
    throw new Error('Unsupported native capture animation manifest');
  }
  const bindings = new Map();
  for (const row of manifest.bindings) {
    if (!SHA.test(row?.sourceSha256) || bindings.has(row.sourceSha256)) throw new Error('Invalid capture model source');
    if (!row.near && !row.far) throw new Error('Missing capture animation model');
    for (const asset of [row.near, row.far]) {
      if (asset && (!privateAsset(asset.url) || !SHA.test(asset.sha256))) throw new Error('Invalid capture animation URL or digest');
    }
    captureStates(row.states);
    if (!Array.isArray(row.contexts) || !row.contexts.length || row.contexts.some(context =>
      !integerList(context?.ids, 0, 65535) || !integerRange(context.from, 0, 14) ||
      !integerRange(context.until, context.from + 1, 15) || !integerList(context.civs, 0, 13))) {
      throw new Error('Invalid capture animation visual context');
    }
    bindings.set(row.sourceSha256, row);
  }
  return bindings;
}

export function regionalCaptureBindings(manifest) {
  const bindings = new Map();
  for (const row of regionalModelEntries(manifest)) {
    if (row.until > 15 || !SHA.test(row.source?.sha256) || !privateAsset(row.url) || !SHA.test(row.sha256) ||
        ((row.lodFarUrl || row.lodFarSha256) && (!privateAsset(row.lodFarUrl) || !SHA.test(row.lodFarSha256)))) {
      throw new Error('Invalid regional capture model source');
    }
    captureStates(row.nativeCaptureStates);
    for (let epoch = row.epoch; epoch < row.until; epoch++) {
      for (const civ of row.regionalCivilizations) bindings.set(`${row.id}:${epoch}:${civ}`, row);
    }
  }
  if (manifest.teamManifest && (!privateAsset(manifest.teamManifest) || !SHA.test(manifest.teamManifestSha256))) {
    throw new Error('Invalid regional capture team texture manifest');
  }
  return bindings;
}

export function captureOverlayMatches(model, overlay) {
  if (!['ee2-native-capture-animation-overlay-v1', 'ee2-original-mesh-v1'].includes(overlay?.format) ||
      !model.source?.sha256 || overlay.source?.sha256 !== model.source.sha256 ||
      JSON.stringify(overlay.normalization) !== JSON.stringify(model.normalization) ||
      !Array.isArray(overlay.parts) || overlay.parts.length !== model.parts.length) return false;
  return model.parts.every((part, index) => {
    const extra = overlay.parts[index];
    return extra?.sourceNode === part.sourceNode && (extra.vertices ?? extra.positions?.length / 3) * 3 === part.positions.length &&
      (Array.isArray(extra.indices) ? extra.indices.length : extra.indices) === part.indices.length;
  });
}

export async function loadCaptureAnimationOverlay(model, overlay, states, base) {
  if (!captureOverlayMatches(model, overlay)) throw new Error('Capture animation source or topology differs');
  const payload = overlay.animationPayload;
  if (!privateAsset(payload?.url)) throw new Error('Missing private capture animation payload');
  const names = new Set(Object.values(states).flatMap(state => state.clips));
  const parts = model.parts.map((part, index) => {
    const animations = overlay.parts[index].animations;
    if (!animations || Object.keys(animations).length !== names.size ||
        [...names].some(name => !Object.hasOwn(animations, name))) throw new Error('Capture animation clips differ');
    return {positions: part.positions, animations};
  });
  const loaded = await loadAnimationPayload({source: model.source, animationPayload: payload, parts}, {base});
  return captureAnimationClips(loaded.parts, states);
}

function captureAnimationClips(parts, states) {
  const names = new Set(Object.values(states).flatMap(state => state.clips));
  const durations = new Map();
  const animations = parts.map(part => Object.fromEntries([...names].map(name => {
    const clip = part.animations?.[name];
    if (!clip?.endpointIncluded || clip.once !== true || clip.sampleIntervals !== clip.frames.length - 1 || clip.frames.length < 2 ||
        clip.duration <= 0 || (durations.has(name) && durations.get(name) !== clip.duration)) {
      throw new Error('Capture animation endpoint or duration differs');
    }
    durations.set(name, clip.duration);
    return [clipKey(name), clip];
  })));
  const clips = Object.fromEntries(Object.entries(states).map(([name, state]) => [name, {
    weights: state.weights.map(Math.fround),
    total: state.weights.slice(1).reduce((sum, weight) => Math.fround(sum + Math.fround(weight)), 1),
    clips: state.clips.map(clip => ({clip: clipKey(clip), duration: durations.get(clip)})),
  }]));
  return {animations, clips};
}

export async function loadRegionalCaptureAsset(model, row, base) {
  if (model?.format !== 'ee2-original-mesh-v1' || model.source?.sha256 !== row.source.sha256 ||
      JSON.stringify(model.normalization) !== JSON.stringify(row.nativeScaleBinding.normalization) ||
      JSON.stringify(model.nativeCaptureStates) !== JSON.stringify(row.nativeCaptureStates) ||
      !privateAsset(model.animationPayload?.url) || !Array.isArray(model.parts) || !model.parts.length) {
    throw new Error('Regional capture asset source, scale or states differ');
  }
  const loaded = await loadAnimationPayload(model, {base});
  const {animations, clips} = captureAnimationClips(loaded.parts, row.nativeCaptureStates);
  const native = new Set(Object.values(row.nativeCaptureStates).flatMap(state => state.clips));
  const parts = loaded.parts.map((part, index) => ({...part, animations:{
    ...Object.fromEntries(Object.entries(part.animations).filter(([key]) => !native.has(key))),
    ...animations[index],
  }}));
  if (!['idle', 'walk', 'combat'].every(clip => parts.some(part => part.animations[clip]))) {
    throw new Error('Regional capture asset lacks ordinary movement or combat');
  }
  return {parts, clips};
}

export function nativeCapturePose(request, clips, detail = 2) {
  const state = (request?.state === 1 || request?.state === 2) && clips?.[STATES[request.state]];
  if (!state || !Number.isFinite(request.elapsed) || !Number.isFinite(request.variation) ||
      request.variation < 0 || request.variation > 1) return null;
  let selected = state.clips[0];
  if (detail > 1) {
    let remaining = request.variation * state.total;
    for (let i = 1; i < state.clips.length; i++) {
      remaining -= state.weights[i];
      if (remaining < 0) { selected = state.clips[i]; break; }
    }
  }
  return {clip: selected.clip, once: true, phase: Math.max(0, Math.min(1, request.elapsed / selected.duration))};
}
