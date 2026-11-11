//! Initial mobile placement for the authenticated Death Valley2 opening.
//!
//! Coordinates use native XY and height Z. The caller owns world authentication,
//! actual city admission and the two explicit shared RNG checkpoints.

const WIDTH: usize = 100;
const HEIGHT: usize = 300;
const STRIDE: usize = WIDTH + 1;
const FOREST_STRIDE: usize = WIDTH / 32 + 1;
const CORNERS: usize = STRIDE * (HEIGHT + 1);
const MAP_CENTER: [f32; 2] = [50.0, 150.0];
const MAX_PROBES: usize = 2000;
const YAW: f32 = f32::from_bits(0xbf490fdb);
const SQRT_TWO: f32 = f32::from_bits(0x3fb504f3);
const CITIZEN_RADIUS: f32 = 0.125;
const SCOUT_RADIUS: f32 = 0.4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rounding {
    Nearest,
    Chop,
}

impl Rounding {
    fn store(self, value: f64) -> f32 {
        let rounded = value as f32;
        if self == Self::Chop && f64::from(rounded).abs() > value.abs() {
            f32::from_bits(rounded.to_bits() - 1)
        } else {
            rounded
        }
    }

    fn cell(self, value: f64) -> i32 {
        const BIAS: f64 = 103079215104.0;
        const ULP: f64 = 1.0 / 65536.0;
        let sum = BIAS + value;
        let biased = if self == Self::Chop && sum - BIAS > value {
            sum - ULP
        } else {
            sum
        };
        (biased.to_bits() >> 16) as i32
    }

    fn density_index(self, value: f64) -> i32 {
        match self {
            Self::Nearest => value.round_ties_even() as i32,
            Self::Chop => value.trunc() as i32,
        }
    }

