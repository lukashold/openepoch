// Optional render-QA voyage through ordinary game commands. All terrain/entity
// inspection is read-only; no save/state/handle field is changed by the fixture.
import {ordinaryEntities,ordinaryUI} from './ordinary-classic-fixture.mjs';

export function sailVoyage(w,{minimumDockDistance=18,targetDockDistance=34,maxTicks=2400}={}){
 const require=(value,message)=>{if(!value)throw Error('Ordinary sail voyage: '+message);};
 require(minimumDockDistance>=12&&targetDockDistance>=minimumDockDistance+8&&Number.isInteger(maxTicks)&&maxTicks>0,'explicit separated voyage limits');
 const entities=ordinaryEntities(w),ship=entities.find(e=>e.team===0&&e.def===58&&e.built>=1),dock=entities.filter(e=>e.team===0&&e.def===5&&e.built>=1).sort((a,b)=>(a.x-ship?.x)**2+(a.y-ship?.y)**2-((b.x-ship?.x)**2+(b.y-ship?.y)**2))[0];
 require(ship&&dock,'a normally produced ship and completed Dock');
 const length=w.terrain_types_len(),size=Math.sqrt(length);require(Number.isInteger(size),'square current world tile classes');
 const types=new Uint8Array(w.memory.buffer,w.terrain_types_ptr(),length).slice(),water=(x,y)=>x>=0&&y>=0&&x<size&&y<size&&types[y*size+x]<=1;
 // A four-neighbour connected subset of the actual water classes avoids any
 // invented terrain/path geometry. The real C_MOVE pathfinder resolves final
 // passability; the candidate endpoint also has a wholly-water3x3 footprint.
 const start=Math.floor(ship.y)*size+Math.floor(ship.x),queue=new Uint32Array(length),distance=new Int32Array(length).fill(-1);let head=0,tail=0;
 require(water(start%size,Math.floor(start/size)),'ship starts on an actual water tile');distance[start]=0;queue[tail++]=start;
 const candidates=[];
 while(head<tail){const index=queue[head++],x=index%size,y=Math.floor(index/size),cx=x+.5,cy=y+.5,fromDock=Math.hypot(cx-dock.x,cy-dock.y);
  if(fromDock>=targetDockDistance&&fromDock<=targetDockDistance+10&&Math.hypot(cx-ship.x,cy-ship.y)>=targetDockDistance-4){
   let open=true;for(let yy=y-1;yy<=y+1;yy++)for(let xx=x-1;xx<=x+1;xx++)if(!water(xx,yy))open=false;
   if(open&&entities.every(e=>Math.hypot(cx-e.x,cy-e.y)>4))candidates.push({x:cx,y:cy,index,steps:distance[index],dockDistance:fromDock});
  }
  for(const [xx,yy]of[[x-1,y],[x+1,y],[x,y-1],[x,y+1]])if(water(xx,yy)){const next=yy*size+xx;if(distance[next]<0){distance[next]=distance[index]+1;queue[tail++]=next;}}
 }
 require(candidates.length,'a connected open-water destination far from Dock');candidates.sort((a,b)=>a.steps-b.steps||a.dockDistance-b.dockDistance||a.index-b.index);const target=candidates[0];
 w.select_none();w.select_point(ship.x,ship.y,0,0,0,0,size,size);require(ordinaryUI(w).sel.def===58,'the real produced ship is selected');
 const selected=ordinaryEntities(w).filter(e=>e.selected);require(selected.length===1&&selected[0].def===58,'a single real ship selection');
 const tick0=w.game_tick();w.cmd_context(target.x,target.y,0);const trace=[];let final=null,moving=false,ticks=0;
 for(;ticks<maxTicks;ticks++){
  w.tick();if(ticks%4!==0)continue;
  const current=ordinaryEntities(w).find(e=>e.slot===selected[0].slot&&e.def===58&&e.team===0);require(current,'the selected ship remains alive');
  const travelled=Math.hypot(current.x-ship.x,current.y-ship.y),dockDistance=Math.hypot(current.x-dock.x,current.y-dock.y),remaining=Math.hypot(current.x-target.x,current.y-target.y);
  trace.push({tick:w.game_tick(),x:current.x,y:current.y,travelled,dockDistance,remaining});
  if(dockDistance>=minimumDockDistance&&remaining>=6&&travelled>=minimumDockDistance-4){final=current;moving=true;break;}
  if(remaining<4)break;
 }
 require(final&&moving,'ordinary movement reaches a separated position while still underway');
 const dx=dock.x-final.x,dy=dock.y-final.y;
 return {format:'ordinary-sail-voyage-v1',source:'Public select_point/cmd_context(C_MOVE)/tick APIs; read-only terrain/entity inspection. No saved-state or entity mutation.',worldSize:size,ship:{def:58,team:0,initial:{x:ship.x,y:ship.y},final:{x:final.x,y:final.y}},dock:{x:dock.x,y:dock.y},target,minimumDockDistance,tick0,tick1:w.game_tick(),ticks:w.game_tick()-tick0,underway:moving,selection:{def:58,count:selected.length},cameraHeading:Math.atan2(-dy,dx)+Math.PI/4,cameraPolicy:'Own fixture framing: Dock direction lies45degrees from screen-right to expose both hull and broad sail surfaces.',connectedWaterTiles:tail,trace};
}
