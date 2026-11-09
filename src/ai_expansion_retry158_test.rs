use super::*;

const OWNER: u8 = 1;

fn center_count(game: &Game) -> usize {
    game.ents[..game.high].iter().filter(|actor| actor.alive && actor.def == TC && actor.team == OWNER).count()
}

fn fixture() -> Game {
    let mut game = Game::new(7, 1, 0, 1, 0);
    game.players[OWNER as usize].epoch = 2;
    game.rebuild_modifiers(OWNER, false);
    let home = game.ents[..game.high].iter().find(|actor| actor.alive && actor.def == TC && actor.team == OWNER).unwrap();
    let (x, y) = (home.x, home.y);
    // Earlier production branches already have their required buildings.
    let established = [BARRACKS, BARRACKS, UNIVERSITY, WAREHOUSE, MILL, STABLE, TEMPLE, MARKET, WORKSHOP, TOWER, TOWER, DOCK];
    for (i, def) in established.into_iter().enumerate() {
        let row = i as i32 / 4;
        let column = i as i32 % 4;
        let index = game.spawn(def, OWNER, x - 8.0 + column as f32 * 4.0, y - 8.0 + row as f32 * 4.0);
        assert_ne!(index, NOIDX);
        game.ents[index].built = 1.0;
    }
    for resource in 0..9 {
        game.add_resource(OWNER, resource, 10_000.0);
    }
    game.players[OWNER as usize].population_limit = 500;
    game.players[OWNER as usize].cap = 500;
    game.players[OWNER as usize].researching = true;
    assert_eq!(center_count(&game), 1);
    game
}

fn account(game: &Game) -> [f32; 2] {
    [game.resource(OWNER, 1), game.resource(OWNER, 3)]
}

#[test]
fn ai_expansion_retries_after_a_blocked_plot_and_uses_the_paid_route() {
    let mut game = fixture();
    assert!(game.ai_retries_expansion());
    let cost = game.construction_cost(OWNER, TC);
    let before = account(&game);
    let initial_owners = game.towner;
    let actual_blocked = game.map.blocked.clone();
    game.map.blocked.fill(true);
    game.ai_step(OWNER);
    assert_eq!(center_count(&game), 1);
    assert_eq!(account(&game), before);
    assert!(!game.players[OWNER as usize].ai_noexp);

    game.map.blocked = actual_blocked;
    game.ai_step(OWNER);
    assert_eq!(center_count(&game), 2);
    assert_eq!(account(&game), [before[0] - f32::from(cost[1]), before[1] - f32::from(cost[3])]);
    let expansion = game.ents[..game.high].iter().find(|actor| actor.alive && actor.def == TC
        && actor.team == OWNER && actor.built < 1.0).unwrap();
    let territory = game.map.terr[tile_of(expansion.x, expansion.y)] as usize;
    assert_eq!(initial_owners[territory], 255);
    assert!(!game.players[OWNER as usize].ai_noexp);
}

#[test]
fn ai_expansion_legacy_latch_recovers_only_in_ordinary_solo() {
    let mut game = fixture();
    game.players[OWNER as usize].ai_noexp = true;
    game.save();
    let mut restored = Game::load(&game.savebuf).expect("ordinary save with the legacy expansion latch");
    assert!(restored.players[OWNER as usize].ai_noexp);
    restored.net = true;
    restored.ai_step(OWNER);
    assert_eq!(center_count(&restored), 1);
    restored.net = false;
    restored.ai_step(OWNER);
    assert_eq!(center_count(&restored), 2);
}

#[test]
fn ai_expansion_waits_when_no_district_is_free() {
    let mut game = fixture();
    let before = account(&game);
    let original = game.towner;
    for owner in &mut game.towner {
        if *owner == 255 { *owner = 0; }
    }
    game.ai_step(OWNER);
    assert_eq!(center_count(&game), 1);
    assert_eq!(account(&game), before);
    assert!(!game.players[OWNER as usize].ai_noexp);
    game.towner = original;
    game.ai_step(OWNER);
    assert_eq!(center_count(&game), 2);
}

#[test]
#[ignore = "writes the isolated WASM comparison input"]
fn ai_expansion_write_legacy_latch_fixture() {
    let mut game = fixture();
    game.players[OWNER as usize].ai_noexp = true;
    game.save();
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../legacy-latch-save.bin");
    std::fs::write(output, &game.savebuf).unwrap();
}
