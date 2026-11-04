// Actual mixed combat in two separate WASM memories and different UI cadences.
import{readFileSync,writeFileSync}from'node:fs';import{createHash}from'node:crypto';import assert from'node:assert/strict';
const bytes=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const sha=b=>createHash('sha256').update(b).digest('hex');let checkpoints=0;const cases=[];
for(const seed of[7,41]){
 const a=(await WebAssembly.instantiate(bytes,{})).instance.exports,b=(await WebAssembly.instantiate(bytes,{})).instance.exports;
 a.init(seed,0,400,2,0);b.init(seed,0,400,2,1);
 for(let tick=1;tick<=1200;tick++){
  a.tick();b.tick();
  if(tick%7===0)a.render(.25,0,0,256,256,-1,0,0);
  if(tick%11===0)b.render(.75,60,60,180,180,-1,0,0);
  if(tick%20===0){assert.equal(a.state_hash(),b.state_hash(),`Combat peer seed${seed}/tick${tick}`);assert.equal(a.game_tick(),b.game_tick());checkpoints++;}
 }
 cases.push({seed,armyTargetPerTeam:400,ticks:1200,hash:a.state_hash()>>>0});
}
const report={format:'native-combat-lockstep-v1',status:'passed',wasmSha256:sha(bytes),checkpoints,cases,scope:'Two independent EE2 WASM memories with different local seats and render cadences, real automatic mixed-army combat. This validates engine determinism, not WebRTC signaling or Internet connectivity.'};
writeFileSync(new URL('../../EE2-Research/analysis/native-combat-lockstep.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
