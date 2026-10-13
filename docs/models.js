// Procedural low-poly 3D models. Like the 2D icons, every mesh is generated in
// code at start-up from boxes, extruded outlines, cylinders and cones: the game
// ships no model files.
//
// Model space: +x is forward, +y is up, z is sideways. Units are authored about
// one unit long and buildings exactly one unit square; the instance size scales
// them. Vertex = position(3) normal(3) colour(3) team(1); team = 1 means "tint
// with the owner's colour", keeping the stored colour as brightness.

const F = 10; // floats per vertex
let V = [];
let centre = [0, 0, 0];

const T = k => ({ team: k === undefined ? 1 : k });      // team-coloured surface with brightness k
const hex = h => [(h >> 16 & 255) / 255, (h >> 8 & 255) / 255, (h & 255) / 255];
const col = c => (typeof c === 'number' ? hex(c) : c);

function tri(a, b, c, colour) {
  let nx = (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]);
  let ny = (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]);
  let nz = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
  const l = Math.hypot(nx, ny, nz) || 1;
  nx /= l; ny /= l; nz /= l;
  // orient the normal away from the primitive's centre so winding never matters
  const mx = (a[0] + b[0] + c[0]) / 3 - centre[0], my = (a[1] + b[1] + c[1]) / 3 - centre[1], mz = (a[2] + b[2] + c[2]) / 3 - centre[2];
  if (nx * mx + ny * my + nz * mz < 0) { nx = -nx; ny = -ny; nz = -nz; }
  const team = colour.team !== undefined;
  const rgb = team ? [colour.team, colour.team, colour.team] : col(colour);
  for (const p of [a, b, c]) V.push(p[0], p[1], p[2], nx, ny, nz, rgb[0], rgb[1], rgb[2], team ? 1 : 0);
}
const quad = (a, b, c, d, colour) => { tri(a, b, c, colour); tri(a, c, d, colour); };

/** Extrude a convex outline [[x,z],...] from y0 to y1; `top` shrinks the upper face toward its centre. */
function slab(pts, y0, y1, colour, top = 1) {
  const cx = pts.reduce((s, p) => s + p[0], 0) / pts.length, cz = pts.reduce((s, p) => s + p[1], 0) / pts.length;
  centre = [cx, (y0 + y1) / 2, cz];
  const lo = pts.map(p => [p[0], y0, p[1]]);
  const hi = pts.map(p => [cx + (p[0] - cx) * top, y1, cz + (p[1] - cz) * top]);
  for (let i = 0; i < pts.length; i++) {
    const j = (i + 1) % pts.length;
    quad(lo[i], lo[j], hi[j], hi[i], colour);
  }
  for (let i = 1; i + 1 < pts.length; i++) { tri(hi[0], hi[i], hi[i + 1], colour); tri(lo[0], lo[i], lo[i + 1], colour); }
}
/** Box centred on (x, z), standing on height y. */
const box = (x, y, z, sx, sy, sz, colour, top = 1) =>
  slab([[x - sx / 2, z - sz / 2], [x + sx / 2, z - sz / 2], [x + sx / 2, z + sz / 2], [x - sx / 2, z + sz / 2]], y, y + sy, colour, top);
const ring = (x, z, r, n = 8, squash = 1) => Array.from({ length: n }, (_, i) => [x + Math.cos(i / n * 6.2832) * r, z + Math.sin(i / n * 6.2832) * r * squash]);
const cyl = (x, y, z, r, h, colour, n = 8, top = 1) => slab(ring(x, z, r, n), y, y + h, colour, top);
const cone = (x, y, z, r, h, colour, n = 8) => slab(ring(x, z, r, n), y, y + h, colour, 0.02);
/** Gabled roof with its ridge along x. */
function roof(x, y, z, sx, sz, h, colour) {
  centre = [x, y + h * 0.3, z];
  const a = [x - sx / 2, y, z - sz / 2], b = [x + sx / 2, y, z - sz / 2], c = [x + sx / 2, y, z + sz / 2], d = [x - sx / 2, y, z + sz / 2];
  const r0 = [x - sx / 2, y + h, z], r1 = [x + sx / 2, y + h, z];
  quad(a, b, r1, r0, colour); quad(d, c, r1, r0, colour); tri(a, d, r0, colour); tri(b, c, r1, colour);
}
const mirror = pts => pts.concat(pts.slice().reverse().map(p => [p[0], -p[1]]));

