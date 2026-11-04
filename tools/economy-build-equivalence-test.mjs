// Historical BAE9 -> 2A12 metadata/UI-only rebuild comparison. Later gameplay
// features, including diplomacy, are deliberately outside this equivalence claim.
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const before=readFileSync(new URL('./fixtures/reference-ee2-economy-bae9.wasm',import.meta.url));
const after=readFileSync(new URL('./fixtures/reference-ee2-economy-2a12.wasm',import.meta.url));
const sha=b=>createHash('sha256').update(b).digest('hex');
assert.equal(sha(before),'bae9b5cf631996ed5533e75a1341646c9b4b37881da86cd60e4e1e084cdf8d66');
assert.equal(sha(after),'2a127d17ed05e27ddafa54bf78f0dfa61e202bbfc8f099644843d073dda59b47');
const a=(await WebAssembly.instantiate(before,{})).instance.exports,b=(await WebAssembly.instantiate(after,{})).instance.exports;
const balances=w=>Array.from({length:4},(_,team)=>Array.from({length:9},(_,slot)=>w.resource_stat(team,slot)));
const report={format:'openepoch-simulation-equivalence-v1',beforeSha256:sha(before),afterSha256:sha(after),ticksPerCase:1200,seeds:[7,41],scope:'Simulation hashes and all four players/nine resource accounts; deliberately excludes UI JSON strings.',cases:[]};
for(const seed of report.seeds)for(let epoch=0;epoch<15;epoch++){
 for(const w of [a,b]){w.init(seed,3,0,1,0);assert.equal(w.skirmish_epoch(epoch),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);assert.equal(w.cmd_train(8),1);}
 const item={seed,epoch:epoch+1,initialHash:a.state_hash()>>>0};assert.equal(a.state_hash(),b.state_hash(),'equal initial state');assert.deepEqual(balances(a),balances(b),'equal initial banks');
 for(let tick=1;tick<=1200;tick++){
  a.tick();b.tick();
  if(tick%200===0){assert.equal(a.state_hash(),b.state_hash(),`seed${seed} epoch${epoch+1} tick${tick}: deterministic state`);assert.deepEqual(balances(a),balances(b),`seed${seed} epoch${epoch+1} tick${tick}: every resource bank`);}
 }
 item.finalHash=a.state_hash()>>>0;item.finalBalances=balances(a);report.cases.push(item);
}
writeFileSync(new URL('../../EE2-Research/analysis/economy-build-equivalence.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log('PASS historical simulation equivalence BAE9→2A12: all15 initial epochs ×2seeds ×1200ticks, three AI players plus a human production command; identical state hashes and all nine resource banks. Current diplomacy gameplay is outside this historical comparison.');
