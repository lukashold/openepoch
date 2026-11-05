import {buildNativeTerrainLightmap,validateTerrainLighting,nativeTerrainCoefficients,LIGHTMAP_LAYOUT} from './native-terrain-lightmap.js';
// Native terrain images and source bindings on OpenEpoch's own generated map.
// UV scale, class blending and water motion are explicit rendering adapters.
const MAP=256;
const ROLES=['grass','patch','forest','rock','beach','coral','rocky','roots'];
const UNITS={grass:0,patch:4,forest:5,rock:6,beach:7,coral:8,rocky:9,roots:10,mask:11,weights:12,heights:13};
const COMMON=`
float fogAt(vec2 p){return texture(uFog,clamp(p/256.,0.,1.)).r;}
vec3 fogged(vec3 c,float f){return c*smoothstep(.02,.45,f)*mix(.5,1.,smoothstep(.55,.95,f));}
`;
export const TERRAIN_SHADERS={vertex:`#version 300 es
layout(location=0) in vec3 aP;layout(location=1) in vec3 aN;
uniform mat4 uVP;out vec3 vP;out vec3 vN;
void main(){vP=aP;vN=aN;gl_Position=uVP*vec4(aP,1.);}
`,fragment:`#version 300 es
precision highp float;
uniform highp sampler2DArray uGrass,uPatch,uForest,uRock,uBeach,uCoral,uRocky,uRoots;
uniform sampler2D uMask,uWeights,uFog,uTerr;
uniform highp sampler2DArray uNativeLightmap;uniform bool uNativeLighting;uniform float uNativeBlocks;uniform vec3 uNativeDiffuse,uNativeAmbient;
uniform vec3 uTeam[8];uniform float uPx,uSeasonBlend;uniform vec2 uSeasonMonths;
uniform bool uClipEnabled;uniform vec4 uClipPlane;in vec3 vP;in vec3 vN;out vec4 outColor;
${COMMON}
vec3 seasonSample(highp sampler2DArray source,vec2 uv){
 vec2 layers=min(uSeasonMonths,vec2(float(textureSize(source,0).z-1)));
 return mix(texture(source,vec3(uv,layers.x)).rgb,texture(source,vec3(uv,layers.y)).rgb,uSeasonBlend);
}
void main(){
 if(uClipEnabled&&dot(vec4(vP,1.),uClipPlane)<0.)discard;
 vec2 w=vP.xz,uv=w/256.;float visibility=fogAt(w);
 if(visibility<.015){outColor=vec4(0.,0.,0.,1.);return;}
 vec4 weights=texture(uWeights,uv);
 // Terr_Mask RGB is black in the source. Its original alpha holds the edge.
 float mask=texture(uMask,w/5.).a;
 weights=max(vec4(0.),weights+(mask-.5)*.24*weights*(1.-weights)*4.);
 weights/=max(dot(weights,vec4(1.)),.0001);
 vec2 tiled=w/4.;
 float patchMask=texture(uMask,w/13.+vec2(.37,.19)).a;
 vec3 grass=mix(seasonSample(uGrass,tiled),seasonSample(uPatch,tiled+vec2(.3,.17)),patchMask*.32);
 vec3 forest=mix(seasonSample(uForest,tiled),seasonSample(uRoots,tiled),patchMask*.55);
 vec3 n=normalize(vN);
 vec3 rock=mix(seasonSample(uRock,tiled),seasonSample(uRocky,tiled),smoothstep(.45,.94,n.y));
 vec3 color=grass*weights.r+forest*weights.g+rock*weights.b+seasonSample(uBeach,tiled)*weights.a;
 color=mix(color,seasonSample(uCoral,tiled),smoothstep(.08,.7,-vP.y));
 if(uNativeLighting){
  vec2 block=floor(clamp(vP.xz,vec2(0.),vec2(255.999))/25.);
  vec2 mapUV=(vP.xz-block*25.)*((1.-1./128.)/25.)+vec2(.5/128.);
  float lightmap=texture(uNativeLightmap,vec3(mapUV,block.x+block.y*uNativeBlocks)).r;
  vec3 stage0=clamp(2.*lightmap*uNativeDiffuse,0.,1.);
  vec3 stage1=clamp(stage0+uNativeAmbient,0.,1.);
  color=clamp(2.*color*stage1,0.,1.);
 }else{
  vec3 sun=normalize(vec3(.42,.82,.39));
  color*=vec3(.55,.57,.59)+vec3(.53,.50,.43)*max(dot(n,sun),0.);
 }
 color*=mix(1.,.65,clamp(-vP.y*.5,0.,1.));
 vec4 t=texture(uTerr,uv);float d=max(.09,uPx*1.3)/256.;float edge=0.;
 edge=max(edge,step(.001,abs(texture(uTerr,uv+vec2(d,0.)).r-t.r)));
 edge=max(edge,step(.001,abs(texture(uTerr,uv-vec2(d,0.)).r-t.r)));
 edge=max(edge,step(.001,abs(texture(uTerr,uv+vec2(0.,d)).r-t.r)));
 edge=max(edge,step(.001,abs(texture(uTerr,uv-vec2(0.,d)).r-t.r)));
 int owner=int(t.g*255.+.5);vec3 border=owner>0&&owner<=8?uTeam[owner-1]:vec3(.9);
 color=mix(color,border,owner>0?.04+edge*.7:edge*.2);
 outColor=vec4(fogged(color,visibility),1.);
}`};

