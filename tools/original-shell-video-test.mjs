import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {mountOriginalShellVideo} from '../docs/original-shell-video.js';

let checks = 0;
function equal(actual, expected) {
  assert.deepEqual(actual, expected);
  checks++;
}

class Video extends EventTarget {
  src = '';
  paused = true;
  hidden = true;
  requests = 0;
  blocked = false;
  getAttribute(name) { return name === 'src' ? this.src : null; }
  async play() {
    this.requests++;
    if (this.blocked) throw new Error('Gesture required');
    this.paused = false;
  }
  pause() { this.paused = true; }
  playing() { this.dispatchEvent(new Event('playing')); }
}
function fixture(profile, {blocked = false, reduce = false} = {}) {
  const video = new Video();
  video.blocked = blocked;
  const page = new EventTarget();
  page.hidden = false;
  const reducedMotion = new EventTarget();
  reducedMotion.matches = reduce;
  const showMenu = mountOriginalShellVideo(video, profile, {page, reducedMotion});
  return {video, page, reducedMotion, showMenu};
}

const {video, page, reducedMotion, showMenu} = fixture('up15', {blocked: true});
equal(new URL(video.src).pathname.endsWith('/shell-video/fe-up15.mp4'), true);
equal(video.hidden, true);
equal(video.requests, 1);
video.blocked = false;
page.dispatchEvent(new Event('pointerdown'));
video.playing();
equal(video.hidden, false);
equal(video.requests, 2);
showMenu(false);
equal(video.paused, true);
page.dispatchEvent(new Event('keydown'));
equal(video.requests, 2);
showMenu(true);
equal(video.paused, false);
page.hidden = true;
page.dispatchEvent(new Event('visibilitychange'));
equal(video.paused, true);
page.hidden = false;
page.dispatchEvent(new Event('visibilitychange'));
equal(video.paused, false);
reducedMotion.matches = true;
reducedMotion.dispatchEvent(new Event('change'));
equal([video.paused, video.hidden], [true, true]);
video.playing();
equal(video.hidden, true);
reducedMotion.matches = false;
reducedMotion.dispatchEvent(new Event('change'));
video.playing();
equal([video.paused, video.hidden], [false, false]);
video.dispatchEvent(new Event('error'));
video.pause();
page.dispatchEvent(new Event('keydown'));
equal([video.paused, video.hidden], [true, true]);

const base = fixture('base', {reduce: true});
equal([base.video.src, base.video.requests], ['', 0]);
base.reducedMotion.matches = false;
base.reducedMotion.dispatchEvent(new Event('change'));
equal(new URL(base.video.src).pathname.endsWith('/shell-video/fe-base.mp4'), true);
const own = fixture('own');
own.showMenu(false);
own.showMenu(true);
equal([own.video.src, own.video.requests], ['', 0]);

const html = await fs.readFile(new URL('../docs/index.html', import.meta.url), 'utf8');
const tag = html.match(/<video\b[^>]*\bid="shell-movie"[^>]*>/)?.[0] ?? '';
for (const attribute of ['muted', 'playsinline', 'loop', 'hidden', 'aria-hidden="true"', 'preload="none"']) {
  equal(tag.includes(' ' + attribute), true);
}
const main = await fs.readFile(new URL('../docs/main.js', import.meta.url), 'utf8');
equal(main.includes("mountOriginalShellVideo($('shell-movie'),originalEnabled?rulesProfile.id:null)"), true);
equal(main.slice(main.indexOf('function begin()'), main.indexOf('function start(nAi')).includes('shellMovie(false);'), true);
equal(main.slice(main.indexOf('function toMenu()'), main.indexOf("$('quit').onclick")).includes('shellMovie(true);'), true);
console.log(JSON.stringify({status: 'passed', checks, scope: 'Actual shell video controller, lifecycle and profile bindings with a substituted media element.'}));
