use crate::ids::DefId;
use super::*;

fn game(epoch:u8)->Game {
    let mut g=Game::new(7,0,0,2,0);g.players[0].epoch=epoch;g.rebuild_modifiers(0,false);
    g.team_vis.fill(255);g
}

#[test]
fn native_fortress_progression_fires_correct_damage_range_reload_and_missile() {
    for (epoch,damage,range,kind) in [(0,75.0,7.0,W_ARROW),(3,100.0,9.0,W_ARROW),(5,115.0,9.0,W_ARROW),
        (7,145.0,10.0,W_SHELL),(9,170.0,10.0,W_SHELL),(11,200.0,11.0,W_SHELL),(13,225.0,11.0,W_SHELL)] {
        let mut g=game(epoch);let f=g.spawn(FORTRESS,0,120.5,120.5);let target=g.spawn(WORKER,1,124.5,120.5);
        g.build_grid();let mut e=g.ents[f];g.fire(&mut e,&DEFS[FORTRESS as usize].w[0],target);
        let p=g.projs.last().expect("real native fortress shot");
        assert_eq!(p.kind,kind);assert!((p.dmg-damage).abs()<0.001);assert_eq!(p.targets,if cfg!(feature="ee2-up15"){LAND|SEA|AIR|SUB}else{LAND|SEA});assert_eq!(e.cd,3.0);
        assert!((DEFS[FORTRESS as usize].w[0].range+g.modifiers[0][FORTRESS as usize][6]-range).abs()<0.001);
    }
}

#[test]
fn fortress_acquires_land_and_surface_ships_but_never_air_or_submarines() {
    let mut g=game(13);let f=g.spawn(FORTRESS,0,120.5,120.5);
    g.spawn(39,1,123.5,120.5);let sub=g.spawn(34,1,123.5,121.5);g.ents[sub].reveal_t=10.0;g.build_grid();
    assert_eq!(g.acquire(&g.ents[f],&DEFS[FORTRESS as usize],11.0),NONE);
    let ship=g.spawn(29,1,123.5,122.5);g.build_grid();
    assert_eq!(g.acquire(&g.ents[f],&DEFS[FORTRESS as usize],11.0),g.h(ship));
}

fn board(g:&mut Game,f:usize,def:DefId)->Ent {
    let b=g.ents[f];let u=g.spawn(def,0,b.x+3.0,b.y);let mut e=g.ents[u];
    e.hp*=0.5;e.order=O_BOARD;e.otgt=g.h(f);
    g.do_board(u,&mut e,&DEFS[def as usize],&mut(0.0,0.0),&mut false);
    g.ents[u]=e;if e.stuck==BOARDED {
        // Complete the same lifecycle the ordinary tick performs. Pack8's
        // complete allocator validation deliberately rejects a fixture that
        // flips alive without returning the slot to its real free list.
        #[cfg(feature="ee2-pack8")]g.kill(u);
        #[cfg(not(feature="ee2-pack8"))]{g.ents[u].alive=false;}
    }
    e
}

#[test]
fn fortress_enforces_native_population_slots_heals_and_roundtrips_crew() {
    let mut g=game(0);let f=g.spawn(FORTRESS,0,120.5,120.5);
    assert_eq!(g.fortress_slot_cost(0,WORKER),Some(1));
    for _ in 0..9 {assert_eq!(board(&mut g,f,WORKER).stuck,BOARDED);}
    let heavy=(9..47).find(|&def|g.fortress_slot_cost(0,def).map_or(false,|n|n>1)).expect("native unit requiring multiple places");
    assert_ne!(board(&mut g,f,heavy).stuck,BOARDED);assert_eq!(g.ents[f].ncargo,9);
    assert_eq!(board(&mut g,f,WORKER).stuck,BOARDED);assert_eq!(g.cargo_slots_used(&g.ents[f]),10);
    assert_ne!(board(&mut g,f,WORKER).stuck,BOARDED);assert_eq!(g.ents[f].ncargo,10);
    assert!((g.fortress_attack_bonus(&g.ents[f])-0.3).abs()<0.00001);
    let mut e=g.ents[f];for _ in 0..20 {g.update_building(f,&mut e,&DEFS[FORTRESS as usize]);}
    assert!(e.cargo_hp[..10].iter().all(|&hp|(56..=57).contains(&hp)));
    g.ents[f]=e;g.save();let loaded=Game::load(&g.savebuf).unwrap();
    assert_eq!(loaded.ents[f].cargo_hp,g.ents[f].cargo_hp);assert_eq!(loaded.cargo_slots_used(&loaded.ents[f]),10);
    assert_eq!(loaded.ents[f].carry.to_bits(),g.ents[f].carry.to_bits());
}

#[test]
fn odd_turn_fortress_crew_can_leave_its_wider_rotated_bounds() {
    let mut g=game(0);g.map.ter.fill(T_GRASS);g.map.res.fill(R_NONE);g.map.blocked.fill(false);
    for i in 0..NN {g.map.upd_pass(i);}
    let f=g.spawn_oriented(FORTRESS,0,104.0,104.0,core::f32::consts::FRAC_PI_4);
    assert_eq!(Game::building_bounds(&g.ents[f]).2,8);
    g.ents[f].ncargo=1;g.ents[f].cargo[0]=WORKER;g.ents[f].cargo_hp[0]=75;
    let before=g.ents[..g.high].iter().filter(|e|e.alive).count();let mut e=g.ents[f];g.unload(&mut e);
    assert_eq!(e.ncargo,0);assert_eq!(g.ents[..g.high].iter().filter(|e|e.alive).count(),before+1);
}

#[test]
fn fortress_technology_masks_apply_without_touching_other_roster_bindings() {
    let mut g=game(1);let before=g.max_hp(0,FORTRESS);let cost=g.unit_cost(0,FORTRESS);
    let (k,_)=crate::ee2_research::NODES.iter().enumerate().find(|(_,n)|n.effects.iter().any(|ef|crate::ids::mask_has(ef.mask,FORTRESS)&&ef.field==4)).unwrap();
    g.researched[0][k/32]|=1<<(k%32);g.rebuild_modifiers(0,false);
    assert_eq!(g.max_hp(0,FORTRESS),before);assert!(g.unit_cost(0,FORTRESS)[1]<cost[1]);assert!(g.unit_cost(0,FORTRESS)[3]<cost[3]);
}
