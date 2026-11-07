//! Independent research simulation. No code from the original executable is used.
use crate::defs::*;
use crate::game::*;
#[cfg(feature = "ee2-reference")]
use crate::ee2_research::{NODES, ADVANCE_COST};

pub const DEF_COUNT: usize = if cfg!(feature="ee2-roster6"){crate::defs::DEFS.len()}else if cfg!(feature="ee2-reference") {57} else {53};
pub type Modifiers = [[f32; 19]; DEF_COUNT];

impl Game {
    /// New exact source families store native absolute Damage, avoiding v/base*base.
    /// Legacy role definitions keep their established multiplier representation.
    pub fn effective_weapon_damage(&self,team:u8,def:DefId,weapon_base:f32)->f32 {
        let value=self.modifiers[team as usize][def as usize][1];
        let damage = if cfg!(feature="ee2-roster6")&&def>=57 {value}else{weapon_base*value};
        #[cfg(feature="ee2-ai9-game-bridge")]
        let damage = crate::ai9_rank::damage(self, team, def, damage);
        damage
    }
    pub fn scaled_weapon_damage(&self,team:u8,def:DefId,weapon_base:f32,player_factor:f32)->f32 {
        #[cfg(feature="ee2-ai9-game-bridge")]
        if self.source_unit_rank(team, def) != 0 {
            return self.effective_weapon_damage(team, def, weapon_base) * player_factor;
        }
        let value=self.modifiers[team as usize][def as usize][1];
        if cfg!(feature="ee2-roster6")&&def>=57 {value*player_factor}else{weapon_base*player_factor*value}
    }
    pub fn max_hp(&self, team: u8, def:DefId) -> f32 {
        let hp = self.modifiers[team as usize][def as usize][0];
        #[cfg(feature="ee2-ai9-game-bridge")]
        let hp = crate::ai9_rank::hp(self, team, def, hp, self.source_unit_rank(team, def));
        hp
    }
    pub fn production_time(&self, team: u8, def:DefId) -> f32 { self.modifiers[team as usize][def as usize][3] }
    pub fn unit_cost(&self, team: u8, def:DefId) -> [u16;9] {
        self.cost_with_modifier(team, def, self.modifiers[team as usize][def as usize][4])
    }
    pub(crate) fn construction_cost(&self, team: u8, def: DefId) -> [u16; 9] {
        let modifier = self.modifiers[team as usize][def as usize][4];
        #[cfg(feature = "ee2-native13")]
        let modifier = {
            let solar = self.count_completed_wonders(team, crate::wonders61::SOLAR_POWER_STATION);
            // 00803E24 adds each deviation from 1 and clamps at C62D54.
            (modifier - solar as f32 * 0.5).max(0.05)
        };
        self.cost_with_modifier(team, def, modifier)
    }
    fn cost_with_modifier(&self, team: u8, def: DefId, m: f32) -> [u16; 9] {
        let d = &DEFS[def as usize];
        let mut cost=[0u16;9];cost[..3].copy_from_slice(&d.cost);
        #[cfg(feature="ee2-reference")]
        if let Some(row)=crate::ee2_units::lookup_class(def,self.players[team as usize].epoch){if let Some(value)=row.cost9{cost=value;}}
        #[cfg(feature="ee2-civ8")]
        {core::array::from_fn(|slot|{
            let civ=self.civilizations[team as usize];
            let german=if civ==0{Some(crate::civ8_runtime::resource_cost_multiplier(civ,slot))}else{None};
            crate::civ8_bonus::natural_price53(cost[slot],m,german)
        })}
        #[cfg(not(feature="ee2-civ8"))]
        {cost.map(|v|(v as f32*m).round().clamp(0.0,65535.0) as u16)}
    }
    pub fn cargo_capacity(&self, team: u8, def:DefId) -> usize {
        let internal = DEFS[def as usize].has(F_RESEARCH|F_EXTRACT|F_DROPOFF|F_GARRISON);
        #[cfg(feature = "ee2-native13")]
        let internal = internal || self.powers.is_some() && def == TEMPLE;
        if internal {
            self.modifiers[team as usize][def as usize][11].round().clamp(0.0, CARGO_MAX as f32) as usize
        } else { CARGO_MAX }
    }
    /// Native 007e63b0 uses one place for Leaders, otherwise UnitType+A0.
    /// 0053a0f0 binds table C14970 ('popCount') through C14974, and 0053acfe
    /// writes that inherited property to +A0. Eligibility comes from Garrisonable.
    pub fn fortress_slot_cost(&self,team:u8,def:DefId)->Option<usize> {
        #[cfg(feature="ee2-native13")]
        if def == crate::ee2_regicide::KING {
            return Some(1);
        }
        #[cfg(feature="ee2-reference")]
        {
            let row=crate::ee2_units::lookup_class(def,self.players[team as usize].epoch)
                .or_else(||crate::ee2_units::ROWS.iter().find(|row|row.id==def))?;
            if !row.fortress_garrison {return None;}
            row.garrison_size.filter(|&v|v>0).map(|v|v as usize)
        }
        #[cfg(not(feature="ee2-reference"))]
        {let _=(team,def);None}
    }
    pub fn cargo_slots_used(&self,e:&Ent)->usize {
        if e.def!=FORTRESS {return e.ncargo as usize;}
        e.cargo[..e.ncargo as usize].iter().map(|&def|self.fortress_slot_cost(e.team,def).unwrap_or(CARGO_MAX)).sum()
    }
    pub fn fortress_attack_bonus(&self,e:&Ent)->f32 {
        #[cfg(feature="ee2-reference")]
        if e.def==FORTRESS {
            let per_slot=crate::ee2_units::lookup_class(FORTRESS,self.players[e.team as usize].epoch).and_then(|r|r.garrison_attack_bonus).unwrap_or(0.0);
            return per_slot*self.cargo_slots_used(e).min(self.cargo_capacity(e.team,e.def)) as f32;
        }
        let _=e;0.0
    }
    pub fn fortress_heal_rate(&self,team:u8)->f32 {
        #[cfg(feature="ee2-reference")]
        {
            let rate = crate::ee2_units::lookup_class(FORTRESS,self.players[team as usize].epoch)
                .and_then(|r|r.garrison_heal).unwrap_or(0.0);
            #[cfg(feature="ee2-native13")]
            let rate = rate * self.crown_healing_building_scale(team, FORTRESS);
            rate
        }
        #[cfg(not(feature="ee2-reference"))]
        {let _=team;0.0}
    }
    pub fn research_count(&self, team: u8, epoch: u8) -> u32 {
        #[cfg(feature = "ee2-reference")]
        { NODES.iter().enumerate().filter(|(k,n)| n.epoch == epoch && self.has_research(team,*k)).count() as u32 }
        #[cfg(not(feature = "ee2-reference"))]
        { let _ = (team,epoch); 0 }
    }
    pub fn has_research(&self, team: u8, k: usize) -> bool {
        k < 180 && self.researched[team as usize][k/32] >> (k%32) & 1 == 1
    }
    #[cfg(feature = "ee2-reference")]
    pub fn research_busy(&self, team: u8, k: usize) -> bool {
        #[cfg(feature = "ee2-ai9-game-bridge")]
        if self.source_research_busy(team, k) { return true; }
        self.has_ordinary_research_job(team, k)
    }
    #[cfg(feature = "ee2-reference")]
    pub(crate) fn has_ordinary_research_job(&self, team: u8, k: usize) -> bool {
        self.ents[..self.high].iter().any(|e| e.alive && e.team == team && e.q[..e.qn as usize].contains(&(ORIGINAL_RESEARCH0+k as QueueItem)))
    }
    pub(crate) fn match_final_epoch(&self) -> u8 {
        #[cfg(feature = "ee2-epoch-range13")]
        { self.final_epoch }
        #[cfg(not(feature = "ee2-epoch-range13"))]
        { MAX_EPOCH }
    }
    pub fn can_advance(&self, team: u8) -> bool {
        let p=&self.players[team as usize];
        if p.epoch >= self.match_final_epoch() || p.researching { return false; }
        #[cfg(feature = "ee2-reference")]
        { self.research_count(team,p.epoch) >= 6 && self.research_points[team as usize]+0.00001 >= ADVANCE_COST[p.epoch as usize+1] }
        #[cfg(not(feature = "ee2-reference"))]
        { self.afford(team,&EPOCH_COST[p.epoch as usize+1]) }
    }
    pub fn advance_payment(&mut self, team: u8, sign: f32) {
        let ep=self.players[team as usize].epoch as usize;
        #[cfg(feature = "ee2-reference")]
        { self.research_points[team as usize] -= ADVANCE_COST[ep+1]*sign; }
        #[cfg(not(feature = "ee2-reference"))]
        { self.pay(team,&EPOCH_COST[ep+1],sign); }
    }
    #[cfg(feature = "ee2-reference")]
    pub fn can_original_research(&self, b: usize, item:QueueItem) -> bool {
        let e=&self.ents[b]; let k=(item-ORIGINAL_RESEARCH0) as usize; let n=&NODES[k];
        n.host == e.def && n.epoch <= self.players[e.team as usize].epoch &&
        !self.has_research(e.team,k) && !self.research_busy(e.team,k) &&
        self.research_points[e.team as usize]+0.00001 >= n.cost
    }
    pub fn rebuild_modifiers(&mut self, team: u8, adjust_health: bool) {
        let old=self.modifiers[team as usize];
        let mut m=[[1.0;19];DEF_COUNT];
        for (i,d) in DEFS.iter().enumerate() {
            m[i][..12].copy_from_slice(&[d.hp,1.0,d.speed,d.time,1.0,d.sight,0.0,1.0,1.0,1.0,1.0,if d.has(F_EXTRACT){6.0}else{UNIVERSITY_SLOTS as f32}]);
            if cfg!(feature="ee2-roster6")&&i>=57 {m[i][1]=d.w[0].dmg;}
            m[i][18]=crate::economy::population_capacity(i as DefId,self.players[team as usize].epoch) as f32;
            #[cfg(feature="ee2-reference")]
            if let Some(row)=crate::ee2_units::lookup_class(i as DefId,self.players[team as usize].epoch){
                if let Some(v)=row.hp{m[i][0]=v;}if let Some(v)=row.buildtime{m[i][3]=v;}
                if let Some(v)=row.sight{m[i][5]=v;}if let Some(v)=row.garrison_slots{m[i][11]=v as f32;}
                if let Some(v)=row.damage{if cfg!(feature="ee2-roster6")&&i>=57 {m[i][1]=v;}else if d.w[0].dmg>0.0{m[i][1]=v/d.w[0].dmg;}}
                if cfg!(feature="ee2-up15") {if let Some(v)=row.raw_speed {m[i][2]=v;}}
                if cfg!(feature="ee2-up15")||i==FORTRESS as usize {if let Some(v)=row.range {m[i][6]=v-d.w[0].range;}}
            }
            #[cfg(feature="ee2-native13")]
            if self.air_takeoff_enabled {
                if let Some(flight) = crate::air31_defs::flight_profile(i as DefId, self.players[team as usize].epoch) {
                    m[i][2] = flight.speed;
                }
            }
            #[cfg(feature="ee2-native13")]
            if i == crate::ee2_regicide::KING as usize {
                crate::ee2_regicide::king_modifiers(self.players[team as usize].epoch, &mut m[i]);
            }
        }
        let baseline=m;
        #[cfg(feature = "ee2-reference")]
        for (k,n) in NODES.iter().enumerate() {
            if !self.has_research(team,k) { continue; }
            for ef in n.effects {
                for (i,row) in m.iter_mut().enumerate() {
                    if !crate::ids::mask_has(ef.mask,i as DefId) { continue; }
                    let f=ef.field as usize; let d=&DEFS[i];
                    let base=if f==6 {d.w[0].range+baseline[i][6]}else{baseline[i][f]};
                    match ef.method {0=>row[f]+=base*ef.value*0.01,1=>row[f]+=ef.value,_=>row[f]=ef.value}
                }
            }
        }
        #[cfg(feature="ee2-civ8")]
        crate::civ8_runtime::apply_modifiers(self.civilizations[team as usize],&baseline,&mut m);
        #[cfg(feature="ee2-native13")]
        self.apply_crown_modifiers(team, &mut m);
        for row in &mut m {
            for f in [0,1,2,3,4,5,7,8,9,10,12,13,14,15,16,17] { row[f]=row[f].max(if f==2||f==1||(cfg!(feature="ee2-up15")&&f==5) {0.0} else {0.05}); }
        }
        if adjust_health {
            #[cfg(feature="ee2-ai9-game-bridge")]
            let ranked_hp: Option<[Option<(f32, f32)>; DEF_COUNT]> = {
                self.native_research.as_ref().filter(|state| state.owner == team).map(|_| std::array::from_fn(|id| {
                    let def = id as DefId;
                    let rank = self.source_unit_rank(team, def);
                    (rank != 0).then(|| (
                        crate::ai9_rank::hp(self, team, def, old[id][0], rank),
                        crate::ai9_rank::hp(self, team, def, m[id][0], rank),
                    ))
                }))
            };
            for e in &mut self.ents[..self.high] {
                if e.alive && e.team == team {
                    let id = e.def as usize;
                    #[cfg(feature="ee2-ai9-game-bridge")]
                    if let Some((before, after)) = ranked_hp.as_ref().and_then(|ranked| ranked[id]) {
                        e.hp = (e.hp / before.max(0.01)) * after;
                        continue;
                    }
                    e.hp *= m[id][0] / old[id][0].max(0.01);
                }
            }
        }
        self.modifiers[team as usize]=m;
        #[cfg(feature="ee2-reference")]
        self.sync_city_ground();
    }
}
