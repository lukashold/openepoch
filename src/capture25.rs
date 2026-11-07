//! Building-capture orders and actor replacement for UP skirmish.

use super::*;
use crate::capture_animation26::{self as animation, Pose};
use crate::capture_math25::{self as native, CaptureFactors, CaptureTarget, CaptureTribute};
use std::collections::BTreeMap;

const NO_OWNER: u8 = u8::MAX;
const MAGIC: u32 = u32::from_le_bytes(*b"CAP1");
const VERSION: u32 = 1;
const VISUAL_VERSION: u32 = 2;
const CLOCK_VERSION: u32 = 3;
const CLOCK_VISUAL_VERSION: u32 = 4;
const NATURAL_ORDER: [usize; 9] = [0, 1, 3, 4, 5, 2, 6, 7, 8];

#[derive(Clone, Copy)]
struct Order {
    target: u32,
    team: u8,
    next: f32,
    flags: u8,
    pose: Option<Pose>,
    pose_at: f32,
}

#[derive(Clone, Copy)]
struct Target {
    progress: f32,
    owner: u8,
    count: u32,
}

impl Default for Target {
    fn default() -> Self { Self { progress: 0.0, owner: NO_OWNER, count: 0 } }
}

#[derive(Clone, Copy)]
struct Gift {
    previous: u8,
    tick: u32,
    at: f32,
}

impl Gift {
    fn active(self, tick: u32, clock: Option<crate::source_clock38::SourceClock>) -> bool {
        if let Some(clock) = clock {
            self.at > 0.0 && f64::from(clock.elapsed) < f64::from(self.at) + 60.0
        } else {
            tick.saturating_sub(self.tick) < 1200
        }
    }
}

pub(crate) struct CaptureSave<'a> {
    fields: &'a [u8],
    visual: bool,
    clock: bool,
}

fn archive_version(visual: bool, clock: bool) -> u32 {
    match (visual, clock) {
        (false, false) => VERSION,
        (true, false) => VISUAL_VERSION,
        (false, true) => CLOCK_VERSION,
        (true, true) => CLOCK_VISUAL_VERSION,
    }
}

#[derive(Default)]
pub(crate) struct State {
    orders: BTreeMap<u32, Order>,
    targets: BTreeMap<u32, Target>,
    gifts: BTreeMap<u32, Gift>,
}

impl State {
    fn remove_order(&mut self, handle: u32) {
        let Some(order) = self.orders.remove(&handle) else { return; };
        if order.flags != 0 {
            let target = self.targets.get_mut(&order.target).expect("Registered capture target");
            target.count -= 1;
            if target.count == 0 { target.owner = NO_OWNER; }
            if target.count == 0 && target.progress == 0.0 { self.targets.remove(&order.target); }
        }
    }

    fn empty(&self) -> bool { self.orders.is_empty() && self.targets.is_empty() && self.gifts.is_empty() }

    fn fields(&self, with_visual: bool, with_clock: bool, mut write: impl FnMut(u32)) {
        for value in [self.orders.len(), self.targets.len(), self.gifts.len()] { write(value as u32); }
        for (&handle, order) in &self.orders {
            for value in [handle, order.target, order.team.into(), order.next.to_bits(), order.flags.into()] { write(value); }
            if with_visual {
                write(order.pose.map_or(0, Pose::packed));
                write(order.pose.map_or(0, |pose| pose.tick));
                if with_clock { write(order.pose_at.to_bits()); }
            }
        }
        for (&handle, target) in &self.targets {
            write(handle);
            write(target.progress.to_bits());
        }
        for (&handle, gift) in &self.gifts {
            for value in [handle, gift.previous.into(), gift.tick] { write(value); }
            if with_clock { write(gift.at.to_bits()); }
        }
    }

    pub(crate) fn hash(&self, with_visual: bool, with_clock: bool, mut mix: impl FnMut(u32)) {
        if self.empty() { return; }
        mix(MAGIC);
        if with_visual || with_clock { mix(archive_version(with_visual, with_clock)); }
        self.fields(with_visual, with_clock, mix);
    }
}

impl Game {
    pub(crate) fn capture_match_enabled(&self) -> bool { self.capture_enabled && self.winner == 0 }

    pub(crate) fn configure_peer_capture(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.capture.empty() || (!enabled && self.capitols.is_some()) { return false; }
        self.capture_enabled = enabled;
        if !enabled { self.capture_animation_enabled = false; }
        true
    }

    pub(crate) fn configure_peer_capture_animation(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.capture.empty()
            || (enabled && !self.capture_enabled) || (!enabled && self.capitols.is_some()) { return false; }
        self.capture_animation_enabled = enabled;
        true
    }

    pub(crate) fn capture_actor_ready(&self, index: usize) -> bool {
        let actor = self.ents[index];
        self.capture_match_enabled() && actor.hp > 0.0 && native::capture_attacker(actor.def).is_some()
            && self.has_conversion_reserve(actor.def, self.h(index))
    }

    fn has_conversion_reserve(&self, def: DefId, handle: u32) -> bool {
        def != crate::priest42_defs::PRIEST || self.powers.as_ref().and_then(|state| state.actor(handle))
            .is_some_and(|power| f64::from(power.charge) > 0.01)
    }

    fn conversion_target_profile(&self, def: DefId) -> Option<CaptureTarget> {
        native::capture_target(def).or_else(|| self.powers.as_ref().and_then(|_| native::conversion_target(def)))
    }

    pub(crate) fn capture_compatible(&self, index: usize, target: usize) -> bool {
        let source = self.ents[index];
        let victim = self.ents[target];
        if source.def == crate::priest42_defs::PRIEST {
            native::conversion_target(victim.def).is_some() && !self.crown_prevents_conversion(victim.team)
        }
        else { native::capture_target(victim.def).is_some() }
    }

    pub(crate) fn priest_conversion_active(&self, handle: u32) -> bool {
        self.get(handle).is_some_and(|index| {
            let actor = self.ents[index];
            actor.def == crate::priest42_defs::PRIEST && actor.order == O_CAPTURE
                && self.capture.orders.get(&handle).is_some_and(|order| order.flags & crate::native_attack_clock::IN_ATTACK != 0)
        })
    }

    pub(crate) fn capture_allowed(&self, owner: u8, target: usize) -> bool {
        let actor = self.ents[target];
        let hill = actor.def == crate::koth64::HILL;
        if !self.capture_match_enabled() || !actor.alive || actor.built != 1.0 || actor.hp <= 0.0
            || !self.capture_relation_allowed(owner, target) || !self.can_see(owner, target)
            || self.conversion_target_profile(actor.def).is_none() {
            return false;
        }
        if self.imperialism_active(target) { return false; }
        if !hill && native::capture_target(actor.def).is_some()
            && self.owns_completed_wonder(actor.team, crate::wonders61::SPHINX) { return false; }
        let state = self.capture.targets.get(&self.h(target)).copied().unwrap_or_default();
        native::can_join_capture(actor.hp, state.count, Some(state.owner).filter(|&seat| seat != NO_OWNER), owner)
    }

