import {normalizeRenderAssetMapping,sourceRenderBindings,manifestAssetKind} from './original-render-namespace.js';
import {bindHouseLodParts,houseLodBindings,houseLodProfile,readHouseLodModel} from './original-house-lod.js?v=range11';
export {createRenderAssetMapping} from './original-render-namespace.js';
// Textured NIF geometry converted locally by tools/extract_ee2_models.py.
// This renderer does not inspect original files or alter the deterministic simulation.
import {DEFAULT_MODEL_LIGHTING,modelMaterial,alphaState,depthState} from './original-material.js?v=material1';
import {nativeModelTransform} from './original-scale.js?v=native1';
import {HOUSE_REFLECTION_CATALOG,bindHouseReflectionBound,admitsHouseReflection} from './house-reflection-bounds.js?v=house-reflection233';
import {modelYearPhase,modelSeasonFrames,modelSeasonBinding} from './original-model-seasons.js?v=modelseason1';
import {createNativeLightingContext,nativeModelPipeline} from './original-lighting.js?v=valley-lightmap148';
import {appendInitialSailParts,nativeInitialSailDiffuse} from './original-sail-composite.js?v=initialsail7';
import {createDynamicSailBinding,createDynamicSailGPU,dynamicSailForces} from './original-dynamic-sails.js?v=native75-sails';
import {appendNativePackAnimations,nativePackPose} from './original-pack-states.js?v=pack8';
import {createNativePackBlendHistory,hasSourcePackBlend} from './original-pack-blend.js?v=pack-blend126';
import {createPackRigFrames,loadPackRigModel,packRigBinding} from './original-pack-rig.js?v=pack-rig129';
import {createBirdFlightFrames,prepareBirdFlightRig} from './dv2-bird-flight-rig.js?v=aircraft-curve257-private';
import {FARM_SEEDING_POSITION_GLSL,farmSeedingPose,loadFarmSeeding} from './dv2-farm-seeding.js?v=farm-seeding282-private';
import {oilRigPartPose} from './dv2-oil-rig-motion.mjs?v=oil-presentation286-private';
import {captureAnimationBindings,captureOverlayMatches,loadCaptureAnimationOverlay,nativeCapturePose,regionalCaptureBindings,loadRegionalCaptureAsset} from './original-capture-animations.js?v=native26';
import {nativeTakeoffPose} from './air-takeoff.js?v=native33';
import {scaffoldInstanceKey,wonderScaffoldStage} from './original-wonder-scaffold.js?v=native63';
import {wonderFenceAtSite} from './original-wonder-fence.js?v=native63';
const navalPoseModels=new Map([
 [29, new Set([
  '41964f675e164306c5e14ad98d75e09d9e76b657b289aa04314873b726ed6655',
  '8da0386f6c4a34b90cafcb317557ad076805c9030affc251997b25a03c96a6af',
  'dd512fc27fb5c5c24fdbd28497d4063bc60e17360252dfa443f23d06c7f08ce5',
 ])],
 [30, new Set([
  '4a8e7bcdb59dc760076cb2c852c632ba445b0e29836a3eebc1ca6391d289cca7',
  'b2de264c62ec26853f74f70a1d6ffe0cfbaa8b531c9704d4e3195d21afa4cd6b',
  'f3e65ebe1b544efbe0f6dff8ef73d1a86fd21affcc7756188d3c05c2e438b387',
 ])],
 [31, new Set([
  '80ceeb714533858cc9690e980f6db5af362373f502a76d7ef6bc8a299efb9793',
  '921c59457b2e50187837ec49f7a441d7ee9d3e18139cbfe9ac1b79c2d325b85c',
  'acc3ba38a90ada0d385062f06910d347b5b46d3f107ef80d6b836486293610ab',
 ])],
 [32, new Set([
  '68f5ffb2ba40047adf09c5fd1179440d16e104d13a971e663259847b1ff889fb',
  'e6c137a050e8b99c7e10f5c0cecaa2f615ed644ba632cd82fa2c55af127cb153',
  '0f0d2d9d6384e897abe7b54e8ef0886324b74559dbe97761c09be2d2046d9b7a',
 ])],
 [33, new Set([
  'eeabc0d7548f60a63aa22959fd054781e8ff9e3f56d9dee494fccde56f57037b',
  '32299c3438ca7aa2ebc8cb682bf1b1fc1b4e0c35db688f59e5fb5c08c4fa0ded',
 ])],
 [34, new Set([
  '95a0b1dcc98b3dd6cd2b0c8e48acfd0f8ac9db01ee0f7350c7459e9bdf5c7353',
  '461c864d8b312b6082ae38540b6c1b5121e2af5b6da99d136ca8c0e45126cb2c',
  '58bdca1af6ff64039ff6311dbe557238dd8892d34bf832a08a2e79208dc73ee8',
 ])],
 [35, new Set([
  '32299c3438ca7aa2ebc8cb682bf1b1fc1b4e0c35db688f59e5fb5c08c4fa0ded',
  '8c010579b6a9551c3658940ab6db0df5588b1be9ab71be0292cf787797fc80f7',
 ])],
 [36, new Set([
  '461c864d8b312b6082ae38540b6c1b5121e2af5b6da99d136ca8c0e45126cb2c',
  '58bdca1af6ff64039ff6311dbe557238dd8892d34bf832a08a2e79208dc73ee8',
 ])],
 [42, new Set([
  'd2143f183f4519503526aff8a09014131deaef5a562cbe341776ec69c9b5a01f',
  '8decd53eeb60b67426096fe7ea990b52b013bcdc7d554c580d139da75a028809',
 ])],
 [45, new Set([
  '58bdca1af6ff64039ff6311dbe557238dd8892d34bf832a08a2e79208dc73ee8',
 ])],
 [58, new Set([
  'f3c70a79746d5f01f8749ab63d72d64525290736a1c998f444247d10ed012a39',
  '5af9ace81bc7c2d174c90673f2ffcc998653cb8c688cd76c4e304482e81fe1eb',
  'abf0a61d4e22df75118ed2637fc44c5d02ecaa98fdcc489fb4951fac8b358d1b',
 ])],
 [82, new Set([
  '447678200be81d99b999c47c4b87d3ac5597d92f8056644d58ce797a52445f17',
  '99835e2be1f47173d5af27ba6eb26e0b4c06a47c9f774291a9d6fdc2ece9842d',
  'e84287b5320e76ea54fb83a0f0dddb6ebaea46818daef0fd9736db7ebac09416',
 ])],
 [84, new Set([
  'a51621715027d57f06086861bd7670b8b1a151d5f988c0cf67e70c06fc6bfad6',
  'ebd48fc1a75cb938135e4a78ba1303347906028e021825942820de63c8466b15',
 ])],
 [108, new Set([
  '42e55eda1853536ce412b6581adbe8fa964b64e7be0fec2f3ab2d1e86d9dc3e7',
  '90a530709f876705c6655bcdf1ad88710bcb585b7d3a24213c154744b45b4204',
  'a3a59d5e3bf5f1f3ca1fd57079780438839ae9fa49b142b2beb0782b25b697c5',
 ])],
]);
const navalBasisModel=(model,id)=>model.normalization?.axis==='x,z,-y'&&navalPoseModels.get(id)?.has(model.source?.sha256);
const nativeNavalYaw=variant=>variant.yawOffset===-Math.PI/2||
 (variant.sourceId===84&&variant.yawOffset===Math.fround(-Math.PI/2));
