//! Stateful airport stationing, finite aircraft ammunition and mission orders.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::air31_defs::{mission_aircraft, FIGHTER_BOMBER};
use crate::carrier36_defs::{self, CARRIER};
use crate::defs::{DefId, AIRFIELD, DEFS};
use crate::game::{Ent, Game, MAXE, NONE, O_ATTACK, O_IDLE, O_RENDEZVOUS};
use crate::map::MapLayout;
use crate::native_rng9::NativeRng;

const AIRPORT_CAPACITY: usize = 10;
pub(crate) const MAX_WAYPOINTS: usize = 5;
const ARRIVAL_DISTANCE: f32 = 2.5;
const TARGET_RANGE: f32 = 5.0;
pub(crate) const SET_ROUTE: i32 = 0;
pub(crate) const CANCEL: i32 = 1;
pub(crate) const SCRAMBLE: i32 = 2;
pub(crate) const ADD: i32 = 3;
pub(crate) const RECALL: i32 = 4;
pub(crate) const AUTO: i32 = 5;
pub(crate) const SAVE_FLAG: u32 = 1 << 6;
const MAGIC: u32 = u32::from_le_bytes(*b"AIR1");
const VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Flying,
    Stationed,
    Returning,
}

#[derive(Clone, Copy)]
enum ReturnPath {
    Direct,
    ReverseMission,
    AfterCurrent,
}

#[derive(Clone, Debug)]
struct Aircraft {
    home: u32,
    ordnance: u16,
    phase: Phase,
    waypoints: VecDeque<[f32; 2]>,
    mission: bool,
    return_pending: bool,
    rendezvous_pending: bool,
}

#[derive(Clone, Default, Debug)]
pub(crate) struct Mission {
    pub(crate) waypoints: Vec<[f32; 2]>,
    pub(crate) target: Option<u32>,
}

#[repr(C)]
pub struct VisualRoute {
    host: u32,
    target: u32,
    count: u32,
    team: u32,
    points: [f32; MAX_WAYPOINTS * 2],
}

const _: [(); 56] = [(); std::mem::size_of::<VisualRoute>()];

#[derive(Clone, Copy, Eq, PartialEq)]
enum Launch {
    Scramble,
    Mission,
}

struct Airport {
    registered: Vec<u32>,
    stored: Vec<u32>,
    takeoff: VecDeque<(u32, Launch)>,
    mission: Option<Mission>,
    automatic: bool,
}

impl Default for Airport {
    fn default() -> Self {
        Self { registered: Vec::new(), stored: Vec::new(), takeoff: VecDeque::new(), mission: None, automatic: true }
    }
}

#[derive(Default)]
pub(crate) struct State {
    aircraft: BTreeMap<u32, Aircraft>,
    airports: BTreeMap<u32, Airport>,
}

fn capacity(definition: DefId, epoch: u8) -> Option<u16> {
    if !mission_aircraft(definition) { return None; }
    crate::ee2_up15_attack::lookup(definition, epoch).map(|attack| attack.ordnance).filter(|&n| n != 0)
}

fn reached(actor: &Ent, [x, y]: [f32; 2]) -> bool {
    (actor.x - x).powi(2) + (actor.y - y).powi(2) <= ARRIVAL_DISTANCE * ARRIVAL_DISTANCE
}

fn launch_jitter(rng: &mut NativeRng) -> f32 {
    rng.next_u32();
    (f64::from(rng.next_word15()) * f64::from(f32::from_bits(0x39000100)) - 2.0) as f32
}

/// The source uses the full source ID in 008416C0. Imported starts will
/// provide it through the actor ledger. Own starts use their stable handle
/// and seat as the phase key until that ledger is available in the match.
fn actor_pulse(tick: u32, phase_key: u32, seconds: f64, dt: f32) -> bool {
    let period = (seconds / f64::from(dt)).ceil() as i32;
    (tick.wrapping_add(phase_key) as i32) % period == 0
}

fn phase_key(handle: u32, team: u8) -> u32 {
    handle | (u32::from(team) + 1) << 27
}

impl Game {
    pub(crate) fn configure_peer_air_carrier(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() ||
            !self.inbox.is_empty() || (enabled && !self.air_landing_enabled) { return false; }
        self.air_carrier_enabled = enabled;
        true
    }

    pub(crate) fn configure_peer_air_missions(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() || !self.inbox.is_empty() {
            return false;
        }
        self.air_missions_enabled = enabled;
        if !enabled {
            self.air_runway_enabled = false;
            self.set_air_takeoff_enabled(false);
        }
        true
    }

    pub(crate) fn air_production_enabled(&self, definition: DefId) -> bool {
        match definition {
            FIGHTER_BOMBER => self.air_missions_enabled,
            CARRIER => self.air_carrier_enabled,
            _ => true,
        }
    }

    pub(crate) fn air_host_slots(&self, host: &Ent) -> usize {
        match host.def {
            AIRFIELD => AIRPORT_CAPACITY,
            CARRIER if self.air_carrier_enabled => carrier36_defs::lookup(self.players[host.team as usize].epoch)
                .and_then(|row| row.garrison_slots).map_or(0, usize::from),
            _ => 0,
        }
    }

