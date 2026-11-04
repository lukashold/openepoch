import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {buildNativeWaterData,loadOriginalWater} from '../docs/original-water.js';
const root=new URL('../../../work/native-water3-qa/',import.meta.url),manifestURL=new URL('../docs/local-original/water-native/manifest.json',import.meta.url),manifest=JSON.parse(await fs.readFile(manifestURL));
await fs.mkdir(root,{recursive:true});
const originalFetch=globalThis.fetch,originalBitmap=globalThis.createImageBitmap;
globalThis.fetch=async url=>{const path=fileURLToPath(url),bytes=await fs.readFile(path);return {ok:true,status:200,json:async()=>JSON.parse(bytes),blob:async()=>({bytes,url:path})};};
globalThis.createImageBitmap=async blob=>({url:blob.url,width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),close(){}});
let checks=0;const check=(a,message)=>{assert.ok(a,message);checks++;};
const fields=[];
try{
 const cases=[['ocean',7,-1],['river',6,-1],['coast',5,-.25],['shore',5,-1],['fallback',7,-1],['reflection',7,-1]];
 for(const [name,kind,level] of cases){
  const size=8,width=128,height=128,heights=new Float32Array(81).fill(level),movement=new Uint8Array(81).fill(kind);
  if(name==='shore')for(let z=0;z<=size;z++)for(let x=0;x<=size;x++){heights[z*9+x]=(x-4)*.25;movement[z*9+x]=x<2?7:x<5?5:x<7?6:0;}
  const computed=buildNativeWaterData({heights,movement,size,paletteAlphaBytes:manifest.paletteAlphaBytes,lookupRGBBytes:manifest.lookupRGBBytes,uvScale:manifest.adapter.uvScale});
  fields.push({name,size,heights:[...heights],movement:[...movement],vertices:[...computed.vertices],indices:[...computed.indices]});
  const gl=new RecordingGL();gl.viewport(0,0,width,height);const module=await loadOriginalWater(gl,true);
  check(module.enabled&&module.errors.length===0,'Real candidate loader');check(module.updateMap({heights,movement,size}),'Initial native field upload');check(!module.updateMap({heights,movement,size}),'Unchanged fields reuse native vertex buffers');
  const fog=gl.createTexture();gl.activeTexture(gl.TEXTURE0+2);gl.bindTexture(gl.TEXTURE_2D,fog);gl.texImage2D(gl.TEXTURE_2D,0,gl.R8,1,1,0,gl.RED,gl.UNSIGNED_BYTE,new Uint8Array([255]));
  for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.NEAREST);
  const reflected=gl.createTexture(),pixels=new Uint8Array(4*4*4);for(let z=0;z<4;z++)for(let x=0;x<4;x++)pixels.set([40+x*40,70+z*30,180-x*20,255],(z*4+x)*4);
  gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,reflected);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,4,4,0,gl.RGBA,gl.UNSIGNED_BYTE,pixels);
  for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.LINEAR);for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,key,gl.CLAMP_TO_EDGE);
  const vp=new Float32Array([2/size,0,0,0,0,0,0,0,0,2/size,0,0,-1,-1,0,1]);
  gl.draws=[];gl.clearColor(.08,.11,.15,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT|gl.STENCIL_BUFFER_BIT);
  const before=[gl.getParameter(gl.DEPTH_WRITEMASK),gl.getParameter(gl.DEPTH_FUNC),gl.isEnabled(gl.BLEND),gl.getParameter(gl.ACTIVE_TEXTURE)];
  if(name==='fallback')module.drawWater({vp,time:12,fogTexture:fog});
  else if(name==='reflection')module.drawWater({vp,time:12,fogTexture:fog,reflectionTexture:reflected,reflectionVP:vp});
  else module.drawBase({vp,time:12,fogTexture:fog,profile:'high'});
  assert.deepEqual([gl.getParameter(gl.DEPTH_WRITEMASK),gl.getParameter(gl.DEPTH_FUNC),gl.isEnabled(gl.BLEND),gl.getParameter(gl.ACTIVE_TEXTURE)],before);checks++;
  check(gl.draws[0].state.depthWrite&&gl.draws[0].state.depthFunc===gl.LEQUAL,'Native base depth writes and LessEqual');
  check(gl.draws[0].state.blend[0]===gl.SRC_ALPHA&&gl.draws[0].state.blend[1]===gl.ONE_MINUS_SRC_ALPHA,'Native source-alpha composition');
  if(name==='reflection'){check(gl.draws.length===2&&!gl.draws[1].state.depthWrite,'Separate native reflection overlay pass');check(gl.draws[1].uniforms.uReflectionAlpha.value===manifest.native.reflectionAlpha,'Native factory alpha reaches actual draw');}
  const payload={format:'openepoch-production-world-capture-v1',width,height,epoch:0,scene:'native-water-candidate-'+name,instances:0,
   scope:'Production native water equations and actual original texture loader on synthetic fixed-height/native-movement grid; no simulation/world/native executable/browser.',
   fixture:{name,size,time:12,background:[.08,.11,.15,1],profile:name==='fallback'?'fallback':'high',reflection:name==='reflection',heights:[...heights],movement:[...movement],vertices:[...computed.vertices],reflectionPixels:[...pixels],reflectionSize:4},...gl.payload()};
  await fs.writeFile(new URL(`capture-${name}.json`,root),JSON.stringify(payload));
 }
 const dry=buildNativeWaterData({heights:new Float32Array(81).fill(1),movement:new Uint8Array(81),size:8,paletteAlphaBytes:manifest.paletteAlphaBytes,lookupRGBBytes:manifest.lookupRGBBytes});check(dry.indices.length===0,'Native grid excludes triangles entirely above water');
 const own=buildNativeWaterData({heights:new Float32Array(81).fill(-1),types:new Uint8Array(64).fill(1),size:8,paletteAlphaBytes:manifest.paletteAlphaBytes,lookupRGBBytes:manifest.lookupRGBBytes});check(own.classification==='own-archipelago-class-adapter','Own class mapping provenance explicit');
 for(const options of [{heights:[]},{movement:new Uint8Array(81).fill(8)},{paletteAlphaBytes:[NaN]},{uvScale:[Infinity,1]}]){assert.throws(()=>buildNativeWaterData({heights:new Float32Array(81).fill(-1),movement:new Uint8Array(81),size:8,paletteAlphaBytes:manifest.paletteAlphaBytes,lookupRGBBytes:manifest.lookupRGBBytes,...options}),RangeError);checks++;}
 const report={format:'native3-water-contract-tests-v1',passed:true,checks,sourceSha256:createHash('sha256').update(await fs.readFile(new URL('../docs/original-water.js',import.meta.url))).digest('hex'),manifestSha256:createHash('sha256').update(await fs.readFile(manifestURL)).digest('hex'),fields};
 await fs.writeFile(new URL('contract-and-fields.json',root),JSON.stringify(report));
 await fs.writeFile(new URL('../../EE2-Research/analysis/native3-water-contract-tests.json',import.meta.url),JSON.stringify({...report,fields:undefined},null,2)+'\n');
 console.log(JSON.stringify({passed:true,checks,cases:cases.length,sourceSha256:report.sourceSha256}));
}finally{globalThis.fetch=originalFetch;globalThis.createImageBitmap=originalBitmap;}
