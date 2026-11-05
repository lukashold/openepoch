//! Source research for the first-epoch skirmish projection.

use core::ptr::addr_of_mut;

use crate::ai9_count::Precision53;
use crate::ai9_planner::{AipRead, BuildRow, Context, Name, PlannerData};
use crate::ai9_player_step::{
    step, ControllerClock, PlannerCallback, PlayerFrame, PlayerPlanner, ResearchControls,
};
use crate::ai9_rank::{self as rank, FAMILY_COUNT, FIRST_RANK, NODE_COUNT, PREVIOUS_NODE_COUNT};
use crate::ai9_research_activation::{ActiveResearch, PlayerTimeScale, ResearchTiming};
use crate::ai9_research_completion::{CompletionLedger, ResearchCompletion};
use crate::ai9_research_debit::StartAccounts;
use crate::ai9_research_dispatch::{ResearchQueries, ResearchState};
use crate::ai9_research_start::{start, PaidResearch, ResearchStart, IN_PROGRESS};
use crate::ai9_research_tick::{cancel, tick_active, ResearchFrame, ResearchTick};
use crate::ai9_research_unlock::{unlock_successors, ImprovementFamilies, ResearchUnlock};
use crate::ai9_tech_tree::{Budget, Node, TechnologyTree};
use crate::defs::{DefId, TC, WAREHOUSE};
use crate::game::{Game, DT};

include!("ai9_game_source_rows.rs");

const PRECISION: Precision53 = Precision53::NearestEven;

pub(crate) struct Tree {
    pub(crate) flags: [u32; NODE_COUNT],
    names: [Name; NODE_COUNT],
    pub(crate) active: Vec<ActiveResearch>,
    pub(crate) pending: Vec<PaidResearch>,
    requirements: [Vec<i32>; NODE_COUNT],
    improvement_hosts: [Name; FAMILY_COUNT],
    family_count: usize,
    elite_dependents: [[i32; 1]; FAMILY_COUNT],
    families: ImprovementFamilies,
    pub(crate) ledger: CompletionLedger,
}

pub(crate) struct DurableResearch {
    pub(crate) owner: u8,
    pub(crate) manual: bool,
    pub(crate) tree: Tree,
    pub(crate) clock: ControllerClock,
    data: PlannerData,
}

struct World<'a> {
    game: &'a mut Game,
    tree: &'a mut Tree,
    owner: u8,
    balance: [f32; 10],
}

fn game() -> Option<&'static mut Game> {
    // This ABI shares the existing single-threaded WASM Game entry points.
    unsafe { (*addr_of_mut!(crate::GAME)).as_deref_mut() }
}

fn snapshot_balance(game: &Game, owner: u8) -> [f32; 10] {
    let mut balance = [0.0; 10];
    for (slot, native) in crate::economy::NATIVE_IDS.into_iter().enumerate() {
        balance[native as usize] = game.resource(owner, slot);
    }
    balance[9] = game.research_points[owner as usize];
    balance
}

impl World<'_> {
    fn completed_host_count(&self, def: DefId) -> i32 {
        let rank_family = usize::from(DefId::MAX - def);
        self.game.ents[..self.game.high]
            .iter()
            .filter(|entity| {
                entity.alive
                    && entity.hp > 0.0
                    && entity.team == self.owner
                    && entity.built >= 1.0
                    && if rank_family < FAMILY_COUNT {
                        rank::unit_family(entity.def, self.game.players[self.owner as usize].epoch)
                            == Some(rank_family)
                    } else {
                        entity.def == def
                    }
            })
            .count() as i32
    }
    fn save_wallet(&mut self) {
        let owner = self.owner as usize;
        for (slot, source_slot) in crate::economy::NATIVE_IDS.into_iter().enumerate() {
            let value = self.balance[source_slot as usize];
            if slot < 3 {
                self.game.players[owner].res[slot] = value;
            } else {
                self.game.extra_resources[owner][slot - 3] = value;
            }
        }
        self.game.research_points[owner] = self.balance[9];
    }
}

impl AipRead for Tree {
    fn registry(&self, _: Context) -> &[Name] {
        &[]
    }
    fn build_rows(&self, _: &Name) -> &[BuildRow] {
        &[]
    }
    fn tech_names(&self) -> &[Name] {
        &self.names[..3]
    }
}

