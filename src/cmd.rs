//! Player commands as data.
//!
//! Every order a player gives becomes one small record. Single-player applies it
//! on the spot; in lockstep multiplayer it is serialised into `outbox`, sent to
//! the peer, and both sides apply it on the same future tick. `apply` is the
//! only way input reaches the simulation, and it only reads simulation state,
//! so both peers stay bit-identical.

use crate::defs::*;
use crate::game::*;
use crate::map::*;

pub const C_MOVE: u8 = 1;
pub const C_AMOVE: u8 = 2;
pub const C_ATTACK: u8 = 3;
pub const C_WORK: u8 = 4;
pub const C_GATHER: u8 = 5;
pub const C_BOARD: u8 = 6;
pub const C_UNLOAD: u8 = 7;
pub const C_RALLY: u8 = 8;
pub const C_STOP: u8 = 9;
pub const C_PLACE: u8 = 10;
pub const C_TRAIN: u8 = 11;
pub const C_CANCEL: u8 = 12;
pub const C_DELETE: u8 = 13;
pub const C_DIPLOMACY:u8=14;
pub const C_TERRITORY:u8=15;
pub const C_PLACE_ROTATED:u8=16;
#[cfg(feature="ee2-pack8")]pub const C_PACK:u8=17;
#[cfg(feature="ee2-native13")]pub const C_CAPTURE:u8=18;
#[cfg(feature="ee2-native13")]pub const C_TRADE_ROUTE:u8=19;
#[cfg(feature="ee2-native13")]pub const C_AIR_MISSION:u8=20;
#[cfg(feature="ee2-native13")]pub(crate) const C_SOLO_RESIGN:u8=21;
#[cfg(feature="ee2-native13")]pub(crate) const C_NET_RESIGN:u8=22;
#[cfg(feature="ee2-native13")]pub(crate) const C_BLESS:u8=23;
#[cfg(feature="ee2-native13")]pub(crate) const C_GATHER_INTEL:u8=24;
#[cfg(feature="ee2-native13")]pub(crate) const C_EXTRACTION:u8=25;
#[cfg(feature="ee2-native13")]pub(crate) const C_MARKET:u8=26;
#[cfg(feature="ee2-native13")]pub(crate) const C_CROWN_CHOOSE:u8=27;
#[cfg(feature="ee2-native13")]pub(crate) const C_LEADER_RALLY:u8=28;
#[cfg(feature="ee2-native13")]pub(crate) const C_LEADER_MOBILIZATION:u8=29;
#[cfg(feature="ee2-native13")]pub(crate) const C_LEADER_CONSTRUCTION:u8=30;
#[cfg(feature="ee2-native13")]pub(crate) const C_LEADER_DIVERSION:u8=31;
#[cfg(feature="ee2-native13")]pub(crate) const C_LEADER_PURCHASE:u8=32;
#[cfg(feature="ee2-native13")]pub(crate) const C_IMPERIALISM:u8=33;
#[cfg(feature="ee2-native13")]pub(crate) const SOLO_RESIGN_SAVE_FLAG:u32=1<<11;

const HEAD: usize = 14; // kind, count, a, b, c

impl Game {
    /// Issue a command as the local player. `hs` are entity handles.
    pub fn submit(&mut self, kind: u8, a: f32, b: f32, c: i32, hs: &[u32]) {
        #[cfg(feature="ee2-native13")]
        if self.solo_resigned || self.network_match_finished() || self.network_observer(self.local) { return; }
        #[cfg(feature="ee2-civ8")]
        if a.is_finite()&&b.is_finite()&&(1..=if cfg!(feature="ee2-pack8"){17}else{16}).contains(&kind){self.civ_setup_open=false;}
        let enqueue = self.net;
        #[cfg(feature="ee2-native13")]
        let enqueue = enqueue || self.solo_command_queue;
        if !enqueue {
            let team = self.local;
            self.apply(team, kind, a, b, c, hs);
            return;
        }
        // big selections are split so the count fits a byte
        let mut chunks = hs.chunks(250);
        let mut first = true;
        loop {
            let part = chunks.next();
            if part.is_none() && !first {
                break;
            }
            first = false;
            let part = part.unwrap_or(&[]);
            self.outbox.push(kind);
            self.outbox.push(part.len() as u8);
            self.outbox.extend_from_slice(&a.to_le_bytes());
            self.outbox.extend_from_slice(&b.to_le_bytes());
            self.outbox.extend_from_slice(&c.to_le_bytes());
            for h in part {
                self.outbox.extend_from_slice(&h.to_le_bytes());
            }
        }
    }

