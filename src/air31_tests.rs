use super::*;
use crate::cmd::C_AIR_MISSION;
use crate::map::{R_GOLD, R_OIL};

fn scenario(epoch: u8, local: u32) -> (Game, usize) {
    let mut game = Game::new(7, 0, 0, 2, local);
    assert!(game.configure_peer_air_missions(true));
    for team in 0..2 {
        game.players[team as usize].epoch = epoch;
        game.rebuild_modifiers(team, false);
        for slot in 0..9 { game.add_resource(team, slot, 1000.0); }
    }
    let airport = game.spawn(AIRFIELD, 0, 84.5, 118.5);
    game.build_grid();
    game.update_fog();
    (game, airport)
}

fn advance_until(game: &mut Game, limit: usize, condition: impl Fn(&Game) -> bool) {
    for _ in 0..limit {
        if condition(game) { return; }
        game.tick();
    }
    assert!(condition(game), "condition unsatisfied at tick {}", game.tick);
}

fn restored(game: &mut Game) -> Game {
    game.save();
    let mut copy = Game::load(&game.savebuf).expect("air mission archive should load");
    assert_eq!(copy.state_hash(), game.state_hash());
    copy.save();
    assert_eq!(copy.savebuf, game.savebuf);
    copy
}

#[test]
fn paid_aircraft_is_stationed_and_scrambles_after_a_save() {
    let (mut game, airport) = scenario(10, 0);
    let airport_handle = game.h(airport);
    let gold = R_GOLD as usize - 1;
    let oil = R_OIL as usize - 1;
    let before = [game.resource(0, 1), game.resource(0, gold), game.resource(0, oil)];
    assert!(game.train(airport, FIGHTER_BOMBER as _));
    assert_eq!([before[0] - game.resource(0, 1), before[1] - game.resource(0, gold),
        before[2] - game.resource(0, oil)], [60.0, 120.0, 90.0]);
    let mut game = restored(&mut game);
    advance_until(&mut game, 1180, |g| g.air_mission_status(airport_handle, 1) == 1);
    assert!(game.tick >= 1170);
    let handle = game.aircraft.airports[&airport_handle].stored[0];
    let index = game.get(handle).unwrap();
    assert_eq!(game.ents[index].def, FIGHTER_BOMBER);
    assert_eq!(game.air_mission_status(handle, 1), 14);
    assert!(!game.visible(index));
    let mut game = restored(&mut game);
    let start_rng = game.source_birth.rng;
    let mut expected = start_rng;
    launch_jitter(&mut expected);
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[airport_handle]);
    advance_until(&mut game, 20, |g| g.visible(index));
    assert_eq!(game.source_birth.rng, expected);
    assert_eq!(game.h(index), handle);
    assert_eq!(game.air_mission_status(handle, 5), 1);
    assert_eq!(game.air_mission_status(airport_handle, 1), 0);
    assert_eq!(restored(&mut game).air_mission_status(handle, 1), 14);
}

#[test]
fn five_waypoints_are_atomic_and_a_cancel_returns_the_same_plane() {
    let (mut game, airport) = scenario(12, 0);
    let airport_handle = game.h(airport);
    let actor = game.spawn(FIGHTER_BOMBER, 0, 84.5, 118.5);
    let handle = game.h(actor);
    assert!(game.air_station_produced(actor, airport));
    let route: [[f32; 2]; 5] = [[105.5, 118.5], [124.5, 118.5], [135.5, 123.5], [140.5, 126.5], [151.5, 128.5]];
    let mut packet = vec![airport_handle, NONE];
    packet.extend(route.iter().flatten().map(|v| v.to_bits()));
    let before = game.state_hash();
    game.apply(0, C_AIR_MISSION, 6.0, 1.0, SET_ROUTE, &packet);
    assert_eq!(game.state_hash(), before);
    let mut corrupt = packet.clone();
    corrupt[3] = f32::NAN.to_bits();
    game.apply(0, C_AIR_MISSION, 5.0, 1.0, SET_ROUTE, &corrupt);
    assert_eq!(game.state_hash(), before);
    game.apply(0, C_AIR_MISSION, 5.0, 1.0, SET_ROUTE, &packet);
    assert_eq!(game.air_mission_status(airport_handle, 3), 1);
    advance_until(&mut game, 20, |g| g.visible(actor));
    assert_eq!(game.air_mission_status(handle, 3), 1);
    advance_until(&mut game, 500, |g| g.ents[actor].x > 113.0);
    let mut game = restored(&mut game);
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, CANCEL, &[airport_handle]);
    assert_eq!(game.air_mission_status(airport_handle, 3), 0);
    assert_eq!(game.aircraft.aircraft[&handle].phase, Phase::Returning);
    assert_eq!(game.aircraft.aircraft[&handle].waypoints[0], route[4]);
    let mut game = restored(&mut game);
    advance_until(&mut game, 2500, |g| !g.visible(actor));
    assert_eq!(game.air_mission_status(airport_handle, 1), 1);
    assert_eq!(game.h(actor), handle);
}

