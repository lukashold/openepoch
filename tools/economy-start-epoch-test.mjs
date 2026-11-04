// Skirmish start epoch is a setup operation with symmetric native stats.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url)),{})).instance.exports;
const units=JSON.parse(readFileSync(new URL('../docs/ee2-units.json',import.meta.url),'utf8'));
assert.equal(typeof w.skirmish_epoch,'function','build must export skirmish_epoch');
const maximum=(id,epoch)=>units.rows.filter(r=>r.id===id&&r.epoch<=epoch).sort((a,b)=>a.epoch-b.epoch).at(-1).fields.hp.value;
function health(){const p=w.save(),b=new Uint8Array(w.memory.buffer,p,w.save_len()).slice(),d=new DataView(b.buffer),es=d.getUint32(4,true),ps=d.getUint32(8,true),high=d.getUint32(32,true),out=[];for(let i=0;i<high;i++){const a=36+4*ps+i*es;if(b[a])out.push({def:b[a+1],team:b[a+2],hp:d.getFloat32(a+40,true)});}return out;}
for(let epoch=0;epoch<15;epoch++){
 w.init(7,3,0,1,0);assert.equal(w.skirmish_epoch(epoch),1,`start in original epoch ${epoch+1}`);
 const balance=Array.from({length:9},(_,s)=>w.resource_stat(0,s));
 for(let team=0;team<4;team++){
  assert.equal(w.stat(team,0),epoch,'all opponents receive the same epoch');
  assert.deepEqual(Array.from({length:9},(_,s)=>w.resource_stat(team,s)),balance,'starting resources remain symmetric');
  for(const id of [0,8])assert.equal(w.research_stat(team,3,id),maximum(id,epoch),'native maximum health at selected epoch');
 }
 for(const e of health())assert.ok(Math.abs(e.hp-maximum(e.def,epoch))<.005,`initial entity ${e.def} stays fully healthy after epoch setup`);
 const hash=w.state_hash();assert.equal(w.skirmish_epoch(15),0,'epoch outside the native15 is rejected');assert.equal(w.state_hash(),hash);
 w.tick();const afterTick=w.state_hash();assert.equal(w.skirmish_epoch(0),0,'cannot change epochs after simulation starts');assert.equal(w.state_hash(),afterTick);
}
w.init(7,0,0,2,0);let hash=w.state_hash();assert.equal(w.skirmish_epoch(4),0,'multiplayer epoch mutation is denied');assert.equal(w.state_hash(),hash);
w.init(7,0,10,1,0);hash=w.state_hash();assert.equal(w.skirmish_epoch(4),0,'benchmark epoch mutation is denied');assert.equal(w.state_hash(),hash);
console.log('PASS skirmish start epoch: all15 native epochs, four-player symmetry, native full-health initialization, invalid/after-start/network/benchmark mutation guards.');
