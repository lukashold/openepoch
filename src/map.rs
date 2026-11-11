//! Procedural map, passability and shared flow-field pathfinding.
//!
//! Pathfinding is the classic source of RTS stutter: N units each running A*.
//! Here a move order computes ONE flow field (a BFS over the tile grid) that
//! every unit heading to that goal shares, so cost is independent of group size.

pub const N: usize = 256;
pub const NN: usize = N * N;
pub(crate) const LEGACY_REGION_CAPACITY: usize = 32;
pub(crate) const REGION_CAPACITY: usize = RegionMask::BITS as usize;
pub(crate) type RegionMask = u64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapLayout {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) stride: usize,
    pub(crate) rows: usize,
}

impl MapLayout {
    const LEGACY: Self = Self { width: N, height: N, stride: N, rows: N };
    #[cfg(feature = "ee2-authored-rect233")]
    pub(crate) const DEATH_VALLEY2: Self = Self { width: 100, height: 300, stride: 128, rows: 512 };

    #[inline]
    pub(crate) fn index(self, x: usize, y: usize) -> usize { y * self.stride + x }

    #[inline]
    pub(crate) fn xy(self, tile: usize) -> [usize; 2] { [tile % self.stride, tile / self.stride] }

    #[inline]
    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        (x as usize) < self.width && (y as usize) < self.height
    }

    #[inline]
    pub(crate) fn contains_point(self, x: f32, y: f32) -> bool {
        (0.0..self.width as f32).contains(&x) && (0.0..self.height as f32).contains(&y)
    }

    #[inline]
    fn tile_of(self, x: f32, y: f32) -> usize {
        self.index((x as i32).clamp(0, self.width as i32 - 1) as usize,
            (y as i32).clamp(0, self.height as i32 - 1) as usize)
    }

    pub(crate) fn profile(self) -> u32 { u32::from(self != Self::LEGACY) }
}

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
pub const R_STONE:u8=4;
pub const R_GOLD:u8=5;
pub const R_TIN:u8=6;
pub const R_SALTPETER:u8=7;
pub const R_OIL:u8=8;
pub const R_URANIUM:u8=9;
pub const R_FISH:u8=10;

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
    pub(crate) denied_territories: RegionMask,
    pub dir: Vec<u8>,
    pub ver: u32,
    pub used: u32,
}

pub struct Map {
    pub(crate) layout: MapLayout,
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
    /// Shared tile-edge borders, matching native neighbor pairs in 00723ba0.
    /// This graph is derived from the map, never serialized as mutable state.
    pub(crate) territory_neighbors: [RegionMask; REGION_CAPACITY],
    /// Terrain elevation at padded tile corners, for the 3D view.
    pub hm: Vec<f32>,
    #[cfg(feature = "ee2-authored-valley37")]
    pub(crate) source_world: Option<Box<crate::native_valley37::Geometry>>,
    #[cfg(feature = "ee2-authored-rect233")]
    pub(crate) rect_surface_index: Vec<Option<usize>>,
}

#[inline]
pub fn tile_of(x: f32, y: f32) -> usize {
    MapLayout::LEGACY.tile_of(x, y)
}

impl Map {
    pub(crate) fn blank(seed: u32) -> Map {
        Self::blank_with_layout(seed, MapLayout::LEGACY)
    }

    pub(crate) fn blank_with_layout(seed: u32, layout: MapLayout) -> Map {
        assert_eq!(layout.stride * layout.rows, NN);
        let s0 = (N as f32 * 0.2) as usize;
        let s1 = N - s0;
        Map {
            layout,
            ter: vec![0; NN],
            h: vec![0.0; NN],
            res: vec![0; NN],
            amt: vec![0; NN],
            blocked: vec![false; NN],
            pass: [vec![false; NN], vec![false; NN]],
            rgba: vec![0; NN * 4],
            dirty: true,
            fields: (0..FIELD_SLOTS).map(|_| Field { key: u32::MAX, denied_territories: 0, dir: Vec::new(), ver: 0, used: 0 }).collect(),
            nav_ver: 1,
            budget: 4,
            queue: Vec::with_capacity(NN),
            starts: [(s0, s0), (s1, s1), (s1, s0), (s0, s1)],
            seed,
            terr: vec![0; NN],
            tseeds: Vec::new(),
            territory_neighbors: [0; REGION_CAPACITY],
            hm: vec![0.0; (layout.stride + 1) * (layout.rows + 1)],
            #[cfg(feature = "ee2-authored-valley37")]
            source_world: None,
            #[cfg(feature = "ee2-authored-rect233")]
            rect_surface_index: Vec::new(),
        }
    }

