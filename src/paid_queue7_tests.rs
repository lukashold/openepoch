use super::*;
use crate::paid_queue7;

fn game(epoch:u8)->(Game,usize,usize){
    let mut g=Game::new(7,0,0,2,0);
    for i in 0..g.high{g.kill(i);}
    g.territory.claims.fill(NONE);
    for p in &mut g.players[..2]{p.epoch=epoch;p.ai=false;p.res=[100000.0;3];p.population_limit=1000;}
    g.extra_resources=[[100000.0;6];4];g.research_points=[10000.0;4];
    for t in 0..2{g.rebuild_modifiers(t,false);}
    let tc=g.spawn(TC,0,30.5,30.5);g.ents[tc].built=1.0;
    let dock=g.spawn(DOCK,0,100.5,100.5);g.ents[dock].built=1.0;
    g.build_grid();g.refresh_display_fog();(g,tc,dock)
}
fn stock(g:&Game)->[f32;9]{core::array::from_fn(|r|g.resource(0,r))}

#[cfg(feature = "ee2-native13")]
#[test]
fn late_population_keeps_two_paid_heads_blocked_then_admits_only_one() {
    let mut game = Game::new(7, 1, 0, 1, 0);
    for i in 0..game.high { game.kill(i); }
    game.players[1].ai = false;
    game.players[0].population_limit = 3;
    game.players[0].res = [1000.0; 3];
    game.extra_resources[0] = [1000.0; 6];
    let town = game.spawn(TC, 0, 64.5, 64.5);
    game.ents[town].built = 1.0;
    let mut mills = [game.spawn(MILL, 0, 70.5, 64.5), game.spawn(MILL, 0, 76.5, 64.5)];
    mills.sort_unstable();
    let [first, second] = mills;
    for mill in [first, second] { game.ents[mill].built = 1.0; }
    let workers = [62.0, 64.0, 66.0].map(|x| game.spawn(WORKER, 0, x, 68.5));
    game.build_grid();
    assert_eq!((game.players[0].pop, game.players[0].cap), (3, 3));
    assert!(!game.can_train(first, 22));
    assert!(game.configure_late_population());
    assert!(!game.configure_late_population());

    let before = stock(&game);
    for mill in [first, second] {
        assert!(game.train(mill, 22));
        let (duration, _) = game.begin_paid_job(mill);
        game.ents[mill].qt = duration;
    }
    let paid = game.queue_payment(first, 0);
    let charged = stock(&game);
    for (slot, original) in before.into_iter().enumerate() {
        assert_eq!(charged[slot], original - 2.0 * paid.price[paid_queue7::NATIVE_IDS[slot]]);
    }
    game.tick();
    assert_eq!(game.players[0].pop, 3);
    for mill in [first, second] {
        assert_eq!(game.ents[mill].qn, 1);
        assert_eq!(game.ents[mill].qt, paid.duration);
    }
    assert_eq!(stock(&game), charged);
    game.save();
    let blocked = game.savebuf.clone();
    let mut resumed = Game::load(&blocked).unwrap();
    assert!(resumed.late_population);
    assert_eq!(resumed.state_hash(), game.state_hash());
    resumed.save();
    assert_eq!(resumed.savebuf, blocked);

    let mut cancelled = Game::load(&blocked).unwrap();
    assert!(cancelled.cancel_paid_job(first, 0));
    assert_eq!(cancelled.ents[first].qt, 0.0);
    assert_eq!(cancelled.ents[second].qt, paid.duration);
    for (slot, balance) in charged.into_iter().enumerate() {
        assert_eq!(cancelled.resource(0, slot), balance + paid.price[paid_queue7::NATIVE_IDS[slot]]);
    }

    for world in [&mut game, &mut resumed] {
        for worker in workers { world.kill(worker); }
        world.tick();
        assert_eq!(world.players[0].pop, 3);
        assert_eq!(world.ents[first].qn, 0);
        assert_eq!(world.ents[second].qn, 1);
        assert_eq!(world.ents[second].qt, paid.duration);
        assert_eq!(stock(world), charged);
        world.save();
    }
    assert_eq!(resumed.state_hash(), game.state_hash());
    assert_eq!(resumed.savebuf, game.savebuf);
}

