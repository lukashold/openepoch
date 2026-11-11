//! First ordinary Dock from the default skirmish boat row. The portable map
//! supplies shoreline plots in place of the original CoastManager segment list.
use crate::{
    defs::{DEFS, DOCK},
    game::{Game, NOIDX, NONE},
    map::{N, T_DEEP},
};

const PORTABLE_RADIUS: i32 = 47;
const COAST_LIMIT: usize = 100;

impl Game {
    pub(crate) fn first_dock_policy(&self) -> bool {
        if self.net || self.bench != 0 || self.humans != 1 || self.winner != 0
            || self.territory_victory.is_some() || self.sole_survivor.is_some() || self.regicide.is_some()
            || self.koth.is_some() || self.crown_victory.is_some() || self.capitols.is_some() || self.hotspots.is_some() {
            return false;
        }
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return false; }
        self.map.world_dimensions() == [N, N]
    }

    pub(super) fn ai_post_first_dock(&mut self, owner: u8, origin: [i32; 2], builders: &[usize]) -> bool {
        if builders.is_empty() || self.crown_holder(1).is_some_and(|holder| holder.owner == owner)
            || !self.afford(owner, &self.construction_cost(owner, DOCK)) { return false; }
        let [home_x, home_y] = origin;
        let size = Self::footprint_side(DOCK, 0);
        let min_x = 2.max(home_x - PORTABLE_RADIUS);
        let min_y = 2.max(home_y - PORTABLE_RADIUS);
        let max_x = (N as i32 - size - 2).min(home_x + PORTABLE_RADIUS);
        let max_y = (N as i32 - size - 2).min(home_y + PORTABLE_RADIUS);
        let max_distance = 2.0_f32.sqrt() * PORTABLE_RADIUS as f32;
        let mut candidates = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let center_x = x + size / 2;
                let center_y = y + size / 2;
                let center = center_y as usize * N + center_x as usize;
                if self.territory_owner_at(center) != owner
                    || self.placement_geometry_error(owner, DOCK, x, y, 0) != 0 { continue; }
                let distance = ((center_x - home_x) as f32).hypot((center_y - home_y) as f32);
                let mut wet = 0;
                let mut samples = 0;
                for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                    for step in [4, 6, 8, 10] {
                        let [sx, sy] = [center_x + dx * step, center_y + dy * step];
                        if !(0..N as i32).contains(&sx) || !(0..N as i32).contains(&sy) { break; }
                        samples += 1;
                        // 004DFE30 counts eTerrainMoveType_Ocean, not River.
                        wet += usize::from(self.map.ter[sy as usize * N + sx as usize] == T_DEEP);
                    }
                }
                let score = (max_distance - distance) * (4.0 / max_distance) + 0.5 * (wet as f32 / samples as f32);
                candidates.push((score, x, y));
            }
        }
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, x, y) in candidates.into_iter().take(COAST_LIMIT) {
            // Limit this initial adapter to an uncontested, owned shore. The
            // original death and threat rasters are not available here.
            if self.ents[..self.high].iter().enumerate().any(|(index, actor)| actor.alive && actor.hp > 0.0
                && DEFS[actor.def as usize].armed() && self.combat_hostile(owner, actor.team)
                && (actor.x - x as f32).abs().max((actor.y - y as f32).abs()) < 16.0
                && self.can_see(owner, index)) {
                continue;
            }
            let turn = self.dock_auto_turn_for(owner, x, y);
            if turn != NONE && self.begin_construction(owner, DOCK, x, y, turn, builders, false) != NOIDX {
                return true;
            }
        }
        false
    }
}
