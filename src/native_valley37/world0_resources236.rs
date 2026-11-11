//! Mutable World0 Unit stocks keyed by the authenticated archive's packed IDs.
use super::{active_game, population234::{SourceUnit, UnitKind}, population234_attach::AttachedPopulation};
use crate::game::fishing81::{FISHING_SHIP, GATHER_RANGE, MAX_FISHERS};
use crate::{cmd::C_RECT_GATHER, defs::{Def, DEFS, LAND, WORKER}, economy, game::{Ent, Game, NONE, O_BUILD, O_GATHER}, map::*};

const STOCK_TAG: &[u8; 4] = b"DVR1";
const REACH: f32 = 0.9;

#[cfg(feature = "ee2-rect277-gather-resume")]
pub(super) enum StockRestart {
    Keep,
    Extra,
    End,
    Switch(u32),
    Forest(u32),
    #[cfg(feature = "ee2-rect278-food-resume")]
    Food(Option<super::food_resume278::FoodSource>),
}

fn land_worker_limit(unit: &SourceUnit) -> usize {
    match unit.kind {
        UnitKind::OrdinaryTree => 3,
        UnitKind::Carcass => 2,
        UnitKind::ForageTree => 6,
        UnitKind::Mineral if unit.resource < 7 => 6,
        _ => 0,
    }
}

fn kind(unit: &SourceUnit) -> u8 {
    if unit.kind == UnitKind::Fish { return R_FISH; }
    economy::NATIVE_IDS.iter().position(|&id| id == unit.resource).map_or(R_NONE, |slot| slot as u8 + 1)
}

fn always_exhaustible(unit: &SourceUnit) -> bool {
    matches!(unit.kind, UnitKind::OrdinaryTree | UnitKind::Fish | UnitKind::Carcass)
}

#[derive(Clone, Copy)]
pub(crate) struct FishSite {
    pub(crate) point: (f32, f32),
    pub(crate) remaining: f32,
}

pub(crate) fn fish_order(actor: &Ent) -> Option<u32> {
    (actor.alive && actor.def == FISHING_SHIP && actor.order == O_GATHER && actor.q[4] != 0)
        .then_some(actor.q[4])
}

#[derive(Clone, Copy)]
pub(crate) struct GatherTarget {
    pub(crate) point: [f32; 2],
    pub(crate) kind: u8,
    pub(crate) remaining: f32,
    pub(super) limit: usize,
    pub(super) exhaustible: bool,
}

impl AttachedPopulation {
    fn target(&self, id: u32) -> Option<GatherTarget> {
        let index = self.index_of(id)?;
        let unit = self.source().units().get(index)?;
        let limit = land_worker_limit(unit);
        if limit == 0 { return None; }
        let [x, y, _] = unit.xyz;
        Some(GatherTarget {
            point: [x, y], kind: kind(unit), remaining: self.remaining[index],
            limit, exhaustible: always_exhaustible(unit),
        })
    }

    fn fish(&self, id: u32) -> Option<FishSite> {
        let index = self.index_of(id)?;
        let unit = &self.source().units()[index];
        if unit.kind != UnitKind::Fish { return None; }
        Some(FishSite { point: (unit.xyz[0], unit.xyz[1]), remaining: self.remaining[index] })
    }

    fn stock_changes(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.source().units().iter().zip(&self.remaining).filter_map(|(unit, &stock)|
            (stock.to_bits() != (unit.initial_stock as f32).to_bits())
                .then_some((unit.packed_id, stock.to_bits())))
    }

    fn take(&mut self, id: u32, amount: f32) -> bool {
        let Some(index) = self.index_of(id) else { return false; };
        let remaining = f64::from(self.remaining[index]) - f64::from(amount);
        self.remaining[index] = if remaining < 1.0 { 0.0 } else { remaining as f32 };
        remaining >= 1.0
    }
}

fn ordered_id(actor: &Ent) -> Option<u32> {
    (actor.alive && actor.def == WORKER && actor.q[3] != 0
        && (actor.order == O_GATHER && actor.otgt == NONE || actor.order == O_BUILD && actor.prev != R_NONE))
        .then_some(actor.q[3])
}

impl Game {
    pub(super) fn world0_target(&self, id: u32) -> Option<GatherTarget> {
        let population = self.rect_population.as_ref()?;
        #[cfg(feature = "ee2-rect242-hunt")]
        if let Some(target) = population.hunt_corpse_target(id) { return Some(target); }
        population.target(id)
    }

    pub(crate) fn world0_gather_order(&self, actor: &Ent) -> Option<GatherTarget> {
        self.world0_target(ordered_id(actor)?)
    }

    pub(crate) fn world0_fish(&self, id: u32) -> Option<FishSite> {
        self.rect_population.as_ref()?.fish(id)
    }

