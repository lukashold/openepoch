//! Regression tests for native high-water radii; no rendering writes sim state.
use super::*;
use crate::city_ground::{self, CITY_END, CITY_MAGIC, EMPTY};

fn game() -> Game { Game::new(7,0,0,2,0) }
fn city(g: &Game, team: u8) -> usize {
    g.ents[..g.high].iter().position(|e|e.alive&&e.def==TC&&e.team==team).unwrap()
}
fn epoch(g: &mut Game, team: u8, value: u8) {g.players[team as usize].epoch=value;g.rebuild_modifiers(team,false);}
fn city_prefix(b:&[u8])->&[u8]{
 #[cfg(feature="ee2-up15")]let b=crate::native_attack_clock::split_save(b).unwrap().0;
 b
}
fn tail(b: &[u8]) -> usize { city_ground::split_save(city_prefix(b)).unwrap().0.len() }
fn put(b:&mut [u8],o:usize,v:u32) { b[o..o+4].copy_from_slice(&v.to_le_bytes()); }

#[test]
fn epoch11_to7_gift_preserves_larger_lifetime_band() {
    let mut g=game();let c=city(&g,0);epoch(&mut g,0,10);epoch(&mut g,1,6);
    assert_eq!(g.city_ground_radius(c),10.0);
    let e=g.ents[c];let region=g.map.terr[tile_of(e.x,e.y)];
    assert!(g.apply_diplomacy(0,1,0,0,0.0));assert!(g.apply_diplomacy(1,0,3,0,0.0));
    assert!(g.apply_territory(0,1,0,region as u32,NONE));
    assert_eq!(g.ents[c].team,1);assert_eq!(g.city_ground_radius(c),10.0);
    assert!(g.original_urban_at(e.x+13.5,e.y)); // radius9 ends below12.73
    epoch(&mut g,1,0);assert_eq!(g.city_ground_radius(c),10.0);
    g.tick();assert_eq!(g.city_ground_radius(c),10.0);
    g.save();let loaded=Game::load(&g.savebuf).unwrap();
    assert_eq!(loaded.city_ground_radius(c),10.0);assert_eq!(loaded.state_hash(),g.state_hash());
}

#[test]
fn epochs_grow_and_never_shrink_and_history_changes_hash() {
    let mut g=game();let c=city(&g,0);let low=g.state_hash();
    for ep in 0..15 {epoch(&mut g,0,ep);assert_eq!(g.city_ground_radius(c),7.0+(ep/3)as f32);}
    epoch(&mut g,0,0);assert_eq!(g.city_ground_radius(c),11.0);
    assert_ne!(g.state_hash(),low);
    let high=g.state_hash();g.city_ground[c].radius=7.0;assert_ne!(g.state_hash(),high);
}

#[test]
fn foundations_wait_for_completion_at_the_then_current_epoch() {
    let mut g=game();let c=g.spawn(TC,0,100.0,100.0);
    epoch(&mut g,0,14);g.ents[c].built=0.1;g.sync_city_ground_entry(c);
    assert_eq!(g.city_ground[c].handle,NONE);assert!(!g.original_urban_at(100.0,100.0));
    epoch(&mut g,0,0);g.ents[c].built=0.9999;g.sync_city_ground_entry(c);
    assert_eq!(g.city_ground_radius(c),0.0);
    // Actual instant-building path observes the completed local Ent snapshot.
    g.net=false;g.cheats_enabled=true;g.instant_build=true;
    let mut e=g.ents[c];g.update_building(c,&mut e,&DEFS[TC as usize]);g.ents[c]=e;
    assert_eq!(g.ents[c].built,1.0);assert_eq!(g.city_ground_radius(c),7.0);
    epoch(&mut g,0,6);assert_eq!(g.city_ground_radius(c),9.0);
}

