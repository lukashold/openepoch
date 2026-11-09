//! Focused acquisition regressions, independent of the browser's selection/UI.
use crate::{defs::*,game::*,map::*};

fn game()->Game {
    let mut g=Game::new(7,0,0,2,0);
    for p in &mut g.players {p.ai=false;}
    g
}
fn city(g:&mut Game,team:u8,id:usize,built:f32)->usize {
    let (x,y)=g.map.tseeds[id];let i=g.spawn(TC,team,x,y);g.ents[i].built=built;i
}
fn fortress(g:&mut Game,team:u8,id:usize,built:f32)->usize {
    let (x,y)=g.map.tseeds[id];let i=g.spawn(FORTRESS,team,x+7.0,y);g.ents[i].built=built;
    assert_eq!(g.map.terr[tile_of(g.ents[i].x,g.ents[i].y)] as usize,id);i
}
fn free(g:&Game,adjacent:bool)->usize {
    (0..g.map.tseeds.len()).find(|&id|g.towner[id]==255&&g.territory_adjacent_owned(0,id)==adjacent).expect("real generated free parcel")
}

#[test]
fn completed_city_claims_adjacent_land_but_foundation_does_not() {
    let mut g=game();let id=free(&g,true);let c=city(&mut g,0,id,0.0);
    g.build_grid();assert_eq!(g.towner[id],255);assert_eq!(g.territory_claim_status(0,id),2);
    g.ents[c].built=0.99999;g.build_grid();assert_eq!(g.towner[id],255);
    g.ents[c].built=1.0;g.build_grid();assert_eq!(g.towner[id],0);
    assert_eq!(g.territory_control_counts(0,id),[1,0,1,0]);
}

#[test]
fn isolated_city_waits_for_own_completed_fortress_and_retains_acquired_right() {
    let mut g=game();let id=free(&g,false);let c=city(&mut g,0,id,1.0);
    g.build_grid();assert_eq!(g.towner[id],255);assert_eq!(g.territory_claim_status(0,id),3);
    fortress(&mut g,1,id,1.0);g.build_grid();assert_eq!(g.towner[id],255);
    let f=fortress(&mut g,0,id,0.999);g.build_grid();assert_eq!(g.towner[id],255);
    assert_eq!(g.territory_claim_status(0,id),4);
    g.ents[f].built=1.0;g.build_grid();assert_eq!(g.towner[id],0);
    g.ents[f].alive=false;g.build_grid();assert_eq!(g.towner[id],0);
    g.ents[c].alive=false;g.build_grid();
    if g.disputed_territory_rules() {
        assert_eq!(g.towner[id], 0);
        assert!(g.territory_is_disputed(id));
    } else {
        assert_eq!(g.towner[id], 255);
    }
}

#[test]
fn neighboring_completed_city_chain_ignores_entity_and_region_scan_order() {
    let mut a=game();let mut pair=None;
    for x in 0..a.map.tseeds.len() {for y in 0..a.map.tseeds.len() {
        if a.towner[x]==255&&a.towner[y]==255&&a.territory_adjacent_owned(0,x)
            &&!a.territory_adjacent_owned(0,y)&&a.map.territory_neighbors[x]&(1<<y)!=0 {pair=Some((x,y));break;}
    }if pair.is_some(){break;}}
    let(x,y)=pair.expect("real two-step expansion graph");let mut b=game();
    city(&mut a,0,y,1.0);city(&mut a,0,x,1.0);
    city(&mut b,0,x,1.0);city(&mut b,0,y,1.0);
    a.build_grid();b.build_grid();assert_eq!(a.towner,b.towner);
    assert_eq!(a.towner[x],0);assert_eq!(a.towner[y],0);
}

#[test]
fn landless_player_can_bootstrap_without_fortress() {
    let mut g=game();
    for (territory, &owner) in g.towner.iter().enumerate() {
        if owner == 0 { g.territory.claims[territory] = NONE; }
    }
    for e in &mut g.ents[..g.high] {if e.team==0&&e.def==TC {e.alive=false;}}
    g.build_grid();assert!(!g.towner.contains(&0));
    let id=free(&g,false);city(&mut g,0,id,1.0);g.build_grid();assert_eq!(g.towner[id],0);
}

#[cfg(feature = "ee2-native13")]
fn disputed_game() -> Game {
    let mut game = Game::new(7, 1, 0, 1, 0);
    for player in &mut game.players { player.ai = false; }
    game
}