    pub(crate) fn take_world0_fish(&mut self, id: u32, amount: f32) -> bool {
        let Some(population) = &mut self.rect_population else { return false; };
        population.fish(id).is_some_and(|site| site.remaining > 0.0) && population.take(id, amount)
    }

    pub(crate) fn world0_fish_visible(&self, team: u8, id: u32) -> Option<FishSite> {
        if !self.fishing_enabled || self.net || team as usize >= self.nplayers { return None; }
        let site = self.world0_fish(id)?;
        (site.remaining > 0.0 && self.real_fog(team, self.map.tile_of(site.point.0, site.point.1)) > 0)
            .then_some(site)
    }

    pub(crate) fn pick_world0_fish(&self, team: u8, x: f32, y: f32) -> Option<u32> {
        if !self.map.layout.contains_point(x, y) { return None; }
        self.rect_population.as_ref()?.resources_at(x as u16, y as u16)
            .filter(|unit| self.world0_fish_visible(team, unit.packed_id).is_some())
            .min_by(|a, b| {
                let distance = |unit: &SourceUnit| (unit.xyz[0] - x).powi(2) + (unit.xyz[1] - y).powi(2);
                distance(a).total_cmp(&distance(b)).then_with(|| a.packed_id.cmp(&b.packed_id))
            }).map(|unit| unit.packed_id)
    }

