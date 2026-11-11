//! Resource order preserves the three legacy save slots. Native EE2 resource
//! IDs are mapped explicitly; technology points remain a separate account.
use crate::{defs::*,game::*,map::*};
pub const RESOURCE_COUNT: usize=if cfg!(feature="ee2-reference") {9}else{3};
pub const RESOURCE_NAMES:[&str;9]=["Nahrung","Holz","Eisen","Stein","Gold","Zinn","Salpeter","Öl","Uran"];
pub const RESOURCE_KEYS:[&str;9]=["food","wood","iron","stone","gold","tin","saltpeter","oil","uranium"];
pub const NATIVE_IDS:[u8;9]=[0,1,5,2,3,4,6,7,8];
// db/Units/epoch4,7,10,13_upgrades.ddf, referenced by resources.ddf.
pub const REVEAL_EPOCH:[u8;9]=[0,0,3,0,0,0,6,9,12];
// Ghidra 00884710 + 00b5526c resource era pairs. End is exclusive.
pub const END_EPOCH:[u8;9]=[15,15,9,15,15,6,12,15,15];
pub fn unlocked(slot:usize,epoch:u8)->bool {slot<RESOURCE_COUNT&&(!cfg!(feature="ee2-reference")||(epoch>=REVEAL_EPOCH[slot]&&epoch<END_EPOCH[slot]))}
// An obsolete deposit is hidden per viewer. Other players may still use it.
pub fn map_resource_kind(kind: u8, epoch: u8) -> u8 {
    let slot = usize::from(kind.wrapping_sub(1));
    if cfg!(feature = "ee2-reference") && END_EPOCH.get(slot).is_some_and(|&end| epoch >= end) {
        R_NONE
    } else {
        kind
    }
}

pub fn map_resource_unrevealed(kind: u8, epoch: u8) -> bool {
    let slot = usize::from(kind.wrapping_sub(1));
    cfg!(feature = "ee2-reference") && REVEAL_EPOCH.get(slot).is_some_and(|&start| epoch < start)
}
pub fn gather_modifier(kind:u8)->usize {if kind<=3 {7+kind as usize}else{8+kind as usize}}
pub fn population(def:DefId,epoch:u8)->u32{
    #[cfg(feature="ee2-reference")]
    if let Some(v)=crate::ee2_units::lookup_class(def,epoch).and_then(|r|r.pop){return v as u32;}
    DEFS[def as usize].pop as u32
}
pub fn population_capacity(def:DefId,epoch:u8)->u32{
    #[cfg(feature="ee2-reference")]
    if let Some(v)=crate::ee2_units::lookup_class(def,epoch).and_then(|r|r.pop_capacity){return v as u32;}
    match def{TC=>15,HOUSE=>10,_=>0}
}
impl Game {
    /// Admits ordinary completed demolition and credits the current stock quote before retirement.
    #[cfg(feature = "ee2-native13")]
    pub(crate) fn settle_completed_skirmish_demolition(&mut self, index: usize) -> bool {
        let actor = self.ents[index];
        if !matches!(actor.def, HOUSE | BARRACKS | TOWER | TEMPLE | crate::granary211::ID)
            || actor.built != 1.0 || !actor.hp.is_finite() || actor.hp <= 0.0
            || (actor.def == TEMPLE && actor.ncargo != 0)
        {
            return true;
        }
        #[cfg(feature = "ee2-authored-valley37")]
        if self.map.source_world.is_some() {
            return true;
        }
        if self.net || self.winner != 0 || self.territory_victory.is_some() || self.sole_survivor.is_some()
            || self.regicide.is_some() || self.koth.is_some() || self.crown_victory.is_some()
            || self.capitols.is_some() || self.hotspots.is_some()
        {
            return true;
        }
        if self.capture_progress(index).is_some() || self.imperialism_active(index) {
            return false;
        }
        let team = actor.team;
        let epoch = self.players[team as usize].epoch;
        let Some(stock) = crate::ee2_units::lookup_class(actor.def, epoch).and_then(|row| row.cost9) else {
            return true;
        };
        let quote = self.unit_cost(team, actor.def);
        if quote == stock && quote == self.construction_cost(team, actor.def) {
            for (slot, amount) in quote.into_iter().enumerate().filter(|&(_, amount)| amount != 0) {
                let scaled = f32::from(amount) * 0.1_f32;
                let credit = crate::civ8_bonus::cost_integer53(f64::from(scaled));
                self.add_resource(team, slot, credit as f32);
            }
        }
        true
    }