#[test]
fn active_sail_keeps_paid_resource_mix_until_main_obsolete_cancel(){
    let(mut g,_,dock)=game(7);let before=stock(&g);assert!(g.train(dock,58));
    let paid=g.queue_payment(dock,0);assert!(!paid.inferred&&paid.purchased);
    assert_eq!(paid.price,[0.0,200.0,0.0,160.0,0.0,290.0,0.0,0.0,0.0,0.0,3.0]);
    g.ents[dock].qt=1.0;g.players[0].epoch=10;g.rebuild_modifiers(0,true);
    assert_ne!(g.unit_cost(0,58),[0,200,290,0,160,0,0,0,0]);
    g.retire_paid_production(0,9);assert_eq!(g.ents[dock].qn,0);assert_eq!(stock(&g),before);
    assert!(g.queue_prices.is_empty());
}

#[cfg(feature="ee2-native13")]
#[test]
fn source_roster_retires_paid_jobs_only_when_the_source_family_expires() {
    let (mut g, _, dock) = game(4);
    g.production_roster_enabled = true;
    let before = stock(&g);
    assert!(g.train(dock, 29));
    assert!(g.train(dock, 29));
    let _ = g.begin_paid_job(dock);
    g.ents[dock].qt = 3.0;
    g.players[0].epoch = 5;
    g.rebuild_modifiers(0, true);
    g.apply_paid_main(0, 4);
    assert_eq!(g.ents[dock].qn, 0);
    assert_eq!(g.ents[dock].qt, 0.0);
    assert_eq!(stock(&g), before);
    assert!(!g.can_train(dock, 29));
    assert!(g.can_train(dock, 30));

    let (mut g, _, _) = game(2);
    g.production_roster_enabled = true;
    let stable = g.spawn(STABLE, 0, 60.5, 60.5);
    g.ents[stable].built = 1.0;
    assert!(g.train(stable, 18));
    let _ = g.begin_paid_job(stable);
    g.ents[stable].qt = 3.0;
    let paid = g.queue_payment(stable, 0);
    let after = stock(&g);
    g.players[0].epoch = 3;
    g.rebuild_modifiers(0, true);
    g.apply_paid_main(0, 2);
    assert_eq!(g.ents[stable].q[0], 18);
    assert_eq!(g.ents[stable].qt, 3.0);
    assert_eq!(g.queue_payment(stable, 0).price, paid.price);
    assert_eq!(stock(&g), after);
    assert!(!g.can_train(stable, 18));
    assert!(g.can_train(stable, 19));
    assert!(g.train(stable, 19));
    assert!(g.train(stable, 21));
    g.players[0].epoch = 10;
    g.rebuild_modifiers(0, true);
    g.apply_paid_main(0, 9);
    assert_eq!(g.ents[stable].qn, 0);
    assert!(!g.can_train(stable, 19));
    assert!(!g.can_train(stable, 21));
    assert!(g.can_train(stable, 23));
    assert!(g.can_train(stable, 26));
}
#[test]
fn research_discount_does_not_retroactively_reduce_captured_scout_refund(){
    let(mut g,tc,_)=game(0);let before=stock(&g);assert!(g.train(tc,57));
    assert_eq!(g.queue_payment(tc,0).price[0],30.0);
    g.researched[0][0]|=1<<3;g.rebuild_modifiers(0,false);assert_eq!(g.unit_cost(0,57)[0],27);
    g.cancel(tc);assert_eq!(stock(&g),before);
}
#[test]
fn mixed_jobs_keep_positions_prices_and_head_progress_when_middle_is_cancelled(){
    let(mut g,tc,_)=game(0);
    for k in 6..12{g.researched[0][k/32]|=1<<(k%32);}
    for job in[57,8,ORIGINAL_RESEARCH0,ORIGINAL_RESEARCH0+3,RESEARCH]{assert!(g.train(tc,job));}
    let before=g.research_points[0];g.ents[tc].qt=5.0;
    assert!(g.cancel_paid_job(tc,2));assert_eq!(g.research_points[0],before+8.0);
    assert_eq!(g.ents[tc].q[..4],[57,8,ORIGINAL_RESEARCH0+3,RESEARCH]);assert_eq!(g.ents[tc].qt,5.0);
    for i in 0..4{let p=g.queue_payment(tc,i);assert_eq!(p.position,i as u32);assert_eq!(p.job,g.ents[tc].q[i]);assert!(!p.inferred);}
    g.ents[tc].qt=g.production_time(0,57);g.tick();assert_eq!(g.ents[tc].q[0],8);
    assert_eq!(g.queue_payment(tc,0).job,8);assert_eq!(g.canonical_queue_payments().len(),3);
}
#[test]
fn research_and_epoch_refund_the_points_actually_paid(){
    let(mut g,tc,_)=game(0);let points=g.research_points[0];
    assert!(g.train(tc,ORIGINAL_RESEARCH0));g.cancel(tc);assert_eq!(g.research_points[0],points);
    for k in 6..12{g.researched[0][k/32]|=1<<(k%32);}
    assert!(g.train(tc,RESEARCH));let paid=g.queue_payment(tc,0).price[9];
    g.players[0].epoch=7;g.cancel(tc);assert_eq!(g.research_points[0],points);
    assert!(paid>0.0);assert!(!g.players[0].researching);
}
#[test]
fn kill_and_generation_recycling_cannot_reuse_a_payment(){
    let(mut g,tc,_)=game(0);assert!(g.train(tc,57));let old=g.h(tc);let bank=stock(&g);
    g.kill(tc);assert!(g.queue_prices.is_empty());assert_eq!(stock(&g),bank);
    let recycled=g.spawn(TC,0,30.5,30.5);assert_eq!(recycled,tc);assert_ne!(g.h(recycled),old);
    g.ents[recycled].built=1.0;assert!(g.train(recycled,8));assert_eq!(g.queue_payment(recycled,0).job,8);
}
#[test]
fn paid_queue_roundtrip_preserves_price_flags_and_rejects_forged_bindings(){
    let(mut g,tc,dock)=game(7);assert!(g.train(tc,57));assert!(g.train(dock,58));g.build_grid();g.refresh_display_fog();
    g.save();let good=g.savebuf.clone();let mut loaded=Game::load(&good).unwrap();
    assert_eq!(loaded.queue_prices,g.canonical_queue_payments());assert_eq!(loaded.state_hash(),g.state_hash());loaded.save();assert!(loaded.savebuf==good);
    #[cfg(feature="ee2-pack8")]let inner=crate::pack8_save::split(&good).unwrap().0;
    #[cfg(not(feature="ee2-pack8"))]let inner=&good[..];
    #[cfg(feature="ee2-pack8")]let base=crate::pack8_save::HEADER;
    #[cfg(not(feature="ee2-pack8"))]let base=0;
    let end=inner.len()-52-if cfg!(feature="ee2-civ8"){36}else{0};let size=u32::from_le_bytes(inner[end-8..end-4].try_into().unwrap())as usize;let start=base+end-size;
    for(offset,value)in[(0,0),(4,3),(8,3),(12,u32::MAX),(16,9),(20,u32::MAX),(24,4),(28,f32::NAN.to_bits()),(28,(-1.0_f32).to_bits()),(72,f32::NAN.to_bits()),(72,(-2.0_f32).to_bits())]{
        let mut bad=good.clone();bad[start+offset..start+offset+4].copy_from_slice(&value.to_le_bytes());assert!(Game::load(&bad).is_none(),"forged offset{offset}");
    }
    assert_eq!(u32::from_le_bytes(inner[..4].try_into().unwrap()),if cfg!(feature="ee2-citizen9"){0x3955454f}else if cfg!(feature="ee2-civ8"){0x3855454f}else{paid_queue7::SAVE_MAGIC});
}

