// OpenEpoch front end: input, camera, WebGL2 drawing and HUD.
// The simulation lives in openepoch.wasm (Rust); this file never touches game rules.
import { buildAtlas, tintAtlas, CELL, ATLAS } from './atlas.js';

const $ = id => document.getElementById(id);
const canvas = $('gl');
const gl = canvas.getContext('webgl2', { antialias: false, alpha: false, powerPreference: 'high-performance' });
if (!gl) { document.body.innerHTML = '<p style="padding:40px">OpenEpoch needs WebGL2. Please use a current browser.</p>'; throw new Error('no webgl2'); }

const MAP = 256, TICK = 0.05, BAR_TOP = 36, BAR_BOTTOM = 170;
const TEAM = [[0.27, 0.59, 1.0], [0.94, 0.27, 0.24], [0.31, 0.86, 0.39], [0.96, 0.82, 0.27], [1, 1, 1], [1, 1, 1], [1, 1, 1], [0.3, 0.75, 1]];

// ---------------------------------------------------------------- wasm
const wasmUrl = new URL('openepoch.wasm', import.meta.url);
let wasm;
try {
  wasm = (await WebAssembly.instantiateStreaming(fetch(wasmUrl), {})).instance.exports;
} catch {
  wasm = (await WebAssembly.instantiate(await (await fetch(wasmUrl)).arrayBuffer(), {})).instance.exports;
}
const mem = () => wasm.memory.buffer;
const dec = new TextDecoder();

