//! Building-capture orders and actor replacement for UP skirmish.

use super::*;
use crate::capture_animation26::{self as animation, Pose};
use crate::capture_math25::{self as native, CaptureFactors, CaptureTarget, CaptureTribute};
use std::collections::BTreeMap;

const NO_OWNER: u8 = u8::MAX;
const MAGIC: u32 = u32::from_le_bytes(*b"CAP1");
const VERSION: u32 = 1;
const VISUAL_VERSION: u32 = 2;
const NATURAL_ORDER: [usize; 9] = [0, 1, 3, 4, 5, 2, 6, 7, 8];

#[derive(Clone, Copy)]
struct Order {
    target: u32,
    team: u8,
    next: f32,
    flags: u8,
    pose: Option<Pose>,
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

    fn fields(&self, with_visual: bool, mut write: impl FnMut(u32)) {
        for value in [self.orders.len(), self.targets.len(), self.gifts.len()] { write(value as u32); }
        for (&handle, order) in &self.orders {
            for value in [handle, order.target, order.team.into(), order.next.to_bits(), order.flags.into()] { write(value); }
            if with_visual {
                write(order.pose.map_or(0, Pose::packed));
                write(order.pose.map_or(0, |pose| pose.tick));
            }
        }
        for (&handle, target) in &self.targets {
            write(handle);
            write(target.progress.to_bits());
        }
        for (&handle, gift) in &self.gifts {
            for value in [handle, gift.previous.into(), gift.tick] { write(value); }
        }
    }

    pub(crate) fn hash(&self, with_visual: bool, mut mix: impl FnMut(u32)) {
        if self.empty() { return; }
        mix(MAGIC);
        if with_visual { mix(VISUAL_VERSION); }
        self.fields(with_visual, mix);
    }
}

impl Game {
    pub(crate) fn capture_match_enabled(&self) -> bool { self.capture_enabled && self.winner == 0 }

    pub(crate) fn configure_peer_capture(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.capture.empty() { return false; }
        self.capture_enabled = enabled;
        if !enabled { self.capture_animation_enabled = false; }
        true
    }

    pub(crate) fn configure_peer_capture_animation(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.capture.empty()
            || (enabled && !self.capture_enabled) { return false; }
        self.capture_animation_enabled = enabled;
        true
    }

    pub(crate) fn capture_actor_ready(&self, index: usize) -> bool {
        self.capture_match_enabled() && self.ents[index].hp > 0.0 && native::capture_attacker(self.ents[index].def).is_some()
    }

    pub(crate) fn capture_allowed(&self, owner: u8, target: usize) -> bool {
        let actor = self.ents[target];
        if !self.capture_match_enabled() || !actor.alive || actor.built != 1.0 || actor.hp <= 0.0
            || !self.hostile(owner, actor.team) || !self.can_see(owner, target) || native::capture_target(actor.def).is_none() {
            return false;
        }
        let state = self.capture.targets.get(&self.h(target)).copied().unwrap_or_default();
        native::can_join_capture(actor.hp, state.count, Some(state.owner).filter(|&seat| seat != NO_OWNER), owner)
    }

    pub(crate) fn capture_progress(&self, index: usize) -> Option<f32> {
        self.capture.targets.get(&self.h(index)).filter(|target| target.count > 0).map(|target| target.progress)
    }

    pub(crate) fn has_active_capture_glow(&self) -> bool {
        self.capture.targets.values().any(|target| target.count > 0)
    }