#[cfg(feature = "ee2-native13")]
#[test]
fn disputed_territory_keeps_owner_across_continue_and_restores_granary_on_completion() {
    let mut g = disputed_game();
    let id = free(&g, true);
    let former_city = city(&mut g, 0, id, 1.0);
    let (x, y) = g.map.tseeds[id];
    let granary = g.spawn(crate::granary211::ID, 0, x + 1.0, y);
    let worker = g.spawn(WORKER, 0, x, y);
    assert_ne!(granary, NOIDX);
    assert_ne!(worker, NOIDX);
    g.build_grid();
    assert_eq!(g.granary_farm_bonus(&g.ents[worker]), 1.0);
    g.kill(former_city);
    g.build_grid();
    assert_eq!(g.towner[id], 0);
    assert!(g.territory_is_disputed(id));
    assert_eq!(g.granary_farm_bonus(&g.ents[worker]), 0.0);
    assert!(!g.territory_valid(0, 1, 0, id as u32, NONE));
    let replacement = city(&mut g, 0, id, 0.5);
    g.build_grid();
    assert!(g.territory_is_disputed(id));
    g.save();
    let mut loaded = Game::load(&g.savebuf).expect("disputed territory continues");
    assert_eq!(loaded.towner[id], 0);
    assert!(loaded.territory_is_disputed(id));
    assert_eq!(loaded.granary_farm_bonus(&loaded.ents[worker]), 0.0);
    loaded.save();
    assert!(loaded.savebuf == g.savebuf);
    loaded.ents[replacement].built = 1.0;
    loaded.build_grid();
    assert!(!loaded.territory_is_disputed(id));
    assert_eq!(loaded.granary_farm_bonus(&loaded.ents[worker]), 1.0);
    assert!(loaded.territory_valid(0, 1, 0, id as u32, NONE));
}

#[cfg(feature = "ee2-native13")]
#[test]
fn disputed_rival_claim_needs_completed_control_and_keeps_old_actors() {
    let mut g = disputed_game();
    let id = free(&g, true);
    let initial = city(&mut g, 0, id, 1.0);
    let (x, y) = g.map.tseeds[id];
    let existing_house = g.spawn(HOUSE, 0, x, y);
    g.build_grid();
    g.kill(initial);
    g.build_grid();
    assert!(g.territory_is_disputed(id));
    let rival = city(&mut g, 1, id, 0.5);
    let isolated = !g.territory_adjacent_owned(1, id);
    let fort = fortress(&mut g, 1, id, 0.5);
    g.build_grid();
    assert!(g.territory_is_disputed(id));
    assert_eq!(g.towner[id], 0);
    g.ents[rival].built = 1.0;
    g.build_grid();
    if isolated {
        assert_eq!(g.towner[id], 0);
        assert!(g.territory_is_disputed(id));
        g.ents[fort].built = 1.0;
        g.build_grid();
    }
    assert_eq!(g.towner[id], 1);
    assert!(g.territory_is_disputed(id));
    assert_eq!(g.ents[existing_house].team, 0);
    g.save();
    let mut continued = Game::load(&g.savebuf).expect("save during disputed rival acquisition");
    assert_eq!(continued.towner[id], 1);
    assert!(continued.territory_is_disputed(id));
    continued.save();
    assert!(continued.savebuf == g.savebuf);
    g.build_grid();
    assert!(!g.territory_is_disputed(id));
    g.players[1].alive = false;
    g.build_grid();
    assert_eq!(g.towner[id], 255);
}

#[cfg(feature = "ee2-native13")]
#[test]
fn disputed_territory_lockstep_is_opt_in_and_resume_rejects_wrong_feature() {
    let mut legacy = game();
    let id = free(&legacy, true);
    let initial = city(&mut legacy, 0, id, 1.0);
    legacy.build_grid();
    legacy.save();
    let mut current = Game::load(&legacy.savebuf).unwrap();
    current.network_disputed_territories = true;
    for game in [&mut legacy, &mut current] {
        game.kill(initial);
        game.build_grid();
        game.save();
    }
    assert_eq!(legacy.towner[id], 255);
    assert_eq!(current.towner[id], 0);
    let restored = Game::load(&current.savebuf).unwrap();
    assert_eq!(restored.towner[id], 0);
    assert!(restored.territory_is_disputed(id));
    assert_ne!(legacy.state_hash(), restored.state_hash());
    assert!(crate::crown_resume_matches(&restored, crate::territory::DISPUTED_PEER_RESUME_FLAG));
    assert!(!crate::crown_resume_matches(&restored, 0));
}

