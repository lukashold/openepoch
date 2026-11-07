//! Crown statistics for the installed UP rules.

use super::*;

#[path = "crown_score54.rs"]
pub(super) mod score;
#[path = "crown_units54.rs"]
mod units;
#[path = "crown_stat_powers54.rs"]
mod stat_powers;
#[path = "crown_glows54.rs"]
mod glows;
#[path = "crown_firing_ranges.rs"]
mod firing_ranges;

use score::{Award, Counters, Holder, Player as CrownPlayer, Rating, Scoreboard, KINDS};
use stat_powers::Stat;
use units::*;

const PLAYERS: usize = 4;
const MAGIC: u32 = u32::from_le_bytes(*b"CRN1");
const VERSION: u32 = 3;
const ECONOMIC: usize = 1;
const IMPERIAL: usize = 2;
const IMMIGRATION: u8 = 3;
const MARKET_STABILITY: u8 = 6;
const FIRING_RANGES: u8 = 5;
const PIRACY: u8 = 7;
const SOCIALIZED_MEDICINE: u8 = 8;
const SUPER_SPIES: u8 = 9;

#[derive(Clone, Copy, Default)]
struct Events {
    mobile_kills: i32,
    building_kills: i32,
    buildings_converted: i32,
    non_buildings_converted: i32,
    gathered: [f32; 9],
    stolen: [f32; 9],
    trade_gold: f32,
}

pub(crate) struct Crowns {
    disabled: bool,
    events: [Events; PLAYERS],
    score: Scoreboard<PLAYERS>,
    choices: [Option<u8>; KINDS],
    immigration_last: f32,
}

impl Crowns {
    fn new(disabled: bool) -> Self {
        Self { disabled, events: [Events::default(); PLAYERS], score: Scoreboard::default(),
            choices: [None; KINDS], immigration_last: 0.0 }
    }

    fn words(&self, mut write: impl FnMut(u32)) {
        write(u32::from(self.disabled));
        for events in self.events {
            for count in [events.mobile_kills, events.building_kills, events.buildings_converted, events.non_buildings_converted] {
                write(count as u32);
            }
            for amount in events.gathered.into_iter().chain(events.stolen).chain([events.trade_gold]) {
                write(amount.to_bits());
            }
        }
        for row in self.score.ratings {
            for rating in row {
                write(rating.score.to_bits());
                write(rating.threshold);
                write(u32::from(rating.eligible_rank));
            }
        }
        for (holder, choice) in self.score.holders.into_iter().zip(self.choices) {
            let words = holder.map_or([0; 3], |value| {
                [u32::from(value.owner) + 1, u32::from(value.epoch), value.expires.to_bits()]
            });
            for word in words { write(word); }
            write(choice.map_or(0, |index| u32::from(index) + 1));
        }
        write(self.immigration_last.to_bits());
    }

    pub(crate) fn hash(&self, mut write: impl FnMut(u32)) {
        write(MAGIC);
        self.words(write);
    }

    pub(crate) fn wrap(&self, core: Vec<u8>) -> Vec<u8> {
        if core.is_empty() { return core; }
        let mut data = Vec::new();
        for word in [MAGIC, VERSION, core.len() as u32] { data.extend_from_slice(&word.to_le_bytes()); }
        self.words(|word| data.extend_from_slice(&word.to_le_bytes()));
        data.extend_from_slice(&core);
        data.extend_from_slice(&crate::save::checksum(&data).to_le_bytes());
        data
    }

    pub(crate) fn read(payload: &[u8], participants: usize, elapsed: f32) -> Option<Self> {
        let mut reader = payload.chunks_exact(4);
        if !reader.remainder().is_empty() { return None; }
        let mut next = || Some(u32::from_le_bytes(reader.next()?.try_into().ok()?));
        let disabled = match next()? { 0 => false, 1 => true, _ => return None };
        let mut state = Self::new(disabled);
        for (owner, events) in state.events.iter_mut().enumerate() {
            for count in [&mut events.mobile_kills, &mut events.building_kills,
                &mut events.buildings_converted, &mut events.non_buildings_converted] {
                *count = next()? as i32;
                if owner >= participants && *count != 0 { return None; }
            }
            for amount in events.gathered.iter_mut().chain(&mut events.stolen).chain([&mut events.trade_gold]) {
                *amount = f32::from_bits(next()?);
                if !amount.is_finite() || *amount < 0.0 || owner >= participants && *amount != 0.0 { return None; }
            }
        }
        for (owner, row) in state.score.ratings.iter_mut().enumerate() {
            for rating in row {
                let value = f32::from_bits(next()?);
                let threshold = next()?;
                let rank = next()?;
                if !value.is_finite() || threshold > 750 || threshold % 50 != 0 || rank as usize > participants
                    || owner >= participants && (value != 0.0 || threshold != 0 || rank != 0) { return None; }
                *rating = Rating { score: value, threshold, eligible_rank: rank as u8 };
            }
        }
        for (kind, holder) in state.score.holders.iter_mut().enumerate() {
            let owner = next()?;
            let epoch = next()?;
            let expires = f32::from_bits(next()?);
            let choice = next()?;
            if owner == 0 {
                if epoch != 0 || expires.to_bits() != 0 || choice != 0 { return None; }
            } else {
                if disabled || owner as usize > participants || !(1..=15).contains(&epoch)
                    || !expires.is_finite() || expires < 0.0 || f64::from(expires) > f64::from(elapsed) + 300.0 { return None; }
                *holder = Some(Holder { owner: owner as u8 - 1, epoch: epoch as u8, expires });
                if choice != 0 {
                    let index = u8::try_from(choice - 1).ok()?;
                    stat_powers::get(kind, index)?;
                    state.choices[kind] = Some(index);
                }
            }
        }
        state.immigration_last = f32::from_bits(next()?);
        if !state.immigration_last.is_finite() || !(0.0..=elapsed).contains(&state.immigration_last)
            || state.choices[ECONOMIC] != Some(IMMIGRATION) && state.immigration_last.to_bits() != 0
            || reader.len() != 0 { return None; }
        Some(state)
    }
}

