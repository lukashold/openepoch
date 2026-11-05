//! The 006E50F0(index,1,1,0) completion path used by the active research tick.

use crate::ai9_research_start::IN_PROGRESS;
use crate::ai9_research_unlock::{unlock_successors, ResearchUnlock, UnlockTrace};
use crate::ai9_tech_tree::{PRODUCTION, REVEALED_AND_UNLOCKED};

#[derive(Default)]
pub struct CompletionLedger {
    /// Technology tree +48/+4C.
    pub history: Vec<i32>,
    /// Technology tree +58 and +68, indexed by category minus two.
    pub current_epoch: [i32; 3],
    pub total: [i32; 3],
}

pub trait ResearchCompletion: ResearchUnlock {
    fn completion_ledger(&mut self) -> &mut CompletionLedger;
    /// Original node definition+20, absent when negative.
    fn completion_upgrade(&self, index: i32) -> Option<i32>;
    /// 0053C540 with the upgrade row and owner of this tree.
    fn apply_completion_upgrade(&mut self, upgrade: i32);
    /// Category 5, 005F0740 after the case-sensitive Veteran test.
    fn apply_completion_improvement(&mut self, index: i32);
    /// The source local-player/suppression/message branch.
    fn research_completed_notice(&mut self, _index: i32) {}
    /// Optional local presentation of 005367A0's output.
    fn successors_unlocked(&mut self, _index: i32, _trace: UnlockTrace) {}
}

pub fn complete(tree: &mut impl ResearchCompletion, index: i32) -> bool {
    let node = tree.node_at(index);
    if node.flags & REVEALED_AND_UNLOCKED != REVEALED_AND_UNLOCKED
        || node.flags & (0x40 | 0x04 | PRODUCTION) != 0
    {
        return false;
    }

    // 00536EE0 sets completed, clears running and removes the first matching
    // prerequisite from each dependent by copying its last array element.
    let flags = tree.flags_at(index);
    *flags = (*flags | 0x04) & !IN_PROGRESS;
    for position in 0..tree.completion_dependents(index).len() {
        let dependent = tree.completion_dependents(index)[position];
        let remaining = tree.remaining_requirements(dependent);
        if let Some(match_at) = remaining
            .iter()
            .position(|&requirement| requirement == index)
        {
            remaining.swap_remove(match_at);
        }
    }
    tree.completion_ledger().history.push(index);
    let category = tree.category_at(index);
    if matches!(category, 2..=4) {
        let same_epoch = i32::from(node.source_epoch) == tree.start_epoch_at_0c();
        let ledger = tree.completion_ledger();
        let bucket = (category - 2) as usize;
        if same_epoch {
            ledger.current_epoch[bucket] = ledger.current_epoch[bucket].wrapping_add(1);
        }
        ledger.total[bucket] = ledger.total[bucket].wrapping_add(1);
    }
    if let Some(upgrade) = tree.completion_upgrade(index) {
        tree.apply_completion_upgrade(upgrade);
    }
    if tree.category_at(index) == 5 {
        tree.apply_completion_improvement(index);
    }
    tree.research_completed_notice(index);
    let unlocks = unlock_successors(tree, index);
    tree.successors_unlocked(index, unlocks);
    true
}
