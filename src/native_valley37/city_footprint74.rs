//! Mode-1 CityCenter collision projected onto the existing whole-tile navigator.
use crate::defs::TC;
use crate::game::{Game, Ent};

const AXIS: f64 = f32::from_bits(0x3f35_04f3) as f64;
const SHRINK: f64 = f32::from_bits(0x3c23_d70a) as f64;
const HALF_X: f64 = (f32::from_bits(0x409e_6455) as f64 * 0.5 - SHRINK) as f32 as f64;
const HALF_Y: f64 = (f32::from_bits(0x4087_c3b6) as f64 * 0.5 - SHRINK) as f32 as f64;
const EXTENT: f64 = (HALF_X + HALF_Y) * AXIS;
const AUTHORED_YAW: u32 = 0x40af_ede0;

#[derive(Clone, Copy)]
pub(crate) struct StartCityMask {
    x: f64,
    y: f64,
}

impl StartCityMask {
    fn at(actor: &Ent, pose: [f32; 3]) -> Option<Self> {
        if actor.def != TC || !actor.alive || actor.built < 1.0 ||
            [actor.x, actor.y, actor.rot].map(f32::to_bits) != pose.map(f32::to_bits) ||
            pose[2].to_bits() != AUTHORED_YAW {
            return None;
        }
        Some(Self {x: f64::from(actor.x), y: f64::from(actor.y)})
    }

    // A coarse cell is the union of the four source 0.5-by-0.5 probes.
    pub(crate) fn blocks(self, tx: i32, ty: i32) -> bool {
        let dx = f64::from(tx) + 0.5 - self.x;
        let dy = f64::from(ty) + 0.5 - self.y;
        dx.abs() <= EXTENT + 0.5 && dy.abs() <= EXTENT + 0.5 &&
            ((dx - dy) * AXIS).abs() <= HALF_X + AXIS &&
            ((dx + dy) * AXIS).abs() <= HALF_Y + AXIS
    }

    fn fits(self, (x, y, width, height): (i32, i32, i32, i32)) -> bool {
        f64::from(x) < self.x - EXTENT && self.x + EXTENT < f64::from(x + width) &&
            f64::from(y) < self.y - EXTENT && self.y + EXTENT < f64::from(y + height)
    }
}

impl Game {
    pub(super) fn enable_valley_start_city_masks(&mut self) -> Option<()> {
        let fixture = self.valley_fixture.as_ref()?;
        let [first, second] = fixture.authored_city_handles()?;
        let indices = [self.get(first)?, self.get(second)?];
        for index in indices {
            let actor = &self.ents[index];
            let pose = fixture.authored_city_pose(self.h(index))?;
            if !StartCityMask::at(actor, pose)?.fits(Self::building_bounds(actor)) { return None; }
        }
        // The authenticated start has just these two buildings, with disjoint footprints.
        for index in indices { self.block(index, false); }
        self.valley_fixture.as_mut()?.native_city_footprints = true;
        for index in indices { self.block(index, true); }
        Some(())
    }

    pub(crate) fn valley_start_city_mask(&self, index: usize) -> Option<StartCityMask> {
        let fixture = self.valley_fixture.as_ref()?;
        if !fixture.native_city_footprints { return None; }
        let actor = &self.ents[index];
        let pose = fixture.authored_city_pose(self.h(index))?;
        StartCityMask::at(actor, pose)
    }
}
