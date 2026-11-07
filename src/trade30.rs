//! Market and dock trade orders in the native ruleset.

use std::collections::{BTreeMap, BTreeSet};

use crate::defs::*;
use crate::game::{Ent, Game, MAXE, NONE, O_IDLE};
use crate::map::{tile_of, DIRS, N, NN, R_GOLD};
use crate::trade30_credit::{self as credit, Account};
use crate::trade30_defs::{is_trader, MERCHANT_SHIP, TRADE_CART};
use crate::trade30_queue::{Exit, Queue, Trader};
use crate::trade30_quote::{self as pricing, Cargo, Quote, Terms};

const MAGIC: u32 = u32::from_le_bytes(*b"TRD1");
const VERSION: u32 = 1;
pub(crate) const SAVE_FLAG: u32 = 1 << 5;
const GOLD: usize = R_GOLD as usize - 1;
const FOREIGN_RESEARCH: usize = 24;
const GOLD_RESEARCH: [usize; 3] = [36, 96, 147];
const LOAD_RESEARCH: [usize; 3] = [51, 111, 171];
const CART_CHANCES: &[f32] = &[0.1, 0.15, 0.2, 0.25, 0.3];
const SHIP_CHANCES: &[f32] = &[0.01, 0.02, 0.04, 0.08, 0.1];

// Beer, tools, clothing, figs, flints, trinkets; silk, tea, herbs, art, rice;
// wool, art, lemons, pottery, turquoise; gems, obsidian, jade, rubber, cotton.
const fn cargo(id: u32, bonus: f32) -> Cargo { Cargo { id, bonus } }
const POOLS: [&[Cargo]; 4] = [
    &[cargo(0, 0.2), cargo(1, 0.2), cargo(2, 0.1), cargo(3, 0.1), cargo(4, 0.1), cargo(5, 0.1)],
    &[cargo(6, 0.2), cargo(7, 0.1), cargo(8, 0.1), cargo(9, 0.2), cargo(10, 0.1)],
    &[cargo(11, 0.1), cargo(9, 0.1), cargo(12, 0.1), cargo(13, 0.2), cargo(14, 0.2)],
    &[cargo(15, 0.2), cargo(16, 0.1), cargo(17, 0.2), cargo(18, 0.1), cargo(19, 0.1)],
];

pub(crate) fn research_note(node: usize) -> Option<&'static str> {
    match node {
        FOREIGN_RESEARCH => Some("Handelsrouten zu neutralen und verbündeten Märkten und Häfen."),
        node if GOLD_RESEARCH.contains(&node) => Some("Handelskarren und Handelsschiffe: Goldertrag pro Feld +10%."),
        node if LOAD_RESEARCH.contains(&node) => Some("Handelskarren und Handelsschiffe: Ladezeit −20%."),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Route {
    home: u32,
    destination: u32,
    returning: bool,
    at_host: bool,
    quote: Quote,
}

impl Route {
    fn host(self) -> u32 {
        if self.returning { self.home } else { self.destination }
    }
}

#[derive(Default)]
pub(crate) struct State {
    routes: BTreeMap<u32, Route>,
    hosts: BTreeMap<u32, Queue>,
}

fn host_for(trader: DefId) -> DefId {
    match trader {
        TRADE_CART => MARKET,
        MERCHANT_SHIP => DOCK,
        _ => unreachable!("only traders have a trade host"),
    }
}

fn domain(trader: DefId) -> usize { usize::from(trader == MERCHANT_SHIP) }

// ADDPERMMUL adds percentage points before the original descriptor's f32 store.
fn trade_factor(baseline: f32, percent: i16) -> f32 {
    (f64::from(100 + percent) * f64::from(0.01f32) * f64::from(baseline)) as f32
}

/// Connectivity for an order is read-only. The flow-field cache and its tick
/// budget are part of live movement and must stay untouched during local input.
fn connected(pass: &[bool], goal: usize) -> Vec<bool> {
    let mut reached = vec![false; NN];
    if !pass[goal] { return reached; }
    let mut queue = vec![goal];
    reached[goal] = true;
    let mut cursor = 0;
    while let Some(&tile) = queue.get(cursor) {
        cursor += 1;
        let (x, y) = ((tile % N) as i32, (tile / N) as i32);
        // The navigation field forbids diagonal corner cuts, so the four
        // cardinal neighbours suffice to compute its connected components.
        for &(dx, dy) in &DIRS[..4] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || nx >= N as i32 || ny < 0 || ny >= N as i32 { continue; }
            let next = ny as usize * N + nx as usize;
            if pass[next] && !reached[next] {
                reached[next] = true;
                queue.push(next);
            }
        }
    }
    reached
}

