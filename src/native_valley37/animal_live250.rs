//! Authenticated authored AnimalManager deadlines, native order and ground tasks.
use super::{animal_wander247, deer_contact249::{advance_at_speed, FootContact},
    deer_foot247::{FootMotion, Rounding}, hash::sha256, population234::{Population, UnitKind},
    rectangular_world::staged_animals, wildlife_hunting240::Pose};
use crate::{game::Game, native_rng9::NativeRng};
use std::collections::BTreeMap;
#[cfg(feature = "ee2-rect252-animal-birth")]
use super::animal_birth252::{self, DEER_RADIUS, NEWBORN_YAW};
#[cfg(feature = "ee2-rect254-aircraft")]
use super::aircraft_live254::{self, Flight};

pub(super) const INPUT_LENGTH: usize = 8518;
const COUNT: usize = 166;
const TAG: &[u8; 4] = b"DVA1";
const GRADE_TAG: &[u8; 4] = b"DVA2";
#[cfg(feature = "ee2-rect252-animal-birth")]
const BIRTH_TAG: &[u8; 4] = b"DVA3";
#[cfg(feature = "ee2-rect253-native-carcass")]
const CARCASS_TAG: &[u8; 4] = b"DVA4";
#[cfg(feature = "ee2-rect254-aircraft")]
const AIRCRAFT_TAG: &[u8; 4] = b"DVA5";
#[cfg(feature = "ee2-rect255-aircraft-visual")]
const AIRCRAFT_VISUAL_TAG: &[u8; 4] = b"DVA6";
#[cfg(feature = "ee2-rect254-aircraft")]
const AIRCRAFT_COUNT: usize = 45;
#[cfg(feature = "ee2-rect252-animal-birth")]
const MAX_WORLD0_SLOTS: usize = 8192;
#[cfg(feature = "ee2-rect252-animal-birth")]
const MAX_LOCAL_ID: u32 = 0x00ff_ffff;
#[cfg(feature = "ee2-rect253-native-carcass")]
pub(super) const MAX_NATIVE_LOCAL_ID: u32 = 0x07ff_fffe;
const INPUT_SHA: [u8; 32] = [0x4c,0x85,0xb9,0x54,0x46,0x3e,0x8a,0x7b,0x46,0x83,0x93,0xd7,0xd2,0x2b,0x66,0x0a,
    0x74,0x05,0x94,0x75,0x01,0x89,0x3b,0xef,0x5f,0x7a,0xd3,0x29,0xa7,0x61,0x2d,0xcc];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Species { Deer, Goat, Llama, Tiger, WhiteWolf, Wolf, Hawk, Vulture }

impl Species {
    fn parse(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"Deer" => Self::Deer, b"Goat" => Self::Goat, b"Llama" => Self::Llama,
            b"Tiger" => Self::Tiger, b"WhiteWolf" => Self::WhiteWolf, b"Wolf" => Self::Wolf,
            b"Hawk" => Self::Hawk, b"Vulture" => Self::Vulture, _ => return None,
        })
    }

    fn foot_speed(self) -> Option<f32> {
        match self {
            Self::Deer => Some(2.07), Self::Goat | Self::Llama => Some(0.805),
            Self::Tiger | Self::WhiteWolf | Self::Wolf => Some(2.53),
            Self::Hawk | Self::Vulture => None,
        }
    }

    fn wander_parameters(self) -> (f32, f32) {
        if self.foot_speed().is_some() { (4.0, 6.0) } else { (5.0, 7.0) }
    }
}

#[derive(Clone, Copy)]
struct Herd {
    home: [f32; 2],
    birth: f32,
    species: Species,
    #[cfg(feature = "ee2-rect252-animal-birth")]
    population: u32,
    #[cfg(feature = "ee2-rect252-animal-birth")]
    cap: u32,
}

#[cfg(feature = "ee2-rect252-animal-birth")]
#[derive(Clone)]
struct BirthState {
    cursor: u32,
    newborns: Vec<(u32, u32)>, // Native ID and forced herd, in actor creation order.
    #[cfg(feature = "ee2-rect253-native-carcass")]
    native_carcasses: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Wander {
    destination: [f32; 3],
    pub(super) walked_since: Option<f32>,
    pub(super) motion: FootMotion,
    pub(super) contact: FootContact,
}

#[derive(Clone)]
struct Animal {
    id: u32,
    herd: u32,
    deadline: f32,
    pose: Pose,
    wander: Option<Wander>,
}

#[cfg(feature = "ee2-rect255-aircraft-visual")]
#[derive(Clone, Copy)]
struct WalkBind {
    id: u32,
    start: f32,
    alternate: bool,
}

#[cfg(feature = "ee2-rect255-aircraft-visual")]
#[derive(Clone)]
struct AircraftVisual {
    rng: NativeRng,
    // Retain past binds even if the bird dies. A bind consumed the stream once.
    binds: Vec<WalkBind>,
}

#[cfg(feature = "ee2-rect255-aircraft-visual")]
impl AircraftVisual {
    fn new() -> Self { Self { rng: NativeRng::from_state(1), binds: Vec::new() } }