const VS=`#version 300 es
layout(location=0) in vec3 aPosition;layout(location=1) in vec3 aNormal;layout(location=2) in vec2 aUV;
layout(location=3) in vec4 iA;layout(location=4) in vec4 iB;layout(location=5) in vec4 iC;
layout(location=6) in vec4 aColor;
layout(location=10) in vec3 aFarmMorphDelta;
#ifdef PACK_RIG
layout(location=7) in vec3 aPackRestPosition;
layout(location=8) in vec3 aPackRestNormal;
layout(location=9) in vec3 aPackSkin;
uniform bool uPackRigActive;uniform int uPackRigOffset;uniform vec4 uPackRigNormalization;
#endif
uniform mat4 uVP;uniform highp int uShadow;uniform bool uAnimated;uniform bool uGuidedRocket;uniform float uYOffset;uniform vec3 uOriginOffset;
uniform bool uNaval;uniform mat3 uNavalBasis;
uniform highp sampler2DArray uFrames;uniform int uFrameWidth;uniform int uFrameCount;uniform bool uAnimationOnce;
#ifdef PACK_BLEND
uniform highp sampler2DArray uPackOutgoingFrames;uniform int uPackOutgoingCount;uniform bool uPackOutgoingOnce;
uniform bool uPackBlendActive;uniform float uPackOutgoingPhase,uPackIncomingPhase,uPackBlendWeight;
#endif
uniform highp sampler2D uTerrain;uniform int uTerrainSize;uniform bool uTerrainEnabled;uniform vec3 uSunDirection;
uniform bool uFarmPlotConform;uniform vec3 uFarmPlotCenter;
uniform bool uFarmSeeding;uniform float uFarmSeedingWeight;uniform vec3 uFarmSeedingShift;
uniform bool uOilRigRigid;uniform vec2 uOilRigCosSin;uniform vec3 uOilRigTranslation;
uniform highp int uNativeShader;uniform vec3 uAmbientLight;uniform vec3 uSunColor;uniform vec4 uColor;uniform float uSpyEmissive;
out vec4 vNativeLighting;out vec3 vNormal;out vec2 vUV;out vec2 vWorld;out vec3 vPosition;out vec4 vColor;out float vY;flat out vec4 vInfo;
${FARM_SEEDING_POSITION_GLSL}
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
void animationPose(highp sampler2DArray frames,int count,bool once,float at,out vec3 local,out vec3 normal){
 float phase=once?clamp(at,0.,1.)*float(count-1):fract(at)*float(count);
 int frame=int(floor(phase)),next=once?min(frame+1,count-1):(frame+1)%count;
 int pixel=gl_VertexID*2;ivec2 uv=ivec2(pixel%uFrameWidth,pixel/uFrameWidth);
 vec3 p0=texelFetch(frames,ivec3(uv,frame),0).xyz,p1=texelFetch(frames,ivec3(uv,next),0).xyz;
 uv.x+=1;vec3 n0=texelFetch(frames,ivec3(uv,frame),0).xyz,n1=texelFetch(frames,ivec3(uv,next),0).xyz;
 local=mix(p0,p1,fract(phase));normal=mix(n0,n1,fract(phase));
}
#ifdef PACK_RIG
void packRigPose(out vec3 position,out vec3 normal){
 int i=uPackRigOffset+int(aPackSkin.x),j=uPackRigOffset+int(aPackSkin.y),layer=int(iC.y);
 vec4 rest=vec4(aPackRestPosition,1.);
 vec3 first,second;
 for(int row=0;row<3;row++){
  vec4 a=texelFetch(uPackOutgoingFrames,ivec3(row,i,layer),0);
  first[row]=dot(a,rest);
  second[row]=dot(texelFetch(uPackOutgoingFrames,ivec3(row,j,layer),0),rest);
  normal[row]=dot(a.xyz,aPackRestNormal);
 }
 vec3 mixed=first*aPackSkin.z+second*(1.-aPackSkin.z);
 position=(vec3(mixed.x,mixed.z,-mixed.y)+uPackRigNormalization.xyz)*uPackRigNormalization.w;
 normal=normalize(vec3(normal.x,normal.z,-normal.y));
}
#endif
void main(){
 vec3 local=aPosition,normal=aNormal;
#ifdef PACK_RIG
 if(uPackRigActive)packRigPose(local,normal);
 else
#endif
 if(uAnimated){
#ifdef PACK_BLEND
  if(uPackBlendActive){
   vec3 outgoing,oldNormal;
   animationPose(uFrames,uFrameCount,uAnimationOnce,uPackIncomingPhase,local,normal);
   animationPose(uPackOutgoingFrames,uPackOutgoingCount,uPackOutgoingOnce,uPackOutgoingPhase,outgoing,oldNormal);
   local=mix(outgoing,local,uPackBlendWeight);normal=mix(oldNormal,normal,uPackBlendWeight);
  }else
#endif
  animationPose(uFrames,uFrameCount,uAnimationOnce,iC.y,local,normal);
 }
 if(uOilRigRigid){
  float c=uOilRigCosSin.x,s=uOilRigCosSin.y;
  local=vec3(local.x,c*local.y-s*local.z,s*local.y+c*local.z)+uOilRigTranslation;
  normal=vec3(normal.x,c*normal.y-s*normal.z,s*normal.y+c*normal.z);
 }
 if(uFarmSeeding)local=farmSeedingPosition(aPosition,aFarmMorphDelta,uFarmSeedingWeight,uFarmSeedingShift+uFarmPlotCenter);
 local.y+=uYOffset;
 float constructionY=local.y;local+=uOriginOffset;
 float c=cos(iA.w),s=sin(iA.w);vec3 p=local*iC.x;
 vec3 world=vec3(iA.x+p.x*c-p.z*s,iA.z+p.y,iA.y+p.x*s+p.z*c);
 vec3 worldNormal=vec3(normal.x*c-normal.z*s,normal.y,normal.x*s+normal.z*c);
 if(uGuidedRocket){
  float pitch=iC.z,cp=cos(pitch);
  vec3 right=vec3(s,0.,-c),forward=vec3(c*cp,sin(pitch),s*cp);
  mat3 basis=mat3(right,cross(forward,right),forward);
  world=vec3(iA.x,iA.z,iA.y)+basis*p;
  worldNormal=basis*normal;
 }else if(uNaval){
  world=vec3(iA.x,iA.z,iA.y)+uNavalBasis*p;
  worldNormal=uNavalBasis*normal;
 }
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
 vNormal=worldNormal;
 // IndexPal20_High/Low.vsh: signed normal dot or constant .5, then the
 // vertex diffuse register is clamped before raster interpolation.
 float lightDot=uNativeShader==2?.5:dot(normalize(vNormal),normalize(uSunDirection));
 vNativeLighting=vec4(clamp(uAmbientLight+lightDot*uSunColor+uSpyEmissive,0.,1.),uColor.a);
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
uniform vec2 uWonderConstructionPlane;
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
 int flags=int(vInfo.w+.5);float fog=texture(uFog,vWorld/vec2(textureSize(uFog,0))).r;
 if((flags&32)==0 && fog<.05)discard;
 if((flags&64)!=0 && vInfo.z<1.){
  if(uWonderConstructionPlane.y>0.){
   if((flags&24)==0 && vY>uWonderConstructionPlane.x+uWonderConstructionPlane.y*vInfo.z)discard;
  }else if(vY>max(.02,uModelHeight*vInfo.z))discard;
 }
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
 int flags=int(vInfo.w+.5);float fog=texture(uFog,vWorld/vec2(textureSize(uFog,0))).r;
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
function applyDrawState(gl,part,shadow,preview,shimmerBlend){
 const a=part.alpha,d=part.depth;
 enabled(gl,gl.DEPTH_TEST,shadow||d.test);gl.depthMask(!shadow&&!preview&&d.write);
 gl.depthFunc([gl.ALWAYS,gl.LESS,gl.EQUAL,gl.LEQUAL,gl.GREATER,gl.NOTEQUAL,gl.GEQUAL,gl.NEVER][d.function]??gl.LEQUAL);
 enabled(gl,gl.BLEND,shadow||preview||a.blend||shimmerBlend);
 if(shadow||preview||shimmerBlend&&!a.blend){gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);return;}
 if(a.blend){
  const factors=[gl.ONE,gl.ZERO,gl.SRC_COLOR,gl.ONE_MINUS_SRC_COLOR,gl.DST_COLOR,gl.ONE_MINUS_DST_COLOR,gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.DST_ALPHA,gl.ONE_MINUS_DST_ALPHA,gl.SRC_ALPHA_SATURATE];
  gl.blendFunc(factors[a.source]??gl.SRC_ALPHA,factors[a.destination]??gl.ONE_MINUS_SRC_ALPHA);
 }
}
function program(gl,fragment=FS,extraUniforms=[],packBlend=false,packRig=false){
 const p=gl.createProgram();
 const define=packBlend?'#define PACK_BLEND\n':'';
 const vertex=VS.replace('#version 300 es\n','#version 300 es\n'+define+(packRig?'#define PACK_RIG\n':''));
 for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
  const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);
  if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(s));
  gl.attachShader(p,s);gl.deleteShader(s);
 }
 gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(p));
 const u={};for(const name of ['uVP','uShadow','uAnimated','uAnimationOnce','uYOffset','uOriginOffset','uDiffuse','uDiffuseNext','uModelSeasonBlend','uFog','uColor','uHasAlpha','uCutoff','uModelHeight','uTeam','uFrames','uFrameWidth','uFrameCount','uTerrain','uTerrainSize','uTerrainEnabled','uSunDirection','uAmbientMaterial','uSpecularMaterial','uEmissiveMaterial','uGlossiness','uSpecularEnabled','uVertexMode','uLightingMode','uAmbientLight','uSunColor','uSpecularLight','uNativeShader','uTextureScale','uNativeLighting','uEye','uAlphaFunction','uClipEnabled','uClipPlane','uFarmPlotConform','uFarmPlotCenter'])u[name]=gl.getUniformLocation(p,name);
 if(packBlend)for(const name of ['uPackOutgoingFrames','uPackOutgoingCount','uPackOutgoingOnce','uPackBlendActive','uPackOutgoingPhase','uPackIncomingPhase','uPackBlendWeight'])u[name]=gl.getUniformLocation(p,name);
 if(packRig)for(const name of ['uPackRigActive','uPackRigOffset','uPackRigNormalization'])u[name]=gl.getUniformLocation(p,name);
 u.uSpyEmissive=gl.getUniformLocation(p,'uSpyEmissive');
 u.uWonderConstructionPlane=gl.getUniformLocation(p,'uWonderConstructionPlane');
 u.uGuidedRocket=gl.getUniformLocation(p,'uGuidedRocket');
 u.uNaval=gl.getUniformLocation(p,'uNaval');u.uNavalBasis=gl.getUniformLocation(p,'uNavalBasis');
 for(const name of ['uFarmSeeding','uFarmSeedingWeight','uFarmSeedingShift'])u[name]=gl.getUniformLocation(p,name);
 for(const name of ['uOilRigRigid','uOilRigCosSin','uOilRigTranslation'])u[name]=gl.getUniformLocation(p,name);
 for(const name of extraUniforms)u[name]=gl.getUniformLocation(p,name);
 return {p,u};
}
const absolute=url=>new URL(url,import.meta.url);
async function json(url){const r=await fetch(absolute(url),{cache:'no-store'});if(!r.ok)throw new Error(`${url}: ${r.status}`);return r.json();}
async function verifiedModelJSON(url,digest,kind='Original sail composite'){
 if(!/^[a-f0-9]{64}$/.test(digest||''))throw new Error('Missing '+kind.toLowerCase()+' hash');
 const r=await fetch(absolute(url),{cache:'no-store'});if(!r.ok)throw new Error(`${kind} ${url}: ${r.status}`);
 const bytes=await r.arrayBuffer(),hash=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
 if(Array.from(hash,value=>value.toString(16).padStart(2,'0')).join('')!==digest)throw new Error(kind+' hash differs');
 return JSON.parse(new TextDecoder().decode(bytes));
}
const OIL_RIG_CURVES_URL='./local-original/oil-rig-motion/curves.json?v=oil-presentation286-private';
const OIL_RIG_CURVES_SHA='3d1910240511c13e5676ac3ce23cab9ec49856abc20f1ee52520bc5d6282e1d3';
function bindOilRigModel(model,curves,lod){
 const normal=model.normalization,source=curves.normalization,nodes=curves.lodNodes[lod];
 if(model.source?.sha256!==curves.sourceSha256||normal?.axis!==source.axis||normal.scale!==source.scale||
  normal.offset.some((value,index)=>value!==source.offset[index])||
  !Object.keys(nodes).every(node=>model.parts.some(part=>part.sourceNode===Number(node)))){
  throw new Error('Original Oil Rig model geometry differs');
 }
 return {curves,nodes};
}
function oilRigClock(actor,read){
 const clock=read(actor);
 if(!clock||(clock.state!==0&&clock.state!==32)||!Number.isFinite(clock.now)||
  !Number.isFinite(clock.started)||clock.started<0)return null;
 const elapsed=Math.fround(Math.max(0,clock.now-clock.started));
 return {clip:clock.state===32?'mine':'idle',time:elapsed>3?Math.fround(elapsed%3):elapsed};
}
// The original Farm work body indexes Seeding_N and Harvest_1/2/3_N
// with its zero-based quadrant. Imported meshes keep N as farmPlot.
const FARM_STATES=['Seeding','Harvest_1','Harvest_2','Harvest_3'];
const FARM_CROP_MODELS=[
 ['FarmWheat',[
  '010385184f4d1b9a7efb3bfaaba0ddb5709b6dffd67e89bd32b841fd5082612d',
  '86d801dd39adeef98b92a29f45631c5a3fbeac1f603e76fdcf7b4c70b8421b2e',
  '213bf6bde1bb08e63b7af7d6fd51bbe6d453d38554d75dc5da0380a5862087d2',
  '8d602f500a72b2366d72ce7d0f04bb66d4e00c40d8d465dd80fbd96633d45a20',
 ]],
 ['FarmSugarCane',[
  'db64d789d4b1a8effffd980f4d48006f3e3227112533eddcde64ec99dbe603e9',
  '608e05091b43b06dd9a527b185a67e928a2792c760d6978371ed13e42bfec012',
  '7ee2d8eb51e8b46a618908e703ac089e55460442bf33d0b0b7a8fd5396897b86',
  'e9a8c5bac4e7424122123ce670d98d742a1d4cf5bdb8a63fabde4ff6a5c4e5d9',
 ]],
 ['FarmYam',[
  'f48fd7e00e1c6b22908296e0bcfd6cc558d1c8ee0ebb3c26ef653e07662be40c',
  '8da0b8665f3c46d7ebdd5dd9242c767cba80fdb66819b4a4f9e401f465056f80',
  'cf0d1de50a1dae7c479bd154889a5d7b5b632fc3ab9dd1b56bbb968f27cd2b0c',
  '4b10c0f530e6415f8dec26aa505906fd2cc953a89c7dffa38c19a564ef30c812',
 ]],
];
const FARM_DEFAULT_WORD=0x6db;
function farmPlotParts(model,parts,family,state){
 const binding=model.nativeFarmComposite;
 if(binding?.family!==family||binding.state!==state)throw new Error('Farm crop state differs');
 const plots=[[],[],[],[]];
 for(const part of parts){
  const plot=plots[part.farmPlot-1];
  if(!plot||!part.farmPlotOffset)throw new Error('Missing original Farm plot binding');
  plot.push(part);
 }
 if(plots.some(plot=>!plot.length))throw new Error('Incomplete Farm plot model');
 return plots;
}
function farmPartsForWord(stages,word){
 if(!Number.isInteger(word)||word<0||word>0xfff)return null;
 const parts=[];
 for(let plot=0;plot<4;plot++){
  const stage=stages[(word>>>(plot*3))&7];
  if(!stage)return null;
  parts.push(...stage[plot]);
 }
 return parts;
}
function bindFarmSeeding(model,family,modelPin){
 if(family.name!==model.nativeFarmComposite?.family||family.meshSha256!==model.source?.sha256||
    family.modelSha256!==modelPin)throw new Error('Original Farm seeding model binding differs');
 return {...model,parts:model.parts.map(part=>{
  const geometry=family.parts.get(part.sourceNode);
  if(!geometry||geometry.position.length!==part.positions.length||!part.farmPlotOffset)
   throw new Error('Original Farm seeding source node differs');
  return {...part,farmSeeding:{family,geometry,sourceNode:part.sourceNode}};
 })};
}
function farmSeedingAges(parts,actor,elapsedForActor){
 if(!parts||!elapsedForActor)return null;
 const ages=[];
 for(const part of parts){
  const plot=part.farmPlot-1;
  if(!part.farmSeeding||ages[plot]!==undefined)continue;
  const requested=elapsedForActor(actor,plot);
  const elapsed=Number.isFinite(requested)?Math.fround(requested):NaN;
  ages[plot]=Number.isFinite(elapsed)&&elapsed>=0?elapsed:null;
 }
 return ages.some(age=>age!==null)?ages:null;
}
async function texture(gl,url,cache,sail=false,digest=null){
 const key=digest?`${url}:${digest}`:sail?'initial-sail:'+url:url;if(cache.has(key))return cache.get(key);
 const promise=(async()=>{
  const t=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,t);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,1,1,0,gl.RGBA,gl.UNSIGNED_BYTE,new Uint8Array([255,255,255,255]));
  if(url){
   const r=await fetch(absolute(url));if(!r.ok)throw new Error(`Material ${url}: ${r.status}`);
   const blob=await r.blob();
   if(digest){
    const hash=new Uint8Array(await crypto.subtle.digest('SHA-256',await blob.arrayBuffer()));
    if(Array.from(hash,byte=>byte.toString(16).padStart(2,'0')).join('')!==digest)throw new Error(`Material ${url}: hash differs`);
   }
   const image=await createImageBitmap(blob,{premultiplyAlpha:'none',colorSpaceConversion:'none'});
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
function gpuPart(gl,part,instanceBuffer,materialTexture,keepPickingGeometry=false,packRigPart=null){
 const positions=part.positions,normals=part.normals,uvs=part.uvs;
 gl.bindVertexArray(null);
 const indices=gl.createBuffer();gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint32Array(part.indices),gl.STATIC_DRAW);
 // Geometry stays immutable; native animation frames live in a vertex texture.
 const makeVAO=(vertices,morphDelta=null)=>{
  const vao=gl.createVertexArray();gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STATIC_DRAW);
  for(const [loc,n,offset] of [[0,3,0],[1,3,12],[2,2,24],[6,4,32]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,n,gl.FLOAT,false,48,offset);}
  if(packRigPart){
   gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,packRigPart.vertices,gl.STATIC_DRAW);
   for(const [location,offset] of [[7,0],[8,12],[9,24]]){
    gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,3,gl.FLOAT,false,36,offset);
   }
  }
  if(morphDelta){
   gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,morphDelta,gl.STATIC_DRAW);
   gl.enableVertexAttribArray(10);gl.vertexAttribPointer(10,3,gl.FLOAT,false,0,0);
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bindBuffer(gl.ARRAY_BUFFER,instanceBuffer);
  for(const loc of [3,4,5]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,4,gl.FLOAT,false,48,(loc-3)*16);gl.vertexAttribDivisor(loc,1);}
  return vao;
 };
 const vao=makeVAO(packedVertices(positions,normals,uvs,part.colors));
 const farmSeeding=part.farmSeeding,seedGeometry=farmSeeding?.geometry;
 const farmSeedingVAO=seedGeometry?makeVAO(packedVertices(seedGeometry.position,seedGeometry.normal,uvs,part.colors),seedGeometry.morphDelta):null;
 let maxY=0;for(let i=1;i<positions.length;i+=3)maxY=Math.max(maxY,positions[i]);
 const frameCache=new Map();
 const pose=clip=>{
  const animation=part.animations?.[clip]||(clip!=='native-takeoff'&&part.animations?.idle);
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
 return {vao,pose,packRigPart,sourceNode:part.sourceNode,animations:part.animations,nativeStateClips:part.nativeStateClips,activities:part.activities,nativeCitizenTool:part.nativeCitizenTool,clipVisibility:part.clipVisibility,count:part.indices.length,texture:materialTexture,material:modelMaterial(part),alpha:alphaState(part),depth:depthState(part),height:maxY,pickGeometry,farmPlot:part.farmPlot,farmPlotOffset:part.farmPlotOffset||null,farmSeeding,farmSeedingVAO,pipeline:{skinned:part.skinned,textureApplyMode:part.textureApplyMode,textureUrl:part.textureUrl,textureSource:part.textureSource,texture:part.texture}};
}

import {regionalModelEntries,selectRegionalModel} from './original-regional-models.js?v=regicide';
import {citizenModelEntries,selectCitizenModel} from './original-citizen-models.js?v=citizen9';
import {compileCitizenWork,augmentCitizenWork,createCitizenWorkHistory} from './dv2-citizen-work-v2.mjs?v=citizen-work289-private';
import {loadAnimationPayload,withoutAnimationPayload,attachAnimationPayload} from './original-animation-payload.js?v=citizen9';
import {loadOriginalScoutIdle} from './original-scout-idle.js?v=native103';
import {loadOriginalHuntPoses} from './original-hunt-poses.js?v=death-valley2-hunt244';
import {createCityCenterTerrainBinding} from './original-citycenter-terrain.js?v=origin1';
export async function loadOriginalModels(gl,enabled,teams,onProgress=()=>{},options={}){
 const result={available:new Set(),models:new Map(),resourceRenderIds:new Set(),errors:[],seasonClimates:['Temperate'],climateErrors:[],metrics:{colorMs:0,shadowMs:0,calls:0,triangles:0,gpuMs:0},draw(){},updateTerrain(){},setLightingContext(){return false;},setLightingSettings(){return false;},setVisualCivilizations(){return false;},setHouseLodProfile(){return false;},setSeasonPhase(){return false;},setSeasonClimate(){return false;},pickResource(){return null;},pickResourceHit(){return null;},clearResourcePicks(){}};if(!enabled)return result;
 let visualCivilizations=null;
 result.setVisualCivilizations=values=>{
  if(values===null){visualCivilizations=null;return true;}
  if(!values||!Number.isInteger(values.length)||values.length<1||values.length>16||Array.from(values).some(value=>!Number.isInteger(value)||value< -1||value>14))throw new RangeError('Invalid visual civilizations');
  visualCivilizations=Object.freeze(Array.from(values));return true;
 };
 if(options.civilizations!==undefined)result.setVisualCivilizations(options.civilizations);
 let assetMapping;try{assetMapping=normalizeRenderAssetMapping(options.assetMapping);}catch(e){result.errors.push(e.message);return result;}
 const privatePack=options.privateModelPack;
 result.assetMapping=assetMapping; let manifest,teamManifest={},scales={},seasonManifest={},seasonTeamManifest={},regionalManifest=null,regionalEntries=[],regionalSeasonManifest={},regionalTeamManifest={},additionalRegionalTeam={},citizenManifest=null,citizenEntries=[],citizenTeamManifest={},yearPhase,seasonClimate='Temperate',lightingData,defaultLighting=DEFAULT_MODEL_LIGHTING;
 try{manifest=privatePack?.manifest??await json(options.manifestURL||'./local-original/models/manifest.json');}catch{return result;}
 result.manifest=manifest;
 result.regionalModels={enabled:!!options.regionalManifestURL,loaded:0,errors:[]};
 if(options.regionalManifestURL){try{
  regionalManifest=await json(options.regionalManifestURL);regionalEntries=regionalModelEntries(regionalManifest);
  if(regionalManifest.seasonManifest)regionalSeasonManifest=await json(regionalManifest.seasonManifest);
  if(regionalManifest.teamManifest)regionalTeamManifest=await json(regionalManifest.teamManifest);
  result.regionalManifest=regionalManifest;
 }catch(e){regionalEntries=[];result.regionalModels.errors.push(e.message);result.errors.push('Regional models: '+e.message);}}
 if(options.additionalRegionalManifestURL){try{
  const additional=await json(options.additionalRegionalManifestURL);
  const rows=regionalModelEntries(additional);
  regionalEntries.push(...rows);
  result.additionalRegionalAudio=rows.filter(row=>row.regionalFallback&&row.sourceFamily);
  if(additional.teamManifest)additionalRegionalTeam=await json(additional.teamManifest);
 }catch(e){result.regionalModels.errors.push(e.message);result.errors.push('Additional regional models: '+e.message);}}
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
 const captureStatus={enabled:typeof options.capturePoseForActor==='function',manifestAvailable:false,models:0,errors:[]};
 const captureCache=new Map();let captureBindings=null;
 result.captureAnimations=captureStatus;
 if(captureStatus.enabled){
  const manifest=await json(options.captureAnimationManifestURL||'./local-original/capture-animations/manifest.json').catch(()=>null);
  if(manifest){try{captureBindings=captureAnimationBindings(manifest);captureStatus.manifestAvailable=true;}
   catch(error){captureStatus.errors.push(error.message);}}
 }
 const regionalCaptureStatus={manifestAvailable:false,near:0,far:0,errors:[]};
 const regionalCaptureCache=new Map();let regionalCaptureRows=null,regionalCaptureTeam=null;
 result.captureRegionalModels=regionalCaptureStatus;
 if(captureStatus.enabled&&options.captureRegionalManifestURL!==false){
  const manifest=await json(options.captureRegionalManifestURL||'./local-original/capture-animations/regional/manifest.json').catch(()=>null);
  if(manifest){try{
   regionalCaptureRows=regionalCaptureBindings(manifest);regionalCaptureStatus.manifestAvailable=true;
   if(manifest.teamManifest)regionalCaptureTeam=await verifiedModelJSON(manifest.teamManifest,manifest.teamManifestSha256,'Regional capture team manifest');
  }catch(error){regionalCaptureStatus.errors.push(error.message);}}
 }
 const productionStatus={manifestAvailable:false,near:0,far:0,errors:[]};
 const productionRows=new Map(),productionCache=new WeakMap();let productionTeam=null;
 result.productionModels=productionStatus;
 if(options.productionRegionalManifestURL){
  const source=await json(options.productionRegionalManifestURL).catch(()=>null);
  if(source){try{
   const rows=regionalModelEntries(source);
   if(source.teamManifest)productionTeam=await verifiedModelJSON(source.teamManifest,source.teamManifestSha256,'Production team manifest');
   for(const row of rows){const variants=productionRows.get(row.id)||[];variants.push(row);productionRows.set(row.id,variants);}
   productionStatus.manifestAvailable=true;
  }catch(error){productionStatus.errors.push(error.message);}}
 }
 const presentationEntries=[options.tradePresentation,options.fighterBomberPresentation,options.carrierPresentation,options.priestPresentation,options.spyPresentation,options.leaderPresentation,options.wonderPresentation,options.diversionPresentation,options.heavyMounted3Presentation,options.kothPresentation,options.fishingPresentation,options.granaryPresentation].filter(Boolean)
  .flatMap(presentation=>presentation.modelRows.map(row=>({...row,presentation})));
 const sailStatus={enabled:options.initialSails!==false,manifestAvailable:false,models:0,parts:0,errors:[]},sailErrors=new Set(),sailComposites=new Map();let sailManifest=null,sailShader=null,dynamicSailShader=null;const dynamicBindings=new WeakMap();
 result.initialSails=sailStatus;
 const sailError=message=>{if(!sailErrors.has(message)){sailErrors.add(message);sailStatus.errors.push(message);result.errors.push(message);}};
 if(sailStatus.enabled){try{
  sailManifest=await json(options.sailManifestURL||'./local-original/sails-native/manifest.json');
  if(sailManifest.format!=='ee2-native-sail-initial-composites-v1'||!Array.isArray(sailManifest.models))throw new Error('Unsupported native initial sail manifest');
  sailStatus.manifestAvailable=true;result.sailManifest=sailManifest;
 }catch{/* Old portable folders and all existing models remain usable. */}}
 const scoutIdle=options.scoutIdle?await loadOriginalScoutIdle({attachAnimationPayload}):null;
 const oilRigCurves=typeof options.oilRigVisualForActor==='function'?
  await verifiedModelJSON(OIL_RIG_CURVES_URL,OIL_RIG_CURVES_SHA,'Original Oil Rig curves').catch(error=>{
   result.errors.push(error.message);return null;
  }):null;
 const citizenWorkManifest=typeof options.citizenWorkVisualForActor==='function'?
  await verifiedModelJSON('./local-original/citizen-work/visibility.json',
   '7b91eb076a394869de6f4bd28bb5a4ddf2e45fd0410dd976236ecabd89adcf8c','Original Citizen work bindings').catch(error=>{
    result.errors.push(error.message);return null;
   }):null;
 const citizenWork=citizenWorkManifest?compileCitizenWork(citizenWorkManifest):null;
 const citizenWorkModels=new Map(citizenWorkManifest?.models.map(row=>[row.url,row])??[]);
 const citizenWorkHistory=citizenWork?createCitizenWorkHistory():null;
 const huntPoses=options.rectHuntPoseManifest?await loadOriginalHuntPoses(options.rectHuntPoseManifest).catch(error=>{
  result.errors.push(error.message);return null;
 }):null;
 const augmentModel=async(model,entry,lod='near',modelURL=entry.url??entry.file)=>{
  if(scoutIdle&&entry.id===57&&(entry.epoch??0)<6&&lod==='near')model=scoutIdle(model);
  if(model.animationPayload){try{model=await loadAnimationPayload(model,{base:import.meta.url});}catch(e){result.citizenModels.errors.push(e.message);result.errors.push('Citizen motion sidecar: '+e.message);model=withoutAnimationPayload(model);}}
  if(entry.id===8&&citizenWorkModels.has(modelURL.replace(/^\.\//,''))){
   model=augmentCitizenWork(model,modelURL,citizenWork);
  }
  if(entry.id===8&&huntPoses){try{model=await huntPoses.attach(model,modelURL);}catch(error){result.errors.push(error.message);}}
  const packRow=packManifest?.models.find(r=>r.id===entry.id&&r.epoch===(entry.epoch??0)&&r.modelURL===(entry.url||entry.file)&&r.sourceSha256===model.source?.sha256);
  if(packRow){try{
   if(!packCache.has(packRow.url))packCache.set(packRow.url,Promise.all([verifiedModelJSON(packRow.modelURL,packRow.parentSha256),verifiedModelJSON(packRow.url,packRow.sha256)]));
   const [exactParent,overlay]=await packCache.get(packRow.url);model=appendNativePackAnimations(exactParent,overlay);packStatus.models++;
  }catch(e){packStatus.errors.push(e.message);result.errors.push('Pack animation overlay: '+e.message);}}
  const captureBinding=captureBindings?.get(model.source?.sha256),captureAsset=captureBinding?.[lod];
  if(captureAsset){try{
   const key=captureAsset.sha256+':'+captureAsset.url;
   if(!captureCache.has(key))captureCache.set(key,verifiedModelJSON(captureAsset.url,captureAsset.sha256,'Capture animation overlay')
    .then(async overlay=>({overlay,...await loadCaptureAnimationOverlay(model,overlay,captureBinding.states,import.meta.url)})));
   const {overlay,animations,clips}=await captureCache.get(key);
   if(!captureOverlayMatches(model,overlay))throw new Error('Capture animation source or topology differs');
   model={...model,nativeCapture:{clips,contexts:captureBinding.contexts},parts:model.parts.map((part,index)=>({...part,animations:{...part.animations,...animations[index]}}))};
   captureStatus.models++;
  }catch(error){if(!captureStatus.errors.includes(error.message))captureStatus.errors.push(error.message);}}
  if(options.airTakeoff)model=await options.airTakeoff.augment(model,lod);
  const tradeSails=entry.presentation?.sailManifest;
  const sourceSails=tradeSails||sailManifest;
  const row=sourceSails?.models?.find(r=>r.source?.sha256===model.source?.sha256&&r.epoch===(entry.epoch??0));if(!sailStatus.enabled||!row||(!row.effect&&!options.dynamicSails))return model;
  try{
   if(!sailComposites.has(row.url))sailComposites.set(row.url,verifiedModelJSON(row.url,row.sha256));
   const composite=await sailComposites.get(row.url);
   if(composite.assetKind!==manifestAssetKind(entry)||(tradeSails?composite.family!==entry.sourceFamily:composite.id!==entry.id)||composite.epoch!==(entry.epoch??0))return model;
   const augmented=appendInitialSailParts(model,composite);
   if(options.dynamicSails&&(!tradeSails||entry.fishingAsset)){try{dynamicBindings.set(augmented,createDynamicSailBinding(model,composite,sourceSails,entry.yawOffset??model.yawOffset??0));}catch(e){sailError(e.message);}}
   return augmented;
  }catch(e){sailError(e.message);return model;}
 };
 try{if(!privatePack)teamManifest=await json(options.teamManifestURL||'./local-original/team-textures/manifest.json');}catch{/* The original neutral materials remain available. */}
 try{scales=privatePack?.scales??await json(options.scaleManifestURL||'./local-original/model-scales.json');}catch{/* Legacy normalized exports remain playable without native scale bindings. */}
 result.scaleManifest=scales;
 result.setLightingContext=context=>{
  for(const key of ['ambient','sun','direction'])if(!Array.isArray(context?.[key])||context[key].length!==3||!context[key].every(Number.isFinite))throw new RangeError('Invalid original model lighting');
  if(Math.hypot(...context.direction)<1e-12)throw new RangeError('Zero original model light direction');
  defaultLighting=context;result.lighting=context;return true;
 };
 result.setLightingSettings=settings=>{if(!lightingData)return false;return result.setLightingContext(createNativeLightingContext(lightingData,settings));};
 if(options.nativeLighting!==false){try{lightingData=await json(options.lightingURL||'./local-original/climate-lighting.json');result.setLightingSettings(options.lightingSettings);}catch{/* Older portable asset folders keep the explicit legacy adapter. */}}
 try{if(!privatePack)seasonManifest=await json('./local-original/model-seasons.json');}catch{/* Static original skins remain available in older asset folders. */}
 if(seasonManifest.teamManifest){try{seasonTeamManifest=await json(seasonManifest.teamManifest);}catch{/* Unverified or absent patch data keeps the correct neutral seasonal image. */}}
 result.seasonManifest=seasonManifest;
 const climateProfiles=options.climateProfiles||{},unavailableClimates=new Set();
 const authoredSequence=(manifest,climate,key)=>manifest.sets?.[climate]?.[key]?.urls;
 const profileSequence=(manifest,climate,key)=>authoredSequence(manifest,climate,key)||climateProfiles[climate]?.body[key];
 result.setSeasonPhase=phase=>{yearPhase=modelYearPhase(phase);return !!(seasonManifest.models||regionalSeasonManifest.models);};
 result.setSeasonClimate=climate=>{
  if(!Object.hasOwn(seasonManifest.sets||{},climate)&&!Object.hasOwn(regionalSeasonManifest.sets||{},climate)&&!Object.hasOwn(climateProfiles,climate))throw new RangeError('Unknown model season climate');
  if(unavailableClimates.has(climate))return false;
  seasonClimate=climate;return true;
 };
 let shader,packRigFrames=null,birdFlightFrames=null;
 if(packStatus.enabled||options.birdRigs?.size){
  try{
   shader=program(gl,FS,[],true,true);
   if(packStatus.enabled)packRigFrames=createPackRigFrames(gl);
   if(options.birdRigs?.size)birdFlightFrames=createBirdFlightFrames(gl);
  }
  catch{/* Keep the existing morph shader when the optional palette cannot compile. */}
 }
 try{shader??=program(gl,FS,[],packStatus.enabled);}catch(e){result.errors.push(e.message);console.warn('Originalmodell-Renderer:',e);return result;}
 const instances=gl.createBuffer(),textures=new Map();
 const dynamicGPU=options.dynamicSails?createDynamicSailGPU(gl,instances):null,dynamicSourceVariants=new Map();
 const navalMatrices=new WeakMap();
 const carrierHullBases=new WeakMap();
 result.carrierHullBasis=actor=>carrierHullBases.get(actor);
 const navalBasisForActor=actor=>{
  if(!navalMatrices.has(actor))navalMatrices.set(actor,options.navalBasisForActor?.(actor)??null);
  return navalMatrices.get(actor);
 };
 result.dynamicSails={enabled:!!dynamicGPU,state:()=>dynamicGPU?.status()??null};
 const actorVariant=(actor,epochs)=>{
  if(!actor||!epochs||!Number.isInteger(actor.def)||!Number.isInteger(actor.team))return null;
  let selected=null;for(const variant of dynamicSourceVariants.get(actor.def)||[]){if(variant.epoch<=(epochs[actor.team]??0)&&(!selected||variant.epoch>selected.epoch))selected=variant;}
  return selected;
 };
 result.resolveSailSource=(actor,epochs)=>dynamicGPU?actorVariant(actor,epochs)?.dynamicSailBinding??null:null;
 result.resolveSailForces=(actor,source,planeIndex,life)=>dynamicSailForces(actor,source,planeIndex,life,{
  rounding:options.dynamicSailRounding||'chop53',windDamping:options.dynamicSailWindDamping??0,
  bodyBasis:source.nativeHullBasis?navalBasisForActor(actor):null});
 result.prepareDynamicSails=inputs=>dynamicGPU?dynamicGPU.prepare(inputs):false;
 result.dynamicSailBounds=handle=>{const key=dynamicGPU?.sourceKey(handle);if(!key)return null;for(const variants of result.models.values())for(const variant of variants)if(variant.dynamicSailBinding?.key===key)return dynamicGPU.bounds(handle,variant,variant.navalBasis?navalBasisForActor:null);return null;};
 result.disposeDynamicSails=()=>dynamicGPU?.dispose();
 let terrainMaterial = null;
 if (options.terrainFallbackManifestURL) {
  try {
   terrainMaterial = createCityCenterTerrainBinding(await json(options.terrainFallbackManifestURL), async url => {
    try { return await texture(gl, url, textures); }
    catch (error) { textures.delete(url); throw error; }
   });
  } catch (error) { result.errors.push('Originales Gelände-Material: ' + error.message); }
 }
 result.bindNativeTerrainMaterial = context => terrainMaterial?.bind(context) ?? Promise.resolve(false);
 result.nativeTerrainMaterialSnapshot = () => terrainMaterial?.snapshot() ?? null;
 const houseLodStatus={enabled:!!options.houseLodManifestURL,models:0,groups:0,errors:[]};
 result.houseLod=houseLodStatus;let houseLodModels=new Map(),houseLodSettings=null;
 const houseLodError=error=>{houseLodStatus.errors.push(error.message);result.errors.push('House LOD: '+error.message);};
 result.setHouseLodProfile=settings=>{if(!houseLodStatus.enabled)return false;houseLodSettings=houseLodProfile(settings);return true;};
 if(houseLodStatus.enabled){try{
  houseLodModels=houseLodBindings(await json(options.houseLodManifestURL));
  if(options.houseLodProfile!==undefined)result.setHouseLodProfile(options.houseLodProfile);
 }catch(error){houseLodModels.clear();houseLodError(error);}}
 const houseReflectionCatalog=houseLodModels.size ? await verifiedModelJSON(HOUSE_REFLECTION_CATALOG.url,
  HOUSE_REFLECTION_CATALOG.sha256,'Original House reflection bounds').catch(()=>null) : null;
 const entries=[...(Array.isArray(manifest.models)?manifest.models:Object.entries(manifest.models||manifest.mapping||{}).map(([id,value])=>typeof value==='string'?{id:+id,url:value}:{id:+id,...value})),...regionalEntries,...citizenEntries,...presentationEntries];
 const farmSeedingElapsedForActor=typeof options.farmSeedingElapsedForActor==='function'?options.farmSeedingElapsedForActor:null;
 const farmSeedingSources=farmSeedingElapsedForActor?await loadFarmSeeding().catch(error=>{
  result.errors.push('Original Farm seeding: '+error.message);return null;
 }):null;
 const loadParts=async (model,keepPickingGeometry=false,modelUrl=null,teamOverride=null,textureHashes=null,packRig=null,shadowNodes=false,seasonOverride=null)=>{
   const parts=[],selectedSeasonManifest=seasonOverride??(regionalSeasonManifest.models?.[modelUrl]?regionalSeasonManifest:seasonManifest);
   let sourceSailIndex=0;for(const part of model.parts){
    const sailIndex=part.nativeSail?sourceSailIndex++:null;
    const url=part.textureUrl||part.texture||manifest.textures?.[part.textureSource?.toLowerCase()]||null;
    let materialTexture,darkTexture,darkTextures;
    try{
     const digest=textureHashes?.get(url);
     if(textureHashes&&!digest)throw new Error(`Unbound material ${url}`);
     materialTexture=await texture(gl,url,textures,!!part.nativeSail,digest);
     if(part.nativeSail){
      darkTexture=await texture(gl,part.nativeSail.damageMaskUrls[part.nativeSail.initialDamageMask],textures,true);
      if(!sailShader)sailShader=program(gl,SAIL_FS,['uDark','uSailDiffuse']);
      if(dynamicGPU){try{darkTextures=await Promise.all(part.nativeSail.damageMaskUrls.map(url=>texture(gl,url,textures,true)));if(!dynamicSailShader)dynamicSailShader=program(gl,DYNAMIC_SAIL_FS,['uDark','uSailDiffuse']);}catch(e){sailError('Dynamic sail mask preload: '+e.message);}}
     }
    }catch(e){if(!part.nativeSail)throw e;sailError(e.message);continue;}
    const gpu=gpuPart(gl,{...part,nativeStateClips:part.nativeStateClips||model.nativeStateClips},instances,materialTexture,keepPickingGeometry,packRig?.parts.get(part.sourceNode));
    if(shadowNodes&&Number.isInteger(part.sourceNode))gpu.shadowOrigin={url:modelUrl,node:part.sourceNode};
    if(part.nativeSail){
     gpu.nativeSail=part.nativeSail;gpu.darkTexture=darkTexture;gpu.darkTextures=darkTextures;gpu.sailIndex=sailIndex;gpu.forTeam=()=>gpu.texture;gpu.forSeason=()=>({first:gpu.texture,next:gpu.texture,blend:0});parts.push(gpu);sailStatus.parts++;continue;
    }
    const seasonalKey=modelSeasonBinding(selectedSeasonManifest,modelUrl,model,part),neutralGPU=new Map([[url,gpu.texture]]),teamGPU=new Map(),requested=new Set();
    // Preload every declared climate image before exposing the model. A seasonal
    // switch never displays an unrelated summer image while its pixels load.
    if(seasonalKey){
     const urls=new Set(Object.values(selectedSeasonManifest.sets||{}).flatMap(climate=>climate[seasonalKey]?.urls||[]));
     for(const source of urls){
      const digest=seasonOverride?textureHashes?.get(source):undefined;
      if(seasonOverride&&!digest)throw new Error(`Unbound seasonal material ${source}`);
      neutralGPU.set(source,await texture(gl,source,textures,false,digest));
     }
     for(const climate of Object.keys(climateProfiles)){
      if(authoredSequence(selectedSeasonManifest,climate,seasonalKey))continue;
      for(const source of climateProfiles[climate].body[seasonalKey]||[]){
       if(neutralGPU.has(source))continue;
       try{neutralGPU.set(source,await texture(gl,source,textures));}
       catch(error){unavailableClimates.add(climate);result.climateErrors.push(climate+': '+error.message);}
      }
     }
    }
    const forSourceTeam=(source,team)=>{
      const suffix=['A','B','C','G'][team],key=source?.split('/').pop().replace(/\.png$/i,'').toLowerCase();
      const patchManifest=teamOverride?.texture_bindings?.[source]?teamOverride
       :citizenTeamManifest.texture_bindings?.[source]?citizenTeamManifest
       :additionalRegionalTeam.texture_bindings?.[source]?additionalRegionalTeam
       :regionalTeamManifest.texture_bindings?.[source]?regionalTeamManifest
       :seasonTeamManifest.texture_bindings?.[source]?seasonTeamManifest:teamManifest;
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
    const usesTerrainMaterial = terrainMaterial?.matches(modelUrl, model, part);
    gpu.forSeason=team=>{
      if (usesTerrainMaterial) {
        const pair = terrainMaterial.frames(yearPhase);
        if (pair) return pair;
      }
      const sequence=profileSequence(selectedSeasonManifest,seasonClimate,seasonalKey),pair=modelSeasonFrames(sequence,yearPhase,url);
      return {first:forSourceTeam(pair.first,team),next:forSourceTeam(pair.next,team),blend:pair.blend};
    };
    parts.push(gpu);
   }
   return parts;
 };
 const rallyAttachments=new WeakMap();
 result.rallyAttachment=actor=>rallyAttachments.get(actor);
 const actorVisuals=options.sourceVisualForModel?new WeakMap():null;
 const actorSourceScales=options.priestParticleScales?new WeakMap():null;
 result.sourceVisualForActor=actor=>actorVisuals?.get(actor);
 result.sourceScaleForActor=actor=>actorSourceScales?.get(actor);
 const sourceVisual=(model,url)=>options.sourceVisualForModel?.(model,url);
 function rallyTag(model){
  const point=options.rallyTargetTags?.[model.source?.sha256]?.point, n=model.normalization;
  if(!point||!n)return null;
  return [point[0],point[2],-point[1]].map((value,index)=>(value+n.offset[index])*n.scale);
 }
 const regionalCaptureVariant=(entry,lod)=>{
  const url=lod==='near'?entry.url:entry.lodFarUrl,digest=lod==='near'?entry.sha256:entry.lodFarSha256;
  if(!url)return null;
  const key=digest+':'+url;
  if(!regionalCaptureCache.has(key)){
   const record={variant:null};regionalCaptureCache.set(key,record);
   verifiedModelJSON(url,digest,'Regional capture model').then(async model=>{
    const {parts,clips}=await loadRegionalCaptureAsset(model,entry,import.meta.url);
    const transform=nativeModelTransform(model,entry.nativeScaleBinding);
    if(!transform.native)throw new Error('Regional capture model has no exact source scale');
    const gpu=await loadParts({...model,parts},false,url,regionalCaptureTeam);
    record.variant={assetKind:0,parts:gpu,hasClipVisibility:gpu.some(part=>part.clipVisibility),sourceVisual:sourceVisual(model,url),
     sourceScale:transform.native?entry.nativeScaleBinding.sourceScale:null,
     nativeCapture:{clips},nativeScale:transform.scale,originOffset:transform.originOffset,yOffset:0,
     yawOffset:entry.yawOffset??model.yawOffset??0,rallyTag:rallyTag(model)};
    regionalCaptureStatus[lod]++;
   }).catch(error=>{regionalCaptureStatus.errors.push(`${url}: ${error.message}`);});
  }
  return regionalCaptureCache.get(key).variant;
 };
 const productionVariant=(entry,lod)=>{
  const url=lod==='near'?entry.url:entry.lodFarUrl,digest=lod==='near'?entry.sha256:entry.lodFarSha256;
  if(!url)return null;
  let cache=productionCache.get(entry);
  if(!cache){cache={};productionCache.set(entry,cache);}
  if(!cache[lod]){
   const record={variant:null};cache[lod]=record;
   verifiedModelJSON(url,digest,'Original production model').then(async body=>{
    const model=await augmentModel(body,entry,lod),transform=nativeModelTransform(model,entry.nativeScaleBinding);
    if(!transform.native)throw new Error('Production model has no exact source scale');
    const parts=await loadParts(model,false,url,productionTeam);
    if(!parts.length)throw new Error('Production model has no renderable parts');
    record.variant={sourceId:entry.id,assetKind:0,parts,hasClipVisibility:parts.some(part=>part.clipVisibility),sourceVisual:sourceVisual(model,url),
     sourceScale:transform.native?entry.nativeScaleBinding.sourceScale:null,
     nativeScale:transform.scale,originOffset:transform.originOffset,yOffset:0,farRequested:lod==='far',
     yawOffset:entry.yawOffset??model.yawOffset??0,rallyTag:rallyTag(model),nativeTakeoff:model.nativeTakeoff,
     navalBasis:navalBasisModel(model,entry.id),wakeSource:{sha256:model.source?.sha256,scale:entry.nativeScaleBinding.sourceScale}};
    productionStatus[lod]++;
   }).catch(error=>{productionStatus.errors.push(`${url}: ${error.message}`);});
  }
  return cache[lod].variant;
 };
 const productionEntryFor=(sourceId,epoch,civ)=>productionRows.get(sourceId)?.find(row=>row.epoch<=epoch&&epoch<row.until&&
   (row.regionalCivilizations.length===14||row.regionalCivilizations.includes(civ)));
 const scaffold=options.wonderScaffold;
 let scaffoldModules=null;
 if(scaffold){try{
  scaffoldModules=await Promise.all(scaffold.templates.map(async row=>{
   const model=await verifiedModelJSON(row.url,row.sha256,'Original scaffold model');
   const transform=nativeModelTransform(model,{verified:true,sourceScale:scaffold.sourceScale,sourceSha256:row.sourceSha256});
   if(model.format!=='ee2-original-mesh-v1'||!transform.native||model.parts?.length!==1)throw new Error('Original scaffold model differs');
   const parts=await loadParts(model,false,row.url,null,scaffold.textures);
   if(parts.length!==1)throw new Error('Original scaffold material is missing');
   return {parts,assetKind:4,yOffset:0,originOffset:transform.originOffset,nativeScale:transform.scale};
  }));
 }catch(error){result.errors.push(error.message);}}
 const scaffoldBindings=new Map();
 const fence=options.wonderFence;
 let fenceModule=null;
 if(fence){try{
  const row=fence.foundation,model=await verifiedModelJSON(row.url,row.sha256,'Original construction fence');
  const transform=nativeModelTransform(model,{verified:true,sourceScale:fence.sourceScale,sourceSha256:row.sourceSha256});
  if(model.format!=='ee2-original-mesh-v1'||!transform.native||model.parts?.length!==1)throw new Error('Original construction fence differs');
  const parts=await loadParts(model,false,row.url,null,fence.textures);
  if(parts.length!==1)throw new Error('Original construction fence material is missing');
  fenceModule={parts,assetKind:4,yOffset:0,originOffset:transform.originOffset,nativeScale:transform.scale};
 }catch(error){result.errors.push(error.message);}}
 const privateTextures=privatePack?new Map(Object.entries(manifest.texturePins)):null;
 let loaded=0;
 for(const entry of entries){
  onProgress(++loaded,entries.length);
  if(entry.verified===false)continue;
  try{
   const id=+(entry.id??entry.def);if(!Number.isFinite(id))continue;
   const assetKind=manifestAssetKind(entry),bindings=sourceRenderBindings(assetMapping,assetKind,id);if(!bindings.length)continue;
   const modelUrl=entry.url||entry.file;
   let houseBinding=assetKind===0&&id===1?houseLodModels.get(modelUrl):null,body;
   if(houseBinding){try{
    if(!entry.nativeScaleBinding?.verified||entry.nativeScaleBinding.sourceScale!==houseBinding.sourceScale
       ||entry.nativeScaleBinding.sourceSha256!==houseBinding.sourceSha256)throw new Error('House LOD scale differs');
    body=await readHouseLodModel(houseBinding,houseBinding.levels[0],import.meta.url);
   }catch(error){houseBinding=null;houseLodError(error);}}
   const sourceSha=entry.source?.sha256,rigBinding=packRigFrames&&assetKind===0&&!entry.presentation&&packRigBinding(id,sourceSha);
   const qualified=rigBinding&&await loadPackRigModel(absolute(modelUrl),sourceSha,rigBinding);
   const huntModelSha=id===8?huntPoses?.modelHash(modelUrl):null;
   const citizenWorkSha=id===8?citizenWorkModels.get(modelUrl.replace(/^\.\//,''))?.sha256:null;
   const farmFamilies=typeof options.farmFamilyForActor==='function';
   const farmCrops=assetKind===0&&id===2&&entry.nativeFarmComposite?.family==='FarmWheat'&&
    (typeof options.farmVisualForActor==='function'||farmFamilies);
   const farmBaseSha=farmCrops?FARM_CROP_MODELS[0][1][3]:null;
   const oilRigModel=assetKind===0&&id===53&&oilRigCurves;
   const raw=body??qualified?.model??await ((entry.presentation||privatePack||huntModelSha||citizenWorkSha||farmBaseSha||oilRigModel)?
    verifiedModelJSON(modelUrl,huntModelSha??citizenWorkSha??farmBaseSha??(oilRigModel?oilRigCurves.modelSha256.near:entry.sha256),'Original unit model'):json(modelUrl));
   const model=await augmentModel(raw,{...entry,id});
   const rig=qualified?.rig&&model.parts===raw.parts?qualified.rig:null;
   const birdSource=birdFlightFrames&&assetKind===1?options.birdRigs.get(id):null;
   const birdRig=birdSource?prepareBirdFlightRig(birdSource.source,model,birdSource.modelPin):null;
   const house=assetKind===0&&id===1;
   const parts=await loadParts(model,assetKind===1,modelUrl,entry.presentation?.teamManifest,privateTextures??entry.presentation?.textureHashes,rig??birdRig,house,entry.presentation?.seasonManifest);
   if(!parts.length)continue;
   const scaleKey=(entry.assetKind===undefined?'':assetKind+':')+id+':'+(entry.epoch??0),sea=assetKind===0&&((id>=29&&id<=36)||id===42||id===45);
   const scaleBinding=entry.regionalCivilizations||entry.wonderFamily||entry.kothObjective||entry.fishingAsset||entry.granaryAsset?entry.nativeScaleBinding:scales.variants?.[scaleKey];
   const transform=nativeModelTransform(model,scaleBinding);
   const yOffset=!transform.native&&sea?-(model.normalization?.offset?.[1]||0)*(model.normalization?.scale||1):0;
   const variant={sourceId:id,assetKind,scaleKey,epoch:entry.epoch??0,until:entry.until,regionalCivilizations:entry.regionalCivilizations,regionalFallback:entry.regionalFallback===true,sourceIdentity:entry.sourceIdentity,region:entry.region,parts,hasClipVisibility:parts.some(part=>part.clipVisibility),nativeAttackClock:entry.nativeAttackClock===true,yOffset,originOffset:transform.originOffset,nativeScale:transform.native?transform.scale:null,yawOffset:entry.yawOffset??model.yawOffset??0,farUrl:entry.lodFarUrl,far:null,farRequested:false,pickBounds:resourceBounds(parts),rallyTag:rallyTag(model),nativeTakeoff:model.nativeTakeoff};
   if(oilRigModel)variant.nativeOilRig=bindOilRigModel(model,oilRigCurves,'near');
   if(farmCrops){
    const families=farmFamilies?FARM_CROP_MODELS:FARM_CROP_MODELS.slice(0,1);
    variant.farmCropFamilies=await Promise.all(families.map(async ([family,digests],familyIndex)=>{
     try{return await Promise.all(FARM_STATES.map(async (state,stage)=>{
      if(family==='FarmWheat'&&stage===3)return farmPlotParts(model,parts,family,state);
      const url=`./local-original/models/native-${family.toLowerCase()}-${state.toLowerCase()}.json`;
      const crop=await verifiedModelJSON(url,digests[stage],'Original Farm crop');
      const cropBinding=crop.nativeFarmComposite;
      const pose=nativeModelTransform(crop,{verified:true,sourceSha256:crop.source?.sha256,sourceScale:cropBinding?.scale?.world});
      if(cropBinding?.source?.sha256!==model.nativeFarmComposite.source?.sha256||
         !transform.native||!pose.native||pose.scale!==transform.scale||
         pose.originOffset.some((value,index)=>value!==transform.originOffset[index]))throw new Error('Farm crop transform differs');
      let stageModel=crop;
      if(stage===0&&farmSeedingSources?.[familyIndex]){
       try{stageModel=bindFarmSeeding(crop,farmSeedingSources[familyIndex],digests[stage]);}
       catch(error){result.errors.push('Original Farm seeding: '+error.message);}
      }
      const stageParts=await loadParts(stageModel,false,url,entry.presentation?.teamManifest,
       privateTextures??entry.presentation?.textureHashes,null,false,entry.presentation?.seasonManifest);
      return farmPlotParts(crop,stageParts,family,state);
     }));}catch(error){result.errors.push(`Original ${family} crop models: ${error.message}`);return null;}
    }));
   }
   variant.nativeCapture=model.nativeCapture;
   variant.nativeCitizenWork=model.nativeCitizenWork;
   variant.nativePackBlend=options.packAnimations&&assetKind===0&&hasSourcePackBlend(id,model);
   variant.nativePackRig=rig;
   variant.nativeBirdRig=birdRig;
   variant.navalBasis=transform.native&&navalBasisModel(model,id);
   if(variant.navalBasis)variant.wakeSource={sha256:model.source.sha256,scale:scaleBinding.sourceScale};
   if(id===29&&variant.navalBasis&&model.source.sha256===options.boatWakeBodySha)result.boatWakeHull=true;
   variant.hercAttack=options.hercAttack?.acceptsModel(id,model)??false;
   variant.sourceVisual=sourceVisual(model,modelUrl);
   variant.sourceScale=transform.native?scaleBinding.sourceScale:null;
   if(assetKind===0&&id<=1&&transform.native)variant.shadowSource={url:modelUrl,source:model.source?.sha256};
   if(entry.wonderFamily&&transform.native){
    const minimum=model.sourceBounds?.min?.[1], maximum=model.sourceBounds?.max?.[1];
    const {offset,scale}=model.normalization;
    // 0063E12A uses the main model's shared minimum and vertical extent.
    // The imported Y axis is native Z. Every material part uses one plane.
    if(Number.isFinite(minimum)&&Number.isFinite(maximum)&&maximum>minimum){
     variant.wonderConstructionPlane=[(minimum+offset[1])*scale,(maximum-minimum)*scale];
    }
    const scaffoldBinding=scaffoldModules&&scaffold.rows.get(id);
    if(scaffoldBinding?.sourceSha256===model.source?.sha256)variant.wonderScaffold=scaffoldBinding;
    const fenceBinding=fenceModule&&fence.rows.get(id);
    if(fenceBinding?.sourceSha256===model.source?.sha256)variant.wonderFence=fenceBinding;
   }
   if(entry.presentation)variant.presentationEntry=entry;
   if(entry.priestClips)variant.priestClips=entry.priestClips;
   if(entry.spyClips)variant.spyClips=entry.spyClips;
   if(entry.diversionVisual)variant.diversionVisual=entry.diversionVisual;
   if(assetKind===0&&id===22&&entry.verified&&model.databaseVisualBinding?.visualDefinition==='LightArtillery1_01'
      &&entry.epoch===0&&entry.until===2&&entry.source?.sha256===model.source?.sha256){
    // Only clips with the original moving-state KF can start its text-key sound.
    const sources = ['8b98e42397fa4a68b1f8363849636c450582195c14c7a37b0bf46436676dda14',
                     '282a8d02f347eb41d49e26bc93d038e05a13006dc3395f9e95c7703608e3c800'];
    variant.stoneThrowerWheelClips = new Set(model.animationSources?.filter(row=>sources.includes(row.sha256)).map(row=>row.name));
   }
   if(houseBinding){try{
    const levelParts=[parts];
    for(const level of houseBinding.levels.slice(1)){
     const source=await readHouseLodModel(houseBinding,level,import.meta.url);
     levelParts.push(await loadParts(source,false,level.url,null,null,null,true));
    }
    variant.selectHouseLod=bindHouseLodParts(houseBinding,levelParts);
    if(transform.native&&model===body&&variant.yawOffset===0)
     variant.houseReflectionBound=bindHouseReflectionBound(houseReflectionCatalog,modelUrl,houseBinding,model);
    houseLodStatus.models++;houseLodStatus.groups+=houseBinding.groups.length;
   }catch(error){houseLodError(error);}}
   if(dynamicBindings.has(model)){
    variant.dynamicSailBinding=Object.freeze({...dynamicBindings.get(model),nativeHullBasis:variant.navalBasis&&variant.yawOffset===-Math.PI/2});
    const list=dynamicSourceVariants.get(id)||[];list.push(variant);dynamicSourceVariants.set(id,list);
   }
   if(entry.sourceIdentity!==undefined)result.citizenModels.loaded++;else if(entry.regionalCivilizations)result.regionalModels.loaded++;
   if(model.nativeSailComposite){variant.sailComposite=model.nativeSailComposite;sailStatus.models++;}
   for(const {renderId}of bindings){const variants=result.models.get(renderId)||[];variants.push(variant);result.models.set(renderId,variants);result.available.add(renderId);if(assetKind===1)result.resourceRenderIds.add(renderId);}
  }catch(e){result.errors.push(e.message);console.warn('Originalmodell:',entry,e);}
 }
 // These families have no separate legacy original model rows. Use a complete
 // family or its procedural fallback when any required Near stage is missing.
 const presentationCounts=new Map();
 for(const entry of presentationEntries)presentationCounts.set(entry.id,(presentationCounts.get(entry.id)||0)+1);
 for(const [id,count] of presentationCounts){
  for(const {renderId} of sourceRenderBindings(assetMapping,0,id)){
   if(result.models.get(renderId)?.filter(variant=>variant.presentationEntry).length===count)continue;
   result.models.delete(renderId);result.available.delete(renderId);
  }
 }
 const rocket=options.heavyMounted3Presentation?.projectile;
 const guidedBindings=assetMapping&&sourceRenderBindings(assetMapping,2,7);
 if(rocket?.name==='ProjectileRocketHerc'&&guidedBindings?.length===1){
  try{
   const {url,sha256}=rocket.model;
   const model=await verifiedModelJSON(url,sha256,'Original HERC rocket');
   const transform=nativeModelTransform(model,{verified:true,sourceScale:rocket.sourceScale,sourceSha256:rocket.mesh.sha256});
   if(model.format!=='ee2-original-mesh-v1'||model.databaseVisualBinding?.visualDefinition!==rocket.name||
      !transform.native||model.parts.length!==1||model.parts[0].indices.length!==144)throw Error('Original HERC rocket differs');
   const textures=new Map(rocket.textureDependencies.map(row=>[row.url,row.sha256]));
   const parts=await loadParts(model,false,url,null,textures);
   const {renderId}=guidedBindings[0];
   result.models.set(renderId,[{sourceId:7,assetKind:2,epoch:0,parts,guidedRocket:true,yOffset:0,
    originOffset:transform.originOffset,nativeScale:transform.scale,yawOffset:0}]);
   result.available.add(renderId);
  }catch(error){result.errors.push(error.message);}
 }
 for(const list of result.models.values())list.sort((a,b)=>a.epoch-b.epoch);
 const wakeVariants=new Map();
 for(const definition of navalPoseModels.keys()){
  const [binding]=sourceRenderBindings(assetMapping,0,definition);
  if(binding&&result.models.has(binding.renderId))wakeVariants.set(definition,result.models.get(binding.renderId));
 }
 result.boatWakeSourceForActor=({def,epoch},civ)=>{
  const variants=wakeVariants.get(def);
  if(!variants)return null;
  const fallback=selectRegionalModel(variants,epoch,civ);
  if(!fallback)return null;
  const entry=productionEntryFor(def,epoch,civ);
  const variant=(entry&&productionVariant(entry,'near'))||fallback;
  return variant.navalBasis&&nativeNavalYaw(variant)?variant.wakeSource:null;
 };
 result.hasBoatWakeSources=wakeVariants.size>0;
 for(const [renderId,variants] of result.models){
  if(variants.length===1&&variants[0].wonderScaffold)scaffoldBindings.set(renderId,variants[0].wonderScaffold);
 }
 const legacyScaffold=sourceRenderBindings(assetMapping,4,111)[0]?.renderId;
 result.wonderScaffoldModels=scaffoldBindings.size;
 result.maskWonderScaffoldFallback=(data,ranges,fallback)=>{
  if(!scaffoldBindings.size||legacyScaffold===undefined||!ranges[2*legacyScaffold+1])return 0;
  const keys=new Set();
  for(const [renderId,binding] of scaffoldBindings){
   const first=ranges[2*renderId],end=first+ranges[2*renderId+1];
   for(let i=first;i<end;i++)if(wonderScaffoldStage(binding,data,12*i))keys.add(scaffoldInstanceKey(data,12*i));
  }
  let masked=0;
  const first=ranges[2*legacyScaffold],end=first+ranges[2*legacyScaffold+1];
  for(let i=first;i<end;i++)if(keys.has(scaffoldInstanceKey(data,12*i))){fallback[12*i+8]=0;masked++;}
  return masked;
 };
 result.wonderFenceModels=[...result.models.values()].filter(variants=>variants.some(variant=>variant.wonderFence)).length;
 if(result.models.size){
  const profile=lightingData?.profiles?.[options.lightingSettings?.profile??'native155'];
  for(const [name, extra] of Object.entries(climateProfiles)){
   if(name!=='Temperate'&&!unavailableClimates.has(name)&&profile?.climates?.[name]?.Afternoon&&
     (Object.hasOwn(seasonManifest.sets||{},name)||Object.hasOwn(regionalSeasonManifest.sets||{},name)||Object.keys(extra.body).length)){
    result.seasonClimates.push(name);
   }
  }
 }
 gl.useProgram(shader.p);gl.uniform1i(shader.u.uDiffuse,0);gl.uniform1i(shader.u.uDiffuseNext,7);gl.uniform1i(shader.u.uFog,2);gl.uniform1i(shader.u.uFrames,5);gl.uniform1i(shader.u.uTerrain,6);gl.uniform3fv(shader.u.uTeam,new Float32Array(teams.flat()));
 if(packStatus.enabled||birdFlightFrames)gl.uniform1i(shader.u.uPackOutgoingFrames,8);
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
 result.clearTerrain=()=>{heightSize=1;heightReady=false;};
 const emptyFrames=gl.createTexture();gl.activeTexture(gl.TEXTURE5);gl.bindTexture(gl.TEXTURE_2D_ARRAY,emptyFrames);
 gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.RGBA32F,1,1,1);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.NEAREST);gl.activeTexture(gl.TEXTURE0);
 const timer=gl.getExtension('EXT_disjoint_timer_query_webgl2');let gpuQuery=null,nextQuery=0;
 // Keep only the displayed resource transforms. PIP can overwrite the WASM
 // instance buffer after the main frame, and grouping keys are draw internals.
 const pickModels=[];
 const PICK_STRIDE=6;
 let pickData=new Float32Array(0),pickIds=null;
 const pickBases=[];
 result.clearResourcePicks=()=>{pickModels.length=0;pickBases.length=0;pickIds=null;};
 function rememberResources(data,indices,variant,activity,resourceIdentities,forestBases){
  if(resourceIdentities)pickIds??=[];
  const needed=(pickModels.length+indices.length)*PICK_STRIDE;
  if(needed>pickData.length){
   const grown=new Float32Array(Math.max(needed,pickData.length*2));
   grown.set(pickData.subarray(0,pickModels.length*PICK_STRIDE));pickData=grown;
  }
  for(const i of indices){
   if(resourceIdentities?.[i]===0xffff)continue;
   const source=i*12,at=pickModels.length*PICK_STRIDE;
   pickData[at]=data[source];pickData[at+1]=data[source+1];pickData[at+2]=data[source+2];
   pickData[at+3]=data[source+3];pickData[at+4]=data[source+8];pickData[at+5]=activity;
   if(pickIds)pickIds.push(resourceIdentities[i]);
   pickBases.push(forestBases?.get(i));
   pickModels.push(variant);
  }
 }
 result.pickResourceHit=(origin,direction,maxDistance=Infinity,admit)=>{
  if(!origin||!direction||origin.length<3||direction.length<3)return null;
  for(let a=0;a<3;a++)if(!Number.isFinite(origin[a])||!Number.isFinite(direction[a]))return null;
  if(direction[0]**2+direction[1]**2+direction[2]**2<1e-20)return null;
  if(!(maxDistance>0))return null;
  let closest=maxDistance,hit=null;
  for(let i=0;i<pickModels.length;i++){
    if(admit&&!admit(pickIds?.[i]))continue;
    const at=i*PICK_STRIDE,variant=pickModels[i];
    const scale=variant.nativeScale??pickData[at+4];if(!(scale>0))continue;
    const yaw=pickData[at+3]+variant.yawOffset,c=Math.cos(yaw),s=Math.sin(yaw),o=variant.originOffset;
    const dx=origin[0]-pickData[at],dy=origin[1]-pickData[at+2],dz=origin[2]-pickData[at+1];
    const basis=pickBases[i];
    const localOrigin=basis ? [
     (dx*basis[0]+dy*basis[1]+dz*basis[2])/scale-o[0],
     (dx*basis[3]+dy*basis[4]+dz*basis[5])/scale-variant.yOffset-o[1],
     (dx*basis[6]+dy*basis[7]+dz*basis[8])/scale-o[2],
    ] : [(dx*c+dz*s)/scale-o[0],dy/scale-variant.yOffset-o[1],(-dx*s+dz*c)/scale-o[2]];
    const localDirection=basis ? [0,3,6].map(k=>(direction[0]*basis[k]+direction[1]*basis[k+1]+direction[2]*basis[k+2])/scale)
     : [(direction[0]*c+direction[2]*s)/scale,direction[1]/scale,(-direction[0]*s+direction[2]*c)/scale];
    if(!intersectsBounds(localOrigin,localDirection,variant.pickBounds,closest))continue;
    let distance=closest;
    for(const part of variant.parts){
     if(!part.pickGeometry||(part.activities&&!part.activities.includes(pickData[at+5])))continue;
     if(intersectsBounds(localOrigin,localDirection,part.pickGeometry.bounds,distance))distance=rayTriangles(localOrigin,localDirection,part.pickGeometry,distance);
    }
    if(distance<closest){closest=distance;hit={x:pickData[at],z:pickData[at+1],distance,identity:pickIds?.[i]};}
  }
  return hit;
 };
 result.pickResource=(origin,direction,maxDistance)=>{
  const hit=result.pickResourceHit(origin,direction,maxDistance);
  return hit?[hit.x,hit.z]:null;
 };
 const packBlendHistory=packStatus.enabled?createNativePackBlendHistory():null;
 result.buildingShadowCasters=({data,ranges,epochs,zoom,eye,actorMetadata})=>{
  const casters=[];
  for(const actor of actorMetadata?.actors??[]){
   const id=actor.def;
   if((id!==0&&id!==1)||actor.built<1||actor.hpRatio<=0||actor.renderFlags&24)continue;
   const i=actor.index,at=i*12,renderId=Math.round(data[at+4]),team=Math.min(3,Math.round(data[at+5]));
   if(i<ranges[renderId*2]||i>=ranges[renderId*2]+ranges[renderId*2+1]||actor.team!==team||
      actor.renderX!==data[at]||actor.renderY!==data[at+1]||actor.renderAltitude!==data[at+2]||actor.renderYaw!==data[at+3])continue;
   const epoch=epochs[team]||0,civ=visualCivilizations?.[team];
   if(regionalCaptureRows?.has(`${id}:${epoch}:${civ}`)||productionEntryFor(id,epoch,civ))continue;
   const variant=selectRegionalModel((result.models.get(renderId)??[]).filter(row=>row.sourceIdentity===undefined),epoch,civ);
   if(!variant?.shadowSource||variant.sourceId!==id||variant.assetKind!==0||variant.hasClipVisibility)continue;
   const shown=houseLodSettings&&variant.selectHouseLod ? variant.selectHouseLod(variant,data,i,eye,houseLodSettings) :
    (variant.nativeScale*zoom<55&&variant.far ? variant.far : variant);
   if(shown.parts.some(part=>part.animations?.idle||part.clipVisibility))continue;
   const modelParts=id===1 ? shown.parts.map(part=>part.shadowOrigin) : null;
   if(modelParts?.some(part=>!part))continue;
   casters.push({instance:i,url:id===0&&shown===variant.far?variant.farUrl:variant.shadowSource.url,source:variant.shadowSource.source,
    scale:variant.sourceScale,modelParts,pose:[actor.renderX,actor.renderY,actor.renderAltitude,Math.fround(actor.renderYaw+variant.yawOffset)]});
  }
  return casters;
 };
 result.draw=({data,ranges,vp,epochs,zoom=80,shadow=false,eye=[0,200,0],lighting=defaultLighting,clipPlane=null,capture=false,actorMetadata=null,preparedSailMetadata=actorMetadata,civilizations=visualCivilizations,stoneThrowerWheels=null,granaryLoops=null,hercBeamMuzzles=null,packScene=null,volumeShadowInstances=null,resourceIdentities=null,forestBases=null,houseReflection=null,rectHuntFrame=null,commitRectHunt=false})=>{
  const started=performance.now();let calls=0,triangles=0,querying=false;
  // Reflection uses the main snapshot. Only the independent picture-in-picture
  // capture must leave the main animation history alone.
  const packBlendReady=packBlendHistory?.begin(actorMetadata,capture&&!clipPlane);
  const packRigReady=packRigFrames?.begin(actorMetadata,capture&&!clipPlane);
  birdFlightFrames?.begin();
  const scaffoldDraws=[];
  const fenceDraws=[];
  let guidedRocketDraws=0;
  const collectActorPresentation=(actorVisuals||actorSourceScales)&&!shadow&&!clipPlane;
  const collectCarrierBasis=!shadow&&!clipPlane;
  if(collectCarrierBasis)for(const actor of actorMetadata?.actors??[]){
   if(actor.def===84)carrierHullBases.delete(actor);
  }
  if(collectActorPresentation)for(const actor of actorMetadata?.actors??[]){
   actorVisuals?.delete(actor);actorSourceScales?.delete(actor);
  }
  const previousState=drawState(gl);
  if(resourceIdentities&&(!(resourceIdentities instanceof Uint16Array)||resourceIdentities.length!==data.length/12))
   throw new TypeError('Resource identities do not match the render instances');
  const rememberPicks=!shadow&&(!capture||!!resourceIdentities);
  if(rememberPicks)result.clearResourcePicks();
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
  if(packBlendHistory){gl.activeTexture(gl.TEXTURE0+8);gl.bindTexture(gl.TEXTURE_2D_ARRAY,emptyFrames);}
  gl.activeTexture(gl.TEXTURE0+6);gl.bindTexture(gl.TEXTURE_2D,heightTexture);gl.uniform1i(shader.u.uTerrainSize,heightSize);gl.uniform1i(shader.u.uTerrainEnabled,heightReady?1:0);
  let activeShader=shader,sailDiffuse=null;
  const collectStoneThrowerWheels=stoneThrowerWheels&&!shadow&&!capture&&!clipPlane;
  const collectGranaryLoops=granaryLoops&&!shadow&&!capture&&!clipPlane;
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
   if(shadow&&variants[0].guidedRocket)continue;
   const first=ranges[id*2],groups=new Map();
   for(let i=first;i<first+count;i++){
    if(shadow&&volumeShadowInstances?.has(i))continue;
    const rowTeam=Math.round(data[i*12+5]),team=Math.min(3,rowTeam),epoch=epochs[team]||0;
    const actor=actorMetadata?.forInstance?.(i);let identity;
    if(options.citizenManifestURL&&variants[0].assetKind===0&&variants[0].sourceId===8&&actor&&options.sourceIdentityForActor){try{const sourceIdentity=options.sourceIdentityForActor(actor);identity=sourceIdentity===0||sourceIdentity===1?sourceIdentity:null;}catch(e){identity=null;result.citizenModels.errors.push(e.message);}}
    const citizen=selectCitizenModel(variants,epoch,civilizations?.[team],identity,{neutralFallback:options.citizenNeutralFallback??null});
    let variant=citizen===undefined?selectRegionalModel(variants.filter(v=>v.sourceIdentity===undefined),epoch,civilizations?.[team]):citizen;if(!variant)continue;
    const sourceId=variant.sourceId,time=data[i*12+10],seed=data[i*12+6],previewFlags=Math.round(data[i*12+7])&24;
    // The initial site has a separate perimeter and a null main model.
    if(variant.presentationEntry?.wonderFamily&&seed===0&&!previewFlags){
     const at=i*12,binding=variant.wonderFence;
     if(!shadow&&binding&&wonderFenceAtSite(data,at)){
      const yaw=data[at+3]+variant.yawOffset,c=Math.cos(yaw),s=Math.sin(yaw),key=`fence:${team}`;
      const fenceGroup=groups.get(key)||{variant:fenceModule,team,activity:0,clip:'idle',once:false,preview:false,list:[]};
      for(const segment of binding.segments){
       fenceGroup.list.push(data[at]+segment.x*c-segment.y*s,data[at+1]+segment.x*s+segment.y*c,
        data[at+2]+binding.altitude,yaw+segment.yaw,id,team,1,Math.round(data[at+7])&~64,fenceModule.nativeScale,0,0,0);
      }
      fenceDraws.push({id:sourceId,instances:binding.segments.length});groups.set(key,fenceGroup);
     }
     continue;
    }
    const civ=civilizations?.[team],regionalEntry=regionalCaptureRows?.get(`${sourceId}:${epoch}:${civ}`);
    const productionEntry=variant.assetKind===0&&productionEntryFor(sourceId,epoch,civ);
    const matchingActorPose=!previewFlags&&actor?.def===sourceId
     &&actor.index===i&&actor.hpRatio>0&&actor.renderTime===time&&actor.renderX===data[i*12]&&actor.renderY===data[i*12+1]
     &&actor.renderAltitude===data[i*12+2]&&actor.renderYaw===data[i*12+3];
    const matchingActor=matchingActorPose&&actor.team===team;
    const powerClips=variant.priestClips||variant.spyClips;
    const exactActor=(regionalEntry||variant.nativeCapture||variant.nativeTakeoff||productionEntry||powerClips)&&matchingActor;
    const regional=exactActor&&regionalEntry&&regionalCaptureVariant(regionalEntry,'near');
    const production=productionEntry&&productionVariant(productionEntry,'near');
    if(production)variant=production;
    if(regional)variant=regional;
    const variantIndex=regional?'capture-regional':production?'production':variants.indexOf(variant),far=(variant.nativeScale??data[i*12+8])*zoom<55;
    if(houseLodSettings&&variant.selectHouseLod)variant=variant.selectHouseLod(variant,data,i,eye,houseLodSettings);
    else if(far&&regional)variant=regionalCaptureVariant(regionalEntry,'far')||variant;
    else if(far&&production)variant=productionVariant(productionEntry,'far')||variant;
    else if(far&&variant.farUrl){
     if(!variant.farRequested){
      variant.farRequested=true;const near=variant;
      const presentation=near.presentationEntry;
      const huntModelSha=near.sourceId===8?huntPoses?.modelHash(near.farUrl):null;
      const oilRigSha=near.nativeOilRig?.curves.modelSha256.far;
      const citizenWorkSha=near.sourceId===8?citizenWorkModels.get(near.farUrl.replace(/^\.\//,''))?.sha256:null;
      const request=presentation||huntModelSha||citizenWorkSha||oilRigSha?verifiedModelJSON(near.farUrl,huntModelSha??citizenWorkSha??oilRigSha??presentation.lodFarSha256,'Original Far unit model'):json(near.farUrl);
      const shadowNodes=near.assetKind===0&&near.sourceId===1;
      request.then(model=>augmentModel(model,presentation||{id:near.sourceId,assetKind:near.assetKind,epoch:near.epoch},'far',near.farUrl))
       .then(async model=>({model,parts:await loadParts(model,near.assetKind===1,near.farUrl,presentation?.presentation?.teamManifest,null,null,shadowNodes)}))
       .then(({model,parts})=>{near.far={...near,parts,nativeOilRig:oilRigSha?bindOilRigModel(model,near.nativeOilRig.curves,'far'):null,nativePackBlend:false,nativePackRig:null,sourceVisual:sourceVisual(model,near.farUrl),pickBounds:resourceBounds(parts),sailComposite:model.nativeSailComposite,dynamicSailBinding:null,nativeCapture:model.nativeCapture,nativeCitizenWork:model.nativeCitizenWork,rallyTag:rallyTag(model),nativeTakeoff:model.nativeTakeoff,navalBasis:near.navalBasis&&navalBasisModel(model,near.sourceId),hercAttack:options.hercAttack?.acceptsModel(near.sourceId,model)??false};})
       .catch(e=>{result.errors.push(e.message);console.warn('Original-Fernmodell:',e);});
     }
     if(variant.far)variant=variant.far;
    }
    if(capture&&clipPlane&&!admitsHouseReflection(houseReflection,variant,matchingActor?actor:null,houseLodSettings?.mode))continue;
    const shimmer=variant.spyClips&&matchingActorPose&&actor.team===rowTeam?options.spyShimmerForActor?.(actor):
     variant.diversionVisual&&matchingActor&&actor.team===rowTeam?options.diversionShimmerForActor?.(actor,variant.diversionVisual):null;
    if(shimmer?.opacity===0)continue;
    const navalBasis=matchingActor&&variant.navalBasis&&nativeNavalYaw(variant)
     ?navalBasisForActor(actor):null;
    if(collectCarrierBasis&&navalBasis&&variant.sourceId===84)carrierHullBases.set(actor,navalBasis);
    if(collectActorPresentation&&matchingActor){
     if(variant.sourceVisual)actorVisuals?.set(actor,variant.sourceVisual);
     if(variant.sourceScale>0)actorSourceScales?.set(actor,variant.sourceScale);
    }
    if(variant.rallyTag&&actor&&!shadow&&!clipPlane){
     const scale=variant.nativeScale??data[i*12+8], yaw=actor.renderYaw+variant.yawOffset;
     const [x,y,z]=variant.rallyTag.map((value,index)=>(value+variant.originOffset[index])*scale);
     const c=Math.cos(yaw),s=Math.sin(yaw);
     rallyAttachments.set(actor,[actor.renderX+x*c-z*s,actor.renderAltitude+y+variant.yOffset*scale,actor.renderY+x*s+z*c]);
    }
    const activity=Math.round(data[i*12+11]),requestedClip=['idle','walk','wood','mine','food','build','combat','walk-alt'][activity]||'idle';
    const packPose=nativePackPose(actor,{sourceId,team,data,index:i,parts:variant.parts});
    const nativeCapture=variant.nativeCapture;
    const capturePose=!packPose&&nativeCapture&&exactActor
     &&(regional||nativeCapture.contexts.some(context=>epoch>=context.from&&epoch<context.until&&context.ids.includes(actor.def)&&context.civs.includes(civ)))
     ? nativeCapturePose(options.capturePoseForActor(actor),nativeCapture.clips,options.captureAnimationDetail) : null;
    const takeoff=variant.nativeTakeoff&&exactActor?nativeTakeoffPose(variant.nativeTakeoff,options.takeoffElapsedForActor?.(actor)):null;
    const powerPoseForActor=variant.priestClips?options.priestPoseForActor:options.spyPoseForActor;
    const power=powerClips&&exactActor?powerPoseForActor?.(actor):null;
    const powerClip=powerClips?.[power?.state];
    const powerAnimation=powerClip&&variant.parts.find(part=>part.animations?.[powerClip])?.animations?.[powerClip];
    const powerDuration=Number.isFinite(power?.duration)&&power.duration>0?power.duration:powerAnimation?.duration;
    const powerPose=powerAnimation&&Number.isFinite(power.elapsed)&&power.elapsed>=0
      ? {clip:powerClip,once:power.once,phase:power.once?Math.min(1,power.elapsed/powerDuration):
          power.elapsed/powerDuration%1}
      : null;
    const hercPose=matchingActor&&variant.hercAttack?options.hercAttack.poseFor(actor):null;
    const huntCitizen=sourceId===8&&matchingActor;
    const huntActor=huntCitizen&&requestedClip==='combat';
    const huntClip=huntActor&&variant.parts.some(part=>part.animations?.huntCombat)?'huntCombat':'combat';
    const huntAnimation=huntActor?variant.parts.find(part=>part.animations?.[huntClip])?.animations[huntClip]:null;
    const huntPhase=huntCitizen&&rectHuntFrame
      ? options.rectHuntVisualForActor?.(actor,huntAnimation?.duration??1.6666667461395264,rectHuntFrame,commitRectHunt&&!shadow&&!capture&&!clipPlane)
      : huntAnimation?options.rectHuntPhaseForActor?.(actor,huntAnimation.duration):NaN;
    const huntPose=huntAnimation&&Number.isFinite(huntPhase)&&huntPhase>=0&&huntPhase<=1?{clip:huntClip,once:true,phase:huntPhase}:null;
    const workEventActor=sourceId===8&&!previewFlags&&actor?.def===sourceId&&actor.team===team
      &&actor.index===i&&actor.renderTime===time&&actor.renderX===data[i*12]
      &&actor.renderY===data[i*12+1]&&actor.renderAltitude===data[i*12+2]
      &&actor.renderYaw===data[i*12+3];
    const sourceWorkRequest=workEventActor&&variant.nativeCitizenWork
      ?options.citizenWorkVisualForActor?.(actor):null;
    // Native text keys have their own cached scene clock. A pose sample or
    // loaded model is not evidence that the actor was actually drawn.
    if(workEventActor&&variant.nativeCitizenWork&&!shadow)
      options.citizenWorkEventForActor?.(actor,variant,sourceWorkRequest,{vp,capture,clipPlane});
    const workRequest=matchingActor&&sourceId===8&&variant.nativeCitizenWork
      &&requestedClip!=='combat'&&!capturePose&&!powerPose&&!huntPose
      ?sourceWorkRequest:null;
    // Keep the native intrinsic clock and the exact GatherAbility .8f span.
    // The caller supplies the authoritative age and explicit portable precision.
    const citizenWorkPose=workRequest&&(workRequest.span===0||workRequest.span===Math.fround(0.8))
      ?citizenWorkHistory.pose(variant,actor.handle,workRequest):null;
    const dedicatedPose=packPose||capturePose||takeoff||powerPose||hercPose||huntPose||citizenWorkPose;
    const huntElapsed=huntActor&&!huntPose?options.rectHuntElapsedForActor?.(actor):undefined;
    const attackElapsed=Number.isFinite(huntElapsed)&&huntElapsed>=0?huntElapsed:
     variant.nativeAttackClock&&requestedClip==='combat'&&actor?.def===variant.sourceId&&actor.team===team
     &&actor.index===i&&actor.renderTime===time&&actor.renderX===data[i*12]&&actor.renderY===data[i*12+1]
     ? options.stoneAttackElapsedForActor?.(actor) : undefined;
    const timedAttack=Number.isFinite(attackElapsed);
    const defaultClip=variant.parts.some(p=>p.animations?.[requestedClip])?requestedClip:'idle';
    const clip=dedicatedPose?.clip||(timedAttack?(attackElapsed>=0?'combat':'idle'):defaultClip);
    const once=dedicatedPose?.once??(timedAttack&&attackElapsed>=0);
    const animation=variant.parts.find(p=>p.animations?.[clip])?.animations?.[clip];
    const duration=animation?.duration||1;
    let phase=dedicatedPose?.phase??(timedAttack&&attackElapsed>=0?Math.min(1,attackElapsed/duration):((time/duration+seed)%1+1)%1);
    let blend=null;
    if(packBlendReady&&matchingActor&&variant.nativePackBlend&&actor.pack&&animation){
     const pose={clip,once,phase,period:packPose?.once?actor.pack.duration:duration,span:duration,packPose};
     if(packScene)({phase,blend}=packScene.pose(actor,variant,pose,capture&&!clipPlane));
     else blend=packBlendHistory.pose(actor,variant,pose);
    }
    const birdPose=variant.nativeBirdRig&&data[i*12+9]===1
     ? birdFlightFrames.pose(variant.nativeBirdRig,clip,time)
     : null;
    const rigPose=birdPose??(packRigReady&&matchingActor&&variant.nativePackRig
     ? packRigFrames.pose(actor,variant.nativePackRig,clip,phase,blend)
     : null);
    if(hercBeamMuzzles&&matchingActor&&variant.hercAttack)options.hercBeam?.capture(hercBeamMuzzles,actor,clip,phase);
    // Parts with authored visibility events batch only with actors in the same visible state.
    const hiddenParts=variant.hasClipVisibility?variant.parts.filter(part=>{
     if(!part.clipVisibility)return false;
     let visible=false;
     for(const [seconds,show] of part.clipVisibility[clip]||[]){
      if(seconds>phase*duration)break;
      visible=show;
     }
     return !visible;
    }):null;
    const visibilityKey=(hiddenParts?.length?':hidden:'+hiddenParts.map(part=>variant.parts.indexOf(part)).join(','):'')
     +(citizenWorkPose?':citizen:'+citizenWorkPose.stateName+':'+citizenWorkPose.visibilityKey:'');
    const shimmerKey=shimmer&&(shimmer.opacity!==1||shimmer.emissive!==0)?':shimmer:'+shimmer.opacity+':'+shimmer.emissive:'';
    const forestBasis=forestBases?.get(i);
    const navalKey=forestBasis?':forest:'+i:navalBasis?':naval:'+actor.handle:'';
    const blendKey=rigPose?':pack-rig:'+rigPose.page:blend?':pack-blend:'+actor.handle:'';
    const isFarm=variant.farmCropFamilies&&matchingActor&&actor.built===1;
    const requestedFamily=isFarm?options.farmFamilyForActor?.(actor):null;
    const farmFamily=Number.isInteger(requestedFamily)&&variant.farmCropFamilies[requestedFamily]?requestedFamily:0;
    const farmStages=isFarm?variant.farmCropFamilies[farmFamily]:null;
    const farmWord=farmStages?options.farmVisualForActor?.(actor):null;
    const selectedCrop=farmStages?farmPartsForWord(farmStages,farmWord):null;
    const cropParts=selectedCrop??(farmFamily>0?farmStages[3].flat():null);
    const seedAges=farmSeedingAges(selectedCrop,actor,farmSeedingElapsedForActor);
    const cropKey=cropParts?`:crop:${farmFamily}:${selectedCrop?farmWord:FARM_DEFAULT_WORD}${seedAges?':'+seedAges.join(','):''}`:'';
    const oilClock=variant.nativeOilRig&&matchingActor&&actor.built===1?oilRigClock(actor,options.oilRigVisualForActor):null;
    const oilKey=oilClock?':oil-rig:'+actor.handle:'';
    const preview=!!previewFlags,key=variantIndex+':'+team+':'+activity+':'+(far&&variant.farRequested?1:0)+':'+previewFlags+':'+clip+':'+once+(variant.houseLodKey===undefined?'':':house:'+variant.houseLodKey)+visibilityKey+shimmerKey+navalKey+blendKey+cropKey+oilKey;
    if(collectStoneThrowerWheels&&!preview&&actor?.def===22&&epoch<2&&variant.stoneThrowerWheelClips?.has(clip)){
     stoneThrowerWheels.push({handle:actor.handle,clip,x:actor.renderX,y:actor.renderY,altitude:actor.renderAltitude});
    }
    if(collectGranaryLoops&&matchingActor&&actor.built===1&&clip==='idle'&&variant.presentationEntry?.granaryAsset&&variant.presentationEntry.idleLoop){
     granaryLoops.push({handle:actor.handle,x:actor.renderX,y:actor.renderY,altitude:actor.renderAltitude});
    }
    const group=groups.get(key)||{variant,team,activity,clip,once,blend,rigPose,preview,hiddenParts,citizenWorkPose,cropParts,seedAges,oilClock,shimmer:shimmerKey?shimmer:null,navalBasis,forestBasis,list:[],instanceIndices:[],workEventActors:[]};
    if(workEventActor&&variant.nativeCitizenWork&&!shadow)group.workEventActors.push(actor);
    group.instanceIndices.push(i);
    const offset=group.list.length;for(let k=0;k<12;k++)group.list.push(data[i*12+k]);
    group.list[offset+3]+=variant.yawOffset;group.list[offset+9]=rigPose?.layer??phase;
    if(variant.nativeScale!==null)group.list[offset+8]=variant.nativeScale;
    groups.set(key,group);
    const scaffoldStage=!shadow&&variant.wonderScaffold&&wonderScaffoldStage(variant.wonderScaffold,data,i*12);
    if(scaffoldStage){
     const at=i*12,yaw=group.list[offset+3],c=Math.cos(yaw),s=Math.sin(yaw);
     scaffoldDraws.push({id:variant.sourceId,layers:scaffoldStage.layers,instances:scaffoldStage.parts.length});
     for(const tile of scaffoldStage.parts){
      const tileKey=`scaffold:${tile.module}:${team}`,module=scaffoldModules[tile.module];
      const tiles=groups.get(tileKey)||{variant:module,team,activity:0,clip:'idle',once:false,preview:false,list:[]};
      tiles.list.push(data[at]+tile.x*c-tile.y*s,data[at+1]+tile.x*s+tile.y*c,data[at+2]+tile.altitude,
        yaw+tile.yaw,id,team,1,Math.round(data[at+7])&~64,module.nativeScale,0,0,0);
      groups.set(tileKey,tiles);
     }
    }
   }
   for(const {variant,team,list,activity,clip,once,blend,rigPose,preview,hiddenParts,citizenWorkPose,cropParts,seedAges,oilClock,shimmer,navalBasis,forestBasis,instanceIndices,workEventActors} of groups.values()){
    let actualBodyDraw=false;
    const modelBasis=forestBasis??navalBasis;
    if(rememberPicks&&variant.assetKind===1&&variant.pickBounds)rememberResources(data,instanceIndices,variant,activity,resourceIdentities,forestBases);
    selectShader(shader);gl.uniform1f(shader.u.uYOffset,variant.yOffset);
    gl.uniform3fv(shader.u.uOriginOffset,variant.originOffset);
    const constructionPlane=variant.wonderConstructionPlane;
    gl.uniform2f(shader.u.uWonderConstructionPlane,constructionPlane?.[0]??0,constructionPlane?.[1]??0);
    gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);
    const dynamicOwners=dynamicGPU&&!preview&&!shadow&&variant.dynamicSailBinding?instanceIndices.map((index,j)=>{
     const actor=actorMetadata?.forInstance?.(index),at=j*12;
     // A sidecar from another render call/stream must never deform this row.
     if(!actor||actor.def!==variant.sourceId||actor.team!==team||actor.renderX!==list[at]||actor.renderY!==list[at+1]||actor.renderAltitude!==list[at+2]||actor.renderYaw!==data[index*12+3])return null;
     return dynamicGPU.get(actor,actorMetadata,variant.dynamicSailBinding.key,navalBasis?preparedSailMetadata:actorMetadata);
    }):null;
    for(const part of cropParts??variant.parts){
     if(hiddenParts?.includes(part))continue;
     if(citizenWorkPose){
      if(!citizenWorkPose.visiblePartNodes.includes(part.sourceNode))continue;
     }else{
      const activities=part.nativeCitizenTool?.legacyActivities??part.activities;
      if(activities&&!activities.includes(activity))continue;
     }
     // Native cloth shadow events/physics are still separate. Existing body
     // shadows remain byte-identical; no synthetic sail shadow is added here.
     if(part.nativeSail&&(shadow||!sailDiffuse))continue;
     const seed=part.farmSeeding;
     const seedPose=seedAges&&seed?farmSeedingPose(seed.family,seed.sourceNode,seedAges[part.farmPlot-1]):null;
     const isolated=part.nativeSail&&part.darkTextures&&dynamicOwners?.some(Boolean),jobs=isolated?dynamicOwners.map((owner,j)=>({owner,record:new Float32Array(list.slice(j*12,j*12+12)),instances:1})):[{owner:null,record:null,instances:list.length/12}];
     for(const job of jobs){
     const deformed=job.owner?.parts[part.sailIndex],gpu=deformed||part;
     if(job.record){gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,job.record,gl.STREAM_DRAW);}
     selectShader(deformed?dynamicSailShader:part.nativeSail?sailShader:shader);const u=activeShader.u;
     gl.uniform1i(u.uGuidedRocket,variant.guidedRocket?1:0);
     gl.uniform1i(u.uNaval,modelBasis?1:0);
     if(modelBasis)gl.uniformMatrix3fv(u.uNavalBasis,false,modelBasis);
     if(part.nativeSail){gl.uniform1f(u.uYOffset,variant.yOffset);gl.uniform3fv(u.uOriginOffset,variant.originOffset);}
     gl.uniform1i(u.uFarmPlotConform,part.farmPlotOffset?1:0);gl.uniform3fv(u.uFarmPlotCenter,part.farmPlotOffset||[0,0,0]);
     gl.uniform1i(u.uFarmSeeding,seedPose?1:0);
     if(seedPose){gl.uniform1f(u.uFarmSeedingWeight,seedPose.weight);gl.uniform3fv(u.uFarmSeedingShift,seedPose.translation);}
     const oilPart=oilClock?variant.nativeOilRig.nodes[part.sourceNode]:undefined;
     const oilPose=oilPart===undefined?null:oilRigPartPose(variant.nativeOilRig.curves,oilPart,oilClock.clip,oilClock.time);
     gl.uniform1i(u.uOilRigRigid,oilPose?1:0);
     if(oilPose){gl.uniform2f(u.uOilRigCosSin,oilPose.cos,oilPose.sin);gl.uniform3fv(u.uOilRigTranslation,oilPose.translation);}
     const rigPart=rigPose&&part.packRigPart&&activeShader===shader?part.packRigPart:null;
     if((packRigFrames||birdFlightFrames)&&activeShader===shader){
      gl.uniform1i(u.uPackRigActive,rigPart?1:0);
      if(rigPart){
       gl.activeTexture(gl.TEXTURE0+8);gl.bindTexture(gl.TEXTURE_2D_ARRAY,rigPose.texture);
       gl.uniform1i(u.uPackRigOffset,rigPart.offset);gl.uniform4fv(u.uPackRigNormalization,rigPose.rig.normalization);
      }
     }
     const pose=rigPart||variant.nativeOilRig?null:part.pose(clip);gl.uniform1i(u.uAnimated,rigPart||pose?1:0);
     const samples=part.animations?.[clip];
     if(packBlendHistory&&activeShader===shader){
      const outgoingSamples=!rigPart&&blend&&part.animations?.[blend.outgoing.clip];
      const outgoingPose=pose&&samples&&outgoingSamples?part.pose(blend.outgoing.clip):null;
      const blendPart=outgoingPose&&outgoingPose.width===pose.width;
      gl.uniform1i(u.uPackBlendActive,blendPart?1:0);
      if(blendPart){
       gl.activeTexture(gl.TEXTURE0+8);gl.bindTexture(gl.TEXTURE_2D_ARRAY,outgoingPose.texture);
       gl.uniform1i(u.uPackOutgoingCount,outgoingPose.count);gl.uniform1i(u.uPackOutgoingOnce,blend.outgoing.once||outgoingSamples.loopIncludesEnd?1:0);
       gl.uniform1f(u.uPackOutgoingPhase,blend.outgoing.phase);gl.uniform1f(u.uPackIncomingPhase,blend.incomingPhase);gl.uniform1f(u.uPackBlendWeight,blend.weight);
      }
     }
     gl.uniform1i(u.uAnimationOnce,samples&&(once||samples.loopIncludesEnd||(variant.priestClips||variant.spyClips)&&samples.endpointIncluded)?1:0);
     gl.activeTexture(gl.TEXTURE5);gl.bindTexture(gl.TEXTURE_2D_ARRAY,pose?.texture||emptyFrames);
     gl.uniform1i(u.uFrameWidth,pose?.width||1);gl.uniform1i(u.uFrameCount,pose?.count||1);
     const seasonal=part.forSeason(team);
     gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,part.nativeSail?(deformed?part.darkTextures[job.owner.damageMask]:part.darkTexture):seasonal.next);gl.uniform1f(u.uModelSeasonBlend,seasonal.blend);
     gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,seasonal.first);gl.bindVertexArray(seedPose?part.farmSeedingVAO:gpu.vao);
     const m=part.material,a=part.alpha;
     const pipeline=nativeModelPipeline(part.pipeline,lighting);gl.uniform1i(u.uNativeShader,pipeline.shader);gl.uniform1f(u.uTextureScale,pipeline.textureScale);
     const shimmerOpacity=shadow?null:shimmer?.opacity,shimmerBlend=shimmerOpacity<1&&shimmerOpacity!=null;
     const shimmerEmissive=shadow?0:(shimmer?.emissive??0);
     gl.uniform4fv(u.uColor,[...(seedPose?.colors.diffuse??m.diffuse),shimmerOpacity??m.alpha]);gl.uniform1f(u.uModelHeight,deformed?deformed.height:part.height);
     gl.uniform3fv(u.uAmbientMaterial,seedPose?.colors.ambient??m.ambient);gl.uniform3fv(u.uSpecularMaterial,m.specular);
     gl.uniform3fv(u.uEmissiveMaterial,shimmer&&!shadow?[shimmerEmissive,shimmerEmissive,shimmerEmissive]:m.emissive);
     gl.uniform1f(u.uSpyEmissive,shimmerEmissive);
     gl.uniform1f(u.uGlossiness,m.glossiness);gl.uniform1i(u.uSpecularEnabled,m.specularEnabled?1:0);
     gl.uniform1i(u.uVertexMode,m.vertexMode);gl.uniform1i(u.uLightingMode,m.lightingMode);
     gl.uniform1i(u.uHasAlpha,(a.test?1:0)|(a.blend||shimmerBlend?2:0));gl.uniform1f(u.uCutoff,a.threshold);gl.uniform1i(u.uAlphaFunction,a.testFunction);
     let sailState;
     if(part.nativeSail){
      sailState={cull:gl.isEnabled(gl.CULL_FACE),equationRGB:gl.getParameter(gl.BLEND_EQUATION_RGB),equationAlpha:gl.getParameter(gl.BLEND_EQUATION_ALPHA)};
      gl.disable(gl.CULL_FACE);gl.enable(gl.DEPTH_TEST);gl.depthMask(true);gl.depthFunc(gl.LEQUAL);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.blendEquation(gl.FUNC_ADD);
     }else applyDrawState(gl,part,shadow,preview,shimmerBlend);
     gl.drawElementsInstanced(gl.TRIANGLES,gpu.count,gl.UNSIGNED_INT,0,job.instances);
     if(gpu.count>0&&job.instances>0)actualBodyDraw=true;
     if(variant.guidedRocket)guidedRocketDraws+=job.instances;
     if(sailState){if(sailState.cull)gl.enable(gl.CULL_FACE);else gl.disable(gl.CULL_FACE);gl.blendEquationSeparate(sailState.equationRGB,sailState.equationAlpha);}
     calls++;triangles+=gpu.count/3*job.instances;
     }
     // Shared instance attributes are restored for later rigid/other parts.
     if(isolated){gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);}
    }
    if(actualBodyDraw&&workEventActors?.length)for(const actor of workEventActors)
      options.citizenWorkAfterDraw?.(actor,variant,{vp,capture,clipPlane});
   }
  }
  selectShader(shader);restoreDrawState(gl,previousState);
  if(querying)gl.endQuery(timer.TIME_ELAPSED_EXT);
  if(capture)return;
  if(shadow)result.metrics.shadowMs=performance.now()-started;
  else{
   result.metrics.colorMs=performance.now()-started;result.metrics.calls=calls;result.metrics.triangles=triangles;
   result.metrics.wonderScaffolds=scaffoldDraws;result.metrics.wonderFences=fenceDraws;
   result.metrics.guidedRocketDraws=guidedRocketDraws;
  }
 };
 return result;
}