    fn capture_relation_allowed(&self, owner: u8, target: usize) -> bool {
        let actor = self.ents[target];
        if actor.def == crate::koth64::HILL {
            if self.koth.is_none_or(|rule| rule.actor != self.h(target)) { return false; }
            if actor.team == crate::koth64::NATURE { return !self.initial_ceasefire_active(); }
        }
        self.combat_hostile(owner, actor.team)
    }

    pub(crate) fn capture_progress(&self, index: usize) -> Option<f32> {
        self.capture.targets.get(&self.h(index)).filter(|target| target.count > 0).map(|target| target.progress)
    }

    pub(crate) fn has_active_capture_glow(&self) -> bool {
        self.capture.targets.iter().any(|(&handle, target)| target.count > 0 && self.get(handle)
            .is_some_and(|index| native::capture_target(self.ents[index].def).is_some()))
    }

    pub(crate) fn active_capture_glow(&self, handle: u32) -> bool {
        self.get(handle).is_some_and(|index| {
            native::capture_target(self.ents[index].def).is_some() && self.capture_progress(index).is_some()
                && (self.display_visibility() == 2 || self.can_see(self.local, index))
        })
    }

    pub(crate) fn capture_animation_pose(&self, handle: u32) -> Option<Pose> {
        let actor = self.ents[self.get(handle)?];
        if actor.order != O_CAPTURE || !self.capture_animation_enabled { return None; }
        let pose = self.capture.orders.get(&handle)?.pose?;
        let context = animation::context(actor.def, self.players[actor.team as usize].epoch);
        pose.matches_context(context).then_some(pose)
    }

    pub(crate) fn capture_animation_started(&self, handle: u32) -> Option<f32> {
        let pose = self.capture_animation_pose(handle)?;
        if self.source_clock.is_some() { Some(self.capture.orders.get(&handle)?.pose_at) }
        else { Some(pose.tick as f32 * DT) }
    }

    pub(crate) fn order_capture(&mut self, indices: &[usize], target: usize) {
        let object = self.ents[target];
        let target_handle = self.h(target);
        let key = self.key_for(LAND, object.x, object.y);
        for &index in indices {
            let owner = self.ents[index].team;
            if !self.capture_actor_ready(index) || !self.capture_allowed(owner, target) || !self.capture_compatible(index, target) { continue; }
            let handle = self.h(index);
            if self.capture.orders.get(&handle).is_some_and(|order| order.target == target_handle && order.team == owner) {
                continue;
            }
            self.capture.remove_order(handle);
            self.capture.orders.insert(handle, Order {
                target: target_handle, team: owner, next: 0.0, flags: 0, pose: None, pose_at: 0.0,
            });
            self.pack_interrupt(index);
            self.native_stop_attack(index);
            let actor = &mut self.ents[index];
            actor.order = O_CAPTURE;
            actor.otgt = target_handle;
            actor.tgt = NONE;
            actor.fkey = key;
            actor.fslot = u16::MAX;
            actor.stuck = 0;
        }
    }

    fn capture_order_valid(&self, handle: u32, order: Order) -> bool {
        let (Some(index), Some(target)) = (self.get(handle), self.get(order.target)) else { return false; };
        let actor = &self.ents[index];
        actor.order == O_CAPTURE && actor.team == order.team && actor.otgt == order.target
            && self.capture_actor_ready(index) && self.capture_allowed(order.team, target) && self.capture_compatible(index, target)
    }

    pub(crate) fn reconcile_capture_orders(&mut self) {
        if self.capture.orders.is_empty() { return; }
        let invalid: Vec<_> = self.capture.orders.iter().filter_map(|(&handle, &order)|
            (!self.capture_order_valid(handle, order)).then_some(handle)).collect();
        for handle in invalid {
            self.capture.remove_order(handle);
            if let Some(index) = self.get(handle) {
                if self.ents[index].order == O_CAPTURE { Self::set_idle(&mut self.ents[index]); }
            }
        }
    }

    pub(crate) fn forget_capture_actor(&mut self, index: usize) {
        if self.capture.empty() { return; }
        let handle = self.h(index);
        self.capture.remove_order(handle);
        if self.ents[index].order == O_CAPTURE { Self::set_idle(&mut self.ents[index]); }
        self.capture.targets.remove(&handle);
        self.capture.gifts.remove(&handle);
        self.capture.orders.retain(|&actor, order| {
            if order.target != handle { return true; }
            let index = (actor & 0xffff) as usize;
            let entity = &mut self.ents[index];
            if entity.alive && u32::from(entity.gen) == actor >> 16 && entity.order == O_CAPTURE {
                Self::set_idle(entity);
            }
            false
        });
    }

    pub(super) fn tick_capture_state(&mut self) {
        self.reconcile_capture_orders();
        if self.capture.empty() { return; }
        let dt = self.sim_step();
        let entities = &self.ents;
        let powers = self.powers.as_ref();
        let tick = self.tick;
        let living = |handle: u32| {
            let index = (handle & 0xffff) as usize;
            entities.get(index).filter(|e| e.alive && e.hp > 0.0 && u32::from(e.gen) == handle >> 16)
        };
        self.capture.targets.retain(|&handle, target| {
            let Some(actor) = living(handle) else { return false; };
            // OpenEpoch uses generation handles. Adapt the original seat bits
            // and retain the one-second stagger over this map's slot IDs.
            let serial = powers.and_then(|state| state.actor(handle)).map_or(handle & 0xffff, |actor| actor.serial);
            let native_owner = if actor.team == crate::koth64::NATURE { 0 } else { u32::from(actor.team) + 1 };
            let phase = native_owner << 27 | serial;
            if target.count == 0 && native::capture_event_due(tick, phase, dt) {
                target.progress = if native::capture_target(actor.def).is_some() {
                    native::decay_capture_progress(target.progress)
                } else {
                    crate::power42_math::store_toward_zero((f64::from(target.progress) - f64::from(0.1_f32)).max(0.0))
                };
            }
            target.count > 0 || target.progress > 0.0
        });
        let clock = self.source_clock;
        self.capture.gifts.retain(|&handle, gift| living(handle).is_some() && gift.active(tick, clock));
    }

    pub(crate) fn mark_capture_gift(&mut self, index: usize) {
        let entity = self.ents[index];
        if self.capture_enabled && self.tick > 0 && native::capture_target(entity.def).is_some() {
            self.capture.gifts.insert(self.h(index), Gift { previous: entity.team, tick: self.tick, at: self.sim_elapsed() });
        }
    }