impl ResearchStart for World<'_> {
    fn node_at(&self, index: i32) -> Node {
        let index = index as usize;
        Node {
            key: index as u32,
            source_epoch: SOURCE_EPOCH.get(index).copied().unwrap_or(1),
            flags: self.tree.flags[index],
            prerequisite_index: if let Some(family) = rank::family_at(index) {
                i32::from(DefId::MAX - family as DefId)
            } else {
                SOURCE_GAME_HOST[index].map_or(-1, i32::from)
            },
        }
    }
    fn flags_at(&mut self, index: i32) -> &mut u32 {
        &mut self.tree.flags[index as usize]
    }
    fn category_at(&self, index: i32) -> i32 {
        category(index as usize)
    }
    fn prerequisite_count(&mut self, index: i32) -> i32 {
        self.completed_host_count(index as DefId)
    }
    fn research_budget(&mut self, node: Node) -> Budget {
        Budget {
            balance: self.balance,
            cost: quote(self.game, self.owner, node.key as usize),
        }
    }
    fn production_start_gate(&mut self, _: i32, _: i32) -> bool {
        false
    }
    fn start_epoch_at_0c(&self) -> i32 {
        i32::from(self.game.players[self.owner as usize].epoch) + 1
    }
    fn maximum_epoch(&mut self) -> i32 {
        i32::from(self.game.match_final_epoch()) + 1
    }
    fn next_main_flags(&mut self, epoch: i32) -> Option<u32> {
        (epoch == 2).then_some(self.tree.flags[3])
    }
    fn quote_start(&mut self, index: i32, cost: &mut [f32; 11]) {
        *cost = quote(self.game, self.owner, index as usize);
    }
    fn start_accounts(&mut self) -> StartAccounts<'_> {
        StartAccounts {
            balance: &mut self.balance,
            natural_spent: None,
            precision: PRECISION,
        }
    }
    fn active_research(&self) -> &[ActiveResearch] {
        &self.tree.active
    }
    fn append_active_research(&mut self, active: ActiveResearch) {
        self.tree.active.push(active);
    }
    fn pending_research(&mut self) -> &mut Vec<PaidResearch> {
        &mut self.tree.pending
    }
    fn research_timing(&self, index: i32) -> ResearchTiming<'_> {
        ResearchTiming {
            node_seconds: duration(index as usize),
            player_scale: PlayerTimeScale::Technology(1.0),
            area_scales: &[],
            minimum_area_scale: 0.05,
            precision: PRECISION,
        }
    }
}

impl ResearchUnlock for World<'_> {
    fn epoch_nodes(&self, epoch: i32) -> &[i32] {
        match epoch {
            1 if self.tree.family_count == FAMILY_COUNT => &[0, 4, 5, 6, 7, 8, 9, 10, 11, 1, 2],
            1 => &[0, 4, 5, 1, 2],
            2 => &[3],
            _ => &[],
        }
    }
    fn completion_dependents(&self, index: i32) -> &[i32] {
        let index = index as usize;
        rank::family_at(index)
            .filter(|&family| rank::pair(family)[0] == index)
            .map_or(&[], |family| self.tree.elite_dependents[family].as_slice())
    }
    fn remaining_requirements(&mut self, index: i32) -> &mut Vec<i32> {
        &mut self.tree.requirements[index as usize]
    }
    fn node_civ_gate(&self, _: i32) -> i32 {
        0
    }
    fn civilization(&self) -> i32 {
        i32::from(self.game.civilizations[self.owner as usize])
    }
    fn main_threshold(&self) -> i32 {
        6
    }
    fn improvement_families(&self) -> &ImprovementFamilies {
        &self.tree.families
    }
    fn current_improvement_host(&self, index: i32) -> Option<&Name> {
        rank::family_at(index as usize).map(|family| &self.tree.improvement_hosts[family])
    }
    fn improvement_hide_list(&self, _: i32) -> &[i32] {
        &[]
    }
}

impl ResearchCompletion for World<'_> {
    fn completion_ledger(&mut self) -> &mut CompletionLedger {
        &mut self.tree.ledger
    }
    fn completion_upgrade(&self, index: i32) -> Option<i32> {
        game_id(index).map(|id| id as i32)
    }
    fn apply_completion_upgrade(&mut self, upgrade: i32) {
        let id = upgrade as usize;
        self.game.researched[self.owner as usize][id / 32] |= 1 << (id % 32);
        self.game.rebuild_modifiers(self.owner, true);
    }
    fn apply_completion_improvement(&mut self, index: i32) {
        let index = index as usize;
        let family = rank::family_at(index).unwrap();
        let after = (index - rank::pair(family)[0] + 1) as u8;
        // The completed flag is the saved source rank. Apply the health
        // fraction change to existing units at the completion boundary.
        rank::rescale_live_hp(self.game, self.owner, family, after - 1, after);
    }
}

