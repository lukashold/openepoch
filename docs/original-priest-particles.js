import {createLegacyEmitter,createLegacyRand} from './legacy-particles.js?v=native47';
import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js';

const VS=`#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in vec4 aColor;
uniform mat4 uVP;
out vec2 vUV;
out vec4 vColor;
void main(){vUV=aUV;vColor=aColor;gl_Position=uVP*vec4(aPosition,1.);}`;
const FS=`#version 300 es
precision highp float;
uniform sampler2D uTexture;
in vec2 vUV;
in vec4 vColor;
out vec4 frag;
void main(){
 vec4 source=texture(uTexture,vUV)*vColor;
 if(source.a<=0.)discard;
 frag=source;
}`;
const kinds=['convert','converted'];
const keys=['smoke','heal01','fx_nuke_glow'];
const uv=[[0,0],[0,1],[1,0],[1,0],[0,1],[1,1]];
const finite=(array,length)=>array?.length===length&&Array.from(array).every(Number.isFinite);

function portableSeed(key){
 let seed=2166136261;
 for(let i=0;i<key.length;i++)seed=Math.imul(seed^key.charCodeAt(i),16777619);
 return seed>>>0;
}

export function createPriestParticleFrames(manifest,{seedFor=portableSeed}={}){
 const effects=Object.fromEntries(kinds.map(kind=>[kind,manifest.effects[kind].emitters]));
 for(const emitters of Object.values(effects)){
  for(const emitter of emitters){
   if(!keys.includes(emitter.baseTexture)||emitter.properties.NiAlphaProperty?.Flags!==4621
      ||emitter.properties.NiAlphaProperty?.Threshold!==0)throw Error('Unsupported Priest particle material');
   createLegacyEmitter(emitter,()=>0);
  }
 }
 let active=new Map();
 return {
  advance(events){
   const next=new Map();
   for(const {key,kind,elapsed:rawElapsed} of events){
    const elapsed=Math.fround(rawElapsed);
    if(typeof key!=='string'||!kinds.includes(kind)||!(elapsed>=0&&elapsed<2)||next.has(key))continue;
    let effect=active.get(key);
    if(effect&&(effect.kind!==kind||elapsed<effect.elapsed))continue;
    if(!effect){
     const rand=createLegacyRand(seedFor(key));
     effect={kind,elapsed:0,emitters:effects[kind].map(emitter=>({texture:emitter.baseTexture,
      runtime:createLegacyEmitter(emitter,rand),particles:[]}))};
    }
    if(elapsed>effect.elapsed){
     effect.elapsed=elapsed;
     for(const row of effect.emitters){row.runtime.step(elapsed);row.particles=row.runtime.snapshot();}
    }
    next.set(key,effect);
   }
   active=next;
  },
  reset(){active.clear();},
  effect(key){return active.get(key);},
 };
}

