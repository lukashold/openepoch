import {compileCitizenWork, augmentCitizenWork as augmentV1,
  createCitizenWorkHistory as createVisibilityHistory} from './dv2-citizen-work.mjs';

export {compileCitizenWork};
const word = new DataView(new ArrayBuffer(4));

function requireValue(condition, message) {
  if (!condition) throw new Error('Citizen work clock: ' + message);
}

function roundedFloat(value, precision) {
  const nearest = Math.fround(value);
  if (precision === 'nearest' || nearest <= value) return nearest;
  word.setFloat32(0, nearest, true);
  word.setUint32(0, word.getUint32(0, true) - 1, true);
  return word.getFloat32(0, true);
}

export function citizenWorkClock(duration, span, elapsed, precision) {
  requireValue(precision === 'nearest' || precision === 'towardZero', 'an explicit precision mode is required');
  requireValue(Number.isFinite(duration) && duration > 0 && Math.fround(duration) === duration,
    'intrinsic duration must be a positive native float');
  requireValue(Number.isFinite(span) && span >= 0 && Math.fround(span) === span,
    'span must be a nonnegative native float');
  requireValue(Number.isFinite(elapsed) && elapsed >= 0 && Math.fround(elapsed) === elapsed,
    'supplied elapsed time must be a nonnegative native float');
  const scale = span === 0 ? 1 : roundedFloat(duration / span, precision);
  const scaledTime = roundedFloat(scale * elapsed, precision);
  requireValue(Number.isFinite(scale) && Number.isFinite(scaledTime), 'nonfinite native playback result');
  const time = scaledTime > duration ? roundedFloat(scaledTime % duration, precision) : scaledTime;
  return Object.freeze({duration, span, elapsed, precision, scale, scaledTime, time, phase: time / duration});
}

export function augmentCitizenWork(model, modelURL, manifest) {
  const augmented = augmentV1(model, modelURL, manifest);
  const grids = {};
  for (const part of model.parts) {
    for (const [name, clip] of Object.entries(part.animations ?? {})) {
      const inclusive = clip.endpointIncluded === true || clip.loopIncludesEnd === true;
      const intervals = inclusive ? clip.frames.length - 1 : clip.frames.length;
      requireValue(!grids[name] || grids[name].sampleClamped === inclusive && grids[name].sampleIntervals === intervals,
        'canonical parts disagree on their animation sample grid');
      grids[name] = Object.freeze({sampleClamped: inclusive, sampleIntervals: intervals});
    }
  }
  return {...augmented, nativeCitizenWork: Object.freeze({...augmented.nativeCitizenWork,
    sampleGrids: Object.freeze(grids)})};
}

export function createCitizenWorkHistory() {
  const visibility = createVisibilityHistory();
  return Object.freeze({
    reset() { visibility.reset(); },
    forget(handle) { visibility.forget(handle); },
    pose(model, handle, request) {
      const work = model?.nativeCitizenWork;
      const clip = work?.states.nativeStateClips[request?.stateName];
      requireValue(clip && work.sampleGrids?.[clip] && request?.mode === 0,
        'a verified V2 native state and loop mode0 are required');
      const clock = citizenWorkClock(work.binding.clips[clip].duration, request.span, request.elapsed, request.precision);
      const pose = visibility.pose(model, handle, {stateName: request.stateName,
        elapsed: clock.time, mode: 0, span: 0});
      return Object.freeze({...pose, ...clock, ...work.sampleGrids[clip], once: false});
    },
  });
}
