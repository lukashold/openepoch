//! Authenticated authored Valley building cursor and Farm terrain admission.

use super::{Geometry, VALLEY_SHA};
use crate::{
    cmd::{C_VALLEY_FARM_CURSOR, C_VALLEY_HOUSE_CURSOR},
    defs::{DefId, FARM, HOUSE},
    game::Game,
    hotspot_start66::{quad_from_basis, raster_quad, snap_from_basis},
    placement::PlacementSite,
};

pub(crate) const INITIAL: f32 = f32::from_bits(0xbf49_0fdb);
const PERIOD: f32 = f32::from_bits(0x40c9_0fdb);
const STEPS: [f32; 5] = [
    f32::from_bits(0x3fc9_0fda),
    f32::from_bits(0x3f49_0fdb),
    f32::from_bits(0x3ec9_0fdb),
    f32::from_bits(0x3e49_0fd8),
    f32::from_bits(0x3dc9_0fdf),
];
const BODY: f32 = f32::from_bits(0x4007_c3b6);
const PADDED: f32 = f32::from_bits(0x4062_5692);
const CORNERS: [[f32; 2]; 4] = [[1., 1.], [1., -1.], [-1., -1.], [-1., 1.]];

#[derive(Clone, Copy)]
pub(crate) struct BuildingCursorPose {
    def: DefId,
    xyz: [f32; 3],
    yaw: f32,
}

impl BuildingCursorPose {
    pub(crate) fn site(self) -> PlacementSite {
        PlacementSite::valley_cursor(self.def, self.xyz[0], self.xyz[1], self.yaw)
    }
}

pub(crate) struct BuildingCursor {
    def: DefId,
    yaw: f32,
    requested: Option<[f32; 3]>,
    resolved: Option<BuildingCursorPose>,
    validation_revision: u32,
}

impl Default for BuildingCursor {
    fn default() -> Self {
        Self { def: FARM, yaw: INITIAL, requested: None, resolved: None, validation_revision: 0 }
    }
}

fn valid_yaw(yaw: f32) -> bool {
    (INITIAL..=PERIOD).contains(&yaw)
}

fn basis(yaw: f32) -> [f32; 2] {
    let (sin, cos) = f64::from(yaw).sin_cos();
    [cos as f32, sin as f32]
}

fn in_world(world: &Geometry, [x,y]: [f32; 2]) -> bool {
    (0.0..world.width as f32).contains(&x) && (0.0..world.height as f32).contains(&y)
}

fn nearby([x,y]: [f32; 2]) -> impl Iterator<Item=[f32; 2]> {
    // 00862DB0 alternates half-step directions around the four signed corners.
    (1..=3).flat_map(move |ring| {
        (0..2 * ring).flat_map(move |k| {
            let radius = ring as f32 * 0.5;
            let step = ((k + 1) / 2) as f32 * 0.5;
            let signed = if k % 2 == 0 { step } else { -step };
            CORNERS.into_iter().map(move |[sx, sy]| {
                [(x + sx * radius) + sy * signed, (y + sy * radius) - sx * signed]
            })
        })
    })
}

impl Game {
    pub(crate) fn valley_farm_cursor_enabled(&self) -> bool {
        !self.net && self.bench == 0 && self.valley_fixture.is_some() &&
            self.map.source_world.as_ref().is_some_and(|world| world.source_sha == VALLEY_SHA)
    }

    pub(crate) fn valley_house_cursor_enabled(&self) -> bool {
        self.valley_farm_cursor_enabled() && self.uses_valley_house_movement(HOUSE)
    }

    pub(crate) fn valley_farm_terrain_allows(&self, x: f32, y: f32, yaw: f32) -> bool {
        if !self.valley_farm_cursor_enabled() || !valid_yaw(yaw) { return false; }
        let Some(world) = self.map.source_world.as_ref() else { return false; };
        let Some(quad) = quad_from_basis([x, y], [PADDED; 2], basis(yaw)) else { return false; };
        let Some(raster) = raster_quad(quad, world.width as u32, world.height as u32, true) else { return false; };
        if raster.contains_outside_corner || raster.corners.is_empty() { return false; }
        let stride = world.width + 1;
        let material = |at: usize| world.move_palette[world.materials[at] as usize];
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut sum = 0.0_f64;
        for [x,y] in &raster.corners {
            let at = *x as usize + *y as usize * stride;
            let h = world.heights[at];
            if h < 0.0 || material(at) != 0 || material(at+1) == 3 || material(at+stride) == 3 {
                return false;
            }
            min = min.min(h);
            max = max.max(h);
            sum += f64::from(h);
        }
        let reference = (sum / raster.corners.len() as f64) as f32;
        // s_maxBldgElevDifferential is 1.0, split across both sides of the
        // reference height by 00862421. The default height quantum is zero.
        f64::from(max) - f64::from(reference) <= 0.5 &&
            f64::from(reference) - f64::from(min) <= 0.5
    }

    pub(crate) fn valley_cursor_reset(&mut self, def: DefId, yaw: f32) {
        let yaw = if valid_yaw(yaw) { yaw } else { INITIAL };
        self.valley_farm_cursor = BuildingCursor { def, yaw, ..BuildingCursor::default() };
    }

    pub(crate) fn valley_farm_cursor_pose(&self) -> Option<BuildingCursorPose> {
        self.valley_farm_cursor_enabled().then_some(self.valley_farm_cursor.resolved).flatten()
    }

