use super::*;use crate::projectiles8 as ps;

#[cfg(feature = "ee2-ai9-game-bridge")]
#[test]
fn light_infantry_rank_changes_the_damage_captured_by_a_real_arrow() {
    crate::init(7, 0, 0, 1, 0);
    assert_eq!(crate::ai9_game_bridge::ai9_game_research_setup(), 1);
    let game = crate::g();
    for slot in [0, 1, 5] {
        game.add_resource(0, slot, 1_000.0);
    }
    let shooter = game.spawn(10, 0, 100.5, 100.5);
    for index in 0..2 {
        let rank = crate::ai9_rank::ai9_game_improvement_job(index as u32 + 1);
        assert!(game.train(shooter, rank));
        for _ in 0..1_201 {
            game.tick();
        }
        let mut attacker = game.ents[shooter];
        let target = game.spawn(WORKER, 1, attacker.x + 3.0, attacker.y);
        game.build_grid();
        game.update_fog();
        let weapon = DEFS[attacker.def as usize].w[0];
        game.fire(&mut attacker, &weapon, target);
        let source_damage = ((110 + 10 * index) as f64 * f64::from(0.01_f32) * 7.0) as f32;
        assert_eq!(game.projs.pop().unwrap().dmg.to_bits(), source_damage.to_bits());
        game.kill(target);
    }
}

#[cfg(feature = "ee2-ai9-game-bridge")]
#[test]
fn source_family_ranks_keep_wounded_health_and_change_real_attacks() {
    crate::init(7, 0, 0, 1, 0);
    assert_eq!(crate::ai9_game_bridge::ai9_game_research_setup_manual(), 1);
    let game = crate::g();
    for slot in 0..9 {
        game.add_resource(0, slot, 1_000.0);
    }
    let families = [(9, 75.0, 5.0), (22, 85.0, 35.0), (29, 319.0, 42.0)];
    let actors = families.map(|(def, _, _)| game.spawn(def, 0, 100.5, 100.5));
    for &index in &actors {
        game.ents[index].hp *= 0.375;
    }
    for rank in 1..=2 {
        for (family, &index) in actors.iter().enumerate() {
            let job = crate::ai9_rank::ai9_game_improvement_family_job(family as u32 + 1, rank);
            assert!(game.train(index, job));
        }
        for _ in 0..1_605 {
            game.tick();
        }
        for (family, (&index, &(def, base_hp, base_damage))) in actors.iter().zip(&families).enumerate() {
            let scale = f64::from(100 + 10 * rank) * f64::from(0.01_f32);
            let max = ((f64::from(base_hp) * scale) as f32 + 0.5).floor();
            let damage = (f64::from(base_damage) * scale) as f32;
            assert_eq!(game.max_hp(0, def), max);
            assert_eq!(game.ents[index].hp, max * 0.375);
            let mut attacker = game.ents[index];
            let target = game.spawn(WORKER, 1, attacker.x + 0.75, attacker.y);
            game.build_grid();
            game.update_fog();
            let weapon = DEFS[attacker.def as usize].w[0];
            let hp = game.ents[target].hp;
            game.fire(&mut attacker, &weapon, target);
            if family == 0 {
                let attacker_profile = crate::ee2_combat::profile(def, 0);
                let multiplier = crate::ee2_combat::multiplier_from_profile(attacker_profile, WORKER, 0)
                    .unwrap_or(MULT[W_MELEE as usize][DEFS[WORKER as usize].armor as usize]);
                assert_eq!(game.ents[target].hp, hp - damage * multiplier);
            } else {
                assert_eq!(game.projs.pop().unwrap().dmg.to_bits(), damage.to_bits());
            }
            game.kill(target);
        }
    }
}

