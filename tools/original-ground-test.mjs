// Independent CPU/renderer contract checks. FakeGL records state and payloads;
// it deliberately does not execute GPU shaders or claim visual equivalence.
import assert from 'node:assert/strict';
import {groundPlacement,selectGroundSprite,groundEpochSlot,nativeUrbanAt,GROUND_CITY_RADIUS_FACTOR,groundSeasonFrames,sampleGroundHeight,loadOriginalGround} from '../docs/original-ground.js';

const near=(actual,expected,message)=>assert.ok(Math.abs(actual-expected)<1e-5,`${message}: ${actual} versus ${expected}`);
const entry={id:0,decal:'UnderFixture',nativeSize:[4,2],width:8,depth:4,offsetX:2,offsetY:-1,
 rural:[{url:'local-original/ground/test.png',uv:[0,0,1,1],seasonalUrls:['local-original/ground/test-winter.png',...Array(4).fill('local-original/ground/test.png')]},{url:'local-original/ground/test-era2.png'},{url:'local-original/ground/test-era3.png'}],
 urban:[{url:'local-original/ground/test-urban1.png'},{url:'local-original/ground/test-urban2.png'},{url:'local-original/ground/test-era3.png'}],
 death:{rural:[{url:'local-original/ground/test-death1.png'},{url:'local-original/ground/test-death2.png'},{url:'local-original/ground/test-death3.png'}],
        urban:[{url:'local-original/ground/test-death1.png'},{url:'local-original/ground/test-death2.png'},{url:'local-original/ground/test-death3.png'}]}};
const instance=({x=10,z=20,angle=Math.PI/2,flags=64,built=1,size=2,id=0}={})=>new Float32Array([x,z,0,angle,id,0,built,flags,size,1,0,0]);
const source=instance(),unchanged=source.slice(),entryCopy=structuredClone(entry);
const placement=groundPlacement(entry,source);
near(placement[0],10.5,'signed offset rotated into world X');near(placement[1],21,'signed offset rotated into world Z');
near(placement[2],Math.PI/2,'instance yaw retained');near(placement[3],4,'isotropic native width');near(placement[4],2,'isotropic native depth');
near(placement[3]/placement[4],entry.width/entry.depth,'native aspect retained');
assert.deepEqual(source,unchanged,'placement never mutates the instance');assert.deepEqual(entry,entryCopy,'placement never mutates manifest fields');
assert.equal(selectGroundSprite(entry),entry.rural[0],'native stone-age rural slot zero');
assert.equal(selectGroundSprite({urban:entry.urban}),null,'missing rural source has no guessed urban replacement');
for(let epoch=0;epoch<15;epoch++){
 const slot=epoch<5?0:epoch<10?1:2;assert.equal(groundEpochSlot(epoch+1),slot);assert.equal(selectGroundSprite(entry,{epoch}),entry.rural[slot]);
 assert.equal(selectGroundSprite(entry,{epoch,urban:true}),entry.urban[slot]);assert.equal(selectGroundSprite(entry,{epoch,urban:true,state:'Death'}),entry.death.urban[slot]);
 assert.equal(selectGroundSprite(entry,{epoch,hasOwner:false}),entry.rural[0],'world decal keeps first source slot regardless of player ages');
}
const nativePlacement=groundPlacement({...entry,placementProfile:'native-world-undertexture'},source);
for(const [actual,expected]of nativePlacement.map((value,i)=>[value,[11,22,Math.PI/2,8,4][i]]))near(actual,expected,'native world dimensions and offsets');
assert.deepEqual(groundPlacement({...entry,placementProfile:'native-world-undertexture'},instance({size:20})),nativePlacement,'native undertexture dimensions do not depend on rasterized gameplay footprint size');
const city={x:0,z:0,radius:1,team:3};assert.equal(nativeUrbanAt(1.3,0,[city]),true,'native city mask is the strict L1 diamond, not a radius-one circle');
assert.equal(nativeUrbanAt(GROUND_CITY_RADIUS_FACTOR,0,[city]),false,'native city boundary is excluded');assert.equal(nativeUrbanAt(GROUND_CITY_RADIUS_FACTOR-1e-6,0,[city]),true);
assert.equal(nativeUrbanAt(0,0,[{...city,active:false}]),false);assert.equal(nativeUrbanAt(0,0,[]),false);assert.equal(nativeUrbanAt(NaN,0,[city]),false);
const roundedRadiusNine=Math.fround(9*GROUND_CITY_RADIUS_FACTOR);assert.ok(roundedRadiusNine<9*GROUND_CITY_RADIUS_FACTOR);
assert.equal(nativeUrbanAt(roundedRadiusNine,0,[{...city,radius:9}]),true,'x87 comparison retains the radius product above the rounded float boundary');
assert.deepEqual(groundSeasonFrames(entry.rural[0]),{first:entry.rural[0].url,next:entry.rural[0].url,blend:0},'unspecified season retains the labelled physical preview');
for(const [phase,first,next,blend]of[[0,0,1,0],[.1,0,1,.5],[.2,1,2,0],[.9,4,0,.5],[1,0,1,0],[-.1,4,0,.5]]){
 const chosen=groundSeasonFrames(entry.rural[0],phase);assert.equal(chosen.first,entry.rural[0].seasonalUrls[first]);assert.equal(chosen.next,entry.rural[0].seasonalUrls[next]);near(chosen.blend,blend,'five-entry source sequence uses year fraction, not a month index');
}
for(const row of [instance({flags:0}),instance({flags:8}),instance({flags:64|8}),instance({flags:64|16}),instance({built:.99}),instance({size:0})]){
 const before=row.slice();assert.equal(groundPlacement(entry,row),null,'mobile, ghost, unfinished or invalid-size instance skipped');assert.deepEqual(row,before);
}
assert.equal(groundPlacement({...entry,nativeSize:[0,2]},source),null);
assert.equal(groundPlacement({...entry,width:0},source),null);