impl Game {
    pub(crate) fn crown_trade_routes(&self, owner: u8) -> i32 {
        self.trade.routes.keys().filter(|&&handle| {
            self.get(handle).is_some_and(|index| self.ents[index].team == owner)
        }).count() as i32
    }

    pub(crate) fn configure_peer_trade_route(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() || !self.inbox.is_empty() {
            return false;
        }
        self.trade_enabled = enabled;
        true
    }

    pub(crate) fn trade_actor_ready(&self, index: usize) -> bool {
        let actor = &self.ents[index];
        self.trade_enabled && actor.alive && actor.hp > 0.0 && is_trader(actor.def)
    }

    pub(crate) fn trade_hidden(&self, index: usize, actor: &Ent) -> bool {
        self.trade.is_hidden(index, actor)
    }

    pub(crate) fn trade_wait_active(&self, handle: u32) -> bool {
        self.get(handle).is_some_and(|index| self.trade_actor_ready(index)) &&
            self.trade.routes.get(&handle).is_some_and(|route| route.at_host)
    }

    pub(crate) fn trade_interrupt(&mut self, index: usize) {
        if self.trade_enabled && is_trader(self.ents[index].def) {
            self.trade.cancel(self.h(index));
        }
    }

    pub(crate) fn trade_host(&self, team: u8, trader: DefId, handle: u32, own: bool) -> Option<usize> {
        let index = self.get(handle)?;
        let host = &self.ents[index];
        if host.def != host_for(trader) || host.built < 1.0 || host.hp <= 0.0 { return None; }
        let allowed = host.team == team || (!own && self.has_research(team, FOREIGN_RESEARCH) &&
            !self.hostile(team, host.team));
        allowed.then_some(index)
    }

    pub(crate) fn trade_host_for_selection(&self, index: usize, home: u32, selection: &[usize]) -> bool {
        let handle = self.h(index);
        handle != home && selection.iter().copied().any(|actor| {
            let unit = &self.ents[actor];
            self.trade_actor_ready(actor) &&
                self.trade_host(unit.team, unit.def, handle, home == NONE).is_some() &&
                self.visible_to_trader(unit.team, index) &&
                (home == NONE || self.trade_host(unit.team, unit.def, home, true).is_some())
        })
    }

    fn trade_port(&self, host: usize, trader: DefId) -> Option<(f32, f32)> {
        let mut building = self.ents[host];
        building.rx = -1.0;
        let point = self.spawn_point(&building, &DEFS[building.def as usize], &DEFS[trader as usize]);
        self.map.pass[domain(trader)][tile_of(point.0, point.1)].then_some(point)
    }

