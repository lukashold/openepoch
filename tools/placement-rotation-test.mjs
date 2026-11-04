// Independent API regressions of the real WASM placement/rotation commands.
// Map fixtures isolate terrain, exploration and builder/path validity. Eight
// 45-degree choices are an extra adapter option; original CTRL input rotates 90
// degrees according to the executable audit. Square tile BBoxes,
// selected-worker reachability are reconstruction adapters. Completed-building
// TC/Fortress claim requirements are independently proven original rules.
// Run after Import-EE2-Assets.ps1 on a local EE2 installation: the native NIF and
// Ground integration checks intentionally require its local-original outputs.
// Current verification uses docs/*.wasm and shipped own/source-data fixtures;
// no work/ snapshot or machine-specific historic WASM is a test dependency.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {Lockstep} from '../docs/net.js';
import {groundPlacement} from '../docs/original-ground.js';
import {loadOriginalModels} from '../docs/original-renderer.js';
import {assertCurrentSave,revealFixture,saveLayout} from './save-fixture.mjs';

const docs=new URL('../docs/',import.meta.url),reports=new URL('../../EE2-Research/analysis/',import.meta.url);
const blobs={ee2:readFileSync(new URL('openepoch-ee2.wasm',docs)),base:readFileSync(new URL('openepoch.wasm',docs))};
const source=JSON.parse(readFileSync(new URL('ee2-units.json',docs)));
const visualFixture=JSON.parse(readFileSync(new URL('original-graphics-fixture.json',docs)));
const groundManifest=JSON.parse(readFileSync(new URL('local-original/ground-decals.json',docs)));
const sizes=new Map(visualFixture.definitions.filter(d=>d.building).map(d=>[d.id,d.size]));
const N=256,NN=N*N,NONE=0xffffffff,C_PLACE=10,C_ROTATED=16;
const sha=b=>createHash('sha256').update(b).digest('hex');
let checks=0;const completed=[];
const check=Object.fromEntries(['ok','equal','notEqual','deepEqual'].map(k=>[k,(...a)=>{checks++;assert[k](...a);} ]));
const report={format:'placement-rotation-tests-v1',status:'running',scope:'Real WASM town-center expansion with native completed-city/adjacent acquisition, eight45-degree orientations as an explicit extra option (original CTRL rotates90), conservative square-BBox/builder-reachability adapters, actual citizen movement/construction, rendering/source costs, strict authenticated bytes, save compatibility and two-peer lockstep. No claim of complete original placement/pathfinding equivalence.',wasmSha256:sha(blobs.ee2),baseWasmSha256:sha(blobs.base),sourceAuditSha256:sha(readFileSync(new URL('ee2-units.json',docs))),checks:0,completed};
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
function layout(b){const l=saveLayout(b),d=l.d,psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),ents=36+4*psz,terrain=ents+high*esz;return {...l,psz,esz,high,ents,terrain,res:terrain+NN,amt:terrain+2*NN,eco:l.end+132};}
function entities(b){const l=layout(b),out=[];check.equal(l.esz,148,'unchanged existing Ent layout');for(let i=0;i<l.high;i++){const at=l.ents+i*l.esz;if(!b[at])continue;const gen=l.d.getUint16(at+4,true);out.push({i,at,handle:((gen<<16)|i)>>>0,id:b[at+1],team:b[at+2],x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),rot:l.d.getFloat32(at+32,true),prot:l.d.getFloat32(at+36,true),hp:l.d.getFloat32(at+40,true),order:b[at+48],target:l.d.getUint32(at+60,true),built:l.d.getFloat32(at+80,true)});}return out;}
const own=(w,team,id)=>entities(save(w)).filter(e=>e.team===team&&(id===undefined||e.id===id));
const find=(w,handle)=>entities(save(w)).find(e=>e.handle===handle);
const stocks=(w,team=0)=>Array.from({length:w.resource_count()},(_,s)=>w.resource_stat(team,s));
const select=(w,e)=>{w.select_none();w.select_point(e.x,e.y,0,0,0,0,N,N);};
const near=(a,b,text,eps=1e-5)=>check.ok(Math.abs(a-b)<eps,`${text}: ${a} versus ${b}`);
const bbox=(id,turn)=>Math.ceil(sizes.get(id)*(turn%2?Math.SQRT2:1));
function wire(kind,a,b,c,handles=[]){const out=new Uint8Array(14+handles.length*4),d=new DataView(out.buffer);out[0]=kind;out[1]=handles.length;d.setFloat32(2,a,true);d.setFloat32(6,b,true);d.setUint32(10,c>>>0,true);handles.forEach((h,i)=>d.setUint32(14+i*4,h,true));return out;}
function deliver(w,team,packet){const p=w.inbox_alloc(packet.length);new Uint8Array(w.memory.buffer,p,packet.length).set(packet);w.net_apply(team);}
function invalid(w,operation,label){const before=save(w),state=w.state_hash(),bank=stocks(w);check.equal(operation(),0,label);check.equal(w.state_hash(),state,label+' state hash unchanged');check.deepEqual(stocks(w),bank,label+' pays nothing');check.deepEqual(save(w),before,label+' save bytes unchanged');check.equal(w.outbox_len(),0,label+' queues no bytes');}
function invalidWire(w,team,packet,label){const before=w.state_hash(),bank=stocks(w),actors=entities(save(w));deliver(w,team,packet);check.equal(w.state_hash(),before,label+' rejected');check.deepEqual(stocks(w),bank,label+' pays nothing');check.deepEqual(entities(save(w)),actors,label+' creates/mutates no actors');}
function setStock(bytes,team,value){const l=layout(bytes);for(let s=0;s<(l.reference?9:3);s++)l.d.setFloat32(s<3?36+team*l.psz+s*4:l.eco+8+(team*6+s-3)*4,value,true);}
function editTerrain(bytes,x0,y0,x1,y1,type){const l=layout(bytes);for(let y=y0;y<=y1;y++)for(let x=x0;x<=x1;x++){bytes[l.terrain+y*N+x]=type;bytes[l.res+y*N+x]=0;l.d.setUint16(l.amt+(y*N+x)*2,0,true);}}
function move(bytes,e,x,y){const l=layout(bytes);for(const [at,v]of [[8,x],[12,y],[16,x],[20,y],[24,0],[28,0]])l.d.setFloat32(e.at+at,v,true);bytes[e.at+48]=0;l.d.setUint32(e.at+60,NONE,true);l.d.setUint32(e.at+68,NONE,true);}
async function make({profile='ee2',mode=0,reveal=true,clear=true,seed=7}={}){
 const w=(await WebAssembly.instantiate(blobs[profile],{})).instance.exports;
 for(const name of ['footprint_size','place_error_rotated','cmd_place_rotated','render_rotated','dock_orientation'])check.equal(typeof w[name],'function','new placement ABI '+name);
 w.init(seed,1,0,1,0);check.equal(w.skirmish_visibility(mode),1);
 const b=save(w),l=layout(b);assertCurrentSave(b);for(let t=0;t<4;t++){b[36+t*l.psz+27]=0;setStock(b,t,10000);}
 if(clear){b.fill(3,l.terrain,l.terrain+NN);b.fill(0,l.res,l.res+NN);b.fill(0,l.amt,l.amt+2*NN);}
 if(reveal)revealFixture(b);check.equal(load(w,b),1);return w;
}
function spatialSite(w,id,owner,{turn=0,home=own(w,0,0)[0],avoid=[],legal=true,adjacent=false}={}){
 const s=bbox(id,turn),cx=Math.floor(home.x),cy=Math.floor(home.y);
 for(let r=5;r<220;r++)for(let k=-r;k<=r;k++)for(const [x,y]of [[cx+k,cy-r],[cx+k,cy+r],[cx-r,cy+k],[cx+r,cy+k]]){
  if(x<3||y<3||x+s>253||y+s>253)continue;
  const region=w.territory_at(x+s/2,y+s/2);if(w.territory_owner(region)!==owner||avoid.includes(region)||(adjacent&&w.territory_claim_stat(0,region,1)!==1))continue;
  if(legal&&w.place_error_rotated(id,x,y,turn)!==0)continue;
  if(!legal){const b=save(w),l=layout(b);if(entities(b).some(e=>sizes.has(e.id)&&Math.abs(e.x-x-s/2)<sizes.get(e.id)/2+s/2+.5&&Math.abs(e.y-y-s/2)<sizes.get(e.id)/2+s/2+.5))continue;}
  return {x,y,s,region,cx:x+s/2,cy:y+s/2};
 }
 assert.fail(`no ${legal?'legal':'spatial'} site id${id}/owner${owner}/turn${turn}`);
}
function placed(w,id,at){return own(w,0,id).find(e=>Math.abs(e.x-at.cx)<.01&&Math.abs(e.y-at.cy)<.01);}
function rendered(w,id,x,y,{turn=0,ghost=false}={}){
 const n=w.render_rotated(1,0,0,256,256,ghost?id:-1,Math.floor(x),Math.floor(y),turn),data=new Float32Array(w.memory.buffer,w.inst_ptr(),n*12);
 for(let i=0;i<n;i++){const row=Array.from(data.subarray(i*12,i*12+12));if(row[4]===id&&(ghost?!!(Math.round(row[7])&24):Math.abs(row[0]-x)<.01&&Math.abs(row[1]-y)<.01))return row;}
 assert.fail(`missing rendered ${ghost?'ghost':'building'} ${id}`);
}