pub(crate) fn split(data: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if !data.starts_with(&MAGIC.to_le_bytes()) { return Some((data, None)); }
    let word = |offset: usize| Some(u32::from_le_bytes(data.get(offset..offset + 4)?.try_into().ok()?));
    let end = data.len().checked_sub(4)?;
    let len = word(8)? as usize;
    let start = end.checked_sub(len)?;
    if word(4)? != VERSION || len == 0 || start <= 12 || word(end)? != crate::save::checksum(&data[..end]) { return None; }
    Some((&data[start..end], Some(&data[12..start])))
}

fn count_units(counters: &mut Counters, definition: DefId) {
    let flags = MASKS[definition as usize];
    counters.military_units += i32::from(flags & MILITARY != 0);
    counters.military_buildings += i32::from(flags & MILITARY_BUILDING != 0);
    counters.economic_buildings += i32::from(flags & ECONOMIC_BUILDING != 0);
    counters.humans += i32::from(flags & HUMAN != 0);
    counters.roads_and_bridges += i32::from(flags & ROAD != 0);
    counters.cultural_units += i32::from(flags & CULTURAL != 0);
    counters.wonders += i32::from(flags & WONDER != 0);
}

fn add_resource_total(total: &mut f32, amount: f32) {
    *total = score::source_math::f(score::source_math::add(f64::from(*total), f64::from(amount))) as f32;
}

fn add_effect_scale(scale: &mut f32, factor: f32) {
    let delta = score::source_math::sub(f64::from(factor), 1.0);
    *scale = score::source_math::f(score::source_math::add(f64::from(*scale), delta)) as f32;
}

fn resource_integer(amount: f64) -> i32 {
    if (0.0..2147483648.0).contains(&amount) { amount as i32 } else { i32::MIN }
}

fn total_natural(amounts: [f32; 9]) -> i32 {
    resource_integer(amounts.into_iter().map(f64::from).fold(0.0, score::source_math::add))
}

impl Game {
    pub(crate) fn configure_crowns(&mut self, disabled: bool) -> bool {
        if self.net { return false; }
        self.configure_crown_rules(disabled)
    }

    pub(crate) fn configure_crowns_peer(&mut self, disabled: bool) -> bool {
        self.net && self.configure_crown_rules(disabled)
    }

    fn configure_crown_rules(&mut self, disabled: bool) -> bool {
        if self.tick != 0 || self.bench != 0 || self.crowns.is_some() { return false; }
        self.crowns = Some(Crowns::new(disabled));
        true
    }

    pub(crate) fn crown_rules(&self) -> u32 {
        self.crowns.as_ref().map_or(0, |crowns| 1 + u32::from(crowns.disabled))
    }

    pub(crate) fn crown_rating(&self, owner: usize, kind: usize) -> Option<Rating> {
        if owner >= self.nplayers || kind >= KINDS { return None; }
        if owner != self.local as usize && !self.crown_intel_active(self.local, owner as u8) { return None; }
        self.crowns.as_ref().map(|crowns| crowns.score.ratings[owner][kind])
    }

    pub(crate) fn crown_holder(&self, kind: usize) -> Option<Holder> {
        self.crowns.as_ref()?.score.holders.get(kind).copied().flatten()
    }

    pub(crate) fn can_choose_crown_power(&self, kind: usize, index: u8) -> bool {
        self.can_choose_crown_power_for(self.local, kind, index)
    }

    fn can_choose_crown_power_for(&self, owner: u8, kind: usize, index: u8) -> bool {
        let Some(power) = stat_powers::get(kind, index) else { return false; };
        let Some(crowns) = &self.crowns else { return false; };
        if owner as usize >= self.nplayers || !self.players[owner as usize].alive { return false; }
        let epoch = self.players[owner as usize].epoch + 1;
        (power.first_epoch..=power.last_epoch).contains(&epoch)
            && crowns.score.holders[kind].is_some_and(|holder| holder.owner == owner)
            && crowns.choices[kind].is_none()
    }

    pub(crate) fn choose_crown_power(&mut self, kind: usize, index: u8) -> bool {
        self.choose_crown_power_for(self.local, kind, index)
    }

    pub(crate) fn choose_crown_power_for(&mut self, owner: u8, kind: usize, index: u8) -> bool {
        if !self.can_choose_crown_power_for(owner, kind, index) { return false; }
        self.crowns.as_mut().unwrap().choices[kind] = Some(index);
        self.rebuild_modifiers(owner, true);
        true
    }

    pub(crate) fn crown_super_spies(&self, owner: u8) -> bool {
        self.owner_crown_power(owner, IMPERIAL) == Some(SUPER_SPIES)
    }

    pub(crate) fn crown_healing_building_scale(&self, owner: u8, definition: DefId) -> f32 {
        if matches!(definition, TC | FORTRESS | DOCK | AIRFIELD)
            && self.owner_crown_power(owner, IMPERIAL) == Some(SOCIALIZED_MEDICINE)
        {
            1.33
        } else {
            1.0
        }
    }

