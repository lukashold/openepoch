//! 005367A0 successor selection, 00536D30 epoch tally and 00536E10 flag updates.

use crate::ai9_planner::Name;
use crate::ai9_research_start::ResearchStart;
use std::collections::BTreeMap;

const CIV_REGION: [i32; 14] = [0, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3];

/// The case-sensitive family table built at 00535B4E in source index order.
/// Host names are captured after the native per-player host substitutions.
#[derive(Default)]
pub struct ImprovementFamilies(BTreeMap<String, Vec<i32>>);

impl ImprovementFamilies {
    pub fn from_init_rows<'a>(
        rows: impl IntoIterator<Item = (i32, i32, Option<&'a Name>)>,
    ) -> Self {
        let mut families = BTreeMap::<String, Vec<i32>>::new();
        for (index, category, name) in rows {
            if category == 5 {
                if let Some(name) = name {
                    families
                        .entry(name.as_str().to_owned())
                        .or_default()
                        .push(index);
                }
            }
        }
        Self(families)
    }

    pub fn members(&self, name: Option<&Name>) -> &[i32] {
        name.and_then(|name| self.0.get(name.as_str()))
            .map_or(&[], Vec::as_slice)
    }
}

pub trait ResearchUnlock: ResearchStart {
    /// Ordered source node indices from the epoch table at +89C/+8A0.
    fn epoch_nodes(&self, epoch: i32) -> &[i32];
    /// Source definition+48 indices, in array order.
    fn completion_dependents(&self, index: i32) -> &[i32];
    /// Live node+24 requirements, with count at +28.
    fn remaining_requirements(&mut self, index: i32) -> &mut Vec<i32>;
    /// Source node definition+3C, eTechTreeCiv.
    fn node_civ_gate(&self, index: i32) -> i32;
    /// Native player civilization at TechTree database+99C, with -1 for all.
    fn civilization(&self) -> i32;
    /// Runtime C13D40. The PE initializer is six.
    fn main_threshold(&self) -> i32;
    fn improvement_families(&self) -> &ImprovementFamilies;
    /// Name of the live node+20 host. 00537C50 looks it up byte-for-byte.
    fn current_improvement_host(&self, index: i32) -> Option<&Name>;
    /// Definition+40 indices, count at +44.
    fn improvement_hide_list(&self, index: i32) -> &[i32];
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct UnlockTrace {
    pub unlocked: Vec<i32>,
    /// Explicit partial-tree boundary when an expected source epoch is absent.
    pub unresolved_main_epoch: Option<i32>,
}

fn civilization_matches(civ: i32, gate: i32) -> bool {
    if civ == -1 || gate == 0 {
        return true;
    }
    if gate < 15 {
        return civ == gate - 1;
    }
    CIV_REGION
        .get(civ as usize)
        .is_some_and(|&region| region == gate - 15)
}

fn epoch_tech_count(tree: &impl ResearchUnlock, epoch: i32) -> i32 {
    tree.epoch_nodes(epoch)
        .iter()
        .filter(|&&index| {
            matches!(tree.category_at(index), 2..=4) && tree.node_at(index).flags & 0x04 != 0
        })
        .count() as i32
}

fn enable(tree: &mut impl ResearchUnlock, index: i32) {
    *tree.flags_at(index) |= 0x02;
    if tree.category_at(index) != 5 {
        return;
    }
    let epoch = tree.node_at(index).source_epoch;
    fn family(tree: &impl ResearchUnlock, index: i32) -> &[i32] {
        tree.improvement_families()
            .members(tree.current_improvement_host(index))
    }
    for position in 0..family(tree, index).len() {
        let peer = family(tree, index)[position];
        let node = tree.node_at(peer);
        if node.source_epoch < epoch && node.flags & 0x10 == 0 {
            *tree.flags_at(peer) = (node.flags & !0x03) | 0x10;
        }
    }
    for position in 0..tree.improvement_hide_list(index).len() {
        let peer = tree.improvement_hide_list(index)[position];
        if tree.category_at(peer) == 5 {
            *tree.flags_at(peer) &= !0x01;
        }
    }
}

fn unlock_if_ready(
    tree: &mut impl ResearchUnlock,
    index: i32,
    next_main: bool,
    output: &mut Vec<i32>,
) {
    let node = tree.node_at(index);
    if node.flags & 0x42 != 0
        || (!next_main
            && (node.flags & 0x10 != 0
                || !civilization_matches(tree.civilization(), tree.node_civ_gate(index))))
    {
        return;
    }
    let ready = if tree.category_at(index) == 1 && node.source_epoch > 1 {
        epoch_tech_count(tree, i32::from(node.source_epoch) - 1) >= tree.main_threshold()
    } else {
        tree.remaining_requirements(index).is_empty()
    };
    if ready {
        *tree.flags_at(index) |= 0x01;
        enable(tree, index);
        output.push(index);
    }
}

pub fn unlock_successors(tree: &mut impl ResearchUnlock, completed: i32) -> UnlockTrace {
    let epoch = i32::from(tree.node_at(completed).source_epoch);
    let mut output = UnlockTrace::default();
    if tree.category_at(completed) == 1 {
        for position in 0..tree.epoch_nodes(epoch).len() {
            let index = tree.epoch_nodes(epoch)[position];
            if index != completed {
                unlock_if_ready(tree, index, false, &mut output.unlocked);
            }
        }
        return output;
    }

    for position in 0..tree.completion_dependents(completed).len() {
        let index = tree.completion_dependents(completed)[position];
        unlock_if_ready(tree, index, false, &mut output.unlocked);
    }
    if epoch < 15 && epoch < tree.maximum_epoch() && matches!(tree.category_at(completed), 2..=4) {
        let next = epoch + 1;
        let main = tree
            .epoch_nodes(next)
            .iter()
            .copied()
            .find(|&index| tree.category_at(index) == 1);
        if let Some(index) = main {
            unlock_if_ready(tree, index, true, &mut output.unlocked);
        } else {
            output.unresolved_main_epoch = Some(next);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_init_order_and_case_sensitive_lookup_use_only_hosted_improvements() {
        let exact = Name::new("LightInfantry").unwrap();
        let different_case = Name::new("lightinfantry").unwrap();
        let other = Name::new("HeavyInfantry1").unwrap();
        let table = ImprovementFamilies::from_init_rows([
            (2, 5, Some(&exact)),
            (3, 1, Some(&exact)),
            (4, 5, None),
            (5, 5, Some(&different_case)),
            (7, 5, Some(&exact)),
        ]);
        assert_eq!(table.members(Some(&exact)), [2, 7]);
        assert_eq!(table.members(Some(&different_case)), [5]);
        assert!(table.members(Some(&other)).is_empty());
        assert!(table.members(None).is_empty());
    }
}