// ---------------------------------------------------------------- GL setup
function shader(type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src); gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) + src);
  return s;
}
function program(vs, fs) {
  const p = gl.createProgram();
  gl.attachShader(p, shader(gl.VERTEX_SHADER, vs)); gl.attachShader(p, shader(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {};
  const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const name = gl.getActiveUniform(p, i).name.replace('[0]', ''); u[name] = gl.getUniformLocation(p, name); }
  return { p, u };
}

const NOISE = `
float hash(vec2 p){ p = fract(p*vec2(123.34, 456.21)); p += dot(p, p+45.32); return fract(p.x*p.y); }
float noise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }`;

const terrain = program(`#version 300 es
layout(location=0) in vec2 aPos; uniform vec4 uCam; out vec2 vW;
void main(){ vW = uCam.xy + aPos*vec2(1.,-1.)/uCam.zw; gl_Position = vec4(aPos,0.,1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog; uniform float uTime; in vec2 vW; out vec4 o;
${NOISE}
void main(){
  vec2 uv = vW/${MAP}.;
  if (uv.x<0.||uv.y<0.||uv.x>1.||uv.y>1.) { o = vec4(0.015,0.03,0.05,1.); return; }
  vec4 m = texture(uMap, uv);
  float n = noise(vW*2.7)*.6 + noise(vW*9.)*.4;
  vec3 land = m.rgb*(0.86 + 0.28*n);
  float wv = noise(vW*1.3 + uTime*vec2(.35,.22))*.6 + noise(vW*3.1 - uTime*vec2(.2,.4))*.4;
  vec3 water = m.rgb*(0.8 + 0.45*wv) + vec3(.9,.95,1.)*smoothstep(.72,.9,wv)*.12;
  float shore = smoothstep(.42,.62,m.a);
  vec3 c = mix(water, land, shore);
  float foam = smoothstep(.12,.4,m.a)*(1.-smoothstep(.4,.62,m.a));
  c += foam*(.16 + .12*sin(uTime*1.7 + n*9.));
  float f = texture(uFog, uv).r;
  c *= smoothstep(0.02,.45,f)*mix(.5,1.,smoothstep(.55,.95,f));
  o = vec4(c,1.);
}`);

const sprites = program(`#version 300 es
layout(location=0) in vec2 aC; layout(location=1) in vec4 aA; layout(location=2) in vec4 aB;
uniform vec4 uCam; out vec2 vUv; out vec2 vL; out vec2 vW; flat out vec4 vB;
void main(){
  float c=cos(aA.w), s=sin(aA.w); vec2 p=aC*aA.z;
  vec2 w = aA.xy + vec2(p.x*c-p.y*s, p.x*s+p.y*c);
  vW=w; vL=aC; vB=aB;
  vUv = (vec2(mod(aB.x,16.), floor(aB.x/16.)) + aC + .5)/16.;
  gl_Position = vec4((w-uCam.xy)*uCam.zw*vec2(1.,-1.),0.,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uAtlas, uFog; uniform vec3 uTeam[8]; uniform float uPx;
in vec2 vUv; in vec2 vL; in vec2 vW; flat in vec4 vB; out vec4 o;
void main(){
  int fl = int(vB.w+.5); int team = int(vB.y+.5); vec4 col;
  if (vB.x > 239.5) {
    float r = length(vL)*2.;
    if (vB.x < 240.5) {                       // selection ring
      float w = clamp(uPx*1.5, .02, .2);
      float a = smoothstep(.86-w,.86-w*.5,r)*(1.-smoothstep(.86+w*.5,.86+w,r));
      col = vec4(mix(vec3(.6,1.,.6), uTeam[team], .35),1.)*a*.9;
    } else if (vB.x < 241.5) {                // health / progress bar
      float y = abs(vL.y), x = vL.x+.5;
      if (y > .075) discard;
      vec3 c = team==7 ? vec3(.3,.75,1.) : mix(vec3(.95,.2,.15), vec3(.25,.9,.3), smoothstep(.2,.7,vB.z));
      if (x > vB.z) c = vec3(.1);
      if (y > .045) c = vec3(0.);
      col = vec4(c,1.);
    } else {                                  // explosion
      float t = vB.z, q = r/(.3+.7*t);
      float a = (1.-smoothstep(.55,1.,q))*(1.-t*t);
      vec3 c = mix(vec3(1.,.95,.7), vec3(1.,.4,.05), clamp(q*.7+t*.6,0.,1.));
      col = vec4(mix(c, vec3(.15), smoothstep(.5,1.,t))*a, a*.7);
    }
  } else {
    vec4 t = texture(uAtlas, vUv);
    if (t.a < .02) discard;
    vec3 c = t.rgb/t.a; float a = t.a;
    float k = clamp((min(c.r,c.b)-c.g)*3.,0.,1.);
    c = mix(c, uTeam[team]*max(c.r,c.b), k);
    if ((fl&1)!=0) { c = vec3(0.); a *= .3; }
    if ((fl&2)!=0) { c = mix(c, vec3(.1,.38,.55), .5); a *= .62; }
    if ((fl&4)!=0) c = mix(c, vec3(1.), .5);
    if ((fl&64)!=0 && vB.z < 1.) { float g = dot(c, vec3(.33)); c = mix(vec3(g)*vec3(1.,.9,.7), c, vB.z*.7); a *= .5+.5*vB.z; }
    if ((fl&8)!=0) { c = mix(c, vec3(.3,1.,.4), .35); a *= .65; }
    if ((fl&16)!=0) { c = mix(c, vec3(1.,.15,.15), .6); a *= .65; }
    col = vec4(c*a, a);
  }
  if ((fl&32)==0) {
    float f = texture(uFog, vW/${MAP}.).r;
    col.rgb *= mix(.5,1.,smoothstep(.55,.95,f));
    col *= smoothstep(.02,.45,f);
  }
  o = col;
}`);

function tex(unit, internal, w, h, format, type, filter, data) {
  const t = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  if (data !== undefined) gl.texImage2D(gl.TEXTURE_2D, 0, internal, w, h, 0, format, type, data);
  return t;
}
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
const atlasCanvas = buildAtlas();
const iconAtlas = tintAtlas(atlasCanvas, TEAM[0].map(v => v * 255));
gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
tex(0, gl.RGBA, 0, 0, 0, 0, gl.LINEAR_MIPMAP_LINEAR);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, atlasCanvas);
gl.generateMipmap(gl.TEXTURE_2D);
gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
tex(1, gl.RGBA, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, gl.LINEAR, null);
tex(2, gl.R8, MAP, MAP, gl.RED, gl.UNSIGNED_BYTE, gl.LINEAR, null);

