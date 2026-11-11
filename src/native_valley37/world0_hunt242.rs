//! Direct Citizen hunting for the authored World0 population.
use super::{
    active_game,
    citizen_facing243::citizen_facing_nearest,
    hunt_order241::{self as order, AttackRound, Event, Rounding},
    population234::{SourceUnit, UnitKind},
    population234_attach::AttachedPopulation,
    wildlife_hunting240::{self as native, AnimalProfile, Attacker, FoodStock, Pose, Task},
    world0_resources236::GatherTarget,
};
use crate::{
    cmd::C_RECT_HUNT,
    defs::{Def, LAND, WORKER},
    game::{Ent, Game, BOARDED, NONE, O_RECT_HUNT},
    map::{R_FOOD, R_URANIUM},
    native_attack_clock as clock,
};
use std::collections::BTreeMap;
#[cfg(feature = "ee2-rect253-native-carcass")]
use std::collections::BTreeSet;

const CORPSE_BIT: u32 = 0x0100_0000;
const TAG: &[u8; 4] = b"DVH1";
#[cfg(feature = "ee2-rect244-hunt-visual")]
const VISUAL_TAG: &[u8; 4] = b"DVH2";
#[cfg(feature = "ee2-rect253-native-carcass")]
const CARCASS_TAG: &[u8; 4] = b"DVH3";
#[cfg(feature = "ee2-rect253-native-carcass")]
const QUEUED_GATHER: u32 = 1 << 8;

#[derive(Clone, Copy)]
pub(super) struct WildlifeState {
    pub(super) hp: f32,
    stock: f32,
    corpse: u32,
}

fn food(profile: AnimalProfile) -> Option<FoodStock> {
    profile.corpse?.food
}

fn hunter(actor: &Ent) -> bool {
    actor.alive && actor.hp > 0.0 && actor.def == WORKER && actor.stuck != BOARDED
}

pub(super) fn hunt_target(actor: &Ent) -> Option<u32> {
    (hunter(actor) && actor.order == O_RECT_HUNT).then_some(actor.q[3])
}

impl AttachedPopulation {
    pub(super) fn hunt_pose(&self, unit: &SourceUnit) -> Pose {
        #[cfg(feature = "ee2-rect250-animals")]
        if let Some(pose) = self.animals.as_ref().and_then(|animals| animals.pose(unit.packed_id)) { return pose; }
        #[cfg(feature = "ee2-rect246-deer-live")]
        if let Some(state) = self.deer_motion.get(&unit.packed_id) { return state.pose; }
        Pose { xyz: unit.xyz, yaw: unit.yaw }
    }

    fn animal_profile(&self, unit: &SourceUnit) -> Option<AnimalProfile> {
        if unit.kind != UnitKind::LiveAnimal { return None; }
        let name = self.source().types().get(usize::from(unit.type_index))?;
        native::source_profile(name)
    }

    fn animal_hp(&self, unit: &SourceUnit, profile: AnimalProfile) -> f32 {
        self.wildlife.get(&unit.packed_id).map_or(f32::from(profile.hp), |state| state.hp)
    }

    #[cfg(feature = "ee2-rect253-native-carcass")]
    fn native_carcass_cursor(&self) -> Option<u32> {
        self.animals.as_ref()?.native_carcass_cursor()
    }

    fn corpse_source(&self, corpse: u32) -> Option<u32> {
        if corpse == 0 { return None; }
        #[cfg(feature = "ee2-rect253-native-carcass")]
        if self.native_carcass_cursor().is_some() {
            return self.wildlife.iter().find_map(|(&id, state)| (state.corpse == corpse).then_some(id));
        }
        let id = corpse ^ CORPSE_BIT;
        (self.wildlife.get(&id)?.corpse == corpse).then_some(id)
    }

    pub(super) fn hunt_corpse_target(&self, corpse: u32) -> Option<GatherTarget> {
        let id = self.corpse_source(corpse)?;
        let state = self.wildlife.get(&id)?;
        let unit = self.unit(id)?;
        let stock = food(self.animal_profile(unit)?)?;
        let [x, y, _] = self.hunt_pose(unit).xyz;
        Some(GatherTarget {
            point: [x, y], kind: R_FOOD, remaining: state.stock,
            limit: usize::from(stock.max_workers), exhaustible: stock.exhaustible,
        })
    }

