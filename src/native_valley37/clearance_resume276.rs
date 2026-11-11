//! Preserve the head below the temporary City-clearance Move.
use crate::{
    defs::{FARM, LAND, WORKER},
    game::{Ent, Game, BOARDED, NONE, O_BUILD, O_GATHER, O_IDLE, O_MOVE},
    map::R_NONE,
};
#[cfg(feature = "ee2-rect277-gather-resume")]
use super::world0_resources236::StockRestart;
#[cfg(feature = "ee2-rect278-food-resume")]
use super::food_resume278::FoodSource;

pub(crate) const TAG: &[u8; 4] = b"DCE1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Moving,
    Settling,
    Complete,
    Invalid,
    #[cfg(feature = "ee2-rect277-gather-resume")]
    StockExtra,
    #[cfg(feature = "ee2-rect277-gather-resume")]
    StockEnded,
    #[cfg(feature = "ee2-rect277-gather-resume")]
    ForestEnded,
    #[cfg(feature = "ee2-rect278-food-resume")]
    FarmExtra,
    #[cfg(feature = "ee2-rect278-food-resume")]
    FoodEnded,
    #[cfg(feature = "ee2-rect279-farm-active")]
    FarmActive,
}

impl Phase {
    fn stock(self) -> bool {
        #[cfg(feature = "ee2-rect277-gather-resume")]
        { matches!(self, Self::StockExtra | Self::StockEnded | Self::ForestEnded) }
        #[cfg(not(feature = "ee2-rect277-gather-resume"))]
        { false }
    }