#[test]
fn aircraft_attack_uses_finite_rounds_then_rearms_at_an_airport_pulse() {
    let (mut game, airport) = scenario(10, 0);
    let airport_handle = game.h(airport);
    let actor = game.spawn(FIGHTER_BOMBER, 0, 90.5, 118.5);
    let handle = game.h(actor);
    assert!(game.air_station_produced(actor, airport));
    let target = game.spawn(AIRFIELD, 1, 104.5, 118.5);
    game.ents[target].hp = 100_000.0;
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[airport_handle]);
    advance_until(&mut game, 20, |g| g.visible(actor));
    for _ in 0..4000 {
        game.team_vis[crate::map::tile_of(104.5, 118.5)] = 255;
        if game.air_mission_status(handle, 1) == 0 { break; }
        if game.ents[actor].order != O_ATTACK { game.order_attack(&[actor], target); }
        game.tick();
    }
    assert_eq!(game.air_mission_status(handle, 1), 0);
    assert!(game.ents[target].hp < 100_000.0);
    assert_eq!(game.aircraft.aircraft[&handle].phase, Phase::Returning);
    advance_until(&mut game, 2000, |g| !g.visible(actor));
    assert_eq!(game.air_mission_status(handle, 1), 0);
    game.save();
    let (_, air) = split_air_save(&game.savebuf).unwrap();
    assert!(State::decode(&game, air.unwrap()).is_some(), "AIR1 payload roundtrips on the source game");
    let mut projectile_bytes = Vec::new();
    game.append_projectiles(&mut projectile_bytes);
    let (mut probe, _) = scenario(10, 0);
    assert!(probe.read_projectiles(Some(&projectile_bytes)).is_some(), "projectile payload roundtrips");
    let mut game = restored(&mut game);
    advance_until(&mut game, 20, |g| g.air_mission_status(handle, 1) == 14);
    assert_eq!(game.air_mission_status(airport_handle, 1), 1);
}

#[test]
fn airborne_planes_reserve_their_seat_and_rebase_after_home_loss() {
    let (mut game, airport) = scenario(10, 0);
    let backup = game.spawn(AIRFIELD, 0, 118.5, 118.5);
    let home = game.h(airport);
    let backup_handle = game.h(backup);
    let actors: Vec<_> = (0..AIRPORT_CAPACITY).map(|_| {
        let actor = game.spawn(FIGHTER_BOMBER, 0, 84.5, 118.5);
        assert!(game.air_station_produced(actor, airport));
        actor
    }).collect();
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[home]);
    advance_until(&mut game, 20, |g| g.visible(actors[0]));
    assert_eq!(game.aircraft.airports[&home].registered.len(), AIRPORT_CAPACITY);
    assert_eq!(game.aircraft.airports[&home].stored.len(), AIRPORT_CAPACITY - 1);
    let extra = game.spawn(FIGHTER_BOMBER, 0, 84.5, 118.5);
    assert!(!game.air_station_produced(extra, airport));
    assert!(game.visible(extra));
    let flying = game.h(actors[0]);
    assert_eq!(restored(&mut game).aircraft.airports[&home].registered[0], flying);
    game.ents[airport].hp = 0.0;
    game.tick();
    assert!(actors[1..].iter().all(|&actor| !game.ents[actor].alive));
    assert_eq!(game.aircraft.aircraft[&flying].home, backup_handle);
    assert_eq!(game.aircraft.aircraft[&flying].phase, Phase::Returning);
    advance_until(&mut game, 1200, |g| g.air_mission_status(backup_handle, 1) == 1);
    assert_eq!(game.aircraft.airports[&backup_handle].stored, [flying]);
}

