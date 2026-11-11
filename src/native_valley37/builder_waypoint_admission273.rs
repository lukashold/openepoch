//! Admit original Build proposals against the current rectangular world.
use super::{
    builder_arrival272::in_work_band,
    builder_waypoint273::{self, BuildWaypointAdmission},
    city_minerals269::BoxBound,
};
use crate::{game::{Ent, Game}, map::RegionMask};

struct WaypointAdmission<'a> {
    game: &'a Game,
    site: &'a Ent,
    denied: RegionMask,
    component: Option<Vec<u8>>,
}

impl<'a> WaypointAdmission<'a> {
    fn new(game: &'a Game, worker: &Ent, site: &'a Ent) -> Self {
        Self {
            game, site,
            denied: game.border_egress_mask(worker), component: None,
        }
    }

    fn tile_at(&self, [x, y]: [f32; 2]) -> Option<usize> {
        let [w, h] = self.game.map.world_dimensions();
        (x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32)
            .then(|| self.game.map.tile_of(x, y))
    }

    fn permits_half_cell(&self, cell: [i32; 2]) -> bool {
        let map = &self.game.map;
        let [x, y] = cell.map(|axis| axis.div_euclid(2));
        if !map.layout.contains(x, y) { return false; }
        let tile = map.tile_index(x as usize, y as usize);
        map.pass[0][tile] && map.permits_territory(tile, self.denied)
    }

    fn segment(&self, from: [f32; 2], to: [f32; 2]) -> Result<(), Option<[i32; 2]>> {
        if self.tile_at(from).is_none() || self.tile_at(to).is_none() { return Err(None); }
        let start = from.map(|axis| f64::from(axis) * 2.0);
        let end = to.map(|axis| f64::from(axis) * 2.0);
        let mut cell = start.map(|axis| axis.floor() as i32);
        let goal = end.map(|axis| axis.floor() as i32);
        let delta = [end[0] - start[0], end[1] - start[1]];
        let step = delta.map(|axis| if axis > 0.0 { 1 } else { -1 });
        let mut on_land = self.permits_half_cell(cell);
        while cell != goal {
            let crossing = |axis: usize| {
                if cell[axis] == goal[axis] { return f64::INFINITY; }
                let next = cell[axis] + i32::from(step[axis] > 0);
                (f64::from(next) - start[axis]) / delta[axis]
            };
            let (tx, ty) = (crossing(0), crossing(1));
            if tx == ty && on_land {
                let side_x = [cell[0] + step[0], cell[1]];
                let side_y = [cell[0], cell[1] + step[1]];
                if !self.permits_half_cell(side_x) { return Err(Some(side_x)); }
                if !self.permits_half_cell(side_y) { return Err(Some(side_y)); }
            }
            if tx <= ty { cell[0] += step[0]; }
            if ty <= tx { cell[1] += step[1]; }
            let passable = self.permits_half_cell(cell);
            // A mover can leave a blocked starting run. A later blocked
            // crossing would enter an obstacle.
            if !passable && on_land { return Err(Some(cell)); }
            on_land |= passable;
        }
        if on_land { Ok(()) } else { Err(Some(cell)) }
    }

    fn failed_cell_hits_site(&self, [hx, hy]: [i32; 2]) -> bool {
        // 00870CC0 tests the first failed leaf's vertical prism against the
        // pending City's cached collision box, distinct from the work band.
        let leaf = BoxBound::from_half_extents(
            [hx as f32 * 0.5 + 0.25, hy as f32 * 0.5 + 0.25, 0.0], [0.25, 0.25, 1024.0], 0.0,
        );
        let [x, y] = [self.site.x, self.site.y];
        let city = BoxBound::from_half_extents(
            [x, y, self.game.map.height(x, y)],
            [f32::from_bits(0x401d_c07d), f32::from_bits(0x4007_1fde), 1.0], self.site.rot,
        );
        leaf.overlaps(city)
    }

    fn point_reachable(&mut self, from: [f32; 2], to: [f32; 2]) -> bool {
        let Some(tile) = self.tile_at(to) else { return false; };
        let map = &self.game.map;
        if !map.pass[0][tile] || !map.permits_territory(tile, self.denied) { return false; }
        if self.segment(from, to).is_ok() { return true; }
        let Some(origin) = self.tile_at(from) else { return false; };
        self.component.get_or_insert_with(|| map.land_component(origin, self.denied))[tile] != 255
    }
}

impl BuildWaypointAdmission for WaypointAdmission<'_> {
    fn ground_height(&mut self, [x, y]: [f32; 2]) -> Option<f32> {
        self.tile_at([x, y]).map(|_| self.game.map.height(x, y))
    }

    fn point(&mut self, [wx, wy, _]: [f32; 3], [x, y, _]: [f32; 3]) -> bool {
        in_work_band([x, y], [self.site.x, self.site.y], self.site.rot)
            && self.point_reachable([wx, wy], [x, y])
    }

    fn inward(&mut self, [x, y, _]: [f32; 3], [sx, sy, _]: [f32; 3]) -> bool {
        match self.segment([x, y], [sx, sy]) {
            Ok(()) => true,
            Err(cell) => cell.is_some_and(|leaf| self.failed_cell_hits_site(leaf)),
        }
    }

    fn outward(&mut self, [x, y, _]: [f32; 3], to: [f32; 2]) -> bool {
        self.segment([x, y], to).is_ok()
    }
}

impl Game {
    pub(super) fn rect_city_source_workpoint(&self, worker: &Ent, site: &Ent, in_band: bool) -> Option<[f32; 2]> {
        let position = |actor: &Ent| [actor.x, actor.y, self.map.height(actor.x, actor.y)];
        let mut admission = WaypointAdmission::new(self, worker, site);
        builder_waypoint273::search(position(worker), position(site), site.rot, in_band, &mut admission)
            .map(|[x, y, _]| [x, y])
    }

    pub(super) fn rect_city_can_follow_workpoint(&self, worker: &Ent, site: &Ent) -> bool {
        WaypointAdmission::new(self, worker, site)
            .point_reachable([worker.x, worker.y], [worker.ox, worker.oy])
    }

    pub(super) fn rect_city_workpoint_has_clear_tail(&self, worker: &Ent, site: &Ent) -> bool {
        WaypointAdmission::new(self, worker, site)
            .segment([worker.x, worker.y], [worker.ox, worker.oy]).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::WaypointAdmission;
    use crate::game::{Ent, Game, E0};

    #[test]
    fn failed_half_cell_uses_pending_body_for_inward_recovery() {
        let game = Game::new(7, 0, 0, 2, 0);
        let site = Ent { x: 72.0, y: 24.0, ..E0 };
        let admission = WaypointAdmission::new(&game, &E0, &site);
        // Both cells are in the same portable tile. Only the first source
        // failure prism overlaps the intended City's collision body.
        assert!(admission.failed_cell_hits_site([148, 48]));
        assert!(!admission.failed_cell_hits_site([149, 48]));
    }
}
