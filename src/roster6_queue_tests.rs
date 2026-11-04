use crate::*;

fn fixture() -> (Game, usize, usize) {
    let mut game = Game::new(7, 0, 0, 1, 0);
    for e in &mut game.ents[..game.high] { e.sel = false; }
    game.players[0].res = [10000.0; 3];
    game.players[0].cap = 100;
    game.research_points[0] = 10000.0;
    let a = game.spawn(TC, 0, 50.0, 50.0);
    let b = game.spawn(TC, 0, 60.0, 50.0);
    for i in [a, b] { game.ents[i].built = 1.0; game.ents[i].sel = true; }
    (game, a, b)
}

#[test]
fn selected_job_uses_shortest_selected_producer() {
    let (mut game, a, b) = fixture();
    game.ents[a].q[..3].fill(ids::unit_job(WORKER)); game.ents[a].qn = 3;
    game.ents[b].q[0] = ids::unit_job(WORKER); game.ents[b].qn = 1;
    assert_eq!(submit_selected_queue_job(&mut game, ids::unit_job(WORKER)), 1);
    assert_eq!((game.ents[a].qn, game.ents[b].qn), (3, 2));
    assert_eq!(game.ents[b].q[1], ids::unit_job(WORKER));
}

#[test]
fn selected_research_has_no_global_fallback() {
    let (mut game, a, b) = fixture();
    game.ents[a].sel = false; game.ents[b].sel = false;
    assert_eq!(submit_selected_queue_job(&mut game, ids::research_job(0)), 0);
    assert!(game.can_train(a, ids::research_job(0)), "a valid unselected host exists");
    game.ents[b].sel = true;
    assert_eq!(submit_selected_queue_job(&mut game, ids::research_job(0)), 1);
    assert_eq!(game.ents[b].q[0], 0x10000);
    assert_eq!(game.ents[a].qn, 0);
}

#[test]
fn selected_job_rejects_full_incomplete_wrong_owner_and_independent_tech() {
    let (mut game, a, b) = fixture();
    game.ents[a].qn = 5;
    game.ents[b].built = 0.5;
    assert_eq!(submit_selected_queue_job(&mut game, ids::unit_job(WORKER)), 0);
    game.ents[a].qn = 0; game.ents[a].team = 1;
    assert_eq!(submit_selected_queue_job(&mut game, ids::unit_job(WORKER)), 0);
    game.ents[a].team = 0;
    assert_eq!(submit_selected_queue_job(&mut game, ids::tech_job(0)), 0);
    assert_eq!(submit_selected_queue_job(&mut game, 0x40000), 0);
}

#[test]
fn selected_queue_network_payload_preserves_tagged_research_and_epoch() {
    let (mut game, a, b) = fixture();
    game.ents[a].sel = false;
    game.net = true;
    assert_eq!(submit_selected_queue_job(&mut game, ids::research_job(0)), 1);
    assert_eq!(i32::from_le_bytes(game.outbox[10..14].try_into().unwrap()), 0x10000);
    assert_eq!(u32::from_le_bytes(game.outbox[14..18].try_into().unwrap()), game.h(b));
    game.outbox.clear();
    game.researched[0][0] = 0xfff;
    assert!(game.can_advance(0));
    assert_eq!(submit_selected_queue_job(&mut game, ids::EPOCH_JOB), 1);
    assert_eq!(i32::from_le_bytes(game.outbox[10..14].try_into().unwrap()), 0x30000);
    assert_eq!(game.ents[b].qn, 0, "lockstep queues apply only at the scheduled tick");
}