    pub(super) fn world0_reservers(&self, id: u32) -> usize {
        self.ents[..self.high].iter().filter(|actor|
            actor.order == O_GATHER && ordered_id(actor) == Some(id)).count()
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    pub(super) fn world0_restart(&self, index: usize, actor: &Ent) -> Option<StockRestart> {
        use super::forest_play164::individual_score;
        use StockRestart::*;

        if actor.order != O_GATHER { return None; }
        #[cfg(feature = "ee2-rect278-food-resume")]
        if actor.q[0] == u32::from(R_FOOD) { return self.rect_food_restart(index, actor); }
        let stock = ordered_id(actor)?;
        let population = self.rect_population.as_ref()?;
        let old = population.target(stock)?;
        // Exhausted sources have their own cargo and end behavior.
        if old.kind == R_FOOD || old.remaining <= 0.0 { return None; }
        let old_actor_stock = ordered_id(&self.ents[index])
            .filter(|_| self.ents[index].order == O_GATHER);
        let spare = |id, target: GatherTarget| target.limit as i32 - self.world0_reservers(id) as i32
            + i32::from(old_actor_stock == Some(id));
        let component = self.map.land_component(self.map.tile_of(actor.x, actor.y),
            self.denied_border_territories(actor));
        let admitted = |target: GatherTarget| {
            let tile = self.map.tile_of(target.point[0], target.point[1]);
            self.map.pass[0][tile] && component[tile] != 255
        };
        let old_contact = admitted(old);
        let old_spare = spare(stock, old);
        if old_contact && old_spare > 0 { return Some(Keep); }

        let center = (old.point[0], old.point[1]);
        let world0 = population.source().units().iter().filter_map(|unit| {
            if kind(unit) != old.kind { return None; }
            let point = (unit.xyz[0], unit.xyz[1]);
            if individual_score(center, point, 1) > 25.0 { return None; }
            let id = unit.packed_id;
            let target = population.target(id)?;
            if target.remaining <= 0.0 || !admitted(target) { return None; }
            Some((Switch(id), individual_score(center, point, spare(id, target))))
        });
        let promoted = (old.kind == R_WOOD).then(|| self.clearance_promoted_stock(actor, center))
            .flatten().map(|(row, score)| (Forest(row), score));
        let individual = world0.chain(promoted).min_by(|a, b| a.1.total_cmp(&b.1));
        if old.kind == R_WOOD && individual.as_ref().is_none_or(|(_, score)| *score >= 1.0) {
            if let Some(row) = self.clearance_compressed_stock(actor, old.point) {
                return Some(Forest(row));
            }
        }
        match individual {
            Some((Switch(id), _)) if id == stock => (),
            Some((selected, _)) => return Some(selected),
            None if !old_contact => return Some(End),
            None => (),
        }
        Some(if old_spare <= 0 { Extra } else { Keep })
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    pub(super) fn authored_food_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.rect_population.iter().flat_map(|population| {
            population.source().units().iter().filter_map(|unit| {
                (kind(unit) == R_FOOD && land_worker_limit(unit) > 0).then_some(unit.packed_id)
            }).chain(population.food_corpse_ids())
        })
    }

    fn world0_allowed(&self, team: u8, id: u32) -> Option<GatherTarget> {
        if self.net || team as usize >= self.nplayers { return None; }
        let target = self.world0_target(id)?;
        let [x, y] = target.point;
        let tile = self.map.tile_of(x, y);
        (target.remaining > 0.0 && self.citizen_can_gather(team, target.kind)
            && self.real_fog(team, tile) > 0).then_some(target)
    }

    pub(crate) fn pick_world0_resource(&self, team: u8, x: f32, y: f32) -> Option<u32> {
        if !self.map.layout.contains_point(x, y) { return None; }
        self.rect_population.as_ref()?.resources_at(x as u16, y as u16)
            .filter(|unit| self.world0_allowed(team, unit.packed_id).is_some())
            .min_by(|a, b| {
                let distance = |unit: &SourceUnit| (unit.xyz[0] - x).powi(2) + (unit.xyz[1] - y).powi(2);
                distance(a).total_cmp(&distance(b)).then_with(|| a.packed_id.cmp(&b.packed_id))
            }).map(|unit| unit.packed_id)
    }

    #[cfg(feature = "ee2-rect239-fresh")]
    pub(crate) fn nearest_world0_ai_resource(
        &self, worker: usize, resource: u8, reach_squared: f32,
    ) -> Option<(u32, f32)> {
        let actor = &self.ents[worker];
        let population = self.rect_population.as_ref()?;
        let denied = self.denied_border_territories(actor);
        #[cfg(feature = "ee2-rect291-ai-scout")]
        let component = self.map.land_component(self.map.tile_of(actor.x, actor.y), denied);
        let ids = population.source().units().iter()
            .filter_map(|unit| (kind(unit) == resource).then_some(unit.packed_id));
        #[cfg(feature = "ee2-rect291-ai-scout")]
        let ids = ids.chain(population.food_corpse_ids().filter(|_| resource == R_FOOD));
        ids.filter_map(|id| {
            let target = self.world0_allowed(actor.team, id)?;
            let [x, y] = target.point;
            let distance = (x - actor.x).powi(2) + (y - actor.y).powi(2);
            let tile = self.map.tile_of(x, y);
            if distance > reach_squared || !self.map.pass[0][tile] ||
                !self.map.permits_territory(tile, denied) || self.world0_reservers(id) >= target.limit
            {
                return None;
            }
            #[cfg(feature = "ee2-rect291-ai-scout")]
            if component[tile] == 255 { return None; }
            Some((id, distance))
        }).min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
    }

    pub(crate) fn order_world0_resource(&mut self, team: u8, id: u32, workers: &[usize]) {
        if let Some(site) = self.world0_fish_visible(team, id) {
            self.order_world0_fish(id, site, workers);
            return;
        }
        let Some(target) = self.world0_allowed(team, id) else { return; };
        self.order_world0_land_resource(team, id, workers, target);
    }

    #[cfg(feature = "ee2-rect253-native-carcass")]
    pub(super) fn carcass_followup_admissible(&self, actor: &Ent, corpse: u32, [x, y]: [f32; 2]) -> bool {
        if corpse == 0 || !self.citizen_can_gather(actor.team, R_FOOD) { return false; }
        let tile = self.map.tile_of(x, y);
        self.map.pass[0][tile] && self.map.permits_territory(tile, self.denied_border_territories(actor))
    }

    #[cfg(feature = "ee2-rect253-native-carcass")]
    pub(super) fn order_world0_carcass_followup(&mut self, team: u8, id: u32, index: usize) {
        let Some(target) = self.rect_population.as_ref().and_then(|population| population.hunt_corpse_target(id)) else { return; };
        if target.remaining > 0.0 && self.carcass_followup_admissible(&self.ents[index], id, target.point) {
            self.order_world0_land_resource(team, id, &[index], target);
        }
    }

    fn order_world0_land_resource(&mut self, team: u8, id: u32, workers: &[usize], target: GatherTarget) {
        let [x, y] = target.point;
        let tile = self.map.tile_of(x, y);
        let key = self.key_for(LAND, x, y);
        let mut reserved = self.world0_reservers(id);
        for &index in workers {
            let actor = &self.ents[index];
            if actor.team != team || actor.def != WORKER || !actor.alive || actor.hp <= 0.0
                || actor.stuck == crate::game::BOARDED
                || actor.order == O_GATHER && ordered_id(actor) == Some(id)
            {
                continue;
            }
            if reserved >= target.limit { break; }
            let carried = if actor.carry <= 0.0 { 0 }
                else if actor.q[2] != 0 { actor.q[2] } else { actor.q[0] };
            #[cfg(feature = "ee2-rect276-clearance-resume")]
            self.cancel_rect_clearance(index);
            let actor = &mut self.ents[index];
            actor.order = O_GATHER;
            actor.otile = tile as u32;
            actor.otgt = NONE;
            actor.tgt = NONE;
            actor.fkey = key;
            actor.fslot = u16::MAX;
            actor.stuck = 0;
            actor.prev = R_NONE;
            let returning = carried != 0 && (actor.q[1] == 1 || carried != u32::from(target.kind));
            actor.q = [u32::from(target.kind), u32::from(returning), carried, id, 0];
            crate::native_attack_clock::stop(&mut self.attack_clock[index].flags);
            reserved += 1;
        }
    }

    pub(crate) fn do_world0_gather(&mut self, actor: &mut Ent, def: &Def,
        want: &mut (f32, f32), moving: &mut bool) {
        let Some(id) = ordered_id(actor) else { return; };
        let Some(target) = self.world0_gather_order(actor) else { Self::set_idle(actor); return; };
        if !self.citizen_can_gather(actor.team, target.kind) {
            actor.carry = 0.0;
            actor.q[1] = 0;
            actor.q[2] = 0;
            actor.q[4] = 0;
            Self::set_idle(actor);
            return;
        }
        if actor.q[1] == 1 || target.remaining == 0.0 {
            actor.q[4] = 0;
            if actor.carry > 0.0 {
                actor.q[1] = 1;
                self.return_cargo(actor, def, want, moving);
            } else { actor.q[1] = 0; }
            if actor.carry == 0.0 && target.remaining == 0.0 { Self::set_idle(actor); }
            return;
        }
        let [x, y] = target.point;
        let (dx, dy) = (x - actor.x, y - actor.y);
        if dx * dx + dy * dy > REACH * REACH {
            actor.q[4] = 0;
            *want = self.steer(actor, def, x, y);
            *moving = true;
            return;
        }
        actor.rot = dy.atan2(dx);
        if actor.q[4] == 0 {
            #[cfg(feature = "ee2-rect277-gather-resume")]
            if self.ents[..self.high].iter().filter(|worker| {
                worker.order == O_GATHER && worker.q[4] == 1 && ordered_id(worker) == Some(id)
            }).count() >= target.limit {
                return;
            }
            actor.q[4] = 1;
            return;
        }
        let player = self.players[actor.team as usize];
        let Some(stats) = crate::ee2_units::lookup_class(WORKER, player.epoch) else { return; };
        let (Some(rates), Some(capacity)) = (stats.gather, stats.carry) else { return; };
        let amount = rates[usize::from(target.kind - 1)] * self.sim_step();
        let population = self.rect_population.as_mut().unwrap();
        #[cfg(feature = "ee2-rect242-hunt")]
        let collected = !target.exhaustible || population.take_hunt_corpse(id, amount)
            .unwrap_or_else(|| population.take(id, amount));
        #[cfg(not(feature = "ee2-rect242-hunt"))]
        let collected = !target.exhaustible || population.take(id, amount);
        if !collected {
            for worker in &mut self.ents[..self.high] {
                if worker.order == O_GATHER && ordered_id(worker) == Some(id) { worker.q[4] = 0; }
            }
            actor.q[4] = 0;
            return;
        }
        let scale = self.crown_gather_scale(actor.team, WORKER, target.kind, false, target.kind == R_FOOD);
        let factor = self.modifiers[actor.team as usize][WORKER as usize][economy::gather_modifier(target.kind)] * scale;
        let carried = f64::from(factor) * f64::from(player.gather) * f64::from(amount) + f64::from(actor.carry);
        actor.carry = carried.min(f64::from(capacity)) as f32;
        actor.q[2] = u32::from(target.kind);
        actor.q[1] = u32::from(carried >= f64::from(capacity));
        actor.q[4] = 1 - actor.q[1];
    }

    #[cfg(feature = "ee2-rect289-citizen-work")]
    pub(crate) fn observe_world0_citizen_work(&mut self, index:usize, before:&Ent, actor:&Ent, moving:bool) {
        if actor.order != O_GATHER { return; }
        let Some(id) = ordered_id(actor) else { return; };
        let Some(target) = self.world0_target(id) else { return; };
        // Gather's full-cargo path delegates to Deposit. Only its actual
        // approach Move requests a carrying state, even when motion is blocked.
        if before.q[1] == 1 || target.remaining == 0.0 {
            if moving && actor.carry > 0.0 && actor.q[1] == 1 {
                self.request_citizen_work(index, if actor.q[2] == u32::from(R_WOOD) {29} else {30}, 0.0);
            } else {
                self.retain_citizen_work(index);
            }
            return;
        }
        // Claiming a cutter slot first queues the source approach child. The
        // ordinary resource state is requested only by an already active,
        // successful nonfull work pulse. A full pulse retains the last tuple.
        if moving { return; }
        if before.q[4] == 0 {
            self.retain_citizen_work(index);
            return;
        }
        if actor.q[1] != 0 || actor.q[4] != 1 {
            self.retain_citizen_work(index);
            return;
        }
        let tree_food = self.rect_population.as_ref().and_then(|pop| pop.unit(id))
            .is_some_and(|unit| unit.kind == UnitKind::ForageTree);
        let (state, span) = match target.kind {
            R_FOOD => (if tree_food {22} else {20}, 0.0),
            R_WOOD => (28, f32::from_bits(0x3f4c_cccd)),
            R_STONE => (26, f32::from_bits(0x3f4c_cccd)),
            R_SALTPETER => (27, f32::from_bits(0x3f4c_cccd)),
            _ => (25, f32::from_bits(0x3f4c_cccd)),
        };
        self.request_citizen_work(index, state, span);
    }

    pub(crate) fn world0_stock_section(&self) -> Option<Vec<u8>> {
        let population = self.rect_population.as_ref()?;
        let rows: Vec<_> = population.stock_changes().collect();
        if rows.is_empty() && !self.ents[..self.high].iter().any(|actor| ordered_id(actor).is_some() || fish_order(actor).is_some()) { return None; }
        let mut out = Vec::with_capacity(8 + rows.len() * 8);
        out.extend_from_slice(STOCK_TAG);
        out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
        for (id, stock) in rows {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&stock.to_le_bytes());
        }
        Some(out)
    }