// ---------------------------------------------------------------- palette
const SKIN = 0xe0b088, CLOTH = 0x4a4038, WOOD = 0x8a5a2b, DWOOD = 0x5d3b1a, STEEL = 0xaab3bd, DSTEEL = 0x4d555e, GUN = 0x2d3136;
const STONE = 0xa9a498, DSTONE = 0x7d786c, TILE = 0xb5653a, SAIL = 0xefe6cf, OLIVE = 0x5d6b3a, NAVY = 0x6a747e, DECK = 0x8d97a1;

// ---------------------------------------------------------------- units
function inf(o) {
  box(0, 0, -0.07, 0.09, 0.22, 0.08, CLOTH); box(0, 0, 0.07, 0.09, 0.22, 0.08, CLOTH);
  box(0, 0.22, 0, 0.14, 0.24, 0.27, T());
  box(0.02, 0.46, 0, 0.13, 0.13, 0.13, o.head);
  if (o.helm) box(0.02, 0.57, 0, 0.16, 0.05, 0.16, o.helm);
  if (o.wl) box(0.1 + o.wl / 2, o.wy || 0.33, 0.17, o.wl, o.wt || 0.035, o.wt || 0.035, o.wc);
  if (o.tip) box(0.1 + o.wl + 0.03, 0.31, 0.17, 0.08, 0.08, 0.05, STEEL, 0.2);
  if (o.shield) box(0.11, 0.2, -0.17, 0.04, 0.24, 0.16, o.shield);
  if (o.bow) { box(0.14, 0.2, 0.17, 0.03, 0.34, 0.03, DWOOD); }
  if (o.pack) box(-0.1, 0.24, 0, 0.08, 0.18, 0.2, o.pack);
}
function cav(o) {
  for (const [x, z] of [[0.2, 0.08], [0.2, -0.08], [-0.2, 0.08], [-0.2, -0.08]]) box(x, 0, z, 0.06, 0.2, 0.06, o.horse);
  box(0, 0.2, 0, 0.6, 0.2, 0.2, o.armor || o.horse);
  box(0.3, 0.3, 0, 0.12, 0.26, 0.12, o.horse); box(0.4, 0.46, 0, 0.2, 0.1, 0.1, o.horse);
  box(-0.32, 0.25, 0, 0.06, 0.16, 0.05, 0x2b1a0c);
  box(-0.03, 0.4, 0, 0.14, 0.24, 0.22, T()); box(-0.01, 0.64, 0, 0.12, 0.12, 0.12, o.head);
  if (o.helm) box(-0.01, 0.74, 0, 0.14, 0.05, 0.14, o.helm);
  box(0.05 + o.wl / 2, 0.52, 0.15, o.wl, 0.03, 0.03, o.wc);
}
function tracks(L, W, h = 0.13) { box(0, 0, -W / 2 + 0.06, L, h, 0.12, 0x1c1e21); box(0, 0, W / 2 - 0.06, L, h, 0.12, 0x1c1e21); }
function wheels(xs, W) { for (const x of xs) for (const z of [-W / 2 + 0.04, W / 2 - 0.04]) cyl(x, 0, z, 0.075, 0.15, 0x1c1e21, 6); }
function tank(L, W, hull, tr, barrel) {
  tracks(L, W); box(0, 0.06, 0, L * 0.96, 0.15, W - 0.14, hull, 0.9);
  box(-0.04, 0.21, 0, tr * 1.25, 0.12, tr, T(), 0.85);
  box(tr * 0.5 + barrel / 2, 0.245, 0, barrel, 0.045, 0.045, GUN);
  box(-0.3 * L, 0.21, 0, 0.12, 0.03, W * 0.5, T(0.7));
}
function gunCarriage(L, W, hull, barrel, wide) {
  wheels([-0.18, 0.12], W); box(0, 0.08, 0, L, 0.1, W - 0.12, hull);
  box(-0.06, 0.18, 0, 0.2, 0.12, 0.2, T());
  box(barrel / 2 - 0.08, 0.24, 0, barrel, wide, wide, GUN);
}

