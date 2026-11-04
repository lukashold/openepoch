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
    // Existing control-loss adapter: a city loss frees the parcel, rather than
    // persisting native disputed ownership, which requires a future save format.
    g.ents[c].alive=false;g.build_grid();assert_eq!(g.towner[id],255);
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
    let mut g=game();for e in &mut g.ents[..g.high] {if e.team==0&&e.def==TC {e.alive=false;}}
    g.build_grid();assert!(!g.towner.contains(&0));
    let id=free(&g,false);city(&mut g,0,id,1.0);g.build_grid();assert_eq!(g.towner[id],0);
}

#[test]
fn pending_isolated_claim_roundtrips_and_old56_city_rights_migrate() {
    let mut g=game();let id=free(&g,false);city(&mut g,0,id,1.0);g.build_grid();
    g.save();let modern=Game::load(&g.savebuf).unwrap();assert_eq!(modern.towner[id],255);
    // A genuine pre-CGD save has no future extension attached to its old header.
    let mut old=crate::city_ground::split_save(&g.savebuf).unwrap().0.to_vec();old[12..16].copy_from_slice(&56u32.to_le_bytes());
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
