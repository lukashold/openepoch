//! Opt-in authored tree actors and the single-player citizen wood order.
use super::{active_game, forest_registry::ForestId, forest_replay163::{ForestReplay, WOOD}};
use crate::{
    cmd::C_VALLEY_FOREST_GATHER,
    defs::{Def, LAND, WORKER},
    game::{Ent, Game, NONE, O_VALLEY_FOREST},
    map::R_WOOD,
};

const WORKER_LIMIT: u32 = 3;
const APPROACH_OFFSET: f32 = (f32::from_bits(1061625612) as f64 * 0.9) as f32;
const CUT_REACH: f32 = 0.9;

pub(super) fn actor_id(row: u32) -> u32 {
    // A private handle namespace outside the normal 8192 entity slots.
    0x4000_8000 | row
}

fn approaches(x: f32, y: f32) -> [(f32, f32); 4] {
    [(x + APPROACH_OFFSET, y), (x - APPROACH_OFFSET, y),
     (x, y + APPROACH_OFFSET), (x, y - APPROACH_OFFSET)]
}

fn remove_wood(remaining: &mut f32, amount: f32) -> bool {
    // 007EF044 stores a float but compares the unrounded subtraction to 1.
    let left = f64::from(*remaining) - f64::from(amount);
    *remaining = if left < 1.0 { 0.0 } else { left as f32 };
    *remaining > 0.0
}

fn carry_wood(carry: f32, amount: f32, worker: f32, player: f32, cap: f32) -> (f32, bool) {
    // 007EE27E spills the worker factor to f32 before the remaining x87 operations.
    let updated = f64::from(worker) * f64::from(player) * f64::from(amount) + f64::from(carry);
    (updated.min(f64::from(cap)) as f32, updated >= f64::from(cap))
}

#[cfg(any(feature = "ee2-valley169-forest-retarget", feature = "ee2-rect277-gather-resume"))]
pub(super) fn individual_score(center: (f32, f32), tree: (f32, f32), spare: i32) -> f32 {
    use crate::ai9_count::Precision53::TowardZero;
    let dx = TowardZero.add(f64::from(center.0), -f64::from(tree.0));
    let dy = TowardZero.add(f64::from(center.1), -f64::from(tree.1));
    let distance = TowardZero.store_f32(TowardZero.add(
        TowardZero.mul(dx, dx), TowardZero.mul(dy, dy),
    ));
    if spare > 0 { return distance; }
    TowardZero.store_f32(TowardZero.add(f64::from(distance), f64::from(200 - 200 * spare)))
}

fn assign_forest_order(e: &mut Ent, row: u32, tile: u32,
    (goal_x, goal_y): (f32, f32), key: u32) {
    let carried = if e.carry > 0.0 { e.q[2] } else { 0 };
    e.order = O_VALLEY_FOREST;
    e.ox = goal_x;
    e.oy = goal_y;
    e.otile = tile;
    e.otgt = NONE;
    e.tgt = NONE;
    e.fkey = key;
    e.fslot = u16::MAX;
    e.stuck = 0;
    e.q = [u32::from(R_WOOD), u32::from(carried != 0 && carried != u32::from(R_WOOD)), carried, row, 0];
}

impl Game {
    pub(in crate::native_valley37) fn valley_forest_play(&self) -> Option<&ForestReplay> {
        #[cfg(feature = "ee2-rect235-forest")]
        if let Some(forest) = &self.rect_forest { return Some(&forest.replay); }
        self.valley_fixture.as_ref()?.forest_replay.as_ref().filter(|forest| forest.playable)
    }

    fn forest_play_mut(&mut self) -> Option<&mut ForestReplay> {
        #[cfg(feature = "ee2-rect235-forest")]
        if let Some(forest) = &mut self.rect_forest { return Some(&mut forest.replay); }
        self.valley_fixture.as_mut()?.forest_replay.as_mut()
    }