    pub(crate) fn active_capture_glow(&self, handle: u32) -> bool {
        self.get(handle).is_some_and(|index| {
            self.capture_progress(index).is_some()
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

    pub(crate) fn order_capture(&mut self, indices: &[usize], target: usize) {
        let object = self.ents[target];
        let target_handle = self.h(target);
        let key = self.key_for(LAND, object.x, object.y);
        for &index in indices {
            let owner = self.ents[index].team;
            if !self.capture_actor_ready(index) || !self.capture_allowed(owner, target) { continue; }
            let handle = self.h(index);
            if self.capture.orders.get(&handle).is_some_and(|order| order.target == target_handle && order.team == owner) {
                continue;
            }
            self.capture.remove_order(handle);
            self.capture.orders.insert(handle, Order { target: target_handle, team: owner, next: 0.0, flags: 0, pose: None });
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
            && self.capture_actor_ready(index) && self.capture_allowed(order.team, target)
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
        let entities = &self.ents;
        let tick = self.tick;
        let living = |handle: u32| {
            let index = (handle & 0xffff) as usize;
            entities.get(index).filter(|e| e.alive && e.hp > 0.0 && u32::from(e.gen) == handle >> 16)
        };
        self.capture.targets.retain(|&handle, target| {
            let Some(actor) = living(handle) else { return false; };
            // OpenEpoch uses generation handles. Adapt the original seat bits
            // and retain the one-second stagger over this map's slot IDs.
            let phase = (u32::from(actor.team) + 1) << 27 | handle & 0xffff;
            if target.count == 0 && native::capture_event_due(tick, phase, DT) {
                target.progress = native::decay_capture_progress(target.progress);
            }
            target.count > 0 || target.progress > 0.0
        });
        self.capture.gifts.retain(|&handle, gift| {
            living(handle).is_some() && tick.saturating_sub(gift.tick) < 1200
        });
    }

    pub(crate) fn mark_capture_gift(&mut self, index: usize) {
        let entity = self.ents[index];
        if self.capture_enabled && self.tick > 0 && native::capture_target(entity.def).is_some() {
            self.capture.gifts.insert(self.h(index), Gift { previous: entity.team, tick: self.tick });
        }
    }

    pub(super) fn do_capture(&mut self, index: usize, actor: &mut Ent, def: &Def) -> (Option<f32>, Option<(f32, f32)>) {
        let handle = (u32::from(actor.gen) << 16) | index as u32;
        let Some(target) = self.get(actor.otgt).filter(|&target| self.capture_allowed(actor.team, target)) else {
            self.capture.remove_order(handle);
            Self::set_idle(actor);
            return (None, None);
        };
        let Some(attack) = native::capture_attacker(actor.def) else { return (None, None); };
        let building = self.ents[target];
        let dx = building.x - actor.x;
        let dy = building.y - actor.y;
        let radius = Self::building_half(&building);
        let edge = (dx.abs() - radius).max(0.0).hypot((dy.abs() - radius).max(0.0));
        let Some(order) = self.capture.orders.get_mut(&handle) else {
            Self::set_idle(actor);
            return (None, None);
        };
        let locked = crate::native_attack_clock::STARTED | crate::native_attack_clock::IN_ATTACK;
        if edge > attack.range + def.rad() && order.flags & locked != locked {
            crate::native_attack_clock::stop(&mut order.flags);
            order.pose = None;
            return (None, Some(self.steer(actor, def, building.x, building.y)));
        }
        let target_handle = actor.otgt;
        let state = self.capture.targets.entry(target_handle).or_default();
        if order.flags == 0 {
            state.count += 1;
            state.owner = actor.team;
        }
        let round = crate::native_attack_clock::step(self.tick as f32 * DT, attack.reload_seconds,
            attack.impact_seconds, &mut order.next, &mut order.flags);
        if self.capture_animation_enabled && round == crate::native_attack_clock::Event::Start {
            let context = animation::context(actor.def, self.players[actor.team as usize].epoch);
            Pose::apply(&mut order.pose, &mut self.source_birth.rng, context, self.tick);
        }
        actor.cd = (order.next - self.tick as f32 * DT).max(0.0);
        if round == crate::native_attack_clock::Event::Fire {
            self.capture_impact(index, actor, target, attack);
        }
        (Some(dy.atan2(dx)), None)
    }

    fn capture_impact(&mut self, attacker_index: usize, actor: &mut Ent, index: usize, attack: native::CaptureAttacker) {
        let building = self.ents[index];
        let mut profile = native::capture_target(building.def).expect("Admitted capture target");
        let research = [5, 65, 125].iter().filter(|&&id| self.has_research(building.team, id)).count() as f32 * 10.0;
        let regional = crate::civ8_traits::projected_starting_percent(self.civilizations[building.team as usize] as i8,
            building.def, "CaptureResistance").unwrap_or(0.0);
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
        let target = self.capture.targets.get_mut(&actor.otgt).expect("Admitted capture round");
        let factors = CaptureFactors {
            epoch_gap: i32::from(self.players[actor.team as usize].epoch) - i32::from(self.players[building.team as usize].epoch),
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

    fn capture_owner_city(&self, building: &Ent) -> bool {
        self.ents[..self.high].iter().enumerate().any(|(index, city)| {
            if city.team != building.team { return false; }
            let radius = self.city_ground_radius(index);
            radius > 0.0 && f64::from((city.x - building.x).abs() + (city.y - building.y).abs())
                < f64::from(radius) * f64::from(core::f32::consts::SQRT_2)
        })
    }

    fn capture_tribute(&mut self, handle: u32, building: &Ent, recipient: u8, tribute: CaptureTribute) {
        let same_gift = self.capture.gifts.get(&handle).is_some_and(|gift| gift.previous == recipient
            && self.tick.saturating_sub(gift.tick) < 1200);
        if same_gift { return; }
        let (mut rate, slots): (f32, &[usize]) = match tribute {
            CaptureTribute::Natural(rate) => (rate, &NATURAL_ORDER),
            CaptureTribute::Gold(rate) => (rate, &[4]),
            CaptureTribute::Tech(rate) => (rate, &[9]),
            CaptureTribute::None => return,
        };
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
            }
        }
    }

    fn complete_capture(&mut self, index: usize, owner: u8, profile: CaptureTarget) -> bool {
        let old = self.ents[index];
        if self.free.len() <= usize::from(old.ncargo) { return false; }
        if old.ncargo > 0 {
            let mut evacuated = old;
            self.unload(index, &mut evacuated);
            if evacuated.ncargo != 0 { return false; }
            self.ents[index] = evacuated;
        }
        let old_handle = self.h(index);
        self.capture_tribute(old_handle, &old, owner, profile.tribute);
        let health = (old.hp / self.max_hp(old.team, old.def)).clamp(0.0, 1.0);
        let city = self.city_ground[index];
        let new = self.spawn_oriented(old.def, owner, old.x, old.y, old.rot);
        let new_handle = self.h(new);
        self.ents[new].hp *= health;
        if old.def == TC {
            if city.handle == old_handle { self.city_ground[new] = crate::city_ground::CityBand { handle: new_handle, ..city }; }
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
        self.block(new, true);
        self.rebuild_territories();
        self.void_landless_alliances();
        self.prune_territory_offers();
        self.refresh_match_alive();
        self.build_grid();
        self.update_fog();
        if old.team == self.local || owner == self.local { self.ev(30.0, old.x, old.y); }
        true
    }

    pub(crate) fn wrap_capture_save(&self, core: Vec<u8>) -> Vec<u8> {
        if self.capture.empty() || core.is_empty() { return core; }
        let mut out = Vec::with_capacity(core.len() + 128);
        let version = if self.capture_animation_enabled { VISUAL_VERSION } else { VERSION };
        for word in [MAGIC, version, core.len() as u32] { out.extend_from_slice(&word.to_le_bytes()); }
        self.capture.fields(self.capture_animation_enabled, |word| out.extend_from_slice(&word.to_le_bytes()));
        out.extend_from_slice(&core);
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out
    }

    pub(crate) fn restore_capture_save(&mut self, fields: Option<(&[u8], bool)>) -> Option<()> {
        let Some((fields, with_visual)) = fields else {
            return self.ents[..self.high].iter().all(|actor| !actor.alive || actor.order != O_CAPTURE).then_some(());
        };
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
            let source = self.ents[self.get(handle)?];
            let victim = self.ents[self.get(target)?];
            if source.hp <= 0.0 || source.team != team || source.order != O_CAPTURE || source.otgt != target
                || native::capture_attacker(source.def).is_none() || native::capture_target(victim.def).is_none()
                || victim.hp <= 0.0 || victim.built != 1.0 || !self.hostile(team, victim.team)
                || !next.is_finite() || next < 0.0 || flags & !0x87 != 0
                || (flags == 0 && next != 0.0) || (flags != 0 && flags & 0x80 == 0)
                || (flags & 1 != 0 && flags & 4 == 0)
                || state.orders.last_key_value().is_some_and(|(&old, _)| old >= handle) {
                return None;
            }
            let mut pose = None;
            if with_visual {
                let packed = word(fields, offset)?;
                let tick = word(fields, offset + 4)?;
                offset += 8;
                if packed != 0 || tick != 0 {
                    if flags == 0 || tick > self.tick { return None; }
                    pose = Some(Pose::restore(packed, tick, source.def)?);
                }
            }
            state.orders.insert(handle, Order { target, team, next, flags, pose });
        }
        for _ in 0..target_count {
            let handle = word(fields, offset)?;
            let progress = f32::from_bits(word(fields, offset + 4)?);
            offset += 8;
            let target = self.ents[self.get(handle)?];
            if target.hp <= 0.0 || target.built != 1.0 || native::capture_target(target.def).is_none()
                || !(0.0..=1.0).contains(&progress)
                || state.targets.last_key_value().is_some_and(|(&old, _)| old >= handle) { return None; }
            state.targets.insert(handle, Target { progress, ..Target::default() });
        }
        for _ in 0..gift_count {
            let handle = word(fields, offset)?;
            let previous = u8::try_from(word(fields, offset + 4)?).ok()?;
            let tick = word(fields, offset + 8)?;
            offset += 12;
            let target = self.ents[self.get(handle)?];
            if usize::from(previous) >= self.nplayers || previous == target.team || tick == 0
                || tick > self.tick || target.hp <= 0.0 || native::capture_target(target.def).is_none()
                || state.gifts.last_key_value().is_some_and(|(&old, _)| old >= handle) { return None; }
            state.gifts.insert(handle, Gift { previous, tick });
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

pub(crate) fn split_capture_save(bytes: &[u8]) -> Option<(&[u8], Option<(&[u8], bool)>)> {
    if word(bytes, 0) != Some(MAGIC) { return Some((bytes, None)); }
    let core_len = word(bytes, 8)? as usize;
    let counts = [word(bytes, 12)? as usize, word(bytes, 16)? as usize, word(bytes, 20)? as usize];
    let with_visual = match word(bytes, 4)? { VERSION => false, VISUAL_VERSION => true, _ => return None };
    if core_len == 0 || counts.iter().any(|&count| count > MAXE) { return None; }
    let order_bytes = if with_visual { 28 } else { 20 };
    let start = 24 + counts[0] * order_bytes + counts[1] * 8 + counts[2] * 12;
    let end = start.checked_add(core_len)?;
    if end.checked_add(4)? != bytes.len() || crate::pack8_save::checksum(bytes.get(..end)?) != word(bytes, end)? { return None; }
    Some((bytes.get(start..end)?, Some((bytes.get(12..start)?, with_visual))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{C_CAPTURE, C_STOP};

    fn scenario(definition: DefId) -> (Game, usize, usize) {
        let mut game = Game::new(7, 0, 0, 2, 0);
        assert!(game.configure_peer_capture(true));
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
}
