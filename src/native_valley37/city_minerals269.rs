//! Interactive City placement against the authored stationary mineral boxes.
use super::population234::{SourceUnit, UnitKind};
use crate::game::{Ent, Game};

const SIZE_SCALE: f32 = f32::from_bits(0x3fb5_04f3);
const EDGE_CONTACT: f32 = f32::from_bits(0x3f35_04f3);
const CITY_OPEN_SIDES: u8 = 0b0001;
const MINERAL_OPEN_SIDES: u8 = 0b1111;

#[derive(Clone, Copy)]
pub(super) struct BoxBound {
    center: [f32; 3],
    half: [f32; 3],
    yaw: f32,
    axes: [[f32; 2]; 2],
}

impl BoxBound {
    #[cfg(feature = "ee2-rect271-build-egress")]
    pub(super) fn from_half_extents(center: [f32; 3], half: [f32; 3], yaw: f32) -> Self {
        let [cos, sin] = basis(yaw);
        Self { center, half, yaw, axes: [[cos, sin], [-sin, cos]] }
    }

    pub(super) fn new(mut origin: [f32; 3], size: [f32; 2], height: f32, yaw: f32, offset_x: f32) -> Self {
        let [cos, sin] = basis(yaw);
        let [hx, hy] = size.map(|extent| {
            let scaled = extent * SIZE_SCALE;
            (f64::from(scaled) * 0.5 - f64::from(0.01_f32)) as f32
        });
        // The source Box factory returns a rotated negative Undertexture
        // offset, followed by the caller's F32 world-position stores.
        let offset = offset_x * SIZE_SCALE;
        origin[0] += offset * cos;
        origin[1] += offset * sin;
        origin[2] += height * 0.5;
        Self { center: origin, half: [hx, hy, height * 0.5], yaw, axes: [[cos, sin], [-sin, cos]] }
    }

    pub(super) fn overlaps(self, other: Self) -> bool {
        let delta = [0, 1, 2].map(|i| f64::from(other.center[i]) - f64::from(self.center[i]));
        // For these Z-up boxes the cross axes reduce to the four horizontal
        // face normals and the common vertical axis.
        if delta[2].abs() > f64::from(self.half[2]) + f64::from(other.half[2]) {
            return false;
        }
        [(self, other), (other, self)].into_iter().all(|(a, b)| {
            a.axes.into_iter().enumerate().all(|(i, axis)| {
                let axis = axis.map(f64::from);
                let distance = (delta[0] * axis[0] + delta[1] * axis[1]).abs();
                let projection = |j: usize| {
                    let [x, y] = b.axes[j].map(f64::from);
                    (x * axis[0] + y * axis[1]).abs() * f64::from(b.half[j])
                };
                distance <= f64::from(a.half[i]) + projection(0) + projection(1)
            })
        })
    }

    fn diagonal(self) -> [f32; 2] {
        let [u, v] = self.axes;
        let [hx, hy, _] = self.half;
        // 00863AF0 retains the second X product in the x87 register.
        let x = f64::from(hx * u[0]) + f64::from(hy) * f64::from(v[0]);
        [x as f32, hx * u[1] + hy * v[1]]
    }

    #[cfg(feature = "ee2-rect270-city-actors")]
    pub(super) fn with_center_z(mut self, z: f32) -> Self {
        self.center[2] = z;
        self
    }

    #[cfg(feature = "ee2-rect270-city-actors")]
    pub(super) fn overlaps_upright_capsule(self, center: [f32; 3], radius: f32, extent: f32) -> bool {
        let [dx, dy, dz] = [0, 1, 2].map(|i| f64::from(center[i]) - f64::from(self.center[i]));
        let radius = f64::from(radius);
        // The native vertical-capsule tests use inclusive projected
        // intervals, including at the expanded box corners.
        dz.abs() <= f64::from(self.half[2]) + f64::from(extent) + radius
            && self.axes.into_iter().enumerate().all(|(i, [x, y])| {
                (dx * f64::from(x) + dy * f64::from(y)).abs() <= f64::from(self.half[i]) + radius
            })
    }
}

fn basis(yaw: f32) -> [f32; 2] {
    let (sin, cos) = f64::from(yaw).sin_cos();
    [cos as f32, sin as f32]
}