    pub(crate) fn air_mission_status(&self, handle: u32, field: u32) -> u32 {
        if !self.air_missions_enabled { return 0; }
        let Some(index) = self.get(handle) else { return 0; };
        let actor = &self.ents[index];
        if actor.hp <= 0.0 || actor.team != self.local { return 0; }
        if self.air_mission_host(handle, actor.team).is_some() {
            let airport = self.aircraft.airports.get(&handle);
            return match field {
                0 => 1,
                1 => airport.map_or(0, |a| a.stored.len() as u32),
                2 => self.air_host_slots(actor) as u32,
                3 => u32::from(airport.is_some_and(|a| a.mission.is_some())),
                4 => u32::from(airport.is_none_or(|a| a.automatic)),
                6 => u32::from(actor.built >= 1.0),
                _ => 0,
            };
        }
        let Some(frame) = self.aircraft.aircraft.get(&handle) else { return 0; };
        match field {
            0 => 2,
            1 => frame.ordnance.into(),
            2 => actor_capacity(self, &actor).unwrap_or(0).into(),
            3 => frame.mission.into(),
            5 => u32::from(frame.phase != Phase::Stationed && self.air_host(frame.home, actor.team).is_some()),
            _ => 0,
        }
    }

    pub(crate) fn air_mission_visual_route(&self, host: u32) -> Option<VisualRoute> {
        if !self.air_missions_enabled { return None; }
        let index = self.air_mission_host(host, self.local)?;
        let team = self.ents[index].team;
        let mission = self.aircraft.airports.get(&host).and_then(|airport| airport.mission.as_ref());
        let mut route = VisualRoute { host, target: NONE, count: 0, team: team.into(), points: [0.0; MAX_WAYPOINTS * 2] };
        if let Some(mission) = mission {
            route.count = mission.waypoints.len() as u32;
            for (at, point) in mission.waypoints.iter().enumerate() {
                route.points[at * 2..at * 2 + 2].copy_from_slice(point);
            }
            route.target = mission.target.filter(|&target| self.get(target).is_some_and(|index| {
                self.ents[index].hp > 0.0 && self.can_see(team, index)
            })).unwrap_or(NONE);
        }
        Some(route)
    }

    pub(crate) fn air_valid_target(&self, team: u8, handle: u32) -> bool {
        self.get(handle).is_some_and(|i| {
            let actor = &self.ents[i];
            actor.hp > 0.0 && !self.air_parked(i, actor) && !self.trade_hidden(i, actor) &&
                (actor.team == team || self.real_fog(team, self.map.tile_of(actor.x, actor.y)) >=
                    if crate::defs::DEFS[actor.def as usize].building { 128 } else { 255 }) &&
                self.detected(team, i)
        })
    }

    pub(crate) fn air_command(&mut self, team: u8, a: f32, b: f32, action: i32, words: &[u32]) {
        if !self.air_missions_enabled { return; }
        if action == SET_ROUTE {
            let points = a as usize;
            let selected = b as usize;
            if a < 0.0 || b < 1.0 || a.fract() != 0.0 || b.fract() != 0.0 || points > MAX_WAYPOINTS ||
                selected > 200 || words.len() != selected + 1 + points * 2 { return; }
            let target = words[selected];
            let target = (target != NONE).then_some(target);
            if target.is_some_and(|target| !self.air_valid_target(team, target)) || (points == 0 && target.is_none()) { return; }
            let waypoints = words[selected + 1..].chunks_exact(2).map(|point| {
                let point = [f32::from_bits(point[0]), f32::from_bits(point[1])];
                self.map.layout.contains_point(point[0], point[1]).then_some(point)
            }).collect::<Option<Vec<_>>>();
            let Some(waypoints) = waypoints else { return; };
            let mut hosts = words[..selected].iter().filter_map(|&h| self.air_mission_host(h, team)).collect::<Vec<_>>();
            hosts.sort_unstable();
            hosts.dedup();
            for host in hosts { self.air_assign(host, Some(Mission { waypoints: waypoints.clone(), target })); }
            return;
        }
        if a != 0.0 || b != 0.0 { return; }
        let mut actors = words.iter().filter_map(|&handle| self.get(handle))
            .filter(|&index| self.ents[index].team == team && self.ents[index].hp > 0.0).collect::<Vec<_>>();
        actors.sort_unstable();
        actors.dedup();
        for index in actors {
            let host = self.air_host(self.h(index), team).is_some();
            match action {
                CANCEL => { self.air_assign(index, None); }
                SCRAMBLE if host => self.air_scramble(index),
                ADD if host => self.air_add_to_mission(index),
                ADD => self.air_enroll_aircraft(index),
                AUTO if host => self.air_toggle_automatic(index),
                RECALL => self.air_recall(index),
                _ => {}
            }
        }
    }

    pub(crate) fn air_register(&mut self, index: usize) {
        if !self.air_missions_enabled { return; }
        let actor = self.ents[index];
        let Some(ordnance) = actor_capacity(self, &actor) else { return; };
        self.aircraft.aircraft.insert(self.h(index), Aircraft {
            home: NONE, ordnance, phase: Phase::Flying, waypoints: VecDeque::new(), mission: false,
            return_pending: false, rendezvous_pending: false,
        });
    }

    fn air_rendezvous(&mut self, host: usize) {
        if self.ents[host].def != CARRIER || self.ents[host].order == O_RENDEZVOUS { return; }
        self.order_stop(&[host]);
        let (x, y) = (self.ents[host].x, self.ents[host].y);
        let key = self.key_for(crate::defs::SEA, x, y);
        let carrier = &mut self.ents[host];
        carrier.order = O_RENDEZVOUS;
        carrier.fkey = key;
        carrier.otgt = NONE;
    }

    pub(crate) fn air_station_produced(&mut self, index: usize, airport: usize) -> bool {
        if !self.air_missions_enabled || !mission_aircraft(self.ents[index].def) || self.ents[airport].def != AIRFIELD {
            return false;
        }
        let handle = self.h(index);
        let home = self.h(airport);
        let mut air = std::mem::take(&mut self.aircraft);
        let stationed = air.station(self, handle, home);
        self.aircraft = air;
        stationed
    }

    fn air_mission_host(&self, handle: u32, team: u8) -> Option<usize> {
        let index = self.get(handle)?;
        let e = &self.ents[index];
        (e.hp > 0.0 && e.team == team && self.air_host_slots(e) != 0 && !self.dock_parked(index, e)).then_some(index)
    }

