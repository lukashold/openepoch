use super::*;

fn open_row(game: &Game) -> (usize, usize) {
    for y in 24..N - 24 {
        for x in 24..N - 24 {
            let empty = game.ents[..game.high].iter().all(|actor| {
                !actor.alive || (actor.x - x as f32).abs().max((actor.y - y as f32).abs()) > 12.0
            });
            if empty && (x..=x + 8).all(|column| game.map.pass[0][y * N + column]) {
                return (x, y);
            }
        }
    }
    panic!("fixture needs an open row")
}

fn move_once(seconds: f64) -> (f32, f32, f32, f32) {
    let mut game = Game::new(18, 1, 0, 1, 0);
    let (x, y) = open_row(&game);
    let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
    let worker = game.spawn(WORKER, 0, x, y);
    game.order_move(&[worker], x + 8.0, y, false);
    assert_eq!(game.ents[worker].order, O_MOVE);
    assert!(game.tick_with_source_delta(seconds));
    let actor = game.ents[worker];
    (x, actor.x, actor.vx, game.sim_step())
}

#[test]
fn commanded_land_motion_uses_injected_step_for_displacement() {
    let short = move_once(0.025);
    let long = move_once(0.075);
    assert!(short.2 > 0.0);
    assert_eq!(short.2, long.2);
    assert_eq!(short.1, short.0 + short.2 * short.3);
    assert_eq!(long.1, long.0 + long.2 * long.3);
    assert!(long.1 > short.1);
}

fn gather_once(seconds: f64) -> f32 {
    let mut game = Game::new(18, 1, 0, 1, 0);
    let (x, y) = open_row(&game);
    let tile = y * N + x;
    game.map.res[tile] = R_WOOD;
    game.map.amt[tile] = 100;
    let worker = game.spawn(WORKER, 0, x as f32 + 0.5, y as f32 + 0.5);
    game.order_gather(&[worker], tile);
    assert_eq!(game.ents[worker].order, O_GATHER);
    assert!(game.tick_with_source_delta(seconds));
    game.ents[worker].carry
}

#[test]
fn commanded_gather_uses_injected_step_before_cargo_floor() {
    let short = gather_once(0.025);
    let long = gather_once(0.075);
    assert!(short > 0.0);
    assert!((long - 3.0 * short).abs() < 1.0e-6);
}

#[test]
fn paid_production_keeps_first_step_and_uses_injected_time_afterward() {
    let mut game = Game::new(18, 1, 0, 1, 0);
    let city = game.ents[..game.high].iter().position(|actor| {
        actor.alive && actor.team == 0 && actor.def == TC
    }).expect("initial city");
    assert!(game.train(city, WORKER as QueueItem));
    assert!(game.tick_with_source_delta(0.075));
    let first = if cfg!(feature = "ee2-paidqueue7") { 0.01 } else { game.sim_step() };
    assert_eq!(game.ents[city].qt, first);
    assert!(game.tick_with_source_delta(0.025));
    assert_eq!(game.ents[city].qt, first + game.sim_step());
}

#[cfg(feature = "ee2-native13")]
#[test]
fn commanded_attack_round_fires_on_the_absolute_source_deadline_after_restore() {
    use crate::native_attack_clock::{EXECUTED, STARTED};

    let mut game = Game::new(18, 1, 0, 1, 0);
    let (x, y) = open_row(&game);
    let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
    let attacker = game.spawn(9, 0, x, y);
    let defender = game.spawn(WORKER, 1, x + 1.0, y);
    game.ents[defender].hp = 1000.0;
    game.update_fog();
    game.order_attack(&[attacker], defender);
    assert_eq!(game.ents[attacker].order, O_ATTACK);
    assert!(game.tick_with_source_delta(0.075));
    let weapon = DEFS[9].w[0];
    let reload = crate::ee2_units::lookup_class(9, 0).unwrap().reload.unwrap_or(weapon.cd);
    let delay = crate::ee2_up15_attack::lookup(9, 0).unwrap().apply_time;
    let round = game.attack_clock[attacker];
    assert_eq!(round.flags & (STARTED | EXECUTED), STARTED);
    assert_eq!(round.next, (f64::from(game.sim_elapsed()) + f64::from(reload)) as f32);
    assert_eq!(game.ents[defender].hp, 1000.0);
    game.save();
    let mut copy = Game::load(&game.savebuf).expect("source attack save");
    copy.save();
    assert_eq!(copy.savebuf, game.savebuf);

    let mut hit = false;
    for step in [0.025, 0.075].into_iter().cycle().take(12) {
        assert!(game.tick_with_source_delta(step));
        assert!(copy.tick_with_source_delta(step));
        assert_eq!(copy.state_hash(), game.state_hash());
        hit = f64::from(round.next) - f64::from(game.sim_elapsed()) < f64::from(reload) - f64::from(delay);
        assert_eq!(game.attack_clock[attacker].flags & EXECUTED != 0, hit);
        assert_eq!(game.ents[defender].hp < 1000.0, hit);
        if hit { break; }
    }
    assert!(hit);
}

#[cfg(feature = "ee2-native13")]
#[test]
fn named_projectile_moves_with_source_delta_and_retains_launch_attribution() {
    let mut game = Game::new(18, 1, 0, 1, 0);
    let (x, y) = open_row(&game);
    let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
    let attacker = game.spawn(10, 0, x, y);
    let defender = game.spawn(HOUSE, 1, x + 5.0, y);
    game.ents[defender].hp = 1000.0;
    game.update_fog();
    assert!(game.tick_with_source_delta(0.025));
    let mut source = game.ents[attacker];
    game.fire(attacker, &mut source, &DEFS[10].w[0], defender);
    let launch = *game.projs.last().expect("native named projectile");
    assert_eq!(launch.attack_source, game.h(attacker));
    assert_eq!(launch.speed, 6.0);
    assert!(game.tick_with_source_delta(0.075));
    let projectile = game.projs[0];
    assert_eq!(projectile.px, launch.x);
    let dx = projectile.tx - launch.x;
    let dy = projectile.ty - launch.y;
    let length = (dx * dx + dy * dy).sqrt();
    let travelled = launch.speed * game.sim_step();
    assert_eq!(projectile.x, launch.x + dx / length * travelled);
    assert_eq!(projectile.y, launch.y + dy / length * travelled);
    assert_ne!(projectile.x, launch.x + dx / length * (launch.speed * DT));
    game.save();
    let mut copy = Game::load(&game.savebuf).expect("source projectile save");
    assert_eq!(copy.projs[0].attack_source, launch.attack_source);
    copy.save();
    assert_eq!(copy.savebuf, game.savebuf);
    for step in [0.025, 0.075].into_iter().cycle().take(20) {
        assert!(game.tick_with_source_delta(step));
        assert!(copy.tick_with_source_delta(step));
        assert_eq!(copy.state_hash(), game.state_hash());
        if game.projs.is_empty() { break; }
    }
    assert!(game.projs.is_empty());
    assert!(game.ents[defender].hp < 1000.0);
}