// ---------------------------------------------------------------- ships
const hullPts = (L, W) => mirror([[L / 2, 0], [L * 0.3, -W * 0.42], [-L * 0.36, -W / 2], [-L / 2, -W * 0.3]]);
function hull(L, W, h, colour, deck) {
  slab(hullPts(L, W), -0.06, h, colour, 1.0);
  slab(hullPts(L * 0.93, W * 0.8), h, h + 0.012, deck, 1.0);
  box(-L * 0.4, h * 0.3, 0, 0.05, h * 0.5, W * 0.9, T());
}
function mast(x, h, W) { box(x, 0.1, 0, 0.03, h, 0.03, DWOOD); box(x + 0.02, 0.1 + h * 0.3, 0, 0.02, h * 0.6, W * 1.5, SAIL); box(x + 0.035, 0.1 + h * 0.55, 0, 0.012, h * 0.14, W * 1.5, T()); }
function turret(x, dir, r = 0.045, y = 0.1) { cyl(x, y, 0, r, 0.045, DSTEEL, 8); for (const z of r > 0.05 ? [-0.02, 0.02] : [0]) box(x + dir * (r + 0.07), y + 0.012, z, 0.16, 0.018, 0.018, GUN); }
function sub(L, W, nuke) {
  const body = nuke ? 0x2f363e : 0x3f4850;
  slab(mirror([[L / 2, 0], [L * 0.42, -W * 0.36], [L * 0.2, -W / 2], [-L * 0.3, -W / 2], [-L / 2, -W * 0.12]]), 0, W * 0.75, body, 0.72);
  box(L * 0.1, W * 0.7, 0, L * 0.16, W * 0.75, W * 0.34, T(0.9), 0.85);
  box(L * 0.1, W * 1.4, 0, 0.02, 0.1, 0.02, DSTEEL);
  box(-L * 0.47, W * 0.1, 0, 0.07, W * 0.5, W * 1.25, 0x262b31);
  if (nuke) for (let x = -L * 0.26; x <= -L * 0.02; x += 0.07) for (const z of [-0.035, 0.035]) cyl(x, W * 0.72, z, 0.022, 0.012, 0x14171a, 6);
}

// ---------------------------------------------------------------- aircraft (centred on y = 0)
function prop(big) {
  const body = big ? 0x59634f : 0x737c64, wing = big ? 0x6f7a62 : 0x8b947a;
  const L = big ? 0.95 : 0.74, span = big ? 1.1 : 0.84;
  slab(mirror([[L / 2, 0], [L * 0.36, -0.06], [-L / 2, -0.03]]), -0.05, 0.07, body, 0.7);
  slab(mirror([[0.16, 0], [0.14, -span / 2], [0, -span / 2], [-0.04, 0]]), 0, 0.025, wing);
  slab(mirror([[-L * 0.36, 0], [-L * 0.38, -0.2], [-L * 0.48, -0.2], [-L * 0.48, 0]]), 0.02, 0.04, wing);
  box(-L * 0.44, 0.03, 0, 0.12, 0.16, 0.02, T());
  for (const z of [-span / 2 + 0.07, span / 2 - 0.07]) box(0.07, 0.026, z, 0.1, 0.006, 0.1, T());
  box(L * 0.18, 0.06, 0, 0.14, 0.04, 0.08, 0xbfe3f5);
  if (big) for (const z of [-0.34, -0.18, 0.18, 0.34]) box(0.16, -0.03, z, 0.16, 0.06, 0.06, GUN);
  else cyl(L / 2, -0.09, 0, 0.11, 0.008, 0xdddddd, 8, 1);
}
function jet() {
  slab(mirror([[0.5, 0], [0.3, -0.05], [-0.42, -0.06], [-0.46, 0]]), -0.04, 0.06, 0xa3aeba, 0.7);
  slab(mirror([[0.1, 0], [-0.22, -0.36], [-0.34, -0.36], [-0.28, 0]]), 0, 0.02, 0x8d98a4);
  slab(mirror([[-0.34, 0], [-0.4, -0.19], [-0.48, -0.19], [-0.46, 0]]), 0.005, 0.02, 0x7b8692);
  for (const z of [-0.07, 0.07]) box(-0.4, 0.04, z, 0.12, 0.15, 0.015, T());
  box(0.22, 0.05, 0, 0.16, 0.04, 0.07, 0x1e3a52);
  for (const z of [-0.3, 0.3]) box(-0.26, 0.021, z, 0.1, 0.006, 0.09, T());
  cyl(-0.47, -0.02, 0, 0.04, 0.05, 0xffb347, 6);
}
function stealth() {
  slab(mirror([[0.46, 0], [-0.18, -0.58], [-0.3, -0.52], [-0.2, -0.34], [-0.34, -0.22], [-0.22, -0.1]]), 0, 0.035, 0x2b2f36);
  slab(mirror([[0.32, 0], [0.04, -0.13], [-0.12, -0.09]]), 0.035, 0.085, 0x3a4049, 0.5);
  box(0.2, 0.08, 0, 0.12, 0.012, 0.06, 0x0f1c29);
  for (const z of [-0.42, 0.42]) box(-0.2, 0.036, z, 0.07, 0.006, 0.07, T());
}
function heli() {
  slab(mirror([[0.34, 0], [0.22, -0.11], [-0.16, -0.12], [-0.22, 0]]), -0.1, 0.1, 0x59634f, 0.75);
  box(0.2, 0.02, 0, 0.14, 0.09, 0.14, 0xbfe3f5, 0.7);
  box(-0.38, 0, 0, 0.4, 0.05, 0.05, 0x59634f); box(-0.56, 0, 0, 0.06, 0.18, 0.02, T());
  box(-0.02, -0.06, 0, 0.14, 0.03, 0.44, 0x4b5443); for (const z of [-0.2, 0.2]) box(0.02, -0.1, z, 0.2, 0.05, 0.05, 0xe9ecef);
  box(-0.06, 0.1, 0, 0.12, 0.04, 0.2, T());
  box(0, 0.14, 0, 0.03, 0.05, 0.03, GUN); box(0, 0.19, 0, 1.0, 0.012, 0.05, 0x303030); box(0, 0.19, 0, 0.05, 0.012, 1.0, 0x303030);
}

