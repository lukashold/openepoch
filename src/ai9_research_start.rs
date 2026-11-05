//! Technology-tree start transaction at 006E3F50. Node indices refer to
//! the source tree table, independently of OpenEpoch production job IDs.

use crate::ai9_research_activation::{ActiveResearch, LocalStartUi, ResearchTiming, StartNotice};
use crate::ai9_research_debit::StartAccounts;
use crate::ai9_tech_tree::{Budget, Node, OBSOLETE_OR_DISABLED, PRODUCTION, REVEALED_AND_UNLOCKED};

pub(crate) const IN_PROGRESS: u32 = 0x20;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct PaidResearch {
    pub index: i32,
    pub cost: [f32; 11],
}

pub trait ResearchStart {
    fn node_at(&self, index: i32) -> Node;
    fn flags_at(&mut self, index: i32) -> &mut u32;
    fn category_at(&self, index: i32) -> i32;
    /// 005E3FF0(Player) followed by 005F05C0(index).
    fn prerequisite_count(&mut self, index: i32) -> i32;
    /// 00857430 dynamic costs and the ten balance words at Player+9C.
    fn research_budget(&mut self, node: Node) -> Budget;
    /// Production-only leaf 006E2F90 with the final argument fixed at 1.
    fn production_start_gate(&mut self, index: i32, context: i32) -> bool;
    fn start_epoch_at_0c(&self) -> i32;
    /// GameInfo+2C, following 00597050.
    fn maximum_epoch(&mut self) -> i32;
    /// Resolve via 00536670 and 00537C40. None is the -1 index result.
    fn next_main_flags(&mut self, next_epoch: i32) -> Option<u32>;
    /// 00857430 writes a fresh dynamic price into a zero-initialized vector.
    fn quote_start(&mut self, index: i32, cost: &mut [f32; 11]);
    /// Source account and optional statistics for the owner of this tree.
    fn start_accounts(&mut self) -> StartAccounts<'_>;
    fn active_research(&self) -> &[ActiveResearch];
    fn append_active_research(&mut self, active: ActiveResearch);
    /// Pending records at tree+38, independent of the active list at +18.
    fn pending_research(&mut self) -> &mut Vec<PaidResearch>;
    fn research_timing(&self, index: i32) -> ResearchTiming<'_>;
    fn local_start_ui(&self, _index: i32) -> Option<LocalStartUi> {
        None
    }
    fn research_started(&mut self, _notice: StartNotice) {}
}

pub(crate) fn meets_prerequisite(tree: &mut impl ResearchStart, node: Node) -> bool {
    node.prerequisite_index < 0 || tree.prerequisite_count(node.prerequisite_index) > 0
}

pub(crate) fn activate_paid(tree: &mut impl ResearchStart, paid: PaidResearch) {
    let index = paid.index;
    let active = tree.research_timing(index).active(paid);
    tree.append_active_research(active);
    if let Some(notice) = tree
        .local_start_ui(index)
        .and_then(|ui| ui.notice(index, tree.category_at(index)))
    {
        tree.research_started(notice);
    }
}

// 006E33E0 as called by the start transaction with paying = 1.
fn admitted(tree: &mut impl ResearchStart, node: Node, index: i32, context: i32) -> bool {
    if node.flags & REVEALED_AND_UNLOCKED != REVEALED_AND_UNLOCKED
        || node.flags & OBSOLETE_OR_DISABLED != 0
    {
        return false;
    }
    if node.flags & PRODUCTION != 0 {
        return tree.production_start_gate(index, context);
    }
    meets_prerequisite(tree, node) && tree.research_budget(node).affordable()
}