// fullscreen triangle for the terrain
const vaoT = gl.createVertexArray(); gl.bindVertexArray(vaoT);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
// instanced quad for everything else: one draw call per frame
const vaoS = gl.createVertexArray(); gl.bindVertexArray(vaoS);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-.5, -.5, .5, -.5, -.5, .5, .5, .5]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
const instBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
for (const loc of [1, 2]) { gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 4, gl.FLOAT, false, 32, (loc - 1) * 16); gl.vertexAttribDivisor(loc, 1); }
gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

gl.useProgram(sprites.p);
gl.uniform1i(sprites.u.uAtlas, 0); gl.uniform1i(sprites.u.uFog, 2); gl.uniform3fv(sprites.u.uTeam, TEAM.flat());
gl.useProgram(terrain.p);
gl.uniform1i(terrain.u.uMap, 1); gl.uniform1i(terrain.u.uFog, 2);

// ---------------------------------------------------------------- state
const cam = { x: 128, y: 128, zoom: 28 };
const mouse = { x: 0, y: 0, down: false, sx: 0, sy: 0, drag: false, pan: false, inside: false };
const keys = new Set();
let running = false, paused = false, speed = 1, acc = 0, last = 0, bench = 0;
let ghost = null;        // { id, size } while placing a building
let attackMode = false;  // A pressed: next left click is attack-move
let ui = null, actSig = '', lastGroup = { n: -1, t: 0 }, lastClick = 0;
let simMs = 0, frameMs = 16.7, worstMs = 0, fogTick = -1, lastUi = 0, lastMini = 0;
const SPEEDS = [0.5, 1, 2, 4];

const toWorld = (px, py) => [cam.x + (px - innerWidth / 2) / cam.zoom, cam.y + (py - innerHeight / 2) / cam.zoom];
function clampCam() {
  cam.zoom = Math.min(72, Math.max(5, cam.zoom));
  cam.x = Math.min(MAP, Math.max(0, cam.x)); cam.y = Math.min(MAP, Math.max(0, cam.y));
}
function toast(msg) { const t = $('toast'); t.textContent = msg; t.style.opacity = 1; clearTimeout(toast.h); toast.h = setTimeout(() => t.style.opacity = 0, 1800); }
function ghostTile() {
  const [wx, wy] = toWorld(mouse.x, mouse.y);
  return [Math.round(wx - ghost.size / 2), Math.round(wy - ghost.size / 2)];
}
function viewRect() {
  const hw = innerWidth / 2 / cam.zoom, hh = innerHeight / 2 / cam.zoom;
  return [cam.x - hw, cam.y - hh, cam.x + hw, cam.y + hh];
}

function start(nAi, seed, benchUnits) {
  bench = benchUnits;
  wasm.init(seed >>> 0, nAi, benchUnits);
  cam.x = benchUnits ? MAP / 2 : wasm.start_x(); cam.y = benchUnits ? MAP / 2 : wasm.start_y(); cam.zoom = benchUnits ? 9 : 28;
  ghost = null; attackMode = false; paused = false; speed = 1; acc = 0; fogTick = -1; actSig = ''; worstMs = 0;
  $('speed').textContent = '1×'; $('pause').textContent = 'Pause';
  $('menu').style.display = 'none'; $('end').style.display = 'none';
  running = true; last = performance.now();
}