    fn bind(&mut self, id: u32, start: f32) {
        // The stock Hawk/Vulture Walk binder at animation quality >= 2.
        // Scope the C9BF0C stream to the authored bird bind events.
        let alternate = self.rng.next_word15() <= 7561;
        self.binds.push(WalkBind { id, start, alternate });
    }

    fn get(&self, id: u32) -> Option<&WalkBind> {
        self.binds.iter().find(|bind| bind.id == id)
    }
}

#[derive(Clone)]
pub(super) struct AnimalManager {
    rows: Vec<Animal>,
    herds: BTreeMap<u32, Herd>,
    indices: BTreeMap<u32, usize>,
    graded: bool,
    #[cfg(feature = "ee2-rect252-animal-birth")]
    herd_order: Vec<u32>,
    #[cfg(feature = "ee2-rect252-animal-birth")]
    births: Option<BirthState>,
    #[cfg(feature = "ee2-rect254-aircraft")]
    aircraft: Option<BTreeMap<u32, Flight>>,
    #[cfg(feature = "ee2-rect255-aircraft-visual")]
    aircraft_visual: Option<AircraftVisual>,
}

impl AnimalManager {
    pub(super) fn decode(bytes: &[u8], population: &Population) -> Option<Self> {
        if bytes.len() != INPUT_LENGTH || sha256(bytes) != INPUT_SHA { return None; }
        let mut data = Cursor(bytes);
        let _next_herd_serial = data.word()?;
        if data.word()? as usize != COUNT { return None; }
        let mut input_rows = Vec::with_capacity(COUNT);
        for _ in 0..COUNT {
            let (id, herd, deadline, species) = (data.word()?, data.word()?, data.scalar()?, data.species()?);
            let unit = population.units().iter().find(|unit| unit.packed_id == id)?;
            let name = population.types().get(usize::from(unit.type_index))?;
            if unit.kind != UnitKind::LiveAnimal || Species::parse(name.as_bytes()) != Some(species) { return None; }
            input_rows.push((Animal { id, herd, deadline, pose: Pose { xyz: unit.xyz, yaw: unit.yaw }, wander: None }, species));
        }
        if data.word()? as usize != COUNT { return None; }
        let mut herds = BTreeMap::new();
        #[cfg(feature = "ee2-rect252-animal-birth")]
        let mut herd_order = Vec::with_capacity(COUNT);
        for _ in 0..COUNT {
            let (id, x, y) = (data.word()?, data.scalar()?, data.scalar()?);
            let (_population, _cap) = (data.word()?, data.word()?);
            let (birth, species) = (data.scalar()?, data.species()?);
            if herds.insert(id, Herd { home: [x, y], birth, species,
                #[cfg(feature = "ee2-rect252-animal-birth")]
                population: _population,
                #[cfg(feature = "ee2-rect252-animal-birth")]
                cap: _cap,
            }).is_some() { return None; }
            #[cfg(feature = "ee2-rect252-animal-birth")]
            herd_order.push(id);
        }
        if !data.0.is_empty() || input_rows.iter().any(|(row, species)| herds.get(&row.herd).map(|herd| herd.species) != Some(*species)) {
            return None;
        }
        // The original visits low-byte buckets in increasing order and keeps the archive chain order.
        input_rows.sort_by_key(|(row, _)| row.id & 255);
        let rows: Vec<_> = input_rows.into_iter().map(|(row, _)| row).collect();
        let indices: BTreeMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
        #[cfg(feature = "ee2-rect252-animal-birth")]
        herd_order.sort_by_key(|id| id & 15);
        (indices.len() == COUNT).then_some(Self { rows, herds, indices, graded: cfg!(feature = "ee2-rect251-animal-slope"),
            #[cfg(feature = "ee2-rect252-animal-birth")]
            herd_order,
            #[cfg(feature = "ee2-rect252-animal-birth")]
            births: None,
            #[cfg(feature = "ee2-rect254-aircraft")]
            aircraft: None,
            #[cfg(feature = "ee2-rect255-aircraft-visual")]
            aircraft_visual: None,
        })
    }

    fn row(&self, id: u32) -> Option<&Animal> { self.rows.get(*self.indices.get(&id)?) }

    pub(super) fn pose(&self, id: u32) -> Option<Pose> { Some(self.row(id)?.pose) }

    pub(super) fn set_pose(&mut self, id: u32, pose: Pose) {
        if let Some(&index) = self.indices.get(&id) { self.rows[index].pose = pose; }
    }

    pub(super) fn take_wander(&mut self, id: u32) -> Option<Wander> {
        self.rows.get_mut(*self.indices.get(&id)?)?.wander.take()
    }

