// Procedural sprite atlas. Every sprite is drawn here with Canvas2D at start-up:
// the game ships no image files at all. Sprites are top-down and face +x.
// Pure magenta marks "team colour" pixels; the shader recolours them per player.

export const CELL = 128;
export const ATLAS = 2048;
const M = '#ff00ff', MD = '#b000b0', OUT = 'rgba(0,0,0,.6)';
const WOOD = '#8a5a2b', DWOOD = '#5d3b1a', STEEL = '#aab3bd', DSTEEL = '#4d555e', GUN = '#2d3136';
const SKIN = '#e0b088', SAIL = '#efe6cf', STONE = '#a9a498', DSTONE = '#7d786c';

function ell(c, x, y, rx, ry, f, s) {
  c.beginPath(); c.ellipse(x, y, rx, ry, 0, 0, 7);
  if (f) { c.fillStyle = f; c.fill(); }
  if (s) { c.strokeStyle = s; c.lineWidth = 2; c.stroke(); }
}
function rr(c, x, y, w, h, r, f, s) {
  c.beginPath(); c.roundRect(x, y, w, h, r);
  if (f) { c.fillStyle = f; c.fill(); }
  if (s) { c.strokeStyle = s; c.lineWidth = 2; c.stroke(); }
}
function line(c, x0, y0, x1, y1, w, col) {
  c.beginPath(); c.moveTo(x0, y0); c.lineTo(x1, y1);
  c.lineWidth = w; c.strokeStyle = col; c.lineCap = 'round'; c.stroke();
}
function poly(c, pts, f, s) {
  c.beginPath(); c.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) c.lineTo(pts[i][0], pts[i][1]);
  c.closePath();
  if (f) { c.fillStyle = f; c.fill(); }
  if (s) { c.strokeStyle = s; c.lineWidth = 2; c.lineJoin = 'round'; c.stroke(); }
}
const mirror = pts => pts.concat(pts.slice().reverse().map(p => [p[0], -p[1]]));

