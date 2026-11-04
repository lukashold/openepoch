// Exact hash/save bytes for all five unchanged feature profiles.
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';import{save}from'./roster6-state.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),workspace=path.resolve(root,'../../../..'),sha=b=>createHash('sha256').update(b).digest('hex'),make=async b=>(await WebAssembly.instantiate(b,{})).instance.exports;
const specs=[['own0',path.join(workspace,'work/native-roster6/baseline5/openepoch.wasm')],['reference1',path.join(workspace,'work/native-roster6/baseline5/openepoch-ee2.wasm')],['up5',path.join(root,'tools/fixtures/native5-up15.wasm')],['roster6',path.join(root,'tools/fixtures/native61v2-up15.wasm')],['joint7',path.join(workspace,'work/native-joint7/builds/openepoch-joint7.wasm')]];
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};const profiles=[];
for(const[name,beforePath]of specs){const before=fs.readFileSync(beforePath),after=fs.readFileSync(path.join(work,'builds/openepoch-'+name+'.wasm'));let cases=0;
 for(const seed of[7,41])for(const requested of[0,5,9,14]){
  const old=await make(before),current=await make(after);for(const w of[old,current]){w.init(seed,0,0,1,0);const epoch=Math.min(requested,w.epoch_count()-1);eq(w.skirmish_setup(500,2),1);eq(w.skirmish_epoch(epoch),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);eq(w.cmd_train(8),1);eq(w.cmd_train(8),1);}
  for(let tick=0;tick<=1000;tick++){
   eq(current.state_hash(),old.state_hash(),name+' off hash '+tick);
   if(tick%100===0)eq(sha(save(current)),sha(save(old)),name+' exact full off save '+tick);
   if(tick<1000){old.tick();current.tick();if(tick%23===0)old.render(.1,0,0,256,256,-1,0,0);if(tick%29===0)current.render(.9,0,0,256,256,-1,0,0);}
  }cases++;
 }
 profiles.push({profile:name,beforeSha256:sha(before),rebuiltSha256:sha(after),cases,ticksPerCase:1000});console.log(JSON.stringify({profile:name,cases,status:'PASS'}));
}
const report={format:'native-civ8-off-profiles-v1',status:'PASS',checks,profiles,activeWrites:false,scope:'Profiles0/1/5/6/7 preserve exact state hashes and every saved byte under the candidate source, including render cadence differences.'};
fs.writeFileSync(path.join(work,'civ8-off-profiles-report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:'PASS',checks,profiles:profiles.length}));
