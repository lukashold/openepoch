//! OpenEpoch — an original epoch-spanning real-time strategy game.
//!
//! This crate is the whole simulation, compiled to WebAssembly with zero
//! dependencies. The page (see `docs/`) owns input, WebGL2 drawing, sound and
//! networking and talks to it through the plain C ABI below.

mod ai;
mod cmd;
mod defs;
mod game;
mod map;
mod render;
mod save;

use cmd::*;
use core::ptr::addr_of_mut;
use defs::*;
use game::*;
use map::*;

static mut GAME: Option<Box<Game>> = None;
static mut LOADBUF: Vec<u8> = Vec::new();

fn g() -> &'static mut Game {
    // SAFETY: wasm32-unknown-unknown is single threaded and `init` runs first.
    unsafe { (*addr_of_mut!(GAME)).as_mut().expect("init not called") }
}

/// `humans` = 2 starts a lockstep multiplayer game; `local` is the seat this client plays.
#[no_mangle]
pub extern "C" fn init(seed: u32, n_ai: u32, bench: u32, humans: u32, local: u32) {
    unsafe {
        *addr_of_mut!(GAME) = Some(Box::new(Game::new(seed, n_ai, bench, humans, local)));
    }
}

#[no_mangle]
pub extern "C" fn tick() {
    g().tick();
}

// ---- frame data -----------------------------------------------------------

#[no_mangle]
pub extern "C" fn render(alpha: f32, x0: f32, y0: f32, x1: f32, y1: f32, ghost: i32, gx: i32, gy: i32) -> u32 {
    g().render(alpha, x0, y0, x1, y1, ghost, gx, gy)
}
#[no_mangle]
pub extern "C" fn inst_ptr() -> *const f32 {
    g().inst.as_ptr()
}
#[no_mangle]
pub extern "C" fn ranges_ptr() -> *const u32 {
    g().ranges.as_ptr()
}
#[no_mangle]
pub extern "C" fn map_ptr() -> *const u8 {
    g().map.rgba.as_ptr()
}
/// Returns 1 once after the terrain texture changed.
#[no_mangle]
pub extern "C" fn map_dirty() -> u32 {
    let d = g().map.dirty;
    g().map.dirty = false;
    d as u32
}
#[no_mangle]
pub extern "C" fn hmap_ptr() -> *const f32 {
    g().map.hm.as_ptr()
}
#[no_mangle]
pub extern "C" fn fog_ptr() -> *const u8 {
    g().vis.as_ptr()
}
#[no_mangle]
pub extern "C" fn terr_dirty() -> u32 {
    g().terr_dirty as u32
}
#[no_mangle]
pub extern "C" fn terr_ptr() -> *const u8 {
    g().terr_tex();
    g().terr_rgba.as_ptr()
}
#[no_mangle]
pub extern "C" fn minimap() -> *const u8 {
    g().minimap();
    g().mini.as_ptr()
}
#[no_mangle]
pub extern "C" fn ui() -> *const u8 {
    g().build_ui();
    g().ui.as_ptr()
}
#[no_mangle]
pub extern "C" fn ui_len() -> u32 {
    g().ui.len() as u32
}
#[no_mangle]
pub extern "C" fn start_x() -> f32 {
    g().map.starts[g().local as usize].0 as f32
}
#[no_mangle]
pub extern "C" fn start_y() -> f32 {
    g().map.starts[g().local as usize].1 as f32
}
#[no_mangle]
pub extern "C" fn events_ptr() -> *const f32 {
    g().events.as_ptr()
}
#[no_mangle]
pub extern "C" fn events_len() -> u32 {
    g().events.len() as u32
}
#[no_mangle]
pub extern "C" fn events_clear() {
    g().events.clear();
}

// ---- save games -----------------------------------------------------------

#[no_mangle]
pub extern "C" fn save() -> *const u8 {
    g().save();
    g().savebuf.as_ptr()
}
#[no_mangle]
pub extern "C" fn save_len() -> u32 {
    g().savebuf.len() as u32
}
/// Reserve `len` bytes for a save file; the page copies the file there, then calls `load_apply`.
#[no_mangle]
pub extern "C" fn load_alloc(len: u32) -> *mut u8 {
    unsafe {
        let b = &mut *addr_of_mut!(LOADBUF);
        b.clear();
        b.resize(len as usize, 0);
        b.as_mut_ptr()
    }
}
#[no_mangle]
pub extern "C" fn load_apply() -> u32 {
    unsafe {
        match Game::load(&*addr_of_mut!(LOADBUF)) {
            Some(game) => {
                *addr_of_mut!(GAME) = Some(Box::new(game));
                1
            }
            None => 0,
        }
    }
}

// ---- lockstep multiplayer -------------------------------------------------