// ---------------------------------------------------------------- land units
function inf(c, o) {
  ell(c, -3, 4, 22, 28, 'rgba(0,0,0,.22)');
  const w = o.w;
  if (w === 'tool') { line(c, 4, 20, 34, 20, 4, WOOD); line(c, 34, 10, 34, 30, 5, STEEL); }
  if (w === 'club') line(c, 4, 22, 34, 27, 8, DWOOD);
  if (w === 'sling') { line(c, 4, 22, 24, 31, 2, '#3b2a18'); ell(c, 26, 32, 5, 5, '#8a8a8a', OUT); }
  if (w === 'spear') { line(c, -18, 22, 46, 22, 4, WOOD); poly(c, [[44, 17], [60, 22], [44, 27]], STEEL, OUT); }
  if (w === 'sword') { line(c, 8, 22, 46, 22, 5, '#dfe5ec'); line(c, 12, 15, 12, 29, 4, DWOOD); }
  if (w === 'bow') {
    c.beginPath(); c.arc(8, 0, 30, -1.05, 1.05); c.lineWidth = 4; c.strokeStyle = DWOOD; c.stroke();
    line(c, 23, -26, 23, 26, 1.2, '#eee'); line(c, 4, 0, 40, 0, 2, '#d8c9a0');
  }
  if (w === 'xbow') { line(c, 6, 8, 42, 8, 6, DWOOD); line(c, 34, -12, 34, 28, 4, DSTEEL); }
  if (w === 'musket') { line(c, 0, 15, 54, 15, 4, GUN); line(c, 0, 15, 18, 15, 7, DWOOD); }
  if (w === 'rifle') { line(c, 0, 15, 46, 15, 5, GUN); line(c, 0, 15, 14, 15, 7, DWOOD); }
  if (w === 'smg') { line(c, 4, 14, 38, 14, 6, GUN); line(c, 20, 14, 20, 24, 5, GUN); rr(c, -16, -10, 10, 20, 3, '#4b5a35', OUT); }
  ell(c, 8, -19, 8, 6, M, OUT); ell(c, 9, 18, 9, 6, M, OUT);
  ell(c, 0, 0, 15, 26, M, OUT);
  ell(c, -3, 0, 6, 18, MD);
  if (o.shield) ell(c, 15, -22, 6, 14, o.shield, OUT);
  ell(c, 4, 0, 12, 12, o.head, OUT);
  if (o.helm) ell(c, 3, 0, 8, 8, o.helm);
}
function cav(c, o) {
  ell(c, -2, 5, 50, 22, 'rgba(0,0,0,.22)');
  line(c, -40, 0, -54, 0, 6, '#2b1a0c');
  ell(c, 0, 0, 44, 16, o.horse, OUT);
  ell(c, 44, 0, 15, 9, o.horse, OUT);
  line(c, 22, 0, 40, 0, 5, '#2b1a0c');
  if (o.w === 'lance') { line(c, -8, 18, 58, 18, 4, WOOD); poly(c, [[56, 13], [70, 18], [56, 23]], STEEL, OUT); }
  if (o.w === 'sword') { line(c, 6, 20, 44, 26, 5, '#dfe5ec'); }
  if (o.w === 'carbine') { line(c, 0, 16, 42, 16, 5, GUN); }
  if (o.armor) rr(c, -30, -15, 44, 30, 8, o.armor, OUT);
  ell(c, -6, 0, 13, 20, M, OUT);
  ell(c, -3, 0, 10, 10, o.head, OUT);
  if (o.helm) ell(c, -4, 0, 6, 6, o.helm);
}
function veh(c, o) {
  const L = o.L, W = o.W;
  rr(c, -L / 2 - 2, -W / 2 + 4, L + 4, W + 2, 8, 'rgba(0,0,0,.25)');
  if (o.wheels) {
    for (const x of o.wheels) { rr(c, x - 9, -W / 2 - 3, 18, 9, 4, '#1c1e21'); rr(c, x - 9, W / 2 - 6, 18, 9, 4, '#1c1e21'); }
  } else {
    rr(c, -L / 2, -W / 2, L, W, 7, '#22252a', OUT);
    for (let x = -L / 2 + 6; x < L / 2 - 4; x += 9) { line(c, x, -W / 2 + 2, x, -W / 2 + 9, 2, '#3b4047'); line(c, x, W / 2 - 9, x, W / 2 - 2, 2, '#3b4047'); }
  }
  rr(c, -L / 2 + 3, -W / 2 + 8, L - 6, W - 16, 6, o.hull, OUT);
  rr(c, -L / 2 + 7, -W / 2 + 11, 12, W - 22, 3, M);
  if (o.k === 'tank') {
    line(c, 4, 0, o.barrel, 0, o.cal || 6, GUN);
    ell(c, 0, 0, o.tr, o.tr * 0.82, M, OUT); ell(c, -3, -3, 5, 5, MD, OUT);
  }
  if (o.k === 'gun') { line(c, -L / 2 + 14, 0, o.barrel, 0, o.cal || 8, GUN); rr(c, -8, -12, 20, 24, 4, M, OUT); line(c, o.barrel - 8, 0, o.barrel, 0, (o.cal || 8) + 4, GUN); }
  if (o.k === 'flak') { line(c, 0, -6, 40, -6, 4, GUN); line(c, 0, 6, 40, 6, 4, GUN); ell(c, 0, 0, 13, 13, M, OUT); }
  if (o.k === 'rocket') { rr(c, -26, -18, 54, 36, 4, M, OUT); for (let y = -12; y <= 12; y += 8) line(c, -20, y, 26, y, 5, '#e9ecef'); }
  if (o.k === 'sam') { rr(c, -18, -16, 30, 32, 4, M, OUT); for (const y of [-9, 9]) { line(c, -24, y, 30, y, 7, '#f1f3f5'); poly(c, [[30, y - 4], [40, y], [30, y + 4]], '#d9342b'); } }
}
function treb(c) {
  rr(c, -40, -26, 80, 56, 6, 'rgba(0,0,0,.22)');
  for (const y of [-22, 22]) line(c, -40, y, 40, y, 8, DWOOD);
  line(c, -30, -22, -30, 22, 6, WOOD); line(c, 22, -22, 22, 22, 6, WOOD);
  rr(c, -8, -26, 16, 10, 2, '#22252a'); rr(c, -8, 16, 16, 10, 2, '#22252a');
  line(c, -46, 0, 50, 0, 7, WOOD); rr(c, -52, -10, 18, 20, 3, DSTONE, OUT);
  ell(c, 52, 0, 8, 8, '#6f6f6f', OUT); rr(c, -6, -8, 14, 16, 3, M, OUT);
}
function cannon(c) {
  ell(c, 0, 4, 44, 26, 'rgba(0,0,0,.22)');
  rr(c, -10, -30, 22, 10, 4, DWOOD, OUT); rr(c, -10, 20, 22, 10, 4, DWOOD, OUT);
  line(c, -46, 0, -8, 0, 6, WOOD); rr(c, -14, -14, 24, 28, 4, M, OUT);
  line(c, -6, 0, 46, 0, 12, '#3a3f45'); line(c, 40, 0, 48, 0, 15, '#24272b');
}