#[cfg(feature = "ee2-native13")]
#[test]
fn disputed_map_pixels_and_ui_do_not_disclose_explored_enemy_state() {
    let mut g = disputed_game();
    let id = g.towner.iter().position(|&owner| owner == 1).unwrap();
    let city = g.get(g.territory.claims[id]).unwrap();
    g.kill(city);
    g.build_grid();
    let tile = g.map.terr.iter().position(|&region| usize::from(region) == id).unwrap();
    let ui_disputed = |game: &Game| {
        let mut json = String::new();
        game.append_territory_ui(&mut json);
        json.split_once(&format!("\"id\":{id},")).unwrap().1
            .split('}').next().unwrap().contains("\"disputed\":true")
    };
    g.vis.fill(128);
    g.terr_tex();
    assert_eq!(&g.terr_rgba[tile * 4..tile * 4 + 4], &[id as u8, 2, 0, 255]);
    assert!(!ui_disputed(&g));
    g.vis[tile] = 255;
    g.terr_tex();
    assert_eq!(g.terr_rgba[tile * 4 + 2], 2);
    assert!(ui_disputed(&g));
    g.team_vis.fill(0);
    g.refresh_display_fog();
    assert!(g.terr_dirty);
    g.terr_tex();
    assert_eq!(g.terr_rgba[tile * 4 + 2], 0);
    g.local = 1;
    g.vis.fill(0);
    g.terr_tex();
    assert_eq!(g.terr_rgba[tile * 4 + 2], 2);
    assert!(ui_disputed(&g));
}

#[test]
fn pending_isolated_claim_roundtrips_and_old56_city_rights_migrate() {
    let mut g=game();let id=free(&g,false);city(&mut g,0,id,1.0);g.build_grid();
    g.save();let modern=Game::load(&g.savebuf).unwrap();assert_eq!(modern.towner[id],255);
    // A genuine pre-CGD save has no future extension attached to its old header.
    #[cfg(feature="ee2-up15")]let saved=crate::native_attack_clock::split_save(&g.savebuf).unwrap().0;
    #[cfg(not(feature="ee2-up15"))]let saved=&g.savebuf[..];
    let mut old=crate::city_ground::split_save(saved).unwrap().0.to_vec();old[12..16].copy_from_slice(&56u32.to_le_bytes());
    #[cfg(feature="ee2-up15")]
    {assert!(Game::load(&old).is_none());return;}
    let mut migrated=Game::load(&old).unwrap();assert_eq!(migrated.towner[id],0);
    migrated.save();assert_eq!(u32::from_le_bytes(migrated.savebuf[12..16].try_into().unwrap()),57);
    assert_eq!(Game::load(&migrated.savebuf).unwrap().towner[id],0);
}

#[test]
fn maximum_two_fortresses_counts_all_completed_but_only_own_foundations() {
    let mut g=game();let id=g.map.terr[tile_of(g.map.starts[0].0 as f32,g.map.starts[0].1 as f32)] as usize;
    fortress(&mut g,0,id,0.0);let foreign=fortress(&mut g,1,id,0.0);
    let (x,y)=g.map.tseeds[id];let(tx,ty)=(x as i32+13,y as i32+13);
    assert_eq!(g.map.terr[(ty+2) as usize*N+(tx+2) as usize] as usize,id);
    assert_ne!(g.placement_geometry_error(0,FORTRESS,tx,ty,0),10);
    g.ents[foreign].built=1.0;assert_eq!(g.placement_geometry_error(0,FORTRESS,tx,ty,0),10);
    g.ents[foreign].built=0.0;
    fortress(&mut g,0,id,0.0);assert_eq!(g.placement_geometry_error(0,FORTRESS,tx,ty,0),10);
}

#[test]
fn unknown_claim_ui_does_not_disclose_opponent_foundations() {
    let mut g=game();let id=free(&g,false);city(&mut g,1,id,0.0);
    g.vis.fill(0);g.team_vis.fill(0);g.build_grid();let mut s=String::new();g.append_territory_ui(&mut s);
    let needle=format!("\"id\":{},",id);let row=s.split(&needle).nth(1).unwrap().split('}').next().unwrap();
    for key in ["townCenters","fortresses","eligibleTownCenters","eligibleFortresses","claimStatus"] {assert!(row.contains(&format!("\"{}\":0",key)),"{row}");}
    assert!(row.contains("\"adjacent\":false"));
}
