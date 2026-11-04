//! Isolated UP8 civilization state and source-projected starting modifiers.
//! Existing saves have no provable civilization and migrate with neutral14.
use crate::{defs::*,game::*};
pub const LEGACY_NEUTRAL:u8=14;
pub const COUNT:u32=14;
pub fn source_ttciv(def:DefId)->u8 {
    crate::civ8_gates::CANDIDATES.iter().find(|r|r.proposed_def==def).map(|r|r.ttciv).unwrap_or(0)
}
pub fn applies(civ:u8,def:DefId)->bool {
    let tt=source_ttciv(def);
    tt==0 || (civ<LEGACY_NEUTRAL && crate::civ8_gates::matches_ttciv(tt,civ as i8))
}

impl Game {
    /// New-game setup only. Random15 must be resolved by the caller. Unused
    /// seats stay neutral; legacy games cannot acquire retroactive traits.
    pub fn configure_civilizations(&mut self,civs:[u8;4])->bool {
        if !self.civ_setup_open || self.tick!=0 || self.civilizations.iter().any(|&c|c!=LEGACY_NEUTRAL) ||
           civs.iter().any(|&c|c>LEGACY_NEUTRAL) ||
           civs[self.nplayers..].iter().any(|&c|c!=LEGACY_NEUTRAL) ||
           self.ents[..self.high].iter().any(|e|e.alive&&DEFS[e.def as usize].building&&e.qn>0) {return false;}
        self.civilizations=civs;
        self.civ_setup_open=false;
        for team in 0..self.nplayers {self.rebuild_modifiers(team as u8,true);}
        self.recount_paid_players();true
    }
    pub fn definition_civ_available(&self,team:u8,def:DefId)->bool {
        self.civilizations.get(team as usize).is_some_and(|&c|applies(c,def))
    }
}

///All current trait methods are AddPermMul; each contribution uses the same
///unmodified source baseline, rather than compounding trait and research buffs.
pub fn apply_modifiers(civ:u8,baseline:&crate::research::Modifiers,m:&mut crate::research::Modifiers) {
    if civ>=LEGACY_NEUTRAL{return;}
    for (field,name) in [(0,"HitPoints"),(1,"Damage"),(4,"Cost"),(7,"GatherTech"),(13,"GatherGold"),(18,"PopCap")] {
        for (id,row) in m.iter_mut().enumerate() {
            let percent=crate::civ8_traits::projected_starting_percent(civ as i8,id as DefId,name).unwrap();
            if percent!=0.0 {row[field]+=baseline[id][field]*percent*0.01;}
        }
    }
}

///German source custom action5 stores separate RESOURCE multipliers:
///native0..3 basic,4..8 special; neither describes a class of units. The
///separate normal+German biased-integer stages are applied in research::unit_cost.
pub fn resource_cost_multiplier(civ:u8,core_slot:usize)->f32 {
    if civ!=0{return 1.0;}
    let (basic,special)=crate::civ8_bonus::cost_modifiers(0.0,-25.0);
    let native=crate::economy::NATIVE_IDS[core_slot];
    if native<4{basic}else{special}
}
