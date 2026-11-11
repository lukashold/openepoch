import {packedTerrainPasses,sourceMaskShader} from './pangea-source-mask.js?v=native80-blend111';
import {buildNativeTerrainLightmap,nativeTerrainCoefficients} from './native-terrain-lightmap.js?v=pangea-light113';
import {createNativeLightingContext} from './original-lighting.js?v=lighting6';

import {createPostWaterFogDraw,postWaterFogFragment} from './pangea-postwater-fog.js?v=postwater121';

const STRIDE=257;
const PALETTE=32;
const TROPICAL={
  climate:'Tropical',size:160,materials:19,textures:18,format:'native79-private-pangea-art110-v1',
  pin:'3b9b8e54b05b26497202505a1098c93f6bf7f43f292f7975cdb2f45a56b12cf0',
  folder:new URL('./local-original/pangea-art110/',import.meta.url),
};
const TEMPERATE={
  climate:'Temperate',size:250,materials:22,textures:154,format:'native82-private-pangea-temperate115-v1',
  pin:'bc468c27017bf1a1aae8fcbdd31013a858047d5916638dbe6341878fdac6c28c',
  folder:new URL('./local-original/pangea-temperate115/',import.meta.url),
};
const BLEND_SHA='d97c2f1fd45bced701d475fc1210fe94b34c730678af4c5ac4a0011cd6488a09';
const LIGHTING_SHA='1b38767bd3ad11c2d45b816235ea009dae274c9e92df1e155599b8501f5fc872';
const PATH=/^textures\/[a-z0-9_]+\.png$/;
const HASH=/^[0-9a-f]{64}$/;
const vertex=`#version 300 es
layout(location=0) in vec3 aP;layout(location=1) in vec3 aN;
uniform mat4 uVP;out vec3 vP;out vec3 vN;
void main(){vP=aP;vN=aN;gl_Position=uVP*vec4(aP,1.0);}`;
const fragment=`#version 300 es
precision highp float;precision highp int;
uniform highp sampler2DArray uMaterials;
uniform sampler2D uCorners,uFog,uTerritory,uMapFog;
uniform ivec2 uSlots[32];uniform float uPhase,uPx,uMapSide;
uniform vec3 uTeam[8];uniform bool uClipEnabled;uniform vec4 uClip;
uniform bool uMapFogEnabled,uDeferMapFog;uniform float uMapFogSide;
uniform highp sampler2DArray uNativeLightmap;
uniform bool uNativeLighting;uniform float uNativeBlocks;uniform vec3 uNativeDiffuse,uNativeAmbient;
in vec3 vP;in vec3 vN;out vec4 outColor;
${sourceMaskShader}
void main(){
 vec2 p=vP.xz,worldUV=p/256.0;
 vec2 dx=dFdx(p)*.4,dz=dFdy(p)*.4;
 if(uClipEnabled&&dot(vec4(vP,1.0),uClip)<0.0)discard;
 // The water base writes depth. Submerged terrain keeps its existing mask
 // when the late terrain draw would be occluded by that surface.
 float fog=uMapFogEnabled?1.-texture(uMapFog,p/uMapFogSide).r:texture(uFog,worldUV).r;
 if(uDeferMapFog&&vP.y>=0.)fog=1.;
 if(uMapFogEnabled?fog==0.:fog<0.015){outColor=vec4(0.,0.,0.,1.);return;}
 ivec2 cell=clamp(ivec2(floor(p)),ivec2(0),ivec2(int(uMapSide)-1));
 vec2 f=clamp(p-vec2(cell),0.0,1.0);
 vec3 colour=sourceTerrain(cell,f,dx,dz);
 if(uNativeLighting){
  vec2 block=floor(clamp(p,vec2(0.),vec2(uMapSide-.001))/25.);
  vec2 uv=(p-block*25.)*((1.-1./128.)/25.)+vec2(.5/128.);
  float light=texture(uNativeLightmap,vec3(uv,block.x+block.y*uNativeBlocks)).r;
  vec3 stage0=clamp(2.*light*uNativeDiffuse,0.,1.);
  vec3 stage1=clamp(stage0+uNativeAmbient,0.,1.);
  colour=clamp(2.*colour*stage1,0.,1.);
 }else{
  vec3 sun=normalize(vec3(.42,.82,.39));
  colour*=vec3(.55,.57,.59)+vec3(.53,.50,.43)*max(dot(normalize(vN),sun),0.0);
 }
 vec4 territory=texture(uTerritory,worldUV);
 float stepSize=max(.09,uPx*1.3)/256.0;
 float edge=0.0;
 edge=max(edge,step(.001,abs(texture(uTerritory,worldUV+vec2(stepSize,0.0)).r-territory.r)));
 edge=max(edge,step(.001,abs(texture(uTerritory,worldUV-vec2(stepSize,0.0)).r-territory.r)));
 edge=max(edge,step(.001,abs(texture(uTerritory,worldUV+vec2(0.0,stepSize)).r-territory.r)));
 edge=max(edge,step(.001,abs(texture(uTerritory,worldUV-vec2(0.0,stepSize)).r-territory.r)));
 int owner=int(territory.g*255.0+.5);
 vec3 border=owner>0&&owner<=8?uTeam[owner-1]:vec3(.9);
 colour=mix(colour,border,owner>0?.04+edge*.7:edge*.2);
 float reveal=uMapFogEnabled?fog:smoothstep(.02,.45,fog)*mix(.5,1.0,smoothstep(.55,.95,fog));
 outColor=vec4(colour*reveal,1.0);
}`;

