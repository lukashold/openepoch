import {originalLoopAudible, originalSpatialMix} from './original-spatial-audio.js?v=native39audio';

export const OIL_RIG_CUE = Object.freeze({
  event: 'Resource_oil',
  url: new URL('./local-original/oil-rig/fx_oil_rig_15_2.wav', import.meta.url).href,
  sha256: 'a48293cd58dcbf031412d0f46e32cf0e71d0d3a69d577dd6c69de4ac436148c3',
  volume: 0.5,
  maximumInstances: 3,
});

const AUDIO_WORD = new Uint32Array(1);
const AUDIO_FLOAT = new Float32Array(AUDIO_WORD.buffer);
const GARRISONED = 32;
const CULL_SECONDS = 1;

export function readOilRigAudioTracks(wasm) {
  const count = wasm.valley_oil_audio_count() >>> 0;
  if (count > 65535) return [];
  const tracks = [];
  for (let row = 0; row < count; row++) {
    const handle = wasm.valley_oil_audio_word(row, 0) >>> 0;
    if (handle === 0xffffffff || wasm.valley_oil_visual_state(handle) !== GARRISONED) continue;
    const float = field => {
      AUDIO_WORD[0] = wasm.valley_oil_audio_word(row, field);
      return AUDIO_FLOAT[0];
    };
    const [x,y,altitude,started] = [1,2,3,4].map(float);
    const durable = wasm.valley_oil_audio_word(row, 5);
    if (![x,y,altitude,started].every(Number.isFinite) || started < 0 || (durable !== 0 && durable !== 1)) continue;
    tracks.push({handle,x,y,altitude,started,durable:durable===1});
  }
  return tracks;
}

export function createOilRigLoopPlayback({ready, buffer, load, start, stop, clock}) {
  const emitters = new Map();
  let session = null;

  function release(emitter) {
    if (!emitter.source) return;
    const source = emitter.source;
    emitter.source = null;
    stop(source);
  }

  function reset() {
    for (const emitter of emitters.values()) release(emitter);
    emitters.clear();
    session = null;
  }

  function update(world, tracks, view) {
    if (!ready() || !world) { reset(); return; }
    if (session !== world) reset();
    session = world;
    const rows = new Map();
    for (const track of tracks) {
      const {handle, x, y, altitude, started, durable} = track;
      if (!Number.isInteger(handle) || handle < 0 || handle >= 0xffffffff ||
          ![x, y, altitude, started].every(Number.isFinite)) continue;
      rows.set(handle, originalSpatialMix(view, x, y, altitude));
      const previous = emitters.get(handle);
      if (previous?.durable && (!durable || previous.started !== started)) {
        release(previous);
        emitters.delete(handle);
      }
      const emitter = emitters.get(handle);
      if (emitter) { emitter.started = started; emitter.durable = durable; }
      else emitters.set(handle, {source:null, offset:0, began:0, cullAt:null, started, durable});
    }
    for (const [handle, emitter] of emitters) {
      if (rows.has(handle)) continue;
      release(emitter);
      emitters.delete(handle);
    }
    const priority = ([handle, {volume}]) => {
      const emitter = emitters.get(handle);
      return volume * (emitter.source && emitter.cullAt === null ? 1.25 : 1);
    };
    const audible = [...rows].filter(([, {volume}]) => originalLoopAudible(volume))
      .sort((a, b) => priority(b) - priority(a) || a[0] - b[0]);
    const desired = new Map(audible.slice(0, OIL_RIG_CUE.maximumInstances));
    const clip = buffer();
    const now = clock();
    for (const [handle, emitter] of emitters) {
      if (!emitter.source) continue;
      if (emitter.cullAt !== null && now >= emitter.cullAt) {
        emitter.offset = (emitter.offset + Math.max(0, emitter.cullAt - emitter.began)) % emitter.source.buffer.duration;
        emitter.cullAt = null;
        release(emitter);
      } else if (!desired.has(handle)) {
        emitter.cullAt ??= now + CULL_SECONDS;
        const remaining = emitter.cullAt - now;
        const {volume,pan} = rows.get(handle);
        // The native suspended channel uses current spatial volume and a linear one-second ramp.
        emitter.source.updateSpatialMix(Math.fround(volume * remaining),pan,remaining);
      }
    }
    if (!clip) { if (desired.size) load(); return; }
    for (const [handle, {volume, pan}] of desired) {
      const emitter = emitters.get(handle);
      const gain = Math.fround(OIL_RIG_CUE.volume * volume);
      if (emitter.source) { emitter.cullAt = null; emitter.source.updateSpatialMix(gain, pan); continue; }
      const source = start(clip, gain, pan, 'oil-rig', true, emitter.offset);
      if (!source) continue;
      emitter.source = source;
      emitter.began = now;
      const ended = source.onended;
      source.onended = () => {
        ended?.();
        if (emitter.source === source) { emitter.source = null; emitter.cullAt = null; }
      };
    }
  }

  return Object.freeze({update, reset, active: () => [...emitters]
    .filter(([, row]) => row.source && row.cullAt === null).map(([handle]) => handle)});
}