// ---------------------------------------------------------------- frame loop
function frame(now) {
  requestAnimationFrame(frame);
  const dt = Math.min(0.1, (now - last) / 1000); last = now;
  if (!running) return;
  frameMs += (dt * 1000 - frameMs) * 0.05;
  worstMs = Math.max(worstMs * 0.995, dt * 1000);

  // camera
  const pan = 22 / cam.zoom * 60 * dt * (keys.has('Shift') ? 2 : 1);
  if (keys.has('ArrowLeft') || (mouse.inside && mouse.x < 4)) cam.x -= pan;
  if (keys.has('ArrowRight') || (mouse.inside && mouse.x > innerWidth - 5)) cam.x += pan;
  if (keys.has('ArrowUp') || (mouse.inside && mouse.y < 4)) cam.y -= pan;
  if (keys.has('ArrowDown') || (mouse.inside && mouse.y > innerHeight - 5)) cam.y += pan;
  clampCam();

  // fixed-step simulation, decoupled from the display rate
  if (!paused) {
    acc += dt * speed;
    let steps = 0;
    const t0 = performance.now();
    while (acc >= TICK && steps < 8) { wasm.tick(); acc -= TICK; steps++; }
    if (steps) simMs += ((performance.now() - t0) / steps - simMs) * 0.1;
    if (acc > TICK) acc = TICK; // never spiral: drop time instead of stalling the frame
  }
  const alpha = paused ? 1 : acc / TICK;

  // resize
  const dpr = Math.min(2, devicePixelRatio || 1);
  const w = Math.round(innerWidth * dpr), h = Math.round(innerHeight * dpr);
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  gl.viewport(0, 0, w, h);

  // textures that change
  if (wasm.map_dirty()) {
    gl.activeTexture(gl.TEXTURE1);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(mem(), wasm.map_ptr(), MAP * MAP * 4));
  }
  const tickNow = ui ? ui.tick : 0;
  if (fogTick !== tickNow || fogTick < 0) {
    fogTick = tickNow;
    gl.activeTexture(gl.TEXTURE2);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RED, gl.UNSIGNED_BYTE, new Uint8Array(mem(), wasm.fog_ptr(), MAP * MAP));
  }

  const camU = [cam.x, cam.y, 2 * cam.zoom / innerWidth, 2 * cam.zoom / innerHeight];
  gl.useProgram(terrain.p); gl.bindVertexArray(vaoT);
  gl.uniform4fv(terrain.u.uCam, camU); gl.uniform1f(terrain.u.uTime, now / 1000);
  gl.drawArrays(gl.TRIANGLES, 0, 3);

  const [x0, y0, x1, y1] = viewRect();
  let gid = -1, gx = 0, gy = 0;
  if (ghost) { gid = ghost.id; [gx, gy] = ghostTile(); }
  const n = wasm.render(alpha, x0, y0, x1, y1, gid, gx, gy);
  if (n) {
    gl.useProgram(sprites.p); gl.bindVertexArray(vaoS);
    gl.uniform4fv(sprites.u.uCam, camU); gl.uniform1f(sprites.u.uPx, 1 / cam.zoom);
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(mem(), wasm.inst_ptr(), n * 8), gl.STREAM_DRAW);
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, n);
  }

  if (now - lastUi > 100) { lastUi = now; updateUi(n); }
  if (now - lastMini > 250) { lastMini = now; drawMini(); }
}

// ---------------------------------------------------------------- HUD
const HOT = ['Q', 'W', 'E', 'R', 'T', 'Z', 'X', 'C', 'V', 'B'];
function icon(id, px) {
  const c = document.createElement('canvas'); c.width = c.height = px;
  c.getContext('2d').drawImage(iconAtlas, (id % 16) * CELL, ((id / 16) | 0) * CELL, CELL, CELL, 0, 0, px, px);
  return c;
}
function costText(c) { return [['Food', c[0]], ['Wood', c[1]], ['Metal', c[2]]].filter(x => x[1]).map(x => `${x[1]} ${x[0]}`).join(' · ') || 'free'; }