// Records the real native-NIF loader's draw payload, without pretending to
// execute a GPU shader. Geometry is the installed/exported Warehouse NIF; image
// decoding alone is stubbed because pixel fidelity has its own source tests.
async function nativeDrawProbe(){
 const manifest=JSON.parse(readFileSync(new URL('local-original/models/manifest.json',docs)));
 const entry=manifest.models.find(m=>m.id===52&&m.epoch===0);check.ok(entry);
 const native=JSON.parse(readFileSync(new URL(entry.url,docs)));check.ok(native.normalization&&native.parts.length);
 let next=1;const gl={draws:[],unit:0,vao:null,array:null,uniforms:new Map()};
 const constants=['ARRAY_BUFFER','ELEMENT_ARRAY_BUFFER','STATIC_DRAW','STREAM_DRAW','FLOAT','UNSIGNED_INT','UNSIGNED_BYTE','TRIANGLES',
  'TEXTURE_2D','TEXTURE_2D_ARRAY','RGBA','RGBA32F','RGBA8','VERTEX_SHADER','FRAGMENT_SHADER','COMPILE_STATUS','LINK_STATUS',
  'TEXTURE_MIN_FILTER','TEXTURE_MAG_FILTER','LINEAR_MIPMAP_LINEAR','LINEAR','NEAREST','TEXTURE_WRAP_S','TEXTURE_WRAP_T','REPEAT','UNPACK_FLIP_Y_WEBGL',
  'BLEND','DEPTH_TEST','DEPTH_WRITEMASK','DEPTH_FUNC','BLEND_SRC_RGB','BLEND_DST_RGB','BLEND_SRC_ALPHA','BLEND_DST_ALPHA',
  'ALWAYS','LESS','EQUAL','LEQUAL','GREATER','NOTEQUAL','GEQUAL','NEVER','ONE','ZERO','SRC_ALPHA','ONE_MINUS_SRC_ALPHA'];
 constants.forEach((key,i)=>gl[key]=100+i);for(let i=0;i<8;i++)gl['TEXTURE'+i]=1000+i;
 gl.createProgram=gl.createShader=gl.createTexture=gl.createBuffer=()=>({id:next++});
 gl.createVertexArray=()=>({id:next++,attributes:new Map()});
 gl.bindVertexArray=vao=>gl.vao=vao;
 gl.bindBuffer=(target,buffer)=>{if(target===gl.ARRAY_BUFFER)gl.array=buffer;else if(gl.vao)gl.vao.element=buffer;else gl.defaultElement=buffer;};
 gl.bufferData=(target,data)=>{const buffer=target===gl.ARRAY_BUFFER?gl.array:gl.vao?.element||gl.defaultElement;check.ok(buffer);buffer.data=new data.constructor(data);};
 gl.vertexAttribPointer=(loc,size,type,normalized,stride,offset)=>gl.vao.attributes.set(loc,{buffer:gl.array,stride,offset});
 gl.getUniformLocation=(p,name)=>name;gl.uniform1i=gl.uniform1f=(name,value)=>gl.uniforms.set(name,value);
 gl.uniform3fv=gl.uniform4fv=(name,value)=>gl.uniforms.set(name,Array.from(value));
 gl.uniformMatrix4fv=(name,transpose,value)=>gl.uniforms.set(name,Array.from(value));
 gl.getShaderParameter=gl.getProgramParameter=()=>true;gl.getExtension=()=>null;
 const capabilities=new Set(),state=new Map([[gl.DEPTH_WRITEMASK,true],[gl.DEPTH_FUNC,gl.LEQUAL],
  [gl.BLEND_SRC_RGB,gl.ONE],[gl.BLEND_DST_RGB,gl.ZERO],[gl.BLEND_SRC_ALPHA,gl.ONE],[gl.BLEND_DST_ALPHA,gl.ZERO]]);
 gl.isEnabled=cap=>capabilities.has(cap);gl.enable=cap=>capabilities.add(cap);gl.disable=cap=>capabilities.delete(cap);gl.getParameter=key=>state.get(key);
 gl.depthMask=value=>state.set(gl.DEPTH_WRITEMASK,value);gl.depthFunc=value=>state.set(gl.DEPTH_FUNC,value);
 gl.blendFuncSeparate=(src,dst,srcAlpha,dstAlpha)=>{state.set(gl.BLEND_SRC_RGB,src);state.set(gl.BLEND_DST_RGB,dst);state.set(gl.BLEND_SRC_ALPHA,srcAlpha);state.set(gl.BLEND_DST_ALPHA,dstAlpha);};
 gl.blendFunc=(src,dst)=>gl.blendFuncSeparate(src,dst,src,dst);
 gl.activeTexture=unit=>gl.unit=unit;
 for(const method of ['shaderSource','compileShader','attachShader','deleteShader','linkProgram','useProgram','enableVertexAttribArray',
  'vertexAttribDivisor','bindTexture','texImage2D','texStorage3D','texSubImage3D','texParameteri','generateMipmap','pixelStorei'])gl[method]=()=>{};
 gl.drawElementsInstanced=(mode,count,type,offset,instances)=>{check.ok(count>0);gl.draws.push({count,instances,data:new Float32Array(gl.vao.attributes.get(3).buffer.data)});};
 const oldFetch=globalThis.fetch,oldBitmap=globalThis.createImageBitmap;
 try{
  globalThis.fetch=async input=>{
   const path=decodeURIComponent(new URL(input).pathname).split('/docs/')[1];
   return {ok:true,status:200,async json(){if(path==='local-original/models/manifest.json')return {models:[entry]};if(path==='local-original/team-textures/manifest.json')return {textures:{}};check.equal(path,entry.url);return native;},async blob(){return {url:path};}};
  };
  globalThis.createImageBitmap=async()=>({width:1,height:1,close(){}});
  const renderer=await loadOriginalModels(gl,true,Array.from({length:8},()=>[1,1,1]));check.deepEqual(renderer.errors,[]);check.equal(renderer.models.get(52)[0].parts.length,native.parts.length);
  return row=>{
   gl.draws.length=0;const ranges=new Uint32Array(512);ranges[52*2+1]=1;
   renderer.draw({data:new Float32Array(row),ranges,vp:new Float32Array(16),epochs:[0,0,0,0],zoom:80});
   check.equal(gl.draws.length,native.parts.length,'all genuine NIF parts reach the draw API');
   for(const draw of gl.draws){check.equal(draw.instances,1);near(draw.data[3],row[3],'native NIF renderer receives actual WASM building orientation');near(draw.data[8],row[8],'native NIF normalization gets unrotated instance size');}
  };
 }finally{globalThis.fetch=oldFetch;globalThis.createImageBitmap=oldBitmap;}
}
let probeNativeDraw;