fn rotate([x, y]: [f32; 2], yaw: f32) -> [f32; 2] {
    let [c, s] = basis(yaw).map(f64::from);
    let (x, y) = (f64::from(x), f64::from(y));
    [(x * c - y * s) as f32, (x * s + y * c) as f32]
}

struct EdgeFrame {
    delta: [f32; 2],
    own: [f32; 2],
    other: [f32; 2],
}

impl EdgeFrame {
    fn new(own: BoxBound, other: BoxBound) -> Self {
        Self {
            delta: rotate([other.center[0] - own.center[0], other.center[1] - own.center[1]], -own.yaw),
            own: rotate(own.diagonal(), -own.yaw),
            other: rotate(other.diagonal(), -own.yaw),
        }
    }

    fn touches_edge(&self) -> bool {
        (0..2).any(|axis| {
            let residual = f64::from(self.delta[axis].abs())
                - (f64::from(self.other[axis].abs()) + f64::from(self.own[axis]));
            (residual as f32).abs() <= EDGE_CONTACT
        })
    }

    fn respects_open_sides(&self, mask: u8) -> bool {
        let classified = |axis: usize| {
            let delta = f64::from(self.delta[axis]);
            let projection = f64::from(self.other[axis]).abs();
            let half = f64::from(self.own[axis]);
            if delta - projection >= half {
                Some(1 << axis)
            } else if delta + projection <= -half {
                Some(4 << axis)
            } else {
                None
            }
        };
        // 008639F0 replaces the X classification when a Y edge classifies.
        classified(1).or_else(|| classified(0)).is_none_or(|side| mask & side == 0)
    }
}

#[derive(Clone, Copy)]
pub(super) struct PlacementBoxes {
    pub(super) outer: BoxBound,
    pub(super) inner: BoxBound,
    pub(super) open_sides: u8,
}

impl PlacementBoxes {
    pub(super) fn blocks(self, existing: Self) -> bool {
        if !self.outer.overlaps(existing.outer) {
            return false;
        }
        if self.inner.overlaps(existing.inner) {
            return true;
        }
        // Only an outer hit reaches the interactive 008639F0 exception.
        // The first edge residual is in the existing actor's local frame.
        let edge = EdgeFrame::new(existing.inner, self.inner);
        !(edge.touches_edge() && edge.respects_open_sides(existing.open_sides)
            && EdgeFrame::new(self.inner, existing.inner).respects_open_sides(self.open_sides))
    }
}

fn mineral_boxes(mineral: &SourceUnit) -> PlacementBoxes {
    // Initialized ResourceNotTree profile: source UnitHeight is 1.0.
    PlacementBoxes {
        outer: BoxBound::new(mineral.xyz, [1.5; 2], 1.0, mineral.yaw, 0.0),
        inner: BoxBound::new(mineral.xyz, [0.3; 2], 1.0, mineral.yaw, 0.0),
        open_sides: MINERAL_OPEN_SIDES,
    }
}

impl Game {
    pub(super) fn rect_native_city_collision(&self, site: &Ent) -> bool {
        self.uses_generated_city_foundation(site.def) && self.rect_ai_clock.is_some()
            && self.supports_rectangular_city_foundations()
    }

    pub(super) fn rect_city_placement_boxes(&self, site: &Ent) -> PlacementBoxes {
        let origin = [site.x, site.y, self.map.height(site.x, site.y)];
        // Context +74 uses City footprint, +84 uses the Undertexture shape.
        // The prospective placement wrapper supplies height 5 in both modes.
        PlacementBoxes {
            outer: BoxBound::new(origin, [4.0, 3.5], 5.0, site.rot, 0.0),
            inner: BoxBound::new(origin, [4.0, 3.0], 5.0, site.rot, 0.25),
            open_sides: CITY_OPEN_SIDES,
        }
    }

    pub(crate) fn rect_city_mineral_blocked(&self, site: &Ent) -> bool {
        if !self.rect_native_city_collision(site) {
            return false;
        }
        let Some(population) = &self.rect_population else { return false; };
        let city = self.rect_city_placement_boxes(site);
        population.source().units().iter().filter(|unit| unit.kind == UnitKind::Mineral && unit.hp_fraction > 0.0)
            .any(|unit| city.blocks(mineral_boxes(unit)))
    }
}
