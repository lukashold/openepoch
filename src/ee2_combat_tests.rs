use super::*;
use crate::ee2_combat as native;
fn game(attacker_epoch:u8,defender_epoch:u8)->Game {
 let mut g=Game::new(7,0,0,2,0);
 for e in &mut g.ents[..g.high]{e.alive=false;}
 g.players[0].epoch=attacker_epoch;g.players[1].epoch=defender_epoch;
 g.rebuild_modifiers(0,false);g.rebuild_modifiers(1,false);g.team_vis.fill(255);g
}
fn close(actual:f32,expected:f32){assert!((actual-expected).abs()<0.002,"{actual} != {expected}");}
#[test]
fn native_rps_replaces_weapon_armor_matrix_for_direct_damage(){
 let mut g=game(0,1);let target=g.spawn(11,1,110.5,110.5);g.ents[target].hp=1000.0;
 let profile=native::profile(10,0);
 g.damage_from(target,100.0,W_SHELL,profile);
 close(g.ents[target].hp,850.0); // native150%, without own shell/infantry0.7
}
#[test]
fn native_upgrade_adjustment_multiplies_rps_damage(){
 let mut g=game(10,10);let target=g.spawn(10,1,110.5,110.5);g.ents[target].hp=1000.0;
 g.damage_from(target,100.0,W_SHELL,native::profile(26,10));
 close(g.ents[target].hp,850.0); // native250% × upgrade60%
}
#[test]
fn unmapped_attack_profile_preserves_existing_fallback(){
 let mut g=game(0,1);let target=g.spawn(11,1,110.5,110.5);g.ents[target].hp=1000.0;
 g.damage_from(target,100.0,W_SHELL,native::UNKNOWN_PROFILE);
 close(g.ents[target].hp,930.0);
}
#[test]
fn native_zero_matchup_blocks_fire_and_damage(){
 let mut g=game(10,10);let fort=g.spawn(FORTRESS,0,106.5,110.5);let target=g.spawn(37,1,110.5,110.5);
 g.ents[target].hp=1000.0;g.build_grid();let mut e=g.ents[fort];
 let mut forced=DEFS[FORTRESS as usize].w[0];forced.targets=LAND|SEA|AIR|SUB;
 g.fire(&mut e,&forced,target);assert!(g.projs.is_empty());
 g.damage_from(target,100.0,W_ARROW,native::profile(FORTRESS,10));close(g.ents[target].hp,1000.0);
}
#[test]
fn zero_rps_blocks_acquisition_and_attack_order_despite_weapon_mask(){
 let mut g=game(13,13);let marine=g.spawn(17,0,106.5,110.5);let target=g.spawn(37,1,110.5,110.5);g.build_grid();
 #[cfg(not(feature="ee2-up15"))]
 assert!(DEFS[17].weapon_vs(AIR).is_some(),"Own role previously has an anti-air weapon");
 let mut query_definition=DEFS[17];query_definition.w[0].targets|=AIR;
 assert_eq!(native::can_target(17,13,37,13),Some(false));
 assert_eq!(g.acquire(&g.ents[marine],&query_definition,20.0),NONE);
 g.order_attack(&[marine],target);assert_ne!(g.ents[marine].order,O_ATTACK);assert_ne!(g.ents[marine].otgt,g.h(target));
 let mut e=g.ents[marine];g.fire(&mut e,&query_definition.w[0],target);assert!(g.projs.is_empty());assert_eq!(e.cd,0.0);
}
#[test]
fn projectile_keeps_attacker_upgrade_at_launch(){
 let mut g=game(10,10);let attacker=g.spawn(26,0,106.5,110.5);let target=g.spawn(10,1,110.5,110.5);
 g.ents[target].hp=1000.0;g.build_grid();let mut e=g.ents[attacker];g.fire(&mut e,&DEFS[26].w[0],target);
 let shot=*g.projs.last().expect("Real projectile");assert_eq!(shot.attacker_profile,native::profile(26,10));
 g.players[0].epoch=14;
 for _ in 0..100{g.update_projs();if g.projs.is_empty(){break;}}
 assert!(g.projs.is_empty());close(g.ents[target].hp,1000.0-shot.dmg*native::multiplier_from_profile(shot.attacker_profile,10,10).unwrap());
 assert_ne!(native::multiplier_from_profile(shot.attacker_profile,10,10),native::multiplier_from_profile(native::profile(26,14),10,10));
}
#[test]
fn projectile_uses_defenders_current_epoch_at_impact(){
 let mut g=game(10,10);let attacker=g.spawn(26,0,106.5,110.5);let target=g.spawn(10,1,110.5,110.5);
 g.ents[target].hp=1000.0;g.build_grid();let mut e=g.ents[attacker];g.fire(&mut e,&DEFS[26].w[0],target);
 let shot=*g.projs.last().expect("Real projectile");g.players[1].epoch=14;
 for _ in 0..100{g.update_projs();if g.projs.is_empty(){break;}}
 close(g.ents[target].hp,1000.0-shot.dmg*native::multiplier_from_profile(shot.attacker_profile,10,14).unwrap());
 assert_ne!(native::multiplier_from_profile(shot.attacker_profile,10,10),native::multiplier_from_profile(shot.attacker_profile,10,14));
}
#[test]
fn splash_resolves_each_defenders_class_separately(){
 let mut g=game(10,10);let attacker=g.spawn(26,0,106.5,110.5);let infantry=g.spawn(10,1,110.5,110.5);let building=g.spawn(HOUSE,1,110.5,110.5);
 g.ents[infantry].hp=1000.0;g.ents[building].hp=1000.0;g.build_grid();let mut e=g.ents[attacker];
 let mut shell=DEFS[26].w[0];shell.splash=2.0;g.fire(&mut e,&shell,infantry);
 #[cfg(feature="ee2-up15")] {g.projs.last_mut().unwrap().splash=2.0;}
 let shot=*g.projs.last().expect("Splash projectile");
 for _ in 0..100{g.update_projs();if g.projs.is_empty(){break;}}
 close(g.ents[infantry].hp,1000.0-shot.dmg*native::multiplier_from_profile(shot.attacker_profile,10,10).unwrap());
 close(g.ents[building].hp,1000.0-shot.dmg*native::multiplier_from_profile(shot.attacker_profile,HOUSE,10).unwrap());
 assert_ne!(native::multiplier_from_profile(shot.attacker_profile,10,10),native::multiplier_from_profile(shot.attacker_profile,HOUSE,10));
}
#[test]
fn secondary_splash_half_radius_takes_quarter_damage(){
 let mut g=game(10,10);let attacker=g.spawn(26,0,106.5,110.5);let primary=g.spawn(10,1,110.5,110.5);let secondary=g.spawn(10,1,112.5,110.5);
 g.ents[primary].hp=1000.0;g.ents[secondary].hp=1000.0;g.build_grid();let mut e=g.ents[attacker];
 let mut shell=DEFS[26].w[0];shell.splash=4.0;g.fire(&mut e,&shell,primary);
 #[cfg(feature="ee2-up15")] {g.projs.last_mut().unwrap().splash=4.0;}
 let shot=*g.projs.last().expect("Splash projectile");
 for _ in 0..100{g.update_projs();if g.projs.is_empty(){break;}}
 let damage=shot.dmg*native::multiplier_from_profile(shot.attacker_profile,10,10).unwrap();
 close(g.ents[primary].hp,1000.0-damage);close(g.ents[secondary].hp,1000.0-damage*0.25);
}
#[test]
fn base_city_radius_progresses_and_queries_every_owner(){
 for epoch in 0..15 {
  let mut g=game(0,epoch);let city=g.spawn(TC,1,100.0,100.0);
  let radius=7.0+(epoch/3) as f32;let reach=radius*1.4142135381698608_f32;
  assert!(g.original_urban_at(100.0,100.0));
  assert!(g.original_urban_at(100.0+reach-0.0001,100.0));
  assert!(!g.original_urban_at(100.0+reach+0.0001,100.0));
  g.ents[city].built=0.99;assert!(!g.original_urban_at(100.0,100.0));
  g.ents[city].built=1.0;g.ents[city].alive=false;assert!(!g.original_urban_at(100.0,100.0));
 }
}
#[test]
fn city_boundary_uses_strict_native_extended_comparison(){
 let mut g=game(3,3);g.spawn(TC,0,0.0,0.0);
 let boundary=8.0*1.4142135381698608_f32;
 assert!(!g.original_urban_at(boundary,0.0));
 assert!(g.original_urban_at(f32::from_bits(boundary.to_bits()-1),0.0));
 assert!(!g.original_urban_at(f32::NAN,0.0));assert!(!g.original_urban_at(0.0,f32::INFINITY));
}