impl ResearchTick for World<'_> {
    fn active_research_mut(&mut self) -> &mut Vec<ActiveResearch> {
        &mut self.tree.active
    }
    fn research_host_hp(&mut self, _: i32) -> Option<f32> {
        None
    }
}

impl TechnologyTree for World<'_> {
    fn node(&mut self, name: &Name) -> Option<Node> {
        self.tree
            .names
            .iter()
            .position(|key| key == name)
            .map(|i| self.node_at(i as i32))
    }
    fn epoch_at_10(&self) -> i32 {
        self.start_epoch_at_0c()
    }
    fn warehouse_variant(&mut self) -> bool {
        false
    }
    fn production_budget(&mut self, _: &Name) -> Budget {
        unreachable!()
    }
    fn production_category(&mut self, _: Node) -> i32 {
        unreachable!()
    }
    fn production_category_gate(&mut self, _: &Name) -> bool {
        unreachable!()
    }
    fn research_index(&mut self, name: &Name) -> i32 {
        self.tree.names.iter().position(|key| key == name).unwrap() as i32
    }
}

impl ResearchQueries for World<'_> {
    fn mode(&mut self) -> i32 {
        0
    }
    fn alternate_mode(&mut self) {
        unreachable!();
    }
    fn status8_measure(&mut self, _: &Name) -> Option<i32> {
        None
    }
    fn epoch_alternative(&mut self) -> Option<Name> {
        None
    }
    fn begin_epoch_fallback(&mut self) {
        unreachable!();
    }
    fn fallback_count(&self) -> usize {
        0
    }
    fn fallback_category(&mut self, _: usize) -> i32 {
        unreachable!()
    }
    fn fallback_name(&mut self, _: usize) -> Name {
        unreachable!()
    }
}

impl PlayerPlanner for World<'_> {
    fn ai_disabled(&self) -> bool {
        false
    }
    fn aip_word_at_0c(&self) -> u32 {
        2
    }
    fn refresh_aip(&mut self, _: &mut PlannerData) {
        unreachable!();
    }
    fn has_planner_context_at_34(&self) -> bool {
        self.completed_host_count(TC) > 0
    }
    fn find_planner_context(&mut self, _: bool) -> Option<i32> {
        None
    }
    fn planner_auxiliary_ready(&mut self) -> bool {
        false
    }
    fn planner_callback(&mut self, _: PlannerCallback) {}
    fn research_controls(&self) -> ResearchControls {
        ResearchControls {
            interval: RESEARCH_INTERVAL,
            epoch_delay: EPOCH_DELAY,
            measure_threshold: MEASURE_THRESHOLD,
        }
    }
    fn planner_random_interval(&self) -> [f32; 2] {
        RANDOM_INTERVAL
    }
    fn planner_word15(&mut self) -> u32 {
        self.game.source_birth.rng.next_word15()
    }
}

fn source_index(id: usize) -> Option<usize> {
    SOURCE_GAME_ID
        .iter()
        .position(|&value| value == Some(id))
        .or_else(|| u32::try_from(id).ok().and_then(rank::index))
}

pub(crate) fn category(index: usize) -> i32 {
    SOURCE_BRANCH.get(index).copied().unwrap_or(5)
}

fn quote(game: &Game, owner: u8, index: usize) -> [f32; 11] {
    if let Some(cost) = SOURCE_COST.get(index) {
        return *cost;
    }
    // Category 5 takes 008571B0's neutral first price stage. The subsequent
    // German natural-resource callback still applies to that same quote.
    let civ = game.civilizations[owner as usize];
    let mut cost = rank::COSTS[index - FIRST_RANK];
    if civ == 0 {
        for (slot, native) in crate::economy::NATIVE_IDS.into_iter().enumerate() {
            let source_slot = native as usize;
            let multiplier = crate::civ8_runtime::resource_cost_multiplier(civ, slot);
            cost[source_slot] =
                crate::civ8_bonus::natural_price53(cost[source_slot] as u16, 1.0, Some(multiplier))
                    as f32;
        }
    }
    cost
}

fn duration(index: usize) -> f32 {
    SOURCE_TIME.get(index).copied().unwrap_or_else(|| rank::DURATIONS[index - FIRST_RANK])
}

