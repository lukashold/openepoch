import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';import{save}from'./roster6-state.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),sha=b=>createHash('sha256').update(b).digest('hex');
const before=fs.readFileSync(path.join(work,'baseline/availability61v2.wasm')),after=fs.readFileSync(path.join(work,'builds/openepoch-classic6.wasm'));
const make=async b=>(await WebAssembly.instantiate(b,{})).instance.exports;let checks=0;
const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
for(const seed of[7,41])for(const epoch of[0,5,9,14]){
 const a=await make(before),b=await make(after);for(const w of[a,b]){w.init(seed,0,0,1,0);eq(w.schema_revision(),6);eq(w.skirmish_epoch(epoch),1);eq(w.skirmish_setup(500,2),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);eq(w.cmd_queue_job(8),1);eq(w.cmd_queue_job(57),1);}
 for(let t=0;t<=800;t++){
  eq(a.state_hash(),b.state_hash(),'off-feature6 hash');
  if(t%200===0)eq(sha(save(a)),sha(save(b)),'off-feature6 full save bytes');
  if(t<800){a.tick();b.tick();if(t%23===0)a.render(.1,0,0,256,256,-1,0,0);if(t%29===0)b.render(.9,0,0,256,256,-1,0,0);}
 }
}
const report={format:'native-paidqueue7-off-profile6-v1',status:'PASS',checks,beforeSha256:sha(before),rebuiltSha256:sha(after),cases:8,ticksPerCase:800,scope:'Feature6 retains exact state hash and every persisted byte. Profiles0/1/5 have their separate1074-check regression.'};
fs.writeFileSync(path.join(work,'off-profile6-report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
