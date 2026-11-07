import {normalizeRenderAssetMapping,sourceRenderBindings} from './original-render-namespace.js';
// Original Under* building sprites, extracted locally from EE2. Only rendering
// is adapted here; ownership, collision, terrain and the WASM state are untouched.
// Native VisualDecal006040D0 uses epoch numbers1..5,6..10,11..15. Urban
// membership is supplied by the caller's CityManager query, never guessed from
// ownership or a minimap colour. World-owned decals keep source slot zero.
const STRIDE=12,MAP=256,HN=257,GRID=32;

export const GROUND_CITY_RADIUS_FACTOR=1.4142135381698608;
export function groundEpochSlot(nativeEpoch=1,hasOwner=true){
  if(!hasOwner)return 0;
  if(!Number.isFinite(nativeEpoch))return 0;
  return nativeEpoch>=11?2:nativeEpoch>=6?1:0;
}
export function selectGroundSprite(entry,{epoch=0,urban=false,hasOwner=true,state='Idle'}={}){
  const groups=state==='Idle'?entry:state==='Death'?entry.death:null;
  return groups?.[urban?'urban':'rural']?.[groundEpochSlot(epoch+1,hasOwner)]||null;
}
export function nativeUrbanAt(x,z,cities){
  if(!Number.isFinite(x)||!Number.isFinite(z))return false;
  x=Math.fround(x);z=Math.fround(z);
  for(const city of cities){
    if(city.active===false||!Number.isFinite(city.x)||!Number.isFinite(city.z)||!(city.radius>0))continue;
    const dx=Math.abs(Math.fround(x-Math.fround(city.x))),dz=Math.abs(Math.fround(z-Math.fround(city.z)));
    // Native x87 stores the L1 sum in float, but compares against an extended
    // radius product. Keeping the product in JS double preserves that boundary.
    if(Math.fround(dx+dz)<Math.fround(city.radius)*GROUND_CITY_RADIUS_FACTOR)return true;
  }
  return false;
}
export function groundSeasonFrames(sprite,phase,urls=sprite?.seasonalUrls){
  if(!sprite)return null;
  if(!Number.isFinite(phase)||!urls?.length)return{first:sprite.url,next:sprite.url,blend:0};
  let normalized=phase%1;if(normalized<0)normalized+=1;
  const position=normalized*urls.length,index=Math.floor(position);
  return{first:urls[index],next:urls[(index+1)%urls.length],blend:position-index};
}

export function groundPlacement(entry,instance){
  const flags=Math.round(instance[7]);
  if((flags&64)===0||(flags&24)!==0||instance[6]<1||!(instance[8]>0))return null;
  if(!entry.nativeSize?.every(n=>Number.isFinite(n)&&n>0)||!(entry.width>0)||!(entry.depth>0))return null;
  // Native UndertextureSize/Offset values are world dimensions independent of
  // NIF/model normalization. Old manifests retain their explicitly sized adapter.
  const scale=['native-world-undertexture','native-unit-factory-undertexture'].includes(entry.placementProfile)?1:instance[8]/Math.max(...entry.nativeSize),angle=instance[3];
  const ox=entry.offsetX*scale,oz=entry.offsetY*scale,c=Math.cos(angle),s=Math.sin(angle);
  return [instance[0]+ox*c-oz*s,instance[1]+ox*s+oz*c,angle,entry.width*scale,entry.depth*scale];
}

// Same two triangles as main.js's terrain, including the b-c diagonal.
export function sampleGroundHeight(heights,x,z){
  x=Math.max(0,Math.min(MAP-.0001,x));z=Math.max(0,Math.min(MAP-.0001,z));
  const ix=Math.floor(x),iz=Math.floor(z),fx=x-ix,fz=z-iz,i=iz*HN+ix;
  const a=heights[i],b=heights[i+1],c=heights[i+HN],d=heights[i+HN+1];
  return fx+fz<=1?a+(b-a)*fx+(c-a)*fz:d+(c-d)*(1-fx)+(b-d)*(1-fz);
}

