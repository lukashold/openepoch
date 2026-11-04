// Source-only trait projection for the proposed80-definition candidate.
// No running engine imports this module. Events remain typed source records.
use super::civ8_gates::matches_ttciv;
pub struct Factor {pub name:&'static str,pub ttciv:u8,pub field:&'static str,pub value:f32,pub targets:&'static [u16]}
pub const FACTORS:&[Factor]=&[
Factor{name:"RegionalTraitUpgrade_WE_01",ttciv:15,field:"GatherTech",value:10.0_f32,targets:&[47]},
Factor{name:"RegionalTraitUpgrade_WE_02",ttciv:15,field:"CaptureResistance",value:20.0_f32,targets:&[47]},
Factor{name:"RegionalTraitUpgrade_WE_03",ttciv:15,field:"HitPoints",value:20.0_f32,targets:&[47]},
Factor{name:"RegionalTraitUpgrade_ME_01",ttciv:17,field:"TradeRouteDrop",value:10.0_f32,targets:&[]},
Factor{name:"RegionalTraitUpgrade_ME_02",ttciv:17,field:"CaptureResistance",value:20.0_f32,targets:&[50]},
Factor{name:"RegionalTraitUpgrade_ME_03",ttciv:17,field:"HitPoints",value:20.0_f32,targets:&[50]},
Factor{name:"RegionalTraitUpgrade_FE_04",ttciv:16,field:"PopCap",value:25.0_f32,targets:&[0]},
Factor{name:"RegionalTraitUpgrade_FE_02",ttciv:16,field:"CaptureResistance",value:20.0_f32,targets:&[0]},
Factor{name:"RegionalTraitUpgrade_FE_03",ttciv:16,field:"HitPoints",value:20.0_f32,targets:&[0]},
Factor{name:"RegionalTraitUpgrade_AM_01",ttciv:18,field:"GatherTech",value:10.0_f32,targets:&[51]},
Factor{name:"RegionalTraitUpgrade_AM_02",ttciv:18,field:"CaptureResistance",value:20.0_f32,targets:&[51]},
Factor{name:"RegionalTraitUpgrade_AM_03",ttciv:18,field:"HitPoints",value:20.0_f32,targets:&[51]},
Factor{name:"CivUpgrade_German_02",ttciv:1,field:"HitPoints",value:10.0_f32,targets:&[9, 11, 13, 15, 16, 17]},
Factor{name:"CivUpgrade_English_01",ttciv:2,field:"Cost",value:-25.0_f32,targets:&[5, 50]},
Factor{name:"CivUpgrade_English_02",ttciv:2,field:"Damage",value:10.0_f32,targets:&[58]},
Factor{name:"CivUpgrade_English_03",ttciv:2,field:"Damage",value:10.0_f32,targets:&[33, 35]},
Factor{name:"CivUpgrade_English_04",ttciv:2,field:"Damage",value:10.0_f32,targets:&[29]},
Factor{name:"CivUpgrade_Roman_01",ttciv:3,field:"Cost",value:-25.0_f32,targets:&[3, 4, 48]},
Factor{name:"CivUpgrade_Roman_02",ttciv:3,field:"Damage",value:10.0_f32,targets:&[9, 11, 13, 15, 16, 17]},
Factor{name:"CivUpgrade_American_01",ttciv:4,field:"TradeRouteDrop",value:25.0_f32,targets:&[]},
Factor{name:"CivUpgrade_American_02",ttciv:4,field:"Damage",value:10.0_f32,targets:&[37, 38, 39, 40, 44]},
Factor{name:"CivUpgrade_Greek_01",ttciv:5,field:"Cost",value:-25.0_f32,targets:&[47]},
Factor{name:"CivUpgrade_Greek_02",ttciv:5,field:"HitPoints",value:10.0_f32,targets:&[10, 12, 14]},
Factor{name:"CivUpgrade_Egyptian_01",ttciv:9,field:"Cost",value:-25.0_f32,targets:&[]},
Factor{name:"CivUpgrade_Egyptian_02",ttciv:9,field:"HitPoints",value:10.0_f32,targets:&[21, 23, 43, 62, 64, 68, 69, 72]},
Factor{name:"CivUpgrade_Turkish_01",ttciv:10,field:"Cost",value:-25.0_f32,targets:&[]},
Factor{name:"CivUpgrade_Turkish_02",ttciv:10,field:"HitPoints",value:25.0_f32,targets:&[]},
Factor{name:"CivUpgrade_Turkish_03",ttciv:10,field:"Damage",value:10.0_f32,targets:&[20, 24, 65, 74]},
Factor{name:"CivUpgrade_Babylonian_01",ttciv:11,field:"PowerRechargeTime",value:-25.0_f32,targets:&[57, 61]},
Factor{name:"CivUpgrade_Babylonian_02",ttciv:11,field:"Damage",value:10.0_f32,targets:&[10, 12, 14]},
Factor{name:"CivUpgrade_Korean_01",ttciv:7,field:"ConversionResistance",value:25.0_f32,targets:&[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 25, 27, 28, 57, 61, 62, 63, 64, 65, 67, 68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79]},
Factor{name:"CivUpgrade_Korean_02",ttciv:7,field:"Damage",value:10.0_f32,targets:&[18, 19, 26, 41, 46, 63, 66, 67, 70, 71, 73, 75, 79]},
Factor{name:"CivUpgrade_Japanese_01",ttciv:8,field:"Cost",value:-25.0_f32,targets:&[3, 49]},
Factor{name:"CivUpgrade_Japanese_02",ttciv:8,field:"Damage",value:10.0_f32,targets:&[37, 39, 44]},
Factor{name:"CivUpgrade_Chinese_01",ttciv:6,field:"Cost",value:-25.0_f32,targets:&[0]},
Factor{name:"CivUpgrade_Chinese_02",ttciv:6,field:"HitPoints",value:10.0_f32,targets:&[9, 11, 13, 15, 16, 17]},
Factor{name:"CivUpgrade_Inca_01",ttciv:12,field:"GatherGold",value:10.0_f32,targets:&[8]},
Factor{name:"CivUpgrade_Inca_02",ttciv:12,field:"HitPoints",value:10.0_f32,targets:&[10, 12, 14]},
Factor{name:"CivUpgrade_Aztec_01",ttciv:13,field:"Damage",value:20.0_f32,targets:&[22, 25, 27, 28, 76, 77, 78]},
Factor{name:"CivUpgrade_Mayan_01",ttciv:14,field:"PowerRechargeTime",value:-25.0_f32,targets:&[]},
Factor{name:"CivUpgrade_Mayan_02",ttciv:14,field:"HitPoints",value:10.0_f32,targets:&[]},
];
pub fn projected_starting_percent(civ:i8,def:u16,field:&str)->Option<f32> {
 if !(0..14).contains(&civ) { return None; }
 Some(FACTORS.iter().filter(|r| matches_ttciv(r.ttciv,civ) && r.field==field && r.targets.contains(&def)).map(|r|r.value).fold(0.0_f32,|total,value|total+value))
}