// ---------------------------------------------------------------- ships
function hull(c, L, W, f, s) {
  c.beginPath(); c.moveTo(L / 2, 0);
  c.quadraticCurveTo(L / 4, -W / 2, -L / 3, -W / 2); c.quadraticCurveTo(-L / 2, -W / 2, -L / 2, 0);
  c.quadraticCurveTo(-L / 2, W / 2, -L / 3, W / 2); c.quadraticCurveTo(L / 4, W / 2, L / 2, 0);
  if (f) { c.fillStyle = f; c.fill(); } if (s) { c.strokeStyle = s; c.lineWidth = 2; c.stroke(); }
}
function ship(c, o) {
  const L = o.L, W = o.W;
  c.save(); c.translate(-3, 5); hull(c, L, W, 'rgba(0,0,0,.25)'); c.restore();
  // wake
  line(c, -L / 2 - 10, -4, -L / 2, -2, 3, 'rgba(255,255,255,.5)'); line(c, -L / 2 - 10, 4, -L / 2, 2, 3, 'rgba(255,255,255,.5)');
  if (o.oars) for (let x = -L / 4; x <= L / 5; x += 9) line(c, x, -W / 2 - 9, x, W / 2 + 9, 2, WOOD);
  hull(c, L, W, o.hull, OUT);
  c.save(); c.scale(0.86, 0.72); hull(c, L, W, o.deck); c.restore();
  rr(c, -L / 2 + 6, -W / 2 + 5, 8, W - 10, 2, M);
  for (const x of o.masts || []) {
    c.beginPath(); c.moveTo(x - 3, -W * 0.85); c.quadraticCurveTo(x + 12, 0, x - 3, W * 0.85); c.quadraticCurveTo(x + 4, 0, x - 3, -W * 0.85);
    c.fillStyle = SAIL; c.fill(); c.strokeStyle = OUT; c.lineWidth = 1.5; c.stroke();
    line(c, x + 2, -W * 0.3, x + 2, W * 0.3, 5, M); ell(c, x - 3, 0, 3, 3, DWOOD);
  }
  if (o.bridge) { rr(c, o.bridge[0], -W * 0.3, o.bridge[1], W * 0.6, 3, M, OUT); rr(c, o.bridge[0] + 4, -W * 0.16, o.bridge[1] - 10, W * 0.32, 2, MD); }
  for (const x of o.stacks || []) ell(c, x, 0, 5, 5, '#1d1f22', OUT);
  for (const t of o.turrets || []) {
    const dir = t[1], r = t[2] || 7;
    for (const y of r > 7 ? [-3, 3] : [0]) line(c, t[0], y, t[0] + dir * r * 2.6, y, 2.6, GUN);
    ell(c, t[0], 0, r, r, DSTEEL, OUT);
  }
  for (const x of o.vls || []) { rr(c, x - 6, -7, 12, 14, 2, '#2a2e33', OUT); for (const y of [-3.5, 3.5]) ell(c, x - 2, y, 2, 2, '#e9ecef'); ell(c, x + 3, 0, 2, 2, '#e9ecef'); }
  for (const g of o.guns || []) { line(c, g, -W / 2 + 3, g, -W / 2 - 4, 3, GUN); line(c, g, W / 2 - 3, g, W / 2 + 4, 3, GUN); }
}
function sub(c, L, W, nuke) {
  line(c, -L / 2 - 8, 0, -L / 2 + 4, 0, 3, 'rgba(255,255,255,.4)');
  ell(c, 0, 0, L / 2, W / 2, nuke ? '#2f363e' : '#3f4850', OUT);
  ell(c, -4, -2, L / 2 - 10, W / 2 - 5, nuke ? '#3a424b' : '#4c565f');
  poly(c, [[-L / 2 + 4, 0], [-L / 2 - 4, -W / 2 - 2], [-L / 2 + 10, -2]], '#262b31', OUT);
  poly(c, [[-L / 2 + 4, 0], [-L / 2 - 4, W / 2 + 2], [-L / 2 + 10, 2]], '#262b31', OUT);
  rr(c, L * 0.08, -W * 0.3, L * 0.2, W * 0.6, 4, M, OUT);
  line(c, L * 0.12, 0, L * 0.24, 0, 2, MD);
  if (nuke) {
    for (let x = -L * 0.3; x <= -L * 0.02; x += 9) for (const y of [-4, 4]) ell(c, x, y, 2.6, 2.6, '#14171a');
    line(c, L * 0.1, -W * 0.3, L * 0.1, -W * 0.55, 3, '#262b31'); line(c, L * 0.1, W * 0.3, L * 0.1, W * 0.55, 3, '#262b31');
  }
}