async function footprintChecks(){
 const w=await make();
 for(const [id,size]of sizes)for(let turn=0;turn<8;turn++)check.equal(w.footprint_size(id,turn),bbox(id,turn),`square tile-BBox adapter ${id}, turn${turn}`);
 for(const id of [8,57,256,NONE])for(const turn of [0,1,7])check.equal(w.footprint_size(id,turn),0,'nonbuilding/invalid ID cannot alias a building');
 for(const turn of [8,255,NONE,-1])check.equal(w.footprint_size(1,turn),0,'rotation outside0..7 rejected');
 check.equal(w.footprint_size(1,0),2);check.equal(w.footprint_size(1,1),3);check.equal(w.footprint_size(0,1),6);
 completed.push('eight45-degree choices; positive/square odd-angle tile BBoxes; invalid IDs/turns');
}

async function expansionChecks(){
 const w=await make(),worker=own(w,0,8)[0],home=own(w,0,0)[0];select(w,worker);
 const neutral=spatialSite(w,0,255,{adjacent:true}),before=stocks(w),cost=Array.from({length:9},(_,s)=>w.unit_cost_stat(0,0,s));
 const native=source.class_rows.find(r=>r.id===0&&r.epoch===0);check.equal(native.fields.cost9.verified,true);check.deepEqual(cost,native.fields.cost9.value,'native nine-slot TC cost');
 check.equal(w.cmd_place_rotated(0,neutral.x,neutral.y,0),1,'selected reachable citizen starts a neutral TC');
 const tc=placed(w,0,neutral);check.ok(tc);check.equal(tc.built,0);check.equal(w.territory_owner(neutral.region),255,'Native minimum excludes a city-center foundation');check.equal(w.territory_claim_stat(0,neutral.region,0),2);
 check.deepEqual(stocks(w),before.map((v,s)=>v-cost[s]),'native cost paid exactly once');check.equal(find(w,worker.handle).order,5);check.equal(find(w,worker.handle).target,tc.handle);
 run(w,150);const moving=find(w,worker.handle);check.ok(Math.hypot(moving.x-worker.x,moving.y-worker.y)>1,'citizen actually walks toward remote construction');
 run(w,2300);check.equal(find(w,tc.handle).built,1,'walk and actual source buildtime finish the TC without cheats');check.equal(w.territory_owner(neutral.region),0);check.equal(own(w,0,0).length,2);
 const round=save(w),state=w.state_hash();run(w,60);check.equal(load(w,round),1);check.equal(w.state_hash(),state);check.equal(w.territory_owner(neutral.region),0,'new claim survives reload');
 const ownSite=spatialSite(w,0,0,{legal:false,home});check.equal(w.place_error_rotated(0,ownSite.x,ownSite.y,0),5,'native one-TC-per-province rule');invalid(w,()=>w.cmd_place_rotated(0,ownSite.x,ownSite.y,0),'second TC in same province');
 const foreign=spatialSite(w,0,1,{legal:false});check.equal(w.place_error_rotated(0,foreign.x,foreign.y,0),2);invalid(w,()=>w.cmd_place_rotated(0,foreign.x,foreign.y,0),'foreign province TC');
 completed.push('neutral TC claim, native9-slot cost, actual citizen movement/source-time build, persistent ownership, same-province and foreign limits');
}