    pub(super) fn word(&self, id: u32, field: u32, now: f32) -> Option<u32> {
        let row = self.row(id)?;
        let herd = self.herds.get(&row.herd)?;
        #[cfg(feature = "ee2-rect254-aircraft")]
        if let Some(aircraft) = self.aircraft.as_ref().filter(|_| herd.species.foot_speed().is_none()) {
            if let Some(value) = aircraft.get(&id).and_then(|flight| flight.word(field)) { return Some(value); }
            #[cfg(feature = "ee2-rect255-aircraft-visual")]
            if let Some(visual) = self.aircraft_visual.as_ref().filter(|_| aircraft.contains_key(&id)) {
                match field {
                    56 => return Some(visual.get(id).map_or(2, |bind| u32::from(bind.alternate))),
                    57 => return visual.get(id).map(|bind| bind.start.to_bits()),
                    _ => {}
                }
            }
            if matches!(field, 40 | 41) { return Some(0); }
        }
        Some(match field {
            17 => (now - row.wander?.walked_since?).to_bits(),
            18..=20 => row.wander?.destination[(field - 18) as usize].to_bits(),
            21..=24 => row.wander?.contact.words()[(field - 21) as usize].to_bits(),
            25 => row.deadline.to_bits(),
            26 => row.herd,
            27..=28 => herd.home[(field - 27) as usize].to_bits(),
            29 => u32::from(row.wander.is_some()),
            30 => herd.birth.to_bits(),
            #[cfg(feature = "ee2-rect252-animal-birth")]
            38 if self.births.is_some() => herd.population,
            #[cfg(feature = "ee2-rect252-animal-birth")]
            39 if self.births.is_some() => herd.cap,
            _ => return None,
        })
    }

    fn encode(&self, _include_visual: bool) -> Vec<u8> {
        let tag = if self.graded { GRADE_TAG } else { TAG };
        #[cfg(feature = "ee2-rect252-animal-birth")]
        let tag = if self.births.is_some() { BIRTH_TAG } else { tag };
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let tag = if self.native_carcass_cursor().is_some() { CARCASS_TAG } else { tag };
        #[cfg(feature = "ee2-rect254-aircraft")]
        let tag = if self.aircraft.is_some() { AIRCRAFT_TAG } else { tag };
        #[cfg(feature = "ee2-rect255-aircraft-visual")]
        let visual = self.aircraft_visual.as_ref().filter(|_| _include_visual);
        #[cfg(feature = "ee2-rect255-aircraft-visual")]
        let tag = if visual.is_some() { AIRCRAFT_VISUAL_TAG } else { tag };
        let mut bytes = tag.to_vec();
        bytes.extend_from_slice(&INPUT_SHA);
        let mut word = |value: u32| bytes.extend_from_slice(&value.to_le_bytes());
        #[cfg(feature = "ee2-rect252-animal-birth")]
        if let Some(births) = &self.births {
            word(births.cursor);
            word(self.herd_order.len() as u32);
            for &id in &self.herd_order {
                let herd = self.herds[&id];
                for value in [id, herd.population, herd.birth.to_bits()] { word(value); }
            }
            word(births.newborns.len() as u32);
            for &(id, herd) in &births.newborns { word(id); word(herd); }
        }
        word(self.rows.len() as u32);
        for row in &self.rows {
            word(row.id);
            for value in [row.deadline].into_iter().chain(row.pose.xyz).chain([row.pose.yaw]) { word(value.to_bits()); }
            word(u32::from(row.wander.is_some()));
            if let Some(wander) = row.wander {
                for value in wander.destination { word(value.to_bits()); }
                word(wander.walked_since.map_or(u32::MAX, f32::to_bits));
                for value in [wander.motion.speed, wander.motion.yaw_rate].into_iter().chain(wander.motion.velocity).chain(wander.contact.words()) {
                    word(value.to_bits());
                }
            }
        }
        #[cfg(feature = "ee2-rect254-aircraft")]
        if let Some(aircraft) = &self.aircraft {
            word(aircraft.len() as u32);
            for (&id, &flight) in aircraft {
                word(id);
                flight.encode(&mut word);
            }
        }
        #[cfg(feature = "ee2-rect255-aircraft-visual")]
        if let Some(visual) = visual {
            // Bind order reconstructs the RNG state and the selections on load.
            word(visual.binds.len() as u32);
            for bind in &visual.binds {
                word(bind.id);
                word(bind.start.to_bits());
            }
        }
        bytes
    }
}

#[cfg(feature = "ee2-rect252-animal-birth")]
impl AnimalManager {
    pub(super) fn enable_births(&mut self, cursor: u32) {
        self.births = Some(BirthState { cursor, newborns: Vec::new(),
            #[cfg(feature = "ee2-rect253-native-carcass")]
            native_carcasses: true,
        });
        #[cfg(feature = "ee2-rect254-aircraft")]
        { self.aircraft = Some(BTreeMap::new()); }
        #[cfg(feature = "ee2-rect255-aircraft-visual")]
        { self.aircraft_visual = Some(AircraftVisual::new()); }
    }

