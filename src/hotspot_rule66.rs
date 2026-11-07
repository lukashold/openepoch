//! HotSpots mode 8 rule kernel. The world adapter provides accepted source
//! positions and native-ordered spatial hits for the exact completed Fortress
//! type. No world generation, native save format or engine clock is implied.

pub const FIRST_CLAIM_SCORE: u32 = 100;
pub const FORTRESS_QUERY_RADIUS: f32 = 0.1;
pub const DISCOVERY_RADIUS: u8 = 2;
pub const MARKER_SCALE: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Unclaimed,
    Nature,
    Player(Seat),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat(u8);

impl Seat {
    pub fn new(id: u8) -> Option<Self> {
        (1..16).contains(&id).then_some(Self(id))
    }

    pub fn id(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FortressHit {
    pub owner: Owner,
    pub hp: f32,
}

/// Native BoxBound1 fields supplied by the standing actor's collision record.
#[derive(Clone, Copy, Debug)]
pub struct StandingBox {
    pub center: [f32; 2],
    pub axes: [[f32; 2]; 2],
    pub half_extents: [f32; 2],
}

/// 006D91D0, reverse dispatch from shape5 against BoxBound1. Both projected
/// XY contacts are inclusive. There is no Z comparison or rounded corner.
pub fn standing_box_intersects(xy: [f32; 2], body: StandingBox) -> Option<bool> {
    if xy.iter().chain(&body.center).chain(body.axes.iter().flatten()).any(|v| !v.is_finite()) ||
        body.half_extents.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return None;
    }
    let delta = std::array::from_fn::<_, 2, _>(|axis| {
        add_chop(f64::from(body.center[axis]), -f64::from(xy[axis]))
    });
    Some(body.axes.iter().zip(body.half_extents).all(|(&[x, y], half)| {
        let radius = add_chop(f64::from(FORTRESS_QUERY_RADIUS), f64::from(half));
        let projection = add_chop(mul_chop(delta[1], f64::from(y)), mul_chop(delta[0], f64::from(x)));
        projection.abs() <= radius
    }))
}

/// 008650F0 first removes nonpositive/unordered HP by swapping in the last
/// result. The first survivor then answers 005B0D30. The provider has already
/// filtered exact Fortress type and reproduced spatial traversal order.
pub fn fortress_owner(hits: &[FortressHit]) -> Owner {
    hits.first()
        .filter(|hit| hit.hp > 0.0)
        .or_else(|| hits.iter().rev().find(|hit| hit.hp > 0.0))
        .map_or(Owner::Unclaimed, |hit| hit.owner)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub position: [f32; 3],
    pub owner: Owner,
    pub changes: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hotspots {
    points: Vec<Point>,
    current: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Poll {
    pub award: Option<Seat>,
    pub reveal: Option<usize>,
    pub winning_team: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Roster {
    pub has_score_record: [bool; 16],
    fixed_teams: [u8; 16],
}

impl Roster {
    pub fn new(has_score_record: [bool; 16], fixed_teams: [u8; 16]) -> Option<Self> {
        fixed_teams.iter().all(|&team| team < 16).then_some(Self {
            has_score_record,
            fixed_teams,
        })
    }
}

impl Hotspots {
    /// 005B0810 supplies one candidate origin per point to 0082AF60. A search
    /// accepts and flattens in place, so later searches see earlier changes.
    /// An unsuccessful search leaves this point's fresh default unchanged.
    pub fn from_start(
        count: usize,
        generated: &[[f32; 2]],
        authored: &[[f32; 2]],
        failed_search_xyz: [f32; 3],
        mut accept_and_flatten: impl FnMut([f32; 2]) -> Option<[f32; 3]>,
    ) -> Option<Self> {
        if !(1..=20).contains(&count) || failed_search_xyz.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let origins = if generated.len() == count { generated } else { authored.get(..count)? };
        if origins.iter().flatten().any(|v| !v.is_finite()) { return None; }
        let positions: Vec<_> = origins.iter().map(|&xy| {
            accept_and_flatten(xy).unwrap_or(failed_search_xyz)
        }).collect();
        Self::new(&positions)
    }

    pub fn new(positions: &[[f32; 3]]) -> Option<Self> {
        Self::from_native_archive(positions, 0)
    }

    /// 005B0990 loads the saved point locations and current cursor. Individual
    /// HotPoint allocations start with Unclaimed / zero and are polled again.
    pub fn from_native_archive(positions: &[[f32; 3]], current: usize) -> Option<Self> {
        let points = positions.iter().map(|&position| Point {
            position,
            owner: Owner::Unclaimed,
            changes: 0,
        });
        Self::restore(points.collect(), current)
    }

    /// Durable kernel state. Map startup and the adapter's archive envelope
    /// own the positions and this snapshot respectively.
    pub fn restore(points: Vec<Point>, current: usize) -> Option<Self> {
        if !(1..=20).contains(&points.len()) || current > points.len() ||
            points.iter().flat_map(|point| point.position).any(|v| !v.is_finite()) {
            return None;
        }
        Some(Self { points, current })
    }

    pub fn snapshot(&self) -> (&[Point], usize) {
        (&self.points, self.current)
    }

    pub fn revealed(&self) -> usize {
        (self.current + 1).min(self.points.len())
    }

    pub fn visible_points(&self) -> &[Point] {
        &self.points[..self.revealed()]
    }

    pub fn personal_count(&self, seat: Seat) -> usize {
        self.points.iter().filter(|point| point.owner == Owner::Player(seat)).count()
    }

    /// 005AF770. Apply every observation before advancing at most one point.
    pub fn poll(&mut self, observations: &[Owner], roster: &Roster) -> Option<Poll> {
        if observations.len() != self.points.len() { return None; }
        for (point, &owner) in self.points.iter_mut().zip(observations) {
            if point.owner != owner {
                point.owner = owner;
                point.changes = point.changes.wrapping_add(1);
            }
        }

        let mut result = Poll::default();
        if let Some(Point { owner: Owner::Player(seat), changes: 1, .. }) = self.points.get(self.current) {
            result.award = roster.has_score_record[usize::from(seat.id())].then_some(*seat);
            self.current += 1;
            result.reveal = (self.current < self.points.len()).then_some(self.current);
        }

        if self.current == self.points.len() {
            let mut counts = [0u8; 16];
            for point in &self.points {
                match point.owner {
                    Owner::Unclaimed => return Some(result),
                    Owner::Nature => (),
                    Owner::Player(seat) => counts[usize::from(roster.fixed_teams[usize::from(seat.id())])] += 1,
                }
            }
            let maximum = counts.iter().copied().max().unwrap_or(0);
            if maximum != 0 {
                let mut leading = counts.iter().enumerate().filter(|&(_, &count)| count == maximum);
                let (team, _) = leading.next().expect("a positive maximum has a team");
                result.winning_team = leading.next().is_none().then_some(team as u8);
            }
        }
        Some(result)
    }

    /// 005B0B50. Call from both ordinary placement preview and authoritative
    /// admission. `body_radius` is computed from native body extents, with its
    /// f32 store. It is distinct from the reserved-box startup search radius.
    pub fn allows_placement(&self, xy: [f32; 2], fortress: bool, body_radius: f32, native_ai: bool) -> bool {
        if xy.iter().any(|v| !v.is_finite()) || !body_radius.is_finite() || body_radius < 0.0 {
            return false;
        }
        let points = if native_ai { &self.points } else { self.visible_points() };
        points.iter().all(|point| {
            let dx = add_chop(f64::from(xy[0]), -f64::from(point.position[0])).abs();
            let dy = add_chop(f64::from(xy[1]), -f64::from(point.position[1])).abs();
            let near = dx.min(dy);
            let small = mul_chop(near, f64::from(0.3f32));
            let distance = add_chop(dx.max(dy), small);
            add_chop(distance, -f64::from(body_radius)) >= 3.5 || (fortress && distance <= 2.0)
        })
    }
}

pub(crate) fn add_chop(left: f64, right: f64) -> f64 {
    let nearest = left + right;
    let virtual_right = nearest - left;
    let residual = (left - (nearest - virtual_right)) + (right - virtual_right);
    chop_residual(nearest, residual)
}

pub(crate) fn mul_chop(left: f64, right: f64) -> f64 {
    let nearest = left * right;
    chop_residual(nearest, left.mul_add(right, -nearest))
}

fn chop_residual(nearest: f64, residual: f64) -> f64 {
    if (nearest > 0.0 && residual < 0.0) || (nearest < 0.0 && residual > 0.0) {
        f64::from_bits(nearest.to_bits() - 1)
    } else { nearest }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Owner { Owner::Player(Seat::new(id).unwrap()) }

    fn roster() -> Roster {
        let mut teams = std::array::from_fn(|id| id as u8);
        teams[3] = 1;
        let mut present = [true; 16];
        present[3] = false;
        Roster::new(present, teams).unwrap()
    }

    #[test]
    fn filters_dead_by_swap_before_taking_first() {
        let a = FortressHit { owner: p(1), hp: 1200.0 };
        let b = FortressHit { owner: p(2), hp: 70.0 };
        let dead = FortressHit { owner: p(3), hp: 0.0 };
        let nan = FortressHit { owner: p(3), hp: f32::NAN };
        assert_eq!(fortress_owner(&[dead, a, b, nan]), p(2));
        assert_eq!(fortress_owner(&[a, dead, b, nan]), p(1));
        assert_eq!(fortress_owner(&[dead, nan]), Owner::Unclaimed);
    }

    #[test]
    fn ordered_reveal_tie_loss_and_same_team_reclaim() {
        let roster = roster();
        let mut world = Hotspots::new(&[[10.0, 10.0, 0.0], [20.0, 20.0, 0.0]]).unwrap();
        assert_eq!(world.revealed(), 1);
        assert_eq!(world.poll(&[Owner::Unclaimed, p(2)], &roster), Some(Poll::default()));
        let a = world.poll(&[p(1), p(2)], &roster).unwrap();
        assert_eq!(a, Poll { award: Seat::new(1), reveal: Some(1), winning_team: None });
        let b = world.poll(&[p(1), p(2)], &roster).unwrap();
        assert_eq!(b, Poll { award: Seat::new(2), reveal: None, winning_team: None });
        assert_eq!(world.poll(&[Owner::Unclaimed, p(2)], &roster).unwrap().winning_team, None);
        let (points, current) = world.snapshot();
        let mut restored = Hotspots::restore(points.to_vec(), current).unwrap();
        assert_eq!(world, restored);
        let c = restored.poll(&[p(3), p(1)], &roster).unwrap();
        assert_eq!(c, Poll { winning_team: Some(1), ..Poll::default() });
        assert_eq!(restored.personal_count(Seat::new(1).unwrap()), 1);
    }

    #[test]
    fn placement_edges_and_hidden_ai() {
        let world = Hotspots::new(&[[0.0, 0.0, 0.0], [20.0, 20.0, 0.0]]).unwrap();
        assert!(world.allows_placement([3.5, 0.0], false, 0.0, false));
        assert!(!world.allows_placement([3.5f32.next_down(), 0.0], false, 0.0, false));
        assert!(world.allows_placement([2.0, 0.0], true, 2.0, false));
        assert!(!world.allows_placement([2.0f32.next_up(), 0.0], true, 2.0, false));
        assert!(!world.allows_placement([3.2, 1.0], false, 0.001, false));
        assert!(world.allows_placement([20.0, 20.0], false, 1.0, false));
        assert!(!world.allows_placement([20.0, 20.0], false, 1.0, true));
        assert!(!world.allows_placement([f32::NAN, 20.0], true, 1.0, false));
    }

    #[test]
    fn nature_no_award_and_missing_score_record_still_advances() {
        let roster = roster();
        let mut world = Hotspots::new(&[[0.0, 0.0, 0.0]]).unwrap();
        assert!(world.poll(&[], &roster).is_none());
        assert_eq!(world.poll(&[Owner::Nature], &roster), Some(Poll::default()));
        assert_eq!(world.poll(&[p(1)], &roster), Some(Poll::default()));
        let mut world = Hotspots::new(&[[0.0, 0.0, 0.0]]).unwrap();
        assert_eq!(world.poll(&[p(3)], &roster), Some(Poll { winning_team: Some(1), ..Poll::default() }));
    }

    #[test]
    fn startup_uses_exact_count_and_a_fresh_default_per_origin() {
        let authored = [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];
        let generated = [[20.0, 30.0], [40.0, 50.0]];
        let mut visits = Vec::new();
        let search = |xy: [f32; 2]| {
            visits.push(xy);
            (visits.len() == 1).then_some([xy[0] + 0.5, xy[1] + 0.5, 9.0])
        };
        let world = Hotspots::from_start(2, &generated, &authored, [0.0; 3], search).unwrap();
        assert_eq!(visits, generated);
        assert_eq!(world.snapshot().0[0].position, [20.5, 30.5, 9.0]);
        assert_eq!(world.snapshot().0[1].position, [0.0; 3]);
        let world = Hotspots::from_start(2, &generated[..1], &authored, [0.0; 3], |[x, y]| {
            Some([x, y, 0.0])
        }).unwrap();
        assert_eq!(world.snapshot().0[1].position, [3.0, 4.0, 0.0]);
        assert!(Hotspots::from_start(4, &generated, &authored, [0.0; 3], |_| panic!()).is_none());
    }

    #[test]
    fn native_archive_retains_reveal_cursor_and_rebuilds_owner_cache() {
        let positions = [[10.0, 10.0, 0.0], [20.0, 20.0, 0.0]];
        let mut restored = Hotspots::from_native_archive(&positions, 1).unwrap();
        assert_eq!(restored.revealed(), 2);
        assert_eq!(restored.personal_count(Seat::new(1).unwrap()), 0);
        let poll = restored.poll(&[p(1), p(1)], &roster()).unwrap();
        assert_eq!(poll, Poll { award: Seat::new(1), reveal: None, winning_team: Some(1) });
        assert_eq!(restored.personal_count(Seat::new(1).unwrap()), 2);
        assert!(Hotspots::from_native_archive(&positions, 3).is_none());
    }

    #[test]
    fn standing_fortress_box_has_inclusive_projected_edges() {
        let mut body = StandingBox { center: [0.0; 2], axes: [[1.0, 0.0], [0.0, 1.0]], half_extents: [1.0; 2] };
        assert_eq!(standing_box_intersects([1.09, 1.09], body), Some(true));
        assert_eq!(standing_box_intersects([1.1f32.next_down(), 0.0], body), Some(true));
        assert_eq!(standing_box_intersects([1.1, 0.0], body), Some(false));
        body.half_extents = [0.0; 2];
        assert_eq!(standing_box_intersects([FORTRESS_QUERY_RADIUS, 0.0], body), Some(true));
        assert_eq!(standing_box_intersects([FORTRESS_QUERY_RADIUS.next_up(), 0.0], body), Some(false));
        body.axes[0][0] = f32::NAN;
        assert_eq!(standing_box_intersects([0.0; 2], body), None);
    }
}
