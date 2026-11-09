import {originalWaterBodyUV} from './native-water-body-uv.js?v=native-water-sampling232';
import {createReflectionOverlay,waterStencilScope} from './reflection-overlay.js?v=native-water-presentation231';
import {sourceWaterIndices} from './reflection-scene.js?v=native-water-presentation231';
// Native high-water equations: see native-water-appearance-evidence.json.
// Activated after the frozen visual-seasons2 build. UV scales and own-map mapping
// are explicit adapters; source media, kernels and shader channel roles are exact.
const f32=Math.fround,INV255=f32(1/255),MEAN13=f32(1/13),MEAN33=f32(1/33),MEAN29=f32(1/29);
const CARDINAL=[[1,0],[-1,0],[0,1],[0,-1]],RING=[...CARDINAL,[-1,-1],[1,1],[-1,1],[1,-1]];
const COLOR13=[[0,0],...RING,...CARDINAL.map(([x,z])=>[x*2,z*2])];
const OCEAN33=[[0,0],...[1,2,3,4].flatMap(r=>RING.map(([x,z])=>[x*r,z*r]))];
const COAST29=[[0,0,5],...RING.map(([x,z])=>[x,z,2]),...RING.map(([x,z])=>[x*2,z*2,1])];
export const OWN_WATER_CLASS_ADAPTER=Object.freeze([7,5,0,0,4,3]);
const checkArray=(a,n)=>a?.length===n&&Array.from(a).every(Number.isFinite);

/** Native vertex channel filters on this adapter's corner grid. */
export function buildNativeWaterData({heights,types,movement,size=256,waterHeight=0,paletteAlphaBytes,lookupRGBBytes,uvScale=[1/12,1/12],classMapping=OWN_WATER_CLASS_ADAPTER,sourceUV=false}){
 if(!Number.isInteger(size)||size<1||size>1024||!checkArray(heights,(size+1)**2)||!Number.isFinite(waterHeight)||!checkArray(uvScale,2))throw new RangeError('Invalid water height grid');
 if(!checkArray(paletteAlphaBytes,256)||paletteAlphaBytes.some(n=>!Number.isInteger(n)||n<0||n>255)||lookupRGBBytes?.length<3||lookupRGBBytes.slice(0,3).some(rgb=>!checkArray(rgb,3)||rgb.some(n=>!Number.isInteger(n)||n<0||n>255)))throw new RangeError('Invalid original water lookup data');
 if(movement){if(!checkArray(movement,size*size)&&!checkArray(movement,(size+1)**2))throw new RangeError('Invalid native water movement field');if(movement.some(n=>!Number.isInteger(n)||n<0||n>7))throw new RangeError('Unknown native movement class');}
 else if(!checkArray(types,size*size)||!checkArray(classMapping,6)||types.some(n=>!Number.isInteger(n)||n<0||n>5)||classMapping.some(n=>!Number.isInteger(n)||n<0||n>7))throw new RangeError('Invalid own-map water class adapter');
 const width=size+1,n=width*width,raw=new Float32Array(n*4),vertices=new Float32Array(n*8),indices=[];
 const point=(x,z)=>Math.max(0,Math.min(size,z))*width+Math.max(0,Math.min(size,x));
 const tile=(x,z)=>Math.min(size-1,z)*size+Math.min(size-1,x);
 for(let z=0;z<=size;z++)for(let x=0;x<=size;x++){
  const i=z*width+x,h=heights[i],kind=movement?(movement.length===n?movement[i]:movement[tile(x,z)]):classMapping[types[tile(x,z)]];
  const rgb=lookupRGBBytes[h>=0?2:kind===7?0:kind===5?2:1],depth=waterHeight-h;
  const paletteIndex=depth<=0?0:depth>=2?255:Math.trunc(depth/2*255);
  raw.set([f32(rgb[0]*INV255),f32(paletteAlphaBytes[paletteIndex]*INV255),kind===7?1:kind===5?.5:0,h>=0?.5:kind===5?f32(.35):0],i*4);
 }
 for(let z=0;z<=size;z++)for(let x=0;x<=size;x++){
  let red=0,alpha=0,green=0,blue=0;
  for(const [dx,dz] of COLOR13){const j=point(x+dx,z+dz)*4;red+=raw[j];alpha+=raw[j+1];}
  for(const [dx,dz] of OCEAN33)green+=raw[point(x+dx,z+dz)*4+2];
  for(const [dx,dz,weight] of COAST29)blue+=raw[point(x+dx,z+dz)*4+3]*weight;
  let u,v;
  if(sourceUV){
   [u,v]=originalWaterBodyUV(x,size-z);
  }else{
   const warp=Math.cos((x+z)*.25)*f32(.03);
   u=x*uvScale[0]+warp;v=z*uvScale[1]-warp;
  }
  vertices.set([x,z,f32(red*MEAN13),f32(green*MEAN33),f32(blue*MEAN29),f32(alpha*MEAN13),u,v],(z*width+x)*8);
 }
 for(let z=0;z<size;z++)for(let x=0;x<size;x++){
  const a=z*width+x,b=a+1,c=a+width,d=c+1;
  if([a,b,d].some(i=>heights[i]<waterHeight))indices.push(a,b,d);
  if([a,d,c].some(i=>heights[i]<waterHeight))indices.push(a,d,c);
 }
 return {vertices,indices:new Uint32Array(indices),raw,classification:movement?'explicit-native-movement-ids':'own-archipelago-class-adapter',size,waterHeight};
}

