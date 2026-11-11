//! Food reactivation searches nearby stocks and then the worker's own Farms.
use super::{forest_play164::individual_score, world0_resources236::StockRestart};
use crate::{
    ai9_count::Precision53::TowardZero,
    defs::{FARM, LAND, WORKER},
    game::{Ent, Game, BOARDED, NONE, O_GATHER},
    map::{R_FOOD, R_NONE},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FoodSource {
    World0(u32),
    Farm(u32),
}

impl FoodSource {
    pub(super) fn order(actor: &Ent) -> Option<Self> {
        if actor.order != O_GATHER || actor.q[0] != u32::from(R_FOOD) { return None; }
        if actor.otgt != NONE { return Some(Self::Farm(actor.otgt)); }
        (actor.q[3] != 0).then_some(Self::World0(actor.q[3]))
    }
}

impl Game {
    pub(crate) fn rect_farm_reservers(&self, owner: u8, handle: u32) -> usize {
        self.ents[..self.high].iter().filter(|actor| actor.alive && actor.hp > 0.0
            && actor.def == WORKER && actor.team == owner && actor.stuck != BOARDED
            && FoodSource::order(actor) == Some(FoodSource::Farm(handle))).count()
    }

    fn food_farm(&self, owner: u8, handle: u32) -> Option<&Ent> {
        let farm = &self.ents[self.get(handle)?];
        (farm.def == FARM && farm.team == owner && farm.hp > 0.0 && farm.built >= 1.0).then_some(farm)
    }

    fn food_route(&self, owner: u8, source: FoodSource, component: &[u8]) -> Option<u32> {
        let reachable = |tile: usize| self.map.pass[0][tile] && component[tile] != 255;
        match source {
            FoodSource::World0(id) => {
                let stock = self.world0_target(id)?;
                let tile = self.map.tile_of(stock.point[0], stock.point[1]);
                reachable(tile).then_some(tile as u32)
            }
            FoodSource::Farm(handle) => {
                let farm = self.food_farm(owner, handle)?;
                let (x, y, width, height) = Self::building_bounds(farm);
                let reach = Self::building_half(farm) + 0.95;
                // The building can block its centre. Use a connected tile
                // in the same work band as the ordinary Farm gather body.
                (y - 1..=y + height).flat_map(|ty| (x - 1..=x + width).map(move |tx| (tx, ty)))
                    .filter(|&(tx, ty)| self.map.layout.contains(tx, ty))
                    .map(|(tx, ty)| self.map.tile_index(tx as usize, ty as usize))
                    .filter_map(|tile| {
                        let [tx, ty] = self.map.tile_center(tile);
                        let (dx, dy) = ((tx - farm.x).abs(), (ty - farm.y).abs());
                        (reachable(tile) && dx.max(dy) <= reach).then_some((tile as u32, dx * dx + dy * dy))
                    }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(tile, _)| tile)
            }
        }
    }

    fn food_point(&self, owner: u8, source: FoodSource) -> Option<[f32; 2]> {
        match source {
            FoodSource::World0(id) => self.world0_target(id)
                .filter(|stock| stock.kind == R_FOOD && stock.remaining > 0.0).map(|stock| stock.point),
            FoodSource::Farm(handle) => self.food_farm(owner, handle).map(|farm| [farm.x, farm.y]),
        }
    }

    fn food_has_spare(&self, index: usize, owner: u8, source: FoodSource) -> bool {
        let current = &self.ents[index];
        let self_reserved = current.alive && current.hp > 0.0 && current.team == owner
            && current.stuck != BOARDED && FoodSource::order(current) == Some(source);
        let (reserved, limit) = match source {
            FoodSource::World0(id) => (self.world0_reservers(id), self.world0_target(id).unwrap().limit),
            FoodSource::Farm(handle) => (self.rect_farm_reservers(owner, handle), 1),
        };
        reserved < limit + usize::from(self_reserved)
    }

    fn food_component(&self, actor: &Ent) -> Vec<u8> {
        self.map.land_component(self.map.tile_of(actor.x, actor.y), self.denied_border_territories(actor))
    }

    pub(super) fn rect_food_restart(&self, index: usize, actor: &Ent) -> Option<StockRestart> {
        let old = FoodSource::order(actor)?;
        let [x, y] = self.food_point(actor.team, old)?;
        let component = self.food_component(actor);
        let old_contact = self.food_route(actor.team, old, &component).is_some();
        if old_contact && self.food_has_spare(index, actor.team, old) { return Some(StockRestart::Keep); }

        // Saturation adds at least 200 in the original, so a full source
        // cannot beat the Food score ceiling of 150.
        let mut best = None;
        let mut score_limit = 150.0;
        let world0 = self.authored_food_ids().map(FoodSource::World0);
        let farms = self.ents[..self.high].iter().enumerate().filter_map(|(index, farm)| {
            (farm.alive && farm.def == FARM && farm.team == actor.team).then(|| FoodSource::Farm(self.h(index)))
        });
        for candidate in world0.chain(farms) {
            if candidate == old { continue; }
            let Some([cx, cy]) = self.food_point(actor.team, candidate) else { continue; };
            let distance = individual_score((x, y), (cx, cy), 1);
            let score = match candidate {
                FoodSource::World0(_) if distance <= 25.0 => TowardZero.store_f32(TowardZero.add(f64::from(distance), 50.0)),
                FoodSource::World0(_) => continue,
                FoodSource::Farm(_) => distance,
            };
            if score < score_limit && self.food_has_spare(index, actor.team, candidate)
                && self.food_route(actor.team, candidate, &component).is_some()
            {
                best = Some(candidate);
                score_limit = score;
            }
        }
        if best.is_none() && old_contact { return Some(StockRestart::Extra); }
        Some(StockRestart::Food(best))
    }

    pub(super) fn bind_rect_food(&self, actor: &mut Ent, source: FoodSource) -> bool {
        let Some([x, y]) = self.food_point(actor.team, source) else { return false; };
        if !self.citizen_can_gather(actor.team, R_FOOD) { return false; }
        let key = self.food_route(actor.team, source, &self.food_component(actor))
            .unwrap_or_else(|| self.key_for(LAND, x, y));
        (actor.otgt, actor.q[3]) = match source {
            FoodSource::World0(id) => (NONE, id),
            FoodSource::Farm(handle) => (handle, 0),
        };
        actor.q[4] = 0;
        actor.otile = self.map.tile_of(x, y) as u32;
        actor.prev = R_NONE;
        actor.fkey = key;
        actor.fslot = u16::MAX;
        true
    }

    pub(super) fn archive_food_source(&self, source: FoodSource, high: usize) -> bool {
        match source {
            FoodSource::World0(id) => self.world0_target(id).is_some_and(|stock| stock.kind == R_FOOD),
            FoodSource::Farm(handle) => handle != NONE && ((handle & 0xffff) as usize) < high
                // A Farm can disappear before this queued goal is updated.
                // A reused slot must never bind the old goal to its new actor.
                && self.get(handle).is_none_or(|index| self.ents[index].def == FARM),
        }
    }

    pub(crate) fn rect_farm_yields_work(&self, index: usize, actor: &Ent, x: f32, y: f32, reach: f32) -> bool {
        self.rect_farm_extra(index) && self.ents[..self.high].iter().enumerate().any(|(other, worker)| {
            other != index && worker.alive && worker.hp > 0.0 && worker.team == actor.team
                && worker.def == WORKER && worker.stuck != BOARDED
                && FoodSource::order(worker) == Some(FoodSource::Farm(actor.otgt))
                && worker.q[1] == 0 && (worker.carry <= 0.0 || matches!(worker.q[2], 0 | 1))
                && (x - worker.x).abs().max((y - worker.y).abs()) <= reach
                && !self.rect_farm_pending(other)
        })
    }
}