    pub(super) fn crown_market_stable(&self, owner: u8) -> bool {
        self.owner_crown_power(owner, ECONOMIC) == Some(MARKET_STABILITY)
    }

    pub(crate) fn crown_power(&self, kind: usize) -> Option<u8> {
        self.owner_crown_power(self.local, kind)
    }

    fn owner_crown_power(&self, owner: u8, kind: usize) -> Option<u8> {
        if self.crown_holder(kind)?.owner != owner { return None; }
        self.crowns.as_ref()?.choices.get(kind).copied().flatten()
    }

    pub(crate) fn crown_gather_scale(&self, owner: u8, definition: DefId, resource: u8, farm: bool, forage: bool) -> f32 {
        if definition != WORKER { return 1.0; }
        match (self.owner_crown_power(owner, ECONOMIC), resource) {
            (Some(1), R_FOOD) if farm => 2.0,
            (Some(2), R_FOOD) if forage => 2.0,
            (Some(4), R_GOLD) | (Some(5), R_WOOD) | (Some(7), R_STONE) => 2.0,
            (Some(8), R_METAL | R_TIN | R_SALTPETER) => 2.0,
            _ => 1.0,
        }
    }

    pub(crate) fn crown_generation_scale(&self, owner: u8, definition: DefId, resource: u8) -> f32 {
        match (definition, resource) {
            (OIL_RIG, 7) | (URANIUM_MINE, 8) if self.owner_crown_power(owner, ECONOMIC) == Some(8) => 2.0,
            (UNIVERSITY | TEMPLE, 9) if self.owner_crown_power(owner, IMPERIAL) == Some(10) => 1.1,
            _ => 1.0,
        }
    }

    pub(crate) fn crown_prevents_conversion(&self, owner: u8) -> bool {
        self.owner_crown_power(owner, IMPERIAL) == Some(4)
    }

    pub(crate) fn crown_conversion_range(&self, owner: u8, base: f32) -> f32 {
        if self.owner_crown_power(owner, IMPERIAL) != Some(6) { return base; }
        score::source_math::f(score::source_math::mul(f64::from(base), 1.25)) as f32
    }

    pub(crate) fn crown_city_protected(&self, actor: &Ent) -> bool {
        self.owner_crown_power(actor.team, IMPERIAL) == Some(0)
            && MASKS[actor.def as usize] & BUILDING != 0 && self.capture_owner_city(actor)
    }

    pub(crate) fn crown_powers_active_for(&self, owner: u8) -> bool {
        self.crowns.as_ref().is_some_and(|crowns| {
            crowns.score.holders.iter().zip(crowns.choices).any(|(holder, choice)| {
                choice.is_some() && holder.is_some_and(|holder| holder.owner == owner)
            })
        })
    }

    pub(crate) fn crown_visuals_present(&self) -> bool {
        self.crowns.as_ref().is_some_and(|crowns| crowns.choices.iter().any(Option::is_some))
    }