async function naturalStartChecks(){
 const w=(await WebAssembly.instantiate(blobs.ee2,{})).instance.exports;
 w.init(1,1,0,1,0);check.equal(w.skirmish_setup(500,1),1);check.equal(w.skirmish_visibility(2),1);check.equal(w.stat(0,0),0);
 const workerHandle=w.select_idle_worker();check.notEqual(workerHandle,NONE);const worker=own(w,0,8).find(e=>e.i===workerHandle);check.ok(worker);
 const at={x:65,y:61,cx:67,cy:63,s:4,region:6};check.equal(w.territory_at(at.cx,at.cy),6);check.equal(w.territory_owner(6),255);
 const before=save(w),l=layout(before);check.equal(before[l.realFog+at.y*N+at.x],0,'natural QA site starts outside genuine exploration');
 check.equal(w.place_error_rotated(0,at.x,at.y,0),0,'untouched seed1/start-age/one-AI/high-supplies/revealed start permits reachable neutral expansion');
 check.equal(w.territory_claim_stat(0,6,1),1,'Original natural expansion site is adjacent to owned land');check.equal(w.cmd_place_rotated(0,at.x,at.y,0),1);const tc=placed(w,0,at);check.ok(tc);check.equal(w.territory_owner(6),255,'Natural foundation does not prematurely acquire territory');
 run(w,150);const moved=find(w,worker.handle);check.ok(Math.hypot(moved.x-worker.x,moved.y-worker.y)>1,'real start citizen traverses the generated map toward the site');
 run(w,1900);check.equal(find(w,tc.handle).built,1,'natural expansion completes without resource/map/state mutation or cheats');
 check.equal(w.territory_owner(6),0,'Completed natural adjacent city acquires territory');
 check.equal(w.cheat_status(0),0);check.equal(w.cheat_status(1),0);
 report.naturalBrowserFixture={seed:1,ai:1,startEpoch:0,supplies:'high',visibility:2,footprintTopLeft:[65,61],center:[67,63],territory:6,mutation:'none; actual public start/placement/tick APIs only',completedWithoutCheats:true};
 completed.push('untouched generated seed1: reachable neutral TC completes from actual start citizen, native costs and mode2, without fixture/map edits or cheats');
}

