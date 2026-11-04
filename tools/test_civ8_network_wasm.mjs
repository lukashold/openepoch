import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';import{save,load}from'./roster6-state.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),arg=(k,f)=>process.argv.find(s=>s.startsWith('--'+k+'='))?.slice(k.length+3)||f,blob=fs.readFileSync(arg('wasm',path.resolve(root,'../builds/openepoch-civ8.wasm'))),sha=b=>createHash('sha256').update(b).digest('hex'),make=async()=>(await WebAssembly.instantiate(blob,{})).instance.exports;
let checks=0,packets=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
const peers=[await make(),await make()];
for(let seat=0;seat<2;seat++){const w=peers[seat];w.init(41,0,0,2,seat);eq(w.configure_civilizations(0xee60),1);eq(w.player_civilization(0),0);eq(w.player_civilization(1),6);eq(w.schema_revision(),8);}
eq(peers[0].state_hash(),peers[1].state_hash(),'same resolved-civilization start');
function flush(sender,team,receivers){const bytes=new Uint8Array(sender.memory.buffer,sender.outbox_ptr(),sender.outbox_len()).slice();assert.ok(bytes.length>0);checks++;sender.outbox_clear();for(const w of receivers){const p=w.inbox_alloc(bytes.length);new Uint8Array(w.memory.buffer,p,bytes.length).set(bytes);w.net_apply(team);}packets++;}
for(let seat=0;seat<2;seat++){
 const w=peers[seat];w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);eq(w.cmd_queue_job(8),1);eq(w.cmd_queue_job(57),1);flush(w,seat,peers);eq(peers[0].state_hash(),peers[1].state_hash(),'actual source-country packet application');
}
for(let tick=0;tick<1000;tick++){for(const w of peers)w.tick();if(tick%17===0)peers[0].render(.1,0,0,256,256,-1,0,0);if(tick%29===0)peers[1].render(.9,0,0,256,256,-1,0,0);eq(peers[0].state_hash(),peers[1].state_hash(),'country peers '+tick);}
const restored=await make();eq(load(restored,save(peers[1])),1);eq(restored.state_hash(),peers[1].state_hash());eq(restored.player_civilization(1),6);eq(restored.configure_civilizations(0xeee3),0,'no midgame country swap');
for(let tick=0;tick<200;tick++){restored.tick();for(const w of peers)w.tick();eq(restored.state_hash(),peers[0].state_hash(),'restored multiplayer country state');}
const different=await make();different.init(41,0,0,2,0);eq(different.configure_civilizations(0xeee3),1);assert.notEqual(different.state_hash(),peers[0].state_hash());checks++;
const report={format:'native-civ8-actual-two-seat-civilization-v1',status:'PASS',checks,actualPackets:packets,ticks:1000,restoredTicks:200,wasmSha256:sha(blob),civs:[0,6],activeWrites:false,limits:['Root frontend must carry canonical resolved civpack and reject previous fingerprints/schemas; no new native10-player claim.']};
fs.writeFileSync(arg('report',path.resolve(root,'../civ8-network-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
