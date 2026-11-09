//! Shared placement validation. Conservative rotated grid bounds and shoreline
//! auto-orientation are explicit adapters; native building input rotates 90deg.
use crate::{defs::*, game::*, map::*};
pub const ROTATION_STEPS: u32 = 8;
pub struct PlacementCache {
    pub key: Option<(u32, i32, i32, i32, i32)>,
    pub approach: Vec<u32>,
    pub queue: Vec<u32>,
}
impl Default for PlacementCache {
    fn default() -> Self {
        Self {
            key: None,
            approach: vec![NONE; NN],
            queue: Vec::with_capacity(NN),
        }
    }
}
impl Game {
    pub fn footprint_side(def:DefId, turn: u32) -> i32 {
        let (width, height) = Self::footprint_axes(def, turn);
        width.max(height)
    }
    pub fn footprint_axes(def: DefId, turn: u32) -> (i32, i32) {
        let Some(d) = DEFS.get(def as usize) else {
            return (0, 0);
        };
        if !d.building || turn >= ROTATION_STEPS {
            return (0, 0);
        }
        #[cfg(feature = "ee2-native13")]
        if let Some([x, y]) = crate::wonders61::shape(def, true) {
            if turn % 2 != 0 { return (0, 0); }
            let axes = (x.ceil() as i32, y.ceil() as i32);
            return if turn % 4 == 0 { axes } else { (axes.1, axes.0) };
        }
        let scale = if turn & 1 == 0 {
            1.0
        } else {
            core::f32::consts::SQRT_2
        };
        let side = (d.size * scale - 0.00001).ceil() as i32;
        (side, side)
    }
    pub fn angle_for_turn(turn: u32) -> f32 {
        turn as f32 * core::f32::consts::FRAC_PI_4
    }
    pub fn building_bounds(e: &Ent) -> (i32, i32, i32, i32) {
        #[cfg(feature = "ee2-native13")]
        if e.def == crate::koth64::HILL {
            let side = crate::koth64::COLLISION_SIDE;
            return Self::rect_bounds(e, [side, side]);
        }
        #[cfg(feature = "ee2-native13")]
        if let Some(size) = crate::wonders61::shape(e.def, false) {
            return Self::rect_bounds(e, size);
        }
        let size = DEFS[e.def as usize].size;
        let span = size * (e.rot.cos().abs() + e.rot.sin().abs());
        let x0 = (e.x - span * 0.5 + 0.0001).floor() as i32;
        let y0 = (e.y - span * 0.5 + 0.0001).floor() as i32;
        let x1 = (e.x + span * 0.5 - 0.0001).ceil() as i32;
        let y1 = (e.y + span * 0.5 - 0.0001).ceil() as i32;
        let side = (x1 - x0).max(y1 - y0);
        (x0, y0, side, side)
    }
    #[cfg(feature = "ee2-native13")]
    pub(crate) fn building_blocks_tile(e: &Ent, x: i32, y: i32) -> bool {
        if e.def == crate::koth64::HILL {
            // The fixed 45-degree Mode-1 rectangle is a diamond. The tile
            // blocks if any of its four original half-cells hits that diamond.
            let gap = |center: f32, tile: i32| (center - (tile as f32 + 0.5)).abs().max(0.5) - 0.5;
            let extent = (crate::koth64::COLLISION_SIDE * 0.5 - 0.01) * core::f32::consts::SQRT_2;
            return gap(e.x, x) + gap(e.y, y) < extent;
        }
        true
    }
    #[cfg(feature = "ee2-native13")]
    fn rect_bounds(e: &Ent, [width, height]: [f32; 2]) -> (i32, i32, i32, i32) {
        let (sin, cos) = e.rot.sin_cos();
        let half_x = (width * cos.abs() + height * sin.abs()) * 0.5;
        let half_y = (width * sin.abs() + height * cos.abs()) * 0.5;
        let x0 = (e.x - half_x + 0.0001).floor() as i32;
        let y0 = (e.y - half_y + 0.0001).floor() as i32;
        let x1 = (e.x + half_x - 0.0001).ceil() as i32;
        let y1 = (e.y + half_y - 0.0001).ceil() as i32;
        (x0, y0, x1 - x0, y1 - y0)
    }
    pub fn building_half(e: &Ent) -> f32 {
        let (_, _, width, height) = Self::building_bounds(e);
        width.max(height) as f32 * 0.5
    }
    pub fn building_within(e: &Ent, dx: f32, dy: f32, padding: f32) -> bool {
        let (_, _, width, height) = Self::building_bounds(e);
        dx <= width as f32 * 0.5 + padding && dy <= height as f32 * 0.5 + padding
    }
    pub fn construction_available(&self, team: u8, def: DefId) -> bool {
        #[cfg(feature = "ee2-native13")]
        if def == crate::koth64::HILL { return false; }
        let Some(definition) = DEFS.get(def as usize) else { return false; };
        #[cfg(feature = "ee2-native13")]
        let required_epoch = if def == DOCK && self.first_dock_policy() { 0 } else { definition.epoch };
        #[cfg(not(feature = "ee2-native13"))]
        let required_epoch = definition.epoch;
        if team as usize >= self.nplayers || !definition.building
            || required_epoch > self.players[team as usize].epoch { return false; }
        #[cfg(feature = "ee2-native13")]
        if crate::wonders61::is_wonder(def) {
            return !self.net && self.bench == 0 && self.powers.is_some()
                && crate::wonders61::available(def, self.civilizations[team as usize] as i8, self.players[team as usize].epoch);
        }
        true
    }
    pub fn entity_radius(&self, e: &Ent) -> f32 {
        #[cfg(feature = "ee2-native13")]
        if self.heavy_mounted3 && e.def == crate::defs::LIGHT_MOUNTED3 {
            // The native ellipse branch at 0053AE90 stores SizeX/SizeY unscaled.
            return 0.75;
        }
        if DEFS[e.def as usize].building {
            Self::building_half(e)
        } else {
            DEFS[e.def as usize].rad()
        }
    }
    pub fn footprint_known(&self, team: u8, def:DefId, tx: i32, ty: i32, turn: u32) -> bool {
        let (width, height) = Self::footprint_axes(def, turn);
        if width == 0 || tx < 0 || ty < 0 || tx > N as i32 - width || ty > N as i32 - height {
            return false;
        }
        if self.display_visibility() >= 1 && !self.players[team as usize].ai {
            return true;
        }
        (ty..ty + height)
            .all(|y| (tx..tx + width).all(|x| self.real_fog(team, y as usize * N + x as usize) > 0))
    }
    pub fn placement_geometry_error(&self, team: u8, def:DefId, tx: i32, ty: i32, turn: u32) -> u32 {
        let (width, height) = Self::footprint_axes(def, turn);
        let [map_width, map_height] = self.map.world_dimensions().map(|dimension| dimension as i32);
        if width == 0
            || team as usize >= self.nplayers
            || tx < 2
            || ty < 2
            || tx > map_width - 2 - width
            || ty > map_height - 2 - height
        {
            return 1;
        }
        let center = (ty + height / 2) as usize * N + (tx + width / 2) as usize;
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if self.valley_rejects_building_site(def, (tx, ty, width, height), turn) { return 1; }
        #[cfg(feature = "ee2-native13")]
        if !self.hotspot_allows_placement(team, def, [tx as f32 + width as f32 * 0.5, ty as f32 + height as f32 * 0.5]) {
            return 12;
        }
        let territory = self.map.terr[center];
        let owner = self.towner[territory as usize];
        let foreign = owner != team && owner != 255;
        #[cfg(feature = "ee2-native13")]
        {
            let native_foreign_site = foreign && (owner as usize) < self.nplayers
                && self.diplomacy.borders.is_some()
                && matches!(def, HOUSE | BARRACKS | DOCK | FARM | MILL | STABLE |
                    MARKET | TEMPLE | UNIVERSITY | WAREHOUSE);
            let capacity = match def {
                HOUSE => 6,
                MARKET | TEMPLE | UNIVERSITY => 1,
                _ => 0,
            };
            if native_foreign_site && capacity != 0 && self.ents[..self.high].iter().filter(|e| {
                e.alive && e.hp > 0.0 && e.def == def && (e.built >= 1.0 || e.team == team)
                    && self.map.terr[tile_of(e.x, e.y)] == territory
            }).count() >= capacity {
                return if def == HOUSE { 13 } else { 14 };
            }
            if owner != team && (owner != 255 || matches!(def, TOWER | FORTRESS)) && self.initial_ceasefire_active() {
                return 11;
            }
            if foreign && (!native_foreign_site || self.foreign_construction_is_restricted(team, owner)) {
                return 2;
            }
        }
        #[cfg(not(feature = "ee2-native13"))]
        if foreign {
            return 2;
        }
        if def == TC
            && self.ents[..self.high].iter().any(|e| {
                e.alive
                    && e.hp > 0.0
                    && e.def == TC
                    && self.map.terr[tile_of(e.x, e.y)] == territory
            })
        {
            return 5;
        }
        if def == FORTRESS && self.ents[..self.high].iter().filter(|e| {
            e.alive && e.hp > 0.0 && e.def == FORTRESS && (e.built>=1.0 || e.team==team)
                && self.map.terr[tile_of(e.x,e.y)] == territory
        }).count() >= 2 {
            return 10;
        }
        let deposit = Self::extractor_kind(def);
        if deposit != R_NONE && self.map.res[center] != deposit {
            return 1;
        }
        #[cfg(feature = "ee2-native13")]
        if self.wonder_sites.iter().map(|&index| &self.ents[index]).any(|actor| {
            if !actor.alive || actor.hp <= 0.0 { return false; }
            let Some(size) = crate::wonders61::shape(actor.def, true) else { return false; };
            let (x, y, w, h) = Self::rect_bounds(actor, size);
            tx < x + w && x < tx + width && ty < y + h && y < ty + height
        }) { return 1; }
        let mut water = 0u32;
        let mut land = 0u32;
        #[cfg(feature="ee2-roster6")]
        if def==60 && !crate::roster6_placement::near_accepted_water(&self.map,tx as f32+width as f32*0.5,ty as f32+height as f32*0.5){return 1;}
        for y in ty..ty + height {
            for x in tx..tx + width {
                let t = y as usize * N + x as usize;
                if self.map.blocked[t] || (self.map.res[t] != R_NONE && self.map.res[t] != deposit)
                {
                    return 1;
                }
                if def == DOCK {
                    if self.map.ter[t] == T_ROCK {
                        return 1;
                    }
                    if self.map.ter[t] <= T_SHALLOW {
                        water += 1;
                    } else {
                        land += 1;
                    }
                } else if !self.map.pass[0][t] {
                    return 1;
                }
            }
        }
        if def == DOCK {
            let s = width;
            let mut land_access = false;
            let mut sea_exit = false;
            for k in -1..=s {
                for &(x, y) in &[
                    (tx + k, ty - 1),
                    (tx + k, ty + s),
                    (tx - 1, ty + k),
                    (tx + s, ty + k),
                ] {
                    let t = y as usize * N + x as usize;
                    land_access |= self.map.pass[0][t];
                    sea_exit |= self.map.pass[1][t];
                }
            }
            let cells = (s * s) as u32;
            if land * 10 < cells || water * 4 < cells || water < 3 || !land_access || !sea_exit {
                return 1;
            }
        }
        #[cfg(feature = "ee2-native13")]
        {
            let mut site = Self::planned_building(def, tx, ty, turn);
            site.team = team;
            if self.factory_placement_blocked(&site) { return 1; }
        }
        0
    }
    /// Reverse flood from every exterior work tile. It is independent of client
    /// selection, cached by nav version/site; click revalidates current workers.
    pub fn prepare_build_routes(&mut self, (tx, ty, width, height): (i32, i32, i32, i32)) {
        let key = (self.map.nav_ver, tx, ty, width, height);
        if self.placement_cache.key == Some(key) {
            return;
        }
        let cache = &mut self.placement_cache;
        cache.approach.fill(NONE);
        cache.queue.clear();
        let pass = &self.map.pass[0];
        for y in ty - 1..=ty + height {
            for x in tx - 1..=tx + width {
                if x >= tx && x < tx + width && y >= ty && y < ty + height {
                    continue;
                }
                if x < 0 || y < 0 || x >= N as i32 || y >= N as i32 {
                    continue;
                }
                let t = y as usize * N + x as usize;
                if pass[t] {
                    cache.approach[t] = t as u32;
                    cache.queue.push(t as u32);
                }
            }
        }
        let mut head = 0;
        while head < cache.queue.len() {
            let t = cache.queue[head] as usize;
            head += 1;
            let (x, y) = ((t % N) as i32, (t / N) as i32);
            for (k, &(dx, dy)) in DIRS.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 {
                    continue;
                }
                let q = ny as usize * N + nx as usize;
                if !pass[q] || cache.approach[q] != NONE {
                    continue;
                }
                if k >= 4
                    && !(pass[y as usize * N + nx as usize] && pass[ny as usize * N + x as usize])
                {
                    continue;
                }
                cache.approach[q] = cache.approach[t];
                cache.queue.push(q as u32);
            }
        }
        cache.key = Some(key);
    }
    pub fn valid_builders(&self, team: u8, ids: &[usize]) -> Vec<usize> {
        ids.iter()
            .copied()
            .filter(|&i| {
                self.ents.get(i).map_or(false, |e| {
                    e.alive && e.hp > 0.0 && e.team == team && DEFS[e.def as usize].has(F_WORKER)
                })
            })
            .collect()
    }
    pub fn placement_error_for(
        &mut self,
        team: u8,
        def:DefId,
        tx: i32,
        ty: i32,
        turn: u32,
        ids: &[usize],
        check_fog: bool,
    ) -> u32 {
        let (width, height) = Self::footprint_axes(def, turn);
        if width == 0
            || team as usize >= self.nplayers
            || tx < 2
            || ty < 2
            || tx > N as i32 - 2 - width
            || ty > N as i32 - 2 - height
        {
            return 1;
        }
        if !self.construction_available(team, def) {
            return 8;
        }
        if !self.afford(team, &self.construction_cost(team, def)) {
            return 4;
        }
        if check_fog && !self.footprint_known(team, def, tx, ty, turn) {
            return 3;
        }
        let geometry = self.placement_geometry_error(team, def, tx, ty, turn);
        if geometry != 0 {
            return geometry;
        }
        let builders = self.valid_builders(team, ids);
        if builders.is_empty() {
            return 6;
        }
        let bounds = Self::building_bounds(&Self::planned_building(def, tx, ty, turn));
        self.prepare_build_routes(bounds);
        if !builders.iter().any(|&i| {
            self.placement_cache.approach[tile_of(self.ents[i].x, self.ents[i].y)] != NONE
        }) {
            return 7;
        }
        if self.free.is_empty() {
            return 9;
        }
        0
    }
    pub fn begin_construction(
        &mut self,
        team: u8,
        def:DefId,
        tx: i32,
        ty: i32,
        turn: u32,
        ids: &[usize],
        check_fog: bool,
    ) -> usize {
        if self.placement_error_for(team, def, tx, ty, turn, ids, check_fog) != 0 {
            return NOIDX;
        }
        let builders: Vec<(usize, u32)> = self
            .valid_builders(team, ids)
            .into_iter()
            .filter_map(|i| {
                let t = self.placement_cache.approach[tile_of(self.ents[i].x, self.ents[i].y)];
                (t != NONE).then_some((i, t))
            })
            .collect();
        let bi = self.place_with_rotation(def, tx, ty, team, turn);
        if bi == NOIDX {
            return bi;
        }
        let (x0, y0, width, height) = Self::building_bounds(&self.ents[bi]);
        let ids: Vec<usize> = builders.iter().map(|p| p.0).collect();
        let handle = self.h(bi);
        for (i, tile) in builders {
            let e = &mut self.ents[i];
            e.prev = match e.order {
                O_GATHER if e.otgt == NONE => e.q[0] as u8,
                O_BUILD => e.prev,
                _ => 0,
            };
            e.order = O_BUILD;
            e.q[0] = R_FOOD as QueueItem;
            e.otgt = handle;
            e.tgt = NONE;
            e.fkey = tile;
            e.fslot = u16::MAX;
            e.stuck = 0;
        }
        #[cfg(feature = "ee2-valley74-city-footprint")]
        if self.valley_house_reserves_placement_only(&self.ents[bi]) { return bi; }
        // Own ground units caught by a new foundation step aside; construction
        // itself cannot progress until its builders have left the blocked footprint.
        let mut displaced = Vec::new();
        for i in 0..self.high {
            let e = self.ents[i];
            if !e.alive
                || e.team != team
                || DEFS[e.def as usize].building
                || DEFS[e.def as usize].dom != LAND
                || ids.contains(&i)
            {
                continue;
            }
            if e.x >= x0 as f32
                && e.x < (x0 + width) as f32
                && e.y >= y0 as f32
                && e.y < (y0 + height) as f32
            {
                displaced.push(i);
            }
        }
        for i in displaced {
            let e = self.ents[i];
            let tile = self.map.snap(e.x, e.y, 0);
            self.order_move(
                &[i],
                (tile % N) as f32 + 0.5,
                (tile / N) as f32 + 0.5,
                false,
            );
        }
        bi
    }
    pub fn place_with_rotation(&mut self, def:DefId, tx: i32, ty: i32, team: u8, turn: u32) -> usize {
        if !self.construction_available(team, def)
            || self.placement_geometry_error(team, def, tx, ty, turn) != 0
        {
            return NOIDX;
        }
        let cost = self.construction_cost(team, def);
        if !self.afford(team, &cost) {
            return NOIDX;
        }
        let site = Self::planned_building(def, tx, ty, turn);
        let initial_progress = 1.0;
        #[cfg(feature = "ee2-valley74-city-footprint")]
        let initial_progress = if self.uses_valley_house_movement(def) { 0.0 } else { initial_progress };
        let i = self.spawn_oriented_with_progress(def, team, site.x, site.y, site.rot, initial_progress);
        if i != NOIDX {
            let health = self.max_hp(team, def) * 0.1;
            #[cfg(feature = "ee2-native13")]
            let health = if self.uses_generated_city_foundation(def) { 1.0 } else { health };
            #[cfg(feature = "ee2-valley74-city-footprint")]
            let health = if self.uses_valley_house_construction(def) { 1.0 } else { health };
            self.pay(team, &cost, 1.0);
            self.ents[i].built = 0.0;
            self.ents[i].hp = health;
            #[cfg(feature="ee2-native13")]
            self.wonder_notice(crate::wonders61::Notice::Started, def, team);
            #[cfg(feature="ee2-reference")]
            self.sync_city_ground_entry(i);
            if def == TC {
                self.rebuild_territories();
            }
            #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
            self.recount_paid_players();
        }
        i
    }

    fn planned_building(def: DefId, tx: i32, ty: i32, turn: u32) -> Ent {
        let (width, height) = Self::footprint_axes(def, turn);
        Ent { def, x: tx as f32 + width as f32 * 0.5, y: ty as f32 + height as f32 * 0.5,
            rot: Self::angle_for_turn(turn), ..E0 }
    }
    pub fn construction_multiplier(&self, e: &Ent) -> f32 {
        if e.def == TC {
            return 1.0;
        }
        let owner = self.towner[self.map.terr[tile_of(e.x, e.y)] as usize];
        if owner == e.team {
            1.0
        } else if owner == 255 {
            2.0
        } else {
            3.0
        }
    }
    /// Cardinal shoreline adapter in this engine's XY plane. The original uses a
    /// sampled water normal; this grid proxy is intentionally not native math.
    pub fn dock_auto_turn(&self, tx: i32, ty: i32) -> u32 {
        self.dock_auto_turn_for(self.local, tx, ty)
    }
    pub fn dock_auto_turn_for(&self, team: u8, tx: i32, ty: i32) -> u32 {
        let s = Self::footprint_side(DOCK, 0);
        if self.placement_geometry_error(team, DOCK, tx, ty, 0) != 0 {
            return NONE;
        }
        let mut land = [0; 4];
        let mut water = [0; 4];
        for k in 0..s {
            for (side, (x, y)) in [
                (tx + k, ty - 1),
                (tx + s, ty + k),
                (tx + k, ty + s),
                (tx - 1, ty + k),
            ]
            .into_iter()
            .enumerate()
            {
                let t = y as usize * N + x as usize;
                land[side] += self.map.pass[0][t] as u32;
                water[side] += self.map.pass[1][t] as u32;
            }
        }
        let mut best = 0;
        let mut score = i32::MIN;
        for side in 0..4 {
            let v =
                land[side] as i32 * 3 + water[(side + 2) % 4] as i32 - land[(side + 2) % 4] as i32;
            if v > score {
                score = v;
                best = side;
            }
        }
        ((best as u32 * 2) + 4) % 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flat() -> Game {
        let mut g = Game::new(7, 0, 0, 2, 0);
        for p in &mut g.players[..2] {
            p.epoch = if cfg!(feature = "ee2-reference") {
                0
            } else {
                1
            };
            p.res = [10000.0; 3];
        }
        g.extra_resources = [[10000.0; 6]; 4];
        for t in 0..2 {
            g.rebuild_modifiers(t, false);
        }
        g.map.ter.fill(T_GRASS);
        g.map.res.fill(R_NONE);
        g.map.amt.fill(0);
        g.map.blocked.fill(false);
        for e in &g.ents[..g.high] {
            if e.alive && DEFS[e.def as usize].building {
                let (x, y, width, height) = Game::building_bounds(e);
                for yy in y..y + height {
                    for xx in x..x + width {
                        g.map.blocked[yy as usize * N + xx as usize] = true;
                    }
                }
            }
        }
        for i in 0..NN {
            g.map.upd_pass(i);
        }
        g.map.nav_ver += 1;
        g.diplomacy.visibility = 2;
        g.refresh_display_fog();
        g
    }
    fn site(g: &Game, def:DefId, turn: u32) -> (i32, i32) {
        let side = Game::footprint_side(def, turn);
        let (x, y) = g.map.starts[3];
        (x as i32 - side / 2, y as i32 - side / 2)
    }
    fn worker(g: &mut Game, x: f32, y: f32) {
        let e = &mut g.ents[1];
        e.x = x;
        e.y = y;
        e.px = x;
        e.py = y;
        e.ox = x;
        e.oy = y;
        e.sel = true;
        g.build_grid();
        g.update_fog();
    }
    #[cfg(feature = "ee2-native13")]
    #[test]
    fn foreign_build_uses_citizen_border_terms_not_relation() {
        let mut g = flat();
        let (x, y) = site(&g, HOUSE, 0);
        let territory = g.map.terr[tile_of(x as f32 + 1.5, y as f32 + 1.5)] as usize;
        g.towner[territory] = 1;
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 2);
        assert!(g.configure_sight_terms() && g.configure_timed_treaties(false) && g.configure_border_treaties());
        for relation in [crate::diplomacy::WAR, crate::diplomacy::NEUTRAL, crate::diplomacy::ALLIED] {
            g.diplomacy.relation[1] = relation;
            g.diplomacy.relation[4] = relation;
            assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 0);
        }
        for (term, error) in [(1, 0), (2, 2), (3, 2)] {
            g.diplomacy.borders.as_mut().unwrap().set_term(0, 1, term);
            assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), error);
        }
        g.diplomacy.borders.as_mut().unwrap().territory_count_changed(0, 1, 0);
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 0);
        assert_eq!(g.placement_geometry_error(0, TC, x, y, 0), 2);
        assert!(g.configure_initial_ceasefire(1));
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 11);
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn foreign_house_limit_counts_completed_buildings_and_own_foundations() {
        let mut g = flat();
        let (x, y) = site(&g, HOUSE, 0);
        g.map.terr.fill(1);
        g.towner[1] = 1;
        assert!(g.configure_sight_terms() && g.configure_timed_treaties(false) && g.configure_border_treaties());
        for n in 0..5 {
            assert_ne!(g.spawn(HOUSE, 1, 20.0 + 5.0 * n as f32, 20.0), NOIDX);
        }
        let foreign_site = g.spawn(HOUSE, 1, 20.0, 25.0);
        assert_ne!(foreign_site, NOIDX);
        g.ents[foreign_site].built = 0.25;
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 0);
        let own_site = g.spawn(HOUSE, 0, 25.0, 25.0);
        assert_ne!(own_site, NOIDX);
        g.ents[own_site].built = 0.25;
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 13);
        g.ents[own_site].alive = false;
        g.ents[foreign_site].built = 1.0;
        assert_eq!(g.placement_geometry_error(0, HOUSE, x, y, 0), 13);
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn foreign_economy_uses_the_civilian_border_filter() {
        let mut game = flat();
        let (x, y) = site(&game, HOUSE, 0);
        game.map.terr.fill(1);
        game.towner[1] = 1;
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(false)
            && game.configure_border_treaties());
        for (term, expected) in [(0, 0), (1, 0), (2, 2), (3, 2)] {
            game.diplomacy.borders.as_mut().unwrap().set_term(0, 1, term);
            for def in [FARM, MILL, STABLE, MARKET, TEMPLE, UNIVERSITY, WAREHOUSE] {
                assert_eq!(game.placement_geometry_error(0, def, x, y, 0), expected, "def {def}");
            }
            for def in [TC, TOWER, FORTRESS, WORKSHOP] {
                assert_eq!(game.placement_geometry_error(0, def, x, y, 0), 2, "def {def}");
            }
        }
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn foreign_economy_one_per_type_counts_completed_and_own_foundations() {
        let mut game = flat();
        let (x, y) = site(&game, HOUSE, 0);
        game.map.terr.fill(1);
        game.towner[1] = 1;
        assert!(game.configure_sight_terms() && game.configure_timed_treaties(false)
            && game.configure_border_treaties());
        for (offset, def) in [MARKET, TEMPLE, UNIVERSITY].into_iter().enumerate() {
            let coord = 20.0 + 6.0 * offset as f32;
            let other = game.spawn(def, 1, coord, 20.0);
            assert_ne!(other, NOIDX);
            game.ents[other].built = 0.25;
            assert_eq!(game.placement_geometry_error(0, def, x, y, 0), 0);
            let own = game.spawn(def, 0, coord, 26.0);
            assert_ne!(own, NOIDX);
            game.ents[own].built = 0.25;
            assert_eq!(game.placement_geometry_error(0, def, x, y, 0), 14);
            game.ents[own].alive = false;
            game.ents[other].built = 1.0;
            assert_eq!(game.placement_geometry_error(0, def, x, y, 0), 14);
        }
    }

    #[test]
    fn neutral_city_known_maps_workers_and_one_per_territory() {
        let mut g = flat();
        let (x, y) = site(&g, TC, 0);
        worker(&mut g, x as f32 - 4.5, y as f32 + 2.5);
        g.diplomacy.visibility = 0;
        assert_eq!(g.placement_error_for(0, TC, x, y, 0, &[1], true), 3);
        for mode in [1, 2] {
            g.diplomacy.visibility = mode;
            assert_eq!(g.placement_error_for(0, TC, x, y, 0, &[1], true), 0);
        }
        let bank = g.resource(0, 1);
        let cost = g.unit_cost(0, TC);
        let i = g.begin_construction(0, TC, x, y, 0, &[1], true);
        assert_ne!(i, NOIDX);
        assert_eq!(g.resource(0, 1), bank - cost[1] as f32);
        assert_eq!(g.placement_geometry_error(0, TC, x + 8, y, 0), 5);
        assert_eq!(g.ents[1].otgt, g.h(i));
    }
    #[test]
    fn no_builder_foreign_stale_nonworker_and_disconnected_land_are_rejected() {
        let mut g = flat();
        let (x, y) = site(&g, HOUSE, 0);
        assert_eq!(g.placement_error_for(0, HOUSE, x, y, 0, &[], true), 6);
        assert_eq!(g.placement_error_for(0, HOUSE, x, y, 0, &[0, 6], true), 6);
        let before = g.state_hash();
        assert_eq!(g.begin_construction(0, HOUSE, x, y, 0, &[], true), NOIDX);
        assert_eq!(g.state_hash(), before);
        worker(&mut g, 40.5, 40.5);
        for yy in 38..=42 {
            for xx in 38..=42 {
                if xx == 38 || xx == 42 || yy == 38 || yy == 42 {
                    let t = yy * N + xx;
                    g.map.ter[t] = T_DEEP;
                    g.map.upd_pass(t);
                }
            }
        }
        g.map.nav_ver += 1;
        assert_eq!(g.placement_error_for(0, HOUSE, x, y, 0, &[1], true), 7);
    }
    #[test]
    fn all_eight_turns_block_save_and_render_the_same_bounding_box() {
        for turn in 0..8 {
            let mut g = flat();
            let (x, y) = site(&g, HOUSE, turn);
            worker(&mut g, x as f32 - 4.5, y as f32 + 1.5);
            let side = Game::footprint_side(HOUSE, turn);
            assert!(side >= DEFS[HOUSE as usize].size as i32);
            let i = g.begin_construction(0, HOUSE, x, y, turn, &[1], true);
            assert_ne!(i, NOIDX);
            assert_eq!(Game::building_bounds(&g.ents[i]), (x, y, side, side));
            assert_eq!(g.ents[i].rot, Game::angle_for_turn(turn));
            for yy in y..y + side {
                for xx in x..x + side {
                    assert!(g.map.blocked[yy as usize * N + xx as usize]);
                }
            }
            g.render_with_rotation(1.0, 0.0, 0.0, 120.0, 120.0, -1, 0, 0, 0);
            g.render_with_rotation(
                1.0,
                x as f32 - 10.0,
                y as f32 - 10.0,
                x as f32 + 15.0,
                y as f32 + 15.0,
                -1,
                0,
                0,
                0,
            );
            let row = g.inst.chunks(12).find(|p| p[4] == crate::ids::unit_render(HOUSE)).unwrap();
            assert_eq!(row[3], g.ents[i].rot);
            let scaffold = g
                .inst
                .chunks(12)
                .find(|p| p[4] == crate::render::S_SCAFFOLD)
                .unwrap();
            assert_eq!(scaffold[3], g.ents[i].rot);
            g.save();
            let h = g.state_hash();
            let loaded = Game::load(&g.savebuf).unwrap();
            assert_eq!(loaded.state_hash(), h);
            assert_eq!(Game::building_bounds(&loaded.ents[i]), (x, y, side, side));
            for yy in y..y + side {
                for xx in x..x + side {
                    assert!(loaded.map.blocked[yy as usize * N + xx as usize]);
                }
            }
        }
    }
    #[test]
    fn odd_turn_city_finishes_after_real_worker_walk_without_cheats() {
        let mut g = flat();
        let (x, y) = site(&g, TC, 1);
        worker(&mut g, x as f32 - 5.5, y as f32 + 3.5);
        let start = g.ents[1].x;
        let i = g.begin_construction(0, TC, x, y, 1, &[1], true);
        assert_ne!(i, NOIDX);
        let mut finished = false;
        for _ in 0..2400 {
            g.tick();
            if g.ents[i].built >= 1.0 {
                finished = true;
                break;
            }
        }
        assert!(
            finished,
            "Worker must reach the outside of the six-tile rotated city and finish building"
        );
        assert!(g.ents[1].x > start + 1.0);
        assert!(!g.instant_build);
        assert_eq!(g.construction_multiplier(&g.ents[i]), 1.0);
    }
    #[test]
    fn neutral_house_has_double_construction_time_and_builder_inside_steps_out() {
        let mut g = flat();
        let (x, y) = site(&g, HOUSE, 1);
        let side = Game::footprint_side(HOUSE, 1);
        worker(
            &mut g,
            x as f32 + side as f32 * 0.5,
            y as f32 + side as f32 * 0.5,
        );
        let i = g.begin_construction(0, HOUSE, x, y, 1, &[1], true);
        assert_ne!(i, NOIDX);
        assert_eq!(g.construction_multiplier(&g.ents[i]), 2.0);
        let one = (g.production_time(0, HOUSE) / DT).ceil() as u32;
        for _ in 0..one {
            g.tick();
        }
        assert!(g.ents[i].built < 0.65);
        assert!(g.ents[i].built > 0.2);
        for _ in 0..one + 200 {
            g.tick();
        }
        assert_eq!(g.ents[i].built, 1.0);
        let e = g.ents[1];
        assert!(
            e.x < x as f32
                || e.x >= (x + side) as f32
                || e.y < y as f32
                || e.y >= (y + side) as f32
        );
    }
    #[test]
    fn dock_mixed_shore_and_ocean_exit_work_at_all_turns() {
        for turn in 0..8 {
            let mut g = flat();
            let side = Game::footprint_side(DOCK, turn);
            let (x, y) = (90, 58);
            for yy in 50..80 {
                for xx in 80..115 {
                    let t = yy * N + xx;
                    g.map.terr[t] = 0;
                    g.map.ter[t] = if yy >= 60 { T_DEEP } else { T_GRASS };
                    g.map.upd_pass(t);
                }
            }
            g.map.nav_ver += 1;
            worker(&mut g, 88.5, 55.5);
            assert_eq!(g.placement_error_for(0, DOCK, x, y, turn, &[1], true), 0);
            let auto = g.dock_auto_turn(x, y);
            assert!(auto < 8 && auto % 2 == 0);
            let i = g.begin_construction(0, DOCK, x, y, turn, &[1], true);
            assert_ne!(i, NOIDX);
            for _ in 0..2200 {
                g.tick();
                if g.ents[i].built >= 1.0 {
                    break;
                }
            }
            assert_eq!(
                g.ents[i].built, 1.0,
                "Dock workers must build from real land outside rotated mask"
            );
            assert_eq!(Game::building_bounds(&g.ents[i]).2, side);
        }
    }
    #[test]
    fn rotated_placement_wire_is_lockstep_and_rejects_fractional_tiles() {
        let mut a = flat();
        let mut b = flat();
        b.local = 1;
        let (x, y) = site(&a, TC, 1);
        worker(&mut a, x as f32 - 4.5, y as f32 + 3.5);
        worker(&mut b, x as f32 - 4.5, y as f32 + 3.5);
        let h = a.state_hash();
        a.apply(
            0,
            crate::cmd::C_PLACE_ROTATED,
            x as f32 + 0.5,
            y as f32,
            (TC as i32) | (1 << 8),
            &[a.h(1)],
        );
        assert_eq!(a.state_hash(), h);
        a.submit(
            crate::cmd::C_PLACE_ROTATED,
            x as f32,
            y as f32,
            (TC as i32) | (1 << 8),
            &[a.h(1)],
        );
        let wire = a.outbox.clone();
        a.inbox = wire.clone();
        b.inbox = wire;
        a.net_apply(0);
        b.net_apply(0);
        assert_eq!(a.state_hash(), b.state_hash());
        for _ in 0..160 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }
}