    pub(crate) fn restore_world0_stock(&mut self, bytes: &[u8]) -> Option<()> {
        let population = self.rect_population.as_mut()?;
        let body = bytes.strip_prefix(STOCK_TAG)?;
        let (count, records) = body.split_at_checked(4)?;
        let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
        if count > population.remaining.len() || records.len() != count * 8 { return None; }
        let mut previous = None;
        let mut changes = Vec::with_capacity(count);
        for record in records.chunks_exact(8) {
            let id = u32::from_le_bytes(record[..4].try_into().ok()?);
            let stock = f32::from_le_bytes(record[4..].try_into().ok()?);
            let index = population.index_of(id)?;
            let unit = &population.source().units()[index];
            if previous.is_some_and(|last| index <= last) ||
                (land_worker_limit(unit) == 0 && unit.kind != UnitKind::Fish) || !always_exhaustible(unit)
                || unit.kind == UnitKind::Fish && !self.fishing_enabled
                || stock.to_bits() != 0 && !(1.0..unit.initial_stock as f32).contains(&stock)
            {
                return None;
            }
            changes.push((index, stock));
            previous = Some(index);
        }
        for (index, stock) in changes { population.remaining[index] = stock; }
        Some(())
    }

    pub(crate) fn valid_world0_orders(&self, high: usize, has_section: bool) -> bool {
        let Some(population) = self.rect_population.as_ref() else { return true; };
        let mut reserved = std::collections::BTreeMap::<u32, usize>::new();
        #[cfg(feature = "ee2-rect277-gather-resume")]
        let mut active = std::collections::BTreeMap::<u32, usize>::new();
        let mut has_orders = false;
        for (_index, actor) in self.ents[..high].iter().enumerate() {
            if let Some(id) = fish_order(actor) {
                let Some(site) = population.fish(id) else { return false; };
                has_orders = true;
                let count = reserved.entry(id).or_default();
                *count += 1;
                if !self.fishing_enabled || self.map.tile_of(site.point.0, site.point.1) as u32 != actor.otile
                    || !self.map.layout.contains_point(actor.ox, actor.oy)
                    || (actor.ox - site.point.0).hypot(actor.oy - site.point.1) > GATHER_RANGE
                    || *count > MAX_FISHERS || actor.prev > 1 || actor.prev == 1 && actor.q[1] == 1
                {
                    return false;
                }
                continue;
            }
            if actor.alive && actor.def == FISHING_SHIP && actor.order == O_GATHER { return false; }
            let Some(id) = ordered_id(actor) else { continue; };
            has_orders = true;
            let Some(target) = self.world0_target(id) else { return false; };
            let [x, y] = target.point;
            if self.map.tile_of(x, y) as u32 != actor.otile { return false; }
            if actor.order == O_BUILD {
                if actor.prev != target.kind { return false; }
                continue;
            }
            #[cfg(not(feature = "ee2-rect277-gather-resume"))]
            {
                let count = reserved.entry(id).or_default();
                *count += 1;
                if *count > target.limit { return false; }
            }
            #[cfg(feature = "ee2-rect277-gather-resume")]
            {
                // The original keeps Gather reservations while a Deposit
                // child runs and can resume beyond the nominal slot count.
                // Only simultaneous cutters are bounded by the stock limit.
                let cutters = active.entry(id).or_default();
                *cutters += usize::from(actor.q[4] == 1);
                if *cutters > target.limit { return false; }
            }
            if actor.q[0] != u32::from(target.kind) || actor.q[1] > 1 || actor.q[4] > 1
                || actor.q[1] == 1 && actor.q[4] == 1
            {
                return false;
            }
        }
        has_section == (has_orders || population.stock_changes().next().is_some())
    }