    fn promote_forest_row(&mut self, row: u32) -> Option<()> {
        #[cfg(feature = "ee2-rect287-oil-economy")]
        if self.rect_native_ids.is_some() && self.rect_native_cursor()? >= super::animal_live250::MAX_NATIVE_LOCAL_ID {
            return None;
        }
        #[cfg(feature = "ee2-rect235-forest")]
        if let Some(forest) = &mut self.rect_forest {
            forest.promote(row)?;
            #[cfg(feature = "ee2-rect287-oil-economy")]
            if self.rect_native_ids.is_some() { self.note_rect_owned_allocation(); }
            return Some(());
        }
        self.forest_play_mut()?.promote(ForestId(row), actor_id(row), WOOD)
    }

    fn forest_position(&self, row: u32) -> Option<(f32, f32)> {
        #[cfg(feature = "ee2-rect235-forest")]
        if let Some(forest) = &self.rect_forest { return forest.gather_position(row); }
        let tree = self.map.source_world.as_deref()?.forest.as_ref()?.source_rows.get(row as usize)?;
        if tree.flags & 0x60 != 0x20 { return None }
        let [x, y, _] = tree.position();
        Some((x, y))
    }

    fn forest_gather_team(&self, team: u8) -> bool {
        #[cfg(feature = "ee2-rect239-fresh")]
        if self.rect_forest.is_some() && !self.net && (team as usize) < self.nplayers
            && self.players[team as usize].ai
        {
            return true;
        }
        team == self.local
    }

    fn forest_actor_can_order(&self, team: u8, row: u32) -> Option<(f32, f32)> {
        let forest = self.valley_forest_play()?;
        if self.net || !self.forest_gather_team(team) || self.solo_resigned || self.winner != 0 ||
            forest.promoted.get(row as usize)?.is_some_and(|actor| actor.wood == 0.0) { return None }
        let (x, y) = self.forest_position(row)?;
        (self.real_fog(team, self.map.tile_of(x, y)) == 255).then_some((x, y))
    }

    fn forest_staff(&self, row: u32) -> (u32, u32) {
        self.ents[..self.high].iter().filter(|e| e.alive && e.def == WORKER && self.forest_gather_team(e.team)
            && e.order == O_VALLEY_FOREST && e.q[3] == row)
            .fold((0, 0), |(reserved, active), e| (reserved + 1, active + e.q[4]))
    }

    // Bounded portable admission. The source uses the movement-group graph;
    // until its Valley grid is available we admit only unobstructed approaches.
    fn forest_approach_clear(&self, e: &Ent, (x, y): (f32, f32)) -> bool {
        let [width, height] = self.map.world_dimensions();
        if !(0.6..width as f32 - 0.6).contains(&x) || !(0.6..height as f32 - 0.6).contains(&y) { return false }
        let denied = self.denied_border_territories(e);
        let distance = ((e.x - x).powi(2) + (e.y - y).powi(2)).sqrt();
        let steps = (distance * 8.0).ceil().max(1.0) as u32;
        (0..=steps).all(|step| {
            let t = step as f32 / steps as f32;
            let tile = self.map.tile_of(e.x + (x - e.x) * t, e.y + (y - e.y) * t);
            self.map.pass[0][tile] && self.map.permits_territory(tile, denied)
        })
    }

    fn forest_approach(&self, e: &Ent, x: f32, y: f32) -> Option<(f32, f32)> {
        let distance = |(x, y): (f32, f32)| (e.x - x).powi(2) + (e.y - y).powi(2);
        approaches(x, y).into_iter().filter(|&point| self.forest_approach_clear(e, point))
            .min_by(|&a, &b| distance(a).total_cmp(&distance(b)))
    }

    #[cfg(feature = "ee2-rect239-fresh")]
    pub(crate) fn nearest_rect_ai_forest(&self, worker: usize, reach_squared: f32) -> Option<(u32, f32)> {
        let e = &self.ents[worker];
        if !self.citizen_can_gather(e.team, R_WOOD) { return None; }
        #[cfg(feature = "ee2-rect291-ai-scout")]
        let component = self.map.land_component(self.map.tile_of(e.x, e.y), self.denied_border_territories(e));
        self.rect_forest.as_ref()?.gather_rows().filter_map(|(row, (x, y))| {
            let distance = (x - e.x).powi(2) + (y - e.y).powi(2);
            if distance > reach_squared || self.forest_actor_can_order(e.team, row).is_none()
                || self.forest_staff(row).0 >= WORKER_LIMIT || self.forest_approach(e, x, y).is_none()
            {
                return None;
            }
            #[cfg(feature = "ee2-rect291-ai-scout")]
            {
                let goal = self.forest_approach(e, x, y)?;
                if component[self.map.tile_of(goal.0, goal.1)] == 255 { return None; }
            }
            Some((row, distance))
        }).min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
    }

