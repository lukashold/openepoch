//! Farm plot and Citizen work state for authored Death Valley 2.
use std::collections::BTreeMap;
#[cfg(feature = "ee2-rect281-farm-family")]
use std::collections::BTreeSet;

use crate::ai9_count::Precision53::TowardZero;
use crate::defs::{Def, FARM, LAND, WORKER};
use crate::game::{Ent, Game, BOARDED, NONE, O_GATHER};
use crate::map::R_FOOD;

pub(crate) const TAG: &[u8; 4] = b"FCP1";
#[cfg(feature = "ee2-rect281-farm-family")]
pub(crate) const FAMILY_TAG: &[u8; 4] = b"FCP2";
#[cfg(feature = "ee2-rect282-farm-seeding")]
pub(crate) const VISUAL_TAG: &[u8; 4] = b"FCP3";
#[cfg(feature = "ee2-rect283-farm-recovery")]
pub(crate) const RECOVERY_TAG: &[u8; 4] = b"FCP4";
#[cfg(feature = "ee2-rect281-farm-family")]
const WHEAT: u32 = 0;
#[cfg(feature = "ee2-rect281-farm-family")]
const YAM: u32 = 2;
const OFFSET: f32 = f32::from_bits(0x3f33_3333);
const REACH_SQUARED: f32 = f32::from_bits(0x3db8_51ec);

#[derive(Clone, Copy, PartialEq, Eq)]
struct PlotGoal {
    farm: u32,
    plot: u8,
}

#[derive(Clone, Copy, Default)]
struct Farmer {
    timer: f32,
    goal: Option<PlotGoal>,
}

#[cfg(feature = "ee2-rect282-farm-seeding")]
#[derive(Clone, Copy)]
struct CropVisual {
    stage: u8,
    started: f32,
}

#[cfg(feature = "ee2-rect283-farm-recovery")]
struct FarmVisualRequest {
    plot: u8,
    visual: CropVisual,
}

#[derive(Default)]
pub(crate) struct FarmCrops {
    plots: BTreeMap<u32, u16>,
    farmers: BTreeMap<u32, Farmer>,
    #[cfg(feature = "ee2-rect281-farm-family")]
    yam_farms: BTreeSet<u32>,
    #[cfg(feature = "ee2-rect282-farm-seeding")]
    visuals: BTreeMap<(u32, u8), CropVisual>,
    #[cfg(feature = "ee2-rect283-farm-recovery")]
    latest: BTreeMap<u32, FarmVisualRequest>,
    #[cfg(feature = "ee2-rect283-farm-recovery")]
    mounted: BTreeMap<u32, [Option<CropVisual>; 4]>,
}

impl FarmCrops {
    fn word(&self, farm: u32) -> u16 {
        self.plots.get(&farm).copied().unwrap_or(0)
    }

    fn set_plot(&mut self, goal: PlotGoal, state: u16) {
        let shift = goal.plot * 3;
        let word = (self.word(goal.farm) & !(7 << shift)) | (state << shift);
        if word == 0 {
            self.plots.remove(&goal.farm);
        } else {
            self.plots.insert(goal.farm, word);
        }
    }

    fn clear_goal(&mut self, handle: u32) {
        if let Some(row) = self.farmers.get_mut(&handle) {
            row.goal = None;
            if row.timer == 0.0 { self.farmers.remove(&handle); }
        }
    }

    #[cfg(all(feature = "ee2-rect282-farm-seeding", not(feature = "ee2-rect283-farm-recovery")))]
    fn request_visual(&mut self, goal: PlotGoal, stage: u8, now: f32) {
        let visual = self.visuals.entry((goal.farm, goal.plot)).or_insert(CropVisual {
            stage: 3,
            started: 0.0,
        });
        // Each original FarmPlotSM coalesces the same request independently.
        if visual.stage != stage {
            visual.stage = stage;
            visual.started = if stage == 0 { now } else { 0.0 };
        }
    }