export const WATER_SHADERS={vertex:`#version 300 es
layout(location=0) in vec2 aP;uniform mat4 uVP;out vec2 vW;
void main(){vW=aP;gl_Position=uVP*vec4(aP.x,.012,aP.y,1.);}
`,fragment:`#version 300 es
precision highp float;
uniform sampler2D uNormal,uLayer1,uLayer2,uColorLut,uHeights,uFog,uReflection;
uniform bool uHasReflection;uniform mat4 uReflectionVP;
uniform float uTime;uniform vec3 uEye;in vec2 vW;out vec4 outColor;
${COMMON}
float heightAt(vec2 w){
 vec2 p=clamp(w,vec2(0.),vec2(255.9999));ivec2 i=ivec2(floor(p));vec2 f=fract(p);
 float a=texelFetch(uHeights,i,0).r,b=texelFetch(uHeights,i+ivec2(1,0),0).r;
 float c=texelFetch(uHeights,i+ivec2(0,1),0).r,d=texelFetch(uHeights,i+ivec2(1,1),0).r;
 return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
void main(){
 vec2 uv=vW/256.;
 if(any(lessThan(uv,vec2(0.)))||any(greaterThan(uv,vec2(1.)))){outColor=vec4(.025,.075,.12,1.);return;}
 // The 257x257 corner height field is independent of minimap coloration.
 float floorHeight=heightAt(vW);
 if(floorHeight>=.006)discard;
 float visibility=fogAt(vW);if(visibility<.015)discard;
 float depth=max(0.,-floorHeight);
 // Exact flow constants from WaterReflection_LOD_HIGH.NSF and Water_LOD_HIGH.NSF.
 // uTime is this adapter's seconds; native CM_Defined time phase is still separate.
 vec2 bumpUV=vW/8.;
 vec2 bump0=bumpUV+fract(uTime*vec2(.01125,.02));
 vec2 bump1=(bumpUV+fract(uTime*vec2(-.01125,.01))).yx;
 vec3 bump=(texture(uNormal,bump0).rgb+texture(uNormal,bump1).rgb)-1.;
 vec3 n=normalize(vec3(bump.x*.84,1.,bump.y*.84));
 vec3 layer1=texture(uLayer1,vW/12.+uTime*vec2(.0155,.02)).rgb;
 vec3 layer2=texture(uLayer2,vW/17.+uTime*vec2(-.015,.01)).rgb;
 vec3 base=mix(layer1,layer2,.42);
 vec3 ocean=texelFetch(uColorLut,ivec2(0,0),0).rgb;
 vec3 coast=texelFetch(uColorLut,ivec2(2,0),0).rgb;
 base*=mix(coast,ocean,smoothstep(.05,.9,depth));
 vec3 eye=normalize(uEye-vec3(vW.x,0.,vW.y));
 float fresnel=.05+.38*pow(1.-max(dot(n,eye),0.),5.);
 vec3 reflectionColor=vec3(.52,.69,.82);
 if(uHasReflection){
  vec4 projected=uReflectionVP*vec4(vW.x,0.,vW.y,1.);
  vec2 reflectionUV=projected.xy/max(projected.w,.0001)*.5+.5;
  // Native PS averages the two samples, maps to [-1,1], then offsets by0.1.
  if(projected.w>0.)reflectionColor=texture(uReflection,clamp(reflectionUV+bump.xy*.1,vec2(.001),vec2(.999))).rgb;
 }
 base=mix(base,reflectionColor,fresnel);
 vec3 halfVector=normalize(normalize(vec3(.42,.82,.39))+eye);
 float reflection=pow(max(dot(n,halfVector),0.),96.);
 base+=vec3(.95,.96,.91)*reflection*.55;
 float alpha=smoothstep(0.,.12,depth)*mix(.48,.88,smoothstep(.05,.9,depth));
 outColor=vec4(fogged(base,visibility)*alpha,alpha);
}`};