    pub(super) fn do_capture(&mut self, index: usize, actor: &mut Ent, def: &Def) -> (Option<f32>, Option<(f32, f32)>) {
        let handle = (u32::from(actor.gen) << 16) | index as u32;
        let Some(target) = self.get(actor.otgt).filter(|&target| self.capture_allowed(actor.team, target)
            && self.capture_compatible(index, target)) else {
            self.capture.remove_order(handle);
            Self::set_idle(actor);
            return (None, None);
        };
        let Some(attack) = native::capture_attacker(actor.def) else { return (None, None); };
        let building = self.ents[target];
        let dx = building.x - actor.x;
        let dy = building.y - actor.y;
        let edge = if DEFS[building.def as usize].building {
            let radius = Self::building_half(&building);
            (dx.abs() - radius).max(0.0).hypot((dy.abs() - radius).max(0.0))
        } else { (dx.hypot(dy) - DEFS[building.def as usize].rad()).max(0.0) };
        let now = self.sim_elapsed();
        let range = if actor.def == crate::priest42_defs::PRIEST {
            self.crown_conversion_range(actor.team, attack.range)
        } else { attack.range };
        let Some(order) = self.capture.orders.get_mut(&handle) else {
            Self::set_idle(actor);
            return (None, None);
        };
        let locked = crate::native_attack_clock::STARTED | crate::native_attack_clock::IN_ATTACK;
        if edge > range + def.rad() && order.flags & locked != locked {
            crate::native_attack_clock::stop(&mut order.flags);
            order.pose = None;
            order.pose_at = 0.0;
            return (None, Some(self.steer(actor, def, building.x, building.y)));
        }
        let target_handle = actor.otgt;
        let state = self.capture.targets.entry(target_handle).or_default();
        if order.flags == 0 {
            state.count += 1;
            state.owner = actor.team;
        }
        let round = if actor.def == crate::priest42_defs::PRIEST {
            crate::native_attack_clock::step_toward_zero(now, attack.reload_seconds,
                attack.impact_seconds, &mut order.next, &mut order.flags)
        } else { crate::native_attack_clock::step(now, attack.reload_seconds,
            attack.impact_seconds, &mut order.next, &mut order.flags) };
        if actor.def != crate::priest42_defs::PRIEST && self.capture_animation_enabled && round == crate::native_attack_clock::Event::Start {
            let context = animation::context(actor.def, self.players[actor.team as usize].epoch);
            let previous = order.pose;
            Pose::apply(&mut order.pose, &mut self.source_birth.rng, context, self.tick);
            if order.pose != previous { order.pose_at = now; }
        }
        actor.cd = (order.next - now).max(0.0);
        if actor.def == crate::priest42_defs::PRIEST && round == crate::native_attack_clock::Event::Start {
            self.attach_priest_effect(handle, crate::power42::FX_CONVERT);
            self.ev(33.0, actor.x, actor.y);
        }
        if round == crate::native_attack_clock::Event::Fire {
            self.capture_impact(index, actor, target, attack);
        }
        (Some(dy.atan2(dx)), None)
    }

    fn capture_impact(&mut self, attacker_index: usize, actor: &mut Ent, index: usize, attack: native::CaptureAttacker) {
        if actor.def == crate::priest42_defs::PRIEST {
            self.priest_conversion_impact(attacker_index, actor, index, attack);
            return;
        }
        let building = self.ents[index];
        let mut profile = native::capture_target(building.def).expect("Admitted capture target");
        let (target_epoch, research, regional) = if building.team == crate::koth64::NATURE {
            (0, 0.0, 0.0)
        } else {
            let research = [5, 65, 125].iter().filter(|&&id| self.has_research(building.team, id)).count() as f32 * 10.0;
            let regional = crate::civ8_traits::projected_starting_percent(self.civilizations[building.team as usize] as i8,
                building.def, "CaptureResistance").unwrap_or(0.0);
            (self.players[building.team as usize].epoch, research, regional)
        };
        if research + regional != 0.0 {
            profile.time_modifier = (f64::from(profile.time_modifier) * f64::from(100.0 + research + regional)
                * f64::from(0.01_f32)) as f32;
        }
        let occupants = building.ncargo;
        let damage = native::capture_counter_damage(profile, occupants);
        if damage > 0.0 && !self.god_mode_for(actor.team) {
            actor.hp = (actor.hp - damage).max(0.0);
            actor.flash = 0.12;
        }
        if actor.hp <= 0.0 { return; }
        if profile.damage_rate > 0.0 && building.hp / self.max_hp(building.team, building.def) > 0.25
            && !self.god_mode_for(building.team) {
            let target = &mut self.ents[index];
            target.hp = (target.hp - profile.damage_rate).max(0.0);
            target.flash = 0.12;
        }
        let killed = self.ents[index].hp <= 0.0;
        if let Some(signals) = &mut self.battle_signals {
            let victim = crate::battle_signals::Actor::new(index, &building);
            let source = crate::battle_signals::Actor::new(attacker_index, actor);
            if signals.hit_capture(self.local, victim, source, killed) { self.ev(15.0, building.x, building.y); }
        }
        if killed { return; }
        let target_effect = if self.capture_owner_city(&building) { native::CITY_CAPTURE_RESISTANCE } else { 1.0 };
        let target_effect = if self.crown_city_protected(&building) { target_effect * 1.3 } else { target_effect };
        let target = self.capture.targets.get_mut(&actor.otgt).expect("Admitted capture round");
        let factors = CaptureFactors {
            epoch_gap: i32::from(self.players[actor.team as usize].epoch) - i32::from(target_epoch),
            contributors: target.count,
            garrison_count: occupants,
            player_factor: 1.0,
            target_effect,
            attacker_effect: 1.0,
        };
        let (progress, done) = native::apply_capture_delta(target.progress, native::capture_delta(attack, profile, factors));
        target.progress = progress;
        if done && self.complete_capture(index, actor.team, profile) { Self::set_idle(actor); }
    }

