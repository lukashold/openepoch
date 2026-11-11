//! Reaction and live position carrier for the authored World's Deer units.
use super::{
    deer_destination246::find_circle_destination,
    deer_reaction245::{self as native, AttackRecord, AttackWatch, FleeQuery},
    hunt_order241::{CITIZEN_ATTACK_RANGE, CITIZEN_DAMAGE, CITIZEN_RADIUS, CITIZEN_RELOAD},
    population234::{SourceUnit, UnitKind},
    population234_attach::AttachedPopulation,
    rectangular_save234::{deer_version, DEER_CONTACT_TAG, DEER_FOOT_TAG, DEER_TAG},
    wildlife_hunting240::Pose,
};
#[cfg(feature = "ee2-rect247-deer-foot")]
use super::deer_foot247::FootMotion;
#[cfg(feature = "ee2-rect249-deer-contact")]
use super::deer_contact249::FootContact;
use crate::{defs::WORKER, game::{angdiff, Ent, Game, BOARDED, MAXE}, map::{T_ROCK, T_SAND}, native_rng9::NativeRng};
use std::collections::BTreeMap;

const DEER_RADIUS: f32 = 0.225;
const DEER_SPEED: f32 = 2.07;
const DEER_TURN: f32 = 720.0;
const DECISION_PERIOD: f64 = 0.5;

#[derive(Clone, Copy)]
struct Flight {
    destination: [f32; 3],
    walked_since: Option<f32>,
    #[cfg(feature = "ee2-rect247-deer-foot")]
    foot: Option<FootMotion>,
    #[cfg(feature = "ee2-rect249-deer-contact")]
    contact: Option<FootContact>,
}

impl Flight {
    fn mode(self) -> u32 {
        #[cfg(feature = "ee2-rect249-deer-contact")]
        if self.contact.is_some() { return 3; }
        #[cfg(feature = "ee2-rect247-deer-foot")]
        if self.foot.is_some() { return 2; }
        1
    }
}

pub(super) struct DeerMotion {
    pub(super) pose: Pose,
    watch: AttackWatch,
    decision_clock: f32,
    flight: Option<Flight>,
}

impl DeerMotion {
    fn new(unit: &SourceUnit) -> Self {
        Self { pose: Pose { xyz: unit.xyz, yaw: unit.yaw }, watch: AttackWatch::default(),
            decision_clock: 0.0, flight: None }
    }
}

impl AttachedPopulation {
    fn deer(&self, id: u32) -> Option<&SourceUnit> {
        let unit = self.unit(id)?;
        (unit.kind == UnitKind::LiveAnimal && self.source().types()[usize::from(unit.type_index)] == "Deer")
            .then_some(unit)
    }
}

impl Game {
    pub(super) fn observe_rect_deer_hit(&mut self, id: u32, attacker: u32, hp: f32, full_hp: f32) {
        let now = self.sim_elapsed();
        let Some(population) = &mut self.rect_population else { return; };
        let Some(unit) = population.deer(id) else { return; };
        let initial = DeerMotion::new(unit);
        #[cfg(feature = "ee2-rect250-animals")]
        let initial = DeerMotion { pose: population.animals.as_ref().and_then(|animals| animals.pose(id)).unwrap_or(initial.pose), ..initial };
        let state = population.deer_motion.entry(id).or_insert(initial);
        state.watch.observe_direct_hit(hp / full_hp, true, attacker, now, &mut state.decision_clock);
    }

    pub(super) fn finish_rect_deer(&mut self, id: u32) -> Option<u32> {
        let mut corpse = None;
        #[cfg(feature = "ee2-rect250-animals")]
        if let Some(animals) = self.rect_population.as_mut().and_then(|population| population.animals.as_mut()) {
            animals.take_wander(id);
            #[cfg(feature = "ee2-rect252-animal-birth")]
            { corpse = animals.record_death(id); }
        }
        if let Some(deer) = self.rect_population.as_mut().and_then(|population| population.deer_motion.get_mut(&id)) {
            deer.watch = AttackWatch::default();
            deer.decision_clock = 0.0;
            deer.flight = None;
        }
        corpse
    }

    pub(crate) fn step_rect_deer(&mut self) {
        let Some(population) = &mut self.rect_population else { return; };
        if population.deer_motion.is_empty() { return; }
        let mut deer_motion = std::mem::take(&mut population.deer_motion);
        for (&id, deer) in &mut deer_motion {
            #[cfg(feature = "ee2-rect250-animals")]
            if let Some(pose) = self.rect_population.as_ref().and_then(|population| population.animals.as_ref()?.pose(id)) {
                deer.pose = pose;
            }
            self.step_deer(id, deer);
            #[cfg(feature = "ee2-rect250-animals")]
            if let Some(animals) = self.rect_population.as_mut().and_then(|population| population.animals.as_mut()) {
                animals.set_pose(id, deer.pose);
            }
        }
        self.rect_population.as_mut().unwrap().deer_motion = deer_motion;
    }

