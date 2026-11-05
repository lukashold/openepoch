// Execute the production Main frame with actual render metadata and cloth WASM.
// GL geometry calls are substituted; native GPU pixels are checked separately.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {loadSourceSailRuntime} from '../docs/source-sail-runtime.js';
import {createFrontendNamespace} from '../docs/render-namespace.js';
import {createRenderAssetMapping} from '../docs/original-render-namespace.js';
import {createPackRenderMetadata} from '../docs/original-pack-states.js';
import {createRenderActorMetadata} from '../docs/render-actor-metadata.js';
import {createNativeLightingContext} from '../docs/original-lighting.js';
import {buildOrdinarySailFixture} from './ordinary-sail-fixture.mjs';
const option=name=>process.argv.find(v=>v.startsWith('--'+name+'='))?.slice(name.length+3);
const target=(name,fallback)=>option(name)?pathToFileURL(path.resolve(option(name))):new URL(fallback,import.meta.url);
const mainURL=target('main','../docs/main.js'),wasmURL=target('wasm','../docs/openepoch-ee2-up15.wasm');
const mainBytes=await fs.readFile(mainURL),wasmBytes=await fs.readFile(wasmURL),main=mainBytes.toString();
const start=main.indexOf('function frame(now) {'),end=main.indexOf('// ---------------------------------------------------------------- HUD',start);
assert.ok(start>=0&&end>start);const section=main.slice(start,end);
const w=(await WebAssembly.instantiate(wasmBytes,{})).instance.exports;
assert.equal(typeof w.render_meta_version,'function','This frame proof requires the metadata feature');
buildOrdinarySailFixture(w,{epoch:5});
const namespace=createFrontendNamespace(w,{referenceMode:true,assetMapping:createRenderAssetMapping(w)});
const packMetadata=createPackRenderMetadata(w),packActorReader=createRenderActorMetadata(w);
const manifest=JSON.parse(await fs.readFile(new URL('../docs/local-original/sails-native/manifest.json',import.meta.url)));
const source={key:'actual-caravel',planes:manifest.effects.CaravelSail.sails.map(p=>({...p.grid,pinned:p.pinned}))};
const lighting=createNativeLightingContext(JSON.parse(await fs.readFile(new URL('../docs/local-original/climate-lighting.json',import.meta.url))));
let currentMetadata=null,currentSeed=null,expectClothPrepared=true;const calls=[],draws=[];
const models={lighting,available:new Set(Array.from({length:namespace.slotCount},(_,i)=>i)),
 resolveSailSource:actor=>actor.def===58?source:null,
 resolveSailForces:()=>({gravity:[0,0,-32.174],windMin:[0,0,0],windMax:[1,0,0]}),
 prepareDynamicSails({controller,metadata}){
  currentMetadata=metadata;const actor=metadata.actors.find(a=>a.def===58);
  if(actor)currentSeed=controller.get(actor.handle)?.parts[0].cloth.seed;
  calls.push({kind:'prepare',metadata,seed:currentSeed});
 },disposeDynamicSails(){},
 draw(options){if(expectClothPrepared)assert.equal(options.actorMetadata,currentMetadata,'Cloth owner and every scene pass retain the exact same enriched actor identity');draws.push(options);calls.push({kind:'draw',seed:currentSeed});},
};
const runtime=await loadSourceSailRuntime({wasm:w,namespace,models,enabled:true,decorateMetadata:metadata=>packMetadata?packMetadata.snapshot(metadata):metadata,fetcher:async url=>new Response(await fs.readFile(url))});
assert.equal(runtime.metrics.enabled,true,runtime.metrics.error);
const sourceSails={metrics:runtime.metrics,advance:options=>{calls.push({kind:'advance',...options});return runtime.advance(options);}};
const noop=()=>{},program={p:{},u:new Proxy({},{get:()=>null})};
const gl=new Proxy({getContextAttributes:()=>({stencil:true})},{get:(o,k)=>o[k]??noop});
const canvas={width:0,height:0,dataset:{}},quality={value:'2'};
const context=vm.createContext({console,Float32Array,Uint32Array,Set,Math,
 running:true,worldMaterialPending:null,initialMusicEpoch:0,musicCivilization:0,paused:false,net:null,speed:2,last:1000,frameMs:16.7,worstMs:0,bench:0,ui:{epoch:5},simMs:0,rulesProfile:{ruleset:w.ruleset()},
 sfx:{music:noop,menuMusic:noop,play:noop},cam:{x:100,y:100,zoom:40},keys:new Set(),referenceMode:true,ghost:null,mouse:{inside:false},
 updateOriginalCamera:noop,rotateOriginalCameraHeading:noop,updateCamera:noop,panCameraScreen:noop,advance:noop,TICK:.05,acc:0,
 lastAuto:10000,lastUi:10000,lastMini:10000,lastWorkSound:10000,saveGame:noop,devicePixelRatio:1,innerWidth:1280,innerHeight:720,
 $:()=>quality,canvas,gl,wasm:w,MAP:256,STRIDE:12,renderNamespace:namespace,sourceSails,packMetadata,packActorReader,
 f32:(ptr,n)=>new Float32Array(w.memory.buffer,ptr,n),u8:(ptr,n)=>new Uint8Array(w.memory.buffer,ptr,n),
 bindTexture:noop,mapTexture:{},territoryTexture:{},fogTexture:{},uploadOriginalTerrainClasses:noop,fogTick:-1,
 seasonSession:{update:()=>({})},qaMode:true,view:{vp:new Float32Array(16),eye:[0,100,0],u:[0,1,0]},
 viewRect:()=>[0,0,256,256],ghostTile:()=>[0,0],updatePlacement:noop,
 originalModels:models,originalTerrain:{enabled:true,drawTerrain:noop,drawWater:noop},reflectionError:null,
 originalReflection:{metrics:{},render:({drawScene,vp,eye})=>{drawScene({vp,eye,capture:true,clipPlane:[0,1,0,-.05]});return {texture:{},vp};}},
 originalGround:{draw:noop,metrics:{instances:0}},originalWater:{enabled:true,drawWater:noop,metrics:{}},
 originalAssets:{enabled:true},originalAssetsTexture:{},vaoT:{},vaoW:{},vaoM:{},vaoQ:{},instBuf:{},
 mesh:{count:[],first:[]},instOffset:noop,terrain:program,water:program,models:program,quads:program,
 updateUi:noop,drawMini:noop});
