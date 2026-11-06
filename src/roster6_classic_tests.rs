use super::*;

fn game(epoch:u8)->Game {
    let mut g=Game::new(7,0,0,2,0);
    for i in 0..g.high {g.kill(i);}
    g.territory.claims.fill(NONE);
    for team in 0..2 {
        g.players[team].epoch=epoch;
        g.players[team].res=[100000.0;3];
        g.extra_resources[team]=[100000.0;6];
        g.rebuild_modifiers(team as u8,false);
        let tc=g.spawn(TC,team as u8,30.5+team as f32*150.0,30.5);
        g.ents[tc].built=1.0;
    }
    g.team_vis.fill(255);g.build_grid();g
}

#[test]
fn real_source_scout_trains_from_city_center_with_wide_definition() {
    let mut g=game(0);
    let tc=g.ents[..g.high].iter().position(|e|e.alive&&e.team==0&&e.def==TC).unwrap();
    let before=g.resource(0,0);
    assert!(g.train(tc,57));
    assert_eq!(g.ents[tc].q[0],57);
    assert_eq!(before-g.resource(0,0),30.0);
    for _ in 0..655 {g.tick();}
    assert!(g.ents[..g.high].iter().any(|e|e.alive&&e.team==0&&e.def==57));
    assert_eq!(g.max_hp(0,57),35.0);
    assert_eq!(g.modifiers[0][57][2],2.875);
}

#[test]
fn fighting_sail_trains_from_dock_and_uses_all_nine_source_cost_slots() {
    let mut g=game(5);
    let t=(3*N+3..NN-3*N).find(|&t|g.map.pass[1][t]&&g.map.pass[1][t+1]&&g.map.pass[1][t-1]&&g.map.pass[1][t+N]&&g.map.pass[1][t-N]).unwrap();
    let dock=g.spawn(DOCK,0,(t%N)as f32+0.5,(t/N)as f32+0.5);g.ents[dock].built=1.0;
    let before:[f32;9]=core::array::from_fn(|r|g.resource(0,r));
    assert!(g.train(dock,58));
    let cost=g.unit_cost(0,58);
    assert_eq!(cost,[0,187,248,0,150,0,0,0,0]);
    for r in 0..9 {assert_eq!(before[r]-g.resource(0,r),cost[r]as f32);}
    for _ in 0..1305 {g.tick();}
    assert!(g.ents[..g.high].iter().any(|e|e.alive&&e.team==0&&e.def==58));
}

#[test]
fn air_defense_native_rps_rejects_ground_and_acquires_aircraft() {
    let mut g=game(10);
    let own=g.spawn(59,0,100.5,100.5);g.ents[own].built=1.0;
    let land=g.spawn(9,1,103.5,100.5);
    let air=g.spawn(37,1,105.5,100.5);
    assert!(!g.rps_allows(&g.ents[own],&g.ents[land]));
    assert!(g.rps_allows(&g.ents[own],&g.ents[air]));
    g.build_grid();
    assert_eq!(g.acquire(&g.ents[own],&DEFS[59],12.0),g.h(air));
    g.ents[air].hp=1000.0;
    let mut e=g.ents[own];g.fire(own,&mut e,&DEFS[59].w[0],air);
    assert!(g.ents[air].hp<1000.0);
}

#[test]
fn coastal_defense_uses_real_ballista_ammo_and_native_ship_matchup() {
    let mut g=game(1);
    let own=g.spawn(60,0,100.5,100.5);g.ents[own].built=1.0;
    let ship=g.spawn(29,1,105.5,100.5);
    assert!(g.rps_allows(&g.ents[own],&g.ents[ship]));
    let attack=crate::ee2_up15_attack::lookup(60,1).unwrap();
    assert!(attack.missile);assert!(attack.projectile_speed.is_some());
    let mut e=g.ents[own];g.fire(own,&mut e,&DEFS[60].w[0],ship);
    let p=g.projs.last().unwrap();
    assert_eq!(p.speed,attack.projectile_speed.unwrap());
    assert_eq!(p.attacker_profile,crate::ee2_combat::profile(60,1));
}

#[test]
fn all_new_definitions_roundtrip_and_render_in_distinct_namespace() {
    let mut g=game(14);
    for id in 57..61 {let i=g.spawn(id,0,70.5+(id-57)as f32*10.0,70.5);g.ents[i].built=1.0;}
    g.build_grid();g.refresh_display_fog();
    g.save();let saved=g.savebuf.clone();
    let mut loaded=Game::load(&saved).unwrap();
    assert_eq!(loaded.state_hash(),g.state_hash());loaded.save();assert!(loaded.savebuf==saved,"first save diff {:?}",loaded.savebuf.iter().zip(&saved).enumerate().filter(|(_, (a,b))|a!=b).map(|(i,(a,b))|(i,*a,*b)).take(8).collect::<Vec<_>>());
    for id in 57..61 {let raw=crate::ids::unit_render(id)as u32;assert_eq!(crate::ids::render_kind(raw),0);assert_eq!(crate::ids::render_source(raw),id as u32);}
    assert_eq!(crate::ids::render_capacity(),256+DEFS.len());
}