    #[cfg(feature = "ee2-rect278-food-resume")]
    pub(super) fn food_corpse_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.wildlife.values().filter_map(|state| (state.corpse != 0).then_some(state.corpse))
    }

    pub(super) fn take_hunt_corpse(&mut self, corpse: u32, amount: f32) -> Option<bool> {
        let id = self.corpse_source(corpse)?;
        let state = self.wildlife.get_mut(&id)?;
        let remaining = f64::from(state.stock) - f64::from(amount);
        state.stock = if remaining < 1.0 { 0.0 } else { remaining as f32 };
        Some(remaining >= 1.0)
    }
}

impl Game {
    fn edible_prey(&self, id: u32) -> Option<(&SourceUnit, AnimalProfile, f32, f32)> {
        let population = self.rect_population.as_ref()?;
        let unit = population.unit(id)?;
        let profile = population.animal_profile(unit)?;
        food(profile)?;
        let name = &population.source().types()[usize::from(unit.type_index)];
        let radius = order::edible_land_radius(name)?;
        let hp = population.animal_hp(unit, profile);
        (hp > 0.0).then_some((unit, profile, hp, radius))
    }

    fn huntable_now(&self, team: u8, id: u32) -> Option<[f32; 2]> {
        if self.net || usize::from(team) >= self.nplayers { return None; }
        let (unit, _, _, _) = self.edible_prey(id)?;
        let [x, y, _] = self.rect_population.as_ref()?.hunt_pose(unit).xyz;
        (self.real_fog(team, self.map.tile_of(x, y)) == 255).then_some([x, y])
    }

    /// Compatibility planner candidates, admitted through the ordinary hunt
    /// command's live/edible/current-sight predicate. Actual attacks, source RNG,
    /// fresh corpse allocation, queued Gather and cargo returns stay unchanged.
    #[cfg(feature = "ee2-rect291-ai-scout")]
    pub(crate) fn nearest_rect_ai_prey(&self, worker: usize, reach_squared: f32) -> Option<(u32, f32)> {
        let actor = self.ents[worker];
        let population = self.rect_population.as_ref()?;
        let denied = self.denied_border_territories(&actor);
        let component = self.map.land_component(self.map.tile_of(actor.x, actor.y), denied);
        (0..population.unit_count()).filter_map(|slot| {
            let unit = population.unit_by_slot(slot)?;
            let id = unit.packed_id;
            let [x, y] = self.huntable_now(actor.team, id)?;
            let (_, profile, _, _) = self.edible_prey(id)?;
            let assigned = self.ents[..self.high].iter().filter(|other|
                other.team == actor.team && hunt_target(other) == Some(id)).count();
            if assigned >= usize::from(food(profile)?.max_workers) { return None; }
            let distance = (x - actor.x).powi(2) + (y - actor.y).powi(2);
            let tile = self.map.tile_of(x, y);
            (distance <= reach_squared && self.map.pass[0][tile] && component[tile] != 255
                && self.map.permits_territory(tile, denied)).then_some((id, distance))
        }).min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
    }

    pub(crate) fn order_world0_hunt(&mut self, team: u8, id: u32, workers: &[usize]) {
        let Some([x, y]) = self.huntable_now(team, id) else { return; };
        let key = self.key_for(LAND, x, y);
        for &index in workers {
            let actor = &self.ents[index];
            if !hunter(actor) || actor.team != team || hunt_target(actor) == Some(id) { continue; }
            #[cfg(feature = "ee2-rect276-clearance-resume")]
            self.cancel_rect_clearance(index);
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            self.clear_rect_hunt_visual(index);
            let actor = &mut self.ents[index];
            let carried = if actor.carry <= 0.0 { 0 }
                else if actor.q[2] != 0 { actor.q[2] } else { actor.q[0] };
            actor.order = O_RECT_HUNT;
            actor.otgt = NONE;
            actor.tgt = NONE;
            actor.ox = x;
            actor.oy = y;
            // Hunt has no tile order. Preserve q[1] as the ordinary cargo-return flag.
            actor.otile = 0;
            actor.fkey = key;
            actor.fslot = u16::MAX;
            actor.stuck = 0;
            actor.prev = 0;
            actor.q = [carried, 0, carried, id, 0];
            clock::stop(&mut self.attack_clock[index].flags);
        }
    }