    fn insert_animal(&mut self, row: Animal) {
        let bucket = row.id & 255;
        let at = self.rows.partition_point(|other| (other.id & 255) <= bucket);
        self.rows.insert(at, row);
        for (index, row) in self.rows.iter().enumerate().skip(at) { self.indices.insert(row.id, index); }
    }

    fn note_allocation(&mut self) -> Option<u32> {
        let births = self.births.as_mut()?;
        let id = births.next_id()?;
        births.cursor = id;
        Some(id)
    }

    pub(super) fn record_death(&mut self, id: u32) -> Option<u32> {
        self.births.as_ref()?;
        let herd_id = self.row(id)?.herd;
        #[cfg(feature = "ee2-rect254-aircraft")]
        if let Some(aircraft) = &mut self.aircraft { aircraft.remove(&id); }
        let herd = self.herds.get_mut(&herd_id)?;
        herd.population = herd.population.saturating_sub(1);
        let births = self.births.as_mut()?;
        let allocated = births.next_id()?;
        births.cursor = allocated;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        if births.native_carcasses { return Some(allocated); }
        None
    }

    #[cfg(feature = "ee2-rect253-native-carcass")]
    pub(super) fn native_carcass_cursor(&self) -> Option<u32> {
        self.births.as_ref().filter(|births| births.native_carcasses).map(|births| births.cursor)
    }
}

#[cfg(feature = "ee2-rect252-animal-birth")]
impl BirthState {
    fn max_id(&self) -> u32 {
        #[cfg(feature = "ee2-rect253-native-carcass")]
        if self.native_carcasses { return MAX_NATIVE_LOCAL_ID; }
        MAX_LOCAL_ID
    }

    fn next_id(&self) -> Option<u32> {
        (self.cursor < self.max_id()).then(|| self.cursor + 1)
    }
}

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(count)?;
        self.0 = rest;
        Some(head)
    }

    fn word(&mut self) -> Option<u32> { Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?)) }

    fn scalar(&mut self) -> Option<f32> {
        let value = f32::from_bits(self.word()?);
        value.is_finite().then_some(value)
    }

    fn xyz(&mut self) -> Option<[f32; 3]> { Some([self.scalar()?, self.scalar()?, self.scalar()?]) }

    fn species(&mut self) -> Option<Species> {
        let length = usize::from(u16::from_le_bytes(self.take(2)?.try_into().ok()?));
        Species::parse(self.take(length)?)
    }
}

impl Game {
    fn rect_animal_dead(&self, id: u32) -> bool {
        self.rect_population.as_ref().and_then(|population| population.wildlife.get(&id)).is_some_and(|state| state.hp == 0.0)
    }

    pub(crate) fn advance_rect_animals(&mut self) {
        let Some(mut manager) = self.rect_population.as_mut().and_then(|population| population.animals.take()) else { return; };
        let now = self.sim_elapsed();
        let dt = self.sim_step();
        #[cfg(feature = "ee2-rect251-animal-slope")]
        let graded = manager.graded;
        for row in &mut manager.rows {
            #[cfg(feature = "ee2-rect254-aircraft")]
            if let Some(flight) = manager.aircraft.as_mut().and_then(|aircraft| aircraft.get_mut(&row.id)) {
                #[cfg(feature = "ee2-rect255-aircraft-visual")]
                if flight.pending_walk() {
                    if let Some(visual) = &mut manager.aircraft_visual { visual.bind(row.id, now); }
                }
                row.pose = flight.advance(row.id, now, dt, |point| self.rect_aircraft_height(point));
                continue;
            }
            let Some(mut wander) = row.wander else { continue; };
            let [x, y, _] = row.pose.xyz;
            let [gx, gy, _] = wander.destination;
            let speed = manager.herds[&row.herd].species.foot_speed().unwrap();
            let yaw = if row.pose.yaw < 0.0 { row.pose.yaw + core::f32::consts::TAU } else { row.pose.yaw };
            let passable = |point| self.deer_step_passable([x, y], point);
            #[cfg(feature = "ee2-rect251-animal-slope")]
            let step = if graded {
                self.rect_graded_ground_step([x, y], yaw, [gx, gy], dt, now, wander.motion, &mut wander.contact, speed, passable)
            } else { advance_at_speed([x, y], yaw, [gx, gy], dt, now, wander.motion, &mut wander.contact, Rounding::Chop53, speed, passable) };
            #[cfg(not(feature = "ee2-rect251-animal-slope"))]
            let step = advance_at_speed([x, y], yaw, [gx, gy], dt, now, wander.motion, &mut wander.contact, Rounding::Chop53, speed, passable);
            if step.xy != [x, y] {
                let [nx, ny] = step.xy;
                #[cfg(feature = "ee2-rect251-animal-slope")]
                let z = if graded { self.rect_ground_commit_height(row.pose.xyz[2], [nx, ny], wander.contact.last_block == now) }
                    else { self.map.height(nx, ny) };
                #[cfg(not(feature = "ee2-rect251-animal-slope"))]
                let z = self.map.height(nx, ny);
                row.pose.xyz = [nx, ny, z];
                wander.walked_since.get_or_insert(now);
            }
            row.pose.yaw = step.yaw;
            wander.motion = step.motion;
            row.wander = (!step.arrived).then_some(wander);
        }
        self.rect_population.as_mut().unwrap().animals = Some(manager);
    }

