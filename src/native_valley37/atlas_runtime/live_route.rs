//! Read-only native route plan for an ordinary live authored Valley Citizen Move.

use super::{
    active_game,
    atlas_ray::{self, FootCost, Leaf},
    atlas_reader::{quantized_floor, Precision53},
    edge_metrics::{self, Profile},
    fine_search::{self, Candidate},
    ordered_neighbors,
    route_smoothing::{self, Waypoint},
    AtlasView,
};
use crate::{
    defs::WORKER,
    game::{Game, BOARDED, O_IDLE, O_MOVE},
};

const MODE: Precision53 = Precision53::NearestEven;
const COST: FootCost = FootCost {
    clearance: f32::from_bits(0x3db504f3),
    density_weight: 1.5,
};
const MAX_EXPANSIONS: usize = 1024;
const UNAVAILABLE: u32 = u32::MAX;

type Position = [f32; 2];
type NodeId = [u16; 3];

pub(super) struct Trace {
    tick: u32,
    actor: u32,
    generation: u32,
    raw: Vec<Waypoint>,
    retained: Vec<usize>,
}

fn half(point: Position) -> [i32; 2] {
    point.map(|value| quantized_floor(f64::from(value) * 2.0, MODE))
}

fn identity(leaf: Leaf) -> NodeId {
    [
        leaf.origin_half[0],
        leaf.origin_half[1],
        u16::from(leaf.level),
    ]
}

fn leaf_at(view: &AtlasView<'_>, point: [i32; 2]) -> Option<Leaf> {
    let leaf = view.leaf(point)?;
    Some(Leaf {
        origin_half: leaf.origin_half,
        level: leaf.level,
        class: leaf.class,
        density: leaf.density,
    })
}

fn route(
    view: &AtlasView<'_>,
    owner: i32,
    origin: Position,
    endpoint: Position,
) -> Option<(Vec<Waypoint>, Vec<usize>)> {
    if !origin
        .into_iter()
        .chain(endpoint)
        .all(|axis| (0.0..=256.0).contains(&axis))
    {
        return None;
    }
    let origin_cell = half(origin);
    let endpoint_cell = half(endpoint);
    let start_leaf = leaf_at(view, origin_cell)?;
    let goal_leaf = leaf_at(view, endpoint_cell)?;
    if start_leaf.class != 0
        || goal_leaf.class != 0
        || view.admit(owner, 2, origin_cell, endpoint_cell) != 2
    {
        return None;
    }
    let start = identity(start_leaf);
    let goal = identity(goal_leaf);
    if start == goal {
        return None;
    }
    let profile = Profile {
        clearance: COST.clearance,
        density_weight: COST.density_weight,
        heuristic_divisor: MODE.store_f32(f64::from(f32::from_bits(0x3f999999)) * 1.5),
        cross_weight: Some(f32::from_bits(0x3b83126f)),
    };
    let position = |node: NodeId| {
        if node == start {
            return origin;
        }
        if node == goal {
            return endpoint;
        }
        let offset = f32::from(1_u16 << node[2]) * 0.25;
        [node[0], node[1]].map(|value| f32::from(value) * 0.5 + offset)
    };
    let admits = |leaf: Leaf| match leaf.class {
        0 => Some(view.admit(owner, 2, origin_cell, leaf.origin_half.map(i32::from)) == 2),
        0x28 => Some(false),
        _ => None,
    };
    let guess = |from| edge_metrics::heuristic(MODE, origin, from, endpoint, profile);
    let mut expansions = 0;
    let path = fine_search::find(
        MODE,
        start,
        goal,
        guess(origin),
        |id| {
            expansions += 1;
            if expansions > MAX_EXPANSIONS {
                return None;
            }
            let leaf = leaf_at(view, [i32::from(id[0]), i32::from(id[1])])?;
            if identity(leaf) != id {
                return None;
            }
            let from = position(id);
            let mut edges = Vec::new();
            for next in ordered_neighbors::edges(leaf, |point| leaf_at(view, point))? {
                if admits(next)? {
                    let node = identity(next);
                    let to = position(node);
                    edges.push(Candidate {
                        node,
                        edge_cost: edge_metrics::ordinary_edge_cost(
                            MODE,
                            from,
                            to,
                            leaf.density,
                            profile,
                        ),
                        heuristic: guess(to),
                    });
                }
            }
            Some(edges)
        },
        |_| (),
    )?;
    let raw: Vec<_> = path
        .into_iter()
        .map(|(node, cumulative_cost)| Waypoint {
            position: position(node),
            tag: 0,
            cumulative_cost,
        })
        .collect();
    let retained = route_smoothing::select(&raw, |from, to| {
        let ray = atlas_ray::make_ray(MODE, from.position, to.position);
        atlas_ray::test_segment(
            MODE,
            ray,
            COST,
            |point| leaf_at(view, half(point)),
            |_, next| admits(next),
            |_| (),
        )
    })?;
    Some((raw, retained))
}

