//! Portable watch and flee proposal for the stock authored Deer path.
//!
//! The caller owns the UnitAI decision gate, source clock/RNG, current packed IDs, threat
//! scoring, geometry and task admission/commit. See DeerReaction244 for original witnesses.

pub const WATCH_CAPACITY: usize = 9;
pub const DEER_TETHER: f32 = 10.0;
pub const FLEE_DECISION_DELAY: f32 = 1.0;
pub const FLEE_COMMIT_COOLDOWN: f32 = 5.0;
pub const MOVE_TASK_TAG: u32 = 3;

const THREAT_WINDOW: f64 = 1.0;
const ANGLE_STEP: f32 = f32::from_bits(0x3d00_adfd);
const DISTANCE_STEP: f32 = f32::from_bits(0x364c_ce66);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AttackRecord {
    pub packed_id: u32,
    pub time: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttackWatch {
    len: usize,
    rows: [AttackRecord; WATCH_CAPACITY],
}

impl AttackWatch {
    pub fn records(&self) -> &[AttackRecord] {
        &self.rows[..self.len]
    }

    /// Restore source insertion order. Reject overfull, duplicate or non-finite saved rows.
    pub fn from_records(records: &[AttackRecord]) -> Option<Self> {
        if records.len() > WATCH_CAPACITY {
            return None;
        }
        let mut watch = Self::default();
        for (index, record) in records.iter().enumerate() {
            if !record.time.is_finite()
                || records[..index]
                    .iter()
                    .any(|earlier| earlier.packed_id == record.packed_id)
            {
                return None;
            }
            watch.rows[index] = *record;
        }
        watch.len = records.len();
        Some(watch)
    }

    /// Called before HP application, with the victim's current HP fraction and controller gate.
    /// The decision clock is the caller's actual `UnitAI+0x0c` field.
    pub fn observe_direct_hit(
        &mut self,
        victim_hp_fraction: f32,
        has_controller: bool,
        packed_attacker: u32,
        now: f32,
        decision_clock: &mut f32,
    ) -> bool {
        if !has_controller
            || !victim_hp_fraction.is_finite()
            || victim_hp_fraction <= 0.0
            || !now.is_finite()
        {
            return false;
        }
        *decision_clock = 0.0;
        let record = AttackRecord {
            packed_id: packed_attacker,
            time: now,
        };
        if let Some(index) = self
            .records()
            .iter()
            .position(|row| row.packed_id == packed_attacker)
        {
            self.rows[index] = record;
        } else if self.len < WATCH_CAPACITY {
            self.rows[self.len] = record;
            self.len += 1;
        } else {
            // The original scans left to right with a strict comparison and an initial now + 1.
            let mut oldest = None;
            let mut oldest_time = f64::from(now) + 1.0;
            for (index, row) in self.records().iter().enumerate() {
                let time = f64::from(row.time);
                if time < oldest_time {
                    oldest = Some(index);
                    oldest_time = time;
                }
            }
            if let Some(index) = oldest {
                self.rows[index] = record;
            }
        }
        true
    }

    /// `lookup` resolves the full packed ID and `excluded` supplies the original 00857450 gate.
    /// `score` supplies the unrounded source result of 0086E630. The native best spills to F32.
    pub fn select_threat<T>(
        &self,
        now: f32,
        mut lookup: impl FnMut(u32) -> Option<T>,
        mut excluded: impl FnMut(&T) -> bool,
        mut score: impl FnMut(&T) -> f64,
    ) -> Option<T> {
        if !now.is_finite() {
            return None;
        }
        let mut best = None;
        let mut best_score = 0.0_f32;
        for record in self.records() {
            if f64::from(now) - f64::from(record.time) >= THREAT_WINDOW {
                continue;
            }
            let Some(candidate) = lookup(record.packed_id) else {
                continue;
            };
            if !excluded(&candidate) {
                let strength = score(&candidate);
                if strength > f64::from(best_score) {
                    best_score = strength as f32;
                    best = Some(candidate);
                }
            }
        }
        best
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FleeQuery {
    /// Both native actor arguments are this fleeing actor.
    pub actor: u32,
    pub origin: [f32; 3],
    pub direction: [f32; 2],
    /// The same word is used for both the minimum and maximum search radius.
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FleeTask {
    pub tag: u32,
    pub destination: [f32; 3],
    pub decision_delay: f32,
    pub disposition: u32,
}

fn draw_high15(shared_rng: &mut u32) -> u32 {
    *shared_rng = shared_rng
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223);
    (*shared_rng >> 16) & 0x7fff
}

/// The original rotates its spilled F32 vector with individually spilled sine/cosine.
pub fn rotate_direction(vector: [f32; 2], angle_step: i32) -> [f32; 2] {
    let radians = f64::from(angle_step) * f64::from(ANGLE_STEP);
    let (sin, cos) = radians.sin_cos();
    let (sin, cos) = (f64::from(sin as f32), f64::from(cos as f32));
    let [x, y] = vector.map(f64::from);
    [(cos * x - sin * y) as f32, (cos * y + sin * x) as f32]
}

/// Call after the native-style threat selector and decision gate. Both draws precede geometry.
pub fn flee_query(
    shared_rng: &mut u32,
    actor: u32,
    origin: [f32; 3],
    threat: [f32; 3],
    tether: f32,
) -> FleeQuery {
    let angle_step = (draw_high15(shared_rng) % 101) as i32 - 50;
    let direction = rotate_direction([origin[0] - threat[0], origin[1] - threat[1]], angle_step);
    let distance = 0.25 + f64::from(draw_high15(shared_rng)) * f64::from(DISTANCE_STEP);
    let radius = (distance * f64::from(tether)) as f32;
    FleeQuery {
        actor,
        origin,
        direction,
        radius,
    }
}

/// The geometry callback supplies the accepted destination. The task-admission callback may
/// reject it. This function only proposes, leaving queue ownership and cooldown to the caller.
pub fn propose_flee(
    shared_rng: &mut u32,
    actor: u32,
    origin: [f32; 3],
    threat: [f32; 3],
    tether: f32,
    disposition: u32,
    geometry: impl FnOnce(FleeQuery) -> Option<[f32; 3]>,
    admit_task: impl FnOnce(&FleeTask) -> bool,
) -> Option<FleeTask> {
    let destination = geometry(flee_query(shared_rng, actor, origin, threat, tether))?;
    let task = FleeTask {
        tag: MOVE_TASK_TAG,
        destination,
        decision_delay: FLEE_DECISION_DELAY,
        disposition,
    };
    admit_task(&task).then_some(task)
}