    #[cfg(feature = "ee2-valley169-forest-retarget")]
    fn continuation_tree(&self, e: &Ent, center: (f32, f32)) -> Option<u32> {
        let forest = self.valley_forest_play()?;
        let individual = forest.promoted.iter().enumerate().filter_map(|(index, actor)| {
            if !actor.is_some_and(|actor| actor.wood > 0.0) { return None; }
            let row = index as u32;
            let (x, y) = self.forest_position(row)?;
            if individual_score(center, (x, y), 1) > 25.0 || self.forest_approach(e, x, y).is_none() {
                return None;
            }
            let spare = WORKER_LIMIT as i32 - self.forest_staff(row).0 as i32;
            Some((row, individual_score(center, (x, y), spare), spare))
        }).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((row, score, _)) = individual {
            if score < 1.0 { return Some(row); }
        }
        // The registry keeps original row identity and source probe order.
        // Native region labels and compressed bucket traversal are separate.
        let compressed = forest.registry.select((0..forest.promoted.len() as u32).map(ForestId),
            [center.0, center.1], 5.0, f32::from_bits(1061625612),
            |[x, y]| self.forest_approach_clear(e, (x, y)));
        compressed.map(|id| id.0).or_else(|| individual.filter(|row| row.2 > 0).map(|row| row.0))
    }