    pub(crate) fn schedule_rect_animals(&mut self) {
        let Some(mut manager) = self.rect_population.as_mut().and_then(|population| population.animals.take()) else { return; };
        let now = self.sim_elapsed();
        let mut rng = self.source_birth.rng.state();
        for row in &mut manager.rows {
            if row.deadline > now || self.rect_animal_dead(row.id) { continue; }
            let herd = manager.herds[&row.herd];
            let (radius, interval) = herd.species.wander_parameters();
            let busy = row.wander.is_some() || self.rect_deer_fleeing(row.id);
            #[cfg(feature = "ee2-rect254-aircraft")]
            let busy = busy || manager.aircraft.as_ref().and_then(|aircraft| aircraft.get(&row.id))
                .is_some_and(Flight::busy);
            let [x, y, _] = row.pose.xyz;
            animal_wander247::update_due(&mut rng, &mut row.deadline, now, busy, herd.home, radius, interval, Rounding::Chop53, |goal, _| {
                if herd.species.foot_speed().is_some() && self.rect_ground_ray_clear([x, y], goal) {
                    row.wander = Some(Wander { destination: [goal[0], goal[1], self.map.height(goal[0], goal[1])], walked_since: None,
                        motion: FootMotion::default(), contact: FootContact::default() });
                }
                #[cfg(feature = "ee2-rect254-aircraft")]
                if herd.species.foot_speed().is_none() {
                    if let Some(aircraft) = &mut manager.aircraft {
                        // AnimalManager passes raw proposal XY and a spilled terrain Z to the task.
                        let z = super::deer_foot247::store(self.rect_aircraft_height(goal), Rounding::Chop53);
                        let requested = [goal[0], goal[1], z];
                        match aircraft.entry(row.id) {
                            std::collections::btree_map::Entry::Occupied(mut entry) => entry.get_mut().restart(requested),
                            std::collections::btree_map::Entry::Vacant(entry) => {
                                entry.insert(Flight::start(row.pose, requested, |point| self.rect_aircraft_height(point)));
                            }
                        }
                    }
                }
            });
        }
        #[cfg(feature = "ee2-rect252-animal-birth")]
        self.schedule_rect_births(&mut manager, &mut rng, now);
        self.source_birth.rng = NativeRng::from_state(rng);
        self.rect_population.as_mut().unwrap().animals = Some(manager);
    }

    pub(crate) fn rect_animal_section(&self) -> Option<Vec<u8>> {
        Some(self.rect_population.as_ref()?.animals.as_ref()?.encode(true))
    }

    pub(crate) fn hash_rect_animals(&self, mix: &mut impl FnMut(u32)) {
        if let Some(manager) = self.rect_population.as_ref().and_then(|population| population.animals.as_ref()) {
            let bytes = manager.encode(false);
            for word in bytes.chunks_exact(4) { mix(u32::from_le_bytes(word.try_into().unwrap())); }
        }
    }

