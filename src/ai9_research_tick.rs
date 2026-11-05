//! 006E5D90's +18 active technology-list pass and 006E4690's pending promotion.
//! The separate +28 hosted production pass has its own record consumer.

use crate::ai9_count::Precision53;
use crate::ai9_research_activation::ActiveResearch;
use crate::ai9_research_completion::{complete, ResearchCompletion};
use crate::ai9_research_start::{activate_paid, meets_prerequisite, IN_PROGRESS};

#[derive(Clone, Copy)]
pub struct ResearchFrame {
    /// The source frame delta at C9C3E8.
    pub delta: f32,
    /// The current owner's C9C368 byte is nonzero.
    pub immediate: bool,
    pub precision: Precision53,
}

pub trait ResearchTick: ResearchCompletion {
    fn active_research_mut(&mut self) -> &mut Vec<ActiveResearch>;
    /// 00864670 host lookup followed by the health word at +58.
    fn research_host_hp(&mut self, handle: i32) -> Option<f32>;
    /// 006E4690's local missing-host notice, before cancellation.
    fn pending_host_lost(&mut self, _index: i32) {}
    /// The 006E4D30 notification, subject to the provider's local-player gate.
    fn research_cancelled_notice(&mut self, _index: i32) {}
}

pub fn promote_pending(tree: &mut impl ResearchTick) -> bool {
    while !tree.pending_research().is_empty() {
        let paid = tree.pending_research().remove(0);
        let node = tree.node_at(paid.index);
        if meets_prerequisite(tree, node) {
            activate_paid(tree, paid);
            return true;
        }
        tree.pending_host_lost(paid.index);
        cancel(tree, paid.index, -1, true);
    }
    false
}

/// 006E4D30. The index search for a refund precedes the index/host removal.
pub fn cancel(tree: &mut impl ResearchTick, index: i32, host: i32, suppress_notice: bool) -> bool {
    let flags = tree.flags_at(index);
    if *flags & IN_PROGRESS == 0 {
        return false;
    }
    *flags &= !IN_PROGRESS;

    let active = tree
        .active_research()
        .iter()
        .find(|row| row.index == index)
        .copied();
    let mut promote = false;
    if let Some(row) = active {
        tree.start_accounts().refund(&row.cost);
        if let Some(position) = tree
            .active_research()
            .iter()
            .position(|row| row.index == index && row.host_handle == host)
        {
            tree.active_research_mut().remove(position);
        }
        promote = tree.category_at(index) != 5;
    } else {
        let pending = tree.pending_research();
        if let Some(position) = pending.iter().position(|row| row.index == index) {
            let paid = pending.remove(position);
            tree.start_accounts().refund(&paid.cost);
        }
    }

    if !suppress_notice {
        tree.research_cancelled_notice(index);
    }
    if promote {
        promote_pending(tree);
    }
    true
}

pub fn tick_active(tree: &mut impl ResearchTick, frame: ResearchFrame) {
    for position in (0..tree.active_research().len()).rev() {
        let active = tree.active_research()[position];
        let valid_host = active.host_handle < 0
            || tree
                .research_host_hp(active.host_handle)
                .is_some_and(|hp| hp > 0.0);
        if !valid_host
            || (tree.category_at(active.index) == 5
                && tree.node_at(active.index).flags & 0x12 != 0x02)
        {
            *tree.flags_at(active.index) &= !IN_PROGRESS;
            tree.start_accounts().refund(&active.cost);
            tree.active_research_mut().remove(position);
            continue;
        }
        if active.host_handle >= 0 {
            continue;
        }
        if !frame.immediate {
            let sum = frame
                .precision
                .add(frame.delta as f64, active.elapsed as f64);
            let elapsed = if sum > active.duration as f64 {
                active.duration
            } else {
                frame.precision.store_f32(sum)
            };
            tree.active_research_mut()[position].elapsed = elapsed;
            if !matches!(
                elapsed.partial_cmp(&active.duration),
                Some(core::cmp::Ordering::Equal | core::cmp::Ordering::Greater)
            ) {
                continue;
            }
        }
        complete(tree, active.index);
        tree.active_research_mut().remove(position);
        if tree.category_at(active.index) != 5 {
            promote_pending(tree);
        }
    }
}