const HEIGHT=`
float ground(vec2 w){
 vec2 p=clamp(w,vec2(0.),vec2(255.9999)),f=fract(p);ivec2 i=ivec2(floor(p));
 float a=texelFetch(uHeight,i,0).r,b=texelFetch(uHeight,i+ivec2(1,0),0).r;
 float c=texelFetch(uHeight,i+ivec2(0,1),0).r,d=texelFetch(uHeight,i+ivec2(1,1),0).r;
 return f.x+f.y<=1.?a+(b-a)*f.x+(c-a)*f.y:d+(c-d)*(1.-f.x)+(b-d)*(1.-f.y);
}`;
const VS=`#version 300 es
precision highp float;
layout(location=0)in vec2 aUV;layout(location=1)in vec4 iPlace;layout(location=2)in float iDepth;
uniform mat4 uVP;uniform highp sampler2D uHeight;out vec2 vUV;out vec2 vWorld;
${HEIGHT}
void main(){
 vec2 p=(aUV-.5)*vec2(iPlace.w,iDepth);float c=cos(iPlace.z),s=sin(iPlace.z);
 vWorld=iPlace.xy+vec2(p.x*c-p.y*s,p.x*s+p.y*c);vUV=aUV;
 gl_Position=uVP*vec4(vWorld.x,ground(vWorld)+.018,vWorld.y,1.);
}`;
const FS=`#version 300 es
precision highp float;
uniform sampler2D uDiffuse,uDiffuseB,uFog;uniform float uSeasonMix,uGlowAlpha;uniform vec3 uTint;uniform highp sampler2D uHeight;uniform mat4 uVP;
in vec2 vUV;in vec2 vWorld;out vec4 outColor;
${HEIGHT}
void main(){
 if(any(lessThan(vWorld,vec2(0.)))||any(greaterThan(vWorld,vec2(256.))))discard;
 if(uGlowAlpha>=0.){
  vec4 color=texture(uDiffuse,vUV);
  outColor=vec4(color.rgb*(color.a*uGlowAlpha),0.);
 }else if(uGlowAlpha< -1.5){
  vec4 color=texture(uDiffuse,vUV);
  outColor=vec4(color.rgb*uTint*color.a,color.a);
 }else{
  vec4 color=mix(texture(uDiffuse,vUV),texture(uDiffuseB,vUV),uSeasonMix);float fog=texture(uFog,vWorld/256.).r;
  if(color.a<.004||fog<.05)discard;
  float light=.55+.6*dot(normalize(vec3(.42,.82,.39)),vec3(0.,1.,0.));
  color.rgb*=light*smoothstep(.02,.45,fog)*mix(.5,1.,smoothstep(.55,.95,fog));
  outColor=vec4(color.rgb*color.a,color.a);
 }
 // Depth-test against the actual terrain triangles, even where the overlay
 // grid crosses a terrain diagonal; never write into the depth buffer.
 vec4 clip=uVP*vec4(vWorld.x,ground(vWorld)+.018,vWorld.y,1.);
 gl_FragDepth=clip.z/clip.w*.5+.5;
}`;
export const ORIGINAL_GROUND_SHADERS=Object.freeze({vertex:VS,fragment:FS});

function createProgram(gl){
  const p=gl.createProgram();
  try{
    for(const [type,source]of[[gl.VERTEX_SHADER,VS],[gl.FRAGMENT_SHADER,FS]]){
      const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);
      if(!gl.getShaderParameter(s,gl.COMPILE_STATUS)){const message=gl.getShaderInfoLog(s);gl.deleteShader(s);throw Error(message);}
      gl.attachShader(p,s);gl.deleteShader(s);
    }
    gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(p));
    const u={};for(const name of['uVP','uDiffuse','uDiffuseB','uSeasonMix','uGlowAlpha','uTint','uFog','uHeight'])u[name]=gl.getUniformLocation(p,name);
    return {p,u};
  }catch(error){gl.deleteProgram(p);throw error;}
}

