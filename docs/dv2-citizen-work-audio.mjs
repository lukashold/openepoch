// Presentation-only one-shots. Native event clocks and actor/session validity
// belong to the caller; asset loading never schedules a delayed hit.
// Audio positions are [worldX, worldY, altitude]. The backend must invoke the
// rate supplier only after successful source/sample allocation, as native does.
import {originalOneShotAudible} from './original-spatial-audio.js?v=native39audio';

export function createCitizenWorkPlayback({events, ready, buffer, load, start, stop, clock,
  mix, frequencyFor}) {
  if (!(events instanceof Map) || ![ready,buffer,load,start,stop,clock,mix,frequencyFor]
      .every(value => typeof value === 'function')) throw Error('Invalid Citizen work audio backend');
  const declarations = new Map();
  for (const [name, cue] of events) {
    if (typeof name !== 'string' || cue.name !== name || !Number.isFinite(cue.volume)
        || cue.volume < 0 || !Number.isInteger(cue.maximumInstances) || cue.maximumInstances < 1
        || cue.loop !== 0 || !Number.isFinite(cue.pitchVariation) || cue.pitchVariation < 0
        || !(cue.sampleRate > 0)) throw Error('Invalid Citizen work sound declaration');
    declarations.set(name, Object.freeze({...cue}));
  }
  const voices = new Map();
  let session = null;

  function release(source) {
    voices.delete(source);
    stop(source);
  }
  function reset() {
    for (const source of [...voices.keys()]) release(source);
    session = null;
  }
  function update(world, view) {
    if (!ready() || world === null || world === undefined) { reset(); return; }
    if (session !== world) reset();
    session = world;
    for (const [source, row] of voices) {
      const spatial = mix(view, ...row.position);
      source.updateSpatialMix?.(Math.fround(row.cue.volume * spatial.volume), spatial.pan);
    }
  }
  function play(name, actor, view) {
    const cue = declarations.get(name);
    if (!cue || !ready() || session === null || actor?.session !== session
        || !Number.isInteger(actor.handle) || actor.handle < 0 || actor.handle >= 0xffffffff
        || !Array.isArray(actor.position) || actor.position.length !== 3
        || !actor.position.every(Number.isFinite)) return false;
    const clip = buffer(cue);
    if (!clip) {
      // Preloading may finish later; only a later genuine event may play it.
      load(cue)?.catch?.(() => {});
      return false;
    }
    const now = clock();
    if (!Number.isFinite(now) || now < 0) return false;
    const matching = [...voices].filter(([, row]) => row.cue.name === name);
    if (matching.length >= cue.maximumInstances) {
      // 005113D0 -> 005120B0: declaration-global cap, oldest strictly earlier
      // than audio-now; ties retain array order. Admission precedes audibility.
      let oldest;
      for (const entry of matching) {
        const row = entry[1];
        if (row.started < now && (!oldest || row.started < oldest[1].started)) oldest = entry;
      }
      if (!oldest) return false;
      release(oldest[0]);
    }
    const spatial = mix(view, ...actor.position);
    if (!originalOneShotAudible(spatial.volume)) return false;
    const rate = () => {
      const value = frequencyFor(cue);
      if (!Number.isFinite(value) || value <= 0) throw Error('Invalid work sound frequency');
      return value;
    };
    const source = start(clip, Math.fround(cue.volume * spatial.volume), spatial.pan,
      'citizen-work:' + name, false, 0, rate);
    if (!source) return false;
    voices.set(source, {cue, started:now, handle:actor.handle, position:[...actor.position]});
    const ended = source.onended;
    source.onended = (...args) => { voices.delete(source); ended?.(...args); };
    return true;
  }
  return Object.freeze({update, play, reset,
    preload:() => Promise.all([...declarations.values()].map(cue => load(cue))),
    voices:() => [...voices.values()].map(row => ({name:row.cue.name, handle:row.handle,
      started:row.started, position:[...row.position]})),
  });
}
