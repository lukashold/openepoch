// Production terrain/model loaders and draw passes, recorded for native GLES.
// No browser, UI, original executable, or external network is used.
import fs from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {loadOriginalModels} from '../docs/original-renderer.js';
import {loadOriginalTerrain} from '../docs/original-terrain.js';
import {loadOriginalWater} from '../docs/original-water.js';
import {loadOriginalGround} from '../docs/original-ground.js';
import {updateOriginalCamera} from '../docs/original-camera.js';
import {createSeasonClock,normalizeSeasonSettings} from '../docs/original-seasons.js';
import {createOriginalReflection} from '../docs/original-reflection.js';
import {createSeasonSession} from '../docs/season-session.js';
import {resolveRulesProfile,loadRulesWasm} from '../docs/rules-profile.js';

// Existing epoch/scene arguments remain valid. An optional month position2.5
// starts at month2, advances actual WASM ticks, then samples its halfway phase.
const rawArgs=process.argv.slice(2),headingOption=rawArgs.find(arg=>arg.startsWith('--heading=')),tagOption=rawArgs.find(arg=>arg.startsWith('--tag=')),profileOption=rawArgs.find(arg=>arg.startsWith('--profile=')),wantsFarm=rawArgs.includes('--farm'),args=rawArgs.filter(arg=>!arg.startsWith('--heading=')&&!arg.startsWith('--tag=')&&!arg.startsWith('--profile=')&&arg!=='--farm');
const requestedProfile=profileOption?.split('=')[1]??'base';
if(!['base','up15'].includes(requestedProfile))throw Error('World capture supports explicit base/up15 numerical profiles');
const rulesProfile=resolveRulesProfile(`http://localhost/?rules=ee2&profile=${requestedProfile}`);
const captureTag=tagOption?.split('=')[1]??'';if(captureTag&&!/^[a-z0-9-]{1,40}$/.test(captureTag))throw Error('Invalid capture tag');
const headingDegrees=Number(headingOption?.split('=')[1]??0),epoch=Number(args[0]??0),scene=args[1]??'town';
if(!Number.isFinite(headingDegrees))throw Error('Heading must be a finite number of degrees');
const explicitMonth=args[2]!==undefined,requestedMonthPosition=Number(args[2]??6);
const useReflection=args[4]==='reflection';
const shoreObjects=useReflection&&args[5]==='shore-objects';
if(!Number.isInteger(epoch)||epoch<0||epoch>14||!['town','coast'].includes(scene))throw Error('Expected native epoch0..14 and town/coast scene');
if(!Number.isFinite(requestedMonthPosition)||requestedMonthPosition<0||requestedMonthPosition>=12)throw Error('Month position must be0..11.999...');
const seasonSettings=normalizeSeasonSettings({cycle:true,startMonth:Math.floor(requestedMonthPosition),minutesPerSeason:Number(args[3]??5)});
const clock=createSeasonClock(seasonSettings),monthTicks=clock.atTick(0).ticksPerYear/12;
const steps=Math.round((requestedMonthPosition-seasonSettings.startMonth)*monthTicks);
const suffix=(explicitMonth?`-month${String(requestedMonthPosition).replace('.','p')}`:'')+(useReflection?'-reflection':'')+(shoreObjects?'-shore-objects':'')+(headingOption?`-heading${String(headingDegrees).replace('.','p')}`:'')+(wantsFarm?'-farm':'')+(profileOption?`-profile-${requestedProfile}`:'')+(captureTag?`-${captureTag}`:'');
const destination=new URL(`../../../work/world-${epoch}-${scene}${suffix}.json`,import.meta.url);
const wasmPath=new URL(`../docs/${rulesProfile.wasm}`,import.meta.url),bytes=await fs.readFile(wasmPath);
const sha=value=>createHash('sha256').update(value).digest('hex');
const captureScriptSha256=sha(await fs.readFile(new URL(import.meta.url)));
const sourceNames=['main.js','index.html','rules-profile.js','season-session.js','original-terrain.js','original-water.js','original-ground.js','original-ground-native.js','original-scale.js','original-renderer.js','original-model-seasons.js','original-camera.js','original-seasons.js','original-reflection.js','original-hud.js','original-hud-layout.js','local-original/terrain-native/manifest.json','local-original/water-native/manifest.json','local-original/ground-decals.json','local-original/models/manifest.json','local-original/model-seasons.json','local-original/farm-composites.json','local-original/hud-native/manifest.json','local-original/season-model-textures/team/manifest.json','local-original/team-textures/manifest.json'];
const sourceFiles=Object.fromEntries(await Promise.all(sourceNames.map(async name=>[name,sha(await fs.readFile(new URL(`../docs/${name}`,import.meta.url)))])));
const wasm=await loadRulesWasm(rulesProfile,new URL('../docs/',import.meta.url),{fetchFile:async()=>({ok:true,arrayBuffer:async()=>bytes})});
wasm.init(7,1,0,1,0);wasm.skirmish_epoch(epoch);wasm.skirmish_visibility(2);
let farmFixture=null;
if(wantsFarm){
 wasm.cheat(0);wasm.cheat(2);wasm.cheat(3);wasm.select_idle_worker();
 const sx=Math.floor(wasm.start_x()),sz=Math.floor(wasm.start_y());let placed=null;
 for(let radius=4;radius<=24&&!placed;radius++)for(let dz=-radius;dz<=radius&&!placed;dz++)for(let dx=-radius;dx<=radius;dx++){
  if(Math.max(Math.abs(dx),Math.abs(dz))!==radius)continue;const x=sx+dx,z=sz+dz;
  if(wasm.place_error_rotated(2,x,z,0)===0&&wasm.cmd_place(2,x,z)===1){placed={x,z};break;}
 }
 if(!placed)throw Error('No valid production Farm placement found');wasm.tick();farmFixture={...placed,def:2,constructionTicks:1,source:'Actual cheat/select_idle_worker/place_error_rotated/cmd_place/tick production APIs; no Rust/save fixture'};
}
if(typeof wasm.ground_urban_at!=='function')throw Error('Current world proof requires the rebuilt read-only ground_urban_at WASM export');
for(let step=0;step<steps;step++)wasm.tick();
const uiPtr=wasm.ui(),uiLen=wasm.ui_len(),ui=JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,uiPtr,uiLen)));
const simulationTick=typeof wasm.game_tick==='function'?wasm.game_tick():ui.tick;
if(simulationTick!==ui.tick)throw Error('Read-only simulation tick and production UI snapshot disagree');
const seasonPhase=clock.atTick(simulationTick),epochs=[0,1,2,3].map(team=>wasm.stat(team,0));
const width=1280,height=720;
const heights=new Float32Array(wasm.memory.buffer,wasm.hmap_ptr(),257**2).slice();
const types=new Uint8Array(wasm.memory.buffer,wasm.terrain_types_ptr(),256**2).slice();
const groundAt=(x,z)=>{x=Math.max(0,Math.min(255.9999,x));z=Math.max(0,Math.min(255.9999,z));const i=Math.floor(z)*257+Math.floor(x),fx=x%1,fz=z%1;return Math.max(0,(heights[i]*(1-fx)+heights[i+1]*fx)*(1-fz)+(heights[i+257]*(1-fx)+heights[i+258]*fx)*fz);};
const cam={x:wasm.start_x(),y:wasm.start_y(),nativeFraction:.28,heading:headingDegrees*Math.PI/180},view={vp:new Float32Array(16)};
if(scene==='coast'){
 let best=Infinity;
 for(let z=1;z<255;z++)for(let x=1;x<255;x++)if(types[z*256+x]===1&&[types[z*256+x+1],types[z*256+x-1],types[(z+1)*256+x],types[(z-1)*256+x]].some(t=>t>=2)){
  const distance=(x-cam.x)**2+(z-cam.y)**2;if(distance<best){best=distance;cam.coast=[x,z];}
 }
 if(cam.coast)[cam.x,cam.y]=cam.coast;
}
if(shoreObjects){
 // Select an existing native-geometry tree with water toward the actual camera.
 // No simulation entity/map is moved: this only frames a visible reflection.
 const allCount=wasm.render(1,0,0,256,256,-1,0,0),all=new Float32Array(wasm.memory.buffer,wasm.inst_ptr(),allCount*12).slice();let best=Infinity;
 for(let i=0;i<allCount;i++)if(all[i*12+4]===100||all[i*12+4]===101){
  const tx=all[i*12],tz=all[i*12+1];
  for(let dz=1;dz<=3;dz++)for(let dx=-2;dx<=2;dx++){
   const x=Math.floor(tx)+dx,z=Math.floor(tz)+dz;
   if(x<2||z<2||x>253||z>253||types[z*256+x]>=2)continue;
   const distance=Math.hypot(x+.5-tx,z+.5-tz),score=distance+.015*Math.hypot(tx-wasm.start_x(),tz-wasm.start_y());
   if(distance<=3&&score<best){best=score;cam.x=x+.5;cam.y=z+2.5;cam.shoreObject={id:all[i*12+4],x:tx,z:tz,water:[x,z],distance};}
  }
 }
 if(!cam.shoreObject)throw Error('No actual shore object for reflection proof');
 cam.nativeFraction=.08;
}
updateOriginalCamera(cam,view,width,height,groundAt);wasm.set_eye(...view.eye);
const count=wasm.render(1,cam.x-42,cam.y-48,cam.x+42,cam.y+42,-1,0,0);
const data=new Float32Array(wasm.memory.buffer,wasm.inst_ptr(),count*12).slice();
const ranges=new Uint32Array(wasm.memory.buffer,wasm.ranges_ptr(),512).slice();
const used=new Set(),neededEpochs=new Map();for(let i=0;i<count;i++){
 const id=data[i*12+4],team=Math.min(3,Math.round(data[i*12+5]));used.add(id);
 const set=neededEpochs.get(id)||new Set();set.add(epochs[team]);neededEpochs.set(id,set);
}
const nativeFetch=globalThis.fetch,nativeBitmap=globalThis.createImageBitmap;
globalThis.fetch=async url=>{
 const path=fileURLToPath(url),buffer=await fs.readFile(path);
 return {ok:true,status:200,json:async()=>{
  const value=JSON.parse(buffer);
  if(path.replaceAll('\\','/').endsWith('/models/manifest.json')){
   const latest=new Map();for(const row of value.models)if(used.has(row.id))for(const neededEpoch of neededEpochs.get(row.id)){
    const key=`${row.id}:${neededEpoch}`;if(row.epoch<=neededEpoch&&(!latest.has(key)||latest.get(key).epoch<row.epoch))latest.set(key,row);
   }
   const unique=new Map([...latest.values()].map(row=>[`${row.id}:${row.epoch}:${row.url}`,row]));
   return {...value,models:[...unique.values()].map(row=>({...row,lodFarUrl:undefined}))};
  }
  return value;
 },blob:async()=>({bytes:buffer,url:path})};
};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
try{
 const gl=new RecordingGL(),teams=[[.27,.59,1],[.94,.27,.24],[.31,.86,.39],[.96,.82,.27],[1,1,1],[1,1,1],[1,1,1],[.3,.75,1]];
 const terrain=await loadOriginalTerrain(gl,true,teams),models=await loadOriginalModels(gl,true,teams),ground=await loadOriginalGround(gl,true),nativeWater=await loadOriginalWater(gl,true);
 if(!terrain.enabled||!ground.enabled||!nativeWater.enabled||terrain.errors.length||models.errors.length||ground.errors.length||nativeWater.errors.length)throw Error(JSON.stringify({terrain:terrain.errors,models:models.errors,ground:ground.errors,water:nativeWater.errors}));
 terrain.updateMap({heights,types});nativeWater.updateMap({heights,types});models.updateTerrain(heights);ground.updateTerrain(heights);
 const seasonSession=createSeasonSession(wasm,terrain,ground,models);seasonSession.configure(seasonSettings);
 if(JSON.stringify(seasonSession.update())!==JSON.stringify(seasonPhase))throw Error('Production season session phase differs from actual tick clock');
 const urbanQueries=[];
 const isUrbanAt=(x,z,team,id,index)=>{const urban=!!wasm.ground_urban_at(x,z);urbanQueries.push({x,z,team,id,index,urban});return urban;};
 const tex=(unit,internal,format,w,h,pixels)=>{const t=gl.createTexture();gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,t);gl.texImage2D(gl.TEXTURE_2D,0,internal,w,h,0,format,gl.UNSIGNED_BYTE,pixels);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);return t;};
 const fog=tex(2,gl.R8,gl.RED,256,256,new Uint8Array(wasm.memory.buffer,wasm.fog_ptr(),65536).slice());
 const territoryPtr=wasm.terr_ptr(),territories=tex(3,gl.RGBA,gl.RGBA,256,256,new Uint8Array(wasm.memory.buffer,territoryPtr,65536*4).slice());
 const vao=gl.createVertexArray();gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());
 const vertex=new Float32Array(257**2*6),h=(x,z)=>heights[Math.max(0,Math.min(256,z))*257+Math.max(0,Math.min(256,x))];
 for(let z=0,k=0;z<=256;z++)for(let x=0;x<=256;x++){
  const nx=h(x-1,z)-h(x+1,z),nz=h(x,z-1)-h(x,z+1),length=Math.hypot(nx,2,nz);vertex.set([x,h(x,z),z,nx/length,2/length,nz/length],k);k+=6;
 }
 gl.bufferData(gl.ARRAY_BUFFER,vertex);for(const [loc,offset] of [[0,0],[1,12]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,3,gl.FLOAT,false,24,offset);}
 const indices=new Uint32Array(256**2*6);for(let z=0,k=0;z<256;z++)for(let x=0;x<256;x++){const a=z*257+x;indices.set([a,a+257,a+1,a+1,a+257,a+258],k);k+=6;}
 gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,indices);
 const waterVao=gl.createVertexArray();gl.bindVertexArray(waterVao);gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-400,-400,656,-400,-400,656,656,656]));gl.enableVertexAttribArray(0);gl.vertexAttribPointer(0,2,gl.FLOAT,false,0,0);
 // Resolve deferred team textures before retaining the final production pass.
 models.draw({data,ranges,vp:view.vp,epochs,zoom:cam.zoom,eye:view.eye});
 await new Promise(resolve=>setTimeout(resolve,100));gl.draws=[];
 gl.viewport(0,0,width,height);
 let reflection=null;const reflectionStart=gl.draws.length;
 if(useReflection){
  const renderer=createOriginalReflection(gl,{resolutionScale:1,maxSize:2048});
  reflection=renderer.render({vp:view.vp,eye:view.eye,width,height,waterHeight:0,drawScene:pass=>{
   terrain.drawTerrain({vao,px:1/cam.zoom,fogTexture:fog,territoryTexture:territories,...pass});
   models.draw({data,ranges,epochs,zoom:cam.zoom,...pass});
  }});
 }
 const reflectionDraws=gl.draws.length-reflectionStart;
 gl.disable(gl.BLEND);gl.depthMask(true);
 terrain.drawTerrain({vao,vp:view.vp,px:1/cam.zoom,fogTexture:fog,territoryTexture:territories});
 gl.enable(gl.BLEND);gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
 ground.draw({data,ranges,vp:view.vp,epochs,isUrbanAt});
 gl.depthMask(false);gl.enable(gl.STENCIL_TEST);gl.stencilFunc(gl.EQUAL,0,255);gl.stencilOp(gl.KEEP,gl.KEEP,gl.INCR);
 models.draw({data,ranges,vp:view.vp,epochs,zoom:cam.zoom,eye:view.eye,shadow:true});
 gl.disable(gl.STENCIL_TEST);gl.depthMask(true);
 models.draw({data,ranges,vp:view.vp,epochs,zoom:cam.zoom,eye:view.eye});
 gl.depthMask(false);nativeWater.drawWater({vp:view.vp,time:12,fogTexture:fog,reflectionTexture:reflection?.texture,reflectionVP:reflection?.vp??view.vp});
 const recorded=gl.payload();
 const shaderHashes=Object.fromEntries(Object.entries(recorded.programs).map(([id,program])=>[id,{vertexSha256:sha(program.vertex),fragmentSha256:sha(program.fragment)}]));
 const payload={format:'openepoch-production-world-capture-v1',width,height,epoch:epoch+1,scene,cam,eye:view.eye,instances:count,modelBindings:models.manifest.models.length,epochs,rulesProfile,
  season:{requestedMonthPosition,actualSimulationTick:simulationTick,tickSource:typeof wasm.game_tick==='function'?'WASM game_tick':'WASM UI snapshot',steppedTicks:steps,...seasonPhase},
  ground:{enabled:ground.enabled,policy:ground.policy,entries:ground.entries.size,metrics:{...ground.metrics},urbanSource:'WASM ground_urban_at; active source-backed Town Center city bands',urbanQueries},
  sourceHashes:{wasmSha256:sha(bytes),captureScriptSha256,files:sourceFiles,shaders:shaderHashes},
  reflection:{enabled:useReflection,draws:reflectionDraws,width:reflection?.width,height:reflection?.height,vp:reflection?Array.from(reflection.vp):undefined,eye:reflection?.eye,clipPlane:useReflection?[0,1,0,0]:undefined,texture:reflection?.texture.id,policy:'Planar world-y0 reflection, terrain and object clip; framebuffer resolution and shader discard are WebGL adapters.'},
  modelSeason:{available:typeof models.setSeasonPhase==='function',metrics:{...models.metrics}},
  nativeWater:{enabled:nativeWater.enabled,metrics:{...nativeWater.metrics},material:'Source Skin1/2/4 product/lerp vertex channels; separate reflection alpha0.32 overlay',adapters:nativeWater.manifest.adapter},
  farmFixture,
  scope:'WASM seed7 world, production simulation-tick calendar, camera/loaders/material, epoch/urban/seasonal building ground, terrain-conforming stencil shadows and terrain/water draw calls. World-only native GLES replay; no HUD, browser, native executable or original-game visual equality claim.',...recorded};
 await fs.writeFile(destination,JSON.stringify(payload));
 console.log(JSON.stringify({output:fileURLToPath(destination),epoch:epoch+1,scene,instances:count,modelBindings:payload.modelBindings,draws:payload.draws.length,textureCount:Object.keys(payload.textures).length,season:payload.season,ground:payload.ground.metrics,urbanQueries:urbanQueries.length,wasmSha256:payload.sourceHashes.wasmSha256}));
}finally{globalThis.fetch=nativeFetch;globalThis.createImageBitmap=nativeBitmap;}