#[test]
fn descriptor_cardinality_and_fingerprint_cannot_be_crossed() {
    let mut g=game(14);g.save();let good=g.savebuf.clone();
    let mut b=good.clone();b[12..16].copy_from_slice(&57u32.to_le_bytes());
    assert!(Game::load(&b).is_none(),"61 fingerprint with57 count");
    let n=b.len()-52;b=good.clone();b[n+12..n+44].copy_from_slice(&crate::roster6_registry::PREVIOUS_57);
    assert!(Game::load(&b).is_none(),"57 fingerprint with61 count");
    assert!(Game::load(&good).is_some());
}

#[test]
fn new_source_damage_avoids_legacy_ratio_rounding_and_research_targets_scout(){
    let mut g=game(14);assert_eq!(g.effective_weapon_damage(0,59,DEFS[59].w[0].dmg),59.0);
    g=game(0);assert_eq!(g.unit_cost(0,57)[0],30);assert_eq!(g.production_time(0,57),32.5);
    // CSV values are source-bound; native x87 FPU mode at the arithmetic setter
    // remains unresolved, so percentage-result bits test this implementation only.
    // Raw CSV5 EconomicTechUpgrade_E1_Slot4a/b targets TT_CitizenScout -10%.
    g.researched[0][0]|=1<<3;g.rebuild_modifiers(0,false);
    assert_eq!(g.unit_cost(0,57)[0],27);assert_eq!(g.production_time(0,57),29.25);
}

#[test]
fn legacy_weapon_multiplication_order_is_preserved_bit_for_bit(){
    let mut g=game(14);
    for (base,player,mul)in[(41.0_f32,1.27_f32,59.0_f32/41.0_f32),(137.0,1.18,0.83),(2.3,0.75,9.1)] {
        g.modifiers[0][9][1]=mul;
        assert_eq!(g.scaled_weapon_damage(0,9,base,player).to_bits(),(base*player*mul).to_bits());
    }
}

#[test]
fn sail_permanent_damage_research_survives_epoch_in_current_arithmetic_adapter(){
    let mut g=game(7);
    for k in [28,88] {g.researched[0][k/32]|=1<<(k%32);}
    g.rebuild_modifiers(0,false);
    assert_eq!(g.effective_weapon_damage(0,58,DEFS[58].w[0].dmg).to_bits(),180.0_f32.to_bits());
    let own=g.spawn(58,0,100.5,100.5);let target=g.spawn(29,1,105.5,100.5);
    let mut e=g.ents[own];g.fire(own,&mut e,&DEFS[58].w[0],target);
    assert_eq!(g.projs.last().unwrap().dmg.to_bits(),180.0_f32.to_bits());
    g.players[0].epoch=9;g.rebuild_modifiers(0,false);g.projs.clear();
    g.fire(own,&mut e,&DEFS[58].w[0],target);
    assert_eq!(g.projs.last().unwrap().dmg.to_bits(),204.0_f32.to_bits());
    g.build_grid();g.refresh_display_fog();g.save();let mut loaded=Game::load(&g.savebuf).unwrap();
    assert_eq!(loaded.effective_weapon_damage(0,58,DEFS[58].w[0].dmg).to_bits(),204.0_f32.to_bits());
    loaded.save();assert!(loaded.savebuf==g.savebuf);
}

#[test]
fn four_source_production_boundaries_are_separate_from_survivor_numeric_rows(){
    for epoch in 0..15 {
        let mut g=game(epoch);let dock=g.spawn(DOCK,0,100.5,100.5);g.ents[dock].built=1.0;
        let tc=g.ents[..g.high].iter().position(|e|e.alive&&e.team==0&&e.def==TC).unwrap();
        assert_eq!(g.can_train(tc,57),true,"Scout epoch{}",epoch);
        assert_eq!(g.can_train(dock,58),(5..10).contains(&epoch),"Sail epoch{}",epoch);
        assert_eq!(DEFS[59].epoch<=epoch,epoch>=9);assert_eq!(DEFS[60].epoch<=epoch,epoch>=1);
        for id in 57..61 {assert!(crate::ee2_units::lookup_class(id,epoch.max(DEFS[id as usize].epoch)).is_some());}
    }
    assert_eq!(DEFS[58].until,10);assert_eq!(DEFS[57].until,15);assert_eq!(DEFS[59].until,15);assert_eq!(DEFS[60].until,15);
}
#[test]
fn loading_survivors_and_compatibility_queues_remains_unchanged(){
    let mut g=game(14);let ship=g.spawn(58,0,100.5,100.5);let h=g.h(ship);
    let dock=g.spawn(DOCK,0,110.5,110.5);g.ents[dock].built=1.0;
    // Pre-fix61 could contain this now-unorderable job. Preserve persisted state.
    g.ents[dock].q[0]=58;g.ents[dock].qn=1;g.ents[dock].qt=3.0;
    g.build_grid();g.refresh_display_fog();g.save();let mut loaded=Game::load(&g.savebuf).unwrap();
    assert_eq!(loaded.get(h),Some(ship));assert_eq!(loaded.ents[dock].q[0],58);assert_eq!(loaded.ents[dock].qt,3.0);
    assert!(!loaded.can_train(dock,58));loaded.save();assert!(loaded.savebuf==g.savebuf);
}
