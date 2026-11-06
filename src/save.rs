//! Save games.
//!
//! The map is regenerated from its seed, then everything that can change is
//! written on top: players, entities, harvested tiles and explored fog.
//! Modern joint8 retains in-flight projectiles in its separate PRJ1 outer
//! sidecar. Older profiles omit them; presentation-only explosions are transient.
//! Persisted attack-round windup/cooldown state is separate for the UP profile.

use crate::defs::*;
use crate::game::*;
use crate::map::*;
use core::mem::size_of;

const MAGIC: u32 = if cfg!(feature="ee2-paidqueue7"){0x3755_454f}else if cfg!(feature="ee2-roster6"){0x3655_454f}else if cfg!(feature="ee2-up15") {0x3555_454f} else if cfg!(feature = "ee2-reference") { 0x3252_454f } else { 0x3250_454f };

/// View a slice of plain `#[repr(C)]` values as bytes.
fn raw<T: Copy>(s: &[T]) -> &[u8] {
    // SAFETY: only used with Ent / Player / u16, which are plain old data; the
    // bytes are written out and read back by the same build (sizes are checked).
    unsafe { core::slice::from_raw_parts(s.as_ptr() as *const u8, core::mem::size_of_val(s)) }
}

fn fill<T: Copy>(dst: &mut [T], src: &[u8]) {
    assert_eq!(core::mem::size_of_val(dst), src.len());
    // SAFETY: as above; `src` holds exactly the bytes `raw` produced for this type.
    unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr() as *mut u8, src.len()) }
}