function updateUi(drawn) {
  ui = JSON.parse(dec.decode(new Uint8Array(mem(), wasm.ui(), wasm.ui_len())));
  for (let i = 0; i < 3; i++) $('r' + i).textContent = ui.res[i];
  $('pop').textContent = `${ui.pop}/${ui.cap}`;
  $('epoch').textContent = ui.ename;
  $('idle').textContent = `Idle workers: ${ui.idle}`;
  $('idle').style.visibility = ui.idle && !bench ? 'visible' : 'hidden';
  $('perf').textContent = `${Math.round(1000 / frameMs)} fps · worst ${worstMs.toFixed(0)} ms · sim ${simMs.toFixed(2)} ms/tick · ${ui.n} entities · ${drawn} drawn`;

  const s = ui.sel, info = $('info');
  if (!s.n) info.innerHTML = bench ? '<h3>Stress test</h3><span style="color:var(--dim)">Two armies are kept at full strength and thrown at each other. Zoom and pan freely; you can still select and command the blue side.</span>' : '<span style="color:var(--dim)">Nothing selected</span>';
  else {
    let h = `<h3>${s.n > 1 && s.types.length > 1 ? s.n + ' units' : s.name + (s.n > 1 ? ' ×' + s.n : '')}</h3>`;
    if (s.n === 1) {
      h += `<div class="hp"><i style="width:${100 * s.hp / s.max}%"></i></div>${s.hp} / ${s.max} HP`;
      if (s.dmg) h += ` · ${s.dmg} dmg · range ${s.rng}`;
      if (s.built < 1) h += `<br>Under construction: ${Math.round(s.built * 100)}%`;
    }
    if (s.types.length > 1) h += '<div class="types">' + s.types.map(t => `<span>${t[2]} ×${t[1]}</span>`).join('') + '</div>';
    if (s.q.length) h += `<div class="q" id="queue"></div><div class="hp"><i style="width:${s.qp * 100}%;background:#4aa3ff"></i></div>`;
    info.innerHTML = h;
    const q = $('queue');
    if (q) { for (const id of s.q) { if (id === 255) { const b = document.createElement('b'); b.textContent = '⬆ Epoch'; q.append(b); } else q.append(icon(id, 30)); } q.title = 'Right-click an action button or press Backspace to cancel the last item'; }
  }

  const sig = ui.act.map(a => a.k + a.id).join(',');
  const acts = $('acts');
  if (sig !== actSig) {
    actSig = sig; acts.innerHTML = '';
    ui.act.forEach((a, i) => {
      const b = document.createElement('button'); b.className = 'act'; b.dataset.i = i;
      if (a.k === 'e') { const d = document.createElement('div'); d.style.cssText = 'font-size:30px;height:44px;line-height:44px'; d.textContent = '⬆'; b.append(d); }
      else b.append(icon(a.id, 44));
      const sm = document.createElement('small'); sm.textContent = a.name; b.append(sm);
      if (HOT[i]) { const k = document.createElement('kbd'); k.textContent = HOT[i]; b.append(k); }
      b.onclick = () => doAct(i);
      b.oncontextmenu = e => { e.preventDefault(); wasm.cmd_cancel(); };
      b.onmouseenter = () => { const a = ui.act[i]; if (!a) return; const t = $('tip'); t.innerHTML = `<b>${a.name}</b> <span class="c">${costText(a.cost)}</span><br>${a.tip}`; t.style.display = 'block'; };
      b.onmouseleave = () => $('tip').style.display = 'none';
      acts.append(b);
    });
  }
  [...acts.children].forEach((b, i) => b.classList.toggle('no', !ui.act[i].ok));

  if (ui.win && !bench && $('end').style.display !== 'flex') {
    $('endtext').textContent = ui.win === 1 ? 'Victory!' : 'Defeat';
    $('end').style.display = 'flex';
  }
}

function doAct(i) {
  const a = ui && ui.act[i];
  if (!a) return;
  if (a.k === 'b') { ghost = { id: a.id, size: a.size }; attackMode = false; return; }
  if (!wasm.cmd_train(a.id)) toast(a.ok ? 'Queue is full' : ui.pop >= ui.cap && a.k === 't' ? 'Build more houses' : 'Not enough resources');
}

