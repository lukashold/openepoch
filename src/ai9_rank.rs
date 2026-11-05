//! LightInfantry rank actions use their own command tag and the source family.

use crate::ai9_count::{Precision53, BIAS};
use crate::defs::{DefId, QueueItem};
use crate::game::Game;

include!("ai9_rank_rows.rs");

pub(crate) const VETERAN: usize = 4;
pub(crate) const ELITE: usize = 5;
pub(crate) const NODE_COUNT: usize = 6;
pub(crate) const HOST: &str = "LightInfantry";

pub(crate) fn index(job: QueueItem) -> Option<usize> {
    JOBS.iter().position(|&id| id == job).map(|i| VETERAN + i)
}

pub(crate) fn class(def: DefId, epoch: u8) -> Option<&'static crate::ee2_units::UnitRow> {
    crate::ee2_units::lookup_class(def, epoch)
        .filter(|row| row.source_unit == HOST && row.source_epoch == 1)
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
    let Some(base) = class(def, game.players[owner as usize].epoch).and_then(|row| row.hp) else {
        return unranked;
    };
    let value = with_temporary(base, unranked, rank);
    // HitPoints callback 005F53D0 stores an integer UnitType+8C.
    let biased = Precision53::NearestEven.add(f64::from(value) + 0.5, BIAS);
    ((biased.to_bits() as i64) >> 16) as i32 as f32
}

pub(crate) fn damage(game: &Game, owner: u8, def: DefId, unranked: f32) -> f32 {
    let rank = game.source_light_infantry_rank(owner);
    if rank == 0 {
        return unranked;
    }
    match class(def, game.players[owner as usize].epoch).and_then(|row| row.damage) {
        Some(base) => with_temporary(base, unranked, rank),
        None => unranked,
    }
}

/// Native health at actor+58 is normalized (00842C20). The Game keeps an
/// absolute HP word, so live actors carry the same health fraction forward.
pub(crate) fn rescale_live_hp(game: &mut Game, owner: u8, before: u8, after: u8) {
    if before == after {
        return;
    }
    for index in 0..game.high {
        let entity = &game.ents[index];
        if !entity.alive
            || entity.team != owner
            || class(entity.def, game.players[owner as usize].epoch).is_none()
        {
            continue;
        }
        let base = game.modifiers[owner as usize][entity.def as usize][0];
        let previous = hp(game, owner, entity.def, base, before);
        let next = hp(game, owner, entity.def, base, after);
        game.ents[index].hp *= next / previous;
    }
}

#[no_mangle]
pub extern "C" fn ai9_game_improvement_job(rank: u32) -> u32 {
    rank.checked_sub(1)
        .and_then(|index| JOBS.get(index as usize).copied())
        .unwrap_or(u32::MAX)
}

#[no_mangle]
pub extern "C" fn ai9_game_unit_rank(team: u32, def: u32) -> u32 {
    let Some(game) = (unsafe { (*core::ptr::addr_of_mut!(crate::GAME)).as_deref() }) else {
        return 0;
    };
    if team as usize >= game.nplayers || def as usize >= crate::defs::DEFS.len()
        || class(def as DefId, game.players[team as usize].epoch).is_none()
    {
        return 0;
    }
    u32::from(game.source_light_infantry_rank(team as u8))
}
