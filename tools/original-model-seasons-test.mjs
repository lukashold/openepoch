import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {loadOriginalModels} from '../docs/original-renderer.js';
import {modelYearPhase,modelSeasonFrames,modelSeasonBinding} from '../docs/original-model-seasons.js';
const docs=fileURLToPath(new URL('../docs/',import.meta.url)),sha=b=>createHash('sha256').update(b).digest('hex');
const manifest=JSON.parse(await fs.readFile(path.join(docs,'local-original/model-seasons.json')));
let checks=0;const check=(v,message)=>{assert.ok(v,message);checks++;};
for(const phase of [{month0:-1},{month0:12},{month0:.5},{month0:0,month1:2},{month0:0,blend:NaN},{month0:0,blend:-1},{month0:0,blend:2}]){assert.throws(()=>modelYearPhase(phase),RangeError);checks++;}
assert.deepEqual(modelSeasonFrames(null,.5,'static'),{first:'static',next:'static',blend:0});checks++;
assert.deepEqual(modelSeasonFrames(['a','b'],NaN,'static'),{first:'static',next:'static',blend:0});checks++;
assert.deepEqual(modelSeasonFrames(['a','b'],-.25,'static'),{first:'b',next:'a',blend:.5});checks++;
for(const [climate,sets]of Object.entries(manifest.sets))for(const [key,set]of Object.entries(sets)){
 check(set.urls.length===set.paths.length,`${climate}/${key}: source duplicates/order retained`);
 for(let month=0;month<12;month+=.25){
  const phase=modelYearPhase({month0:Math.floor(month),blend:month%1}),pair=modelSeasonFrames(set.urls,phase,'static');
  const n=month/12*set.paths.length,i=Math.floor(n)%set.urls.length;
  check(pair.first===set.urls[i]&&pair.next===set.urls[(i+1)%set.urls.length],`${climate}/${key}: source annual slots`);
  check(Math.abs(pair.blend-(pair.first===pair.next?0:n-Math.floor(n)))<1e-12,`${climate}/${key}: source interpolation`);
 }
}
const previousFetch=globalThis.fetch,previousBitmap=globalThis.createImageBitmap,requests=new Set();
globalThis.fetch=async input=>{
 const filename=fileURLToPath(input),bytes=await fs.readFile(filename);requests.add(filename);
 return {ok:true,status:200,json:async()=>{
  const value=JSON.parse(bytes);
  if(filename.replaceAll('\\','/').endsWith('/models/manifest.json'))value.models=value.models.filter(r=>(r.id===0&&r.epoch===0)||r.id===100||r.id===101);
  return value;
 },blob:async()=>({bytes,url:filename})};
};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
const flush=()=>new Promise(resolve=>setImmediate(resolve));
async function until(predicate){for(let k=0;k<200&&!predicate();k++)await new Promise(resolve=>setTimeout(resolve,5));}
try{
 const gl=new RecordingGL(),renderer=await loadOriginalModels(gl,true,[[0,0,1],[1,0,0],[0,1,0],[1,1,0]]);
 assert.deepEqual(renderer.errors,[]);checks++;
 assert.deepEqual([...renderer.available],[0,100,101]);checks++;
 check(renderer.setSeasonPhase({month0:0}),'Production season API enabled');
 assert.throws(()=>renderer.setSeasonClimate('guessed'),RangeError);checks++;
 const oak=renderer.models.get(100)[0].parts[0],city=renderer.models.get(0)[0].parts[0];
 check(oak.forSeason(0).first.image.endsWith('tre_oak_wi01.png'),'January oak uses explicitly bound winter skin');
 check(city.forSeason(0).first.image.endsWith('bld_citycenter1_we_wi.png'),'Winter TC immediately has the correct resident neutral skin');
 await until(()=>city.forSeason(0).first.image.endsWith('bld_citycenter1_we_wi-A.png'));
 check(city.forSeason(0).first.image.endsWith('bld_citycenter1_we_wi-A.png'),'Verified original winter player patch replaces neutral skin');
 renderer.setSeasonPhase({month0:2,blend:.5});
 const spring=oak.forSeason(0);check(spring.first.image.endsWith('tre_oak_wi01.png')&&spring.next.image.endsWith('tre_oak_summer.png')&&spring.blend===.5,'Oak winter→summer interpolation uses original adjacent textures');
 renderer.setSeasonClimate('Arid');check(oak.forSeason(0).first===oak.forSeason(0).next&&oak.forSeason(0).blend===0,'Original arid list stays constant');
 renderer.setSeasonClimate('Temperate');
 const identity=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);
 const data=new Float32Array([10,10,0,0,0,0,1,96,5,0,0,0,20,10,0,0,100,0,1,32,1,0,0,0]);
 const ranges=new Uint32Array(512);ranges[0]=0;ranges[1]=1;ranges[200]=1;ranges[201]=1;
 renderer.draw({data,ranges,vp:identity,epochs:[0],zoom:100});
 const normal=gl.draws.at(-1),metrics=structuredClone(renderer.metrics);
 check(normal.uniforms.uModelSeasonBlend.value===.5,'Production draw uploads seasonal blend');
 check(normal.uniforms.uClipEnabled.value===0,'Main draw explicitly disables stale clip planes');
 check(normal.textures['7:3553']!==normal.textures['0:3553'],'Both adjacent seasonal samplers are independently bound');
 renderer.draw({data,ranges,vp:identity,epochs:[0],zoom:100,capture:true,clipPlane:[0,1,0,-.5]});
 check(gl.draws.at(-1).uniforms.uClipEnabled.value===1,'Capture enables world clipping');
 assert.deepEqual(gl.draws.at(-1).uniforms.uClipPlane.value,[0,1,0,-.5]);checks++;
 assert.deepEqual(renderer.metrics,metrics,'Capture preserves main-pass statistics');checks++;
 renderer.draw({data,ranges,vp:identity,epochs:[0],zoom:.01});
 await until(()=>renderer.models.get(0)[0].far);
 const far=renderer.models.get(0)[0].far;
 check(!!far,'Real native far LOD loads');
 check(far.parts.some(part=>part.forSeason(0).blend===.5),'Far LOD keeps its own explicit seasonal node binding');
 const model=JSON.parse(await fs.readFile(path.join(docs,'local-original/models/tre_oak1.json'))),part=model.parts[0];
 check(modelSeasonBinding(manifest,'local-original/models/tre_oak1.json',model,part)==='tre_oak01','Exact NIF source-node lookup');
 check(modelSeasonBinding(manifest,'local-original/models/tre_oak1.json',{...model,source:{sha256:'changed'}},part)===null,'Changed NIF source cannot silently reuse binding');
 check(modelSeasonBinding(manifest,'local-original/models/tre_oak1.json',model,{...part,sourceNode:99999})===null,'No source-node/name guessing');
 assert.deepEqual(renderer.errors,[]);checks++;
}finally{globalThis.fetch=previousFetch;globalThis.createImageBitmap=previousBitmap;}
const baseline=JSON.parse(await fs.readFile(new URL('../../../work/season-model-baseline/model-hashes.json',import.meta.url)));
for(const [url,digest]of Object.entries(baseline)){check(sha(await fs.readFile(path.join(docs,url)))===digest,`Byte-identical geometry/animation ${url}`);}
const report={status:'passed',format:'ee2-model-season-loader-tests-v1',checks,uiLaunched:false,sourceManifestSha256:sha(await fs.readFile(path.join(docs,'local-original/model-seasons.json'))),rendererSha256:sha(await fs.readFile(path.join(docs,'original-renderer.js'))),unchangedModelFiles:Object.keys(baseline).length,requestedLocalAssets:requests.size,
 scope:'Real asset production loader/near/far/team/phase/capture tests using a WebGL call recorder; numerical annual sampling for every imported climate/set. Actual GLES pixels are tested separately.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native-model-season-loader-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