    pub(crate) fn hash_world0_resources(&self, mix: &mut impl FnMut(u32)) {
        let Some(population) = self.rect_population.as_ref() else { return; };
        let changed = population.stock_changes().count();
        if changed != 0 {
            mix(u32::from_le_bytes(*STOCK_TAG));
            mix(changed as u32);
            for (id, stock) in population.stock_changes() { mix(id); mix(stock); }
        }
        for (index, actor) in self.ents[..self.high].iter().enumerate() {
            if let Some(id) = ordered_id(actor) {
                mix(u32::from_le_bytes(*b"DVRO"));
                mix(self.h(index));
                mix(id);
                mix(actor.q[4]);
            }
            if let Some(id) = fish_order(actor) {
                mix(u32::from_le_bytes(*b"DVFO"));
                mix(self.h(index));
                mix(id);
                mix(u32::from(actor.prev));
            }
        }
    }
}

#[cfg(feature = "ee2-rect253-native-carcass")]
#[no_mangle]
pub extern "C" fn native_rect_carcass_worker_word(handle: u32) -> u32 {
    let active = || {
        let game = active_game()?;
        let actor = &game.ents[game.get(handle)?];
        if actor.order != O_GATHER { return None; }
        let id = ordered_id(actor)?;
        game.rect_population.as_ref()?.hunt_corpse_target(id)?;
        Some(id)
    };
    active().unwrap_or(u32::MAX)
}

