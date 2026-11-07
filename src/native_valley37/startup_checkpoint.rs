//! Read-only checkpoints for the locally authenticated authored playtest.
use super::active_game;
use crate::{game::Game, native_rng9::NativeRng};

const SOURCE_RESOURCE_ORDER: [usize; 9] = [0, 1, 3, 4, 5, 2, 6, 7, 8];

pub(super) fn active_authored_test() -> Option<&'static Game> {
    let game = active_game()?;
    let fixture = game.valley_fixture.as_ref()?;
    (fixture.query(6) == 1 && game.tick == 0 && game.nplayers == 2 && !game.net && game.civilizations[..2] == [4, 8] &&
        !game.players[0].ai && game.players[1].ai).then_some(game)
}

#[no_mangle]
pub extern "C" fn valley55_start_player_word(owner: u32, field: u32) -> u32 {
    let Some(game) = active_authored_test() else { return u32::MAX; };
    if !(1..=2).contains(&owner) { return u32::MAX; }
    let team = owner as usize - 1;
    match field {
        0 => owner,
        1 => u32::from(game.civilizations[team]),
        2 => u32::from(game.players[team].ai),
        3 => u32::from(game.players[team].epoch) + 1,
        4 => game.players[team].population_limit,
        5..=13 => game.resource(team as u8, SOURCE_RESOURCE_ORDER[field as usize - 5]).to_bits(),
        _ => u32::MAX,
    }
}

/// 004F5F40 selects from the verified Egyptian/Normal text order. The caller
/// supplies the RNG at that exact checkpoint. Fields never change either RNG.
#[no_mangle]
pub extern "C" fn valley55_personality_word(before_personality: u32, field: u32) -> u32 {
    if active_authored_test().is_none() { return u32::MAX; }
    let mut rng = NativeRng::from_state(before_personality);
    let index = rng.next_word15() % 2;
    match field {
        0 => index, // Mil_Crown.dll, Eco_Crown.dll
        1 => rng.state(),
        2 => 0x7a,
        3 => 5,
        4 => 11,
        5 => 8,
        6 => 16,
        7 => 0, // original startup commit authority
        _ => u32::MAX,
    }
}