    pub(crate) fn do_world0_hunt(&mut self, index: usize, actor: &mut Ent, def: &Def,
        want: &mut (f32, f32), moving: &mut bool) -> Option<f32>
    {
        let Some(id) = hunt_target(actor) else { Self::set_idle(actor); return None; };
        let Some((unit, profile, hp, radius)) = self.edible_prey(id) else {
            #[cfg(feature = "ee2-rect253-native-carcass")]
            if actor.q[4] & QUEUED_GATHER != 0 {
                let corpse = self.rect_population.as_ref().unwrap().wildlife[&id].corpse;
                self.ents[index] = *actor;
                self.order_world0_carcass_followup(actor.team, corpse, index);
                *actor = self.ents[index];
            }
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            self.clear_rect_hunt_visual(index);
            if actor.order != O_RECT_HUNT { return None; }
            Self::set_idle(actor);
            return None;
        };
        let pose = self.rect_population.as_ref().unwrap().hunt_pose(unit);
        let [x, y, _] = pose.xyz;
        #[cfg(feature = "ee2-rect246-deer-live")]
        if actor.ox.to_bits() != x.to_bits() || actor.oy.to_bits() != y.to_bits() {
            let previous_tile = self.map.tile_of(actor.ox, actor.oy);
            if self.map.tile_of(x, y) != previous_tile {
                actor.fkey = self.key_for(LAND, x, y);
                actor.fslot = u16::MAX;
            }
            actor.ox = x;
            actor.oy = y;
        }
        let flags = actor.q[4] as u8;
        let mut round = AttackRound {
            next: f32::from_bits(actor.otile), in_attack: flags & clock::IN_ATTACK != 0,
            started: flags & clock::STARTED != 0, executed: flags & clock::EXECUTED != 0,
        };
        let position = [actor.x, actor.y];
        let in_range = order::ground_reach(position, [x, y], radius,
            order::CITIZEN_ATTACK_RANGE, Rounding::Nearest);
        if !round.started && !round.in_attack && !in_range {
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            self.clear_rect_hunt_visual(index);
            *want = self.steer(actor, def, x, y);
            *moving = true;
            return None;
        }
        let face = (y - actor.y).atan2(x - actor.x);
        let aimed = in_range && citizen_facing_nearest(position, [x, y], actor.rot);
        let event = order::step_ready(&mut round, self.sim_elapsed(), order::CITIZEN_RELOAD,
            order::CITIZEN_APPLY_TIME, 1.0, aimed, Rounding::Nearest);
        if !round.in_attack && round.next == 0.0 { return Some(face); }
        actor.otile = round.next.to_bits();
        actor.q[4] = u32::from(clock::INIT
            | u8::from(round.in_attack) * clock::IN_ATTACK
            | u8::from(round.started) * clock::STARTED
            | u8::from(round.executed) * clock::EXECUTED);
        if event == Event::Start {
            // Attack vtable +80 advances the shared source stream before Play(Attack1).
            let _sample = self.source_birth.rng.next_word15();
            #[cfg(feature = "ee2-rect244-hunt-visual")]
            self.begin_rect_hunt_visual(index, _sample);
        }
        if event != Event::Fire { return Some(face); }

        #[cfg(feature = "ee2-rect246-deer-live")]
        self.observe_rect_deer_hit(id, self.h(index), hp, f32::from(profile.hp));
        let hp = (hp - order::CITIZEN_DAMAGE).max(0.0);
        let stock = if hp == 0.0 {
            native::registered_death(profile, pose).and_then(|corpse| corpse.corpse_type.food)
                .map_or(0.0, |food| f32::from(food.amount))
        } else { 0.0 };
        if hp != 0.0 {
            self.rect_population.as_mut().unwrap().wildlife.insert(id, WildlifeState { hp, stock, corpse: 0 });
        }
        if hp == 0.0 {
            #[cfg(feature = "ee2-rect253-native-carcass")]
            let native_carcass = self.rect_population.as_ref().unwrap().native_carcass_cursor().is_some();
            #[cfg(not(feature = "ee2-rect253-native-carcass"))]
            let native_carcass = false;
            #[cfg(feature = "ee2-rect246-deer-live")]
            let allocated = self.finish_rect_deer(id);
            #[cfg(not(feature = "ee2-rect246-deer-live"))]
            let allocated: Option<u32> = None;
            let corpse = if native_carcass { allocated.unwrap_or(0) } else { CORPSE_BIT | id };
            let stock = if corpse == 0 { 0.0 } else { stock };
            self.rect_population.as_mut().unwrap().wildlife.insert(id, WildlifeState { hp, stock, corpse });
            self.ents[index] = *actor;
            #[cfg(feature = "ee2-rect253-native-carcass")]
            if native_carcass {
                for i in 0..self.high {
                    let worker = &self.ents[i];
                    if hunt_target(worker) == Some(id) && self.carcass_followup_admissible(worker, corpse, [x, y]) {
                        self.ents[i].q[4] |= QUEUED_GATHER;
                    }
                }
                *actor = self.ents[index];
                return Some(face);
            }
            let attackers: Vec<_> = self.ents[..self.high].iter().enumerate()
                .filter(|(_, actor)| hunt_target(actor) == Some(id))
                .map(|(i, actor)| Some(Attacker {
                    handle: self.h(i), current: Some(Task { tag: native::ATTACK_TAG, target: id }),
                    has_gather_ability: self.citizen_can_gather(actor.team, R_FOOD),
                })).collect();
            for (handle, request) in native::corpse_gather_requests(id, corpse, &attackers) {
                if let Some(i) = self.get(handle) {
                    self.order_world0_resource(self.ents[i].team, request.target, &[i]);
                }
            }
            for other in &mut self.ents[..self.high] {
                if hunt_target(other) == Some(id) { Self::set_idle(other); }
            }
            *actor = self.ents[index];
        }
        Some(face)
    }

