//! VCHotSpots. Normal solo starts admit the generated no-profile route.
//! The portable map feeds source search and terrain kernels through its own
//! corner heights, land classes and placement grid. It has no native index.

use crate::defs::{DEFS, DefId, FORTRESS, F_WORKER};
use crate::game::{Ent, Game};
use crate::hotspot_rule66::{self, FortressHit, Hotspots, Owner, Point, Roster, Seat, StandingBox};
use crate::hotspot_start66::RingCandidates;
use crate::hotspot_terrain66::{BuildingTerrainType, SourceTerrain};
use crate::map::{N, NN, T_ROCK, T_SAND, tile_of};

#[path = "district_support72.rs"]
mod district_support72;
use district_support72::SupportPlan;
pub(crate) use district_support72::ROWS as SUPPORT_ROWS;

const TAG: u32 = u32::from_le_bytes(*b"HSP1");
const MULTI_TAG: u32 = u32::from_le_bytes(*b"HSP2");
const HEADER: usize = 56;
const MULTI_HEADER: usize = 36;
const BODY: f32 = f32::from_bits(0x4087_c3b6);
const BODY_RADIUS: f32 = f32::from_bits(0x403f_ffff);
const RESERVED: f32 = f32::from_bits(0x40cb_a591);
const YAW: f32 = f32::from_bits(0xbf49_0fdb);

#[derive(Clone)]
pub(crate) struct Match {
    rule: Hotspots,
    flattened: Vec<f32>,
    score: [u32; 4],
    support: SupportPlan,
}

fn terrain_type() -> BuildingTerrainType {
    BuildingTerrainType { body_size: [BODY; 2], footprint_size: [RESERVED; 2],
        under_size: [BODY; 2], under_offset: [0.0; 2] }
}

fn basis(angle: f32) -> [f32; 2] {
    let (sin, cos) = f64::from(angle).sin_cos();
    [cos as f32, sin as f32]
}

fn portable_materials(map: &crate::map::Map) -> Vec<u16> {
    (0..=N).flat_map(|y| (0..=N).map(move |x| {
        let tile = map.ter[y.min(N - 1) * N + x.min(N - 1)];
        u16::from(tile < T_SAND || tile == T_ROCK)
    })).collect()
}

fn owner_word(owner: Owner) -> u32 {
    match owner { Owner::Unclaimed => u32::MAX, Owner::Nature => 0, Owner::Player(seat) => u32::from(seat.id()) }
}

pub(crate) fn portable_plot(xy: [f32; 2]) -> ([i32; 2], [i32; 2]) {
    let (width, height) = Game::footprint_axes(FORTRESS, 0);
    let size = [width, height];
    let corner = std::array::from_fn(|axis| (xy[axis] - size[axis] as f32 * 0.5).round() as i32);
    (corner, size)
}

pub(crate) fn fortress_body(actor: &Ent) -> StandingBox {
    let extent = BODY * 0.5 - 0.01f32;
    let [cos, sin] = basis(actor.rot);
    StandingBox { center: [actor.x, actor.y], axes: [[cos, sin], [-sin, cos]], half_extents: [extent; 2] }
}

impl Match {
    pub(crate) fn hash(&self, outcome: u8, mut mix: impl FnMut(u32)) {
        let (points, current) = self.rule.snapshot();
        if points.len() > 1 {
            self.durable_words(outcome, &mut mix);
            for point in points { mix(owner_word(point.owner)); mix(point.changes); }
            self.support.hash(&mut mix);
            return;
        }
        let point = &points[0];
        for word in [TAG, 1, current as u32, u32::from(outcome), self.flattened[0].to_bits(),
            point.position[0].to_bits(), point.position[1].to_bits(), point.position[2].to_bits(),
            owner_word(point.owner), point.changes, self.score[0], self.score[1], self.score[2], self.score[3]] {
            mix(word);
        }
    }

    pub(crate) fn wrap(&self, core: Vec<u8>, outcome: u8) -> Vec<u8> {
        let count = self.rule.snapshot().0.len();
        let header = if count == 1 { HEADER } else { MULTI_HEADER + count * 16 };
        let mut bytes = Vec::with_capacity(header + core.len() + 4);
        let mut push = |word: u32| bytes.extend_from_slice(&word.to_le_bytes());
        if count == 1 { self.hash(outcome, &mut push); }
        else { self.durable_words(outcome, &mut push); }
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        self.support.wrap(bytes)
    }