const mini = $('mini'), mctx = mini.getContext('2d');
function drawMini() {
  mctx.putImageData(new ImageData(new Uint8ClampedArray(mem(), wasm.minimap(), MAP * MAP * 4), MAP, MAP), 0, 0);
  const [x0, y0, x1, y1] = viewRect();
  mctx.strokeStyle = '#fff'; mctx.lineWidth = 1.5; mctx.strokeRect(x0, y0, x1 - x0, y1 - y0);
}
function miniWorld(e) { const r = mini.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * MAP, (e.clientY - r.top) / r.height * MAP]; }
mini.addEventListener('mousedown', e => {
  const [x, y] = miniWorld(e);
  if (e.button === 2) wasm.cmd_context(x, y, 0); else { cam.x = x; cam.y = y; mini.dragging = true; }
});
addEventListener('mousemove', e => { if (mini.dragging) { [cam.x, cam.y] = miniWorld(e); clampCam(); } });
addEventListener('mouseup', () => mini.dragging = false);
mini.addEventListener('contextmenu', e => e.preventDefault());

// ---------------------------------------------------------------- input
const box = $('box');
canvas.addEventListener('contextmenu', e => e.preventDefault());
canvas.addEventListener('mousedown', e => {
  mouse.x = e.clientX; mouse.y = e.clientY;
  if (!running) return;
  const [wx, wy] = toWorld(e.clientX, e.clientY);
  if (e.button === 1) { mouse.pan = true; e.preventDefault(); return; }
  if (e.button === 2) {
    if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; return; }
    wasm.cmd_context(wx, wy, 0);
    return;
  }
  if (ghost) {
    const [tx, ty] = ghostTile();
    if (wasm.cmd_place(ghost.id, tx, ty)) { if (!e.shiftKey) ghost = null; } else toast('Cannot build there');
    return;
  }
  if (attackMode) { wasm.cmd_context(wx, wy, 1); attackMode = false; canvas.style.cursor = 'default'; return; }
  mouse.down = true; mouse.drag = false; mouse.sx = e.clientX; mouse.sy = e.clientY;
});
addEventListener('mousemove', e => {
  if (mouse.pan) { cam.x -= e.movementX / cam.zoom; cam.y -= e.movementY / cam.zoom; clampCam(); }
  mouse.x = e.clientX; mouse.y = e.clientY; mouse.inside = true;
  if (mouse.down) {
    if (Math.abs(mouse.x - mouse.sx) + Math.abs(mouse.y - mouse.sy) > 6) mouse.drag = true;
    if (mouse.drag) {
      box.style.display = 'block';
      box.style.left = Math.min(mouse.x, mouse.sx) + 'px'; box.style.top = Math.min(mouse.y, mouse.sy) + 'px';
      box.style.width = Math.abs(mouse.x - mouse.sx) + 'px'; box.style.height = Math.abs(mouse.y - mouse.sy) + 'px';
    }
  }
});
document.addEventListener('mouseleave', () => mouse.inside = false);
addEventListener('mouseup', e => {
  if (e.button === 1) mouse.pan = false;
  if (e.button !== 0 || !mouse.down) return;
  mouse.down = false; box.style.display = 'none';
  const add = e.shiftKey ? 1 : 0;
  if (mouse.drag) {
    const [ax, ay] = toWorld(Math.min(mouse.x, mouse.sx), Math.min(mouse.y, mouse.sy));
    const [bx, by] = toWorld(Math.max(mouse.x, mouse.sx), Math.max(mouse.y, mouse.sy));
    wasm.select_rect(ax, ay, bx, by, add);
  } else {
    const [wx, wy] = toWorld(mouse.x, mouse.y);
    const now = performance.now(), dbl = now - lastClick < 300; lastClick = now;
    wasm.select_point(wx, wy, add, dbl ? 1 : 0, ...viewRect());
  }
  lastUi = 0;
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  const [wx, wy] = toWorld(e.clientX, e.clientY);
  cam.zoom *= Math.exp(-e.deltaY * 0.0012); clampCam();
  // keep the point under the cursor fixed
  cam.x = wx - (e.clientX - innerWidth / 2) / cam.zoom; cam.y = wy - (e.clientY - innerHeight / 2) / cam.zoom; clampCam();
}, { passive: false });