    pub(crate) fn air_cargo_for_dock(&self, handle: u32) -> Option<u32> {
        let Some(home) = self.aircraft.airports.get(&handle) else { return Some(0); };
        (home.registered.len() == home.stored.len()).then_some(home.stored.len() as u32)
    }

    fn air_host(&self, handle: u32, team: u8) -> Option<usize> {
        self.air_mission_host(handle, team).filter(|&index| self.ents[index].built >= 1.0)
    }

    pub(crate) fn air_parked(&self, index: usize, actor: &Ent) -> bool {
        self.aircraft.is_parked(index, actor)
    }

    pub(crate) fn air_landing_home(&self, handle: u32) -> Option<u32> {
        let frame = self.aircraft.aircraft.get(&handle)?;
        if frame.phase != Phase::Returning || frame.waypoints.len() != 1 { return None; }
        let index = self.get(handle)?;
        self.air_host(frame.home, self.ents[index].team)?;
        Some(frame.home)
    }

    pub(crate) fn air_audio_returning(&self, handle: u32) -> bool {
        self.aircraft.aircraft.get(&handle).is_some_and(|frame| frame.phase == Phase::Returning)
    }

    pub(crate) fn air_can_fire(&self, index: usize) -> bool {
        self.aircraft.aircraft.get(&self.h(index)).is_none_or(|a| a.phase == Phase::Flying && a.ordnance > 0)
    }

    pub(crate) fn air_fire(&mut self, index: usize, has_missile_name: bool) {
        let handle = self.h(index);
        if let Some(frame) = self.aircraft.aircraft.get_mut(&handle) {
            if has_missile_name && frame.ordnance == 1 { frame.return_pending = true; }
            frame.ordnance = frame.ordnance.saturating_sub(1);
        }
    }

    pub(crate) fn air_interrupt(&mut self, index: usize) {
        if !self.air_missions_enabled || !mission_aircraft(self.ents[index].def) { return; }
        let handle = self.h(index);
        if let Some(frame) = self.aircraft.aircraft.get_mut(&handle).filter(|frame| frame.phase != Phase::Stationed) {
            self.landing.clear(handle);
            frame.phase = Phase::Flying;
            frame.mission = false;
            frame.rendezvous_pending = false;
            frame.waypoints.clear();
        }
    }

    pub(crate) fn air_remove(&mut self, index: usize) {
        if !self.air_missions_enabled { return; }
        let handle = self.h(index);
        self.takeoff.clear(handle);
        self.landing.clear(handle);
        self.air_audio.forget(handle);
        self.aircraft.aircraft.remove(&handle);
        for airport in self.aircraft.airports.values_mut() {
            airport.registered.retain(|&actor| actor != handle);
            airport.stored.retain(|&actor| actor != handle);
            airport.takeoff.retain(|&(actor, _)| actor != handle);
        }
        if self.aircraft.airports.contains_key(&handle) {
            let mut air = std::mem::take(&mut self.aircraft);
            air.lost_host(self, handle);
            self.aircraft = air;
        }
    }

    pub(crate) fn air_order_land(&mut self, aircraft: &[usize], airport: usize) {
        if !self.air_missions_enabled || self.air_host_slots(&self.ents[airport]) == 0 { return; }
        let home = self.h(airport);
        let mut air = std::mem::take(&mut self.aircraft);
        for &index in aircraft {
            let handle = self.h(index);
            air.return_to(self, handle, home, ReturnPath::Direct);
        }
        self.aircraft = air;
    }

    pub(crate) fn air_assign(&mut self, host: usize, mission: Option<Mission>) -> bool {
        if !self.air_missions_enabled { return false; }
        let handle = self.h(host);
        if self.air_mission_host(handle, self.ents[host].team).is_none() { return false; }
        let mut air = std::mem::take(&mut self.aircraft);
        air.assign(self, handle, mission);
        self.aircraft = air;
        true
    }

    pub(crate) fn air_scramble(&mut self, host: usize) {
        if !self.air_missions_enabled { return; }
        let handle = self.h(host);
        if let Some(airport) = self.aircraft.airports.get_mut(&handle) {
            airport.queue(Launch::Scramble);
        }
    }

    pub(crate) fn air_add_to_mission(&mut self, host: usize) {
        if !self.air_missions_enabled { return; }
        let handle = self.h(host);
        if let Some(airport) = self.aircraft.airports.get_mut(&handle).filter(|a| a.mission.is_some()) {
            airport.queue(Launch::Mission);
        }
    }

    pub(crate) fn air_enroll_aircraft(&mut self, index: usize) {
        let handle = self.h(index);
        let mut air = std::mem::take(&mut self.aircraft);
        air.enroll(self, handle);
        self.aircraft = air;
    }

    pub(crate) fn air_toggle_automatic(&mut self, host: usize) {
        if !self.air_missions_enabled { return; }
        let handle = self.h(host);
        if self.air_host(handle, self.ents[host].team).is_some() {
            let airport = self.aircraft.airports.entry(handle).or_default();
            airport.automatic = !airport.automatic;
        }
    }

    pub(crate) fn air_recall(&mut self, index: usize) {
        if !self.air_missions_enabled { return; }
        let handle = self.h(index);
        let mut air = std::mem::take(&mut self.aircraft);
        if self.air_host_slots(&self.ents[index]) != 0 {
            air.recall(self, handle);
        } else if let Some(frame) = air.aircraft.get(&handle) {
            let home = air.available_home(self, self.ents[index], frame.home);
            air.return_to(self, handle, home, ReturnPath::ReverseMission);
        }
        self.aircraft = air;
    }