impl Game {
    pub fn save(&mut self) {
        let mut b = std::mem::take(&mut self.savebuf);
        b.clear();
        let head = [MAGIC, size_of::<Ent>() as u32, size_of::<Player>() as u32, SAVE_DEFINITION_COUNT as u32, self.seed0, self.n_ai, self.tick, self.rng.0, self.high as u32];
        for v in head {
            b.extend_from_slice(&v.to_le_bytes());
        }
        #[cfg(not(feature="ee2-roster6"))]
        {b.extend_from_slice(raw(&self.players));b.extend_from_slice(raw(&self.ents[..self.high]));}
        #[cfg(feature="ee2-roster6")]
        {crate::save6::write_players(&self.players,&mut b);crate::save6::write_entities(&self.ents[..self.high],&mut b);}
        b.extend_from_slice(&self.map.ter);
        b.extend_from_slice(&self.map.res);
        b.extend_from_slice(raw(&self.map.amt));
        b.extend_from_slice(&self.vis);
        let options = self.cheats_enabled as u32 | ((self.god_mode as u32) << 1) | ((self.fog_inverted as u32) << 2);
        #[cfg(feature="ee2-native13")]
        let options = options | if self.infantry_roster_enabled { crate::infantry_roster27::SAVE_FLAG } else { 0 };
        #[cfg(feature="ee2-native13")]
        let options = options | if self.production_roster_enabled { crate::production_roster29::SAVE_FLAG } else { 0 };
        #[cfg(feature="ee2-native13")]
        let options = options | if self.trade_enabled { crate::game::trade30::SAVE_FLAG } else { 0 };
        #[cfg(feature="ee2-native13")]
        let options = options | if self.air_missions_enabled { crate::air31::SAVE_FLAG } else { 0 };
        #[cfg(feature="ee2-native13")]
        let options = options | if self.air_runway_enabled { crate::runway32::SAVE_FLAG } else { 0 };
        #[cfg(feature="ee2-native13")]
        let options = options | if self.air_takeoff_enabled { crate::takeoff33::SAVE_FLAG } else { 0 };
        for v in [options, self.instant_build as u32, self.tech_points] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&0x3153_4552u32.to_le_bytes()); // RES1 research extension
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(raw(&self.research_points));
        for bits in self.researched {b.extend_from_slice(raw(&bits));}
        if cfg!(feature="ee2-reference"){
            b.extend_from_slice(&0x314f_4345u32.to_le_bytes()); // ECO1: six extra stock accounts, no Player layout change
            b.extend_from_slice(&1u32.to_le_bytes());
            b.extend_from_slice(raw(&self.extra_resources));
        }
        self.write_diplomacy(&mut b);self.write_territory(&mut b);
        #[cfg(feature="ee2-reference")]
        self.write_city_ground(&mut b);
        #[cfg(feature="ee2-up15")]
        self.write_attack_clock(&mut b);
        #[cfg(all(feature="ee2-roster6",not(feature="ee2-paidqueue7")))]
        crate::save6::append_descriptor(&mut b);
        #[cfg(feature="ee2-paidqueue7")]
        {self.write_paid_queue(&mut b);crate::paid_queue7::append_descriptor(&mut b);}
        #[cfg(feature="ee2-civ8")]
        crate::civ8_save::write(self,&mut b);
        #[cfg(feature="ee2-citizen9")] if !crate::citizen9::mark_inner(&mut b){b.clear();}
        #[cfg(feature="ee2-pack8")] {if !b.is_empty(){b=self.wrap_pack_save(&b);}}
        #[cfg(feature="ee2-ai9-game-bridge")]
        if let Some(state) = &self.native_research {
            b = crate::ai9_game_save::wrap(self, state, &b).unwrap_or_default();
        }
        #[cfg(feature="ee2-epoch-range13")]
        { b = crate::match_epoch_range13::wrap(b, self.final_epoch); }
        #[cfg(feature="ee2-native13")]
        { b = crate::ee2_territory_victory::wrap(b, self); }
        #[cfg(feature="ee2-native13")]
        { b = crate::ee2_sole_survivor::wrap(b, self); }
        #[cfg(feature="ee2-native13")]
        { b = crate::ee2_regicide::wrap(b, self); }
        #[cfg(feature="ee2-native13")]
        { b = crate::territory_names21::wrap(b, self.territory_names.as_ref()); }
        #[cfg(feature="ee2-native13")]
        { b = crate::city_names22::wrap(b, self.city_names.as_ref()); }
        #[cfg(feature="ee2-native13")]
        { b = crate::city_memory23::wrap(b, self.city_memory.as_ref()); }
        #[cfg(feature="ee2-native13")]
        { b = self.wrap_capture_save(b); }
        #[cfg(feature="ee2-native13")]
        { b = self.wrap_trade_save(b); }
        #[cfg(feature="ee2-native13")]
        { b = self.wrap_air_save(b); }
        #[cfg(feature="ee2-native13")]
        { b = self.wrap_takeoff_save(b); }
        self.savebuf = b;
    }

    pub fn load(buf: &[u8]) -> Option<Game> {
        #[cfg(feature="ee2-native13")]
        let (buf, takeoff) = crate::takeoff33::split_save(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, air) = crate::air31::split_air_save(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, trade) = crate::game::trade30::split_save(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, capture) = split_capture_save(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, city_memory) = crate::city_memory23::split(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, city_names) = crate::city_names22::split(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, territory_names) = crate::territory_names21::split(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, regicide) = crate::ee2_regicide::split(buf)?;
        let actor_definition_count = SAVE_DEFINITION_COUNT;
        #[cfg(feature="ee2-native13")]
        let actor_definition_count = if regicide.is_some() { crate::ee2_regicide::KING as usize + 1 } else { actor_definition_count };
        #[cfg(feature="ee2-native13")]
        let actor_definition_count = if trade.is_some() { crate::trade30_defs::TRADE_DEFINITION_COUNT } else { actor_definition_count };
        #[cfg(feature="ee2-native13")]
        let actor_definition_count = if air.is_some() { DEFS.len() } else { actor_definition_count };
        #[cfg(feature="ee2-native13")]
        let (buf, sole_survivor) = crate::ee2_sole_survivor::split(buf)?;
        #[cfg(feature="ee2-native13")]
        let (buf, territory_victory) = crate::ee2_territory_victory::split(buf)?;
        #[cfg(feature="ee2-epoch-range13")]
        let (buf, final_epoch) = crate::match_epoch_range13::split(buf)?;
        #[cfg(feature="ee2-ai9-game-bridge")]
        let (buf, native_research) = crate::ai9_game_save::split(buf)?;
        #[cfg(feature="ee2-citizen9")] let current_source9=crate::citizen9::current_outer(buf);
        #[cfg(feature="ee2-pack8")] let (pack_core,pack_records)=crate::pack8_save::split(buf)?;
        #[cfg(feature="ee2-pack8")] let buf=pack_core;
        #[cfg(feature="ee2-citizen9")] let source9_normalized=crate::citizen9::normalize_inner(buf,current_source9)?;
        #[cfg(feature="ee2-citizen9")] let buf=source9_normalized.as_slice();
        #[cfg(feature="ee2-civ8")] let(civ_normalized,civilizations)=crate::civ8_save::normalize(buf)?;
        #[cfg(feature="ee2-civ8")] let buf=civ_normalized.as_slice();
        #[cfg(feature="ee2-paidqueue7")] let (paid_normalized,paid_chunk)=crate::paid_queue7::split_input(buf)?;
        #[cfg(feature="ee2-paidqueue7")] let buf=paid_normalized.as_slice();
        #[cfg(feature="ee2-roster6")]
        let normalized=crate::save6::normalize_input(buf, actor_definition_count)?;
        #[cfg(feature="ee2-roster6")]
        let buf=normalized.as_slice();
        #[cfg(feature="ee2-up15")]
        let(buf,attack_chunk)=crate::native_attack_clock::split_save(buf)?;
        #[cfg(feature="ee2-reference")]
        let (buf,city_chunk)=crate::city_ground::split_save(buf)?;
        const H: usize = 9 * 4;
        if buf.len() < H {
            return None;
        }
        let u = |k: usize| u32::from_le_bytes([buf[k * 4], buf[k * 4 + 1], buf[k * 4 + 2], buf[k * 4 + 3]]);
        let high = u(8) as usize;
        if high>MAXE{return None;}
        let (psz, esz) = (size_of::<[Player; 4]>(), size_of::<Ent>() * high);
        // a save from another build has a different layout: refuse it rather than misread it
        let end=H+psz+esz+NN*5;
        let total_trailer=buf.len().checked_sub(end)?;
        let has_territory=total_trailer>=crate::diplomacy::DIP_BYTES+crate::territory::TERR_BYTES;
        let remainder=if has_territory{total_trailer-crate::territory::TERR_BYTES}else{total_trailer};
        let has_diplomacy=remainder>=crate::diplomacy::DIP_BYTES;
        let trailer=if has_diplomacy{remainder-crate::diplomacy::DIP_BYTES}else{remainder};
        if u(0) != (if cfg!(feature="ee2-paidqueue7"){0x3655_454f}else{MAGIC}) || u(1) as usize != size_of::<Ent>() || u(2) as usize != size_of::<Player>() || (u(3) as usize != SAVE_DEFINITION_COUNT && u(3)!=47 && !(cfg!(feature="ee2-reference")&&(u(3)==53||u(3)==55||u(3)==56))) || ![0,12,132,236].contains(&trailer) {
            return None;
        }
        #[cfg(feature="ee2-up15")]
        if u(3) as usize!=SAVE_DEFINITION_COUNT || trailer!=236 || !has_diplomacy || !has_territory || city_chunk.is_none(){return None;}
        if u(5)>3{return None;}
        #[cfg(feature="ee2-reference")]
        if city_chunk.is_some() && (trailer!=236 || !has_diplomacy || !has_territory || u(3) as usize!=SAVE_DEFINITION_COUNT) {return None;}
        let humans=if has_diplomacy{u32::from_le_bytes(buf[end+trailer+8..end+trailer+12].try_into().ok()?)}else{1};
        if !(1..=2).contains(&humans){return None;}
        let mut g = Game::new(u(4), u(5), 0, humans, 0);
        g.tick = u(6);
        g.rng = Rng(u(7));
        let mut o = H;
        let mut take = |n: usize| {
            o += n;
            &buf[o - n..o]
        };
        fill(&mut g.players, take(psz));
        // wipe what `new` spawned, then drop the saved world in
        for i in 0..NN {
            g.map.blocked[i] = false;
        }
        for e in g.ents.iter_mut() {
            e.alive = false;
        }
        g.territory.claims.fill(NONE);
        fill(&mut g.ents[..high], take(esz));
        if cfg!(feature="ee2-reference")&&trailer<236 {for e in &mut g.ents[..high]{if e.alive&&e.def==WORKER{e.carry=0.0;e.q[1]=0;e.q[2]=0;}}}
        g.map.ter.copy_from_slice(take(NN));
        g.map.res.copy_from_slice(take(NN));
        fill(&mut g.map.amt, take(NN * 2));
        g.vis.copy_from_slice(take(NN));
        let end = H + psz + esz + NN * 5;
        #[cfg(feature="ee2-native13")]
        {
            g.trade_enabled = false;
            g.air_missions_enabled = false;
            g.air_runway_enabled = false;
            g.air_takeoff_enabled = false;
        }
        if trailer >= 12 {
            let v = |k| u32::from_le_bytes(buf[end+k..end+k+4].try_into().unwrap());
            let allowed = if cfg!(feature = "ee2-up15") { 7 } else { 1 };
            #[cfg(feature="ee2-native13")]
            let allowed = allowed | crate::infantry_roster27::SAVE_FLAG | crate::production_roster29::SAVE_FLAG |
                crate::game::trade30::SAVE_FLAG | crate::air31::SAVE_FLAG | crate::runway32::SAVE_FLAG |
                crate::takeoff33::SAVE_FLAG;
            let flags = v(0);
            if flags & !allowed != 0 || v(4) > 1 { return None; }
            g.cheats_enabled = flags & 1 != 0;
            g.god_mode = flags & 2 != 0;
            g.fog_inverted = flags & 4 != 0;
            #[cfg(feature="ee2-native13")]
            {
                g.infantry_roster_enabled = flags & crate::infantry_roster27::SAVE_FLAG != 0;
                g.production_roster_enabled = flags & crate::production_roster29::SAVE_FLAG != 0;
                g.trade_enabled = flags & crate::game::trade30::SAVE_FLAG != 0;
                g.air_missions_enabled = flags & crate::air31::SAVE_FLAG != 0;
                g.air_runway_enabled = flags & crate::runway32::SAVE_FLAG != 0;
                g.air_takeoff_enabled = flags & crate::takeoff33::SAVE_FLAG != 0;
                if g.air_runway_enabled && !g.air_missions_enabled { return None; }
                if g.air_takeoff_enabled && !g.air_runway_enabled { return None; }
            }
            g.instant_build = v(4) == 1 && (cfg!(feature = "ee2-up15") || g.cheats_enabled);
            g.tech_points = v(8);
        }
        if trailer>=132 {
            let ext=&buf[end+12..];
            if u32::from_le_bytes(ext[..4].try_into().ok()?)!=0x3153_4552 || u32::from_le_bytes(ext[4..8].try_into().ok()?)!=1 {return None;}
            fill(&mut g.research_points,&ext[8..24]);
            for t in 0..4 {fill(&mut g.researched[t],&ext[24+t*24..48+t*24]);}
            if g.research_points.iter().any(|v| !v.is_finite() || *v<0.0) {return None;}
        }
        if trailer==236 {
            let ext=&buf[end+132..];
            if u32::from_le_bytes(ext[..4].try_into().ok()?)!=0x314f_4345 || u32::from_le_bytes(ext[4..8].try_into().ok()?)!=1{return None;}
            fill(&mut g.extra_resources,&ext[8..104]);
        }
        if has_diplomacy{g.read_diplomacy(&buf[end+trailer..end+trailer+crate::diplomacy::DIP_BYTES])?;}
        if (g.god_mode || g.fog_inverted) && g.humans != 1 { return None; }
        if has_territory{g.read_territory(&buf[end+trailer+crate::diplomacy::DIP_BYTES..])?;}
        if g.extra_resources.iter().flatten().any(|v|!v.is_finite()||*v<0.0)||g.players.iter().flat_map(|p|p.res).any(|v|!v.is_finite()||v<0.0){return None;}
        if g.map.res.iter().any(|&v|v as usize>crate::economy::RESOURCE_COUNT){return None;}
        #[cfg(feature = "ee2-reference")]
        if u(3)==47 {
            // Preserve the cultural age of saves made by the former eight-epoch ruleset.
            let epochs=[0u8,2,5,9,10,11,13,14];
            let old_epochs=g.players.map(|p|p.epoch as usize);
            for p in &mut g.players {
                if p.epoch>7 {return None;}
                p.epoch=epochs[p.epoch as usize]; p.researching=false;p.tech_busy=0;
            }
            for e in &mut g.ents[..high] {
                if e.qn>5 || e.team as usize>=g.nplayers {return None;}
                if !e.alive || !DEFS.get(e.def as usize)?.building {continue;}
                let mut n=0;
                for j in 0..e.qn as usize {
                    let q=e.q[j];
                    if q>=TECH0 {
                        let cost=if q==RESEARCH {
                            let ep=old_epochs[e.team as usize];if ep>=7 {return None;} EPOCH_COST[ep+1]
                        }else {
                            let tech=TECHS.get((q-TECH0) as usize)?;tech.cost
                        };
                        for r in 0..3 {g.players[e.team as usize].res[r]+=cost[r] as f32;}
                        if j==0 {e.qt=0.0;}
                    }
                    else {e.q[n]=q;n+=1;}
                }
                e.qn=n as u8;
            }
            g.research_points[0]+=g.tech_points as f32;g.tech_points=0;
        }
        #[cfg(feature="ee2-epoch-range13")]
        { g.final_epoch = final_epoch; }
        for p in &g.players[..g.nplayers] {if p.epoch>g.match_final_epoch() {return None;}}
        #[cfg(feature="ee2-native13")]
        let valid_definition = |id: DefId| {
            (id as usize) < actor_definition_count &&
                (id != crate::ee2_regicide::KING || regicide.is_some()) &&
                (!crate::trade30_defs::is_trader(id) || trade.is_some()) &&
                (id != crate::air31_defs::FIGHTER_BOMBER || air.is_some())
        };
        #[cfg(not(feature="ee2-native13"))]
        let valid_definition = |id: DefId| (id as usize) < actor_definition_count;
        for e in &g.ents[..high] {
            if !e.alive {continue;}
            if !valid_definition(e.def) || e.team as usize>=g.nplayers || e.qn>5 || e.ncargo as usize>CARGO_MAX {return None;}
            if !e.x.is_finite()||!e.y.is_finite()||!e.rot.is_finite()||!e.prot.is_finite(){return None;}
            if DEFS[e.def as usize].building{let(tx,ty,side)=Game::building_bounds(e);if tx<0||ty<0||tx+side>N as i32||ty+side>N as i32{return None;}}
            if e.cargo[..e.ncargo as usize].iter().any(|&c| !valid_definition(c)) {return None;}
            if DEFS[e.def as usize].building && e.q[..e.qn as usize].iter().any(|&q| {
                #[cfg(feature="ee2-native13")]
                if trade.is_some() && crate::trade30_defs::is_trader(q as DefId) { return false; }
                #[cfg(feature="ee2-native13")]
                if air.is_some() && q as DefId == crate::air31_defs::FIGHTER_BOMBER { return false; }
                q != RESEARCH && !original_research(q) && q as usize >= SAVE_DEFINITION_COUNT &&
                    (q < TECH0 || (q - TECH0) as usize >= TECHS.len())
            }) { return None; }
            #[cfg(feature="ee2-epoch-range13")]
            if g.final_epoch < MAX_EPOCH && DEFS[e.def as usize].building &&
                g.players[e.team as usize].epoch == g.final_epoch && e.q[..e.qn as usize].contains(&RESEARCH) {
                return None;
            }
        }
        #[cfg(feature="ee2-civ8")]
        {
            if civilizations[g.nplayers..].iter().any(|&c|c!=crate::civ8_runtime::LEGACY_NEUTRAL){return None;}
            g.civilizations=civilizations;
        }
        for t in 0..g.nplayers {g.rebuild_modifiers(t as u8,false);}
        #[cfg(all(feature="ee2-reference",not(feature="ee2-up15")))]
        if trailer<236&&(u(3)==47||u(3)==53){
            let old=crate::ee2_profile::legacy(crate::defs::prototype_defs());
            for e in &mut g.ents[..high]{
                if !e.alive{continue;}let id=e.def as usize;let base=old[id].hp;let mut maximum=base;
                for (k,node) in crate::ee2_research::NODES.iter().enumerate(){
                    if g.researched[e.team as usize][k/32]>>(k%32)&1==0{continue;}
                    for effect in node.effects{if effect.field==0&&effect.mask>>id&1==1{match effect.method{0=>maximum+=base*effect.value*0.01,1=>maximum+=effect.value,_=>maximum=effect.value}}}
                }
                let target=g.modifiers[e.team as usize][id][0];e.hp=(e.hp/maximum.max(0.01)*target).clamp(0.0,target);
            }
        }
        g.high = high;
        #[cfg(feature="ee2-reference")]
        g.read_city_ground(city_chunk)?;
        #[cfg(feature="ee2-up15")]
        g.read_attack_clock(attack_chunk)?;
        #[cfg(feature="ee2-paidqueue7")] g.read_paid_queue(paid_chunk.as_deref())?;
        #[cfg(feature="ee2-civ8")]
        {
            for e in &g.ents[..high]{
                if e.alive&&DEFS[e.def as usize].building{
                    for &job in &e.q[..e.qn as usize]{
                        if (job as usize)<DEFS.len()&&!g.definition_civ_available(e.team,job as DefId){return None;}
                    }
                }
            }
            if g.queue_prices.iter().any(|p|(61..DEFS.len()as u32).contains(&p.job)&&p.inferred){return None;}
            g.civ_setup_open=false;
        }
        g.free = (0..MAXE).rev().filter(|&i| !g.ents[i].alive).map(|i| i as u16).collect();
        for i in 0..high {
            g.ents[i].fslot = u16::MAX; // flow-field cache starts empty
            if g.ents[i].alive && DEFS[g.ents[i].def as usize].building {
                let(tx0,ty0,s)=Game::building_bounds(&g.ents[i]);
                for ty in ty0..ty0+s{for tx in tx0..tx0+s{g.map.blocked[ty as usize*N+tx as usize]=true;}}
            }
        }
        if cfg!(feature="ee2-reference")&&trailer<236 {g.map.seed_extra_resources(g.nplayers);}
        for i in 0..NN {
            g.map.upd_pass(i);
            g.map.recolor(i);
        }
        g.map.nav_ver += 1;
        // Validate claimed city handles before ownership reconstruction can
        // replace a malformed or stale handle with a convenient different city.
        if has_territory{
            for k in 0..32{let h=g.territory.claims[k];if h==NONE{continue;}let i=g.get(h)?;let e=g.ents[i];if k>=g.map.tseeds.len()||e.def!=TC||e.hp<=0.0||g.map.terr[tile_of(e.x,e.y)] as usize!=k{return None;}}
        }
        if cfg!(feature="ee2-reference") && u(3)<57 {g.seed_legacy_territory_claims();}
        #[cfg(feature="ee2-native13")]
        g.restore_trade_save(trade)?;
        #[cfg(feature="ee2-native13")]
        g.restore_air_save(air)?;
        #[cfg(feature="ee2-native13")]
        g.restore_takeoff_save(takeoff)?;
        g.build_grid();
        if has_territory&&!g.validate_territory_state(){return None;}
        if !has_diplomacy {
            g.team_vis.fill(0);
            g.team_vis[..NN].copy_from_slice(&g.vis);
            g.update_fog();
        }else{g.refresh_display_fog();}
        g.terr_dirty = true;
        #[cfg(feature="ee2-pack8")] g.read_pack_save(pack_records)?;
        #[cfg(feature="ee2-ai9-game-bridge")]
        if let Some(records) = native_research {
            g.native_research = Some(crate::ai9_game_save::decode(&g, records)?);
        }
        #[cfg(feature="ee2-native13")]
        if let Some((rule, outcome)) = territory_victory {
            g.restore_territory_victory(rule, outcome)?;
        }
        #[cfg(feature="ee2-native13")]
        if let Some((rule, outcome)) = sole_survivor {
            g.restore_sole_survivor(rule, outcome)?;
        }
        #[cfg(feature="ee2-native13")]
        if let Some((rule, outcome)) = regicide {
            g.restore_regicide(rule, outcome)?;
        }
        #[cfg(feature="ee2-native13")]
        if let Some(names) = territory_names {
            g.restore_territory_names(names)?;
        }
        #[cfg(feature="ee2-native13")]
        if let Some(names) = city_names {
            g.restore_city_names(names)?;
        }
        #[cfg(feature="ee2-native13")]
        if let Some(memory) = city_memory {
            g.restore_city_memory(memory)?;
        }
        #[cfg(feature="ee2-native13")]
        g.restore_capture_save(capture)?;
        Some(g)
    }
}
