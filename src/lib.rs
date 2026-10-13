//! OpenEpoch — an original epoch-spanning real-time strategy game.
//!
//! This crate is the whole simulation, compiled to WebAssembly with zero
//! dependencies. The page (see `docs/`) owns input, WebGL2 drawing and the HUD
//! and talks to it through the plain C ABI below.

mod ai;
mod defs;
mod game;
mod map;
mod render;

use core::ptr::addr_of_mut;
use defs::*;
use game::*;
use map::*;

static mut GAME: Option<Box<Game>> = None;

fn g() -> &'static mut Game {
    // SAFETY: wasm32-unknown-unknown is single threaded and `init` runs first.
    unsafe { (*addr_of_mut!(GAME)).as_mut().expect("init not called") }
}

#[no_mangle]
pub extern "C" fn init(seed: u32, n_ai: u32, bench: u32) {
    unsafe {
        *addr_of_mut!(GAME) = Some(Box::new(Game::new(seed, n_ai, bench)));
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
pub extern "C" fn fog_ptr() -> *const u8 {
    g().vis.as_ptr()
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
    g().map.starts[0].0 as f32
}
#[no_mangle]
pub extern "C" fn start_y() -> f32 {
    g().map.starts[0].1 as f32
}

// ---- player input ---------------------------------------------------------

fn selected(g: &Game) -> Vec<usize> {
    (0..g.high).filter(|&i| g.ents[i].alive && g.ents[i].sel && g.ents[i].team == 0).collect()
}

fn clear_sel(g: &mut Game) {
    for e in g.ents[..g.high].iter_mut() {
        e.sel = false;
    }
}

/// Entity under the cursor that the player can see. `own`: 1 = only own, 0 = any.
fn pick(g: &Game, x: f32, y: f32, own: bool) -> usize {
    let mut best = NOIDX;
    let mut bd = f32::MAX;
    for i in 0..g.high {
        let e = &g.ents[i];
        if !e.alive || (own && e.team != 0) || !g.visible(i) {
            continue;
        }
        let d = &DEFS[e.def as usize];
        let (dx, dy) = ((e.x - x).abs(), (e.y - y).abs());
        let hit = if d.building { dx.max(dy) <= d.size * 0.5 } else { dx * dx + dy * dy <= (d.rad() + 0.35).powi(2) };
        // units win over the building they stand next to; aircraft win over everything
        let score = dx * dx + dy * dy + if d.building { 100.0 } else if d.dom == AIR { -50.0 } else { 0.0 };
        if hit && score < bd {
            bd = score;
            best = i;
        }
    }
    best
}

#[no_mangle]
pub extern "C" fn select_rect(x0: f32, y0: f32, x1: f32, y1: f32, add: u32) {
    let g = g();
    if add == 0 {
        clear_sel(g);
    }
    let mut any = false;
    for pass in 0..2 {
        // prefer military units; only grab workers when the box holds nothing else
        for e in g.ents[..g.high].iter_mut() {
            let d = &DEFS[e.def as usize];
            if e.alive && e.team == 0 && !d.building && e.x >= x0 && e.x <= x1 && e.y >= y0 && e.y <= y1 && (pass == 1 || !d.has(F_WORKER)) {
                e.sel = true;
                any = true;
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
        let def = g.ents[i].def;
        for e in g.ents[..g.high].iter_mut() {
            if e.alive && e.team == 0 && e.def == def && e.x >= vx0 && e.x <= vx1 && e.y >= vy0 && e.y <= vy1 {
                e.sel = true;
            }
        }
    } else {
        g.ents[i].sel = !(add != 0 && g.ents[i].sel);
    }
}

#[no_mangle]
pub extern "C" fn select_idle_worker() -> u32 {
    let g = g();
    for i in 0..g.high {
        let e = g.ents[i];
        if e.alive && e.team == 0 && e.def == WORKER && e.order == O_IDLE && !e.sel {
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

/// Right click: attack / gather / build / move / set rally, depending on what is under the cursor.
#[no_mangle]
pub extern "C" fn cmd_context(x: f32, y: f32, attack_move: u32) {
    let g = g();
    let ids = selected(g);
    if ids.is_empty() {
        return;
    }
    if DEFS[g.ents[ids[0]].def as usize].building {
        for &i in &ids {
            g.ents[i].rx = x;
            g.ents[i].ry = y;
        }
        return;
    }
    if attack_move != 0 {
        g.order_move(&ids, x, y, true);
        return;
    }
    let workers: Vec<usize> = ids.iter().copied().filter(|&i| g.ents[i].def == WORKER).collect();
    let others: Vec<usize> = ids.iter().copied().filter(|&i| g.ents[i].def != WORKER).collect();
    let j = pick(g, x, y, false);
    if j != NOIDX {
        let t = g.ents[j];
        if t.team != 0 {
            g.order_attack(&ids, j);
            return;
        }
        if DEFS[t.def as usize].building && (t.built < 1.0 || t.def == FARM) && !workers.is_empty() {
            g.order_building(&workers, j);
            g.order_move(&others, x, y, false);
            return;
        }
    }
    let tile = tile_of(x, y);
    if g.map.res[tile] != R_NONE && g.vis[tile] > 0 && !workers.is_empty() {
        g.order_gather(&workers, tile);
        g.order_move(&others, x, y, false);
        return;
    }
    g.order_move(&ids, x, y, false);
}

#[no_mangle]
pub extern "C" fn cmd_stop() {
    let g = g();
    let ids = selected(g);
    g.order_stop(&ids);
}

/// Place a building (top-left tile) and send the selected workers to build it.
#[no_mangle]
pub extern "C" fn cmd_place(def: u32, tx: i32, ty: i32) -> u32 {
    let g = g();
    let b = g.place(def as u8, tx, ty, 0);
    if b == NOIDX {
        return 0;
    }
    let ids = selected(g);
    g.order_building(&ids, b);
    1
}

#[no_mangle]
pub extern "C" fn cmd_train(def: u32) -> u32 {
    let g = g();
    let def = def as u8;
    let want = if def == RESEARCH { TC } else { DEFS[def as usize % DEFS.len()].made_at };
    // spread the order over the selected buildings: shortest queue first
    let best = selected(g).into_iter().filter(|&i| g.ents[i].def == want && g.ents[i].built >= 1.0).min_by_key(|&i| g.ents[i].qn);
    match best {
        Some(b) => g.train(b, def) as u32,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn cmd_cancel() {
    let g = g();
    if let Some(b) = selected(g).into_iter().filter(|&i| g.ents[i].qn > 0).max_by_key(|&i| g.ents[i].qn) {
        g.cancel(b);
    }
}

#[no_mangle]
pub extern "C" fn cmd_delete() {
    let g = g();
    for i in selected(g) {
        g.ents[i].hp = 0.0;
    }
}

#[no_mangle]
pub extern "C" fn group_set(n: u32) {
    let g = g();
    let hs: Vec<u32> = selected(g).into_iter().map(|i| g.h(i)).collect();
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

/// Centre of the current selection (for "jump to group"); x in the low half, packed as two calls.
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

/// Cheat / sandbox helper used by the debug key: grant resources.
#[no_mangle]
pub extern "C" fn grant(amount: f32) {
    for r in g().players[0].res.iter_mut() {
        *r += amount;
    }
}

/// Read-only peek at any player, for tests and tooling. what: 0 epoch, 1 pop, 2 cap, 3..=5 resources, 6 buildings.
#[no_mangle]
pub extern "C" fn stat(team: u32, what: u32) -> f32 {
    let p = &g().players[team as usize & 3];
    match what {
        0 => p.epoch as f32,
        1 => p.pop as f32,
        2 => p.cap as f32,
        3..=5 => p.res[what as usize - 3],
        _ => p.buildings as f32,
    }
}