    fn priest_conversion_impact(&mut self, attacker_index: usize, actor: &mut Ent, index: usize, mut attack: native::CaptureAttacker) {
        let victim = self.ents[index];
        let target_handle = self.h(index);
        let target = native::conversion_target(victim.def).expect("admitted mobile conversion");
        if self.has_research(actor.team, 23) { attack.time_modifier *= 0.9; }
        let research = [54, 114, 174].iter().filter(|&&id| self.has_research(victim.team, id)).count() as f32 * 10.0;
        let regional = if self.civilizations[victim.team as usize] == 6 { 25.0 } else { 0.0 };
        let target_time = target.time_modifier + target.time_modifier * ((research + regional) * 0.01);
        let count = self.capture.targets[&target_handle].count;
        let delta = crate::power42_math::conversion_impact_delta(
            i32::from(self.players[actor.team as usize].epoch) - i32::from(self.players[victim.team as usize].epoch),
            count, attack.time_modifier, target_time, 1.0, 1.0, 1.0, attack.reload_seconds);
        let handle = (u32::from(actor.gen) << 16) | attacker_index as u32;
        let delta = self.spend_unit_reserve(handle, delta);
        if delta > 0.0 { self.attach_priest_effect(target_handle, crate::power42::FX_CONVERTED); }
        let target = self.capture.targets.get_mut(&target_handle).unwrap();
        let done = f64::from(target.progress) + f64::from(delta) >= f64::from(native::CAPTURE_COMPLETE);
        target.progress = crate::power42_math::conversion_progress(target.progress, delta);
        if done && self.replace_converted_actor(index, actor.team, CaptureTribute::None) {
            self.ev(32.0, victim.x, victim.y);
            self.proselytize_near_conversion(actor.team, victim.x, victim.y);
            Self::set_idle(actor);
        } else if self.powers.as_ref().and_then(|state| state.actor(handle))
            .is_some_and(|power| f64::from(power.charge) <= 0.01) {
            self.capture.remove_order(handle);
            Self::set_idle(actor);
        }
    }

    fn proselytize_near_conversion(&mut self, owner: u8, x: f32, y: f32) {
        let effects = self.count_completed_wonders(owner, crate::wonders61::GREAT_PYRAMID);
        for _ in 0..effects {
            // 00803210 rebuilds the candidate vector for every application.
            // This uses the current world grid traversal as the spatial adapter.
            let candidates: Vec<_> = self.grid.items.iter().copied().filter_map(|slot| {
                let index = slot as usize;
                let e = &self.ents[index];
                let dx = f64::from(e.x) - f64::from(x);
                let dy = f64::from(e.y) - f64::from(y);
                (e.alive && e.hp > 0.0 && e.team != owner && e.stuck != BOARDED
                    && native::conversion_target(e.def).is_some() && dx * dx + dy * dy <= 9.0).then(|| self.h(index))
            }).collect();
            for handle in candidates {
                let Some(index) = self.get(handle) else { continue; };
                // The original consumes the roll before it checks NoConversion.
                if crate::wonders61::proselytizing_roll(self.source_birth.rng.next_word15())
                    && !self.crown_prevents_conversion(self.ents[index].team) {
                    self.replace_converted_actor(index, owner, CaptureTribute::None);
                }
            }
        }
    }

    pub(crate) fn priest_conversion_elapsed(&self, handle: u32, render_time: f32) -> Option<f32> {
        let actor = self.ents[self.get(handle)?];
        let order = self.capture.orders.get(&handle)?;
        let locked = crate::native_attack_clock::STARTED | crate::native_attack_clock::IN_ATTACK;
        if actor.def != crate::priest42_defs::PRIEST || actor.order != O_CAPTURE || order.flags & locked != locked { return None; }
        let (start, end) = if self.source_clock.is_some() { (self.sim_previous_elapsed(), self.sim_elapsed()) }
            else { (self.sim_elapsed(), (self.tick as f32 + 1.0) * DT) };
        if !(start..=end).contains(&render_time) { return None; }
        Some(((f64::from(render_time) + 2.0 - f64::from(order.next)) as f32).clamp(0.0, 2.0))
    }

    pub(super) fn capture_owner_city(&self, building: &Ent) -> bool {
        self.ents[..self.high].iter().enumerate().any(|(index, city)| {
            if city.team != building.team { return false; }
            let radius = self.city_ground_radius(index);
            radius > 0.0 && f64::from((city.x - building.x).abs() + (city.y - building.y).abs())
                < f64::from(radius) * f64::from(core::f32::consts::SQRT_2)
        })
    }

    fn capture_tribute(&mut self, handle: u32, building: &Ent, recipient: u8, tribute: CaptureTribute) {
        let same_gift = self.capture.gifts.get(&handle).is_some_and(|gift|
            gift.previous == recipient && gift.active(self.tick, self.source_clock));
        if same_gift { return; }
        let (base, slots): (f32, &[usize]) = match tribute {
            CaptureTribute::Natural(rate) => (rate, &NATURAL_ORDER),
            CaptureTribute::Gold(rate) => (rate, &[4]),
            CaptureTribute::Tech(rate) => (rate, &[9]),
            CaptureTribute::None => return,
        };
        let modified = f64::from(self.wonder_capture_yield(building.team)) - 1.0 + f64::from(base);
        if modified <= f64::from(0.001_f32) { return; }
        let mut rate = modified as f32;
        let area = self.map.terr[tile_of(building.x, building.y)] as usize;
        if self.towner[area] == building.team { rate += 0.05; }
        if self.capture_owner_city(building) { rate += 0.05; }
        for &slot in slots {
            let stock = if slot == 9 { self.research_points[building.team as usize] } else { self.resource(building.team, slot) };
            if stock < if slot == 9 { 10.0 } else { 50.0 } { continue; }
            let biased = f64::from(stock) * f64::from(rate) + 0.5 + 103_079_215_104.0;
            let amount = ((biased.to_bits() >> 16) as i32) as f32;
            if slot == 9 {
                self.research_points[building.team as usize] = (stock - amount).clamp(0.0, 999_999.0);
                self.research_points[recipient as usize] = (self.research_points[recipient as usize] + amount).clamp(0.0, 999_999.0);
            } else {
                self.add_resource(building.team, slot, -amount);
                self.add_resource(recipient, slot, amount);
                self.crown_stolen(recipient, slot, amount);
            }
        }
    }

    fn complete_capture(&mut self, index: usize, capturer: u8, profile: CaptureTarget) -> bool {
        let old = self.ents[index];
        let handle = self.h(index);
        let capitol = self.capitol_capture(handle, old.team, capturer);
        let owner = self.capitol_capture_owner(handle, capturer);
        if !self.replace_converted_actor(index, owner, profile.tribute) { return false; }
        if let Some(capture) = capitol {
            self.finish_capitol_capture(capture, owner);
        } else if old.def == crate::koth64::HILL && old.team != owner {
            if self.events.len() < 3 * 64 {
                let event = 128 + 4 * old.team + owner;
                self.events.extend_from_slice(&[f32::from(event), old.x, old.y]);
            }
        } else if old.team == self.local || owner == self.local {
            self.ev(30.0, old.x, old.y);
        }
        self.wonder_notice(crate::wonders61::Notice::Captured, old.def, owner);
        true
    }

    pub(crate) fn convert_last_clicked(&mut self) -> bool {
        let Some(index) = self.get(self.conversion_click) else { return false; };
        let old = self.ents[index];
        if old.hp <= 0.0 || old.stuck == BOARDED { return false; }
        let tribute = match native::capture_target(old.def) {
            Some(profile) => profile.tribute,
            None if matches!(old.def, 8..=28 | 46 | 57 | 61..=79 | crate::trade30_defs::TRADE_CART | crate::priest42_defs::PRIEST) => CaptureTribute::None,
            None => return false,
        };
        if !self.replace_converted_actor(index, self.local, tribute) { return false; }
        self.ev(31.0, old.x, old.y);
        true
    }