const hex=buffer=>Array.from(new Uint8Array(buffer),word=>word.toString(16).padStart(2,'0')).join('');

async function verifiedFile(path,expected,folder){
  const response=await fetch(new URL(path,folder),{cache:'no-store'});
  if(!response.ok)throw new Error(`Pangea ${path} ${response.status}`);
  const bytes=await response.arrayBuffer();
  if(hex(await crypto.subtle.digest('SHA-256',bytes))!==expected)throw new Error(`Pangea ${path} Prüfsumme`);
  return bytes;
}

function makeProgram(gl,fragmentSource=fragment){
  const program=gl.createProgram();
  try{
    for(const [kind,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragmentSource]]){
      const shader=gl.createShader(kind);
      try{
        gl.shaderSource(shader,source);
        gl.compileShader(shader);
        if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program,shader);
      }finally{gl.deleteShader(shader);}
    }
    gl.linkProgram(program);
    if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(program));
    return program;
  }catch(error){gl.deleteProgram(program);throw error;}
}

export function checkedPalette(manifest,bytes,profile=TROPICAL){
  const corners=profile.size+1;
  if(manifest.format!==profile.format||manifest.localOnly!==true||manifest.tileSize!==profile.size||
     manifest.dimensions?.join()!==[corners,corners].join()||
     manifest.textureSize!==256||manifest.materials?.length!==profile.materials||manifest.textures?.length!==profile.textures||
     manifest.materialPlane?.url!=='materials.bin'||!HASH.test(manifest.materialPlane.sha256))throw new Error('Pangea Materialkatalog');
  const ids=new Map();
  for(const [index,row] of manifest.textures.entries()){
    if(row.layer!==index||!PATH.test(row.url)||!HASH.test(row.sha256))throw new Error('Pangea Texturbindung');
  }
  for(const row of manifest.materials){
    if(!Number.isInteger(row.id)||row.id<0||row.id>=PALETTE||ids.has(row.id)||typeof row.name!=='string'||
       row.layers?.length!==12||!row.layers.every(value=>Number.isInteger(value)&&value>=0&&value<manifest.textures.length)){
      throw new Error('Pangea Materialbindung');
    }
    ids.set(row.id,row);
  }
  if(bytes){
    if(bytes.length!==corners**2)throw new Error('Pangea Rastergröße');
    const counts=new Uint32Array(PALETTE);
    for(const id of bytes){if(!ids.has(id))throw new Error('Pangea Rastermaterial');counts[id]++;}
    for(const [id,row] of ids){if(counts[id]!==row.count)throw new Error('Pangea Materialanzahl');}
  }
  return ids;
}

export const loadPangeaTropicalArt=(gl,terrainBuffer,team,options)=>loadPangeaArt(gl,terrainBuffer,team,TROPICAL,options);
export const loadPangeaTemperateArt=(gl,terrainBuffer,team,options)=>loadPangeaArt(gl,terrainBuffer,team,TEMPERATE,options);