#[test]
fn carrier_reserves_a_seat_in_flight_rearms_without_repair_and_rebases_on_loss() {
    for (epoch, seats) in [(10, 4), (13, 6)] {
        let (mut game, airport) = scenario(epoch, 0);
        assert!(!game.configure_peer_air_carrier(true));
        assert!(game.configure_peer_air_runway(true));
        assert!(game.configure_peer_air_takeoff(true));
        assert!(game.configure_peer_air_landing(true));
        assert!(game.configure_peer_air_carrier(true));
        let carrier = game.spawn(CARRIER, 0, 97.5, 120.5);
        let host = game.h(carrier);
        let actors: Vec<_> = (0..seats).map(|_| game.spawn(FIGHTER_BOMBER, 0, 97.5, 120.5)).collect();
        let mut air = std::mem::take(&mut game.aircraft);
        for &index in &actors { let handle = game.h(index); assert!(air.station(&mut game, handle, host)); }
        game.aircraft = air;
        assert_eq!(game.air_mission_status(host, 2), seats);
        assert_eq!(game.air_mission_status(host, 1), seats);
        let handle = game.h(actors[0]);
        let health = game.ents[actors[0]].hp * 0.5;
        game.ents[actors[0]].hp = health;
        game.aircraft.aircraft.get_mut(&handle).unwrap().ordnance = 1;
        let mut game = restored(&mut game);
        let pulse = (game.tick + 1..=game.tick + 20).find(|&tick| actor_pulse(tick, phase_key(host, 0), 1.0, crate::game::DT)).unwrap();
        advance_until(&mut game, 20, |g| g.air_mission_status(handle, 1) != 1);
        assert_eq!(game.tick, pulse);
        assert_eq!(game.ents[actors[0]].hp, health);
        assert_eq!(game.air_mission_status(handle, 1), capacity(FIGHTER_BOMBER, epoch).unwrap().into());
        game.air_scramble(carrier);
        advance_until(&mut game, 20, |g| !g.air_parked(actors[0], &g.ents[actors[0]]));
        assert_eq!(game.air_mission_status(host, 1), seats - 1);
        let extra = game.spawn(FIGHTER_BOMBER, 0, 97.5, 120.5);
        game.air_order_land(&[extra], carrier);
        assert_eq!(game.aircraft.aircraft[&game.h(extra)].home, NONE);
        let mut game = restored(&mut game);
        game.ents[carrier].hp = 0.0;
        game.tick();
        assert!(actors[1..].iter().all(|&index| !game.ents[index].alive));
        assert_eq!(game.aircraft.aircraft[&handle].home, game.h(airport));
        assert_eq!(game.aircraft.aircraft[&handle].phase, Phase::Returning);
    }
}

#[test]
fn carrier_notified_when_an_appended_return_becomes_active() {
    let (mut game, _) = scenario(10, 0);
    assert!(game.configure_peer_air_runway(true));
    assert!(game.configure_peer_air_takeoff(true));
    assert!(game.configure_peer_air_landing(true));
    assert!(game.configure_peer_air_carrier(true));
    let carrier = game.spawn(CARRIER, 0, 97.5, 120.5);
    let host = game.h(carrier);
    let actor = game.spawn(FIGHTER_BOMBER, 0, 107.5, 120.5);
    let handle = game.h(actor);
    game.order_move(&[carrier], 112.0, 122.0, false);
    let mut air = std::mem::take(&mut game.aircraft);
    air.aircraft.get_mut(&handle).unwrap().waypoints.push_back([107.5, 120.5]);
    air.return_to(&mut game, handle, host, ReturnPath::AfterCurrent);
    game.aircraft = air;
    assert!(game.aircraft.aircraft[&handle].rendezvous_pending);
    assert_ne!(game.ents[carrier].order, O_RENDEZVOUS);
    let mut game = restored(&mut game);
    game.tick();
    assert!(!game.aircraft.aircraft[&handle].rendezvous_pending);
    assert_eq!(game.ents[carrier].order, O_RENDEZVOUS);
    assert_eq!(game.aircraft.aircraft[&handle].waypoints.len(), 1);
}