// ---------------------------------------------------------------- aircraft
function prop(c, big) {
  const wing = big ? mirror([[14, 0], [14, -10], [6, -58], [-8, -58], [-6, -10]]) : mirror([[16, 0], [16, -8], [10, -44], [-2, -44], [0, -8]]);
  poly(c, wing, big ? '#6f7a62' : '#8b947a', OUT);
  const tail = big ? mirror([[-40, 0], [-38, -20], [-50, -20], [-50, 0]]) : mirror([[-30, 0], [-28, -15], [-38, -15], [-38, 0]]);
  poly(c, tail, big ? '#6f7a62' : '#8b947a', OUT);
  ell(c, 0, 0, big ? 50 : 38, big ? 9 : 7, big ? '#59634f' : '#737c64', OUT);
  if (big) for (const y of [-34, -18, 18, 34]) { ell(c, 12, y, 9, 4, '#3a3f45', OUT); line(c, 21, y - 7, 21, y + 7, 2, 'rgba(230,230,230,.7)'); }
  else line(c, 38, -10, 38, 10, 2.5, 'rgba(230,230,230,.8)');
  const tip = big ? 50 : 37;
  ell(c, 4, -tip, 6, 6, M, OUT); ell(c, 4, tip, 6, 6, M, OUT);
  ell(c, big ? 30 : 12, 0, big ? 8 : 9, 4, '#bfe3f5', OUT);
  rr(c, -22, -3, 14, 6, 2, M);
}
function jet(c) {
  poly(c, mirror([[10, 0], [8, -8], [-22, -36], [-34, -36], [-26, -8]]), '#8d98a4', OUT);
  poly(c, mirror([[-34, 0], [-36, -18], [-48, -20], [-46, 0]]), '#7b8692', OUT);
  poly(c, mirror([[52, 0], [30, -6], [-40, -7], [-46, 0]]), '#a3aeba', OUT);
  ell(c, 24, 0, 11, 4, '#1e3a52', OUT);
  poly(c, [[-18, -30], [-28, -30], [-26, -22], [-14, -20]], M); poly(c, [[-18, 30], [-28, 30], [-26, 22], [-14, 20]], M);
  line(c, -30, -5, -40, -5, 3, M); line(c, -30, 5, -40, 5, 3, M);
  ell(c, -47, 0, 4, 5, '#ffb347');
}
function stealth(c) {
  const p = mirror([[46, 0], [-18, -58], [-30, -52], [-20, -34], [-34, -22], [-22, -10]]);
  poly(c, p, '#2b2f36', '#11141a');
  poly(c, mirror([[30, 0], [4, -14], [-10, -10]]), '#3a4049');
  ell(c, 20, 0, 9, 4, '#0f1c29');
  line(c, -18, -44, -24, -40, 4, M); line(c, -18, 44, -24, 40, 4, M);
}
function heli(c) {
  line(c, -10, 0, -52, 0, 6, '#59634f'); rr(c, -58, -9, 8, 18, 2, M, OUT);
  rr(c, -10, -22, 16, 44, 3, '#4b5443', OUT); for (const y of [-20, 20]) rr(c, -6, y - 4, 16, 8, 2, '#e9ecef', OUT);
  ell(c, 6, 0, 28, 13, '#59634f', OUT); ell(c, 22, 0, 10, 8, '#bfe3f5', OUT); rr(c, -12, -8, 14, 16, 3, M);
  ell(c, 2, 0, 50, 50, 'rgba(210,215,220,.16)');
  for (let a = 0; a < 3; a++) { const t = a * 2.094 + 0.4; line(c, 2, 0, 2 + Math.cos(t) * 50, Math.sin(t) * 50, 3, 'rgba(40,40,40,.75)'); }
  ell(c, 2, 0, 4, 4, '#1c1e21');
}

