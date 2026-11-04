import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {loadOriginalModels} from '../docs/original-renderer.js';
const docs=new URL('../docs/',import.meta.url),read=name=>fs.readFile(new URL(name,docs)),sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const full=JSON.parse(await read('local-original/models/manifest.json'));
const subset={...full,models:full.models.filter(row=>(row.id===0||row.id===9)&&row.epoch===0)};
assert.equal(subset.models.length,2,'Source town center and maceman are present');
const originalFetch=globalThis.fetch,originalBitmap=globalThis.createImageBitmap,requests=[];
globalThis.fetch=async url=>{
 const pathname=fileURLToPath(url).replaceAll('\\','/');requests.push(pathname);
 if(pathname.endsWith('/lighting-test-models.json'))return {ok:true,json:async()=>subset};
 if(pathname.endsWith('/lighting-test-scales.json'))return {ok:true,json:async()=>JSON.parse(await read('local-original/model-scales.json'))};
 const bytes=await fs.readFile(url);
 return {ok:true,status:200,json:async()=>JSON.parse(bytes),blob:async()=>({url:pathname,bytes})};
};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
let checks=0;const equal=(a,b,msg)=>{assert.deepEqual(a,b,msg);checks++;};
const options={manifestURL:'lighting-test-models.json',scaleManifestURL:'lighting-test-scales.json'};
const matrix=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
const input={data:new Float32Array([10,10,0,0,0,0,1,32,5,0,0,0,12,10,0,0,9,0,1,32,1,0,0,0]),ranges:new Uint32Array(512),vp:matrix,epochs:[0],zoom:100};
input.ranges[0]=0;input.ranges[1]=1;input.ranges[18]=1;input.ranges[19]=1;
try{
 const gl=new RecordingGL(),renderer=await loadOriginalModels(gl,true,[[1,0,0],[0,1,0],[0,0,1],[1,1,0]],()=>{},options);
 equal(renderer.errors,[]);equal([...renderer.available],[0,9]);equal(renderer.lighting.native,true);equal(renderer.lighting.brightness,1.5);
 equal(renderer.lighting.period,'Afternoon','Explicit application preset');
 renderer.draw(input);
 const parts=[...renderer.models.values()].flatMap(v=>v[0].parts);equal(gl.draws.length,parts.length);
 for(let i=0;i<parts.length;i++){
  const state=gl.draws[i].uniforms,part=parts[i];
  equal(state.uNativeLighting.value,1);equal(state.uTextureScale.value,4);
  equal(state.uNativeShader.value,part.pipeline.skinned?1:0);
  equal(state.uAmbientLight.value,renderer.lighting.ambient);equal(state.uSunColor.value,renderer.lighting.sun);equal(state.uSpecularLight.value,renderer.lighting.specular);
 }
 equal(parts.some(p=>p.pipeline.skinned),true);equal(parts.some(p=>!p.pipeline.skinned),true);
 equal(renderer.setLightingSettings({period:'Night',detail:'Low'}),true);renderer.draw(input);
 equal(gl.draws.at(-1).uniforms.uNativeShader.value,2);equal(gl.draws.at(-1).uniforms.uSunColor.value,renderer.lighting.sun);
 const legacy={ambient:[.1,.2,.3],sun:[.4,.5,.6],direction:[0,1,0]};renderer.draw({...input,lighting:legacy});
 equal(gl.draws.at(-1).uniforms.uNativeShader.value,0);equal(gl.draws.at(-1).uniforms.uTextureScale.value,1);equal(gl.draws.at(-1).uniforms.uNativeLighting.value,0);
 equal(gl.draws.at(-1).uniforms.uSpecularLight.value,legacy.sun);
 renderer.setLightingContext(legacy);renderer.draw(input);equal(gl.draws.at(-1).uniforms.uAmbientLight.value,legacy.ambient);
 for(const bad of [{},{...legacy,direction:[0,0,0]},{...legacy,ambient:[NaN,0,0]}]){assert.throws(()=>renderer.setLightingContext(bad),RangeError);checks++;}
 assert.throws(()=>renderer.setLightingSettings({climate:'guessed'}),RangeError);checks++;
 const disabled=await loadOriginalModels(new RecordingGL(),false,[]);equal(disabled.setLightingContext(legacy),false);
 const oldGL=new RecordingGL(),old=await loadOriginalModels(oldGL,true,[[1,0,0]],()=>{},{...options,nativeLighting:false});
 old.draw(input);equal(oldGL.draws.at(-1).uniforms.uTextureScale.value,1);equal(oldGL.draws.at(-1).uniforms.uNativeLighting.value,0);
 equal(requests.some(url=>url.endsWith('/lighting-test-models.json')),true);equal(requests.some(url=>url.endsWith('/lighting-test-scales.json')),true);
 const result={status:'passed',build:'native5',format:'ee2-native-lighting-loader-test-v1',checks,sourceHashes:{renderer:sha(await read('original-renderer.js')),lighting:sha(await read('original-lighting.js')),data:sha(await read('local-original/climate-lighting.json'))},scope:'Production loader and draw calls: source light defaults, static/skinned stages, nativeHigh/Low changes, explicit legacy draw/set overrides, disabled/older asset paths and alternate model/scale manifest options. No UI launch.'};
 await fs.writeFile(process.argv[2]||new URL('../../EE2-Research/analysis/native5-lighting-loader-tests.json',import.meta.url),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}finally{globalThis.fetch=originalFetch;globalThis.createImageBitmap=originalBitmap;}
