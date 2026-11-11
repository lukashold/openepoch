// The caller verifies each canonical model JSON against its manifest binding.
// This adapter changes presentation metadata and keeps its geometry and samples.
const compiledData = new WeakMap();
const digestPattern = /^[a-f0-9]{64}$/;

function requireValue(condition, message) {
  if (!condition) throw new Error('Citizen work presentation: ' + message);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}

function assetURL(value) {
  requireValue(typeof value === 'string', 'model URL is required');
  return value.startsWith('./') ? value.slice(2) : value;
}

export function compileCitizenWork(manifest) {
  requireValue(manifest?.format === 'ee2-native-citizen-work-visibility-v1', 'unsupported manifest');
  const source = structuredClone(manifest);
  requireValue(Array.isArray(source.models) && source.models.length > 0, 'missing model bindings');
  const models = new Map();
  for (const binding of source.models) {
    const url = assetURL(binding.url);
    requireValue(!models.has(url) && digestPattern.test(binding.sha256), 'duplicate or unpinned model');
    requireValue(digestPattern.test(binding.sourceSha256), 'missing original NIF identity');
    const geometry = source.sourceGeometry?.[binding.sourceSha256];
    const states = source.stateSets?.[binding.stateSet];
    requireValue(geometry && states, 'missing source geometry or states');
    requireValue(Array.isArray(binding.partNodes) && new Set(binding.partNodes).size === binding.partNodes.length,
      'invalid selected source nodes');
    for (const node of binding.partNodes) {
      requireValue(Number.isInteger(node) && node >= 0 && geometry.parts?.[node], 'unbound source node');
    }
    for (const [state, clip] of Object.entries(states.nativeStateClips)) {
      requireValue(Array.isArray(states.states[state]?.textKeys) && binding.clips?.[clip]?.duration > 0,
        'unbound state or clip');
    }
    for (const [tool, rows] of Object.entries(states.toolVisibilityByState)) {
      requireValue(Object.keys(states.nativeStateClips).every(state => Array.isArray(rows[state])),
        'missing tool state for ' + tool);
      for (const [state, operations] of Object.entries(rows)) {
        requireValue(states.states[state] && Array.isArray(operations), 'unbound tool state');
        let previous = -Infinity;
        for (const [time, visible] of operations) {
          requireValue(Number.isFinite(time) && time >= 0 && time >= previous && (visible === 0 || visible === 1),
            'invalid tool visibility operation for ' + tool);
          previous = time;
        }
      }
    }
    models.set(url, binding);
  }
  freeze(source);
  const compiled = Object.freeze({format: source.format, modelCount: models.size});
  compiledData.set(compiled, {source, models});
  return compiled;
}

export function augmentCitizenWork(model, modelURL, manifest) {
  const compiled = compiledData.has(manifest) ? manifest : compileCitizenWork(manifest);
  const {source, models} = compiledData.get(compiled);
  const binding = models.get(assetURL(modelURL));
  requireValue(binding, 'model URL is outside the verified Citizen bindings');
  requireValue(model?.format === 'ee2-original-mesh-v1' && model.source?.sha256 === binding.sourceSha256,
    'canonical model source differs');
  requireValue(model.animationPayload?.url === binding.animationPayload, 'canonical animation payload differs');
  requireValue(Array.isArray(model.parts) && model.parts.length === binding.partNodes.length,
    'canonical model part count differs');
  const geometry = source.sourceGeometry[binding.sourceSha256];
  const states = source.stateSets[binding.stateSet];
  const tools = {};
  const parts = model.parts.map((part, index) => {
    requireValue(part.sourceNode === binding.partNodes[index], 'canonical source node order differs');
    for (const [clip, animation] of Object.entries(part.animations ?? {})) {
      const expected = binding.clips[clip];
      requireValue(expected && animation.duration === expected.duration && animation.frames?.length === expected.frames,
        'canonical animation sampling differs');
    }
    const original = geometry.parts[part.sourceNode];
    if (!original.tool) return part;
    requireValue(Number.isInteger(original.toolFlags) && states.toolVisibilityByState[original.tool],
      'missing original tool visibility');
    tools[original.tool] = Object.freeze({name: original.tool, node: original.toolNode,
      flags: original.toolFlags, defaultVisible: (original.toolFlags & 1) === 0});
    const {activities, ...body} = part;
    return {...body, nativeCitizenTool: Object.freeze({sourceNode: part.sourceNode,
      tool: original.tool, node: original.toolNode, flags: original.toolFlags,
      path: original.path, legacyActivities: Object.freeze([...(activities ?? [])])})};
  });
  const nativeCitizenWork = Object.freeze({binding, states, tools: Object.freeze(tools),
    sourceSha256: binding.sourceSha256});
  return {...model, parts, nativeCitizenWork};
}

export function createCitizenWorkHistory() {
  const actors = new Map();
  return Object.freeze({
    reset() { actors.clear(); },
    forget(handle) { actors.delete(handle); },
    pose(model, handle, request) {
      const work = model?.nativeCitizenWork;
      requireValue(work && Number.isInteger(handle) && handle >= 0 && handle < 0xffffffff,
        'verified model and full actor handle are required');
      requireValue(request && typeof request.stateName === 'string' && Number.isFinite(request.elapsed) && request.elapsed >= 0,
        'authoritative state and nonnegative elapsed time are required');
      requireValue(request.mode === 0 && request.span === 0, 'only intrinsic looping work requests are supported');
      const clip = work.states.nativeStateClips[request.stateName];
      const state = work.states.states[request.stateName];
      const duration = work.binding.clips[clip]?.duration;
      requireValue(state && Number.isFinite(duration) && duration > 0, 'unknown native state');
      let actor = actors.get(handle);
      if (!actor || actor.source !== work.sourceSha256) {
        actor = {source: work.sourceSha256, tools: Object.fromEntries(
          Object.values(work.tools).map(tool => [tool.name, tool.defaultVisible]))};
        actors.set(handle, actor);
      }
      for (const tool of Object.keys(work.tools)) {
        for (const [time, visible] of work.states.toolVisibilityByState[tool][request.stateName]) {
          if (time <= request.elapsed) actor.tools[tool] = visible === 1;
        }
      }
      const visiblePartNodes = model.parts.filter(part => !part.nativeCitizenTool || actor.tools[part.nativeCitizenTool.tool])
        .map(part => part.sourceNode);
      // The existing morph shader samples these canonical frames. It folds a
      // looping endpoint to its first sample. No raw KF interpolation is added.
      const time = request.elapsed > duration ? Math.fround(request.elapsed % duration) : request.elapsed;
      return Object.freeze({stateName: request.stateName, clip, elapsed: request.elapsed,
        duration, mode: 0, span: 0, once: false, phase: time / duration,
        visiblePartNodes: Object.freeze(visiblePartNodes), visibilityKey: visiblePartNodes.join(','),
        eventDeclarations: Object.freeze(state.textKeys.filter(event => event.name !== 'HIDE_NODE' && event.name !== 'SHOW_NODE')),
        sampling: 'canonical-morph-frames'});
    },
  });
}
