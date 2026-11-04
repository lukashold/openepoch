import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';import{save,load}from'./roster6-state.mjs';import{saveLayout,refreshFixtureCityGround}from'./save-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),roster=path.resolve(work,'..'),sha=b=>createHash('sha256').update(b).digest('hex');let checks=0;
const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
const fixtures=option('fixtures',null);assert.ok(fixtures,'Use --fixtures=PATH for archived baseline5/ and builds/ binaries. Current UP61 is not the old UP5 oracle.');
const builds=option('builds',path.join(fixtures,'builds'));
const eq=(a,b,label)=>{assert.equal(a,b,label);checks++;},same=(a,b,label)=>eq(sha(a),sha(b),label);
const report={format:'native-classic6-legacy-isolation-v1',cases:[],binaries:{}};
for(const[key,file,newFile]of[['own','openepoch.wasm','rebuilt-own.wasm'],['base','openepoch-ee2.wasm','rebuilt-base.wasm'],['up5','openepoch-ee2-up15.wasm','rebuilt-up5.wasm']]){
 const old=fs.readFileSync(path.join(fixtures,'baseline5',file)),rebuilt=fs.readFileSync(path.join(builds,newFile));report.binaries[key]={frozen:sha(old),rebuilt:sha(rebuilt)};const make=async b=>(await WebAssembly.instantiate(b,{})).instance.exports;
 for(const seed of[7,41])for(const epoch of(key==='own'?[0,2,5,7]:[0,5,10,14])){
  const a=await make(old),b=await make(rebuilt);for(const w of[a,b]){w.init(seed,2,0,1,0);eq(w.skirmish_epoch(epoch),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);eq(w.cmd_train(8),1);}
  for(let tick=0;tick<=1200;tick++){if(tick%200===0){eq(a.state_hash(),b.state_hash(),key+' frozen simulation hash');same(save(a),save(b),key+' complete saved bytes');}if(tick<1200){a.tick();b.tick();}}
  report.cases.push({key,seed,epoch,ticks:1200,saveSha256:sha(save(a))});
 }
 // Source-valid custom player factor catches accidental multiplication reorder.
 const a=await make(old),b=await make(rebuilt);a.init(71,0,0,2,0);let fixture=save(a),l=saveLayout(fixture),d=l.d,at=36+4*d.getUint32(8,true),stride=d.getUint32(4,true),actors=[];
 for(let t=0;t<4;t++){fixture[36+t*d.getUint32(8,true)+24]=key==='own'?7:14;fixture[36+t*d.getUint32(8,true)+27]=0;d.setFloat32(36+t*d.getUint32(8,true)+48,1.27,true);}
 for(let i=0;i<d.getUint32(32,true);i++){const o=at+i*stride;if(!fixture[o]||fixture[o+1]!==8)continue;const team=fixture[o+2];if(actors[team]!==undefined){fixture[o]=0;continue;}actors[team]=i;fixture[o+1]=team===0?16:8;for(const[offset,v]of[[8,100.5+team*5],[12,100.5],[16,100.5+team*5],[20,100.5],[24,0],[28,0],[40,30],[44,0],[80,1]])d.setFloat32(o+offset,v,true);fixture[o+48]=0;fixture[o+89]=0;for(const offset of[60,64,68,72])d.setUint32(o+offset,0xffffffff,true);}
 fixture=refreshFixtureCityGround(fixture);eq(load(a,fixture),1);eq(load(b,fixture),1);
 for(let t=0;t<=200;t++){eq(a.state_hash(),b.state_hash(),key+' actual combat with stored player multiplier');if(t%20===0)same(save(a),save(b),key+' exact combat save');if(t<200){a.tick();b.tick();}}
}
report.status='PASS';report.checks=checks;fs.writeFileSync(option('report',path.join(root,'../EE2-Research/analysis/native61-legacy-profile-tests.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:report.status,checks,cases:report.cases.length}));