    /// Apply every command in `inbox` on behalf of `team`.
    pub fn net_apply(&mut self, team: u8) {
        let buf = std::mem::take(&mut self.inbox);
        self.apply_commands(team, &buf);
        self.inbox = buf;
    }

    #[cfg(feature="ee2-native13")]
    pub(crate) fn apply_solo_commands(&mut self) {
        if self.net || !self.solo_command_queue || self.outbox.is_empty() { return; }
        let mut buf = std::mem::take(&mut self.outbox);
        self.apply_commands(self.local, &buf);
        buf.clear();
        if self.outbox.is_empty() { self.outbox = buf; }
    }

    fn apply_commands(&mut self, team: u8, buf: &[u8]) {
        let mut o = 0;
        let mut hs: Vec<u32> = Vec::new();
        while o + HEAD <= buf.len() {
            let (kind, n) = (buf[o], buf[o + 1] as usize);
            let f = |k: usize| [buf[o + k], buf[o + k + 1], buf[o + k + 2], buf[o + k + 3]];
            let (a, b, c) = (f32::from_le_bytes(f(2)), f32::from_le_bytes(f(6)), i32::from_le_bytes(f(10)));
            o += HEAD;
            if o + n * 4 > buf.len() {
                break;
            }
            hs.clear();
            for k in 0..n {
                hs.push(u32::from_le_bytes([buf[o + k * 4], buf[o + k * 4 + 1], buf[o + k * 4 + 2], buf[o + k * 4 + 3]]));
            }
            o += n * 4;
            self.apply(team, kind, a, b, c, &hs);
        }
    }

