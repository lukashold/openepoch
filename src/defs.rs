//! Static game data: every building and unit, its stats and its weapons.
//! All names and numbers are original to this project.

// Movement / targeting domains (bit flags so weapons can list several).
pub const LAND: u8 = 1;
pub const SEA: u8 = 2;
pub const AIR: u8 = 4;
pub const SUB: u8 = 8;

// Unit flags.
pub const F_WORKER: u16 = 1;
pub const F_DETECT: u16 = 2;
pub const F_STEALTH: u16 = 4;
pub const F_PLANE: u16 = 8;

// Weapon kinds.
pub const W_MELEE: u8 = 0;
pub const W_ARROW: u8 = 1;
pub const W_BULLET: u8 = 2;
pub const W_SHELL: u8 = 3;
pub const W_TORP: u8 = 4;
pub const W_MISSILE: u8 = 5;
pub const W_BOMB: u8 = 6;

// Armour classes.
pub const A_INF: u8 = 0;
pub const A_CAV: u8 = 1;
pub const A_VEH: u8 = 2;
pub const A_BLD: u8 = 3;
pub const A_SHIP: u8 = 4;
pub const A_AIR: u8 = 5;

/// Damage multiplier, indexed `[weapon kind][armour class]`.
pub const MULT: [[f32; 6]; 7] = [
    [1.0, 1.0, 0.5, 0.5, 0.3, 0.0],  // melee
    [1.0, 0.8, 0.25, 0.15, 0.4, 0.6], // arrow
    [1.0, 0.9, 0.4, 0.2, 0.3, 0.8],  // bullet
    [0.7, 0.8, 1.0, 1.6, 1.0, 0.1],  // shell
    [0.0, 0.0, 0.0, 1.0, 1.5, 0.0],  // torpedo
    [0.8, 1.0, 1.2, 1.2, 1.2, 1.5],  // missile
    [1.0, 1.0, 1.0, 1.8, 1.0, 0.0],  // bomb
];

#[derive(Clone, Copy)]
pub struct Wpn {
    pub range: f32,
    pub dmg: f32,
    pub cd: f32,
    pub kind: u8,
    pub targets: u8,
    pub splash: f32,
}

pub const NOW: Wpn = Wpn { range: 0.0, dmg: 0.0, cd: 1.0, kind: 0, targets: 0, splash: 0.0 };

const fn w(range: f32, dmg: f32, cd: f32, kind: u8, targets: u8, splash: f32) -> Wpn {
    Wpn { range, dmg, cd, kind, targets, splash }
}

#[derive(Clone, Copy)]
pub struct Def {
    pub name: &'static str,
    pub epoch: u8,
    /// Epoch at which the unit is hidden from build menus (replaced by a successor).
    pub until: u8,
    pub dom: u8,
    pub building: bool,
    /// Units: sprite diameter in tiles. Buildings: square footprint edge in tiles.
    pub size: f32,
    pub hp: f32,
    pub speed: f32,
    pub sight: f32,
    pub armor: u8,
    pub cost: [u16; 3],
    pub time: f32,
    pub pop: u8,
    pub made_at: u8,
    pub flags: u16,
    pub w: [Wpn; 2],
}

impl Def {
    #[inline]
    pub fn rad(&self) -> f32 {
        if self.building { self.size * 0.5 } else { self.size * 0.36 }
    }
    #[inline]
    pub fn has(&self, f: u16) -> bool {
        self.flags & f != 0
    }
    #[inline]
    pub fn armed(&self) -> bool {
        self.w[0].targets != 0
    }
    /// The weapon this unit would use against something in domain `dom`.
    #[inline]
    pub fn weapon_vs(&self, dom: u8) -> Option<&Wpn> {
        if self.w[0].targets & dom != 0 {
            Some(&self.w[0])
        } else if self.w[1].targets & dom != 0 {
            Some(&self.w[1])
        } else {
            None
        }
    }
    #[inline]
    pub fn max_range(&self) -> f32 {
        let a = if self.w[0].targets != 0 { self.w[0].range } else { 0.0 };
        let b = if self.w[1].targets != 0 { self.w[1].range } else { 0.0 };
        if a > b { a } else { b }
    }
    #[inline]
    pub fn all_targets(&self) -> u8 {
        self.w[0].targets | self.w[1].targets
    }
}