pub(crate) fn game_id(index: i32) -> Option<usize> {
    SOURCE_GAME_ID
        .get(usize::try_from(index).ok()?)
        .copied()
        .flatten()
}

pub(crate) fn captured_payment(game: &Game, owner: u8, index: i32) -> Option<[f32; 11]> {
    (game_id(index).is_some() || rank::family_at(index as usize).is_some())
        .then(|| quote(game, owner, index as usize))
}

pub(crate) fn captured_duration(index: i32) -> Option<f32> {
    (game_id(index).is_some() || rank::family_at(index as usize).is_some()).then(|| duration(index as usize))
}

impl DurableResearch {
    pub(crate) fn restore(
        owner: u8,
        manual: bool,
        flags: [u32; NODE_COUNT],
        clock: ControllerClock,
        active: Vec<ActiveResearch>,
        pending: Vec<PaidResearch>,
        ledger: CompletionLedger,
    ) -> Self {
        let names = std::array::from_fn(|index| {
            let name = SOURCE_NAMES.get(index).copied().unwrap_or_else(|| rank::NAMES[index - FIRST_RANK]);
            Name::new(name).unwrap()
        });
        let family_count = if manual { FAMILY_COUNT } else { 1 };
        let improvement_hosts = rank::HOSTS.map(|host| Name::new(host).unwrap());
        let families = ImprovementFamilies::from_init_rows((0..family_count).flat_map(|family| {
            rank::pair(family).map(|index| (index as i32, 5, Some(&improvement_hosts[family])))
        }));
        let elite_dependents = std::array::from_fn(|family| [rank::pair(family)[1] as i32]);
        let mut requirements = std::array::from_fn(|_| Vec::new());
        for family in 0..family_count {
            let [veteran, elite] = rank::pair(family);
            if flags[veteran] & 4 == 0 {
                requirements[elite].push(veteran as i32);
            }
        }
        let tree = Tree {
            flags,
            names,
            active,
            pending,
            requirements,
            improvement_hosts,
            family_count,
            elite_dependents,
            families,
            ledger,
        };
        let mut data = PlannerData::default();
        data.apply(&tree);
        Self {
            owner,
            manual,
            tree,
            clock,
            data,
        }
    }

    fn with_wallet<T>(&mut self, game: &mut Game, run: impl FnOnce(&mut World<'_>) -> T) -> T {
        let balance = snapshot_balance(game, self.owner);
        let mut world = World {
            game,
            tree: &mut self.tree,
            owner: self.owner,
            balance,
        };
        let result = run(&mut world);
        world.save_wallet();
        result
    }

    fn tick(&mut self, game: &mut Game) {
        let now = game.tick as f32 * DT;
        let balance = snapshot_balance(game, self.owner);
        let mut world = World {
            game,
            tree: &mut self.tree,
            owner: self.owner,
            balance,
        };
        let research_tick = ResearchFrame { delta: DT, immediate: false, precision: PRECISION };
        if self.manual {
            // Human orders use the native job lifecycle. They do not run the
            // research planner or draw from the source production RNG.
            self.clock.last_frame = now;
            tick_active(&mut world, research_tick);
            world.save_wallet();
            return;
        }
        let frame = PlayerFrame {
            now,
            planner_interval: PLANNER_INTERVAL,
            missing_context_delay: MISSING_CONTEXT_DELAY,
            research_tick,
        };
        // Every projected AIP name resolves and all clocks come from Game ticks.
        step(&mut world, Some((&mut self.clock, &mut self.data)), frame).unwrap();
        world.save_wallet();
    }

    fn refund_before_detaching(&mut self, game: &mut Game) {
        self.with_wallet(game, |world| {
            let active = std::mem::take(&mut world.tree.active);
            let pending = std::mem::take(&mut world.tree.pending);
            for row in active {
                world.start_accounts().refund(&row.cost);
            }
            for row in pending {
                world.start_accounts().refund(&row.cost);
            }
        });
    }

    pub(crate) fn supports(&self, index: usize) -> bool {
        index < if self.manual { NODE_COUNT } else { PREVIOUS_NODE_COUNT }
    }

    fn rank(&self, family: usize) -> u8 {
        let [veteran, elite] = rank::pair(family);
        if self.tree.flags[elite] & 4 != 0 {
            2
        } else if self.tree.flags[veteran] & 4 != 0 {
            1
        } else {
            0
        }
    }
}

impl Game {
    fn enable_source_research(&mut self, manual: bool) -> bool {
        let owner = self.local;
        if self.native_research.is_some()
            || self.net
            || self.bench != 0
            || owner != 0
            || owner as usize >= self.nplayers
            || self.players[owner as usize].epoch != 0
        {
            return false;
        }
        let mut flags = [0; NODE_COUNT];
        flags[0] = 0x07;
        for (index, id) in SOURCE_GAME_ID.into_iter().enumerate() {
            let Some(id) = id else { continue };
            let core = &crate::ee2_research::NODES[id];
            if Some(core.host) != SOURCE_GAME_HOST[index]
                || core.cost != SOURCE_COST[index][9]
                || core.time != SOURCE_TIME[index]
                || self.has_ordinary_research_job(owner, id)
            {
                return false;
            }
            if self.has_research(owner, id) {
                flags[index] = 0x07;
            }
        }
        let now = self.tick as f32 * DT;
        let clock = ControllerClock {
            last_frame: now,
            last_planner: now,
            next_random_deadline: now,
            previous_aip_word: 2,
            research: ResearchState {
                last_check: now,
                last_list_submit: now,
            },
        };
        let mut state = DurableResearch::restore(
            owner,
            manual,
            flags,
            clock,
            Vec::new(),
            Vec::new(),
            CompletionLedger::default(),
        );
        state.with_wallet(self, |world| unlock_successors(world, 0));
        self.civ_setup_open = false;
        self.native_research = Some(state);
        true
    }

