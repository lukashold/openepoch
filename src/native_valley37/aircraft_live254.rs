//! Authored Airplane task, event-1 motion and deterministic checkpoint state.
use super::{
    aircraft252::{self, AirPhase, AirplaneMove, AirplaneState, CircleStart, FlightFrame},
    deer_foot247::{self, FootMotion, Rounding, TAU},
    wildlife_hunting240::Pose,
};

pub(super) const MAP_MAX: [f32; 2] = [100.0, 300.0];
const MODE: Rounding = Rounding::Chop53;

#[derive(Clone, Copy)]
pub(super) struct Flight {
    state: AirplaneState,
    root: Option<Root>,
}

#[derive(Clone, Copy)]
struct Root {
    requested: [f32; 3],
    task: AirplaneMove,
}

impl Flight {
    pub(super) fn start(pose: Pose, requested: [f32; 3], height: impl FnMut([f32; 2]) -> f64) -> Self {
        let yaw = deer_foot247::store(deer_foot247::wrap_positive(f64::from(pose.yaw), MODE), MODE);
        Self {
            state: aircraft252::start_airplane(pose.xyz, yaw, MODE, height),
            root: Some(Root { requested, task: AirplaneMove::direct(requested) }),
        }
    }

    pub(super) fn busy(&self) -> bool { self.root.is_some() }

    #[cfg(feature = "ee2-rect255-aircraft-visual")]
    pub(super) fn pending_walk(&self) -> bool {
        self.state.phase == AirPhase::Takeoff && self.state.motion.speed == 0.0
    }

    pub(super) fn restart(&mut self, requested: [f32; 3]) {
        assert!(!self.busy());
        let task = aircraft252::restart_airplane(&mut self.state, AirplaneMove::direct(requested), MAP_MAX)
            .expect("an unlinked aircraft remains in phase 3");
        self.root = Some(Root { requested, task });
    }

    pub(super) fn advance(&mut self, id: u32, now: f32, dt: f32, height: impl FnMut([f32; 2]) -> f64) -> Pose {
        // At the next UnitAI actor update, remove a previously completed root before event 1.
        if self.root.is_some_and(|root| root.task.arrived) { self.root = None; }
        if let Some(root) = &mut self.root {
            let frame = FlightFrame { dt, now, packed_actor_id: id, map_max_xy: MAP_MAX };
            aircraft252::step_airplane(&mut self.state, &mut root.task, frame, MODE, height)
                .expect("a linked active Move requires phase 1 or 2");
        } else {
            assert!(aircraft252::step_airplane_circle(&mut self.state, dt, MODE, height));
        }
        Pose { xyz: self.state.position, yaw: self.state.yaw }
    }

    pub(super) fn word(&self, field: u32) -> Option<u32> {
        Some(match field {
            18..=20 => self.root?.task.target[(field - 18) as usize].to_bits(),
            29 => u32::from(self.busy()),
            40 => self.state.phase as u32,
            41 => self.root.map_or(0, |root| if root.task.arrived { 2 } else { 1 }),
            42 => u32::from(self.root?.task.direct_path_ready),
            43 => self.state.motion.speed.to_bits(),
            44 => self.state.motion.yaw_rate.to_bits(),
            45..=46 => self.state.motion.velocity[(field - 45) as usize].to_bits(),
            47 => self.state.takeoff_agl.to_bits(),
            48..=50 => self.state.circle?.anchor[(field - 48) as usize].to_bits(),
            51 => u32::from(self.state.circle?.turn_left),
            52..=54 => self.root?.requested[(field - 52) as usize].to_bits(),
            55 => u32::from(self.state.unreachable),
            _ => return None,
        })
    }

    pub(super) fn encode(self, word: &mut impl FnMut(u32)) {
        word(self.state.phase as u32);
        for value in [self.state.motion.speed, self.state.motion.yaw_rate]
            .into_iter().chain(self.state.motion.velocity).chain([self.state.takeoff_agl]) {
            word(value.to_bits());
        }
        word(self.state.circle.map_or(0, |circle| if circle.turn_left { 2 } else { 1 })
            | 4 * u32::from(self.state.unreachable));
        if let Some(circle) = self.state.circle {
            for value in circle.anchor { word(value.to_bits()); }
        }
        word(self.root.map_or(0, |root| 1 + u32::from(root.task.direct_path_ready) + 2 * u32::from(root.task.arrived)));
        if let Some(root) = self.root {
            for value in root.requested { word(value.to_bits()); }
        }
    }

    pub(super) fn decode(pose: Pose, word: &mut impl FnMut() -> Option<u32>) -> Option<Self> {
        let phase = match word()? {
            1 => AirPhase::Takeoff,
            2 => AirPhase::Travel,
            3 => AirPhase::Circle,
            _ => return None,
        };
        let motion = FootMotion {
            speed: scalar(word)?, yaw_rate: scalar(word)?, velocity: [scalar(word)?, scalar(word)?],
        };
        let takeoff_agl = scalar(word)?;
        let state_flags = word()?;
        if state_flags > 6 { return None; }
        let circle = match state_flags & 3 {
            0 => None,
            flag @ (1 | 2) => {
                let anchor = xyz(word)?;
                if !point_in_map(anchor) { return None; }
                Some(CircleStart { anchor, turn_left: flag == 2 })
            }
            _ => return None,
        };
        let root = match word()? {
            0 => None,
            flags @ (1 | 2 | 4) => {
                let requested = xyz(word)?;
                if !(0..2).all(|axis| (-5.0..=MAP_MAX[axis] + 5.0).contains(&requested[axis])) {
                    return None;
                }
                let mut task = AirplaneMove::direct(requested);
                if flags != 1 { task.clamp_path(MAP_MAX); }
                task.arrived = flags == 4;
                Some(Root { requested, task })
            }
            _ => return None,
        };
        if !(0.0..=TAU).contains(&f64::from(pose.yaw)) || !source_position(pose.xyz)
            || !(0.0..=3.45).contains(&motion.speed)
            || motion.velocity.into_iter().any(|v| v.abs() > motion.speed)
            || motion.yaw_rate.abs() > f32::from_bits(0x3fc9_0fda)
            || match phase {
                AirPhase::Takeoff => circle.is_some() || root.is_none_or(|r| r.task.direct_path_ready)
                    || motion.yaw_rate != 0.0,
                AirPhase::Travel => root.is_none_or(|r| r.task.arrived || circle.is_some() && !r.task.direct_path_ready),
                AirPhase::Circle => circle.is_none() || root.is_some_and(|r| !r.task.arrived),
            }
        { return None; }
        Some(Self { state: AirplaneState { position: pose.xyz, yaw: pose.yaw, motion,
            phase, takeoff_agl, circle, unreachable: state_flags & 4 != 0 }, root })
    }
}

fn scalar(word: &mut impl FnMut() -> Option<u32>) -> Option<f32> {
    let value = f32::from_bits(word()?);
    value.is_finite().then_some(value)
}

fn xyz(word: &mut impl FnMut() -> Option<u32>) -> Option<[f32; 3]> {
    Some([scalar(word)?, scalar(word)?, scalar(word)?])
}

fn point_in_map(xyz: [f32; 3]) -> bool {
    (0..2).all(|axis| (0.0..=MAP_MAX[axis]).contains(&xyz[axis]))
}

pub(super) fn source_position(xyz: [f32; 3]) -> bool {
    // Flight commits XYZ directly. Height leaf indexing uses signed cell
    // coordinates and probes up to two units beyond the pose.
    xyz[..2].iter().all(|v| (-32765.0..=32764.0).contains(v)) && xyz[2].is_finite()
}