#[no_mangle]
pub extern "C" fn native_rect_world0_unit_count() -> u32 {
    active_game().and_then(|game| game.rect_population.as_ref())
        .map_or(0, |population| population.unit_count() as u32)
}

#[cfg(feature = "ee2-rect252-animal-birth")]
#[no_mangle]
pub extern "C" fn native_rect_world0_birth_count() -> u32 {
    active_game().and_then(|game| game.rect_population.as_ref()).map_or(0, |population| population.newborns.len() as u32)
}

#[no_mangle]
pub extern "C" fn native_rect_world0_unit_word(slot: usize, field: u32) -> u32 {
    let Some(game) = active_game() else { return u32::MAX; };
    let Some(population) = game.rect_population.as_ref() else { return u32::MAX; };
    let Some(unit) = population.unit_by_slot(slot) else { return u32::MAX; };
    #[cfg(feature = "ee2-rect246-deer-live")]
    let (xyz, yaw) = {
        let pose = population.hunt_pose(unit);
        (pose.xyz, pose.yaw)
    };
    #[cfg(not(feature = "ee2-rect246-deer-live"))]
    let (xyz, yaw) = (unit.xyz, unit.yaw);
    let value = match field {
        0 => unit.packed_id,
        1 => u32::from(unit.type_index),
        2 => unit.kind as u32,
        3 => u32::from(unit.resource),
        4..=6 => xyz[(field - 4) as usize].to_bits(),
        7 => yaw.to_bits(),
        8 => unit.visibility,
        9 => unit.initial_stock,
        10 => population.remaining.get(slot).copied().unwrap_or(0.0).to_bits(),
        11 => unit.flags,
        12 => u32::from(game.vis[game.map.tile_of(xyz[0], xyz[1])]),
        #[cfg(feature = "ee2-rect242-hunt")]
        13..=16 => game.world0_hunt_word(unit, field),
        #[cfg(feature = "ee2-rect246-deer-live")]
        17 => game.rect_deer_walk_word(unit.packed_id),
        #[cfg(feature = "ee2-rect246-deer-live")]
        18..=20 => game.rect_deer_destination_word(unit.packed_id, (field - 18) as usize),
        #[cfg(feature = "ee2-rect249-deer-contact")]
        21..=24 => game.rect_deer_contact_word(unit.packed_id, (field - 21) as usize),
        #[cfg(feature = "ee2-rect251-animal-slope")]
        31..=36 => game.rect_grade_word(unit.packed_id, field).unwrap_or(u32::MAX),
        #[cfg(feature = "ee2-rect252-animal-birth")]
        37 => u32::from(slot >= population.source().units().len()),
        _ => u32::MAX,
    };
    #[cfg(feature = "ee2-rect250-animals")]
    if value == u32::MAX {
        if let Some(animals) = &population.animals {
            if field == 29 && game.rect_deer_fleeing(unit.packed_id) { return 2; }
            return animals.word(unit.packed_id, field, game.sim_elapsed()).unwrap_or(value);
        }
    }
    value
}