    pub(crate) fn owns_source_research(&self, owner: u8, id: usize) -> bool {
        source_index(id).is_some_and(|index| self.native_research.as_ref()
            .is_some_and(|state| state.owner == owner && state.supports(index)))
    }

    pub(crate) fn source_research_busy(&self, owner: u8, id: usize) -> bool {
        let Some(index) = source_index(id) else {
            return false;
        };
        self.native_research
            .as_ref()
            .is_some_and(|state| state.owner == owner && state.supports(index)
                && state.tree.flags[index] & IN_PROGRESS != 0)
    }

    pub(crate) fn can_source_research(&self, owner: u8, host: DefId, id: usize) -> bool {
        let Some(index) = source_index(id) else {
            return false;
        };
        let epoch = self.players[owner as usize].epoch;
        self.native_research.as_ref().is_some_and(|state| {
            state.owner == owner
                && state.supports(index)
                && epoch + 1 < rank::RESET_SOURCE_EPOCH
                && host_matches(index, host, epoch)
                && state.tree.flags[index] == 0x03
                && Budget {
                    balance: snapshot_balance(self, owner),
                    cost: quote(self, owner, index),
                }
                .affordable()
        })
    }

    pub(crate) fn request_source_research(&mut self, owner: u8, id: usize) -> bool {
        let Some(index) = source_index(id) else {
            return false;
        };
        let Some(mut state) = self.native_research.take() else {
            return false;
        };
        let result = state.owner == owner && state.supports(index)
            && self.players[owner as usize].epoch + 1 < rank::RESET_SOURCE_EPOCH
            && state.with_wallet(self, |world| start(world, index as i32, -1));
        self.native_research = Some(state);
        result
    }

    pub(crate) fn tick_source_research(&mut self) {
        let Some(mut state) = self.native_research.take() else {
            return;
        };
        if self.players[state.owner as usize].epoch + 1 >= rank::RESET_SOURCE_EPOCH {
            state.refund_before_detaching(self);
            return;
        }
        state.tick(self);
        self.native_research = Some(state);
    }

    pub(crate) fn source_unit_rank(&self, owner: u8, def: DefId) -> u8 {
        let Some(state) = self.native_research.as_ref().filter(|state| state.owner == owner) else {
            return 0;
        };
        rank::unit_family(def, self.players[owner as usize].epoch).map_or(0, |family| state.rank(family))
    }

    pub(crate) fn source_light_artillery_production(&self, owner: u8, def: DefId) -> bool {
        const LIGHT_ARTILLERY: DefId = 22;
        if def != LIGHT_ARTILLERY {
            return false;
        }
        let epoch = self.players[owner as usize].epoch;
        if cfg!(feature = "ee2-native13") && !self.net && self.bench == 0 {
            // The UP tech tree assigns the entire LightArtillery1 class to the Mill.
            return crate::ee2_units::lookup_class(def, epoch).is_some();
        }
        epoch + 1 < rank::RESET_SOURCE_EPOCH
            && self.native_research.as_ref().is_some_and(|state| state.owner == owner && state.manual)
    }