    pub(crate) fn trade_candidates(&self, team: u8, home: u32, destination: u32, selection: &[usize]) -> Vec<usize> {
        if !self.trade_enabled || home == destination { return Vec::new(); }
        let mut candidates = selection.iter().copied().filter(|&i| self.trade_actor_ready(i) && self.ents[i].team == team);
        let Some(first) = candidates.next() else { return Vec::new(); };
        let Some(home_index) = self.get(home) else { return Vec::new(); };
        let trader = match self.ents[home_index].def {
            MARKET => TRADE_CART,
            DOCK => MERCHANT_SHIP,
            _ => return Vec::new(),
        };
        let Some(home_index) = self.trade_host(team, trader, home, true) else { return Vec::new(); };
        let Some(destination_index) = self.trade_host(team, trader, destination, false) else { return Vec::new(); };
        if !self.visible_to_trader(team, destination_index) { return Vec::new(); }
        let Some(home_port) = self.trade_port(home_index, trader) else { return Vec::new(); };
        let Some(destination_port) = self.trade_port(destination_index, trader) else { return Vec::new(); };
        let dc = domain(trader);
        let reached = connected(&self.map.pass[dc], tile_of(destination_port.0, destination_port.1));
        if !reached[tile_of(home_port.0, home_port.1)] { return Vec::new(); }
        let mut accepted: Vec<_> = std::iter::once(first).chain(candidates).filter(|&i| {
            let unit = &self.ents[i];
            unit.def == trader && reached[self.map.snap(unit.x, unit.y, dc)]
        }).collect();
        // A repeated handle in a wire packet is one unit and one initial cargo.
        accepted.sort_unstable();
        accepted.dedup();
        accepted
    }

    fn visible_to_trader(&self, team: u8, host: usize) -> bool {
        let e = &self.ents[host];
        e.team == team || self.real_fog(team, tile_of(e.x, e.y)) >= 128
    }

    pub(crate) fn order_trade_route(&mut self, team: u8, home: u32, destination: u32, selection: &[usize]) {
        if home == NONE {
            let choices = self.trade_automatic_choices(team, destination, selection);
            for (index, home) in choices { self.start_trade(index, home, destination); }
            return;
        }
        let accepted = self.trade_candidates(team, home, destination, selection);
        for index in accepted {
            self.start_trade(index, home, destination);
        }
    }

    fn start_trade(&mut self, index: usize, home: u32, destination: u32) {
        let home_index = self.get(home).unwrap();
        let destination_index = self.get(destination).unwrap();
        let unit = self.ents[index];
        let port = self.trade_port(destination_index, unit.def).unwrap();
        let quote = self.trade_quote(index, home_index, destination_index, unit.team);
        self.order_move(&[index], port.0, port.1, false);
        self.trade.routes.insert(self.h(index), Route { home, destination, returning: false, at_host: false, quote });
    }

    pub(crate) fn trade_automatic_choices(&self, team: u8, destination: u32, selection: &[usize]) -> Vec<(usize, u32)> {
        if !self.trade_enabled || selection.is_empty() { return Vec::new(); }
        let Some(index) = self.get(destination) else { return Vec::new(); };
        let trader = match self.ents[index].def {
            MARKET => TRADE_CART,
            DOCK => MERCHANT_SHIP,
            _ => return Vec::new(),
        };
        if self.trade_host(team, trader, destination, false).is_none() || !self.visible_to_trader(team, index) {
            return Vec::new();
        }
        let Some(port) = self.trade_port(index, trader) else { return Vec::new(); };
        let dc = domain(trader);
        let reached = connected(&self.map.pass[dc], tile_of(port.0, port.1));
        let hosts: Vec<_> = self.trade_home_indices(team, trader, destination, &reached).collect();
        let mut chosen: Vec<_> = selection.iter().copied().filter_map(|i| {
            let unit = self.ents[i];
            if unit.team != team || unit.def != trader || !self.trade_actor_ready(i) ||
                !reached[self.map.snap(unit.x, unit.y, dc)] { return None; }
            let prior = self.trade.routes.get(&self.h(i)).and_then(|route| self.get(route.home)).filter(|home| hosts.contains(home));
            let home = prior.or_else(|| self.nearest_trade_host(unit, hosts.iter().copied()))?;
            Some((i, self.h(home)))
        }).collect();
        chosen.sort_unstable_by_key(|&(index, _)| index);
        chosen.dedup_by_key(|entry| entry.0);
        chosen
    }