function snapshot(gl){
  const state={active:gl.getParameter(gl.ACTIVE_TEXTURE),program:gl.getParameter(gl.CURRENT_PROGRAM),
    vao:gl.getParameter(gl.VERTEX_ARRAY_BINDING),array:gl.getParameter(gl.ARRAY_BUFFER_BINDING),
    depth:gl.getParameter(gl.DEPTH_WRITEMASK),blend:gl.isEnabled(gl.BLEND),
    src:gl.getParameter(gl.BLEND_SRC_RGB),dst:gl.getParameter(gl.BLEND_DST_RGB),
    srcA:gl.getParameter(gl.BLEND_SRC_ALPHA),dstA:gl.getParameter(gl.BLEND_DST_ALPHA),
    flip:gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL)};
  gl.activeTexture(gl.TEXTURE0);state.diffuse=gl.getParameter(gl.TEXTURE_BINDING_2D);
  gl.activeTexture(gl.TEXTURE6);state.height=gl.getParameter(gl.TEXTURE_BINDING_2D);
  gl.activeTexture(gl.TEXTURE0+7);state.diffuseB=gl.getParameter(gl.TEXTURE_BINDING_2D);
  return state;
}
function restore(gl,state){
  gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,state.diffuse);
  gl.activeTexture(gl.TEXTURE6);gl.bindTexture(gl.TEXTURE_2D,state.height);
  gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,state.diffuseB);gl.activeTexture(state.active);
  gl.useProgram(state.program);gl.bindVertexArray(state.vao);gl.bindBuffer(gl.ARRAY_BUFFER,state.array);
  gl.depthMask(state.depth);if(state.blend)gl.enable(gl.BLEND);else gl.disable(gl.BLEND);
  gl.blendFuncSeparate(state.src,state.dst,state.srcA,state.dstA);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,state.flip);
}

