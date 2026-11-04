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
uniform vec3 uTeam[8];uniform float uPx,uSeasonBlend;uniform vec2 uSeasonMonths;
uniform bool uClipEnabled;uniform vec4 uClipPlane;in vec3 vP;in vec3 vN;out vec4 outColor;
${COMMON}
vec3 seasonSample(highp sampler2DArray source,vec2 uv){
 return mix(texture(source,vec3(uv,uSeasonMonths.x)).rgb,texture(source,vec3(uv,uSeasonMonths.y)).rgb,uSeasonBlend);
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
 vec3 sun=normalize(vec3(.42,.82,.39));
 color*=vec3(.55,.57,.59)+vec3(.53,.50,.43)*max(dot(n,sun),0.);
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
async function imageTexture(gl,url,repeat=true){
 const response=await fetch(absolute(url));if(!response.ok)throw new Error(`${url}: ${response.status}`);
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

export async function loadOriginalTerrain(gl,enabled,teams){
 const result={enabled:false,errors:[],metrics:{terrainCalls:0,waterCalls:0,seasonSlot:6,seasonNext:7,seasonBlend:0,images:0,classification:'none'},
  policy:'native-images-own-map-material-adapter',updateMap(){},drawTerrain(){},drawWater(){},setSeasonPhase(){return false;},async setSeason(){return false;}};
 if(!enabled)return result;
 try{
  const response=await fetch(absolute('./local-original/terrain-native/manifest.json'),{cache:'no-store'});
  if(!response.ok)throw new Error(`Native terrain manifest: ${response.status}`);
  const manifest=await response.json();result.manifest=manifest;
  if(manifest.format!=='ee2-native-terrain-v1')throw new Error('Unsupported native terrain manifest');
  const terrain=makeProgram(gl,TERRAIN_SHADERS),water=makeProgram(gl,WATER_SHADERS),cache=new Map();
  const texture=(url,repeat=true)=>{if(!cache.has(url))cache.set(url,imageTexture(gl,url,repeat));return cache.get(url);};
  const mask=await texture(manifest.maskUrl),normal=await texture(manifest.water.normal),layer1=await texture(manifest.water.layer1),layer2=await texture(manifest.water.layer2),colorLut=await texture(manifest.water.colorLut,false);
  const images=new Map();
  const bitmap=url=>{
   if(!images.has(url))images.set(url,(async()=>{const response=await fetch(absolute(url));if(!response.ok)throw new Error(`${url}: ${response.status}`);return createImageBitmap(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});})());
   return images.get(url);
  };
  const roles={};let lastFields=null,season={month0:6,month1:7,blend:0};
  // All twelve native monthly images are resident, so phase changes cannot
  // depend on asynchronous fetch order, rendering speed or a wall clock.
  for(const role of ROLES){
   const definition=manifest.textureDefinitions[manifest.runtimeRoles[role].textureDefinition];
   const urls=manifest.seasonalSets[definition.seasonalSet].urls;
   const frames=await Promise.all(urls.map(bitmap)),width=frames[0].width,height=frames[0].height;
   if(frames.length!==12||frames.some(frame=>frame.width!==width||frame.height!==height))throw new Error('Native monthly texture sizes must match without rescaling');
   const array=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D_ARRAY,array);
   gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1+Math.floor(Math.log2(Math.max(width,height))),gl.RGBA8,width,height,12);
   for(let month=0;month<12;month++)gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,month,width,height,1,gl.RGBA,gl.UNSIGNED_BYTE,frames[month]);
   gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
   gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_S,gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_T,gl.REPEAT);gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
   roles[role]=array;
  }
  for(const frame of await Promise.all(images.values()))frame.close();
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
  result.updateMap=({heights,types,size=MAP})=>{
   if(size!==MAP)throw new RangeError('Native renderer currently requires the 256x256 simulation map');
   if(lastFields&&sameArray(lastFields.heights,heights)&&sameArray(lastFields.types,types))return false;
   const fields=buildTerrainFields({heights,types,size});
   bind(UNITS.weights,weightsTexture);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,MAP,MAP,0,gl.RGBA,gl.UNSIGNED_BYTE,fields.weights);
   bind(UNITS.heights,heightTexture);gl.texImage2D(gl.TEXTURE_2D,0,gl.R32F,MAP+1,MAP+1,0,gl.RED,gl.FLOAT,fields.heights);
   // The shader interpolates four exact corners, requiring no float-linear extension.
   gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
   lastFields=fields;result.metrics.classification=fields.classification;gl.activeTexture(gl.TEXTURE0);return true;
  };
  result.drawTerrain=({vao,indexCount=MAP*MAP*6,vp,px=1/28,fogTexture,territoryTexture,clipPlane=null,capture=false})=>{
   if(!lastFields)return;
   if(clipPlane&&(!Array.isArray(clipPlane)&&!ArrayBuffer.isView(clipPlane)||clipPlane.length!==4||!Array.from(clipPlane).every(Number.isFinite)))throw new RangeError('Invalid terrain clip plane');
   gl.useProgram(terrain.program);gl.bindVertexArray(vao);gl.uniformMatrix4fv(terrain.uniform('uVP'),false,vp);gl.uniform1f(terrain.uniform('uPx'),px);gl.uniform3fv(terrain.uniform('uTeam'),new Float32Array(teams.flat()));
   gl.uniform1i(terrain.uniform('uClipEnabled'),clipPlane?1:0);gl.uniform4fv(terrain.uniform('uClipPlane'),clipPlane||[0,0,0,0]);
   for(const role of ROLES){bind(UNITS[role],roles[role],gl.TEXTURE_2D_ARRAY);gl.uniform1i(terrain.uniform('u'+role[0].toUpperCase()+role.slice(1)),UNITS[role]);}
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
  result.enabled=true;
 }catch(error){result.errors.push(error.message);console.warn('Original terrain:',error);}
 return result;
}