    fn gather(self) -> bool {
        #[cfg(feature = "ee2-rect279-farm-active")]
        if self == Self::FarmActive { return true; }
        #[cfg(feature = "ee2-rect278-food-resume")]
        if matches!(self, Self::FarmExtra | Self::FoodEnded) { return true; }
        self.stock()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Clearance {
    phase: Phase,
    order: u8,
    owner: u8,
    flags: u8,
    target: u32,
    key: u32,
}

impl Clearance {
    #[cfg(feature = "ee2-rect279-farm-active")]
    fn active_farm(self) -> Option<u32> {
        (self.phase == Phase::FarmActive || self.phase == Phase::FarmExtra && self.flags & 1 != 0)
            .then_some(self.target)
    }

    fn words(self, handle: u32) -> [u32; 4] {
        [handle, u32::from_le_bytes([self.phase as u8, self.order, self.owner, self.flags]),
            self.target, self.key]
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    fn from_farm(self) -> bool {
        #[cfg(feature = "ee2-rect279-farm-active")]
        if self.phase == Phase::FarmActive { return true; }
        self.phase == Phase::FarmExtra || self.phase == Phase::FoodEnded && self.flags & 1 != 0
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    fn food_source(self) -> FoodSource {
        if self.from_farm() { FoodSource::Farm(self.target) } else { FoodSource::World0(self.target) }
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    fn food_next(self) -> Option<FoodSource> {
        if self.flags & 2 != 0 { return Some(FoodSource::Farm(self.key)); }
        (self.key != 0).then_some(FoodSource::World0(self.key))
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    fn world0_reservation(self) -> bool {
        #[cfg(feature = "ee2-rect278-food-resume")]
        if self.phase == Phase::FoodEnded { return !self.from_farm(); }
        self.phase.stock()
    }

    fn matches_gather(self, actor: &Ent) -> bool {
        #[cfg(feature = "ee2-rect279-farm-active")]
        if self.phase == Phase::FarmActive {
            return FoodSource::order(actor) == Some(FoodSource::Farm(self.target));
        }
        #[cfg(feature = "ee2-rect278-food-resume")]
        if matches!(self.phase, Phase::FarmExtra | Phase::FoodEnded) {
            return FoodSource::order(actor) == Some(self.food_source()) && (!self.from_farm() || actor.q[3] == 0);
        }
        actor.order == O_GATHER && actor.otgt == NONE && actor.q[3] == self.target
    }
}

impl Game {
    fn valid_farm_extra_flags(flags: u8) -> bool {
        #[cfg(feature = "ee2-rect279-farm-active")]
        { flags <= 1 }
        #[cfg(not(feature = "ee2-rect279-farm-active"))]
        { flags == 0 }
    }

    #[cfg(feature = "ee2-rect279-farm-active")]
    pub(crate) fn rect_farm_work_enabled(&self) -> bool {
        self.rect_ai_clock.is_some() && self.supports_rectangular_city_foundations()
    }

    #[cfg(feature = "ee2-rect279-farm-active")]
    pub(crate) fn rect_farm_cutting(&self, index: usize) -> bool {
        self.rect_clearance.get(&self.h(index)).is_some_and(|row| {
            let actor = &self.ents[index];
            row.active_farm().is_some() && row.matches_gather(actor)
                && self.get(actor.otgt).is_some_and(|farm| {
                    let farm = &self.ents[farm];
                    farm.def == FARM && farm.team == actor.team && farm.hp > 0.0 && farm.built >= 1.0
                })
        })
    }

    #[cfg(feature = "ee2-rect279-farm-active")]
    pub(crate) fn release_rect_farm_work(&mut self, index: usize) {
        #[cfg(feature = "ee2-rect280-farm-crop")]
        self.clear_rect_farm_goal(index);
        let handle = self.h(index);
        let Some(row) = self.rect_clearance.get_mut(&handle) else { return; };
        if row.phase == Phase::FarmExtra {
            row.flags = 0;
        } else if row.phase == Phase::FarmActive {
            self.rect_clearance.remove(&handle);
        }
    }

    #[cfg(feature = "ee2-rect279-farm-active")]
    pub(crate) fn rect_farm_work(&mut self, index: usize, actor: &Ent) -> Option<bool> {
        if !self.rect_farm_work_enabled() { return None; }
        let handle = self.h(index);
        if self.rect_clearance.get(&handle).and_then(|row| row.active_farm()) == Some(actor.otgt) {
            return Some(true);
        }
        let occupied = self.rect_clearance.iter().any(|(&other, row)| {
            row.active_farm() == Some(actor.otgt) && self.get(other).is_some_and(|slot| {
                let worker = &self.ents[slot];
                worker.hp > 0.0 && worker.stuck != BOARDED && row.matches_gather(worker)
            })
        });
        if occupied { return Some(false); }
        match self.rect_clearance.get_mut(&handle) {
            Some(row) if row.phase == Phase::FarmExtra => row.flags = 1,
            None => {
                self.rect_clearance.insert(handle, Clearance {
                    phase: Phase::FarmActive, order: O_GATHER, owner: actor.team, flags: 0, target: actor.otgt, key: 0,
                });
            }
            Some(_) => return Some(false),
        }
        // 007EE833 acquires the cutter, sets the worker's active byte,
        // and returns. Extraction starts with its next regular body.
        Some(false)
    }

    pub(super) fn rect_clearance_can_suspend(&self, actor: &Ent, site: u32) -> bool {
        if actor.def != WORKER || actor.stuck == BOARDED || actor.vx != 0.0 || actor.vy != 0.0 {
            return false;
        }
        match actor.order {
            O_BUILD => actor.otgt != site && actor.rx == -1.0 && self.clearance_build_target(actor).is_some(),
            O_GATHER => self.clearance_gather_key(actor).is_some(),
            _ => false,
        }
    }

    pub(super) fn rect_clearance_suspend(&mut self, index: usize, x: f32, y: f32) {
        let actor = self.ents[index];
        let handle = self.h(index);
        #[cfg(feature = "ee2-rect277-gather-resume")]
        let extra = self.rect_clearance.get(&handle).is_some_and(|row| {
            #[cfg(feature = "ee2-rect279-farm-active")]
            if row.phase == Phase::FarmActive { return true; }
            #[cfg(feature = "ee2-rect278-food-resume")]
            if row.phase == Phase::FarmExtra { return true; }
            row.phase == Phase::StockExtra
        });
        #[cfg(not(feature = "ee2-rect277-gather-resume"))]
        let extra = false;
        if self.rect_clearance.contains_key(&handle) && !extra {
            return;
        }
        #[cfg(feature = "ee2-rect280-farm-crop")]
        self.clear_rect_farm_goal(index);
        let key = self.key_for(LAND, x, y);
        self.rect_clearance.insert(handle, Clearance { phase: Phase::Moving, order: actor.order,
            owner: actor.team, flags: 0, target: actor.otgt, key: actor.fkey });
        self.native_stop_attack(index);
        let actor = &mut self.ents[index];
        actor.order = O_MOVE;
        actor.ox = x;
        actor.oy = y;
        actor.fkey = key;
        actor.fslot = u16::MAX;
        actor.otgt = NONE;
        actor.tgt = NONE;
        actor.stuck = 0;
    }

    pub(crate) fn cancel_rect_clearance(&mut self, index: usize) {
        #[cfg(feature = "ee2-rect280-farm-crop")]
        self.clear_rect_farm_goal(index);
        self.rect_clearance.remove(&self.h(index));
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    pub(crate) fn rect_farm_extra(&self, index: usize) -> bool {
        self.rect_clearance.get(&self.h(index)).is_some_and(|row| row.phase == Phase::FarmExtra)
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    pub(super) fn rect_farm_pending(&self, index: usize) -> bool {
        self.rect_clearance.get(&self.h(index))
            .is_some_and(|row| matches!(row.phase, Phase::FarmExtra | Phase::FoodEnded))
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    pub(crate) fn rect_pending_farm_extent(&self) -> usize {
        self.rect_clearance.values().filter(|row| row.order == O_GATHER).flat_map(|row| {
            let old = (row.from_farm() || !row.phase.gather() && row.target != NONE).then_some(row.target);
            let next = if row.phase == Phase::FoodEnded && row.flags & 2 != 0 { Some(row.key) } else { None };
            [old, next].into_iter().flatten()
        }).map(|handle| (handle & 0xffff) as usize + 1).max().unwrap_or(0)
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    fn install_stock_restart(&mut self, handle: u32, actor: &Ent, restart: StockRestart) {
        let (phase, flags, target, key) = match restart {
            StockRestart::Keep => {
                self.rect_clearance.remove(&handle);
                return;
            }
            #[cfg(feature = "ee2-rect278-food-resume")]
            StockRestart::Extra if actor.otgt != NONE => (Phase::FarmExtra, 0, actor.otgt, 0),
            StockRestart::Extra => (Phase::StockExtra, 0, actor.q[3], 0),
            StockRestart::End => (Phase::StockEnded, 0, actor.q[3], 0),
            StockRestart::Switch(id) => (Phase::StockEnded, 0, actor.q[3], id),
            StockRestart::Forest(row) if self.clearance_promote_forest(row) => (Phase::ForestEnded, 0, actor.q[3], row),
            StockRestart::Forest(_) => (Phase::StockEnded, 0, actor.q[3], 0),
            #[cfg(feature = "ee2-rect278-food-resume")]
            StockRestart::Food(next) => {
                let (target, from_farm) = if actor.otgt != NONE { (actor.otgt, 1) } else { (actor.q[3], 0) };
                let (key, to_farm) = match next {
                    Some(FoodSource::World0(id)) => (id, 0),
                    Some(FoodSource::Farm(id)) => (id, 2),
                    None => (0, 0),
                };
                (Phase::FoodEnded, from_farm | to_farm, target, key)
            }
        };
        self.rect_clearance.insert(handle, Clearance {
            phase, order: O_GATHER, owner: actor.team, flags, target, key,
        });
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    fn pop_stock_end(&mut self, index: usize, handle: u32, actor: &mut Ent, row: Clearance) {
        self.rect_clearance.remove(&handle);
        if row.phase == Phase::ForestEnded && self.clearance_start_forest(actor, row.key) {
            return;
        }
        if row.phase == Phase::StockEnded && row.key != 0 {
            if let Some(target) = self.world0_target(row.key).filter(|target| {
                target.remaining > 0.0 && u32::from(target.kind) == actor.q[0]
                    && self.citizen_can_gather(actor.team, target.kind)
            }) {
                actor.q[3] = row.key;
                actor.q[4] = 0;
                actor.otile = self.map.tile_of(target.point[0], target.point[1]) as u32;
                actor.fkey = self.key_for(LAND, target.point[0], target.point[1]);
                actor.fslot = u16::MAX;
                if let Some(restart) = self.world0_restart(index, actor) {
                    self.install_stock_restart(handle, actor, restart);
                    return;
                }
            }
        }
        Self::set_idle(actor);
    }

    // Run before the ordinary order body. An old goal becomes visible on
    // reactivation, then receives its first regular update on the next pulse.
    pub(crate) fn rect_clearance_before_update(&mut self, index: usize, actor: &mut Ent) -> bool {
        let handle = self.h(index);
        let Some(row) = self.rect_clearance.get(&handle).copied() else { return false; };
        let order = if row.phase.gather() { O_GATHER } else { O_MOVE };
        if actor.team != row.owner || actor.order != order
            || row.phase.gather() && !row.matches_gather(actor)
        {
            self.rect_clearance.remove(&handle);
            return false;
        }
        match row.phase {
            Phase::Moving => false,
            Phase::Settling => {
                actor.order = O_IDLE;
                false
            }
            Phase::Complete => {
                let mut restored = *actor;
                restored.order = row.order;
                restored.otgt = row.target;
                restored.fkey = row.key;
                restored.fslot = u16::MAX;
                restored.stuck = 0;
                restored.ox = actor.x;
                restored.oy = actor.y;
                let key = match row.order {
                    O_BUILD => self.clearance_build_target(&restored).and_then(|target| {
                        if target.built >= 1.0 { return Some(row.key); }
                        self.prepare_build_routes(Self::building_bounds(&target));
                        let key = self.placement_build_key(&restored);
                        (key != NONE).then_some(key)
                    }),
                    O_GATHER => self.clearance_gather_key(&restored),
                    _ => None,
                };
                if let Some(key) = key {
                    #[cfg(feature = "ee2-rect277-gather-resume")]
                    let restart = self.world0_restart(index, &restored);
                    #[cfg(feature = "ee2-rect277-gather-resume")]
                    let waits = restart.is_none() && self.clearance_stock_full(&restored);
                    #[cfg(not(feature = "ee2-rect277-gather-resume"))]
                    let waits = self.clearance_stock_full(&restored);
                    if row.order == O_GATHER && waits { return true; }
                    if row.order == O_BUILD {
                        Self::assign_build_key(&mut restored, key);
                    } else {
                        restored.fkey = key;
                        if restored.q[3] != 0 { restored.q[4] = 0; }
                        #[cfg(feature = "ee2-rect278-food-resume")]
                        if let Some(source) = FoodSource::order(&restored).filter(|_| restart.is_some()) {
                            self.bind_rect_food(&mut restored, source);
                        }
                    }
                    *actor = restored;
                    #[cfg(feature = "ee2-rect277-gather-resume")]
                    if let Some(restart) = restart {
                        #[cfg(feature = "ee2-rect280-farm-crop")]
                        self.start_rect_farm_goal(index, actor);
                        self.install_stock_restart(handle, actor, restart);
                        return true;
                    }
                    self.rect_clearance.remove(&handle);
                } else {
                    self.rect_clearance.get_mut(&handle).unwrap().phase = Phase::Invalid;
                }
                true
            }
            Phase::Invalid => {
                self.rect_clearance.remove(&handle);
                Self::set_idle(actor);
                true
            }
            #[cfg(feature = "ee2-rect277-gather-resume")]
            Phase::StockExtra => {
                let Some(stock) = self.world0_target(row.target) else {
                    self.rect_clearance.remove(&handle);
                    Self::set_idle(actor);
                    return true;
                };
                if self.world0_reservers(row.target) <= stock.limit {
                    self.rect_clearance.remove(&handle);
                }
                false
            }
            #[cfg(feature = "ee2-rect277-gather-resume")]
            Phase::StockEnded | Phase::ForestEnded => {
                self.pop_stock_end(index, handle, actor, row);
                true
            }
            #[cfg(feature = "ee2-rect278-food-resume")]
            Phase::FarmExtra => {
                if self.rect_farm_reservers(actor.team, row.target) <= 1 {
                    #[cfg(feature = "ee2-rect279-farm-active")]
                    if row.flags & 1 != 0 {
                        self.rect_clearance.get_mut(&handle).unwrap().phase = Phase::FarmActive;
                        self.rect_clearance.get_mut(&handle).unwrap().flags = 0;
                        return false;
                    }
                    self.rect_clearance.remove(&handle);
                }
                false
            }
            #[cfg(feature = "ee2-rect279-farm-active")]
            Phase::FarmActive => false,
            #[cfg(feature = "ee2-rect278-food-resume")]
            Phase::FoodEnded => {
                self.rect_clearance.remove(&handle);
                #[cfg(feature = "ee2-rect280-farm-crop")]
                self.clear_rect_farm_goal(index);
                if let Some(next) = row.food_next() {
                    let mut restored = *actor;
                    if self.bind_rect_food(&mut restored, next) {
                        if let Some(restart) = self.rect_food_restart(index, &restored) {
                            *actor = restored;
                            #[cfg(feature = "ee2-rect280-farm-crop")]
                            self.start_rect_farm_goal(index, actor);
                            self.install_stock_restart(handle, actor, restart);
                            return true;
                        }
                    }
                }
                actor.otgt = NONE;
                actor.q[3] = 0;
                actor.q[4] = 0;
                Self::set_idle(actor);
                true
            }
        }
    }

    pub(crate) fn rect_clearance_after_update(&mut self, index: usize, actor: &mut Ent) {
        let handle = self.h(index);
        let Some(row) = self.rect_clearance.get_mut(&handle) else { return; };
        if row.phase.gather() {
            if !row.matches_gather(actor) {
                self.rect_clearance.remove(&handle);
            }
            return;
        }
        if actor.order != O_IDLE {
            return;
        }
        row.phase = if actor.vx == 0.0 && actor.vy == 0.0 { Phase::Complete } else { Phase::Settling };
        let before = &self.ents[index];
        actor.order = O_MOVE;
        actor.ox = before.ox;
        actor.oy = before.oy;
    }

    fn clearance_build_target(&self, actor: &Ent) -> Option<Ent> {
        let target = self.ents[self.get(actor.otgt)?];
        (target.team == actor.team && target.hp > 0.0 && crate::defs::DEFS[target.def as usize].building)
            .then_some(target)
    }

    fn clearance_gather_key(&self, actor: &Ent) -> Option<u32> {
        let kind = u8::try_from(actor.q[0]).ok()?;
        if kind == R_NONE || !self.citizen_can_gather(actor.team, kind) { return None; }
        if actor.otgt != NONE {
            let farm = &self.ents[self.get(actor.otgt)?];
            return (farm.team == actor.team && farm.def == FARM && farm.hp > 0.0 && farm.built == 1.0)
                .then(|| self.key_for(LAND, farm.x, farm.y));
        }
        if actor.q[3] != 0 {
            let target = self.world0_gather_order(actor)?;
            return (target.kind == kind && (target.remaining > 0.0 || actor.carry > 0.0))
                .then(|| self.key_for(LAND, target.point[0], target.point[1]));
        }
        let tile = actor.otile as usize;
        if *self.map.res.get(tile)? != kind && actor.carry == 0.0 { return None; }
        let [x, y] = self.map.tile_center(tile);
        Some(self.key_for(LAND, x, y))
    }

    fn clearance_stock_full(&self, actor: &Ent) -> bool {
        actor.otgt == NONE && self.world0_gather_order(actor).is_some_and(|target| {
            u32::from(target.kind) == actor.q[0] && self.citizen_can_gather(actor.team, target.kind)
                && (target.remaining > 0.0 || actor.carry > 0.0)
                && self.world0_reservers(actor.q[3]) >= target.limit
        })
    }

    pub(crate) fn rect_clearance_section(&self) -> Option<Vec<u8>> {
        if self.rect_clearance.is_empty() { return None; }
        let mut bytes = TAG.to_vec();
        bytes.extend((self.rect_clearance.len() as u32).to_le_bytes());
        for (&handle, &row) in &self.rect_clearance {
            for word in row.words(handle) { bytes.extend(word.to_le_bytes()); }
        }
        Some(bytes)
    }

    pub(crate) fn restore_rect_clearance(&mut self, bytes: &[u8], high: usize) -> Option<()> {
        let (count, rows) = bytes.strip_prefix(TAG)?.split_at_checked(4)?;
        let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
        if count == 0 || count > high || rows.len() != count * 16
            || self.rect_ai_clock.is_none() || !self.supports_rectangular_city_foundations()
        { return None; }
        let mut previous = None;
        for raw in rows.chunks_exact(16) {
            let [handle, meta, target, key] =
                std::array::from_fn(|i| u32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap()));
            let [phase, order, owner, flags] = meta.to_le_bytes();
            let phase = match phase {
                0 => Phase::Moving, 1 => Phase::Settling, 2 => Phase::Complete, 3 => Phase::Invalid,
                #[cfg(feature = "ee2-rect277-gather-resume")]
                4 => Phase::StockExtra,
                #[cfg(feature = "ee2-rect277-gather-resume")]
                5 => Phase::StockEnded,
                #[cfg(feature = "ee2-rect277-gather-resume")]
                6 => Phase::ForestEnded,
                #[cfg(feature = "ee2-rect278-food-resume")]
                7 => Phase::FarmExtra,
                #[cfg(feature = "ee2-rect278-food-resume")]
                8 => Phase::FoodEnded,
                #[cfg(feature = "ee2-rect279-farm-active")]
                9 => Phase::FarmActive,
                _ => return None,
            };
            let index = self.get(handle)?;
            let actor = &self.ents[index];
            let tiles = self.map.res.len();
            if index >= high || previous.is_some_and(|last| handle <= last) || actor.team != owner
                || actor.def != WORKER || actor.hp <= 0.0
                || actor.stuck == BOARDED || actor.fkey as usize >= tiles
                || !self.map.layout.contains_point(actor.ox, actor.oy)
            { return None; }
            let row = Clearance { phase, order, owner, flags, target, key };
            #[cfg(feature = "ee2-rect279-farm-active")]
            if phase == Phase::FarmActive {
                if order != O_GATHER || !row.matches_gather(actor) || actor.q[1] != 0 || flags != 0 || key != 0
                    || !self.archive_food_source(FoodSource::Farm(target), high)
                    || self.rect_clearance.values().any(|other| other.active_farm() == Some(target))
                { return None; }
                self.rect_clearance.insert(handle, row);
                previous = Some(handle);
                continue;
            }
            #[cfg(feature = "ee2-rect278-food-resume")]
            if matches!(phase, Phase::FarmExtra | Phase::FoodEnded) {
                if order != O_GATHER || !row.matches_gather(actor) || actor.q[1] > 1 || actor.q[4] != 0
                    || !self.archive_food_source(row.food_source(), high)
                    || (phase == Phase::FarmExtra && (!Self::valid_farm_extra_flags(flags) || key != 0))
                    || (phase == Phase::FoodEnded && (flags & !3 != 0 || actor.vx != 0.0 || actor.vy != 0.0
                        || row.food_next().is_some_and(|next| next == row.food_source() || !self.archive_food_source(next, high))))
                { return None; }
                #[cfg(feature = "ee2-rect279-farm-active")]
                if row.active_farm().is_some_and(|farm| actor.q[1] != 0
                    || self.rect_clearance.values().any(|other| other.active_farm() == Some(farm)))
                { return None; }
                self.rect_clearance.insert(handle, row);
                previous = Some(handle);
                continue;
            }
            if flags != 0 || actor.otgt != NONE { return None; }
            #[cfg(feature = "ee2-rect277-gather-resume")]
            if phase.stock() {
                if order != O_GATHER || actor.order != O_GATHER || target == 0 || actor.q[3] != target
                    || self.world0_target(target).is_none()
                    || (phase != Phase::StockExtra && (actor.q[4] != 0 || actor.vx != 0.0 || actor.vy != 0.0))
                    || (phase == Phase::StockExtra && key != 0)
                    || (phase == Phase::StockEnded && key != 0 && (key == target || self.world0_target(key)
                        .is_none_or(|stock| u32::from(stock.kind) != actor.q[0])))
                    || (phase == Phase::ForestEnded && (actor.q[0] != u32::from(crate::map::R_WOOD)
                        || self.rect_forest.as_ref().and_then(|forest| forest.replay.promoted.get(key as usize))
                            .is_none_or(Option::is_none)))
                { return None; }
                self.rect_clearance.insert(handle, row);
                previous = Some(handle);
                continue;
            }
            if actor.order != O_MOVE || !matches!(order, O_BUILD | O_GATHER) || key as usize >= tiles
                || (target != NONE && (target & 0xffff) as usize >= self.ents.len())
                || (order == O_BUILD && target == NONE)
                || (order == O_GATHER && target == NONE && actor.otile as usize >= tiles)
                || (matches!(phase, Phase::Complete | Phase::Invalid) && (actor.vx != 0.0 || actor.vy != 0.0))
            { return None; }
            // Older versions trimmed a deleted trailing Farm slot even while
            // its old handle was still linked below the clearance Move.
            #[cfg(feature = "ee2-rect278-food-resume")]
            if order == O_GATHER && target != NONE {
                let slot = (target & 0xffff) as usize;
                if slot >= high && self.ents[slot].gen == (target >> 16) as u16 {
                    self.ents[slot].gen = self.ents[slot].gen.wrapping_add(1);
                }
            }
            self.rect_clearance.insert(handle, row);
            previous = Some(handle);
        }
        Some(())
    }

    pub(crate) fn hash_rect_clearance(&self, mut mix: impl FnMut(u32)) {
        if self.rect_clearance.is_empty() { return; }
        mix(u32::from_le_bytes(*TAG));
        for (&handle, &row) in &self.rect_clearance {
            for word in row.words(handle) { mix(word); }
            let actor = &self.ents[(handle & 0xffff) as usize];
            for word in actor.q { mix(word); }
            for word in [actor.otile, u32::from(actor.prev), actor.fkey, actor.ox.to_bits(),
                actor.oy.to_bits(), actor.vx.to_bits(), actor.vy.to_bits(), actor.rot.to_bits(), actor.prot.to_bits()]
            { mix(word); }
            #[cfg(feature = "ee2-rect277-gather-resume")]
            mix(u32::from(actor.stuck));
        }
    }
}

#[cfg(all(test, feature = "ee2-rect277-gather-resume"))]
mod tests {
    use super::*;
    use crate::{
        game::O_VALLEY_FOREST,
        map::R_WOOD,
        native_valley37::rectangular_world::{spatial_tests::prepare_authenticated_inputs, staged_game},
    };

    #[test]
    #[ignore = "requires explicit local OPENEPOCH_PRIVATE_FIXTURES; no original map data is distributed"]
    fn compressed_replacement_is_promoted_while_the_old_stock_is_reserved() {
        prepare_authenticated_inputs();
        let mut game = staged_game(7, 1).unwrap();
        let stock = 806;
        let target = game.world0_target(stock).unwrap();
        let [x, y] = target.point;
        let key = game.key_for(LAND, x, y);
        let index = game.spawn(WORKER, 0, x, y);
        for _ in 0..target.limit {
            let ordinary = game.spawn(WORKER, 0, x, y);
            let actor = &mut game.ents[ordinary];
            actor.order = O_GATHER;
            actor.q[3] = stock;
        }
        let mut actor = game.ents[index];
        actor.order = O_MOVE;
        actor.otile = game.map.tile_of(x, y) as u32;
        actor.q[0] = u32::from(R_WOOD);
        actor.q[1] = 1;
        actor.q[2] = u32::from(R_WOOD);
        actor.q[3] = stock;
        actor.carry = 15.0;
        game.ents[index] = actor;
        let handle = game.h(index);
        game.rect_clearance.insert(handle, Clearance {
            phase: Phase::Complete, order: O_GATHER, owner: 0, flags: 0, target: NONE, key,
        });

        let first = game.clearance_compressed_stock(&actor, target.point).unwrap();
        assert!(game.rect_clearance_before_update(index, &mut actor));
        game.ents[index] = actor;
        let pending = game.rect_clearance[&handle];
        assert!(pending.phase == Phase::ForestEnded);
        assert_eq!((pending.key, actor.order, game.world0_reservers(stock)),
            (first, O_GATHER, target.limit + 1));
        assert_ne!(game.clearance_compressed_stock(&actor, target.point), Some(first));
        assert!(game.rect_forest_section().is_some());
        assert!(game.rect_clearance_before_update(index, &mut actor));
        game.ents[index] = actor;
        assert_eq!((actor.order, actor.q[3], game.world0_reservers(stock)),
            (O_VALLEY_FOREST, first, target.limit));
        assert_eq!((actor.q[1], actor.q[2], actor.carry), (1, u32::from(R_WOOD), 15.0));
        assert!(!game.rect_clearance.contains_key(&handle));
    }
}
