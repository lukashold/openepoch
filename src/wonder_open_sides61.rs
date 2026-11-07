//! Mode 2 clearance boxes and the Robotics Factory's positive-X side.
//! The regular map continues to use the current integer placement projection.

use crate::{defs::*, game::{Ent, Game}, wonders61};

const FACTORY: DefId = wonders61::FIRST + 8;
const CONTACT: f32 = core::f32::consts::FRAC_1_SQRT_2;

#[derive(Clone, Copy)]
struct ClearanceBox {
    center: [f32; 2],
    half: [f32; 2],
    angle: f32,
}

fn clearance_box(site: &Ent, epoch: u8) -> Option<ClearanceBox> {
    let explicit = match site.def {
        TC => ([4.0, 3.0], [-0.25, 0.0]),
        HOUSE => ([1.5, 1.5], [0.0, 0.0]),
        BARRACKS | STABLE => ([3.0, 2.0], [-0.5, 0.0]),
        WORKSHOP | MILL | HERC_FACILITY => ([2.5, 2.5], [0.0, 0.25]),
        AIRFIELD => ([5.0, 3.0], [0.0, 0.0]),
        UNIVERSITY | FORTRESS => ([3.0, 3.0], [0.0, 0.0]),
        MARKET => ([2.0, 2.0], [0.0, 0.0]),
        TEMPLE => ([2.5, 2.5], [0.0, 0.0]),
        WAREHOUSE => ([2.0, 1.0], [0.0, 0.0]),
        95 => ([5.5, 5.5], [0.10, 0.25]),
        90..=101 if site.def != 92 => (wonders61::lookup(site.def)?.raw_footprint?, [0.0, 0.0]),
        _ => (crate::ee2_units::lookup_class(site.def, epoch)?.raw_size?, [0.0, 0.0]),
    };
    let scale = core::f32::consts::SQRT_2;
    let [x, y] = explicit.1.map(|offset| -offset * scale);
    let (sin, cos) = site.rot.sin_cos();
    let center = [site.x + x * cos - y * sin, site.y + x * sin + y * cos];
    let half = explicit.0.map(|size| (size * scale * 0.5 - 0.01).max(0.0));
    Some(ClearanceBox { center, half, angle: site.rot })
}

fn touches_factory_side(factory: ClearanceBox, neighbor: ClearanceBox) -> bool {
    let [half_x, half_y] = factory.half;
    let [width, height] = neighbor.half;
    let (sin, cos) = factory.angle.sin_cos();
    let dx = neighbor.center[0] - factory.center[0];
    let dy = neighbor.center[1] - factory.center[1];
    let local_x = dx * cos + dy * sin;
    let local_y = dy * cos - dx * sin;
    let (relative_sin, relative_cos) = (neighbor.angle - factory.angle).sin_cos();
    let projected_x = width * relative_cos.abs() + height * relative_sin.abs();
    let projected_y = width * relative_sin.abs() + height * relative_cos.abs();
    // 008639F0 classifies X endpoints first. Classification on either Y
    // side supersedes X, so a diagonal neighbor at the corner is allowed.
    let gap = local_x - (projected_x + half_x);
    (0.0..=CONTACT).contains(&gap) && local_y.abs() < projected_y + half_y
}

impl Game {
    pub(crate) fn factory_placement_blocked(&self, site: &Ent) -> bool {
        let box_for = |e: &Ent| clearance_box(e, self.players[e.team as usize].epoch);
        if site.def == FACTORY {
            let Some(factory) = box_for(site) else { return false; };
            return self.ents[..self.high].iter().filter(|e| e.alive && e.hp > 0.0 && DEFS[e.def as usize].building)
                .filter_map(box_for).any(|neighbor| touches_factory_side(factory, neighbor));
        }
        if self.wonder_sites.is_empty() { return false; }
        let Some(neighbor) = box_for(site) else { return false; };
        self.wonder_sites.iter().map(|&index| &self.ents[index])
            .filter(|e| e.def == FACTORY && e.hp > 0.0)
            .filter_map(box_for).any(|factory| touches_factory_side(factory, neighbor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_x_requires_contact_and_y_classification_takes_priority() {
        let factory = ClearanceBox { center: [0.0, 0.0], half: [5.0, 3.0], angle: 0.0 };
        let neighbor = |x, y| ClearanceBox { center: [x, y], half: [1.0, 1.0], angle: 0.0 };
        assert!(touches_factory_side(factory, neighbor(6.0, 0.0)));
        assert!(touches_factory_side(factory, neighbor(6.5, 0.0)));
        assert!(!touches_factory_side(factory, neighbor(6.75, 0.0)));
        assert!(!touches_factory_side(factory, neighbor(-6.5, 0.0)));
        assert!(!touches_factory_side(factory, neighbor(6.5, 4.0)));
        let turned = ClearanceBox { angle: core::f32::consts::FRAC_PI_2, ..factory };
        assert!(touches_factory_side(turned, neighbor(0.0, 6.5)));
        assert!(!touches_factory_side(turned, neighbor(6.5, 0.0)));
    }
}