    pub(crate) fn tick_aircraft(&mut self) {
        if !self.air_missions_enabled { return; }
        let mut air = std::mem::take(&mut self.aircraft);
        air.update(self);
        self.aircraft = air;
    }

    pub(crate) fn air_transferred_assets(&mut self, changed: &[(usize, u8)]) {
        if !self.air_missions_enabled { return; }
        for &(index, _) in changed {
            let handle = self.h(index);
            self.landing.clear(handle);
            self.air_audio.forget(handle);
            if let Some(airport) = self.aircraft.airports.get_mut(&handle) {
                self.landing.clear_host(handle);
                airport.mission = None;
                airport.takeoff.clear();
            }
            if let Some(aircraft) = self.aircraft.aircraft.get_mut(&handle) {
                aircraft.mission = false;
                aircraft.rendezvous_pending = false;
                aircraft.waypoints.clear();
                if aircraft.phase == Phase::Returning { aircraft.phase = Phase::Flying; }
            }
        }
        let mut state = std::mem::take(&mut self.aircraft);
        state.reconcile_owners(self);
        self.aircraft = state;
    }

    pub(crate) fn wrap_air_save(&self, core: Vec<u8>) -> Vec<u8> {
        if !self.air_missions_enabled || core.is_empty() { return core; }
        let has_jobs = self.ents[..self.high].iter().any(|e| e.alive &&
            crate::defs::DEFS[e.def as usize].building && e.q[..e.qn as usize].contains(&(FIGHTER_BOMBER as _)));
        if !has_jobs && self.aircraft.aircraft.is_empty() && self.aircraft.airports.is_empty() { return core; }
        let mut out = Vec::with_capacity(core.len() + 64);
        for value in [MAGIC, VERSION, core.len() as u32] { out.extend_from_slice(&value.to_le_bytes()); }
        self.aircraft.fields(|value| out.extend_from_slice(&value.to_le_bytes()));
        out.extend_from_slice(&core);
        out.extend_from_slice(&crate::pack8_save::checksum(&out).to_le_bytes());
        out
    }

    pub(crate) fn restore_air_save(&mut self, fields: Option<&[u8]>) -> Option<()> {
        if let Some(fields) = fields {
            if !self.air_missions_enabled { return None; }
            self.aircraft = State::decode(self, fields)?;
        } else if self.air_missions_enabled && self.ents[..self.high].iter().any(|actor| actor.alive &&
            actor_capacity(self, actor).is_some()) {
            return None;
        }
        Some(())
    }
}

fn actor_capacity(game: &Game, actor: &Ent) -> Option<u16> {
    capacity(actor.def, game.players.get(actor.team as usize)?.epoch)
}

pub(crate) fn split_air_save(bytes: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    let word = |start: usize| Some(u32::from_le_bytes(bytes.get(start..start + 4)?.try_into().ok()?));
    if word(0) != Some(MAGIC) { return Some((bytes, None)); }
    if word(4)? != VERSION { return None; }
    let end = bytes.len().checked_sub(4)?;
    let start = end.checked_sub(word(8)? as usize)?;
    let fields = bytes.get(12..start)?;
    if fields.len() % 4 != 0 || fields.len() > MAXE * 42 * 4 { return None; }
    if crate::pack8_save::checksum(&bytes[..end]) != word(end)? { return None; }
    Some((bytes.get(start..end)?, Some(fields)))
}

fn read_points(layout: MapLayout, words: &mut impl Iterator<Item = u32>, count: usize, limit: usize) -> Option<Vec<[f32; 2]>> {
    if count > limit { return None; }
    (0..count).map(|_| {
        let point = [f32::from_bits(words.next()?), f32::from_bits(words.next()?)];
        layout.contains_point(point[0], point[1]).then_some(point)
    }).collect()
}

impl Airport {
    fn queue(&mut self, kind: Launch) {
        for &handle in &self.stored {
            if self.takeoff.iter().all(|&(queued, _)| queued != handle) {
                self.takeoff.push_back((handle, kind));
            }
        }
    }
}

impl State {
    pub(crate) fn is_active(&self, handle: u32) -> bool {
        self.aircraft.get(&handle).is_some_and(|frame| frame.phase != Phase::Stationed)
    }

    pub(crate) fn is_parked(&self, index: usize, actor: &Ent) -> bool {
        if !mission_aircraft(actor.def) { return false; }
        let handle = (u32::from(actor.gen) << 16) | index as u32;
        self.aircraft.get(&handle).is_some_and(|frame| frame.phase == Phase::Stationed)
    }

    pub(crate) fn hash(&self, mut mix: impl FnMut(u32)) {
        mix(MAGIC);
        self.fields(mix);
    }

    fn fields(&self, mut add: impl FnMut(u32)) {
        add(self.aircraft.len() as u32);
        for (&handle, aircraft) in &self.aircraft {
            let phase = match aircraft.phase { Phase::Flying => 0, Phase::Stationed => 1, Phase::Returning => 2 };
            let flags = phase | (u32::from(aircraft.mission) << 2) | (u32::from(aircraft.return_pending) << 3) |
                (u32::from(aircraft.rendezvous_pending) << 4);
            for word in [handle, aircraft.home, aircraft.ordnance.into(), flags,
                aircraft.waypoints.len() as u32] { add(word); }
            for point in &aircraft.waypoints {
                for &coord in point { add(coord.to_bits()); }
            }
        }
        add(self.airports.len() as u32);
        for (&handle, airport) in &self.airports {
            let mission = airport.mission.as_ref();
            for word in [handle, u32::from(airport.automatic), airport.registered.len() as u32,
                airport.stored.len() as u32, airport.takeoff.len() as u32,
                mission.and_then(|m| m.target).unwrap_or(NONE), mission.map_or(0, |m| m.waypoints.len() as u32)] {
                add(word);
            }
            if let Some(mission) = mission {
                for point in &mission.waypoints {
                    for &coord in point { add(coord.to_bits()); }
                }
            }
            for &actor in &airport.registered { add(actor); }
            for &actor in &airport.stored { add(actor); }
            for &(actor, launch) in &airport.takeoff {
                add(actor);
                add(u32::from(launch == Launch::Mission));
            }
        }
    }