#[test]
fn empty_aircraft_without_a_pending_return_uses_the_two_second_actor_pulse() {
    let (mut game, airport) = scenario(10, 0);
    let home = game.h(airport);
    let index = game.spawn(37, 0, 84.5, 118.5);
    let handle = game.h(index);
    assert!(game.air_station_produced(index, airport));
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[home]);
    advance_until(&mut game, 20, |g| g.visible(index));
    game.ents[index].x = 130.5;
    game.aircraft.aircraft.get_mut(&handle).unwrap().ordnance = 1;
    game.air_fire(index, false);
    assert!(!game.aircraft.aircraft[&handle].return_pending);
    let expected = (game.tick + 1..=game.tick + 40)
        .find(|&tick| actor_pulse(tick, phase_key(handle, 0), 2.0, crate::game::DT)).unwrap();
    advance_until(&mut game, 40, |g| g.aircraft.aircraft[&handle].phase == Phase::Returning);
    assert_eq!(game.tick, expected);
}

#[test]
fn a_real_launch_uses_both_airport_runway_markers_and_preserves_the_rng_draws() {
    let (mut game, airport) = scenario(10, 0);
    assert!(game.configure_peer_air_runway(true));
    game.ents[airport].rot = std::f32::consts::FRAC_PI_4;
    let host = game.h(airport);
    let actor = game.spawn(FIGHTER_BOMBER, 0, 84.5, 118.5);
    let handle = game.h(actor);
    assert!(game.air_station_produced(actor, airport));
    let pose = crate::runway32::departure(&game.ents[airport]);
    let mut rng = game.source_birth.rng;
    let jitter = launch_jitter(&mut rng);
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[host]);
    advance_until(&mut game, 20, |g| g.visible(actor));
    let e = game.ents[actor];
    assert_eq!((e.x, e.y, e.rot), (pose.x, pose.y, pose.heading));
    assert_eq!((e.px, e.py, e.prot), (pose.x, pose.y, pose.heading));
    assert_eq!((e.ox, e.oy), (game.ents[airport].x + jitter, game.ents[airport].y + jitter));
    assert_eq!(game.source_birth.rng, rng);
    assert_eq!(game.h(actor), handle);
    assert!(restored(&mut game).air_runway_enabled);
}

#[test]
fn visual_route_snapshot_hides_an_unseen_actor_and_preserves_simulation_state() {
    let (mut game, airport) = scenario(10, 0);
    let host = game.h(airport);
    let enemy = game.spawn(AIRFIELD, 1, 110.5, 119.5);
    let target = game.h(enemy);
    let tile = crate::map::tile_of(game.ents[enemy].x, game.ents[enemy].y);
    game.team_vis[tile] = 255;
    assert!(game.air_assign(airport, Some(Mission { waypoints: vec![[120.0, 119.0]], target: Some(target) })));
    let before = game.state_hash();
    let snapshot = game.air_mission_visual_route(host).unwrap();
    assert_eq!((snapshot.host, snapshot.target, snapshot.team, snapshot.count), (host, target, 0, 1));
    assert_eq!(&snapshot.points[..2], &[120.0, 119.0]);
    assert_eq!(game.state_hash(), before);
    game.team_vis[tile] = 128;
    assert_eq!(game.air_mission_visual_route(host).unwrap().target, NONE);
    assert!(game.air_mission_visual_route(target).is_none());
}

