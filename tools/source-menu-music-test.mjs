import assert from 'node:assert/strict';
import {createMenuMusicUpdate} from '../docs/source-menu-music.js';

const files = ['menu-a', 'menu-b'];
const words = [32767, 32766, 32767];
let draws = 0;
const menu = createMenuMusicUpdate(files, {word15: () => words[draws++]});
const stream = positionMs => ({positionMs, durationMs: 10000});

menu.reset();
assert.deepEqual(menu.snapshot(), {next: 1, pending: true});
assert.deepEqual(menu.update(null), {kind: 'play', file: files[0], fadeSeconds: 2});
assert.equal(draws, 2, 'Load reset and missing-stream reset each consume one word');
assert.equal(menu.update(stream(7999)), null);
assert.equal(menu.update(stream(8000)), null);
assert.deepEqual(menu.update(stream(8001)), {kind: 'play', file: files[1], fadeSeconds: 2});
assert.equal(menu.update(stream(0)), null);
assert.deepEqual(menu.update(stream(9999)), {kind: 'play', file: files[0], fadeSeconds: 2});
assert.equal(draws, 2, 'Sequential transitions consume no random word');
assert.deepEqual(menu.update(null), {kind: 'play', file: files[1], fadeSeconds: 2});
assert.equal(draws, 3);
assert.throws(() => createMenuMusicUpdate([], {word15: () => 0}));
console.log('PASS source menu reset, missing-stream RNG, near-end boundary and sequential wrap');
