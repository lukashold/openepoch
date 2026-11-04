// Prove that read-only rendering exports preserve the preceding game simulation.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

const project=new URL('../',import.meta.url),research=new URL('../EE2-Research/analysis/',project);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const cases=[];
for(const [profile,file,epochs] of [['ee2','openepoch-ee2.wasm',[0,7,14]],['base','openepoch.wasm',[0,3,7]]]){
 const before=readFileSync(new URL(`tools/fixtures/reference-${profile==='ee2'?'ee2':'base'}-fortress1.wasm`,project));
 const after=readFileSync(new URL(`docs/${file}`,project));
 for(const seed of [7,41])for(const epoch of epochs){
  const a=(await WebAssembly.instantiate(before,{})).instance.exports;
  const b=(await WebAssembly.instantiate(after,{})).instance.exports;
  for(const w of [a,b]){
   w.init(seed,3,0,1,0);assert.equal(w.skirmish_epoch(epoch),1);
   w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);
   assert.equal(w.cmd_train(8),1);
  }
  const counts=new Array(6).fill(0);
  assert.equal(b.terrain_types_len(),65536);
  const p=b.terrain_types_ptr();
  for(const type of new Uint8Array(b.memory.buffer,p,b.terrain_types_len())){
   assert.ok(type<6,`invalid map type ${type}`);counts[type]++;
  }
  assert.equal(counts.reduce((x,y)=>x+y,0),65536);
  const snapshots=[];
  for(let tick=0;tick<=600;tick++){
   // Repeated rendering reads may allocate neither commands nor simulation work.
   b.terrain_types_ptr();b.terrain_types_len();
   if(tick%50===0){
    assert.equal(b.state_hash(),a.state_hash(),`${profile}/seed${seed}/epoch${epoch}/tick${tick}`);
    for(let team=0;team<4;team++)for(let resource=0;resource<(profile==='ee2'?9:3);resource++){
     assert.equal(b.resource_stat(team,resource),a.resource_stat(team,resource));
    }
    snapshots.push(b.state_hash()>>>0);
   }
   if(tick<600){a.tick();b.tick();}
  }
  const saved=[];
  for(const w of [a,b]){const ptr=w.save();saved.push(new Uint8Array(w.memory.buffer,ptr,w.save_len()).slice());}
  assert.deepEqual(saved[0],saved[1],`serialized state ${profile}/${seed}/${epoch}`);
  cases.push({profile,seed,epoch:epoch+1,ticks:600,terrainTypeCounts:counts,hashes:snapshots,beforeSha256:hash(before),afterSha256:hash(after)});
 }
}
const report={format:'graphics-build-equivalence-v1',status:'passed',scope:'Previous fortress1 vs rendering-export build: identical game hashes, player resources and serialized saves. Terrain read-only exports exercised every tick. This does not establish original EE2 equivalence or visual fidelity.',cases};
writeFileSync(new URL('graphics-build-equivalence.json',research),JSON.stringify(report,null,2)+'\n');
console.log(`PASS ${cases.length} scenarios,600 ticks each; exact simulation and saved-state equivalence`);