    #[cfg(feature = "ee2-rect283-farm-recovery")]
    fn request_visual(&mut self, goal: PlotGoal, stage: u8, now: f32) {
        if self.latest.get(&goal.farm).is_some_and(|row| row.plot == goal.plot && row.visual.stage == stage) {
            return;
        }
        let visual = CropVisual { stage, started: if stage == 0 { now } else { 0.0 } };
        self.latest.insert(goal.farm, FarmVisualRequest { plot: goal.plot, visual });
        let plots = self.mounted.entry(goal.farm).or_insert_with(|| {
            // FCP3 has per-child history but no actor request. Retain its mounted
            // poses when the first genuine request supplies the missing actor state.
            std::array::from_fn(|plot| self.visuals.remove(&(goal.farm, plot as u8)))
        });
        let child = &mut plots[usize::from(goal.plot)];
        if child.is_none_or(|row| row.stage != stage) {
            *child = Some(visual);
        }
    }

    #[cfg(feature = "ee2-rect282-farm-seeding")]
    fn child_visual(&self, farm: u32, plot: u8) -> Option<CropVisual> {
        #[cfg(feature = "ee2-rect283-farm-recovery")]
        if let Some(plots) = self.mounted.get(&farm) {
            return plots[usize::from(plot)];
        }
        self.visuals.get(&(farm, plot)).copied()
    }

    pub(crate) fn forget(&mut self, handle: u32, farm: bool) {
        self.farmers.remove(&handle);
        if farm {
            #[cfg(feature = "ee2-rect281-farm-family")]
            self.yam_farms.remove(&handle);
            #[cfg(feature = "ee2-rect282-farm-seeding")]
            for plot in 0..4 {
                self.visuals.remove(&(handle, plot));
            }
            #[cfg(feature = "ee2-rect283-farm-recovery")]
            {
                self.latest.remove(&handle);
                self.mounted.remove(&handle);
            }
            self.plots.remove(&handle);
            self.farmers.retain(|_, worker| {
                if worker.goal.is_some_and(|goal| goal.farm == handle) { worker.goal = None; }
                worker.goal.is_some() || worker.timer > 0.0
            });
        }
    }

    fn words(&self) -> impl Iterator<Item = u32> + '_ {
        let words = [self.plots.len() as u32, self.farmers.len() as u32].into_iter()
            .chain(self.plots.iter().flat_map(|(&farm, &word)| [farm, u32::from(word)]))
            .chain(self.farmers.iter().flat_map(|(&worker, row)| {
                let (farm, plot) = row.goal.map_or((NONE, NONE), |goal| (goal.farm, u32::from(goal.plot)));
                [worker, row.timer.to_bits(), farm, plot]
            }));
        #[cfg(feature = "ee2-rect281-farm-family")]
        let words = words.chain(
            matches!(self.section_tag(), Some(tag) if tag != TAG).then_some(self.yam_farms.len() as u32).into_iter()
                .chain(self.yam_farms.iter().flat_map(|&handle| [handle, YAM])),
        );
        #[cfg(feature = "ee2-rect282-farm-seeding")]
        let words = words.chain(
            self.section_tag().is_some_and(|tag| tag != TAG && tag != FAMILY_TAG)
                .then_some(self.visuals.len() as u32).into_iter()
                .chain(self.visuals.iter().flat_map(|(&(farm, plot), visual)| {
                    [farm, u32::from(plot), u32::from(visual.stage), visual.started.to_bits()]
                })),
        );
        #[cfg(feature = "ee2-rect283-farm-recovery")]
        let words = words.chain(
            (!self.latest.is_empty()).then_some(self.latest.len() as u32).into_iter()
                .chain(self.latest.iter().flat_map(|(&farm, request)| {
                    [farm, u32::from(request.plot), u32::from(request.visual.stage), request.visual.started.to_bits()]
                })),
        );
        words
    }

    fn section_tag(&self) -> Option<&'static [u8; 4]> {
        #[cfg(feature = "ee2-rect283-farm-recovery")]
        if !self.latest.is_empty() { return Some(RECOVERY_TAG); }
        #[cfg(feature = "ee2-rect282-farm-seeding")]
        if !self.visuals.is_empty() { return Some(VISUAL_TAG); }
        #[cfg(feature = "ee2-rect281-farm-family")]
        if !self.yam_farms.is_empty() { return Some(FAMILY_TAG); }
        (!self.plots.is_empty() || !self.farmers.is_empty()).then_some(TAG)
    }

    pub(crate) fn section(&self) -> Option<Vec<u8>> {
        let mut bytes = self.section_tag()?.to_vec();
        for word in self.words() { bytes.extend_from_slice(&word.to_le_bytes()); }
        Some(bytes)
    }

    pub(crate) fn hash(&self, mix: &mut impl FnMut(u32)) {
        let Some(tag) = self.section_tag() else { return; };
        mix(u32::from_le_bytes(*tag));
        for word in self.words() { mix(word); }
    }
}

