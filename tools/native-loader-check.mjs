// Exercise every shipped model, ground and terrain binding through production
// loaders. PNG data stays on disk; separate GLES tests compile actual shaders.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {loadOriginalModels} from '../docs/original-renderer.js';
import {loadOriginalTerrain} from '../docs/original-terrain.js';
import {loadOriginalGround} from '../docs/original-ground.js';
const previousFetch=globalThis.fetch,previousBitmap=globalThis.createImageBitmap,paths=new Set();
globalThis.fetch=async url=>{const path=fileURLToPath(url),bytes=await fs.readFile(path);paths.add(path);return {ok:true,status:200,json:async()=>JSON.parse(bytes),blob:async()=>({bytes,url:path})};};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
try{
 const gl=new RecordingGL(),teams=Array.from({length:8},()=>[1,1,1]);
 const models=await loadOriginalModels(gl,true,teams),terrain=await loadOriginalTerrain(gl,true,teams),ground=await loadOriginalGround(gl,true);
 assert.deepEqual(models.errors,[]);assert.deepEqual(terrain.errors,[]);assert.deepEqual(ground.errors,[]);
 assert.equal(models.manifest.models.length,156);assert.equal(terrain.enabled,true);assert.equal(ground.entries.size,13);
 const report={format:'openepoch-native-loader-check-v1',passed:true,modelBindings:models.manifest.models.length,modelKinds:models.available.size,groundBindings:ground.entries.size,
  loadedFiles:paths.size,errors:0,programCount:gl.programs.size,textureObjects:gl.textures.size,
  scope:'Complete current production JS loader graph using real local JSON/PNG files and a recording GL contract. Actual native GLES shader/pixel tests are separate; no browser/UI interaction claim.'};
 report.sources=Object.fromEntries(await Promise.all(['original-renderer.js','original-camera.js','original-scale.js','original-material.js','original-terrain.js'].map(async name=>[name,createHash('sha256').update(await fs.readFile(new URL('../docs/'+name,import.meta.url))).digest('hex')])));
 await fs.writeFile(new URL('../../EE2-Research/analysis/native-loader-check.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify(report,null,2));
}finally{globalThis.fetch=previousFetch;globalThis.createImageBitmap=previousBitmap;}
