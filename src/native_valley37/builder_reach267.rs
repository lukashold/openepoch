//! Leave a coarse startup City reservation through the native-clear half-cells.
use super::{fresh239::is_start_city, mobile_placement239::BoxShape};
use crate::{defs::{DEFS, TC, WORKER}, game::{Ent, Game, NONE, O_BUILD}, map::{T_ROCK, T_SAND}};

const TILE_MASK: u32 = 0xffff;

impl Game {
    fn start_city_for_builder(&self, builder: &Ent) -> Option<Ent> {
        let tile = self.map.tile_of(builder.x, builder.y);
        if builder.def != WORKER || !self.generated_city_foundations
            || self.rect_ai_clock.is_none() || !self.supports_rectangular_city_foundations()
            || !self.map.blocked[tile]
        {
            return None;
        }
        // Fresh239 allocates City, Citizen and Scout in that order for each seat.
        // The first-generation handle and admitted pose survive a cold restore.
        let index = self.get(u32::from(builder.team) * 3)?;
        let city = self.ents[index];
        let (x, y, w, h) = Self::building_bounds(&city);
        if city.def != TC || city.team != builder.team || city.built < 1.0 || city.hp <= 0.0
            || !is_start_city(city.x, city.y, city.rot)
            || !(x as f32..(x + w) as f32).contains(&builder.x)
            || !(y as f32..(y + h) as f32).contains(&builder.y)
        {
            return None;
        }
        let overlaps_other = self.ents[..self.high].iter().enumerate().any(|(i, other)| {
            if i == index || !other.alive || !DEFS[other.def as usize].building {
                return false;
            }
            let (ox, oy, ow, oh) = Self::building_bounds(other);
            x < ox + ow && ox < x + w && y < oy + oh && oy < y + h
        });
        (!overlaps_other).then_some(city)
    }