vm.runInContext(section+'\nglobalThis.runFrame=frame;',context);
let checks=0;const eq=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;},good=(v,note)=>{assert.ok(v,note);checks++;};
const frame=now=>{calls.length=0;draws.length=0;context.runFrame(now);};
const hash=w.state_hash(),p=w.save(),saved=new Uint8Array(w.memory.buffer,p,w.save_len()).slice();
frame(1016);
eq(calls.map(c=>c.kind),['advance','prepare','draw','draw','draw'],'Exactly one solver advance/upload precedes reflection, shadow and main');
eq(calls[0].deltaSeconds,.032);eq(calls[0].runPhysics,true);
eq(draws.map(d=>[!!d.capture,!!d.shadow]),[[true,false],[false,true],[false,false]]);
eq(draws.every(d=>d.actorMetadata===draws[0].actorMetadata),true);eq(new Set(calls.filter(c=>c.kind==='draw').map(c=>c.seed)).size,1,'Draw passes cannot change source RNG');
const movingSeed=currentSeed;context.paused=true;frame(1032);eq(calls[0].runPhysics,false);eq(currentSeed,movingSeed);
context.paused=false;context.net={ls:{waiting:true}};frame(1048);eq(calls[0].deltaSeconds,.016);eq(calls[0].runPhysics,false);eq(currentSeed,movingSeed);
context.net.ls.waiting=false;frame(1064);eq(calls[0].runPhysics,true);good(currentSeed!==movingSeed);
context.net=null;context.running=false;frame(1080);eq(calls.length,0,'A menu frame does not create or advance source cloth');
context.running=true;context.viewRect=()=>[-1000,-1000,-999,-999];const lastSeed=currentSeed;frame(1096);
eq(calls.filter(c=>c.kind==='draw').length,0);eq(currentMetadata.count,0);eq(runtime.metrics.owners,1);eq(currentSeed,lastSeed,'A fully culled but live owner retains its state');
eq(w.state_hash(),hash);const after=w.save();eq(new Uint8Array(w.memory.buffer,after,w.save_len()),saved,'Presentation frame execution leaves the simulation unchanged');
// Pack state and cloth must share one post-render snapshot in every scene pass.
const packFrames=[];context.viewRect=()=>[0,0,256,256];context.net=null;context.paused=false;
for(const id of [65,68,76,78]){
 const state=await fs.readFile(new URL(`./fixtures/civ8-produced/civ8-produced-${id}.oes`,import.meta.url));
 const ptr=w.load_alloc(state.length);new Uint8Array(w.memory.buffer,ptr,state.length).set(state);eq(w.load_apply(),1);
 good(w.cmd_pack(1)>0);const beforeHash=w.state_hash(),savePtr=w.save(),beforeSave=new Uint8Array(w.memory.buffer,savePtr,w.save_len()).slice();
 frame(2000+id*32);eq(draws.length,3);good(draws.every(d=>d.actorMetadata===draws[0].actorMetadata));
 const actor=draws[0].actorMetadata.actors.find(a=>a.def===id);good(actor?.pack?.capable);eq(actor.pack.phase,2);eq(actor.pack.duration,1);
 eq(w.state_hash(),beforeHash);const afterPtr=w.save();eq(new Uint8Array(w.memory.buffer,afterPtr,w.save_len()),beforeSave,'Real Main pack/cloth frame preserves the running task');
 packFrames.push({id,handle:actor.handle,phase:actor.pack.phase,frame:draws[0].actorMetadata.frame,session:draws[0].actorMetadata.session});
}
expectClothPrepared=false;context.sourceSails=null;
const fallbackHash=w.state_hash(),fallbackPtr=w.save(),fallbackSave=new Uint8Array(w.memory.buffer,fallbackPtr,w.save_len()).slice();
frame(7000);eq(calls.map(c=>c.kind),['draw','draw','draw'],'A missing cloth backend does not disable Pack presentation');
eq(draws.every(d=>d.actorMetadata===draws[0].actorMetadata),true);eq(draws[0].actorMetadata.actors.find(a=>a.def===78)?.pack.phase,2);
eq(w.state_hash(),fallbackHash);const fallbackAfter=w.save();eq(new Uint8Array(w.memory.buffer,fallbackAfter,w.save_len()),fallbackSave);
const sha=x=>createHash('sha256').update(x).digest('hex');
const report={format:'native-joint8-sail-pack-main-frame-tests-v1',status:'passed',checks,packFrames,mainSha256:sha(mainBytes),wasmSha256:sha(wasmBytes),runtimeSha256:sha(await fs.readFile(new URL('../docs/source-sail-runtime.js',import.meta.url))),
 scope:'Executes actual Main frame, actual ordinary ship production, sorted actor ABI and own visual WASM; substitutes geometry/GL calls. Native GPU checks remain separate.'};
if(option('report'))await fs.writeFile(path.resolve(option('report')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