// ---------------------------------------------------------------- buildings (fill the cell)
function roof(c, x, y, w, h, col, dark) {
  rr(c, x, y, w, h, 3, col, OUT);
  c.fillStyle = dark; c.fillRect(x + 2, y + h / 2, w - 4, h / 2 - 2);
  line(c, x + 2, y + h / 2, x + w - 2, y + h / 2, 2, 'rgba(0,0,0,.35)');
}
const B = {
  tc(c) {
    rr(c, -58, -58, 116, 116, 8, DSTONE, OUT); rr(c, -52, -52, 104, 104, 6, STONE);
    for (const [x, y] of [[-56, -56], [36, -56], [-56, 36], [36, 36]]) rr(c, x, y, 20, 20, 3, DSTONE, OUT);
    roof(c, -36, -36, 72, 72, '#b5653a', '#96512c');
    rr(c, -14, -14, 28, 28, 4, M, OUT); rr(c, -7, -7, 14, 14, 2, MD);
    line(c, 0, -52, 0, -38, 4, M); line(c, 0, 38, 0, 52, 4, M);
  },
  house(c) { rr(c, -50, -44, 100, 92, 4, 'rgba(0,0,0,.2)'); roof(c, -48, -46, 96, 88, '#b9723f', '#96582e'); rr(c, 20, -34, 14, 14, 2, DSTONE, OUT); rr(c, -40, -8, 16, 16, 2, M, OUT); },
  farm(c) {
    rr(c, -60, -60, 120, 120, 4, '#7d6a36', '#4e4220');
    for (let y = -52; y < 56; y += 12) { line(c, -54, y, 54, y, 6, '#c9a93f'); line(c, -54, y + 6, 54, y + 6, 3, '#6c8a34'); }
    roof(c, -54, -54, 44, 40, '#a85d34', '#8a4a27'); rr(c, -22, -44, 10, 10, 2, M, OUT);
  },
  barracks(c) {
    rr(c, -58, -58, 116, 116, 5, '#8d7a55', OUT);
    roof(c, -54, -54, 108, 50, '#6e4a2c', '#59391f'); rr(c, -54, 2, 40, 52, 3, '#7d5a36', OUT);
    for (const x of [0, 22, 44]) { ell(c, x, 30, 7, 7, '#d9c9a0', OUT); ell(c, x, 30, 3, 3, '#c0392b'); }
    rr(c, -44, -46, 24, 14, 2, M, OUT); rr(c, 20, -46, 24, 14, 2, M, OUT);
  },
  workshop(c) {
    rr(c, -58, -58, 116, 116, 5, '#6b6f73', OUT); roof(c, -54, -54, 108, 76, '#7d858d', '#666d75');
    for (let x = -46; x < 50; x += 18) line(c, x, -52, x, 20, 2, 'rgba(0,0,0,.25)');
    ell(c, 36, 40, 12, 12, '#3a3f45', OUT); ell(c, 36, 40, 6, 6, '#15171a');
    rr(c, -50, 28, 50, 24, 3, '#4d555e', OUT); rr(c, -44, -44, 30, 16, 2, M, OUT);
    ell(c, -10, 40, 9, 9, STEEL, OUT);
  },
  dock(c) {
    rr(c, -60, -60, 120, 120, 3, '#8a6a42', '#4e3a20');
    for (let y = -54; y < 58; y += 10) line(c, -58, y, 58, y, 1.5, 'rgba(0,0,0,.3)');
    rr(c, -34, -20, 68, 44, 4, '#1f5d7d', OUT);
    roof(c, -54, -54, 54, 28, '#a85d34', '#8a4a27'); rr(c, 22, -52, 30, 16, 2, M, OUT);
    line(c, 40, 40, 4, 10, 5, '#3a3f45'); ell(c, 40, 40, 8, 8, DSTEEL, OUT);
    for (const [x, y] of [[-56, -56], [56, -56], [-56, 56], [56, 56]]) ell(c, x, y, 5, 5, DWOOD, OUT);
  },
  airfield(c) {
    rr(c, -62, -62, 124, 124, 3, '#4a4e52', '#2b2e31');
    rr(c, -58, -14, 116, 34, 2, '#33363a'); for (let x = -50; x < 54; x += 18) line(c, x, 3, x + 8, 3, 3, '#e9ecef');
    roof(c, -56, -58, 60, 38, '#8a939c', '#727b84'); rr(c, -48, -50, 22, 12, 2, M, OUT);
    rr(c, 26, -56, 16, 16, 3, STONE, OUT); ell(c, 34, -48, 5, 5, '#bfe3f5', OUT);
    rr(c, -54, 30, 30, 26, 3, '#727b84', OUT); rr(c, -14, 30, 30, 26, 3, '#727b84', OUT);
    ell(c, 44, 44, 9, 9, M, OUT);
  },
  tower(c) {
    ell(c, 4, 6, 52, 52, 'rgba(0,0,0,.25)'); ell(c, 0, 0, 54, 54, DSTONE, OUT); ell(c, 0, 0, 42, 42, STONE, OUT);
    for (let a = 0; a < 8; a++) { const t = a * 0.785; rr(c, Math.cos(t) * 47 - 6, Math.sin(t) * 47 - 6, 12, 12, 2, DSTONE, OUT); }
    ell(c, 0, 0, 22, 22, M, OUT); ell(c, 0, 0, 10, 10, MD);
  },
};