    pub(super) fn replace_converted_actor(&mut self, index: usize, owner: u8, tribute: CaptureTribute) -> bool {
        let old = self.ents[index];
        if self.free.len() <= usize::from(old.ncargo) { return false; }
        if old.ncargo > 0 {
            let mut evacuated = old;
            self.unload(index, &mut evacuated);
            if evacuated.ncargo != 0 { return false; }
            self.ents[index] = evacuated;
        }
        let old_handle = self.h(index);
        if old.team != owner {
            self.capture_tribute(old_handle, &old, owner, tribute);
            self.crown_conversion(owner, old.def);
        }
        let health = (old.hp / self.max_hp(old.team, old.def)).clamp(0.0, 1.0);
        let city = self.city_ground[index];
        let citizen = if old.def == WORKER { self.source_identity(old_handle) } else { None };
        let new = self.spawn_oriented(old.def, owner, old.x, old.y, old.rot);
        let new_handle = self.h(new);
        if old.def == crate::koth64::HILL { self.koth_owner_replaced(old_handle, new_handle, owner); }
        self.ents[new].hp *= health;
        self.ents[new].built = old.built;
        if let Some(identity) = citizen {
            use crate::native_rng9::CitizenIdentity::{Female, Male};
            self.set_citizen_identity(new, if identity == 1 { Female } else { Male });
        }
        if old.def == TC {
            self.rebind_capitol(old_handle, new_handle);
            if city.handle == old_handle { self.city_ground[new] = crate::city_ground::CityBand { handle: new_handle, ..city }; }
            if old.built < 1.0 { self.city_ground[new] = crate::city_ground::EMPTY; }
            for claim in &mut self.territory.claims {
                if *claim == old_handle { *claim = new_handle; }
            }
            if let Some(names) = &mut self.city_names {
                if let Some(name) = names.ids.remove(&old_handle) { names.ids.insert(new_handle, name); }
            }
        }
        let (fx_len, event_len) = (self.fx.len(), self.events.len());
        self.kill(index);
        self.fx.truncate(fx_len);
        self.events.truncate(event_len);
        if DEFS[old.def as usize].building { self.block(new, true); }
        self.rebuild_territories();
        self.void_landless_alliances();
        self.prune_territory_offers();
        self.refresh_match_alive();
        self.build_grid();
        self.update_fog();
        true
    }

    pub(crate) fn wrap_capture_save(&self, core: Vec<u8>) -> Vec<u8> {
        if self.capture.empty() || core.is_empty() { return core; }
        let mut out = Vec::with_capacity(core.len() + 128);
        let clock = self.source_clock.is_some();
        let version = archive_version(self.capture_animation_enabled, clock);
        for word in [MAGIC, version, core.len() as u32] { out.extend_from_slice(&word.to_le_bytes()); }
        self.capture.fields(self.capture_animation_enabled, clock, |word| out.extend_from_slice(&word.to_le_bytes()));
        out.extend_from_slice(&core);
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out
    }