#[no_mangle]
pub extern "C" fn outbox_ptr() -> *const u8 {
    g().outbox.as_ptr()
}
#[no_mangle]
pub extern "C" fn outbox_len() -> u32 {
    g().outbox.len() as u32
}
#[no_mangle]
pub extern "C" fn outbox_clear() {
    g().outbox.clear();
}
#[no_mangle]
pub extern "C" fn inbox_alloc(len: u32) -> *mut u8 {
    let b = &mut g().inbox;
    b.clear();
    b.resize(len as usize, 0);
    b.as_mut_ptr()
}
/// Apply the commands in the inbox for player `team`.
#[no_mangle]
pub extern "C" fn net_apply(team: u32) {
    g().net_apply(team as u8);
}
#[no_mangle]
pub extern "C" fn state_hash() -> u32 {
    g().state_hash()
}

// ---- selection (purely local, never part of the simulation) ---------------

fn selected(g: &Game) -> Vec<usize> {
    (0..g.high).filter(|&i| g.ents[i].alive && g.ents[i].sel && g.ents[i].team == g.local).collect()
}
fn handles(g: &Game, ids: &[usize]) -> Vec<u32> {
    ids.iter().map(|&i| g.h(i)).collect()
}

fn clear_sel(g: &mut Game) {
    for e in g.ents[..g.high].iter_mut() {
        e.sel = false;
    }
}

/// Tell the simulation where the camera is, so clicks hit what the player sees.
#[no_mangle]
pub extern "C" fn set_eye(x: f32, height: f32, y: f32) {
    g().eye = [x, height, y];
}

/// Where a point at altitude `a` appears on the ground plane when seen from the camera.
/// A plane 3.4 tiles up is drawn well "behind" its map position under a tilted camera.
fn seen_at(g: &Game, x: f32, y: f32, a: f32) -> (f32, f32) {
    let e = g.eye;
    if e[1] <= a + 1.0 {
        return (x, y);
    }
    let k = e[1] / (e[1] - a);
    (e[0] + (x - e[0]) * k, e[2] + (y - e[2]) * k)
}

/// Apparent ground-plane position of an entity's body.
fn seen_ent(g: &Game, e: &Ent, d: &Def) -> (f32, f32) {
    let ground = g.map.height(e.x, e.y).max(0.0);
    let a = if d.building {
        if e.def == DOCK { 0.1 } else { ground + 0.2 * d.size }
    } else {
        match d.dom {
            SEA => 0.1,
            SUB => 0.0,
            AIR => 3.4 + ground * 0.6,
            _ => ground + 0.3 * d.size,
        }
    };
    seen_at(g, e.x, e.y, a)
}

/// Entity under the cursor that the player can see. `(x, y)` is a point on the terrain.
fn pick(g: &Game, x: f32, y: f32, own: bool) -> usize {
    let (x, y) = seen_at(g, x, y, g.map.height(x, y).max(0.0));
    let mut best = NOIDX;
    let mut bd = f32::MAX;
    for i in 0..g.high {
        let e = &g.ents[i];
        if !e.alive || (own && e.team != g.local) || !g.visible(i) {
            continue;
        }
        let d = &DEFS[e.def as usize];
        let (ex, ey) = seen_ent(g, e, d);
        let (dx, dy) = ((ex - x).abs(), (ey - y).abs());
        let hit = if d.building { dx.max(dy) <= d.size * 0.5 + 0.2 } else { dx * dx + dy * dy <= (d.rad() + 0.45).powi(2) };
        // units win over the building they stand next to; aircraft win over everything
        let score = dx * dx + dy * dy + if d.building { 100.0 } else if d.dom == AIR { -50.0 } else { 0.0 };
        if hit && score < bd {
            bd = score;
            best = i;
        }
    }
    best
}

/// Box selection. The box is a screen rectangle, which the tilted camera turns
/// into a quadrilateral on the ground: corners are passed in order around it.
#[no_mangle]
pub extern "C" fn select_quad(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, dx: f32, dy: f32, add: u32) {
    let g = g();
    if add == 0 {
        clear_sel(g);
    }
    let flat = |x: f32, y: f32| seen_at(g, x, y, g.map.height(x, y).max(0.0));
    let q = [flat(ax, ay), flat(bx, by), flat(cx, cy), flat(dx, dy)];
    let inside = |x: f32, y: f32| {
        let mut sign = 0.0f32;
        for k in 0..4 {
            let (p, n) = (q[k], q[(k + 1) % 4]);
            let c = (n.0 - p.0) * (y - p.1) - (n.1 - p.1) * (x - p.0);
            if c * sign < 0.0 {
                return false;
            }
            if c != 0.0 {
                sign = c;
            }
        }
        true
    };
    let me = g.local;
    let mut any = false;
    for pass in 0..2 {
        // prefer military units; only grab workers when the box holds nothing else
        for i in 0..g.high {
            let e = &g.ents[i];
            let d = &DEFS[e.def as usize];
            if e.alive && e.team == me && !d.building && (pass == 1 || !d.has(F_WORKER)) {
                let (sx, sy) = seen_ent(g, e, d);
                if inside(sx, sy) {
                    g.ents[i].sel = true;
                    any = true;
                }
            }
        }
        if any {
            break;
        }
    }
}