    fn decode(game: &Game, fields: &[u8]) -> Option<Self> {
        let mut words = fields.chunks_exact(4).map(|word| u32::from_le_bytes(word.try_into().unwrap()));
        let mut state = Self::default();
        let count = words.next()? as usize;
        if count > MAXE { return None; }
        for _ in 0..count {
            let handle = words.next()?;
            let home = words.next()?;
            let ordnance = u16::try_from(words.next()?).ok()?;
            let flags = words.next()?;
            let points = words.next()? as usize;
            let phase = match flags & 3 { 0 => Phase::Flying, 1 => Phase::Stationed, 2 => Phase::Returning, _ => return None };
            let mission = flags & 4 != 0;
            let return_pending = flags & 8 != 0;
            let rendezvous_pending = flags & 16 != 0;
            let unit = &game.ents[game.get(handle)?];
            if flags > 31 || ordnance > 32 || !mission_aircraft(unit.def) ||
                (phase != Phase::Flying && mission) ||
                (return_pending && (ordnance != 0 || phase != Phase::Flying)) ||
                (rendezvous_pending && (phase != Phase::Returning || points < 2)) ||
                (phase == Phase::Stationed && (unit.order != O_IDLE || points != 0)) ||
                (phase == Phase::Returning && points == 0) ||
                (home == NONE && (phase != Phase::Flying || mission)) ||
                state.aircraft.last_key_value().is_some_and(|(&last, _)| last >= handle) {
                return None;
            }
            if home != NONE {
                let host = game.air_host(home, unit.team)?;
                if rendezvous_pending && game.ents[host].def != CARRIER { return None; }
            }
            let waypoints = read_points(game.map.layout, &mut words, points, MAX_WAYPOINTS + 2)?.into();
            state.aircraft.insert(handle, Aircraft { home, ordnance, phase, waypoints, mission, return_pending, rendezvous_pending });
        }
        let count = words.next()? as usize;
        if count > MAXE { return None; }
        let mut parked = BTreeSet::new();
        let mut assigned = BTreeSet::new();
        for _ in 0..count {
            let handle = words.next()?;
            let auto = words.next()?;
            let assigned_count = words.next()? as usize;
            let stored_count = words.next()? as usize;
            let queue_count = words.next()? as usize;
            let target = words.next()?;
            let points = words.next()? as usize;
            let slots = game.air_host_slots(&game.ents[game.get(handle)?]);
            if auto > 1 || slots == 0 || assigned_count > slots || stored_count > assigned_count || queue_count > stored_count ||
                state.airports.last_key_value().is_some_and(|(&last, _)| last >= handle) { return None; }
            let target = (target != NONE).then_some(target);
            if target.is_some_and(|target| (target & 0xffff) as usize >= MAXE) { return None; }
            let waypoints = read_points(game.map.layout, &mut words, points, MAX_WAYPOINTS)?;
            let mission = (target.is_some() || !waypoints.is_empty()).then_some(Mission { waypoints, target });
            let registered = (0..assigned_count).map(|_| {
                let actor = words.next()?;
                let frame = state.aircraft.get(&actor)?;
                (frame.home == handle && assigned.insert(actor)).then_some(actor)
            }).collect::<Option<Vec<_>>>()?;
            let stored = (0..stored_count).map(|_| {
                let actor = words.next()?;
                let frame = state.aircraft.get(&actor)?;
                (frame.phase == Phase::Stationed && registered.contains(&actor) && parked.insert(actor)).then_some(actor)
            }).collect::<Option<Vec<_>>>()?;
            let mut queued = BTreeSet::new();
            let takeoff = (0..queue_count).map(|_| {
                let actor = words.next()?;
                let launch = match words.next()? {
                    0 => Launch::Scramble,
                    1 if mission.is_some() => Launch::Mission,
                    _ => return None,
                };
                (stored.contains(&actor) && queued.insert(actor)).then_some((actor, launch))
            }).collect::<Option<VecDeque<_>>>()?;
            state.airports.insert(handle, Airport { registered, stored, takeoff, mission, automatic: auto == 1 });
        }
        if words.next().is_some() { return None; }
        for (&handle, aircraft) in &state.aircraft {
            if (aircraft.phase == Phase::Stationed) != parked.contains(&handle) ||
                (aircraft.home != NONE) != assigned.contains(&handle) ||
                (aircraft.mission && !state.airports.get(&aircraft.home).is_some_and(|a| a.mission.is_some())) { return None; }
        }
        if game.ents[..game.high].iter().enumerate().any(|(index, unit)| {
            unit.alive && actor_capacity(game, unit).is_some() &&
                !state.aircraft.contains_key(&game.h(index))
        }) { return None; }
        Some(state)
    }

    fn station(&mut self, game: &mut Game, handle: u32, home: u32) -> bool {
        let Some(index) = game.get(handle) else { return false; };
        let Some(host) = game.air_host(home, game.ents[index].team) else { return false; };
        if self.aircraft.get(&handle).is_none_or(|frame| frame.phase == Phase::Stationed) ||
            !self.reserve(game, host, home, handle) { return false; }
        game.observe_air_parking_audio(index);
        let frame = self.aircraft.get_mut(&handle).unwrap();
        frame.phase = Phase::Stationed;
        frame.mission = false;
        frame.return_pending = false;
        frame.rendezvous_pending = false;
        frame.waypoints.clear();
        game.takeoff.clear(handle);
        game.landing.clear(handle);
        self.airports.get_mut(&home).unwrap().stored.push(handle);
        let (x, y) = (game.ents[host].x, game.ents[host].y);
        game.order_stop(&[index]);
        let actor = &mut game.ents[index];
        (actor.x, actor.y, actor.px, actor.py) = (x, y, x, y);
        (actor.vx, actor.vy) = (0.0, 0.0);
        true
    }