    pub(crate) fn world_dimensions(&self) -> [usize; 2] {
        #[cfg(feature = "ee2-authored-valley37")]
        if let Some(world) = &self.source_world { return [world.width, world.height]; }
        self.simulation_dimensions()
    }

    /// Keep the legacy square-coordinate behavior for authored Valley, whose
    /// original play surface is separately bounded inside the square backing.
    pub(crate) fn simulation_dimensions(&self) -> [usize; 2] { [self.layout.width, self.layout.height] }

    #[inline]
    pub(crate) fn tile_of(&self, x: f32, y: f32) -> usize { self.layout.tile_of(x, y) }

    #[inline]
    pub(crate) fn tile_xy(&self, tile: usize) -> [usize; 2] { self.layout.xy(tile) }

    #[inline]
    pub(crate) fn tile_center(&self, tile: usize) -> [f32; 2] {
        self.tile_xy(tile).map(|axis| axis as f32 + 0.5)
    }

    #[inline]
    pub(crate) fn tile_index(&self, x: usize, y: usize) -> usize { self.layout.index(x, y) }

    pub(crate) fn backing_dimensions(&self) -> [usize; 2] { [self.layout.stride, self.layout.rows] }

    pub(crate) fn corner_stride(&self) -> usize { self.layout.stride + 1 }