    fn trade_quote(&mut self, index: usize, home: usize, destination: usize, cargo_owner: u8) -> Quote {
        let (unit, a, b) = (self.ents[index], self.ents[home], self.ents[destination]);
        let epoch_gap = i32::from(self.players[b.team as usize].epoch) - i32::from(self.players[unit.team as usize].epoch);
        let civ = self.civilizations[cargo_owner as usize];
        let region = crate::civ8_gates::REGION_BY_CIV.get(civ as usize).copied().unwrap_or(0);
        let pool = POOLS[region as usize];
        let tech_chances = if unit.def == MERCHANT_SHIP && !self.has_research(unit.team, FOREIGN_RESEARCH) {
            SHIP_CHANCES
        } else {
            CART_CHANCES
        };
        let terms = Terms { gold_per_tile: self.trade_gold_rate(unit.team), tech_per_tile: 0.0,
            tech_per_epoch: 0.0, tech_chances };
        pricing::quote(&terms, [a.x, a.y], [b.x, b.y], unit.team == b.team, epoch_gap, pool, None,
            || self.source_birth.rng.next_word15() as u16).unwrap()
    }

    fn trade_gold_rate(&self, team: u8) -> f32 {
        let civ = self.civilizations[team as usize] as i8;
        let traits: i16 = crate::civ8_traits::FACTORS.iter().filter(|effect| effect.field == "TradeRouteDrop" &&
            crate::civ8_gates::matches_ttciv(effect.ttciv, civ)).map(|effect| effect.value as i16).sum();
        let research = GOLD_RESEARCH.into_iter().filter(|&node| self.has_research(team, node)).count() as i16;
        trade_factor(0.2, traits + 10 * research)
    }

    fn trade_load_time(&self, team: u8) -> f32 {
        let research = LOAD_RESEARCH.into_iter().filter(|&node| self.has_research(team, node)).count() as i16;
        trade_factor(5.0, -20 * research)
    }

    fn trade_account(&self, team: u8) -> Account {
        Account { gold: self.resource(team, GOLD), technology: self.research_points[team as usize] }
    }

    fn write_trade_account(&mut self, team: u8, account: Account) {
        self.extra_resources[team as usize][GOLD - 3] = account.gold;
        self.research_points[team as usize] = account.technology;
    }

    fn trade_delivery(&mut self, team: u8, foreign: u8, paid_at_destination: bool, quote: Quote) {
        let mut owner = self.trade_account(team);
        let mut partner = self.trade_account(foreign);
        let owner_gold = owner.gold;
        let partner_gold = partner.gold;
        let foreign_account = (team != foreign).then_some((&mut partner, paid_at_destination));
        credit::deliver(&mut owner, foreign_account, Account { gold: quote.gold, technology: quote.tech });
        self.write_trade_account(team, owner);
        self.crown_trade_gold(team, owner.gold - owner_gold);
        if team != foreign {
            self.write_trade_account(foreign, partner);
            self.crown_trade_gold(foreign, partner.gold - partner_gold);
        }
    }

    fn trade_replacement(&self, unit: Ent, destination: u32) -> Option<u32> {
        let dc = domain(unit.def);
        let reached = connected(&self.map.pass[dc], self.map.snap(unit.x, unit.y, dc));
        self.nearest_trade_host(unit, self.trade_home_indices(unit.team, unit.def, destination, &reached)).map(|i| self.h(i))
    }

    fn trade_home_indices<'a>(&'a self, team: u8, trader: DefId, destination: u32, reached: &'a [bool]) -> impl Iterator<Item = usize> + 'a {
        (0..self.high).filter(move |&i| {
            let handle = self.h(i);
            handle != destination && self.trade_host(team, trader, handle, true).is_some() &&
                self.trade_port(i, trader).is_some_and(|(x, y)| reached[tile_of(x, y)])
        })
    }

