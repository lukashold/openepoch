//! The ordinary generated HotSpots AI. Source task admission is fed by the
//! portable land cohort and visible combat state. Orders use the existing AI.

use super::*;
use crate::hotspot_rule66::{add_chop, mul_chop, standing_box_intersects, Owner};
use crate::hotspots66::{fortress_body, portable_plot};

const MIN_ARMY: usize = 3;
const APPROACH_RADIUS: f32 = 8.0;

struct Objective {
    xy: [f32; 2],
    owner: Owner,
    pending: bool,
    target: Option<usize>,
    priority: f64,
    home_distance: f32,
}

fn distance_sq(xy: [f32; 2], actor: &Ent) -> f32 {
    (xy[0] - actor.x).powi(2) + (xy[1] - actor.y).powi(2)
}

fn store_chop(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() { f32::from_bits(nearest.to_bits() - 1) } else { nearest }
}

impl Game {
    pub(super) fn ai_contest_hotspots(&mut self, team: u8, army: &[usize], workers: &[usize], home: [f32; 2]) -> bool {
        let Some((visible, total)) = self.ai_hotspot_points() else { return false; };
        if self.net || !self.players[team as usize].ai || !self.construction_available(team, FORTRESS)
            || self.initial_ceasefire_active() { return false; }

        let own = visible.iter().filter(|point| matches!(point.owner, Owner::Player(seat) if seat.id() == team + 1)).count();
        let others = visible.iter().filter(|point| matches!(point.owner, Owner::Player(seat) if seat.id() != team + 1)).count();
        let contest = own + 1 < others && own + others + 1 >= total;
        let cohort: Vec<_> = army.iter().copied().filter(|&index| {
            let actor = &self.ents[index];
            let def = &DEFS[actor.def as usize];
            actor.alive && actor.hp > 0.0 && actor.team == team && def.dom == LAND && def.armed()
                && (matches!(actor.order, O_IDLE | O_MOVE | O_AMOVE) || actor.order == O_ATTACK && self.get(actor.otgt).is_some_and(|target| {
                    let fortress = &self.ents[target];
                    fortress.def == FORTRESS && self.combat_hostile(team, fortress.team) && visible.iter().any(|point| {
                        let [x, y, _] = point.position;
                        standing_box_intersects([x, y], fortress_body(fortress)) == Some(true)
                    })
                }))
        }).collect();
        if cohort.len() < MIN_ARMY { return false; }
        let available: f64 = cohort.iter().map(|&index| {
            let actor = &self.ents[index];
            f64::from(self.hill_combat_strength(team, actor.def, actor.hp))
        }).sum();
        let civilian = self.hill_combat_strength(team, WORKER, self.max_hp(team, WORKER));

        let mut best = None;
        for point in visible {
            let [x, y, _] = point.position;
            let xy = [x, y];
            let defending = self.ents[..self.high].iter().any(|actor| {
                actor.alive && actor.hp > 0.0 && actor.team == team && actor.def == FORTRESS
                    && actor.built < 1.0 && standing_box_intersects(xy, fortress_body(actor)) == Some(true)
            });
            if defending {
                // An already paid objective keeps the sole executable slot.
                best = Some(Objective { xy, owner: point.owner, pending: true, target: None, priority: 0.0, home_distance: 0.0 });
                break;
            }
            match point.owner {
                Owner::Player(seat) if seat.id() == team + 1 || !self.combat_hostile(team, seat.id() - 1) => continue,
                Owner::Unclaimed | Owner::Nature if contest => continue,
                _ => {}
            }
            // 004BE510 searches the marker's 0.1 collider for visible foreign
            // Fortresses, including a construction site requesting that type.
            // Portable unfinished actors already carry the intended definition.
            let target = self.ents[..self.high].iter().enumerate().find_map(|(index, actor)| {
                (actor.alive && actor.hp > 0.0 && actor.def == FORTRESS && self.combat_hostile(team, actor.team)
                    && standing_box_intersects(xy, fortress_body(actor)) == Some(true) && self.can_see(team, index)).then_some(index)
            });
            // Strategic strength admits a new task. The active Battalion's
            // separate 004BE510 target refresh does not reapply that test.
            let engaged = target.is_some_and(|target| {
                let handle = self.h(target);
                let key = self.key_for(LAND, x, y);
                cohort.iter().any(|&index| {
                    let actor = &self.ents[index];
                    (actor.order == O_AMOVE && actor.fkey == key) || (actor.order == O_ATTACK && actor.otgt == handle)
                })
            });
            let local_strength: f64 = self.ents[..self.high].iter().enumerate().filter(|&(index, actor)| {
                actor.alive && actor.hp > 0.0 && self.combat_hostile(team, actor.team)
                    && DEFS[actor.def as usize].armed() && distance_sq(xy, actor) <= APPROACH_RADIUS.powi(2)
                    && self.can_see(team, index)
            }).map(|(_, actor)| f64::from(self.hill_combat_strength(actor.team, actor.def, actor.hp))).sum();
            let threat = store_chop(local_strength.max(mul_chop(f64::from(civilian), 5.0)));
            if !engaged && f64::from(threat) * 0.5 > available { continue; }
            // AIP kind 7 base and the exact owner -1 bonus. A portable home
            // distance tie-break stands in for the native strategic raster.
            let priority = if point.owner == Owner::Unclaimed { add_chop(30.0, 6.0) } else { 30.0 };
            let home_distance = (x - home[0]).powi(2) + (y - home[1]).powi(2);
            if best.as_ref().is_none_or(|old| {
                priority > old.priority || priority == old.priority && home_distance < old.home_distance
            }) {
                best = Some(Objective { xy, owner: point.owner, pending: false, target, priority, home_distance });
            }
        }
        let Some(Objective { xy: [x, y], owner, pending, target, .. }) = best else { return false; };
        if let Some(target) = target {
            let handle = self.h(target);
            let fresh: Vec<_> = cohort.iter().copied().filter(|&index| {
                let actor = &self.ents[index];
                actor.order != O_ATTACK || actor.otgt != handle
            }).collect();
            if !fresh.is_empty() { self.order_attack(&fresh, target); }
            return true;
        }
        let xy = [x, y];
        let key = self.key_for(LAND, x, y);
        let mut nearby = 0;
        let approaching: Vec<_> = cohort.iter().copied().filter(|&index| {
            let actor = &self.ents[index];
            if distance_sq(xy, actor) <= APPROACH_RADIUS.powi(2) {
                nearby += 1;
                false
            } else {
                actor.order != O_AMOVE || actor.fkey != key
            }
        }).collect();
        self.order_move(&approaching, x, y, true);
        if pending || nearby < MIN_ARMY || owner != Owner::Unclaimed { return true; }

        // HOTSPOT uses the known portable plot for the visible marker. The
        // common construction path validates reachability, placement and pays.
        let ([tx, ty], _) = portable_plot(xy);
        let crew: Vec<_> = workers.iter().copied().filter(|&index| {
            let actor = &self.ents[index];
            actor.alive && actor.hp > 0.0 && matches!(actor.order, O_IDLE | O_GATHER)
        }).take(3).collect();
        if !crew.is_empty() { self.begin_construction(team, FORTRESS, tx, ty, 0, &crew, false); }
        true
    }
}