    fn durable_words(&self, outcome: u8, mut mix: impl FnMut(u32)) {
        let (points, current) = self.rule.snapshot();
        for word in [MULTI_TAG, 2, points.len() as u32, current as u32, u32::from(outcome)] { mix(word); }
        for score in self.score { mix(score); }
        for (point, height) in points.iter().zip(&self.flattened) {
            for value in point.position { mix(value.to_bits()); }
            mix(height.to_bits());
        }
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<(Match, u8)>)> {
    let (bytes, support) = SupportPlan::split(bytes)?;
    if bytes.starts_with(&MULTI_TAG.to_le_bytes()) { return split_multiple(bytes, support); }
    if !bytes.starts_with(&TAG.to_le_bytes()) { return Some((bytes, None)); }
    let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let end = bytes.len().checked_sub(4)?;
    if end <= HEADER || word(4)? != 1 || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
    let current = word(8)? as usize;
    let outcome = word(12)?;
    let flattened = f32::from_bits(word(16)?);
    let position = [f32::from_bits(word(20)?), f32::from_bits(word(24)?), f32::from_bits(word(28)?)];
    let owner = match word(32)? {
        u32::MAX => Owner::Unclaimed,
        1..=4 => Owner::Player(Seat::new(word(32)? as u8)?),
        _ => return None,
    };
    let changes = word(36)?;
    let score = [word(40)?, word(44)?, word(48)?, word(52)?];
    if outcome > 2 || current > 1 || !flattened.is_finite() || !(0.0..=128.0).contains(&flattened)
        || position.iter().take(2).any(|x| !(4.0..=(N - 4) as f32).contains(x))
        || !(0.0..=128.0).contains(&position[2])
        || score.iter().any(|&value| value != 0 && value != hotspot_rule66::FIRST_CLAIM_SCORE)
        || score.iter().sum::<u32>() != current as u32 * hotspot_rule66::FIRST_CLAIM_SCORE
        || (changes == 0 && (owner != Owner::Unclaimed || current != 0)) { return None; }
    let rule = Hotspots::restore(vec![Point { position, owner, changes }], current)?;
    Some((&bytes[HEADER..end], Some((Match { rule, flattened: vec![flattened], score, support }, outcome as u8))))
}

fn split_multiple(bytes: &[u8], support: SupportPlan) -> Option<(&[u8], Option<(Match, u8)>)> {
    let word = |at| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let count = word(8)? as usize;
    if word(4)? != 2 || !(2..=4).contains(&count) { return None; }
    if !support.valid_for(count, |_| true) { return None; }
    let header = MULTI_HEADER + count * 16;
    let end = bytes.len().checked_sub(4)?;
    if end <= header || crate::save::checksum(&bytes[..end]) != word(end)? { return None; }
    let current = word(12)? as usize;
    let outcome = word(16)?;
    let score = [word(20)?, word(24)?, word(28)?, word(32)?];
    let award = hotspot_rule66::FIRST_CLAIM_SCORE;
    if current > count || outcome > 2 || score.iter().any(|&value| value > award * count as u32 || value % award != 0)
        || score.iter().sum::<u32>() > current as u32 * award { return None; }
    let mut positions = Vec::with_capacity(count);
    let mut flattened = Vec::with_capacity(count);
    for index in 0..count {
        let at = MULTI_HEADER + index * 16;
        let values = [word(at)?, word(at + 4)?, word(at + 8)?, word(at + 12)?].map(f32::from_bits);
        if values[..2].iter().any(|x| !(4.0..=(N - 4) as f32).contains(x))
            || values[2..].iter().any(|x| !(0.0..=128.0).contains(x)) { return None; }
        positions.push([values[0], values[1], values[2]]);
        flattened.push(values[3]);
    }
    // Native save writes positions and cursor. New HotPoint allocations reset
    // observed owners/change counts before the next committed victory phase.
    let rule = Hotspots::from_native_archive(&positions, current)?;
    Some((&bytes[header..end], Some((Match { rule, flattened, score, support }, outcome as u8))))
}

impl Game {
    fn hotspot_worker_component(&self) -> Vec<bool> {
        let mut reached = vec![false; NN];
        let mut pending = Vec::new();
        for actor in &self.ents[..self.high] {
            if actor.alive && actor.hp > 0.0 && actor.team == self.local && DEFS[actor.def as usize].has(F_WORKER) {
                let tile = tile_of(actor.x, actor.y);
                reached[tile] = true;
                pending.push(tile);
            }
        }
        while let Some(tile) = pending.pop() {
            let [x, y] = [(tile % N) as i32, (tile / N) as i32];
            // An admitted diagonal has two cardinal approaches in the same
            // component. Cardinal traversal therefore preserves reachability.
            for [nx, ny] in [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]] {
                if nx < 0 || ny < 0 || nx >= N as i32 || ny >= N as i32 { continue; }
                let next = ny as usize * N + nx as usize;
                if !reached[next] && self.map.pass[0][next] {
                    reached[next] = true;
                    pending.push(next);
                }
            }
        }
        reached
    }