    fn reserve(&mut self, game: &Game, host: usize, home: u32, handle: u32) -> bool {
        let old = self.aircraft[&handle].home;
        let airport = self.airports.entry(home).or_default();
        if airport.registered.contains(&handle) { return true; }
        if airport.registered.len() >= game.air_host_slots(&game.ents[host]) { return false; }
        airport.registered.push(handle);
        if let Some(previous) = self.airports.get_mut(&old) {
            previous.registered.retain(|&actor| actor != handle);
        }
        self.aircraft.get_mut(&handle).unwrap().home = home;
        true
    }

    fn lost_host(&mut self, game: &mut Game, home: u32) {
        let Some(airport) = self.airports.remove(&home) else { return; };
        for handle in airport.stored {
            if let Some(index) = game.get(handle) {
                self.aircraft.remove(&handle);
                game.kill(index);
            }
        }
        let Some(index) = game.get(home) else { return; };
        let origin = [game.ents[index].x, game.ents[index].y];
        for handle in airport.registered.into_iter().rev() {
            game.landing.clear(handle);
            let Some(index) = game.get(handle) else { continue; };
            let Some(frame) = self.aircraft.get_mut(&handle) else { continue; };
            frame.home = NONE;
            frame.phase = Phase::Flying;
            frame.mission = false;
            frame.rendezvous_pending = false;
            frame.waypoints.clear();
            let replacement = self.nearest_home(game, game.ents[index].team, origin, home);
            self.return_to(game, handle, replacement, ReturnPath::Direct);
        }
    }

    fn recall(&mut self, game: &mut Game, home: u32) {
        let mut participants: Vec<_> = self.aircraft.iter().filter_map(|(&handle, frame)| {
            (frame.home == home && frame.phase == Phase::Flying && frame.mission).then_some(handle)
        }).collect();
        participants.sort_unstable_by_key(|handle| std::cmp::Reverse(handle & 0xffff));
        for handle in participants { self.return_to(game, handle, home, ReturnPath::ReverseMission); }
        if let Some(airport) = self.airports.get_mut(&home) {
            airport.takeoff.retain(|&(_, kind)| kind != Launch::Mission);
        }
    }

    fn assign(&mut self, game: &mut Game, home: u32, mission: Option<Mission>) {
        if let Some(mission) = &mission {
            let participants: Vec<_> = self.aircraft.iter().filter_map(|(&handle, frame)| {
                (frame.home == home && frame.phase == Phase::Flying && frame.mission).then_some(handle)
            }).collect();
            for handle in participants {
                if self.aircraft[&handle].ordnance == 0 {
                    self.return_to(game, handle, home, ReturnPath::ReverseMission);
                } else if let Some(index) = game.get(handle) {
                    let frame = self.aircraft.get_mut(&handle).unwrap();
                    frame.waypoints = mission.waypoints.iter().copied().collect();
                    if let Some(&[x, y]) = frame.waypoints.front() {
                        game.order_move(&[index], x, y, false);
                    }
                }
            }
        } else {
            self.recall(game, home);
        }
        self.airports.entry(home).or_default().mission = mission;
    }

    fn enroll(&mut self, game: &mut Game, handle: u32) {
        let Some(index) = game.get(handle) else { return; };
        let Some(frame) = self.aircraft.get_mut(&handle) else { return; };
        let home = frame.home;
        let Some(airport) = self.airports.get_mut(&home) else { return; };
        let Some(mission) = airport.mission.as_ref() else { return; };
        if frame.phase == Phase::Stationed {
            if airport.takeoff.iter().all(|&(queued, _)| queued != handle) {
                airport.takeoff.push_back((handle, Launch::Mission));
            }
        } else if frame.phase == Phase::Flying {
            if frame.ordnance == 0 {
                self.return_to(game, handle, home, ReturnPath::ReverseMission);
                return;
            }
            frame.mission = true;
            frame.waypoints = mission.waypoints.iter().copied().collect();
            if let Some(&[x, y]) = frame.waypoints.front() {
                game.order_move(&[index], x, y, false);
            }
        }
    }

    fn available_home(&self, game: &Game, actor: Ent, prior: u32) -> u32 {
        if game.air_host(prior, actor.team).is_some() { return prior; }
        self.nearest_home(game, actor.team, [actor.x, actor.y], NONE)
    }

    fn nearest_home(&self, game: &Game, team: u8, [x, y]: [f32; 2], exclude: u32) -> u32 {
        let z = game.map.height(x, y);
        (0..game.high).filter_map(|index| {
            let handle = game.h(index);
            if handle == exclude { return None; }
            let airport = &game.ents[game.air_host(handle, team)?];
            if self.airports.get(&handle).is_some_and(|a| a.registered.len() >= game.air_host_slots(airport)) { return None; }
            let dx = f64::from(x) - f64::from(airport.x);
            let dy = f64::from(y) - f64::from(airport.y);
            let dz = f64::from(z) - f64::from(game.map.height(airport.x, airport.y));
            let distance = (dx * dx + dy * dy + dz * dz) as f32;
            Some((handle, distance))
        }).min_by(|(_, a), (_, b)| a.total_cmp(b)).map_or(NONE, |(handle, _)| handle)
    }

