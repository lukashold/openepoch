use super::*;
use crate::cmd::{C_MOVE, C_STOP, C_TRADE_ROUTE};
use crate::diplomacy::{NEUTRAL, WAR};
use crate::map::{T_GRASS, T_SHALLOW};

fn terrain_game(unit: DefId, local: u32) -> (Game, usize, usize, usize) {
    terrain_game_with_seats(unit, local, 2)
}

fn terrain_game_with_seats(unit: DefId, local: u32, humans: u32) -> (Game, usize, usize, usize) {
    let mut game = Game::new(7, 0, 0, humans, local);
    if humans == 2 { assert!(game.configure_peer_trade_route(true)); }
    assert!(game.trade_enabled);
    for team in 0..2 {
        game.players[team as usize].epoch = 2;
        game.rebuild_modifiers(team, false);
    }
    let terrain = if unit == TRADE_CART { T_GRASS } else { T_SHALLOW };
    game.map.ter.fill(terrain);
    for tile in 0..NN { game.map.upd_pass(tile); }
    let home = game.spawn(host_for(unit), 0, 63.5, 118.5);
    let destination = game.spawn(host_for(unit), 0, 83.5, 118.5);
    let trader = game.spawn(unit, 0, 72.5, 122.5);
    game.build_grid();
    game.update_fog();
    (game, home, destination, trader)
}

#[test]
fn source_clock_market_slot_accumulates_injected_delta_and_replays_paid_cargo() {
    let (mut game, home, destination, actor) = terrain_game_with_seats(TRADE_CART, 0, 1);
    let handle = game.h(actor);
    let initial = game.resource(0, GOLD);
    command(&mut game, home, destination, &[actor]);
    let quote = game.trade.routes[&handle].quote;
    for _ in 0..500 {
        assert!(game.tick_with_source_delta(0.075));
        if game.trade_hidden(actor, &game.ents[actor]) { break; }
    }
    assert!(game.trade_hidden(actor, &game.ents[actor]));
    let host = game.trade.routes[&handle].host();
    let prior = game.trade.hosts[&host].snapshot().1;
    assert!(game.tick_with_source_delta(0.025));
    assert_eq!(game.sim_step().to_bits(), 0.025f32.to_bits() - 1);
    let expected = (f64::from(prior) + f64::from(game.sim_step()) / 5.0) as f32;
    assert_eq!(game.trade.hosts[&host].snapshot().1, expected);
    let mut copy = reload(&mut game);
    let mut progress = expected;
    for delta in [0.025, 0.075, 0.125].into_iter().cycle().take(100) {
        assert!(game.tick_with_source_delta(delta));
        assert!(copy.tick_with_source_delta(delta));
        progress = (f64::from(progress) + f64::from(game.sim_step()) / 5.0) as f32;
        assert_eq!(copy.state_hash(), game.state_hash());
        assert_eq!(game.trade_hidden(actor, &game.ents[actor]), progress < 1.0);
        if progress >= 1.0 { break; }
        assert_eq!(game.trade.hosts[&host].snapshot().1, progress);
    }
    assert!(progress >= 1.0);
    assert_eq!(game.resource(0, GOLD), initial + quote.gold);
}

fn command(game: &mut Game, home: usize, destination: usize, traders: &[usize]) {
    let home = game.h(home);
    let destination = game.h(destination);
    let traders: Vec<_> = traders.iter().map(|&trader| game.h(trader)).collect();
    game.apply(0, C_TRADE_ROUTE, (home & 0xffff) as f32, (home >> 16) as f32, destination as i32, &traders);
}

fn advance_until(game: &mut Game, limit: usize, condition: impl Fn(&Game) -> bool) {
    for _ in 0..limit {
        if condition(game) { return; }
        game.tick();
    }
    assert!(condition(game), "condition unsatisfied at tick {}", game.tick);
}

fn reload(game: &mut Game) -> Game {
    game.save();
    let loaded = Game::load(&game.savebuf).expect("trade save should load");
    assert_eq!(loaded.state_hash(), game.state_hash(), "state changed on save at {}", game.tick);
    loaded
}