function setSpeed(i) { speed = SPEEDS[Math.min(SPEEDS.length - 1, Math.max(0, i))]; $('speed').textContent = speed + '×'; }
function togglePause() { paused = !paused; $('pause').textContent = paused ? 'Resume' : 'Pause'; }
function jumpToSelection() { const x = wasm.sel_center(0); if (x >= 0) { cam.x = x; cam.y = wasm.sel_center(1); } }
function idleWorker() { const i = wasm.select_idle_worker(); if (i !== 0xffffffff) { cam.x = wasm.ent_x(i); cam.y = wasm.ent_y(i); lastUi = 0; } }

addEventListener('keydown', e => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
  keys.add(e.key);
  if (!running) return;
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (e.code.startsWith('Digit')) {
    const n = +e.code[5];
    if (e.ctrlKey) { wasm.group_set(n); toast(`Group ${n} set`); }
    else if (wasm.group_get(n)) { const now = performance.now(); if (lastGroup.n === n && now - lastGroup.t < 400) jumpToSelection(); lastGroup = { n, t: now }; lastUi = 0; }
    e.preventDefault(); return;
  }
  if (e.ctrlKey || e.metaKey) return;
  const hi = HOT.indexOf(k);
  if (hi >= 0 && ui && ui.act[hi]) { doAct(hi); return; }
  switch (k) {
    case 'A': attackMode = true; ghost = null; canvas.style.cursor = 'crosshair'; break;
    case 'S': wasm.cmd_stop(); break;
    case 'Delete': wasm.cmd_delete(); break;
    case 'Backspace': wasm.cmd_cancel(); break;
    case 'Escape': if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; } else wasm.select_rect(-1, -1, -1, -1, 0); lastUi = 0; break;
    case 'F': idleWorker(); break;
    case 'H': cam.x = wasm.start_x(); cam.y = wasm.start_y(); break;
    case ' ': jumpToSelection(); e.preventDefault(); break;
    case 'P': togglePause(); break;
    case '+': case '=': setSpeed(SPEEDS.indexOf(speed) + 1); break;
    case '-': setSpeed(SPEEDS.indexOf(speed) - 1); break;
    case 'F9': wasm.grant(1000); toast('+1000 resources (sandbox cheat)'); break;
  }
});
addEventListener('keyup', e => keys.delete(e.key));
addEventListener('blur', () => { keys.clear(); mouse.inside = false; });

$('idle').onclick = idleWorker;
$('pause').onclick = togglePause;
$('speed').onclick = () => setSpeed((SPEEDS.indexOf(speed) + 1) % SPEEDS.length);
const toMenu = () => { running = false; $('menu').style.display = 'flex'; $('end').style.display = 'none'; };
$('quit').onclick = toMenu; $('again').onclick = toMenu;
const seedOf = () => { const v = parseInt($('seed').value, 10); return Number.isFinite(v) && v > 0 ? v : (Math.random() * 2 ** 31) | 0 || 1; };
$('play').onclick = () => start(+$('nai').value, seedOf(), 0);
for (const b of document.querySelectorAll('[data-bench]')) b.onclick = () => start(1, seedOf(), +b.dataset.bench);

// ?bench=2000 or ?seed=42 start immediately (handy for profiling and screenshots)
const qs = new URLSearchParams(location.search);
if (qs.has('bench')) start(1, +qs.get('seed') || 7, +qs.get('bench') || 2000);
else if (qs.has('seed')) start(+qs.get('ai') || 1, +qs.get('seed') || 1, 0);
// ?warm=600 fast-forwards that many ticks; ?zoom= ?x= ?y= place the camera
for (let i = +qs.get('warm') || 0; i > 0 && running; i--) wasm.tick();
for (const k of ['zoom', 'x', 'y']) if (qs.has(k)) cam[k] = +qs.get(k);
window.openepoch = { wasm, cam, start };
requestAnimationFrame(frame);
