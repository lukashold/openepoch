// Headless tests + micro benchmark: node tools/smoke.mjs
// Exits non-zero if any check fails.
import { readFileSync } from 'node:fs';
import { Lockstep } from '../docs/net.js';

const bytes = readFileSync(process.env.WASM || new URL('../docs/openepoch.wasm', import.meta.url));
const load = async () => (await WebAssembly.instantiate(bytes, {})).instance.exports;
// always fetch the pointer first: the call may grow (and so detach) the wasm memory
const ui = w => { const p = w.ui(); return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer, p, w.ui_len()))); };
const put = (w, ptr, data) => new Uint8Array(w.memory.buffer, ptr, data.length).set(data);
let failed = 0;
const check = (name, ok, info = '') => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${name} ${info}`); if (!ok) failed++; };
const run = (w, n) => { for (let i = 0; i < n; i++) w.tick(); };

// ---- 1) AI skirmish: economy, epochs, technologies, territories
{
  const w = await load();
  w.init(42, 2, 0, 1, 0);
  const t0 = performance.now();
  run(w, 20 * 60 * 30);
  const ms = (performance.now() - t0) / 36000;
  const s = [1, 2].map(t => ({ epoch: w.stat(t, 0), pop: w.stat(t, 1), terr: w.stat(t, 7), tech: w.stat(t, 8) }));
  console.log('     after 30 min:', JSON.stringify(s), `${ms.toFixed(4)} ms/tick`);
  check('AI advances epochs', s.some(p => p.epoch >= 3));
  check('AI researches technologies', s.some(p => p.tech >= 2));
  check('AI claims a second territory', s.some(p => p.terr >= 2));
}

// ---- 2) player flow: build, train, transport, save / load
{
  const w = await load();
  w.init(42, 1, 0, 1, 0);
  const [sx, sy] = [w.start_x(), w.start_y()];
  w.grant(5000);
  w.select_quad(sx - 6, sy - 6, sx + 6, sy - 6, sx + 6, sy + 6, sx - 6, sy + 6, 0);
  check('box select grabs the 5 workers', ui(w).sel.n === 5);
  check('cannot build outside own territory', w.place_error(1, (sx + 90) | 0, (sy + 90) | 0) !== 0);
  let placed = 0;
  for (let dx = -8; dx < 8 && !placed; dx++) for (let dy = 4; dy < 9 && !placed; dy++) placed = w.cmd_place(3, sx + dx, sy + dy) ? [dx, dy] : 0;
  check('barracks placed', !!placed);
  run(w, 900);
  w.select_none(); w.select_point(sx + placed[0] + 1.5, sy + placed[1] + 1.5, 0, 0, 0, 0, 256, 256);
  check('barracks finished', ui(w).sel.name === 'Barracks' && ui(w).sel.built === 1);
  w.select_none(); w.select_point(sx, sy, 0, 0, 0, 0, 256, 256);
  const acts = ui(w).act.map(a => a.name);
  check('town center offers a technology', acts.includes('Stone Tools'), acts.join(','));
  check('research queued', w.cmd_train(200) === 1);
  run(w, 20 * 30);
  check('research completed', w.stat(0, 8) === 1 && ui(w).techs.includes('Stone Tools'));

  // save, keep playing, load: the world must be exactly as saved
  const hash = w.state_hash();
  const sp = w.save();
  const save = new Uint8Array(w.memory.buffer, sp, w.save_len()).slice();
  run(w, 500);
  check('state moved on after saving', w.state_hash() !== hash);
  put(w, w.load_alloc(save.length), save);
  check('load accepted', w.load_apply() === 1);
  check('load restores the exact state', w.state_hash() === hash, `(${save.length} bytes)`);
  save[0] ^= 0xff;
  put(w, w.load_alloc(save.length), save);
  check('corrupt save rejected', w.load_apply() === 0);
  run(w, 200);
  check('simulation runs on after load', ui(w).tick > 0);
}

// ---- 3) lockstep multiplayer: two instances, laggy link, must stay identical
{
  const [a, b] = [await load(), await load()];
  a.init(99, 1, 0, 2, 0); b.init(99, 1, 0, 2, 1);
  const wire = { toA: [], toB: [] };
  let now = 0;
  const la = new Lockstep(a, 0, buf => wire.toB.push([now + 1 + ((Math.random() * 6) | 0), buf]));
  const lb = new Lockstep(b, 1, buf => wire.toA.push([now + 1 + ((Math.random() * 6) | 0), buf]));
  const deliver = (q, ls) => { while (q.length && q[0][0] <= now) ls.receive(q.shift()[1]); };
  const act = (w, r) => {
    const [sx, sy] = [w.start_x(), w.start_y()];
    w.select_quad(sx - 12, sy - 12, sx + 12, sy - 12, sx + 12, sy + 12, sx - 12, sy + 12, 0);
    if (r < 0.3) w.cmd_context(sx + (Math.random() - 0.5) * 30, sy + (Math.random() - 0.5) * 30, r < 0.1 ? 1 : 0);
    else if (r < 0.5) w.cmd_place(1 + ((Math.random() * 3) | 0), (sx + (Math.random() - 0.5) * 24) | 0, (sy + (Math.random() - 0.5) * 24) | 0);
    else { w.select_none(); w.select_point(sx, sy, 0, 0, 0, 0, 256, 256); w.cmd_train(r < 0.8 ? 8 : 200); }
  };
  let stalls = 0;
  for (now = 0; now < 9000; now++) {
    deliver(wire.toA, la); deliver(wire.toB, lb);
    if (Math.random() < 0.05) act(a, Math.random());
    if (Math.random() < 0.05) act(b, Math.random());
    // the two clients run at slightly different speeds, like real machines
    if (Math.random() < 0.9 && !la.step()) stalls++;
    if (Math.random() < 0.8 && !lb.step()) stalls++;
  }
  const t = Math.min(ui(a).tick, ui(b).tick);
  check('lockstep advanced', t > 3000, `(${ui(a).tick} / ${ui(b).tick} ticks, ${stalls} stalls waiting for the peer)`);
  check('no desync detected in ' + la.turn + ' turns', !la.desync && !lb.desync);
  // bring both to the same tick and compare the full fingerprint
  while (ui(a).tick < ui(b).tick) { deliver(wire.toA, la); now++; la.step(); }
  while (ui(b).tick < ui(a).tick) { deliver(wire.toB, lb); now++; lb.step(); }
  check('both peers hold identical state', a.state_hash() === b.state_hash() && ui(a).tick === ui(b).tick);
  check('each seat saw its own economy grow', a.stat(0, 1) > 5 && b.stat(1, 1) > 5, `(pop ${a.stat(0, 1)} / ${b.stat(1, 1)})`);
}

// ---- 4) stress test
for (const n of [1000, 2000, 3500]) {
  const w = await load();
  w.init(7, 1, n, 1, 0);
  run(w, 600);
  let worst = 0; const t0 = performance.now();
  for (let i = 0; i < 400; i++) { const s = performance.now(); w.tick(); worst = Math.max(worst, performance.now() - s); }
  const avg = (performance.now() - t0) / 400;
  const inst = w.render(0.5, 0, 0, 256, 256, -1, 0, 0);
  console.log(`     bench: ${ui(w).n} entities, avg ${avg.toFixed(3)} ms/tick, worst ${worst.toFixed(2)} ms, ${inst} instances`);
}

console.log(failed ? `\n${failed} check(s) FAILED` : '\nall checks passed');
process.exit(failed ? 1 : 0);
