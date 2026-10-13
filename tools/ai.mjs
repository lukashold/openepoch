import { readFileSync } from 'node:fs';
const { instance } = await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch.wasm', import.meta.url)), {});
const w = instance.exports;
w.init(+process.argv[2] || 42, 2, 0);
for (let i = 0; i <= 20 * 60 * 40; i++) {
  w.tick();
  if (i % (20 * 60 * 4) === 0) console.log('min', i / 1200, [0, 1, 2].map(t => `T${t} ep${w.stat(t, 0)} pop${w.stat(t, 1)}/${w.stat(t, 2)} res ${[3, 4, 5].map(k => w.stat(t, k) | 0)} bld${w.stat(t, 6)}`).join(' | '));
}