// Compute barycentric heights independently on a deliberately non-coplanar
// cell. Bilinear interpolation would give different answers in this fixture.
const HN=257,heights=new Float32Array(HN*HN),cx=5,cz=7,cell=cz*HN+cx;
heights[cell]=2;heights[cell+1]=10;heights[cell+HN]=14;heights[cell+HN+1]=40;
function barycentric(p,a,b,c){
 const d=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1]);
 const wa=((b[1]-c[1])*(p[0]-c[0])+(c[0]-b[0])*(p[1]-c[1]))/d;
 const wb=((c[1]-a[1])*(p[0]-c[0])+(a[0]-c[0])*(p[1]-c[1]))/d;
 return wa*a[2]+wb*b[2]+(1-wa-wb)*c[2];
}
const a=[0,0,2],b=[1,0,10],c=[0,1,14],d=[1,1,40];
for(const [x,z] of [[.25,.25],[.8,.6],[.2,.8],[0,0],[1,0],[0,1],[1,1],[.9,.100001],[.9,.099999]]){
 const expected=x+z<=1?barycentric([x,z],a,c,b):barycentric([x,z],b,c,d);
 near(sampleGroundHeight(heights,cx+x,cz+z),expected,'native a-c-b / b-c-d terrain triangle');
}
const triangleValue=sampleGroundHeight(heights,cx+.25,cz+.25),bilinear=2*.75*.75+10*.25*.75+14*.75*.25+40*.25*.25;
assert.ok(Math.abs(triangleValue-bilinear)>.5,'saddle fixture distinguishes triangle from bilinear interpolation');
const plane=new Float32Array(HN*HN);for(let z=0;z<HN;z++)for(let x=0;x<HN;x++)plane[z*HN+x]=2*x+3*z;
const planeCopy=plane.slice();
for(const [x,z]of[[-3,-7],[0,17.5],[256,256],[270,34.2],[11.25,256]]){
 const clamp=v=>Math.max(0,Math.min(255.9999,v));near(sampleGroundHeight(plane,x,z),2*clamp(x)+3*clamp(z),'map edge clamp uses valid final cell');
}
assert.deepEqual(plane,planeCopy,'height sampling is read only');