fn volley(splash:bool,recycle:bool)->(Game,usize,usize){
 let mut g=Game::new(7,0,0,2,0);for t in 0..2{g.players[t].epoch=5;g.rebuild_modifiers(t as u8,false);}
 let a=g.spawn(if splash{65}else{12},0,100.5,100.5);let target=g.spawn(WORKER,1,105.5,100.5);let nearby=g.spawn(WORKER,1,105.7,100.5);
 g.build_grid();g.update_fog();
 let mut attacker=g.ents[a];let weapon=DEFS[attacker.def as usize].w[0];for _ in 0..3{g.fire(&mut attacker,&weapon,target);}g.ents[a]=attacker;
 assert_eq!(g.projs.len(),3,"real ranged fire must create an in-flight volley");g.kill(a);
 if recycle{let old=g.h(target);g.kill(target);let next=g.spawn(WORKER,1,105.5,100.5);assert_eq!(next,target);assert_ne!(g.h(next),old);assert!(g.projs.iter().all(|p|p.tgt==old));}
 g.tick();(g,target,nearby)
}
#[test]fn real_volley_save_before_impact_retains_order_damage_deaths_and_future_ticks(){
 for splash in[false,true]{for recycle in[false,true]{
  let(mut a,target,nearby)=volley(splash,recycle);let hp=a.ents[target].hp;let hp_near=a.ents[nearby].hp;assert!(!a.projs.is_empty());
  a.save();let bytes=a.savebuf.clone();let mut b=Game::load(&bytes).unwrap();assert_eq!(a.state_hash(),b.state_hash());b.save();assert!(b.savebuf==bytes);assert!(!b.projectile_history_unknown);
  for tick in 0..80{a.tick();b.tick();assert_eq!(a.state_hash(),b.state_hash(),"splash{splash} recycled{recycle} tick{tick}");a.save();b.save();assert!(a.savebuf==b.savebuf,"complete continuation bytes");}
  assert!(a.projs.is_empty()&&b.projs.is_empty());
  if !recycle||splash{assert!(a.ents[target].hp<hp||!a.ents[target].alive);}else{assert_eq!(a.ents[target].hp,hp,"stale direct handle must not hit its replacement");}
  if splash{assert!(a.ents[nearby].hp<hp_near||!a.ents[nearby].alive);}else{assert_eq!(a.ents[nearby].hp,hp_near);}
 }}
}
#[test]fn projectile_codec_strict_fields_history_count_and_atomic_rejection(){
 let(mut g,_,_)=volley(false,false);let mut bytes=Vec::new();g.append_projectiles(&mut bytes);let hash=g.state_hash();let mut cases=Vec::new();
 for(p,v)in[(0,0),(4,2),(8,u32::MAX),(12,4001),(16,68),(20,2),(24,28),(28,4001),(ps::HEADER,f32::NAN.to_bits()),(ps::HEADER+4,(-1.0f32).to_bits()),(ps::HEADER+8,f32::INFINITY.to_bits()),(ps::HEADER+12,257.0f32.to_bits()),(ps::HEADER+24,MAXE as u32),(ps::HEADER+28,(-1.0f32).to_bits()),(ps::HEADER+32,(-1.0f32).to_bits()),(ps::HEADER+36,7),(ps::HEADER+40,g.nplayers as u32),(ps::HEADER+44,f32::NEG_INFINITY.to_bits()),(ps::HEADER+48,16),(ps::HEADER+52,65536),(ps::HEADER+56,(-1.0f32).to_bits()),(ps::HEADER+60,0)]{let mut b=bytes.clone();b[p..p+4].copy_from_slice(&v.to_le_bytes());cases.push(b);}
 for chop in[1,4,31,32,bytes.len()-1]{cases.push(bytes[..chop].to_vec());}
 let mut extra=bytes.clone();extra.push(0);cases.push(extra);
 for bad in cases{assert!(g.read_projectiles(Some(&bad)).is_none());assert_eq!(g.state_hash(),hash);}
 g.read_projectiles(Some(&bytes)).unwrap();assert_eq!(g.state_hash(),hash);
 let p=g.projs[0];g.projs=vec![p;4000];let mut full=Vec::new();g.append_projectiles(&mut full);g.read_projectiles(Some(&full)).unwrap();assert_eq!(g.projs.len(),4000);
}
#[test]fn every_projectile_word_vector_order_and_unknown_marker_affects_hash(){
 let(mut g,_,_)=volley(true,true);let before=g.state_hash();
 g.projs[1].dmg+=1.0;let different=g.state_hash();assert_ne!(different,before);g.projs.swap(0,1);assert_ne!(g.state_hash(),different);g.projs.swap(0,1);g.projs[1].dmg-=1.0;assert_eq!(g.state_hash(),before);
 macro_rules! field{($name:ident,$delta:expr)=>{{let old=g.projs[0].$name;g.projs[0].$name=old+$delta;assert_ne!(g.state_hash(),before,stringify!($name));g.projs[0].$name=old;}}}
 field!(x,0.1);field!(y,0.1);field!(px,0.1);field!(py,0.1);field!(tx,0.1);field!(ty,0.1);field!(tgt,65536);field!(speed,1.0);field!(dmg,1.0);field!(kind,1);field!(team,1);field!(splash,1.0);field!(targets,1);field!(attacker_profile,1);field!(friendly_ratio,0.1);field!(d0,1.0);
 g.projectile_history_unknown=true;assert_ne!(g.state_hash(),before);
}
