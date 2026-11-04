import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {buildModels} from '../docs/models.js';
import {revealFixture} from './save-fixture.mjs';
const mesh=buildModels();assert.equal(mesh.stride,44);assert.ok(mesh.data.every(Number.isFinite));
const economy=JSON.parse(readFileSync(new URL('../docs/ee2-economy.json',import.meta.url)));
const iron= economy.naturalResources.find(resource=>resource.name==='Iron');
assert.equal(iron.revealEpoch,4);assert.equal(iron.revealEpochZeroBased,3);
for(let id=8;id<=17;id++){
 const bones=new Set(Array.from({length:mesh.count[id]},(_,j)=>mesh.data[(mesh.first[id]+j)*11+10]));
 for(let bone=1;bone<=5;bone++)assert.ok(bones.has(bone),`human ${id} joint ${bone}`);
}
for(let bone=6;bone<=9;bone++)assert.ok(Array.from({length:mesh.count[8]},(_,j)=>mesh.data[(mesh.first[8]+j)*11+10]).includes(bone),`worker tool ${bone}`);
for(const file of ['openepoch.wasm','openepoch-ee2.wasm']){
 const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/'+file,import.meta.url)),{})).instance.exports;
 const reference=file==='openepoch-ee2.wasm';
 const save=()=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
 const load=b=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
 // Native resource/tool animation is tested on a short, explored, passable route,
 // independently of scouting and generated ore placement. Base-profile fixtures
 // and their original assertions remain unchanged.
 function resourceFixture(model){
  const b=save(),d=new DataView(b.buffer),psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),ents=36+4*psz,terrain=ents+high*esz,NN=256*256;
  const resources=terrain+NN,amount=terrain+2*NN,fog=terrain+4*NN;
  assert.equal(d.getUint32(12,true),57);const epoch=model===102?iron.revealEpochZeroBased:0;
  for(let team=0;team<4;team++){b[36+team*psz+24]=epoch;b[36+team*psz+27]=0;}
  revealFixture(b);
  const tx=Math.floor(w.start_x())+9,ty=Math.floor(w.start_y())+3;
  for(let y=ty-5;y<=ty+5;y++)for(let x=tx-5;x<=tx+5;x++){
   const t=y*256+x;b[terrain+t]=3;b[resources+t]=0;d.setUint16(amount+2*t,0,true);
  }
  const tile=ty*256+tx;b[resources+tile]=model===100?2:model===102?3:1;d.setUint16(amount+2*tile,2000,true);
  let worker;
  for(let i=0;i<high;i++){const at=ents+i*esz;if(b[at]&&b[at+1]===8&&b[at+2]===0){worker=at;break;}}
  assert.notEqual(worker,undefined);
  for(const [offset,value]of [[8,tx+.5],[12,ty+4.5],[16,tx+.5],[20,ty+4.5]])d.setFloat32(worker+offset,value,true);
  assert.equal(load(b),1);w.select_none();w.select_point(tx+.5,ty+4.5,0,0,0,0,256,256);
  assert.equal(w.resource_active(0,model===100?1:model===102?2:0),1,'source epoch permits this gathering activity');
  return [tx+.5,ty+.5];
 }
 const frame=(near=false)=>{const x=w.start_x(),y=w.start_y();const n=w.render(.5,...(near?[x-25,y-25,x+25,y+25]:[0,0,256,256]),-1,0,0),p=w.inst_ptr(),b=new Float32Array(w.memory.buffer,p,n*12);return Array.from({length:n},(_,i)=>Array.from(b.slice(i*12,i*12+12)));};
 const worker=()=>{const x=w.sel_center(0),y=w.sel_center(1);return frame(reference).filter(a=>a[4]===8&&a[5]===0).sort((a,b)=>Math.hypot(a[0]-x,a[1]-y)-Math.hypot(b[0]-x,b[1]-y))[0];};
 for(const [model,activity] of [[100,2],[102,3],[103,4]]){
  w.init(42,1,0,1,0);const x=w.start_x(),y=w.start_y();w.select_idle_worker();
  const at=reference?resourceFixture(model):null;
  const target=frame(reference).filter(a=>(a[4]===model||(model===100&&a[4]===101))&&(!at||Math.floor(a[0])===Math.floor(at[0])&&Math.floor(a[1])===Math.floor(at[1]))).sort((a,b)=>Math.hypot(a[0]-x,a[1]-y)-Math.hypot(b[0]-x,b[1]-y))[0];assert.ok(target,`${file} renders the actual gathering target ${model}`);
  w.cmd_context(Math.floor(target[0])+.5,Math.floor(target[1])+.5,0);
  let walking=false,working=false;
  for(let i=0;i<500;i++){w.tick();if(i%5===0){const a=worker();assert.ok(a);walking||=a[11]===1;working||=a[11]===activity;}}
  assert.ok(walking,`${file} walked to resource ${model}`);assert.ok(working,`${file} performed work ${activity}`);
  const hash=w.state_hash();frame();assert.equal(w.state_hash(),hash,'render must not mutate simulation');
 }
 w.init(42,1,0,1,0);w.grant(5000);w.select_idle_worker();const x=w.start_x(),y=w.start_y();let site;
 for(let dx=-7;dx<8&&!site;dx++)for(let dy=4;dy<9&&!site;dy++)if(w.cmd_place(1,x+dx,y+dy))site=[dx,dy];assert.ok(site);
 let building=false,scaffold=false;
 for(let i=0;i<400;i++){w.tick();if(i%5===0){building||=worker()?.[11]===5;scaffold||=frame(true).some(a=>a[4]===111&&a[5]===0);}}
 assert.ok(building,'worker performs build animation');assert.ok(scaffold,'unfinished building shows scaffold');
 for(let i=0;i<1000;i++)w.tick();assert.ok(!frame(true).some(a=>a[4]===111&&a[5]===0),'scaffold disappears when completed');
 console.log('PASS',file,'human articulation, all four tools, walking/gathering/building state and render determinism');
}
