// Original DDF worker caps, tested through real gather orders and saved carry state.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {revealFixture} from './save-fixture.mjs';
const bytes=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const w=(await WebAssembly.instantiate(bytes,{})).instance.exports;
const N=256,NN=N*N;
const save=()=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=b=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const run=n=>{for(let i=0;i<n;i++)w.tick();};
function layout(b){const d=new DataView(b.buffer),ps=d.getUint32(8,true),es=d.getUint32(4,true),high=d.getUint32(32,true),ents=36+4*ps,ter=ents+es*high;return {d,ps,es,high,ents,ter,res:ter+NN,amt:ter+2*NN,fog:ter+4*NN};}
function entities(b){const l=layout(b),result=[];for(let i=0;i<l.high;i++){const a=l.ents+i*l.es;if(b[a]&&b[a+2]===0)result.push({a,i,def:b[a+1],x:l.d.getFloat32(a+8,true),y:l.d.getFloat32(a+12,true),carry:l.d.getFloat32(a+116,true)});}return result;}
const pick=(x,y,same=0)=>{w.select_none();w.select_point(x,y,0,same,0,0,N,N);};
for(const [kind,slot,epoch,workerCount,cap,label]of [[2,1,0,5,3,'wood/tree'],[1,0,0,7,6,'food/forage patch'],[3,2,3,7,6,'iron deposit']]){
 w.init(7,0,0,1,0);w.grant(1000);
 if(workerCount>5){w.cheat(0);w.cheat(3);pick(w.start_x(),w.start_y());for(let k=5;k<workerCount;k++)assert.equal(w.cmd_train(8),1);run(5);}
 const b=save(),l=layout(b),all=entities(b),tc=all.find(e=>e.def===0),workers=all.filter(e=>e.def===8);
 assert.equal(workers.length,workerCount);
 for(let t=0;t<4;t++){b[36+t*l.ps+24]=epoch;b[36+t*l.ps+27]=0;}
 revealFixture(b);
 const x=Math.floor(tc.x)+9,y=Math.floor(tc.y)+3,tile=y*N+x;
 for(let ty=y-4;ty<=y+4;ty++)for(let tx=x-4;tx<=x+4;tx++){const q=ty*N+tx;b[l.ter+q]=3;b[l.res+q]=0;l.d.setUint16(l.amt+2*q,0,true);}
 b[l.res+tile]=kind;l.d.setUint16(l.amt+2*tile,2000,true);
 for(const [k,e]of workers.entries()){
  const angle=k*2*Math.PI/workerCount,px=x+.5+1.2*Math.cos(angle),py=y+.5+1.2*Math.sin(angle);
  for(const [o,v]of [[8,px],[12,py],[16,px],[20,py]])l.d.setFloat32(e.a+o,v,true);
 }
 assert.equal(load(b),1);const first=entities(save()).find(e=>e.def===8);pick(first.x,first.y,1);
 const before=w.resource_stat(0,slot);w.cmd_context(x+.5,y+.5,0);run(100);
 const after=entities(save()).filter(e=>e.def===8),gathering=after.filter(e=>e.carry>1),total=after.reduce((sum,e)=>sum+e.carry,0);
 assert.equal(gathering.length,cap,`${label}: exact native concurrent worker cap`);
 assert.ok(Math.abs(total-cap*.85*5)<.08,`${label}: ${total} carried, expected ${cap*.85*5}`);
 assert.equal(w.resource_stat(0,slot),before,`${label}: harvested material is still carried, not remotely credited`);
}
// Farm is a distinct native ResourceFarm, maxNumOfWorkers=1, farmRate=4.37.
w.init(7,0,0,1,0);w.grant(1000);w.cheat(0);w.cheat(3);
let b=save(),l=layout(b);for(let t=0;t<4;t++){b[36+t*l.ps+24]=1;b[36+t*l.ps+27]=0;}revealFixture(b);assert.equal(load(b),1);
assert.notEqual(w.select_idle_worker(),0xffffffff,'Select an actual citizen before farm placement preview');
const sx=Math.floor(w.start_x()),sy=Math.floor(w.start_y());let site;
for(let r=4;r<30&&!site;r++)for(let q=-r;q<=r&&!site;q++)for(const p of [[sx+q,sy-r],[sx+q,sy+r],[sx-r,sy+q],[sx+r,sy+q]])if(w.place_error(2,...p)===0){site=p;break;}
assert.ok(site);assert.equal(w.cmd_place(2,...site),1);run(5);
// These two workers have later entity IDs than the farm. A per-building reset
// halfway through the entity loop used to let them bypass the one-worker cap.
pick(w.start_x(),w.start_y());assert.equal(w.cmd_train(8),1);assert.equal(w.cmd_train(8),1);run(5);
b=save();l=layout(b);const farm=entities(b).find(e=>e.def===2),farmers=entities(b).filter(e=>e.def===8);
assert.ok(farm);
assert.equal(farmers.length,7);assert.ok(farmers.some(e=>e.i<farm.i)&&farmers.some(e=>e.i>farm.i),'farm fixture mixes older and newly trained entity IDs');
for(const [k,e]of farmers.entries()){
 const angle=k*2*Math.PI/farmers.length,x=farm.x+2*Math.cos(angle),y=farm.y+2*Math.sin(angle);
 for(const [off,value]of [[8,x],[12,y],[16,x],[20,y]])l.d.setFloat32(e.a+off,value,true);
}
assert.equal(load(b),1);const f=entities(save()).find(e=>e.def===8);pick(f.x,f.y,1);const foodBefore=w.resource_stat(0,0);w.cmd_context(farm.x,farm.y,0);run(40);
const carries=entities(save()).filter(e=>e.def===8).map(e=>e.carry);
assert.equal(carries.filter(v=>v>1).length,1,'only one citizen works a native farm');
assert.ok(Math.abs(carries.reduce((a,v)=>a+v,0)-8.74)<.08,'one farm produces4.37/s, not four simultaneous worker streams');
assert.equal(w.resource_stat(0,0),foodBefore,'farm food is carried before deposit');
console.log('PASS original worker limits: wood3, forage6, iron6, farm1; extra citizens cannot exceed native caps, with native carry rates and delayed bank credit.');