export async function loadOriginalGround(gl,enabled,options={}){
  const result={enabled:false,entries:new Map(),errors:[],manifest:null,
    policy:'native-epoch-slots; CityManager callback, explicit rural fallback when absent',metrics:{calls:0,instances:0},
    seasonClimates:['Temperate'],climateErrors:[],updateTerrain(){},draw(){},drawCaptureGlow(){},drawBlessGlow(){},drawExtraction(){},setSeasonClimate(){return false;},setSeasonPhase(){return false;}};
  if(!enabled)return result;
  let assetMapping;try{assetMapping=normalizeRenderAssetMapping(options.assetMapping);}catch(error){result.errors.push(error.message);return result;}
  result.assetMapping=assetMapping;
  const fetchFile=options.fetch||fetch,decode=options.decode||createImageBitmap;
  const state=snapshot(gl),textures=new Map(),blessTextures=new Map();let shader,vao,instances,height,glowTexture,extractionTexture;
  const groundProfiles=Object.entries(options.climateProfiles||{}).filter(([name])=>name!=='Temperate');
  async function loadTexture(url){
    let texture=textures.get(url);
    if(texture)return texture;
    const response=await fetchFile(new URL(url,import.meta.url));
    if(!response.ok)throw Error(`${url}: ${response.status}`);
    const image=await decode(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
    texture=gl.createTexture();gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);image.close();
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
    gl.generateMipmap(gl.TEXTURE_2D);textures.set(url,texture);
    return texture;
  }
  try{
    const response=await fetchFile(new URL(options.manifestURL??'./local-original/ground-decals.json',import.meta.url),{cache:'no-store'});
    if(!response.ok)throw Error(`Ground manifest: ${response.status}`);
    const manifest=await response.json();result.manifest=manifest;
    // Deduplicate physical source images shared by several building bindings.
    for(const entry of manifest.entries||[]){
      const bindings=sourceRenderBindings(assetMapping,0,+entry.id);if(!bindings.length)continue;
      const entryTextures=new Map();
      // All source-declared slots are distinct bindings. Preload their physical
      // images once so a later epoch/context change cannot retain the old floor.
      const declared=[...(entry.rural||[]),...(entry.urban||[]),...(entry.death?.rural||[]),...(entry.death?.urban||[])];
      const sprites=declared.flatMap(sprite=>{
        const climateUrls=groundProfiles.flatMap(([,profile])=>profile.ground[sprite.sprite?.toLowerCase()]||[]);
        return [sprite,...(sprite.seasonalUrls||[]).map(url=>({...sprite,url})),...climateUrls.map(url=>({...sprite,url,optionalClimate:true}))];
      });
      for(const sprite of sprites){
      if(!sprite?.url||entryTextures.has(sprite.url))continue;
      // Unsupported source transforms must fail explicitly, never be ignored.
      if(sprite.flipU||sprite.flipV||sprite.rotate90||sprite.uv?.some((v,i)=>v!==[0,0,1,1][i])){
        result.errors.push(`${entry.decal}: unsupported sprite transform`);continue;
      }
      try{
        entryTextures.set(sprite.url,await loadTexture(sprite.url));
      }catch(error){(sprite.optionalClimate?result.climateErrors:result.errors).push(error.message);}
      }
      if(entryTextures.size)for(const {renderId}of bindings)result.entries.set(renderId,{entry,sourceId:+entry.id,assetKind:0,texture:entryTextures.get(selectGroundSprite(entry)?.url),textures:entryTextures});
    }
    if(!result.entries.size)return result;
    if(options.captureGlow){
      try{glowTexture=await loadTexture('./local-original/capture-glow/glow_bad.png');}
      catch(error){result.captureGlowError=error.message;}
    }
    if(options.blessGlow){
      for(const kind of ['good','neutral']){
        try{blessTextures.set(kind,await loadTexture(`./local-original/priest-fx/glow_${kind}.png`));}
        catch(error){(result.blessGlowErrors??=[]).push(error.message);}
      }
    }
    if(options.extractionDecal){
      try{extractionTexture=await loadTexture(options.extractionDecal);}
      catch(error){result.extractionError=error.message;}
    }
    shader=createProgram(gl);vao=gl.createVertexArray();gl.bindVertexArray(vao);
    const vertices=new Float32Array((GRID+1)**2*2),indices=new Uint16Array(GRID*GRID*6);
    for(let z=0,k=0;z<=GRID;z++)for(let x=0;x<=GRID;x++){vertices[k++]=x/GRID;vertices[k++]=z/GRID;}
    for(let z=0,k=0;z<GRID;z++)for(let x=0;x<GRID;x++){
      const a=z*(GRID+1)+x,b=a+1,c=a+GRID+1,d=c+1;
      indices.set([a,c,b,b,c,d],k);k+=6;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);gl.vertexAttribPointer(0,2,gl.FLOAT,false,8,0);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,gl.createBuffer());gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,indices,gl.STATIC_DRAW);
    instances=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,instances);
    for(const [loc,size,offset]of[[1,4,0],[2,1,16]]){
      gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,size,gl.FLOAT,false,20,offset);gl.vertexAttribDivisor(loc,1);
    }
    height=gl.createTexture();gl.activeTexture(gl.TEXTURE6);gl.bindTexture(gl.TEXTURE_2D,height);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.R32F,HN,HN,0,gl.RED,gl.FLOAT,new Float32Array(HN*HN));
    for(const name of[gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,name,gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
    gl.useProgram(shader.p);gl.uniform1i(shader.u.uDiffuse,0);gl.uniform1i(shader.u.uDiffuseB,7);gl.uniform1i(shader.u.uFog,2);gl.uniform1i(shader.u.uHeight,6);
    result.enabled=true;
  }catch(error){result.errors.push(error.message);console.warn('Original-Gebäudeböden:',error);}
  finally{restore(gl,state);}
  if(!result.enabled)return result;
  for(const [name,profile] of groundProfiles){
    const ready=[...result.entries.values()].every(({entry,textures})=>[...entry.rural,...entry.urban].every(sprite=>{
      const sequence=profile.ground[sprite.sprite.toLowerCase()];
      return sequence?.length&&sequence.every(url=>textures.has(url));
    }));
    if(ready)result.seasonClimates.push(name);
  }
  let yearPhase, climateGround=null;
  result.setSeasonClimate=name=>{
    if(!result.seasonClimates.includes(name))return false;
    climateGround=name==='Temperate'?null:options.climateProfiles[name].ground;
    return true;
  };
  result.setSeasonPhase=({month0,month1=(month0+1)%12,blend=0})=>{
    if(!Number.isInteger(month0)||month0<0||month0>11||month1!==(month0+1)%12||!Number.isFinite(blend)||blend<0||blend>1)throw new RangeError('Invalid ground season phase');
    yearPhase=(month0+blend)/12;return true;
  };
  result.updateTerrain=heights=>{
    if(heights.length!==HN*HN)throw Error('Ground height map must be 257 × 257');
    const state=snapshot(gl);
    try{gl.activeTexture(gl.TEXTURE6);gl.bindTexture(gl.TEXTURE_2D,height);gl.texSubImage2D(gl.TEXTURE_2D,0,0,0,HN,HN,gl.RED,gl.FLOAT,heights);}
    finally{restore(gl,state);}
  };
  result.draw=({data,ranges,vp,epochs=[],isUrbanAt})=>{
    result.metrics.calls=0;result.metrics.instances=0;
    const groups=[];
    for(const [id,{entry,textures,sourceId}]of result.entries){
      const first=ranges[id*2],count=ranges[id*2+1],byTexture=new Map();
      for(let i=first;i<first+count;i++){
        const instance=data.subarray(i*STRIDE,(i+1)*STRIDE),team=Math.round(instance[5]);
        const placement=groundPlacement(entry,instance);if(!placement)continue;
        const hasOwner=epochs[team]!==undefined,urban=typeof isUrbanAt==='function'&&!!isUrbanAt(instance[0],instance[1],team,sourceId,i,id);
        const sprite=selectGroundSprite(entry,{epoch:epochs[team]??0,hasOwner,urban});
        const frames=groundSeasonFrames(sprite,yearPhase,climateGround?.[sprite?.sprite?.toLowerCase()]);
        const texture=textures.get(frames?.first),textureB=textures.get(frames?.next);
        if(!texture||!textureB)continue;
        const key=JSON.stringify([frames.first,frames.next,frames.blend]);
        if(placement){const group=byTexture.get(key)||{texture,textureB,blend:frames.blend,list:[]};group.list.push(...placement);byTexture.set(key,group);}
      }
      for(const group of byTexture.values())groups.push(group);
    }
    drawGroups(groups,vp,-1);
  };
  result.drawCaptureGlow=({vp,list,alpha})=>{
    if(glowTexture&&list.length)drawGroups([{texture:glowTexture,textureB:glowTexture,blend:0,list}],vp,alpha);
  };
  result.drawBlessGlow=({vp,good,neutral,alpha})=>{
    const groups=[];
    for(const [kind,list] of [['neutral',neutral],['good',good]]){
      const texture=blessTextures.get(kind);
      if(texture&&list?.length)groups.push({texture,textureB:texture,blend:0,list});
    }
    if(groups.length)drawGroups(groups,vp,alpha);
  };
  result.drawExtraction=({vp,list,tint})=>{
    if(!extractionTexture||!list.length)return 0;
    drawGroups([{texture:extractionTexture,textureB:extractionTexture,blend:0,list}],vp,-2,tint);
    return list.length/5;
  };
  function drawGroups(groups,vp,glowAlpha,tint){
    if(!groups.length)return;
    const state=snapshot(gl);
    try{
      gl.enable(gl.BLEND);gl.depthMask(false);
      if(glowAlpha>=0)gl.blendFuncSeparate(gl.ONE,gl.ONE,gl.ZERO,gl.ONE);
      else gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
      gl.useProgram(shader.p);gl.uniformMatrix4fv(shader.u.uVP,false,vp);gl.bindVertexArray(vao);
      gl.uniform1f(shader.u.uGlowAlpha,glowAlpha);
      if(tint)gl.uniform3fv(shader.u.uTint,tint);
      gl.activeTexture(gl.TEXTURE6);gl.bindTexture(gl.TEXTURE_2D,height);
      for(const {texture,textureB,blend,list}of groups){
        gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
        gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,textureB);gl.uniform1f(shader.u.uSeasonMix,blend);
        gl.bindBuffer(gl.ARRAY_BUFFER,instances);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);
        gl.drawElementsInstanced(gl.TRIANGLES,GRID*GRID*6,gl.UNSIGNED_SHORT,0,list.length/5);
        if(glowAlpha<0){result.metrics.calls++;result.metrics.instances+=list.length/5;}
      }
    }finally{restore(gl,state);}
  }
  return result;
}