    fn find_hotspot(&self, origin: [f32; 2], heights: &[f32], materials: &[u16], reachable: &[bool]) -> Option<([f32; 3], f32)> {
        let [width, height] = self.map.world_dimensions();
        let mut rings = RingCandidates::new(origin, 4.5, width as u32, height as u32)?;
        let terrain = SourceTerrain::new(width as u32, height as u32, heights, materials, &[0, 1])?;
        let yaw = basis(YAW);
        while let Some(candidate) = rings.next_with(basis) {
            let Some(xy) = crate::hotspot_start66::snap_from_basis(candidate, [BODY; 2], yaw) else { continue; };
            let [x, y] = xy;
            if !(4.0..=(width - 4) as f32).contains(&x) || !(4.0..=(height - 4) as f32).contains(&y) { continue; }
            let ([tx, ty], [fw, fh]) = portable_plot(xy);
            if self.placement_geometry_error(self.local, FORTRESS, tx, ty, 0) != 0 { continue; }
            // The portable generator can put the nearest clear site on an
            // island. Use the actual player's work-access component.
            let accessible = (ty - 1..=ty + fh).any(|y| {
                (tx - 1..=tx + fw).any(|x| {
                    let tile = y as usize * N + x as usize;
                    (x < tx || x >= tx + fw || y < ty || y >= ty + fh)
                        && reachable[tile] && self.map.pass[0][tile]
                })
            });
            if !accessible { continue; }
            let Some(probe) = terrain.probe(xy, terrain_type(), [yaw[0], -yaw[1]], 0.0) else { continue; };
            if probe.rejection.is_some() { continue; }
            let (raster, flattened) = terrain.flatten_writes(xy, [BODY; 2], yaw, 0.0)?;
            if raster.contains_outside_corner || raster.corners.is_empty() { continue; }
            let z = heights[y as usize * (N + 1) + x as usize];
            return Some(([x, y, z], flattened));
        }
        None
    }

    fn apply_hotspot_ground(heights: &mut [f32], xy: [f32; 2], height: f32) -> Option<()> {
        let size = [BODY + f32::from_bits(0x3fb5_04f3) + f32::from_bits(0x3a83_126f); 2];
        let quad = crate::hotspot_start66::quad_from_basis(xy, size, basis(YAW))?;
        let raster = crate::hotspot_start66::raster_quad(quad, N as u32, N as u32, true)?;
        if raster.contains_outside_corner || raster.corners.is_empty() { return None; }
        for [x, y] in raster.corners { heights[y as usize * (N + 1) + x as usize] = height; }
        Some(())
    }

    fn discover_hotspot(&mut self, xy: [f32; 2]) {
        let [cx, cy] = xy.map(|value| value.floor() as i32);
        let radius = i32::from(hotspot_rule66::DISCOVERY_RADIUS);
        for y in cy - radius..=cy + radius {
            for x in cx - radius..=cx + radius {
                if (x - cx).pow(2) + (y - cy).pow(2) <= radius * radius {
                    for seat in 0..self.nplayers {
                        let fog = &mut self.team_vis[seat * NN + y as usize * N + x as usize];
                        *fog = (*fog).max(128);
                    }
                }
            }
        }
        self.refresh_display_fog();
    }