    pub fn native_generation(&self,def:DefId,team:u8,resource:u8)->f32{
        #[cfg(feature="ee2-up15")] {
            let rate=crate::ee2_up15_economy::generation(def,self.players[team as usize].epoch,resource);
            #[cfg(feature = "ee2-rect288-oil-research")]
            let rate = if def == OIL_RIG && resource == crate::map::R_OIL - 1 {
                self.oil_research_rate(team)
            } else { rate };
            #[cfg(feature="ee2-native13")]
            let rate=rate*self.crown_generation_scale(team,def,resource);
            rate
        }
        #[cfg(not(feature="ee2-up15"))] {let _=(def,team,resource);0.0}
    }
    pub fn resource(&self,team:u8,slot:usize)->f32 {
        if slot<3 {self.players[team as usize].res[slot]}else{self.extra_resources[team as usize][slot-3]}
    }
    pub fn add_resource(&mut self,team:u8,slot:usize,amount:f32){
        let value=self.resource(team,slot)+amount;
        let value=if cfg!(feature="ee2-reference"){value.clamp(0.0,999999.0)}else{value};
        self.set_resource(team,slot,value);
    }
    pub(crate) fn set_resource(&mut self,team:u8,slot:usize,value:f32){
        if slot<3 {self.players[team as usize].res[slot]=value;}else{self.extra_resources[team as usize][slot-3]=value;}
    }
    pub fn resource_unlocked(&self,team:u8,kind:u8)->bool {kind>0&&unlocked(kind as usize-1,self.players[team as usize].epoch)}
    pub fn citizen_can_gather(&self,team:u8,kind:u8)->bool {
        self.resource_unlocked(team,kind)&&(!cfg!(feature="ee2-reference")||kind<R_OIL)
    }
    pub fn extractor_kind(def:DefId)->u8 {match def {OIL_RIG=>R_OIL,URANIUM_MINE=>R_URANIUM,_=>R_NONE}}
    fn accepts_gathered_resource(building: &Ent, kind: u8) -> bool {
        let def = building.def;
        #[cfg(feature = "ee2-native13")]
        if def == crate::granary211::ID { return kind == R_FOOD && building.hp > 0.0; }
        DEFS[def as usize].has(F_DROPOFF) || def == DOCK && kind == R_FOOD
    }
    pub fn return_cargo(&mut self,e:&mut Ent,d:&Def,want:&mut(f32,f32),moving:&mut bool)->bool{
        let kind=e.q[2] as u8;
        if kind==R_NONE||!self.resource_unlocked(e.team,kind){e.carry=0.0;e.q[1]=0;e.q[2]=0;return false;}
        let dropoff=self.ents[..self.high].iter().enumerate().filter(|(_,b)|b.alive&&b.built>=1.0&&b.team==e.team&&Self::accepts_gathered_resource(b,kind))
            .min_by(|(_,a),(_,b)|((a.x-e.x).powi(2)+(a.y-e.y).powi(2)).total_cmp(&((b.x-e.x).powi(2)+(b.y-e.y).powi(2)))).map(|(i,_)|i);
        let Some(j)=dropoff else{return false;};let b=self.ents[j];
        if (b.x-e.x).abs().max((b.y-e.y).abs())>Self::building_half(&b)+0.7{
            e.fkey=self.key_for(LAND,b.x,b.y);*want=self.steer(e,d,b.x,b.y);*moving=true;return true;
        }
        // Native Dropoff bonus: CityCenter/Warehouse, 2% per garrisoned citizen.
        let per_slot={
            #[cfg(feature="ee2-up15")] {crate::ee2_up15_economy::lookup(b.def,self.players[b.team as usize].epoch).and_then(|r|r.dropoff_bonus).unwrap_or(0.0)}
            #[cfg(not(feature="ee2-up15"))] {0.02}
        };
        let bonus=if DEFS[b.def as usize].has(F_DROPOFF){1.0+per_slot*(b.ncargo as usize).min(self.cargo_capacity(b.team,b.def)) as f32}else{1.0};
        let delivered = e.carry * bonus;
        self.add_resource(e.team, kind as usize - 1, delivered);
        #[cfg(feature = "ee2-native13")]
        self.crown_gathered(e.team, kind as usize - 1, delivered);
        e.carry=0.0;e.q[1]=0;e.q[2]=0;
        let [x, y] = self.get(e.otgt).map_or_else(
            || self.map.tile_center(e.otile as usize),
            |farm| [self.ents[farm].x, self.ents[farm].y],
        );
        #[cfg(feature = "ee2-authored-rect233")]
        let [x, y] = self.world0_gather_order(e).map_or([x, y], |target| target.point);
        e.fkey=self.key_for(LAND,x,y);
        false
    }

    /// The original Deposit child marks its approach Move as minor type1.
    /// Contact completion and a missing dropoff make no new visual request.
    #[cfg(feature = "ee2-rect289-citizen-work")]
    pub(crate) fn return_cargo_with_work_visual(&mut self,index:usize,e:&mut Ent,d:&Def,
        want:&mut(f32,f32),moving:&mut bool) {
        if self.return_cargo(e,d,want,moving) {
            self.request_citizen_work(index, if e.q[2] == u32::from(R_WOOD) {29} else {30}, 0.0);
        } else {
            self.retain_citizen_work(index);
        }
    }
}

#[cfg(feature="ee2-reference")]
pub const fn apply_original_defs<const COUNT:usize>(mut d:[Def;COUNT])->[Def;COUNT]{
    d[0].flags|=F_DROPOFF;d[52].flags|=F_DROPOFF;
    let mut first=[99u8;COUNT];let mut last=[0u8;COUNT];let mut r=0;
    while r<crate::ee2_units::ROWS.len(){
        let row=&crate::ee2_units::ROWS[r];let id=row.id as usize;
        if row.epoch<first[id]{
            first[id]=row.epoch;
            if let Some(v)=row.hp{d[id].hp=v;}
            if let Some(v)=row.buildtime{d[id].time=v;}
            if let Some(v)=row.sight{d[id].sight=v;}
            if let Some(v)=row.pop{d[id].pop=v;}
            d[id].made_at=match row.source_host.as_bytes(){b"CityCenter"=>TC,b"Barracks"=>BARRACKS,b"Stable"=>STABLE,b"Mill"=>MILL,b"HercFacility"=>HERC_FACILITY,b"Dock"=>DOCK,b"Airport"=>AIRFIELD,_=>d[id].made_at};
        }
        if row.until>last[id]{last[id]=row.until;}
        r+=1;
    }
    let mut id=0;while id<COUNT {if first[id]!=99{d[id].epoch=first[id];if !d[id].building{d[id].until=last[id];}}id+=1;}
    d
}
