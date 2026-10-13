// Headless smoke test + micro benchmark: node tools/smoke.mjs
import { readFileSync } from 'node:fs';
const { instance } = await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch.wasm', import.meta.url)), {});
const w = instance.exports;
const ui = () => JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.ui(), w.ui_len())));

// 1) a normal skirmish, AI only, 25 simulated minutes
w.init(42, 2, 0);
let t0 = performance.now();
for (let i = 0; i < 20 * 60 * 25; i++) { w.tick(); if (i % (20 * 60 * 5) === 0) console.log('min', i / 1200, JSON.stringify(ui()).slice(0, 150)); }
console.log('skirmish: %s ms/tick avg', ((performance.now() - t0) / 30000).toFixed(4), 'entities', ui().n, 'win', ui().win);

// 2) stress test
for (const n of [1000, 2000, 3500]) {
  w.init(7, 1, n);
  for (let i = 0; i < 600; i++) w.tick();
  let worst = 0; t0 = performance.now();
  for (let i = 0; i < 400; i++) { const a = performance.now(); w.tick(); worst = Math.max(worst, performance.now() - a); }
  const avg = (performance.now() - t0) / 400;
  const inst = w.render(0.5, 0, 0, 256, 256, -1, 0, 0);
  console.log(`bench ${n * 2} target: entities ${ui().n}, avg ${avg.toFixed(3)} ms/tick, worst ${worst.toFixed(2)} ms, instances ${inst}`);
}