    pub(crate) fn valley_farm_cursor_component(&self, field: u32) -> f32 {
        match (self.valley_farm_cursor_pose(), field) {
            (Some(pose), 0..=2) => pose.xyz[field as usize],
            (_, 3) => self.valley_farm_cursor.yaw,
            _ => f32::NAN,
        }
    }

    pub(crate) fn valley_farm_cursor_validation_revision(&self) -> u32 {
        self.valley_farm_cursor.validation_revision
    }

    fn valley_farm_resolve(&mut self, origin: [f32; 2]) {
        let Some(world) = self.map.source_world.as_ref() else { return; };
        let def = self.valley_farm_cursor.def;
        let yaw = self.valley_farm_cursor.yaw;
        let Some([x,y]) = snap_from_basis(origin, [BODY; 2], basis(yaw)) else {
            self.valley_farm_cursor.resolved = None;
            return;
        };
        let xi = (x as i32).clamp(0, world.width as i32) as usize;
        let yi = (y as i32).clamp(0, world.height as i32) as usize;
        let next = [x,y,world.heights[yi * (world.width + 1) + xi].max(0.0)];
        let requested = self.valley_farm_cursor.requested.filter(|p| {
            let dx = f64::from(p[0]) - f64::from(x);
            let dy = f64::from(p[1]) - f64::from(y);
            dx*dx + dy*dy < 0.5
        }).unwrap_or(next);
        let cursor = &mut self.valley_farm_cursor;
        if cursor.requested != Some(requested) || cursor.resolved.is_none_or(|pose| pose.yaw != yaw) {
            cursor.validation_revision = cursor.validation_revision.wrapping_add(1);
        }
        self.valley_farm_cursor.requested = Some(requested);
        let direct = BuildingCursorPose { def, xyz: requested, yaw };
        let mut resolved = direct;
        if self.placement_geometry_for(self.local, direct.site()) != 0 {
            for [x,y] in nearby([requested[0],requested[1]]) {
                if !in_world(world, [x,y]) { continue; }
                let z = world.base_height(&world.heights,x,y);
                let candidate = BuildingCursorPose { def, xyz: [x,y,z], yaw };
                if self.placement_geometry_for(self.local, candidate.site()) == 0 {
                    resolved = candidate;
                    break;
                }
            }
        }
        self.valley_farm_cursor.resolved = Some(resolved);
    }

    pub(crate) fn valley_farm_cursor_pointer(&mut self, x: f32, y: f32) -> u32 {
        if !self.valley_farm_cursor_enabled() { return 1; }
        if !self.map.source_world.as_ref().is_some_and(|world| in_world(world, [x,y])) {
            self.valley_farm_cursor.resolved = None;
            return 1;
        }
        self.valley_farm_resolve([x,y]);
        self.valley_farm_cursor_error()
    }

    pub(crate) fn valley_farm_cursor_rotate(&mut self, direction: i32, mode: u32) -> u32 {
        let Some(&step) = STEPS.get(mode as usize) else { return 1; };
        if !matches!(direction, -1 | 1) { return 1; }
        self.valley_farm_cursor_force_rotation(-(direction as f32) * step)
    }

    pub(crate) fn valley_farm_cursor_force_rotation(&mut self, step: f32) -> u32 {
        let Some(pose) = self.valley_farm_cursor_pose() else { return 1; };
        let period = f64::from(PERIOD);
        let yaw = f64::from(pose.yaw) - f64::from(step);
        self.valley_farm_cursor.yaw = yaw.rem_euclid(period) as f32;
        self.valley_farm_resolve([pose.xyz[0],pose.xyz[1]]);
        self.valley_farm_cursor_error()
    }

    pub(crate) fn valley_farm_cursor_error(&mut self) -> u32 {
        let Some(pose) = self.valley_farm_cursor_pose() else { return 1; };
        let ids = crate::selected(self);
        self.placement_error_at(self.local, pose.site(), &ids, true)
    }

    pub(crate) fn valley_farm_cursor_place(&mut self) -> u32 {
        if self.winner != 0 || self.valley_farm_cursor_error() != 0 { return 0; }
        let Some(pose) = self.valley_farm_cursor_pose() else { return 0; };
        let handles = crate::handles(self, &crate::selected(self));
        let command = if pose.def == HOUSE { C_VALLEY_HOUSE_CURSOR } else { C_VALLEY_FARM_CURSOR };
        self.submit(command, pose.xyz[0], pose.xyz[1], pose.yaw.to_bits() as i32, &handles);
        1
    }

    pub(crate) fn valley_farm_cursor_time_multiplier(&self) -> f32 {
        self.valley_farm_cursor_pose().map_or(1.0, |pose| {
            let mut actor = pose.site().actor;
            actor.team = self.local;
            self.construction_multiplier(&actor)
        })
    }

    pub(crate) fn apply_valley_cursor_pose(&mut self, team: u8, def: DefId, x: f32, y: f32, yaw: f32, ids: &[usize]) {
        let enabled = if def == HOUSE { self.valley_house_cursor_enabled() } else { self.valley_farm_cursor_enabled() };
        if team == self.local && self.winner == 0 && enabled && valid_yaw(yaw) &&
            self.map.source_world.as_ref().is_some_and(|world| in_world(world, [x,y])) {
            self.begin_construction_at(team, PlacementSite::valley_cursor(def,x,y,yaw), ids, true);
        }
    }
}