#[test]
fn class_reprice_converts_pending_resources_but_keeps_active_price(){
    let(mut g,_,dock)=game(7);for _ in 0..5{assert!(g.train(dock,58));}
    let head=g.queue_payment(dock,0);g.ents[dock].qt=1.0;
    let before=stock(&g);g.players[0].epoch=9;g.rebuild_modifiers(0,false);g.apply_paid_main(0,8);
    assert_eq!(g.ents[dock].qn,4);assert_eq!(g.queue_payment(dock,0),head);assert_eq!(g.ents[dock].qt,1.0);
    for p in 1..4{assert_eq!(g.queue_payment(dock,p).price,[0.0,207.0,0.0,170.0,0.0,0.0,307.0,0.0,0.0,0.0,3.0]);}
    assert_eq!(g.resource(0,1),before[1]+179.0);
    assert_eq!(g.resource(0,4),before[4]+99.5);
    g.players[0].epoch=10;g.rebuild_modifiers(0,false);g.apply_paid_main(0,9);
    assert_eq!(g.ents[dock].qn,0);assert!(g.queue_prices.is_empty());
    assert_eq!(g.resource(0,2),before[2]+290.0);assert_eq!(g.resource(0,6),before[6]+921.0);
}
#[test]
fn pending_discount_reprice_returns_surplus_without_changing_active_invoice(){
    let(mut g,tc,_)=game(0);for _ in 0..5{assert!(g.train(tc,57));}
    let before=g.resource(0,0);g.researched[0][0]|=1<<3;g.rebuild_modifiers(0,false);
    g.reprice_paid_pending(0,57);assert_eq!(g.ents[tc].qn,5);
    assert_eq!(g.queue_payment(tc,0).price[0],30.0);for p in 1..5{assert_eq!(g.queue_payment(tc,p).price[0],27.0);}
    assert_eq!(g.resource(0,0),before+12.0);
}
#[test]
fn an_unpaid_front_job_waits_for_resources_then_captures_the_real_charge(){
    let(mut g,tc,_)=game(0);assert!(g.train(tc,57));let mut paid=g.queue_payment(tc,0);paid.purchased=false;g.record_queue_payment(paid);
    g.players[0].res[0]=0.0;g.tick();assert_eq!(g.ents[tc].qt,0.0);assert!(!g.queue_payment(tc,0).purchased);
    g.players[0].res[0]=100.0;g.tick();assert!(g.ents[tc].qt>0.0);assert_eq!(g.resource(0,0),70.0);
    let paid=g.queue_payment(tc,0);assert!(paid.purchased&&!paid.inferred);assert_eq!(paid.price[0],30.0);
}
#[test]
fn partial_purchase_uses_native_double_bias_bits_in_the_declared_nearest53_context(){
    let rate=1.100000023841858_f32;
    assert_eq!(paid_queue7::partial_purchase_integer53(337.1499938964844_f32,rate),306);
    assert_eq!(paid_queue7::partial_purchase_integer53(131.4499969482422_f32,rate),120);
    assert_eq!((337.1499938964844_f32/rate+0.5_f32).floor(),307.0);
    assert_eq!((131.4499969482422_f32/rate+0.5_f32).floor(),119.0);
}
#[test]
fn gold_product_is_not_rounded_before_the_source_account_store(){
    let(mut g,_,_)=game(7);g.extra_resources[0][1]=1.3_f32;
    g.change_reprice_gold53(0,90.0_f64*0.009999999776482582_f64);
    assert_eq!(g.resource(0,4).to_bits(),2.200000047683716_f32.to_bits());
    assert_ne!(g.resource(0,4).to_bits(),(1.3_f32+(90.0_f64*0.009999999776482582_f64)as f32).to_bits());
}
#[test]
fn duration_is_latched_at_work_start_and_not_rewritten_by_later_research(){
    let(mut g,tc,_)=game(0);assert!(g.train(tc,57));assert!(g.train(tc,57));
    assert_eq!(g.queue_payment(tc,0).duration,-1.0);assert_eq!(g.queue_payment(tc,1).duration,-1.0);
    g.tick();assert_eq!(g.queue_payment(tc,0).duration,32.5);assert_eq!(g.ents[tc].qt,0.009999999776482582_f32);
    g.researched[0][0]|=1<<3;g.rebuild_modifiers(0,false);assert_eq!(g.production_time(0,57),29.25);
    for _ in 0..100{g.tick();}let elapsed=g.ents[tc].qt;
    assert_eq!(g.queue_time_limit(tc,0),32.5);assert_eq!(g.queue_time_limit(tc,1),29.25);
    g.save();let mut loaded=Game::load(&g.savebuf).unwrap();assert_eq!(loaded.queue_payment(tc,0).duration,32.5);assert_eq!(loaded.ents[tc].qt,elapsed);
    assert_eq!(loaded.queue_payment(tc,1).duration,-1.0);
    for _ in 0..600{g.tick();loaded.tick();assert_eq!(g.state_hash(),loaded.state_hash());}
    assert_eq!(g.ents[tc].qn,1);assert_eq!(g.queue_payment(tc,0).duration,29.25);
}