fn plot_for(worker: &Ent, farm: &Ent) -> u8 {
    let dx = TowardZero.store_f32(f64::from(worker.x) - f64::from(farm.x));
    let dy = TowardZero.store_f32(f64::from(worker.y) - f64::from(farm.y));
    match (f64::from(dy) + f64::from(dx) < 0.0, f64::from(dy) - f64::from(dx) < 0.0) {
        (true, true) => 0,
        (false, true) => 1,
        (false, false) => 2,
        (true, false) => 3,
    }
}

fn point(farm: &Ent, plot: u8) -> [f32; 2] {
    let [dx, dy] = [[0.0, -OFFSET], [OFFSET, 0.0], [0.0, OFFSET], [-OFFSET, 0.0]][plot as usize];
    [
        TowardZero.store_f32(f64::from(farm.x) + f64::from(dx)),
        TowardZero.store_f32(f64::from(farm.y) + f64::from(dy)),
    ]
}

fn in_reach(worker: &Ent, [x, y]: [f32; 2]) -> bool {
    let dx = f64::from(worker.x) - f64::from(x);
    let dy = f64::from(worker.y) - f64::from(y);
    dx * dx + dy * dy < f64::from(REACH_SQUARED)
}

impl Game {
    #[cfg(feature = "ee2-rect281-farm-family")]
    pub(crate) fn select_rect_farm_family(&mut self, index: usize) {
        if !self.rect_crop_nonblocking(&self.ents[index]) { return; }
        // The bound Death Valley 2 terrain is Temperate. Its ordered table is Wheat, Yam.
        if self.source_birth.rng.next_word15() % 2 == 1 {
            self.rect_farm_crops.yam_farms.insert(self.h(index));
        }
    }

    pub(crate) fn rect_crop_nonblocking(&self, actor: &Ent) -> bool {
        actor.def == FARM && self.rect_farm_work_enabled()
    }

    pub(crate) fn clear_rect_farm_goal(&mut self, index: usize) {
        self.rect_farm_crops.clear_goal(self.h(index));
    }

    fn schedule_rect_farm_goal(&mut self, handle: u32, worker: &Ent, farm: &Ent) -> PlotGoal {
        let previous = plot_for(worker, farm);
        let roll = self.source_birth.rng.next_word15() % 3;
        let goal = PlotGoal { farm: worker.otgt, plot: (previous + 1 + roll as u8) & 3 };
        self.rect_farm_crops.farmers.entry(handle).or_default().goal = Some(goal);
        goal
    }

    pub(crate) fn start_rect_farm_goal(&mut self, index: usize, worker: &Ent) {
        if worker.def != WORKER || worker.order != O_GATHER || worker.q[0] != u32::from(R_FOOD)
            || worker.stuck == BOARDED || worker.q[1] != 0 || !self.rect_farm_work_enabled()
        { return; }
        let Some(farm) = self.get(worker.otgt).map(|slot| self.ents[slot]) else { return; };
        if farm.def == FARM && farm.team == worker.team && farm.built >= 1.0 && farm.hp > 0.0 {
            self.schedule_rect_farm_goal(self.h(index), worker, &farm);
        }
    }