#[test]
fn source_clock_aircraft_preserves_takeoff_audio_and_landing_across_saves() {
    let mut game = Game::new(7, 1, 0, 1, 0);
    game.players[0].epoch = 10;
    game.rebuild_modifiers(0, false);
    let airport = game.spawn(AIRFIELD, 0, 84.5, 118.5);
    let home = game.h(airport);
    let actor = game.spawn(FIGHTER_BOMBER, 0, 84.5, 118.5);
    let handle = game.h(actor);
    assert!(game.air_station_produced(actor, airport));
    game.build_grid();
    game.update_fog();
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, SCRAMBLE, &[home]);
    let mut steps = [1.0 / 30.0, 0.041, 0.025].into_iter().cycle();
    for _ in 0..50 {
        assert!(game.tick_with_source_delta(steps.next().unwrap()));
        if game.visible(actor) { break; }
    }
    assert!(game.visible(actor));
    assert!(game.takeoff.rolling(handle));
    assert_eq!(game.takeoff_start_seconds(handle), None);
    game.prepare_air_audio(1.0);
    assert!(game.air_audio.current.is_empty());
    let mut copy = restored(&mut game);
    assert_eq!(copy.takeoff_start_seconds(handle), None);
    let before = game.ents[actor];
    assert!(game.tick_with_source_delta(0.075));
    assert!(copy.tick_with_source_delta(0.075));
    assert_eq!(game.state_hash(), copy.state_hash());
    let started = game.sim_elapsed();
    assert_eq!(game.takeoff_start_seconds(handle), Some(started));
    assert_eq!(game.ents[actor].x, (f64::from(before.x) + f64::from(game.ents[actor].vx) * f64::from(game.sim_step())) as f32);
    game.prepare_air_audio(1.0);
    assert_eq!(game.air_audio.current.len(), 1);
    assert_eq!(game.air_audio.transition_seconds(0), Some(started));
    let mut game = restored(&mut game);
    assert_eq!(game.takeoff_start_seconds(handle), Some(started));
    assert_eq!(game.air_audio.transition_seconds(0), None);
    game.prepare_air_audio(1.0);
    assert_eq!(game.air_audio.current.len(), 1);
    let [first, second] = game.air_host_markers(&game.ents[airport]);
    let dx = second[0] - first[0];
    let dy = second[1] - first[1];
    let scale = dx.hypot(dy);
    let target = [84.5 + (dx * 7.0 - dy * 30.0) / scale, 118.5 + (dy * 7.0 + dx * 30.0) / scale];
    game.order_move(&[actor], target[0], target[1], false);
    for _ in 0..600 {
        assert!(game.tick_with_source_delta(steps.next().unwrap()));
        if (game.ents[actor].x - target[0]).hypot(game.ents[actor].y - target[1]) < 8.0 { break; }
    }
    assert!(!game.takeoff.rolling(handle));
    let cruise = game.takeoff_height(handle, 1.0).unwrap();
    assert!(cruise > game.map.height(game.ents[actor].x, game.ents[actor].y).max(0.0) + 4.0);
    game.apply(0, C_AIR_MISSION, 0.0, 0.0, RECALL, &[handle]);
    for _ in 0..1800 {
        assert!(game.tick_with_source_delta(steps.next().unwrap()));
        if game.landing.status(handle, 0) == 4 { break; }
    }
    assert_eq!(game.landing.status(handle, 0), 4);
    let mut copy = restored(&mut game);
    let mut descent = false;
    for step in steps.take(1800) {
        assert!(game.tick_with_source_delta(step));
        assert!(copy.tick_with_source_delta(step));
        assert_eq!(game.state_hash(), copy.state_hash());
        let left = game.ents[actor];
        let right = copy.ents[actor];
        assert_eq!([left.x, left.y, left.rot, left.vx, left.vy], [right.x, right.y, right.rot, right.vx, right.vy]);
        assert_eq!(game.takeoff_height(handle, 1.0), copy.takeoff_height(handle, 1.0));
        if !game.visible(actor) { break; }
        descent |= game.landing.status(handle, 0) == 5;
    }
    assert!(descent);
    assert!(!game.visible(actor));
    assert_eq!(game.air_mission_status(home, 1), 1);
    assert_eq!(game.h(actor), handle);
}