// Building ids (also their index in `DEFS`).
pub const TC: u8 = 0;
pub const HOUSE: u8 = 1;
pub const FARM: u8 = 2;
pub const BARRACKS: u8 = 3;
pub const WORKSHOP: u8 = 4;
pub const DOCK: u8 = 5;
pub const AIRFIELD: u8 = 6;
pub const TOWER: u8 = 7;
pub const WORKER: u8 = 8;
pub const BUILT_BY_WORKER: u8 = 255;

pub const EPOCH_NAMES: [&str; 6] = ["Stone Age", "Bronze Age", "Middle Ages", "Industrial Age", "Modern Age", "Atomic Age"];
/// Cost to advance *to* epoch index i (index 0 unused).
pub const EPOCH_COST: [[u16; 3]; 6] = [[0, 0, 0], [400, 300, 0], [800, 600, 300], [1200, 900, 700], [1800, 1200, 1300], [2500, 1800, 2200]];
pub const EPOCH_TIME: f32 = 40.0;
pub const MAX_EPOCH: u8 = 5;

const fn bld(name: &'static str, epoch: u8, size: f32, hp: f32, cost: [u16; 3], time: f32, dom: u8, flags: u16, wp: Wpn) -> Def {
    Def { name, epoch, until: 99, dom, building: true, size, hp, speed: 0.0, sight: 9.0, armor: A_BLD, cost, time, pop: 0, made_at: BUILT_BY_WORKER, flags, w: [wp, NOW] }
}

#[allow(clippy::too_many_arguments)]
const fn unit(name: &'static str, epoch: u8, until: u8, dom: u8, made_at: u8, cost: [u16; 3], time: f32, pop: u8, hp: f32, speed: f32, armor: u8, size: f32, flags: u16, w0: Wpn, w1: Wpn) -> Def {
    let sight = if dom == AIR { 11.0 } else { 8.0 };
    Def { name, epoch, until, dom, building: false, size, hp, speed, sight, armor, cost, time, pop, made_at, flags, w: [w0, w1] }
}

const G: u8 = LAND | SEA; // "surface" targets

