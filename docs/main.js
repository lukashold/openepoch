// OpenEpoch front end: input, camera, WebGL2 drawing, HUD, sound, saves, networking.
// The simulation lives in openepoch.wasm (Rust); this file never touches game rules.
import { buildAtlas, tintAtlas, CELL } from './atlas.js';
import { buildModels } from './models.js';
import * as sfx from './audio.js';
import { Lockstep, host as rtcHost, join as rtcJoin } from './net.js';

const $ = id => document.getElementById(id);
const canvas = $('gl');
const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, powerPreference: 'high-performance' });
if (!gl) { document.body.innerHTML = '<p style="padding:40px">OpenEpoch needs WebGL2. Please use a current browser.</p>'; throw new Error('no webgl2'); }

const MAP = 256, TICK = 0.05, STRIDE = 12;
const FOV = 35 * Math.PI / 180, PITCH = 55 * Math.PI / 180, TANF = Math.tan(FOV / 2);
const TEAM = [[0.27, 0.59, 1.0], [0.94, 0.27, 0.24], [0.31, 0.86, 0.39], [0.96, 0.82, 0.27], [1, 1, 1], [1, 1, 1], [1, 1, 1], [0.3, 0.75, 1]];

// ---------------------------------------------------------------- wasm
const wasmUrl = new URL('openepoch.wasm', import.meta.url);
let wasm;
try {
  wasm = (await WebAssembly.instantiateStreaming(fetch(wasmUrl), {})).instance.exports;
} catch {
  wasm = (await WebAssembly.instantiate(await (await fetch(wasmUrl)).arrayBuffer(), {})).instance.exports;
}
// Always take the pointer BEFORE touching memory.buffer: the call may grow (and detach) the memory.
const u8 = (ptr, len) => new Uint8Array(wasm.memory.buffer, ptr, len);
const f32 = (ptr, len) => new Float32Array(wasm.memory.buffer, ptr, len);
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
const FOG = `float fogAt(vec2 w){ return texture(uFog, w/${MAP}.).r; }
vec3 fogged(vec3 c, float f){ return c*smoothstep(.02,.45,f)*mix(.5,1.,smoothstep(.55,.95,f)); }`;
const SUN = 'const vec3 SUN = vec3(0.42, 0.82, 0.39);';

const terrain = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; uniform mat4 uVP; out vec3 vP; out vec3 vN;
void main(){ vP = aP; vN = aN; gl_Position = uVP*vec4(aP,1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog, uTerr; uniform vec3 uTeam[8]; uniform float uPx; in vec3 vP; in vec3 vN; out vec4 o;
${NOISE}${FOG}${SUN}
void main(){
  vec2 w = vP.xz, uv = w/${MAP}.;
  vec4 m = texture(uMap, uv);
  float n = noise(w*2.7)*.6 + noise(w*9.)*.4;
  vec3 c = m.rgb*(0.84 + 0.3*n);
  float lit = .55 + .6*max(dot(normalize(vN), SUN), 0.);
  c *= lit;
  c *= mix(1., .5, clamp(-vP.y*.9, 0., 1.));                       // sea bed fades with depth
  // territory borders
  vec4 t = texture(uTerr, uv);
  float d = max(.09, uPx*1.3)/${MAP}.;
  float e = 0.;
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(d,0.)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv+vec2(0.,d)).r - t.r)));
  e = max(e, step(.001, abs(texture(uTerr, uv-vec2(0.,d)).r - t.r)));
  int owner = int(t.g*255.+.5);
  vec3 tc = owner > 0 ? uTeam[owner-1] : vec3(.9);
  c = mix(c, tc, owner > 0 ? .06 + e*.7 : e*.2);
  o = vec4(fogged(c, fogAt(w)), 1.);
}`);

const water = program(`#version 300 es
layout(location=0) in vec2 aP; uniform mat4 uVP; out vec2 vW;
void main(){ vW = aP; gl_Position = uVP*vec4(aP.x, 0., aP.y, 1.); }`,
`#version 300 es
precision highp float;
uniform sampler2D uMap, uFog; uniform float uTime; in vec2 vW; out vec4 o;
${NOISE}${FOG}
void main(){
  vec2 uv = vW/${MAP}.;
  if (uv.x<0.||uv.y<0.||uv.x>1.||uv.y>1.) { o = vec4(.012,.03,.05,1.); return; }
  vec4 m = texture(uMap, uv);
  float wv = noise(vW*1.3 + uTime*vec2(.35,.22))*.6 + noise(vW*3.1 - uTime*vec2(.2,.4))*.4;
  vec3 c = mix(vec3(.05,.2,.36), vec3(.1,.42,.58), smoothstep(0.,.3,m.a))*(.75+.5*wv) + vec3(.9,.95,1.)*smoothstep(.7,.9,wv)*.14;
  float foam = smoothstep(.1,.36,m.a)*(1.-smoothstep(.36,.6,m.a));
  c += foam*(.2 + .14*sin(uTime*1.7 + wv*9.));
  float a = (1.-smoothstep(.4,.72,m.a))*.74;
  o = vec4(fogged(c, fogAt(vW))*a, a);
}`);

const INST = `layout(location=3) in vec4 iA; layout(location=4) in vec4 iB; layout(location=5) in vec4 iC;`;
const models = program(`#version 300 es
layout(location=0) in vec3 aP; layout(location=1) in vec3 aN; layout(location=2) in vec4 aCol; ${INST}
uniform mat4 uVP; out vec3 vN; out vec4 vCol; out vec2 vW; out float vLy; flat out vec4 vB;
void main(){
  float c=cos(iA.w), s=sin(iA.w); vec3 p=aP*iC.x;
  vec3 w = vec3(iA.x + p.x*c - p.z*s, iA.z + p.y, iA.y + p.x*s + p.z*c);
  vN = vec3(aN.x*c - aN.z*s, aN.y, aN.x*s + aN.z*c);
  vCol = aCol; vW = w.xz; vLy = aP.y; vB = iB;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8];
