// Controlled long economic runs. Attack waves are postponed in saved scenarios,
// so the test observes resource progression instead of ending in an early battle.
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {refreshFixtureCityGround} from './save-fixture.mjs';
const wasm=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const source=JSON.parse(readFileSync(new URL('../docs/ee2-economy.json',import.meta.url),'utf8'));
const standard=source.startingResourceLevels.find(r=>r.NAME==='Standard_1');
const keys=['FOOD','WOOD','IRON','STONE','GOLD','TIN','SALTPETER','OIL','URANIUM'];
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
function layout(b){const d=new DataView(b.buffer),ps=d.getUint32(8,true),es=d.getUint32(4,true),high=d.getUint32(32,true),entities=36+4*ps,end=entities+high*es+256*256*5;return{d,ps,es,high,entities,end,eco:end+132};}
function census(w,minutes){
 const b=save(w),l=layout(b),teams=Array.from({length:4},(_,team)=>({team,minutes,epoch:w.stat(team,0)+1,population:w.stat(team,1),capacity:w.stat(team,2),resources:keys.map((_,slot)=>w.resource_stat(team,slot)),researchPoints:w.research_stat(team,0,0),workers:0,workerOrders:{},carrying:0,army:0,garrisoned:0,buildings:{},unfinished:0,queued:0,maxQueueSeconds:0}));
 for(let i=0;i<l.high;i++){
  const at=l.entities+i*l.es;if(!b[at])continue;const team=teams[b[at+2]],id=b[at+1],hp=l.d.getFloat32(at+40,true),built=l.d.getFloat32(at+80,true);assert.ok(Number.isFinite(hp)&&hp>0,'living entity has finite positive health');assert.ok(Number.isFinite(built)&&built>=0&&built<=1,'finite construction progress');
  if(id===8){team.workers++;const order=b[at+48];team.workerOrders[order]=(team.workerOrders[order]||0)+1;if(l.d.getFloat32(at+116,true)>.01)team.carrying++;}
  else if(id>=9&&id<=46)team.army++;
  else{team.buildings[id]=(team.buildings[id]||0)+1;if(built<1)team.unfinished++;team.garrisoned+=b[at+145];if(b[at+89]){team.queued+=b[at+89];team.maxQueueSeconds=Math.max(team.maxQueueSeconds,l.d.getFloat32(at+92,true));}}
 }
 for(const t of teams)for(const value of t.resources)assert.ok(Number.isFinite(value)&&value>=0&&value<=999999,'all nine AI banks remain bounded and finite');
 return {minutes,stateHash:w.state_hash()>>>0,teams};
}
const report={format:'openepoch-ee2-economic-soak-v1',wasmSha256:createHash('sha256').update(wasm).digest('hex'),scope:'Headless simulation only. Four AI controllers; combat-wave deadlines postponed in a save fixture. Native Standard starting stocks. No engine edits, cheats, speed multiplier or rendered-FPS claim.',cases:[],warnings:[],observations:[],progressionFailures:[]};
for(const [seed,epoch,minutes]of [[7,0,20],[41,9,10]]){
 const w=(await WebAssembly.instantiate(wasm,{})).instance.exports;w.init(seed,3,0,1,0);const b=save(w),l=layout(b);
 for(let team=0;team<4;team++){
  const p=36+team*l.ps;b[p+24]=epoch;b[p+27]=1;l.d.setUint32(p+28,0x7fffffff,true);
  for(let slot=0;slot<9;slot++)l.d.setFloat32(slot<3?p+4*slot:l.eco+8+(team*6+slot-3)*4,standard[keys[slot]],true);
 }
 assert.equal(load(w,refreshFixtureCityGround(b)),1);const result={seed,startingEpoch:epoch+1,minutes,snapshots:[census(w,0)]},began=performance.now();
 for(let minute=1;minute<=minutes;minute++){
  for(let tick=0;tick<1200;tick++)w.tick();result.snapshots.push(census(w,minute));
  if(minute%5===0)console.log(`AI economy seed${seed} epoch${epoch+1}: ${minute}/${minutes} simulated minutes`);
 }
 result.wallSeconds=(performance.now()-began)/1000;result.meanSimulationMillisecondsPerTick=result.wallSeconds*1000/(minutes*1200);
 for(let team=0;team<4;team++){
  let frozen=0;
  for(let i=1;i<result.snapshots.length;i++){
   const a=result.snapshots[i-1].teams[team],c=result.snapshots[i].teams[team];
   const sig=t=>JSON.stringify([t.epoch,t.population,t.army,t.garrisoned,t.buildings,t.resources.map(v=>Math.round(v*10)),Math.round(t.researchPoints*10)]);
   frozen=sig(a)===sig(c)&&c.workers>0?frozen+1:0;
   if(frozen===3)report.warnings.push({seed,startingEpoch:epoch+1,team,minute:i,kind:'possible_economic_stall',detail:c});
  }
  const final=result.snapshots.at(-1).teams[team];
  if(minutes>=10&&!final.buildings[47]&&final.researchPoints===0){
   report.observations.push({seed,startingEpoch:epoch+1,team,minute:minutes,kind:'no_research_infrastructure',severity:'observation_not_asserted_failure',wood:final.resources[1],stone:final.resources[3],oil:final.resources[7],note:'No university or technology points despite continued unit/economy growth. Native university requires150wood+150stone; inspect build and resource priorities.'});
  }
  if(epoch===0&&minutes>=20){
   if(final.army===0)report.progressionFailures.push({seed,team,kind:'no_army_after_twenty_minutes',workers:final.workers,resources:final.resources});
   if(!final.buildings[47])report.progressionFailures.push({seed,team,kind:'no_university_after_twenty_minutes',workers:final.workers,resources:final.resources});
  }
 }
 report.cases.push(result);
}
const destination=new URL('../../EE2-Research/analysis/economy-ai-soak.json',import.meta.url);writeFileSync(destination,JSON.stringify(report,null,2)+'\n');
assert.equal(report.progressionFailures.length,0,'AI must establish an army and university in the controlled20-minute starting-epoch scenario: '+JSON.stringify(report.progressionFailures));
console.log('PASS AI economic soak: bounded/finite stocks and state over20+10 simulated minutes; observations written to',destination.pathname,'warnings',report.warnings.length);
for(const test of report.cases)console.log(JSON.stringify({seed:test.seed,startingEpoch:test.startingEpoch,minutes:test.minutes,wallSeconds:+test.wallSeconds.toFixed(3),meanTickMs:+test.meanSimulationMillisecondsPerTick.toFixed(4),final:test.snapshots.at(-1).teams}));
