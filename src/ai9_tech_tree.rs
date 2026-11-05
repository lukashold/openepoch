//! 004E7B80 for the technology dispatcher's (0, 0, status, 0) arguments,
//! plus 004E7EA0. The tree supplies source nodes and the remaining engine
//! transactions through explicit callbacks.
use crate::ai9_planner::Name;
use crate::ai9_research_start::{self, ResearchStart, IN_PROGRESS};

pub(crate) const REVEALED_AND_UNLOCKED: u32 = 0x03;
const UNLOCKED: u32 = 0x02;
const COMPLETED: u32 = 0x04;
pub(crate) const PRODUCTION: u32 = 0x08;
pub(crate) const OBSOLETE_OR_DISABLED: u32 = 0x50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub key: u32,
    pub source_epoch: i16,
    pub flags: u32,
    /// Signed node word at +20, passed to the 005F05C0 lookup.
    pub prerequisite_index: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub balance: [f32; 10],
    pub cost: [f32; 11],
}

impl Budget {
    pub(crate) fn affordable(&self) -> bool {
        // 004E7C9D and 006E2E8D use the same ordered comparison against
        // exactly ten balance words. The eleventh cost remains outside it.
        !self
            .balance
            .iter()
            .zip(self.cost.iter())
            .any(|(balance, cost)| balance < cost)
    }
}

pub trait TechnologyTree: ResearchStart {
    /// Lookup 006E2410. A key identifies the returned node in this tree.
    fn node(&mut self, name: &Name) -> Option<Node>;
    fn epoch_at_10(&self) -> i32;
    /// 004E3720("CityCenterWarehouse"), checked for Warehouse only.
    fn warehouse_variant(&mut self) -> bool;
    /// 008573E0 cost and the ten balance words at Player+9C.
    fn production_budget(&mut self, name: &Name) -> Budget;
    fn production_category(&mut self, node: Node) -> i32;
    /// 0086C200 name lookup followed by 005E3DB0.
    fn production_category_gate(&mut self, name: &Name) -> bool;
    /// 006E21E0 reads the same name entry as 006E2410 and returns its index.
    fn research_index(&mut self, name: &Name) -> i32;
}

pub fn check(tree: &mut impl TechnologyTree, name: &Name, status: &mut Option<i32>) -> bool {
    if name.as_str().eq_ignore_ascii_case("Warehouse") && tree.warehouse_variant() {
        *status = Some(7);
        return false;
    }
    let Some(node) = tree.node(name) else {
        // 004E7BE2 branches to a false return without storing the status.
        return false;
    };
    let production = node.flags & PRODUCTION != 0;
    if !production
        && (i32::from(node.source_epoch) < tree.epoch_at_10()
            || node.flags & (IN_PROGRESS | COMPLETED) != 0)
    {
        return true;
    }
    if node.flags & REVEALED_AND_UNLOCKED != REVEALED_AND_UNLOCKED
        || node.flags & OBSOLETE_OR_DISABLED != 0
    {
        *status = Some(5);
        return false;
    }

    if production {
        if !tree.production_budget(name).affordable() {
            *status = Some(4);
            return false;
        }
        if matches!(tree.production_category(node), 1..=3 | 5..=11)
            && !tree.production_category_gate(name)
        {
            *status = Some(3);
            return false;
        }
    } else {
        if !ai9_research_start::meets_prerequisite(tree, node) {
            *status = Some(8);
            return false;
        }
        if !tree.research_budget(node).affordable() {
            *status = Some(4);
            return false;
        }
    }
    true
}