    fn return_to(&mut self, game: &mut Game, handle: u32, home: u32, path: ReturnPath) {
        let Some(index) = game.get(handle) else { return; };
        let Some(host) = game.air_host(home, game.ents[index].team) else { return; };
        let Some(frame) = self.aircraft.get(&handle).filter(|frame| frame.phase != Phase::Stationed) else { return; };
        if frame.phase == Phase::Returning && frame.home == home && !frame.rendezvous_pending { return; }
        let previous = frame.home;
        if !self.reserve(game, host, home, handle) { return; }
        game.landing.clear(handle);
        let frame = self.aircraft.get_mut(&handle).unwrap();
        if !matches!(path, ReturnPath::AfterCurrent) { frame.waypoints.clear(); }
        if matches!(path, ReturnPath::ReverseMission) && frame.mission {
            if let Some(mission) = self.airports.get(&previous).and_then(|a| a.mission.as_ref()) {
                frame.waypoints.extend(mission.waypoints.iter().rev().copied());
            }
        }
        frame.phase = Phase::Returning;
        frame.mission = false;
        frame.return_pending = false;
        // Event9 is delivered on replacement, or when an appended return is promoted.
        frame.rendezvous_pending = game.ents[host].def == CARRIER &&
            matches!(path, ReturnPath::AfterCurrent) && !frame.waypoints.is_empty();
        if !frame.rendezvous_pending { game.air_rendezvous(host); }
        let airport = &game.ents[host];
        frame.waypoints.push_back([airport.x, airport.y]);
        let [x, y] = frame.waypoints[0];
        game.order_move(&[index], x, y, false);
    }

    fn update(&mut self, game: &mut Game) {
        self.aircraft.retain(|&handle, _| game.get(handle).is_some());
        self.reconcile_owners(game);
        self.update_airports(game);
        let mut active: Vec<_> = self.aircraft.keys().copied().collect();
        active.sort_unstable_by_key(|handle| handle & 0xffff);
        for handle in active {
            let index = game.get(handle).unwrap();
            let actor = game.ents[index];
            let frame = &self.aircraft[&handle];
            let periodic_return = frame.phase == Phase::Flying && frame.ordnance == 0 &&
                actor_pulse(game.tick, phase_key(handle, actor.team), 2.0, game.sim_step());
            if frame.return_pending || periodic_return {
                let path = if frame.return_pending { ReturnPath::AfterCurrent } else { ReturnPath::Direct };
                let frame = self.aircraft.get_mut(&handle).unwrap();
                frame.return_pending = false;
                let prior = frame.home;
                let home = self.available_home(game, actor, prior);
                self.return_to(game, handle, home, path);
            }
            let frame = self.aircraft.get_mut(&handle).unwrap();
            if frame.phase == Phase::Stationed { continue; }
            if game.takeoff.rolling(handle) { continue; }
            if frame.home != NONE && game.air_host(frame.home, actor.team).is_none() {
                game.landing.clear(handle);
                let previous = frame.home;
                frame.home = NONE;
                frame.phase = Phase::Flying;
                frame.mission = false;
                frame.rendezvous_pending = false;
                frame.waypoints.clear();
                if let Some(airport) = self.airports.get_mut(&previous) {
                    airport.registered.retain(|&actor| actor != handle);
                }
                continue;
            }
            if frame.waypoints.is_empty() {
                if frame.mission {
                    self.approach_mission_target(game, handle, index);
                }
                continue;
            }
            if frame.phase == Phase::Returning && frame.waypoints.len() == 1 && game.use_runway_landing(index) {
                let home = frame.home;
                if game.landing.complete(handle, home) { self.station(game, handle, home); }
                continue;
            }
            if !reached(&actor, frame.waypoints[0]) { continue; }
            if frame.phase == Phase::Returning && frame.waypoints.len() == 1 {
                let home = frame.home;
                self.station(game, handle, home);
                continue;
            }
            frame.waypoints.pop_front();
            if frame.rendezvous_pending && frame.waypoints.len() == 1 {
                frame.rendezvous_pending = false;
                if let Some(host) = game.air_host(frame.home, actor.team) { game.air_rendezvous(host); }
            }
            if let Some(&[x, y]) = frame.waypoints.front() {
                game.order_move(&[index], x, y, false);
            } else if frame.mission {
                self.approach_mission_target(game, handle, index);
            }
        }
        if game.air_carrier_enabled {
            let inbound: BTreeSet<_> = self.aircraft.values().filter_map(|frame| {
                (frame.phase == Phase::Returning && !frame.rendezvous_pending).then_some(frame.home)
            }).collect();
            for &home in self.airports.keys() {
                let Some(host) = game.get(home) else { continue; };
                let carrier = &mut game.ents[host];
                if carrier.order == O_RENDEZVOUS && !inbound.contains(&home) &&
                    (carrier.x - carrier.ox).powi(2) + (carrier.y - carrier.oy).powi(2) <= 0.25 {
                    Game::set_idle(carrier);
                }
            }
        }
    }

    fn reconcile_owners(&mut self, game: &Game) {
        for (&handle, frame) in &mut self.aircraft {
            let Some(index) = game.get(handle) else { continue; };
            let owner = game.ents[index].team;
            let nested = frame.phase == Phase::Stationed && game.get(frame.home).is_some_and(|host| {
                let carrier = &game.ents[host];
                carrier.hp > 0.0 && carrier.team == owner && game.dock_parked(host, carrier)
            });
            if frame.home == NONE || nested || game.air_host(frame.home, owner).is_some() { continue; }
            if let Some(airport) = self.airports.get_mut(&frame.home) {
                airport.registered.retain(|&actor| actor != handle);
                airport.stored.retain(|&actor| actor != handle);
                airport.takeoff.retain(|&(actor, _)| actor != handle);
            }
            frame.home = NONE;
            frame.mission = false;
            frame.rendezvous_pending = false;
            frame.waypoints.clear();
            frame.phase = Phase::Flying;
        }
        // A transferred airport can keep same-owner parked aircraft. Their
        // former host mission was canceled with the transfer.
        for frame in self.aircraft.values_mut() {
            if frame.mission && !self.airports.get(&frame.home).is_some_and(|a| a.mission.is_some()) {
                frame.mission = false;
                frame.waypoints.clear();
            }
        }
    }