    fn nearest_trade_host(&self, unit: Ent, hosts: impl Iterator<Item = usize>) -> Option<usize> {
        let height = self.map.height(unit.x, unit.y);
        let distance = |i: usize| {
            let host = &self.ents[i];
            let dx = f64::from(host.x) - f64::from(unit.x);
            let dy = f64::from(host.y) - f64::from(unit.y);
            let dz = f64::from(self.map.height(host.x, host.y)) - f64::from(height);
            (dx * dx + dy * dy + dz * dz) as f32
        };
        hosts.map(|i| (i, distance(i))).min_by(|(_, a), (_, b)| a.total_cmp(b)).map(|(i, _)| i)
    }

    pub(super) fn tick_trade(&mut self) {
        if self.trade.routes.is_empty() { return; }
        let mut state = std::mem::take(&mut self.trade);
        state.update(self);
        self.trade = state;
    }

    pub(crate) fn wrap_trade_save(&self, core: Vec<u8>) -> Vec<u8> {
        if !self.trade_enabled || core.is_empty() { return core; }
        let uses_trade_defs = self.ents[..self.high].iter().any(|e| e.alive && (is_trader(e.def) ||
            e.cargo[..e.ncargo as usize].iter().copied().any(is_trader) ||
            (DEFS[e.def as usize].building && e.q[..e.qn as usize].iter().any(|&q| is_trader(q as DefId)))));
        if !uses_trade_defs { return core; }
        let mut bytes = Vec::with_capacity(core.len() + 64);
        for value in [MAGIC, VERSION, core.len() as u32] { bytes.extend_from_slice(&value.to_le_bytes()); }
        self.trade.fields(|value| bytes.extend_from_slice(&value.to_le_bytes()));
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::pack8_save::checksum(&bytes).to_le_bytes());
        bytes
    }

    pub(crate) fn restore_trade_save(&mut self, payload: Option<&[u8]>) -> Option<()> {
        if let Some(payload) = payload {
            if !self.trade_enabled { return None; }
            self.trade = State::decode(self, payload)?;
        }
        Some(())
    }
}

pub(crate) fn split_save(bytes: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    let word = |start: usize| Some(u32::from_le_bytes(bytes.get(start..start + 4)?.try_into().ok()?));
    if word(0) != Some(MAGIC) { return Some((bytes, None)); }
    if word(4)? != VERSION { return None; }
    let checksum = bytes.len().checked_sub(4)?;
    let start = checksum.checked_sub(word(8)? as usize)?;
    let payload = bytes.get(12..start)?;
    if payload.len() % 4 != 0 || payload.len() > (11 * MAXE + 2) * 4 { return None; }
    if crate::pack8_save::checksum(&bytes[..checksum]) != word(checksum)? { return None; }
    Some((bytes.get(start..checksum)?, Some(payload)))
}

impl State {
    pub(super) fn is_hidden(&self, index: usize, actor: &Ent) -> bool {
        if !is_trader(actor.def) { return false; }
        let handle = (u32::from(actor.gen) << 16) | index as u32;
        self.routes.get(&handle).is_some_and(|route| route.at_host && self.loading(handle, *route))
    }

    pub(super) fn hash(&self, mix: &mut impl FnMut(u32)) {
        mix(MAGIC);
        self.fields(mix);
    }

    fn fields(&self, mut mix: impl FnMut(u32)) {
        mix(self.routes.len() as u32);
        for (&handle, route) in &self.routes {
            for value in [handle, route.home, route.destination, u32::from(route.returning) | u32::from(route.at_host) << 1,
                route.quote.cargo, route.quote.gold.to_bits(), route.quote.tech.to_bits()] {
                mix(value);
            }
        }
        mix(self.hosts.len() as u32);
        for (&handle, slot) in &self.hosts {
            let (active, progress, waiting) = slot.snapshot();
            for value in [handle, active.map_or(NONE, |a| a.actor), progress.to_bits(), waiting.len() as u32] {
                mix(value);
            }
            for trader in waiting { mix(trader.actor); }
        }
    }