// ---------------------------------------------------------------- scenery & effects
function tree(c, conifer) {
  ell(c, 6, 8, 40, 38, 'rgba(0,0,0,.25)');
  if (conifer) {
    for (let k = 0; k < 3; k++) {
      const r = 44 - k * 13, pts = [];
      for (let a = 0; a < 16; a++) { const t = a * 0.3927 + k * 0.2, q = a % 2 ? r * 0.62 : r; pts.push([Math.cos(t) * q, Math.sin(t) * q]); }
      poly(c, pts, ['#2f5d33', '#3c7340', '#4c8a4e'][k], 'rgba(0,0,0,.35)');
    }
  } else {
    const blobs = [[0, 0, 34, '#3f7a36'], [-16, -12, 20, '#4a8a3e'], [14, -14, 20, '#4f9343'], [12, 14, 20, '#447f3a'], [-14, 14, 18, '#4a8a3e'], [0, -4, 16, '#5ea24d']];
    ell(c, 0, 0, 38, 38, '#356a2f', 'rgba(0,0,0,.35)');
    for (const [x, y, r, col] of blobs) ell(c, x, y, r, r, col);
  }
}
function ore(c) {
  ell(c, 4, 8, 50, 40, 'rgba(0,0,0,.25)');
  const rocks = [[-18, 4, 26, 22], [16, -6, 28, 24], [6, 20, 22, 16], [-10, -20, 18, 14]];
  for (const [x, y, rx, ry] of rocks) { ell(c, x, y, rx, ry, '#767d86', OUT); ell(c, x - 4, y - 5, rx * 0.6, ry * 0.5, '#98a1ab'); }
  for (const [x, y] of [[-22, 0], [14, -10], [22, 2], [4, 18], [-8, -22], [-4, 6]]) poly(c, [[x, y - 5], [x + 4, y], [x, y + 5], [x - 4, y]], '#dfe9f5');
}
function berry(c) {
  ell(c, 4, 8, 42, 36, 'rgba(0,0,0,.25)'); ell(c, 0, 0, 40, 36, '#3d7a35', OUT);
  for (const [x, y] of [[-14, -10], [12, -14], [18, 8], [-6, 14], [-22, 8], [2, -2]]) ell(c, x, y, 13, 12, '#4f9343');
  for (const [x, y] of [[-18, -12], [8, -18], [20, 4], [-4, 16], [-24, 8], [4, 0], [14, -6], [-10, 2], [10, 16]]) ell(c, x, y, 4.5, 4.5, '#d6263d', 'rgba(0,0,0,.4)');
}
const P = {
  arrow(c) { line(c, -40, 0, 40, 0, 5, '#d8c9a0'); poly(c, [[36, -9], [56, 0], [36, 9]], '#e9ecef', OUT); line(c, -40, 0, -52, -8, 4, '#fff'); line(c, -40, 0, -52, 8, 4, '#fff'); },
  bullet(c) { line(c, -50, 0, 30, 0, 9, 'rgba(255,220,120,.55)'); line(c, 0, 0, 44, 0, 9, '#fff3c4'); },
  shell(c) { ell(c, 0, 0, 34, 34, 'rgba(255,170,60,.35)'); ell(c, 0, 0, 20, 20, '#2b2b2b', '#000'); ell(c, -6, -6, 6, 6, '#777'); },
  torp(c) { for (let k = 0; k < 4; k++) ell(c, -30 - k * 9, 0, 7 - k, 7 - k, 'rgba(255,255,255,.45)'); ell(c, 10, 0, 40, 9, '#59626c', OUT); ell(c, 40, 0, 10, 8, '#c0392b', OUT); },
  missile(c) {
    ell(c, -46, 0, 14, 8, 'rgba(255,200,80,.9)'); ell(c, -54, 0, 9, 5, 'rgba(255,120,40,.8)');
    rr(c, -34, -7, 70, 14, 6, '#f1f3f5', OUT); poly(c, [[34, -7], [54, 0], [34, 7]], '#d9342b', OUT);
    poly(c, [[-34, -7], [-40, -18], [-22, -7]], '#868e96', OUT); poly(c, [[-34, 7], [-40, 18], [-22, 7]], '#868e96', OUT);
  },
  bomb(c) { ell(c, 0, 0, 36, 16, '#30343a', '#000'); poly(c, [[-30, 0], [-48, -16], [-40, 0], [-48, 16]], '#555c64', OUT); },
};
function flag(c) { ell(c, -20, 44, 14, 6, 'rgba(0,0,0,.3)'); line(c, -20, 44, -20, -48, 6, '#e9ecef'); poly(c, [[-17, -48], [40, -32], [-17, -14]], M, OUT); }

