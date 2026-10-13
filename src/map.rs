//! Procedural map, passability and shared flow-field pathfinding.
//!
//! Pathfinding is the classic source of RTS stutter: N units each running A*.
//! Here a move order computes ONE flow field (a BFS over the tile grid) that
//! every unit heading to that goal shares, so cost is independent of group size.

pub const N: usize = 256;
pub const NN: usize = N * N;

pub const T_DEEP: u8 = 0;
pub const T_SHALLOW: u8 = 1;
pub const T_SAND: u8 = 2;
pub const T_GRASS: u8 = 3;
pub const T_FOREST: u8 = 4;
pub const T_ROCK: u8 = 5;

pub const R_NONE: u8 = 0;
pub const R_FOOD: u8 = 1;
pub const R_WOOD: u8 = 2;
pub const R_METAL: u8 = 3;

/// Cardinals first so BFS prefers straight moves; index 8 means "at goal".
pub const DIRS: [(i32, i32); 8] = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1)];
const FIELD_SLOTS: usize = 96;

pub struct Rng(pub u32);
impl Rng {
    #[inline]
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next() & 0xff_ffff) as f32 / 16_777_216.0
    }
    #[inline]
    pub fn range(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

#[inline]
pub fn hash(x: i32, y: i32, s: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}
#[inline]
fn hf(x: i32, y: i32, s: u32) -> f32 {
    (hash(x, y, s) & 0xff_ffff) as f32 / 16_777_216.0
}
fn vnoise(x: f32, y: f32, s: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let fx = x - xi;
    let fy = y - yi;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let (xi, yi) = (xi as i32, yi as i32);
    let a = hf(xi, yi, s);
    let b = hf(xi + 1, yi, s);
    let c = hf(xi, yi + 1, s);
    let d = hf(xi + 1, yi + 1, s);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}
fn fbm(x: f32, y: f32, s: u32, oct: u32) -> f32 {
    let (mut a, mut f, mut sum, mut norm) = (0.5, 1.0, 0.0, 0.0);
    for o in 0..oct {
        sum += a * vnoise(x * f, y * f, s.wrapping_add(o * 101));
        norm += a;
        a *= 0.5;
        f *= 2.0;
    }
    sum / norm
}

pub struct Field {
    pub key: u32,
    pub dir: Vec<u8>,
    pub ver: u32,
    pub used: u32,
}

pub struct Map {
    pub ter: Vec<u8>,
    pub h: Vec<f32>,
    pub res: Vec<u8>,
    pub amt: Vec<u16>,
    pub blocked: Vec<bool>,
    /// `pass[0]` = land movement, `pass[1]` = sea movement.
    pub pass: [Vec<bool>; 2],
    pub rgba: Vec<u8>,
    pub dirty: bool,
    pub fields: Vec<Field>,
    pub nav_ver: u32,
    pub budget: u32,
    queue: Vec<u32>,
    pub starts: [(usize, usize); 4],
    pub seed: u32,
    /// Territory id of every tile, and the point each territory grew from.
    pub terr: Vec<u8>,
    pub tseeds: Vec<(f32, f32)>,
    /// Terrain elevation at tile corners, `(N + 1)^2` values, for the 3D view.
    pub hm: Vec<f32>,
}

#[inline]
pub fn tile_of(x: f32, y: f32) -> usize {
    let tx = (x as i32).clamp(0, N as i32 - 1) as usize;
    let ty = (y as i32).clamp(0, N as i32 - 1) as usize;
    ty * N + tx
}

impl Map {
    pub fn new(seed: u32, players: usize) -> Map {
        let s0 = (N as f32 * 0.2) as usize;
        let s1 = N - s0;
        let mut m = Map {
            ter: vec![0; NN],
            h: vec![0.0; NN],
            res: vec![0; NN],
            amt: vec![0; NN],
            blocked: vec![false; NN],
            pass: [vec![false; NN], vec![false; NN]],
            rgba: vec![0; NN * 4],
            dirty: true,
            fields: (0..FIELD_SLOTS).map(|_| Field { key: u32::MAX, dir: Vec::new(), ver: 0, used: 0 }).collect(),
            nav_ver: 1,
            budget: 4,
            queue: Vec::with_capacity(NN),
            starts: [(s0, s0), (s1, s1), (s1, s0), (s0, s1)],
            seed,
            terr: vec![0; NN],
            tseeds: Vec::new(),
            hm: vec![0.0; (N + 1) * (N + 1)],
        };
        let mut seed = seed;
        for attempt in 0..24 {
            m.gen_terrain(seed, players);
            if m.connected(players) {
                break;
            }
            seed = seed.wrapping_mul(31).wrapping_add(7);
            if attempt == 23 {
                m.carve(players);
            }
        }
        m.seed = seed;
        m.gen_resources(seed, players);
        for i in 0..NN {
            m.upd_pass(i);
            m.recolor(i);
        }
        m.gen_territories();
        m.gen_heights();
        m
    }

    /// Voronoi regions around the four corner starts plus a jittered grid of extra seeds.
    fn gen_territories(&mut self) {
        let mut rng = Rng(self.seed ^ 0x51ed_270b | 1);
        let mut seeds: Vec<(f32, f32)> = self.starts.iter().map(|s| (s.0 as f32, s.1 as f32)).collect();
        for gy in 0..5 {
            for gx in 0..5 {
                let x = (gx as f32 + 0.25 + rng.f() * 0.5) * N as f32 / 5.0;
                let y = (gy as f32 + 0.25 + rng.f() * 0.5) * N as f32 / 5.0;
                if seeds.iter().all(|s| (s.0 - x).powi(2) + (s.1 - y).powi(2) > 38.0 * 38.0) {
                    seeds.push((x, y));
                }
            }
        }
        for i in 0..NN {
            let (x, y) = ((i % N) as f32 + 0.5, (i / N) as f32 + 0.5);
            let mut best = 0;
            let mut bd = f32::MAX;
            for (k, s) in seeds.iter().enumerate() {
                let d = (s.0 - x).powi(2) + (s.1 - y).powi(2);
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            self.terr[i] = best as u8;
        }
        self.tseeds = seeds;
    }

    fn elev(&self, i: usize) -> f32 {
        let h = self.h[i];
        match self.ter[i] {
            T_DEEP | T_SHALLOW => ((h - 0.455) * 7.0).clamp(-1.6, -0.08),
            T_ROCK => 1.5 + (h - 0.83) * 6.0,
            _ => ((h - 0.455) * 3.0).clamp(0.04, 1.3),
        }
    }

    fn gen_heights(&mut self) {
        let hn = N + 1;
        for vy in 0..hn {
            for vx in 0..hn {
                let (mut sum, mut n) = (0.0, 0.0);
                for (ox, oy) in [(0i32, 0i32), (-1, 0), (0, -1), (-1, -1)] {
                    let (tx, ty) = (vx as i32 + ox, vy as i32 + oy);
                    if tx >= 0 && ty >= 0 && tx < N as i32 && ty < N as i32 {
                        sum += self.elev(ty as usize * N + tx as usize);
                        n += 1.0;
                    }
                }
                self.hm[vy * hn + vx] = sum / n;
            }
        }
    }

    /// Bilinear terrain elevation at a world position.
    pub fn height(&self, x: f32, y: f32) -> f32 {
        let hn = N + 1;
        let x = x.clamp(0.0, N as f32 - 0.001);
        let y = y.clamp(0.0, N as f32 - 0.001);
        let (xi, yi) = (x as usize, y as usize);
        let (fx, fy) = (x - xi as f32, y - yi as f32);
        let i = yi * hn + xi;
        let a = self.hm[i] + (self.hm[i + 1] - self.hm[i]) * fx;
        let b = self.hm[i + hn] + (self.hm[i + hn + 1] - self.hm[i + hn]) * fx;
        a + (b - a) * fy
    }

    fn gen_terrain(&mut self, seed: u32, players: usize) {
        for y in 0..N {
            for x in 0..N {
                let i = y * N + x;
                let (fx, fy) = (x as f32, y as f32);
                let mut v = fbm(fx * 0.021, fy * 0.021, seed, 5);
                let ex = fx.min(N as f32 - 1.0 - fx) / N as f32;
                let ey = fy.min(N as f32 - 1.0 - fy) / N as f32;
                let e = ex.min(ey);
                v -= (0.10 - e).max(0.0) * 4.0;
                for p in 0..players {
                    let (sx, sy) = self.starts[p];
                    let d2 = (fx - sx as f32).powi(2) + (fy - sy as f32).powi(2);
                    v += 0.28 * (-d2 / (24.0 * 24.0)).exp();
                }
                self.h[i] = v;
                self.ter[i] = if v < 0.42 {
                    T_DEEP
                } else if v < 0.455 {
                    T_SHALLOW
                } else if v < 0.48 {
                    T_SAND
                } else if v > 0.83 {
                    T_ROCK
                } else if fbm(fx * 0.07, fy * 0.07, seed ^ 0x9e37, 3) > 0.60 {
                    T_FOREST
                } else {
                    T_GRASS
                };
            }
        }
        // Flat clearing around every start.
        for p in 0..players {
            let (sx, sy) = self.starts[p];
            for y in sy - 7..=sy + 7 {
                for x in sx - 7..=sx + 7 {
                    let d2 = (x as i32 - sx as i32).pow(2) + (y as i32 - sy as i32).pow(2);
                    if d2 <= 49 {
                        self.ter[y * N + x] = T_GRASS;
                    }
                }
            }
        }
    }

    fn land(&self, i: usize) -> bool {
        self.ter[i] >= T_SAND && self.ter[i] != T_ROCK
    }

    fn connected(&mut self, players: usize) -> bool {
        let mut seen = vec![false; NN];
        let (sx, sy) = self.starts[0];
        self.queue.clear();
        self.queue.push((sy * N + sx) as u32);
        seen[sy * N + sx] = true;
        let mut head = 0;
        while head < self.queue.len() {
            let c = self.queue[head] as usize;
            head += 1;
            let (cx, cy) = ((c % N) as i32, (c / N) as i32);
            for &(dx, dy) in &DIRS[..4] {
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                    continue;
                }
                let ni = ny as usize * N + nx as usize;
                if !seen[ni] && self.land(ni) {
                    seen[ni] = true;
                    self.queue.push(ni as u32);
                }
            }
        }
        (1..players).all(|p| seen[self.starts[p].1 * N + self.starts[p].0])
    }

    /// Last resort: cut a land bridge between starts.
    fn carve(&mut self, players: usize) {
        for p in 1..players {
            let (ax, ay) = (self.starts[0].0 as f32, self.starts[0].1 as f32);
            let (bx, by) = (self.starts[p].0 as f32, self.starts[p].1 as f32);
            for k in 0..=400 {
                let t = k as f32 / 400.0;
                let (x, y) = ((ax + (bx - ax) * t) as i32, (ay + (by - ay) * t) as i32);
                for oy in -2..=2 {
                    for ox in -2..=2 {
                        let i = tile_of((x + ox) as f32, (y + oy) as f32);
                        if !self.land(i) {
                            self.ter[i] = T_SAND;
                        }
                    }
                }
            }
        }
    }

    fn blob(&mut self, rng: &mut Rng, cx: i32, cy: i32, count: u32, kind: u8, amt: u16) {
        let (mut x, mut y) = (cx, cy);
        for _ in 0..count * 3 {
            if x > 2 && y > 2 && x < N as i32 - 3 && y < N as i32 - 3 {
                let i = y as usize * N + x as usize;
                if (self.ter[i] == T_GRASS || (kind == R_WOOD && self.ter[i] == T_SAND)) && self.res[i] == R_NONE {
                    self.res[i] = kind;
                    self.amt[i] = amt;
                    if kind == R_WOOD {
                        self.ter[i] = T_FOREST;
                    }
                }
            }
            let d = DIRS[rng.range(4) as usize];
            x = (x + d.0).clamp(cx - 2, cx + 2);
            y = (y + d.1).clamp(cy - 2, cy + 2);
        }
    }

    fn gen_resources(&mut self, seed: u32, players: usize) {
        let mut rng = Rng(seed | 1);
        for i in 0..NN {
            self.res[i] = R_NONE;
            self.amt[i] = 0;
            if self.ter[i] == T_FOREST {
                self.res[i] = R_WOOD;
                self.amt[i] = 150;
            }
        }
        for _ in 0..70 {
            let (x, y) = (rng.range(N as u32) as i32, rng.range(N as u32) as i32);
            self.blob(&mut rng, x, y, 8, R_METAL, 1500);
        }
        for _ in 0..40 {
            let (x, y) = (rng.range(N as u32) as i32, rng.range(N as u32) as i32);
            self.blob(&mut rng, x, y, 6, R_FOOD, 350);
        }
        // Guaranteed starting resources, same layout for everyone.
        for p in 0..players {
            let (sx, sy) = (self.starts[p].0 as i32, self.starts[p].1 as i32);
            let a0 = rng.f() * 6.283;
            let kinds = [(R_FOOD, 6.5, 8, 350), (R_METAL, 8.0, 9, 1500), (R_WOOD, 9.5, 14, 150), (R_WOOD, 9.5, 14, 150), (R_METAL, 13.0, 9, 1500), (R_FOOD, 12.0, 8, 350)];
            for (k, &(kind, dist, n, amt)) in kinds.iter().enumerate() {
                let a = a0 + k as f32 * 1.05;
                let (x, y) = (sx + (a.cos() * dist) as i32, sy + (a.sin() * dist) as i32);
                // clear whatever was there so the blob always lands
                for oy in -2..=2 {
                    for ox in -2..=2 {
                        let i = tile_of((x + ox) as f32, (y + oy) as f32);
                        if self.land(i) || self.ter[i] == T_ROCK {
                            self.ter[i] = T_GRASS;
                            self.res[i] = R_NONE;
                        }
                    }
                }
                self.blob(&mut rng, x, y, n, kind, amt);
            }
        }
    }

    pub fn upd_pass(&mut self, i: usize) {
        let t = self.ter[i];
        let b = self.blocked[i];
        self.pass[0][i] = t >= T_SAND && t != T_ROCK && !b;
        self.pass[1][i] = t <= T_SHALLOW && !b;
    }

    /// Base colour of a tile for the terrain texture / minimap. Alpha marks land (255) vs water (0).
    pub fn recolor(&mut self, i: usize) {
        let (x, y) = ((i % N) as i32, (i / N) as i32);
        let n = (hash(x, y, 77) & 255) as f32 / 255.0 - 0.5;
        let h = self.h[i];
        let (r, g, b, a) = match self.ter[i] {
            T_DEEP => {
                let d = ((0.42 - h) * 4.0).clamp(0.0, 1.0);
                (20.0 - 10.0 * d, 62.0 - 26.0 * d, 112.0 - 34.0 * d, 0.0)
            }
            T_SHALLOW => (36.0, 104.0, 142.0, 40.0),
            T_SAND => (200.0 + n * 14.0, 184.0 + n * 12.0, 132.0 + n * 10.0, 255.0),
            T_ROCK => (112.0 + n * 26.0, 106.0 + n * 24.0, 100.0 + n * 22.0, 255.0),
            T_FOREST => (62.0 + n * 10.0, 108.0 + n * 14.0, 50.0 + n * 8.0, 255.0),
            _ => {
                let k = ((h - 0.48) * 2.2).clamp(0.0, 1.0);
                (84.0 + k * 34.0 + n * 12.0, 134.0 - k * 14.0 + n * 14.0, 60.0 + k * 6.0 + n * 8.0, 255.0)
            }
        };
        let o = i * 4;
        self.rgba[o] = r as u8;
        self.rgba[o + 1] = g as u8;
        self.rgba[o + 2] = b as u8;
        self.rgba[o + 3] = a as u8;
        self.dirty = true;
    }

    /// Nearest tile passable for domain class `dc` (0 land, 1 sea).
    pub fn snap(&self, x: f32, y: f32, dc: usize) -> usize {
        let t = tile_of(x, y);
        if self.pass[dc][t] {
            return t;
        }
        let (cx, cy) = ((t % N) as i32, (t / N) as i32);
        for r in 1..40i32 {
            let mut best = usize::MAX;
            let mut bd = i32::MAX;
            for k in -r..=r {
                for &(nx, ny) in &[(cx + k, cy - r), (cx + k, cy + r), (cx - r, cy + k), (cx + r, cy + k)] {
                    if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                        continue;
                    }
                    let ni = ny as usize * N + nx as usize;
                    let d = (nx - cx).pow(2) + (ny - cy).pow(2);
                    if self.pass[dc][ni] && d < bd {
                        bd = d;
                        best = ni;
                    }
                }
            }
            if best != usize::MAX {
                return best;
            }
        }
        t
    }

    /// Nearest tile holding resource `kind` within `maxr` rings of `t`.
    pub fn nearest_res(&self, t: usize, kind: u8, maxr: i32) -> Option<usize> {
        if self.res[t] == kind {
            return Some(t);
        }
        let (cx, cy) = ((t % N) as i32, (t / N) as i32);
        for r in 1..=maxr {
            let mut best = None;
            let mut bd = i32::MAX;
            for k in -r..=r {
                for &(nx, ny) in &[(cx + k, cy - r), (cx + k, cy + r), (cx - r, cy + k), (cx + r, cy + k)] {
                    if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                        continue;
                    }
                    let ni = ny as usize * N + nx as usize;
                    let d = (nx - cx).pow(2) + (ny - cy).pow(2);
                    if self.res[ni] == kind && d < bd {
                        bd = d;
                        best = Some(ni);
                    }
                }
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }

    fn build_field(&mut self, slot: usize, key: u32) {
        let goal = (key & 0xffff) as usize;
        let dc = (key >> 16) as usize & 1;
        let mut dir = std::mem::take(&mut self.fields[slot].dir);
        dir.clear();
        dir.resize(NN, 255);
        let pass = &self.pass[dc];
        let q = &mut self.queue;
        q.clear();
        q.push(goal as u32);
        dir[goal] = 8;
        let mut head = 0;
        while head < q.len() {
            let c = q[head] as usize;
            head += 1;
            let (cx, cy) = ((c % N) as i32, (c / N) as i32);
            for k in 0..8 {
                let (dx, dy) = DIRS[k];
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                    continue;
                }
                let ni = ny as usize * N + nx as usize;
                if dir[ni] != 255 || !pass[ni] {
                    continue;
                }
                if k >= 4 && !(pass[cy as usize * N + nx as usize] && pass[ny as usize * N + cx as usize]) {
                    continue; // no cutting corners
                }
                // store the direction that leads from `ni` back toward the goal
                dir[ni] = if k < 4 { (k as u8 + 2) % 4 } else { 4 + (k as u8 - 4 + 2) % 4 };
                q.push(ni as u32);
            }
        }
        let f = &mut self.fields[slot];
        f.dir = dir;
        f.key = key;
        f.ver = self.nav_ver;
    }

    /// Direction index (0..8) to walk from `tile` toward the goal encoded in `key`,
    /// 8 at the goal, 255 if unknown/unreachable. `slot` caches the field location.
    pub fn flow(&mut self, slot: &mut u16, key: u32, tile: usize, tick: u32) -> u8 {
        let mut s = *slot as usize;
        if s >= FIELD_SLOTS || self.fields[s].key != key {
            s = usize::MAX;
            let mut lru = 0;
            let mut oldest = u32::MAX;
            for (k, f) in self.fields.iter().enumerate() {
                if f.key == key {
                    s = k;
                    break;
                }
                if f.used < oldest {
                    oldest = f.used;
                    lru = k;
                }
            }
            if s == usize::MAX {
                if self.budget == 0 || oldest + 2 > tick {
                    return 255; // over budget this tick: caller steers directly
                }
                self.budget -= 1;
                self.build_field(lru, key);
                s = lru;
            }
            *slot = s as u16;
        }
        if self.fields[s].ver != self.nav_ver && self.budget > 0 {
            self.budget -= 1;
            self.build_field(s, key);
        }
        self.fields[s].used = tick;
        self.fields[s].dir[tile]
    }
}