    pub(crate) fn restore_capture_save(&mut self, fields: Option<CaptureSave<'_>>) -> Option<()> {
        let Some(CaptureSave { fields, visual: with_visual, clock: with_clock }) = fields else {
            return self.ents[..self.high].iter().all(|actor| !actor.alive || actor.order != O_CAPTURE).then_some(());
        };
        if with_clock != self.source_clock.is_some() { return None; }
        let now = self.sim_elapsed();
        let mut state = State::default();
        let order_count = word(fields, 0)? as usize;
        let target_count = word(fields, 4)? as usize;
        let gift_count = word(fields, 8)? as usize;
        let mut offset = 12;
        for _ in 0..order_count {
            let handle = word(fields, offset)?;
            let target = word(fields, offset + 4)?;
            let team = u8::try_from(word(fields, offset + 8)?).ok()?;
            let next = f32::from_bits(word(fields, offset + 12)?);
            let flags = u8::try_from(word(fields, offset + 16)?).ok()?;
            offset += 20;
            let source_index = self.get(handle)?;
            let target_index = self.get(target)?;
            let source = self.ents[source_index];
            let victim = self.ents[target_index];
            if source.hp <= 0.0 || source.team != team || source.order != O_CAPTURE || source.otgt != target
                || native::capture_attacker(source.def).is_none() || !self.has_conversion_reserve(source.def, handle)
                || !self.capture_compatible(source_index, target_index)
                || self.conversion_target_profile(victim.def).is_none()
                || victim.hp <= 0.0 || victim.built != 1.0 || !self.capture_relation_allowed(team, target_index)
                || !next.is_finite() || next < 0.0 || flags & !0x87 != 0
                || (flags == 0 && next != 0.0) || (flags != 0 && flags & 0x80 == 0)
                || (flags & 1 != 0 && flags & 4 == 0)
                || state.orders.last_key_value().is_some_and(|(&old, _)| old >= handle) {
                return None;
            }
            let mut pose = None;
            let mut pose_at = 0.0;
            if with_visual {
                let packed = word(fields, offset)?;
                let tick = word(fields, offset + 4)?;
                offset += 8;
                if packed != 0 || tick != 0 {
                    if flags == 0 || tick > self.tick { return None; }
                    pose = Some(Pose::restore(packed, tick, source.def)?);
                }
                if with_clock {
                    pose_at = f32::from_bits(word(fields, offset)?);
                    offset += 4;
                    if pose.is_some() {
                        if !(0.0..=now).contains(&pose_at) || pose_at == 0.0 { return None; }
                    } else if pose_at.to_bits() != 0 { return None; }
                }
            }
            state.orders.insert(handle, Order { target, team, next, flags, pose, pose_at });
        }
        for _ in 0..target_count {
            let handle = word(fields, offset)?;
            let progress = f32::from_bits(word(fields, offset + 4)?);
            offset += 8;
            let target = self.ents[self.get(handle)?];
            if target.hp <= 0.0 || target.built != 1.0 || self.conversion_target_profile(target.def).is_none()
                || !(0.0..=1.0).contains(&progress)
                || state.targets.last_key_value().is_some_and(|(&old, _)| old >= handle) { return None; }
            state.targets.insert(handle, Target { progress, ..Target::default() });
        }
        for _ in 0..gift_count {
            let handle = word(fields, offset)?;
            let previous = u8::try_from(word(fields, offset + 4)?).ok()?;
            let tick = word(fields, offset + 8)?;
            offset += 12;
            let at = if with_clock {
                let at = f32::from_bits(word(fields, offset)?);
                offset += 4;
                if !(0.0..=now).contains(&at) || at == 0.0 { return None; }
                at
            } else { tick as f32 * DT };
            let target = self.ents[self.get(handle)?];
            if usize::from(previous) >= self.nplayers || previous == target.team || tick == 0
                || tick > self.tick || target.hp <= 0.0 || native::capture_target(target.def).is_none()
                || state.gifts.last_key_value().is_some_and(|(&old, _)| old >= handle) { return None; }
            state.gifts.insert(handle, Gift { previous, tick, at });
        }
        for order in state.orders.values().filter(|order| order.flags != 0) {
            let target = state.targets.get_mut(&order.target)?;
            if target.count > 0 && target.owner != order.team { return None; }
            target.owner = order.team;
            target.count += 1;
        }
        if offset != fields.len() || state.empty() || (self.winner != 0 && !state.orders.is_empty())
            || state.targets.values().any(|target| target.count == 0 && target.progress == 0.0)
            || self.ents[..self.high].iter().enumerate().any(|(index, actor)|
                actor.alive && actor.order == O_CAPTURE && !state.orders.contains_key(&self.h(index))) {
            return None;
        }
        self.capture_enabled = true;
        self.capture_animation_enabled = with_visual;
        self.capture = state;
        Some(())
    }
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

pub(crate) fn split_capture_save(bytes: &[u8]) -> Option<(&[u8], Option<CaptureSave<'_>>)> {
    if word(bytes, 0) != Some(MAGIC) { return Some((bytes, None)); }
    let core_len = word(bytes, 8)? as usize;
    let counts = [word(bytes, 12)? as usize, word(bytes, 16)? as usize, word(bytes, 20)? as usize];
    let (visual, clock) = match word(bytes, 4)? {
        VERSION => (false, false), VISUAL_VERSION => (true, false),
        CLOCK_VERSION => (false, true), CLOCK_VISUAL_VERSION => (true, true), _ => return None,
    };
    if core_len == 0 || counts.iter().any(|&count| count > MAXE) { return None; }
    let order_bytes = if visual { if clock { 32 } else { 28 } } else { 20 };
    let gift_bytes = if clock { 16 } else { 12 };
    let start = 24 + counts[0] * order_bytes + counts[1] * 8 + counts[2] * gift_bytes;
    let end = start.checked_add(core_len)?;
    if end.checked_add(4)? != bytes.len() || crate::pack8_save::checksum(bytes.get(..end)?) != word(bytes, end)? { return None; }
    Some((bytes.get(start..end)?, Some(CaptureSave { fields: bytes.get(12..start)?, visual, clock })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_CAPTURE, C_STOP};

    #[test]
    fn pyramid_rolls_once_per_eligible_neighbor_after_conversion() {
        use crate::{native_rng9::NativeRng, wonders61::{GREAT_PYRAMID, proselytizing_roll}};
        assert!(proselytizing_roll(1638));
        assert!(!proselytizing_roll(1639));
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        let center = [120.0, 120.0];
        let first = game.spawn(9, 1, 120.0, 120.5);
        let second = game.spawn(9, 1, 121.0, 120.0);
        let outside = game.spawn(9, 1, 120.0, 124.0);
        let [first, second, outside] = [game.h(first), game.h(second), game.h(outside)];
        game.build_grid();
        let seed = (0..100_000).find(|&seed| {
            let mut rng = NativeRng::from_state(seed);
            proselytizing_roll(rng.next_word15()) && !proselytizing_roll(rng.next_word15())
        }).unwrap();
        game.source_birth.rng = NativeRng::from_state(seed);
        game.proselytize_near_conversion(0, center[0], center[1]);
        assert_eq!(game.source_birth.rng.state(), seed);
        game.spawn(GREAT_PYRAMID, 0, 85.0, 85.0);
        let mut expected = NativeRng::from_state(seed);
        expected.next_word15();
        expected.next_word15();
        game.proselytize_near_conversion(0, center[0], center[1]);
        assert_eq!(game.source_birth.rng.state(), expected.state());
        assert!(game.get(first).is_none());
        assert!(game.get(second).is_some());
        assert!(game.get(outside).is_some());
    }

    fn scenario(definition: DefId) -> (Game, usize, usize) {
        scenario_with_seats(definition, 2)
    }

    fn scenario_with_seats(definition: DefId, humans: u32) -> (Game, usize, usize) {
        let mut game = Game::new(7, 0, 0, humans, 0);
        if humans == 2 { assert!(game.configure_peer_capture(true)); }
        let (x, y) = (25..220).flat_map(|y| (25..210).map(move |x| (x, y))).find(|&(x,y)| {
            game.map.starts[..2].iter().all(|&(sx,sy)| sx.abs_diff(x) + sy.abs_diff(y) > 40)
                && (y-3..=y+3).all(|j| (x-3..=x+8).all(|i| game.map.pass[0][j*N+i]))
        }).unwrap();
        let building = game.spawn(definition, 1, x as f32 + 0.5, y as f32 + 0.5);
        let soldier = game.spawn(10, 0, x as f32 + 6.5, y as f32 + 0.5);
        game.build_grid();
        game.update_fog();
        (game, soldier, building)
    }

    fn issue(game: &mut Game, attacker: usize, building: usize) {
        game.apply(0, C_CAPTURE, 0.0, 0.0, game.h(building) as i32, &[game.h(attacker)]);
        assert_eq!(game.ents[attacker].order, O_CAPTURE);
    }

    #[test]
    fn koth_nature_capture_uses_a_normal_impact_and_restarts_the_timer() {
        let mut game = Game::new_hill_world(7, 1);
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        game.battle_signals = Some(crate::battle_signals::BattleSignals::default());
        let hill = game.place_koth(128.0, 128.0).unwrap();
        let target = game.get(hill).unwrap();
        let soldier = game.spawn(10, 0, 132.0, 128.0);
        game.build_grid();
        game.update_fog();
        assert!(game.capture_allowed(0, target));
        issue(&mut game, soldier, target);
        for _ in 0..20 {
            game.tick();
            if game.capture_progress(target).is_some() { break; }
        }
        game.capture.targets.get_mut(&hill).expect("Hill capture registration").progress = 0.989;
        game.save();
        let mut game = Game::load(&game.savebuf).expect("registered Hill capture archive");
        for _ in 0..60 {
            game.tick();
            if game.get(hill).is_none() { break; }
        }
        let rule = game.koth.expect("same match rule after capture");
        assert_ne!(rule.actor, hill);
        let actor = game.ents[game.get(rule.actor).unwrap()];
        assert_eq!((actor.team, actor.hp), (0, 1000.0));
        assert_eq!(rule.holder, 0);
        assert!(rule.acquired_at > 0.0);
        let (territory, actor_index) = game.koth_territory().unwrap();
        assert_eq!(game.towner[territory], 0);
        assert_eq!(game.territory.claims[territory], game.h(actor_index));
        assert!(!game.territory_valid(0, 1, 0, territory as u32, NONE));
        game.apply(0, crate::cmd::C_DELETE, 0.0, 0.0, 0, &[rule.actor]);
        assert!(game.get(rule.actor).is_some());
        assert_eq!(game.winner, 0);
        game.tick += 12002;
        game.tick();
        assert_eq!(game.winner, 1);
        game.save();
        let restored = Game::load(&game.savebuf).expect("finished Hill match archive");
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(restored.winner, 1);
    }

    #[test]
    fn convert_replaces_a_wounded_citizen_without_retaining_orders_or_click() {
        let (mut game, _, house) = scenario_with_seats(HOUSE, 1);
        for (team, identity) in [(1, crate::native_rng9::CitizenIdentity::Female), (0, crate::native_rng9::CitizenIdentity::Male)] {
            let (x, y) = (game.ents[house].x + 6.0, game.ents[house].y + 2.0);
            let old = game.spawn_oriented(WORKER, team, x, y, 1.25);
            let handle = game.h(old);
            game.set_citizen_identity(old, identity);
            game.ents[old].hp = game.max_hp(team, WORKER) * 0.375;
            game.ents[old].carry = 8.0;
            game.ents[old].order = O_MOVE;
            game.conversion_click = handle;
            let fx = game.fx.len();
            game.events.clear();
            assert!(game.convert_last_clicked());
            assert_eq!(game.get(handle), None);
            let index = game.ents[..game.high].iter().enumerate().rev()
                .find(|(_, actor)| actor.alive && actor.def == WORKER && actor.x == x && actor.y == y)
                .unwrap().0;
            let new = game.ents[index];
            assert_eq!((new.team, new.hp, new.rot, new.carry, new.order),
                (0, game.max_hp(0, WORKER) * 0.375, 1.25, 0.0, O_IDLE));
            assert_eq!(game.source_identity(game.h(index)), Some(identity as u32));
            assert_eq!(game.fx.len(), fx);
            assert_eq!(game.events, [31.0, x, y]);
            assert!(!game.convert_last_clicked());
            game.kill(index);
        }
    }

    #[test]
    fn convert_keeps_rotated_foundation_blocked_and_rejects_missing_slots() {
        let (mut game, _, _) = scenario_with_seats(HOUSE, 1);
        let foundation = game.spawn_oriented(HOUSE, 1, 226.0, 214.0, core::f32::consts::FRAC_PI_4);
        game.ents[foundation].built = 0.4;
        let handle = game.h(foundation);
        game.conversion_click = handle;
        assert!(game.convert_last_clicked());
        assert_eq!(game.get(handle), None);
        let new = game.ents[..game.high].iter().find(|actor| actor.alive && actor.def == HOUSE && actor.x == 226.0).unwrap();
        assert_eq!((new.team, new.built, new.rot), (0, 0.4, core::f32::consts::FRAC_PI_4));
        let (x, y, width, height) = Game::building_bounds(new);
        for j in y..y + height {
            for i in x..x + width { assert!(game.map.blocked[j as usize * N + i as usize]); }
        }
        for definition in [FARM, 29, crate::trade30_defs::MERCHANT_SHIP, crate::air31_defs::FIGHTER_BOMBER] {
            let index = game.spawn(definition, 1, 228.0, 216.0);
            game.conversion_click = game.h(index);
            let hash = game.state_hash();
            assert!(!game.convert_last_clicked());
            assert_eq!(game.state_hash(), hash);
        }
    }

    #[test]
    fn source_clock_capture_saves_exact_pose_and_previous_owner_gift_time() {
        let (mut game, soldier, building) = scenario_with_seats(HOUSE, 1);
        game.source_birth.rng = crate::native_rng9::NativeRng::from_state(12345);
        assert!(game.tick_with_source_delta(0.125));
        let gift = game.spawn(HOUSE, 0, 241.5, 241.5);
        let gift_handle = game.h(gift);
        game.mark_capture_gift(gift);
        game.ents[gift].team = 1;
        let stamp = game.capture.gifts[&gift_handle];
        assert_eq!(stamp.at, 0.125);
        assert_eq!(stamp.previous, 0);
        assert!(stamp.active(1201, Some(crate::source_clock38::SourceClock { elapsed: 60.0, ..Default::default() })));
        assert!(!stamp.active(481, Some(crate::source_clock38::SourceClock { elapsed: 60.125, ..Default::default() })));
        issue(&mut game, soldier, building);
        let handle = game.h(soldier);
        for _ in 0..220 {
            assert!(game.tick_with_source_delta(0.075));
            if game.capture_animation_pose(handle).is_some() { break; }
        }
        let started = game.capture_animation_started(handle).expect("visible source capture request");
        assert_eq!(started, game.sim_elapsed());
        assert_ne!(started, game.tick as f32 * DT);
        game.save();
        assert_eq!(game.savebuf.get(16..20), Some(b"CAP1".as_slice()));
        assert_eq!(word(&game.savebuf, 20), Some(CLOCK_VISUAL_VERSION));
        let mut copy = Game::load(&game.savebuf).expect("source capture archive");
        copy.save();
        assert_eq!(copy.savebuf, game.savebuf);
        assert_eq!(copy.capture_animation_started(handle), Some(started));
        assert_eq!(copy.capture.gifts[&gift_handle].at, stamp.at);
        for delta in [0.025, 0.075, 0.1].into_iter().cycle().take(30) {
            assert!(game.tick_with_source_delta(delta));
            assert!(copy.tick_with_source_delta(delta));
            assert_eq!(copy.state_hash(), game.state_hash());
        }
    }

    #[test]
    fn command_walks_to_target_and_resumes_windup_from_the_exact_save() {
        let (mut game, soldier, building) = scenario(HOUSE);
        issue(&mut game, soldier, building);
        let start = game.ents[soldier].x;
        for _ in 0..160 {
            game.tick();
            if game.capture_progress(building).is_some() { break; }
        }
        assert!(game.ents[soldier].x < start - 1.0);
        assert_eq!(game.capture_progress(building), Some(0.0));
        let handle = game.h(building);
        game.capture.targets.get_mut(&handle).unwrap().progress = 0.985;
        game.save();
        let save = game.savebuf.clone();
        assert!(save.starts_with(b"CAP1"));
        let mut restored = Game::load(&save).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        restored.save();
        assert_eq!(restored.savebuf, save);
        let bad = save.len() - 5;
        let mut malformed = save.clone();
        malformed[bad] ^= 1;
        assert!(Game::load(&malformed).is_none());
        for _ in 0..60 {
            game.tick();
            restored.tick();
            assert_eq!(game.state_hash(), restored.state_hash());
            if game.get(handle).is_none() { break; }
        }
        assert!(game.get(handle).is_none());
        assert_eq!(game.ents[soldier].order, O_IDLE);
        assert!(game.ents.iter().any(|unit| unit.alive && unit.team == 0 && unit.def == HOUSE));
        assert!(game.capture.empty());
    }

    #[test]
    fn negotiated_capture_poses_use_shared_round_draws_and_save_the_accepted_stamp() {
        let (mut game, soldier, building) = scenario(HOUSE);
        assert!(game.configure_peer_capture_animation(true));
        let handle = game.h(soldier);
        game.source_birth.rng = crate::native_rng9::NativeRng::from_state(12345);
        issue(&mut game, soldier, building);
        assert!(!game.configure_peer_capture_animation(false));
        assert_eq!(game.capture_animation_pose(handle), None);
        for _ in 0..160 {
            game.tick();
            if game.capture_animation_pose(handle).is_some() { break; }
            assert_eq!(game.source_birth.rng.state(), 12345);
        }
        let first = game.capture_animation_pose(handle).unwrap();
        assert_eq!(first.tick, game.tick);
        assert_eq!(first.request(), 1084 << 2 | 1);
        assert_eq!(game.source_birth.rng.state(), 71072467);
        assert_eq!(game.capture_progress(building), Some(0.0));
        game.save();
        let saved = game.savebuf.clone();
        assert_eq!(word(&saved, 4), Some(VISUAL_VERSION));
        let mut restored = Game::load(&saved).unwrap();
        assert_eq!(restored.capture_animation_pose(handle), Some(first));
        restored.save();
        assert_eq!(restored.savebuf, saved);
        let mut future = saved.clone();
        future[48..52].copy_from_slice(&(game.tick + 1).to_le_bytes());
        let end = future.len() - 4;
        let checksum = crate::pack8_save::checksum(&future[..end]);
        future[end..].copy_from_slice(&checksum.to_le_bytes());
        assert!(Game::load(&future).is_none());

        let mut oracle = game.source_birth.rng;
        oracle.next_word15();
        oracle.next_word15();
        for _ in 0..95 {
            game.tick();
            restored.tick();
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(game.capture_animation_pose(handle), Some(first));
            if game.source_birth.rng.state() != 71072467 { break; }
        }
        assert_eq!(game.source_birth.rng, oracle);
        game.apply(0, C_STOP, 0.0, 0.0, 0, &[handle]);
        assert_eq!(game.capture_animation_pose(handle), None);
    }

    #[test]
    fn stopping_releases_the_contributor_and_hides_the_bar_before_decay() {
        let (mut game, soldier, building) = scenario(HOUSE);
        issue(&mut game, soldier, building);
        let victim = game.h(building);
        assert!(!game.has_active_capture_glow());
        assert!(!game.active_capture_glow(victim));
        for _ in 0..180 {
            game.tick();
            if game.capture_progress(building).is_some_and(|progress| progress > 0.0) { break; }
        }
        let progress = game.capture_progress(building).unwrap();
        assert!(progress > 0.0);
        assert!(game.has_active_capture_glow());
        assert!(game.active_capture_glow(victim));
        game.apply(0, C_STOP, 0.0, 0.0, 0, &[game.h(soldier)]);
        assert_eq!(game.capture_progress(building), None);
        assert_eq!(game.capture.targets[&victim].progress, progress);
        assert_eq!(game.capture.targets[&victim].count, 0);
        assert!(!game.has_active_capture_glow());
        assert!(!game.active_capture_glow(victim));
        for _ in 0..20 { game.tick(); }
        assert!(!game.capture.targets.contains_key(&victim));
    }

    #[test]
    fn city_replacement_rekeys_claim_and_name_and_settles_original_tribute() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let (x,y) = game.map.starts[1];
        let area = game.map.terr[y*N+x] as usize;
        let old_handle = game.territory.claims[area];
        let old = game.get(old_handle).unwrap();
        let old_pose = game.ents[old];
        game.city_names = Some(Default::default());
        game.city_names.as_mut().unwrap().ids.insert(old_handle, 1);
        game.ents[old].hp *= 0.5;
        assert!(game.complete_capture(old,0,native::capture_target(TC).unwrap()));
        let new_handle = game.territory.claims[area];
        let new = game.get(new_handle).unwrap();
        assert_ne!(old_handle,new_handle);
        assert!(game.get(old_handle).is_none());
        assert_eq!(game.towner[area],0);
        assert_eq!(game.city_names.as_ref().unwrap().ids[&new_handle],1);
        assert_eq!(game.city_ground[new].handle,new_handle);
        assert_eq!((game.ents[new].x,game.ents[new].y,game.ents[new].rot),(old_pose.x,old_pose.y,old_pose.rot));
        assert_eq!(game.ents[new].hp, game.max_hp(0,TC)*0.5);
        assert_eq!(game.resource(1,0),320.0);
        assert_eq!(game.resource(0,0),480.0);
        assert!(game.map.blocked[tile_of(old_pose.x,old_pose.y)]);
    }

    #[test]
    fn capitol_capture_replaces_its_city_and_returns_it_to_an_ally() {
        let mut game = Game::new(7, 2, 0, 1, 0);
        assert!(game.configure_match_capitols(0xe281, 500, 2, 0, 0, 6, 2, 0));
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        let (x, y) = game.map.starts[1];
        let area = game.map.terr[y * N + x] as usize;
        let original = game.territory.claims[area];
        let profile = native::capture_target(TC).unwrap();
        assert!(game.complete_capture(game.get(original).unwrap(), 2, profile));
        let occupied = game.territory.claims[area];
        assert_ne!(occupied, original);
        assert!(game.get(original).is_none());
        game.tick();
        assert_eq!(game.capitol_status(3, 1), 2);
        assert!(game.capitol_status(2, 1) >= 119);
        assert_eq!(game.capitol_status(5, 2), 100);
        game.save();
        let restored = Game::load(&game.savebuf).expect("Scored Capitol archive");
        assert_eq!(restored.capitol_status(5, 2), 100);
        assert_eq!(restored.state_hash(), game.state_hash());
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        for _ in 0..100 { game.tick(); }
        assert!(game.apply_diplomacy(0, 1, 1, 1, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        game.events.resize(3 * 64, 0.0);
        assert!(game.complete_capture(game.get(occupied).unwrap(), 0, profile));
        assert!(game.get(occupied).is_none());
        assert_eq!(game.towner[area], 1);
        assert!(game.events.ends_with(&[150.0, 9.0, 2.0]));
        assert_eq!(game.events.len(), 3 * 65);
        assert_eq!(game.capitol_status(5, 1), 0);
        game.tick();
        assert_eq!(game.capitol_status(3, 1), 1);
        assert_eq!(game.capitol_status(2, 1), -1);
    }
}