    pub(super) fn world0_hunt_word(&self, unit: &SourceUnit, field: u32) -> u32 {
        let population = self.rect_population.as_ref().unwrap();
        let profile = population.animal_profile(unit);
        let state = population.wildlife.get(&unit.packed_id);
        match field {
            13 => profile.map_or(u32::MAX, |p| population.animal_hp(unit, p).to_bits()),
            14 => state.map_or(0, |state| state.corpse),
            15 => state.map_or(0, |state| state.stock.to_bits()),
            16 => u32::from(profile.is_some_and(|p| food(p).is_some() && population.animal_hp(unit, p) > 0.0)),
            _ => u32::MAX,
        }
    }

    pub(crate) fn wildlife_section(&self) -> Option<Vec<u8>> {
        let population = self.rect_population.as_ref()?;
        if population.wildlife.is_empty() && !self.ents[..self.high].iter().any(|e| hunt_target(e).is_some()) {
            return None;
        }
        let mut bytes = Vec::with_capacity(8 + 12 * population.wildlife.len());
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        let visuals: Vec<_> = self.rect_hunt_visual_rows().collect();
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        let tag = if visuals.is_empty() { TAG } else { VISUAL_TAG };
        #[cfg(not(feature = "ee2-rect244-hunt-visual"))]
        let tag = TAG;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let native_wire = population.native_carcass_cursor().is_some();
        #[cfg(not(feature = "ee2-rect253-native-carcass"))]
        let native_wire = false;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let tag = if native_wire { CARCASS_TAG } else { tag };
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&(population.wildlife.len() as u32).to_le_bytes());
        for (&id, state) in &population.wildlife {
            for word in [id, state.hp.to_bits(), state.stock.to_bits()] { bytes.extend_from_slice(&word.to_le_bytes()); }
            if native_wire { bytes.extend_from_slice(&state.corpse.to_le_bytes()); }
        }
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        if !visuals.is_empty() {
            bytes.extend_from_slice(&(visuals.len() as u32).to_le_bytes());
            for row in visuals {
                for word in row { bytes.extend_from_slice(&word.to_le_bytes()); }
            }
        }
        Some(bytes)
    }

    pub(crate) fn restore_wildlife(&mut self, bytes: &[u8]) -> Option<()> {
        let population = self.rect_population.as_mut()?;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let native_cursor = population.native_carcass_cursor();
        #[cfg(not(feature = "ee2-rect253-native-carcass"))]
        let native_cursor: Option<u32> = None;
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        let with_visuals = bytes.starts_with(VISUAL_TAG);
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        let tag = if with_visuals { VISUAL_TAG } else { TAG };
        #[cfg(not(feature = "ee2-rect244-hunt-visual"))]
        let tag = TAG;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let tag = if native_cursor.is_some() { CARCASS_TAG } else { tag };
        let (count, body) = bytes.strip_prefix(tag)?.split_at_checked(4)?;
        let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
        if count > population.unit_count() { return None; }
        let width = if native_cursor.is_some() { 16 } else { 12 };
        let (records, tail) = body.split_at_checked(count.checked_mul(width)?)?;
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        let visuals = if with_visuals || native_cursor.is_some() && !tail.is_empty() { super::hunt_visual244::read_visual_rows(tail)? }
            else if tail.is_empty() { BTreeMap::new() } else { return None; };
        #[cfg(not(feature = "ee2-rect244-hunt-visual"))]
        if !tail.is_empty() { return None; }
        let mut states = BTreeMap::new();
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let mut allocated = BTreeSet::new();
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let initial_cursor = if native_cursor.is_some() {
            population.source().units().iter().map(|unit| unit.local_id()).max()? + 3 * self.nplayers as u32
        } else { 0 };
        for row in records.chunks_exact(width) {
            let id = u32::from_le_bytes(row[..4].try_into().ok()?);
            let hp = f32::from_le_bytes(row[4..8].try_into().ok()?);
            let stock = f32::from_le_bytes(row[8..12].try_into().ok()?);
            let profile = population.animal_profile(population.unit(id)?)?;
            let maximum = f32::from(food(profile)?.amount);
            if states.last_key_value().is_some_and(|(&prev, _)| prev >= id)
                || hp.to_bits() != 0 && (!(hp > 0.0 && hp < f32::from(profile.hp))
                    || hp % order::CITIZEN_DAMAGE != 0.0)
                || stock.to_bits() != 0 && (hp != 0.0 || !(1.0..=maximum).contains(&stock))
            {
                return None;
            }
            let corpse = if native_cursor.is_some() {
                u32::from_le_bytes(row[12..16].try_into().ok()?)
            } else if hp == 0.0 { CORPSE_BIT | id } else { 0 };
            #[cfg(feature = "ee2-rect253-native-carcass")]
            if let Some(cursor) = native_cursor {
                if corpse == 0 {
                    if stock != 0.0 || hp == 0.0 && cursor != super::animal_live250::MAX_NATIVE_LOCAL_ID { return None; }
                } else if hp != 0.0 || corpse <= initial_cursor || corpse > cursor || corpse <= id
                    || population.unit(corpse).is_some() || !allocated.insert(corpse) { return None; }
            }
            states.insert(id, WildlifeState { hp, stock, corpse });
        }
        population.wildlife = states;
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        { population.hunt_visuals = visuals; }
        Some(())
    }

    pub(crate) fn valid_wildlife_orders(&self, high: usize, has_section: bool) -> bool {
        #[cfg(feature = "ee2-rect244-hunt-visual")]
        if !self.valid_rect_hunt_visual_rows(high) { return false; }
        let mut has_order = false;
        for actor in self.ents[..high].iter().filter(|e| e.alive && e.order == O_RECT_HUNT) {
            has_order = true;
            let Some(id) = hunt_target(actor) else { return false; };
            let Some(population) = self.rect_population.as_ref() else { return false; };
            let Some(unit) = population.unit(id) else { return false; };
            let [x, y, _] = population.hunt_pose(unit).xyz;
            let alive = self.edible_prey(id).is_some();
            let deadline = f32::from_bits(actor.otile);
            let flags = actor.q[4];
            #[cfg(feature = "ee2-rect253-native-carcass")]
            let pending = {
                let queued = flags & QUEUED_GATHER != 0;
                if alive && queued { return false; }
                if !alive {
                    let Some(state) = population.wildlife.get(&id) else { return false; };
                    if population.native_carcass_cursor().is_none() || state.hp != 0.0
                        || queued && !self.carcass_followup_admissible(actor, state.corpse, [x, y]) { return false; }
                }
                if queued { QUEUED_GATHER } else { 0 }
            };
            #[cfg(not(feature = "ee2-rect253-native-carcass"))]
            let pending = if alive { 0 } else { return false; };
            if !has_section || actor.tgt != NONE || actor.otgt != NONE || actor.prev != 0
                || alive && (actor.ox.to_bits() != x.to_bits() || actor.oy.to_bits() != y.to_bits())
                || actor.q[0] != actor.q[2] || actor.q[2] > u32::from(R_URANIUM) || actor.q[1] != 0
                || !deadline.is_finite() || deadline < 0.0
                || ![0, 0x85, 0x87, 0x86, 0x82].contains(&(flags ^ pending))
                || ((flags ^ pending) == 0) != (actor.otile == 0)
            {
                return false;
            }
        }
        let has_mutations = self.rect_population.as_ref().is_some_and(|p| !p.wildlife.is_empty());
        has_section == (has_order || has_mutations)
    }

    pub(crate) fn hash_world0_wildlife(&self, mix: &mut impl FnMut(u32)) {
        if let Some(bytes) = self.wildlife_section() {
            for word in bytes.chunks_exact(4) { mix(u32::from_le_bytes(word.try_into().unwrap())); }
        }
        for (index, actor) in self.ents[..self.high].iter().enumerate() {
            if hunt_target(actor).is_some() {
                mix(self.h(index));
                mix(actor.otile);
                for &word in &actor.q { mix(word); }
            }
        }
    }
}