// ---------------------------------------------------------------- buildings (footprint -0.5..0.5)
const base = (c, h = 0.05) => box(0, -0.4, 0, 0.98, 0.4 + h, 0.98, c);
const B = {
  tc() {
    base(DSTONE); box(0, 0.05, 0, 0.78, 0.2, 0.78, STONE);
    for (const [x, z] of [[-0.4, -0.4], [0.4, -0.4], [-0.4, 0.4], [0.4, 0.4]]) { box(x, 0.05, z, 0.16, 0.32, 0.16, DSTONE); box(x, 0.37, z, 0.2, 0.05, 0.2, STONE); }
    box(0, 0.25, 0, 0.5, 0.06, 0.5, TILE); box(0, 0.25, 0, 0.3, 0.3, 0.3, STONE);
    slab(ring(0, 0, 0.26, 4).map(p => [p[0], p[1]]), 0.55, 0.74, T(), 0.02);
    box(0, 0.05, 0.4, 0.14, 0.14, 0.04, DWOOD);
  },
  house() { base(0x6f6a5e, 0.02); box(0, 0.02, 0, 0.8, 0.26, 0.68, 0xd9c9a0); roof(0, 0.28, 0, 0.9, 0.78, 0.26, TILE); box(0.22, 0.3, 0.16, 0.1, 0.3, 0.1, DSTONE); box(0.41, 0.02, 0, 0.02, 0.18, 0.12, T()); },
  farm() {
    base(0x7d6a36, 0.02);
    for (let z = -0.42; z < 0.45; z += 0.12) box(0.12, 0.02, z, 0.7, 0.04, 0.07, z * 100 % 2 ? 0xc9a93f : 0xb89a36);
    box(-0.32, 0.02, -0.3, 0.28, 0.16, 0.3, 0xd9c9a0); roof(-0.32, 0.18, -0.3, 0.32, 0.34, 0.14, 0xa85d34);
    box(-0.32, 0.02, 0.2, 0.03, 0.26, 0.03, DWOOD); box(-0.32, 0.22, 0.2, 0.02, 0.07, 0.14, T());
  },
  barracks() {
    base(0x8d7a55); box(0, 0.05, -0.2, 0.9, 0.24, 0.46, 0x9c8562); roof(0, 0.29, -0.2, 0.96, 0.52, 0.2, 0x6e4a2c);
    box(-0.3, 0.05, 0.26, 0.3, 0.16, 0.34, 0x7d5a36); box(-0.3, 0.21, 0.26, 0.34, 0.03, 0.38, 0x59391f);
    for (const x of [0.02, 0.2, 0.38]) { box(x, 0.05, 0.3, 0.03, 0.16, 0.03, DWOOD); cyl(x, 0.2, 0.3, 0.06, 0.02, 0xd9c9a0, 8); }
    box(0.46, 0.05, 0.44, 0.03, 0.5, 0.03, DWOOD); box(0.46, 0.4, 0.37, 0.02, 0.14, 0.14, T());
  },
  workshop() {
    base(0x6b6f73); box(0, 0.05, -0.1, 0.9, 0.3, 0.66, 0x8a8f95);
    for (const x of [-0.3, 0, 0.3]) roof(x, 0.35, -0.1, 0.3, 0.66, 0.1, 0x666d75);
    cyl(0.34, 0.05, 0.36, 0.07, 0.6, 0x5a3a2a, 8, 0.8); box(-0.2, 0.05, 0.36, 0.44, 0.14, 0.2, DSTEEL);
    box(-0.2, 0.19, 0.36, 0.2, 0.03, 0.12, T()); box(0, 0.05, 0.24, 0.2, 0.2, 0.02, T(0.8));
  },
  dock() {
    box(0, -0.3, 0, 0.98, 0.36, 0.98, 0x8a6a42);
    for (const [x, z] of [[-0.46, -0.46], [0.46, -0.46], [-0.46, 0.46], [0.46, 0.46], [0, 0.46], [0, -0.46]]) cyl(x, -0.3, z, 0.035, 0.46, DWOOD, 6);
    box(-0.26, 0.06, -0.26, 0.4, 0.18, 0.36, 0xd9c9a0); roof(-0.26, 0.24, -0.26, 0.46, 0.42, 0.14, 0xa85d34);
    box(0.3, 0.06, 0.3, 0.06, 0.4, 0.06, DSTEEL); box(0.16, 0.42, 0.3, 0.36, 0.04, 0.04, DSTEEL); box(0, 0.3, 0.3, 0.02, 0.12, 0.02, GUN);
    box(0.3, 0.06, -0.3, 0.2, 0.1, 0.2, DWOOD); box(-0.05, 0.06, 0.02, 0.02, 0.3, 0.02, DWOOD); box(-0.05, 0.28, 0.08, 0.02, 0.08, 0.12, T());
  },
  airfield() {
    base(0x4a4e52, 0.02); box(0, 0.02, 0.1, 0.96, 0.01, 0.26, 0x33363a);
    for (let x = -0.4; x < 0.42; x += 0.16) box(x, 0.03, 0.1, 0.07, 0.004, 0.02, 0xe9ecef);
    box(-0.24, 0.02, -0.3, 0.44, 0.14, 0.3, 0x8a939c); roof(-0.24, 0.16, -0.3, 0.44, 0.3, 0.1, 0x727b84);
    box(0.32, 0.02, -0.34, 0.1, 0.4, 0.1, STONE); box(0.32, 0.42, -0.34, 0.16, 0.08, 0.16, 0xbfe3f5); box(0.32, 0.5, -0.34, 0.18, 0.02, 0.18, T());
    box(-0.3, 0.02, 0.38, 0.22, 0.1, 0.16, 0x727b84); box(0.0, 0.02, 0.38, 0.22, 0.1, 0.16, 0x727b84);
  },
  tower() {
    cyl(0, -0.4, 0, 0.44, 0.45, DSTONE, 8); cyl(0, 0.05, 0, 0.34, 0.75, STONE, 8, 0.82);
    cyl(0, 0.8, 0, 0.36, 0.12, DSTONE, 8);
    for (let a = 0; a < 8; a += 2) box(Math.cos(a * 0.785) * 0.3, 0.92, Math.sin(a * 0.785) * 0.3, 0.12, 0.08, 0.12, DSTONE);
    cone(0, 0.92, 0, 0.24, 0.3, T(), 8);
  },
};

