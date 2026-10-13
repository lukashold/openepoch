// Sound effects, synthesised with WebAudio at play time: no audio files.
// Every sound is a short burst of filtered noise and/or a swept oscillator.

let ctx = null, master = null, noise = null;
let muted = false, voices = 0;
const lastPlayed = new Map();

function ensure() {
  if (ctx) return true;
  const AC = window.AudioContext || window.webkitAudioContext;
  if (!AC) return false;
  ctx = new AC();
  master = ctx.createGain(); master.gain.value = 0.5;
  const comp = ctx.createDynamicsCompressor();   // keeps a 500-gun broadside from clipping
  master.connect(comp); comp.connect(ctx.destination);
  noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
  const d = noise.getChannelData(0);
  for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  return true;
}
/** Browsers only allow audio after a user gesture; call this from one. */
export function unlock() { if (ensure() && ctx.state === 'suspended') ctx.resume(); }
export function setMuted(m) { muted = m; }
export const isMuted = () => muted;

function env(g, t, peak, attack, decay) {
  g.gain.setValueAtTime(0.0001, t);
  g.gain.exponentialRampToValueAtTime(Math.max(peak, 0.0002), t + attack);
  g.gain.exponentialRampToValueAtTime(0.0001, t + attack + decay);
}
function out(vol, pan) {
  const g = ctx.createGain();
  let node = g;
  if (ctx.createStereoPanner) { const p = ctx.createStereoPanner(); p.pan.value = pan; g.connect(p); node = p; }
  node.connect(master);
  return g;
}
function burst(t, vol, pan, { type = 'lowpass', f0 = 800, f1 = f0, q = 1, attack = 0.004, decay = 0.1 }) {
  const src = ctx.createBufferSource(); src.buffer = noise; src.loop = true;
  src.loopStart = Math.random() * 0.5;
  const flt = ctx.createBiquadFilter(); flt.type = type; flt.Q.value = q;
  flt.frequency.setValueAtTime(f0, t); flt.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + attack + decay);
  const g = out(vol, pan); env(g, t, vol, attack, decay);
  src.connect(flt); flt.connect(g);
  src.start(t, src.loopStart); src.stop(t + attack + decay + 0.03);
  voices++; src.onended = () => voices--;
}
function tone(t, vol, pan, { wave = 'sine', f0 = 440, f1 = f0, attack = 0.004, decay = 0.12 }) {
  const o = ctx.createOscillator(); o.type = wave;
  o.frequency.setValueAtTime(f0, t); o.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + attack + decay);
  const g = out(vol, pan); env(g, t, vol, attack, decay);
  o.connect(g); o.start(t); o.stop(t + attack + decay + 0.03);
  voices++; o.onended = () => voices--;
}

// kind -> [minimum gap between plays in seconds, recipe]
const SOUNDS = {
  1: [0.05, (t, v, p) => burst(t, v * 0.35, p, { type: 'bandpass', f0: 3200, f1: 900, q: 2, decay: 0.09 })],                 // arrow
  2: [0.035, (t, v, p) => burst(t, v * 0.5, p, { type: 'highpass', f0: 1400, f1: 500, decay: 0.06 })],                        // gunshot
  3: [0.07, (t, v, p) => { burst(t, v * 0.8, p, { f0: 900, f1: 120, decay: 0.28 }); tone(t, v * 0.7, p, { f0: 110, f1: 38, decay: 0.3 }); }], // cannon
  4: [0.2, (t, v, p) => tone(t, v * 0.5, p, { f0: 260, f1: 70, decay: 0.35 })],                                                // torpedo
  5: [0.1, (t, v, p) => burst(t, v * 0.5, p, { type: 'bandpass', f0: 500, f1: 2600, q: 1.5, attack: 0.03, decay: 0.4 })],      // missile
  6: [0.2, (t, v, p) => tone(t, v * 0.3, p, { f0: 1300, f1: 500, decay: 0.5 })],                                               // bomb whistle
  7: [0.06, (t, v, p) => { tone(t, v * 0.22, p, { wave: 'square', f0: 900 + Math.random() * 500, f1: 500, decay: 0.04 }); burst(t, v * 0.2, p, { type: 'highpass', f0: 3000, decay: 0.03 }); }], // melee
  10: [0.06, (t, v, p) => { burst(t, v * 0.9, p, { f0: 700, f1: 80, decay: 0.45 }); tone(t, v * 0.8, p, { f0: 90, f1: 30, decay: 0.4 }); }],   // explosion
  11: [0.2, (t, v, p) => { burst(t, v, p, { f0: 500, f1: 50, decay: 1.1 }); tone(t, v * 0.9, p, { f0: 70, f1: 24, decay: 0.9 }); }],            // building collapses
  12: [0.3, (t, v) => { tone(t, v * 0.3, 0, { wave: 'triangle', f0: 660, decay: 0.09 }); tone(t + 0.09, v * 0.3, 0, { wave: 'triangle', f0: 880, decay: 0.14 }); }], // unit ready
  13: [0.5, (t, v) => [523, 659, 784, 1047].forEach((f, i) => tone(t + i * 0.11, v * 0.35, 0, { wave: 'triangle', f0: f, decay: 0.3 }))],     // research / epoch
  14: [0.4, (t, v) => { tone(t, v * 0.3, 0, { wave: 'triangle', f0: 440, decay: 0.1 }); tone(t + 0.1, v * 0.3, 0, { wave: 'triangle', f0: 660, decay: 0.2 }); }], // building done
  15: [9, (t, v) => { for (let i = 0; i < 3; i++) tone(t + i * 0.16, 0.3, 0, { wave: 'square', f0: 740, f1: 620, decay: 0.1 }); }],          // under attack
  20: [0.03, (t, v) => tone(t, v * 0.18, 0, { f0: 720, decay: 0.035 })],                                                       // ui click
  21: [0.15, (t, v) => tone(t, v * 0.3, 0, { wave: 'square', f0: 170, f1: 120, decay: 0.14 })],                                 // ui error
  22: [0.05, (t, v) => tone(t, v * 0.2, 0, { wave: 'triangle', f0: 520, f1: 640, decay: 0.06 })],                               // order acknowledged
};

/** Play sound `kind` at volume 0..1 and stereo position -1..1. */
export function play(kind, vol = 1, pan = 0) {
  const s = SOUNDS[kind];
  if (muted || !s || vol < 0.02 || !ensure() || ctx.state !== 'running') return;
  const now = ctx.currentTime;
  if (now - (lastPlayed.get(kind) ?? -99) < s[0]) return;   // rate limit per kind
  if (voices > 28 && kind < 12) return;                     // polyphony cap for battle noise
  lastPlayed.set(kind, now);
  s[1](now, Math.min(1, vol), Math.max(-1, Math.min(1, pan)));
}
