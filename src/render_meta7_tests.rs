use super::*;
use crate::{defs::*, game::*};

fn records() -> Vec<Record> {
    if count() == 0 { return Vec::new(); }
    unsafe { core::slice::from_raw_parts(ptr(), count() as usize).to_vec() }
}
fn saved() -> Vec<u8> { let p=crate::save(); unsafe{core::slice::from_raw_parts(p,crate::save_len()as usize).to_vec()} }
fn load(bytes:&[u8])->u32{let p=crate::load_alloc(bytes.len()as u32);unsafe{core::ptr::copy_nonoverlapping(bytes.as_ptr(),p,bytes.len());}crate::load_apply()}
fn world(){crate::init(7,0,0,1,0);assert_eq!(crate::skirmish_visibility(2),1);}
fn full(alpha:f32)->u32{crate::render(alpha,0.0,0.0,256.0,256.0,-1,0,0)}

#[test]
fn dto_has_exact_64byte_layout(){
    assert_eq!(core::mem::size_of::<Record>(),64);assert_eq!(core::mem::align_of::<Record>(),4);
    assert_eq!([
        core::mem::offset_of!(Record,handle),core::mem::offset_of!(Record,def),core::mem::offset_of!(Record,team),core::mem::offset_of!(Record,order),
        core::mem::offset_of!(Record,hp_ratio),core::mem::offset_of!(Record,built),core::mem::offset_of!(Record,render_x),core::mem::offset_of!(Record,render_y),
        core::mem::offset_of!(Record,render_altitude),core::mem::offset_of!(Record,sim_x),core::mem::offset_of!(Record,sim_y),core::mem::offset_of!(Record,velocity_x),
        core::mem::offset_of!(Record,velocity_y),core::mem::offset_of!(Record,render_yaw),core::mem::offset_of!(Record,render_time),core::mem::offset_of!(Record,render_flags),
    ],[0,4,8,12,16,20,24,28,32,36,40,44,48,52,56,60]);
}

#[test]
fn preinit_queries_and_empty_render_do_not_trap(){
    unsafe{*core::ptr::addr_of_mut!(crate::GAME)=None;}
    begin_frame();
    assert_eq!(crate::render_meta_version(),1);assert_eq!(crate::render_meta_stride(),64);
    assert!(crate::render_meta_ptr().is_null());assert_eq!(crate::render_meta_len(),0);assert_eq!(crate::render_meta_count(),0);
    for h in[0,1,8192,65536,u32::MAX]{assert_eq!(crate::entity_alive(h),0);}
    let before=session();assert_eq!(full(0.5),0);assert_eq!(session(),before);assert_eq!(count(),0);
    assert_eq!(crate::render_rotated(0.5,0.0,0.0,256.0,256.0,-1,0,0,0),0);
}

#[test]
fn same_stable_sort_carries_actual_handle_zero_and_nonactors(){
    world();let n=full(0.37);let rows=records();assert_eq!(n as usize,rows.len());assert_eq!(crate::render_meta_len(),n*64);
    assert!(rows.iter().any(|r|r.handle==0));assert_eq!(crate::entity_alive(0),1);
    assert!(rows.iter().any(|r|r.handle==NO_ACTOR));
    let game=crate::g();
    for (i,r) in rows.iter().enumerate(){
        let v=&game.inst[i*12..i*12+12];
        assert_eq!([r.render_x,r.render_y,r.render_altitude,r.render_yaw,r.render_time],[v[0],v[1],v[2],v[3],v[10]]);
        assert_eq!(r.render_flags,v[7]as u32);
        if r.handle==NO_ACTOR{assert_eq!([r.def,r.team,r.order],[NO_ACTOR;3]);assert_eq!([r.hp_ratio,r.built,r.sim_x,r.sim_y,r.velocity_x,r.velocity_y],[0.0;6]);}
        else{
            let j=game.get(r.handle).unwrap();let e=&game.ents[j];
            assert_eq!([r.def,r.team,r.order],[e.def as u32,e.team as u32,e.order as u32]);
            assert_eq!([r.hp_ratio,r.built,r.sim_x,r.sim_y,r.velocity_x,r.velocity_y],[(e.hp/game.max_hp(e.team,e.def)).clamp(0.0,1.0),e.built,e.x,e.y,e.vx,e.vy]);
            assert_eq!(v[4],crate::ids::unit_render(e.def));
        }
    }
}

#[test]
fn renders_are_byte_readonly_for_game_save_and_hash(){
    world();let original=saved();let hash=crate::state_hash();let token=session();
    for k in 0..16{
        crate::render_rotated(k as f32/15.0,0.0,0.0,256.0,256.0,0,35,35,(k%4)as u32);
        assert_eq!(saved(),original);assert_eq!(crate::state_hash(),hash);assert_eq!(session(),token);
    }
}