class FakeGL{
 constructor(){
  Object.assign(this,{ARRAY_BUFFER:0x8892,ELEMENT_ARRAY_BUFFER:0x8893,ARRAY_BUFFER_BINDING:0x8894,STATIC_DRAW:0x88e4,STREAM_DRAW:0x88e0,
   FLOAT:0x1406,UNSIGNED_BYTE:0x1401,UNSIGNED_SHORT:0x1403,TRIANGLES:4,TEXTURE0:0x84c0,TEXTURE2:0x84c2,TEXTURE6:0x84c6,
   TEXTURE_2D:0x0de1,TEXTURE_BINDING_2D:0x8069,ACTIVE_TEXTURE:0x84e0,CURRENT_PROGRAM:0x8b8d,VERTEX_ARRAY_BINDING:0x85b5,
   RGBA:0x1908,RED:0x1903,R32F:0x822e,VERTEX_SHADER:0x8b31,FRAGMENT_SHADER:0x8b30,COMPILE_STATUS:0x8b81,LINK_STATUS:0x8b82,
   TEXTURE_MIN_FILTER:0x2801,TEXTURE_MAG_FILTER:0x2800,LINEAR_MIPMAP_LINEAR:0x2703,LINEAR:0x2601,NEAREST:0x2600,
   TEXTURE_WRAP_S:0x2802,TEXTURE_WRAP_T:0x2803,CLAMP_TO_EDGE:0x812f,UNPACK_FLIP_Y_WEBGL:0x9240,BLEND:0x0be2,DEPTH_WRITEMASK:0x0b72,
   BLEND_SRC_RGB:0x80c9,BLEND_DST_RGB:0x80c8,BLEND_SRC_ALPHA:0x80cb,BLEND_DST_ALPHA:0x80ca,ONE:1,ONE_MINUS_SRC_ALPHA:0x0303});
  this.serial=1;this.active=7;this.program=null;this.defaultVao={attributes:new Map(),element:null};this.vao=this.defaultVao;this.array=null;
  this.depth=true;this.blend=false;this.factors=[0x0302,0x0304,0x0306,0x0307];this.flip=true;this.units=new Map();
  this.textures=[];this.buffers=[];this.programs=[];this.draws=[];this.uploads=[];this.calls=0;
 }
 object(kind){return{kind,id:this.serial++};}
 getParameter(p){this.calls++;const values=new Map([[this.ACTIVE_TEXTURE,this.TEXTURE0+this.active],[this.CURRENT_PROGRAM,this.program],
  [this.VERTEX_ARRAY_BINDING,this.vao],[this.ARRAY_BUFFER_BINDING,this.array],[this.DEPTH_WRITEMASK,this.depth],
  [this.BLEND_SRC_RGB,this.factors[0]],[this.BLEND_DST_RGB,this.factors[1]],[this.BLEND_SRC_ALPHA,this.factors[2]],[this.BLEND_DST_ALPHA,this.factors[3]],
  [this.UNPACK_FLIP_Y_WEBGL,this.flip],[this.TEXTURE_BINDING_2D,this.units.get(this.active)||null]]);assert.ok(values.has(p),`unimplemented query ${p}`);return values.get(p);}
 isEnabled(p){assert.equal(p,this.BLEND);return this.blend;}enable(p){assert.equal(p,this.BLEND);this.blend=true;}disable(p){assert.equal(p,this.BLEND);this.blend=false;}
 depthMask(v){this.depth=v;}blendFunc(s,d){this.factors=[s,d,s,d];}blendFuncSeparate(...v){this.factors=v;}
 activeTexture(v){this.active=v-this.TEXTURE0;}bindTexture(target,t){assert.equal(target,this.TEXTURE_2D);this.units.set(this.active,t);}
 createTexture(){const t={...this.object('texture'),params:new Map()};this.textures.push(t);return t;}
 pixelStorei(p,v){assert.equal(p,this.UNPACK_FLIP_Y_WEBGL);this.flip=v;}
 texImage2D(...args){const t=this.units.get(this.active);assert.ok(t);const image=args.length===6?args[5]:null;
  Object.assign(t,{format:args[2],width:image?.width??args[3],height:image?.height??args[4],pixels:image?new Uint8Array(image.pixels):new args[8].constructor(args[8]),uploadFlip:this.flip});this.uploads.push({unit:this.active,texture:t,image});}
 texSubImage2D(target,level,x,z,width,height,format,type,pixels){const t=this.units.get(this.active);assert.equal(target,this.TEXTURE_2D);assert.ok(t);assert.equal(t.format,this.R32F);t.pixels=new pixels.constructor(pixels);t.update={width,height,format,type};}
 texParameteri(target,k,v){assert.equal(target,this.TEXTURE_2D);this.units.get(this.active).params.set(k,v);}generateMipmap(target){assert.equal(target,this.TEXTURE_2D);this.units.get(this.active).mipmap=true;}
 createBuffer(){const b=this.object('buffer');this.buffers.push(b);return b;}bindBuffer(target,b){if(target===this.ARRAY_BUFFER)this.array=b;else{assert.equal(target,this.ELEMENT_ARRAY_BUFFER);this.vao.element=b;}}
 bufferData(target,data,usage){const b=target===this.ARRAY_BUFFER?this.array:this.vao.element;assert.ok(b);b.data=new data.constructor(data);b.usage=usage;}
 createVertexArray(){return{...this.object('vao'),attributes:new Map(),element:null};}bindVertexArray(v){this.vao=v||this.defaultVao;}
 enableVertexAttribArray(loc){this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),enabled:true});}
 vertexAttribPointer(loc,size,type,normalized,stride,offset){assert.ok(this.array);this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),buffer:this.array,size,type,normalized,stride,offset});}
 vertexAttribDivisor(loc,divisor){this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),divisor});}
 createProgram(){const p={...this.object('program'),uniforms:new Map(),shaders:[]};this.programs.push(p);return p;}deleteProgram(p){p.deleted=true;}
 createShader(type){return{...this.object('shader'),type};}shaderSource(s,source){s.source=source;}compileShader(){}getShaderParameter(){return !this.failShader;}getShaderInfoLog(){return 'fixture shader compilation rejection';}
 attachShader(p,s){p.shaders.push(s);}deleteShader(){}linkProgram(){}getProgramParameter(){return true;}getProgramInfoLog(){return '';}
 getUniformLocation(program,name){return{program,name};}useProgram(p){this.program=p;}uniform1i(loc,value){assert.equal(this.program,loc.program);this.program.uniforms.set(loc.name,value);}
 uniform1f(loc,value){this.uniform1i(loc,value);}
 uniformMatrix4fv(loc,transpose,values){assert.equal(transpose,false);assert.equal(this.program,loc.program);this.program.uniforms.set(loc.name,new values.constructor(values));}
 drawElementsInstanced(mode,count,type,offset,n){
  assert.ok(this.vao.element?.data instanceof Uint16Array);const attribute=this.vao.attributes.get(1);assert.equal(attribute.divisor,1);
  this.draws.push({mode,count,type,offset,n,vao:this.vao,indices:this.vao.element.data.slice(),instances:attribute.buffer.data.slice(),
   program:this.program,uniforms:new Map(this.program.uniforms),diffuse:this.units.get(0),diffuseB:this.units.get(7),fog:this.units.get(2),height:this.units.get(6),depth:this.depth,blend:this.blend,factors:[...this.factors]});
 }
}
function baseline(gl,blend=false){
 gl.program=gl.createProgram();gl.vao=gl.createVertexArray();gl.array=gl.createBuffer();gl.array.data=new Float32Array([9,8,7]);
 gl.vao.element=gl.createBuffer();gl.vao.element.data=new Uint16Array([2,1,0]);gl.depth=true;gl.blend=blend;
 for(let unit=0;unit<8;unit++)gl.units.set(unit,gl.createTexture());gl.active=7;return state(gl);
}
function state(gl){return{program:gl.program,vao:gl.vao,array:gl.array,element:gl.vao.element,depth:gl.depth,blend:gl.blend,
 factors:[...gl.factors],flip:gl.flip,active:gl.active,units:[...gl.units]};}
