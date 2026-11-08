//! Original NavalMove depth transition for the three paid Submarine stages.
use crate::defs::DefId;
use crate::game::{Ent, Game, O_MOVE};
use crate::naval_heave72::{store, surface_heave};

pub(crate) const SAVE_FLAG: u32 = 1 << 20;
const CRUISE_DEPTH: f64 = -1.0;
const BOTTOM_CLEARANCE: f64 = 0.5;
const VERTICAL_SPEED: f64 = 0.5;

pub(crate) fn is_submarine(definition: DefId) -> bool {
    matches!(definition, 34 | 36 | 45)
}

fn base_depth(actor: &Ent) -> f32 {
    f32::from_bits(actor.q[3])
}

pub(crate) fn valid_depth(actor: &Ent) -> bool {
    // The portable random terrain's upper bound is below 4.0. Ground clearance
    // is positive, so a shallow Move goal can temporarily raise the hull.
    (-1.0..=4.5).contains(&base_depth(actor))
}

fn advance(depth: f32, ground: f32, elapsed: f32, move_goal: bool) -> f32 {
    let depth = f64::from(depth);
    let target = if move_goal { (f64::from(ground) + BOTTOM_CLEARANCE).max(CRUISE_DEPTH) } else { 0.0 };
    let change = f64::from(elapsed) * VERTICAL_SPEED;
    // 007F3C14..007F3C64 compares the x87 value before the final f32 store,
    // clamping both the dive and the rise to the selected target.
    let next = if depth > target { (depth - change).max(target) } else { (depth + change).min(target) };
    store(next)
}

impl Game {
    pub(crate) fn submarine_depth_allowed(&self) -> bool {
        if self.net || self.humans != 1 || self.bench != 0 || self.powers.is_none() { return false; }
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() { return false; }
        true
    }

    pub(crate) fn configure_submarine_depth(&mut self) -> bool {
        if self.tick != 0 || self.submarine_depth_enabled || !self.submarine_depth_allowed() { return false; }
        self.submarine_depth_enabled = true;
        true
    }

    pub(crate) fn update_submarine_depth(&self, actor: &mut Ent, elapsed: f32) {
        if !self.submarine_depth_enabled || !is_submarine(actor.def) { return; }
        let move_goal = actor.order == O_MOVE;
        let ground = if move_goal { self.map.height(actor.x, actor.y) } else { 0.0 };
        actor.q[3] = advance(base_depth(actor), ground, elapsed, move_goal).to_bits();
    }

    /// Return the committed body altitude and the separate tilt delta.
    pub(crate) fn naval_body_pose(&self, index: usize) -> Option<(u32, f32, f32)> {
        let actor = &self.ents[index];
        if self.submarine_depth_enabled && is_submarine(actor.def) && actor.hp > 0.0 {
            let id = self.naval_actor_id(index)?;
            let base = f64::from(base_depth(actor));
            let wave = f64::from(surface_heave(id, self.sim_elapsed()));
            let altitude = store(base + wave); // 007F3C99
            let tilt_delta = store(f64::from(altitude) - base); // 007F39FB
            return Some((id, altitude, tilt_delta));
        }
        self.naval_surface_pose(index).map(|(id, altitude)| (id, altitude, altitude))
    }
}