    pub(crate) fn restore_rect_animals(&mut self, bytes: &[u8]) -> Option<()> {
        let mut manager = staged_animals()?;
        let visual_wire = cfg!(feature = "ee2-rect255-aircraft-visual") && bytes.starts_with(b"DVA6");
        #[cfg(feature = "ee2-rect254-aircraft")]
        let aircraft_wire = bytes.starts_with(AIRCRAFT_TAG) || visual_wire;
        #[cfg(not(feature = "ee2-rect254-aircraft"))]
        let aircraft_wire = false;
        #[cfg(feature = "ee2-rect253-native-carcass")]
        let carcass_wire = bytes.starts_with(CARCASS_TAG) || aircraft_wire;
        #[cfg(not(feature = "ee2-rect253-native-carcass"))]
        let carcass_wire = false;
        #[cfg(feature = "ee2-rect252-animal-birth")]
        let birth_wire = bytes.starts_with(BIRTH_TAG) || carcass_wire;
        #[cfg(not(feature = "ee2-rect252-animal-birth"))]
        let birth_wire = false;
        let payload = if let Some(payload) = bytes.strip_prefix(TAG) {
            manager.graded = false;
            payload
        } else if birth_wire {
            manager.graded = true;
            &bytes[4..]
        } else {
            if !cfg!(feature = "ee2-rect251-animal-slope") { return None; }
            manager.graded = true;
            bytes.strip_prefix(GRADE_TAG)?
        };
        let mut data = Cursor(payload);
        if data.take(INPUT_SHA.len())? != INPUT_SHA { return None; }
        #[cfg(feature = "ee2-rect252-animal-birth")]
        if birth_wire {
            let source = self.rect_population.as_ref()?.source();
            let minimum = source.units().iter().map(|unit| unit.local_id()).max()?;
            let mut births = BirthState { cursor: data.word()?, newborns: Vec::new(),
                #[cfg(feature = "ee2-rect253-native-carcass")]
                native_carcasses: carcass_wire,
            };
            let base = if carcass_wire { minimum + 3 * self.nplayers as u32 } else { minimum };
            if !(base..=births.max_id()).contains(&births.cursor) || data.word()? as usize != manager.herd_order.len() { return None; }
            for &id in &manager.herd_order {
                if data.word()? != id { return None; }
                let herd = manager.herds.get_mut(&id)?;
                let population = data.word()?;
                let birth = data.scalar()?;
                if population > herd.cap || (herd.species != Species::Deer && (population > herd.population || birth != herd.birth))
                    || (herd.species == Species::Deer && birth < herd.birth) { return None; }
                herd.population = population;
                herd.birth = birth;
            }
            let count = data.word()? as usize;
            if count > MAX_WORLD0_SLOTS - source.units().len() { return None; }
            let mut previous = base;
            births.newborns.reserve(count);
            for _ in 0..count {
                let (id, herd) = (data.word()?, data.word()?);
                if id <= previous || id > births.cursor || manager.herds.get(&herd)?.species != Species::Deer { return None; }
                previous = id;
                births.newborns.push((id, herd));
                manager.insert_animal(Animal { id, herd, deadline: 0.0, pose: Pose { xyz: [0.0; 3], yaw: NEWBORN_YAW }, wander: None });
            }
            manager.births = Some(births);
        }
        if data.word()? as usize != manager.rows.len() { return None; }
        let now = self.sim_elapsed();
        let contains = |xyz: [f32; 3]| self.map.layout.contains_point(xyz[0], xyz[1]);
        for row in &mut manager.rows {
            if data.word()? != row.id { return None; }
            let deadline = data.scalar()?;
            let xyz = data.xyz()?;
            let yaw = data.scalar()?;
            let position_valid = contains(xyz);
            #[cfg(feature = "ee2-rect254-aircraft")]
            let position_valid = position_valid || aircraft_wire && manager.herds[&row.herd].species.foot_speed().is_none()
                && aircraft_live254::source_position(xyz);
            if deadline < row.deadline || !position_valid || !(-core::f32::consts::PI..=core::f32::consts::TAU).contains(&yaw) {
                return None;
            }
            row.deadline = deadline;
            row.pose = Pose { xyz, yaw };
            row.wander = match data.word()? {
                0 => None,
                1 => {
                    let destination = data.xyz()?;
                    let start = data.word()?;
                    let walked_since = (start != u32::MAX).then(|| f32::from_bits(start));
                    let motion = FootMotion { speed: data.scalar()?, yaw_rate: data.scalar()?, velocity: [data.scalar()?, data.scalar()?] };
                    let contact = FootContact::read([data.scalar()?, data.scalar()?, data.scalar()?, data.scalar()?], now)?;
                    let speed = manager.herds[&row.herd].species.foot_speed()?;
                    if !contains(destination) || !self.map.layout.contains_point(contact.anchor[0], contact.anchor[1])
                        || walked_since.is_some_and(|t| !(0.0..=now).contains(&t)) || !(0.0..=speed).contains(&motion.speed)
                        || motion.yaw_rate.abs() > 720.0_f32.to_radians() || motion.velocity.into_iter().any(|v| v.abs() > speed) { return None; }
                    Some(Wander { destination, walked_since, motion, contact })
                }
                _ => return None,
            };
        }
        #[cfg(feature = "ee2-rect254-aircraft")]
        if aircraft_wire {
            let count = data.word()? as usize;
            if count > AIRCRAFT_COUNT { return None; }
            let mut previous = 0;
            let mut aircraft = BTreeMap::new();
            for _ in 0..count {
                let id = data.word()?;
                let row = manager.row(id)?;
                if id <= previous || manager.herds[&row.herd].species.foot_speed().is_some() { return None; }
                previous = id;
                let flight = Flight::decode(row.pose, &mut || data.word())?;
                aircraft.insert(id, flight);
            }
            if manager.rows.iter().any(|row| !contains(row.pose.xyz) && !aircraft.contains_key(&row.id)) { return None; }
            manager.aircraft = Some(aircraft);
        }
        #[cfg(feature = "ee2-rect255-aircraft-visual")]
        if visual_wire {
            let count = data.word()? as usize;
            if count > AIRCRAFT_COUNT { return None; }
            let mut visual = AircraftVisual::new();
            let mut previous = None;
            for _ in 0..count {
                let id = data.word()?;
                let start = data.scalar()?;
                let index = *manager.indices.get(&id)?;
                let species = manager.herds[&manager.rows[index].herd].species;
                if !(0.0 < start && start <= now) || species.foot_speed().is_some()
                    || visual.get(id).is_some()
                    || previous.is_some_and(|(time, order)| (start, index) <= (time, order))
                { return None; }
                previous = Some((start, index));
                visual.bind(id, start);
            }
            manager.aircraft_visual = Some(visual);
        }
        if !data.0.is_empty() { return None; }
        let population = self.rect_population.as_mut()?;
        #[cfg(feature = "ee2-rect252-animal-birth")]
        if let Some(births) = &manager.births {
            for &(id, _) in &births.newborns {
                let row = manager.row(id)?;
                population.newborns.push(population.newborn_deer(id, row.pose.xyz)?);
            }
        }
        population.animals = Some(manager);
        Some(())
    }