/** Sprite id → drawing (HUD icons). Ids 0..=42 match the unit/building table in src/defs.rs. */
const SPRITES = {
  0: B.tc, 1: B.house, 2: B.farm, 3: B.barracks, 4: B.workshop, 5: B.dock, 6: B.airfield, 7: B.tower,
  8: c => inf(c, { w: 'tool', head: '#d9c074' }),
  9: c => inf(c, { w: 'club', head: SKIN }),
  10: c => inf(c, { w: 'sling', head: SKIN }),
  11: c => inf(c, { w: 'spear', head: SKIN, helm: '#b08d57', shield: '#8b6b3d' }),
  12: c => inf(c, { w: 'bow', head: SKIN, helm: '#5b7a3a' }),
  13: c => inf(c, { w: 'sword', head: STEEL, helm: '#cfd6dd', shield: '#c7ccd1' }),
  14: c => inf(c, { w: 'xbow', head: STEEL, helm: '#6c757d' }),
  15: c => inf(c, { w: 'musket', head: '#222', helm: '#111' }),
  16: c => inf(c, { w: 'rifle', head: '#5d6b3a', helm: '#4b5a2d' }),
  17: c => inf(c, { w: 'smg', head: '#3d4a3a', helm: '#2b352a' }),
  18: c => cav(c, { horse: '#8a5a2b', w: 'sword', head: SKIN, helm: '#b08d57' }),
  19: c => cav(c, { horse: '#5a3a1c', w: 'lance', head: STEEL, helm: '#cfd6dd', armor: '#b9c0c7' }),
  20: treb,
  21: c => cav(c, { horse: '#3b2a1a', w: 'carbine', head: '#222', helm: '#111' }),
  22: cannon,
  23: c => veh(c, { L: 84, W: 54, hull: '#5d6b3a', k: 'tank', tr: 17, barrel: 52 }),
  24: c => veh(c, { L: 80, W: 46, hull: '#5d6b3a', k: 'gun', barrel: 58, cal: 8, wheels: [-22, 14] }),
  25: c => veh(c, { L: 80, W: 48, hull: '#66704a', k: 'flak', wheels: [26] }),
  26: c => veh(c, { L: 96, W: 58, hull: '#6c6f5a', k: 'tank', tr: 21, barrel: 62, cal: 7 }),
  27: c => veh(c, { L: 92, W: 52, hull: '#59634f', k: 'rocket', wheels: [-30, -6, 30] }),
  28: c => veh(c, { L: 88, W: 50, hull: '#59634f', k: 'sam', wheels: [-28, 28] }),
  29: c => ship(c, { L: 96, W: 26, hull: DWOOD, deck: '#b08a55', masts: [6], oars: true }),
  30: c => ship(c, { L: 100, W: 34, hull: DWOOD, deck: '#b08a55', masts: [-22, 10, 32], guns: [-10, 8] }),
  31: c => ship(c, { L: 108, W: 32, hull: '#3b2a1a', deck: '#a37c48', masts: [-28, 4, 30], guns: [-22, -8, 6, 20] }),
  32: c => ship(c, { L: 110, W: 22, hull: '#5c6670', deck: '#89939d', bridge: [-10, 26], stacks: [-22], turrets: [[30, 1], [-36, -1]] }),
  33: c => ship(c, { L: 120, W: 34, hull: '#525b64', deck: '#7f8992', bridge: [-14, 30], stacks: [-24, -34], turrets: [[36, 1, 10], [20, 1, 10], [-46, -1, 10]], guns: [-2, 8] }),
  34: c => sub(c, 100, 20, false),
  35: c => ship(c, { L: 116, W: 26, hull: '#5c6670', deck: '#8d97a1', bridge: [-8, 30], stacks: [-20], turrets: [[40, 1]], vls: [24, -34] }),
  36: c => sub(c, 120, 26, true),
  37: c => prop(c, false),
  38: c => prop(c, true),
  39: jet,
  40: stealth,
  41: heli,
  42: c => { ship(c, { L: 104, W: 40, hull: '#6b5a45', deck: '#8a7a5a', bridge: [-44, 24] }); rr(c, -14, -12, 50, 24, 3, '#4d555e', OUT); },
  100: c => tree(c, false), 101: c => tree(c, true), 102: ore, 103: berry,
  104: P.arrow, 105: P.bullet, 106: P.shell, 107: P.torp, 108: P.missile, 109: P.bomb, 110: flag,
};

