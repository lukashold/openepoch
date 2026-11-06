//! Airport runway points shared by the original E11 and E13 body models.

use crate::carrier36_defs::CARRIER;
use crate::game::{Ent, Game};

pub(crate) const SAVE_FLAG: u32 = 1 << 7;
pub(crate) const HASH_TAG: u32 = u32::from_le_bytes(*b"RWY1");
const SCALE: f64 = f32::from_bits(0x3ce7b46a) as f64;
// Convert the source NIF X,Y plane to the renderer's X,-Y ground plane.
const START: [f32; 2] = [f32::from_bits(0xc247a497), -f32::from_bits(0x42c823f0)];
const END: [f32; 2] = [f32::from_bits(0xc247a497), -f32::from_bits(0xc2db79b7)];
const DECK_SCALE: f64 = f32::from_bits(0x3d90d0c3) as f64;
const DECK_MARKERS: [[[u32; 3]; 2]; 2] = [
    [[0xbfad375b, 0x41a6c6d2, 0x40c7b85a], [0xbfad3790, 0xc1caac68, 0x40c7b85a]],
    [[0x3f3df4e5, 0xbe7358d6, 0x4108ad5d], [0x3f3df4e5, 0x41b1952b, 0x4107d09d]],
];

#[derive(Clone, Copy)]
pub(crate) struct Departure {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
}

fn world_point(host: &Ent, [x, y]: [f32; 2], rotation: f32, scale: f64) -> [f32; 2] {
    let (sin, cos) = rotation.sin_cos();
    let (s, c, x, y) = (f64::from(sin), f64::from(cos), f64::from(x), f64::from(y));
    // 008E6970 stores each rotated component as f32. 008741E0 stores the
    // scaled Y before translation, while X remains extended through addition.
    let rx = f64::from((x * c - y * s) as f32);
    let ry = f64::from((x * s + y * c) as f32);
    [
        (rx * scale + f64::from(host.x)) as f32,
        (f64::from((ry * scale) as f32) + f64::from(host.y)) as f32,
    ]
}

pub(crate) fn departure(airport: &Ent) -> Departure {
    departure_at(markers(airport))
}

pub(crate) fn departure_at([[x, y], [end_x, end_y]]: [[f32; 2]; 2]) -> Departure {
    // 007E9CA0..CB8 stores the differences before 00A1DD10 performs
    // FPATAN. Keep the signed angle of the renderer's ground coordinates.
    let heading = f64::from(end_y - y).atan2(f64::from(end_x - x)) as f32;
    Departure { x, y, heading }
}

pub(crate) fn markers(airport: &Ent) -> [[f32; 2]; 2] {
    [START, END].map(|point| world_point(airport, point, airport.rot, SCALE))
}

impl Game {
    fn carrier_tags(&self, host: &Ent) -> [[f32; 3]; 2] {
        let upgraded = self.players[host.team as usize].epoch >= 13;
        DECK_MARKERS[usize::from(upgraded)].map(|point| point.map(f32::from_bits))
    }

    pub(crate) fn air_host_markers(&self, host: &Ent) -> [[f32; 2]; 2] {
        if host.def != CARRIER { return markers(host); }
        let rotation = host.rot - std::f32::consts::FRAC_PI_2;
        self.carrier_tags(host).map(|[x, y, _]| world_point(host, [x, -y], rotation, DECK_SCALE))
    }

    pub(crate) fn carrier_deck_offset(&self, host: &Ent, tag: usize) -> f32 {
        (f64::from(self.carrier_tags(host)[tag][2]) * DECK_SCALE) as f32
    }
}

impl Game {
    pub(crate) fn configure_peer_air_runway(&mut self, enabled: bool) -> bool {
        if !self.net || self.bench != 0 || self.tick != 0 || !self.outbox.is_empty() ||
            !self.inbox.is_empty() || (enabled && !self.air_missions_enabled) {
            return false;
        }
        self.air_runway_enabled = enabled;
        if !enabled { self.set_air_takeoff_enabled(false); }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::E0;

    #[test]
    fn airport_rotation_rotates_the_runway_and_the_departure_direction() {
        let mut airport = Ent { x: 100.5, y: 110.5, ..E0 };
        let distance = |a: f32, b: f32| (a - b).abs();
        let (base_x, base_y) = (-1.411688, -2.830412);
        for step in 0..8 {
            let angle = step as f32 * std::f32::consts::FRAC_PI_4;
            airport.rot = angle;
            let (sin, cos) = angle.sin_cos();
            let pose = departure(&airport);
            assert!(distance(pose.x, airport.x + base_x * cos - base_y * sin) < 0.0001);
            assert!(distance(pose.y, airport.y + base_x * sin + base_y * cos) < 0.0001);
            assert!(distance(pose.heading.cos(), -sin) < 0.0001);
            assert!(distance(pose.heading.sin(), cos) < 0.0001);
        }
    }

    #[test]
    fn runway_capability_requires_air_missions_and_tick_zero() {
        let mut world = Game::new(7, 0, 0, 2, 0);
        assert!(!world.configure_peer_air_runway(true));
        assert!(world.configure_peer_air_missions(true));
        assert!(world.configure_peer_air_runway(true));
        assert!(world.configure_peer_air_missions(false));
        assert!(!world.air_runway_enabled);
        world.tick();
        assert!(!world.configure_peer_air_runway(false));
    }
}