/** Actual own-map class IDs -> normalized material weights, never RGB guesses. */
export function buildTerrainFields({heights,types,size=MAP}){
 if(!Number.isInteger(size)||size<1||size>1024)throw new RangeError('Invalid terrain size');
 if(!heights||heights.length!==(size+1)**2)throw new RangeError('Terrain heights must contain (size+1)^2 corners');
 for(const h of heights)if(!Number.isFinite(h))throw new RangeError('Non-finite terrain height');
 let classes=types;
 if(classes&&classes.length!==size*size)throw new RangeError('Terrain types must contain size^2 tile IDs');
 if(!classes){
  classes=new Uint8Array(size*size);
  for(let y=0;y<size;y++)for(let x=0;x<size;x++){
   const i=y*(size+1)+x,h=(heights[i]+heights[i+1]+heights[i+size+1]+heights[i+size+2])*.25;
   classes[y*size+x]=h<-.25?0:h<0?1:h<.12?2:h>1.5?5:3;
  }
 }
 for(const id of classes)if(!Number.isInteger(id)||id<0||id>5)throw new RangeError('Unknown terrain class ID');
 const channels=[3,3,3,0,1,2],kernel=[1,4,6,4,1];
 const horizontal=new Float32Array(size*size*4),weights=new Uint8Array(horizontal.length);
 for(let y=0;y<size;y++)for(let x=0;x<size;x++)for(let k=-2;k<=2;k++){
  const sample=classes[y*size+Math.min(size-1,Math.max(0,x+k))],offset=(y*size+x)*4;
  horizontal[offset+channels[sample]]+=kernel[k+2]/16;
 }
 for(let y=0;y<size;y++)for(let x=0;x<size;x++){
  const totals=[0,0,0,0];
  for(let k=-2;k<=2;k++)for(let c=0;c<4;c++)totals[c]+=horizontal[(Math.min(size-1,Math.max(0,y+k))*size+x)*4+c]*kernel[k+2]/16;
  for(let c=0;c<4;c++)weights[(y*size+x)*4+c]=Math.round(totals[c]*255);
 }
 return {weights,heights:new Float32Array(heights),types:new Uint8Array(classes),classification:types?'simulation-terrain-ids':'height-adapter'};
}