#[test]
fn duplicate_positions_and_render_auxiliaries_never_alias_actor_identity(){
    world();let game=crate::g();let a=game.spawn(WORKER,0,80.5,80.5);let b=game.spawn(WORKER,0,80.5,80.5);
    game.ents[a].hp=game.max_hp(0,WORKER)*0.25;game.ents[b].hp=game.max_hp(0,WORKER)*0.75;
    game.ents[a].sel=true;game.ents[b].sel=true;let ha=game.h(a);let hb=game.h(b);assert_ne!(ha,hb);
    full(0.0);let rows=records();assert_eq!(rows.iter().filter(|r|r.handle==ha).count(),1);assert_eq!(rows.iter().filter(|r|r.handle==hb).count(),1);
    assert_eq!(rows.iter().find(|r|r.handle==ha).unwrap().hp_ratio,0.25);assert_eq!(rows.iter().find(|r|r.handle==hb).unwrap().hp_ratio,0.75);
    assert!(rows.iter().any(|r|r.handle==NO_ACTOR&&r.render_x==80.5&&r.render_y==80.5));
}

#[test]
fn culling_is_not_despawn_and_recycling_checks_generation(){
    world();let game=crate::g();let a=game.spawn(WORKER,0,100.5,100.5);let old=game.h(a);
    full(1.0);assert!(records().iter().any(|r|r.handle==old));
    crate::render(1.0,-30.0,-30.0,-20.0,-20.0,-1,0,0);assert_eq!(count(),0);assert_eq!(crate::entity_alive(old),1);
    // Explicit own fixture for slot reuse, not a claim of a native Death event.
    let game=crate::g();game.ents[a].alive=false;game.ents[a].gen=game.ents[a].gen.wrapping_add(1);game.free.push(a as u16);
    let b=game.spawn(WORKER,0,100.5,100.5);assert_eq!(a,b);let new=game.h(b);assert_ne!(old,new);
    assert_eq!(crate::entity_alive(old),0);assert_eq!(crate::entity_alive(new),1);assert_eq!(crate::entity_alive(NO_ACTOR),0);
    full(1.0);assert!(!records().iter().any(|r|r.handle==old));assert!(records().iter().any(|r|r.handle==new));
}

#[test]
fn session_changes_only_on_init_or_successful_load(){
    world();full(0.5);let bytes=saved();let token=session();let hash=crate::state_hash();let rows=records();let frame_before=frame();
    let mut bad=bytes.clone();bad[0]^=1;assert_eq!(load(&bad),0);
    assert_eq!(session(),token);assert_eq!(frame(),frame_before);assert_eq!(records(),rows);assert_eq!(saved(),bytes);assert_eq!(crate::state_hash(),hash);
    assert_eq!(load(&bytes),1);assert_ne!(session(),token);assert_eq!(frame(),0);assert_eq!(count(),0);assert!(ptr().is_null());
    let second=session();crate::init(7,0,0,1,0);assert_ne!(session(),second);assert_eq!(frame(),0);assert_eq!(count(),0);
}

#[test]
fn ghost_scaffold_and_bar_rows_have_no_actor_handle(){
    world();let game=crate::g();let i=game.spawn(HOUSE,0,60.5,60.5);game.ents[i].built=0.4;game.ents[i].sel=true;let handle=game.h(i);
    let n=crate::render_rotated(0.5,0.0,0.0,256.0,256.0,HOUSE as i32,60,60,1);let rows=records();assert_eq!(rows.len(),n as usize);
    assert_eq!(rows.iter().filter(|r|r.handle==handle).count(),1);
    assert_eq!(rows.iter().find(|r|r.handle==handle).unwrap().built,0.4);
    assert!(rows.iter().any(|r|r.handle==NO_ACTOR&&(r.render_flags&(crate::render::FL_GHOST_OK as u32|crate::render::FL_GHOST_BAD as u32))!=0));
    let game=crate::g();for(i,r)in rows.iter().enumerate(){let id=game.inst[i*12+4]as u32;if crate::ids::render_kind(id)!=0{assert_eq!(r.handle,NO_ACTOR);}}
}

#[test]
fn capacity_truncation_keeps_all_three_buffers_aligned(){
    world();let game=crate::g();game.map.res.fill(crate::map::R_WOOD);game.map.blocked.fill(false);game.vis.fill(255);
    let n=full(1.0);assert_eq!(n,60_000);assert_eq!(count(),n);assert_eq!(crate::render_meta_len(),n*64);
    assert!(records().iter().all(|r|r.handle==NO_ACTOR));assert_eq!(crate::g().inst.len(),n as usize*12);
    assert_eq!(crate::g().ranges.chunks_exact(2).map(|r|r[1]).sum::<u32>(),n);
}