export function buildAtlas() {
  const cv = document.createElement('canvas');
  cv.width = cv.height = ATLAS;
  const c = cv.getContext('2d');
  for (const id in SPRITES) {
    const col = id % 16, row = (id / 16) | 0;
    c.save();
    c.beginPath(); c.rect(col * CELL, row * CELL, CELL, CELL); c.clip();
    c.translate(col * CELL + CELL / 2, row * CELL + CELL / 2);
    SPRITES[id](c);
    c.restore();
  }
  return cv;
}

/** Copy of the atlas with team pixels painted in `rgb`, for HUD icons. */
export function tintAtlas(src, rgb) {
  const cv = document.createElement('canvas');
  cv.width = cv.height = ATLAS;
  const c = cv.getContext('2d');
  c.drawImage(src, 0, 0);
  const img = c.getImageData(0, 0, ATLAS, ATLAS), d = img.data;
  for (let i = 0; i < d.length; i += 4) {
    if (!d[i + 3]) continue;
    const r = d[i], g = d[i + 1], b = d[i + 2];
    const k = Math.min(1, Math.max(0, (Math.min(r, b) - g) * 3 / 255));
    if (k > 0) {
      const l = Math.max(r, b) / 255;
      d[i] = r + (rgb[0] * l - r) * k; d[i + 1] = g + (rgb[1] * l - g) * k; d[i + 2] = b + (rgb[2] * l - b) * k;
    }
  }
  c.putImageData(img, 0, 0);
  return cv;
}