async function validationChecks(){
 const w=await make(),worker=own(w,0,8)[0];select(w,worker);const at=spatialSite(w,1,0),control=save(w);
 w.select_none();check.equal(w.place_error_rotated(1,at.x,at.y,0),6);invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'empty builder selection');
 select(w,own(w,0,0)[0]);invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'selected building is not a builder');check.equal(load(w,control),1);select(w,worker);
 for(const turn of [8,255,NONE,-1])invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,turn),'invalid turn '+turn);
 for(const id of [8,57,256,NONE])invalid(w,()=>w.cmd_place_rotated(id,at.x,at.y,0),'invalid definition '+id);
 for(const [x,y]of [[-1,at.y],[at.x,-1],[255,at.y],[at.x,255],[-2147483648,at.y]])invalid(w,()=>w.cmd_place_rotated(1,x,y,0),'outside map');
 let b=control.slice(),l=layout(b);setStock(b,0,0);check.equal(load(w,b),1);select(w,worker);check.equal(w.place_error_rotated(1,at.x,at.y,0),4);invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'unaffordable building');
 b=control.slice();l=layout(b);b[l.res+at.y*N+at.x]=2;l.d.setUint16(l.amt+(at.y*N+at.x)*2,100,true);check.equal(load(w,b),1);select(w,worker);check.equal(w.place_error_rotated(1,at.x,at.y,0),1);invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'resource blocks footprint');
 b=control.slice();editTerrain(b,at.x,at.y,at.x+1,at.y+1,0);check.equal(load(w,b),1);select(w,worker);check.equal(w.place_error_rotated(1,at.x,at.y,0),1);invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'water blocks land building');
 // Terrain and natural fog are individually valid. Only a water ring cuts the
 // selected citizen off from the unchanged passable interior footprint.
 b=control.slice();for(let y=at.y-2;y<=at.y+3;y++)for(let x=at.x-2;x<=at.x+3;x++)if(x<at.x||x>at.x+1||y<at.y||y>at.y+1)editTerrain(b,x,y,x,y,0);
 check.equal(load(w,b),1);select(w,worker);check.equal(w.place_error_rotated(1,at.x,at.y,0),7,'passable island is unreachable by selected land citizen');invalid(w,()=>w.cmd_place_rotated(1,at.x,at.y,0),'unreachable land path');
 completed.push('no/wrong builder, invalid definitions/turns/map coordinates, exact-cost failure, deposits/water and disconnected selected-worker land path');
}

async function visibilityChecks(){
 const known=await make(),worker=own(known,0,8)[0];select(known,worker);const at=spatialSite(known,0,255);
 for(const mode of [0,1,2]){
  const w=await make({mode,reveal:false}),builder=own(w,0,8)[0];select(w,builder);
  const natural=save(w),l=layout(natural),target=at.y*N+at.x;check.equal(natural[l.realFog+target],0,'fixture target has no natural exploration');
  const before=natural.slice(l.realFog,l.realFog+4*NN);
  check.equal(w.place_error_rotated(0,at.x,at.y,0),mode===0?3:0,'start map mode governs building terrain knowledge');
  check.deepEqual(save(w).slice(l.realFog,l.realFog+4*NN),before,'placement knowledge does not grant genuine team exploration');
  if(mode===0)invalid(w,()=>w.cmd_place_rotated(0,at.x,at.y,0),'unknown neutral territory in mode0');
  else{check.equal(w.cmd_place_rotated(0,at.x,at.y,0),1);check.ok(placed(w,0,at));}
 }
 // A single unknown tile at the expanded diagonal corner is sufficient to
 // reject an otherwise-known entire45-degree footprint in normal fog mode.
 const w=await make(),builder=own(w,0,8)[0];select(w,builder);const diagonal=spatialSite(w,1,0,{turn:1});let b=save(w),l=layout(b);const tile=(diagonal.y+2)*N+diagonal.x+2;b[l.realFog+tile]=0;b[l.displayFog+tile]=0;
 check.equal(load(w,b),1);select(w,builder);check.equal(w.place_error_rotated(1,diagonal.x,diagonal.y,1),3,'odd-angle enlarged footprint checks every explored tile');
 completed.push('map modes0/1/2 building knowledge, unchanged genuine per-team fog, full diagonal-BBox scouting');
}