// ---------------------------------------------------------------- scenery, projectiles
function tree(conifer) {
  cyl(0, 0, 0, 0.07, 0.3, 0x5a3d22, 5);
  if (conifer) { cone(0, 0.22, 0, 0.36, 0.5, 0x2f5d33, 6); cone(0, 0.5, 0, 0.27, 0.45, 0x3c7340, 6); cone(0, 0.78, 0, 0.17, 0.38, 0x4c8a4e, 6); }
  else { slab(ring(0, 0, 0.4, 7), 0.28, 0.62, 0x3f7a36, 0.85); slab(ring(0.04, 0.03, 0.32, 7), 0.62, 0.9, 0x4f9343, 0.5); }
}
function ore() {
  for (const [x, z, r, h] of [[-0.14, 0.04, 0.2, 0.26], [0.16, -0.06, 0.22, 0.36], [0.06, 0.22, 0.15, 0.18], [-0.1, -0.22, 0.13, 0.16]]) slab(ring(x, z, r, 5), 0, h, 0x767d86, 0.5);
  for (const [x, z, y] of [[-0.16, 0.02, 0.24], [0.17, -0.07, 0.33], [0.05, 0.2, 0.16]]) box(x, y, z, 0.07, 0.06, 0.07, 0xdfe9f5, 0.3);
}
function berry() {
  slab(ring(0, 0, 0.36, 7), 0, 0.26, 0x3d7a35, 0.7);
  for (const [x, z] of [[-0.16, -0.1], [0.12, -0.14], [0.18, 0.08], [-0.05, 0.16], [0.02, -0.02], [-0.2, 0.1]]) box(x, 0.2, z, 0.07, 0.08, 0.07, 0xd6263d);
}
const P = {
  arrow() { box(0, 0, 0, 0.5, 0.025, 0.025, 0xd8c9a0); box(0.26, -0.01, 0, 0.08, 0.05, 0.05, 0xe9ecef, 0.2); },
  bullet() { box(0, 0, 0, 0.34, 0.03, 0.03, 0xfff3c4); },
  shell() { cyl(0, -0.06, 0, 0.09, 0.14, 0x2b2b2b, 6, 0.6); },
  torp() { box(0, 0, 0, 0.6, 0.09, 0.09, 0x59626c); box(0.33, 0, 0, 0.08, 0.09, 0.09, 0xc0392b, 0.5); box(-0.45, 0.07, 0, 0.3, 0.02, 0.16, 0xe8f4fa); },
  missile() { box(0, 0, 0, 0.5, 0.07, 0.07, 0xf1f3f5); box(0.29, 0, 0, 0.1, 0.07, 0.07, 0xd9342b, 0.2); box(-0.24, -0.03, 0, 0.1, 0.13, 0.02, 0x868e96); box(-0.24, 0.025, 0, 0.1, 0.02, 0.13, 0x868e96); box(-0.33, 0.005, 0, 0.14, 0.06, 0.06, 0xffb347, 0.4); },
  bomb() { slab(mirror([[0.2, 0], [0.1, -0.08], [-0.14, -0.08], [-0.2, 0]]), -0.07, 0.07, 0x30343a, 0.6); box(-0.2, -0.07, 0, 0.08, 0.14, 0.14, 0x555c64); },
};
function flag() { box(0, 0, 0, 0.04, 1.1, 0.04, 0xe9ecef); box(0, 0.78, 0.2, 0.02, 0.3, 0.4, T()); }