    fn step_deer(&mut self, id: u32, deer: &mut DeerMotion) {
        let now = self.sim_elapsed();
        if let Some(flight) = deer.flight {
            deer.flight = self.advance_deer_flight(&mut deer.pose, flight, now);
            return;
        }

        if deer.watch.records().is_empty() || f64::from(now) - DECISION_PERIOD <= f64::from(deer.decision_clock) {
            return;
        }
        // 00846A66 records the current decision time before selection and admission.
        deer.decision_clock = now;
        let threat = deer.watch.select_threat(now,
            |handle| self.get(handle).map(|index| self.ents[index]),
            |actor| actor.hp <= 0.0 || actor.stuck == BOARDED,
            |actor| self.deer_threat(actor, deer.pose.xyz));
        let Some(threat) = threat else { return; };
        let origin = deer.pose.xyz;
        let threat = [threat.x, threat.y, self.map.height(threat.x, threat.y)];
        let mut rng = self.source_birth.rng.state();
        let task = native::propose_flee(&mut rng, id, origin, threat, native::DEER_TETHER, 0,
            |query| self.deer_destination(query, deer.pose.yaw), |_| true);
        self.source_birth.rng = NativeRng::from_state(rng);
        if let Some(task) = task {
            let flight = Flight { destination: task.destination, walked_since: None,
                #[cfg(feature = "ee2-rect247-deer-foot")]
                foot: Some(FootMotion::default()),
                #[cfg(feature = "ee2-rect249-deer-contact")]
                contact: Some(FootContact::default()),
            };
            #[cfg(feature = "ee2-rect250-animals")]
            let flight = self.rect_population.as_mut().and_then(|population| population.animals.as_mut()?.take_wander(id))
                .map_or(flight, |wander| Flight { walked_since: wander.walked_since,
                    foot: Some(wander.motion), contact: Some(wander.contact), ..flight });
            deer.flight = Some(flight);
            deer.decision_clock += native::FLEE_COMMIT_COOLDOWN;
        }
    }

    fn advance_deer_flight(&self, pose: &mut Pose, mut flight: Flight, now: f32) -> Option<Flight> {
        let [x, y, _] = pose.xyz;
        let [gx, gy, _] = flight.destination;
        let dt = self.sim_step();
        #[cfg(feature = "ee2-rect247-deer-foot")]
        if let Some(foot) = flight.foot {
            use super::deer_foot247::{deer_foot_step, Rounding};
            let yaw = if pose.yaw < 0.0 { pose.yaw + core::f32::consts::TAU } else { pose.yaw };
            let passable = |point| self.deer_step_passable([x, y], point);
            let legacy_step = || {
                let step = deer_foot_step([x, y], yaw, [gx, gy], dt, foot, Rounding::Chop53);
                (step.xy == [x, y] || passable(step.xy)).then_some(step)
            };
            #[cfg(feature = "ee2-rect251-animal-slope")]
            let graded = self.rect_animal_grade_active() && flight.contact.is_some();
            #[cfg(feature = "ee2-rect249-deer-contact")]
            let result = if let Some(contact) = &mut flight.contact {
                #[cfg(feature = "ee2-rect251-animal-slope")]
                if graded { self.rect_graded_ground_step([x, y], yaw, [gx, gy], dt, now, foot, contact, DEER_SPEED, passable) }
                else { super::deer_contact249::advance([x, y], yaw, [gx, gy], dt, now, foot, contact, Rounding::Chop53, passable) }
                #[cfg(not(feature = "ee2-rect251-animal-slope"))]
                super::deer_contact249::advance([x, y], yaw, [gx, gy], dt, now, foot, contact, Rounding::Chop53, passable)
            } else { legacy_step()? };
            #[cfg(not(feature = "ee2-rect249-deer-contact"))]
            let result = legacy_step()?;
            let [nx, ny] = result.xy;
            if result.xy != [x, y] {
                #[cfg(feature = "ee2-rect251-animal-slope")]
                let z = if graded { self.rect_ground_commit_height(pose.xyz[2], [nx, ny], flight.contact.unwrap().last_block == now) }
                    else { self.map.height(nx, ny) };
                #[cfg(not(feature = "ee2-rect251-animal-slope"))]
                let z = self.map.height(nx, ny);
                pose.xyz = [nx, ny, z];
                flight.walked_since.get_or_insert(now);
            }
            pose.yaw = result.yaw;
            flight.foot = Some(result.motion);
            return (!result.arrived).then_some(flight);
        }

        // Version 4 stored no velocity. Preserve its step and checkpoint behavior.
        let (dx, dy) = (f64::from(gx) - f64::from(x), f64::from(gy) - f64::from(y));
        let length = dx.hypot(dy);
        if length == 0.0 { return None; }
        let step = f64::from(DEER_SPEED) * f64::from(dt);
        let ratio = (step / length).min(1.0);
        let next = [(f64::from(x) + dx * ratio) as f32, (f64::from(y) + dy * ratio) as f32];
        if !self.deer_step_passable([x, y], next) { return None; }
        if next == [x, y] { return Some(flight); }
        let facing = dy.atan2(dx) as f32;
        let turn = DEER_TURN.to_radians() * dt;
        pose.yaw = angdiff(pose.yaw + angdiff(facing, pose.yaw).clamp(-turn, turn), 0.0);
        pose.xyz = if step >= length { flight.destination }
            else { [next[0], next[1], self.map.height(next[0], next[1])] };
        flight.walked_since.get_or_insert(now);
        (step < length).then_some(flight)
    }