    fn city_radius(self) -> f32 {
        f32::from_bits(match self {
            Self::Nearest => 0x40708862,
            Self::Chop => 0x40708861,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct City {
    pub xyz: [f32; 3],
    pub yaw: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StationaryKind {
    Mineral,
    CityCenter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stationary {
    pub xy: [f32; 2],
    pub yaw: f32,
    pub kind: StationaryKind,
}

/// A live positive-mass vertical capsule from the source movement root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moving {
    pub xyz: [f32; 3],
    pub radius: f32,
}

pub struct World<'a> {
    pub heights: &'a [f32],
    pub movement_by_corner: &'a [u8],
    pub cliff_patterns: &'a [u32],
    pub forest_100: &'a [u32],
    pub stationary: &'a [Stationary],
    pub moving: &'a [Moving],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gender {
    Female,
    Male,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementError {
    InvalidWorld,
    InvalidCity,
    InvalidActor,
    UnverifiedTerrain,
    NoSiteInVerifiedSearch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainCheck {
    pub xy: [f32; 2],
    pub movement: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    pub xyz: [f32; 3],
    pub context: TerrainCheck,
    pub moving_blocked: Option<bool>,
    pub final_terrain: Option<TerrainCheck>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub xyz: [f32; 3],
    pub yaw: f32,
    pub rng_after: u32,
    pub proposal_xy: [f32; 2],
    pub probes: Vec<Probe>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Citizen {
    pub gender: Gender,
    pub rng_after_gender: u32,
    pub placed: Placed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub citizen: Citizen,
    pub scout: Placed,
}

pub fn place_citizen(
    world: &World<'_>,
    city: City,
    rng_before_gender: u32,
    rounding: Rounding,
) -> Result<Citizen, PlacementError> {
    let (rng_after_gender, word) = advance(rng_before_gender);
    let gender = if word & 1 == 0 {
        Gender::Female
    } else {
        Gender::Male
    };
    let placed = search(
        world,
        city,
        rng_after_gender,
        rounding,
        CITIZEN_RADIUS,
        None,
    )?;
    Ok(Citizen {
        gender,
        rng_after_gender,
        placed,
    })
}

pub fn place_scout(
    world: &World<'_>,
    city: City,
    citizen: &Citizen,
    rng_before_scout: u32,
    rounding: Rounding,
) -> Result<Placed, PlacementError> {
    let previous = Moving {
        xyz: citizen.placed.xyz,
        radius: CITIZEN_RADIUS,
    };
    search(
        world,
        city,
        rng_before_scout,
        rounding,
        SCOUT_RADIUS,
        Some(previous),
    )
}

pub fn place_pair(
    world: &World<'_>,
    city: City,
    rng_before_gender: u32,
    rng_before_scout: u32,
    rounding: Rounding,
) -> Result<Pair, PlacementError> {
    let citizen = place_citizen(world, city, rng_before_gender, rounding)?;
    let scout = place_scout(world, city, &citizen, rng_before_scout, rounding)?;
    Ok(Pair { citizen, scout })
}

fn advance(state: u32) -> (u32, u32) {
    let next = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (next, (next >> 16) & 0x7fff)
}

fn rotate([x, y]: [f32; 2], angle: f64, rounding: Rounding) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    let [s, c] = [sin, cos].map(|value| f64::from(rounding.store(value)));
    let [x, y] = [x, y].map(f64::from);
    [x * c - y * s, x * s + y * c].map(|value| rounding.store(value))
}

fn proposal(city: City, state: u32, rounding: Rounding) -> (u32, [f32; 2]) {
    let delta = std::array::from_fn::<_, 2, _>(|i| {
        f64::from(rounding.store(f64::from(MAP_CENTER[i]) - f64::from(city.xyz[i])))
    });
    let length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
    let direction =
        delta.map(|value| rounding.store(if length > 0.0 { value / length } else { value }));
    let (next, word) = advance(state);
    let half_sweep = f64::from(f32::from_bits(0x3fc90fdb));
    let multiplier = f64::from(f32::from_bits(0x38000100));
    let angle = (2.0 * half_sweep) * f64::from(word) * multiplier - half_sweep;
    let rotated = rotate(direction, angle, rounding);
    let radius = f64::from(rounding.city_radius());
    let xy = std::array::from_fn(|i| {
        rounding.store(f64::from(rotated[i]) * radius + f64::from(city.xyz[i]))
    });
    (next, xy)
}

fn xy_valid([x, y]: [f32; 2]) -> bool {
    (0.0..WIDTH as f32).contains(&x) && (0.0..HEIGHT as f32).contains(&y)
}

fn xy([x, y, _]: [f32; 3]) -> [f32; 2] {
    [x, y]
}

fn z_valid(value: f32) -> bool {
    (-32.0..=32.0).contains(&value)
}

impl World<'_> {
    fn validate(&self, city: City, previous: Option<Moving>) -> Result<(), PlacementError> {
        if self.heights.len() != CORNERS
            || self.movement_by_corner.len() != CORNERS
            || self.cliff_patterns.len() != WIDTH * HEIGHT
            || self.forest_100.len() != FOREST_STRIDE * HEIGHT
        {
            return Err(PlacementError::InvalidWorld);
        }
        if !xy_valid(xy(city.xyz)) || !z_valid(city.xyz[2]) || city.yaw.to_bits() != YAW.to_bits() {
            return Err(PlacementError::InvalidCity);
        }
        if self
            .stationary
            .iter()
            .any(|s| !xy_valid(s.xy) || !s.yaw.is_finite())
            || self.moving.iter().chain(previous.iter()).any(|s| {
                !xy_valid(xy(s.xyz)) || !z_valid(s.xyz[2]) || !(0.01..=8.0).contains(&s.radius)
            })
        {
            return Err(PlacementError::InvalidActor);
        }
        Ok(())
    }

    fn forest(&self, [x, y]: [i32; 2]) -> bool {
        if x < 0 || y < 0 || x as usize >= WIDTH || y as usize >= HEIGHT {
            return false;
        }
        let (x, y) = (x as usize, y as usize);
        self.forest_100[y * FOREST_STRIDE + x / 32] & (0x80000000 >> (x % 32)) != 0
    }

    fn flat_height(&self, x: i32, y: i32) -> Result<f32, PlacementError> {
        if x < 1 || y < 1 || x >= WIDTH as i32 - 1 || y >= HEIGHT as i32 - 1 {
            return Err(PlacementError::UnverifiedTerrain);
        }
        let (x, y) = (x as usize, y as usize);
        if self.cliff_patterns[y * WIDTH + x] != 0 {
            return Err(PlacementError::UnverifiedTerrain);
        }
        let at = y * STRIDE + x;
        let word = self.heights[at];
        if !word.is_finite()
            || word <= 0.0
            || [at + 1, at + STRIDE, at + STRIDE + 1]
                .into_iter()
                .any(|i| self.heights[i].to_bits() != word.to_bits())
        {
            return Err(PlacementError::UnverifiedTerrain);
        }
        Ok(word)
    }

    fn terrain(
        &self,
        point: [f32; 2],
        rounding: Rounding,
        shapes: &[BoxShape],
    ) -> Result<TerrainCheck, PlacementError> {
        let [hx, hy] = point.map(|v| rounding.cell(f64::from(v) * 2.0));
        let (x, y) = (hx / 2, hy / 2);
        self.flat_height(x, y)?;
        let corner = ((y + hy % 2) as usize) * STRIDE + (x + hx % 2) as usize;
        let center = [f64::from(hx) * 0.5 + 0.25, f64::from(hy) * 0.5 + 0.25];
        // 004A9B00's first forest test is necessary for the unported density branch.
        if self.movement_by_corner[corner] != 0
            || self.forest(center.map(|v| rounding.density_index(v)))
        {
            return Err(PlacementError::UnverifiedTerrain);
        }
        let movement = shapes
            .iter()
            .find(|shape| shape.overlaps_cell(center))
            .map_or(0, |shape| shape.block_class);
        Ok(TerrainCheck {
            xy: point,
            movement,
        })
    }
}

pub(super) struct BoxShape {
    xy: [f64; 2],
    axes: [[f64; 2]; 2],
    half: [f64; 2],
    block_class: u8,
}

impl BoxShape {
    #[cfg(feature = "ee2-rect267-builder-reach")]
    pub(super) fn city(xy: [f32; 2], yaw: f32) -> Self {
        Self::new(Stationary { xy, yaw, kind: StationaryKind::CityCenter })
    }

    fn new(actor: Stationary) -> Self {
        let (size, block_class) = match actor.kind {
            StationaryKind::Mineral => ([0.3f32; 2], 0x27),
            StationaryKind::CityCenter => ([3.5, 3.0], 0x28),
        };
        // Mobile238 constructs the source mode-one cache in nearest53 before searching.
        let half =
            size.map(|v| f64::from((f64::from(v * SQRT_TWO) * 0.5 - f64::from(0.01f32)) as f32));
        let (sin, cos) = f64::from(actor.yaw).sin_cos();
        let [s, c] = [sin, cos].map(|v| f64::from(v as f32));
        Self {
            xy: actor.xy.map(f64::from),
            axes: [[c, s], [-s, c]],
            half,
            block_class,
        }
    }

    fn overlaps_cell(&self, center: [f64; 2]) -> bool {
        const CELL_HALF: f64 = 0.25;
        let [x, y] = center;
        let delta = [x - self.xy[0], y - self.xy[1]];
        for i in 0..2 {
            let projected =
                self.axes[0][i].abs() * self.half[0] + self.axes[1][i].abs() * self.half[1];
            if delta[i].abs() > CELL_HALF + projected {
                return false;
            }
        }
        for i in 0..2 {
            let [a, b] = self.axes[i];
            if (delta[0] * a + delta[1] * b).abs() > self.half[i] + (a.abs() + b.abs()) * CELL_HALF
            {
                return false;
            }
        }
        true
    }

    #[cfg(feature = "ee2-rect267-builder-reach")]
    pub(super) fn blocks_half_cell(&self, cell: [i32; 2]) -> bool {
        self.overlaps_cell(cell.map(|axis| f64::from(axis) * 0.5 + 0.25))
    }
}

fn moving_blocks(point: [f32; 2], radius: f32, obstacle: Moving) -> bool {
    // Dispatch slots CF883C/CF881C select 006D9190 for tags 5 and 4.
    // The two XY differences and both squares stay in the x87 registers.
    let delta =
        std::array::from_fn::<_, 2, _>(|i| f64::from(point[i]) - f64::from(obstacle.xyz[i]));
    let sum = f64::from(radius) + f64::from(obstacle.radius);
    delta[0] * delta[0] + delta[1] * delta[1] < sum * sum
}

fn search(
    world: &World<'_>,
    city: City,
    state: u32,
    rounding: Rounding,
    radius: f32,
    previous: Option<Moving>,
) -> Result<Placed, PlacementError> {
    world.validate(city, previous)?;
    let shapes: Vec<_> = world
        .stationary
        .iter()
        .copied()
        .chain(std::iter::once(Stationary {
            xy: xy(city.xyz),
            yaw: city.yaw,
            kind: StationaryKind::CityCenter,
        }))
        .map(BoxShape::new)
        .collect();
    let (rng_after, proposal_xy) = proposal(city, state, rounding);
    let ring_limit =
        1 + rounding.cell(((WIDTH * WIDTH + HEIGHT * HEIGHT) as f64).sqrt() / f64::from(radius));
    let query_radius = rounding.store(f64::from(radius) * 0.8);
    let mut context_xy = [0.0f32; 2];
    let mut probes = Vec::new();
    for ring in 0..ring_limit {
        let step = if ring == 0 {
            687.0
        } else {
            rounding.store(1.0 / f64::from(ring))
        };
        let distance = rounding.store(f64::from(radius) * f64::from(ring));
        let mut angle = 0.0;
        while angle < f32::from_bits(0x40c90fdb) {
            if probes.len() == MAX_PROBES {
                return Err(PlacementError::NoSiteInVerifiedSearch);
            }
            let offset = rotate([distance, 0.0], f64::from(angle), rounding);
            let point = std::array::from_fn(|i| {
                rounding.store(f64::from(proposal_xy[i]) + f64::from(offset[i]))
            });
            angle = rounding.store(f64::from(angle) + f64::from(step));
            let [x, y] = point.map(|v| rounding.cell(f64::from(v)));
            let z = world.flat_height(x, y)?;
            let delta =
                std::array::from_fn::<_, 2, _>(|i| f64::from(point[i]) - f64::from(context_xy[i]));
            // 0085FE30 preserves the context coordinates below a squared distance of .5.
            if delta[0] * delta[0] + delta[1] * delta[1] >= 0.5 {
                context_xy = point;
            }
            let context = world.terrain(context_xy, rounding, &shapes)?;
            let mut probe = Probe {
                xyz: [point[0], point[1], z],
                context,
                moving_blocked: None,
                final_terrain: None,
            };
            if context.movement == 0
                && !world.forest(context_xy.map(|v| rounding.cell(f64::from(v))))
            {
                let blocked = world
                    .moving
                    .iter()
                    .chain(previous.iter())
                    .any(|&actor| moving_blocks(point, query_radius, actor));
                probe.moving_blocked = Some(blocked);
                if !blocked {
                    let final_terrain = world.terrain(point, rounding, &shapes)?;
                    probe.final_terrain = Some(final_terrain);
                    if final_terrain.movement == 0 {
                        let xyz = probe.xyz;
                        probes.push(probe);
                        return Ok(Placed {
                            xyz,
                            yaw: YAW,
                            rng_after,
                            proposal_xy,
                            probes,
                        });
                    }
                }
            }
            probes.push(probe);
        }
    }
    Err(PlacementError::NoSiteInVerifiedSearch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tangent_capsule_pair_is_clear() {
        let actor = Moving {
            xyz: [20.0, 20.0, 10.0],
            radius: 0.125,
        };
        let tangent = 20.25f32;
        let inside = f32::from_bits(tangent.to_bits() - 1);
        let outside = f32::from_bits(tangent.to_bits() + 1);
        assert!(moving_blocks([inside, 20.0], 0.125, actor));
        assert!(!moving_blocks([tangent, 20.0], 0.125, actor));
        assert!(!moving_blocks([outside, 20.0], 0.125, actor));
    }
}