    pub(crate) fn do_rect_farm_crop(&mut self, index: usize, worker: &mut Ent, def: &Def,
        farm: usize, want: &mut (f32, f32), moving: &mut bool)
    {
        let handle = self.h(index);
        let farm = self.ents[farm];
        let goal = self.rect_farm_crops.farmers.get(&handle).and_then(|row| row.goal)
            .filter(|goal| goal.farm == worker.otgt)
            .unwrap_or_else(|| self.schedule_rect_farm_goal(handle, worker, &farm));
        let destination = point(&farm, goal.plot);
        [worker.ox, worker.oy] = destination;
        if !in_reach(worker, destination) {
            let key = self.key_for(LAND, destination[0], destination[1]);
            if worker.fkey != key {
                worker.fkey = key;
                worker.fslot = u16::MAX;
            }
            *want = self.steer(worker, def, destination[0], destination[1]);
            *moving = true;
            return;
        }
        if self.rect_farm_work(index, worker) != Some(true) {
            #[cfg(feature = "ee2-rect289-citizen-work")]
            self.retain_citizen_work(index);
            return;
        }

        let dt = self.sim_step();
        let row = self.rect_farm_crops.farmers.get_mut(&handle).unwrap();
        row.timer = TowardZero.store_f32(f64::from(row.timer) + f64::from(dt));
        let timer = row.timer;
        let plot = (self.rect_farm_crops.word(goal.farm) >> (goal.plot * 3)) & 7;
        #[cfg(feature = "ee2-rect289-citizen-work")]
        self.request_citizen_work(index, if plot == 0 {23} else {24}, 0.0);
        #[cfg(feature = "ee2-rect282-farm-seeding")]
        {
            // The source requests the current stage before writing the next work stage.
            let now = self.sim_elapsed();
            self.rect_farm_crops.request_visual(goal, plot as u8, now);
        }
        if timer > [8.0, 2.0, 4.0, 6.0][plot as usize] {
            self.rect_farm_crops.set_plot(goal, (plot + 1) & 3);
        }
        if plot != 0 && self.harvest_rect_farm(index, worker, dt) { return; }
        if plot == 0 && timer > 8.0 || plot == 3 && timer > 6.0 {
            let next = self.schedule_rect_farm_goal(handle, worker, &farm);
            self.rect_farm_crops.farmers.get_mut(&handle).unwrap().timer = 0.0;
            [worker.ox, worker.oy] = point(&farm, next.plot);
        }
    }

    fn harvest_rect_farm(&mut self, index: usize, worker: &mut Ent, dt: f32) -> bool {
        let player = self.players[worker.team as usize];
        let Some(stats) = crate::ee2_units::lookup_class(worker.def, player.epoch) else { return false; };
        let (Some(rate), Some(limit)) = (stats.farm_rate, stats.farm_carry) else { return false; };
        let amount = TowardZero.store_f32(f64::from(rate) * f64::from(dt));
        let scale = self.crown_gather_scale(worker.team, worker.def, R_FOOD, true, false)
            + self.granary_farm_bonus(worker);
        let modifier = self.modifiers[worker.team as usize][worker.def as usize]
            [crate::economy::gather_modifier(R_FOOD)] * scale;
        let factor = TowardZero.mul(f64::from(player.gather), f64::from(modifier));
        let updated = TowardZero.add(f64::from(worker.carry), TowardZero.mul(f64::from(amount), factor));
        worker.carry = TowardZero.store_f32(updated.min(f64::from(limit)));
        worker.q[2] = u32::from(R_FOOD);
        if updated < f64::from(limit) { return false; }
        worker.q[1] = 1;
        self.release_rect_farm_work(index);
        true
    }