    fn deer_threat(&self, actor: &Ent, deer: [f32; 3]) -> f64 {
        if actor.def != WORKER { return 0.0; }
        let point = [actor.x, actor.y, self.map.height(actor.x, actor.y)];
        let distance_squared = deer.into_iter().zip(point)
            .map(|(target, source)| (f64::from(target) - f64::from(source)).powi(2)).sum::<f64>() as f32;
        let reach = (f64::from(CITIZEN_ATTACK_RANGE) + f64::from(CITIZEN_RADIUS) + f64::from(DEER_RADIUS)) as f32;
        let distance = f64::from(distance_squared).sqrt();
        let weight = if distance <= f64::from(reach) + 1.0 { 1.0 }
            else { (2.0 * f64::from(reach) / distance).min(1.0) as f32 };
        f64::from(CITIZEN_DAMAGE) / f64::from(CITIZEN_RELOAD) * f64::from(weight)
    }

    fn deer_passable(&self, point: [f32; 2]) -> bool {
        let [x, y] = point;
        self.map.layout.contains_point(x, y) && self.map.pass[0][self.map.tile_of(x, y)]
    }

    pub(super) fn deer_step_passable(&self, from: [f32; 2], to: [f32; 2]) -> bool {
        let [x, y] = to;
        if !self.map.layout.contains_point(x, y) { return false; }
        let tile = self.map.tile_of(x, y);
        if self.map.pass[0][tile] { return true; }
        // An authentic archived animal may start in a coarsely blocked city
        // cell. Let it exit that cell without crossing a new blocked one.
        let terrain = self.map.ter[tile];
        tile == self.map.tile_of(from[0], from[1]) && terrain >= T_SAND && terrain != T_ROCK
    }

    pub(super) fn rect_ground_ray_clear(&self, [x, y]: [f32; 2], [gx, gy]: [f32; 2]) -> bool {
        if !self.deer_passable([gx, gy]) { return false; }
        let radius = (f64::from(gx) - f64::from(x)).hypot(f64::from(gy) - f64::from(y));
        let samples = (radius * 4.0).ceil() as u32;
        (1..=samples).all(|i| {
            let t = i as f32 / samples as f32;
            self.deer_step_passable([x, y], [x + (gx - x) * t, y + (gy - y) * t])
        })
    }

    fn deer_destination(&self, query: FleeQuery, _yaw: f32) -> Option<[f32; 3]> {
        let [x, y, _] = query.origin;
        find_circle_destination(query.origin, query.direction, query.radius, DEER_RADIUS, |[gx, gy, _]| {
            if !self.rect_ground_ray_clear([x, y], [gx, gy]) { return None; }
            #[cfg(all(feature = "ee2-rect247-deer-foot", not(feature = "ee2-rect249-deer-contact")))]
            if !self.deer_foot_clear([x, y], _yaw, [gx, gy]) { return None; }
            Some([gx, gy, self.map.height(gx, gy)])
        })
    }

    #[cfg(feature = "ee2-rect250-animals")]
    pub(super) fn rect_deer_fleeing(&self, id: u32) -> bool {
        self.rect_population.as_ref().and_then(|population| population.deer_motion.get(&id))
            .is_some_and(|deer| deer.flight.is_some())
    }

