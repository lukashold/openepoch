// Contract tests through the real renderer loader/draw API, using tiny synthetic
// geometry and a stateful WebGL2 fake. Browser checks cover actual GPU shaders.
import assert from 'node:assert/strict';
import {loadOriginalModels} from '../docs/original-renderer.js';

class FakeGL {
  constructor(){
    Object.assign(this,{ARRAY_BUFFER:0x8892,ELEMENT_ARRAY_BUFFER:0x8893,STATIC_DRAW:0x88e4,DYNAMIC_DRAW:0x88e8,STREAM_DRAW:0x88e0,
      FLOAT:0x1406,UNSIGNED_INT:0x1405,UNSIGNED_BYTE:0x1401,TRIANGLES:4,TEXTURE0:0x84c0,TEXTURE1:0x84c1,TEXTURE2:0x84c2,
      TEXTURE3:0x84c3,TEXTURE4:0x84c4,TEXTURE5:0x84c5,TEXTURE_2D:0x0de1,TEXTURE_2D_ARRAY:0x8c1a,
      RGBA:0x1908,RGBA32F:0x8814,RGBA8:0x8058,VERTEX_SHADER:0x8b31,FRAGMENT_SHADER:0x8b30,
      COMPILE_STATUS:0x8b81,LINK_STATUS:0x8b82,TEXTURE_MIN_FILTER:0x2801,TEXTURE_MAG_FILTER:0x2800,
      LINEAR_MIPMAP_LINEAR:0x2703,LINEAR:0x2601,NEAREST:0x2600,TEXTURE_WRAP_S:0x2802,TEXTURE_WRAP_T:0x2803,
      REPEAT:0x2901,UNPACK_FLIP_Y_WEBGL:0x9240});
    Object.assign(this,{R32F:0x822e,RED:0x1903,CLAMP_TO_EDGE:0x812f,BLEND:0xbe2,DEPTH_TEST:0xb71,DEPTH_WRITEMASK:0xb72,DEPTH_FUNC:0xb74,
      BLEND_SRC_RGB:0x80c9,BLEND_DST_RGB:0x80c8,BLEND_SRC_ALPHA:0x80cb,BLEND_DST_ALPHA:0x80ca,
      ZERO:0,ONE:1,SRC_COLOR:0x300,ONE_MINUS_SRC_COLOR:0x301,SRC_ALPHA:0x302,ONE_MINUS_SRC_ALPHA:0x303,DST_ALPHA:0x304,ONE_MINUS_DST_ALPHA:0x305,
      DST_COLOR:0x306,ONE_MINUS_DST_COLOR:0x307,SRC_ALPHA_SATURATE:0x308,NEVER:0x200,LESS:0x201,EQUAL:0x202,LEQUAL:0x203,GREATER:0x204,NOTEQUAL:0x205,GEQUAL:0x206,ALWAYS:0x207});
    this.enabled=new Set([this.BLEND,this.DEPTH_TEST]);this.parameters=new Map([[this.DEPTH_WRITEMASK,true],[this.DEPTH_FUNC,this.LEQUAL],
      [this.BLEND_SRC_RGB,this.ONE],[this.BLEND_DST_RGB,this.ONE_MINUS_SRC_ALPHA],[this.BLEND_SRC_ALPHA,this.ONE],[this.BLEND_DST_ALPHA,this.ONE_MINUS_SRC_ALPHA]]);
    this.next=1;this.unit=0;this.defaultVAO={id:0,attributes:new Map(),element:null};this.vao=this.defaultVAO;
    this.array=null;this.units=new Map();this.uniforms=new Map();this.draws=[];this.buffers=[];this.textures=[];
  }
  object(kind){return {kind,id:this.next++};}
  createProgram(){return this.object('program');}
  createShader(type){return {...this.object('shader'),type};}
  shaderSource(shader,source){shader.source=source;}
  compileShader(){} getShaderParameter(){return true;} getShaderInfoLog(){return '';}
  attachShader(){} deleteShader(){} linkProgram(){} getProgramParameter(){return true;} getProgramInfoLog(){return '';}
  getUniformLocation(program,name){return {program,name};} useProgram(program){this.program=program;}
  uniform1i(loc,value){if(loc)this.uniforms.set(loc.name,value);} uniform1f(loc,value){this.uniform1i(loc,value);}
  uniform3fv(loc,value){if(loc)this.uniforms.set(loc.name,Array.from(value));} uniform4fv(loc,value){this.uniform3fv(loc,value);}
  uniformMatrix4fv(loc,transpose,value){this.uniform3fv(loc,value);}
  createBuffer(){const b=this.object('buffer');this.buffers.push(b);return b;}
  bindBuffer(target,buffer){if(target===this.ARRAY_BUFFER)this.array=buffer;else if(target===this.ELEMENT_ARRAY_BUFFER)this.vao.element=buffer;else throw Error('unexpected buffer target');}
  bufferData(target,values,usage){const b=target===this.ARRAY_BUFFER?this.array:this.vao.element;assert.ok(b,'upload requires a bound buffer');b.data=ArrayBuffer.isView(values)?new values.constructor(values):values;b.usage=usage;}
  createVertexArray(){return {...this.object('vao'),attributes:new Map(),element:null};}
  bindVertexArray(vao){this.vao=vao||this.defaultVAO;}
  enableVertexAttribArray(loc){const a=this.vao.attributes.get(loc)||{};a.enabled=true;this.vao.attributes.set(loc,a);}
  vertexAttribPointer(loc,size,type,normalized,stride,offset){assert.ok(this.array,'attribute requires an array buffer');this.vao.attributes.set(loc,{...this.vao.attributes.get(loc),buffer:this.array,size,type,normalized,stride,offset});}
  vertexAttribDivisor(loc,value){const a=this.vao.attributes.get(loc)||{};a.divisor=value;this.vao.attributes.set(loc,a);}
  createTexture(){const t={...this.object('texture'),params:new Map()};this.textures.push(t);return t;}
  activeTexture(unit){this.unit=unit-this.TEXTURE0;}
  bindTexture(target,texture){this.units.set(`${this.unit}:${target}`,texture);}
  bound(target,unit=this.unit){return this.units.get(`${unit}:${target}`);}
  texImage2D(...args){const t=this.bound(args[0]);assert.ok(t,'image upload requires texture');if(args.length===9){t.width=args[3];t.height=args[4];t.pixelData=args[8];}else{t.width=args[5].width;t.height=args[5].height;t.imageUrl=args[5].url;}}
  texStorage3D(target,levels,format,width,height,depth){const t=this.bound(target);assert.ok(t);Object.assign(t,{levels,format,width,height,depth});}
  texSubImage3D(target,level,x,y,z,width,height,depth,format,type,pixels){const t=this.bound(target);assert.ok(t);assert.equal(width,t.width);assert.equal(height,t.height);assert.equal(depth,t.depth);t.pixelData=new pixels.constructor(pixels);t.dataType=type;}
  texParameteri(target,key,value){assert.ok(this.bound(target));this.bound(target).params.set(key,value);}
  generateMipmap(target){this.bound(target).mipmaps=true;} pixelStorei(){} getExtension(){return null;}
  isEnabled(cap){return this.enabled.has(cap);} enable(cap){this.enabled.add(cap);} disable(cap){this.enabled.delete(cap);}
  getParameter(key){assert.ok(this.parameters.has(key),'unexpected GL parameter');return this.parameters.get(key);}
  depthMask(value){this.parameters.set(this.DEPTH_WRITEMASK,value);} depthFunc(value){this.parameters.set(this.DEPTH_FUNC,value);}
  blendFuncSeparate(src,dst,srcA,dstA){this.parameters.set(this.BLEND_SRC_RGB,src);this.parameters.set(this.BLEND_DST_RGB,dst);this.parameters.set(this.BLEND_SRC_ALPHA,srcA);this.parameters.set(this.BLEND_DST_ALPHA,dstA);}
  blendFunc(src,dst){this.blendFuncSeparate(src,dst,src,dst);}
  drawElementsInstanced(mode,count,type,offset,instances){
    assert.ok(this.vao.element?.data instanceof Uint32Array,'draw must retain its own element buffer');
    const inst=this.vao.attributes.get(3);assert.equal(inst.divisor,1);
    this.draws.push({vao:this.vao,indices:Array.from(this.vao.element.data),mode,count,type,offset,instances,
      instanceData:new Float32Array(inst.buffer.data),uniforms:new Map(this.uniforms),
      diffuse:this.bound(this.TEXTURE_2D,0),frames:this.bound(this.TEXTURE_2D_ARRAY,5),heightmap:this.bound(this.TEXTURE_2D,6),
      blend:this.isEnabled(this.BLEND),depthTest:this.isEnabled(this.DEPTH_TEST),state:new Map(this.parameters)});
  }
}