async function outsideBuildRateChecks(){
 const rows=[];
 for(const owner of [0,255]){
  const w=await make(),worker=own(w,0,8)[0],home=own(w,0,0)[0];select(w,worker);
  const at=spatialSite(w,1,owner,{home:owner===0?{x:home.x+9,y:home.y+9}:home});
  const b=save(w);move(b,worker,at.cx-1.6,at.cy);check.equal(load(w,b),1);select(w,find(w,worker.handle));
  const bank=stocks(w),cost=bank.map((_,s)=>w.unit_cost_stat(0,1,s));check.equal(w.cmd_place_rotated(1,at.x,at.y,0),1,'non-TC neutral-land construction is allowed');
  const e=placed(w,1,at);check.ok(e);check.deepEqual(stocks(w),bank.map((v,s)=>v-cost[s]),'neutral build changes time, not cost');
  run(w,350);rows.push({w,e,owner,progress:find(w,e.handle).built});check.equal(w.territory_owner(at.region),owner,'house alone does not claim a neutral province');
 }
 const native=source.class_rows.find(r=>r.id===1&&r.epoch===0).fields.buildtime;check.equal(native.verified,true);check.equal(native.value,35);
 near(rows[0].progress,350*.05/native.value,'one adjacent citizen uses native house time',.025);
 near(rows[1].progress,350*.05/native.value/2,'neutral-land multiplier is explicit2x build-time policy',.025);
 run(rows[0].w,400);check.equal(find(rows[0].w,rows[0].e.handle).built,1);
 run(rows[1].w,400);check.ok(find(rows[1].w,rows[1].e.handle).built<.6,'neutral house does not finish at owned-territory time');
 run(rows[1].w,750);check.equal(find(rows[1].w,rows[1].e.handle).built,1);
 completed.push('non-TC neutral terrain: equal source cost, no territory claim and measured2x build-time rule with actual adjacent citizen');
}

async function dockChecks(){
 const w=await make(),worker=own(w,0,8)[0],home=own(w,0,0)[0];select(w,worker);
 const origin=spatialSite(w,5,0,{turn:1,legal:false,home:{x:home.x+14,y:home.y+8}}),control=save(w);
 const coast=(turn,{direction=0,allWater=false,pocket=false,allLand=false,rock=false,resource=false}={})=>{
  const b=control.slice(),l=layout(b),s=bbox(5,turn),mid=Math.floor(s/2),cx=origin.x+s/2,cy=origin.y+s/2;
  for(let y=origin.y-5;y<=origin.y+s+5;y++)for(let x=origin.x-5;x<=origin.x+s+5;x++){
   const local=direction===0?x-origin.x:direction===1?y-origin.y:direction===2?origin.x+s-1-x:origin.y+s-1-y;
   let type=allLand?3:allWater?1:local<mid?3:1;
   if(pocket&&(x<origin.x||x>=origin.x+s||y<origin.y||y>=origin.y+s))type=3;
   editTerrain(b,x,y,x,y,type);
  }
  if(rock)editTerrain(b,origin.x,origin.y,origin.x,origin.y,5);
  if(resource){b[l.res+origin.y*N+origin.x]=2;l.d.setUint16(l.amt+(origin.y*N+origin.x)*2,100,true);}
  const px=direction===0?origin.x-3:direction===2?origin.x+s+2:cx;
  const py=direction===1?origin.y-3:direction===3?origin.y+s+2:cy;
  // An all-water negative fixture leaves the citizen outside the coast patch.
  move(b,worker,allWater?home.x+5:px,allWater?home.y+4:py);
  return {b,s,cx,cy,x:origin.x,y:origin.y};
 };
 for(let turn=0;turn<8;turn++){
  const f=coast(turn);check.equal(load(w,f.b),1);select(w,find(w,worker.handle));
  check.equal(w.place_error_rotated(5,f.x,f.y,turn),0,'mixed real land/water coast accepts manual orientation '+turn);
  const auto=w.dock_orientation(f.x,f.y);check.ok([0,2,4,6].includes(auto),'shore-derived automatic orientation is cardinal');
  check.equal(w.cmd_place_rotated(5,f.x,f.y,turn),1);const dock=placed(w,5,f);check.ok(dock);near(dock.rot,turn*Math.PI/4,'dock retains manually requested orientation');
  if(turn===2){run(w,2600);check.equal(find(w,dock.handle).built,1,'land citizen completes real mixed-coast dock');const saved=save(w),state=w.state_hash();check.equal(load(w,saved),1);check.equal(w.state_hash(),state);near(find(w,dock.handle).rot,Math.PI/2,'dock save preserves direction');}
 }
 const autoDirections=[];
 for(let direction=0;direction<4;direction++){
  const f=coast(0,{direction});check.equal(load(w,f.b),1);select(w,find(w,worker.handle));check.equal(w.place_error_rotated(5,f.x,f.y,0),0);
  const turn=w.dock_orientation(f.x,f.y);check.ok([0,2,4,6].includes(turn));autoDirections.push(turn);
 }
 check.equal(new Set(autoDirections).size,4,'four genuine shoreline directions resolve to four different cardinal choices');
 for(const [name,options]of [['all land',{allLand:true}],['all water without land edge',{allWater:true}],['water pocket without sea exit',{pocket:true}],['rock inside footprint',{rock:true}],['deposit inside footprint',{resource:true}]]){
  const f=coast(1,options);check.equal(load(w,f.b),1);select(w,find(w,worker.handle));check.equal(w.place_error_rotated(5,f.x,f.y,1),1,name+' rejects terrain before payment');invalid(w,()=>w.cmd_place_rotated(5,f.x,f.y,1),name);
 }
 completed.push('actual mixed-land/water dock BBoxes, all8 manual directions, cardinal shore adapter, real builder completion/save and missing-land/missing-sea/rock/deposit rejection');
}