    #[cfg(feature = "ee2-rect250-animals")]
    pub(super) fn rect_deer_flee_pose(&self, id: u32) -> Option<Pose> {
        let deer = self.rect_population.as_ref()?.deer_motion.get(&id)?;
        deer.flight.map(|_| deer.pose)
    }

    #[cfg(feature = "ee2-rect251-animal-slope")]
    pub(super) fn rect_deer_flee_motion(&self, id: u32) -> Option<FootMotion> {
        self.rect_population.as_ref()?.deer_motion.get(&id)?.flight?.foot
    }

    #[cfg(all(feature = "ee2-rect247-deer-foot", not(feature = "ee2-rect249-deer-contact")))]
    fn deer_foot_clear(&self, mut xy: [f32; 2], mut yaw: f32, target: [f32; 2]) -> bool {
        use super::deer_foot247::{deer_foot_step, Rounding};
        // The straight ray can miss a blocked corner entered while turning.
        // Bound the admission work even when an adaptive scheduler uses tiny steps.
        let dt = self.sim_step().max(0.01);
        let mut motion = FootMotion::default();
        if yaw < 0.0 { yaw += core::f32::consts::TAU; }
        for _ in 0..1024 {
            let step = deer_foot_step(xy, yaw, target, dt, motion, Rounding::Chop53);
            if !self.deer_step_passable(xy, step.xy) { return false; }
            if step.arrived { return true; }
            xy = step.xy;
            yaw = step.yaw;
            motion = step.motion;
        }
        false
    }

    pub(super) fn rect_deer_walk_word(&self, id: u32) -> u32 {
        self.rect_population.as_ref().and_then(|p| p.deer_motion.get(&id))
            .and_then(|state| state.flight?.walked_since)
            .map_or(u32::MAX, |start| (self.sim_elapsed() - start).to_bits())
    }

    pub(super) fn rect_deer_destination_word(&self, id: u32, axis: usize) -> u32 {
        self.rect_population.as_ref().and_then(|p| p.deer_motion.get(&id))
            .and_then(|state| state.flight).map_or(u32::MAX, |flight| flight.destination[axis].to_bits())
    }

    #[cfg(feature = "ee2-rect249-deer-contact")]
    pub(super) fn rect_deer_contact_word(&self, id: u32, index: usize) -> u32 {
        self.rect_population.as_ref().and_then(|p| p.deer_motion.get(&id))
            .and_then(|state| state.flight?.contact).map_or(u32::MAX, |contact| contact.words()[index].to_bits())
    }

    pub(crate) fn deer_motion_section(&self) -> Option<Vec<u8>> {
        let deer = &self.rect_population.as_ref()?.deer_motion;
        if deer.is_empty() { return None; }
        let tag = match deer.values().filter_map(|state| state.flight.map(Flight::mode)).max() {
            Some(3) => DEER_CONTACT_TAG,
            Some(2) => DEER_FOOT_TAG,
            _ => DEER_TAG,
        };
        let mut bytes = tag.to_vec();
        let mut word = |value: u32| bytes.extend_from_slice(&value.to_le_bytes());
        word(deer.len() as u32);
        for (&id, state) in deer {
            word(id);
            for value in state.pose.xyz.into_iter().chain([state.pose.yaw, state.decision_clock]) { word(value.to_bits()); }
            if let Some(flight) = state.flight {
                word(flight.mode());
                for value in flight.destination { word(value.to_bits()); }
                word(flight.walked_since.map_or(u32::MAX, f32::to_bits));
                #[cfg(feature = "ee2-rect247-deer-foot")]
                if let Some(foot) = flight.foot {
                    word(foot.speed.to_bits());
                    word(foot.yaw_rate.to_bits());
                    for value in foot.velocity { word(value.to_bits()); }
                }
                #[cfg(feature = "ee2-rect249-deer-contact")]
                if let Some(contact) = flight.contact {
                    for value in contact.words() { word(value.to_bits()); }
                }
            } else {
                word(0);
            }
            word(state.watch.records().len() as u32);
            for row in state.watch.records() { word(row.packed_id); word(row.time.to_bits()); }
        }
        Some(bytes)
    }