const positions=[0,0,0,1,0,0,0,1,0],normals=[0,1,0,0,1,0,0,1,0],uvs=[0,0,1,0,0,1];
const moving=positions.map((v,i)=>i%3===0?v+10:v);
const makePart=(name,indices,url,animated=true)=>({name,positions:[...positions],normals:[...normals],uvs:[...uvs],indices,
  textureUrl:url,color:[1,1,1,1],alpha:{flags:0,threshold:100},
  ...(animated?{animations:{idle:{duration:2,frames:[{positions:[...positions],normals:[...normals]},{positions:moving,normals:[1,0,0,1,0,0,1,0,0]}]}}}:{})});
const nativeA=()=>({...makePart('near-A',[0,1,2],urls.a),colors:[1,.5,.25,.75,1,.5,.25,.75,1,.5,.25,.75],
 material:{ambient:[.2,.3,.4],diffuse:[.6,.7,.8],specular:[.9,.8,.7],emissive:[.01,.02,.03],glossiness:24,alpha:.8},
 nativeProperties:{NiAlphaProperty:{Flags:4845,Threshold:125},NiSpecularProperty:{Flags:1},NiZBufferProperty:{Flags:3,Function:3},NiVertexColorProperty:{'Vertex Mode':2,'Lighting Mode':1}}});