async function orientationChecks(profile='ee2'){
 const w=await make({profile}),worker=own(w,0,8)[0];select(w,worker);const at=spatialSite(w,52,0,{turn:1}),control=save(w),entry=groundManifest.entries.find(e=>e.id===52);
 check.ok(entry,'asymmetric original Warehouse ground proof');
 for(let turn=0;turn<8;turn++){
  check.equal(load(w,control),1);select(w,worker);const side=bbox(52,turn),site={...at,s:side,cx:at.x+side/2,cy:at.y+side/2};
  const ghostState=w.state_hash(),ghost=rendered(w,52,site.x,site.y,{turn,ghost:true});
  near(ghost[3],turn*Math.PI/4,'ghost uses same requested yaw');near(ghost[0],site.cx,'ghost uses enlarged footprint center');near(ghost[1],site.cy,'ghost uses enlarged footprint center');near(ghost[8],sizes.get(52),'ghost native render size does not grow with BBox');
  check.equal(groundPlacement(entry,new Float32Array(ghost)),null,'ground overlay never appears on build ghost');check.equal(w.state_hash(),ghostState,'ghost rendering never changes simulation hash');
  const before=stocks(w),cost=before.map((_,s)=>w.unit_cost_stat(0,52,s));check.equal(w.cmd_place_rotated(52,site.x,site.y,turn),1);
  let e=placed(w,52,site);check.ok(e);near(e.rot,turn*Math.PI/4,'saved foundation actual yaw');near(e.prot,e.rot,'rotation interpolation starts at requested orientation');
  check.deepEqual(stocks(w),before.map((v,s)=>v-cost[s]),'rotation never changes unit cost');
  const foundation=save(w),foundationHash=w.state_hash();check.equal(load(w,foundation),1);check.equal(w.state_hash(),foundationHash,'unfinished foundation orientation and worker order survive reload');near(find(w,e.handle).rot,e.rot,'unfinished saved yaw');
  run(w,1900);e=find(w,e.handle);check.equal(e.built,1,'actual building construction completes');near(e.rot,turn*Math.PI/4,'yaw persists through build completion');
  const row=rendered(w,52,e.x,e.y);near(row[3],turn*Math.PI/4,'native NIF instance yaw');near(row[8],sizes.get(52),'render size remains unrotated native-model adapter size');check.equal(row[6],1);check.ok(Math.round(row[7])&64);
  probeNativeDraw(row);
  const placement=groundPlacement(entry,new Float32Array(row));check.ok(placement);near(placement[2],turn*Math.PI/4,'original ground uses actual building yaw');
  near(placement[3]/placement[4],entry.width/entry.depth,'native asymmetric ground aspect survives rotation');
  const state=w.state_hash(),saved=save(w);run(w,20);check.equal(load(w,saved),1);check.equal(w.state_hash(),state,'save roundtrip preserves orientation hash');near(find(w,e.handle).rot,e.rot,'save roundtrip preserves actual yaw');assertCurrentSave(saved);
  if(turn===0){
   const rotatedOnly=saved.slice(),l=layout(rotatedOnly),same=entities(rotatedOnly).find(row=>row.handle===e.handle);
   l.d.setFloat32(same.at+32,Math.PI/2,true);l.d.setFloat32(same.at+36,Math.PI/2,true);
   check.equal(load(w,rotatedOnly),1);check.notEqual(w.state_hash(),state,'building-only yaw change is protected by state hash');check.equal(load(w,saved),1);check.equal(w.state_hash(),state);
  }
  if(turn%2){check.equal(w.place_error_rotated(1,site.x+side-1,site.y,0),1,'odd-angle conservative BBox corner remains blocked');check.equal(load(w,saved),1);check.equal(w.place_error_rotated(1,site.x+side-1,site.y,0),1,'load reconstructs rotated blocked footprint');}
 }
 // Legacy callers preserve their zero-turn behavior and corresponding bytes.
 check.equal(load(w,control),1);select(w,worker);const at0={...at,cx:at.x+1,cy:at.y+1};check.equal(w.cmd_place(52,at.x,at.y),1);const old=placed(w,52,at0);check.ok(old);near(old.rot,0,'legacy cmd_place default turn0');
 completed.push(`${profile}: all8 real construction/save/render/Ground rotations; diagonal occupancy; old cmd_place default0`);
}