#[test]
fn both_traders_complete_two_legs_and_resume_from_travel_and_loading_saves() {
    for unit in [TRADE_CART, MERCHANT_SHIP] {
        let (mut game, home, destination, actor) = terrain_game(unit, 0);
        let handle = game.h(actor);
        let initial = game.resource(0, GOLD);
        command(&mut game, home, destination, &[actor]);
        let cargo = game.trade.routes[&handle].quote;
        let mut peer = reload(&mut game);
        for _ in 0..500 {
            assert_eq!(game.state_hash(), peer.state_hash());
            if game.trade_hidden(actor, &game.ents[actor]) { break; }
            game.tick();
            peer.tick();
        }
        assert!(game.trade_hidden(actor, &game.ents[actor]));
        assert!(!game.visible(actor));
        assert!(!game.can_see(0, actor));
        assert_eq!(game.resource(0, GOLD), initial);
        game.tick();
        let mut peer = reload(&mut game);
        for _ in 0..101 {
            assert_eq!(game.state_hash(), peer.state_hash());
            game.tick();
            peer.tick();
        }
        assert_eq!(game.resource(0, GOLD), initial + cargo.gold);
        let route = game.trade.routes[&handle];
        assert!(route.returning);
        assert!(game.visible(actor));
        advance_until(&mut game, 500, |g| !g.trade.routes[&handle].returning);
        assert_eq!(game.resource(0, GOLD), (initial + cargo.gold) + route.quote.gold);
    }
}

#[test]
fn production_is_paid_and_legacy_save_and_peer_keep_trade_disabled() {
    for unit in [TRADE_CART, MERCHANT_SHIP] {
        let (mut game, home, _, _) = terrain_game(unit, 0);
        let before = [game.resource(0, 0), game.resource(0, 1), game.resource(0, GOLD)];
        assert!(game.train(home, unit as QueueItem));
        let expected = if unit == TRADE_CART { [25.0, 25.0, 0.0] } else { [0.0, 40.0, 20.0] };
        let paid = [game.resource(0, 0), game.resource(0, 1), game.resource(0, GOLD)];
        assert_eq!(std::array::from_fn::<_, 3, _>(|i| before[i] - paid[i]), expected);
        let mut restored = reload(&mut game);
        advance_until(&mut restored, 660, |g| g.ents[home].qn == 0);
        assert_eq!(restored.ents[..restored.high].iter().filter(|e| e.alive && e.def == unit).count(), 2);

        let mut old = Game::new(7, 0, 0, 2, 0);
        assert!(!old.trade_enabled);
        old.save();
        assert_eq!(split_save(&old.savebuf).unwrap().1, None);
        let loaded = Game::load(&old.savebuf).unwrap();
        assert_eq!(old.state_hash(), loaded.state_hash());
        assert!(!loaded.trade_enabled);
        old.tick();
        assert!(!old.configure_peer_trade_route(true));
    }
}

#[test]
fn two_seats_apply_the_same_packet_and_render_queries_leave_state_unchanged() {
    let (mut left, home, destination, actor) = terrain_game(TRADE_CART, 0);
    let (mut right, _, _, _) = terrain_game(TRADE_CART, 1);
    let home = left.h(home);
    let destination = left.h(destination);
    let actor_handle = left.h(actor);
    left.submit(C_TRADE_ROUTE, (home & 0xffff) as f32, (home >> 16) as f32, destination as i32,
        &[actor_handle, actor_handle]);
    assert!(left.trade.routes.is_empty());
    left.inbox.clone_from(&left.outbox);
    right.inbox.clone_from(&left.outbox);
    left.net_apply(0);
    right.net_apply(0);
    let initial = left.resource(0, GOLD);
    assert_eq!(left.trade.routes.len(), 1);
    for _ in 0..480 {
        let state = left.state_hash();
        assert_eq!(right.state_hash(), state);
        let _ = right.visible(actor);
        let _ = left.can_see(0, actor);
        assert_eq!(left.state_hash(), state);
        left.tick();
        right.tick();
    }
    assert!(left.resource(0, GOLD) > initial);
}

#[test]
fn foreign_permission_uses_actual_research_and_hostility_aborts_an_incomplete_load() {
    let (mut game, home, destination, actor) = terrain_game(TRADE_CART, 0);
    let handle = game.h(actor);
    game.ents[destination].team = 1;
    game.team_vis[tile_of(83.5, 118.5)] = 128;
    game.diplomacy.relation[1] = NEUTRAL;
    game.diplomacy.relation[4] = NEUTRAL;
    let gold = game.resource(0, GOLD);
    let rng = game.source_birth.rng;
    command(&mut game, home, destination, &[actor]);
    assert_eq!(game.source_birth.rng, rng);
    assert!(game.trade.routes.is_empty());
    game.researched[0][FOREIGN_RESEARCH / 32] |= 1 << (FOREIGN_RESEARCH % 32);
    command(&mut game, home, destination, &[actor]);
    assert!(game.trade.routes.contains_key(&handle));
    advance_until(&mut game, 500, |g| g.trade_hidden(actor, &g.ents[actor]));
    game.diplomacy.relation[1] = WAR;
    game.diplomacy.relation[4] = WAR;
    game.tick();
    assert_eq!(game.resource(0, GOLD), gold);
    assert!(game.visible(actor));
    assert!(!game.trade.routes.contains_key(&handle));
}

