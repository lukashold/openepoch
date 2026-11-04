// Isolated generated candidate; no active game module imports this file.
pub const REGION_BY_CIV:[u8;14]=[0,0,0,0,0,1,1,1,2,2,2,3,3,3];
pub fn matches_ttciv(tt:u8,civ:i8)->bool {
 if tt==0 || civ == -1 { return true; }
 if !(0..14).contains(&civ) || tt>18 { return false; }
 if tt<15 { civ==tt as i8-1 } else { REGION_BY_CIV[civ as usize]==tt-15 }
}
#[derive(Clone,Copy,Debug)] pub struct Production {pub proposed_def:u16,pub family:&'static str,pub start:u8,pub end:u8,pub ttciv:u8,pub host:&'static str}
pub const CANDIDATES:&[Production]=&[
Production{proposed_def:61,family:"UniqueUnitAmerican1",start:0,end:5,ttciv:4,host:"Barracks"},
Production{proposed_def:62,family:"UniqueUnitAmerican3",start:10,end:15,ttciv:4,host:"Stable"},
Production{proposed_def:63,family:"UniqueUnitAztec2",start:5,end:10,ttciv:13,host:"Stable"},
Production{proposed_def:64,family:"UniqueUnitBabylonian2",start:5,end:10,ttciv:11,host:"Stable"},
Production{proposed_def:65,family:"UniqueUnitChinese2",start:5,end:10,ttciv:6,host:"Mill"},
Production{proposed_def:66,family:"UniqueUnitChinese3",start:10,end:15,ttciv:6,host:"Stable"},
Production{proposed_def:67,family:"UniqueUnitEgyptian1",start:0,end:5,ttciv:9,host:"Barracks"},
Production{proposed_def:68,family:"UniqueUnitEgyptian2",start:5,end:10,ttciv:9,host:"Stable"},
Production{proposed_def:69,family:"UniqueUnitEgyptian3",start:10,end:15,ttciv:9,host:"Stable"},
Production{proposed_def:70,family:"UniqueUnitEnglish2",start:5,end:10,ttciv:2,host:"Stable"},
Production{proposed_def:71,family:"UniqueUnitGerman2",start:5,end:10,ttciv:1,host:"Stable"},
Production{proposed_def:72,family:"UniqueUnitGerman3",start:10,end:15,ttciv:1,host:"Stable"},
Production{proposed_def:73,family:"UniqueUnitGreek2",start:5,end:10,ttciv:5,host:"Stable"},
Production{proposed_def:74,family:"UniqueUnitInca3",start:10,end:15,ttciv:12,host:"Mill"},
Production{proposed_def:75,family:"UniqueUnitKorean1",start:0,end:5,ttciv:7,host:"Barracks"},
Production{proposed_def:76,family:"UniqueUnitKorean2",start:5,end:10,ttciv:7,host:"Mill"},
Production{proposed_def:77,family:"UniqueUnitMayan2",start:5,end:10,ttciv:14,host:"Mill"},
Production{proposed_def:78,family:"UniqueUnitMayan3",start:10,end:15,ttciv:14,host:"Mill"},
Production{proposed_def:79,family:"UniqueUnitTurkish1",start:0,end:5,ttciv:10,host:"Barracks"},
];
pub fn available(p:&Production,civ:i8,epoch:u8)->bool {
 epoch>=p.start && epoch<p.end && matches_ttciv(p.ttciv,civ)
}