/** Model id -> generator. Ids 0..=42 match the unit/building table in src/defs.rs. */
const MODELS = {
  0: B.tc, 1: B.house, 2: B.farm, 3: B.barracks, 4: B.workshop, 5: B.dock, 6: B.airfield, 7: B.tower,
  8: () => inf({ head: 0xd9c074, wl: 0.22, wc: WOOD, pack: 0x7a5a34 }),
  9: () => inf({ head: SKIN, wl: 0.26, wc: DWOOD, wt: 0.06 }),
  10: () => inf({ head: SKIN, wl: 0.16, wc: 0x3b2a18 }),
  11: () => inf({ head: SKIN, helm: 0xb08d57, wl: 0.5, wc: WOOD, tip: 1, shield: 0x8b6b3d }),
  12: () => inf({ head: SKIN, helm: 0x5b7a3a, bow: 1 }),
  13: () => inf({ head: STEEL, helm: 0xcfd6dd, wl: 0.34, wc: 0xdfe5ec, shield: 0xc7ccd1 }),
  14: () => inf({ head: STEEL, helm: 0x6c757d, wl: 0.3, wc: DWOOD, wt: 0.05 }),
  15: () => inf({ head: 0x222222, helm: 0x111111, wl: 0.46, wc: GUN }),
  16: () => inf({ head: 0x5d6b3a, helm: 0x4b5a2d, wl: 0.4, wc: GUN, wt: 0.04 }),
  17: () => inf({ head: 0x3d4a3a, helm: 0x2b352a, wl: 0.32, wc: GUN, wt: 0.05, pack: 0x4b5a35 }),
  18: () => cav({ horse: 0x8a5a2b, head: SKIN, helm: 0xb08d57, wl: 0.3, wc: 0xdfe5ec }),
  19: () => cav({ horse: 0x5a3a1c, armor: 0xb9c0c7, head: STEEL, helm: 0xcfd6dd, wl: 0.6, wc: WOOD }),
  20: () => { for (const z of [-0.22, 0.22]) box(0, 0, z, 0.8, 0.07, 0.07, DWOOD); box(-0.3, 0.04, 0, 0.06, 0.06, 0.44, WOOD); box(0.2, 0.04, 0, 0.06, 0.06, 0.44, WOOD); for (const z of [-0.2, 0.2]) box(0, 0.07, z, 0.07, 0.5, 0.06, WOOD); box(0.05, 0.56, 0, 0.95, 0.06, 0.06, WOOD); box(-0.4, 0.34, 0, 0.16, 0.2, 0.18, DSTONE); cyl(0.5, 0.5, 0, 0.07, 0.1, 0x6f6f6f, 6); box(0, 0.36, 0, 0.1, 0.1, 0.36, T()); },
  21: () => cav({ horse: 0x3b2a1a, head: 0x222222, helm: 0x111111, wl: 0.4, wc: GUN }),
  22: () => { for (const z of [-0.24, 0.24]) cyl(0, 0, z, 0.15, 0.06, DWOOD, 8); box(-0.3, 0.08, 0, 0.4, 0.05, 0.06, WOOD); box(0, 0.12, 0, 0.2, 0.1, 0.3, T()); box(0.18, 0.2, 0, 0.56, 0.1, 0.1, 0x3a3f45); box(0.45, 0.19, 0, 0.06, 0.12, 0.12, 0x24272b); },
  23: () => tank(0.84, 0.54, OLIVE, 0.3, 0.42),
  24: () => gunCarriage(0.74, 0.46, OLIVE, 0.7, 0.07),
  25: () => { tracks(0.4, 0.5); wheels([0.28], 0.5); box(0, 0.07, 0, 0.8, 0.14, 0.38, 0x66704a); box(0.26, 0.21, 0, 0.2, 0.1, 0.34, 0x66704a); cyl(-0.1, 0.21, 0, 0.13, 0.06, T(), 8); for (const z of [-0.05, 0.05]) box(0.08, 0.3, z, 0.4, 0.03, 0.03, GUN); },
  26: () => tank(0.96, 0.58, 0x6c6f5a, 0.36, 0.52),
  27: () => { wheels([-0.3, -0.06, 0.3], 0.52); box(0, 0.08, 0, 0.9, 0.12, 0.4, 0x59634f); box(0.32, 0.2, 0, 0.2, 0.12, 0.36, 0x59634f); box(-0.12, 0.2, 0, 0.5, 0.16, 0.36, T(), 0.95); for (const z of [-0.12, -0.04, 0.04, 0.12]) box(-0.1, 0.37, z, 0.48, 0.05, 0.05, 0xe9ecef); },
  28: () => { wheels([-0.28, 0.28], 0.5); box(0, 0.08, 0, 0.88, 0.12, 0.4, 0x59634f); box(-0.1, 0.2, 0, 0.3, 0.1, 0.34, T()); for (const z of [-0.09, 0.09]) { box(0, 0.33, z, 0.6, 0.07, 0.07, 0xf1f3f5); box(0.33, 0.33, z, 0.08, 0.07, 0.07, 0xd9342b, 0.2); } },
  29: () => { hull(0.96, 0.24, 0.07, DWOOD, 0xb08a55); mast(0.06, 0.5, 0.2); for (let x = -0.24; x <= 0.2; x += 0.09) box(x, 0.05, 0, 0.012, 0.012, 0.5, WOOD); },
  30: () => { hull(1.0, 0.32, 0.1, DWOOD, 0xb08a55); mast(-0.22, 0.5, 0.2); mast(0.1, 0.62, 0.24); mast(0.32, 0.4, 0.16); box(-0.36, 0.1, 0, 0.22, 0.08, 0.24, WOOD); },
  31: () => { hull(1.08, 0.3, 0.1, 0x3b2a1a, 0xa37c48); mast(-0.28, 0.56, 0.2); mast(0.04, 0.7, 0.24); mast(0.3, 0.5, 0.18); for (const x of [-0.2, -0.05, 0.1]) box(x, 0.05, 0, 0.03, 0.03, 0.34, GUN); },
  32: () => { hull(1.1, 0.2, 0.09, NAVY, DECK); box(-0.02, 0.1, 0, 0.26, 0.1, 0.13, T()); box(0, 0.2, 0, 0.12, 0.07, 0.1, 0xbfc7cf); cyl(-0.2, 0.1, 0, 0.035, 0.16, 0x1d1f22, 6); turret(0.3, 1); turret(-0.38, -1); box(0, 0.27, 0, 0.015, 0.14, 0.015, GUN); },
  33: () => { hull(1.2, 0.32, 0.11, 0x525b64, 0x7f8992); box(-0.06, 0.12, 0, 0.34, 0.12, 0.2, T()); box(-0.04, 0.24, 0, 0.16, 0.12, 0.13, 0xbfc7cf); for (const x of [-0.22, -0.32]) cyl(x, 0.12, 0, 0.04, 0.2, 0x1d1f22, 6); turret(0.38, 1, 0.07, 0.12); turret(0.22, 1, 0.07, 0.165); turret(-0.46, -1, 0.07, 0.12); box(-0.04, 0.36, 0, 0.02, 0.2, 0.02, GUN); },
  34: () => sub(1.0, 0.18, false),
  35: () => { hull(1.16, 0.24, 0.1, NAVY, DECK); box(0, 0.11, 0, 0.3, 0.14, 0.16, T()); box(0.02, 0.25, 0, 0.12, 0.1, 0.12, 0xbfc7cf); cyl(-0.18, 0.11, 0, 0.035, 0.18, 0x1d1f22, 6); turret(0.42, 1); for (const x of [0.24, -0.36]) { box(x, 0.11, 0, 0.14, 0.03, 0.14, 0x2a2e33); for (const z of [-0.035, 0.035]) cyl(x, 0.14, z, 0.02, 0.01, 0xe9ecef, 6); } cone(0.02, 0.35, 0, 0.05, 0.1, 0xe9ecef, 6); },
  36: () => sub(1.2, 0.22, true),
  37: () => prop(false), 38: () => prop(true), 39: jet, 40: stealth, 41: heli,
  42: () => { hull(1.0, 0.36, 0.08, 0x6b5a45, 0x8a7a5a); box(-0.3, 0.09, 0, 0.2, 0.16, 0.26, T()); box(-0.3, 0.25, 0, 0.14, 0.06, 0.2, 0xbfc7cf); for (const z of [-0.16, 0.16]) box(0.1, 0.09, z, 0.56, 0.07, 0.02, 0x4d555e); box(0.44, 0.09, 0, 0.03, 0.12, 0.26, 0x4d555e); },
  100: () => tree(false), 101: () => tree(true), 102: ore, 103: berry,
  104: P.arrow, 105: P.bullet, 106: P.shell, 107: P.torp, 108: P.missile, 109: P.bomb, 110: flag,
};

/** Build every mesh into one vertex buffer. Returns { data, first[id], count[id] } in vertices. */
export function buildModels() {
  const first = new Int32Array(256), count = new Int32Array(256);
  const all = [];
  for (const id in MODELS) {
    V = [];
    MODELS[id]();
    first[id] = all.length / F; count[id] = V.length / F;
    for (let i = 0; i < V.length; i++) all.push(V[i]);
  }
  return { data: new Float32Array(all), first, count, stride: F * 4 };
}