const BASE_VS=`#version 300 es
layout(location=0) in vec2 aXZ;layout(location=1) in vec4 aColor;layout(location=2) in vec2 aUV;
uniform mat4 uVP;uniform float uWaterHeight;out vec4 vColor;out vec2 vUV;out vec2 vWorld;
void main(){vColor=aColor;vUV=aUV;vWorld=aXZ;gl_Position=uVP*vec4(aXZ.x,uWaterHeight,aXZ.y,1.);}`;
const FOG=`float visibility=texture(uFog,clamp(vWorld/256.,0.,1.)).r;if(visibility<.015)discard;
color.rgb*=smoothstep(.02,.45,visibility)*mix(.5,1.,smoothstep(.55,.95,visibility));`;
export const NATIVE_WATER_SHADERS={base:{vertex:BASE_VS,fragment:`#version 300 es
precision highp float;uniform sampler2D uDark,uDetail,uGloss,uFog;uniform vec2 uTime;uniform int uDetailLevel;
in vec4 vColor;in vec2 vUV;in vec2 vWorld;out vec4 outColor;
void main(){
 vec2 uv0=vUV+uTime*vec2(.0155,.02),uv1=vUV+uTime*vec2(-.015,.01);
 vec4 color;
 if(uDetailLevel==0){
   color=texture(uDark,vUV);
   color.a*=vColor.a;
 }else if(uDetailLevel==1){
   color=texture(uDark,uv0)*texture(uDetail,uv1)*vColor.rrra;
 }else{
   vec4 a=texture(uDark,uv0)*texture(uDetail,uv1),b=texture(uGloss,uv0)*texture(uGloss,uv1);
   vec4 mixed=mix(b,a,vColor.g);color=vec4(mixed.rgb*vColor.r,mixed.a*vColor.a);color.gb+=vec2(vColor.b);
 }
 ${FOG}outColor=color;
}`},reflection:{vertex:`#version 300 es
layout(location=0) in vec2 aXZ;uniform mat4 uVP,uReflectionVP;uniform float uWaterHeight;uniform vec2 uNormalScale;
out vec2 vWorld;out vec2 vNormalUV;out vec4 vProjected;
void main(){vec4 world=vec4(aXZ.x,uWaterHeight,aXZ.y,1.);vWorld=aXZ;vNormalUV=aXZ*uNormalScale;vProjected=uReflectionVP*world;gl_Position=uVP*world;}`,
fragment:`#version 300 es
precision highp float;uniform sampler2D uNormal,uReflection,uFog;uniform float uTime,uReflectionAlpha;
in vec2 vWorld;in vec2 vNormalUV;in vec4 vProjected;out vec4 outColor;
void main(){
 if(vProjected.w<=0.)discard;
 vec2 uv0=vNormalUV+fract(uTime*vec2(.01125,.02)),uv1=(vNormalUV+fract(uTime*vec2(-.01125,.01))).yx;
 vec2 offset=(texture(uNormal,uv0).rg+texture(uNormal,uv1).rg-1.)*.1;
 vec2 reflected=vProjected.xy/vProjected.w*.5+.5;
 vec4 color=texture(uReflection,clamp(reflected+offset,vec2(.001),vec2(.999)))*vec4(1.,1.,1.,uReflectionAlpha);
 ${FOG}outColor=color;
}`}};