    fn decode(game: &Game, bytes: &[u8]) -> Option<Self> {
        let mut words = bytes.chunks_exact(4).map(|word| u32::from_le_bytes(word.try_into().unwrap()));
        let mut state = Self::default();
        let count = words.next()? as usize;
        if count > MAXE { return None; }
        for _ in 0..count {
            let actor = words.next()?;
            let home = words.next()?;
            let destination = words.next()?;
            let flags = words.next()?;
            let quote = Quote { cargo: words.next()?, gold: f32::from_bits(words.next()?), tech: f32::from_bits(words.next()?) };
            let unit = &game.ents[game.get(actor)?];
            if !is_trader(unit.def) || home == destination || flags > 3 ||
                quote.cargo >= 20 || !(1.0..=150.0).contains(&quote.gold) || ![0.0, 1.0].contains(&quote.tech) {
                return None;
            }
            for handle in [home, destination] {
                if handle == NONE || (handle & 0xffff) as usize >= MAXE { return None; }
                if let Some(index) = game.get(handle) {
                    if game.ents[index].def != host_for(unit.def) || game.ents[index].built < 1.0 { return None; }
                }
            }
            let at_host = flags & 2 != 0;
            if at_host && unit.order != O_IDLE { return None; }
            let route = Route { home, destination, returning: flags & 1 != 0, at_host, quote };
            if state.routes.last_key_value().is_some_and(|(&last, _)| last >= actor) { return None; }
            state.routes.insert(actor, route);
        }
        let count = words.next()? as usize;
        if count > state.routes.len() { return None; }
        let mut registered = BTreeSet::new();
        for _ in 0..count {
            let host = words.next()?;
            let active = words.next()?;
            let progress = f32::from_bits(words.next()?);
            let waiting = words.next()? as usize;
            if !progress.is_finite() || progress < 0.0 || waiting >= state.routes.len() ||
                state.hosts.last_key_value().is_some_and(|(&last, _)| last >= host) {
                return None;
            }
            game.get(host)?;
            let mut read_trader = |actor| {
                let route = state.routes.get(&actor)?;
                if !route.at_host || route.host() != host || !registered.insert(actor) { return None; }
                let owner = game.ents[game.get(actor)?].team;
                Some(Trader { actor, owner })
            };
            let active = read_trader(active)?;
            let waiting = (0..waiting).map(|_| read_trader(words.next()?)).collect::<Option<Vec<_>>>()?;
            state.hosts.insert(host, Queue::restore(active, progress, waiting));
        }
        if words.next().is_some() || registered.len() != state.routes.values().filter(|route| route.at_host).count() { return None; }
        Some(state)
    }

    fn cancel(&mut self, handle: u32) {
        if let Some(route) = self.routes.remove(&handle).filter(|route| route.at_host) {
            if let Some(slot) = self.hosts.get_mut(&route.host()) {
                slot.remove(handle, |actor| self.routes.contains_key(&actor));
            }
        }
        self.hosts.retain(|_, slot| !slot.is_empty());
    }

    fn stop(&mut self, game: &mut Game, handle: u32) {
        self.cancel(handle);
        if let Some(index) = game.get(handle) { game.order_stop(&[index]); }
    }