async function loadPangeaArt(gl,terrainBuffer,team,profile,{postWaterFog=false}={}){
  const {size,folder,climate}=profile;
  const cornersSide=size+1;
  const decoder=new TextDecoder();
  let lighting=null,lightingError='';
  const readyLighting=verifiedFile('climate-lighting.json',LIGHTING_SHA,new URL('./local-original/',import.meta.url))
    .then(bytes=>{lighting=createNativeLightingContext(JSON.parse(decoder.decode(bytes)),{climate});})
    .catch(error=>{lightingError=error.message;});
  const [pack,blendPack]=await Promise.all([verifiedFile('manifest.json',profile.pin,folder),
    verifiedFile('blend-manifest.json',BLEND_SHA,TROPICAL.folder)]);
  const manifest=JSON.parse(decoder.decode(pack));
  const blend=JSON.parse(decoder.decode(blendPack));
  checkedPalette(manifest,undefined,profile);
  const raster=new Uint8Array(await verifiedFile(manifest.materialPlane.url,manifest.materialPlane.sha256,folder));
  const palette=checkedPalette(manifest,raster,profile);
  const definitions=profile===TROPICAL?blend.definitions:manifest.materials;
  const passes=packedTerrainPasses(raster,size,definitions);
  const basisU=new Float32Array(blend.basis.flatMap(([origin,,dx,,dz])=>[origin,dx,dz]));
  const basisV=new Float32Array(blend.basis.flatMap(([,origin,,dx,,dz])=>[origin,dx,dz]));
  const maskBytes=await verifiedFile(blend.mask.url,blend.mask.sha256,TROPICAL.folder);
  const maskImage=await createImageBitmap(new Blob([maskBytes],{type:'image/png'}),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
  if(maskImage.width!==256||maskImage.height!==256)throw new Error('Pangea Maskenauflösung');
  const art=await Promise.all(manifest.textures.map(async row=>{
    const bytes=await verifiedFile(row.url,row.sha256,folder);
    const image=await createImageBitmap(new Blob([bytes],{type:'image/png'}),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
    if(image.width!==256||image.height!==256)throw new Error('Pangea Texturauflösung');
    return image;
  }));
  const shader=makeProgram(gl);
  const uniforms=Object.fromEntries(['uVP','uMaterials','uCorners','uPasses','uSourceMask','uMaskU[0]','uMaskV[0]',
    'uFog','uTerritory','uSlots[0]','uPhase','uPx','uMapSide','uTeam','uClipEnabled','uClip','uMapFog','uMapFogEnabled','uMapFogSide','uDeferMapFog',
    'uNativeLightmap','uNativeLighting','uNativeBlocks','uNativeDiffuse','uNativeAmbient']
    .map(name=>[name,gl.getUniformLocation(shader,name)]));
  const vao=gl.createVertexArray();
  gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,terrainBuffer);
  gl.enableVertexAttribArray(0);gl.vertexAttribPointer(0,3,gl.FLOAT,false,24,0);
  gl.enableVertexAttribArray(1);gl.vertexAttribPointer(1,3,gl.FLOAT,false,24,12);
  const indices=new Uint32Array(size*size*6);
  for(let z=0,k=0;z<size;z++)for(let x=0;x<size;x++,k+=6){
    const a=z*STRIDE+x,b=a+1,c=a+STRIDE,d=c+1;
    indices[k]=a;indices[k+1]=c;indices[k+2]=d;indices[k+3]=a;indices[k+4]=d;indices[k+5]=b;
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,indices,gl.STATIC_DRAW);
  const texture=gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D_ARRAY,texture);
  gl.texStorage3D(gl.TEXTURE_2D_ARRAY,9,gl.RGBA8,256,256,art.length);
  for(const [layer,image] of art.entries()){
    gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,layer,256,256,1,gl.RGBA,gl.UNSIGNED_BYTE,image);
    image.close();
  }
  gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
  gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  const corners=gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D,corners);
  const alignment=gl.getParameter(gl.UNPACK_ALIGNMENT);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT,1);
  try{gl.texImage2D(gl.TEXTURE_2D,0,gl.R8,cornersSide,cornersSide,0,gl.RED,gl.UNSIGNED_BYTE,raster);}
  finally{gl.pixelStorei(gl.UNPACK_ALIGNMENT,alignment);}
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  const terrainPasses=gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D,terrainPasses);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA16UI,size,size,0,gl.RGBA_INTEGER,gl.UNSIGNED_SHORT,passes);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  const sourceMask=gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D,sourceMask);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,gl.RGBA,gl.UNSIGNED_BYTE,maskImage);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  maskImage.close();
  const lightTexture=gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D_ARRAY,lightTexture);
  gl.texImage3D(gl.TEXTURE_2D_ARRAY,0,gl.R8,1,1,1,0,gl.RED,gl.UNSIGNED_BYTE,new Uint8Array([0]));
  for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D_ARRAY,key,gl.LINEAR);
  for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D_ARRAY,key,gl.CLAMP_TO_EDGE);
  gl.bindVertexArray(null);
  await readyLighting;
  const coefficients=lighting?nativeTerrainCoefficients(lighting):null;
  const diffuse=coefficients?.diffuseBytes.map(byte=>byte/255)??[0,0,0];
  const ambient=coefficients?.ambientBytes.map(byte=>byte/255)??[0,0,0];
  const teams=new Float32Array(team.flat());
  const slots=new Int32Array(PALETTE*2);
  const metrics={climate,side:size,triangles:indices.length/3,materials:palette.size,textures:art.length,sourceMask:true,
    maskedTiles:passes.filter((group,index)=>index%4===1&&group>0).length,draws:0,month0:-1,month1:-1,blend:0,
    nativeLighting:false,lightingError,lightingPreset:lighting?.todKey??null,lightBlocks:1,lightBuilds:0,occludedSamples:0};
  function updateMap(heights){
    if(!lighting)return;
    const width=size+1,cropped=new Float32Array(width**2);
    for(let row=0;row<width;row++)cropped.set(heights.subarray(row*STRIDE,row*STRIDE+width),row*width);
    const {data,resolution,blocks,layers,occludedSamples}=buildNativeTerrainLightmap({
      heights:cropped,size,direction:lighting.direction,sourceDiagonal:true});
    gl.bindTexture(gl.TEXTURE_2D_ARRAY,lightTexture);
    gl.texImage3D(gl.TEXTURE_2D_ARRAY,0,gl.R8,resolution,resolution,layers,0,gl.RED,gl.UNSIGNED_BYTE,data);
    Object.assign(metrics,{nativeLighting:true,lightBlocks:blocks,lightBuilds:metrics.lightBuilds+1,occludedSamples});
  }
  function phase({month0,month1,blend}){
    if(![month0,month1].every(value=>Number.isInteger(value)&&value>=0&&value<12))return;
    if(metrics.month0!==month0||metrics.month1!==month1){
      for(const [id,row] of palette){slots[id*2]=row.layers[month0];slots[id*2+1]=row.layers[month1];}
      metrics.month0=month0;metrics.month1=month1;
    }
    metrics.blend=blend;
  }
  phase({month0:6,month1:7,blend:0});
  let drawMapFog=null;
  if(postWaterFog){
    try{drawMapFog=createPostWaterFogDraw(gl,makeProgram(gl,postWaterFogFragment),vao,indices.length);}
    catch(error){metrics.postWaterFogError=String(error);}
  }
  return {metrics,updateMap,setSeasonPhase:phase,drawMapFog,
    draw({vp,px=1/28,fogTexture,territoryTexture,mapFog=null,deferMapFog=false,clipPlane=null,capture=false}){
    gl.useProgram(shader);gl.bindVertexArray(vao);
    gl.uniformMatrix4fv(uniforms.uVP,false,vp);gl.uniform1f(uniforms.uPx,px);gl.uniform1f(uniforms.uMapSide,size);
    gl.uniform3fv(uniforms.uTeam,teams);gl.uniform2iv(uniforms['uSlots[0]'],slots);gl.uniform1f(uniforms.uPhase,metrics.blend);
    gl.uniform3fv(uniforms['uMaskU[0]'],basisU);gl.uniform3fv(uniforms['uMaskV[0]'],basisV);
    gl.uniform1i(uniforms.uNativeLighting,metrics.nativeLighting?1:0);gl.uniform1f(uniforms.uNativeBlocks,metrics.lightBlocks);
    gl.uniform3fv(uniforms.uNativeDiffuse,diffuse);gl.uniform3fv(uniforms.uNativeAmbient,ambient);
    gl.uniform1i(uniforms.uClipEnabled,clipPlane?1:0);gl.uniform4fv(uniforms.uClip,clipPlane||[0,0,0,0]);
    gl.uniform1i(uniforms.uMapFogEnabled,mapFog?1:0);
    gl.uniform1i(uniforms.uDeferMapFog,mapFog&&deferMapFog?1:0);
    gl.uniform1f(uniforms.uMapFogSide,mapFog?.side??1);
    gl.uniform1i(uniforms.uMapFog,mapFog?7:2);
    if(mapFog){gl.activeTexture(gl.TEXTURE7);gl.bindTexture(gl.TEXTURE_2D,mapFog.texture);}
    for(const [name,unit,target,id] of [['uMaterials',0,gl.TEXTURE_2D_ARRAY,texture],['uCorners',1,gl.TEXTURE_2D,corners],
      ['uFog',2,gl.TEXTURE_2D,fogTexture],['uTerritory',3,gl.TEXTURE_2D,territoryTexture],
      ['uPasses',4,gl.TEXTURE_2D,terrainPasses],['uSourceMask',5,gl.TEXTURE_2D,sourceMask],
      ['uNativeLightmap',6,gl.TEXTURE_2D_ARRAY,lightTexture]]){
      gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,id);gl.uniform1i(uniforms[name],unit);
    }
    gl.drawElements(gl.TRIANGLES,indices.length,gl.UNSIGNED_INT,0);
    if(!capture&&!clipPlane)metrics.draws++;
    gl.activeTexture(gl.TEXTURE0);
  }};
}
