//! VCHotSpots. Normal solo starts presently admit one generated point.
//! The portable map feeds source search and terrain kernels through its own
//! corner heights, land classes and placement grid. It has no native index.

use crate::defs::{DEFS, DefId, FORTRESS, F_WORKER};
use crate::game::Game;
use crate::hotspot_rule66::{self, FortressHit, Hotspots, Owner, Point, Roster, Seat, StandingBox};
use crate::hotspot_start66::RingCandidates;
use crate::hotspot_terrain66::{BuildingTerrainType, SourceTerrain};
use crate::map::{N, NN, T_ROCK, T_SAND, tile_of};

const TAG: u32 = u32::from_le_bytes(*b"HSP1");
const HEADER: usize = 56;
const BODY: f32 = f32::from_bits(0x4087_c3b6);
const BODY_RADIUS: f32 = f32::from_bits(0x403f_ffff);
const RESERVED: f32 = f32::from_bits(0x40cb_a591);
const YAW: f32 = f32::from_bits(0xbf49_0fdb);

#[derive(Clone)]
pub(crate) struct Match {
    rule: Hotspots,
    flattened: f32,
    score: [u32; 4],
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

impl Match {
    pub(crate) fn hash(&self, outcome: u8, mut mix: impl FnMut(u32)) {
        let (points, current) = self.rule.snapshot();
        let point = &points[0];
        for word in [TAG, 1, current as u32, u32::from(outcome), self.flattened.to_bits(),
            point.position[0].to_bits(), point.position[1].to_bits(), point.position[2].to_bits(),
            owner_word(point.owner), point.changes, self.score[0], self.score[1], self.score[2], self.score[3]] {
            mix(word);
        }
    }

    pub(crate) fn wrap(&self, core: Vec<u8>, outcome: u8) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER + core.len() + 4);
        self.hash(outcome, |word| bytes.extend_from_slice(&word.to_le_bytes()));
        bytes.extend_from_slice(&core);
        bytes.extend_from_slice(&crate::save::checksum(&bytes).to_le_bytes());
        bytes
    }
}

pub(crate) fn split(bytes: &[u8]) -> Option<(&[u8], Option<(Match, u8)>)> {
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
    Some((&bytes[HEADER..end], Some((Match { rule, flattened, score }, outcome as u8))))
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

    fn find_hotspot(&self) -> Option<([f32; 3], f32)> {
        let [width, height] = self.map.world_dimensions();
        if [width, height] != [N, N] { return None; }
        // 00706B55 -> 00706BAF, first mode-8 row, no-island/no-profile branch.
        let origin = [width as f32 * 0.5, height as f32 * 0.5];
        let mut rings = RingCandidates::new(origin, 4.5, width as u32, height as u32)?;
        let materials = portable_materials(&self.map);
        let terrain = SourceTerrain::new(width as u32, height as u32, &self.map.hm, &materials, &[0, 1])?;
        let yaw = basis(YAW);
        let (fw, fh) = Self::footprint_axes(FORTRESS, 0);
        let reachable = self.hotspot_worker_component();
        while let Some(candidate) = rings.next_with(basis) {
            let Some(xy) = crate::hotspot_start66::snap_from_basis(candidate, [BODY; 2], yaw) else { continue; };
            let [x, y] = xy;
            if !(4.0..=(width - 4) as f32).contains(&x) || !(4.0..=(height - 4) as f32).contains(&y) { continue; }
            let tx = (x - fw as f32 * 0.5).round() as i32;
            let ty = (y - fh as f32 * 0.5).round() as i32;
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
            let z = self.map.hm[y as usize * (N + 1) + x as usize];
            return Some(([x, y, z], flattened));
        }
        None
    }

    fn apply_hotspot_ground(&mut self, xy: [f32; 2], height: f32) -> Option<()> {
        let size = [BODY + f32::from_bits(0x3fb5_04f3) + f32::from_bits(0x3a83_126f); 2];
        let quad = crate::hotspot_start66::quad_from_basis(xy, size, basis(YAW))?;
        let raster = crate::hotspot_start66::raster_quad(quad, N as u32, N as u32, true)?;
        if raster.contains_outside_corner || raster.corners.is_empty() { return None; }
        for [x, y] in raster.corners { self.map.hm[y as usize * (N + 1) + x as usize] = height; }
        self.map.dirty = true;
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
            || count != 1 || self.hotspots.is_some() { return false; }
        let Some((xyz, flattened)) = self.find_hotspot() else { return false; };
        let Some(rule) = Hotspots::new(&[xyz]) else { return false; };
        if !self.configure_match_epoch_range(civs, pop, supplies, start, end) { return false; }
        self.apply_hotspot_ground([xyz[0], xyz[1]], flattened).expect("search validated the same raster");
        self.discover_hotspot([xyz[0], xyz[1]]);
        self.hotspots = Some(Match { rule, flattened, score: [0; 4] });
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
        let mut hits = Vec::new();
        let [x, y, _] = mode.rule.snapshot().0[0].position;
        let extent = (BODY * 0.5) - 0.01f32;
        for actor in &self.ents[..self.high] {
            if !actor.alive || actor.def != FORTRESS || actor.built < 1.0 { continue; }
            let Some(seat) = actor.team.checked_add(1).and_then(Seat::new) else { continue; };
            let [cos, sin] = basis(actor.rot);
            let body = StandingBox { center: [actor.x, actor.y], axes: [[cos, sin], [-sin, cos]], half_extents: [extent; 2] };
            if hotspot_rule66::standing_box_intersects([x, y], body) == Some(true) {
                hits.push(FortressHit { owner: Owner::Player(seat), hp: actor.hp });
            }
        }
        let roster = Roster::new(std::array::from_fn(|seat| seat > 0 && seat <= self.nplayers && self.players[seat - 1].alive),
            std::array::from_fn(|seat| seat as u8)).expect("unique fixed teams fit");
        if let Some(result) = mode.rule.poll(&[hotspot_rule66::fortress_owner(&hits)], &roster) {
            if let Some(seat) = result.award { mode.score[usize::from(seat.id() - 1)] += hotspot_rule66::FIRST_CLAIM_SCORE; }
            if let Some(winner) = result.winning_team { self.winner = if winner == self.local + 1 { 1 } else { 2 }; }
        }
        self.hotspots = Some(mode);
    }

    pub(crate) fn restore_hotspots(&mut self, mode: Match, outcome: u8) -> Option<()> {
        #[cfg(feature = "ee2-authored-valley37")]
        if self.valley_fixture.is_some() { return None; }
        let point = mode.rule.snapshot().0[0];
        if self.net || self.bench != 0 || self.humans != 1 || self.local != 0
            || mode.score[self.nplayers..].iter().any(|&score| score != 0)
            || matches!(point.owner, Owner::Player(seat) if usize::from(seat.id()) > self.nplayers) { return None; }
        self.apply_hotspot_ground([point.position[0], point.position[1]], mode.flattened)?;
        self.hotspots = Some(mode);
        self.winner = outcome;
        Some(())
    }

    pub(crate) fn hotspot_status(&self, field: u32, seat: usize) -> i32 {
        let Some(mode) = &self.hotspots else { return if field == 0 { 0 } else { -1 }; };
        match field {
            0 => 1,
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
}
