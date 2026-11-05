// MusicCtrlGame decisions from the installed controller. The caller supplies
// the original presentation stream state and the audio backend's observation.
// Simulation never sees or advances this stream.
const WORD_SCALE = 32769 / 1073741824; // exact F32 at B41418
const FADE_SECONDS = 2;

export function createPresentationRandom(initialState) {
  if (!Number.isInteger(initialState) || initialState < 0 || initialState > 0xffffffff) {
    throw new RangeError('A supplied u32 presentation state is required');
  }
  let state = initialState;
  return Object.freeze({
    word15() {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
      return (state >>> 16) & 0x7fff;
    },
    state: () => state,
  });
}

export function chooseAmbientPiece(settings, regionSet, epochSet, random) {
  const {fracSilentPieces, timeSilenceLow, timeSilenceHigh, numLoopsLow, numLoopsHigh} = settings;
  const score = random.word15() * WORD_SCALE;
  if (score < fracSilentPieces) {
    const seconds = Math.fround((timeSilenceHigh - timeSilenceLow) * random.word15() * WORD_SCALE + timeSilenceLow);
    return {kind: 'silence', seconds};
  }
  // 50EAA0 appends the region list in source order. 50EE60 reserves the
  // epoch tail and returns its starting index. Repeated names retain weight.
  const files = [...(regionSet?.files ?? []), ...(epochSet?.files ?? [])];
  if (files.length === 0) return {kind: 'silence', seconds: 60};
  const file = files[random.word15() % files.length];
  const loops = (numLoopsLow + random.word15() % (numLoopsHigh - numLoopsLow + 1)) & 0xffff;
  return {kind: 'play', file, loops: loops || 2};
}

export function createAmbientUpdate(settings, random) {
  let region = null, epoch = null, silent = false, seconds = 0, loops = 0, nearEnd = false;

  function reset() {
    region = null;
    epoch = null;
    silent = false;
    seconds = 0;
    loops = 0;
    nearEnd = false;
  }

  function update({membership, elapsedSeconds, stream}) {
    if (!stream) reset();
    if (membership && (membership.region !== region || membership.epoch !== epoch)) {
      region = membership.region;
      epoch = membership.epoch;
      const plan = chooseAmbientPiece(settings, region, epoch, random);
      silent = plan.kind === 'silence';
      if (silent) {
        seconds = plan.seconds;
        return stream ? {kind: 'fade', fadeSeconds: FADE_SECONDS} : null;
      }
      loops = plan.loops;
      nearEnd = false;
      return {...plan, fadeSeconds: FADE_SECONDS};
    }
    if (silent) {
      const remaining = seconds - Math.fround(elapsedSeconds);
      seconds = Math.fround(remaining);
      if (remaining <= 0) reset();
    }
    if (stream) {
      if (stream.positionMs < 4000) nearEnd = false;
      else if (!nearEnd && stream.durationMs - stream.positionMs < 2000) {
        nearEnd = true;
        loops--;
        if (loops < 1) reset();
      }
    }
    return null;
  }

  return Object.freeze({reset, update, snapshot: () => ({region, epoch, silent, seconds, loops, nearEnd})});
}
