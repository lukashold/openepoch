import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{buildOrdinaryCiv8Unit}from'./ordinary-civ8-fixture.mjs';import{save,load}from'./roster6-state.mjs';import{assertCurrentSave}from'./save-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),arg=(key,fallback)=>process.argv.find(x=>x.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
const wasm=arg('wasm',path.resolve(root,'../builds/openepoch-civ8.wasm')),blob=fs.readFileSync(wasm),sha=b=>createHash('sha256').update(b).digest('hex'),make=async()=>(await WebAssembly.instantiate(blob,{})).instance.exports;
const catalogue=JSON.parse(fs.readFileSync(path.resolve(root,'../civ8-sources.json'),'utf8'));let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};const outcomes=[];
for(const family of catalogue.candidateFamilies){
 const w=await make(),result=buildOrdinaryCiv8Unit(w,family);checks+=16;
 const oldHash=w.state_hash(),before=save(w),peer=await make();eq(load(peer,before),1,'actual new-definition save load');eq(peer.state_hash(),oldHash,'new-definition state hash');eq(sha(save(peer)),sha(before),'new-definition exact roundtrip');
 const layout=assertCurrentSave(before);eq(layout.d.getUint32(12,true),80);eq(peer.player_civilization(0),family.TTCiv-1);
 for(let t=0;t<40;t++){w.tick();peer.tick();eq(w.state_hash(),peer.state_hash(),'new-definition peer continuation');}
 const{state,...record}=result;outcomes.push({...record,stateSha256:sha(state)});
 console.log(JSON.stringify({family:family.sourceFamily,buildTicks:record.buildTicks,trainTicks:record.trainTicks}));
}
const report={format:'native-civ8-ordinary-production-v1',status:'PASS',checks,wasmSha256:sha(blob),sourceCatalogueSha256:sha(fs.readFileSync(path.resolve(root,'../civ8-sources.json'))),outcomes,activeWrites:false,originalExecuted:false,limitations:['Complete attack/pack/turret/civ event behavior remains separately reviewed before activation.']};
fs.writeFileSync(arg('report',path.resolve(root,'../civ8-production-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:'PASS',checks,families:outcomes.length,wasmSha256:report.wasmSha256}));