const rgba=new Uint8Array([120,30,90,64, 5,250,111,0, 255,17,22,190, 1,2,3,255]),images=[],requests=[];
const manifest={entries:[entry,{...entry,id:1,decal:'OtherBuilding',offsetX:0,offsetY:0},
 {...entry,id:2,decal:'UnsupportedFlip',rural:[{url:'forbidden.png',flipU:true}],urban:[],death:null},{...entry,id:3,decal:'NoRural',rural:[],urban:[],death:null}]};
const manifestCopy=structuredClone(manifest);
const sourceUrls=new Set([...entry.rural,...entry.urban,...entry.death.rural,...entry.death.urban].flatMap(s=>[s.url,...(s.seasonalUrls||[])]));
const fetchFile=async input=>{const path=new URL(input).pathname;requests.push(path);if(path.endsWith('/ground-decals.json'))return{ok:true,json:async()=>manifest};assert.ok([...sourceUrls].some(url=>path.endsWith('/'+url)));return{ok:true,blob:async()=>({path})};};
const decode=async(blob,options)=>{assert.deepEqual(options,{premultiplyAlpha:'none',colorSpaceConversion:'none'});const image={width:2,height:2,pixels:rgba.slice(),closed:false,close(){this.closed=true;this.pixels.fill(0);}};images.push(image);return image;};
const vp=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);