    pub(crate) fn restore_deer_motion(&mut self, bytes: &[u8]) -> Option<()> {
        let now = self.sim_elapsed();
        let population = self.rect_population.as_mut()?;
        let version = deer_version(bytes)?;
        let max_mode = version - 3;
        if max_mode == 3 && !cfg!(feature = "ee2-rect249-deer-contact")
            || max_mode >= 2 && !cfg!(feature = "ee2-rect247-deer-foot") { return None; }
        let body = &bytes[4..];
        let mut words = body.chunks_exact(4);
        if !words.remainder().is_empty() { return None; }
        let mut word = || words.next().map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()));
        let count = word()? as usize;
        if count == 0 || count > population.unit_count() { return None; }
        let mut states = BTreeMap::new();
        let mut seen_mode = 0;
        for _ in 0..count {
            let id = word()?;
            population.deer(id)?;
            if states.last_key_value().is_some_and(|(&previous, _)| previous >= id) { return None; }
            let xyz = read_xyz(&mut word)?;
            let yaw = f32::from_bits(word()?);
            let decision_clock = f32::from_bits(word()?);
            if !self.map.layout.contains_point(xyz[0], xyz[1])
                || !(-core::f32::consts::PI..=core::f32::consts::TAU).contains(&yaw)
                || !(0.0..=now + native::FLEE_COMMIT_COOLDOWN).contains(&decision_clock)
            { return None; }
            let flight = match word()? {
                0 => None,
                mode => {
                    if mode > max_mode { return None; }
                    seen_mode = seen_mode.max(mode);
                    let destination = read_xyz(&mut word)?;
                    let start = word()?;
                    let walked_since = (start != u32::MAX).then(|| f32::from_bits(start));
                    if !self.map.layout.contains_point(destination[0], destination[1])
                        || walked_since.is_some_and(|start| !(0.0..=now).contains(&start))
                    { return None; }
                    #[cfg(feature = "ee2-rect247-deer-foot")]
                    let foot = if mode >= 2 {
                        let foot = FootMotion {
                            speed: f32::from_bits(word()?), yaw_rate: f32::from_bits(word()?),
                            velocity: [f32::from_bits(word()?), f32::from_bits(word()?)],
                        };
                        if !(0.0..=DEER_SPEED).contains(&foot.speed)
                            || !(-DEER_TURN.to_radians()..=DEER_TURN.to_radians()).contains(&foot.yaw_rate)
                            || foot.velocity.into_iter().any(|v| !(-DEER_SPEED..=DEER_SPEED).contains(&v))
                        { return None; }
                        Some(foot)
                    } else { None };
                    #[cfg(feature = "ee2-rect249-deer-contact")]
                    let contact = if mode == 3 {
                        let values = [word()?, word()?, word()?, word()?].map(f32::from_bits);
                        let contact = FootContact::read(values, now)?;
                        if !self.map.layout.contains_point(contact.anchor[0], contact.anchor[1]) { return None; }
                        Some(contact)
                    } else { None };
                    Some(Flight { destination, walked_since,
                        #[cfg(feature = "ee2-rect247-deer-foot")]
                        foot,
                        #[cfg(feature = "ee2-rect249-deer-contact")]
                        contact,
                    })
                }
            };
            let len = word()? as usize;
            if len > native::WATCH_CAPACITY { return None; }
            let mut rows = [AttackRecord::default(); native::WATCH_CAPACITY];
            for row in &mut rows[..len] {
                row.packed_id = word()?;
                row.time = f32::from_bits(word()?);
                if (row.packed_id & 0xffff) as usize >= MAXE || !(0.0..=now).contains(&row.time) { return None; }
            }
            let watch = AttackWatch::from_records(&rows[..len])?;
            let hp = population.wildlife.get(&id)?.hp;
            if hp == 0.0 {
                if len != 0 || flight.is_some() || decision_clock.to_bits() != 0 { return None; }
            } else if len == 0 {
                return None;
            }
            states.insert(id, DeerMotion { pose: Pose { xyz, yaw }, watch, decision_clock, flight });
        }
        if max_mode > 1 && max_mode != seen_mode { return None; }
        if word().is_some() { return None; }
        population.deer_motion = states;
        Some(())
    }

    pub(crate) fn hash_rect_deer(&self, mix: &mut impl FnMut(u32)) {
        if let Some(bytes) = self.deer_motion_section() {
            for word in bytes.chunks_exact(4) { mix(u32::from_le_bytes(word.try_into().unwrap())); }
        }
    }
}

fn read_xyz(word: &mut impl FnMut() -> Option<u32>) -> Option<[f32; 3]> {
    let point = [word()?, word()?, word()?].map(f32::from_bits);
    point.into_iter().all(f32::is_finite).then_some(point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_first_candidate_applies_both_circle_extents() {
        let mut rng = 171;
        let query = native::flee_query(&mut rng, 937, [50.0, 150.0, 1.0], [49.0, 149.5, 2.0], native::DEER_TETHER);
        let point = find_circle_destination(query.origin, query.direction, query.radius, DEER_RADIUS, Some).unwrap();
        assert_eq!(point.map(f32::to_bits), [0x4254_1763, 0x4314_2e62, 0x3f80_0000]);
    }
}
