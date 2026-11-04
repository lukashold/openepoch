import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {parseCheat} from '../docs/cheats.js';
import {assertCurrentSave} from './save-fixture.mjs';
assert.equal(parseCheat('  SeA   MONKEYS '),3);assert.equal(parseCheat('GIVE TECH'),4);assert.equal(parseCheat('not a cheat'),null);
for(const file of ['openepoch.wasm']){
 const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/'+file,import.meta.url)),{})).instance.exports;
 const ui=()=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
 const run=n=>{for(let i=0;i<n;i++)w.tick();};
 const save=()=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
 const load=b=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
 w.init(42,1,0,1,0);let h=w.state_hash();assert.equal(w.cheat(2),0);assert.equal(w.state_hash(),h);
 assert.equal(w.cheat(0),1);assert.equal(w.cheat(2),1);assert.deepEqual([3,4,5].map(k=>w.stat(0,k)),[10400,10400,10200]);assert.equal(w.stat(1,3),400);
 assert.equal(w.cheat(4),1);assert.equal(w.cheat_status(2),50);assert.equal(w.cheat(3),1);
 const x=w.start_x(),y=w.start_y();
 const tc=()=>{w.select_none();w.select_point(x,y,0,0,0,0,256,256);};
 tc();assert.equal(w.cmd_train(200),1);w.tick();assert.equal(w.stat(0,8),1);assert.equal(w.cheat_status(2),25);
 // Instant production: queued worker completes in one tick; research is separate.
 tc();const count=ui().n;assert.equal(w.cmd_train(8),1);w.tick();assert.equal(ui().sel.q.length,0);w.tick();assert.equal(ui().n,count+1);
 assert.equal(w.cmd_train(255),1);w.tick();assert.equal(w.stat(0,0),0);assert.equal(w.cheat_status(2),25);
 w.select_quad(x-6,y-6,x+6,y-6,x+6,y+6,x-6,y+6,0);
 let site;for(let dx=-8;dx<8&&!site;dx++)for(let dy=4;dy<9&&!site;dy++)if(w.cmd_place(3,x+dx,y+dy))site=[Math.trunc(x+dx)+1.5,Math.trunc(y+dy)+1.5];
 assert.ok(site);w.tick();w.select_none();w.select_point(...site,0,0,0,0,256,256);assert.equal(ui().sel.built,1);
 const bytes=save();h=w.state_hash();w.cheat(1);run(10);assert.equal(load(bytes),1);assert.equal(w.state_hash(),h);assert.deepEqual([0,1,2].map(w.cheat_status),[1,1,25]);
 assert.equal(w.cheat(3),2);assert.equal(w.cheat_status(1),0);assert.equal(w.cheat(1),1);assert.equal(w.cheat(4),0);assert.equal(w.cheat_status(2),25);
 // Prior build saves had no cheat trailer: still load, default inactive and no credits.
 const current=assertCurrentSave(bytes);assert.equal(load(bytes.slice(0,current.end)),1);assert.deepEqual([0,1,2].map(w.cheat_status),[0,0,0]);
 w.init(42,1,0,1,0);assert.deepEqual([0,1,2].map(w.cheat_status),[0,0,0]);
 w.init(42,0,0,2,0);h=w.state_hash();for(let c=0;c<5;c++)assert.equal(w.cheat(c),3);assert.equal(w.state_hash(),h);
 console.log('PASS',file,'cheat gating, loot, research credit, instant construction/production, toggles, saves including old format, multiplayer exclusion');
}

