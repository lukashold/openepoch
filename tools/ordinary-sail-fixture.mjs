// A medieval ship is produced through the same selected-citizen/Dock queue API
// used by Main. Serialized state is read only to inspect terrain and completion.
import {ordinaryEntities,ordinaryUI} from './ordinary-classic-fixture.mjs';
import {save} from './roster6-state.mjs';

export function buildOrdinarySailFixture(w,{seed=7,epoch=5,instantBuild=true,maxTicksPerStep=12000}={}){
 const require=(value,label)=>{if(!value)throw Error('Ordinary sail fixture: '+label);},trace=[];
 require([5,7].includes(epoch),'use an original cloth-sail epoch');
 require(w.definition_count()>=59,'typed FightingSail definition');
 w.init(seed,0,0,1,0);require(w.skirmish_epoch(epoch)===1,'epoch');require(w.skirmish_setup(500,2)===1,'resource setting');require(w.skirmish_visibility(2)===1,'revealed map');
 if(instantBuild){require(w.cheat(0)===1,'enable game cheats');require(w.cheat(3)===1,'instant construction cheat');}
 const until=(predicate,label)=>{for(let t=0;t<=maxTicksPerStep;t++){if(predicate())return t;w.tick();}throw Error('Ordinary sail fixture timed out: '+label);};
 w.select_none();require((w.select_idle_worker()>>>0)!==0xffffffff,'selected citizen');
 const worker=ordinaryEntities(w).find(e=>e.selected&&e.team===0&&e.def===8);require(worker,'citizen position');
 const bytes=save(w),d=new DataView(bytes.buffer),at=36+4*d.getUint32(8,true)+d.getUint32(4,true)*d.getUint32(32,true),ter=bytes.subarray(at,at+65536),res=bytes.subarray(at+65536,at+131072),sites=[];
 for(let y=3;y<250;y++)for(let x=3;x<250;x++){
  let water=0,land=0,blocked=false;
  for(let yy=y;yy<y+3;yy++)for(let xx=x;xx<x+3;xx++){const tile=yy*256+xx;if(res[tile]||ter[tile]===5)blocked=true;if(ter[tile]<=1)water++;else land++;}
  if(!blocked&&land>=1&&water>=3)sites.push({x,y,dist:(x+1.5-worker.x)**2+(y+1.5-worker.y)**2});
 }
 sites.sort((a,b)=>a.dist-b.dist||a.y-b.y||a.x-b.x);
 const site=sites.find(p=>w.place_error_rotated(5,p.x,p.y,0)===0);require(site,'legal Dock placement');require(w.cmd_place_rotated(5,site.x,site.y,0)===1,'Dock command');
 const x=site.x+1.5,y=site.y+1.5,dockTicks=until(()=>ordinaryEntities(w).some(e=>e.team===0&&e.def===5&&Math.abs(e.x-x)<.1&&Math.abs(e.y-y)<.1&&e.built>=1),'Dock construction');
 trace.push({action:'place',def:5,x:site.x,y:site.y,turn:0,ticks:dockTicks});
 w.select_point(x,y,0,0,0,0,256,256);require(ordinaryUI(w).sel.def===5,'Dock selected');require(w.cmd_queue_job(58)===1,'FightingSail production command');
 const sailTicks=until(()=>ordinaryEntities(w).some(e=>e.team===0&&e.def===58&&e.built>=1),'FightingSail production');trace.push({action:'train',def:58,ticks:sailTicks});
 return{format:'ordinary-sail-fixture-v1',seed,epoch,instantBuild,gameTick:w.game_tick(),trace,entities:ordinaryEntities(w).filter(e=>e.team===0),source:'Public settings/select/queue/place/tick/cheat exports; serialized bytes are inspected without writing simulation memory.'};
}