let disabledFetch=0;const glDisabled=new FakeGL();const disabledState=baseline(glDisabled),beforeCalls=glDisabled.calls;
const disabled=await loadOriginalGround(glDisabled,false,{fetch:()=>{disabledFetch++;throw Error('must not fetch');},decode});
assert.equal(disabled.enabled,false);assert.equal(disabledFetch,0);assert.equal(glDisabled.calls,beforeCalls);disabled.draw({});disabled.updateTerrain(null);assert.deepEqual(state(glDisabled),disabledState);

const gl=new FakeGL(),initial=baseline(gl,true),originalArray=gl.array.data.slice(),originalElements=gl.vao.element.data.slice();
const renderer=await loadOriginalGround(gl,true,{fetch:fetchFile,decode});
assert.equal(renderer.enabled,true);assert.deepEqual([...renderer.entries.keys()],[0,1]);assert.ok(renderer.errors.some(e=>e.includes('unsupported sprite transform')));
assert.equal(requests.filter(p=>p.endsWith('/ground/test.png')).length,1,'shared physical sprite decoded/uploaded once');
assert.equal(images.length,sourceUrls.size);assert.ok(images.every(image=>image.closed),'all source image handles close after upload');
assert.equal(renderer.entries.get(0).textures.size,sourceUrls.size,'all epoch, rural/urban and death sources are available before the first draw');
for(const url of sourceUrls)assert.equal(requests.filter(path=>path.endsWith('/'+url)).length,1,'physical source image loaded once across every category and building');
assert.deepEqual(state(gl),initial,'load restores program/VAO/buffers/depth/blend/flip/units0..7');
assert.deepEqual(manifest,manifestCopy,'loader does not rewrite source metadata');
const diffuse=renderer.entries.get(0).texture;
assert.deepEqual(diffuse.pixels,rgba,'RGBA payload is neither recolored nor alpha-premultiplied');assert.equal(diffuse.uploadFlip,false);
assert.equal(diffuse.format,gl.RGBA);assert.equal(diffuse.mipmap,true);
assert.equal(diffuse.params.get(gl.TEXTURE_WRAP_S),gl.CLAMP_TO_EDGE);assert.equal(diffuse.params.get(gl.TEXTURE_WRAP_T),gl.CLAMP_TO_EDGE);

