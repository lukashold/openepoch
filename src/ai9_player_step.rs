//! Research path through the player update, AI wrapper and 004E31D0 planner.
//! Opaque callbacks stand for the surrounding world/production decisions.

use crate::ai9_planner::PlannerData;
use crate::ai9_research_dispatch::{
    dispatch, InvalidInput, Pass, ResearchQueries, ResearchState, Timing,
};
use crate::ai9_research_tick::{tick_active, ResearchFrame, ResearchTick};
use crate::ai9_tech_tree::TechnologyTree;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControllerClock {
    pub last_frame: f32,
    pub last_planner: f32,
    pub next_random_deadline: f32,
    pub previous_aip_word: u32,
    pub research: ResearchState,
}

pub struct PlayerFrame {
    pub research_tick: ResearchFrame,
    /// Source game time C9C3E4, distinct from the current frame delta.
    pub now: f32,
    /// Live C08DDC and C08DEC, with PE initializers 1 and 60 seconds.
    pub planner_interval: f32,
    pub missing_context_delay: f32,
}

pub struct ResearchControls {
    pub interval: f32,
    pub epoch_delay: i32,
    pub measure_threshold: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlannerCallback {
    Update4d0e20,
    Update4ee090(f32),
    SetContext(i32),
    AipChanged4e5150,
    AipChanged4cdde0,
    Before4e7400,
    Before4e6210,
    Before4e9830,
    AuxiliaryPair,
    After4e54b0,
    After4e75f0,
    After4e8e20,
    Finish4ff3b0,
}

pub trait PlayerPlanner: ResearchTick + TechnologyTree + ResearchQueries {
    fn ai_disabled(&self) -> bool;
    fn aip_word_at_0c(&self) -> u32;
    /// 004F1200 may replace the profile, including its technology list.
    fn refresh_aip(&mut self, data: &mut PlannerData);
    fn has_planner_context_at_34(&self) -> bool;
    /// First 008672D0 iterator, then the player's optional alternate iterator.
    fn find_planner_context(&mut self, alternate: bool) -> Option<i32>;
    /// 007FE530 readiness condition for the two auxiliary updates.
    fn planner_auxiliary_ready(&mut self) -> bool;
    fn planner_callback(&mut self, callback: PlannerCallback);
    /// Loaded after earlier planner callbacks so a profile refresh takes effect.
    fn research_controls(&self) -> ResearchControls;
    /// Runtime AIP+364/+368, consumed only on a due RNG deadline.
    fn planner_random_interval(&self) -> [f32; 2];
    /// Caller uses the same state consumed by native births and Count.
    fn planner_word15(&mut self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePass {
    Human,
    AiDisabled,
    PlannerNotDue,
    MissingPlannerContext,
    Research(Pass),
}

pub fn step(
    tree: &mut impl PlayerPlanner,
    ai: Option<(&mut ControllerClock, &mut PlannerData)>,
    frame: PlayerFrame,
) -> Result<FramePass, InvalidInput> {
    // 005E2CF1 always updates the existing technology rows before 005E2D00.
    tick_active(tree, frame.research_tick);
    let Some((clock, data)) = ai else {
        return Ok(FramePass::Human);
    };

    let now = frame.now;
    let precision = frame.research_tick.precision;
    let delta = precision.store_f32(precision.add(now as f64, -(clock.last_frame as f64)));
    clock.last_frame = now;
    if tree.ai_disabled() {
        return Ok(FramePass::AiDisabled);
    }
    if tree.aip_word_at_0c() < 2 {
        tree.refresh_aip(data);
    }
    tree.planner_callback(PlannerCallback::Update4d0e20);
    tree.planner_callback(PlannerCallback::Update4ee090(delta));
    let result = planner_step(tree, clock, data, &frame);
    tree.planner_callback(PlannerCallback::Finish4ff3b0);
    result
}

fn planner_step(
    tree: &mut impl PlayerPlanner,
    clock: &mut ControllerClock,
    data: &mut PlannerData,
    frame: &PlayerFrame,
) -> Result<FramePass, InvalidInput> {
    let now = frame.now;
    let precision = frame.research_tick.precision;
    let delta = precision.add(now as f64, -(clock.last_planner as f64));
    if delta < frame.planner_interval as f64 {
        return Ok(FramePass::PlannerNotDue);
    }
    clock.last_planner = now;
    if !tree.has_planner_context_at_34() {
        let context = tree
            .find_planner_context(false)
            .or_else(|| tree.find_planner_context(true));
        let Some(context) = context else {
            clock.last_planner =
                precision.store_f32(precision.add(now as f64, frame.missing_context_delay as f64));
            return Ok(FramePass::MissingPlannerContext);
        };
        tree.planner_callback(PlannerCallback::SetContext(context));
    }

    let word = tree.aip_word_at_0c();
    let changed = word != clock.previous_aip_word;
    if changed {
        clock.previous_aip_word = word;
        tree.planner_callback(PlannerCallback::AipChanged4e5150);
        tree.planner_callback(PlannerCallback::AipChanged4cdde0);
    }
    tree.planner_callback(PlannerCallback::Before4e7400);
    if !changed {
        tree.planner_callback(PlannerCallback::Before4e6210);
    }
    tree.planner_callback(PlannerCallback::Before4e9830);
    let controls = tree.research_controls();
    let timing = Timing {
        now,
        precision,
        check_interval: controls.interval,
        epoch_delay: controls.epoch_delay,
        measure_threshold: controls.measure_threshold,
    };
    let result = dispatch(&mut clock.research, data, timing, tree)?;
    if tree.has_planner_context_at_34() && tree.planner_auxiliary_ready() {
        tree.planner_callback(PlannerCallback::AuxiliaryPair);
    }
    tree.planner_callback(PlannerCallback::After4e54b0);
    tree.planner_callback(PlannerCallback::After4e75f0);
    tree.planner_callback(PlannerCallback::After4e8e20);
    if now >= clock.next_random_deadline {
        let [minimum, maximum] = tree.planner_random_interval().map(f64::from);
        let width = precision.add(maximum, -minimum);
        let word = tree.planner_word15();
        let offset = precision.mul(
            precision.mul(width, word as f64),
            f32::from_bits(0x38000100) as f64,
        );
        clock.next_random_deadline =
            precision.store_f32(precision.add(precision.add(offset, now as f64), minimum));
    }
    Ok(FramePass::Research(result))
}