#[no_mangle]
pub extern "C" fn cmd_rect_gather(id: u32) -> u32 {
    if active_game().is_none() { return 0; }
    let game = crate::g();
    if game.solo_resigned || game.winner != 0 { return 0; }
    let fish = game.world0_fish_visible(game.local, id);
    if fish.is_some() {
        if game.ents[..game.high].iter().filter(|actor| fish_order(actor) == Some(id)).count() >= MAX_FISHERS {
            return 0;
        }
    } else if game.world0_allowed(game.local, id).is_none_or(|target| game.world0_reservers(id) >= target.limit) {
        return 0;
    }
    let workers: Vec<_> = crate::selected(game).into_iter().filter(|&index| {
        if let Some(site) = fish { return game.can_order_world0_fish(id, site, index); }
        let definition = game.ents[index].def;
        DEFS[definition as usize].has(crate::defs::F_WORKER)
    }).collect();
    if workers.is_empty() { return 0; }
    let handles = crate::handles(game, &workers);
    game.submit(C_RECT_GATHER, 0.0, 0.0, id as i32, &handles);
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defs::TC, native_valley37::rectangular_world::{spatial_tests::prepare_authenticated_inputs, staged_game}};

    fn fixture() -> (Game, usize) {
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 1).unwrap();
        for seat in 0..2 {
            let player = &mut game.players[seat];
            player.alive = true;
            player.ai = seat != 0;
            player.ai_next_attack = 4800;
            player.population_limit = 250;
            player.res = [400.0; 3];
            player.recompute();
            game.rebuild_modifiers(seat as u8, false);
        }
        game.spawn(TC, 0, 54.75, 200.5);
        game.spawn(TC, 1, 88.45, 10.05);
        let worker = game.spawn(WORKER, 0, 54.78, 204.15);
        game.build_grid();
        game.update_fog();
        (game, worker)
    }

    fn stock(game: &Game, id: u32) -> f32 {
        let pop = game.rect_population.as_ref().unwrap();
        pop.remaining[pop.index_of(id).unwrap()]
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn world0_three_original_trees_gather_and_resume_as_independent_stocks() {
        let (mut game, worker) = fixture();
        for id in [924, 888, 884] {
            let [x, y] = game.rect_population.as_ref().unwrap().target(id).unwrap().point;
            assert_eq!(game.pick_world0_resource(0, x, y), Some(id));
        }
        assert_eq!(game.pick_world0_resource(0, 54.78, 204.15), Some(924));
        let tile = game.map.tile_of(54.78, 204.15);
        assert_eq!((game.map.res[tile], game.map.amt[tile]), (R_NONE, 0));
        game.save();
        assert_eq!(&game.savebuf[..8], b"DVS1\x01\0\0\0");
        if std::env::var_os("EE2_WRITE_WORLD0_CHECKPOINT").is_some() {
            std::fs::write("../private-stock-opening.dvs1", &game.savebuf).unwrap();
        }
        let handle = game.h(worker);
        game.apply(1, C_RECT_GATHER, 0.0, 0.0, 924, &[handle]);
        assert_eq!(game.ents[worker].q[3], 0);
        game.apply(0, C_RECT_GATHER, 0.0, 0.0, 924, &[handle]);
        assert_eq!(game.ents[worker].q[3], 924);
        game.save();
        assert_eq!(&game.savebuf[..8], b"DVS1\x02\0\0\0");
        assert_eq!(Game::load(&game.savebuf).unwrap().state_hash(), game.state_hash());
        game.tick();
        assert_eq!(stock(&game, 924), 1277.0);
        assert_eq!(game.ents[worker].q[4], 1);
        game.tick();
        assert_eq!(stock(&game, 924).to_bits(), 0x449f_9ea4);
        assert_eq!(game.ents[worker].carry.to_bits(), 0x3d2e_147b);
        assert_eq!([stock(&game, 888), stock(&game, 884)], [1277.0; 2]);
        game.save();
        let checkpoint = game.savebuf.clone();
        let mut resumed = Game::load(&checkpoint).unwrap();
        assert_eq!(resumed.state_hash(), game.state_hash());
        resumed.save();
        assert_eq!(resumed.savebuf, checkpoint);
        if std::env::var_os("EE2_WRITE_WORLD0_CHECKPOINT").is_some() {
            std::fs::write("../private-stock-checkpoint.dvs1", &checkpoint).unwrap();
        }
        for _ in 0..8 {
            game.tick();
            resumed.tick();
            assert_eq!(resumed.state_hash(), game.state_hash());
        }
        let mut game = resumed;
        let pop = game.rect_population.as_mut().unwrap();
        let index = pop.index_of(924).unwrap();
        pop.remaining[index] = 1.01;
        let carried = game.ents[worker].carry;
        game.tick();
        assert_eq!(stock(&game, 924), 0.0);
        assert_eq!(game.ents[worker].carry, carried);
        assert_eq!([stock(&game, 888), stock(&game, 884)], [1277.0; 2]);
        assert_ne!(game.pick_world0_resource(0, 54.78, 204.15), Some(924));
        for _ in 0..120 {
            game.tick();
            if game.ents[worker].order != O_GATHER { break; }
        }
        assert!(game.players[0].res[1] > 400.0);
        assert_eq!(game.ents[worker].carry, 0.0);
        assert_eq!((game.map.res[tile], game.map.amt[tile]), (R_NONE, 0));
    }

    #[cfg(feature = "ee2-rect235-forest")]
    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn world0_and_compressed_forest_resume_together() {
        use crate::{cmd::C_VALLEY_FOREST_GATHER, game::O_VALLEY_FOREST};
        use crate::native_valley37::{hash::sha256, rectangular_save234 as checkpoint};
        let (mut game, stock_worker) = fixture();
        let row = game.rect_forest.as_ref().unwrap().source(775).unwrap();
        let [x, y, _] = row.position();
        let forest_worker = game.spawn(WORKER, 0, x + 0.2, y);
        game.build_grid();
        game.update_fog();
        let stock_handle = game.h(stock_worker);
        let forest_handle = game.h(forest_worker);
        game.apply(0, C_RECT_GATHER, 0.0, 0.0, 924, &[stock_handle]);
        game.apply(0, C_VALLEY_FOREST_GATHER, 0.0, 0.0, 775, &[forest_handle]);
        assert_eq!(game.ents[forest_worker].order, O_VALLEY_FOREST);
        for _ in 0..2 { game.tick(); }
        assert_eq!(stock(&game, 924).to_bits(), 0x449f_9ea4);
        game.save();
        let saved = game.savebuf.clone();
        let (_, locator) = checkpoint::split(&saved).unwrap();
        let locator = locator.unwrap();
        assert!(locator.stock.unwrap().starts_with(b"DVR1"));
        assert!(locator.forest.unwrap().starts_with(b"DVF1"));
        let mut restored = Game::load(&saved).expect("both mutable sections restored before validating workers");
        assert_eq!(restored.state_hash(), game.state_hash());
        restored.save();
        assert_eq!(restored.savebuf, saved);
        if std::env::var_os("EE2_WRITE_WORLD0_CHECKPOINT").is_some() {
            std::fs::write("../private-world0-forest-checkpoint.dvs1", &saved).unwrap();
        }
        for _ in 0..8 {
            game.tick();
            restored.tick();
            assert_eq!(game.state_hash(), restored.state_hash());
        }
        let mut near_depleted = game.rect_forest_section().unwrap();
        assert_eq!(near_depleted.len(), 24);
        assert_eq!(&near_depleted[8..12], &775u32.to_le_bytes());
        near_depleted[12..16].copy_from_slice(&1.01f32.to_bits().to_le_bytes());
        game.restore_rect_forest_section(&near_depleted).unwrap();
        game.tick();
        let stump = game.rect_forest_section().unwrap();
        assert_eq!(&stump[12..16], &0f32.to_bits().to_le_bytes());
        assert_ne!(&stump[20..24], &u32::MAX.to_le_bytes());
        game.save();
        let mut felled = Game::load(&game.savebuf).unwrap();
        assert_eq!(felled.state_hash(), game.state_hash());
        felled.save();
        assert_eq!(felled.savebuf, game.savebuf);
        if std::env::var_os("EE2_WRITE_WORLD0_CHECKPOINT").is_some() {
            std::fs::write("../private-world0-stump-checkpoint.dvs1", &felled.savebuf).unwrap();
        }
        for section in [locator.stock.unwrap(), locator.forest.unwrap()] {
            let mut forged = saved.clone();
            let index = section.as_ptr() as usize - saved.as_ptr() as usize;
            forged[index] ^= 1;
            let end = forged.len() - 32;
            let digest = sha256(&forged[..end]);
            forged[end..].copy_from_slice(&digest);
            assert!(Game::load(&forged).is_none());
        }
    }

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn world0_capacity_and_bad_stock_archives_are_atomic() {
        let (mut game, worker) = fixture();
        let extra: Vec<_> = (0..3).map(|i| game.spawn(WORKER, 0, 55.0 + i as f32 * 0.3, 204.0)).collect();
        let mut selected = vec![game.h(worker)];
        selected.extend(extra.iter().map(|&i| game.h(i)));
        game.apply(0, C_RECT_GATHER, 0.0, 0.0, 924, &selected);
        assert_eq!(game.world0_reservers(924), 3);
        assert_eq!(game.ents[extra[2]].q[3], 0);
        game.apply(0, C_RECT_GATHER, 0.0, 0.0, 888, &[selected[3]]);
        assert_eq!(game.world0_reservers(888), 1);
        let mut bad = b"DVR1".to_vec();
        bad.extend_from_slice(&2u32.to_le_bytes());
        for (id, value) in [(924u32, 1200.0f32), (888, f32::NAN)] {
            bad.extend_from_slice(&id.to_le_bytes());
            bad.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        let before = game.state_hash();
        assert!(game.restore_world0_stock(&bad).is_none());
        assert_eq!(game.state_hash(), before);
        assert_eq!(stock(&game, 924), 1277.0);
    }
}
