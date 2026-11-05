use super::*;
use crate::{pack8 as p,pack8_defs as pd,pack8_save as ps,cmd::*};

fn world(epoch:u8)->Game{
 let mut g=Game::new(7,0,0,2,0);for i in 0..g.high{g.kill(i);}g.territory.claims.fill(NONE);
 for t in 0..2{g.players[t].epoch=epoch;g.players[t].ai=false;g.rebuild_modifiers(t as u8,false);}
 // A deliberately explicit test terrain, not an imported native map claim.
 for y in 60..145{for x in 60..145{let k=y*N+x;g.map.ter[k]=T_GRASS;g.map.res[k]=R_NONE;g.map.blocked[k]=false;g.map.upd_pass(k);}}
 g.team_vis.fill(255);g.diplomacy.visibility=2;g.build_grid();g
}
fn actor(g:&mut Game)->usize{let i=g.spawn(20,0,100.5,100.5);assert!(g.pack_spec(&g.ents[i]).unwrap().capable);i}
fn seal(b:&mut[u8]){let n=b.len()-12;let h=b[..n].iter().fold(2166136261u32,|a,&v|(a^v as u32).wrapping_mul(16777619));b[n+4..n+8].copy_from_slice(&h.to_le_bytes());}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_le_bytes());}

#[test]fn pack8_source_capability_is_states_not_positive_time(){
 assert!(!pd::lookup(8,0).unwrap().capable);assert!(!pd::lookup(20,0).is_some_and(|r|r.capable));assert!(pd::lookup(20,1).unwrap().capable);
 assert!(!pd::lookup(58,5).unwrap().capable);assert!(!pd::lookup(58,7).unwrap().capable);assert!(pd::lookup(58,9).unwrap().capable);
 for id in[65,68,76,78]{assert!(pd::ROWS.iter().any(|r|r.id==id&&r.capable));}
}
#[test]fn pack8_activation_and_resume_preserve_zero_stamp_quirk_and_strict_deadline(){
 let mut g=world(1);let i=actor(&mut g);let e=g.ents[i];assert!(g.pack_enqueue(i,&e,true,true));assert!(!g.pack_states[i].started);assert_eq!(g.pack_states[i].start,0.0);
 g.tick=100;g.pack_activate(i);let s=g.pack_states[i];assert_eq!(s.start,5.0);assert_eq!(s.visual_start,5.0);
 g.tick=110;g.pack_activate(i);assert_eq!(g.pack_states[i].start,5.0);assert_eq!(g.pack_states[i].visual_start,5.0);
 g.tick=120;g.pack_tick_actor(i,&e);assert_eq!(g.pack_states[i].phase,p::UNPACK);
 g.tick=121;g.pack_tick_actor(i,&e);assert_eq!(g.pack_states[i].phase,p::NONE);
 g.pack_states[i]=p::State::fresh(g.h(i));g.tick=0;g.pack_enqueue(i,&e,true,true);g.pack_activate(i);assert_eq!(g.pack_states[i].start,0.0);
 g.tick=1;g.pack_activate(i);assert_eq!(g.pack_states[i].start,DT);assert_eq!(g.pack_states[i].visual_start,0.0);
}
#[test]fn pack8_rejected_queue_still_changes_flags_without_synthetic_task(){
 let mut g=world(1);let i=actor(&mut g);let e=g.ents[i];assert!(g.pack_enqueue(i,&e,true,false));assert_eq!(g.pack_states[i].flags,p::UNPACKED);assert_eq!(g.pack_states[i].phase,p::NONE);assert_eq!(g.pack_states[i].ai_until,0.0);
}
#[test]fn pack8_real_attack_unpacks_before_source_attack_round_and_real_move_packs(){
 let mut g=world(1);let i=actor(&mut g);let t=g.spawn(HOUSE,1,107.5,100.5);g.ents[t].hp=10000.0;g.build_grid();g.order_attack(&[i],t);
 g.tick();assert_eq!(g.pack_states[i].phase,p::UNPACK);assert_eq!(g.pack_states[i].flags&p::UNPACKED,p::UNPACKED);let initial=g.ents[t].hp;let x=g.ents[i].x;
 for _ in 0..19{g.tick();assert_eq!(g.ents[t].hp,initial);assert_eq!(g.ents[i].x,x);}
 for _ in 0..25{g.tick();}assert!(g.attack_clock[i].flags&crate::native_attack_clock::INIT!=0);
 g.ents[t].hp=0.0;g.tick();g.order_move(&[i],125.5,100.5,false);let before=(g.ents[i].x,g.ents[i].y);for _ in 0..10{g.tick();if g.pack_states[i].phase==p::PACK{break;}}assert_eq!(g.pack_states[i].phase,p::PACK,"{:?},order{}",g.pack_states[i],g.ents[i].order);assert_eq!((g.ents[i].x,g.ents[i].y),before);
 for _ in 0..19{g.tick();assert_eq!((g.ents[i].x,g.ents[i].y),before);}
 for _ in 0..8{g.tick();}assert!((g.ents[i].x-before.0).abs()+(g.ents[i].y-before.1).abs()>0.0);
}
#[test]fn pack8_stop_interrupt_does_not_rollback_logical_flag_or_ai_deadline(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);let deadline=g.pack_states[i].ai_until;
 g.tick=25;g.order_stop(&[i]);let s=g.pack_states[i];assert_eq!(s.phase,p::NONE);assert_eq!(s.flags,p::UNPACKED|p::MANUAL);assert_eq!(s.ai_until,deadline);
 for _ in 0..60{g.tick();}assert_eq!(g.pack_states[i].flags,p::UNPACKED|p::MANUAL);assert_eq!(g.pack_states[i].phase,p::NONE);
}
#[test]fn pack8_direct_move_mode_clears_manual_and_queues_pack_before_waypoint(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);for _ in 0..23{g.tick();}let h=g.h(i);g.apply(0,C_MOVE,125.5,100.5,0,&[h]);let s=g.pack_states[i];assert_eq!(s.flags,0);assert_eq!(s.phase,p::PACK);assert_eq!(s.start,g.pack_now());assert!(s.started);
}
#[test]fn pack8_epoch_change_does_not_retroactively_rewrite_captured_task(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);let s=g.pack_states[i];g.players[0].epoch=0;g.rebuild_modifiers(0,false);assert!(!g.pack_spec(&g.ents[i]).is_some_and(|r|r.capable));assert_eq!(g.pack_states[i],s);
 for _ in 0..22{g.tick();}assert_eq!(g.pack_states[i].phase,p::NONE);let e=g.ents[i];assert!(g.pack_wait_for_move(i,&e,1.0,0.0));assert_eq!(g.pack_states[i].flags&p::UNPACKED,0);
}
#[test]fn pack8_modern_roundtrip_preserves_task_visual_and_future_handle_allocator(){
 for with_pack in[false,true]{let mut g=world(1);let i=actor(&mut g);let j=g.spawn(9,0,110.5,100.5);g.kill(j);let k=g.spawn(10,0,112.5,100.5);g.kill(i);g.kill(k);let i=actor(&mut g);if with_pack{g.tick=20;g.order_pack(&[i],true);for _ in 0..6{g.tick();}}
  g.build_grid();g.refresh_display_fog();g.save();let bytes=g.savebuf.clone();let mut l=Game::load(&bytes).unwrap();assert_eq!(g.state_hash(),l.state_hash());assert!(g.free==l.free);l.save();assert!(l.savebuf==bytes,"first diff {:?}",l.savebuf.iter().zip(&bytes).position(|(a,b)|a!=b));
  for n in 0..4{let a=g.spawn(9,0,120.0+n as f32,100.0);let b=l.spawn(9,0,120.0+n as f32,100.0);assert_eq!(g.h(a),l.h(b));assert_eq!(g.state_hash(),l.state_hash());}
  for _ in 0..100{g.tick();l.tick();assert_eq!(g.state_hash(),l.state_hash());}
 }
}
#[test]fn pack8_legacy_load_marks_unknown_history_without_guessing_prior_flags_or_allocator(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);g.save();let inner=ps::split(&g.savebuf).unwrap().0;
 // Deliberately create an old numeric8 component for the migration test. A
 // current OEU9 component alone is rejected because its BIR1 is mandatory.
 #[cfg(feature="ee2-citizen9")]let numeric8=crate::citizen9::normalize_inner(inner,true).unwrap();
 #[cfg(feature="ee2-citizen9")]let inner=numeric8.as_slice();
 assert_eq!(u32::from_le_bytes(inner[..4].try_into().unwrap()),if cfg!(feature="ee2-civ8"){0x3855454f}else{crate::paid_queue7::SAVE_MAGIC});let mut l=Game::load(inner).unwrap();assert_eq!(l.pack_states[i].flags,0);assert!(l.pack_states[i].history_unknown);assert_ne!(l.state_hash(),g.state_hash());
 l.save();let l2=Game::load(&l.savebuf).unwrap();assert_eq!(l.state_hash(),l2.state_hash());assert!(l2.pack_states[i].history_unknown);
}
#[test]fn pack8_death_generation_reuse_clears_all_lifetime_state(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);let h=g.h(i);g.kill(i);assert_eq!(g.pack_states[i],p::EMPTY);let j=actor(&mut g);assert_eq!(i,j);assert_ne!(h,g.h(j));assert_eq!(g.pack_states[j],p::State::fresh(g.h(j)));
}
#[test]fn pack8_render_sidecar_uses_same_sorted_real_handles_and_never_advances_tasks(){
 let mut g=world(1);let i=actor(&mut g);let j=actor(&mut g);g.ents[j].x+=2.0;g.tick=20;g.order_pack(&[i],true);g.build_grid();g.refresh_display_fog();g.save();let before=g.savebuf.clone();let hash=g.state_hash();
 for alpha in[0.0,0.4,1.0]{let count=g.render(alpha,0.0,0.0,256.0,256.0,-1,0,0);assert_eq!(count,crate::pack8_render::count());assert_eq!(count,crate::render_meta7::count());
  let pack=unsafe{core::slice::from_raw_parts(crate::pack8_render::ptr(),count as usize)};let actors=unsafe{core::slice::from_raw_parts(crate::render_meta7::ptr(),count as usize)};
  for(p,a)in pack.iter().zip(actors){assert_eq!(p.handle,a.handle);if p.handle==g.h(i){assert_eq!(p.visual,crate::pack8::UNPACK as u32);assert_eq!(p.visual_duration,1.0);}if p.handle==g.h(j){assert_eq!(p.visual,0);}}
  assert_eq!(g.state_hash(),hash);g.save();assert!(g.savebuf==before);
 }
 g.render(1.0,0.0,0.0,1.0,1.0,-1,0,0);assert!(!g.pack_states[i].history_unknown);assert_eq!(g.state_hash(),hash);
}
#[test]fn pack8_malformed_envelope_records_allocator_and_inner_are_atomic(){
 let mut g=world(1);let i=actor(&mut g);g.tick=20;g.order_pack(&[i],true);g.save();let good=g.savebuf.clone();let before=g.state_hash();let core=u32::from_le_bytes(good[8..12].try_into().unwrap())as usize;let r=ps::HEADER+core;let mut cases=Vec::new();
 for(o,v)in[(0,0),(4,if cfg!(feature="ee2-citizen9"){5}else{4}),(8,u32::MAX),(12,MAXE as u32+1),(16,0),(20,DEFS.len()as u32+1),(24,0),(56,0),(60,0),(r,g.h(i)^65536),(r+4,1),(r+8,3),(r+12,2),(r+16,2),(r+20,f32::NAN.to_bits()),(r+24,(-1.0f32).to_bits()),(r+28,f32::INFINITY.to_bits()),(r+32,(1000.0f32).to_bits()),(r+36,4),(r+44,(-1.0f32).to_bits()),(ps::HEADER+12,57)]{let mut b=good.clone();put(&mut b,o,v);seal(&mut b);cases.push(b);}
 let a=r+ps::RECORD;let mut b=good.clone();put(&mut b,a,i as u32);seal(&mut b);cases.push(b);
 let mut b=good.clone();let v=b[a..a+4].to_vec();b[a+4..a+8].copy_from_slice(&v);seal(&mut b);cases.push(b);
 for n in[0,1,4,35,63,64,good.len()-1,good.len()-12]{cases.push(good[..n].to_vec());}let mut b=good.clone();b.push(0);cases.push(b);
 for bad in cases{assert!(Game::load(&bad).is_none());assert_eq!(g.state_hash(),before);}g.save();assert!(g.savebuf==good);
}
#[test]fn pack8_lockstep_commands_render_cadence_and_midtask_load_are_deterministic(){
 let mut a=world(1);let i=actor(&mut a);a.save();let mut b=Game::load(&a.savebuf).unwrap();b.local=1;let h=a.h(i);
 for tick in 0..180{if tick==1{for g in[&mut a,&mut b]{g.apply(0,C_PACK,0.0,0.0,1,&[h]);}}if tick==45{for g in[&mut a,&mut b]{g.apply(0,C_MOVE,125.5,100.5,0,&[h]);}}if tick==80{a.save();a=Game::load(&a.savebuf).unwrap();}
  a.tick();b.tick();for _ in 0..tick%4{a.render(0.4,0.0,0.0,256.0,256.0,-1,0,0);}if tick%7==0{b.save();}assert_eq!(a.state_hash(),b.state_hash());
 }
 let before=a.state_hash();a.apply(1,C_PACK,0.0,0.0,1,&[h]);a.apply(0,C_PACK,0.0,0.0,2,&[h]);a.apply(0,C_PACK,1.0,0.0,1,&[h]);a.apply(0,C_PACK,0.0,0.0,1,&[h^65536]);assert_eq!(a.state_hash(),before);
}
#[test]fn pack8_manual_and_move_wire_commands_apply_on_both_peers_at_the_same_tick(){
 let mut a=world(1);let i=actor(&mut a);a.build_grid();a.save();let mut b=Game::load(&a.savebuf).unwrap();a.net=true;b.net=true;b.local=1;let h=a.h(i);
 for tick in 0..100{if tick==2||tick==35{let before=a.state_hash();let(kind,x,y,c)=if tick==2{(C_PACK,0.0,0.0,1)}else{(C_MOVE,125.5,100.5,0)};a.submit(kind,x,y,c,&[h]);assert_eq!(before,a.state_hash());let wire=std::mem::take(&mut a.outbox);assert_eq!(wire[0],kind);assert_eq!(wire.len(),18);a.inbox=wire.clone();b.inbox=wire;a.net_apply(0);b.net_apply(0);assert_eq!(a.state_hash(),b.state_hash());}
  a.tick();b.tick();if tick%3==0{a.render(0.5,0.0,0.0,256.0,256.0,-1,0,0);}assert_eq!(a.state_hash(),b.state_hash());
 }
 a.save();b.save();assert!(a.savebuf==b.savebuf);
}
#[test]fn pack8_public_eligibility_query_is_readonly_and_uses_current_source_capability(){
 crate::init(7,0,0,1,0);assert_eq!(crate::pack_selected_count(),0);assert_eq!(crate::skirmish_epoch(5),1);
 let g=crate::g();let capable=g.spawn(20,0,100.5,100.5);let worker=g.spawn(WORKER,0,103.5,100.5);let enemy=g.spawn(20,1,106.5,100.5);g.ents[capable].sel=true;g.ents[worker].sel=true;g.ents[enemy].sel=true;g.build_grid();g.save();let bytes=g.savebuf.clone();let before=g.state_hash();
 for _ in 0..5{assert_eq!(crate::pack_selected_count(),1);assert_eq!(crate::state_hash(),before);}let g=crate::g();g.save();assert!(g.savebuf==bytes);g.ents[capable].alive=false;assert_eq!(crate::pack_selected_count(),0);
}
#[cfg(feature="ee2-civ8")]
#[test]fn pack8_all_eighty_definitions_keep_source_capability_and_actual_sorted_metadata(){
 let mut g=world(14);let mut handles=Vec::new();for id in 0..80{let i=g.spawn(id,0,20.5+(id%10)as f32*20.0,20.5+(id/10)as f32*20.0);handles.push((g.h(i),id));g.order_pack(&[i],true);}g.build_grid();g.refresh_display_fog();g.save();let bytes=g.savebuf.clone();let hash=g.state_hash();
 let count=g.render(1.0,0.0,0.0,256.0,256.0,-1,0,0);let actors=unsafe{core::slice::from_raw_parts(crate::render_meta7::ptr(),count as usize)};let packs=unsafe{core::slice::from_raw_parts(crate::pack8_render::ptr(),count as usize)};
 for(h,id)in handles{let at=actors.iter().position(|a|a.handle==h).expect("each actual unit main mesh");assert_eq!(actors[at].def,id as u32);let p=packs[at];assert_eq!(p.handle,h);let capable=pd::lookup(id,14).is_some_and(|s|s.capable);assert_eq!(p.capable,capable as u32);assert_eq!(p.visual,if capable{2}else{0});assert_eq!(p.flags,if capable{10}else{0});}
 assert_eq!(g.state_hash(),hash);g.save();assert!(bytes==g.savebuf);
}