async function strictWireChecks(){
 const w=await make(),worker=own(w,0,8)[0],enemy=own(w,1,8)[0];select(w,worker);const at=spatialSite(w,1,0),control=save(w);
 const rejects=[
  ['no handles',wire(16,at.x,at.y,1)],['enemy builder',wire(16,at.x,at.y,1,[enemy.handle])],['stale builder generation',wire(16,at.x,at.y,1,[worker.handle+65536])],
  ['nonworker own handle',wire(16,at.x,at.y,1,[own(w,0,0)[0].handle])],['fractional X',wire(16,at.x+.5,at.y,1,[worker.handle])],['fractional Y',wire(16,at.x,at.y+.25,1,[worker.handle])],
  ['NaN',wire(16,NaN,at.y,1,[worker.handle])],['infinite',wire(16,at.x,Infinity,1,[worker.handle])],['negative packed',wire(16,at.x,at.y,-1,[worker.handle])],
  ['turn8',wire(16,at.x,at.y,1|(8<<8),[worker.handle])],['reserved bits',wire(16,at.x,at.y,1|(1<<16),[worker.handle])],['bad definition',wire(16,at.x,at.y,255,[worker.handle])],
  ['truncated record',wire(16,at.x,at.y,1,[worker.handle]).slice(0,-1)]
 ];
 for(const [label,packet]of rejects){check.equal(load(w,control),1);invalidWire(w,0,packet,label);}
 check.equal(load(w,control),1);deliver(w,1,wire(16,at.x,at.y,1,[worker.handle]));check.equal(own(w,0,1).length,0,'authenticated sender cannot command another player builder');
 check.equal(load(w,control),1);deliver(w,0,wire(16,at.x,at.y,1|(7<<8),[worker.handle]));const e=placed(w,1,{...at,cx:at.x+1.5,cy:at.y+1.5});check.ok(e);near(e.rot,7*Math.PI/4,'valid authenticated rotated command');
 check.equal(load(w,control),1);deliver(w,0,wire(10,at.x,at.y,1,[worker.handle]));const old=placed(w,1,{...at,cx:at.x+1,cy:at.y+1});check.ok(old);near(old.rot,0,'old C_PLACE10 remains default0');
 completed.push('C_PLACE_ROTATED16 strict integer coordinates/bit packing/own-builder authentication, malformed records and legacy C_PLACE10');
}

async function networkChecks(){
 const ws=[];for(let seat=0;seat<2;seat++){const w=(await WebAssembly.instantiate(blobs.ee2,{})).instance.exports;w.init(7,0,0,2,seat);ws.push(w);}
 const queues=[[],[]],ls=ws.map((w,seat)=>new Lockstep(w,seat,buffer=>queues[1-seat].push(buffer)));
 function advance(n){for(let i=0;i<n;i++)for(let seat=0;seat<2;seat++){for(const b of queues[seat].splice(0))ls[seat].receive(b);check.equal(ls[seat].step(),true);}check.equal(ws[0].state_hash(),ws[1].state_hash(),'both actual peers retain identical placement hashes');check.ok(!ls[0].desync&&!ls[1].desync);}
 const sites=[];
 for(let seat=0;seat<2;seat++){
  const w=ws[seat],worker=own(w,seat,8)[0],home=own(w,seat,0)[0];select(w,worker);let at;
  for(let y=Math.floor(home.y)-9;y<home.y+8&&!at;y++)for(let x=Math.floor(home.x)-9;x<home.x+8;x++)if(w.place_error_rotated(1,x,y,seat?7:1)===0){at={x,y};break;}
  check.ok(at,'native generated/known coastal-free owned housing site');const turn=seat?7:1;check.equal(w.cmd_place_rotated(1,at.x,at.y,turn),1);check.equal(own(w,seat,1).length,0,'network API only queues');
  const n=w.outbox_len(),packet=new Uint8Array(w.memory.buffer,w.outbox_ptr(),n).slice(),d=new DataView(packet.buffer);check.equal(packet[0],16);check.equal(packet[1],1);check.equal(n,18);check.equal(d.getFloat32(2,true),at.x);check.equal(d.getFloat32(6,true),at.y);check.equal(d.getUint32(10,true),1|(turn<<8));check.equal(d.getUint32(14,true),worker.handle);sites.push({...at,turn});
 }
 advance(10);for(const w of ws)for(let seat=0;seat<2;seat++){const e=own(w,seat,1)[0];check.ok(e);near(e.rot,sites[seat].turn*Math.PI/4,'scheduled peer orientation');}
 advance(1800);for(const w of ws)for(const e of [...own(w,0,1),...own(w,1,1)])check.equal(e.built,1,'both peers complete their actual build');
 report.lockstepTurns=ls[0].turn;completed.push('two actual seats queue exact18-byte rotated commands, execute on identical turns and finish construction without hash divergence');
}

async function all(){probeNativeDraw=await nativeDrawProbe();await footprintChecks();await expansionChecks();await naturalStartChecks();await validationChecks();await visibilityChecks();await outsideBuildRateChecks();await dockChecks();await orientationChecks();await orientationChecks('base');await strictWireChecks();await networkChecks();}
try{await all();report.status='passed';}catch(error){report.status='failed';report.error=error.stack||String(error);throw error;}
finally{report.checks=checks;mkdirSync(reports,{recursive:true});writeFileSync(new URL('placement-rotation-tests.json',reports),JSON.stringify(report,null,2)+'\n');}
console.log(`PASS placement/rotation: ${checks} checks; ${completed.join('; ')}`);