    pub(crate) fn restore_rect_farm_crops(&mut self, bytes: &[u8], high: usize) -> Option<()> {
        if !self.rect_farm_work_enabled() { return None; }
        let (tag, body) = bytes.split_at_checked(4)?;
        #[cfg(feature = "ee2-rect282-farm-seeding")]
        let has_recovery = {
            #[cfg(feature = "ee2-rect283-farm-recovery")]
            { tag == RECOVERY_TAG }
            #[cfg(not(feature = "ee2-rect283-farm-recovery"))]
            { false }
        };
        let has_visuals = {
            #[cfg(feature = "ee2-rect282-farm-seeding")]
            { tag == VISUAL_TAG || has_recovery }
            #[cfg(not(feature = "ee2-rect282-farm-seeding"))]
            { false }
        };
        let has_families = {
            #[cfg(feature = "ee2-rect281-farm-family")]
            { tag == FAMILY_TAG || has_visuals }
            #[cfg(not(feature = "ee2-rect281-farm-family"))]
            { false }
        };
        if !has_families && tag != TAG { return None; }
        let (counts, rows) = body.split_at_checked(8)?;
        let words = |row: &[u8]| -> [u32; 2] {
            [u32::from_le_bytes(row[..4].try_into().unwrap()), u32::from_le_bytes(row[4..8].try_into().unwrap())]
        };
        let [plots, farmers] = words(counts).map(|count| count as usize);
        let total = plots.checked_add(farmers)?;
        if total > high { return None; }
        let (plot_rows, rows) = rows.split_at_checked(plots * 8)?;
        let (farmer_rows, family_rows) = rows.split_at_checked(farmers * 16)?;
        if !has_families && (total == 0 || !family_rows.is_empty()) { return None; }
        let mut state = FarmCrops::default();
        for row in plot_rows.chunks_exact(8) {
            let [handle, word] = words(row);
            let index = self.get(handle)?;
            if index >= high || self.ents[index].def != FARM || self.ents[index].built < 1.0
                || word == 0 || word >= 1 << 12 || (0..4).any(|plot| ((word >> (3 * plot)) & 7) > 3)
                || state.plots.last_key_value().is_some_and(|(&last, _)| last >= handle)
            { return None; }
            state.plots.insert(handle, word as u16);
        }
        for row in farmer_rows.chunks_exact(16) {
            let [handle, timer] = words(row);
            let [farm, plot] = words(&row[8..]);
            let index = self.get(handle)?;
            let worker = &self.ents[index];
            let timer = f32::from_bits(timer);
            if index >= high || worker.def != WORKER || !timer.is_finite() || timer.is_sign_negative()
                || state.farmers.last_key_value().is_some_and(|(&last, _)| last >= handle)
            { return None; }
            let goal = if farm == NONE {
                if plot != NONE || timer == 0.0 { return None; }
                None
            } else {
                let farm_index = self.get(farm)?;
                let target = &self.ents[farm_index];
                if farm_index >= high || target.def != FARM || target.built < 1.0 || plot >= 4
                    || worker.order != O_GATHER || worker.otgt != farm || worker.q[0] != u32::from(R_FOOD)
                    || worker.q[1] != 0 || worker.stuck == BOARDED
                { return None; }
                Some(PlotGoal { farm, plot: plot as u8 })
            };
            state.farmers.insert(handle, Farmer { timer, goal });
        }
        #[cfg(feature = "ee2-rect281-farm-family")]
        let visual_rows = if has_families {
            let (count, rows) = family_rows.split_at_checked(4)?;
            let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
            if count > high || !has_visuals && count == 0 { return None; }
            let (families, visuals) = rows.split_at_checked(count * 8)?;
            if !has_visuals && !visuals.is_empty() { return None; }
            for row in families.chunks_exact(8) {
                let [handle, family] = words(row);
                let index = self.get(handle)?;
                if index >= high || self.ents[index].def != FARM || family != YAM
                    || state.yam_farms.last().is_some_and(|&last| last >= handle)
                { return None; }
                state.yam_farms.insert(handle);
            }
            visuals
        } else {
            &[][..]
        };
        #[cfg(all(feature = "ee2-rect281-farm-family", not(feature = "ee2-rect282-farm-seeding")))]
        debug_assert!(visual_rows.is_empty());
        #[cfg(feature = "ee2-rect282-farm-seeding")]
        if has_visuals {
            let (count, rows) = visual_rows.split_at_checked(4)?;
            let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
            if (!has_recovery && count == 0) || count > high * 4 { return None; }
            let (legacy, tail) = rows.split_at_checked(count * 16)?;
            let latest = if has_recovery {
                let (count, rows) = tail.split_at_checked(4)?;
                let count = u32::from_le_bytes(count.try_into().ok()?) as usize;
                if count == 0 || count > high || rows.len() != count * 16 { return None; }
                rows
            } else {
                if !tail.is_empty() { return None; }
                &[][..]
            };
            for (row, recovered) in legacy.chunks_exact(16).map(|row| (row, false))
                .chain(latest.chunks_exact(16).map(|row| (row, true)))
            {
                let [farm, plot] = words(row);
                let [stage, started] = words(&row[8..]);
                let index = self.get(farm)?;
                if index >= high || self.ents[index].def != FARM || self.ents[index].built < 1.0
                    || plot >= 4 || stage >= 4
                { return None; }
                let key = (farm, plot as u8);
                let logical = u32::from(state.word(farm) >> (plot * 3)) & 7;
                let start = f32::from_bits(started);
                if logical != stage && logical != (stage + 1) % 4
                    || stage != 0 && started != 0
                    || stage == 0 && (!start.is_finite() || start.is_sign_negative() || start > self.sim_elapsed())
                { return None; }
                let visual = CropVisual { stage: stage as u8, started: start };
                #[cfg(feature = "ee2-rect283-farm-recovery")]
                if recovered {
                    if state.latest.last_key_value().is_some_and(|(&last, _)| last >= farm)
                        || state.visuals.range((farm, 0)..=(farm, 3)).next().is_some()
                    { return None; }
                    state.latest.insert(farm, FarmVisualRequest { plot: key.1, visual });
                    let mut plots = [None; 4];
                    plots[usize::from(key.1)] = Some(visual);
                    state.mounted.insert(farm, plots);
                    continue;
                }
                #[cfg(not(feature = "ee2-rect283-farm-recovery"))]
                debug_assert!(!recovered);
                if state.visuals.last_key_value().is_some_and(|(&last, _)| last >= key) { return None; }
                state.visuals.insert(key, visual);
            }
        }
        self.rect_farm_crops = state;
        Some(())
    }
}