function program(gl,sources){
 const p=gl.createProgram();for(const [type,source] of [[gl.VERTEX_SHADER,sources.vertex],[gl.FRAGMENT_SHADER,sources.fragment]]){
  const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));gl.attachShader(p,s);gl.deleteShader(s);
 }gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(p));return {p,u:name=>gl.getUniformLocation(p,name)};
}
async function imageTexture(gl,url){
 const response=await fetch(url);if(!response.ok)throw Error(`Native water image${response.status}`);
 const image=await createImageBitmap(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'}),texture=gl.createTexture();
 gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
 gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
 for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.LINEAR);
 for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,key,gl.REPEAT);
 image.close();return texture;
}
const same=(a,b)=>a==null||b==null?a===b:a.length===b.length&&a.every((value,i)=>value===b[i]);
function state(gl){return {blend:gl.isEnabled(gl.BLEND),depth:gl.isEnabled(gl.DEPTH_TEST),write:gl.getParameter(gl.DEPTH_WRITEMASK),func:gl.getParameter(gl.DEPTH_FUNC),src:gl.getParameter(gl.BLEND_SRC_RGB),dst:gl.getParameter(gl.BLEND_DST_RGB),srcA:gl.getParameter(gl.BLEND_SRC_ALPHA),dstA:gl.getParameter(gl.BLEND_DST_ALPHA),active:gl.getParameter(gl.ACTIVE_TEXTURE)};}
function restore(gl,s){for(const [cap,value] of [[gl.BLEND,s.blend],[gl.DEPTH_TEST,s.depth]])(value?gl.enable:gl.disable).call(gl,cap);gl.depthMask(s.write);gl.depthFunc(s.func);gl.blendFuncSeparate(s.src,s.dst,s.srcA,s.dstA);gl.activeTexture(s.active);}

