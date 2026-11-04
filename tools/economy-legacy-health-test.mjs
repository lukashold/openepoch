// Migrate saves produced by the genuine preceding 53-definition WASM build.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const instantiate=async path=>(await WebAssembly.instantiate(readFileSync(path),{})).instance.exports;
const old=await instantiate(process.env.OLD_ASSET_WASM||new URL('./fixtures/reference-53.wasm',import.meta.url));
const current=await instantiate(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
function entities(b){
 const d=new DataView(b.buffer),size=d.getUint32(4,true),players=d.getUint32(8,true),count=d.getUint32(32,true),out=[];
 for(let i=0;i<count;i++){const at=36+4*players+i*size;if(b[at])out.push({i,at,def:b[at+1],team:b[at+2],hp:d.getFloat32(at+40,true)});}
 return out;
}
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
const close=(a,b,label)=>assert.ok(Math.abs(a-b)<.005,`${label}: ${a}, expected ${b}`);
for(const research of [false,true]){
 old.init(7,0,0,1,0);
 let initial=save(old),d=new DataView(initial.buffer),ps=d.getUint32(8,true);assert.equal(d.getUint32(12,true),53,'genuine old asset build');
 for(let t=0;t<4;t++)initial[36+t*ps+27]=0;assert.equal(load(old,initial),1);
 assert.equal(old.research_stat(0,3,8),40,'old worker health basis');assert.equal(old.research_stat(0,3,0),2400,'old town-center health basis');
 if(research){old.cheat(0);old.cheat(4);assert.equal(old.cmd_research(4),1);run(old,305);assert.equal(old.research_stat(0,2,4),1);assert.equal(old.research_stat(0,3,8),48,'old researched worker maximum');}
 let b=save(old);d=new DataView(b.buffer);const before=entities(b),workers=before.filter(e=>e.team===0&&e.def===8),towns=before.filter(e=>e.def===0),maximum=research?48:40;
 assert.ok(workers.length>=3&&towns.length===2);
 for(const e of workers)d.setFloat32(e.at+40,maximum,true);
 d.setFloat32(workers[1].at+40,maximum*.5,true);d.setFloat32(workers[2].at+40,maximum*.25,true);
 d.setFloat32(towns[0].at+40,2400,true);d.setFloat32(towns[1].at+40,600,true);
 // Stale fractional counters from the old immediate-credit system are not new cargo.
 d.setFloat32(workers[0].at+116,.75,true);
 const balances=[3,4,5].map(f=>old.stat(0,f));assert.equal(load(current,b),1);
 const after=entities(save(current)),workerMax=research?36:30;
 close(current.research_stat(0,3,8),workerMax,'new worker maximum with preserved research');
 for(const [k,fraction]of [[0,1],[1,.5],[2,.25]])close(after.find(e=>e.i===workers[k].i).hp,workerMax*fraction,`worker preserves health fraction ${fraction}`);
 close(after.find(e=>e.i===towns[0].i).hp,8000,'full old town center becomes full native town center');
 close(after.find(e=>e.i===towns[1].i).hp,2000,'wounded town center retains25 percent health');
 assert.deepEqual([0,1,2].map(s=>current.resource_stat(0,s)),balances,'legacy bank values survive health migration');
 assert.equal(current.research_stat(0,2,4),Number(research),'completed technology bit survives migration');
 close(current.research_stat(0,0,0),old.research_stat(0,0,0),'technology point bank survives migration');
 const saved=save(current),view=new DataView(saved.buffer),entry=entities(saved).find(e=>e.i===workers[0].i);assert.equal(view.getFloat32(entry.at+116,true),0,'old credit remainder does not become carried goods');
 const hash=current.state_hash();assert.equal(load(current,saved),1);assert.equal(current.state_hash(),hash,'migration is applied once, then native saves roundtrip');
}
console.log('PASS genuine legacy53 health migration: workers40→30 and researched48→36, full/half/quarter health retained, town centers2400→8000, banks/research preserved, obsolete carry remainder cleared, native roundtrip stable.');