#[test]
fn real_foundation_placement_clears_transient_spawn_registration() {
    let mut g=game();epoch(&mut g,0,14);
    g.players[0].res=[999999.0;3];g.extra_resources[0]=[999999.0;6];
    let mut placed=NOIDX;
    'outer: for y in 8..248 {for x in 8..248 {
        let i=g.place_with_rotation(TC,x,y,0,0);
        if i!=NOIDX {placed=i;break 'outer;}
    }}
    assert_ne!(placed,NOIDX);assert_eq!(g.ents[placed].built,0.0);
    assert_eq!(g.city_ground[placed].handle,NONE);assert_eq!(g.city_ground_radius(placed),0.0);
}

#[test]
fn worker_completion_registers_without_waiting_for_another_frame() {
    let mut g=game();epoch(&mut g,0,6);let c=g.spawn(TC,0,100.0,100.0);
    g.ents[c].built=0.99999;g.sync_city_ground_entry(c);
    let mut worker=g.ents[1];worker.x=102.5;worker.y=100.0;worker.otgt=g.h(c);
    let mut want=(0.0,0.0);let mut moving=false;
    g.do_build(&mut worker,&DEFS[WORKER as usize],&mut want,&mut moving);
    assert_eq!(g.ents[c].built,1.0);assert_eq!(g.city_ground_radius(c),9.0);
}

#[test]
fn death_and_recycled_slot_start_a_new_band_even_after_generation_wrap() {
    let mut g=game();let c=city(&g,0);epoch(&mut g,0,14);let old=g.h(c);
    let(x,y)=(g.ents[c].x,g.ents[c].y);g.ents[c].hp=0.0;
    assert_eq!(g.city_ground_radius(c),0.0);g.kill(c);assert_eq!(g.city_ground[c].handle,NONE);
    epoch(&mut g,0,0);let reused=g.spawn(TC,0,x,y);assert_eq!(reused,c);
    assert_ne!(g.h(c),old);assert_eq!(g.city_ground_radius(c),7.0);
    epoch(&mut g,0,14);g.ents[c].gen=u16::MAX;g.sync_city_ground_entry(c);g.kill(c);
    epoch(&mut g,0,0);assert_eq!(g.spawn(TC,0,x,y),c);assert_eq!(g.ents[c].gen,0);
    assert_eq!(g.city_ground_radius(c),7.0);
}

#[test]
fn source_comparison_retains_float_sum_and_extended_radius_product() {
    let mut g=game();let c=g.spawn(TC,0,0.0,0.0);epoch(&mut g,0,6);
    // Radius9's f32-rounded boundary is below the true product of source floats.
    let rounded=9.0_f32*1.4142135381698608_f32;
    assert!(g.original_urban_at(rounded,0.0));
    assert!(!g.original_urban_at(f32::from_bits(rounded.to_bits()+1),0.0));
    g.ents[c].hp=0.0;assert!(!g.original_urban_at(rounded,0.0));
}

#[test]
fn cgd1_roundtrip_is_exact_and_legacy_synthesizes_only_present_profile() {
    let mut g=game();let c=city(&g,0);epoch(&mut g,0,14);epoch(&mut g,0,0);g.save();
    let bytes=g.savebuf.clone();let offset=tail(&bytes);
    assert_eq!(&bytes[offset..offset+4],&CITY_MAGIC.to_le_bytes());
    assert_eq!(&city_prefix(&bytes)[city_prefix(&bytes).len()-4..],&CITY_END.to_le_bytes());
    let mut loaded=Game::load(&bytes).unwrap();assert_eq!(loaded.city_ground_radius(c),11.0);
    loaded.save();assert_eq!(loaded.savebuf,bytes);
    #[cfg(feature="ee2-up15")]
    {assert!(Game::load(&bytes[..offset]).is_none());return;}
    let legacy=Game::load(&bytes[..offset]).unwrap();assert_eq!(legacy.city_ground_radius(c),7.0);
    // Previous 57 and 56 headers have no CGD tail; also exercise noDIP/noTER.
    let core=36+core::mem::size_of::<[Player;4]>()+core::mem::size_of::<Ent>()*g.high+NN*5;
    for defs in [56u32,57] {for prefix in [0,12,132,236] {
        let mut old=bytes[..core+prefix].to_vec();put(&mut old,12,defs);
        let migrated=Game::load(&old).unwrap();assert_eq!(migrated.city_ground_radius(c),7.0);
    }}
    let dip_only=offset-crate::territory::TERR_BYTES;
    assert!(Game::load(&bytes[..dip_only]).is_some());
}