function program(gl){
 const result=gl.createProgram();
 try{
  for(const [type,source] of [[gl.VERTEX_SHADER,VS],[gl.FRAGMENT_SHADER,FS]]){
   const shader=gl.createShader(type);
   try{
    gl.shaderSource(shader,source);gl.compileShader(shader);
    if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
    gl.attachShader(result,shader);
   }finally{gl.deleteShader(shader);}
  }
  gl.linkProgram(result);
  if(!gl.getProgramParameter(result,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(result));
  return result;
 }catch(error){gl.deleteProgram(result);throw error;}
}

function packDiffuse(component){
 // The DX8 particle vertex stores f32(channel*255) through FISTP.
 const byte=Math.fround(component*255),lower=Math.floor(byte);
 const fraction=byte-lower;
 return (lower+(fraction>.5||fraction===.5&&lower%2===1?1:0))/255;
}

function project({position,yaw,scale},particle,right,up,list){
 const [x,y,z]=particle.position,c=Math.cos(yaw),s=Math.sin(yaw),radius=particle.radius*scale;
 const center=[position[0]+(x*c+y*s)*scale,position[1]+z*scale,position[2]+(x*s-y*c)*scale];
 const color=particle.color.map(packDiffuse);
 for(const [u,v] of uv){
  const du=(2*u-1)*radius,dv=(1-2*v)*radius;
  list.push(...center.map((p,i)=>p+right[i]*du+up[i]*dv),u,v,...color);
 }
}

export async function loadOriginalPriestParticles(gl,{enabled=false,base=new URL('./',import.meta.url),
 fetchFile=globalThis.fetch,decode=globalThis.createImageBitmap,seedFor}={}){
 if(!enabled)return null;
 const release=[],images=new Map();
 try{
  const file=await fetchFile(new URL('local-original/priest-fx/manifest.json',base));
  if(!file.ok)return null;
  const manifest=await file.json();
  if(manifest.format!=='ee2-native-priest-fx-v1')return null;
  const frames=createPriestParticleFrames(manifest,{seedFor});
  for(const key of keys){
   const record=manifest.textures[key];
   if(record.url!==`local-original/priest-fx/${key}.png`)throw Error('Invalid Priest particle texture');
   const source=await fetchFile(new URL(record.url,base));
   if(!source.ok)throw Error('Missing Priest particle texture');
   const image=await decode(await source.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
   images.set(key,image);
   if(image.width!==record.size[0]||image.height!==record.size[1])throw Error('Invalid Priest particle texture size');
  }
  const state=saveSceneGLState(gl),flip=gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
  try{
   const shader=program(gl);release.push(()=>gl.deleteProgram(shader));
   const matrix=gl.getUniformLocation(shader,'uVP'),sampler=gl.getUniformLocation(shader,'uTexture');
   const textures=new Map();
   gl.activeTexture(gl.TEXTURE0);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
   for(const [key,image] of images){
    const texture=gl.createTexture();release.push(()=>gl.deleteTexture(texture));
    gl.bindTexture(gl.TEXTURE_2D,texture);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
    for(const flag of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,flag,gl.LINEAR);
    for(const flag of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,flag,gl.REPEAT);
    textures.set(key,texture);
   }
   const vao=gl.createVertexArray(),buffer=gl.createBuffer();
   release.push(()=>{gl.deleteVertexArray(vao);gl.deleteBuffer(buffer);});
   gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
   for(const [loc,length,offset] of [[0,3,0],[1,2,12],[2,4,20]]){
    gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,length,gl.FLOAT,false,36,offset);
   }
   let disposed=false;
   return {
    advance:frames.advance,reset:frames.reset,
    dispose(){if(!disposed){disposed=true;frames.reset();release.reverse().forEach(remove=>remove());}},
    draw({vp,right,u:up,effects=[]}){
     if(disposed||!effects.length||!finite(right,3)||!finite(up,3))return 0;
     const groups=new Map();
     for(const event of effects){
      const effect=frames.effect(event.key);
      if(!effect||effect.kind!==event.kind||event.visible!==true||!finite(event.position,3)
         ||!Number.isFinite(event.yaw)||!Number.isFinite(event.scale)||!(event.scale>0))continue;
      for(const {texture,particles} of effect.emitters){
       if(!particles.length)continue;
       const list=groups.get(texture)??[];
       for(const particle of particles)project(event,particle,right,up,list);
       groups.set(texture,list);
      }
     }
     if(!groups.size)return 0;
     const before=saveSceneGLState(gl);
     try{
      gl.useProgram(shader);gl.uniformMatrix4fv(matrix,false,vp);gl.uniform1i(sampler,0);
      gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);gl.disable(gl.CULL_FACE);
      gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE);
      gl.activeTexture(gl.TEXTURE0);gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
      let drawn=0;
      for(const [key,list] of groups){
       gl.bindTexture(gl.TEXTURE_2D,textures.get(key));
       gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(list),gl.STREAM_DRAW);
       gl.drawArrays(gl.TRIANGLES,0,list.length/9);drawn+=list.length/54;
      }
      return drawn;
     }finally{restoreSceneGLState(gl,before);}
    },
   };
  }finally{restoreSceneGLState(gl,state);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,flip);}
 }catch{
  release.reverse().forEach(remove=>remove());
  return null;
 }finally{for(const image of images.values())image.close();}
}
