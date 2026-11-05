use crate::ai9_planner::{AipRead, BuildRow, Context, PlannerData};
use crate::ai9_player_step::{
    step, ControllerClock, FramePass, PlayerFrame, PlayerPlanner, ResearchControls,
};
use crate::ai9_research_dispatch::{Pass, ResearchQueries, ResearchState};
use crate::ai9_tech_tree::TechnologyTree;

impl TechnologyTree for Tree {
    fn node(&mut self, name: &Name) -> Option<Node> {
        self.names
            .iter()
            .position(|key| key == name)
            .map(|index| self.node_at(index as i32))
    }
    fn epoch_at_10(&self) -> i32 {
        self.epoch
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
        self.names.iter().position(|key| key == name).unwrap() as i32
    }
}

impl ResearchQueries for Tree {
    fn mode(&mut self) -> i32 {
        0
    }
    fn alternate_mode(&mut self) {
        unreachable!()
    }
    fn status8_measure(&mut self, _: &Name) -> Option<i32> {
        None
    }
    fn epoch_alternative(&mut self) -> Option<Name> {
        None
    }
    fn begin_epoch_fallback(&mut self) {
        unreachable!()
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

impl AipRead for Tree {
    fn registry(&self, _: Context) -> &[Name] {
        &[]
    }
    fn build_rows(&self, _: &Name) -> &[BuildRow] {
        &[]
    }
    fn tech_names(&self) -> &[Name] {
        &self.names
    }
}

impl PlayerPlanner for Tree {
    fn ai_disabled(&self) -> bool {
        self.disabled
    }
    fn aip_word_at_0c(&self) -> u32 {
        self.aip_word
    }
    fn refresh_aip(&mut self, data: &mut PlannerData) {
        self.calls.push(Call::Refresh);
        data.apply(self);
    }
    fn has_planner_context_at_34(&self) -> bool {
        self.has_context
    }
    fn find_planner_context(&mut self, alternate: bool) -> Option<i32> {
        self.calls.push(Call::FindContext(alternate));
        if alternate {
            self.found_context
        } else {
            None
        }
    }
    fn planner_auxiliary_ready(&mut self) -> bool {
        true
    }
    fn planner_callback(&mut self, callback: PlannerCallback) {
        if matches!(callback, PlannerCallback::SetContext(_)) {
            self.has_context = true;
        }
        self.calls.push(Call::Planner(callback));
    }
    fn planner_word15(&mut self) -> u32 {
        self.rng.next_word15()
    }
    fn research_controls(&self) -> ResearchControls {
        ResearchControls {
            interval: 1.0,
            epoch_delay: 20,
            measure_threshold: 20,
        }
    }
    fn planner_random_interval(&self) -> [f32; 2] {
        [6.0, 11.0]
    }
}

fn clock() -> ControllerClock {
    ControllerClock {
        last_frame: 0.0,
        last_planner: 0.0,
        next_random_deadline: 0.0,
        previous_aip_word: 2,
        research: ResearchState {
            last_check: 0.0,
            last_list_submit: 0.0,
        },
    }
}

fn frame(now: f32) -> PlayerFrame {
    let precision = Precision53::NearestEven;
    PlayerFrame {
        research_tick: ResearchFrame {
            delta: 1.0,
            immediate: false,
            precision,
        },
        now,
        planner_interval: 1.0,
        missing_context_delay: 60.0,
    }
}

#[test]
fn ai9_player_step_two_peers_run_two_source_research_jobs() {
    let mut peers = std::array::from_fn::<_, 2, _>(|_| {
        let mut tree = Tree::new(1, vec![]);
        tree.flags = [0x07, 0x03, 0x03];
        tree.categories = [1, 3, 3];
        tree.epoch = 1;
        tree.source_epochs = [1; 3];
        tree.price = [0.0; 11];
        tree.price[9] = 8.0;
        tree.gate_cost = tree.price;
        tree.balance[9] = 16.0;
        tree.timing_scale = 1.0;
        tree.timing_area = [1.0];
        let mut data = PlannerData::default();
        data.apply(&tree);
        (tree, clock(), data)
    });
    for tick in 1..=34 {
        let results = peers.each_mut().map(|(tree, clock, data)| {
            tree.calls.clear();
            step(tree, Some((clock, data)), frame(tick as f32)).unwrap()
        });
        assert_eq!(results[0], results[1]);
        let [(a, ca, da), (b, cb, db)] = &peers;
        assert_eq!((ca, da), (cb, db));
        assert_eq!(
            (&a.active, &a.pending, &a.calls),
            (&b.active, &b.pending, &b.calls)
        );
        assert_eq!((a.flags, a.balance, a.rng), (b.flags, b.balance, b.rng));
        assert_eq!(a.ledger.history, b.ledger.history);
        if tick == 1 {
            assert_eq!(results[0], FramePass::Research(Pass::ListStopped(0)));
            assert_eq!(
                (a.active[0].index, a.active[0].elapsed, a.active[0].duration),
                (1, 0.0, 15.0)
            );
            assert_eq!(a.balance[9], 8.0);
            assert_eq!(
                a.rng.state(),
                0x12345678_u32
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223)
            );
        } else if tick == 16 {
            assert_eq!(a.ledger.history, [1]);
            assert_eq!((a.active[0].index, a.active[0].elapsed), (2, 0.0));
            assert_eq!(a.balance[9], 0.0);
            assert_eq!(
                a.calls[0..3],
                [Call::Completed(1), Call::MaximumEpoch, Call::Unlock(1)]
            );
        } else if tick >= 31 {
            assert_eq!(a.ledger.history, [1, 2]);
            assert_eq!(a.ledger.total, [0, 2, 0]);
            assert!(a.active.is_empty());
        }
    }
}

#[test]
fn ai9_player_step_disabled_and_missing_context_keep_source_clock_order() {
    let mut tree = Tree::new(1, vec![]);
    let mut data = PlannerData::default();
    let mut clock = clock();
    tree.disabled = true;
    assert_eq!(
        step(&mut tree, Some((&mut clock, &mut data)), frame(1.0)),
        Ok(FramePass::AiDisabled)
    );
    assert_eq!(clock.last_frame, 1.0);
    assert!(tree.calls.is_empty());
    tree.disabled = false;
    tree.aip_word = 1;
    tree.has_context = false;
    assert_eq!(
        step(&mut tree, Some((&mut clock, &mut data)), frame(2.0)),
        Ok(FramePass::MissingPlannerContext)
    );
    assert_eq!(clock.last_planner, 62.0);
    assert_eq!(
        tree.calls,
        [
            Call::Refresh,
            Call::Planner(PlannerCallback::Update4d0e20),
            Call::Planner(PlannerCallback::Update4ee090(1.0)),
            Call::FindContext(false),
            Call::FindContext(true),
            Call::Planner(PlannerCallback::Finish4ff3b0)
        ]
    );
    tree.calls.clear();
    assert_eq!(
        step(&mut tree, Some((&mut clock, &mut data)), frame(62.5)),
        Ok(FramePass::PlannerNotDue)
    );
    assert!(!tree
        .calls
        .iter()
        .any(|entry| matches!(entry, Call::FindContext(_))));
    tree.calls.clear();
    tree.found_context = Some(41);
    assert_eq!(
        step(&mut tree, Some((&mut clock, &mut data)), frame(63.0)),
        Ok(FramePass::Research(Pass::ListStopped(0)))
    );
    assert_eq!(clock.previous_aip_word, 1);
    assert_eq!(
        &tree.calls[3..10],
        &[
            Call::FindContext(false),
            Call::FindContext(true),
            Call::Planner(PlannerCallback::SetContext(41)),
            Call::Planner(PlannerCallback::AipChanged4e5150),
            Call::Planner(PlannerCallback::AipChanged4cdde0),
            Call::Planner(PlannerCallback::Before4e7400),
            Call::Planner(PlannerCallback::Before4e9830)
        ]
    );
}