pub fn submit(tree: &mut impl TechnologyTree, name: &Name, flag: &mut u8) -> i32 {
    *flag = 0;
    let Some(node) = tree.node(name) else {
        return 5;
    };
    if node.flags & PRODUCTION != 0 {
        return 5;
    }
    if i32::from(node.source_epoch) < tree.epoch_at_10() {
        return 0;
    }
    if node.flags & UNLOCKED == 0 {
        return 5;
    }
    if node.flags & (COMPLETED | IN_PROGRESS) != 0 {
        *flag = u8::from(node.flags & COMPLETED == 0);
        return 0;
    }
    if !tree.research_budget(node).affordable() {
        return 4;
    }
    let index = tree.research_index(name);
    // 004E7EA0 sets the output byte regardless of 006E3F50's result.
    ai9_research_start::start(tree, index, -1);
    *flag = 1;
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai9_count::Precision53;
    use crate::ai9_planner::{AipRead, BuildRow, Context, PlannerData};
    use crate::ai9_research_activation::{ActiveResearch, PlayerTimeScale, ResearchTiming};
    use crate::ai9_research_debit::StartAccounts;
    use crate::ai9_research_dispatch::{dispatch, Pass, ResearchQueries, ResearchState, Timing};
    use crate::ai9_research_start::PaidResearch;

    struct Tree {
        names: Vec<Name>,
        nodes: Vec<(Name, Node)>,
        looked_up: Vec<String>,
        gates: usize,
        affordability: usize,
        started: Vec<String>,
        budget: Budget,
        category: i32,
        warehouse: bool,
        prerequisite_count: i32,
        third_check_shortage: bool,
        start_quotes: usize,
    }

    impl Tree {
        fn new(records: &[(&str, u32)]) -> Self {
            let mut tree = Self {
                names: Vec::new(),
                nodes: Vec::new(),
                looked_up: Vec::new(),
                gates: 0,
                affordability: 0,
                started: Vec::new(),
                budget: Budget {
                    balance: [0.0; 10],
                    cost: [0.0; 11],
                },
                category: 0,
                warehouse: false,
                prerequisite_count: 1,
                third_check_shortage: false,
                start_quotes: 0,
            };
            for (key, &(text, flags)) in records.iter().enumerate() {
                let name = Name::new(text).unwrap();
                tree.names.push(name.clone());
                if flags != 0 {
                    tree.nodes.push((
                        name,
                        Node {
                            key: key as u32,
                            source_epoch: 1,
                            flags,
                            prerequisite_index: 17,
                        },
                    ));
                }
            }
            tree
        }
    }

    impl TechnologyTree for Tree {
        fn node(&mut self, name: &Name) -> Option<Node> {
            self.looked_up.push(name.as_str().to_owned());
            self.nodes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, node)| *node)
        }
        fn epoch_at_10(&self) -> i32 {
            1
        }
        fn warehouse_variant(&mut self) -> bool {
            self.warehouse
        }
        fn production_budget(&mut self, _: &Name) -> Budget {
            self.budget
        }
        fn production_category(&mut self, _: Node) -> i32 {
            self.category
        }
        fn production_category_gate(&mut self, _: &Name) -> bool {
            false
        }
        fn research_index(&mut self, name: &Name) -> i32 {
            self.nodes.iter().position(|(key, _)| key == name).unwrap() as i32
        }
    }

    impl ResearchStart for Tree {
        fn node_at(&self, index: i32) -> Node {
            self.nodes[index as usize].1
        }
        fn flags_at(&mut self, index: i32) -> &mut u32 {
            &mut self.nodes[index as usize].1.flags
        }
        fn category_at(&self, _: i32) -> i32 {
            5
        }
        fn prerequisite_count(&mut self, index: i32) -> i32 {
            assert_eq!(index, 17);
            self.gates += 1;
            self.prerequisite_count
        }
        fn research_budget(&mut self, _: Node) -> Budget {
            self.affordability += 1;
            let mut budget = self.budget;
            if self.affordability == 3 && self.third_check_shortage {
                budget.cost[9] = budget.balance[9] + 1.0;
            }
            budget
        }
        fn production_start_gate(&mut self, _: i32, _: i32) -> bool {
            unreachable!()
        }
        fn start_epoch_at_0c(&self) -> i32 {
            unreachable!()
        }
        fn maximum_epoch(&mut self) -> i32 {
            unreachable!()
        }
        fn next_main_flags(&mut self, _: i32) -> Option<u32> {
            unreachable!()
        }
        fn quote_start(&mut self, _: i32, cost: &mut [f32; 11]) {
            self.start_quotes += 1;
            *cost = self.budget.cost;
        }
        fn start_accounts(&mut self) -> StartAccounts<'_> {
            StartAccounts {
                balance: &mut self.budget.balance,
                natural_spent: None,
                precision: Precision53::NearestEven,
            }
        }
        fn active_research(&self) -> &[ActiveResearch] {
            unreachable!()
        }
        fn append_active_research(&mut self, active: ActiveResearch) {
            self.started
                .push(self.nodes[active.index as usize].0.as_str().to_owned());
        }
        fn pending_research(&mut self) -> &mut Vec<PaidResearch> {
            unreachable!()
        }
        fn research_timing(&self, _: i32) -> ResearchTiming<'_> {
            ResearchTiming {
                node_seconds: 15.0,
                player_scale: PlayerTimeScale::Neutral,
                area_scales: &[],
                minimum_area_scale: 0.05,
                precision: Precision53::NearestEven,
            }
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

    impl ResearchQueries for Tree {
        fn mode(&mut self) -> i32 {
            0
        }
        fn alternate_mode(&mut self) {
            panic!("Unexpected alternate mode");
        }
        fn status8_measure(&mut self, _: &Name) -> Option<i32> {
            panic!("Unexpected measure");
        }
        fn epoch_alternative(&mut self) -> Option<Name> {
            panic!("Unexpected alternative");
        }
        fn begin_epoch_fallback(&mut self) {
            panic!("Unexpected fallback");
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

    fn dispatch_tree(
        tree: &mut Tree,
    ) -> (
        Result<Pass, crate::ai9_research_dispatch::InvalidInput>,
        ResearchState,
    ) {
        let mut data = PlannerData::default();
        data.apply(tree);
        let mut state = ResearchState {
            last_check: 0.0,
            last_list_submit: 0.0,
        };
        let timing = Timing {
            now: 5.0,
            check_interval: 5.0,
            epoch_delay: 20,
            measure_threshold: 20,
            precision: Precision53::NearestEven,
        };
        let result = dispatch(&mut state, &mut data, timing, tree);
        (result, state)
    }

    #[test]
    fn real_leaves_continue_past_missing_node_after_completed_then_stop_at_running() {
        let mut tree = Tree::new(&[
            ("Done", 0x07),
            ("Unknown", 0),
            ("Running", 0x23),
            ("Later", 0x03),
        ]);
        let (result, state) = dispatch_tree(&mut tree);
        assert_eq!(result, Ok(Pass::ListStopped(0)));
        assert_eq!(state.last_list_submit, 5.0);
        assert_eq!(
            tree.looked_up,
            ["Done", "Done", "Unknown", "Running", "Running"]
        );
        assert!(tree.started.is_empty());
        assert_eq!((tree.gates, tree.affordability), (0, 0));
    }

    #[test]
    fn fresh_node_runs_three_resource_checks_and_quotes_the_payment_once() {
        let mut tree = Tree::new(&[("Agriculture", 0x03)]);
        assert_eq!(dispatch_tree(&mut tree).0, Ok(Pass::ListStopped(0)));
        assert_eq!(tree.started, ["Agriculture"]);
        assert_eq!(
            (tree.gates, tree.affordability, tree.start_quotes),
            (2, 3, 1)
        );
    }

    #[test]
    fn submit_sets_its_flag_even_when_start_transaction_rejects() {
        let mut tree = Tree::new(&[("Agriculture", 0x03)]);
        tree.third_check_shortage = true;
        assert_eq!(dispatch_tree(&mut tree).0, Ok(Pass::ListStopped(0)));
        assert_eq!(
            (tree.gates, tree.affordability, tree.start_quotes),
            (2, 3, 0)
        );
        assert_eq!(tree.nodes[0].1.flags, 0x03);
        assert!(tree.started.is_empty());
    }

    #[test]
    fn missing_node_preserves_status_and_submit_resets_the_flag() {
        let mut tree = Tree::new(&[("Unknown", 0), ("Locked", 0x21), ("Unit", 0x0b)]);
        let mut status = Some(8);
        assert!(!check(
            &mut tree,
            &Name::new("Unknown").unwrap(),
            &mut status
        ));
        assert_eq!(status, Some(8));
        assert_eq!(
            dispatch_tree(&mut tree).0,
            Err(crate::ai9_research_dispatch::InvalidInput::MissingCheckStatus)
        );
        for text in ["Unknown", "Locked", "Unit"] {
            let mut flag = 9;
            assert_eq!(submit(&mut tree, &Name::new(text).unwrap(), &mut flag), 5);
            assert_eq!(flag, 0);
        }
    }

    #[test]
    fn production_check_reads_ten_balance_words_and_the_selected_category_gate() {
        let mut tree = Tree::new(&[("Unit", 0x0b)]);
        let name = Name::new("Unit").unwrap();
        let mut status = None;
        tree.budget.cost[9] = 1.0;
        tree.budget.cost[10] = 1000.0;
        assert!(!check(&mut tree, &name, &mut status));
        assert_eq!(status, Some(4));
        tree.budget.balance[9] = 1.0;
        tree.category = 4;
        assert!(check(&mut tree, &name, &mut status));
        assert_eq!(status, Some(4));
        tree.category = 5;
        assert!(!check(&mut tree, &name, &mut status));
        assert_eq!(status, Some(3));
    }

    #[test]
    fn warehouse_rejection_precedes_lookup_and_tech_gates_keep_separate_codes() {
        let mut tree = Tree::new(&[("Tech", 0x03)]);
        let mut status = None;
        tree.warehouse = true;
        assert!(!check(
            &mut tree,
            &Name::new("warehouse").unwrap(),
            &mut status
        ));
        assert_eq!(status, Some(7));
        assert!(tree.looked_up.is_empty());
        let name = Name::new("Tech").unwrap();
        for count in [-1, 0] {
            tree.prerequisite_count = count;
            assert!(!check(&mut tree, &name, &mut status));
            assert_eq!(status, Some(8));
        }
        assert_eq!(tree.gates, 2);
        assert_eq!(tree.affordability, 0);

        tree.nodes[0].1.prerequisite_index = -2;
        tree.budget.cost[9] = 1.0;
        tree.budget.cost[10] = 1000.0;
        tree.budget.balance[9] = f32::from_bits(1.0_f32.to_bits() - 1);
        assert!(!check(&mut tree, &name, &mut status));
        assert_eq!(status, Some(4));
        assert_eq!(tree.gates, 2);
        let mut flag = 9;
        assert_eq!(submit(&mut tree, &name, &mut flag), 4);
        assert_eq!(flag, 0);
        assert!(tree.started.is_empty());
        tree.budget.balance[9] = 1.0;
        assert!(check(&mut tree, &name, &mut status));
        assert_eq!(submit(&mut tree, &name, &mut flag), 0);
        assert_eq!(flag, 1);
        assert_eq!(tree.started, ["Tech"]);
    }
}