    fn crown_visual_target(&self, actor: &Ent, kind: usize) -> Option<&'static glows::Visual> {
        let holder = self.crown_holder(kind)?;
        let index = self.crowns.as_ref()?.choices[kind]?;
        glows::rows(kind, index).iter().find(|row| {
            if row.owner_only && actor.team != self.local { return false; }
            let admitted = if row.enemy { self.combat_hostile(holder.owner, actor.team) } else { holder.owner == actor.team };
            if !admitted || row.city && !self.capture_owner_city(actor) { return false; }
            match row.selector {
                glows::Selector::Units(units) => units.contains(&actor.def),
                glows::Selector::Convertible => crate::capture_math25::conversion_target(actor.def).is_some(),
                glows::Selector::FiringRange => firing_ranges::descriptor(actor.def, self.players[actor.team as usize].epoch) == row.row,
            }
        })
    }

    fn crown_visible_actor(&self, handle: u32) -> Option<&Ent> {
        let index = self.get(handle)?;
        let actor = &self.ents[index];
        (actor.hp > 0.0 && actor.built >= 1.0 && self.visible(index)).then_some(actor)
    }

    pub(crate) fn crown_actor_visual(&self, handle: u32, kind: usize, field: u32) -> u32 {
        let Some(row) = self.crown_visible_actor(handle).and_then(|actor| self.crown_visual_target(actor, kind))
        else { return 0; };
        match field { 0 => row.row, 1 => row.glow, _ => 0 }
    }

    pub(crate) fn crown_actor_glow(&self, handle: u32) -> u32 {
        let Some(actor) = self.crown_visible_actor(handle) else { return 0; };
        (0..KINDS).filter_map(|kind| self.crown_visual_target(actor, kind))
            .fold(0, |flags, row| flags | row.glow)
    }

    pub(crate) fn apply_crown_modifiers(&self, owner: u8, modifiers: &mut crate::research::Modifiers) {
        if !self.crown_powers_active_for(owner) { return; }
        let crowns = self.crowns.as_ref().unwrap();
        let mut scales = [[1.0_f32; 9]; DEFS.len()];
        for (kind, (&holder, choice)) in crowns.score.holders.iter().zip(crowns.choices).enumerate() {
            if holder.is_none_or(|holder| holder.owner != owner) { continue; }
            let Some(power) = choice.and_then(|index| stat_powers::get(kind, index)) else { continue; };
            if kind == IMPERIAL && choice == Some(FIRING_RANGES) {
                let epoch = self.players[owner as usize].epoch;
                for (id, scale) in scales.iter_mut().enumerate() {
                    let token = firing_ranges::descriptor(id as DefId, epoch);
                    if let Some((_, factor)) = firing_ranges::RANGE_FACTORS.iter().find(|(row, _)| *row == token) {
                        add_effect_scale(&mut scale[3], *factor);
                    }
                }
            }
            for modifier in power.modifiers {
                let slot = match modifier.stat {
                    Stat::Hp => 0, Stat::Attack => 1, Stat::Speed => 2,
                    Stat::Range => 3, Stat::Sight => 4, Stat::Capacity => 5, Stat::Population => 6,
                    Stat::ProductionCost | Stat::UnitCost => 7, Stat::ProductionTime => 8,
                };
                match modifier.stat {
                    Stat::ProductionCost | Stat::ProductionTime => {
                        for (index, definition) in DEFS.iter().enumerate() {
                            if !definition.building && modifier.units.iter().any(|&host| {
                                self.unit_production_available(owner, host, index as DefId)
                            }) {
                                add_effect_scale(&mut scales[index][slot], modifier.scale);
                            }
                        }
                    }
                    _ => {
                        for &definition in modifier.units { add_effect_scale(&mut scales[definition as usize][slot], modifier.scale); }
                    }
                }
            }
        }
        for (id, (row, scales)) in modifiers.iter_mut().zip(scales).enumerate() {
            for (scale, field) in scales.into_iter().zip([0, 1, 2, 6, 5, 11, 18, 4, 3]) {
                if scale == 1.0 { continue; }
                let offset = if field == 6 { f64::from(DEFS[id].w[0].range) } else { 0.0 };
                let current = score::source_math::add(f64::from(row[field]), offset);
                let scaled = score::source_math::mul(current, f64::from(scale));
                row[field] = score::source_math::f(score::source_math::sub(scaled, offset)) as f32;
            }
        }
    }

    pub(crate) fn crown_kill(&mut self, owner: u8, target: DefId) {
        let Some(crowns) = &mut self.crowns else { return; };
        let events = &mut crowns.events[owner as usize];
        let flags = MASKS[target as usize];
        events.mobile_kills = events.mobile_kills.wrapping_add(i32::from(flags & MOBILE != 0));
        events.building_kills = events.building_kills.wrapping_add(i32::from(flags & BUILDING != 0));
    }

    pub(crate) fn crown_piracy_payment(&mut self, owner: u8, attacker: DefId, victim_owner: u8, victim: DefId) {
        if self.owner_crown_power(owner, IMPERIAL) != Some(PIRACY)
            || victim_owner as usize >= self.nplayers || owner == victim_owner
            || !glows::rows(IMPERIAL, PIRACY).iter().any(|row| {
                matches!(row.selector, glows::Selector::Units(units) if units.contains(&attacker))
            }) { return; }
        for (slot, cost) in self.unit_cost(victim_owner, victim).into_iter().enumerate() {
            if cost != 0 { self.add_resource(owner, slot, f32::from(cost) * 0.5); }
        }
    }

    pub(crate) fn crown_conversion(&mut self, owner: u8, target: DefId) {
        let Some(crowns) = &mut self.crowns else { return; };
        let events = &mut crowns.events[owner as usize];
        let flags = MASKS[target as usize];
        events.buildings_converted = events.buildings_converted.wrapping_add(i32::from(flags & BUILDING != 0));
        events.non_buildings_converted = events.non_buildings_converted.wrapping_add(i32::from(flags & NON_BUILDING != 0));
    }

    pub(crate) fn crown_gathered(&mut self, owner: u8, resource: usize, amount: f32) {
        if let Some(crowns) = &mut self.crowns {
            let resource = crate::economy::NATIVE_IDS[resource] as usize;
            add_resource_total(&mut crowns.events[owner as usize].gathered[resource], amount);
        }
    }

    pub(crate) fn crown_stolen(&mut self, owner: u8, resource: usize, amount: f32) {
        if let Some(crowns) = &mut self.crowns {
            let resource = crate::economy::NATIVE_IDS[resource] as usize;
            add_resource_total(&mut crowns.events[owner as usize].stolen[resource], amount);
        }
    }

    pub(crate) fn crown_trade_gold(&mut self, owner: u8, amount: f32) {
        if let Some(crowns) = &mut self.crowns {
            add_resource_total(&mut crowns.events[owner as usize].trade_gold, amount);
        }
    }

    fn tick_crown_immigration(&mut self, now: f32, step: f32) {
        let Some(holder) = self.crown_holder(ECONOMIC) else { return; };
        let owner = holder.owner;
        if self.owner_crown_power(owner, ECONOMIC) != Some(IMMIGRATION)
            || !crate::capture_math25::capture_event_due(self.tick, u32::from(owner) + 1, step) { return; }
        let state = self.crowns.as_mut().unwrap();
        let elapsed = f64::from(now) - f64::from(state.immigration_last);
        if elapsed != 0.0 && elapsed < 90.0 { return; }
        state.immigration_last = now;

        let cities: Vec<_> = self.ents[..self.high].iter().copied().filter(|city| {
            city.alive && city.hp > 0.0 && city.built >= 1.0 && city.team == owner && city.def == TC
        }).collect();
        if cities.is_empty() { return; }
        let city = cities[self.source_birth.rng.next_word15() as usize % cities.len()];
        if self.nplayers <= 1 { return; }
        // The source consumes a draw for the count and the single listed unit
        // type. The other-player civilization draw cannot change generic Citizen.
        self.source_birth.rng.next_word15();
        let (x, y) = self.spawn_point(&city, &DEFS[TC as usize], &DEFS[WORKER as usize]);
        for _ in 0..3 {
            self.source_birth.rng.next_word15();
            self.source_birth.rng.next_word15();
            let identity = self.source_birth.production_identity();
            let index = self.spawn(WORKER, owner, x, y);
            if index != NOIDX { self.set_citizen_identity(index, identity); }
        }
        if owner == self.local { self.ev(34.0, city.x, city.y); }
    }

    pub(super) fn tick_crowns(&mut self) {
        if self.crowns.is_none() { return; }
        let (now, step) = (self.sim_elapsed(), self.sim_step());
        self.tick_crown_immigration(now, step);
        if !Scoreboard::<PLAYERS>::due(self.tick, step) { return; }
        let crowns = self.crowns.as_ref().unwrap();
        let mut players = std::array::from_fn(|owner| {
            let player = self.players[owner];
            if owner >= self.nplayers || !player.alive { return None; }
            let events = crowns.events[owner];
            let mut counters = Counters {
                mobile_kills: events.mobile_kills, building_kills: events.building_kills,
                buildings_converted: events.buildings_converted,
                non_buildings_converted: events.non_buildings_converted,
                gathered_natural: total_natural(events.gathered), stolen_natural: total_natural(events.stolen),
                trade_gold: resource_integer(f64::from(events.trade_gold)), territories: player.terr as i32,
                trade_routes: self.crown_trade_routes(owner as u8), ..Counters::default()
            };
            let mut epoch_technologies = [0; KINDS];
            for (id, node) in crate::ee2_research::NODES.iter().enumerate() {
                if self.has_research(owner as u8, id) {
                    counters.researched[node.branch as usize] += 1;
                    if node.epoch == player.epoch { epoch_technologies[node.branch as usize] += 1; }
                }
            }
            Some(CrownPlayer { epoch: player.epoch + 1, epoch_technologies, adjustment: [0.0; KINDS], counters })
        });
        for unit in &self.ents[..self.high] {
            if !unit.alive || unit.hp <= 0.0 || unit.built < 1.0 { continue; }
            let Some(Some(player)) = players.get_mut(unit.team as usize) else { continue; };
            count_units(&mut player.counters, unit.def);
            for &definition in &unit.cargo[..unit.ncargo as usize] { count_units(&mut player.counters, definition); }
        }
        let crowns = self.crowns.as_mut().unwrap();
        let awards = crowns.score.update(self.tick, now, step, crowns.disabled, &players);
        let mut retired = [false; PLAYERS];
        for (kind, award) in awards.into_iter().enumerate() {
            let previous = match award {
                Award::Unchanged => continue,
                Award::Revoked(holder) => Some(holder),
                Award::Assigned { previous, .. } => previous,
            };
            if let (Some(owner), Some(_)) = (previous.map(|value| value.owner), crowns.choices[kind].take()) {
                retired[owner as usize] = true;
            }
            if kind == ECONOMIC { crowns.immigration_last = 0.0; }
        }
        for (owner, rebuild) in retired.into_iter().enumerate() {
            if rebuild { self.rebuild_modifiers(owner as u8, true); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priest42_defs::PRIEST;

    #[test]
    fn crown_network_selection_market_and_checkpoint_are_owner_authoritative() {
        let mut peers = [Game::new(7, 0, 0, 2, 0), Game::new(7, 0, 0, 2, 1)];
        for game in &mut peers {
            assert!(!game.configure_crowns(false));
            assert!(game.configure_network_resignation(0));
            assert!(game.configure_crowns_peer(false));
            assert!(!game.configure_crowns_peer(true));
            game.researched[0][0] |= 0b1111 << 4;
            for tick in 0..64 {
                assert_eq!(game.tick_with_network_source_slot(30, 30, 1.0, tick == 0), 1);
            }
            assert_eq!(game.crown_holder(ECONOMIC).unwrap().owner, 0);
            game.apply(1, crate::cmd::C_CROWN_CHOOSE, 1.0, 0.0, 6, &[]);
            game.apply(0, crate::cmd::C_CROWN_CHOOSE, 1.0, 1.0, 6, &[]);
            assert_eq!(game.owner_crown_power(0, ECONOMIC), None);
            game.apply(0, crate::cmd::C_CROWN_CHOOSE, 1.0, 0.0, 6, &[]);
            game.apply(0, crate::cmd::C_CROWN_CHOOSE, 1.0, 0.0, 5, &[]);
            assert_eq!(game.owner_crown_power(0, ECONOMIC), Some(6));
            let market = game.spawn(MARKET, 0, 80.0, 80.0);
            let host = game.h(market);
            game.set_resource(0, 4, 700.0);
            assert_eq!(game.market_quote(0, host, 0, 5), Some((580, true)));
            game.apply(0, crate::cmd::C_MARKET, 0.0, 0.0, 5, &[host]);
            assert_eq!(game.resource(0, 4), 120.0);
            game.save_network_resume();
            assert!(!game.savebuf.is_empty());
        }
        assert_eq!(peers[0].state_hash(), peers[1].state_hash());
        assert_eq!(peers[0].savebuf, peers[1].savebuf);
        let checkpoint = &peers[0].savebuf;
        let restored = [0, 1].map(|owner| {
            let mut game = Game::load_network_resume(checkpoint, 2, owner, 64, 30, 1.0).unwrap();
            assert_eq!(game.crown_rules(), 1);
            assert_eq!(game.owner_crown_power(0, ECONOMIC), Some(6));
            assert_eq!(game.resource(0, 4), 120.0);
            assert_eq!(game.tick_with_network_source_slot(30, 30, 1.0, false), 1);
            game.state_hash()
        });
        assert_eq!(restored[0], restored[1]);
    }

    #[test]
    fn crown_world_statistics_and_award_survive_save_and_load() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        assert!(!game.configure_crowns(true));
        game.tick = 20;
        game.tick_crowns();
        let before = game.crown_rating(0, 0).unwrap().score;
        assert_eq!(game.crown_holder(0), None);
        assert_eq!(game.crown_rating(1, 0), None);

        game.researched[0][0] |= 0b1111;
        game.crown_kill(0, WORKER);
        game.crown_kill(0, TC);
        game.crown_conversion(0, TC);
        game.crown_gathered(0, 3, 20.0);
        game.crown_trade_gold(0, 20.0);
        game.crown_stolen(0, 0, 1000.0);
        game.tick = 40;
        game.tick_crowns();
        assert_eq!(game.crown_rating(0, 0), Some(Rating { score: before + 7.25, threshold: 50, eligible_rank: 1 }));
        assert_eq!(game.crown_holder(0), Some(Holder { owner: 0, epoch: 1, expires: 302.0 }));

        game.save();
        assert_eq!(&game.savebuf[..4], b"CRN1");
        let mut loaded = Game::load(&game.savebuf).expect("Crown score archive loads");
        assert_eq!(loaded.state_hash(), game.state_hash());
        loaded.save();
        assert_eq!(loaded.savebuf, game.savebuf);
        let mut corrupt = game.savebuf.clone();
        corrupt[12..16].copy_from_slice(&2_u32.to_le_bytes());
        let end = corrupt.len() - 4;
        let checksum = crate::save::checksum(&corrupt[..end]);
        corrupt[end..].copy_from_slice(&checksum.to_le_bytes());
        assert!(Game::load(&corrupt).is_none());
    }

    #[test]
    fn crown_disabled_setting_preserves_progress_without_awarding_and_old_archives_load() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        game.save();
        assert!(Game::load(&game.savebuf).unwrap().crowns.is_none());
        assert!(game.configure_crowns(true));
        game.researched[0][0] |= 0b1111;
        game.tick = 20;
        game.tick_crowns();
        assert_eq!(game.crown_rating(0, 0).unwrap().eligible_rank, 1);
        assert_eq!(game.crown_holder(0), None);
        assert!(!game.configure_crowns(false));
    }

    #[test]
    fn crown_super_spies_choice_persists_and_leaves_cast_duration_unchanged_after_expiry() {
        use crate::spy46_defs::SPY;
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crowns(false));
        let spy = game.spawn(SPY, 0, 80.0, 80.0);
        let target = game.ents.iter().position(|unit| unit.alive && unit.team == 1 && unit.def == TC).unwrap();
        let max_hp = game.max_hp(0, SPY);
        let speed = game.modifiers[0][SPY as usize][2];
        game.ents[spy].hp = max_hp * 0.5;
        assert!(!game.choose_crown_power(IMPERIAL, SUPER_SPIES));
        game.researched[0][0] |= 0b1111 << 8;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(IMPERIAL, SUPER_SPIES));
        assert!(!game.choose_crown_power(IMPERIAL, SUPER_SPIES));
        assert_eq!(game.max_hp(0, SPY), max_hp * 1.25);
        assert_eq!(game.modifiers[0][SPY as usize][2], speed * 1.25);
        assert_eq!(game.ents[spy].hp, max_hp * 0.625);
        game.tick = 6000;
        game.sabotage_building(0, target, game.sim_elapsed());
        game.save();
        let mut game = Game::load(&game.savebuf).expect("selected power and crowned sabotage load together");
        assert_eq!(game.max_hp(0, SPY), max_hp * 1.25);
        assert_eq!(game.crown_power(IMPERIAL), Some(SUPER_SPIES));
        game.tick = 6040;
        game.tick_crowns();
        assert_eq!(game.max_hp(0, SPY), max_hp);
        assert_eq!(game.ents[spy].hp, max_hp * 0.5);
        assert_eq!(game.crown_power(IMPERIAL), None);
        game.tick = 9000;
        assert!(game.sabotaged(target));
        game.tick = 9001;
        assert!(!game.sabotaged(target));
    }

    #[test]
    fn crown_doctrines_gate_at_the_live_epoch_and_overlapping_speed_scales_add() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        game.researched[0][0] |= 0b1111 | (0b1111 << 8);
        game.tick = 20;
        game.tick_crowns();
        for option in [0, 7, 8] { assert!(!game.can_choose_crown_power(0, option)); }
        assert!(game.can_choose_crown_power(0, 1));
        game.players[0].epoch = 10;
        for option in [0, 7, 8] { assert!(game.can_choose_crown_power(0, option)); }
        game.players[0].epoch = 0;
        let war_galley = 29;
        let base = game.modifiers[0][war_galley];
        assert!(game.choose_crown_power(0, 1));
        assert!(game.choose_crown_power(IMPERIAL, 3));
        let actual = game.modifiers[0][war_galley];
        assert_eq!(actual[0], base[0] * 1.5);
        assert_eq!(actual[2], score::source_math::f(f64::from(base[2]) * 1.75) as f32);
        assert_eq!(actual[5], score::source_math::f(f64::from(base[5]) * 1.25) as f32);
        assert_eq!(actual[1], base[1]);
    }

    #[test]
    fn socialized_medicine_scales_owned_airport_pulse_and_roundtrips() {
        let mut game = Game::new(7, 0, 0, 1, 0);
        assert!(game.configure_crowns(false));
        game.players[0].epoch = 10;
        game.rebuild_modifiers(0, false);
        let airport = game.spawn(AIRFIELD, 0, 84.5, 118.5);
        let airport_handle = game.h(airport);
        let fighter = game.spawn(37, 0, 84.5, 118.5);
        assert!(game.air_station_produced(fighter, airport));
        let base_hp = 50.0;
        game.ents[fighter].hp = base_hp;
        for _ in 0..22 { game.tick(); }
        assert_eq!(game.ents[fighter].hp, base_hp + 20.0);

        for slot in 0..4 {
            let tech = 10 * 12 + 8 + slot;
            game.researched[0][tech / 32] |= 1 << (tech % 32);
        }
        game.tick = 40;
        game.tick_crowns();
        assert!(game.choose_crown_power(IMPERIAL, SOCIALIZED_MEDICINE));
        assert_eq!(game.crown_actor_visual(airport_handle, IMPERIAL, 0), 36);
        assert_eq!(game.crown_healing_building_scale(1, AIRFIELD), 1.0);
        assert_eq!(game.crown_healing_building_scale(0, 84), 1.0);
        assert_eq!(game.fortress_heal_rate(0), 10.0 * 1.33);
        game.ents[fighter].hp = base_hp;
        game.save();
        let mut restored = Game::load(&game.savebuf).expect("Crown repair archive should load");
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        assert_eq!(restored.state_hash(), game.state_hash());
        for _ in 0..20 {
            game.tick();
            restored.tick();
        }
        assert_eq!(game.ents[fighter].hp, base_hp + 20.0 * 1.33);
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn crown_firing_ranges_respects_weapon_and_live_epoch() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        let archer = game.spawn(10, 0, 80.0, 80.0);
        let heavy = game.spawn(9, 0, 81.0, 80.0);
        let priest = game.spawn(PRIEST, 0, 82.0, 80.0);
        let range = |game: &Game, definition: usize| DEFS[definition].w[0].range + game.modifiers[0][definition][6];
        let base_archer = range(&game, 10);
        let base_heavy = range(&game, 9);
        let base_priest = range(&game, PRIEST as usize);
        game.researched[0][0] |= 0b1111 << 8;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(IMPERIAL, FIRING_RANGES));
        assert!((range(&game, 10) - base_archer * 1.2).abs() < 0.0001);
        assert_eq!(range(&game, 9), base_heavy);
        assert_eq!(range(&game, PRIEST as usize), base_priest);
        assert_eq!(game.crown_actor_visual(game.h(archer), IMPERIAL, 0), 28);
        assert_eq!(game.crown_actor_visual(game.h(heavy), IMPERIAL, 0), 0);
        assert_eq!(game.crown_actor_visual(game.h(priest), IMPERIAL, 0), 0);

        game.players[0].epoch = 6;
        game.rebuild_modifiers(0, false);
        assert_eq!(game.crown_actor_visual(game.h(heavy), IMPERIAL, 0), 28);
        let enhanced = range(&game, 9);
        game.crowns.as_mut().unwrap().choices[IMPERIAL] = None;
        game.rebuild_modifiers(0, false);
        assert!((enhanced - range(&game, 9) * 1.2).abs() < 0.0001);
    }

    #[test]
    fn crown_piracy_credits_only_eligible_attributed_kills_and_suppresses_foreign_glow() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        game.researched[0][0] |= 0b1111 << 8;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(IMPERIAL, PIRACY));
        let fleet = game.spawn(29, 0, 80.0, 80.0);
        let rival = game.spawn(29, 1, 81.0, 80.0);
        let victim = game.spawn(29, 1, 82.0, 80.0);
        let unattributed = game.spawn(29, 1, 83.0, 80.0);
        let attacker = Some(crate::battle_signals::Actor::new(fleet, &game.ents[fleet]));
        let status = game.crown_visual_target(&game.ents[rival], IMPERIAL).unwrap();
        assert_eq!(status.glow, 0);
        assert!(game.crown_visual_target(&game.ents[fleet], IMPERIAL).is_none());
        let cost = game.unit_cost(1, 29);
        assert!(cost.iter().any(|&value| value != 0));
        let stock = |game: &Game| -> [f32; 9] { std::array::from_fn(|slot| game.resource(0, slot)) };
        let before = stock(&game);
        let handle = game.h(victim);
        let profile = crate::ee2_combat::profile(29, 0);
        game.native_attack_impact(0, handle, 82.0, 80.0, 5000.0, W_ARROW, SEA, 0.0, 0.0, profile, attacker);
        let credited = std::array::from_fn(|slot| before[slot] + f32::from(cost[slot]) * 0.5);
        assert_eq!(stock(&game), credited);
        game.native_attack_impact(0, handle, 82.0, 80.0, 5000.0, W_ARROW, SEA, 0.0, 0.0, profile, attacker);
        game.native_attack_impact(0, game.h(unattributed), 83.0, 80.0, 5000.0, W_ARROW, SEA, 0.0, 0.0, profile, None);
        assert_eq!(stock(&game), credited);
        assert_eq!(game.crowns.as_ref().unwrap().events[0].stolen, [0.0; 9]);
    }

    #[test]
    fn crown_farming_affects_normal_farm_work_and_expansionism_changes_city_capacity() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        let farm = game.spawn(FARM, 0, 80.0, 80.0);
        let worker = game.spawn(WORKER, 0, 80.0, 80.0);
        game.ents[worker].otgt = game.h(farm);
        game.ents[worker].q[0] = R_FOOD as QueueItem;
        let mut ordinary = game.ents[worker];
        let def = &DEFS[WORKER as usize];
        game.do_gather(worker, &mut ordinary, def, &mut (0.0, 0.0), &mut false);
        assert!(ordinary.carry > 0.0);
        game.ents[farm].carry = 0.0;
        let base_city = game.modifiers[0][TC as usize][18];
        let base_house = game.modifiers[0][HOUSE as usize][18];

        game.researched[0][0] |= (0b1111 << 4) | (0b1111 << 8);
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(ECONOMIC, 1));
        assert!(game.choose_crown_power(IMPERIAL, 2));
        let mut enhanced = game.ents[worker];
        game.do_gather(worker, &mut enhanced, def, &mut (0.0, 0.0), &mut false);
        // The researched food modifier also affects the common rate. Compare
        // the ordinary farm and the Crown farm at the same researched value.
        let crown_amount = enhanced.carry;
        game.crowns.as_mut().unwrap().choices[ECONOMIC] = None;
        game.ents[farm].carry = 0.0;
        let mut unboosted = game.ents[worker];
        game.do_gather(worker, &mut unboosted, def, &mut (0.0, 0.0), &mut false);
        assert_eq!(crown_amount, unboosted.carry * 2.0);
        assert_eq!(game.crown_gather_scale(0, WORKER, R_FOOD, false, true), 1.0);
        assert_eq!(game.modifiers[0][TC as usize][18].round(), (base_city * 1.30).round());
        assert_eq!(game.modifiers[0][HOUSE as usize][18], base_house);
    }

    #[test]
    fn crown_immigration_uses_the_owner_phase_and_keeps_its_deadline_after_load() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        game.researched[0][0] |= 0b1111 << 4;
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(ECONOMIC, IMMIGRATION));

        let workers = |game: &Game| -> Vec<u32> {
            game.ents[..game.high].iter().enumerate()
                .filter(|(_, unit)| unit.alive && unit.team == 0 && unit.def == WORKER)
                .map(|(index, _)| game.h(index)).collect()
        };
        let before = workers(&game);
        let stock: [f32; 9] = std::array::from_fn(|slot| game.resource(0, slot));
        let original_rng = game.source_birth.rng.state();
        for tick in [1799, 1800, 1818] {
            game.tick = tick;
            game.tick_crowns();
            assert_eq!(workers(&game), before);
            assert_eq!(game.source_birth.rng.state(), original_rng);
        }
        game.tick = 1819;
        game.tick_crowns();
        let new: Vec<_> = workers(&game).into_iter().filter(|handle| !before.contains(handle)).collect();
        assert_eq!(new.len(), 3);
        let mut oracle = crate::native_rng9::NativeRng::from_state(original_rng);
        oracle.next_u32();
        oracle.next_u32();
        for handle in new {
            oracle.next_u32();
            oracle.next_u32();
            assert_eq!(game.source_identity(handle), Some((oracle.next_u32() >> 16) & 1));
        }
        assert_eq!(game.source_birth.rng.state(), oracle.state());
        assert_eq!(std::array::from_fn(|slot| game.resource(0, slot)), stock);
        assert_eq!(game.crown_actor_visual(game.h(0), ECONOMIC, 1), 0);

        let first = game.sim_elapsed();
        game.save();
        let mut restored = Game::load(&game.savebuf).unwrap();
        restored.save();
        assert_eq!(restored.savebuf, game.savebuf);
        let due = (1820..).find(|tick| tick % 20 == 19 && f64::from(*tick as f32 * DT) - f64::from(first) >= 90.0).unwrap();
        restored.tick = due - 20;
        restored.tick_crowns();
        assert_eq!(workers(&restored).len(), before.len() + 3);
        restored.tick = due;
        restored.tick_crowns();
        assert_eq!(workers(&restored).len(), before.len() + 6);
    }

    #[test]
    fn crown_war_economy_prices_new_orders_and_keeps_the_running_invoice() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_crowns(false));
        for resource in 0..9 { game.add_resource(0, resource, 4000.0); }
        let barracks = game.spawn(BARRACKS, 0, 80.0, 80.0);
        let unit = (0..DEFS.len() as DefId).find(|&id| game.unit_production_available(0, BARRACKS, id)).unwrap();
        game.researched[0][0] |= 0b1111 << 4;
        game.rebuild_modifiers(0, false);
        assert!(game.train(barracks, unit as QueueItem));
        let (ordinary_time, _) = game.begin_paid_job(barracks);
        game.ents[barracks].qt = 0.01;
        let ordinary = game.queue_payment(barracks, 0);
        let city_cost = game.unit_cost(0, TC);
        game.tick = 20;
        game.tick_crowns();
        assert!(game.choose_crown_power(ECONOMIC, 10));
        assert_eq!(game.queue_payment(barracks, 0).price, ordinary.price);
        assert_eq!(game.begin_paid_job(barracks), (ordinary_time, false));
        assert_eq!(game.unit_cost(0, TC), city_cost);
        let cheaper = game.unit_cost(0, unit);
        assert!(cheaper.into_iter().map(u32::from).sum::<u32>() < ordinary.price[..9].iter().sum::<f32>() as u32);
        let before: [f32; 9] = std::array::from_fn(|slot| game.resource(0, slot));
        assert!(game.train(barracks, unit as QueueItem));
        let after: [f32; 9] = std::array::from_fn(|slot| game.resource(0, slot));
        assert_eq!(std::array::from_fn(|slot| before[slot] - after[slot]), cheaper.map(f32::from));
        assert!(game.queue_time_limit(barracks, 1) < ordinary_time);
    }
}
