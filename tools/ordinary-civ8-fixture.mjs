// Only public settings/select/place/train/tick commands. Saves are inspected,
// never edited. This proves production and movement, not unported abilities.
import{save,saveCore}from'./roster6-state.mjs';
import{ordinaryEntities,ordinaryUI}from'./ordinary-classic-fixture.mjs';
export function buildOrdinaryCiv8Unit(w,c,{seed=7,maxTicks=6000}={}){
 const need=(v,m)=>{if(!v)throw new Error('ordinaryCiv8 '+c.sourceFamily+': '+m);};
 w.init(seed,0,0,1,0);need(w.configure_civilizations(0xeee0+(c.TTCiv-1))===1,'resolved civilization');
 need(w.skirmish_epoch(c.startRuntimeEpoch)===1,'native production epoch');
 need(w.skirmish_setup(500,2)===1,'abundant supplies');need(w.skirmish_visibility(2)===1,'revealed map');
 need(w.cheat(0)===1&&w.cheat(3)===1,'authorized instant construction');
 w.select_none();need((w.select_idle_worker()>>>0)!==0xffffffff,'idle citizen');
 const worker=ordinaryEntities(w).find(e=>e.selected&&e.team===0&&e.def===8);need(worker,'selected citizen record');
 const host={Barracks:3,Stable:49,Mill:48}[c.host],side=w.footprint_size(host,0);need(side>0,'typed host footprint');
 const state=saveCore(w),d=new DataView(state.buffer,state.byteOffset,state.byteLength),at=36+4*d.getUint32(8,true)+d.getUint32(4,true)*d.getUint32(32,true),ter=state.subarray(at,at+65536),res=state.subarray(at+65536,at+131072),sites=[];
 for(let y=2;y<254-side;y++)for(let x=2;x<254-side;x++){
  let valid=true;for(let yy=y;yy<y+side&&valid;yy++)for(let xx=x;xx<x+side;xx++){const tile=yy*256+xx;if(ter[tile]<2||ter[tile]>4||res[tile]){valid=false;break;}}
  if(valid)sites.push({x,y,d:(x+side/2-worker.x)**2+(y+side/2-worker.y)**2});
 }
 sites.sort((a,b)=>a.d-b.d||a.y-b.y||a.x-b.x);
 const site=sites.find(p=>w.place_error_rotated(host,p.x,p.y,0)===0);need(site,'legal producer site');
 need(w.cmd_place_rotated(host,site.x,site.y,0)===1,'producer placement');
 function until(test,label){for(let n=0;n<=maxTicks;n++){if(test())return n;w.tick();}throw new Error('ordinaryCiv8 timeout '+label+' '+c.sourceFamily);}
 const buildTicks=until(()=>ordinaryEntities(w).some(e=>e.team===0&&e.def===host&&e.built>=1),'producer');
 const producer=ordinaryEntities(w).find(e=>e.team===0&&e.def===host&&e.built>=1);
 // The normal picker prioritizes a citizen standing by the producer. Click a
 // visible free part of its footprint, as a player can; never force selection.
 let ui;
 for(const dx of[0,-side*0.35,side*0.35]){for(const dy of[0,-side*0.35,side*0.35]){
  w.select_point(producer.x+dx,producer.y+dy,0,0,0,0,256,256);ui=ordinaryUI(w);if(ui.sel?.def===host)break;
 }if(ui?.sel?.def===host)break;}
 need(ui.sel?.def===host,'actual selected producer');
 need(ui.act.some(a=>a.id===c.proposedDefId&&a.ok),'matching native action');
 for(const action of ui.act.filter(a=>a.k==='t'&&a.id>=61))need(w.definition_ttciv(action.id)===c.TTCiv,'foreign civilization action hidden');
 need(w.cheat(3)===2,'normal unit production after instant construction');
 const before=Array.from({length:9},(_,r)=>w.resource_stat(0,r));
 const cost=Array.from({length:9},(_,r)=>w.unit_cost_stat(0,c.proposedDefId,r));
 need(w.cmd_queue_job(c.proposedDefId)===1,'typed source-family production');
 const after=Array.from({length:9},(_,r)=>w.resource_stat(0,r));
 for(let r=0;r<9;r++)need(Math.abs(before[r]-after[r]-cost[r])<0.001,'paid source cost slot'+r);
 w.tick();
 const active=saveCore(w),ad=new DataView(active.buffer,active.byteOffset,active.byteLength),entity=36+4*ad.getUint32(8,true)+ad.getUint32(4,true)*producer.slot;
 const handle=((ad.getUint16(entity+6,true)<<16)|producer.slot)>>>0,duration=w.queue_duration_raw(handle,0);
 need(duration===w.up15_stat(0,c.proposedDefId,1),'captured real source production duration');
 const trainTicks=1+until(()=>ordinaryEntities(w).some(e=>e.team===0&&e.def===c.proposedDefId),'new unit');
 need(duration===0||trainTicks>2,'production uses normal elapsed time');
 const unit=ordinaryEntities(w).find(e=>e.team===0&&e.def===c.proposedDefId);w.select_point(unit.x,unit.y,0,0,0,0,256,256);
 need(ordinaryUI(w).sel.def===c.proposedDefId,'typed selected source unit');
 return{sourceFamily:c.sourceFamily,def:c.proposedDefId,civ:c.TTCiv-1,epoch:c.startRuntimeEpoch,host,buildTicks,trainTicks,duration,cost,producer:{x:producer.x,y:producer.y},unit,state:save(w),scope:'PublicAPI production/cost/selection and captured normal duration; complete pack/turret/conversion abilities are not claimed.'};
}