    pub(crate) fn configure_match_hotspots(&mut self, civs: u32, pop: u32, supplies: u32, start: u32, end: u32, count: u32) -> bool {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return false; }
        if self.tick != 0 || self.net || self.bench != 0 || self.humans != 1 || self.local != 0
            || self.hotspots.is_some() || self.map.world_dimensions() != [N, N] { return false; }
        let Some(origins) = crate::hotspot_start66::generated_origins(N, N, self.seed0, count) else { return false; };
        let mut heights = self.map.hm.clone();
        let materials = portable_materials(&self.map);
        let reachable = self.hotspot_worker_component();
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(origins.len());
        let mut flattened = Vec::with_capacity(origins.len());
        for origin in origins {
            let Some((xyz, flat)) = self.find_hotspot(origin, &heights, &materials, &reachable) else { return false; };
            // Portable sites must admit distinct ordinary Fortress plots.
            // Reject ambiguous placement instead of moving a source point.
            let (corner, size) = portable_plot([xyz[0], xyz[1]]);
            if positions.iter().any(|&[x, y, _]| {
                let (other, _) = portable_plot([x, y]);
                (0..2).all(|axis| (corner[axis] - other[axis]).abs() < size[axis])
            }) { return false; }
            if Self::apply_hotspot_ground(&mut heights, [xyz[0], xyz[1]], flat).is_none() { return false; }
            positions.push(xyz);
            flattened.push(flat);
        }
        let Some(rule) = Hotspots::new(&positions) else { return false; };
        if positions.iter().any(|&[x, y, _]| {
            let (corner, size) = portable_plot([x, y]);
            let center = std::array::from_fn(|axis| corner[axis] as f32 + size[axis] as f32 * 0.5);
            !rule.allows_placement(center, true, BODY_RADIUS, true)
        }) { return false; }
        if !self.configure_match_epoch_range(civs, pop, supplies, start, end) { return false; }
        self.map.hm = heights;
        self.map.dirty = true;
        self.discover_hotspot([positions[0][0], positions[0][1]]);
        self.hotspots = Some(Match { rule, flattened, score: [0; 4], support: SupportPlan::default() });
        true
    }

    pub(crate) fn hotspot_allows_placement(&self, team: u8, def: DefId, xy: [f32; 2]) -> bool {
        self.hotspots.as_ref().is_none_or(|mode| {
            let radius = if def == FORTRESS { BODY_RADIUS }
            else if let Some([x, y]) = crate::wonders61::shape(def, false) { x.hypot(y) * 0.5 }
            else { DEFS[def as usize].size * core::f32::consts::FRAC_1_SQRT_2 };
            mode.rule.allows_placement(xy, def == FORTRESS, radius, self.players[team as usize].ai)
        })
    }

    pub(crate) fn poll_hotspots(&mut self) {
        let Some(mut mode) = self.hotspots.take() else { return; };
        let mut hits: [Vec<FortressHit>; 4] = std::array::from_fn(|_| Vec::new());
        let (points, _) = mode.rule.snapshot();
        let count = points.len();
        for actor in &self.ents[..self.high] {
            if !actor.alive || actor.def != FORTRESS || actor.built < 1.0 { continue; }
            let Some(seat) = actor.team.checked_add(1).and_then(Seat::new) else { continue; };
            let body = fortress_body(actor);
            for (point, matches) in points.iter().zip(&mut hits) {
                let [x, y, _] = point.position;
                if hotspot_rule66::standing_box_intersects([x, y], body) == Some(true) {
                    matches.push(FortressHit { owner: Owner::Player(seat), hp: actor.hp });
                }
            }
        }
        let roster = Roster::new(std::array::from_fn(|seat| seat > 0 && seat <= self.nplayers && self.players[seat - 1].alive),
            std::array::from_fn(|seat| seat as u8)).expect("unique fixed teams fit");
        let observed = hits.map(|row| hotspot_rule66::fortress_owner(&row));
        if let Some(result) = mode.rule.poll(&observed[..count], &roster) {
            if let Some(seat) = result.award { mode.score[usize::from(seat.id() - 1)] += hotspot_rule66::FIRST_CLAIM_SCORE; }
            if let Some(next) = result.reveal {
                let [x, y, _] = mode.rule.snapshot().0[next].position;
                self.discover_hotspot([x, y]);
            }
            if let Some(winner) = result.winning_team { self.winner = if winner == self.local + 1 { 1 } else { 2 }; }
        }
        self.hotspots = Some(mode);
    }

    pub(crate) fn restore_hotspots(&mut self, mode: Match, outcome: u8) -> Option<()> {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return None; }
        let (points, _) = mode.rule.snapshot();
        if self.net || self.bench != 0 || self.humans != 1 || self.local != 0
            || mode.score[self.nplayers..].iter().any(|&score| score != 0)
            || !mode.support.valid_for(points.len(), |team| team < self.nplayers && self.players[team].ai)
            || points.iter().any(|point| matches!(point.owner, Owner::Player(seat) if usize::from(seat.id()) > self.nplayers)) { return None; }
        for (point, &height) in points.iter().zip(&mode.flattened) {
            let [x, y, _] = point.position;
            Self::apply_hotspot_ground(&mut self.map.hm, [x, y], height)?;
        }
        self.map.dirty = true;
        if points.len() > 1 {
            for point in mode.rule.visible_points() {
                let [x, y, _] = point.position;
                self.discover_hotspot([x, y]);
            }
        }
        self.hotspots = Some(mode);
        self.winner = outcome;
        Some(())
    }

    pub(crate) fn hotspot_status(&self, field: u32, seat: usize) -> i32 {
        let Some(mode) = &self.hotspots else { return if field == 0 { 0 } else { -1 }; };
        match field {
            0 => mode.rule.snapshot().0.len() as i32,
            1 => mode.rule.revealed() as i32,
            2 if seat < self.nplayers => mode.rule.personal_count(Seat::new(seat as u8 + 1).unwrap()) as i32,
            3 if seat < self.nplayers => mode.score[seat] as i32,
            _ => -1,
        }
    }

    pub(crate) fn hotspot_point(&self, index: usize, axis: usize) -> f32 {
        self.hotspots.as_ref().and_then(|mode| mode.rule.visible_points().get(index))
            .and_then(|point| point.position.get(axis)).copied().unwrap_or(f32::NAN)
    }

    pub(crate) fn ai_hotspot_points(&self) -> Option<(&[Point], usize)> {
        let state = &self.hotspots.as_ref()?.rule;
        Some((state.visible_points(), state.snapshot().0.len()))
    }

    pub(crate) fn hotspot_support_demand(&mut self, index: usize, team: u8, row: usize) -> u8 {
        let previous = self.sim_previous_elapsed();
        let plan = &mut self.hotspots.as_mut().expect("active HotSpots plan").support;
        plan.count(index, usize::from(team), row, previous, || self.source_birth.rng.next_word15().into())
    }

    pub(crate) fn hotspot_support_placement(&mut self, index: usize, team: u8, row: usize, found: bool) {
        let plan = &mut self.hotspots.as_mut().expect("active HotSpots plan").support;
        plan.placement(index, usize::from(team), row, found);
    }

    pub(crate) fn finish_hotspot_support(&mut self, team: u8) {
        let seat = usize::from(team);
        if self.net || self.winner != 0 || self.players[seat].buildings == 0 { return; }
        let previous = self.sim_previous_elapsed();
        let Some(mode) = &mut self.hotspots else { return; };
        // The projected manager starts when this adapter first sees a held
        // multi-point district and remains active once armed.
        if mode.rule.snapshot().0.len() <= 1 || !mode.support.armed(seat)
            && !mode.rule.visible_points().iter().any(|point| matches!(point.owner, Owner::Player(owner) if owner.id() == team + 1)) { return; }
        mode.support.finish_pass(seat, previous, || self.source_birth.rng.next_word15().into());
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    use crate::defs::TOWER;

    #[test]
    fn blocked_site_saves_failed_row_and_paid_retry_clears_it() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        assert!(game.configure_match_hotspots(0xee10, 500, 2, 0, 0, 3));
        while game.hotspot_status(2, 1) == 0 && game.tick < 20_000 { game.tick(); }
        assert_eq!(game.hotspot_status(2, 1), 1);

        // Direct the ordinary search to exhaust its candidates, then restore
        // the normal map before writing the archive.
        let pass = game.map.pass[0].clone();
        game.map.pass[0].fill(false);
        let wood = game.resource(1, 1);
        let stone = game.resource(1, 3);
        game.ai_step(1);
        game.map.pass[0] = pass;
        assert_eq!([game.resource(1, 1), game.resource(1, 3)], [wood, stone]);
        game.save();
        let failed = game.savebuf.clone();
        assert!(failed.starts_with(b"HSD3"));
        let failure_count = |bytes: &[u8]| u32::from_le_bytes(bytes[76..80].try_into().unwrap());
        assert_eq!(failure_count(&failed), 1);

        let mut restored = Game::load(&failed).unwrap();
        restored.save();
        assert_eq!(restored.savebuf, failed);
        restored.tick(); // The rule refreshes its transient observed owners.
        restored.set_resource(1, 1, 0.0);
        restored.ai_step(1);
        restored.save();
        assert_eq!(failure_count(&restored.savebuf), 1, "a funds rejection does not add a placement miss");
        restored.set_resource(1, 1, wood);
        let pending = |game: &Game| game.ents[..game.high].iter().any(|actor| {
            actor.alive && actor.team == 1 && actor.def == TOWER && actor.built < 1.0
        });
        let mut paid = None;
        for _ in 0..480 {
            let before = restored.resource(1, 1);
            restored.tick();
            if pending(&restored) {
                paid = Some(before - restored.resource(1, 1));
                break;
            }
        }
        assert_eq!(paid, Some(f32::from(restored.unit_cost(1, TOWER)[1])));
        restored.save();
        assert!(restored.savebuf.starts_with(b"HSD2"));
    }
}
