import {forestClockWord} from './forest-pose.js?v=valley-forest167';

// The optional authored engine owns the phase and timestamps of each forest row.
export function createForestPresentation(wasm, count) {
  const word = wasm.valley164_forest_actor_word;
  const gather = wasm.cmd_valley_forest_gather;
  const enable = wasm.valley164_enable_forest_play;
  if (typeof word !== 'function' || typeof gather !== 'function' || typeof enable !== 'function') return null;
  const phases = new Uint8Array(count);
  const chops = new Float32Array(count);
  const exhaustions = new Float32Array(count);
  let ready = false;
  return {
    start() { return enable() === 1; },
    snapshot() {
      ready = false;
      for (let row = 0; row < count; row++) {
        const phase = word(row, 0) >>> 0;
        if (phase > 2) return null;
        phases[row] = phase;
        if (phase) {
          chops[row] = forestClockWord(word(row, 6) >>> 0);
          exhaustions[row] = forestClockWord(word(row, 7) >>> 0);
        }
      }
      ready = true;
      return {phases, chops, exhaustions, now: wasm.sim_elapsed_seconds()};
    },
    order(row) {
      return ready && Number.isInteger(row) && row >= 0 && row < count && phases[row] < 2 && gather(row) === 1;
    },
    reset() { ready = false; },
  };
}
