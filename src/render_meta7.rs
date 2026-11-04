//! Read-only rendering sidecar. These values are application render metadata,
//! not native visual lifecycle messages or an additional simulation/save state.
//! Kept outside Game/Ent so the established simulation layouts stay unchanged.
use crate::game::Ent;
use core::ptr::addr_of_mut;

pub const STRIDE: u32 = 64;
pub const NO_ACTOR: u32 = u32::MAX;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub handle: u32,
    pub def: u32,
    pub team: u32,
    pub order: u32,
    pub hp_ratio: f32,
    pub built: f32,
    pub render_x: f32,
    pub render_y: f32,
    pub render_altitude: f32,
    pub sim_x: f32,
    pub sim_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub render_yaw: f32,
    pub render_time: f32,
    pub render_flags: u32,
}
const _: [(); 64] = [(); core::mem::size_of::<Record>()];
const EMPTY: Record = Record {
    handle: NO_ACTOR, def: NO_ACTOR, team: NO_ACTOR, order: NO_ACTOR,
    hp_ratio: 0.0, built: 0.0, render_x: 0.0, render_y: 0.0,
    render_altitude: 0.0, sim_x: 0.0, sim_y: 0.0, velocity_x: 0.0,
    velocity_y: 0.0, render_yaw: 0.0, render_time: 0.0, render_flags: 0,
};

struct State {
    session: u32,
    frame: u32,
    unsorted: Vec<Record>,
    sorted: Vec<Record>,
}
static mut STATE: State = State {session: 0, frame: 0, unsorted: Vec::new(), sorted: Vec::new()};

// Like the existing GAME ABI, this sidecar is single-threaded. Native test
// invocations use --test-threads=1; a JS caller reacquires views after growth.
fn state() -> &'static mut State { unsafe { &mut *addr_of_mut!(STATE) } }

/// Only public init and a successful transactional public load call this hook.
/// Failed loads and visual-only frames must never change the session token.
pub fn reset_session() {
    #[cfg(feature="ee2-pack8")]crate::pack8_render::clear();
    let s = state();
    s.session = s.session.wrapping_add(1);
    if s.session == 0 { s.session = 1; }
    s.frame = 0;
    s.unsorted.clear();
    s.sorted.clear();
}

pub fn begin_frame() {
    #[cfg(feature="ee2-pack8")]crate::pack8_render::clear();
    let s = state();
    s.unsorted.clear();
    s.sorted.clear();
}

/// Called immediately after a successful *entity main mesh* put, not inferred
/// afterwards by matching model IDs, seeds, positions, teams or render slots.
pub fn actor(index: usize, handle: u32, e: &Ent, max_hp: f32) {
    let s = state();
    if s.unsorted.len() <= index { s.unsorted.resize(index + 1, EMPTY); }
    s.unsorted[index] = Record {
        handle, def: e.def as u32, team: e.team as u32, order: e.order as u32,
        hp_ratio: (e.hp / max_hp).clamp(0.0, 1.0), built: e.built,
        sim_x: e.x, sim_y: e.y, velocity_x: e.vx, velocity_y: e.vy, ..EMPTY
    };
}

pub fn prepare_sort(count: usize) {
    #[cfg(feature="ee2-pack8")]crate::pack8_render::prepare(count);
    let s = state();
    s.unsorted.resize(count, EMPTY);
    s.sorted.resize(count, EMPTY);
}

/// Uses exactly the same source and destination indices as the existing stable
/// counting sort. All nonactor rows retain NO_ACTOR and sentinel actor fields.
pub fn sorted_row(source: usize, destination: usize, instance: &[f32]) {
    #[cfg(feature="ee2-pack8")]crate::pack8_render::sorted_row(source,destination);
    let s = state();
    let mut record = s.unsorted[source];
    record.render_x = instance[0];
    record.render_y = instance[1];
    record.render_altitude = instance[2];
    record.render_yaw = instance[3];
    record.render_time = instance[10];
    record.render_flags = instance[7] as u32;
    s.sorted[destination] = record;
}

pub fn end_frame() { let s = state(); s.frame = s.frame.wrapping_add(1); }
pub fn ptr() -> *const Record {
    let s = state();
    if s.sorted.is_empty() { core::ptr::null() } else { s.sorted.as_ptr() }
}
pub fn count() -> u32 { state().sorted.len() as u32 }
pub fn session() -> u32 { state().session }
pub fn frame() -> u32 { state().frame }

#[cfg(test)]
#[path="render_meta7_tests.rs"]
mod tests;