/// Click selection. `same` = also select every unit of that type inside the view rect.
#[no_mangle]
pub extern "C" fn select_point(x: f32, y: f32, add: u32, same: u32, vx0: f32, vy0: f32, vx1: f32, vy1: f32) {
    let g = g();
    let i = pick(g, x, y, true);
    if add == 0 {
        clear_sel(g);
    }
    if i == NOIDX {
        return;
    }
    if same != 0 {
        let (def, me) = (g.ents[i].def, g.local);
        for e in g.ents[..g.high].iter_mut() {
            if e.alive && e.team == me && e.def == def && e.x >= vx0 && e.x <= vx1 && e.y >= vy0 && e.y <= vy1 {
                e.sel = true;
            }
        }
    } else {
        g.ents[i].sel = !(add != 0 && g.ents[i].sel);
    }
}

#[no_mangle]
pub extern "C" fn select_none() {
    clear_sel(g());
}

#[no_mangle]
pub extern "C" fn select_idle_worker() -> u32 {
    let g = g();
    for i in 0..g.high {
        let e = g.ents[i];
        if e.alive && e.team == g.local && e.def == WORKER && e.order == O_IDLE && !e.sel {
            clear_sel(g);
            g.ents[i].sel = true;
            return i as u32;
        }
    }
    u32::MAX
}

#[no_mangle]
pub extern "C" fn ent_x(i: u32) -> f32 {
    g().ents[i as usize & (MAXE - 1)].x
}
#[no_mangle]
pub extern "C" fn ent_y(i: u32) -> f32 {
    g().ents[i as usize & (MAXE - 1)].y
}

#[no_mangle]
pub extern "C" fn group_set(n: u32) {
    let g = g();
    let hs = handles(g, &selected(g));
    g.groups[n as usize % 10] = hs;
}

/// Recall a control group; returns how many units are still alive in it.
#[no_mangle]
pub extern "C" fn group_get(n: u32) -> u32 {
    let g = g();
    let hs = g.groups[n as usize % 10].clone();
    if hs.is_empty() {
        return 0;
    }
    clear_sel(g);
    let mut c = 0;
    for h in hs {
        if let Some(i) = g.get(h) {
            g.ents[i].sel = true;
            c += 1;
        }
    }
    c
}

/// Centre of the current selection on one axis, or -1 with nothing selected.
#[no_mangle]
pub extern "C" fn sel_center(axis: u32) -> f32 {
    let g = g();
    let ids = selected(g);
    if ids.is_empty() {
        return -1.0;
    }
    let s: f32 = ids.iter().map(|&i| if axis == 0 { g.ents[i].x } else { g.ents[i].y }).sum();
    s / ids.len() as f32
}

// ---- commands: resolved here against what the player sees, then submitted --

/// Right click: attack / gather / build / board / unload / move / set rally.
#[no_mangle]
pub extern "C" fn cmd_context(x: f32, y: f32, attack_move: u32) {
    let g = g();
    let ids = selected(g);
    if ids.is_empty() {
        return;
    }
    let all = handles(g, &ids);
    if DEFS[g.ents[ids[0]].def as usize].building {
        g.submit(C_RALLY, x, y, 0, &all);
        return;
    }
    if attack_move != 0 {
        g.submit(C_AMOVE, x, y, 0, &all);
        return;
    }
    let part = |g: &Game, f: &dyn Fn(&Ent, &Def) -> bool| -> Vec<u32> { ids.iter().copied().filter(|&i| f(&g.ents[i], &DEFS[g.ents[i].def as usize])).map(|i| g.h(i)).collect() };
    let workers = part(g, &|_, d| d.has(F_WORKER));
    let others = part(g, &|_, d| !d.has(F_WORKER));
    let j = pick(g, x, y, false);
    if j != NOIDX {
        let t = g.ents[j];
        let td = &DEFS[t.def as usize];
        let th = g.h(j) as i32;
        if t.team != g.local {
            g.submit(C_ATTACK, x, y, th, &all);
            return;
        }
        if td.has(F_TRANSPORT) {
            let land = part(g, &|_, d| d.dom == LAND);
            if !land.is_empty() {
                g.submit(C_BOARD, x, y, th, &land);
                return;
            }
        }
        if td.building && (t.built < 1.0 || t.def == FARM) && !workers.is_empty() {
            g.submit(C_WORK, x, y, th, &workers);
            if !others.is_empty() {
                g.submit(C_MOVE, x, y, 0, &others);
            }
            return;
        }
    }
    let tile = tile_of(x, y);
    if g.map.res[tile] != R_NONE && g.vis[tile] > 0 && !workers.is_empty() {
        g.submit(C_GATHER, x, y, tile as i32, &workers);
        if !others.is_empty() {
            g.submit(C_MOVE, x, y, 0, &others);
        }
        return;
    }
    // loaded transports sent onto land put their passengers ashore there
    let loaded = part(g, &|e, d| d.has(F_TRANSPORT) && e.ncargo > 0);
    if !loaded.is_empty() && g.map.ter[tile] >= T_SAND {
        let rest = part(g, &|e, d| !(d.has(F_TRANSPORT) && e.ncargo > 0));
        g.submit(C_UNLOAD, x, y, 0, &loaded);
        if !rest.is_empty() {
            g.submit(C_MOVE, x, y, 0, &rest);
        }
        return;
    }
    g.submit(C_MOVE, x, y, 0, &all);
}