    pub(crate) fn reset_source_rank_for_epoch(&mut self, owner: u8) {
        let Some(state) = self
            .native_research
            .as_mut()
            .filter(|state| state.owner == owner)
        else {
            return;
        };
        state.tree.ledger.current_epoch = [0; 3];
        if self.players[owner as usize].epoch + 2 != rank::RESET_SOURCE_EPOCH {
            return;
        }
        let mut state = self.native_research.take().unwrap();
        for family in 0..state.tree.family_count {
            rank::rescale_live_hp(self, owner, family, state.rank(family), 0);
        }
        state.refund_before_detaching(self);
    }

    fn cancel_source_research(&mut self, id: usize) -> bool {
        let Some(index) = source_index(id) else {
            return false;
        };
        let Some(mut state) = self.native_research.take() else {
            return false;
        };
        let result = state.owner == self.local && state.supports(index)
            && state.with_wallet(self, |world| cancel(world, index as i32, -1, false));
        self.native_research = Some(state);
        result
    }
}

fn host_matches(index: usize, def: DefId, epoch: u8) -> bool {
    if let Some(family) = rank::family_at(index) {
        rank::unit_family(def, epoch) == Some(family)
    } else {
        SOURCE_GAME_HOST[index] == Some(def)
    }
}

#[no_mangle]
pub extern "C" fn ai9_game_research_setup() -> u32 {
    game().is_some_and(|game| game.enable_source_research(false)) as u32
}

#[no_mangle]
pub extern "C" fn ai9_game_research_setup_manual() -> u32 {
    game().is_some_and(|game| game.enable_source_research(true)) as u32
}

#[no_mangle]
pub extern "C" fn ai9_game_research_mode() -> u32 {
    game().and_then(|game| game.native_research.as_ref()).map_or(0, |state| 1 + u32::from(state.manual))
}

/// 0/1 host presence (all/selected), 2/3 order admission (all/selected),
/// 4 current captured/base duration and 16..26 the source-order price words.
#[no_mangle]
pub extern "C" fn ai9_game_research_info(id: u32, field: u32) -> f32 {
    let Some(index) = source_index(id as usize) else { return -1.0; };
    let Some(game) = game() else { return -1.0; };
    let Some(state) = game.native_research.as_ref() else { return -1.0; };
    if !state.supports(index) { return -1.0; }
    let owner = state.owner;
    let epoch = game.players[owner as usize].epoch;
    match field {
        0..=3 => {
            let matches = game.ents[..game.high].iter().any(|entity| {
                entity.alive && entity.hp > 0.0 && entity.team == owner && entity.built >= 1.0
                    && (field & 1 == 0 || entity.sel)
                    && host_matches(index, entity.def, epoch)
                    && (field < 2 || game.can_source_research(owner, entity.def, id as usize))
            });
            u32::from(matches) as f32
        }
        4 => state.tree.active.iter().find(|job| job.index as usize == index)
            .map_or_else(|| duration(index), |job| job.duration),
        16..=26 => quote(game, owner, index)[(field - 16) as usize],
        _ => -1.0,
    }
}

#[no_mangle]
pub extern "C" fn ai9_game_research_elapsed(id: u32) -> f32 {
    let Some(index) = source_index(id as usize) else {
        return -1.0;
    };
    game()
        .and_then(|game| game.native_research.as_ref())
        .and_then(|state| {
            state
                .tree
                .active
                .iter()
                .find(|job| job.index == index as i32)
        })
        .map_or(-1.0, |job| job.elapsed)
}

#[no_mangle]
pub extern "C" fn ai9_game_research_status(id: u32) -> u32 {
    let Some(index) = source_index(id as usize) else {
        return 0;
    };
    let Some(state) = game().and_then(|game| game.native_research.as_ref()) else {
        return 0;
    };
    if !state.supports(index) {
        return 0;
    }
    if state
        .tree
        .active
        .iter()
        .any(|job| job.index == index as i32)
    {
        2
    } else if state
        .tree
        .pending
        .iter()
        .any(|job| job.index == index as i32)
    {
        4
    } else if state.tree.flags[index] & 0x04 != 0 {
        3
    } else {
        if state.tree.flags[index] & 2 == 0 {
            5
        } else {
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn ai9_game_research_cancel(id: u32) -> u32 {
    game().is_some_and(|game| game.cancel_source_research(id as usize)) as u32
}