    pub fn new(seed: u32, players: usize) -> Map {
        Self::generate(seed, players, false)
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn new_for_hill(seed: u32, players: usize) -> Map {
        Self::generate(seed, players, true)
    }

    fn generate(seed: u32, players: usize, hill: bool) -> Map {
        let mut m = Self::blank(seed);
        let mut seed = seed;
        for attempt in 0..24 {
            m.gen_terrain(seed, players);
            if m.connected(players, hill) {
                break;
            }
            seed = seed.wrapping_mul(31).wrapping_add(7);
            if attempt == 23 {
                m.carve(players);
                if hill {
                    m.carve_to((N / 2, N / 2));
                    for y in N / 2 - 3..N / 2 + 3 {
                        for x in N / 2 - 3..N / 2 + 3 {
                            let tile = y * N + x;
                            if !m.land(tile) { m.ter[tile] = T_SAND; }
                        }
                    }
                }
            }
        }
        m.seed = seed;
        m.gen_resources(seed, players);
        if cfg!(feature="ee2-reference") {m.seed_extra_resources(players);}
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
        self.rebuild_territory_neighbors();
    }

    pub fn rebuild_territory_neighbors(&mut self) {
        self.territory_neighbors = [0; REGION_CAPACITY];
        let MapLayout { width, height, stride, .. } = self.layout;
        for y in 0..height {
            for x in 0..width {
                let tile = y * stride + x;
                let a = self.terr[tile] as usize;
                for b in [if x + 1 < width {self.terr[tile + 1] as usize} else {a},
                          if y + 1 < height {self.terr[tile + stride] as usize} else {a}] {
                    if a != b && a < self.tseeds.len() && b < self.tseeds.len() {
                        self.territory_neighbors[a] |= 1 << b;
                        self.territory_neighbors[b] |= 1 << a;
                    }
                }
            }
        }
    }

    pub(crate) fn region_storage_len(&self) -> usize {
        if self.tseeds.len() > LEGACY_REGION_CAPACITY { REGION_CAPACITY } else { LEGACY_REGION_CAPACITY }
    }

    pub(crate) fn region_at(&self, tile: usize) -> Option<usize> {
        let id = usize::from(self.terr[tile]);
        (id < self.tseeds.len()).then_some(id)
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
        #[cfg(feature = "ee2-authored-valley37")]
        if let Some(world) = &self.source_world {
            let x = x.clamp(0.0, world.width as f32);
            let y = y.clamp(0.0, world.height as f32);
            #[cfg(feature = "ee2-authored-rect233")]
            if self.layout == MapLayout::DEATH_VALLEY2 {
                return crate::native_valley37::rectangular_world::source_height(world, &self.rect_surface_index, x, y, true) as f32;
            }
            return world.surface_height(&world.heights, x, y);
        }
        let hn = self.corner_stride();
        let x = x.clamp(0.0, self.layout.width as f32 - 0.001);
        let y = y.clamp(0.0, self.layout.height as f32 - 0.001);
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

    pub(crate) fn connected(&mut self, players: usize, hill: bool) -> bool {
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
            && (!hill || (N / 2 - 3..N / 2 + 3).all(|y| (N / 2 - 3..N / 2 + 3).all(|x| seen[y * N + x])))
    }

    /// Last resort: cut a land bridge between starts.
    fn carve(&mut self, players: usize) {
        for p in 1..players {
            self.carve_to(self.starts[p]);
        }
    }

    fn carve_to(&mut self, end: (usize, usize)) {
        let (ax, ay) = (self.starts[0].0 as f32, self.starts[0].1 as f32);
        let (bx, by) = (end.0 as f32, end.1 as f32);
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

    fn blob(&mut self, rng: &mut Rng, cx: i32, cy: i32, count: u32, kind: u8, amt: u16) {
        let amt=if cfg!(feature="ee2-reference"){if kind==R_WOOD{1277}else{1000}}else{amt};
        let (mut x, mut y) = (cx, cy);
        for _ in 0..count * 3 {
            if x > 2 && y > 2 && x < N as i32 - 3 && y < N as i32 - 3 {
                let i = y as usize * N + x as usize;
                if (self.ter[i] == T_GRASS || (kind == R_WOOD && self.ter[i] == T_SAND)) && self.res[i] == R_NONE && !self.blocked[i] {
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
                self.amt[i] = if cfg!(feature="ee2-reference"){1277}else{150};
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

    /// Deterministic new deposits, also used when migrating a three-resource save.
    pub fn seed_extra_resources(&mut self,players:usize){
        let mut rng=Rng(self.seed^0x91b3_78d5|1);
        for kind in R_STONE..=R_URANIUM {
            for _ in 0..24 {let x=rng.range(N as u32) as i32;let y=rng.range(N as u32) as i32;self.blob(&mut rng,x,y,6,kind,1500);}
            for p in 0..players {
                let (sx,sy)=self.starts[p];
                let a=(kind-R_STONE) as f32*1.0472+0.3;
                let (cx,cy)=(sx as i32+(a.cos()*17.0) as i32,sy as i32+(a.sin()*17.0) as i32);
                // Only unused land is changed: old deposits and structures survive migration.
                for radius in 0..8 {let mut found=false;for oy in -radius..=radius {for ox in -radius..=radius {
                    let t=tile_of((cx+ox) as f32,(cy+oy) as f32);
                    if self.ter[t]==T_GRASS&&self.res[t]==R_NONE&&!self.blocked[t] {
                        self.blob(&mut rng,(t%N) as i32,(t/N) as i32,6,kind,1500);found=true;break;
                    }
                }if found {break;}}if found {break;}}
            }
        }
    }

    pub fn upd_pass(&mut self, i: usize) {
        let [x, y] = self.tile_xy(i);
        let [width, height] = self.world_dimensions();
        if x >= width || y >= height {
            self.pass[0][i] = false;
            self.pass[1][i] = false;
            return;
        }
        let t = self.ter[i];
        let b = self.blocked[i];
        self.pass[0][i] = t >= T_SAND && t != T_ROCK && !b;
        self.pass[1][i] = t <= T_SHALLOW && !b;
    }

    /// Base colour of a tile for the terrain texture / minimap. Alpha marks land (255) vs water (0).
    pub fn recolor(&mut self, i: usize) {
        let [x, y] = self.tile_xy(i).map(|axis| axis as i32);
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
        self.snap_permitted(x, y, dc, 0)
    }

    pub(crate) fn snap_permitted(&self, x: f32, y: f32, dc: usize, denied_territories: RegionMask) -> usize {
        let t = self.tile_of(x, y);
        if self.pass[dc][t] && self.permits_territory(t, denied_territories) {
            return t;
        }
        let [cx, cy] = self.tile_xy(t).map(|axis| axis as i32);
        for r in 1..40i32 {
            let mut best = usize::MAX;
            let mut bd = i32::MAX;
            for k in -r..=r {
                for &(nx, ny) in &[(cx + k, cy - r), (cx + k, cy + r), (cx - r, cy + k), (cx + r, cy + k)] {
                    if !self.layout.contains(nx, ny) {
                        continue;
                    }
                    let ni = self.tile_index(nx as usize, ny as usize);
                    let d = (nx - cx).pow(2) + (ny - cy).pow(2);
                    if self.pass[dc][ni] && d < bd && self.permits_territory(ni, denied_territories) {
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
        let [cx, cy] = self.tile_xy(t).map(|axis| axis as i32);
        for r in 1..=maxr {
            let mut best = None;
            let mut bd = i32::MAX;
            for k in -r..=r {
                for &(nx, ny) in &[(cx + k, cy - r), (cx + k, cy + r), (cx - r, cy + k), (cx + r, cy + k)] {
                    if !self.layout.contains(nx, ny) {
                        continue;
                    }
                    let ni = self.tile_index(nx as usize, ny as usize);
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

    pub(crate) fn permits_territory(&self, tile: usize, denied_territories: RegionMask) -> bool {
        denied_territories.checked_shr(u32::from(self.terr[tile])).unwrap_or(0) & 1 == 0
    }

    fn fill_field(
        layout: MapLayout, pass: &[bool], regions: &[u8], goal: usize, denied_territories: RegionMask,
        dir: &mut Vec<u8>, q: &mut Vec<u32>,
    ) {
        dir.clear();
        dir.resize(NN, 255);
        let allowed = |tile: usize| denied_territories.checked_shr(u32::from(regions[tile])).unwrap_or(0) & 1 == 0;
        q.clear();
        if allowed(goal) {
            q.push(goal as u32);
            dir[goal] = 8;
        }
        let mut head = 0;
        while head < q.len() {
            let c = q[head] as usize;
            head += 1;
            let [cx, cy] = layout.xy(c).map(|axis| axis as i32);
            for k in 0..8 {
                let (dx, dy) = DIRS[k];
                let (nx, ny) = (cx + dx, cy + dy);
                if !layout.contains(nx, ny) {
                    continue;
                }
                let ni = layout.index(nx as usize, ny as usize);
                if dir[ni] != 255 || !pass[ni] || !allowed(ni) {
                    continue;
                }
                if k >= 4 {
                    let x_side = layout.index(nx as usize, cy as usize);
                    let y_side = layout.index(cx as usize, ny as usize);
                    if !(pass[x_side] && pass[y_side] && allowed(x_side) && allowed(y_side)) {
                        continue;
                    }
                }
                // store the direction that leads from `ni` back toward the goal
                dir[ni] = if k < 4 { (k as u8 + 2) % 4 } else { 4 + (k as u8 - 4 + 2) % 4 };
                q.push(ni as u32);
            }
        }
    }

    #[cfg(feature = "ee2-rect273-builder-waypoint")]
    pub(crate) fn land_component(&self, tile: usize, denied: RegionMask) -> Vec<u8> {
        let mut directions = Vec::new();
        Self::fill_field(self.layout, &self.pass[0], &self.terr, tile, denied, &mut directions, &mut Vec::new());
        directions
    }

    /// The same admitted, corner-safe movement graph used by flow fields,
    /// without changing their shared cache, per-tick budget or LRU order.
    #[cfg(feature = "ee2-rect291-ai-scout")]
    pub(crate) fn scouting_component(&self, dc: usize, tile: usize, denied: RegionMask) -> Vec<u8> {
        let mut directions = Vec::new();
        Self::fill_field(self.layout, &self.pass[dc], &self.terr, tile, denied, &mut directions, &mut Vec::new());
        directions
    }

    fn build_field(&mut self, slot: usize, key: u32, denied_territories: RegionMask) {
        let goal = (key & 0xffff) as usize;
        let dc = (key >> 16) as usize & 1;
        let mut dir = std::mem::take(&mut self.fields[slot].dir);
        Self::fill_field(self.layout, &self.pass[dc], &self.terr, goal, denied_territories, &mut dir, &mut self.queue);
        let f = &mut self.fields[slot];
        f.dir = dir;
        f.key = key;
        f.denied_territories = denied_territories;
        f.ver = self.nav_ver;
    }

    /// Direction index (0..8) to walk from `tile` toward the goal encoded in `key`,
    /// 8 at the goal, 255 if unknown/unreachable. `slot` caches the field location.
    pub fn flow(&mut self, slot: &mut u16, key: u32, tile: usize, tick: u32, denied_territories: RegionMask) -> u8 {
        let mut s = *slot as usize;
        if s >= FIELD_SLOTS || self.fields[s].key != key || self.fields[s].denied_territories != denied_territories {
            s = usize::MAX;
            let mut lru = 0;
            let mut oldest = u32::MAX;
            for (k, f) in self.fields.iter().enumerate() {
                if f.key == key && f.denied_territories == denied_territories {
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
                self.build_field(lru, key, denied_territories);
                s = lru;
            }
            *slot = s as u16;
        }
        if self.fields[s].ver != self.nav_ver && self.budget > 0 {
            self.budget -= 1;
            self.build_field(s, key, denied_territories);
        }
        self.fields[s].used = tick;
        self.fields[s].dir[tile]
    }
}