function makeProgram(gl,sources){
 const program=gl.createProgram();
 for(const [type,text] of [[gl.VERTEX_SHADER,sources.vertex],[gl.FRAGMENT_SHADER,sources.fragment]]){
  const shader=gl.createShader(type);gl.shaderSource(shader,text);gl.compileShader(shader);
  if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(shader));
  gl.attachShader(program,shader);gl.deleteShader(shader);
 }
 gl.linkProgram(program);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(program));
 return {program,uniform:name=>gl.getUniformLocation(program,name)};
}
const absolute=url=>new URL(url,import.meta.url);
async function imageTexture(gl,url,repeat=true,resolve=absolute){
 const response=await fetch(resolve(url));if(!response.ok)throw new Error(`${url}: ${response.status}`);
 const image=await createImageBitmap(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
 const texture=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
 gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,repeat?gl.REPEAT:gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,repeat?gl.REPEAT:gl.CLAMP_TO_EDGE);
 const anisotropy=gl.getExtension('EXT_texture_filter_anisotropic');
 if(anisotropy)gl.texParameterf(gl.TEXTURE_2D,anisotropy.TEXTURE_MAX_ANISOTROPY_EXT,Math.min(8,gl.getParameter(anisotropy.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
 gl.generateMipmap(gl.TEXTURE_2D);image.close();return texture;
}
const sameArray=(a,b)=>a&&b&&a.length===b.length&&a.every((value,i)=>value===b[i]);

export async function loadOriginalTerrain(gl,enabled,teams,options={}){
 const result={enabled:false,errors:[],metrics:{terrainCalls:0,waterCalls:0,seasonSlot:6,seasonNext:7,seasonBlend:0,images:0,classification:'none',lightmapBuilds:0,lightmapUploads:0,lightmapBytes:0,lightmapBuildMs:0,nativeLighting:false,lightingFallback:'missing-context'},
  policy:'native-images-own-map-material-adapter',seasonClimates:['Temperate'],climateErrors:[],updateMap(){},drawTerrain(){},drawWater(){},setLightingContext(){return false;},setSeasonClimate(){return false;},setSeasonPhase(){return false;},async setSeason(){return false;}};
 if(!enabled)return result;
 try{
  const resolveAsset=url=>new URL(url,options.assetBaseURL||import.meta.url);
  const response=await fetch(resolveAsset('./local-original/terrain-native/manifest.json'),{cache:'no-store'});
  if(!response.ok)throw new Error(`Native terrain manifest: ${response.status}`);
  const manifest=await response.json();result.manifest=manifest;
  if(manifest.format!=='ee2-native-terrain-v1')throw new Error('Unsupported native terrain manifest');
  const terrain=makeProgram(gl,TERRAIN_SHADERS),water=makeProgram(gl,WATER_SHADERS),cache=new Map();
  const texture=(url,repeat=true)=>{if(!cache.has(url))cache.set(url,imageTexture(gl,url,repeat,resolveAsset));return cache.get(url);};
  const mask=await texture(manifest.maskUrl),normal=await texture(manifest.water.normal),layer1=await texture(manifest.water.layer1),layer2=await texture(manifest.water.layer2),colorLut=await texture(manifest.water.colorLut,false);
  const images=new Map();
  const bitmap=url=>{
   if(!images.has(url))images.set(url,(async()=>{const response=await fetch(resolveAsset(url));if(!response.ok)throw new Error(`${url}: ${response.status}`);return createImageBitmap(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});})());
   return images.get(url);
  };
  let lastFields=null,season={month0:6,month1:7,blend:0};
  const roleSets=new Map(), arrays=new Map();
  const uploadMonthly=async urls=>{
   if(!Array.isArray(urls)||![1,12].includes(urls.length))throw new Error('Invalid native terrain season list');
   const frames=await Promise.all(urls.map(bitmap)),width=frames[0].width,height=frames[0].height;
   if(frames.some(frame=>frame.width!==width||frame.height!==height))throw new Error('Native monthly texture sizes must match without rescaling');
   const array=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D_ARRAY,array);
   gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1+Math.floor(Math.log2(Math.max(width,height))),gl.RGBA8,width,height,frames.length);
   for(let month=0;month<frames.length;month++)gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,month,width,height,1,gl.RGBA,gl.UNSIGNED_BYTE,frames[month]);
   gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
   gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_S,gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_T,gl.REPEAT);gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
   return array;
  };
  async function loadRoles(climate, profile) {
   const textures={};
   for(const role of ROLES){
    const definition=manifest.textureDefinitions[manifest.runtimeRoles[role].textureDefinition];
    let urls=profile ? profile[role].urls : manifest.seasonalSets[definition.seasonalSet].urls;
    if(profile&&urls.every(url=>url===urls[0]))urls=urls.slice(0,1);
    const key=JSON.stringify(urls);
    if(!arrays.has(key))arrays.set(key,uploadMonthly(urls));
    textures[role]=await arrays.get(key);
   }
   roleSets.set(climate,textures);
  }
  await loadRoles('Temperate');
  for(const [name, profile] of Object.entries(options.climateProfiles||{})){
   if(name==='Temperate')continue;
   try{
    await loadRoles(name,profile.terrain);
    result.seasonClimates.push(name);
   }catch(error){result.climateErrors.push(name+': '+error.message);}
  }
  let roles=roleSets.get('Temperate');
  result.setSeasonClimate=name=>{
   if(!roleSets.has(name))return false;
   roles=roleSets.get(name);
   return true;
  };
  for(const frame of await Promise.allSettled(images.values()))if(frame.status==='fulfilled')frame.value.close();
  result.metrics.images=cache.size+images.size;
  const dataTexture=()=>{const t=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,t);for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.LINEAR);for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,key,gl.CLAMP_TO_EDGE);return t;};
  const weightsTexture=dataTexture(),heightTexture=dataTexture();
  result.setSeasonPhase=({month0,month1=(month0+1)%12,blend=0})=>{
   if(!Number.isInteger(month0)||month0<0||month0>11||month1!==(month0+1)%12||!Number.isFinite(blend)||blend<0||blend>1)throw new RangeError('Invalid adjacent-month season phase');
   season={month0,month1,blend};result.metrics.seasonSlot=month0;result.metrics.seasonNext=month1;result.metrics.seasonBlend=blend;return true;
  };
  result.setSeason=async slot=>result.setSeasonPhase({month0:slot});
  await result.setSeason(manifest.adapter.initialSeasonSlot);
  const bind=(unit,t,target=gl.TEXTURE_2D)=>{gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,t);};
  let lighting=null,lightmap=null,atlasTexture=null,bakedHeights=null,bakedDirection=null,bakedNormals=null,mapNormals=null;
  const clock=()=>globalThis.performance?.now?.()??Date.now();
  const arrayTexture=(width,height,layers,data)=>{
   const t=gl.createTexture();bind(LIGHTMAP_LAYOUT.textureUnit,t,gl.TEXTURE_2D_ARRAY);
   gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.R8,width,height,layers);
   gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,0,width,height,layers,gl.RED,gl.UNSIGNED_BYTE,data);
   for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D_ARRAY,key,gl.LINEAR);
   for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D_ARRAY,key,gl.CLAMP_TO_EDGE);
   return t;
  };
  const fallbackTexture=arrayTexture(1,1,1,new Uint8Array([0]));
  const bake=()=>{
   if(!lighting||!lastFields)return false;
   if(lightmap&&sameArray(bakedHeights,lastFields.heights)&&sameArray(bakedDirection,lighting.direction)&&((!mapNormals&&!bakedNormals)||sameArray(mapNormals,bakedNormals)))return false;
   const started=clock();
   try{
    const next=buildNativeTerrainLightmap({heights:lastFields.heights,size:MAP,normals:mapNormals,direction:lighting.direction});
    const nextTexture=arrayTexture(next.resolution,next.resolution,next.layers,next.data);
    if(atlasTexture)gl.deleteTexture(atlasTexture);atlasTexture=nextTexture;lightmap=next;
    bakedHeights=new Float32Array(lastFields.heights);bakedDirection=[...lighting.direction];bakedNormals=mapNormals&&new Float32Array(mapNormals);
    Object.assign(result.metrics,{lightmapBuilds:result.metrics.lightmapBuilds+1,lightmapUploads:result.metrics.lightmapUploads+1,lightmapBytes:next.data.length,lightmapLayers:next.layers,lightmapBuildMs:clock()-started,lightmapOccludedSamples:next.occludedSamples,nativeLighting:true,lightingFallback:null});
    return true;
   }catch(error){
    lightmap=null;result.metrics.nativeLighting=false;result.metrics.lightingFallback='lightmap-build-failed';result.errors.push('Native terrain lightmap: '+error.message);return false;
   }finally{gl.activeTexture(gl.TEXTURE0);}
  };
  result.setLightingContext=context=>{
   if(context===null||context===undefined){lighting=null;result.lighting=null;result.metrics.nativeLighting=false;result.metrics.lightingFallback='missing-context';return false;}
   try{lighting=validateTerrainLighting(context);}catch(error){lighting=null;result.lighting=null;result.metrics.nativeLighting=false;result.metrics.lightingFallback='invalid-context';result.errors.push('Native terrain lighting: '+error.message);return false;}
   result.lighting=lighting;if(!lastFields)result.metrics.lightingFallback='awaiting-map';bake();
   if(lightmap){result.metrics.nativeLighting=true;result.metrics.lightingFallback=null;}
   return true;
  };
  result.updateMap=({heights,types,size=MAP,normals=null})=>{
   if(size!==MAP)throw new RangeError('Native renderer currently requires the 256x256 simulation map');
   if(normals&&(!ArrayBuffer.isView(normals)||normals.length!==(MAP+1)**2*3||!Array.from(normals).every(Number.isFinite)))throw new RangeError('Invalid terrain lightmap normals');
   if(lastFields&&sameArray(lastFields.heights,heights)&&sameArray(lastFields.types,types)&&((!normals&&!mapNormals)||sameArray(normals,mapNormals)))return false;
   const fields=buildTerrainFields({heights,types,size});
   bind(UNITS.weights,weightsTexture);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,MAP,MAP,0,gl.RGBA,gl.UNSIGNED_BYTE,fields.weights);
   bind(UNITS.heights,heightTexture);gl.texImage2D(gl.TEXTURE_2D,0,gl.R32F,MAP+1,MAP+1,0,gl.RED,gl.FLOAT,fields.heights);
   // The shader interpolates four exact corners, requiring no float-linear extension.
   gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
   lastFields=fields;mapNormals=normals&&new Float32Array(normals);result.metrics.classification=fields.classification;bake();gl.activeTexture(gl.TEXTURE0);return true;
  };
  result.drawTerrain=({vao,indexCount=MAP*MAP*6,vp,px=1/28,fogTexture,territoryTexture,clipPlane=null,capture=false})=>{
   if(!lastFields)return;
   if(clipPlane&&(!Array.isArray(clipPlane)&&!ArrayBuffer.isView(clipPlane)||clipPlane.length!==4||!Array.from(clipPlane).every(Number.isFinite)))throw new RangeError('Invalid terrain clip plane');
   gl.useProgram(terrain.program);gl.bindVertexArray(vao);gl.uniformMatrix4fv(terrain.uniform('uVP'),false,vp);gl.uniform1f(terrain.uniform('uPx'),px);gl.uniform3fv(terrain.uniform('uTeam'),new Float32Array(teams.flat()));
   gl.uniform1i(terrain.uniform('uClipEnabled'),clipPlane?1:0);gl.uniform4fv(terrain.uniform('uClipPlane'),clipPlane||[0,0,0,0]);
   for(const role of ROLES){bind(UNITS[role],roles[role],gl.TEXTURE_2D_ARRAY);gl.uniform1i(terrain.uniform('u'+role[0].toUpperCase()+role.slice(1)),UNITS[role]);}
   const nativeEnabled=!!(lighting&&lightmap);gl.uniform1i(terrain.uniform('uNativeLighting'),nativeEnabled?1:0);
   bind(LIGHTMAP_LAYOUT.textureUnit,nativeEnabled?atlasTexture:fallbackTexture,gl.TEXTURE_2D_ARRAY);gl.uniform1i(terrain.uniform('uNativeLightmap'),LIGHTMAP_LAYOUT.textureUnit);
   gl.uniform1f(terrain.uniform('uNativeBlocks'),nativeEnabled?lightmap.blocks:1);
   const coefficients=lighting&&nativeTerrainCoefficients(lighting);
   gl.uniform3fv(terrain.uniform('uNativeDiffuse'),coefficients?coefficients.diffuseBytes.map(v=>v/255):[0,0,0]);
   gl.uniform3fv(terrain.uniform('uNativeAmbient'),coefficients?coefficients.ambientBytes.map(v=>v/255):[0,0,0]);
   gl.uniform2fv(terrain.uniform('uSeasonMonths'),[season.month0,season.month1]);gl.uniform1f(terrain.uniform('uSeasonBlend'),season.blend);
   bind(UNITS.mask,mask);bind(UNITS.weights,weightsTexture);gl.uniform1i(terrain.uniform('uMask'),UNITS.mask);gl.uniform1i(terrain.uniform('uWeights'),UNITS.weights);
   if(fogTexture)bind(2,fogTexture);if(territoryTexture)bind(3,territoryTexture);gl.uniform1i(terrain.uniform('uFog'),2);gl.uniform1i(terrain.uniform('uTerr'),3);
   gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_INT,0);if(!capture)result.metrics.terrainCalls++;gl.activeTexture(gl.TEXTURE0);
  };
  result.drawWater=({vao,vp,time=0,eye=[128,30,160],fogTexture,reflectionTexture,reflectionVP=vp})=>{
   if(!lastFields)return;
   gl.useProgram(water.program);gl.bindVertexArray(vao);gl.uniformMatrix4fv(water.uniform('uVP'),false,vp);gl.uniform1f(water.uniform('uTime'),time);gl.uniform3fv(water.uniform('uEye'),eye);
   for(const [name,unit,t] of [['uNormal',0,normal],['uLayer1',4,layer1],['uLayer2',5,layer2],['uColorLut',6,colorLut],['uHeights',UNITS.heights,heightTexture]]){bind(unit,t);gl.uniform1i(water.uniform(name),unit);}
   bind(7,reflectionTexture||layer1);gl.uniform1i(water.uniform('uReflection'),7);gl.uniform1i(water.uniform('uHasReflection'),reflectionTexture?1:0);gl.uniformMatrix4fv(water.uniform('uReflectionVP'),false,reflectionVP);
   if(fogTexture)bind(2,fogTexture);gl.uniform1i(water.uniform('uFog'),2);
   gl.drawArrays(gl.TRIANGLE_STRIP,0,4);result.metrics.waterCalls++;gl.activeTexture(gl.TEXTURE0);
  };
  result.enabled=true;result.policy='native-lightmap-operations-own-heightfield-input';
  if(options.lighting)result.setLightingContext(options.lighting);
 }catch(error){result.errors.push(error.message);console.warn('Original terrain:',error);}
 return result;
}