const urls={a:'local-original/model-textures/test-a.png',b:'local-original/model-textures/test-b.png',c:'local-original/model-textures/test-c.png',team:'local-original/team-textures/test-a-A.png'};
const additionalResources=Array.from({length:10},(_,i)=>101+i);
const manifest={models:[{id:8,epoch:0,url:'local-original/models/test-near.json',lodFarUrl:'local-original/models/test-far.json',verified:true},
  {id:9,epoch:0,url:'local-original/models/test-static.json',verified:true},
  {id:100,epoch:0,url:'local-original/models/test-tree.json',lodFarUrl:'local-original/models/test-tree-far.json',yawOffset:Math.PI/4,verified:true},
  ...additionalResources.map(id=>({id,epoch:0,url:'local-original/models/test-resource-'+id+'.json',verified:true}))]};
const resourcePart=(small=false)=>({...makePart('canopy',[0,1,2],urls.c,false),positions:small?[0,1,0,1,1,0,0,2,0]:[0,1,0,2,1,0,0,3,0]});
const documents=new Map([
  ['local-original/models/manifest.json',manifest],
  ['local-original/team-textures/manifest.json',{colour_mapping_verified:true,texture_bindings:{[urls.a]:'test-a'},textures:{'test-a':{variants:{A:{verified:true,url:urls.team}}}}}],
  ['local-original/model-scales.json',{variants:{'9:0':{verified:true,sourceSha256:'native-static',sourceScale:.25}}}],
  ['local-original/models/test-near.json',{parts:[nativeA(),{...makePart('near-B',[2,1,0],urls.b),material:{ambient:[1,1,1],diffuse:[1,1,1],specular:[1,1,1],emissive:[0,0,0],glossiness:64,alpha:1},nativeProperties:{NiSpecularProperty:{Flags:0}}}]}],
  ['local-original/models/test-static.json',{source:{sha256:'native-static'},normalization:{scale:.5,offset:[-2,-4,1]},parts:[{...makePart('static-C',[0,2,1],urls.c,false),nativeProperties:{NiZBufferProperty:{Flags:1,Function:4}}}]}],
  ['local-original/models/test-far.json',{parts:[makePart('far-A',[1,0,2],urls.a)]}],
  ['local-original/models/test-tree.json',{parts:[resourcePart()]}],
  ['local-original/models/test-tree-far.json',{parts:[resourcePart(true)]}],
  ...additionalResources.map(id=>['local-original/models/test-resource-'+id+'.json',{parts:[resourcePart()]}]),
]);
const requests=[],deferred=new Map();
const response=key=>({ok:true,status:200,async json(){assert.ok(documents.has(key),`unexpected JSON ${key}`);return structuredClone(documents.get(key));},async blob(){return {url:key};}});
const originalFetch=globalThis.fetch,originalBitmap=globalThis.createImageBitmap;
globalThis.fetch=async input=>{
  const key=new URL(input).pathname.split('/docs/')[1];requests.push(key);
  if(key===urls.team)return new Promise(resolve=>deferred.set(key,()=>resolve(response(key))));
  assert.ok(documents.has(key)||Object.values(urls).includes(key),`unexpected asset ${key}`);return response(key);
};
globalThis.createImageBitmap=async blob=>({width:4,height:4,url:blob.url,close(){}});
const flush=()=>new Promise(resolve=>setImmediate(resolve));
const identity=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);
try{
  const disabled=await loadOriginalModels(new FakeGL(),false,[]);
  assert.equal(disabled.pickResource([0,0,0],[0,0,1]),null,'disabled/fallback renderer exposes a harmless picker');
  const gl=new FakeGL(),preserved=new Map();
  const initialParameters=new Map(gl.parameters),initialEnabled=new Set(gl.enabled);
  for(const unit of [1,2,3,4]){const tex=gl.createTexture(),target=unit===4?gl.TEXTURE_2D_ARRAY:gl.TEXTURE_2D;gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,tex);preserved.set(unit,{target,tex});}
  const renderer=await loadOriginalModels(gl,true,[[0,0,1],[1,0,0],[0,1,0],[1,1,0]]);
  assert.deepEqual(renderer.errors,[]);assert.deepEqual([...renderer.available],[8,9,100,...additionalResources]);
  assert.equal(renderer.pickResource([10.5,2,15],[0,0,-1]),null,'loaded models alone are not visible pick targets');
  assert.ok(!requests.includes('local-original/models/test-far.json'),'far geometry must not load eagerly');
  const near=renderer.models.get(8)[0],staticModel=renderer.models.get(9)[0];
  assert.deepEqual(Array.from(near.parts[0].vao.element.data),[0,1,2]);
  assert.deepEqual(Array.from(near.parts[1].vao.element.data),[2,1,0]);
  assert.deepEqual(Array.from(staticModel.parts[0].vao.element.data),[0,2,1]);
  assert.notEqual(near.parts[0].vao.element,near.parts[1].vao.element);
  assert.equal(near.parts[0].pickGeometry,null,'non-resource meshes do not retain picking indices');
  assert.equal(staticModel.parts[0].pickGeometry,null);
  assert.ok(renderer.models.get(100)[0].parts[0].pickGeometry);

  const data=new Float32Array([
    10,10,0,0, 8,0,.125,0, 1,0,0,0,
    12,10,0,0, 8,0,.375,0, 1,0,0,0,
    16,10,0,0, 9,1,0,0, 1,0,0,0,
  ]),ranges=new Uint32Array(512);ranges[16]=0;ranges[17]=2;ranges[18]=2;ranges[19]=1;
  const draw=(zoom=80)=>{gl.draws=[];renderer.draw({data,ranges,vp:identity,epochs:[0,0,0,0],zoom});return gl.draws;};
  let draws=draw();assert.equal(draws.length,3,'two phase-varied animated instances share one draw per part');
  const a=draws.find(d=>d.vao===near.parts[0].vao),b=draws.find(d=>d.vao===near.parts[1].vao),c=draws.find(d=>d.vao===staticModel.parts[0].vao);
  assert.equal(a.instances,2);assert.equal(b.instances,2);assert.equal(c.instances,1);
  assert.deepEqual(a.indices,[0,1,2]);assert.deepEqual(b.indices,[2,1,0]);assert.deepEqual(c.indices,[0,2,1]);
  assert.equal(a.uniforms.get('uFrames'),5);assert.equal(a.uniforms.get('uDiffuse'),0);assert.equal(a.uniforms.get('uFog'),2);
  assert.equal(a.uniforms.get('uAnimated'),1);assert.equal(c.uniforms.get('uAnimated'),0);
  assert.deepEqual(a.uniforms.get('uColor'),[.6,.7,.8,.8]);assert.deepEqual(a.uniforms.get('uAmbientMaterial'),[.2,.3,.4]);
  assert.deepEqual(a.uniforms.get('uSpecularMaterial'),[.9,.8,.7]);assert.deepEqual(a.uniforms.get('uEmissiveMaterial'),[.01,.02,.03]);
  assert.equal(a.uniforms.get('uGlossiness'),24);assert.equal(a.uniforms.get('uSpecularEnabled'),1);assert.equal(b.uniforms.get('uSpecularEnabled'),0,'nonblack specular colour does not imply enabled specular');
  assert.equal(a.uniforms.get('uVertexMode'),2);assert.equal(a.uniforms.get('uLightingMode'),1);assert.equal(a.uniforms.get('uHasAlpha'),3);assert.equal(a.uniforms.get('uAlphaFunction'),4);assert.equal(a.uniforms.get('uCutoff'),125/255);
  assert.equal(a.blend,true);assert.equal(a.state.get(gl.BLEND_SRC_RGB),gl.SRC_ALPHA);assert.equal(a.state.get(gl.BLEND_DST_RGB),gl.ONE_MINUS_SRC_ALPHA);
  assert.equal(c.instanceData[8],.5,'verified native class scale replaces the gameplay footprint multiplier');assert.deepEqual(c.uniforms.get('uOriginOffset'),[1,2,-.5]);
  assert.equal(b.blend,false,'opaque native parts disable blending');assert.equal(c.state.get(gl.DEPTH_WRITEMASK),false);assert.equal(c.state.get(gl.DEPTH_FUNC),gl.GREATER);
  assert.deepEqual(gl.parameters,initialParameters,'draw restores caller blend/depth state');assert.deepEqual(gl.enabled,initialEnabled);
  const colorAttribute=a.vao.attributes.get(6);assert.equal(colorAttribute.stride,48);assert.equal(colorAttribute.offset,32);
  assert.deepEqual(Array.from(colorAttribute.buffer.data.slice(8,12)),[1,.5,.25,.75]);
  assert.equal(a.uniforms.get('uTerrainEnabled'),0);
  assert.throws(()=>renderer.updateTerrain(new Float32Array([0]),3));assert.throws(()=>renderer.updateTerrain(new Float32Array([0,0,0,NaN]),2));
  const hill=new Float32Array([0,1,2,1,2,3,2,3,4]);renderer.updateTerrain(hill,3);draws=draw();
  const heightDraw=draws.find(d=>d.vao===near.parts[0].vao);assert.equal(heightDraw.uniforms.get('uTerrainEnabled'),1);assert.equal(heightDraw.uniforms.get('uTerrainSize'),3);assert.equal(heightDraw.uniforms.get('uTerrain'),6);
  assert.deepEqual(Array.from(heightDraw.heightmap.pixelData),Array.from(hill));assert.equal(heightDraw.heightmap.params.get(gl.TEXTURE_MIN_FILTER),gl.NEAREST);
  assert.equal(a.instanceData[9],.125);assert.equal(a.instanceData[21],.375,'per-instance seed/phase must survive grouping');
  assert.equal(a.frames.format,gl.RGBA32F);assert.equal(a.frames.depth,2);assert.equal(a.frames.params.get(gl.TEXTURE_MIN_FILTER),gl.NEAREST);
  const texels=a.frames.pixelData,layerStride=a.frames.width*a.frames.height*4;
  assert.deepEqual(Array.from(texels.slice(0,3)),[0,0,0]);assert.deepEqual(Array.from(texels.slice(4,7)),[0,1,0]);
  assert.deepEqual(Array.from(texels.slice(layerStride,layerStride+3)),[10,0,0]);assert.deepEqual(Array.from(texels.slice(layerStride+4,layerStride+7)),[1,0,0]);
  // Interpreting the public GPU texture layout yields distinct in-between poses.
  const firstX=phase=>{const f=(phase%1)*a.frames.depth,index=Math.floor(f),next=(index+1)%a.frames.depth;return texels[index*layerStride]*(1-f%1)+texels[next*layerStride]*(f%1);};
  assert.equal(firstX(a.instanceData[9]),2.5);assert.equal(firstX(a.instanceData[21]),7.5);
  assert.equal(a.diffuse.imageUrl,urls.a,'base material must remain usable while team texture is pending');
  assert.equal(requests.filter(k=>k===urls.team).length,1,'one deferred team request despite multiple instances');
  for(const [unit,{target,tex}] of preserved)assert.equal(gl.bound(target,unit),tex,`texture unit ${unit} must remain intact`);

  deferred.get(urls.team)();await flush();await flush();
  draws=draw();assert.equal(draws.find(d=>d.vao===near.parts[0].vao).diffuse.imageUrl,urls.team,'next frame adopts the resolved team material');
  for(const [unit,{target,tex}] of preserved)assert.equal(gl.bound(target,unit),tex,`asynchronous upload must preserve unit ${unit}`);

  draws=draw(20);assert.ok(requests.includes('local-original/models/test-far.json'),'first far visibility requests native far geometry');
  assert.ok(draws.some(d=>d.vao===near.parts[0].vao),'near geometry is drawn while far geometry loads');
  for(let i=0;i<16&&!near.far;i++)await flush();assert.ok(near.far,'far source eventually loads');
  draws=draw(20);assert.equal(draws.filter(d=>d.instances===2).length,1,'native far source replaces both near parts');
  assert.deepEqual(draws.find(d=>d.instances===2).indices,[1,0,2]);
  assert.equal(requests.filter(k=>k==='local-original/models/test-far.json').length,1,'far source is requested only once');
  draws=draw(80);assert.equal(draws.filter(d=>d.instances===2).length,2,'zooming back restores native near parts');
  assert.deepEqual(Array.from(near.parts[1].vao.element.data),[2,1,0],'asynchronous far creation must not overwrite near index buffers');
  for(const [unit,{target,tex}] of preserved)assert.equal(gl.bound(target,unit),tex,`LOD loading must preserve unit ${unit}`);
  assert.deepEqual(renderer.errors,[]);

  const prop=(id,x,y,{alt=0,rotation=id===100?-Math.PI/4:0,size=1}={})=>[x,y,alt,rotation,id,0,0,0,size,0,0,0];
  function props(instances,zoom=80,shadow=false){
    const rows=instances.toSorted((a,b)=>a[4]-b[4]),values=new Float32Array(rows.flat()),spans=new Uint32Array(512);
    rows.forEach((r,i)=>{if(!spans[r[4]*2+1])spans[r[4]*2]=i;spans[r[4]*2+1]++;});
    renderer.draw({data:values,ranges:spans,vp:identity,epochs:[0,0,0,0],zoom,shadow});return values;
  }
  props([prop(100,10,10)]);
  assert.deepEqual(renderer.pickResource([10.5,2,15],[0,0,-1]),[10,10],'ray through a raised crown selects its actual ground resource');
  assert.equal(renderer.pickResource([10.5,2,15],[0,0,-1],4),null,'terrain before the source triangle occludes the resource');
  assert.deepEqual(renderer.pickResource([10.5,2,15],[0,0,-1],6),[10,10],'resource before the terrain limit remains selectable');
  for(const id of additionalResources){
    props([prop(id,10,10)]);
    assert.ok(renderer.models.get(id)[0].parts[0].pickGeometry,`resource ${id} retains real picking geometry`);
    assert.deepEqual(renderer.pickResource([10.5,2,15],[0,0,-1]),[10,10],`resource ${id} can be selected through visible source geometry`);
  }
  props([prop(100,10,10)]);
  assert.equal(renderer.pickResource([11.9,2.9,15],[0,0,-1]),null,'inside the broad AABB but outside the source triangle is not a hit');
  assert.equal(renderer.pickResource([50,2,15],[0,0,-1]),null,'wrong ray misses');
  assert.equal(renderer.pickResource([10.5,2,15],[0,0,1]),null,'triangles behind the ray do not hit');
  assert.deepEqual(renderer.pickResource([10.5,2,5],[0,0,1]),[10,10],'visible double-sided triangles can be hit from either side');
  assert.equal(renderer.pickResource([NaN,2,15],[0,0,-1]),null);assert.equal(renderer.pickResource([10,2,15],[0,0,0]),null);
  props([prop(100,30,30)],80,true);
  assert.ok(gl.draws.slice(-1)[0].state.get(gl.DEPTH_WRITEMASK)===false,'shadow draw overrides native depth write');
  assert.deepEqual(gl.parameters,initialParameters,'shadow draw restores all caller blend/depth parameters');
  assert.deepEqual(renderer.pickResource([10.5,2,15],[0,0,-1]),[10,10],'shadow pass cannot replace last displayed resource instances');
  const reused=props([prop(100,10,10)]),pipSpans=new Uint32Array(512);
  pipSpans[201]=1;
  reused[0]=30;reused[1]=30;
  renderer.draw({data:reused,ranges:pipSpans,vp:identity,epochs:[0,0,0,0],capture:true});
  assert.deepEqual(renderer.pickResource([10.5,2,15],[0,0,-1]),[10,10],'PIP reuses instance memory without replacing main-view picks');
  renderer.clearResourcePicks();
  assert.equal(renderer.pickResource([10.5,2,15],[0,0,-1]),null,'empty primary viewport clears stale picks');
  props([prop(100,10,10,{alt:4,rotation:Math.PI/4,size:2})]);
  assert.deepEqual(renderer.pickResource([15,7,11],[-1,0,0]),[10,10],'instance scale, altitude and instance+model yaw all invert correctly');
  assert.deepEqual(renderer.pickResource([15,7,11],[-2,0,0]),[10,10],'direction need not be normalized');
  props([prop(100,10,10),prop(101,10,12)]);
  assert.deepEqual(renderer.pickResource([10.5,2,20],[0,0,-1]),[10,12],'nearest source triangle wins across resource kinds');
  props([prop(102,15,20,{size:2}),prop(103,15,22)]);
  assert.deepEqual(renderer.pickResource([15.5,2,30],[0,0,-1]),[15,22],'different instance scales keep comparable world-ray distances');
  props([prop(100,10,10)],20);
  const resourceVariant=renderer.models.get(100)[0];
  for(let i=0;i<16&&!resourceVariant.far;i++)await flush();assert.ok(resourceVariant.far);
  assert.deepEqual(renderer.pickResource([11.5,1.2,15],[0,0,-1]),[10,10],'newly delivered LOD cannot change picking until it has been displayed');
  props([prop(100,10,10)],20);
  assert.equal(renderer.pickResource([11.5,1.2,15],[0,0,-1]),null,'displayed far geometry is picked instead of the old near silhouette');
  assert.deepEqual(renderer.pickResource([10.25,1.25,15],[0,0,-1]),[10,10]);
  resourceVariant.nativeScale=.5;resourceVariant.originOffset=[2,2,-1];
  props([prop(100,10,10,{size:2})],120);
  assert.deepEqual(renderer.pickResource([11.25,1.75,15],[0,0,-1],6),[10,10],'native scale and source origin use the same inverse as the GPU transform');
  assert.equal(renderer.pickResource([11.25,1.75,15],[0,0,-1],5.25),null,'restored source-origin depth participates in terrain occlusion limits');
  resourceVariant.nativeScale=null;resourceVariant.originOffset=[0,0,0];
  props([]);assert.equal(renderer.pickResource([10.25,1.25,15],[0,0,-1]),null,'removed/depleted resources disappear from click targets on the next color frame');
  console.log('PASS original renderer: VAO/index isolation, native morph/LOD/team textures, exact material/source-state uniforms and GL restoration, height-map upload, native class scale/origins and inverse resource picking');
}finally{globalThis.fetch=originalFetch;globalThis.createImageBitmap=originalBitmap;}