pub const DEFS: &[Def] = &[
    // ---- buildings (ids 0..=7) ----
    bld("Town Center", 1, 4.0, 2400.0, [0, 400, 300], 60.0, LAND, F_DETECT, w(7.0, 10.0, 1.5, W_ARROW, G | AIR, 0.0)),
    bld("House", 0, 2.0, 500.0, [0, 60, 0], 14.0, LAND, 0, NOW),
    bld("Farm", 0, 3.0, 400.0, [0, 80, 0], 14.0, LAND, 0, NOW),
    bld("Barracks", 0, 3.0, 1200.0, [0, 150, 0], 24.0, LAND, 0, NOW),
    bld("Workshop", 1, 3.0, 1400.0, [0, 200, 100], 28.0, LAND, 0, NOW),
    bld("Dock", 1, 3.0, 1400.0, [0, 180, 0], 28.0, LAND, 0, NOW),
    bld("Airfield", 4, 4.0, 1800.0, [0, 300, 400], 36.0, LAND, 0, NOW),
    bld("Watchtower", 0, 2.0, 800.0, [0, 150, 100], 24.0, LAND, F_DETECT, w(7.5, 12.0, 1.2, W_ARROW, G | AIR, 0.0)),
    // ---- town center ----
    unit("Worker", 0, 99, LAND, TC, [50, 0, 0], 10.0, 1, 40.0, 2.2, A_INF, 0.8, F_WORKER, NOW, NOW),
    // ---- barracks ----
    unit("Clubman", 0, 2, LAND, BARRACKS, [40, 0, 0], 9.0, 1, 60.0, 2.2, A_INF, 0.85, 0, w(0.5, 6.0, 1.0, W_MELEE, LAND, 0.0), NOW),
    unit("Slinger", 0, 1, LAND, BARRACKS, [30, 20, 0], 9.0, 1, 40.0, 2.2, A_INF, 0.85, 0, w(4.5, 5.0, 1.4, W_ARROW, G, 0.0), NOW),
    unit("Spearman", 1, 2, LAND, BARRACKS, [50, 0, 10], 10.0, 1, 90.0, 2.2, A_INF, 0.85, 0, w(0.6, 9.0, 1.0, W_MELEE, LAND, 0.0), NOW),
    unit("Archer", 1, 2, LAND, BARRACKS, [35, 30, 0], 10.0, 1, 55.0, 2.2, A_INF, 0.85, 0, w(5.5, 8.0, 1.4, W_ARROW, G, 0.0), NOW),
    unit("Swordsman", 2, 4, LAND, BARRACKS, [60, 0, 30], 11.0, 1, 140.0, 2.3, A_INF, 0.85, 0, w(0.6, 14.0, 1.0, W_MELEE, LAND, 0.0), NOW),
    unit("Crossbowman", 2, 3, LAND, BARRACKS, [40, 40, 10], 11.0, 1, 80.0, 2.2, A_INF, 0.85, 0, w(6.0, 14.0, 1.6, W_ARROW, G, 0.0), NOW),
    unit("Musketeer", 3, 4, LAND, BARRACKS, [70, 0, 40], 12.0, 1, 150.0, 2.2, A_INF, 0.85, 0, w(6.0, 22.0, 1.8, W_BULLET, G, 0.0), NOW),
    unit("Rifleman", 4, 5, LAND, BARRACKS, [80, 0, 50], 12.0, 1, 200.0, 2.4, A_INF, 0.85, 0, w(6.5, 16.0, 0.7, W_BULLET, G, 0.0), NOW),
    unit("Marine", 5, 99, LAND, BARRACKS, [100, 0, 70], 13.0, 1, 280.0, 2.6, A_INF, 0.85, 0, w(7.0, 24.0, 0.6, W_BULLET, G, 0.0), w(7.5, 30.0, 2.5, W_MISSILE, AIR, 0.0)),
    // ---- workshop ----
    unit("Horseman", 1, 2, LAND, WORKSHOP, [70, 0, 20], 12.0, 1, 130.0, 3.6, A_CAV, 1.1, 0, w(0.6, 11.0, 1.1, W_MELEE, LAND, 0.0), NOW),
    unit("Knight", 2, 3, LAND, WORKSHOP, [90, 0, 60], 14.0, 2, 220.0, 3.4, A_CAV, 1.15, 0, w(0.6, 18.0, 1.1, W_MELEE, LAND, 0.0), NOW),
    unit("Trebuchet", 2, 3, LAND, WORKSHOP, [0, 150, 80], 18.0, 2, 120.0, 1.3, A_VEH, 1.4, 0, w(9.0, 60.0, 5.0, W_SHELL, G, 1.2), NOW),
    unit("Dragoon", 3, 4, LAND, WORKSHOP, [100, 0, 70], 14.0, 2, 260.0, 3.6, A_CAV, 1.15, 0, w(4.5, 20.0, 1.5, W_BULLET, G, 0.0), NOW),
    unit("Field Cannon", 3, 4, LAND, WORKSHOP, [0, 120, 150], 18.0, 2, 180.0, 1.5, A_VEH, 1.3, 0, w(10.0, 85.0, 4.5, W_SHELL, G, 1.3), NOW),
    unit("Tank", 4, 5, LAND, WORKSHOP, [100, 0, 250], 18.0, 2, 600.0, 3.2, A_VEH, 1.5, 0, w(7.0, 55.0, 1.8, W_SHELL, G, 0.6), NOW),
    unit("Howitzer", 4, 5, LAND, WORKSHOP, [0, 80, 350], 20.0, 2, 250.0, 1.8, A_VEH, 1.5, 0, w(13.0, 130.0, 5.0, W_SHELL, G, 1.6), NOW),
    unit("Flak Halftrack", 4, 5, LAND, WORKSHOP, [0, 0, 200], 14.0, 2, 300.0, 3.2, A_VEH, 1.4, 0, w(8.0, 14.0, 0.3, W_BULLET, AIR, 0.0), NOW),
    unit("Battle Tank", 5, 99, LAND, WORKSHOP, [150, 0, 400], 20.0, 3, 1000.0, 3.6, A_VEH, 1.6, 0, w(8.0, 90.0, 1.6, W_SHELL, G, 0.7), NOW),
    unit("Rocket Artillery", 5, 99, LAND, WORKSHOP, [0, 100, 450], 22.0, 3, 300.0, 2.4, A_VEH, 1.6, 0, w(15.0, 110.0, 3.0, W_MISSILE, G, 2.0), NOW),
    unit("SAM Launcher", 5, 99, LAND, WORKSHOP, [0, 0, 300], 16.0, 2, 300.0, 2.8, A_VEH, 1.5, 0, w(11.0, 120.0, 2.2, W_MISSILE, AIR, 0.0), NOW),
    // ---- dock ----
    unit("Galley", 1, 2, SEA, DOCK, [0, 120, 20], 16.0, 2, 250.0, 2.8, A_SHIP, 1.8, 0, w(5.5, 10.0, 1.5, W_ARROW, G, 0.0), NOW),
    unit("Carrack", 2, 3, SEA, DOCK, [0, 200, 60], 18.0, 2, 450.0, 2.8, A_SHIP, 2.0, 0, w(6.5, 22.0, 2.0, W_SHELL, G, 0.3), NOW),
    unit("Frigate", 3, 4, SEA, DOCK, [0, 250, 150], 20.0, 3, 800.0, 3.0, A_SHIP, 2.2, 0, w(7.5, 40.0, 2.0, W_SHELL, G, 0.4), NOW),
    unit("Destroyer", 4, 99, SEA, DOCK, [0, 100, 400], 20.0, 3, 1100.0, 3.6, A_SHIP, 2.3, F_DETECT, w(8.0, 45.0, 1.5, W_SHELL, G, 0.4), w(5.5, 70.0, 2.0, W_TORP, SUB, 0.0)),
    unit("Battleship", 4, 99, SEA, DOCK, [0, 200, 900], 30.0, 4, 2600.0, 2.4, A_SHIP, 3.2, 0, w(13.0, 160.0, 3.5, W_SHELL, G, 1.5), w(7.0, 10.0, 0.4, W_BULLET, AIR, 0.0)),
    unit("Submarine", 4, 5, SUB, DOCK, [0, 0, 500], 22.0, 3, 600.0, 2.6, A_SHIP, 2.2, F_STEALTH | F_DETECT, w(7.0, 180.0, 4.0, W_TORP, SEA | SUB, 0.0), NOW),
    unit("Missile Cruiser", 5, 99, SEA, DOCK, [0, 150, 1000], 28.0, 4, 1800.0, 3.4, A_SHIP, 2.8, F_DETECT, w(14.0, 140.0, 2.5, W_MISSILE, G, 1.2), w(11.0, 90.0, 1.5, W_MISSILE, AIR, 0.0)),
    unit("Nuclear Submarine", 5, 99, SUB, DOCK, [300, 0, 1500], 34.0, 4, 1400.0, 3.4, A_SHIP, 3.0, F_STEALTH | F_DETECT, w(9.0, 320.0, 3.5, W_TORP, SEA | SUB, 0.0), w(22.0, 400.0, 9.0, W_MISSILE, LAND, 2.5)),
    // ---- airfield ----
    unit("Fighter", 4, 5, AIR, AIRFIELD, [50, 0, 300], 16.0, 2, 220.0, 7.0, A_AIR, 1.5, F_PLANE, w(5.0, 14.0, 0.25, W_BULLET, AIR | G, 0.0), NOW),
    unit("Bomber", 4, 5, AIR, AIRFIELD, [100, 0, 500], 22.0, 3, 400.0, 5.0, A_AIR, 2.0, F_PLANE, w(1.6, 220.0, 4.0, W_BOMB, G, 1.8), NOW),
    unit("Jet Fighter", 5, 99, AIR, AIRFIELD, [100, 0, 500], 18.0, 2, 380.0, 11.0, A_AIR, 1.6, F_PLANE, w(8.0, 70.0, 1.2, W_MISSILE, AIR, 0.0), w(5.0, 16.0, 0.2, W_BULLET, G, 0.0)),
    unit("Stealth Bomber", 5, 99, AIR, AIRFIELD, [200, 0, 900], 26.0, 3, 600.0, 7.5, A_AIR, 2.3, F_PLANE | F_STEALTH, w(1.6, 500.0, 4.5, W_BOMB, G, 2.4), NOW),
    unit("Gunship", 5, 99, AIR, AIRFIELD, [100, 0, 450], 20.0, 2, 450.0, 5.0, A_AIR, 1.6, F_DETECT, w(7.0, 60.0, 1.6, W_MISSILE, G, 0.5), w(4.5, 90.0, 2.5, W_TORP, SUB, 0.0)),
];
