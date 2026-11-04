import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {buildModels} from '../docs/models.js';
import {assertCurrentSave,revealFixture,DIP_BYTES,TER_BYTES,saveLayout} from './save-fixture.mjs';
const mesh=buildModels();for(let id=0;id<53;id++)assert.ok(mesh.count[id]>0,`mesh ${id}`);assert.ok(mesh.data.every(Number.isFinite));
const source=JSON.parse(readFileSync(new URL('../docs/ee2-units.json',import.meta.url)));
const researchSource=JSON.parse(readFileSync(new URL('../docs/research-audit.json',import.meta.url)));
const citizenSource=source.class_rows.find(row=>row.id===8&&row.epoch===0);
const universitySource=source.class_rows.find(row=>row.id===47&&row.epoch===0);
const weaving=researchSource.nodes.find(node=>node.id==='Military1_1').effects.find(effect=>effect.field===0&&effect.targets.includes(8));
assert.equal(citizenSource.fields.hp.verified,true);assert.equal(citizenSource.fields.hp.value,30);
assert.equal(weaving.method,0);assert.equal(weaving.value,20);
const wovenCitizenHP=citizenSource.fields.hp.value*(1+weaving.value/100);
const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url)),{})).instance.exports;
const run=n=>{for(let i=0;i<n;i++)w.tick();};
const ui=()=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
const tree=()=>{const p=w.research_tree();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
const save=()=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=b=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const pick=(x,y)=>{w.select_none();w.select_point(x,y,0,0,0,0,256,256);};
const N=256,NN=N*N;
function layout(bytes){
 const d=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),terrain=36+4*psz+high*esz;
 return {d,psz,terrain,resources:terrain+NN,amount:terrain+2*NN,fog:terrain+4*NN,end:terrain+5*NN};
}
w.init(7,1,0,1,0);w.grant(90000);const x=w.start_x(),y=w.start_y();
// This scenario isolates research/hosts from scouting and map generation. The
// opponent remains a player, but its AI is disabled; the colony has clear land.
let fixture=save(),l=layout(fixture);assert.equal(l.d.getUint32(12,true),57);assert.equal(fixture.length-l.end,236+DIP_BYTES+TER_BYTES+saveLayout(fixture).cityBytes,'nine-resource prefix and diplomacy/territory save extensions');assertCurrentSave(fixture);
fixture[36+l.psz+27]=0;revealFixture(fixture);
for(let ty=Math.floor(y)-24;ty<=Math.floor(y)+24;ty++)for(let tx=Math.floor(x)-24;tx<=Math.floor(x)+24;tx++){
 const tile=ty*N+tx;fixture[l.terrain+tile]=3;fixture[l.resources+tile]=0;l.d.setUint16(l.amount+tile*2,0,true);
}
assert.equal(load(fixture),1);assert.equal(ui().res.length,9);assert.equal(w.research_stat(0,3,8),citizenSource.fields.hp.value);
w.select_idle_worker();const dockSize=ui().act.find(action=>action.k==='b'&&action.id===5)?.size;assert.ok(dockSize>0);
let dockSite;
for(let r=12;r<22&&!dockSite;r++)for(let dx=-r;dx<=r&&!dockSite;dx++)for(const at of [[Math.floor(x)+dx,Math.floor(y)-r],[Math.floor(x)+dx,Math.floor(y)+r]])if(w.place_error(3,...at)===0){dockSite=at;break;}
assert.ok(dockSite,'owned clear patch for a controlled coast');
fixture=save();l=layout(fixture);
for(let ty=dockSite[1]-1;ty<=dockSite[1]+dockSize;ty++)for(let tx=dockSite[0]-1;tx<=dockSite[0]+dockSize;tx++){
 const tile=ty*N+tx,inside=tx>=dockSite[0]&&tx<dockSite[0]+dockSize&&ty>=dockSite[1]&&ty<dockSite[1]+dockSize;
 fixture[l.terrain+tile]=inside&&tx>dockSite[0]?1:3;fixture[l.resources+tile]=0;l.d.setUint16(l.amount+tile*2,0,true);
}
// Keep a genuine water outlet beyond the dock's eastern edge. The surrounding
// land still gives builders a shore; a sealed inland puddle is not a sea exit.
for(let ty=dockSite[1];ty<dockSite[1]+dockSize;ty++)for(let tx=dockSite[0]+dockSize;tx<=dockSite[0]+dockSize+5;tx++){
 const tile=ty*N+tx;fixture[l.terrain+tile]=1;fixture[l.resources+tile]=0;l.d.setUint16(l.amount+tile*2,0,true);
}
assert.equal(load(fixture),1);assert.equal(w.place_error(5,...dockSite),0,'explored mixed dock footprint has land access, enough water and a genuine sea exit');
w.cheat(0);w.cheat(3);assert.equal(w.epoch_count(),15);
const sites=new Map([[0,[x,y]]]);
function build(id){
 assert.equal(w.unit_epoch(id),source.production_class_from[id],`source-backed minimum epoch for host ${id}`);
 assert.ok(w.stat(0,0)>=w.unit_epoch(id),`host ${id} is built only after its native minimum epoch`);
 const worker=w.select_idle_worker();assert.notEqual(worker,0xffffffff,'actual idle research-colony builder');w.select_point(w.ent_x(worker),w.ent_y(worker),0,1,0,0,N,N);const size=ui().act.find(action=>action.k==='b'&&action.id===id)?.size;assert.ok(size>0,`build action for ${id}`);
 let at;const errors={};
 if(id===5)at=dockSite;
 else for(let r=3;r<65&&!at;r++)for(let dx=-r;dx<=r&&!at;dx++)for(const p of [[x+dx,y-r],[x+dx,y+r],[x-r,y+dx],[x+r,y+dx]]){
  // Research is isolated from narrow corridors: keep two walkable tiles around
  // completed hosts and the reserved genuine dock coast, rather than enclosing
  // the selected citizens with successive instant-built foundations.
  const center=p.map(v=>Math.trunc(v)+size/2),reserved=[...sites].map(([id,point])=>({point,size:w.footprint_size(id,0)}));reserved.push({point:dockSite.map(v=>v+dockSize/2),size:dockSize});
  if(reserved.some(other=>Math.abs(center[0]-other.point[0])<(size+other.size)/2+2&&Math.abs(center[1]-other.point[1])<(size+other.size)/2+2))continue;
  const error=w.place_error(id,...p);if(error===0){at=p;break;}errors[error]=(errors[error]||0)+1;
 }
 assert.ok(at,`site for ${id}; errors ${JSON.stringify(errors)}; selected ${JSON.stringify(ui().sel)}`);assert.equal(w.cmd_place(id,...at),1);run(5);
 const center=at.map(v=>Math.trunc(v)+size/2);pick(...center);assert.equal(ui().sel.def,id);assert.equal(ui().sel.built,1);sites.set(id,center);return center;
}
for(const id of [47,3,48,52,51,5])build(id);
// A university accepts exactly three citizens, generates a real per-player bank and keeps population.
const citizen=w.select_idle_worker();assert.notEqual(citizen,0xffffffff);w.select_point(w.ent_x(citizen),w.ent_y(citizen),0,1,0,0,256,256);assert.equal(ui().sel.n,5);
const population=w.stat(0,1);w.cmd_context(...sites.get(47),0);run(2400);pick(...sites.get(47));
const slots=universitySource.fields.garrison_slots.value;assert.equal(slots,3);
assert.equal(ui().sel.cargo,slots);assert.equal(ui().sel.cargomax,slots);assert.equal(w.stat(0,1),population);
const before=w.research_stat(0,0,0);run(1200);const earned=universitySource.fields.generation.value*slots*1200*.05;
assert.ok(Math.abs(w.research_stat(0,0,0)-before-earned)<1.0);assert.equal(w.research_stat(0,0,0)%1,0,'integer university payouts');assert.equal(w.research_stat(1,0,0),0);
// Save an active research colony and continue it after reloading.
let bytes=save(),hash=w.state_hash();run(10);assert.equal(load(bytes),1);assert.equal(w.state_hash(),hash);
pick(...sites.get(47));w.cmd_unload();run(2);assert.equal(ui().sel.cargo,0);assert.equal(w.stat(0,1),population);const stopped=w.research_stat(0,0,0);run(100);assert.equal(w.research_stat(0,0,0),stopped);
const tc=()=>pick(x,y);
tc();assert.equal(w.cmd_train(255),0,'money alone cannot skip six-technology prerequisite');
for(let i=0;i<100;i++)w.cheat(4);const bank=w.research_stat(0,0,0);
assert.equal(w.cmd_research(4),1);assert.equal(w.cmd_research(4),0,'duplicate research rejected');assert.equal(w.research_stat(0,0,0),bank-8);w.cmd_cancel();assert.equal(w.research_stat(0,0,0),bank,'cancel refunds tech points');
assert.equal(w.cmd_research(4),1);run(305);assert.equal(w.research_stat(0,2,4),1);assert.equal(w.research_stat(0,3,8),wovenCitizenHP,'Weberei applies the source 20 percent bonus to the native 30-HP citizen');assert.equal(w.cmd_research(4),0);
// Complete all available technologies in every age, including the correct hosts and point costs.
for(let ep=0;ep<15;ep++){
 for(const id of [49,50])if(!sites.has(id)&&source.production_class_from[id]<=ep)build(id);
 if(ep===10)build(6);
 const nodes=tree().filter(n=>n.epoch===ep);assert.equal(nodes.length,12);
 for(const n of nodes){
  if(n.done)continue;
  assert.equal(w.cmd_research(n.id),1,`epoch ${ep+1}, ${n.name} host ${n.host}`);run(305);
  assert.equal(w.research_stat(0,2,n.id),1,`finished ${n.name}`);
 }
 assert.equal(w.research_stat(0,1,0),12);
 tc();assert.equal(w.cmd_train(255),ep<14?1:0);if(ep<14){run(305);assert.equal(w.stat(0,0),ep+1);}
}
assert.equal(ui().ename,'Synthetikzeitalter');assert.equal(ui().research.total,180);
bytes=save();hash=w.state_hash();run(50);assert.equal(load(bytes),1);assert.equal(w.state_hash(),hash);assert.equal(ui().research.total,180);
// Invalid extension data must fail without replacing the active game.
for(const offset of [layout(bytes).end+20,layout(bytes).end+140]){
 const invalid=bytes.slice();new DataView(invalid.buffer).setFloat32(offset,NaN,true);assert.equal(load(invalid),0);assert.equal(w.state_hash(),hash);
}
console.log('PASS research: 53 fallback meshes, 57 native definitions/nine resources, source-backed university/HP/host epochs, garrison/generation/unload, point economy/refunds/six-tech gate, all 180 technologies/all 15 epochs, save roundtrip and invalid research/economy rejection; controlled explored coast');