const heightCopy=heights.slice();renderer.updateTerrain(heights);assert.deepEqual(state(gl),initial,'terrain upload restores all caller state');assert.deepEqual(heights,heightCopy);
assert.throws(()=>renderer.updateTerrain(new Float32Array(5)),/257/);assert.deepEqual(state(gl),initial,'rejected terrain shape changes no state');
const rows=[instance(),instance({flags:64|8}),instance({built:.5}),instance({flags:0}),instance({id:1,x:30,angle:0})];
const data=new Float32Array(rows.flatMap(r=>Array.from(r))),ranges=new Uint32Array(512);ranges[0]=0;ranges[1]=4;ranges[2]=4;ranges[3]=1;
const inputCopy=data.slice(),rangeCopy=ranges.slice(),vpCopy=vp.slice();
renderer.draw({data,ranges,vp});
assert.deepEqual(renderer.metrics,{calls:2,instances:2});assert.equal(gl.draws.length,2);assert.deepEqual(state(gl),initial,'draw restores all caller state');
assert.deepEqual(data,inputCopy);assert.deepEqual(ranges,rangeCopy);assert.deepEqual(vp,vpCopy);assert.deepEqual(gl.array.data,originalArray);assert.deepEqual(gl.vao.element.data,originalElements);
for(const draw of gl.draws){
 assert.equal(draw.n,1);assert.equal(draw.mode,gl.TRIANGLES);assert.equal(draw.type,gl.UNSIGNED_SHORT);assert.equal(draw.count,32*32*6);
 assert.equal(draw.uniforms.get('uDiffuse'),0);assert.equal(draw.uniforms.get('uFog'),2);assert.equal(draw.uniforms.get('uHeight'),6);
 assert.equal(draw.uniforms.get('uDiffuseB'),7);assert.equal(draw.uniforms.get('uSeasonMix'),0);assert.equal(draw.diffuseB,diffuse);
 assert.equal(draw.diffuse,diffuse);assert.equal(draw.fog,initial.units.find(([u])=>u===2)[1]);assert.equal(draw.height.format,gl.R32F);
 assert.deepEqual(draw.height.pixels,heightCopy);assert.equal(draw.depth,false);assert.equal(draw.blend,true);assert.deepEqual(draw.factors,[gl.ONE,gl.ONE_MINUS_SRC_ALPHA,gl.ONE,gl.ONE_MINUS_SRC_ALPHA]);
 assert.deepEqual(Array.from(draw.indices.slice(0,6)),[0,33,1,1,33,34],'actual overlay index buffer uses a-c-b / b-c-d');
 assert.deepEqual(draw.uniforms.get('uVP'),vpCopy);assert.equal(draw.instances.length,5);
}
for(let i=0;i<5;i++)near(gl.draws[0].instances[i],placement[i],'uploaded placement matches source-scale transform');
const empty=new Uint32Array(512);renderer.draw({data,ranges:empty,vp});assert.deepEqual(renderer.metrics,{calls:0,instances:0});assert.deepEqual(state(gl),initial);
gl.draws=[];renderer.setSeasonPhase({month0:0});renderer.draw({data,ranges,vp});
assert.equal(gl.draws.length,2);assert.equal(gl.draws[0].diffuse,renderer.entries.get(0).textures.get('local-original/ground/test-winter.png'));assert.equal(gl.draws[0].uniforms.get('uSeasonMix'),0);
gl.draws=[];renderer.setSeasonPhase({month0:1,blend:.2});renderer.draw({data,ranges,vp});
assert.equal(gl.draws[0].diffuse,renderer.entries.get(0).textures.get('local-original/ground/test-winter.png'));assert.equal(gl.draws[0].diffuseB,diffuse);near(gl.draws[0].uniforms.get('uSeasonMix'),.5,'calendar1.2months is half of the first five-frame seasonal interval');
assert.deepEqual(state(gl),initial,'seasonal unit7 binding and interpolation draw restore caller state');assert.throws(()=>renderer.setSeasonPhase({month0:12}),RangeError);
const firstEpoch=instance(),secondEpoch=instance({x:30});secondEpoch[5]=1;
const mixedData=new Float32Array([...firstEpoch,...secondEpoch]),mixedRanges=new Uint32Array(512);mixedRanges[1]=2;
renderer.setSeasonPhase({month0:6});gl.draws=[];const query=[];
renderer.draw({data:mixedData,ranges:mixedRanges,vp,epochs:[4,5],isUrbanAt:(x,z,team,id,index)=>{query.push([x,z,team,id,index]);return x>20;}});
assert.equal(gl.draws.length,2);assert.equal(gl.draws[0].diffuse,diffuse,'epoch5 remains source slot zero');
assert.equal(gl.draws[1].diffuse,renderer.entries.get(0).textures.get('local-original/ground/test-urban2.png'),'epoch6 uses source slot one and the supplied city predicate');
assert.deepEqual(query,[[10,20,0,0,0],[30,20,1,0,1]]);assert.deepEqual(state(gl),initial);
const requestsBeforeEpochChange=requests.length;gl.draws=[];
renderer.draw({data:mixedData,ranges:mixedRanges,vp,epochs:[10,14],isUrbanAt:()=>true});
assert.equal(gl.draws.length,1);assert.equal(gl.draws[0].n,2,'late-epoch buildings batch the shared source sprite');assert.equal(gl.draws[0].diffuse,renderer.entries.get(0).textures.get('local-original/ground/test-era3.png'));
assert.equal(requests.length,requestsBeforeEpochChange,'epoch changes use already resident physical source images');

// Missing/invalid assets fail locally and retain the caller's GL contract.
const originalWarn=console.warn;console.warn=()=>{};
try{
 for(const mode of ['manifest','image','decode','shader']){
  const g=new FakeGL(),s=baseline(g,mode==='image');if(mode==='shader')g.failShader=true;
  const m={entries:[entry]};
  const fetch=async input=>String(input).includes('ground-decals.json')?(mode==='manifest'?{ok:false,status:404}:{ok:true,json:async()=>m}):(mode==='image'?{ok:false,status:404}:{ok:true,blob:async()=>({})});
  const badDecode=mode==='decode'?async()=>{throw Error('fixture bad bitmap');}:decode;
  const r=await loadOriginalGround(g,true,{fetch,decode:badDecode});assert.equal(r.enabled,false,mode);assert.ok(r.errors.length,mode);assert.deepEqual(state(g),s,`${mode} failure restores state`);r.draw({});r.updateTerrain(null);assert.deepEqual(state(g),s);
 }
}finally{console.warn=originalWarn;}
console.log('PASS original ground: native 1/6/11 epoch slots, city L1 boundaries, source world dimensions, variable-length seasonal interpolation, per-team/context draw selection, resident sprites, triangle heights and complete caller GL-state restoration. GPU pixels checked separately without UI.');