#[test]
fn legacy_final_stock_research_and_tech_bits_may_coincidentally_equal_city_footer() {
    let mut g=game();g.save();let bytes=g.savebuf.clone();
    let core=36+core::mem::size_of::<[Player;4]>()+core::mem::size_of::<Ent>()*g.high+NN*5;
    for defs in [56u32,57] {for prefix in [12,132,236] {
        let mut old=bytes[..core+prefix].to_vec();put(&mut old,12,defs);
        let end=old.len();put(&mut old,end-4,CITY_END);
        // Another valid resource/research/flag value can also look like a size.
        if prefix>12 {put(&mut old,end-8,20);}
        assert!(city_ground::split_save(&old).unwrap().1.is_none());
        assert_eq!(Game::load(&old).is_some(),!cfg!(feature="ee2-up15"),"legacy defs{defs},prefix{prefix}");
    }}
}

#[test]
fn malformed_city_chunks_are_rejected_before_world_replacement() {
    let mut g=game();g.save();let valid=g.savebuf.clone();let start=tail(&valid);
    let mut cases=Vec::new();
    for (at,value) in [(start,CITY_MAGIC^1),(start+4,2),(start+8,MAXE as u32+1),
        (start+12,NONE),(start+12,g.h(1)),(start+12,g.h(0)^(1<<16)),
        (start+16,f32::NAN.to_bits()),(start+16,f32::INFINITY.to_bits()),
        (start+16,0.0_f32.to_bits()),(start+16,6.0_f32.to_bits()),
        (start+16,12.0_f32.to_bits()),(city_prefix(&valid).len()-8,19),(city_prefix(&valid).len()-4,CITY_END^1)] {
        let mut b=valid.clone();put(&mut b,at,value);cases.push(b);
    }
    let mut b=valid.clone();put(&mut b,start+20,g.h(0));cases.push(b); // duplicate
    let mut b=valid.clone();let a=b[start+12..start+20].to_vec();let z=b[start+20..start+28].to_vec();
    b[start+12..start+20].copy_from_slice(&z);b[start+20..start+28].copy_from_slice(&a);cases.push(b);
    let mut b=valid.clone();put(&mut b,start+8,1);b.drain(start+20..start+28);let len=b.len();put(&mut b,len-8,28);cases.push(b); // missing TC
    for n in 1..=20 {let prefix=city_prefix(&valid);cases.push(prefix[..prefix.len()-n].to_vec());}
    let mut b=valid.clone();b.push(0);cases.push(b);
    let before=g.state_hash();for b in cases {assert!(Game::load(&b).is_none());assert_eq!(g.state_hash(),before);}
    g.save();assert_eq!(g.savebuf,valid);
    epoch(&mut g,0,14);g.save();let mut b=g.savebuf.clone();let start=tail(&b);
    put(&mut b,start+16,7.0_f32.to_bits());assert!(Game::load(&b).is_none());
}

#[test]
fn two_peers_ignore_query_render_and_save_cadence() {
    let mut a=game();let mut b=game();b.local=1;
    for tick in 0..240 {
        if tick==20 {epoch(&mut a,0,14);epoch(&mut b,0,14);}
        if tick==40 {epoch(&mut a,0,0);epoch(&mut b,0,0);}
        a.tick();b.tick();
        for j in 0..(tick%11) {a.original_urban_at(j as f32*13.0,tick as f32*0.7);}
        if tick%3==0 {a.save();}
        if tick%19==0 {b.save();}
        assert_eq!(a.state_hash(),b.state_hash());
        assert_eq!(a.city_ground_radius(0),b.city_ground_radius(0));
    }
    a.save();let loaded=Game::load(&a.savebuf).unwrap();assert_eq!(loaded.state_hash(),b.state_hash());
    assert_ne!(a.city_ground[0].handle,EMPTY.handle);
}