/// The caller supplies a valid index from the tree resolver.
pub fn start(tree: &mut impl ResearchStart, index: i32, context: i32) -> bool {
    let node = tree.node_at(index);
    if node.flags & IN_PROGRESS != 0 || !admitted(tree, node, index, context) {
        return false;
    }
    if matches!(tree.category_at(index), 2..=4) {
        let maximum = tree.maximum_epoch();
        let epoch = tree.start_epoch_at_0c();
        if epoch < maximum
            && tree
                .next_main_flags(epoch + 1)
                .is_some_and(|flags| flags & IN_PROGRESS != 0)
        {
            return false;
        }
    }

    let mut paid = PaidResearch {
        index,
        cost: [0.0; 11],
    };
    tree.quote_start(index, &mut paid.cost);
    tree.start_accounts().debit(&paid.cost);
    *tree.flags_at(index) |= IN_PROGRESS;

    if tree.category_at(index) == 5
        || tree
            .active_research()
            .iter()
            .all(|active| tree.category_at(active.index) == 5)
    {
        activate_paid(tree, paid);
    } else {
        tree.pending_research().push(paid);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai9_count::Precision53;
    use crate::ai9_planner::Name;
    use crate::ai9_player_step::PlannerCallback;
    use crate::ai9_research_activation::PlayerTimeScale;
    use crate::ai9_research_completion::{complete, CompletionLedger, ResearchCompletion};
    use crate::ai9_research_tick::{
        cancel, promote_pending, tick_active, ResearchFrame, ResearchTick,
    };
    use crate::ai9_research_unlock::{
        unlock_successors, ImprovementFamilies, ResearchUnlock, UnlockTrace,
    };
    use crate::native_rng9::NativeRng;

    include!("ai9_player_step_test_impl.rs");

    #[derive(Debug, PartialEq)]
    enum Call {
        Prerequisite(i32),
        Budget,
        ProductionGate(i32, i32),
        MaximumEpoch,
        NextMain(i32),
        Quote(i32),
        Debit,
        Activate,
        Append,
        Notice(StartNotice),
        Host(i32),
        Lost(i32),
        Cancel(i32),
        Upgrade(i32),
        Improve(i32),
        Completed(i32),
        Unlock(i32),
        Planner(PlannerCallback),
        FindContext(bool),
        Refresh,
    }

    struct Tree {
        flags: [u32; 3],
        categories: [i32; 3],
        epoch: i32,
        maximum_epoch: i32,
        next_main: Option<u32>,
        prerequisite: [i32; 3],
        production_gate: bool,
        active: Vec<ActiveResearch>,
        pending: Vec<PaidResearch>,
        activated: Vec<ActiveResearch>,
        price: [f32; 11],
        gate_cost: [f32; 11],
        balance: [f32; 10],
        spent: [f32; 9],
        calls: Vec<Call>,
        ui: Option<LocalStartUi>,
        source_epochs: [i16; 3],
        dependents: [Vec<i32>; 3],
        remaining: [Vec<i32>; 3],
        epoch_rows: [Vec<i32>; 16],
        civ_gate: [i32; 3],
        civ: i32,
        tech_threshold: i32,
        improvement_families: ImprovementFamilies,
        improvement_hosts: [Option<Name>; 3],
        improvement_hide: [Vec<i32>; 3],
        upgrades: [Option<i32>; 3],
        ledger: CompletionLedger,
        hosts: Vec<(i32, f32)>,
        names: [Name; 3],
        disabled: bool,
        aip_word: u32,
        has_context: bool,
        found_context: Option<i32>,
        rng: NativeRng,
        timing_scale: f32,
        timing_area: [f32; 1],
    }

    impl Tree {
        fn new(category: i32, active: Vec<i32>) -> Self {
            Self {
                flags: [0x103, IN_PROGRESS, IN_PROGRESS],
                categories: [category, 5, 1],
                epoch: 3,
                maximum_epoch: 15,
                next_main: None,
                prerequisite: [1; 3],
                production_gate: true,
                active: active
                    .into_iter()
                    .map(|index| ActiveResearch {
                        index,
                        host_handle: -1,
                        elapsed: 0.0,
                        duration: 20.0,
                        cost: [0.0; 11],
                    })
                    .collect(),
                pending: Vec::new(),
                activated: Vec::new(),
                price: [12.5, 0.0, 9.25, 13.0, 0.0, 4.0, 0.5, 7.0, 8.0, 11.25, -0.0],
                gate_cost: [0.0; 11],
                balance: [100.0; 10],
                spent: [0.0; 9],
                calls: Vec::new(),
                ui: None,
                source_epochs: [3; 3],
                dependents: std::array::from_fn(|_| Vec::new()),
                remaining: std::array::from_fn(|_| Vec::new()),
                epoch_rows: std::array::from_fn(|_| Vec::new()),
                civ_gate: [0; 3],
                civ: 0,
                tech_threshold: 6,
                improvement_families: ImprovementFamilies::default(),
                improvement_hosts: [None, None, None],
                improvement_hide: std::array::from_fn(|_| Vec::new()),
                upgrades: [None; 3],
                ledger: CompletionLedger::default(),
                hosts: Vec::new(),
                names: ["Main1", "Economic1_2", "Economic1_4"].map(|name| Name::new(name).unwrap()),
                disabled: false,
                aip_word: 2,
                has_context: true,
                found_context: None,
                rng: NativeRng::from_state(0x12345678),
                timing_scale: 0.5,
                timing_area: [0.75],
            }
        }
    }

    impl ResearchStart for Tree {
        fn node_at(&self, index: i32) -> Node {
            Node {
                key: index as u32,
                source_epoch: self.source_epochs[index as usize],
                flags: self.flags[index as usize],
                prerequisite_index: 17 + index,
            }
        }
        fn flags_at(&mut self, index: i32) -> &mut u32 {
            &mut self.flags[index as usize]
        }
        fn category_at(&self, index: i32) -> i32 {
            self.categories[index as usize]
        }
        fn prerequisite_count(&mut self, index: i32) -> i32 {
            self.calls.push(Call::Prerequisite(index));
            self.prerequisite[(index - 17) as usize]
        }
        fn research_budget(&mut self, _: Node) -> Budget {
            self.calls.push(Call::Budget);
            Budget {
                balance: self.balance,
                cost: self.gate_cost,
            }
        }
        fn production_start_gate(&mut self, index: i32, context: i32) -> bool {
            self.calls.push(Call::ProductionGate(index, context));
            self.production_gate
        }
        fn start_epoch_at_0c(&self) -> i32 {
            self.epoch
        }
        fn maximum_epoch(&mut self) -> i32 {
            self.calls.push(Call::MaximumEpoch);
            self.maximum_epoch
        }
        fn next_main_flags(&mut self, next_epoch: i32) -> Option<u32> {
            self.calls.push(Call::NextMain(next_epoch));
            self.next_main
        }
        fn quote_start(&mut self, index: i32, cost: &mut [f32; 11]) {
            assert_eq!(cost.map(f32::to_bits), [0; 11]);
            self.calls.push(Call::Quote(index));
            *cost = self.price;
        }
        fn start_accounts(&mut self) -> StartAccounts<'_> {
            if let Some(Call::Quote(index)) = self.calls.last() {
                assert_eq!(self.flags[*index as usize] & IN_PROGRESS, 0);
            }
            self.calls.push(Call::Debit);
            StartAccounts {
                balance: &mut self.balance,
                natural_spent: Some(&mut self.spent),
                precision: Precision53::NearestEven,
            }
        }
        fn active_research(&self) -> &[ActiveResearch] {
            &self.active
        }
        fn pending_research(&mut self) -> &mut Vec<PaidResearch> {
            self.calls.push(Call::Append);
            &mut self.pending
        }
        fn append_active_research(&mut self, active: ActiveResearch) {
            assert_ne!(self.flags[active.index as usize] & IN_PROGRESS, 0);
            self.calls.push(Call::Activate);
            self.active.push(active);
            self.activated.push(active);
        }
        fn research_timing(&self, _: i32) -> ResearchTiming<'_> {
            ResearchTiming {
                node_seconds: 15.0,
                player_scale: PlayerTimeScale::Technology(self.timing_scale),
                area_scales: &self.timing_area,
                minimum_area_scale: 0.05,
                precision: Precision53::NearestEven,
            }
        }
        fn local_start_ui(&self, _: i32) -> Option<LocalStartUi> {
            self.ui
        }
        fn research_started(&mut self, notice: StartNotice) {
            assert_eq!(self.active.last().unwrap().index, notice.index);
            self.calls.push(Call::Notice(notice));
        }
    }

    impl ResearchUnlock for Tree {
        fn epoch_nodes(&self, epoch: i32) -> &[i32] {
            &self.epoch_rows[epoch as usize]
        }
        fn completion_dependents(&self, index: i32) -> &[i32] {
            &self.dependents[index as usize]
        }
        fn remaining_requirements(&mut self, index: i32) -> &mut Vec<i32> {
            &mut self.remaining[index as usize]
        }
        fn node_civ_gate(&self, index: i32) -> i32 {
            self.civ_gate[index as usize]
        }
        fn civilization(&self) -> i32 {
            self.civ
        }
        fn main_threshold(&self) -> i32 {
            self.tech_threshold
        }
        fn improvement_families(&self) -> &ImprovementFamilies {
            &self.improvement_families
        }
        fn current_improvement_host(&self, index: i32) -> Option<&Name> {
            self.improvement_hosts[index as usize].as_ref()
        }
        fn improvement_hide_list(&self, index: i32) -> &[i32] {
            &self.improvement_hide[index as usize]
        }
    }

    impl ResearchCompletion for Tree {
        fn completion_ledger(&mut self) -> &mut CompletionLedger {
            &mut self.ledger
        }
        fn completion_upgrade(&self, index: i32) -> Option<i32> {
            self.upgrades[index as usize]
        }
        fn apply_completion_upgrade(&mut self, upgrade: i32) {
            self.calls.push(Call::Upgrade(upgrade));
        }
        fn apply_completion_improvement(&mut self, index: i32) {
            self.calls.push(Call::Improve(index));
        }
        fn research_completed_notice(&mut self, index: i32) {
            self.calls.push(Call::Completed(index));
        }
        fn successors_unlocked(&mut self, index: i32, _: UnlockTrace) {
            self.calls.push(Call::Unlock(index));
        }
    }

    impl ResearchTick for Tree {
        fn active_research_mut(&mut self) -> &mut Vec<ActiveResearch> {
            &mut self.active
        }
        fn research_host_hp(&mut self, handle: i32) -> Option<f32> {
            self.calls.push(Call::Host(handle));
            self.hosts
                .iter()
                .find_map(|&(id, hp)| (id == handle).then_some(hp))
        }
        fn pending_host_lost(&mut self, index: i32) {
            self.calls.push(Call::Lost(index));
        }
        fn research_cancelled_notice(&mut self, index: i32) {
            self.calls.push(Call::Cancel(index));
        }
    }

    #[test]
    fn start_rejections_do_not_requote_debit_or_modify_queue() {
        let mut running = Tree::new(2, vec![2]);
        running.flags[0] |= IN_PROGRESS;
        assert!(!start(&mut running, 0, -1));
        assert!(running.calls.is_empty());

        for flags in [0x1, 0x2, 0x13, 0x43] {
            let mut locked = Tree::new(2, vec![2]);
            locked.flags[0] = flags;
            assert!(!start(&mut locked, 0, -1));
            assert!(locked.calls.is_empty());
        }
        let mut missing = Tree::new(2, vec![2]);
        missing.prerequisite[0] = 0;
        assert!(!start(&mut missing, 0, -1));
        assert_eq!(missing.calls, [Call::Prerequisite(17)]);
        let mut short = Tree::new(2, vec![2]);
        short.gate_cost[9] = f32::from_bits(100.0_f32.to_bits() + 1);
        assert!(!start(&mut short, 0, -1));
        assert_eq!(short.calls, [Call::Prerequisite(17), Call::Budget]);
        assert_eq!(short.flags[0], 0x103);

        for category in 2..=4 {
            let mut blocked = Tree::new(category, vec![2]);
            blocked.next_main = Some(IN_PROGRESS | 0x100);
            assert!(!start(&mut blocked, 0, -1));
            assert_eq!(
                blocked.calls,
                [
                    Call::Prerequisite(17),
                    Call::Budget,
                    Call::MaximumEpoch,
                    Call::NextMain(4)
                ]
            );
            assert_eq!(blocked.flags[0], 0x103);
            assert!(blocked.activated.is_empty() && blocked.pending.is_empty());
        }
    }

    #[test]
    fn production_admission_uses_context_and_bypasses_research_checks() {
        let mut tree = Tree::new(5, vec![]);
        tree.flags[0] |= PRODUCTION;
        tree.production_gate = false;
        assert!(!start(&mut tree, 0, 67));
        assert_eq!(tree.calls, [Call::ProductionGate(0, 67)]);
        tree.calls.clear();
        tree.production_gate = true;
        assert!(start(&mut tree, 0, 67));
        assert_eq!(
            tree.calls,
            [
                Call::ProductionGate(0, 67),
                Call::Quote(0),
                Call::Debit,
                Call::Activate
            ]
        );
    }

    #[test]
    fn category_and_maximum_epoch_gate_control_the_next_main_lookup() {
        let mut at_limit = Tree::new(4, vec![]);
        at_limit.epoch = at_limit.maximum_epoch;
        at_limit.next_main = Some(IN_PROGRESS);
        assert!(start(&mut at_limit, 0, -1));
        assert_eq!(at_limit.calls[2], Call::MaximumEpoch);
        assert_eq!(at_limit.calls[3], Call::Quote(0));

        for flags in [None, Some(0x4)] {
            let mut epoch = Tree::new(2, vec![]);
            epoch.next_main = flags;
            assert!(start(&mut epoch, 0, -1));
            assert_eq!(epoch.calls[3], Call::NextMain(4));
            assert_eq!(epoch.calls[4], Call::Quote(0));
        }
        for category in [-1, 1, 5, 6] {
            let mut other = Tree::new(category, vec![]);
            other.next_main = Some(IN_PROGRESS);
            assert!(start(&mut other, 0, -1));
            assert_eq!(other.calls[2], Call::Quote(0));
        }
    }

    #[test]
    fn fresh_payment_precedes_running_flag_and_exact_paid_record_routing() {
        assert_eq!(std::mem::size_of::<PaidResearch>(), 48);
        assert_eq!(std::mem::size_of::<ActiveResearch>(), 60);
        for (category, active, pending) in [
            (1, vec![], false),
            (1, vec![1, 1], false),
            (1, vec![1, 2, 1], true),
            (1, vec![2], true),
            (5, vec![2], false),
        ] {
            let mut tree = Tree::new(category, active);
            assert!(start(&mut tree, 0, -1));
            assert_eq!(tree.flags[0], 0x123);
            assert_eq!(
                tree.balance,
                [87.0, 100.0, 91.0, 87.0, 100.0, 96.0, 99.0, 93.0, 92.0, 89.0]
            );
            assert_eq!(tree.spent, [13.0, 0.0, 9.0, 13.0, 0.0, 4.0, 1.0, 7.0, 8.0]);
            assert_eq!(
                tree.calls,
                [
                    Call::Prerequisite(17),
                    Call::Budget,
                    Call::Quote(0),
                    Call::Debit,
                    if pending {
                        Call::Append
                    } else {
                        Call::Activate
                    },
                ]
            );
            assert_eq!(
                (tree.pending.len(), tree.activated.len()),
                (pending as usize, !pending as usize)
            );
            let (index, cost) = if pending {
                let row = tree.pending[0];
                (row.index, row.cost)
            } else {
                let row = tree.activated[0];
                assert_eq!(
                    (row.host_handle, row.elapsed, row.duration),
                    (-1, 0.0, 5.625)
                );
                (row.index, row.cost)
            };
            assert_eq!(index, 0);
            assert_eq!(cost.map(f32::to_bits), tree.price.map(f32::to_bits));
            assert!(!start(&mut tree, 0, -1));
            assert_eq!(tree.calls.len(), 5);
        }
    }

    #[test]
    fn local_start_notice_follows_active_insertion_and_waits_for_pending() {
        for active in [vec![], vec![2]] {
            let mut tree = Tree::new(1, active);
            tree.ui = Some(LocalStartUi {
                definition_seconds: 15.0,
                suppressed: false,
                text_id: 17,
            });
            assert!(start(&mut tree, 0, -1));
            if tree.pending.is_empty() {
                assert_eq!(tree.calls[4], Call::Activate);
                assert_eq!(
                    tree.calls[5],
                    Call::Notice(StartNotice {
                        index: 0,
                        message_key: Some("msg_ig_techtree_advBegin"),
                        sound_key: "ui_research_start"
                    })
                );
            } else {
                assert_eq!(tree.calls[4..], [Call::Append]);
            }
        }
    }

    #[test]
    fn pending_lost_prerequisite_clears_running_after_pop_without_refund() {
        let mut tree = Tree::new(1, vec![]);
        tree.prerequisite[1] = 0;
        let lost = PaidResearch {
            index: 1,
            cost: tree.price,
        };
        let survivor = PaidResearch {
            index: 2,
            cost: tree.price.map(|cost| cost * 2.0),
        };
        tree.pending.extend([lost, survivor]);
        let balance = tree.balance;
        assert!(promote_pending(&mut tree));
        assert_eq!(tree.flags[1] & IN_PROGRESS, 0);
        assert_eq!(tree.balance, balance);
        assert_eq!(tree.active[0].index, 2);
        assert_eq!(
            tree.active[0].cost.map(f32::to_bits),
            survivor.cost.map(f32::to_bits)
        );
        assert!(tree.pending.is_empty());
        assert!(tree.calls.contains(&Call::Lost(1)));
        assert!(!tree.calls.contains(&Call::Cancel(1)));
        assert!(!tree.calls.contains(&Call::Debit));
    }

    #[test]
    fn direct_cancel_refunds_pending_but_active_host_mismatch_retains_the_row() {
        let mut tree = Tree::new(1, vec![1]);
        tree.active[0].host_handle = 7;
        tree.active[0].cost = tree.price;
        tree.pending.push(PaidResearch {
            index: 2,
            cost: tree.price,
        });
        assert!(cancel(&mut tree, 2, -1, false));
        assert_eq!(tree.balance[0], 113.0);
        assert_eq!(tree.spent[0], -13.0);
        assert!(tree.pending.is_empty());
        assert!(cancel(&mut tree, 1, 99, false));
        assert_eq!(tree.active[0].host_handle, 7);
        assert_eq!(tree.balance[0], 126.0);
        assert_eq!(tree.flags[1] & IN_PROGRESS, 0);
        assert!(!cancel(&mut tree, 1, 7, false));
        assert_eq!(tree.balance[0], 126.0);
    }

    #[test]
    fn completion_flags_dependencies_history_counters_and_effect_order() {
        let mut tree = Tree::new(3, vec![0]);
        tree.flags = [0x33, 0x23, 0x23];
        tree.categories = [3, 2, 5];
        tree.source_epochs[1] = 2;
        tree.dependents[0] = vec![1, 2];
        tree.remaining[1] = vec![5, 0, 9, 7];
        tree.remaining[2] = vec![0, 19];
        tree.upgrades[0] = Some(221);
        assert!(complete(&mut tree, 0));
        assert_eq!(tree.flags[0], 0x17);
        assert_eq!(tree.remaining[1], [5, 7, 9]);
        assert_eq!(tree.remaining[2], [19]);
        assert_eq!(tree.ledger.current_epoch, [0, 1, 0]);
        assert_eq!(tree.ledger.total, [0, 1, 0]);
        assert_eq!(
            tree.calls,
            [
                Call::Upgrade(221),
                Call::Completed(0),
                Call::MaximumEpoch,
                Call::Unlock(0)
            ]
        );
        assert!(!complete(&mut tree, 0));
        assert!(complete(&mut tree, 1));
        assert!(complete(&mut tree, 2));
        assert_eq!(tree.ledger.history, [0, 1, 2]);
        assert_eq!(tree.ledger.current_epoch, [0, 1, 0]);
        assert_eq!(tree.ledger.total, [1, 1, 0]);
        assert_eq!(
            tree.calls[7..],
            [
                Call::Improve(2),
                Call::Completed(2),
                Call::MaximumEpoch,
                Call::Unlock(2)
            ]
        );
        for flags in [0x22, 0x21, 0x63, 0x27, 0x2b] {
            tree.flags[0] = flags;
            assert!(!complete(&mut tree, 0));
            assert_eq!(tree.flags[0], flags);
        }
    }

    #[test]
    fn main_unlock_tally_counts_only_finished_techs_and_applies_its_distinct_gate() {
        let mut tree = Tree::new(2, vec![]);
        tree.flags = [0x07, 0x23, 0x10];
        tree.categories = [2, 4, 1];
        tree.source_epochs = [1, 1, 2];
        tree.epoch_rows[1] = vec![0, 1];
        tree.epoch_rows[2] = vec![2];
        tree.tech_threshold = 2;
        tree.civ_gate[2] = 16;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
        tree.flags[1] |= 0x04;
        tree.categories[1] = 5;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
        tree.categories[1] = 4;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, [2]);
        assert_eq!(tree.flags[2], 0x13);
        tree.flags[2] = 0x50;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
        tree.flags[2] = 0x10;
        tree.maximum_epoch = 1;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
    }

    #[test]
    fn main_completion_scans_its_epoch_and_improvements_cascade_in_order() {
        let mut tree = Tree::new(1, vec![]);
        tree.categories = [1, 5, 5];
        tree.flags = [0x07, 0, 0x27];
        tree.source_epochs = [2, 2, 1];
        tree.epoch_rows[2] = vec![0, 1];
        tree.civ_gate[1] = 16;
        let host = Name::new("LightInfantry").unwrap();
        tree.improvement_families =
            ImprovementFamilies::from_init_rows([(1, 5, Some(&host)), (2, 5, Some(&host))]);
        tree.improvement_hosts[1] = Some(host);
        tree.improvement_hide[1] = vec![2];
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
        assert_eq!(tree.flags[2], 0x27);
        tree.civ = 5;
        tree.remaining[1] = vec![9];
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, []);
        tree.remaining[1].clear();
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, [1]);
        assert_eq!(tree.flags, [0x07, 0x03, 0x34]);
        tree.flags = [0x07, 0, 0x03];
        tree.source_epochs[2] = 2;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, [1]);
        assert_eq!(tree.flags[2], 0x02);
    }

    #[test]
    fn ordinary_dependents_keep_source_civilization_and_obsolete_gates() {
        let mut tree = Tree::new(3, vec![]);
        tree.categories = [3, 4, 4];
        tree.flags = [0x07, 0, 0x10];
        tree.dependents[0] = vec![2, 1];
        tree.civ_gate = [0, 7, 16];
        assert!(unlock_successors(&mut tree, 0).unlocked.is_empty());
        tree.civ = 6;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, [1]);
        tree.flags[1] = 0;
        tree.flags[2] = 0;
        tree.civ = -1;
        assert_eq!(unlock_successors(&mut tree, 0).unlocked, [2, 1]);
    }

    #[test]
    fn reverse_tick_caps_elapsed_completes_then_promotes_for_the_next_tick() {
        let mut tree = Tree::new(2, vec![0, 1]);
        tree.flags = [0x23; 3];
        tree.active[0].elapsed = 19.75;
        tree.active[1].elapsed = 19.75;
        tree.pending.push(PaidResearch {
            index: 2,
            cost: tree.price,
        });
        let frame = ResearchFrame {
            delta: 1.0,
            immediate: false,
            precision: Precision53::NearestEven,
        };
        tick_active(&mut tree, frame);
        assert_eq!(tree.ledger.history, [1, 0]);
        assert_eq!(tree.active.len(), 1);
        assert_eq!((tree.active[0].index, tree.active[0].elapsed), (2, 0.0));
        tick_active(&mut tree, frame);
        assert_eq!(tree.active[0].elapsed, 1.0);
        tick_active(
            &mut tree,
            ResearchFrame {
                immediate: true,
                ..frame
            },
        );
        assert_eq!(tree.ledger.history, [1, 0, 2]);
        assert!(tree.active.is_empty());
    }

    #[test]
    fn lost_host_and_locked_improvement_refund_while_a_live_host_keeps_its_elapsed() {
        let mut tree = Tree::new(2, vec![0, 1, 2]);
        tree.flags = [0x23, 0x33, 0x23];
        tree.active[0].host_handle = 73;
        tree.active[1].cost = tree.price;
        tree.active[2].host_handle = 77;
        tree.active[2].cost = tree.price;
        tree.hosts.push((73, 1.0));
        let frame = ResearchFrame {
            delta: 1.0,
            immediate: true,
            precision: Precision53::NearestEven,
        };
        tick_active(&mut tree, frame);
        assert_eq!(tree.active.len(), 1);
        assert_eq!((tree.active[0].index, tree.active[0].elapsed), (0, 0.0));
        assert_eq!(tree.flags, [0x23, 0x13, 0x03]);
        assert_eq!(tree.balance[0], 126.0);
        assert!(tree.ledger.history.is_empty());
        assert_eq!(
            tree.calls,
            [Call::Host(77), Call::Debit, Call::Debit, Call::Host(73)]
        );
    }
}