#[no_mangle]
pub extern "C" fn cmd_stop() {
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_STOP, 0.0, 0.0, 0, &hs);
}

/// Unload selected transports where they are.
#[no_mangle]
pub extern "C" fn cmd_unload() {
    let g = g();
    for i in selected(g) {
        if g.ents[i].ncargo > 0 {
            let (x, y, h) = (g.ents[i].x, g.ents[i].y, g.h(i));
            g.submit(C_UNLOAD, x, y, 0, &[h]);
        }
    }
}

/// Why a building cannot go here: 0 ok, 1 terrain/blocked, 2 not our territory, 3 unexplored, 4 cost.
#[no_mangle]
pub extern "C" fn place_error(def: u32, tx: i32, ty: i32) -> u32 {
    let g = g();
    let def = def as u8;
    if def as usize >= DEFS.len() {
        return 1;
    }
    let s = DEFS[def as usize].size as i32;
    if !g.afford(g.local, &DEFS[def as usize].cost) {
        4
    } else if !g.explored(def, tx, ty) {
        3
    } else if g.can_place(def, tx, ty, g.local) {
        0
    } else {
        let owner = g.towner[g.map.terr[tile_of((tx + s / 2) as f32, (ty + s / 2) as f32)] as usize];
        if owner != g.local && !(def == TC && owner == 255) { 2 } else { 1 }
    }
}

/// Place a building (top-left tile) and send the selected workers to build it.
#[no_mangle]
pub extern "C" fn cmd_place(def: u32, tx: i32, ty: i32) -> u32 {
    if place_error(def, tx, ty) != 0 {
        return 0;
    }
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_PLACE, tx as f32, ty as f32, def as i32, &hs);
    1
}

#[no_mangle]
pub extern "C" fn cmd_train(def: u32) -> u32 {
    let g = g();
    let def = def as u8;
    // spread the order over the selected buildings: shortest queue first
    let best = selected(g).into_iter().filter(|&i| g.can_train(i, def)).min_by_key(|&i| g.ents[i].qn);
    match best {
        Some(b) => {
            let h = g.h(b);
            g.submit(C_TRAIN, 0.0, 0.0, def as i32, &[h]);
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn cmd_cancel() {
    let g = g();
    if let Some(b) = selected(g).into_iter().filter(|&i| g.ents[i].qn > 0).max_by_key(|&i| g.ents[i].qn) {
        let h = g.h(b);
        g.submit(C_CANCEL, 0.0, 0.0, 0, &[h]);
    }
}

#[no_mangle]
pub extern "C" fn cmd_delete() {
    let g = g();
    let hs = handles(g, &selected(g));
    g.submit(C_DELETE, 0.0, 0.0, 0, &hs);
}

/// Sandbox cheat for single player: grant resources.
#[no_mangle]
pub extern "C" fn grant(amount: f32) {
    let g = g();
    if !g.net {
        for r in g.players[g.local as usize].res.iter_mut() {
            *r += amount;
        }
    }
}

/// Read-only peek at any player, for tests and tooling.
/// what: 0 epoch, 1 pop, 2 cap, 3..=5 resources, 6 buildings, 7 territories, 8 technologies.
#[no_mangle]
pub extern "C" fn stat(team: u32, what: u32) -> f32 {
    let p = &g().players[team as usize & 3];
    match what {
        0 => p.epoch as f32,
        1 => p.pop as f32,
        2 => p.cap as f32,
        3..=5 => p.res[what as usize - 3],
        6 => p.buildings as f32,
        7 => p.terr as f32,
        _ => p.tech.count_ones() as f32,
    }
}
