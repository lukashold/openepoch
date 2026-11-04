// Actual rules binaries and the production presentation adapter, without a UI.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {createFrontendNamespace} from '../docs/render-namespace.js';
const arg=name=>process.argv.find(value=>value.startsWith('--'+name+'='))?.split('=').slice(1).join('=');
const mappingURL=arg('mapping')?pathToFileURL(path.resolve(arg('mapping'))):new URL('../docs/original-render-namespace.js',import.meta.url);
const {createRenderAssetMapping}=await import(mappingURL);
const wideURL=arg('wasm')?pathToFileURL(path.resolve(arg('wasm'))):new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url);
let checks=0;const check=(value,note)=>{assert.ok(value,note);checks++;},equal=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;};
const sha=bytes=>createHash('sha256').update(bytes).digest('hex'),binaries={};
async function binary(url){const bytes=await fs.readFile(url),{instance:{exports:wasm}}=await WebAssembly.instantiate(bytes,{});return {wasm,sha256:sha(bytes),bytes:bytes.length};}
for(const [name,filename]of [['own','openepoch.wasm'],['base','openepoch-ee2.wasm'],['up5','openepoch-ee2-up15.wasm']]){
 const value=await binary(new URL('../docs/'+filename,import.meta.url)),w=value.wasm;
 if(typeof w.schema_revision==='function')continue;
 const mapping=createRenderAssetMapping(w),api=createFrontendNamespace(w,{referenceMode:name!=='own',assetMapping:mapping});
 equal(mapping,null,'Legacy metadata remains absent');equal(api.rangeLength,512,'Legacy range length');check(!api.wide,'Legacy adapter');
 const sample=Float32Array.from({length:36},(_,i)=>i);equal(api.fallbackInstances(sample),sample,'Legacy GPU instances retain their original buffer');
 for(let id=0;id<256;id++)equal(api.fallbackModelId(id),id,'Legacy fallback model identity');
 equal(api.queueJob(255),{kind:3,value:0},'Legacy epoch job');
 equal(api.queueJob(name==='own'?200:64),{kind:name==='own'?2:1,value:0},'Legacy research/tech job');
 equal(api.queueJob(8),{kind:0,value:8},'Legacy citizen job');binaries[name]={sha256:value.sha256,bytes:value.bytes};
}
const value=await binary(wideURL),w=value.wasm;check([6,7].includes(w.schema_revision()),'Candidate advertises a supported typed schema');
for(const revision of [0xffffffff,5,8]){assert.throws(()=>createFrontendNamespace({...w,schema_revision:()=>revision}),/Unsupported presentation schema/);checks++;}
// These are deliberately called before init, matching production startup.
const mapping=createRenderAssetMapping(w),api=createFrontendNamespace(w,{referenceMode:true,assetMapping:mapping});
equal(api.definitionCount,w.definition_count(),'Static catalogue before init');equal(api.rangeLength,w.ranges_len(),'Range export aliases before init');
for(const [raw,kind,source,template]of [[0,1,100,100],[3,1,103,103],[4,1,104,102],[10,1,110,102],[17,2,1,104],[22,2,6,109],[32,4,110,110],[33,4,111,111],[40,3,240,240],[43,3,243,243],[256,0,0,0],[264,0,8,8]]){
 equal(api.asset(raw),{renderId:raw,kind,sourceId:source},'ABI source identity remains separate from raw render identity');equal(api.fallbackModelId(raw),template,'Typed fallback never aliases a resource to a projectile or flag');
}
for(const id of [0,8,56,64,99,100,179,243,255,256,310,65535])equal(api.queueJob(w.queue_job_encode(0,id)>>>0),{kind:0,value:id},'16-bit unit jobs preserve boundary IDs');
for(let id=0;id<180;id++)equal(api.queueJob(w.queue_job_encode(1,id)>>>0),{kind:1,value:id},'Research has its own namespace');
equal(api.queueJob(w.queue_job_encode(3,0)>>>0),{kind:3,value:0},'Epoch has its own namespace');
// A synthetic larger registry proves the concrete100..110 collision reported
// in review; it is not presented as an original unit/civilization catalogue.
const futureCount=311,futureWasm={...w,definition_count:()=>futureCount,render_range_len:()=>2*(256+futureCount),definition_flags:id=>id===101?1:id===102?6:2,
 render_asset_kind:id=>id>=256&&id<256+futureCount?0:w.render_asset_kind(id),render_asset_source_id:id=>id>=256&&id<256+futureCount?id-256:w.render_asset_source_id(id)};
const future=createFrontendNamespace(futureWasm,{referenceMode:true,assetMapping:createRenderAssetMapping(futureWasm)});
for(const [id,template]of [[64,15],[100,15],[101,0],[102,8],[104,15],[110,15],[255,15],[256,15],[310,15]]){
 equal(future.fallbackModelId(future.unitRenderId(id)),template,'Future units cannot become procedural resource/projectile meshes');equal(future.unitFallbackId(id),template,'HUD fallback uses the same generic unit role');
}
for(const raw of [-1,1.5,NaN,0x100b4,0x40000,0xffffffff,0x100000000])equal(api.queueJob(raw),null,'Malformed job rejected');
assert.throws(()=>createFrontendNamespace(w,{referenceMode:true,assetMapping:mapping.slice(1)}));checks++;
assert.throws(()=>createFrontendNamespace(w,{referenceMode:true,assetMapping:[...mapping,mapping[0]]}));checks++;
const scenes=[];
for(const epoch of [0,7,14]){
 w.init(7,1,0,1,0);w.skirmish_epoch(epoch);w.skirmish_visibility(2);
 for(let tick=0;tick<20;tick++)w.tick();
 const n=w.render(1,0,0,256,256,-1,0,0),ptr=w.inst_ptr(),raw=new Float32Array(w.memory.buffer,ptr,n*12),before=raw.slice(),fallback=api.fallbackInstances(raw);
 equal(raw,before,'Fallback upload cannot mutate authoritative WASM memory');
 check(fallback.buffer!==raw.buffer,'Typed fallback has independent memory');
 const rangesPtr=w.ranges_ptr(),ranges=new Uint32Array(w.memory.buffer,rangesPtr,api.rangeLength);let total=0;
 for(let id=0;id<api.slotCount;id++){
  const first=ranges[id*2],count=ranges[id*2+1];check(first+count<=n,'Every raw range remains within the instance stream');total+=count;
  for(let j=first;j<first+count;j++)equal(raw[j*12+4],id,'Raw range indexes match the actual render stream');
 }
 equal(total,n,'Every actual instance has one range');
 for(let i=0;i<n;i++)for(let field=0;field<12;field++)if(field!==4)equal(fallback[i*12+field],before[i*12+field],'Only the fallback shader asset ID can change');
 for(let id=0;id<api.definitionCount;id++)equal(api.isMobile(id),!!((w.definition_flags(id)>>>0)&2),'Source definition flags drive activity labels');
 equal(api.fallbackInstances(raw).buffer,fallback.buffer,'Steady-state fallback upload reuses its buffer');scenes.push({epoch,instances:n,rangeLength:ranges.length});
}
binaries.roster6={sha256:value.sha256,bytes:value.bytes};
const report={format:'openepoch-native6-render-namespace-tests-v1',passed:true,checks,binaries,scenes,sourceHashes:{adapter:sha(await fs.readFile(new URL('../docs/render-namespace.js',import.meta.url))),mapping:sha(await fs.readFile(mappingURL))},scope:'Actual pre-init metadata, typed jobs, real WASM render streams/ranges and non-mutating fallback buffers; no browser or original executable.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native6-render-namespace-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
