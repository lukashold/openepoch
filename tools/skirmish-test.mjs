import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {buildModels} from '../docs/models.js';
const meshes=buildModels();assert.ok(meshes.data.every(Number.isFinite));
for(let id=0;id<53;id++)assert.ok(meshes.count[id]>0,`mesh ${id}`);
for(const file of ['openepoch.wasm']){
 const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/'+file,import.meta.url)),{})).instance.exports;
 const run=n=>{for(let i=0;i<n;i++)w.tick();};
 const ui=()=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
 w.init(42,1,0,1,0);assert.equal(w.epoch_count(),8);
 assert.equal(w.skirmish_setup(500,1),1);
 for(let t=0;t<2;t++)assert.deepEqual([3,4,5].map(k=>w.stat(t,k)),[800,800,400]);
 const hash=w.state_hash();assert.equal(w.skirmish_setup(999,1),0);assert.equal(w.state_hash(),hash);
 w.tick();assert.equal(w.skirmish_setup(1000,2),0);
 w.grant(90000);const x=w.start_x(),y=w.start_y();
 const selectTC=()=>{w.select_none();w.select_point(x,y,0,0,0,0,256,256);};
 for(let ep=1;ep<=7;ep++){selectTC();assert.equal(w.cmd_train(255),1,`queue epoch ${ep}`);run(850);assert.equal(w.stat(0,0),ep);}
 selectTC();assert.equal(w.cmd_train(255),0,'last epoch has no successor');
 // Construct a workshop with actual workers and train both future ground units.
 w.select_quad(x-6,y-6,x+6,y-6,x+6,y+6,x-6,y+6,0);
 let site;
 for(let dx=-10;dx<10&&!site;dx++)for(let dy=4;dy<12&&!site;dy++)if(w.cmd_place(4,x+dx,y+dy))site=[Math.trunc(x+dx)+1.5,Math.trunc(y+dy)+1.5];
 assert.ok(site,'workshop site');run(1200);w.select_none();w.select_point(...site,0,0,0,0,256,256);
 assert.equal(ui().sel.name,'Workshop');assert.equal(ui().sel.built,1);
 for(const id of [43,46]){assert.equal(w.cmd_train(id),1,`future unit ${id}`);run(800);}
 const n=w.render(.5,0,0,256,256,-1,0,0),p=w.inst_ptr(),ins=new Float32Array(w.memory.buffer,p,n*12);
 for(const id of [43,46])assert.ok(Array.from({length:n},(_,i)=>ins[i*12+4]).includes(id),`spawned ${id}`);
 const sp=w.save(),bytes=new Uint8Array(w.memory.buffer,sp,w.save_len()).slice(),saved=w.state_hash();run(10);
 const dst=w.load_alloc(bytes.length);new Uint8Array(w.memory.buffer,dst,bytes.length).set(bytes);assert.equal(w.load_apply(),1);assert.equal(w.state_hash(),saved);
 console.log('PASS',file,'all 8 epochs, symmetric settings, future production, save roundtrip, 53 meshes');
}