#[test]
fn active_factors_apply_the_original_additive_percentage_and_host_credit() {
    let (mut game, home, destination, actor) = terrain_game(TRADE_CART, 0);
    assert_eq!(game.trade_gold_rate(0), 0.2);
    assert_eq!(game.trade_load_time(0), 5.0);
    game.civilizations[0] = 3; // American
    game.civilizations[1] = 8; // Egyptian
    assert_eq!(game.trade_gold_rate(0), 0.25);
    assert_eq!(game.trade_gold_rate(1), 0.22);
    for node in GOLD_RESEARCH.into_iter().chain(LOAD_RESEARCH).chain([FOREIGN_RESEARCH]) {
        game.researched[0][node / 32] |= 1 << (node % 32);
    }
    assert_eq!(game.trade_gold_rate(0), 0.31);
    assert_eq!(game.trade_load_time(0), 2.0);
    game.ents[destination].team = 1;
    game.team_vis[tile_of(83.5, 118.5)] = 128;
    game.diplomacy.relation[1] = NEUTRAL;
    game.diplomacy.relation[4] = NEUTRAL;
    let gold = [game.resource(0, GOLD), game.resource(1, GOLD)];
    command(&mut game, home, destination, &[actor]);
    let handle = game.h(actor);
    let quote = game.trade.routes[&handle].quote;
    assert!(quote.gold >= 6.2);
    advance_until(&mut game, 500, |g| g.trade_hidden(actor, &g.ents[actor]));
    let start = game.tick;
    advance_until(&mut game, 41, |g| !g.trade_hidden(actor, &g.ents[actor]));
    assert_eq!(game.tick - start, 41);
    let share = f64::from(0.2f32) * f64::from(quote.gold);
    let own = (f64::from((f64::from(gold[0]) + f64::from(quote.gold)) as f32) + share) as f32;
    assert_eq!(game.resource(0, GOLD), own);
    assert_eq!(game.resource(1, GOLD), (f64::from(gold[1]) + share) as f32);
}

#[test]
fn nearest_home_and_waiting_queue_survive_removal_and_explicit_orders() {
    let (mut game, home, destination, actor) = terrain_game(TRADE_CART, 0);
    let replacement = game.spawn(MARKET, 0, 61.5, 128.5);
    let destination_handle = game.h(destination);
    let actor_handle = game.h(actor);
    assert_eq!(game.trade_automatic_choices(0, destination_handle, &[actor]), [(actor, game.h(home))]);
    game.apply(0, C_TRADE_ROUTE, 65535.0, 65535.0, destination_handle as i32, &[actor_handle]);
    game.ents[home].hp = 0.0;
    game.tick();
    assert_eq!(game.trade.routes[&actor_handle].home, game.h(replacement));
    let port = game.trade_port(destination, TRADE_CART).unwrap();
    let second = game.spawn(TRADE_CART, 0, port.0, port.1);
    let third = game.spawn(TRADE_CART, 0, port.0, port.1);
    let second_handle = game.h(second);
    let third_handle = game.h(third);
    command(&mut game, replacement, destination, &[second, third]);
    advance_until(&mut game, 15, |g| g.trade.hosts.get(&destination_handle).is_some_and(|slot| slot.snapshot().2.len() == 1));
    assert_eq!(game.trade.hosts[&destination_handle].active().unwrap().actor, second_handle);
    let loaded = reload(&mut game);
    assert_eq!(loaded.trade.hosts[&destination_handle].snapshot().2[0].actor, third_handle);
    game.apply(0, C_STOP, 0.0, 0.0, 0, &[second_handle]);
    assert!(game.visible(second));
    assert!(!game.visible(third));
    game.apply(0, C_MOVE, 99.5, 118.5, 0, &[third_handle]);
    assert!(game.visible(third));
    assert!(!game.trade.hosts.contains_key(&destination_handle));
    assert!(!game.trade.routes.contains_key(&third_handle));
}