    #[cfg(feature = "ee2-valley169-forest-retarget")]
    fn continue_depleted_forest(&mut self, e: &mut Ent, row: u32) -> bool {
        #[cfg(feature = "ee2-rect235-forest")]
        if self.rect_forest.is_some() { return false; }
        let center = self.forest_position(row).unwrap_or((e.x, e.y));
        let Some(next) = self.continuation_tree(e, center) else { return false };
        let Some((x, y)) = self.forest_position(next) else { return false };
        let Some(goal) = self.forest_approach(e, x, y) else { return false };
        let forest = self.valley_fixture.as_mut().unwrap().forest_replay.as_mut().unwrap();
        if forest.promoted[next as usize].is_none()
            && forest.promote(ForestId(next), actor_id(next), WOOD).is_none() { return false; }
        let key = self.key_for(LAND, goal.0, goal.1);
        assign_forest_order(e, next, self.map.tile_of(x, y) as u32, goal, key);
        true
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    pub(super) fn clearance_compressed_stock(&self, actor: &Ent, center: [f32; 2]) -> Option<u32> {
        let forest = &self.rect_forest.as_ref()?.replay;
        let compressed = forest.promoted.iter().enumerate().filter_map(|(row, promoted)| {
            promoted.is_none().then_some(ForestId(row as u32))
        });
        forest.registry.select(compressed, center, 5.0, f32::from_bits(1061625612),
            |[x, y]| self.forest_approach_clear(actor, (x, y))).map(|id| id.0)
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    pub(super) fn clearance_promoted_stock(&self, actor: &Ent, center: (f32, f32)) -> Option<(u32, f32)> {
        let forest = &self.rect_forest.as_ref()?.replay;
        forest.promoted.iter().enumerate().filter_map(|(row, tree)| {
            if !tree.is_some_and(|tree| tree.wood > 0.0) { return None; }
            let row = row as u32;
            let (x, y) = self.forest_position(row)?;
            if individual_score(center, (x, y), 1) > 25.0 || self.forest_approach(actor, x, y).is_none() {
                return None;
            }
            let spare = WORKER_LIMIT as i32 - self.forest_staff(row).0 as i32;
            Some((row, individual_score(center, (x, y), spare)))
        }).min_by(|a, b| a.1.total_cmp(&b.1))
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    pub(super) fn clearance_promote_forest(&mut self, row: u32) -> bool {
        let Some(entry) = self.rect_forest.as_ref().and_then(|forest| forest.replay.promoted.get(row as usize))
        else { return false; };
        entry.is_some() || self.promote_forest_row(row).is_some()
    }

    #[cfg(feature = "ee2-rect277-gather-resume")]
    pub(super) fn clearance_start_forest(&mut self, actor: &mut Ent, row: u32) -> bool {
        let Some((x, y)) = self.forest_position(row) else { return false; };
        let Some(goal) = self.forest_approach(actor, x, y) else { return false; };
        let Some(forest) = self.valley_forest_play() else { return false; };
        if !forest.promoted.get(row as usize).and_then(Option::as_ref).is_some_and(|tree| tree.wood > 0.0)
            || !self.citizen_can_gather(actor.team, R_WOOD) || self.forest_staff(row).0 >= WORKER_LIMIT
        {
            return false;
        }
        let returning = actor.q[1] == 1 && actor.carry > 0.0;
        assign_forest_order(actor, row, self.map.tile_of(x, y) as u32, goal, self.key_for(LAND, goal.0, goal.1));
        actor.q[1] |= u32::from(returning);
        true
    }

    pub(crate) fn order_valley_forest(&mut self, team: u8, row: u32, ids: &[usize]) {
        let Some((x, y)) = self.forest_actor_can_order(team, row) else { return };
        let (mut reserved, _) = self.forest_staff(row);
        for &index in ids {
            if reserved >= WORKER_LIMIT { break }
            let e = &self.ents[index];
            if e.def != WORKER || e.team != team || (e.order == O_VALLEY_FOREST && e.q[3] == row)
                || (e.carry > 0.0 && e.q[2] == 0) { continue }
            let Some(goal) = self.forest_approach(e, x, y) else { continue };
            let Some(forest) = self.valley_forest_play() else { return; };
            if forest.promoted[row as usize].is_none() && self.promote_forest_row(row).is_none() {
                return;
            }
            let key = self.key_for(LAND, goal.0, goal.1);
            let tile = self.map.tile_of(x, y) as u32;
            self.native_stop_attack(index);
            #[cfg(feature = "ee2-rect276-clearance-resume")]
            self.cancel_rect_clearance(index);
            assign_forest_order(&mut self.ents[index], row, tile, goal, key);
            reserved += 1;
        }
    }

    pub(crate) fn do_valley_forest(&mut self, index: usize, e: &mut Ent, d: &Def,
        want: &mut (f32, f32), moving: &mut bool) -> Option<f32> {
        let row = e.q[3];
        let Some(wood) = self.valley_forest_play().and_then(|f| f.promoted.get(row as usize))
            .and_then(Option::as_ref).map(|actor| actor.wood) else {
            Self::set_idle(e);
            return None;
        };
        if !self.forest_gather_team(e.team) || e.def != WORKER {
            Self::set_idle(e);
            return None;
        }
        #[cfg(feature = "ee2-valley169-forest-retarget")]
        if wood == 0.0 && e.q[1] == 0 && self.continue_depleted_forest(e, row) { return None; }
        if wood == 0.0 || e.q[1] == 1 {
            e.q[4] = 0;
            if e.carry > 0.0 {
                e.q[1] = 1;
                #[cfg(feature = "ee2-rect289-citizen-work")]
                self.return_cargo_with_work_visual(index, e, d, want, moving);
                #[cfg(not(feature = "ee2-rect289-citizen-work"))]
                self.return_cargo(e, d, want, moving);
                if e.q[1] == 0 { e.fkey = self.key_for(LAND, e.ox, e.oy); }
            }
            if wood == 0.0 && e.carry == 0.0 { Self::set_idle(e); }
            return None;
        }
        let (x, y) = self.forest_position(row)?;
        let (dx, dy) = (x - e.x, y - e.y);
        if dx * dx + dy * dy > CUT_REACH * CUT_REACH {
            e.q[4] = 0;
            let (goal_x, goal_y) = (e.ox, e.oy);
            *want = self.steer(e, d, goal_x, goal_y);
            *moving = true;
            return None;
        }
        let face = Some(dy.atan2(dx));
        if e.q[4] == 0 {
            e.q[4] = 1;
            #[cfg(feature = "ee2-rect289-citizen-work")]
            self.retain_citizen_work(index);
            return face;
        }
        let player = self.players[e.team as usize];
        let profile = crate::ee2_units::lookup_class(WORKER, player.epoch)?;
        let rate = profile.gather?[usize::from(R_WOOD - 1)];
        let cap = profile.carry?;
        let amount = rate * self.sim_step();
        let scale = self.crown_gather_scale(e.team, WORKER, R_WOOD, false, false);
        let factor = self.modifiers[e.team as usize][WORKER as usize][crate::economy::gather_modifier(R_WOOD)] * scale;
        #[cfg(feature = "ee2-valley166-forest-clock")]
        let chop_time = self.sim_elapsed();
        let forest = self.forest_play_mut()?;
        let actor = forest.promoted.get_mut(row as usize)?.as_mut()?;
        #[cfg(feature = "ee2-valley166-forest-clock")]
        { actor.last_chop = Some(chop_time); }
        if remove_wood(&mut actor.wood, amount) {
            let (carry, full) = carry_wood(e.carry, amount, factor, player.gather, cap);
            e.carry = carry;
            e.q[2] = u32::from(R_WOOD);
            e.q[1] = u32::from(full);
            e.q[4] = u32::from(!full);
        } else {
            #[cfg(feature = "ee2-valley166-forest-clock")]
            { actor.exhaustion = Some(chop_time); }
            // Earlier workers in this tick must no longer render as active on an exhausted actor.
            for (other, worker) in self.ents[..self.high].iter_mut().enumerate() {
                if other != index && worker.alive && worker.order == O_VALLEY_FOREST && worker.q[3] == row { worker.q[4] = 0; }
            }
            e.q[4] = 0;
        }
        #[cfg(feature = "ee2-rect289-citizen-work")]
        if e.q[4] == 1 && e.q[1] == 0 {
            self.request_citizen_work(index, 28, f32::from_bits(0x3f4c_cccd));
        } else {
            self.retain_citizen_work(index);
        }
        face
    }

    pub(crate) fn valley_forest_chopping_angle(&self, e: &Ent) -> Option<f32> {
        if e.order != O_VALLEY_FOREST || e.q[4] != 1 { return None }
        let (x, y) = self.forest_position(e.q[3])?;
        Some((y - e.y).atan2(x - e.x))
    }

    pub(crate) fn valid_valley_forest_orders(&self, high: usize) -> bool {
        let forest = self.valley_forest_play();
        #[cfg(feature = "ee2-valley166-forest-clock")]
        if forest.is_some_and(|replay| !replay.timing_within(self.sim_elapsed())) { return false; }
        let mut reserved = vec![0; forest.map_or(0, |f| f.promoted.len())];
        self.ents[..high].iter().filter(|e| e.alive && e.order == O_VALLEY_FOREST).all(|e| {
            let row = e.q[3];
            let Some(actor) = forest.and_then(|f| f.promoted.get(row as usize))
                .and_then(Option::as_ref) else { return false };
            let Some((x, y)) = self.forest_position(row) else { return false };
            reserved[row as usize] += 1;
            self.forest_gather_team(e.team) && e.def == WORKER && e.qn == 0 && e.otgt == NONE && e.tgt == NONE
                && e.otile as usize == self.map.tile_of(x, y) && approaches(x, y).contains(&(e.ox, e.oy))
                && e.q[0] == u32::from(R_WOOD) && e.q[1] <= 1 && e.q[4] <= 1
                && (e.q[4] == 0 || (e.q[1] == 0 && actor.wood > 0.0))
                && (0.0..=15.0).contains(&e.carry)
                && (e.carry == 0.0 || self.resource_unlocked(e.team, e.q[2] as u8) && e.q[2] <= u32::from(u8::MAX))
                && reserved[row as usize] <= WORKER_LIMIT
        })
    }
}

#[no_mangle]
pub extern "C" fn valley164_enable_forest_play() -> u32 {
    if active_game().is_none() { return 0 }
    let game = crate::g();
    if game.tick != 0 || game.net || game.humans != 1 || game.n_ai != 1 { return 0 }
    let Some(fixture) = game.valley_fixture.as_mut().filter(|fixture| {
        fixture.native_house_assigned_clearance && fixture.neutral_opening && fixture.forest_replay.is_none()
    }) else { return 0 };
    let Some(source) = game.map.source_world.as_deref().and_then(|world| world.forest.as_ref()) else { return 0 };
    let mut forest = ForestReplay::new(source);
    forest.playable = true;
    fixture.forest_replay = Some(forest);
    1
}

#[no_mangle]
pub extern "C" fn cmd_valley_forest_gather(row: u32) -> u32 {
    let Some(game) = active_game() else { return 0 };
    let Some((x, y)) = game.forest_actor_can_order(game.local, row) else { return 0 };
    if game.forest_staff(row).0 >= WORKER_LIMIT { return 0 }
    let workers = crate::selected(game).into_iter().filter(|&index| {
        let e = &game.ents[index];
        e.def == WORKER && (e.order != O_VALLEY_FOREST || e.q[3] != row)
            && (e.carry <= 0.0 || e.q[2] != 0)
            && game.forest_approach(e, x, y).is_some()
    }).collect::<Vec<_>>();
    if workers.is_empty() { return 0 }
    let handles = crate::handles(game, &workers);
    crate::g().submit(C_VALLEY_FOREST_GATHER, 0.0, 0.0, row as i32, &handles);
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_cargo_words_match_native_witness() {
        let mut wood = WOOD;
        let amount = 0.05_f32 * 0.85_f32;
        assert!(remove_wood(&mut wood, amount));
        assert_eq!(wood.to_bits(), 0x449f_9ea4);
        let (cargo, full) = carry_wood(0.0, amount, 1.0, 1.0, 15.0);
        assert_eq!((cargo.to_bits(), full), (0x3d2e_147b, false));

        wood = 1.125;
        assert!(remove_wood(&mut wood, 0.125));
        assert_eq!(wood.to_bits(), 0x3f80_0000);
        let (cargo, full) = carry_wood(cargo, 0.125, 1.0, 1.0, 15.0);
        assert_eq!((cargo.to_bits(), full), (0x3e2b_851f, false));
        assert!(!remove_wood(&mut wood, 0.125));
        assert_eq!(wood.to_bits(), 0);

        wood = WOOD;
        assert!(remove_wood(&mut wood, 0.125));
        assert_eq!(wood.to_bits(), 0x449f_9c00);
        assert_eq!(carry_wood(14.99, 0.125, 1.0, 1.0, 15.0), (15.0, true));
    }
}

#[no_mangle]
pub extern "C" fn valley164_forest_actor_word(row: u32, field: u32) -> u32 {
    let Some(game) = active_game() else { return NONE };
    let Some(forest) = game.valley_forest_play() else { return NONE };
    let Some(actor) = forest.promoted.get(row as usize) else { return NONE };
    match (field, actor) {
        (0, None) => 0,
        (0, Some(actor)) => if actor.wood == 0.0 { 2 } else { 1 },
        (1, Some(actor)) => actor.actor,
        (2, Some(actor)) => actor.wood.to_bits(),
        (2, None) | (3, _) => WOOD.to_bits(),
        (4, None) | (5, None) => 0,
        (4, Some(_)) => game.forest_staff(row).1,
        (5, Some(_)) => game.forest_staff(row).0,
        #[cfg(feature = "ee2-valley166-forest-clock")]
        (6, Some(actor)) => actor.last_chop.map_or(NONE, f32::to_bits),
        #[cfg(feature = "ee2-valley166-forest-clock")]
        (7, Some(actor)) => actor.exhaustion.map_or(NONE, f32::to_bits),
        _ => NONE,
    }
}

#[cfg(feature = "ee2-rect235-forest")]
#[no_mangle]
pub extern "C" fn cmd_native_rect_forest_gather(row: u32) -> u32 {
    if active_game().is_none_or(|game| game.rect_forest.is_none()) { return 0; }
    cmd_valley_forest_gather(row)
}

#[cfg(feature = "ee2-rect235-forest")]
#[no_mangle]
pub extern "C" fn native_rect_forest_actor_word(row: u32, field: u32) -> u32 {
    if active_game().and_then(|game| game.rect_forest.as_ref())
        .and_then(|forest| forest.gather_position(row)).is_none() { return NONE; }
    valley164_forest_actor_word(row, field)
}
