import {normalizeRenderAssetMapping,sourceRenderBindings,manifestAssetKind} from './original-render-namespace.js';
export {createRenderAssetMapping} from './original-render-namespace.js';
// Textured NIF geometry converted locally by tools/extract_ee2_models.py.
// This renderer does not inspect original files or alter the deterministic simulation.
import {DEFAULT_MODEL_LIGHTING,modelMaterial,alphaState,depthState} from './original-material.js?v=material1';
import {nativeModelTransform} from './original-scale.js?v=native1';
import {modelYearPhase,modelSeasonFrames,modelSeasonBinding} from './original-model-seasons.js?v=modelseason1';
import {createNativeLightingContext,nativeModelPipeline} from './original-lighting.js?v=lighting6';
import {appendInitialSailParts,nativeInitialSailDiffuse} from './original-sail-composite.js?v=initialsail7';
import {createDynamicSailBinding,createDynamicSailGPU,dynamicSailForces} from './original-dynamic-sails.js?v=clothgpu7';
import {appendNativePackAnimations,nativePackPose} from './original-pack-states.js?v=pack8';
const VS=`#version 300 es
layout(location=0) in vec3 aPosition;layout(location=1) in vec3 aNormal;layout(location=2) in vec2 aUV;
layout(location=3) in vec4 iA;layout(location=4) in vec4 iB;layout(location=5) in vec4 iC;
layout(location=6) in vec4 aColor;
uniform mat4 uVP;uniform highp int uShadow;uniform bool uAnimated;uniform float uYOffset;uniform vec3 uOriginOffset;
uniform highp sampler2DArray uFrames;uniform int uFrameWidth;uniform int uFrameCount;uniform bool uAnimationOnce;
uniform highp sampler2D uTerrain;uniform int uTerrainSize;uniform bool uTerrainEnabled;uniform vec3 uSunDirection;
uniform bool uFarmPlotConform;uniform vec3 uFarmPlotCenter;
uniform highp int uNativeShader;uniform vec3 uAmbientLight;uniform vec3 uSunColor;uniform vec4 uColor;
out vec4 vNativeLighting;out vec3 vNormal;out vec2 vUV;out vec2 vWorld;out vec3 vPosition;out vec4 vColor;out float vY;flat out vec4 vInfo;
float terrainAt(vec2 p){
 vec2 sampleAt=clamp(p,vec2(0.),vec2(float(uTerrainSize-1)));ivec2 a=ivec2(floor(sampleAt));ivec2 b=min(a+1,ivec2(uTerrainSize-1));vec2 f=fract(sampleAt);
 float lo=mix(texelFetch(uTerrain,a,0).r,texelFetch(uTerrain,ivec2(b.x,a.y),0).r,f.x);
 float hi=mix(texelFetch(uTerrain,ivec2(a.x,b.y),0).r,texelFetch(uTerrain,b,0).r,f.x);return mix(lo,hi,f.y);
}
float plotGround(vec2 p){
 vec2 q=clamp(p,vec2(0.),vec2(float(uTerrainSize-1)-.0001));ivec2 i=ivec2(floor(q));vec2 f=fract(q);
 float a=texelFetch(uTerrain,i,0).r,b=texelFetch(uTerrain,i+ivec2(1,0),0).r;
 float c=texelFetch(uTerrain,i+ivec2(0,1),0).r,d=texelFetch(uTerrain,i+ivec2(1,1),0).r;
 return f.x+f.y<=1.?a+(b-a)*f.x+(c-a)*f.y:d+(c-d)*(1.-f.x)+(b-d)*(1.-f.y);
}
void main(){
 vec3 local=aPosition,normal=aNormal;
 if(uAnimated){
  float phase=uAnimationOnce?clamp(iC.y,0.,1.)*float(uFrameCount-1):fract(iC.y)*float(uFrameCount);int frame=int(floor(phase));int next=uAnimationOnce?min(frame+1,uFrameCount-1):(frame+1)%uFrameCount;
  int pixel=gl_VertexID*2;ivec2 uv=ivec2(pixel%uFrameWidth,pixel/uFrameWidth);
  vec3 p0=texelFetch(uFrames,ivec3(uv,frame),0).xyz,p1=texelFetch(uFrames,ivec3(uv,next),0).xyz;
  uv.x+=1;vec3 n0=texelFetch(uFrames,ivec3(uv,frame),0).xyz,n1=texelFetch(uFrames,ivec3(uv,next),0).xyz;
  local=mix(p0,p1,fract(phase));normal=mix(n0,n1,fract(phase));
 }
 local.y+=uYOffset;
 float constructionY=local.y;local+=uOriginOffset;
 float c=cos(iA.w),s=sin(iA.w);vec3 p=local*iC.x;
 vec3 world=vec3(iA.x+p.x*c-p.z*s,iA.z+p.y,iA.y+p.x*s+p.z*c);
 if(uFarmPlotConform && uTerrainEnabled && uTerrainSize>1){
  vec3 center=(uFarmPlotCenter+uOriginOffset)*iC.x;
  vec2 at=vec2(iA.x+center.x*c-center.z*s,iA.y+center.x*s+center.z*c);
  //FarmPlotSM0061D4C0 samples each plot center and translates Z only.
  //The triangle interpolation here follows this game's actual terrain mesh.
  world.y+=plotGround(at)-iA.z-center.y;
 }
 if(uShadow==1){
  vec3 source=world;vec2 direction=uSunDirection.xz/max(.1,uSunDirection.y);
  if(uTerrainEnabled){
   // Own terrain-conforming projector, not a claim of the native shadow algorithm.
   for(int step=0;step<3;step++)world.xz=source.xz-direction*max(0.,source.y-terrainAt(world.xz));
   world.y=terrainAt(world.xz)+.035;
  }else{world.xz-=direction*max(p.y,0.);world.y=iA.z+.035;}
 }
 vNormal=vec3(normal.x*c-normal.z*s,normal.y,normal.x*s+normal.z*c);
 // IndexPal20_High/Low.vsh: signed normal dot or constant .5, then the
 // vertex diffuse register is clamped before raster interpolation.
 float lightDot=uNativeShader==2?.5:dot(normalize(vNormal),normalize(uSunDirection));
 vNativeLighting=vec4(clamp(uAmbientLight+lightDot*uSunColor,0.,1.),uColor.a);
 vUV=aUV;vWorld=world.xz;vPosition=world;vColor=aColor;vY=constructionY;vInfo=iB;
 gl_Position=uVP*vec4(world,1.);
}`;
const FS=`#version 300 es
precision highp float;uniform sampler2D uDiffuse;uniform sampler2D uDiffuseNext;uniform float uModelSeasonBlend;uniform sampler2D uFog;uniform vec4 uColor;
uniform highp int uShadow;uniform int uHasAlpha;uniform float uCutoff;uniform float uModelHeight;uniform vec3 uTeam[8];
uniform int uAlphaFunction;uniform vec3 uAmbientMaterial;uniform vec3 uSpecularMaterial;uniform vec3 uEmissiveMaterial;
uniform float uGlossiness;uniform bool uSpecularEnabled;uniform int uVertexMode;uniform int uLightingMode;
uniform vec3 uAmbientLight;uniform vec3 uSunColor;uniform vec3 uSunDirection;uniform vec3 uEye;
uniform vec3 uSpecularLight;uniform highp int uNativeShader;uniform float uTextureScale;uniform bool uNativeLighting;
uniform bool uClipEnabled;uniform vec4 uClipPlane;
in vec4 vNativeLighting;in vec3 vNormal;in vec2 vUV;in vec2 vWorld;in vec3 vPosition;in vec4 vColor;in float vY;flat in vec4 vInfo;out vec4 outColor;
bool alphaPass(float alpha){
 if(uAlphaFunction==0)return true;if(uAlphaFunction==1)return alpha<uCutoff;if(uAlphaFunction==2)return alpha==uCutoff;
 if(uAlphaFunction==3)return alpha<=uCutoff;if(uAlphaFunction==4)return alpha>uCutoff;if(uAlphaFunction==5)return alpha!=uCutoff;
 if(uAlphaFunction==6)return alpha>=uCutoff;return false;
}
void main(){
 if(uClipEnabled && dot(vec4(vPosition,1.),uClipPlane)<0.)discard;
 vec4 texel=mix(texture(uDiffuse,vUV),texture(uDiffuseNext,vUV),uModelSeasonBlend);float coverage=texel.a*(uNativeShader!=0?vNativeLighting.a:uColor.a*(uVertexMode==2?vColor.a:1.));
 if((uHasAlpha&1)!=0 && !alphaPass(coverage))discard;
 int flags=int(vInfo.w+.5);float fog=texture(uFog,vWorld/256.).r;
 if((flags&32)==0 && fog<.05)discard;
 if((flags&64)!=0 && vInfo.z<1. && vY>max(.02,uModelHeight*vInfo.z))discard;
 if(uShadow==1){if((flags&24)!=0||fog<.15||((uHasAlpha&2)!=0&&coverage<.05))discard;outColor=vec4(.018,.025,.014,.2);return;}
 vec3 n=normalize(vNormal),sun=normalize(uSunDirection);float diffuseLight=max(dot(n,sun),0.);
 vec3 ambient=uVertexMode==2?vColor.rgb:uAmbientMaterial,diffuse=uVertexMode==2?vColor.rgb:uColor.rgb;
 vec3 emission=uVertexMode==1?vColor.rgb:uEmissiveMaterial;
 // The explicit native IndexPal shader ignores material RGB and specular.
 // Static material illumination retains the portable adapter; its source
 // texture stage and separate specular-light colour are now applied exactly.
 vec3 primary=emission;
 if(uLightingMode!=0)primary+=ambient*uAmbientLight+diffuse*uSunColor*diffuseLight;
 if(uNativeLighting)primary=clamp(primary,0.,1.);
 vec3 color=texel.rgb*(uNativeShader!=0?vNativeLighting.rgb:primary)*uTextureScale;
 if(uNativeLighting)color=clamp(color,0.,1.);
 if(uNativeShader==0 && uLightingMode!=0){
  if(uSpecularEnabled && diffuseLight>0.){
   vec3 halfAngle=normalize(sun+normalize(uEye-vPosition));
   float specular=uGlossiness==0.?1.:pow(max(dot(n,halfAngle),0.),uGlossiness);
   color+=uSpecularMaterial*uSpecularLight*specular;
  }
 }
 if(uNativeLighting)color=clamp(color,0.,1.);
 if((flags&4)!=0)color=mix(color,vec3(1.),.5);
 float alpha=(uHasAlpha&2)!=0?coverage:1.;
 if((flags&8)!=0){color=mix(color,vec3(.3,1.,.4),.4);alpha=.6;}
 if((flags&16)!=0){color=mix(color,vec3(1.,.15,.15),.65);alpha=.6;}
 if((flags&32)==0)color*=smoothstep(.02,.45,fog)*mix(.5,1.,smoothstep(.55,.95,fog));
 outColor=vec4(color,alpha);
}`;
export const ORIGINAL_MODEL_SHADERS=Object.freeze({vertex:VS,fragment:FS});
export const MODEL_SHADERS=ORIGINAL_MODEL_SHADERS;
// FlagShader.NSF uses its own fixed-function stage sequence. This branch never
// changes the existing static/skinned material shader or its texture operands.
const SAIL_FS=`#version 300 es
precision highp float;
uniform sampler2D uDiffuse,uDark,uFog;uniform vec3 uSailDiffuse;
uniform bool uClipEnabled;uniform vec4 uClipPlane;uniform float uModelHeight;
in vec2 vUV;in vec2 vWorld;in vec3 vPosition;in float vY;flat in vec4 vInfo;out vec4 outColor;
void main(){
 if(uClipEnabled&&dot(vec4(vPosition,1.),uClipPlane)<0.)discard;
 vec4 base=texture(uDiffuse,vUV),dark=texture(uDark,vUV);
 float coverage=base.a*dark.a;if(coverage<=0.)discard;
 int flags=int(vInfo.w+.5);float fog=texture(uFog,vWorld/256.).r;
 if((flags&32)==0&&fog<.05)discard;
 if((flags&64)!=0&&vInfo.z<1.&&vY>max(.02,uModelHeight*vInfo.z))discard;
 vec3 color=clamp(base.rgb*uSailDiffuse*4.,0.,1.)*dark.rgb;
 if((flags&4)!=0)color=mix(color,vec3(1.),.5);
 if((flags&8)!=0){color=mix(color,vec3(.3,1.,.4),.4);coverage=.6;}
 if((flags&16)!=0){color=mix(color,vec3(1.,.15,.15),.65);coverage=.6;}
 if((flags&32)==0)color*=smoothstep(.02,.45,fog)*mix(.5,1.,smoothstep(.55,.95,fog));
 outColor=vec4(color,coverage);
}`;
export const ORIGINAL_INITIAL_SAIL_SHADERS=Object.freeze({vertex:VS,fragment:SAIL_FS});
// The source CPU color routine stores unclamped floats. This explicit float
// vertex adapter retains them until the Flag texture-stage clamp. Native GPU
// color packing/quantization is not claimed by this port.
const DYNAMIC_SAIL_FS=SAIL_FS.replace('in vec2 vUV;', 'in vec4 vColor;in vec2 vUV;').replace('base.rgb*uSailDiffuse*4.', 'base.rgb*vColor.rgb*4.');
export const ORIGINAL_DYNAMIC_SAIL_SHADERS=Object.freeze({vertex:VS,fragment:DYNAMIC_SAIL_FS});
const enabled=(gl,cap,value)=>value?gl.enable(cap):gl.disable(cap);
function drawState(gl){
 return {blend:gl.isEnabled(gl.BLEND),test:gl.isEnabled(gl.DEPTH_TEST),write:gl.getParameter(gl.DEPTH_WRITEMASK),depth:gl.getParameter(gl.DEPTH_FUNC),
  src:gl.getParameter(gl.BLEND_SRC_RGB),dst:gl.getParameter(gl.BLEND_DST_RGB),srcAlpha:gl.getParameter(gl.BLEND_SRC_ALPHA),dstAlpha:gl.getParameter(gl.BLEND_DST_ALPHA)};
}
function restoreDrawState(gl,state){
 enabled(gl,gl.BLEND,state.blend);enabled(gl,gl.DEPTH_TEST,state.test);gl.depthMask(state.write);gl.depthFunc(state.depth);
 gl.blendFuncSeparate(state.src,state.dst,state.srcAlpha,state.dstAlpha);
}
function applyDrawState(gl,part,shadow,preview){
 const a=part.alpha,d=part.depth;
 enabled(gl,gl.DEPTH_TEST,shadow||d.test);gl.depthMask(!shadow&&!preview&&d.write);
 gl.depthFunc([gl.ALWAYS,gl.LESS,gl.EQUAL,gl.LEQUAL,gl.GREATER,gl.NOTEQUAL,gl.GEQUAL,gl.NEVER][d.function]??gl.LEQUAL);
 enabled(gl,gl.BLEND,shadow||preview||a.blend);
 if(shadow||preview){gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);return;}
 if(a.blend){
  const factors=[gl.ONE,gl.ZERO,gl.SRC_COLOR,gl.ONE_MINUS_SRC_COLOR,gl.DST_COLOR,gl.ONE_MINUS_DST_COLOR,gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.DST_ALPHA,gl.ONE_MINUS_DST_ALPHA,gl.SRC_ALPHA_SATURATE];
  gl.blendFunc(factors[a.source]??gl.SRC_ALPHA,factors[a.destination]??gl.ONE_MINUS_SRC_ALPHA);
 }
}
function program(gl,fragment=FS,extraUniforms=[]){
 const p=gl.createProgram();
 for(const [type,source] of [[gl.VERTEX_SHADER,VS],[gl.FRAGMENT_SHADER,fragment]]){
  const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);
  if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(s));
  gl.attachShader(p,s);gl.deleteShader(s);
 }
 gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(p));
 const u={};for(const name of ['uVP','uShadow','uAnimated','uAnimationOnce','uYOffset','uOriginOffset','uDiffuse','uDiffuseNext','uModelSeasonBlend','uFog','uColor','uHasAlpha','uCutoff','uModelHeight','uTeam','uFrames','uFrameWidth','uFrameCount','uTerrain','uTerrainSize','uTerrainEnabled','uSunDirection','uAmbientMaterial','uSpecularMaterial','uEmissiveMaterial','uGlossiness','uSpecularEnabled','uVertexMode','uLightingMode','uAmbientLight','uSunColor','uSpecularLight','uNativeShader','uTextureScale','uNativeLighting','uEye','uAlphaFunction','uClipEnabled','uClipPlane','uFarmPlotConform','uFarmPlotCenter'])u[name]=gl.getUniformLocation(p,name);
 for(const name of extraUniforms)u[name]=gl.getUniformLocation(p,name);
 return {p,u};
}
const absolute=url=>new URL(url,import.meta.url);
async function json(url){const r=await fetch(absolute(url),{cache:'no-store'});if(!r.ok)throw new Error(`${url}: ${r.status}`);return r.json();}
async function verifiedSailJSON(url,digest){
 if(!/^[a-f0-9]{64}$/.test(digest||''))throw new Error('Missing original sail composite hash');
 const r=await fetch(absolute(url),{cache:'no-store'});if(!r.ok)throw new Error(`Original sail ${url}: ${r.status}`);
 const bytes=await r.arrayBuffer(),hash=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
 if(Array.from(hash,value=>value.toString(16).padStart(2,'0')).join('')!==digest)throw new Error('Original sail composite hash differs');
 return JSON.parse(new TextDecoder().decode(bytes));
}
async function texture(gl,url,cache,sail=false){
 const key=sail?'initial-sail:'+url:url;if(cache.has(key))return cache.get(key);
 const promise=(async()=>{
  const t=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,t);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,1,1,0,gl.RGBA,gl.UNSIGNED_BYTE,new Uint8Array([255,255,255,255]));
  if(url){
   const r=await fetch(absolute(url));if(!r.ok)throw new Error(`Material ${url}: ${r.status}`);
   const image=await createImageBitmap(await r.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
   gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,t);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
   gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);image.close();
  }
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,sail?gl.LINEAR:gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,sail?gl.CLAMP_TO_EDGE:gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,sail?gl.CLAMP_TO_EDGE:gl.REPEAT);if(!sail)gl.generateMipmap(gl.TEXTURE_2D);return t;
 })();cache.set(key,promise);return promise;
}
function packedVertices(positions,normals,uvs,colors){
 const verts=new Float32Array(positions.length/3*12);
 for(let i=0;i<positions.length/3;i++)verts.set([positions[i*3],positions[i*3+1],positions[i*3+2],normals[i*3]??0,normals[i*3+1]??1,normals[i*3+2]??0,uvs[i*2]??0,uvs[i*2+1]??0,...(colors?.slice(i*4,i*4+4)||[1,1,1,1])],i*12);
 return verts;
}
export {packedVertices as packOriginalVertices};
function geometryBounds(positions){
 const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];
 for(let i=0;i<positions.length;i+=3)for(let a=0;a<3;a++){min[a]=Math.min(min[a],positions[i+a]);max[a]=Math.max(max[a],positions[i+a]);}
 return {min,max};
}
function resourceBounds(parts){
 const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];let found=false;
 for(const part of parts){if(!part.pickGeometry)continue;found=true;for(let a=0;a<3;a++){min[a]=Math.min(min[a],part.pickGeometry.bounds.min[a]);max[a]=Math.max(max[a],part.pickGeometry.bounds.max[a]);}}
 return found?{min,max}:null;
}
function intersectsBounds(origin,direction,bounds,limit){
 let near=0,far=limit;
 for(let a=0;a<3;a++){
  if(Math.abs(direction[a])<1e-12){if(origin[a]<bounds.min[a]||origin[a]>bounds.max[a])return false;continue;}
  let first=(bounds.min[a]-origin[a])/direction[a],last=(bounds.max[a]-origin[a])/direction[a];
  if(first>last){const swap=first;first=last;last=swap;}
  near=Math.max(near,first);far=Math.min(far,last);if(near>far)return false;
 }
 return true;
}
function rayTriangles(origin,direction,geometry,limit){
 const p=geometry.positions,idx=geometry.indices;let closest=limit;
 for(let i=0;i<idx.length;i+=3){
  const a=idx[i]*3,b=idx[i+1]*3,c=idx[i+2]*3;
  const e1x=p[b]-p[a],e1y=p[b+1]-p[a+1],e1z=p[b+2]-p[a+2];
  const e2x=p[c]-p[a],e2y=p[c+1]-p[a+1],e2z=p[c+2]-p[a+2];
  const px=direction[1]*e2z-direction[2]*e2y,py=direction[2]*e2x-direction[0]*e2z,pz=direction[0]*e2y-direction[1]*e2x;
  const det=e1x*px+e1y*py+e1z*pz;if(Math.abs(det)<1e-10)continue;
  const inv=1/det,tx=origin[0]-p[a],ty=origin[1]-p[a+1],tz=origin[2]-p[a+2];
  const u=(tx*px+ty*py+tz*pz)*inv;if(u<0||u>1)continue;
  const qx=ty*e1z-tz*e1y,qy=tz*e1x-tx*e1z,qz=tx*e1y-ty*e1x;
  const v=(direction[0]*qx+direction[1]*qy+direction[2]*qz)*inv;if(v<0||u+v>1)continue;
  const t=(e2x*qx+e2y*qy+e2z*qz)*inv;if(t>=0&&t<closest)closest=t;
 }
 return closest;
}
function gpuPart(gl,part,instanceBuffer,materialTexture,keepPickingGeometry=false){
 const positions=part.positions,normals=part.normals,uvs=part.uvs;
 gl.bindVertexArray(null);
 const indices=gl.createBuffer();gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint32Array(part.indices),gl.STATIC_DRAW);
 // Geometry stays immutable; native animation frames live in a vertex texture.
 const makeVAO=vertices=>{
  const vao=gl.createVertexArray();gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STATIC_DRAW);
  for(const [loc,n,offset] of [[0,3,0],[1,3,12],[2,2,24],[6,4,32]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,n,gl.FLOAT,false,48,offset);}
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bindBuffer(gl.ARRAY_BUFFER,instanceBuffer);
  for(const loc of [3,4,5]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,4,gl.FLOAT,false,48,(loc-3)*16);gl.vertexAttribDivisor(loc,1);}
  return vao;
 };
 const vao=makeVAO(packedVertices(positions,normals,uvs,part.colors));
 let maxY=0;for(let i=1;i<positions.length;i+=3)maxY=Math.max(maxY,positions[i]);
 const frameCache=new Map();
 const pose=clip=>{
  const animation=part.animations?.[clip]||part.animations?.idle;
  if(!animation?.frames?.length)return null;
  if(!frameCache.has(animation)){
   const width=256,height=Math.ceil(positions.length/3*2/width),frames=animation.frames;
   const pixels=new Float32Array(width*height*4*frames.length);
   for(let f=0;f<frames.length;f++)for(let v=0;v<positions.length/3;v++){
    const offset=f*width*height*4+v*8,a=frames[f];
    pixels.set(a.positions.slice(v*3,v*3+3),offset);pixels.set(a.normals.slice(v*3,v*3+3),offset+4);
   }
   const texture=gl.createTexture();gl.activeTexture(gl.TEXTURE5);gl.bindTexture(gl.TEXTURE_2D_ARRAY,texture);
   gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.RGBA32F,width,height,frames.length);
   gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,0,width,height,frames.length,gl.RGBA,gl.FLOAT,pixels);
   gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
   frameCache.set(animation,{texture,width,count:frames.length});
  }
  return frameCache.get(animation);
 };
 // Only static resource archetypes retain CPU indices for click tests.
 const pickGeometry=keepPickingGeometry?{positions,indices:part.indices,bounds:geometryBounds(positions)}:null;
 return {vao,pose,animations:part.animations,nativeStateClips:part.nativeStateClips,activities:part.activities,count:part.indices.length,texture:materialTexture,material:modelMaterial(part),alpha:alphaState(part),depth:depthState(part),height:maxY,pickGeometry,farmPlotOffset:part.farmPlotOffset||null,pipeline:{skinned:part.skinned,textureApplyMode:part.textureApplyMode,textureUrl:part.textureUrl,textureSource:part.textureSource,texture:part.texture}};
}