export async function loadOriginalWater(gl,enabled,{manifestURL=new URL('./local-original/water-native/manifest.json',import.meta.url)}={}){
 const result={enabled:false,errors:[],metrics:{baseCalls:0,reflectionCalls:0,triangles:0},updateMap(){return false;},drawWater(){},drawBase(){},drawReflection(){},dispose(){}};
 if(!enabled)return result;
 try{
  const response=await fetch(manifestURL);if(!response.ok)throw Error(`Native water manifest${response.status}`);const manifest=await response.json();
  if(manifest.format!=='ee2-native-water-v1')throw Error('Unsupported native water manifest');result.manifest=manifest;
  const textures={};for(const key of ['skin1','skin2','skin3','skin4','skin5','skin6','normal'])textures[key]=await imageTexture(gl,new URL(manifest.images[key].url,manifestURL));
  const base=program(gl,NATIVE_WATER_SHADERS.base),reflection=program(gl,NATIVE_WATER_SHADERS.reflection),vao=gl.createVertexArray(),vbo=gl.createBuffer(),ibo=gl.createBuffer();
  gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vbo);for(const [loc,size,offset] of [[0,2,0],[1,4,8],[2,2,24]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,size,gl.FLOAT,false,32,offset);}gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ibo);
  let fields=null,last=null,sourceOverlay=null;
  const sourceSlots=new Map();
  result.updateMap=options=>{
   const settings={size:256,waterHeight:0,...options};if(last&&last.size===settings.size&&last.waterHeight===settings.waterHeight&&last.sourceUV===settings.sourceUV&&same(last.heights,settings.heights)&&same(last.types,settings.types)&&same(last.movement,settings.movement))return false;
   fields=buildNativeWaterData({...settings,paletteAlphaBytes:manifest.paletteAlphaBytes,lookupRGBBytes:manifest.lookupRGBBytes,uvScale:manifest.adapter.uvScale,classMapping:manifest.adapter.ownClassToMovement});
   gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vbo);gl.bufferData(gl.ARRAY_BUFFER,fields.vertices,gl.STATIC_DRAW);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ibo);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,fields.indices,gl.STATIC_DRAW);
   for(const slot of sourceSlots.values())slot.key=undefined;
   last={...settings,heights:new Float32Array(settings.heights),types:settings.types&&new Uint8Array(settings.types),movement:settings.movement&&new Uint8Array(settings.movement)};
   result.metrics.triangles=fields.indices.length/3;result.metrics.classification=fields.classification;return true;
  };
  const bind=(unit,t)=>{gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,t);};
  const prepare=(p,vp)=>{gl.useProgram(p.p);gl.bindVertexArray(vao);gl.uniformMatrix4fv(p.u('uVP'),false,vp);gl.uniform1f(p.u('uWaterHeight'),fields.waterHeight);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);};
  result.drawBase=({vp,time=0,timeVector=[time,time],fogTexture,profile='high',source=null,detail=2})=>{
   if(!fields?.indices.length)return;if(!Object.hasOwn(manifest.profiles,profile)||!checkArray(timeVector,2))throw new RangeError('Invalid native water material input');const before=state(gl);
   try{prepare(base,vp);if(source)gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,source.buffer);gl.depthMask(true);gl.uniform2fv(base.u('uTime'),timeVector);
    gl.uniform1i(base.u('uDetailLevel'),detail);
    const inputs=detail===0?{dark:'skin3',detail:'skin2',gloss:'skin4'}:manifest.profiles[profile];
    for(const [role,unit] of [['dark',0],['detail',4],['gloss',5]]){bind(unit,textures[inputs[role]]);gl.uniform1i(base.u('u'+role[0].toUpperCase()+role.slice(1)),unit);}
    if(fogTexture)bind(2,fogTexture);gl.uniform1i(base.u('uFog'),2);gl.drawElements(gl.TRIANGLES,source?.count??fields.indices.length,gl.UNSIGNED_INT,0);result.metrics.baseCalls++;
   }finally{if(source)gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ibo);restore(gl,before);}
  };
  result.drawReflection=({vp,time=0,fogTexture,reflectionTexture,reflectionVP=vp})=>{
   if(!fields?.indices.length||!reflectionTexture)return;if(!Number.isFinite(time))throw new RangeError('Invalid reflection time');const before=state(gl);
   try{prepare(reflection,vp);gl.depthMask(false);gl.uniformMatrix4fv(reflection.u('uReflectionVP'),false,reflectionVP);gl.uniform1f(reflection.u('uTime'),time);gl.uniform1f(reflection.u('uReflectionAlpha'),manifest.native.reflectionAlpha);gl.uniform2fv(reflection.u('uNormalScale'),manifest.adapter.normalUvScale);
    bind(0,textures.normal);bind(7,reflectionTexture);if(fogTexture)bind(2,fogTexture);gl.uniform1i(reflection.u('uNormal'),0);gl.uniform1i(reflection.u('uReflection'),7);gl.uniform1i(reflection.u('uFog'),2);
    gl.drawElements(gl.TRIANGLES,fields.indices.length,gl.UNSIGNED_INT,0);result.metrics.reflectionCalls++;
   }finally{restore(gl,before);}
  };
  result.drawSourceWater=({role,bodies,grid,vp,time,fogTexture,detail,reflectionTime,reflectionTexture})=>{
   const metrics=result.metrics.source??={};
   if(!bodies.length){metrics[role]={bodies:0,base:0,grid:0};return;}
   let slot=sourceSlots.get(role);
   if(!slot){slot={buffer:gl.createBuffer()};sourceSlots.set(role,slot);}
   const key=bodies.map(body=>body.id).join(',');
   if(slot.key!==key){
    const data=sourceWaterIndices(bodies,fields.size),previousVao=gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    gl.bindVertexArray(vao);
    try{gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,slot.buffer);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,data,gl.DYNAMIC_DRAW);}
    finally{gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ibo);gl.bindVertexArray(previousVao);}
    slot.count=data.length;slot.key=key;
   }
   const overlayCount=waterStencilScope(gl,({mark,select})=>{
    mark();result.drawBase({vp,time,fogTexture,profile:'high',source:slot,detail});
    if(!grid||!reflectionTexture)return 0;
    select();sourceOverlay??=createReflectionOverlay(gl,textures.normal);
    return sourceOverlay.draw({grid,vp,mapHeight:fields.size,time:reflectionTime,texture:reflectionTexture});
   });
   metrics[role]={bodies:bodies.length,base:slot.count,grid:overlayCount,detail,reflectionTime};
  };
  result.drawWater=options=>{result.drawBase({...options,profile:options.reflectionTexture?'high':'fallback'});result.drawReflection(options);};
  result.dispose=()=>{for(const slot of sourceSlots.values())gl.deleteBuffer(slot.buffer);sourceOverlay?.dispose();for(const texture of Object.values(textures))gl.deleteTexture(texture);gl.deleteProgram(base.p);gl.deleteProgram(reflection.p);gl.deleteBuffer(vbo);gl.deleteBuffer(ibo);gl.deleteVertexArray(vao);fields=null;last=null;result.enabled=false;};
  result.enabled=true;
 }catch(error){result.errors.push(error.message);}
 return result;
}