    fn approach_mission_target(&self, game: &mut Game, handle: u32, index: usize) {
        let frame = &self.aircraft[&handle];
        let Some(target_index) = self.airports.get(&frame.home).and_then(|airport| airport.mission.as_ref())
            .and_then(|mission| mission.target).and_then(|target| game.get(target)) else { return; };
        let actor = game.ents[index];
        let target = game.ents[target_index];
        if target.hp <= 0.0 || !game.air_valid_target(actor.team, game.h(target_index)) { return; }
        if game.combat_hostile(actor.team, target.team) && game.can_see(actor.team, target_index) &&
            DEFS[actor.def as usize].weapon_vs(DEFS[target.def as usize].dom).is_some() &&
            game.rps_allows(&actor, &target) {
            if frame.ordnance > 0 && (actor.order != O_ATTACK || actor.otgt != game.h(target_index)) {
                game.order_attack(&[index], target_index);
            }
            return;
        }
        let distance = (actor.x - target.x).powi(2) + (actor.y - target.y).powi(2);
        if distance > TARGET_RANGE * TARGET_RANGE && (actor.ox, actor.oy) != (target.x, target.y) {
            game.order_move(&[index], target.x, target.y, false);
        } else if distance <= TARGET_RANGE * TARGET_RANGE && actor.order != O_IDLE {
            game.order_stop(&[index]);
        }
    }

    fn update_airports(&mut self, game: &mut Game) {
        for (&home, airport) in &mut self.airports {
            let Some(host) = game.get(home) else { continue; };
            let building = game.ents[host];
            if game.dock_parked(host, &building) { continue; }
            airport.stored.retain(|&handle| {
                let Some(frame) = self.aircraft.get_mut(&handle).filter(|frame| frame.phase == Phase::Stationed) else {
                    return false;
                };
                if game.get(handle).is_some_and(|index| game.ents[index].team == building.team) { return true; }
                frame.home = NONE;
                frame.phase = Phase::Flying;
                false
            });
            if game.air_host(home, building.team).is_none() {
                if building.hp > 0.0 && building.built < 1.0 { continue; }
                for handle in airport.stored.drain(..) {
                    let frame = self.aircraft.get_mut(&handle).unwrap();
                    frame.home = NONE;
                    frame.phase = Phase::Flying;
                }
                airport.takeoff.clear();
                airport.mission = None;
                continue;
            }
            if !actor_pulse(game.tick, phase_key(home, building.team), 1.0, game.sim_step()) { continue; }
            let epoch = game.players[building.team as usize].epoch;
            let repair = 20.0 * game.crown_healing_building_scale(building.team, building.def);
            for &handle in &airport.stored {
                let index = game.get(handle).unwrap();
                let frame = self.aircraft.get_mut(&handle).unwrap();
                let definition = game.ents[index].def;
                if let Some(ordnance) = capacity(definition, epoch) {
                    frame.ordnance = ordnance;
                }
                if building.def == AIRFIELD {
                    let maximum = game.max_hp(building.team, definition);
                    game.ents[index].hp = (game.ents[index].hp + repair).min(maximum);
                }
            }
            if airport.automatic && airport.mission.is_some() {
                for &handle in &airport.stored {
                    let e = &game.ents[game.get(handle).unwrap()];
                    if e.hp >= game.max_hp(e.team, e.def) && airport.takeoff.iter().all(|&(a, _)| a != handle) {
                        airport.takeoff.push_back((handle, Launch::Mission));
                    }
                }
            }
            let Some((handle, launch)) = airport.takeoff.iter().copied().find(|&(actor, _)| airport.stored.contains(&actor)) else {
                airport.takeoff.clear();
                continue;
            };
            while airport.takeoff.pop_front().is_some_and(|(actor, _)| actor != handle) {}
            airport.stored.retain(|&actor| actor != handle);
            let index = game.get(handle).unwrap();
            let frame = self.aircraft.get_mut(&handle).unwrap();
            frame.phase = Phase::Flying;
            frame.mission = launch == Launch::Mission && airport.mission.is_some();
            frame.return_pending = false;
            frame.rendezvous_pending = false;
            frame.waypoints.clear();
            let jitter = launch_jitter(&mut game.source_birth.rng);
            let [width, height] = game.map.simulation_dimensions().map(|side| side as f32);
            let point = [(building.x + jitter).clamp(1.0, width - 1.0), (building.y + jitter).clamp(1.0, height - 1.0)];
            if game.air_runway_enabled {
                let (pose, deck_height) = if let Some(deck) = game.carrier_deck(host, game.sim_elapsed()) {
                    let (pose, height) = deck.departure();
                    (pose, Some(height))
                } else if building.def == CARRIER {
                    (crate::runway32::departure_at(game.air_host_markers(&building)), None)
                } else { (crate::runway32::departure(&building), None) };
                let aircraft = &mut game.ents[index];
                (aircraft.x, aircraft.y) = (pose.x, pose.y);
                (aircraft.px, aircraft.py) = (pose.x, pose.y);
                (aircraft.rot, aircraft.prot) = (pose.heading, pose.heading);
                game.begin_takeoff(index, &building, deck_height);
            }
            frame.waypoints.push_back(point);
            if frame.mission {
                frame.waypoints.extend(airport.mission.as_ref().unwrap().waypoints.iter().copied());
            }
            game.order_move(&[index], point[0], point[1], false);
        }
        self.airports.retain(|&handle, _| game.get(handle).is_some());
    }
}

#[cfg(test)]
#[path = "air31_tests.rs"]
mod tests;