import {regionalModelEntries,selectRegionalModel} from './original-regional-models.js?v=regional9';
import {citizenModelEntries,selectCitizenModel} from './original-citizen-models.js?v=citizen9';
import {loadAnimationPayload,withoutAnimationPayload} from './original-animation-payload.js?v=citizen9';
export async function loadOriginalModels(gl,enabled,teams,onProgress=()=>{},options={}){
 const result={available:new Set(),models:new Map(),resourceRenderIds:new Set(),errors:[],metrics:{colorMs:0,shadowMs:0,calls:0,triangles:0,gpuMs:0},draw(){},updateTerrain(){},setLightingContext(){return false;},setLightingSettings(){return false;},setVisualCivilizations(){return false;},setSeasonPhase(){return false;},setSeasonClimate(){return false;},pickResource(){return null;}};if(!enabled)return result;
 let visualCivilizations=null;
 result.setVisualCivilizations=values=>{
  if(values===null){visualCivilizations=null;return true;}
  if(!values||!Number.isInteger(values.length)||values.length<1||values.length>16||Array.from(values).some(value=>!Number.isInteger(value)||value< -1||value>14))throw new RangeError('Invalid visual civilizations');
  visualCivilizations=Object.freeze(Array.from(values));return true;
 };
 if(options.civilizations!==undefined)result.setVisualCivilizations(options.civilizations);
 let assetMapping;try{assetMapping=normalizeRenderAssetMapping(options.assetMapping);}catch(e){result.errors.push(e.message);return result;}
 result.assetMapping=assetMapping; let manifest,teamManifest={},scales={},seasonManifest={},seasonTeamManifest={},regionalManifest=null,regionalEntries=[],regionalSeasonManifest={},regionalTeamManifest={},citizenManifest=null,citizenEntries=[],citizenTeamManifest={},yearPhase,seasonClimate='Temperate',lightingData,defaultLighting=DEFAULT_MODEL_LIGHTING;
 try{manifest=await json(options.manifestURL||'./local-original/models/manifest.json');}catch{return result;}
 result.manifest=manifest;
 result.regionalModels={enabled:!!options.regionalManifestURL,loaded:0,errors:[]};
 if(options.regionalManifestURL){try{
  regionalManifest=await json(options.regionalManifestURL);regionalEntries=regionalModelEntries(regionalManifest);
  if(regionalManifest.seasonManifest)regionalSeasonManifest=await json(regionalManifest.seasonManifest);
  if(regionalManifest.teamManifest)regionalTeamManifest=await json(regionalManifest.teamManifest);
  result.regionalManifest=regionalManifest;
 }catch(e){regionalEntries=[];result.regionalModels.errors.push(e.message);result.errors.push('Regional models: '+e.message);}}
 result.citizenModels={enabled:!!options.citizenManifestURL,loaded:0,errors:[]};
 if(options.citizenManifestURL){try{
  citizenManifest=await json(options.citizenManifestURL);citizenEntries=citizenModelEntries(citizenManifest);
  if(citizenManifest.teamManifest)citizenTeamManifest=await json(citizenManifest.teamManifest);
  result.citizenManifest=citizenManifest;
 }catch(e){citizenEntries=[];result.citizenModels.errors.push(e.message);result.errors.push('Citizen models: '+e.message);}}
 const packStatus={enabled:!!options.packAnimations,models:0,errors:[]},packCache=new Map();let packManifest=null;
 result.packAnimations=packStatus;
 if(packStatus.enabled){try{
  packManifest=await json(options.packManifestURL||'./local-original/pack-animations/manifest.json');
  if(packManifest.format!=='ee2-native-pack-animation-manifest-v1'||!Array.isArray(packManifest.models))throw new Error('Unsupported native pack animation manifest');
 }catch(e){packStatus.errors.push(e.message);result.errors.push('Pack animation overlay: '+e.message);}}
 const sailStatus={enabled:options.initialSails!==false,manifestAvailable:false,models:0,parts:0,errors:[]},sailErrors=new Set(),sailComposites=new Map();let sailManifest=null,sailShader=null,dynamicSailShader=null;const dynamicBindings=new WeakMap();
 result.initialSails=sailStatus;
 const sailError=message=>{if(!sailErrors.has(message)){sailErrors.add(message);sailStatus.errors.push(message);result.errors.push(message);}};
 if(sailStatus.enabled){try{
  sailManifest=await json(options.sailManifestURL||'./local-original/sails-native/manifest.json');
  if(sailManifest.format!=='ee2-native-sail-initial-composites-v1'||!Array.isArray(sailManifest.models))throw new Error('Unsupported native initial sail manifest');
  sailStatus.manifestAvailable=true;result.sailManifest=sailManifest;
 }catch{/* Old portable folders and all existing models remain usable. */}}
 const augmentModel=async(model,entry)=>{
  if(model.animationPayload){try{model=await loadAnimationPayload(model,{base:import.meta.url});}catch(e){result.citizenModels.errors.push(e.message);result.errors.push('Citizen motion sidecar: '+e.message);model=withoutAnimationPayload(model);}}
  const packRow=packManifest?.models.find(r=>r.id===entry.id&&r.epoch===(entry.epoch??0)&&r.modelURL===(entry.url||entry.file)&&r.sourceSha256===model.source?.sha256);
  if(packRow){try{
   if(!packCache.has(packRow.url))packCache.set(packRow.url,Promise.all([verifiedSailJSON(packRow.modelURL,packRow.parentSha256),verifiedSailJSON(packRow.url,packRow.sha256)]));
   const [exactParent,overlay]=await packCache.get(packRow.url);model=appendNativePackAnimations(exactParent,overlay);packStatus.models++;
  }catch(e){packStatus.errors.push(e.message);result.errors.push('Pack animation overlay: '+e.message);}}
  const row=sailManifest?.models?.find(r=>r.source?.sha256===model.source?.sha256&&r.epoch===(entry.epoch??0));if(!row||(!row.effect&&!options.dynamicSails))return model;
  try{
   if(!sailComposites.has(row.url))sailComposites.set(row.url,verifiedSailJSON(row.url,row.sha256));
   const composite=await sailComposites.get(row.url);
   if(composite.assetKind!==manifestAssetKind(entry)||composite.id!==entry.id||composite.epoch!==(entry.epoch??0))return model;
   const augmented=appendInitialSailParts(model,composite);
   if(options.dynamicSails){try{dynamicBindings.set(augmented,createDynamicSailBinding(model,composite,sailManifest,entry.yawOffset??model.yawOffset??0));}catch(e){sailError(e.message);}}
   return augmented;
  }catch(e){sailError(e.message);return model;}
 };
 try{teamManifest=await json(options.teamManifestURL||'./local-original/team-textures/manifest.json');}catch{/* The original neutral materials remain available. */}
 try{scales=await json(options.scaleManifestURL||'./local-original/model-scales.json');}catch{/* Legacy normalized exports remain playable without native scale bindings. */}
 result.scaleManifest=scales;
 result.setLightingContext=context=>{
  for(const key of ['ambient','sun','direction'])if(!Array.isArray(context?.[key])||context[key].length!==3||!context[key].every(Number.isFinite))throw new RangeError('Invalid original model lighting');
  if(Math.hypot(...context.direction)<1e-12)throw new RangeError('Zero original model light direction');
  defaultLighting=context;result.lighting=context;return true;
 };
 result.setLightingSettings=settings=>{if(!lightingData)return false;return result.setLightingContext(createNativeLightingContext(lightingData,settings));};
 if(options.nativeLighting!==false){try{lightingData=await json(options.lightingURL||'./local-original/climate-lighting.json');result.setLightingSettings(options.lightingSettings);}catch{/* Older portable asset folders keep the explicit legacy adapter. */}}
 try{seasonManifest=await json('./local-original/model-seasons.json');}catch{/* Static original skins remain available in older asset folders. */}
 if(seasonManifest.teamManifest){try{seasonTeamManifest=await json(seasonManifest.teamManifest);}catch{/* Unverified or absent patch data keeps the correct neutral seasonal image. */}}
 result.seasonManifest=seasonManifest;
 result.setSeasonPhase=phase=>{yearPhase=modelYearPhase(phase);return !!(seasonManifest.models||regionalSeasonManifest.models);};
 result.setSeasonClimate=climate=>{
  if(!Object.hasOwn(seasonManifest.sets||{},climate)&&!Object.hasOwn(regionalSeasonManifest.sets||{},climate))throw new RangeError('Unknown model season climate');
  seasonClimate=climate;return true;
 };
 let shader;
 try{shader=program(gl);}catch(e){result.errors.push(e.message);console.warn('Originalmodell-Renderer:',e);return result;}
 const instances=gl.createBuffer(),textures=new Map();
 const dynamicGPU=options.dynamicSails?createDynamicSailGPU(gl,instances):null,dynamicSourceVariants=new Map();
 result.dynamicSails={enabled:!!dynamicGPU,state:()=>dynamicGPU?.status()??null};
 const actorVariant=(actor,epochs)=>{
  if(!actor||!epochs||!Number.isInteger(actor.def)||!Number.isInteger(actor.team))return null;
  let selected=null;for(const variant of dynamicSourceVariants.get(actor.def)||[]){if(variant.epoch<=(epochs[actor.team]??0)&&(!selected||variant.epoch>selected.epoch))selected=variant;}
  return selected;
 };
 result.resolveSailSource=(actor,epochs)=>dynamicGPU?actorVariant(actor,epochs)?.dynamicSailBinding??null:null;
 result.resolveSailForces=(actor,source,planeIndex,life)=>dynamicSailForces(actor,source,planeIndex,life,{rounding:options.dynamicSailRounding||'chop53',windDamping:options.dynamicSailWindDamping??0});
 result.prepareDynamicSails=inputs=>dynamicGPU?dynamicGPU.prepare(inputs):false;
 result.dynamicSailBounds=handle=>{const key=dynamicGPU?.sourceKey(handle);if(!key)return null;for(const variants of result.models.values())for(const variant of variants)if(variant.dynamicSailBinding?.key===key)return dynamicGPU.bounds(handle,variant);return null;};
 result.disposeDynamicSails=()=>dynamicGPU?.dispose();
 const entries=[...(Array.isArray(manifest.models)?manifest.models:Object.entries(manifest.models||manifest.mapping||{}).map(([id,value])=>typeof value==='string'?{id:+id,url:value}:{id:+id,...value})),...regionalEntries,...citizenEntries];
 const loadParts=async (model,keepPickingGeometry=false,modelUrl=null)=>{
   const parts=[],selectedSeasonManifest=regionalSeasonManifest.models?.[modelUrl]?regionalSeasonManifest:seasonManifest;
   let sourceSailIndex=0;for(const part of model.parts){
    const sailIndex=part.nativeSail?sourceSailIndex++:null;
    const url=part.textureUrl||part.texture||manifest.textures?.[part.textureSource?.toLowerCase()]||null;
    let materialTexture,darkTexture,darkTextures;
    try{
     materialTexture=await texture(gl,url,textures,!!part.nativeSail);
     if(part.nativeSail){
      darkTexture=await texture(gl,part.nativeSail.damageMaskUrls[part.nativeSail.initialDamageMask],textures,true);
      if(!sailShader)sailShader=program(gl,SAIL_FS,['uDark','uSailDiffuse']);
      if(dynamicGPU){try{darkTextures=await Promise.all(part.nativeSail.damageMaskUrls.map(url=>texture(gl,url,textures,true)));if(!dynamicSailShader)dynamicSailShader=program(gl,DYNAMIC_SAIL_FS,['uDark','uSailDiffuse']);}catch(e){sailError('Dynamic sail mask preload: '+e.message);}}
     }
    }catch(e){if(!part.nativeSail)throw e;sailError(e.message);continue;}
    const gpu=gpuPart(gl,{...part,nativeStateClips:part.nativeStateClips||model.nativeStateClips},instances,materialTexture,keepPickingGeometry);
    if(part.nativeSail){
     gpu.nativeSail=part.nativeSail;gpu.darkTexture=darkTexture;gpu.darkTextures=darkTextures;gpu.sailIndex=sailIndex;gpu.forTeam=()=>gpu.texture;gpu.forSeason=()=>({first:gpu.texture,next:gpu.texture,blend:0});parts.push(gpu);sailStatus.parts++;continue;
    }
    const seasonalKey=modelSeasonBinding(selectedSeasonManifest,modelUrl,model,part),neutralGPU=new Map([[url,gpu.texture]]),teamGPU=new Map(),requested=new Set();
    // Preload every declared climate image before exposing the model. A seasonal
    // switch never displays an unrelated summer image while its pixels load.
    if(seasonalKey){
     const urls=new Set(Object.values(selectedSeasonManifest.sets||{}).flatMap(climate=>climate[seasonalKey]?.urls||[]));
     for(const source of urls)neutralGPU.set(source,await texture(gl,source,textures));
    }
    const forSourceTeam=(source,team)=>{
      const suffix=['A','B','C','G'][team],key=source?.split('/').pop().replace(/\.png$/i,'').toLowerCase();
      const patchManifest=citizenTeamManifest.texture_bindings?.[source]?citizenTeamManifest:regionalTeamManifest.texture_bindings?.[source]?regionalTeamManifest:seasonTeamManifest.texture_bindings?.[source]?seasonTeamManifest:teamManifest;
      const teamKey=patchManifest.texture_bindings?.[source]||key,variant=patchManifest.textures?.[teamKey]?.variants?.[suffix],cacheKey=source+':'+suffix;
      if(variant?.verified&&patchManifest.colour_mapping_verified){
        if(!requested.has(cacheKey)){
          requested.add(cacheKey);
          texture(gl,variant.url,textures).then(t=>teamGPU.set(cacheKey,t)).catch(e=>{result.errors.push(e.message);console.warn('Original-Spielerfarbe:',e);});
        }
        return teamGPU.get(cacheKey)||neutralGPU.get(source)||gpu.texture;
      }
      return neutralGPU.get(source)||gpu.texture;
    };
    gpu.forTeam=team=>forSourceTeam(url,team);
    gpu.forSeason=team=>{
      const sequence=selectedSeasonManifest.sets?.[seasonClimate]?.[seasonalKey]?.urls,pair=modelSeasonFrames(sequence,yearPhase,url);
      return {first:forSourceTeam(pair.first,team),next:forSourceTeam(pair.next,team),blend:pair.blend};
    };
    parts.push(gpu);
   }
   return parts;
 };
 let loaded=0;
 for(const entry of entries){
  onProgress(++loaded,entries.length);
  if(entry.verified===false)continue;
  try{
   const id=+(entry.id??entry.def);if(!Number.isFinite(id))continue;
   const assetKind=manifestAssetKind(entry),bindings=sourceRenderBindings(assetMapping,assetKind,id);if(!bindings.length)continue;
   const modelUrl=entry.url||entry.file,model=await augmentModel(await json(modelUrl),{...entry,id}),parts=await loadParts(model,assetKind===1,modelUrl);
   if(!parts.length)continue;
   const scaleKey=(entry.assetKind===undefined?'':assetKind+':')+id+':'+(entry.epoch??0),sea=assetKind===0&&((id>=29&&id<=36)||id===42||id===45),transform=nativeModelTransform(model,entry.regionalCivilizations?entry.nativeScaleBinding:scales.variants?.[scaleKey]);
   const yOffset=!transform.native&&sea?-(model.normalization?.offset?.[1]||0)*(model.normalization?.scale||1):0;
   const variant={sourceId:id,assetKind,scaleKey,epoch:entry.epoch??0,until:entry.until,regionalCivilizations:entry.regionalCivilizations,sourceIdentity:entry.sourceIdentity,region:entry.region,parts,yOffset,originOffset:transform.originOffset,nativeScale:transform.native?transform.scale:null,yawOffset:entry.yawOffset??model.yawOffset??0,farUrl:entry.lodFarUrl,far:null,farRequested:false,pickBounds:resourceBounds(parts)};
   if(dynamicBindings.has(model)){variant.dynamicSailBinding=dynamicBindings.get(model);const list=dynamicSourceVariants.get(id)||[];list.push(variant);dynamicSourceVariants.set(id,list);}
   if(entry.sourceIdentity!==undefined)result.citizenModels.loaded++;else if(entry.regionalCivilizations)result.regionalModels.loaded++;
   if(model.nativeSailComposite){variant.sailComposite=model.nativeSailComposite;sailStatus.models++;}
   for(const {renderId}of bindings){const variants=result.models.get(renderId)||[];variants.push(variant);result.models.set(renderId,variants);result.available.add(renderId);if(assetKind===1)result.resourceRenderIds.add(renderId);}
  }catch(e){result.errors.push(e.message);console.warn('Originalmodell:',entry,e);}
 }
 for(const list of result.models.values())list.sort((a,b)=>a.epoch-b.epoch);
 gl.useProgram(shader.p);gl.uniform1i(shader.u.uDiffuse,0);gl.uniform1i(shader.u.uDiffuseNext,7);gl.uniform1i(shader.u.uFog,2);gl.uniform1i(shader.u.uFrames,5);gl.uniform1i(shader.u.uTerrain,6);gl.uniform3fv(shader.u.uTeam,new Float32Array(teams.flat()));
 const heightTexture=gl.createTexture();let heightSize=1,heightReady=false;
 const uploadHeights=(values,size)=>{
  gl.activeTexture(gl.TEXTURE0+6);gl.bindTexture(gl.TEXTURE_2D,heightTexture);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.R32F,size,size,0,gl.RED,gl.FLOAT,values);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
  gl.activeTexture(gl.TEXTURE0);
 };
 uploadHeights(new Float32Array([0]),1);
 result.updateTerrain=(values,size=257)=>{
  if(!Number.isInteger(size)||size<2||size>4096||values?.length!==size*size)throw new Error('Originalmodelle: ungültiges Höhenraster');
  for(const value of values)if(!Number.isFinite(value))throw new Error('Originalmodelle: ungültiger Höhenwert');
  uploadHeights(values instanceof Float32Array?values:new Float32Array(values),size);heightSize=size;heightReady=true;
 };
 const emptyFrames=gl.createTexture();gl.activeTexture(gl.TEXTURE5);gl.bindTexture(gl.TEXTURE_2D_ARRAY,emptyFrames);
 gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.RGBA32F,1,1,1);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.NEAREST);gl.activeTexture(gl.TEXTURE0);
 const timer=gl.getExtension('EXT_disjoint_timer_query_webgl2');let gpuQuery=null,nextQuery=0;
 // Keep references to the last displayed instance buffer, never a per-frame copy.
 const resourceView={data:null,ranges:null,epochs:null,zoom:80,choices:new Map()};
 result.pickResource=(origin,direction,maxDistance=Infinity)=>{
  if(!origin||!direction||origin.length<3||direction.length<3)return null;
  for(let a=0;a<3;a++)if(!Number.isFinite(origin[a])||!Number.isFinite(direction[a]))return null;
  if(direction[0]**2+direction[1]**2+direction[2]**2<1e-20)return null;
  const {data,ranges,epochs,zoom}=resourceView;if(!data?.length||!ranges?.length)return null;
  if(!(maxDistance>0))return null;
  let closest=maxDistance,hit=null;
  for(const id of result.resourceRenderIds){
   const variants=result.models.get(id);if(!variants)continue;
   const first=ranges[id*2],count=ranges[id*2+1];
   for(let i=first;i<first+count;i++){
    const offset=i*12,size=data[offset+8];if(!(size>0))continue;
    const team=Math.min(3,Math.round(data[offset+5])),epoch=epochs[team]||0;
    let base=variants[0];for(const v of variants){if(v.epoch<=epoch)base=v;else break;}
    const activity=Math.round(data[offset+11]),displayScale=base.nativeScale??size,far=displayScale*zoom<55;
    const key=id+':'+variants.indexOf(base)+':'+team+':'+activity+':'+(far&&base.farRequested?1:0)+':'+(Math.round(data[offset+7])&24);
    const variant=resourceView.choices.get(key);if(!variant?.pickBounds)continue;
    const scale=variant.nativeScale??size,yaw=data[offset+3]+variant.yawOffset,c=Math.cos(yaw),s=Math.sin(yaw),o=variant.originOffset;
    const dx=origin[0]-data[offset],dy=origin[1]-data[offset+2],dz=origin[2]-data[offset+1];
    const localOrigin=[(dx*c+dz*s)/scale-o[0],dy/scale-variant.yOffset-o[1],(-dx*s+dz*c)/scale-o[2]];
    const localDirection=[(direction[0]*c+direction[2]*s)/scale,direction[1]/scale,(-direction[0]*s+direction[2]*c)/scale];
    if(!intersectsBounds(localOrigin,localDirection,variant.pickBounds,closest))continue;
    let distance=closest;
    for(const part of variant.parts){
     if(!part.pickGeometry||(part.activities&&!part.activities.includes(activity)))continue;
     if(intersectsBounds(localOrigin,localDirection,part.pickGeometry.bounds,distance))distance=rayTriangles(localOrigin,localDirection,part.pickGeometry,distance);
    }
    if(distance<closest){closest=distance;hit=[data[offset],data[offset+1]];}
   }
  }
  return hit;
 };
 result.draw=({data,ranges,vp,epochs,zoom=80,shadow=false,eye=[0,200,0],lighting=defaultLighting,clipPlane=null,capture=false,actorMetadata=null,civilizations=visualCivilizations})=>{
  const started=performance.now();let calls=0,triangles=0,querying=false;
  const previousState=drawState(gl);
  if(!shadow&&!capture){resourceView.data=data;resourceView.ranges=ranges;resourceView.epochs=epochs;resourceView.zoom=zoom;resourceView.choices.clear();}
  if(!shadow&&!capture&&timer){
   if(gpuQuery&&gl.getQueryParameter(gpuQuery,gl.QUERY_RESULT_AVAILABLE)){
    if(!gl.getParameter(timer.GPU_DISJOINT_EXT))result.metrics.gpuMs=gl.getQueryParameter(gpuQuery,gl.QUERY_RESULT)/1e6;
    gl.deleteQuery(gpuQuery);gpuQuery=null;
   }
   if(!gpuQuery&&started>=nextQuery){nextQuery=started+1000;gpuQuery=gl.createQuery();gl.beginQuery(timer.TIME_ELAPSED_EXT,gpuQuery);querying=true;}
  }
  gl.useProgram(shader.p);gl.uniformMatrix4fv(shader.u.uVP,false,vp);gl.uniform1i(shader.u.uShadow,shadow?1:0);
  gl.uniform1i(shader.u.uClipEnabled,clipPlane?1:0);gl.uniform4fv(shader.u.uClipPlane,clipPlane||[0,0,0,0]);
  gl.uniform3fv(shader.u.uEye,eye);gl.uniform3fv(shader.u.uAmbientLight,lighting.ambient||DEFAULT_MODEL_LIGHTING.ambient);
  gl.uniform3fv(shader.u.uSunColor,lighting.sun||DEFAULT_MODEL_LIGHTING.sun);gl.uniform3fv(shader.u.uSunDirection,lighting.direction||DEFAULT_MODEL_LIGHTING.direction);
  gl.uniform3fv(shader.u.uSpecularLight,lighting.specular||lighting.sun||DEFAULT_MODEL_LIGHTING.sun);gl.uniform1i(shader.u.uNativeLighting,lighting.native?1:0);
  gl.activeTexture(gl.TEXTURE0+6);gl.bindTexture(gl.TEXTURE_2D,heightTexture);gl.uniform1i(shader.u.uTerrainSize,heightSize);gl.uniform1i(shader.u.uTerrainEnabled,heightReady?1:0);
  let activeShader=shader,sailDiffuse=null;
  if(sailShader&&!shadow){try{sailDiffuse=nativeInitialSailDiffuse({ambient:lighting.sourceAmbient,diffuse:lighting.sourceDiffuse,sunDirection:lighting.sourceDirection});}catch(e){sailError(e.message);}}
  const selectShader=target=>{
   if(activeShader===target)return;
   gl.useProgram(target.p);activeShader=target;
   if(target!==sailShader&&target!==dynamicSailShader)return;
   const u=target.u;gl.uniformMatrix4fv(u.uVP,false,vp);gl.uniform1i(u.uShadow,0);gl.uniform1i(u.uClipEnabled,clipPlane?1:0);gl.uniform4fv(u.uClipPlane,clipPlane||[0,0,0,0]);
   gl.uniform1i(u.uDiffuse,0);gl.uniform1i(u.uDark,7);gl.uniform1i(u.uFog,2);gl.uniform1i(u.uFrames,5);gl.uniform1i(u.uTerrain,6);
   gl.uniform1i(u.uTerrainSize,heightSize);gl.uniform1i(u.uTerrainEnabled,heightReady?1:0);gl.uniform3fv(u.uSunDirection,lighting.direction||DEFAULT_MODEL_LIGHTING.direction);
   gl.uniform3fv(u.uSailDiffuse,sailDiffuse);
  };
  for(const [id,variants] of result.models){
   const count=ranges[id*2+1];if(!count)continue;
   const first=ranges[id*2],groups=new Map();
   for(let i=first;i<first+count;i++){
    const team=Math.min(3,Math.round(data[i*12+5])),epoch=epochs[team]||0;
    const actor=actorMetadata?.forInstance?.(i);let identity;
    if(options.citizenManifestURL&&variants[0].assetKind===0&&variants[0].sourceId===8&&actor&&options.sourceIdentityForActor){try{const sourceIdentity=options.sourceIdentityForActor(actor);identity=sourceIdentity===0||sourceIdentity===1?sourceIdentity:null;}catch(e){identity=null;result.citizenModels.errors.push(e.message);}}
    const citizen=selectCitizenModel(variants,epoch,civilizations?.[team],identity,{neutralFallback:options.citizenNeutralFallback??null});
    let variant=citizen===undefined?selectRegionalModel(variants.filter(v=>v.sourceIdentity===undefined),epoch,civilizations?.[team]):citizen;if(!variant)continue;
    const variantIndex=variants.indexOf(variant),far=(variant.nativeScale??data[i*12+8])*zoom<55;
    if(far&&variant.farUrl){
     if(!variant.farRequested){
      variant.farRequested=true;const near=variant;
      json(near.farUrl).then(model=>augmentModel(model,{id:near.sourceId,assetKind:near.assetKind,epoch:near.epoch})).then(async model=>({model,parts:await loadParts(model,near.assetKind===1,near.farUrl)})).then(({model,parts})=>{near.far={...near,parts,pickBounds:resourceBounds(parts),sailComposite:model.nativeSailComposite,dynamicSailBinding:null};}).catch(e=>{result.errors.push(e.message);console.warn('Original-Fernmodell:',e);});
     }
     if(variant.far)variant=variant.far;
    }
    const activity=Math.round(data[i*12+11]),requestedClip=['idle','walk','wood','mine','food','build','combat'][activity]||'idle';
    const packPose=nativePackPose(actorMetadata?.forInstance?.(i),{sourceId:variant.sourceId,team,data,index:i,parts:variant.parts});
    const clip=packPose?.clip||(variant.parts.some(p=>p.animations?.[requestedClip])?requestedClip:'idle'),once=packPose?.once??false;
    const animation=variant.parts.find(p=>p.animations?.[clip])?.animations?.[clip];
    const duration=animation?.duration||1;
    const time=data[i*12+10],seed=data[i*12+6];
    const phase=packPose?.phase??((time/duration+seed)%1+1)%1;
    const preview=!!(Math.round(data[i*12+7])&24),key=variantIndex+':'+team+':'+activity+':'+(far&&variant.farRequested?1:0)+':'+(Math.round(data[i*12+7])&24)+':'+clip+':'+once;
    const group=groups.get(key)||{variant,team,activity,clip,once,preview,list:[],instanceIndices:[]};
    group.instanceIndices.push(i);
    const offset=group.list.length;for(let k=0;k<12;k++)group.list.push(data[i*12+k]);
    group.list[offset+3]+=variant.yawOffset;group.list[offset+9]=phase;
    if(variant.nativeScale!==null)group.list[offset+8]=variant.nativeScale;
    groups.set(key,group);
   }
   for(const [groupKey,{variant,team,list,activity,clip,once,preview,instanceIndices}] of groups){
    if(!shadow&&!capture&&variant.assetKind===1)resourceView.choices.set(id+':'+groupKey,variant);
    selectShader(shader);gl.uniform1f(shader.u.uYOffset,variant.yOffset);
    gl.uniform3fv(shader.u.uOriginOffset,variant.originOffset);
    gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);
    const dynamicOwners=dynamicGPU&&!preview&&!shadow&&variant.dynamicSailBinding?instanceIndices.map((index,j)=>{
     const actor=actorMetadata?.forInstance?.(index),at=j*12;
     // A sidecar from another render call/stream must never deform this row.
     if(!actor||actor.def!==variant.sourceId||actor.team!==team||actor.renderX!==list[at]||actor.renderY!==list[at+1]||actor.renderAltitude!==list[at+2]||actor.renderYaw!==data[index*12+3])return null;
     return dynamicGPU.get(actor,actorMetadata,variant.dynamicSailBinding.key);
    }):null;
    for(const part of variant.parts){
     if(part.activities&&!part.activities.includes(activity))continue;
     // Native cloth shadow events/physics are still separate. Existing body
     // shadows remain byte-identical; no synthetic sail shadow is added here.
     if(part.nativeSail&&(shadow||!sailDiffuse))continue;
     const isolated=part.nativeSail&&part.darkTextures&&dynamicOwners?.some(Boolean),jobs=isolated?dynamicOwners.map((owner,j)=>({owner,record:new Float32Array(list.slice(j*12,j*12+12)),instances:1})):[{owner:null,record:null,instances:list.length/12}];
     for(const job of jobs){
     const deformed=job.owner?.parts[part.sailIndex],gpu=deformed||part;
     if(job.record){gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,job.record,gl.STREAM_DRAW);}
     selectShader(deformed?dynamicSailShader:part.nativeSail?sailShader:shader);const u=activeShader.u;
     if(part.nativeSail){gl.uniform1f(u.uYOffset,variant.yOffset);gl.uniform3fv(u.uOriginOffset,variant.originOffset);}
     gl.uniform1i(u.uFarmPlotConform,part.farmPlotOffset?1:0);gl.uniform3fv(u.uFarmPlotCenter,part.farmPlotOffset||[0,0,0]);
     const pose=part.pose(clip);gl.uniform1i(u.uAnimated,pose?1:0);
     gl.uniform1i(u.uAnimationOnce,once&&part.animations?.[clip]?1:0);
     gl.activeTexture(gl.TEXTURE5);gl.bindTexture(gl.TEXTURE_2D_ARRAY,pose?.texture||emptyFrames);
     gl.uniform1i(u.uFrameWidth,pose?.width||1);gl.uniform1i(u.uFrameCount,pose?.count||1);
     const seasonal=part.forSeason(team);
     gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,part.nativeSail?(deformed?part.darkTextures[job.owner.damageMask]:part.darkTexture):seasonal.next);gl.uniform1f(u.uModelSeasonBlend,seasonal.blend);
     gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,seasonal.first);gl.bindVertexArray(gpu.vao);
     const m=part.material,a=part.alpha;
     const pipeline=nativeModelPipeline(part.pipeline,lighting);gl.uniform1i(u.uNativeShader,pipeline.shader);gl.uniform1f(u.uTextureScale,pipeline.textureScale);
     gl.uniform4fv(u.uColor,[...m.diffuse,m.alpha]);gl.uniform1f(u.uModelHeight,deformed?deformed.height:part.height);
     gl.uniform3fv(u.uAmbientMaterial,m.ambient);gl.uniform3fv(u.uSpecularMaterial,m.specular);gl.uniform3fv(u.uEmissiveMaterial,m.emissive);
     gl.uniform1f(u.uGlossiness,m.glossiness);gl.uniform1i(u.uSpecularEnabled,m.specularEnabled?1:0);
     gl.uniform1i(u.uVertexMode,m.vertexMode);gl.uniform1i(u.uLightingMode,m.lightingMode);
     gl.uniform1i(u.uHasAlpha,(a.test?1:0)|(a.blend?2:0));gl.uniform1f(u.uCutoff,a.threshold);gl.uniform1i(u.uAlphaFunction,a.testFunction);
     let sailState;
     if(part.nativeSail){
      sailState={cull:gl.isEnabled(gl.CULL_FACE),equationRGB:gl.getParameter(gl.BLEND_EQUATION_RGB),equationAlpha:gl.getParameter(gl.BLEND_EQUATION_ALPHA)};
      gl.disable(gl.CULL_FACE);gl.enable(gl.DEPTH_TEST);gl.depthMask(true);gl.depthFunc(gl.LEQUAL);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.blendEquation(gl.FUNC_ADD);
     }else applyDrawState(gl,part,shadow,preview);
     gl.drawElementsInstanced(gl.TRIANGLES,gpu.count,gl.UNSIGNED_INT,0,job.instances);
     if(sailState){if(sailState.cull)gl.enable(gl.CULL_FACE);else gl.disable(gl.CULL_FACE);gl.blendEquationSeparate(sailState.equationRGB,sailState.equationAlpha);}
     calls++;triangles+=gpu.count/3*job.instances;
     }
     // Shared instance attributes are restored for later rigid/other parts.
     if(isolated){gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);}
    }
   }
  }
  selectShader(shader);restoreDrawState(gl,previousState);
  if(querying)gl.endQuery(timer.TIME_ELAPSED_EXT);
  if(capture)return;
  if(shadow)result.metrics.shadowMs=performance.now()-started;
  else{result.metrics.colorMs=performance.now()-started;result.metrics.calls=calls;result.metrics.triangles=triangles;}
 };
 return result;
}