in vec3 vN; in vec4 vCol; in vec2 vW; in float vLy; flat in vec4 vB; out vec4 o;
${FOG}${SUN}
void main(){
  int fl = int(vB.w+.5);
  vec3 c = vCol.a > .5 ? uTeam[int(vB.y+.5)]*vCol.r : vCol.rgb;
  float a = 1.;
  vec3 n = normalize(vN);
  c *= .5 + .62*max(dot(n, SUN), 0.) + .12*max(n.y, 0.);
  if ((fl&64)!=0 && vB.z < 1.) {                                   // under construction: the building rises
    if (vLy > .03 + vB.z*.95) discard;
    c = mix(vec3(dot(c, vec3(.33)))*vec3(1.,.9,.7), c, vB.z);
  }
  if ((fl&4)!=0) c = mix(c, vec3(1.), .5);
  if ((fl&8)!=0) { c = mix(c, vec3(.3,1.,.4), .4); a = .6; }
  if ((fl&16)!=0) { c = mix(c, vec3(1.,.15,.15), .65); a = .6; }
  if ((fl&32)==0) { float f = fogAt(vW); c = fogged(c, f); if (f < .05) discard; }
  o = vec4(c*a, a);
}`);

const quads = program(`#version 300 es
layout(location=0) in vec2 aC; ${INST}
uniform mat4 uVP; uniform vec3 uRight, uUp; out vec2 vL; out vec2 vW; flat out vec4 vB;
void main(){
  vec3 base = vec3(iA.x, iA.z, iA.y); float s = iC.x;
  bool flat_ = iB.x < 240.5 || iB.x > 242.5;                       // rings and shadows lie on the ground
  vec3 w = flat_ ? base + vec3(aC.x, 0., aC.y)*s : base + (uRight*aC.x + uUp*aC.y)*s;
  vL = aC; vW = base.xz; vB = iB;
  gl_Position = uVP*vec4(w,1.);
}`,
`#version 300 es
precision highp float;
uniform sampler2D uFog; uniform vec3 uTeam[8]; uniform float uPx;
in vec2 vL; in vec2 vW; flat in vec4 vB; out vec4 o;
${FOG}
void main(){
  int team = int(vB.y+.5); float r = length(vL)*2.; vec4 col;
  if (vB.x < 240.5) {                                              // selection ring
    float w = clamp(uPx*1.6, .03, .2);
    float a = smoothstep(.86-w,.86-w*.5,r)*(1.-smoothstep(.86+w*.5,.86+w,r));
    col = vec4(mix(vec3(.6,1.,.6), uTeam[team], .35),1.)*a*.95;
  } else if (vB.x < 241.5) {                                       // health / progress bar
    float y = abs(vL.y), x = vL.x+.5;
    if (y > .075) discard;
    vec3 c = team==7 ? vec3(.3,.75,1.) : mix(vec3(.95,.2,.15), vec3(.25,.9,.3), smoothstep(.2,.7,vB.z));
    if (x > vB.z) c = vec3(.1);
    if (y > .045) c = vec3(0.);
    col = vec4(c,1.);
  } else if (vB.x < 242.5) {                                       // explosion
    float t = vB.z, q = r/(.3+.7*t);
    float a = (1.-smoothstep(.55,1.,q))*(1.-t*t);
    vec3 c = mix(vec3(1.,.95,.7), vec3(1.,.4,.05), clamp(q*.7+t*.6,0.,1.));
    col = vec4(mix(c, vec3(.15), smoothstep(.5,1.,t))*a, a*.7);
  } else {                                                         // aircraft shadow
    float a = (1.-smoothstep(.5,1.,r))*.34;
    col = vec4(0.,0.,0.,a)*smoothstep(.3,.9,fogAt(vW));
  }
  o = col;
}`);

function tex(unit, filter) {
  const t = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
tex(1, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);   // terrain colours
tex(2, gl.LINEAR); gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, MAP, MAP, 0, gl.RED, gl.UNSIGNED_BYTE, null);       // fog of war
tex(3, gl.NEAREST); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, MAP, MAP, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);  // territories

// 2D icons for the HUD come from the sprite atlas; the world itself is 3D
const iconAtlas = tintAtlas(buildAtlas(), TEAM[0].map(v => v * 255));

// terrain mesh: (MAP+1)^2 vertices, heights filled in per game
const HN = MAP + 1;
const vaoT = gl.createVertexArray(); gl.bindVertexArray(vaoT);
const terrBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf);
gl.bufferData(gl.ARRAY_BUFFER, HN * HN * 24, gl.DYNAMIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
{
  const idx = new Uint32Array(MAP * MAP * 6);
  for (let y = 0, k = 0; y < MAP; y++) for (let x = 0; x < MAP; x++) {
    const a = y * HN + x, b = a + 1, c = a + HN, d = c + 1;
    idx[k++] = a; idx[k++] = c; idx[k++] = b; idx[k++] = b; idx[k++] = c; idx[k++] = d;
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
}
let heights = new Float32Array(HN * HN);
function uploadTerrain() {
  heights = f32(wasm.hmap_ptr(), HN * HN).slice();
  const v = new Float32Array(HN * HN * 6), h = (x, y) => heights[Math.min(MAP, Math.max(0, y)) * HN + Math.min(MAP, Math.max(0, x))];
  for (let y = 0, k = 0; y < HN; y++) for (let x = 0; x < HN; x++) {
    const nx = h(x - 1, y) - h(x + 1, y), nz = h(x, y - 1) - h(x, y + 1), l = Math.hypot(nx, 2, nz);
    v[k++] = x; v[k++] = h(x, y); v[k++] = y; v[k++] = nx / l; v[k++] = 2 / l; v[k++] = nz / l;
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, terrBuf); gl.bufferSubData(gl.ARRAY_BUFFER, 0, v);
}
const groundAt = (x, y) => {
  x = Math.min(MAP - 0.001, Math.max(0, x)); y = Math.min(MAP - 0.001, Math.max(0, y));
  const xi = x | 0, yi = y | 0, fx = x - xi, fy = y - yi, i = yi * HN + xi;
  const a = heights[i] + (heights[i + 1] - heights[i]) * fx, b = heights[i + HN] + (heights[i + HN + 1] - heights[i + HN]) * fx;
  return Math.max(0, a + (b - a) * fy);
};

const vaoW = gl.createVertexArray(); gl.bindVertexArray(vaoW);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-400, -400, 656, -400, -400, 656, 656, 656]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

const instBuf = gl.createBuffer();
function instanced(vao) {
  gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
  for (const loc of [3, 4, 5]) { gl.enableVertexAttribArray(loc); gl.vertexAttribDivisor(loc, 1); }
}
/** Point the instance attributes at instance `first` (WebGL2 has no base-instance draw). */
function instOffset(first) {
  for (const loc of [3, 4, 5]) gl.vertexAttribPointer(loc, 4, gl.FLOAT, false, STRIDE * 4, first * STRIDE * 4 + (loc - 3) * 16);
}
const mesh = buildModels();
const vaoM = gl.createVertexArray(); gl.bindVertexArray(vaoM);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer()); gl.bufferData(gl.ARRAY_BUFFER, mesh.data, gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, mesh.stride, 0);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, mesh.stride, 12);
gl.enableVertexAttribArray(2); gl.vertexAttribPointer(2, 4, gl.FLOAT, false, mesh.stride, 24);
instanced(vaoM);
const vaoQ = gl.createVertexArray(); gl.bindVertexArray(vaoQ);
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-.5, -.5, .5, -.5, -.5, .5, .5, .5]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
instanced(vaoQ);

gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
const teamFlat = TEAM.flat();
for (const pr of [terrain, water, models, quads]) {
  gl.useProgram(pr.p);
  if (pr.u.uMap) gl.uniform1i(pr.u.uMap, 1);
  if (pr.u.uFog) gl.uniform1i(pr.u.uFog, 2);
  if (pr.u.uTerr) gl.uniform1i(pr.u.uTerr, 3);
  if (pr.u.uTeam) gl.uniform3fv(pr.u.uTeam, teamFlat);
}

// ---------------------------------------------------------------- state
const cam = { x: 128, y: 128, zoom: 28 };
const mouse = { x: 0, y: 0, down: false, sx: 0, sy: 0, drag: false, pan: false, inside: false };
const keys = new Set();
let running = false, paused = false, speed = 1, acc = 0, last = 0, bench = 0;
let net = null;          // { ls: Lockstep, ch: RTCDataChannel } in multiplayer
let ghost = null;        // { id, size } while placing a building
let attackMode = false;  // A pressed: next left click is attack-move
let ui = null, actSig = '', lastGroup = { n: -1, t: 0 }, lastClick = 0;
let simMs = 0, frameMs = 16.7, worstMs = 0, fogTick = -1, lastUi = 0, lastMini = 0, lastAuto = 0, waitSince = 0;
const SPEEDS = [0.5, 1, 2, 4];

// ---------------------------------------------------------------- camera
// The eye sits south of and above the look-at point, so screen-up is map-north.
const view = { eye: [0, 0, 0], f: [0, -Math.sin(PITCH), -Math.cos(PITCH)], u: [0, Math.cos(PITCH), -Math.sin(PITCH)], vp: new Float32Array(16) };
function updateCamera() {
  cam.zoom = Math.min(80, Math.max(6, cam.zoom));
  cam.x = Math.min(MAP, Math.max(0, cam.x)); cam.y = Math.min(MAP, Math.max(0, cam.y));
  const aspect = innerWidth / innerHeight, dist = innerHeight / 2 / (cam.zoom * TANF);
  const e = view.eye = [cam.x, dist * Math.sin(PITCH), cam.y + dist * Math.cos(PITCH)];
  const f = view.f, u = view.u, near = Math.max(0.5, dist * 0.08), far = dist * 3.5 + 120, t = 1 / TANF;
  const tx = -e[0], ty = -(u[1] * e[1] + u[2] * e[2]), tz = f[1] * e[1] + f[2] * e[2];
  // projection * view, written out (right = +x, so the view matrix is sparse)
  const A = (far + near) / (near - far), B = 2 * far * near / (near - far), m = view.vp;
  m[0] = t / aspect; m[1] = 0; m[2] = 0; m[3] = 0;
  m[4] = 0; m[5] = t * u[1]; m[6] = -A * f[1]; m[7] = f[1];
  m[8] = 0; m[9] = t * u[2]; m[10] = -A * f[2]; m[11] = f[2];
  m[12] = t / aspect * tx; m[13] = t * ty; m[14] = A * tz + B; m[15] = -tz;
  if (running) wasm.set_eye(e[0], e[1], e[2]);   // so clicks land on what is drawn, even on aircraft
}
function ray(px, py) {
  const nx = (px / innerWidth * 2 - 1) * TANF * innerWidth / innerHeight, ny = (1 - py / innerHeight * 2) * TANF;
  return [nx, view.f[1] + view.u[1] * ny, view.f[2] + view.u[2] * ny];
}
/** Screen pixel -> point on the y = 0 plane. */
function toPlane(px, py) {
  const d = ray(px, py), e = view.eye, t = -e[1] / Math.min(d[1], -1e-4);
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
/** Screen pixel -> point on the actual terrain (ray-marched against the height map). */
function toWorld(px, py) {
  const d = ray(px, py), e = view.eye, dy = Math.min(d[1], -1e-4);
  let t = (3.2 - e[1]) / dy, prev = t;
  const end = -e[1] / dy;
  for (; t < end; prev = t, t += 0.2) if (e[1] + dy * t <= groundAt(e[0] + d[0] * t, e[2] + d[2] * t)) break;
  t = Math.min(t, end);
  for (let i = 0; i < 6; i++) { const m = (prev + t) / 2; if (e[1] + dy * m <= groundAt(e[0] + d[0] * m, e[2] + d[2] * m)) t = m; else prev = m; }
  return [e[0] + d[0] * t, e[2] + d[2] * t];
}
const corners = () => [toPlane(0, 0), toPlane(innerWidth, 0), toPlane(innerWidth, innerHeight), toPlane(0, innerHeight)];
function viewRect() {
  const c = corners();
  return [Math.min(...c.map(p => p[0])) - 2, Math.min(...c.map(p => p[1])) - 2, Math.max(...c.map(p => p[0])) + 2, Math.max(...c.map(p => p[1])) + 6];
}

function toast(msg) { const t = $('toast'); t.textContent = msg; t.style.opacity = 1; clearTimeout(toast.h); toast.h = setTimeout(() => t.style.opacity = 0, 2200); }
function ghostTile() {
  const [wx, wy] = toWorld(mouse.x, mouse.y);
  return [Math.round(wx - ghost.size / 2), Math.round(wy - ghost.size / 2)];
}

// ---------------------------------------------------------------- starting, saving, loading
function begin() {
  ghost = null; attackMode = false; paused = false; speed = 1; acc = 0; fogTick = -1; actSig = ''; worstMs = 0; ui = null;
  $('speed').textContent = '1×'; $('pause').textContent = 'Pause';
  for (const id of ['speed', 'pause', 'save']) $(id).style.display = net ? 'none' : '';
  $('menu').style.display = 'none'; $('end').style.display = 'none';
  uploadTerrain();
  wasm.events_clear();
  running = true; last = performance.now(); lastAuto = last;
}
function start(nAi, seed, benchUnits, humans = 1, local = 0) {
  bench = benchUnits;
  wasm.init(seed >>> 0, nAi, benchUnits, humans, local);
  cam.x = benchUnits ? MAP / 2 : wasm.start_x(); cam.y = benchUnits ? MAP / 2 : wasm.start_y(); cam.zoom = benchUnits ? 11 : 28;
  begin();
}

const db = () => new Promise((res, rej) => {
  const r = indexedDB.open('openepoch', 1);
  r.onupgradeneeded = () => r.result.createObjectStore('saves');
  r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error);
});
const dbDo = async (mode, fn) => { const d = await db(); return new Promise((res, rej) => { const rq = fn(d.transaction('saves', mode).objectStore('saves')); rq.onsuccess = () => res(rq.result); rq.onerror = () => rej(rq.error); }); };
async function saveGame(silent) {
  if (!running || bench || net) { if (!silent) toast('Saving is only available in single-player games'); return; }
  const ptr = wasm.save(), bytes = u8(ptr, wasm.save_len()).slice();
  try {
    await dbDo('readwrite', s => s.put({ bytes, cam: { ...cam }, when: Date.now(), epoch: ui ? ui.ename : '' }, 'slot1'));
    if (!silent) toast(`Game saved (${(bytes.length / 1024) | 0} KB)`);
    refreshContinue();
  } catch (e) { if (!silent) toast('Could not save: ' + e.message); }
}
async function loadGame() {
  const s = await dbDo('readonly', st => st.get('slot1')).catch(() => null);
  if (!s) return toast('No saved game found');
  const ptr = wasm.load_alloc(s.bytes.length);
  u8(ptr, s.bytes.length).set(s.bytes);
  if (!wasm.load_apply()) return toast('That save is from an older version and cannot be loaded');
  net = null; bench = 0; Object.assign(cam, s.cam);
  begin(); toast('Game loaded');
}
async function refreshContinue() {
  const s = await dbDo('readonly', st => st.get('slot1')).catch(() => null);
  $('continue').style.display = s ? '' : 'none';
  if (s) $('continue').textContent = `Continue (${s.epoch || 'saved game'}, ${new Date(s.when).toLocaleString()})`;
}
refreshContinue();

// ---------------------------------------------------------------- simulation stepping & sound
function advance(dt, now) {
  if (net) {
    acc = Math.min(acc + dt, TICK * 6);                 // allow a short catch-up burst, never a spiral
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8 && net.ls.step()) { acc -= TICK; steps++; }
    if (steps) { simMs += ((performance.now() - t0) / steps - simMs) * 0.1; waitSince = now; }
  } else if (!paused) {
    acc += dt * speed;
    let steps = 0; const t0 = performance.now();
    while (acc >= TICK && steps < 8) { wasm.tick(); acc -= TICK; steps++; }
    if (steps) simMs += ((performance.now() - t0) / steps - simMs) * 0.1;
    if (acc > TICK) acc = TICK;                          // drop time instead of stalling the frame
  }
  // sound cues the simulation queued up
  const n = wasm.events_len();
  if (n) {
    const ev = f32(wasm.events_ptr(), n), range = 26 + 700 / cam.zoom;
    for (let i = 0; i < n; i += 3) {
      const kind = ev[i], dx = ev[i + 1] - cam.x, dy = ev[i + 2] - cam.y;
      if (kind >= 12) sfx.play(kind, 1, 0);
      else { const v = Math.max(0, 1 - Math.hypot(dx, dy) / range); sfx.play(kind, v * v * Math.min(1, 0.35 + cam.zoom / 40), dx / range * 1.4); }
    }
    wasm.events_clear();
  }
}
// A hidden tab gets no animation frames, but a multiplayer peer must keep simulating.
setInterval(() => {
  if (!running || !net || !document.hidden) return;
  const now = performance.now(), dt = Math.min(1.5, (now - last) / 1000); last = now;
  for (let t = dt; t > 0; t -= 0.3) advance(Math.min(t, 0.3), now);
}, 200);

// ---------------------------------------------------------------- frame loop
function frame(now) {
  const dt = Math.min(0.1, (now - last) / 1000); last = now;
  if (!running) return;
  frameMs += (dt * 1000 - frameMs) * 0.05;
  worstMs = Math.max(worstMs * 0.995, dt * 1000);

  const pan = 22 / cam.zoom * 60 * dt * (keys.has('Shift') ? 2 : 1);
  if (keys.has('ArrowLeft') || (mouse.inside && mouse.x < 4)) cam.x -= pan;
  if (keys.has('ArrowRight') || (mouse.inside && mouse.x > innerWidth - 5)) cam.x += pan;
  if (keys.has('ArrowUp') || (mouse.inside && mouse.y < 4)) cam.y -= pan;
  if (keys.has('ArrowDown') || (mouse.inside && mouse.y > innerHeight - 5)) cam.y += pan;
  updateCamera();

  advance(dt, now);
  const alpha = paused || (net && net.ls.waiting) ? 1 : Math.min(1, acc / TICK);
  if (!net && !bench && now - lastAuto > 180000) { lastAuto = now; saveGame(true); }

  const dpr = Math.min(2, devicePixelRatio || 1);
  const w = Math.round(innerWidth * dpr), h = Math.round(innerHeight * dpr);
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  gl.viewport(0, 0, w, h);

  // textures that change
  if (wasm.map_dirty()) { gl.activeTexture(gl.TEXTURE1); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(wasm.map_ptr(), MAP * MAP * 4)); }
  if (wasm.terr_dirty()) { const p = wasm.terr_ptr(); gl.activeTexture(gl.TEXTURE3); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RGBA, gl.UNSIGNED_BYTE, u8(p, MAP * MAP * 4)); }
  const tickNow = ui ? ui.tick : 0;
  if (fogTick !== tickNow || fogTick < 0) {
    fogTick = tickNow;
    gl.activeTexture(gl.TEXTURE2); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MAP, MAP, gl.RED, gl.UNSIGNED_BYTE, u8(wasm.fog_ptr(), MAP * MAP));
  }

  gl.clearColor(0.012, 0.03, 0.05, 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  gl.enable(gl.DEPTH_TEST); gl.depthMask(true); gl.disable(gl.BLEND);
  const px = 1 / cam.zoom;

  gl.useProgram(terrain.p); gl.bindVertexArray(vaoT);
  gl.uniformMatrix4fv(terrain.u.uVP, false, view.vp); gl.uniform1f(terrain.u.uPx, px);
  gl.drawElements(gl.TRIANGLES, MAP * MAP * 6, gl.UNSIGNED_INT, 0);

  const [x0, y0, x1, y1] = viewRect();
  let gid = -1, gx = 0, gy = 0;
  if (ghost) { gid = ghost.id; [gx, gy] = ghostTile(); }
  const n = wasm.render(alpha, x0, y0, x1, y1, gid, gx, gy);
  const ranges = n ? new Uint32Array(wasm.memory.buffer, wasm.ranges_ptr(), 512) : null;
  if (n) {
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, f32(wasm.inst_ptr(), n * STRIDE), gl.STREAM_DRAW);
    // one instanced draw per model type
    gl.enable(gl.BLEND);
    gl.useProgram(models.p); gl.bindVertexArray(vaoM); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.uniformMatrix4fv(models.u.uVP, false, view.vp);
    for (let id = 0; id < 240; id++) {
      const cnt = ranges[id * 2 + 1];
      if (!cnt || !mesh.count[id]) continue;
      instOffset(ranges[id * 2]);
      gl.drawArraysInstanced(gl.TRIANGLES, mesh.first[id], mesh.count[id], cnt);
    }
  }

  // translucent water over the sea bed and anything submerged
  gl.enable(gl.BLEND); gl.depthMask(false);
  gl.useProgram(water.p); gl.bindVertexArray(vaoW);
  gl.uniformMatrix4fv(water.u.uVP, false, view.vp); gl.uniform1f(water.u.uTime, now / 1000);
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

  if (n) {
    gl.useProgram(quads.p); gl.bindVertexArray(vaoQ); gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.uniformMatrix4fv(quads.u.uVP, false, view.vp); gl.uniform1f(quads.u.uPx, px);
    gl.uniform3f(quads.u.uRight, 1, 0, 0); gl.uniform3fv(quads.u.uUp, view.u);
    for (const id of [243, 240, 242, 241]) {            // shadows, rings, explosions, then bars on top of everything
      const cnt = ranges[id * 2 + 1];
      if (!cnt) continue;
      if (id === 241) gl.disable(gl.DEPTH_TEST);
      instOffset(ranges[id * 2]);
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, cnt);
    }
  }

  if (now - lastUi > 100) { lastUi = now; updateUi(n, now); }
  if (now - lastMini > 250) { lastMini = now; drawMini(); }
}

// ---------------------------------------------------------------- HUD
const HOT = ['Q', 'W', 'E', 'R', 'T', 'Z', 'X', 'C', 'V', 'B'];
function icon(id, px) {
  const c = document.createElement('canvas'); c.width = c.height = px;
  c.getContext('2d').drawImage(iconAtlas, (id % 16) * CELL, ((id / 16) | 0) * CELL, CELL, CELL, 0, 0, px, px);
  return c;
}
const glyph = (ch, px) => { const d = document.createElement('div'); d.style.cssText = `font-size:${px * 0.66}px;height:${px}px;line-height:${px}px`; d.textContent = ch; return d; };
const GLYPH = { e: '⬆', r: '⚗', u: '⚓' };
function costText(c) { return [['Food', c[0]], ['Wood', c[1]], ['Metal', c[2]]].filter(x => x[1]).map(x => `${x[1]} ${x[0]}`).join(' · ') || 'free'; }

function updateUi(drawn, now) {
  const p = wasm.ui();
  ui = JSON.parse(dec.decode(u8(p, wasm.ui_len())));
  for (let i = 0; i < 3; i++) $('r' + i).textContent = ui.res[i];
  $('pop').textContent = `${ui.pop}/${ui.cap}`;
  $('epoch').textContent = ui.ename;
  $('terr').textContent = ui.terr;
  $('idle').textContent = `Idle workers: ${ui.idle}`;
  $('idle').style.visibility = ui.idle && !bench ? 'visible' : 'hidden';
  let status = `${Math.round(1000 / frameMs)} fps · worst ${worstMs.toFixed(0)} ms · sim ${simMs.toFixed(2)} ms/tick · ${ui.n} entities · ${drawn} drawn`;
  if (net) {
    if (net.ls.desync) status = '⚠ OUT OF SYNC with the other player · ' + status;
    else if (net.ls.alone) status = 'other player left · ' + status;
    else if (net.ls.waiting && now - waitSince > 600) status = 'waiting for the other player… · ' + status;
    else status = 'multiplayer · ' + status;
  }
  $('perf').textContent = status;

  const s = ui.sel, info = $('info');
  if (!s.n) info.innerHTML = bench ? '<h3>Stress test</h3><span class="dim">Two armies are kept at full strength and thrown at each other. Zoom and pan freely; you can still select and command the blue side.</span>'
    : `<span class="dim">Nothing selected</span>${ui.techs.length ? '<div class="types">' + ui.techs.map(t => `<span>${t}</span>`).join('') + '</div>' : ''}`;
  else {
    let h = `<h3>${s.n > 1 && s.types.length > 1 ? s.n + ' units' : s.name + (s.n > 1 ? ' ×' + s.n : '')}</h3>`;
    if (s.n === 1) {
      h += `<div class="hp"><i style="width:${100 * s.hp / s.max}%"></i></div>${s.hp} / ${s.max} HP`;
      if (s.dmg) h += ` · ${s.dmg} dmg · range ${s.rng}`;
      if (s.built < 1) h += `<br>Under construction: ${Math.round(s.built * 100)}%`;
    }
    if (s.cargomax) h += `<br>Passengers: ${s.cargo} / ${s.cargomax}`;
    if (s.types.length > 1) h += '<div class="types">' + s.types.map(t => `<span>${t[2]} ×${t[1]}</span>`).join('') + '</div>';
    if (s.q.length) h += `<div class="q" id="queue"></div><div class="hp"><i style="width:${s.qp * 100}%;background:#4aa3ff"></i></div>`;
    info.innerHTML = h;
    const q = $('queue');
    if (q) { for (const id of s.q) q.append(id === 255 ? glyph('⬆', 30) : id >= 200 ? glyph('⚗', 30) : icon(id, 30)); q.title = 'Right-click an action button or press Backspace to cancel the last item'; }
  }

  const sig = ui.act.map(a => a.k + a.id).join(',');
  const acts = $('acts');
  if (sig !== actSig) {
    actSig = sig; acts.innerHTML = '';
    ui.act.forEach((a, i) => {
      const b = document.createElement('button'); b.className = 'act'; b.dataset.i = i;
      b.append(GLYPH[a.k] ? glyph(GLYPH[a.k], 44) : icon(a.id, 44));
      const sm = document.createElement('small'); sm.textContent = a.name; b.append(sm);
      if (HOT[i]) { const k = document.createElement('kbd'); k.textContent = HOT[i]; b.append(k); }
      b.onclick = () => doAct(i);
      b.oncontextmenu = e => { e.preventDefault(); wasm.cmd_cancel(); };
      b.onmouseenter = () => { const a = ui.act[i]; if (!a) return; const t = $('tip'); t.innerHTML = `<b>${a.name}</b> <span class="c">${a.k === 'u' ? '' : costText(a.cost)}</span><br>${a.tip}`; t.style.display = 'block'; };
      b.onmouseleave = () => $('tip').style.display = 'none';
      acts.append(b);
    });
  }
  [...acts.children].forEach((b, i) => b.classList.toggle('no', !ui.act[i].ok));

  if (ui.win && !bench && $('end').style.display !== 'flex') {
    $('endtext').textContent = ui.win === 1 ? 'Victory!' : 'Defeat';
    $('end').style.display = 'flex';
    sfx.play(ui.win === 1 ? 13 : 11);
  }
}

function doAct(i) {
  const a = ui && ui.act[i];
  if (!a) return;
  if (a.k === 'b') { ghost = { id: a.id, size: a.size }; attackMode = false; sfx.play(20); return; }
  if (a.k === 'u') { wasm.cmd_unload(); sfx.play(22); return; }
  if (wasm.cmd_train(a.id)) sfx.play(20);
  else { sfx.play(21); toast(a.ok ? 'Queue is full' : a.k === 't' && ui.pop >= ui.cap ? 'Build more houses' : 'Not enough resources'); }
}

const mini = $('mini'), mctx = mini.getContext('2d');
function drawMini() {
  const p = wasm.minimap();
  mctx.putImageData(new ImageData(new Uint8ClampedArray(wasm.memory.buffer, p, MAP * MAP * 4), MAP, MAP), 0, 0);
  const c = corners();
  mctx.strokeStyle = '#fff'; mctx.lineWidth = 1.5; mctx.beginPath();
  c.forEach((q, i) => i ? mctx.lineTo(q[0], q[1]) : mctx.moveTo(q[0], q[1]));
  mctx.closePath(); mctx.stroke();
}
function miniWorld(e) { const r = mini.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * MAP, (e.clientY - r.top) / r.height * MAP]; }
mini.addEventListener('mousedown', e => {
  const [x, y] = miniWorld(e);
  if (e.button === 2) { wasm.cmd_context(x, y, 0); sfx.play(22); } else { cam.x = x; cam.y = y; mini.dragging = true; }
});
addEventListener('mousemove', e => { if (mini.dragging) [cam.x, cam.y] = miniWorld(e); });
addEventListener('mouseup', () => mini.dragging = false);
mini.addEventListener('contextmenu', e => e.preventDefault());

// ---------------------------------------------------------------- input
const box = $('box');
const PLACE_ERR = ['', 'Cannot build there', 'Outside your territory — claim it with a Town Center first', 'Explore the area first', 'Not enough resources'];
addEventListener('pointerdown', sfx.unlock, { once: false });
canvas.addEventListener('contextmenu', e => e.preventDefault());
canvas.addEventListener('mousedown', e => {
  mouse.x = e.clientX; mouse.y = e.clientY;
  if (!running) return;
  const [wx, wy] = toWorld(e.clientX, e.clientY);
  if (e.button === 1) { mouse.pan = true; e.preventDefault(); return; }
  if (e.button === 2) {
    if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; return; }
    wasm.cmd_context(wx, wy, 0);
    if (ui && ui.sel.n) sfx.play(22);
    return;
  }
  if (ghost) {
    const [tx, ty] = ghostTile(), err = wasm.place_error(ghost.id, tx, ty);
    if (!err && wasm.cmd_place(ghost.id, tx, ty)) { sfx.play(20); if (!e.shiftKey) ghost = null; } else { sfx.play(21); toast(PLACE_ERR[err] || PLACE_ERR[1]); }
    return;
  }
  if (attackMode) { wasm.cmd_context(wx, wy, 1); sfx.play(22); attackMode = false; canvas.style.cursor = 'default'; return; }
  mouse.down = true; mouse.drag = false; mouse.sx = e.clientX; mouse.sy = e.clientY;
});
addEventListener('mousemove', e => {
  if (mouse.pan) { cam.x -= e.movementX / cam.zoom; cam.y -= e.movementY / (cam.zoom * Math.sin(PITCH)); }
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
    // a screen rectangle is a trapezoid on the ground under a tilted camera
    const l = Math.min(mouse.x, mouse.sx), r = Math.max(mouse.x, mouse.sx), t = Math.min(mouse.y, mouse.sy), b = Math.max(mouse.y, mouse.sy);
    wasm.select_quad(...toWorld(l, t), ...toWorld(r, t), ...toWorld(r, b), ...toWorld(l, b), add);
  } else {
    const [wx, wy] = toWorld(mouse.x, mouse.y);
    const now = performance.now(), dbl = now - lastClick < 300; lastClick = now;
    wasm.select_point(wx, wy, add, dbl ? 1 : 0, ...viewRect());
  }
  lastUi = 0;
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  const before = toPlane(e.clientX, e.clientY);
  cam.zoom *= Math.exp(-e.deltaY * 0.0012); updateCamera();
  const after = toPlane(e.clientX, e.clientY);     // keep the point under the cursor fixed
  cam.x += before[0] - after[0]; cam.y += before[1] - after[1]; updateCamera();
}, { passive: false });

function setSpeed(i) { if (net) return; speed = SPEEDS[Math.min(SPEEDS.length - 1, Math.max(0, i))]; $('speed').textContent = speed + '×'; }
function togglePause() { if (net) return toast('No pausing in multiplayer'); paused = !paused; $('pause').textContent = paused ? 'Resume' : 'Pause'; }
function jumpToSelection() { const x = wasm.sel_center(0); if (x >= 0) { cam.x = x; cam.y = wasm.sel_center(1); } }
function idleWorker() { const i = wasm.select_idle_worker(); if (i !== -1 && i >>> 0 !== 0xffffffff) { cam.x = wasm.ent_x(i); cam.y = wasm.ent_y(i); lastUi = 0; } }
function toggleMute() { sfx.setMuted(!sfx.isMuted()); $('mute').textContent = sfx.isMuted() ? 'Sound off' : 'Sound on'; }

addEventListener('keydown', e => {
  if (['INPUT', 'SELECT', 'TEXTAREA'].includes(e.target.tagName)) return;
  keys.add(e.key);
  if (!running) return;
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (e.code.startsWith('Digit')) {
    const n = +e.code[5];
    if (e.ctrlKey) { wasm.group_set(n); toast(`Group ${n} set`); }
    else if (wasm.group_get(n)) { const now = performance.now(); if (lastGroup.n === n && now - lastGroup.t < 400) jumpToSelection(); lastGroup = { n, t: now }; lastUi = 0; }
    e.preventDefault(); return;
  }
  if ((e.ctrlKey || e.metaKey) && k === 'S') { e.preventDefault(); saveGame(); return; }
  if (e.ctrlKey || e.metaKey) return;
  const hi = HOT.indexOf(k);
  if (hi >= 0 && ui && ui.act[hi]) { doAct(hi); return; }
  switch (k) {
    case 'A': attackMode = true; ghost = null; canvas.style.cursor = 'crosshair'; break;
    case 'S': wasm.cmd_stop(); break;
    case 'U': wasm.cmd_unload(); break;
    case 'M': toggleMute(); break;
    case 'Delete': wasm.cmd_delete(); break;
    case 'Backspace': wasm.cmd_cancel(); break;
    case 'Escape': if (ghost || attackMode) { ghost = null; attackMode = false; canvas.style.cursor = 'default'; } else wasm.select_none(); lastUi = 0; break;
    case 'F': idleWorker(); break;
    case 'H': cam.x = wasm.start_x(); cam.y = wasm.start_y(); break;
    case ' ': jumpToSelection(); e.preventDefault(); break;
    case 'P': togglePause(); break;
    case '+': case '=': setSpeed(SPEEDS.indexOf(speed) + 1); break;
    case '-': setSpeed(SPEEDS.indexOf(speed) - 1); break;
    case 'F9': if (!net) { wasm.grant(1000); toast('+1000 resources (sandbox cheat)'); } break;
  }
});
addEventListener('keyup', e => keys.delete(e.key));
addEventListener('blur', () => { keys.clear(); mouse.inside = false; });

$('idle').onclick = idleWorker;
$('pause').onclick = togglePause;
$('mute').onclick = toggleMute;
$('save').onclick = () => saveGame();
$('speed').onclick = () => setSpeed((SPEEDS.indexOf(speed) + 1) % SPEEDS.length);
function toMenu() {
  if (running && !net && !bench) saveGame(true);
  if (net) { try { net.ch.close(); net.pc.close(); } catch { /* already gone */ } net = null; }
  running = false; $('menu').style.display = 'flex'; $('end').style.display = 'none'; $('mp').style.display = 'none';
}
$('quit').onclick = toMenu; $('again').onclick = toMenu;
const seedOf = () => { const v = parseInt($('seed').value, 10); return Number.isFinite(v) && v > 0 ? v : (Math.random() * 2 ** 31) | 0 || 1; };
$('play').onclick = () => { net = null; start(+$('nai').value, seedOf(), 0); };
$('continue').onclick = loadGame;
for (const b of document.querySelectorAll('[data-bench]')) b.onclick = () => { net = null; start(1, seedOf(), +b.dataset.bench); };

// ---------------------------------------------------------------- multiplayer lobby
const mpStatus = t => $('mpstatus').textContent = t;
let pending = null;
function channel(local) {
  let ch = null, pc = null;
  return {
    open(c) {
      ch = c;
      if (local === 0) {                                   // the host decides the map and tells the guest
        const cfg = { t: 'start', seed: seedOf(), ai: +$('mpai').value };
        ch.send(JSON.stringify(cfg)); launch(cfg);
      } else mpStatus('Connected — waiting for the host to start…');
    },
    message(d) {
      if (typeof d === 'string') { const m = JSON.parse(d); if (m.t === 'start') launch(m); }
      else if (net) net.ls.receive(d);
    },
    close() { if (net && !net.ls.alone) { net.ls.dropPeer(); toast('The other player disconnected'); } else if (!net) mpStatus('Connection closed.'); },
    setPc(p) { pc = p; },
  };
  function launch(cfg) {
    net = { ch, pc, ls: null };
    wasm.init(cfg.seed >>> 0, cfg.ai, 0, 2, local);
    net.ls = new Lockstep(wasm, local, buf => { if (ch.readyState === 'open') ch.send(buf); });
    bench = 0; cam.x = wasm.start_x(); cam.y = wasm.start_y(); cam.zoom = 28;
    begin(); toast(local === 0 ? 'You are blue. Good luck!' : 'You are red. Good luck!');
  }
}
$('mpopen').onclick = () => { $('mp').style.display = $('mp').style.display === 'block' ? 'none' : 'block'; };
$('mphost').onclick = async () => {
  mpStatus('Creating invite…');
  try {
    const on = channel(0); pending = await rtcHost(on); on.setPc(pending.pc);
    $('mpout').value = pending.code; $('mpin').value = ''; $('mpin').placeholder = 'Paste the reply code from the other player here'; $('mpgo').textContent = 'Connect';
    $('mpgo').onclick = async () => { try { await pending.accept($('mpin').value); mpStatus('Connecting…'); } catch { mpStatus('That reply code is not valid.'); } };
    mpStatus('1. Send the invite code below to the other player.  2. Paste their reply code and press Connect.');
  } catch (e) { mpStatus('WebRTC is not available: ' + e.message); }
};
$('mpjoin').onclick = () => {
  $('mpout').value = ''; $('mpin').value = ''; $('mpin').placeholder = 'Paste the invite code from the host here'; $('mpgo').textContent = 'Create reply code';
  mpStatus('Paste the host’s invite code, then press “Create reply code”.');
  $('mpgo').onclick = async () => {
    try {
      const on = channel(1); pending = await rtcJoin($('mpin').value, on); on.setPc(pending.pc);
      $('mpout').value = pending.code; mpStatus('Send the reply code below back to the host. The game starts when they connect.');
    } catch { mpStatus('That invite code is not valid.'); }
  };
};
$('mpcopy').onclick = () => { $('mpout').select(); navigator.clipboard?.writeText($('mpout').value).then(() => mpStatus('Copied to the clipboard.'), () => {}); };

// ?bench=2000 or ?seed=42 start immediately (handy for profiling and screenshots)
const qs = new URLSearchParams(location.search);
if (qs.has('bench')) start(1, +qs.get('seed') || 7, +qs.get('bench') || 2000);
else if (qs.has('seed')) start(+qs.get('ai') || 1, +qs.get('seed') || 1, 0);
// ?warm=600 fast-forwards that many ticks; ?zoom= ?x= ?y= place the camera
for (let i = +qs.get('warm') || 0; i > 0 && running; i--) wasm.tick();
for (const k of ['zoom', 'x', 'y']) if (qs.has(k)) cam[k] = +qs.get(k);
window.openepoch = { wasm, cam, gl, frame, start, saveGame, loadGame, toWorld, channel, get net() { return net; } };
const loop = now => { requestAnimationFrame(loop); frame(now); };
requestAnimationFrame(loop);
