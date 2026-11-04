// Generated exact source-bound classic families; no placeholder definitions.
use crate::defs::*;
pub const DEF_COUNT:usize=61;
pub const fn initial(base:[Def;57])->[Def;61]{
let mut d=[base[0];61];let mut i=0;while i<57{d[i]=base[i];i+=1;}
d[57]=Def{name:"Scout",epoch:0,until:15,dom:LAND,building:false,size:0.8_f32,hp:35.0_f32,speed:2.875_f32,sight:7.0_f32,armor:A_INF,cost:[30, 0, 0],time:32.5_f32,pop:1,made_at:TC,flags:0,w:[Wpn{range:0.8_f32,dmg:2.0_f32,cd:2.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[58]=Def{name:"FightingSail",epoch:5,until:10,dom:SEA,building:false,size:3.1_f32,hp:980.0_f32,speed:1.725_f32,sight:10.0_f32,armor:A_SHIP,cost:[0, 187, 248],time:65.0_f32,pop:3,made_at:DOCK,flags:0,w:[Wpn{range:9.0_f32,dmg:143.0_f32,cd:2.5_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[59]=Def{name:"AirDefense",epoch:9,until:15,dom:LAND,building:true,size:2.5_f32,hp:500.0_f32,speed:0.0_f32,sight:11.0_f32,armor:A_BLD,cost:[0, 0, 0],time:25.0_f32,pop:0,made_at:BUILT_BY_WORKER,flags:0,w:[Wpn{range:10.0_f32,dmg:41.0_f32,cd:1.0_f32,kind:0,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d[60]=Def{name:"CoastalDefense",epoch:1,until:15,dom:LAND,building:true,size:1.0_f32,hp:400.0_f32,speed:0.0_f32,sight:8.0_f32,armor:A_BLD,cost:[0, 0, 0],time:40.0_f32,pop:0,made_at:BUILT_BY_WORKER,flags:0,w:[Wpn{range:8.0_f32,dmg:45.0_f32,cd:3.0_f32,kind:3,targets:LAND|SEA|AIR|SUB,splash:0.0_f32},NOW]};
d
}
pub const fn production(mut d:[Def;61])->[Def;61]{
d[57].epoch=0;d[57].until=15;
d[58].epoch=5;d[58].until=10;
d[59].epoch=9;d[59].until=15;
d[60].epoch=1;d[60].until=15;
d
}
