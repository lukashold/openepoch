//! A framed research state around the existing Pack save, with strict validation.

use crate::ai9_game_bridge::{
    captured_duration, captured_payment, category, game_id, DurableResearch,
    MISSING_CONTEXT_DELAY, RANDOM_INTERVAL, SOURCE_BINDING_SHA,
};
use crate::ai9_player_step::ControllerClock;
use crate::ai9_rank::{
    self as rank, BINDING_SHA, FAMILY_COUNT, NODE_COUNT, PREVIOUS_BINDING_SHA, PREVIOUS_NODE_COUNT,
    RESET_SOURCE_EPOCH,
};
use crate::ai9_research_activation::ActiveResearch;
use crate::ai9_research_completion::CompletionLedger;
use crate::ai9_research_dispatch::ResearchState;
use crate::ai9_research_start::{PaidResearch, IN_PROGRESS};
use crate::game::Game;

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"A9R1");
const END: u32 = u32::from_le_bytes(*b"A9RE");
const VERSION: u32 = 2;
const MANUAL_VERSION: u32 = 3;
const FAMILY_VERSION: u32 = 4;
const HEADER: usize = 48;
const MAX_WORDS: usize = 125;

pub(crate) enum Records<'a> {
    Economy(&'a [u8]),
    Ranks(&'a [u8]),
    Manual(&'a [u8]),
    Families(&'a [u8]),
}

fn version(state: &DurableResearch) -> u32 {
    if state.manual { FAMILY_VERSION } else { VERSION }
}

fn binding(state: &DurableResearch) -> &[u8; 32] {
    if state.manual { &BINDING_SHA } else { &PREVIOUS_BINDING_SHA }
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2_166_136_261u32, |hash, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}

fn words(state: &DurableResearch) -> Vec<u32> {
    let clock = state.clock;
    let tree = &state.tree;
    let mut out = Vec::with_capacity(MAX_WORDS);
    out.push(u32::from(state.owner));
    let nodes = if state.manual { NODE_COUNT } else { PREVIOUS_NODE_COUNT };
    out.extend_from_slice(&tree.flags[..nodes]);
    out.extend([
        clock.last_frame.to_bits(),
        clock.last_planner.to_bits(),
        clock.next_random_deadline.to_bits(),
        clock.previous_aip_word,
        clock.research.last_check.to_bits(),
        clock.research.last_list_submit.to_bits(),
        tree.active.len() as u32,
        tree.pending.len() as u32,
        tree.ledger.history.len() as u32,
    ]);
    for job in &tree.active {
        out.extend([
            job.index as u32,
            job.host_handle as u32,
            job.elapsed.to_bits(),
            job.duration.to_bits(),
        ]);
        out.extend(job.cost.map(f32::to_bits));
    }
    for job in &tree.pending {
        out.push(job.index as u32);
        out.extend(job.cost.map(f32::to_bits));
    }
    out.extend(tree.ledger.history.iter().map(|&index| index as u32));
    out.extend(tree.ledger.current_epoch.map(|count| count as u32));
    out.extend(tree.ledger.total.map(|count| count as u32));
    out
}

pub(crate) fn hash(state: &DurableResearch, mut mix: impl FnMut(u32)) {
    mix(MAGIC);
    mix(version(state));
    let sha = binding(state);
    for at in (0..sha.len()).step_by(4) {
        mix(word(sha, at).unwrap());
    }
    for value in words(state) {
        mix(value);
    }
}

fn packed(inner: &[u8]) -> bool {
    word(inner, 0) == Some(crate::pack8_save::MAGIC) && word(inner, 4) == Some(4)
}

pub(crate) fn wrap(game: &Game, state: &DurableResearch, inner: &[u8]) -> Option<Vec<u8>> {
    if !packed(inner) || !validate(game, state) {
        return None;
    }
    let fields = words(state);
    let mut out = Vec::with_capacity(HEADER + inner.len() + fields.len() * 4 + 8);
    for value in [
        MAGIC,
        version(state),
        u32::try_from(inner.len()).ok()?,
        fields.len() as u32,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(binding(state));
    out.extend_from_slice(inner);
    for value in fields {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&checksum(&out).to_le_bytes());
    out.extend_from_slice(&END.to_le_bytes());
    Some(out)
}

pub(crate) fn split(buf: &[u8]) -> Option<(&[u8], Option<Records<'_>>)> {
    if word(buf, 0) != Some(MAGIC) {
        return Some((buf, None));
    }
    let version = match (word(buf, 4)?, buf.get(16..HEADER)?) {
        (1, bytes) if bytes == SOURCE_BINDING_SHA => 1,
        (v @ (VERSION | MANUAL_VERSION), bytes) if bytes == PREVIOUS_BINDING_SHA => v,
        (FAMILY_VERSION, bytes) if bytes == BINDING_SHA => FAMILY_VERSION,
        _ => return None,
    };
    let inner_len = word(buf, 8)? as usize;
    let size = word(buf, 12)? as usize;
    let sizes = match version {
        1 => 20..=49,
        FAMILY_VERSION => 28..=MAX_WORDS,
        _ => 22..=68,
    };
    if inner_len < crate::pack8_save::HEADER
        || inner_len > 64 * 1024 * 1024
        || !sizes.contains(&size)
    {
        return None;
    }
    let payload = HEADER.checked_add(inner_len)?;
    let end = payload.checked_add(size.checked_mul(4)?)?;
    if buf.len() != end.checked_add(8)?
        || word(buf, end)? != checksum(buf.get(..end)?)
        || word(buf, end + 4)? != END
    {
        return None;
    }
    let inner = buf.get(HEADER..payload)?;
    if !packed(inner) {
        return None;
    }
    let fields = buf.get(payload..end)?;
    let records = match version {
        1 => Records::Economy(fields),
        MANUAL_VERSION => Records::Manual(fields),
        FAMILY_VERSION => Records::Families(fields),
        _ => Records::Ranks(fields),
    };
    Some((inner, Some(records)))
}

struct Cursor<'a>(&'a [u8]);
impl Cursor<'_> {
    fn next(&mut self) -> Option<u32> {
        let value = word(self.0, 0)?;
        self.0 = self.0.get(4..)?;
        Some(value)
    }
    fn float(&mut self) -> Option<f32> {
        Some(f32::from_bits(self.next()?))
    }
    fn array<const N: usize>(&mut self) -> Option<[u32; N]> {
        let mut out = [0; N];
        for slot in &mut out {
            *slot = self.next()?;
        }
        Some(out)
    }
}

pub(crate) fn decode(game: &Game, records: Records<'_>) -> Option<DurableResearch> {
    let (fields, version) = match records {
        Records::Economy(fields) => (fields, 1),
        Records::Ranks(fields) => (fields, VERSION),
        Records::Manual(fields) => (fields, MANUAL_VERSION),
        Records::Families(fields) => (fields, FAMILY_VERSION),
    };
    let mut cursor = Cursor(fields);
    let owner = u8::try_from(cursor.next()?).ok()?;
    let mut flags = [0; NODE_COUNT];
    match version {
        1 => {
            flags[..4].copy_from_slice(&cursor.array::<4>()?);
            flags[rank::FIRST_RANK] = 0x03;
        }
        FAMILY_VERSION => flags = cursor.array()?,
        _ => flags[..PREVIOUS_NODE_COUNT].copy_from_slice(&cursor.array::<PREVIOUS_NODE_COUNT>()?),
    }
    if version == MANUAL_VERSION {
        for family in 1..FAMILY_COUNT {
            flags[rank::pair(family)[0]] = 0x03;
        }
    }
    let clock = ControllerClock {
        last_frame: cursor.float()?,
        last_planner: cursor.float()?,
        next_random_deadline: cursor.float()?,
        previous_aip_word: cursor.next()?,
        research: ResearchState {
            last_check: cursor.float()?,
            last_list_submit: cursor.float()?,
        },
    };
    let [active_count, pending_count, history_count] = cursor.array()?;
    let (max_active, max_history) = match version {
        1 => (1, 2),
        FAMILY_VERSION => (1 + FAMILY_COUNT as u32, 2 + 2 * FAMILY_COUNT as u32),
        _ => (2, 4),
    };
    if pending_count > 1 || active_count > max_active || history_count > max_history {
        return None;
    }
    let mut active = Vec::new();
    for _ in 0..active_count {
        active.push(ActiveResearch {
            index: cursor.next()? as i32,
            host_handle: cursor.next()? as i32,
            elapsed: cursor.float()?,
            duration: cursor.float()?,
            cost: cursor.array::<11>()?.map(f32::from_bits),
        });
    }
    let mut pending = Vec::new();
    for _ in 0..pending_count {
        pending.push(PaidResearch {
            index: cursor.next()? as i32,
            cost: cursor.array::<11>()?.map(f32::from_bits),
        });
    }
    let mut history = Vec::new();
    for _ in 0..history_count {
        history.push(cursor.next()? as i32);
    }
    let ledger = CompletionLedger {
        history,
        current_epoch: cursor.array::<3>()?.map(|n| n as i32),
        total: cursor.array::<3>()?.map(|n| n as i32),
    };
    if !cursor.0.is_empty() {
        return None;
    }
    if version == 1
        && (game.players[0].epoch != 0
            || active.iter().any(|job| game_id(job.index).is_none())
            || ledger.history.iter().any(|&index| game_id(index).is_none()))
    {
        return None;
    }
    if version < FAMILY_VERSION && active.iter().map(|job| job.index)
        .chain(ledger.history.iter().copied()).any(|index| index as usize >= PREVIOUS_NODE_COUNT)
    {
        return None;
    }
    let manual = version >= MANUAL_VERSION;
    let state = DurableResearch::restore(owner, manual, flags, clock, active, pending, ledger);
    validate(game, &state).then_some(state)
}

fn validate(game: &Game, state: &DurableResearch) -> bool {
    let owner = state.owner;
    let tree = &state.tree;
    let family_count = if state.manual { FAMILY_COUNT } else { 1 };
    if game.net
        || owner != 0
        || owner as usize >= game.nplayers
        || game.players[owner as usize].epoch + 1 >= RESET_SOURCE_EPOCH
        || tree.flags[0] != 0x07
        || tree.flags[3] != 0
        || tree.active.len() > 1 + family_count
        || tree.pending.len() > 1
        || (tree.active.is_empty() && !tree.pending.is_empty())
        || tree.ledger.history.len() > 2 + 2 * family_count
    {
        return false;
    }
    let now = game.sim_elapsed();
    let clock = state.clock;
    if clock.last_frame.to_bits() != now.to_bits()
        || clock.previous_aip_word != 2
        || !(0.0..=now + MISSING_CONTEXT_DELAY).contains(&clock.last_planner)
        || !(0.0..=now + RANDOM_INTERVAL[1]).contains(&clock.next_random_deadline)
        || !(0.0..=now).contains(&clock.research.last_check)
        || !(0.0..=clock.research.last_check).contains(&clock.research.last_list_submit)
    {
        return false;
    }
    if state.manual && (!(0.0..=now).contains(&clock.last_planner)
        || [clock.next_random_deadline, clock.research.last_check, clock.research.last_list_submit]
            .iter().any(|time| time.to_bits() != clock.last_planner.to_bits())) {
        return false;
    }

    let valid_payment = |index, cost: [f32; 11]| {
        captured_payment(game, owner, index)
            .is_some_and(|expected| cost.map(f32::to_bits) == expected.map(f32::to_bits))
    };
    let mut queued = [false; NODE_COUNT];
    let mut active_economy = false;
    let mut active_families = [false; FAMILY_COUNT];
    for job in &tree.active {
        let index = job.index as usize;
        if job.host_handle != -1
            || !state.supports(index)
            || !valid_payment(job.index, job.cost)
            || queued[index]
            || captured_duration(job.index)
                .is_none_or(|expected| expected.to_bits() != job.duration.to_bits())
            || !job.elapsed.is_finite()
            || job.elapsed.is_sign_negative()
            || job.elapsed >= job.duration
        {
            return false;
        }
        let active_kind = if let Some(family) = rank::family_at(index) {
            &mut active_families[family]
        } else {
            &mut active_economy
        };
        if *active_kind {
            return false;
        }
        *active_kind = true;
        queued[index] = true;
    }
    for job in &tree.pending {
        if !active_economy
            || game_id(job.index).is_none()
            || !valid_payment(job.index, job.cost)
            || queued[job.index as usize]
        {
            return false;
        }
        queued[job.index as usize] = true;
    }
    for (index, &queued) in queued.iter().enumerate().take(3).skip(1) {
        let id = game_id(index as i32).unwrap();
        if game.has_ordinary_research_job(owner, id) {
            return false;
        }
        let expected = if queued {
            0x03 | IN_PROGRESS
        } else if game.has_research(owner, id) {
            0x07
        } else {
            0x03
        };
        if tree.flags[index] != expected || (queued && game.has_research(owner, id)) {
            return false;
        }
    }
    let mut completed = [false; NODE_COUNT];
    let mut counts = [0; 3];
    for &id in &tree.ledger.history {
        let index = id as usize;
        let family = rank::family_at(index);
        if !state.supports(index)
            || (game_id(id).is_none() && family.is_none())
            || completed[index]
            || tree.flags[index] != 0x07
            || family.is_some_and(|family| {
                let [veteran, elite] = rank::pair(family);
                index == elite && !completed[veteran]
            })
        {
            return false;
        }
        completed[index] = true;
        let branch = category(index);
        if (2..=4).contains(&branch) {
            counts[(branch - 2) as usize] += 1;
        }
    }
    let rank_flags = |index| {
        if completed[index] {
            0x07
        } else if queued[index] {
            0x03 | IN_PROGRESS
        } else {
            0x03
        }
    };
    for family in 0..family_count {
        let [veteran, elite] = rank::pair(family);
        if tree.flags[veteran] != rank_flags(veteran)
            || tree.flags[elite] != if completed[veteran] { rank_flags(elite) } else { 0 }
            || (queued[elite] && !completed[veteran])
        {
            return false;
        }
    }
    let current = if game.players[owner as usize].epoch == 0 {
        counts
    } else {
        [0; 3]
    };
    tree.ledger.current_epoch == current && tree.ledger.total == counts
}