impl Game {
    pub(crate) fn valley_route_probe(&self, ids: &[usize]) -> Option<usize> {
        let atlas = self.valley_atlas_view()?;
        *atlas.runtime.last_route.borrow_mut() = None;
        let [index] = ids else {
            return None;
        };
        let actor = &self.ents[*index];
        // The independent witness covers fresh routes without treaty costs or
        // prior route replacement. Keep those cases on the existing path.
        (actor.def == WORKER
            && actor.alive
            && actor.hp > 0.0
            && actor.order == O_IDLE
            && actor.team == self.local
            && actor.stuck != BOARDED
            && !self.initial_ceasefire_active()
            && self
                .diplomacy
                .borders
                .as_ref()
                .is_none_or(|rules| rules.term(0, 1) == 0))
        .then_some(*index)
    }

    pub(crate) fn valley_observe_ordinary_route(&mut self, index: usize, endpoint: Position) {
        let actor = &self.ents[index];
        if actor.order != O_MOVE || [actor.ox, actor.oy] != endpoint {
            return;
        }
        let Some(view) = self.valley_atlas_view() else {
            return;
        };
        let Some((raw, retained)) = route(
            &view,
            i32::from(actor.team) + 1,
            [actor.x, actor.y],
            endpoint,
        ) else {
            return;
        };
        let trace = Trace {
            tick: self.tick,
            actor: self.h(index),
            generation: view.runtime.generation,
            raw,
            retained,
        };
        #[cfg(feature = "ee2-valley197-waypoint-motion")]
        let progress = (self.valley_routes.enabled
            && (3..=super::route_motion::MAX_POINTS).contains(&trace.retained.len()))
        .then(|| super::route_motion::RouteProgress {
            actor: trace.actor,
            issued_tick: trace.tick,
            final_goal: endpoint,
            waypoints: trace.retained.iter().map(|&i| trace.raw[i].position).collect(),
            next: 1,
            atlas_generation: trace.generation,
            #[cfg(feature = "ee2-valley204-ground-motion")]
            native: super::ground_motion::NativeMotion::start(actor),
        });
        *view.runtime.last_route.borrow_mut() = Some(trace);
        #[cfg(feature = "ee2-valley197-waypoint-motion")]
        if let Some(progress) = progress {
            self.install_valley_route(progress);
        }
    }
}

fn with_trace(result: impl FnOnce(&Trace) -> Option<u32>) -> Option<u32> {
    let view = active_game()?.valley_atlas_view()?;
    let trace = view.runtime.last_route.borrow();
    let trace = trace
        .as_ref()
        .filter(|trace| trace.generation == view.runtime.generation)?;
    result(trace)
}

#[cfg(feature = "ee2-valley198-waypoint-save")]
impl Game {
    pub(in crate::native_valley37) fn valley_matches_saved_waypoints(
        &self,
        route_progress: &super::route_motion::RouteProgress,
    ) -> bool {
        let Some(index) = self.get(route_progress.actor) else { return false; };
        let Some(view) = self.valley_atlas_view() else { return false; };
        let Some(origin) = route_progress.waypoints.first().copied() else { return false; };
        if view.runtime.generation != route_progress.atlas_generation || self.initial_ceasefire_active()
            || self.diplomacy.borders.as_ref().is_some_and(|rules| rules.term(0, 1) != 0) { return false; }
        let Some((raw, retained)) = route(&view, i32::from(self.ents[index].team) + 1, origin, route_progress.final_goal)
            else { return false; };
        retained.into_iter().map(|i| raw[i].position.map(f32::to_bits))
            .eq(route_progress.waypoints.iter().map(|point| point.map(f32::to_bits)))
    }
}

#[no_mangle]
pub extern "C" fn valley196_route_info(field: u32) -> u32 {
    with_trace(|trace| match field {
        0 => Some(1),
        1 => Some(trace.tick),
        2 => Some(trace.actor),
        3 => Some(trace.generation),
        4 => Some(trace.raw.len() as u32),
        5 => Some(trace.retained.len() as u32),
        _ => None,
    })
    .unwrap_or(if field == 0 { 0 } else { UNAVAILABLE })
}

#[no_mangle]
pub extern "C" fn valley196_route_waypoint(smoothed: u32, index: usize, field: u32) -> u32 {
    with_trace(|trace| {
        let index = match smoothed {
            0 => index,
            1 => *trace.retained.get(index)?,
            _ => return None,
        };
        let waypoint = trace.raw.get(index)?;
        match field {
            0 | 1 => Some(waypoint.position[field as usize].to_bits()),
            2 => Some(waypoint.cumulative_cost.to_bits()),
            _ => None,
        }
    })
    .unwrap_or(UNAVAILABLE)
}