    pub(crate) fn validate_rect_animals(&self) -> Option<()> {
        let manager = self.rect_population.as_ref()?.animals.as_ref()?;
        #[cfg(feature = "ee2-rect252-animal-birth")]
        let mut live_counts = BTreeMap::<u32, u32>::new();
        for row in &manager.rows {
            let dead = self.rect_animal_dead(row.id);
            if let Some(pose) = self.rect_deer_flee_pose(row.id) {
                if pose.xyz != row.pose.xyz || pose.yaw != row.pose.yaw { return None; }
            }
            if row.wander.is_some() && (dead || self.rect_deer_fleeing(row.id)) { return None; }
            #[cfg(feature = "ee2-rect254-aircraft")]
            if let Some(aircraft) = manager.aircraft.as_ref().filter(|_| manager.herds[&row.herd].species.foot_speed().is_none()) {
                let flight = aircraft.get(&row.id);
                let has_flight = flight.is_some();
                if dead && has_flight { return None; }
                #[cfg(feature = "ee2-rect255-aircraft-visual")]
                if let Some(visual) = &manager.aircraft_visual {
                    let bound = visual.get(row.id).is_some();
                    if !dead && bound != flight.is_some_and(|flight| !flight.pending_walk()) { return None; }
                }
                if !dead && !has_flight {
                    let source = self.rect_population.as_ref()?.unit(row.id)?;
                    if source.xyz != row.pose.xyz || source.yaw != row.pose.yaw { return None; }
                }
            }
            #[cfg(feature = "ee2-rect252-animal-birth")]
            if !dead { *live_counts.entry(row.herd).or_default() += 1; }
        }
        #[cfg(feature = "ee2-rect252-animal-birth")]
        if manager.births.is_some() && manager.herds.iter().any(|(id, herd)| live_counts.get(id).copied().unwrap_or(0) != herd.population) {
            return None;
        }
        Some(())
    }
}

#[cfg(feature = "ee2-rect254-aircraft")]
impl Game {
    fn rect_aircraft_height(&self, point: [f32; 2]) -> f64 {
        let world = self.map.source_world.as_deref().expect("authored animal geometry");
        super::rectangular_world::aircraft_height(world, &self.map.rect_surface_index, point)
    }
}

#[cfg(feature = "ee2-rect252-animal-birth")]
impl Game {
    pub(crate) fn note_rect_owned_allocation(&mut self) -> Option<u32> {
        self.rect_population.as_mut()?.animals.as_mut()?.note_allocation()
    }

    #[cfg(feature = "ee2-rect287-oil-economy")]
    pub(crate) fn rect_native_cursor(&self) -> Option<u32> {
        self.rect_population.as_ref()?.animals.as_ref()?.native_carcass_cursor()
    }

    #[cfg(feature = "ee2-rect287-oil-economy")]
    pub(crate) fn rect_native_base(&self) -> Option<u32> {
        self.rect_population.as_ref()?.source().units().iter().map(|unit| unit.local_id()).max()
    }

