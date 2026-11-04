// Real normally produced Civ8 units through the production model loader/draw API.
// Isolated QA input; no browser, original executable or GUI is launched.
import fs from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {RecordingGL} from './recording-webgl.mjs';
import {loadOriginalModels} from '../docs/original-renderer.js';
import {createRenderAssetMapping} from '../docs/original-render-namespace.js';
import {updateOriginalCamera} from '../docs/original-camera.js';
import {buildOrdinaryCiv8Unit} from './ordinary-civ8-fixture.mjs';
import {save} from './roster6-state.mjs';

const root=new URL('../',import.meta.url),docs=new URL('../docs/',import.meta.url);
const output=new URL('../../captures/',import.meta.url);
await fs.mkdir(output,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const option=name=>process.argv.slice(2).find(x=>x.startsWith('--'+name+'='))?.split('=')[1];
const wanted=option('ids')?.split(',').map(Number);
const all=JSON.parse(await fs.readFile(new URL('../../native-classic-next/civ8/civ8-sources.json',root)));
const candidates=all.candidateFamilies.filter(c=>!wanted||wanted.includes(c.proposedDefId));
const bytes=await fs.readFile(new URL('openepoch-civ8-qa.wasm',docs));
assert.equal(sha(bytes),'e41726ecfa9525ec038b8e21eccc29adaf13ca20d40a1f3da8ac026e52285773');
const {instance}=await WebAssembly.instantiate(bytes,{}),w=instance.exports;
assert.equal(w.definition_count(),80);
const assetMapping=createRenderAssetMapping(w),sourceMap=new Map(assetMapping.filter(r=>r.kind===0).map(r=>[r.sourceId,r.renderId]));
const nativeFetch=globalThis.fetch,nativeBitmap=globalThis.createImageBitmap;
let currentId=0;
globalThis.fetch=async input=>{
 let u=new URL(input);
 // This QA explicitly opts into the staged manifest, without changing Main.
 if(u.pathname.endsWith('/team-textures/manifest.json'))u=new URL('local-original/team-textures/manifest-civ8.json',docs);
 const buffer=await fs.readFile(u),path=fileURLToPath(u);
 return {ok:true,status:200,json:async()=>{
  const value=JSON.parse(buffer);
  if(path.replaceAll('\\','/').endsWith('/models/manifest-civ8.json'))return {...value,models:value.models.filter(r=>r.assetKind===0&&r.id===currentId)};
  return value;
 },arrayBuffer:async()=>buffer.buffer.slice(buffer.byteOffset,buffer.byteOffset+buffer.byteLength),blob:async()=>({bytes:buffer,url:path})};
};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
let reports=[];
try{reports=JSON.parse(await fs.readFile(new URL('../model-captures.json',output))).models;}catch{}
try{
 for(const candidate of candidates){
  currentId=candidate.proposedDefId;
  const fixture=buildOrdinaryCiv8Unit(w,candidate,{seed:7});
  const before=save(w),beforeHash=w.state_hash()>>>0;
  const gl=new RecordingGL(),width=640,height=480;
  const teams=[[.27,.59,1],[.94,.27,.24],[.31,.86,.39],[.96,.82,.27],[1,1,1],[1,1,1],[1,1,1],[.3,.75,1]];
  const models=await loadOriginalModels(gl,true,teams,()=>{},{assetMapping,
   manifestURL:'./local-original/models/manifest-civ8.json',scaleManifestURL:'./local-original/model-scales-civ8.json',
   lightingSettings:{profile:'native155'}});
  assert.deepEqual(models.errors,[]);assert.equal(models.models.size,1);
  const variant=models.models.get(sourceMap.get(currentId))[0];
  assert.ok(variant.nativeScale>0,'exact source scale must be applied');
  const heights=new Float32Array(w.memory.buffer,w.hmap_ptr(),257**2).slice();models.updateTerrain(heights);
  const heightAt=(x,z)=>{
   const ix=Math.max(0,Math.min(255,Math.floor(x))),iz=Math.max(0,Math.min(255,Math.floor(z))),fx=Math.max(0,Math.min(1,x-ix)),fz=Math.max(0,Math.min(1,z-iz)),i=iz*257+ix;
   return Math.max(0,(heights[i]*(1-fx)+heights[i+1]*fx)*(1-fz)+(heights[i+257]*(1-fx)+heights[i+258]*fx)*fz);
  };
  const cam={x:fixture.unit.x,y:fixture.unit.y,nativeFraction:.012,heading:Math.PI/6},view={vp:new Float32Array(16)};
  updateOriginalCamera(cam,view,width,height,heightAt);w.set_eye(...view.eye);
  const count=w.render(1,fixture.unit.x-10,fixture.unit.y-10,fixture.unit.x+10,fixture.unit.y+10,-1,0,0);
  const data=new Float32Array(w.memory.buffer,w.inst_ptr(),count*12).slice();
  const ranges=new Uint32Array(w.memory.buffer,w.ranges_ptr(),w.render_range_len()).slice(),rid=sourceMap.get(currentId);
  assert.equal(ranges[rid*2+1],1,'normally produced unit occurs once in actual stream');
  const epochs=Array.from({length:4},(_,i)=>w.stat(i,0));
  const fog=gl.createTexture();gl.activeTexture(gl.TEXTURE0+2);gl.bindTexture(gl.TEXTURE_2D,fog);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.R8,256,256,0,gl.RED,gl.UNSIGNED_BYTE,new Uint8Array(65536).fill(255));
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  const draw=()=>models.draw({data,ranges,vp:view.vp,epochs,zoom:cam.zoom,eye:view.eye});
  draw();await new Promise(resolve=>setTimeout(resolve,100));gl.draws=[];
  gl.viewport(0,0,width,height);
  gl.clearColor(.12,.16,.19,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT|gl.STENCIL_BUFFER_BIT);
  models.draw({data,ranges,vp:view.vp,epochs,zoom:cam.zoom,eye:view.eye,shadow:true});draw();
  assert.deepEqual(models.errors,[]);
  assert.equal(w.state_hash()>>>0,beforeHash);assert.deepEqual(save(w),before,'presentation preserves simulation/save');
  const recorded=gl.payload();
  const {state,...ordinaryFixture}=fixture;
  const payload={format:'openepoch-production-world-capture-v1',width,height,epoch:candidate.startRuntimeEpoch+1,
   scene:'civ8-real-produced-model-'+currentId,instances:count,cam,eye:view.eye,epochs,
   ordinaryFixture,drawnDefinition:currentId,renderId:rid,sourceHashes:{wasmSha256:sha(bytes),
    runnerSha256:sha(await fs.readFile(new URL(import.meta.url))),modelManifestSha256:sha(await fs.readFile(new URL('local-original/models/manifest-civ8.json',docs))),
    scaleManifestSha256:sha(await fs.readFile(new URL('local-original/model-scales-civ8.json',docs)))},
   scope:'Normally produced real Civ8 unit, original source mesh/KFs/materials/source scale, production renderer and actual untouched render data. Isolated model-only original-camera view; no native full-game visual parity claim.',...recorded};
  const dest=new URL('civ8-model-'+currentId+'.json',output);await fs.writeFile(dest,JSON.stringify(payload));
  const entry={id:currentId,sourceFamily:candidate.sourceFamily,trainTicks:fixture.trainTicks,renderId:rid,
   actualInstances:count,draws:recorded.draws.length,nativeScale:variant.nativeScale,capture:fileURLToPath(dest),captureSha256:sha(await fs.readFile(dest)),simulationUnchanged:true};
  reports=reports.filter(row=>row.id!==entry.id);reports.push(entry);reports.sort((a,b)=>a.id-b.id);console.log(JSON.stringify(entry));
 }
 await fs.writeFile(new URL('../model-captures.json',output),JSON.stringify({format:'native-civ8-real-produced-model-captures-v1',wasmSha256:sha(bytes),models:reports},null,2)+'\n');
}finally{globalThis.fetch=nativeFetch;globalThis.createImageBitmap=nativeBitmap;}
