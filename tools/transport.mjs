// Headless test of the transport ship flow: node tools/transport.mjs
import { readFileSync } from 'node:fs';
const w = (await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch.wasm', import.meta.url)), {})).instance.exports;
const ui = () => { const p = w.ui(); return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer, p, w.ui_len()))); };
const run = n => { for (let i = 0; i < n; i++) w.tick(); };
let failed = 0;
const check = (name, ok, info = '') => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${name} ${info}`); if (!ok) failed++; };
const ALL = [0, 0, 256, 0, 256, 256, 0, 256];

let seed = 0, dock = null, sx, sy;
for (seed = 1; seed < 40 && !dock; seed++) {            // find a map whose start territory has a coast
  w.init(seed, 1, 0, 1, 0); w.grant(90000);
  [sx, sy] = [w.start_x(), w.start_y()];
  w.select_none(); w.select_point(sx, sy, 0, 0, ...ALL.slice(0, 2), 256, 256);
  w.cmd_train(255); run(20 * 41);                        // Bronze Age unlocks the dock
  // docks need explored water: walk a worker around is slow, so just look for a spot next to explored shore
  for (let r = 4; r < 60 && !dock; r++) for (let k = -r; k <= r && !dock; k++)
    for (const [x, y] of [[sx + k, sy - r], [sx + k, sy + r], [sx - r, sy + k], [sx + r, sy + k]])
      if (!dock && w.place_error(5, x, y) === 0) dock = [x, y];
}
seed--;
check('found a dock site in own territory', !!dock, `(seed ${seed}, ${dock})`);
w.select_quad(sx - 8, sy - 8, sx + 8, sy - 8, sx + 8, sy + 8, sx - 8, sy + 8, 0);
check('dock placed', w.cmd_place(5, dock[0], dock[1]) === 1);
run(20 * 90);
w.select_none(); w.select_point(dock[0] + 1.5, dock[1] + 1.5, 0, 0, 0, 0, 256, 256);
check('dock finished', ui().sel.name === 'Dock' && ui().sel.built === 1, JSON.stringify(ui().sel).slice(0, 80));
check('dock offers the transport', ui().act.some(a => a.name === 'Transport Ship'));
check('transport queued', w.cmd_train(42) === 1);
run(20 * 20);
// find the ship: it spawned on the water next to the dock
let ship = null;
for (let dx = -4; dx <= 6 && !ship; dx++) for (let dy = -4; dy <= 6 && !ship; dy++) {
  w.select_none(); w.select_point(dock[0] + dx + 0.5, dock[1] + dy + 0.5, 0, 0, 0, 0, 256, 256);
  if (ui().sel.name === 'Transport Ship') ship = [w.sel_center(0), w.sel_center(1)];
}
check('transport built', !!ship, String(ship));
const popBefore = ui().pop;
// all five workers board
// a box would prefer the ship over workers, so pick one worker and "double-click" for all of its kind
const one = w.select_idle_worker();
w.select_point(w.ent_x(one), w.ent_y(one), 0, 1, 0, 0, 256, 256);
const walkers = ui().sel.n;
check('all five workers selected', walkers === 5 && ui().sel.name === 'Worker', `(${walkers} ${ui().sel.name})`);
w.cmd_context(ship[0], ship[1], 0);
run(20 * 60);
w.select_none(); w.select_point(ship[0], ship[1], 0, 0, 0, 0, 256, 256);
let s = ui().sel;
if (s.name !== 'Transport Ship') {                       // it sailed to the beach to meet them: look around
  for (let dx = -12; dx <= 12 && s.name !== 'Transport Ship'; dx++) for (let dy = -12; dy <= 12 && s.name !== 'Transport Ship'; dy++) {
    w.select_none(); w.select_point(ship[0] + dx, ship[1] + dy, 0, 0, 0, 0, 256, 256); s = ui().sel;
  }
}
check('workers boarded', s.name === 'Transport Ship' && s.cargo === walkers, `(${s.cargo}/${s.cargomax}, ${walkers} sent)`);
// (the AI keeps building meanwhile, so count our own units rather than everything on the map)
check('passengers left the map', w.select_idle_worker() >>> 0 === 0xffffffff);
w.select_none(); w.select_point(w.sel_center(0) < 0 ? ship[0] : 0, ship[1], 0, 0, 0, 0, 256, 256);
check('passengers still count as population', ui().pop === popBefore, `(pop ${popBefore} -> ${ui().pop})`);
for (let dx = -12; dx <= 12 && ui().sel.name !== 'Transport Ship'; dx++) for (let dy = -12; dy <= 12 && ui().sel.name !== 'Transport Ship'; dy++) { w.select_none(); w.select_point(ship[0] + dx, ship[1] + dy, 0, 0, 0, 0, 256, 256); }
check('unload button offered', ui().act.some(a => a.k === 'u'));
w.cmd_unload();
run(20 * 5);
check('ship is empty again', ui().sel.cargo === 0);
const back = w.select_idle_worker();
w.select_point(w.ent_x(back), w.ent_y(back), 0, 1, 0, 0, 256, 256);
check('all five workers are back ashore', ui().sel.n === 5 && ui().sel.name === 'Worker', `(${ui().sel.n} ${ui().sel.name})`);
console.log(failed ? `\n${failed} check(s) FAILED` : '\nall checks passed');
process.exit(failed ? 1 : 0);