#[cfg(feature = "ee2-rect253-native-carcass")]
#[no_mangle]
pub extern "C" fn native_rect_carcass_pending_word(handle: u32) -> u32 {
    let pending = || {
        let game = active_game()?;
        let actor = &game.ents[game.get(handle)?];
        if actor.q[4] & QUEUED_GATHER == 0 { return None; }
        let id = hunt_target(actor)?;
        Some(game.rect_population.as_ref()?.wildlife.get(&id)?.corpse)
    };
    pending().unwrap_or(u32::MAX)
}

#[no_mangle]
pub extern "C" fn cmd_rect_hunt(id: u32) -> u32 {
    if active_game().is_none() { return 0; }
    let game = crate::g();
    if game.solo_resigned || game.winner != 0 || game.huntable_now(game.local, id).is_none() { return 0; }
    let workers: Vec<_> = crate::selected(game).into_iter()
        .filter(|&i| hunter(&game.ents[i])).collect();
    if workers.is_empty() { return 0; }
    let handles = crate::handles(game, &workers);
    game.submit(C_RECT_HUNT, 0.0, 0.0, id as i32, &handles);
    1
}

#[no_mangle]
pub extern "C" fn render_rect_hunt_elapsed(handle: u32, render_time: f32) -> f32 {
    let Some(game) = active_game() else { return f32::NAN; };
    let Some(index) = game.get(handle) else { return f32::NAN; };
    let actor = &game.ents[index];
    if hunt_target(actor).is_none() || actor.q[4] & u32::from(clock::STARTED) == 0
        || !render_time.is_finite()
    {
        return f32::NAN;
    }
    let reload = f64::from(order::CITIZEN_RELOAD);
    let next = f64::from(f32::from_bits(actor.otile));
    (f64::from(render_time) + reload - next).clamp(0.0, reload) as f32
}
