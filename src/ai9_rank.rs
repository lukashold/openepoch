//! First-epoch rank actions use a command tag and the source unit family.

use crate::ai9_count::{Precision53, BIAS};
use crate::defs::{DefId, QueueItem};
use crate::game::Game;

include!("ai9_rank_rows.rs");

pub(crate) const FIRST_RANK: usize = 4;
pub(crate) const FAMILY_COUNT: usize = HOSTS.len();
pub(crate) const NODE_COUNT: usize = FIRST_RANK + JOBS.len();
pub(crate) const PREVIOUS_NODE_COUNT: usize = FIRST_RANK + 2;

pub(crate) fn index(job: QueueItem) -> Option<usize> {
    JOBS.iter().position(|&id| id == job).map(|i| FIRST_RANK + i)
}

pub(crate) fn family_at(index: usize) -> Option<usize> {
    (FIRST_RANK..NODE_COUNT).contains(&index).then(|| (index - FIRST_RANK) / 2)
}

pub(crate) const fn pair(family: usize) -> [usize; 2] {
    let veteran = FIRST_RANK + family * 2;
    [veteran, veteran + 1]
}

fn first_class(def: DefId, epoch: u8) -> Option<&'static crate::ee2_units::UnitRow> {
    crate::ee2_units::lookup_class(def, epoch).filter(|row| row.source_epoch == 1)
}

pub(crate) fn unit_family(def: DefId, epoch: u8) -> Option<usize> {
    if epoch + 1 >= RESET_SOURCE_EPOCH {
        return None;
    }
    let class = first_class(def, epoch)?;
    HOSTS.iter().position(|&family| family == class.source_unit)
}

/// Source 005F349A accumulates the temporary word. The existing Game's
/// unranked modifiers retain its already applied permanent/additive effects.
fn with_temporary(base: f32, unranked: f32, rank: u8) -> f32 {
    let p = Precision53::NearestEven;
    let percent = f64::from(100 + TEMP_PERCENT * rank);
    let ranked_base = p.mul(p.mul(percent, f64::from(PERCENT_LITERAL)), f64::from(base));
    p.store_f32(p.add(ranked_base, f64::from(unranked) - f64::from(base)))
}

pub(crate) fn hp(game: &Game, owner: u8, def: DefId, unranked: f32, rank: u8) -> f32 {
    if rank == 0 {
        return unranked;
    }
    let Some(base) = first_class(def, game.players[owner as usize].epoch).and_then(|row| row.hp) else {
        return unranked;
    };
    let value = with_temporary(base, unranked, rank);
    // HitPoints callback 005F53D0 stores an integer UnitType+8C.
    let biased = Precision53::NearestEven.add(f64::from(value) + 0.5, BIAS);
    ((biased.to_bits() as i64) >> 16) as i32 as f32
}

pub(crate) fn damage(game: &Game, owner: u8, def: DefId, unranked: f32) -> f32 {
    let rank = game.source_unit_rank(owner, def);
    if rank == 0 {
        return unranked;
    }
    match first_class(def, game.players[owner as usize].epoch).and_then(|row| row.damage) {
        Some(base) => with_temporary(base, unranked, rank),
        None => unranked,
    }
}

/// Native health at actor+58 is normalized (00842C20). The Game keeps an
/// absolute HP word, so live actors carry the same health fraction forward.
pub(crate) fn rescale_live_hp(game: &mut Game, owner: u8, family: usize, before: u8, after: u8) {
    if before == after {
        return;
    }
    for index in 0..game.high {
        let entity = &game.ents[index];
        if !entity.alive
            || entity.team != owner
            || unit_family(entity.def, game.players[owner as usize].epoch) != Some(family)
        {
            continue;
        }
        let base = game.modifiers[owner as usize][entity.def as usize][0];
        let previous = hp(game, owner, entity.def, base, before);
        let next = hp(game, owner, entity.def, base, after);
        game.ents[index].hp = (entity.hp / previous) * next;
    }
}

#[no_mangle]
pub extern "C" fn ai9_game_improvement_job(rank: u32) -> u32 {
    ai9_game_improvement_family_job(0, rank)
}

#[no_mangle]
pub extern "C" fn ai9_game_improvement_family_job(family: u32, rank: u32) -> u32 {
    if family as usize >= FAMILY_COUNT || !(1..=2).contains(&rank) {
        return u32::MAX;
    }
    JOBS[family as usize * 2 + (rank - 1) as usize]
}

#[no_mangle]
pub extern "C" fn ai9_game_unit_rank(team: u32, def: u32) -> u32 {
    let Some(game) = (unsafe { (*core::ptr::addr_of_mut!(crate::GAME)).as_deref() }) else {
        return 0;
    };
    if team as usize >= game.nplayers || def as usize >= crate::defs::DEFS.len() {
        return 0;
    }
    u32::from(game.source_unit_rank(team as u8, def as DefId))
}
