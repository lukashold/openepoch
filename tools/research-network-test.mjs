import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {Lockstep} from '../docs/net.js';
const bytes=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const units=JSON.parse(readFileSync(new URL('../docs/ee2-units.json',import.meta.url)));
const research=JSON.parse(readFileSync(new URL('../docs/research-audit.json',import.meta.url)));
const citizen=units.class_rows.find(row=>row.id===8&&row.epoch===0),weaving=research.nodes.find(node=>node.id==='Military1_1').effects.find(effect=>effect.field===0&&effect.targets.includes(8));
assert.equal(citizen.fields.hp.verified,true);assert.equal(citizen.fields.hp.value,30);assert.equal(weaving.method,0);assert.equal(weaving.value,20);
const researchedHP=citizen.fields.hp.value*(1+weaving.value/100);
const make=async seat=>{const w=(await WebAssembly.instantiate(bytes,{})).instance.exports;w.init(7,0,0,2,seat);return w;};
const [a,b]=[await make(0),await make(1)];
const q=[[],[]];const ls=[new Lockstep(a,0,x=>q[1].push(x)),new Lockstep(b,1,x=>q[0].push(x))];
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
const pick=(w,x,y)=>{w.select_none();w.select_point(x,y,0,0,0,0,256,256);};
function run(n){for(let t=0;t<n;t++){for(let s=0;s<2;s++){for(const msg of q[s].splice(0))ls[s].receive(msg);assert.equal(ls[s].step(),true);}}assert.equal(a.state_hash(),b.state_hash());assert.ok(!ls[0].desync&&!ls[1].desync);}
const sites=[];
for(const w of [a,b]){
 const x=w.start_x(),y=w.start_y();let at;
 w.select_quad(x-6,y-6,x+6,y-6,x+6,y+6,x-6,y+6,0);
 for(let dx=-8;dx<8&&!at;dx++)for(let dy=4;dy<10&&!at;dy++)if(w.place_error(47,x+dx,y+dy)===0)at=[x+dx,y+dy];
 assert.ok(at);assert.equal(w.cmd_place(47,...at),1);sites.push(at.map(v=>Math.trunc(v)+1.5));
}
run(1800);
for(const [s,w] of [a,b].entries()){
 pick(w,...sites[s]);assert.equal(ui(w).sel.built,1);
 const x=w.start_x(),y=w.start_y();const worker=w.select_idle_worker();assert.notEqual(worker,0xffffffff);w.select_point(w.ent_x(worker),w.ent_y(worker),0,1,0,0,256,256);w.cmd_context(...sites[s],0);
 assert.equal(w.cheat(4),3,'cheats blocked for both seats');
}
run(3000);
for(const [s,w] of [a,b].entries()){
 pick(w,...sites[s]);assert.equal(ui(w).sel.cargo,3);assert.ok(w.research_stat(s,0,0)>=8);assert.equal(w.stat(s,1),5);
 assert.equal(w.cmd_research(4),1);
}
run(400);
for(const [s,w] of [a,b].entries()){assert.equal(w.research_stat(s,2,4),1);assert.equal(w.research_stat(s,3,8),researchedHP,'both seats apply Weberei to the native citizen HP');assert.equal(ui(w).res.length,9);pick(w,...sites[s]);w.cmd_unload();}
run(100);for(const [s,w] of [a,b].entries()){pick(w,...sites[s]);assert.equal(ui(w).sel.cargo,0);assert.equal(w.stat(s,1),5);}
console.log('PASS research lockstep: both players build universities, garrison citizens, earn/spend points, research HP bonus and ungarrison without desync across',ls[0].turn,'turns');
