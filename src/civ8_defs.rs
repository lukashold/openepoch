// Generated80-definition isolated Civ8 candidate; exact source families.
use crate::defs::*;
pub const fn initial(input:[Def;57])->[Def;80]{
let old=crate::roster6_defs::initial(input);let mut d=[old[0];80];
let mut i=0;while i<61{d[i]=old[i];i+=1;}
d[61]=Def{name:"UniqueUnitAmerican1",epoch:0,until:5,dom:LAND,building:false,size:0.45_f32,hp:55.0_f32,speed:3.0_f32,sight:11.0_f32,armor:A_INF,cost:[45, 0, 0],time:40.0_f32,pop:1,made_at:BARRACKS,flags:0,w:[Wpn{range:0.8_f32,dmg:4.0_f32,cd:2.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[62]=Def{name:"UniqueUnitAmerican3",epoch:10,until:15,dom:LAND,building:false,size:1.8_f32,hp:507.0_f32,speed:2.875_f32,sight:8.0_f32,armor:A_INF,cost:[0, 48, 0],time:55.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:7.0_f32,dmg:38.0_f32,cd:3.0_f32,kind:1,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[63]=Def{name:"UniqueUnitAztec2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:175.0_f32,speed:1.725_f32,sight:5.0_f32,armor:A_INF,cost:[90, 0, 0],time:55.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:1.0_f32,dmg:16.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[64]=Def{name:"UniqueUnitBabylonian2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:100.0_f32,speed:2.875_f32,sight:7.0_f32,armor:A_INF,cost:[0, 81, 0],time:49.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:6.0_f32,dmg:17.0_f32,cd:1.6_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[65]=Def{name:"UniqueUnitChinese2",epoch:5,until:10,dom:LAND,building:false,size:1.0_f32,hp:115.0_f32,speed:1.15_f32,sight:7.0_f32,armor:A_INF,cost:[159, 0, 0],time:58.0_f32,pop:3,made_at:MILL,flags:0,w:[Wpn{range:11.0_f32,dmg:55.0_f32,cd:3.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.5_f32},NOW]};
d[66]=Def{name:"UniqueUnitChinese3",epoch:10,until:15,dom:LAND,building:false,size:1.8_f32,hp:390.0_f32,speed:1.725_f32,sight:9.0_f32,armor:A_INF,cost:[99, 0, 0],time:55.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:12.0_f32,dmg:22.0_f32,cd:1.5_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[67]=Def{name:"UniqueUnitEgyptian1",epoch:0,until:5,dom:LAND,building:false,size:1.2_f32,hp:115.0_f32,speed:2.875_f32,sight:5.0_f32,armor:A_INF,cost:[48, 0, 0],time:55.0_f32,pop:2,made_at:BARRACKS,flags:0,w:[Wpn{range:4.0_f32,dmg:9.0_f32,cd:1.0_f32,kind:1,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[68]=Def{name:"UniqueUnitEgyptian2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:140.0_f32,speed:2.875_f32,sight:8.0_f32,armor:A_INF,cost:[85, 0, 0],time:50.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:7.0_f32,dmg:10.0_f32,cd:1.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[69]=Def{name:"UniqueUnitEgyptian3",epoch:10,until:15,dom:LAND,building:false,size:0.8_f32,hp:297.0_f32,speed:2.875_f32,sight:9.0_f32,armor:A_INF,cost:[0, 49, 0],time:52.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:8.0_f32,dmg:38.0_f32,cd:2.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[70]=Def{name:"UniqueUnitEnglish2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:140.0_f32,speed:3.45_f32,sight:5.0_f32,armor:A_INF,cost:[90, 0, 0],time:40.3_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:1.0_f32,dmg:17.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[71]=Def{name:"UniqueUnitGerman2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:150.0_f32,speed:3.45_f32,sight:5.0_f32,armor:A_INF,cost:[77, 0, 0],time:57.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:1.0_f32,dmg:15.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[72]=Def{name:"UniqueUnitGerman3",epoch:10,until:15,dom:LAND,building:false,size:1.8_f32,hp:760.0_f32,speed:1.725_f32,sight:9.0_f32,armor:A_INF,cost:[142, 0, 0],time:60.0_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:12.0_f32,dmg:31.0_f32,cd:1.5_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[73]=Def{name:"UniqueUnitGreek2",epoch:5,until:10,dom:LAND,building:false,size:0.8_f32,hp:120.0_f32,speed:3.45_f32,sight:5.0_f32,armor:A_INF,cost:[50, 0, 0],time:45.5_f32,pop:2,made_at:STABLE,flags:0,w:[Wpn{range:1.0_f32,dmg:8.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[74]=Def{name:"UniqueUnitInca3",epoch:10,until:15,dom:LAND,building:false,size:1.5_f32,hp:200.0_f32,speed:1.15_f32,sight:9.0_f32,armor:A_INF,cost:[0, 107, 0],time:71.5_f32,pop:3,made_at:MILL,flags:0,w:[Wpn{range:13.0_f32,dmg:120.0_f32,cd:4.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:2.0_f32},NOW]};
d[75]=Def{name:"UniqueUnitKorean1",epoch:0,until:5,dom:LAND,building:false,size:0.8_f32,hp:110.0_f32,speed:3.45_f32,sight:5.0_f32,armor:A_INF,cost:[50, 0, 0],time:58.0_f32,pop:2,made_at:BARRACKS,flags:0,w:[Wpn{range:1.0_f32,dmg:8.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[76]=Def{name:"UniqueUnitKorean2",epoch:5,until:10,dom:LAND,building:false,size:1.4_f32,hp:180.0_f32,speed:1.15_f32,sight:7.0_f32,armor:A_INF,cost:[103, 0, 0],time:52.0_f32,pop:3,made_at:MILL,flags:0,w:[Wpn{range:11.0_f32,dmg:36.0_f32,cd:3.0_f32,kind:1,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[77]=Def{name:"UniqueUnitMayan2",epoch:5,until:10,dom:LAND,building:false,size:1.0_f32,hp:207.0_f32,speed:1.15_f32,sight:7.0_f32,armor:A_INF,cost:[113, 0, 0],time:52.0_f32,pop:3,made_at:MILL,flags:0,w:[Wpn{range:11.0_f32,dmg:30.0_f32,cd:3.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[78]=Def{name:"UniqueUnitMayan3",epoch:10,until:15,dom:LAND,building:false,size:1.0_f32,hp:256.0_f32,speed:1.15_f32,sight:9.0_f32,armor:A_INF,cost:[78, 0, 0],time:55.0_f32,pop:3,made_at:MILL,flags:0,w:[Wpn{range:13.0_f32,dmg:31.0_f32,cd:3.0_f32,kind:5,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[79]=Def{name:"UniqueUnitTurkish1",epoch:0,until:5,dom:LAND,building:false,size:0.8_f32,hp:140.0_f32,speed:3.45_f32,sight:5.0_f32,armor:A_INF,cost:[60, 0, 0],time:50.0_f32,pop:2,made_at:BARRACKS,flags:0,w:[Wpn{range:1.0_f32,dmg:11.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d
}
pub const fn production(mut d:[Def;80])->[Def;80]{
d[57].epoch=0;d[57].until=15;
d[58].epoch=5;d[58].until=10;
d[59].epoch=9;d[59].until=15;
d[60].epoch=1;d[60].until=15;
d[61].epoch=0;d[61].until=5;
d[62].epoch=10;d[62].until=15;
d[63].epoch=5;d[63].until=10;
d[64].epoch=5;d[64].until=10;
d[65].epoch=5;d[65].until=10;
d[66].epoch=10;d[66].until=15;
d[67].epoch=0;d[67].until=5;
d[68].epoch=5;d[68].until=10;
d[69].epoch=10;d[69].until=15;
d[70].epoch=5;d[70].until=10;
d[71].epoch=5;d[71].until=10;
d[72].epoch=10;d[72].until=15;
d[73].epoch=5;d[73].until=10;
d[74].epoch=10;d[74].until=15;
d[75].epoch=0;d[75].until=5;
d[76].epoch=5;d[76].until=10;
d[77].epoch=5;d[77].until=10;
d[78].epoch=10;d[78].until=15;
d[79].epoch=0;d[79].until=5;
d
}