    fn update(&mut self, game: &mut Game) {
        self.update_hosts(game);
        let mut ordered: Vec<_> = self.routes.keys().copied().collect();
        ordered.sort_unstable_by_key(|handle| handle & 0xffff);
        for handle in ordered {
            let Some(index) = game.get(handle) else {
                self.cancel(handle);
                continue;
            };
            let mut route = self.routes[&handle];
            let unit = game.ents[index];
            let Some(destination) = game.trade_host(unit.team, unit.def, route.destination, false) else {
                if route.at_host && self.loading(handle, route) && game.get(route.host()).is_some() { continue; }
                self.stop(game, handle);
                continue;
            };
            let mut home = game.trade_host(unit.team, unit.def, route.home, true);
            if home.is_none() && !route.at_host {
                if let Some(replacement) = game.trade_replacement(unit, route.destination) {
                    route.home = replacement;
                    self.routes.insert(handle, route);
                    home = game.get(replacement);
                    if route.returning {
                        if let Some(port) = home.and_then(|i| game.trade_port(i, unit.def)) {
                            game.order_move(&[index], port.0, port.1, false);
                        }
                    }
                }
            }
            let Some(home) = home else {
                if route.at_host && self.loading(handle, route) { continue; }
                self.stop(game, handle);
                continue;
            };
            if route.at_host { continue; }
            let host = if route.returning { home } else { destination };
            let Some(port) = game.trade_port(host, unit.def) else {
                self.stop(game, handle);
                continue;
            };
            let distance_sq = (unit.x - port.0).powi(2) + (unit.y - port.1).powi(2);
            if distance_sq <= 0.35 * 0.35 {
                game.order_stop(&[index]);
                route.at_host = true;
                self.routes.insert(handle, route);
                self.hosts.entry(route.host()).or_default().arrive(Trader { actor: handle, owner: unit.team }, true);
            } else if unit.order == O_IDLE || (unit.ox, unit.oy) != port {
                game.order_move(&[index], port.0, port.1, false);
            }
        }
    }

    fn loading(&self, handle: u32, route: Route) -> bool {
        self.hosts.get(&route.host()).is_some_and(|slot| slot.active().is_some_and(|a| a.actor == handle))
    }

    fn update_hosts(&mut self, game: &mut Game) {
        let mut hosts = std::mem::take(&mut self.hosts);
        for (&host, slot) in &mut hosts {
            let Some(index) = game.get(host).filter(|&i| game.ents[i].hp > 0.0) else {
                for actor in slot.actors() { self.stop(game, actor); }
                slot.clear();
                continue;
            };
            let active = slot.active().and_then(|a| game.get(a.actor).map(|i| (a, i)));
            let Some((trader, actor)) = active else {
                if let Some(stale) = slot.active() { slot.remove(stale.actor, |id| game.get(id).is_some()); }
                continue;
            };
            let unit = game.ents[actor];
            let mut route = self.routes[&trader.actor];
            let destination = game.trade_host(unit.team, unit.def, route.destination, false);
            let hostile = destination.is_none() || game.hostile(unit.team, game.ents[index].team);
            let exit = game.trade_port(index, unit.def);
            let decision = slot.update(game.sim_step(), game.trade_load_time(unit.team), true, hostile, exit.is_some(),
                |id| game.get(id).is_some());
            let Some(decision) = decision else { continue; };
            let (x, y) = exit.unwrap();
            let e = &mut game.ents[actor];
            e.x = x;
            e.y = y;
            e.px = x;
            e.py = y;
            let home = game.trade_host(unit.team, unit.def, route.home, true)
                .or_else(|| game.trade_replacement(unit, route.destination).and_then(|h| game.get(h)));
            if matches!(decision, Exit::Loaded(_)) {
                if let (Some(_), Some(destination)) = (home, game.get(route.destination)) {
                    game.trade_delivery(unit.team, game.ents[destination].team, !route.returning, route.quote);
                }
            }
            let (Some(home), Some(destination)) = (home, destination) else {
                self.stop(game, trader.actor);
                continue;
            };
            if matches!(decision, Exit::Hostile(_)) {
                self.stop(game, trader.actor);
                continue;
            }
            route.home = game.h(home);
            let cargo_owner = if route.returning { unit.team } else { game.ents[destination].team };
            route.quote = game.trade_quote(actor, home, destination, cargo_owner);
            route.returning = !route.returning;
            route.at_host = false;
            let next = if route.returning { home } else { destination };
            if let Some((x, y)) = game.trade_port(next, unit.def) {
                game.order_move(&[actor], x, y, false);
                self.routes.insert(trader.actor, route);
            } else {
                self.stop(game, trader.actor);
            }
        }
        hosts.retain(|_, slot| !slot.is_empty());
        self.hosts = hosts;
    }
}

#[cfg(test)]
#[path = "trade30_tests.rs"]
mod tests;