#[cfg(feature = "ee2-rect281-farm-family")]
#[no_mangle]
pub extern "C" fn native_rect_farm_family(handle: u32) -> u32 {
    let Some(game) = super::active_game() else { return NONE; };
    let Some(farm) = game.get(handle).map(|slot| &game.ents[slot]) else { return NONE; };
    if !game.rect_crop_nonblocking(farm) { return NONE; }
    if game.rect_farm_crops.yam_farms.contains(&handle) { YAM } else { WHEAT }
}

#[no_mangle]
pub extern "C" fn native_rect_farm_visual(handle: u32) -> u32 {
    let Some(game) = super::active_game() else { return NONE; };
    let Some(farm) = game.get(handle).map(|slot| &game.ents[slot]) else { return NONE; };
    if !game.rect_crop_nonblocking(farm) { return NONE; }
    #[cfg(feature = "ee2-rect283-farm-recovery")]
    if let Some(plots) = game.rect_farm_crops.mounted.get(&handle) {
        return plots.iter().enumerate().fold(0, |word, (plot, state)| {
            word | (state.map_or(3, |row| u32::from(row.stage)) << (plot * 3))
        });
    }
    let word = u32::from(game.rect_farm_crops.word(handle));
    let mut visual = (0..4).fold(0, |output, plot| {
        let state = (word >> (plot * 3)) & 7;
        output | (if state == 0 { 3 } else { state }) << (plot * 3)
    });
    for (&worker, row) in &game.rect_farm_crops.farmers {
        let Some(goal) = row.goal.filter(|goal| goal.farm == handle) else { continue; };
        let Some(index) = game.get(worker) else { continue; };
        if row.timer > 0.0 && (word >> (goal.plot * 3)) & 7 == 0
            && game.rect_farm_cutting(index) && in_reach(&game.ents[index], point(farm, goal.plot))
        {
            visual &= !(7 << (goal.plot * 3));
        }
    }
    #[cfg(feature = "ee2-rect282-farm-seeding")]
    for plot in 0..4 {
        if let Some(state) = game.rect_farm_crops.child_visual(handle, plot) {
            let shift = plot * 3;
            visual = (visual & !(7 << shift)) | (u32::from(state.stage) << shift);
        }
    }
    visual
}

#[cfg(feature = "ee2-rect282-farm-seeding")]
#[no_mangle]
pub extern "C" fn native_rect_farm_seeding_elapsed(handle: u32, plot: u32) -> f32 {
    let elapsed = || {
        let game = super::active_game()?;
        let plot = u8::try_from(plot).ok().filter(|&plot| plot < 4)?;
        let visual = game.rect_farm_crops.child_visual(handle, plot)?;
        if visual.stage != 0 { return None; }
        Some(TowardZero.store_f32(f64::from(game.sim_elapsed()) - f64::from(visual.started)))
    };
    elapsed().unwrap_or(f32::NAN)
}

#[no_mangle]
pub extern "C" fn native_rect_farm_worker(handle: u32, field: u32) -> u32 {
    let Some(game) = super::active_game() else { return NONE; };
    let Some(row) = game.rect_farm_crops.farmers.get(&handle) else { return NONE; };
    match field {
        0 => row.timer.to_bits(),
        1 => row.goal.map_or(NONE, |goal| goal.farm),
        2 => row.goal.map_or(NONE, |goal| u32::from(goal.plot)),
        _ => NONE,
    }
}