    pub fn apply(&mut self, team: u8, kind: u8, a: f32, b: f32, c: i32, hs: &[u32]) {
        if !a.is_finite() || !b.is_finite() || team as usize >= self.nplayers {
            return;
        }
        #[cfg(feature="ee2-native13")]
        if self.solo_resigned || self.network_match_finished() || self.network_observer(team) { return; }
        #[cfg(feature="ee2-native13")]
        if kind == C_NET_RESIGN {
            if self.grouped_coop.is_none() && a == 0.0 && b == 0.0 && c == 0 && hs.is_empty() { self.apply_network_departure(team); }
            return;
        }
        #[cfg(feature="ee2-native13")]
        if kind == C_SOLO_RESIGN {
            if !self.net && self.bench == 0 && self.solo_command_queue && self.winner == 0 &&
                team == self.local && a == 0.0 && b == 0.0 && c == 0 && hs.is_empty() {
                self.solo_resigned = true;
                self.players[team as usize].alive = false;
                self.winner = 2;
            }
            return;
        }
        #[cfg(feature = "ee2-native13")]
        if kind == C_AIR_MISSION {
            self.air_command(team, a, b, c, hs);
            return;
        }
        #[cfg(feature="ee2-native13")]
        if kind == C_MARKET {
            if let &[host] = hs {
                if b == 0.0 && (0.0..=8.0).contains(&a) && a.fract() == 0.0 {
                    self.market_trade(team, host, a as u8, c);
                }
            }
            return;
        }
        #[cfg(feature="ee2-native13")]
        if kind == C_CROWN_CHOOSE {
            if self.net && hs.is_empty() && b == 0.0 && (0.0..=2.0).contains(&a) && a.fract() == 0.0 {
                if let Ok(choice) = u8::try_from(c) { self.choose_crown_power_for(team, a as usize, choice); }
            }
            return;
        }
        #[cfg(feature="ee2-civ8")]
        if (1..=if cfg!(feature="ee2-pack8"){17}else{16}).contains(&kind){self.civ_setup_open=false;}
        if kind==C_TERRITORY{
            if c<0||(c as u32)&0xffff_0000!=0{return;}
            let bits=c as u32;
            #[cfg(feature="ee2-native13")]
            if bits>>8==crate::territory::TERR_GIFT_BATCH_ACTION {
                if a == -1.0 && b == -1.0 {
                    self.apply_territory_gift_batch(team, bits & 255, hs);
                }
                return;
            }
            if !hs.is_empty(){return;}
            let dec=|v:f32|if v==-1.0{Some(u32::MAX)}else if v>=0.0&&v<32.0&&v.fract()==0.0{Some(v as u32)}else{None};
            if let(Some(give),Some(want))=(dec(a),dec(b)){self.apply_territory(team,bits&255,(bits>>8)&255,give,want);}return;
        }
        if kind==C_DIPLOMACY {
            if c < 0 || (c as u32) & 0xff00_0000 != 0 { return; }
            let bits = c as u32;
            let (target, action, terms) = (bits & 255, (bits >> 8) & 255, (bits >> 16) & 255);
            if hs.is_empty() && b == 0.0 {
                self.apply_diplomacy(team, target, action, terms, a);
            }
            #[cfg(feature = "ee2-native13")]
            if b >= 0.0 && b <= hs.len() as f32 && b.fract() == 0.0 {
                if let Some(territories) = crate::alliance_territories::AllianceTerritories::new(b as usize, hs) {
                    self.apply_diplomacy_territories(team, target, action, terms, a, territories);
                }
            }
            return;
        }
        // never trust the wire: only our own living entities take orders
        let ids: Vec<usize> = hs.iter().filter_map(|&h| self.get(h)).filter(|&i| self.ents[i].team == team).collect();
        #[cfg(feature = "ee2-native13")]
        let ids: Vec<_> = ids.into_iter().filter(|&i| !self.dock_parked(i, &self.ents[i])).collect();
        let target = self.get(c as u32);
        match kind {
            #[cfg(feature="ee2-pack8")]
            C_PACK=>{if a==0.0&&b==0.0&&(c==0||c==1){self.order_pack(&ids,c==1);}},
            C_MOVE => {self.order_move(&ids,a,b,false);#[cfg(feature="ee2-pack8")]self.pack_direct_move(&ids);},
            C_AMOVE => {self.order_move(&ids,a,b,true);#[cfg(feature="ee2-pack8")]self.pack_direct_move(&ids);},
            C_ATTACK => {
                #[cfg(feature = "ee2-valley71-neutral-qa")]
                if self.valley_neutral_opening() && a == crate::native_valley37::FORCE_ATTACK && b == -1.0 {
                    if let Some(j) = target { self.order_valley_attack(&ids, j); }
                    return;
                }
                if let Some(j) = target.filter(|&j| self.combat_hostile(team,self.ents[j].team)&&self.can_see(team,j)) {
                    self.order_attack(&ids, j);
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_CAPTURE => {
                if a == 0.0 && b == 0.0 {
                    if let Some(j) = target { self.order_capture(&ids, j); }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_TRADE_ROUTE => {
                let word = |v: f32| (0.0..=65535.0).contains(&v) && v.fract() == 0.0;
                if word(a) && word(b) {
                    self.order_trade_route(team, a as u32 | ((b as u32) << 16), c as u32, &ids);
                }
            }
            C_WORK => {
                if let Some(j) = target.filter(|&j| self.ents[j].team == team && DEFS[self.ents[j].def as usize].building) {
                    self.order_building(&ids, j);
                }
            }
            C_GATHER => {
                if c >= 0 && (c as usize) < NN && self.map.res[c as usize] != R_NONE && self.real_fog(team,c as usize)>0 {
                    self.order_gather(&ids, c as usize);
                }
            }
            C_BOARD => {
                if let Some(j) = target.filter(|&j| self.ents[j].team == team) {
                    self.order_board(&ids, j);
                }
            }
            C_UNLOAD => self.order_unload(&ids, a, b),
            C_RALLY => {
                for &i in &ids {
                    if DEFS[self.ents[i].def as usize].building {
                        self.ents[i].rx = a.clamp(1.0, N as f32 - 1.0);
                        self.ents[i].ry = b.clamp(1.0, N as f32 - 1.0);
                    }
                }
            }
            C_STOP => self.order_stop(&ids),
            C_PLACE|C_PLACE_ROTATED=>{
                if a.fract()!=0.0||b.fract()!=0.0||a<0.0||b<0.0||a>=N as f32||b>=N as f32||c<0{return;}
                let bits=c as u32;let shift=if cfg!(feature="ee2-roster6"){16}else{8};let mask=(1u32<<shift)-1;
                if(kind==C_PLACE&&bits>=DEFS.len()as u32)||(kind==C_PLACE_ROTATED&&bits>>(shift+8)!=0){return;}
                let def=(bits&mask)as DefId;let turn=bits>>shift;if def as usize>=DEFS.len()||turn>=crate::placement::ROTATION_STEPS{return;}
                self.begin_construction(team,def,a as i32,b as i32,turn,&ids,true);
            }
            C_TRAIN => {
                if let Some(&i)=ids.first().filter(|_|c>=0&&(cfg!(feature="ee2-roster6")||c<=255)) {
                    self.train(i, c as QueueItem);
                }
            }
            C_CANCEL => {
                if let Some(&i) = ids.first() {
                    #[cfg(feature = "ee2-native13")]
                    if !self.net && c == crate::spy46_defs::SPY as i32 {
                        if a == 0.0 && b == 0.0 && self.ents[i].def == UNIVERSITY {
                            let host = &self.ents[i];
                            if let Some(position) = host.q[..host.qn as usize].iter().rposition(|&job| job == c as QueueItem) {
                                self.cancel_paid_job(i, position);
                            }
                        }
                        return;
                    }
                    self.cancel(i);
                }
            }
            C_DELETE => {
                let mut city=false;
                for &i in &ids {
                    #[cfg(feature = "ee2-native13")]
                    if self.ents[i].def == crate::koth64::HILL || crate::diversion61_defs::slot(self.ents[i].def).is_some() { continue; }
                    city |= self.ents[i].def == TC;
                    #[cfg(feature = "ee2-native13")]
                    if self.ents[i].def == DOCK { self.retire_dock_garrison(i); }
                    #[cfg(feature = "ee2-native13")]
                    if self.ents[i].def == TC { self.retire_city_heal(i); }
                    #[cfg(feature = "ee2-native13")]
                    { self.retire_city_name(i); self.forget_capture_actor(i); }
                    self.ents[i].hp = 0.0;
                }
                if city{self.rebuild_territories();self.void_landless_alliances();}
                self.prune_territory_offers();
            }
            #[cfg(feature = "ee2-native13")]
            C_BLESS => {
                if self.powers.is_some() && !self.net {
                    if let Some(target) = self.get(c as u32) { self.order_bless(&ids, target); }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_LEADER_RALLY => {
                if a == 0.0 && b == 0.0 && matches!(c, 0 | 1) {
                    for &index in &ids { self.order_leader_power(index, c == 1); }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_LEADER_DIVERSION => {
                if c == 0 {
                    for &index in &ids { self.order_leader_diversion(index, a, b); }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_LEADER_CONSTRUCTION => {
                if a == 0.0 && b == 0.0 {
                    if let Some(target) = target {
                        for &index in &ids { self.order_leader_construction(index, target); }
                    }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_LEADER_MOBILIZATION => {
                if a == 0.0 && b == 0.0 {
                    for &index in &ids { self.order_leader_mobilization(index, c as u32); }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_LEADER_PURCHASE => {
                if a == 0.0 && b == 0.0 {
                    if let Some(target) = target {
                        for &index in &ids { self.order_leader_purchase(index, target); }
                    }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_IMPERIALISM => {
                if a == 0.0 && b == 0.0 {
                    if let Some(target) = target {
                        for &index in &ids { self.order_imperialism(index, target); }
                    }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_GATHER_INTEL => {
                if !self.net && a >= 0.0 && a.fract() == 0.0 && b == 0.0 {
                    if let Some(operation) = spy_intel46::SpyOperation::read(a as u32) {
                        if let Some(target) = target.filter(|&target| self.spy_operation_target(team, target, operation)) {
                            self.order_spy_operation(&ids, target, operation);
                        }
                    }
                }
            }
            #[cfg(feature = "ee2-native13")]
            C_EXTRACTION => {
                if !self.net && (c == 1 || c == 0 && a == 0.0 && b == 0.0) {
                    for &index in &ids {
                        self.order_spy_extraction(index, (c == 1).then_some((a, b)));
                    }
                }
            }
            _ => {}
        }
        #[cfg(feature = "ee2-native13")]
        self.reconcile_capture_orders();
        #[cfg(feature = "ee2-native13")]
        self.reconcile_bless_casts();
        #[cfg(feature = "ee2-native13")]
        self.reconcile_spy_intel();
        #[cfg(feature = "ee2-native13")]
        self.reconcile_spy_extraction();
        #[cfg(all(feature="ee2-pack8",feature="ee2-civ8"))]
        self.recount_paid_players();
    }
}