    fn schedule_rect_births(&mut self, manager: &mut AnimalManager, rng: &mut u32, now: f32) {
        if manager.births.is_none() { return; }
        for index in 0..manager.herd_order.len() {
            let herd_id = manager.herd_order[index];
            let mut herd = manager.herds[&herd_id];
            if herd.species != Species::Deer || herd.population == 0 || now <= herd.birth { continue; }
            let births = manager.births.as_ref().unwrap();
            let born = animal_birth252::tick_birth(rng, &mut herd.birth, &mut herd.population, herd.cap, herd.home, now, |point| {
                let population = self.rect_population.as_ref()?;
                if population.unit_count() >= MAX_WORLD0_SLOTS { return None; }
                let id = (births.next_id()?..=births.max_id()).find(|&id| population.unit(id).is_none())?;
                let xyz = animal_birth252::find_birth_site(point, self.map.simulation_dimensions(), |xy|
                    self.rect_birth_site(manager, xy))?;
                Some((id, xyz))
            });
            manager.herds.insert(herd_id, herd);
            if let Some(born) = born {
                let population = self.rect_population.as_mut().unwrap();
                let source = population.newborn_deer(born.id, born.xyz).expect("bound Deer type");
                population.newborns.push(source);
                manager.insert_animal(Animal { id: born.id, herd: herd_id, deadline: born.first_wander,
                    pose: Pose { xyz: born.xyz, yaw: NEWBORN_YAW }, wander: None });
                let births = manager.births.as_mut().unwrap();
                births.cursor = born.id;
                births.newborns.push((born.id, herd_id));
            }
        }
    }

    fn rect_birth_site(&self, manager: &AnimalManager, [x, y]: [f32; 2]) -> Option<f32> {
        use crate::{defs::{DEFS, LAND}, game::BOARDED};
        if !self.map.layout.contains_point(x, y) || !self.map.pass[0][self.map.tile_of(x, y)] { return None; }
        let intersects = |x1: f32, y1: f32, radius: f32| {
            let reach = DEER_RADIUS + radius;
            (x - x1).powi(2) + (y - y1).powi(2) < reach * reach
        };
        if manager.rows.iter().any(|row| {
            let [x, y, _] = row.pose.xyz;
            intersects(x, y, DEER_RADIUS) && manager.herds[&row.herd].species.foot_speed().is_some()
                && !self.rect_animal_dead(row.id)
        }) || self.ents[..self.high].iter().any(|actor| {
            if !actor.alive || actor.hp <= 0.0 || actor.stuck == BOARDED { return false; }
            let def = &DEFS[actor.def as usize];
            !def.building && def.dom == LAND && intersects(actor.x, actor.y, def.rad())
        }) { return None; }
        Some(self.rect_ground_commit_height(0.0, [x, y], false))
    }
}

#[cfg(feature = "ee2-rect252-animal-birth")]
#[no_mangle]
pub extern "C" fn native_rect_animal_allocator() -> u32 {
    super::active_game().and_then(|game| game.rect_population.as_ref()?.animals.as_ref()?.births.as_ref())
        .map_or(u32::MAX, |births| births.cursor)
}

#[cfg(feature = "ee2-rect251-animal-slope")]
impl Game {
    pub(super) fn rect_animal_grade_active(&self) -> bool {
        self.rect_population.as_ref().and_then(|population| population.animals.as_ref()).is_some_and(|manager| manager.graded)
    }

    pub(super) fn rect_grade_height(&self, [x, y]: [f32; 2]) -> f64 {
        let world = self.map.source_world.as_deref().unwrap();
        super::rectangular_world::ground_grade_height(world, &self.map.rect_surface_index, x, y)
    }

    pub(super) fn rect_ground_commit_height(&self, old: f32, xy: [f32; 2], fallback: bool) -> f32 {
        if fallback { old } else { super::deer_foot247::store(self.rect_grade_height(xy), Rounding::Chop53) }
    }

    pub(super) fn rect_graded_ground_step(
        &self, xy: [f32; 2], yaw: f32, target: [f32; 2], dt: f32, now: f32, motion: FootMotion,
        contact: &mut FootContact, speed: f32, passable: impl FnMut([f32; 2]) -> bool,
    ) -> super::deer_foot247::FootStep {
        let rounding = Rounding::Chop53;
        let grade = super::deer_slope250::foot_grade_limits(xy, motion, speed, rounding, |point| self.rect_grade_height(point));
        super::deer_contact249::advance_with_limits(xy, yaw, target, dt, now, motion, contact, rounding, speed, grade.limits, passable)
    }

    pub(super) fn rect_grade_word(&self, id: u32, field: u32) -> Option<u32> {
        let manager = self.rect_population.as_ref()?.animals.as_ref()?;
        if !manager.graded { return None; }
        let row = manager.row(id)?;
        let motion = self.rect_deer_flee_motion(id).or_else(|| row.wander.map(|wander| wander.motion))?;
        let speed = manager.herds[&row.herd].species.foot_speed()?;
        let [x, y, _] = row.pose.xyz;
        let value = match field {
            34 => motion.speed,
            35..=36 => motion.velocity[(field - 35) as usize],
            31..=33 => {
                let grade = super::deer_slope250::foot_grade_limits([x, y], motion, speed, Rounding::Chop53, |point| self.rect_grade_height(point));
                [grade.grade, grade.limits.forward, grade.limits.angular][(field - 31) as usize]
            }
            _ => return None,
        };
        Some(value.to_bits())
    }
}