    fn start_city_exit_clear(&self, builder: &Ent, city: &Ent, exit: usize) -> bool {
        if !self.map.pass[0][exit] {
            return false;
        }
        let (x, y, w, h) = Self::building_bounds(city);
        let [ex, ey] = self.map.tile_xy(exit).map(|v| v as i32);
        if !(x - 1..=x + w).contains(&ex) || !(y - 1..=y + h).contains(&ey)
            || ((x..x + w).contains(&ex) && (y..y + h).contains(&ey))
        {
            return false;
        }

        let shape = BoxShape::city([city.x, city.y], city.rot);
        let start = [builder.x, builder.y].map(|v| f64::from(v) * 2.0);
        let goal = self.map.tile_center(exit).map(|v| f64::from(v) * 2.0);
        let low = [0, 1].map(|i| start[i].min(goal[i]).ceil() as i32 - 1);
        let high = [0, 1].map(|i| start[i].max(goal[i]).floor() as i32);
        let denied = self.border_egress_mask(builder);
        for hy in low[1]..=high[1] {
            for hx in low[0]..=high[0] {
                if !touches_cell(start, goal, [hx, hy]) {
                    continue;
                }
                let (tx, ty) = (hx.div_euclid(2), hy.div_euclid(2));
                if !self.map.layout.contains(tx, ty) {
                    return false;
                }
                let tile = self.map.tile_index(tx as usize, ty as usize);
                let terrain = self.map.ter[tile];
                if !self.map.permits_territory(tile, denied) || terrain < T_SAND || terrain == T_ROCK
                    || (!self.map.pass[0][tile] && !((x..x + w).contains(&tx) && (y..y + h).contains(&ty)))
                    || shape.blocks_half_cell([hx, hy])
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn rect_start_build_key(&self, builder: &Ent) -> u32 {
        let Some(city) = self.start_city_for_builder(builder) else { return NONE; };
        let (x, y, w, h) = Self::building_bounds(&city);
        let denied = self.border_egress_mask(builder);
        let mut best = (f32::INFINITY, NONE);
        for ey in y - 1..=y + h {
            for ex in x - 1..=x + w {
                if !self.map.layout.contains(ex, ey) {
                    continue;
                }
                let exit = self.map.tile_index(ex as usize, ey as usize);
                let work = self.placement_cache.approach[exit];
                if work == NONE || !self.map.permits_territory(work as usize, denied) {
                    continue;
                }
                let [gx, gy] = self.map.tile_center(exit);
                let distance = (gx - builder.x).powi(2) + (gy - builder.y).powi(2);
                if distance < best.0 && self.start_city_exit_clear(builder, &city, exit) {
                    // This combined key is only used during route assignment.
                    // The worker's persisted flow key always stays a LAND key.
                    best = (distance, (exit as u32) << 16 | work);
                }
            }
        }
        best.1
    }

    pub(crate) fn rect_build_egress(&mut self, builder: &mut Ent, site: &Ent) -> Option<(f32, f32)> {
        if !Self::has_rect_build_exit(builder) {
            return None;
        }
        if site.built >= 1.0 || self.map.pass[0][self.map.tile_of(builder.x, builder.y)] {
            Self::clear_rect_build_exit(builder);
            return None;
        }
        let mut exit = (-builder.rx - 2.0) as usize;
        if !self.start_city_for_builder(builder).is_some_and(|city| self.start_city_exit_clear(builder, &city, exit)) {
            self.prepare_build_routes(Self::building_bounds(site));
            let key = self.rect_start_build_key(builder);
            builder.vx = 0.0;
            builder.vy = 0.0;
            if key == NONE {
                return Some((0.0, 0.0));
            }
            Self::assign_build_key(builder, key);
            exit = (key >> 16) as usize;
        }
        let [x, y] = self.map.tile_center(exit);
        let (dx, dy) = (x - builder.x, y - builder.y);
        let distance = dx.hypot(dy).max(1e-4);
        Some((dx / distance, dy / distance))
    }

    pub(crate) fn has_rect_build_exit(builder: &Ent) -> bool {
        builder.order == O_BUILD && builder.def == WORKER && builder.rx < -1.0
    }

    pub(crate) fn clear_rect_build_exit(builder: &mut Ent) {
        if Self::has_rect_build_exit(builder) {
            builder.rx = -1.0;
        }
    }

    pub(crate) fn assign_rect_build_exit(builder: &mut Ent, key: u32) -> u32 {
        let exit = key >> 16;
        if exit == 0 {
            if builder.def == WORKER && builder.rx < -1.0 {
                builder.rx = -1.0;
            }
            return key;
        }
        // Only buildings use the saved rally coordinates. A negative rally X
        // on a construction worker stores the temporary native-clear exit,
        // leaving the ordinary flow key usable by earlier save readers.
        builder.rx = -(exit as f32) - 2.0;
        builder.vx = 0.0;
        builder.vy = 0.0;
        key & TILE_MASK
    }

    pub(crate) fn valid_rect_build_exit(&self, builder: &Ent) -> bool {
        if !Self::has_rect_build_exit(builder) {
            return true;
        }
        if !self.generated_city_foundations
            || self.rect_ai_clock.is_none() || !self.supports_rectangular_city_foundations()
        {
            return false;
        }
        let exit = -builder.rx - 2.0;
        if !(1.0..=TILE_MASK as f32).contains(&exit) || exit.fract() != 0.0 || builder.fkey > TILE_MASK {
            return false;
        }
        [exit as u32, builder.fkey].into_iter().all(|tile| {
            let [x, y] = self.map.tile_xy(tile as usize).map(|v| v as i32);
            self.map.layout.contains(x, y)
        })
    }
}

// Slab intersection on the half-cell grid. Include edge/corner contact so
// an exit cannot cut through a native blocked cell or a coarse diagonal gap.
fn touches_cell(start: [f64; 2], goal: [f64; 2], cell: [i32; 2]) -> bool {
    let (mut near, mut far) = (0.0f64, 1.0f64);
    for i in 0..2 {
        let edge = f64::from(cell[i]);
        let delta = goal[i] - start[i];
        if delta == 0.0 {
            if !(edge..=edge + 1.0).contains(&start[i]) {
                return false;
            }
            continue;
        }
        let a = (edge - start[i]) / delta;
        let b = (edge + 1.0 - start[i]) / delta;
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    near <= far
}
