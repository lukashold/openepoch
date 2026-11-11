//! Stock fresh-route frontier and first-discovery path reconstruction.

use super::atlas_reader::Precision53;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scores {
    pub cumulative: f32,
    pub priority: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate<Node> {
    pub node: Node,
    pub edge_cost: f32,
    pub heuristic: f32,
}

#[derive(Clone, Copy)]
pub enum Visit<Node> {
    Discovered {parent: Option<Node>, node: Node, scores: Scores},
    Popped {node: Node, scores: Scores},
}

#[derive(Clone, Copy)]
struct Entry<Node> {
    node: Node,
    priority: f32,
}

fn push<Node: Copy>(heap: &mut Vec<Entry<Node>>, entry: Entry<Node>) {
    let mut i = heap.len();
    heap.push(entry);
    while i > 0 {
        let parent = (i - 1) / 2;
        if heap[parent].priority <= entry.priority { break; }
        heap[i] = heap[parent];
        i = parent;
    }
    heap[i] = entry;
}

fn pop<Node: Copy>(heap: &mut Vec<Entry<Node>>) -> Option<Node> {
    let first = *heap.first()?;
    let last = heap.pop().unwrap();
    if heap.is_empty() { return Some(first.node); }
    let mut i = 0;
    loop {
        let left = i * 2 + 1;
        let right = left + 1;
        if left >= heap.len() { break; }
        let child = if right < heap.len() && heap[right].priority < heap[left].priority { right } else { left };
        if last.priority <= heap[child].priority { break; }
        heap[i] = heap[child];
        i = child;
    }
    heap[i] = last;
    Some(first.node)
}

fn scores(mode: Precision53, previous: f32, edge_cost: f32, heuristic: f32) -> Scores {
    let cumulative = mode.store_f32(mode.add(f64::from(previous), f64::from(edge_cost)));
    let weighted = mode.mul(f64::from(heuristic), f64::from(f32::from_bits(0x3f7e76c9)));
    let priority = mode.store_f32(mode.add(f64::from(cumulative), weighted));
    Scores {cumulative, priority}
}

/// The provider yields already admitted edges in original atlas iteration
/// order. A missing provider result aborts the attempt, including partial paths.
pub fn find<Node: Copy + Ord>(
    mode: Precision53,
    start: Node,
    goal: Node,
    start_heuristic: f32,
    mut neighbors: impl FnMut(Node) -> Option<Vec<Candidate<Node>>>,
    mut visit: impl FnMut(Visit<Node>),
) -> Option<Vec<(Node, f32)>> {
    let initial = scores(mode, 0.0, 0.0, start_heuristic);
    let mut seen = BTreeMap::from([(start, (None, initial))]);
    let mut heap = vec![Entry {node: start, priority: initial.priority}];
    visit(Visit::Discovered {parent: None, node: start, scores: initial});
    while let Some(node) = pop(&mut heap) {
        let (_, current) = seen[&node];
        visit(Visit::Popped {node, scores: current});
        if node == goal {
            let mut path = Vec::new();
            let mut next = Some(node);
            while let Some(point) = next {
                let (parent, cost) = seen[&point];
                path.push((point, cost.cumulative));
                next = parent;
            }
            path.reverse();
            return Some(path);
        }
        for candidate in neighbors(node)? {
            let std::collections::btree_map::Entry::Vacant(entry) = seen.entry(candidate.node) else { continue; };
            let value = scores(mode, current.cumulative, candidate.edge_cost, candidate.heuristic);
            entry.insert((Some(node), value));
            push(&mut heap, Entry {node: candidate.node, priority: value.priority});
            visit(Visit::Discovered {parent: Some(node), node: candidate.node, scores: value});
        }
    }
    None
}
