//! UP Conquest presence uses the source behavior bindings for the current roster.

use crate::defs::{DEFS, SAVE_DEFINITION_COUNT};
use crate::game::Game;

include!("ee2_conquest_rows.rs");
const _: [(); SAVE_DEFINITION_COUNT] = [(); CONQUEST_CAPABLE.len()];
const _: [(); DEFS.len()] = [(); crate::granary211::ID as usize + 1];

fn capable(def: crate::ids::DefId) -> bool {
    matches!(def, crate::ee2_regicide::KING | crate::air31_defs::FIGHTER_BOMBER | crate::carrier36_defs::CARRIER |
        crate::priest42_defs::PRIEST | crate::heavy_mounted3_61::ID) || crate::leader58_defs::branch(def).is_some() || crate::wonders61::is_wonder(def) ||
        CONQUEST_CAPABLE.get(def as usize).copied().unwrap_or(false)
}

pub(crate) fn living_seats(game: &Game) -> [bool; 4] {
    let mut living = [false; 4];
    for actor in &game.ents[..game.high] {
        if !actor.alive || !(actor.hp > 0.0) || actor.built < 1.0 {
            continue;
        }
        let Some(owner) = living.get_mut(actor.team as usize) else { continue; };
        if *owner {
            continue;
        }
        *owner = capable(actor.def)
            || (0..actor.ncargo as usize).any(|slot| {
                actor.cargo_hp[slot] > 0 && capable(actor.cargo[slot])
            });
    }
    living
}

pub(crate) fn survivors_are_allied(game: &Game) -> bool {
    for seat in 0..game.nplayers {
        if game.players[seat].alive
            && (0..seat).any(|other| game.players[other].alive && !game.allied(seat as u8, other as u8))
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defs::{BARRACKS, FARM, HOUSE, TC, WORKER};
    use crate::diplomacy::ALLIED;
    use crate::game::NONE;
    use crate::map::{tile_of, N};

    fn skirmish() -> Game {
        let mut game = Game::new(7, 2, 0, 1, 0);
        for player in &mut game.players[..game.nplayers] {
            player.ai = false;
        }
        game
    }

    fn clear_seat(game: &mut Game, seat: u8) {
        for actor in &mut game.ents[..game.high] {
            if actor.team == seat {
                actor.hp = 0.0;
            }
        }
    }

    fn step_result(game: &mut Game) {
        for _ in 0..20 {
            game.tick();
        }
    }

    #[test]
    fn a_citizen_without_a_building_keeps_the_seat_alive() {
        let mut game = skirmish();
        clear_seat(&mut game, 0);
        let citizen = game.spawn(WORKER, 0, 130.5, 130.5);
        step_result(&mut game);
        assert!(game.players[0].alive);
        assert_eq!(game.players[0].buildings, 0);
        assert_eq!(game.winner, 0);
        game.save();
        let mut loaded = Game::load(&game.savebuf).unwrap();
        step_result(&mut loaded);
        assert!(loaded.players[0].alive);
        loaded.ents[citizen].hp = 0.0;
        step_result(&mut loaded);
        assert!(!loaded.players[0].alive);
        assert_eq!(loaded.winner, 2);
    }

    #[test]
    fn a_priest_counts_even_with_no_power_reserve() {
        let mut game = skirmish();
        assert!(game.configure_priest_skirmish());
        clear_seat(&mut game, 0);
        game.spawn(crate::priest42_defs::PRIEST, 0, 130.5, 130.5);
        assert!(living_seats(&game)[0]);
    }

    #[test]
    fn completed_latent_attack_counts_but_farms_and_foundations_do_not() {
        let mut game = skirmish();
        clear_seat(&mut game, 1);
        clear_seat(&mut game, 2);
        game.spawn(FARM, 1, 125.5, 125.5);
        let foundation = game.spawn(BARRACKS, 1, 135.5, 130.5);
        game.ents[foundation].built = 0.4;
        let house = game.spawn(HOUSE, 2, 145.5, 130.5);
        step_result(&mut game);
        assert_eq!(game.players[1].buildings, 2);
        assert!(!game.players[1].alive);
        assert!(game.players[2].alive);
        assert_eq!(game.winner, 0);
        game.ents[house].hp = 0.0;
        step_result(&mut game);
        assert_eq!(game.winner, 1);
    }

    #[test]
    fn meaningful_cargo_counts_as_long_as_its_host_lives() {
        let mut game = skirmish();
        clear_seat(&mut game, 0);
        let cargo_ship = game.spawn(42, 0, 120.5, 120.5);
        game.ents[cargo_ship].ncargo = 1;
        game.ents[cargo_ship].cargo[0] = WORKER;
        game.ents[cargo_ship].cargo_hp[0] = 35;
        step_result(&mut game);
        assert!(game.players[0].alive);
        assert_eq!(game.winner, 0);
        game.ents[cargo_ship].hp = 0.0;
        step_result(&mut game);
        assert!(!game.players[0].alive);
        assert_eq!(game.winner, 2);
    }

    #[test]
    fn coalition_requires_all_pairs_of_surviving_players() {
        let mut game = skirmish();
        for index in [1, 4, 6, 9] {
            game.diplomacy.relation[index] = ALLIED;
        }
        step_result(&mut game);
        assert_eq!(game.winner, 0);
        game.diplomacy.relation[2] = ALLIED;
        step_result(&mut game);
        assert_eq!(game.winner, 0);
        game.diplomacy.relation[8] = ALLIED;
        step_result(&mut game);
        assert_eq!(game.winner, 1);
    }

    #[test]
    fn gifting_the_last_city_keeps_a_surviving_citizen_in_conquest() {
        let mut game = skirmish();
        let city = game.ents[..game.high].iter()
            .find(|actor| actor.alive && actor.def == TC && actor.team == 0).unwrap();
        let territory = game.map.terr[tile_of(city.x, city.y)] as u32;
        let refuge = game.map.terr.iter().enumerate()
            .position(|(tile, &region)| u32::from(region) != territory && game.map.pass[0][tile])
            .unwrap();
        let citizen = game.spawn(WORKER, 0, (refuge % N) as f32 + 0.5, (refuge / N) as f32 + 0.5);
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(game.apply_territory(0, 1, 0, territory, NONE));
        assert_eq!(game.players[0].buildings, 0);
        assert!(game.players[0].alive);
        assert_eq!(game.ents[citizen].team, 0);
        step_result(&mut game);
        assert_eq!(game.winner, 0);
    }
}
