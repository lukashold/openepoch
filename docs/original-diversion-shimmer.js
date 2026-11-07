// VisualShimmerDC DiversionShimmer and the common 0065D040/0065D4C0 path.
const visualNames = new Set(['DiversionUnit_Epoch11', 'DiversionUnit_Epoch12', 'DiversionUnit_Epoch13']);
const f32 = Math.fround;
const initial = Object.freeze({opacity: 1, amplitude: 0, emissive: 0});
const tolerance = f32(.001);
const blendRate = f32(.1);
const ownOpacity = f32(.5);
const ownAmplitude = f32(.2);
const ownEmissive = f32(.4);
const waveFrequency = f32(2 * Math.PI) / 2;

function blend(previous, target) {
  const difference = target - previous;
  return Math.abs(f32(difference)) < tolerance ? target : f32(previous + difference * blendRate);
}

export function stepDiversionShimmer(previous = initial, {own, enabled, worldTime, forceOpaque = false}) {
  if (forceOpaque) return {opacity: 1, amplitude: previous.amplitude, emissive: 0};
  const amplitude = blend(previous.amplitude, enabled && own ? ownAmplitude : 0);
  const opacityTarget = enabled ? own ? ownOpacity : 1 : 0;
  const base = blend(previous.opacity, opacityTarget);
  const pulse = amplitude > 0 ? amplitude * .5 * (Math.cos(waveFrequency * f32(worldTime)) + 1) : 0;
  const opacity = Math.min(1, Math.max(0, f32(base + pulse)));
  return {opacity, amplitude, emissive: own ? f32((1 - opacity) * ownEmissive) : 0};
}

// A renderer may sample the same admitted actor for the main view, water,
// shadow and PIP. All passes share one update and the same material sample.
export function createDiversionShimmerFrames() {
  let token = null, viewer = null, time = 0, previous = new Map(), current = new Map();

  function reset() {
    token = null;
    viewer = null;
    previous.clear();
    current.clear();
  }

  function begin(nextToken, nextViewer, worldTime) {
    if (!Number.isSafeInteger(nextToken) || nextToken < 0 || !Number.isInteger(nextViewer) || nextViewer < 0 || !Number.isFinite(worldTime)) {
      reset();
      return;
    }
    if (nextToken === token && nextViewer === viewer) return;
    previous = nextViewer === viewer && nextToken === token + 1 ? current : new Map();
    current = new Map();
    token = nextToken;
    viewer = nextViewer;
    time = worldTime;
  }

  function sample(actor, sourceVisual, enabled, forceOpaque = false) {
    if (token === null || !visualNames.has(sourceVisual) || !(actor?.hpRatio > 0) || typeof enabled !== 'boolean'
      || !Number.isInteger(actor.handle) || actor.handle < 0 || actor.handle >= 0xffffffff || !Number.isInteger(actor.team) || actor.team < 0) return null;
    const {handle, team} = actor;
    const last = current.get(handle);
    if (last) return last.team === team && last.sourceVisual === sourceVisual ? last.material : null;
    const old = previous.get(handle);
    const material = Object.freeze(stepDiversionShimmer(old?.team === team && old.sourceVisual === sourceVisual ? old.material : undefined,
      {own: team === viewer, enabled, worldTime: time, forceOpaque}));
    current.set(handle, {team, sourceVisual, material});
    return material;
  }

  return Object.freeze({begin, sample, reset});
}
