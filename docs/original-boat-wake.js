import {add,div,mul,store32,sub} from './source-cloth-arithmetic.js';
import {guidedImpactInside} from './guided-herc-impact.js?v=native78-wake';
import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js';

const manifestPath='./local-original/boat-wake/manifest.json';
const manifestSha256='0ea3fa84c35d5672245f3c1e90a7924ba81175726c4a77df61d099755315ff30';
const highManifestSha256='74ae41c04153c1f58130a47a814370be059039210b30071418246871bec39945';
const stride=52;
const finite=(values,length)=>values?.length===length&&Array.prototype.every.call(values,Number.isFinite);

export function readNavalWakeActors(wasm,alpha){
 const count=wasm.render_naval_wakes(alpha)>>>0;
 if(!count)return [];
 const ptr=wasm.render_naval_wakes_ptr()>>>0, memory=wasm.memory.buffer;
 if(!ptr||ptr%4||ptr+count*stride>memory.byteLength)return [];
 const rows=new DataView(memory,ptr,count*stride),result=[];
 for(let i=0;i<count;i++){
  const offset=i*stride,word=slot=>rows.getUint32(offset+slot*4,true),value=slot=>rows.getFloat32(offset+slot*4,true);
  const basis=Array.from({length:9},(_,index)=>value(4+index));
  if(word(1)<=1&&finite(basis,9)&&Number.isFinite(value(2))&&Number.isFinite(value(3)))
   result.push({handle:word(0),moving:word(1)===1,x:value(2),y:value(3),basis});
 }
 return result;
}

export function wakeTrack({method,keys},seconds){
 if(seconds<=keys[0][0])return keys[0][1];
 for(let i=1;i<keys.length;i++){
  const [end,value,incoming]=keys[i];
  if(seconds>end)continue;
  const [start,prior,,outgoing]=keys[i-1];
  if(method===2&&incoming!==undefined&&outgoing!==undefined){
   // Float-key callback 00971570 uses the previous key's second tangent
   // and the next key's first. 00976AC0 evaluates the cubic in this order.
   const t=store32(div(sub(seconds,start),sub(end,start)));
   const change=sub(value,prior);
   const cubic=sub(add(outgoing,incoming),add(change,change));
   const quadratic=sub(mul(change,3),add(add(outgoing,outgoing),incoming));
   return store32(add(mul(add(mul(add(mul(cubic,t),quadratic),t),outgoing),t),prior));
  }
  const t=(seconds-start)/(end-start);
  // The inherited compact tracks have zero tangents.
  const mix=method===2?t*t*(3-2*t):t;
  return Math.fround(prior+(value-prior)*mix);
 }
 return keys.at(-1)[1];
}

export function wakeTagXY(actor,{point},scale){
 const [a,,c,d,,f,g,,i]=actor.basis;
 const [x,y,z]=point;
 // 008E6970 writes each rotated coordinate before scale and world translation.
 // Model Y is reflected into the browser's ground Z axis.
 const rx=store32(add(add(d*z,-g*y),a*x));
 const ry=store32(add(add(f*z,c*x),-i*y));
 return [store32(add(actor.x,store32(rx*scale))),store32(add(actor.y,store32(ry*scale)))];
}

export function createBoatWakeFrames({hull,descriptor,models},{seed=()=>Math.floor(Date.now()/1000)}={}){
 let session,random=0,owners=new Map(),waiting=[],ready=[],active=[];
 let detail=1,interval;
 const detailFactors=[Math.fround(1.3),Math.fround(1.2),Math.fround(1.1),.8];
 const supportsDetail=value=>Number.isInteger(value)&&value>=0&&value<=(models.crest?3:1);
 function setDetail(value){
  detail=supportsDetail(value)?value:1;
  interval=store32(mul(descriptor.movedTimer,detailFactors[detail]));
  return detail;
 }
 setDetail(detail);
 const stats={owners:0,waiting:0,ready:0,moving:0,idle:0,crest:0};
 function reset(next){
  session=next;random=seed()>>>0;owners.clear();waiting=[];ready=[];active=[];
 }
 const word=()=>((random=(Math.imul(random,0x91b1022c)+0x323c3a29)>>>0)>>>16)&0x7fff;
 function advance({world,delta,actors,views,visible}){
  if(world!==session)reset(world);
  if(!Number.isFinite(delta)||delta<0)return;
  delta=store32(delta);
  // 006244F6 rejects a sphere touching a plane. Guided impact uses the
  // separately traced inclusive test, so select the strict boundary here.
  const admissible=effect=>visible(effect.x,effect.y)&&views.some(vp=>guidedImpactInside(vp,effect.x,effect.y,descriptor.z,true));
  // 00670AA0 registers a transient child in the staging list. It is advanced
  // only when the next visual update transfers that list into the active scene.
  active=active.concat(ready).filter(effect=>{
   effect.age=store32(add(effect.age,delta));
   return effect.age<models[effect.kind].duration;
  });
  ready=[];
  waiting=waiting.filter(effect=>{
   const elapsed=add(effect.age,delta);
   effect.age=store32(elapsed);
   if(elapsed>=descriptor.pendingSeconds)return false;
   if(!admissible(effect))return true;
   ready.push(effect);
   return false;
  });
  const current=new Set();
  for(const actor of actors){
   current.add(actor.handle);
   let owner=owners.get(actor.handle);
   if(!owner){
    owner={moving:false,counter:store32(word()*descriptor.idleTimer*0.000030518509447574615)};
    owners.set(actor.handle,owner);
   }
   if(actor.moving!==owner.moving){
    owner.moving=actor.moving;
    owner.counter=owner.moving?descriptor.movedTimer:0;
   }
   owner.counter=store32(add(owner.counter,delta));
   if(owner.counter<(owner.moving?interval:descriptor.idleTimer))continue;
   owner.counter=0;
   if(detail===0)continue;
   const yaw=store32(Math.atan2(-actor.basis[6],actor.basis[0]));
   const emit=(kind,x,y)=>{
    const effect={kind,x,y,yaw,age:0};
    (admissible(effect)?ready:waiting).push(effect);
   };
   if(owner.moving){
    for(const tag of hull.tags){
     const point=wakeTagXY(actor,tag,hull.scale);
     // The source's tagged branch calls both models at 00639867/006398BB.
     // Only the tagless branch tests the high-detail L1 flag.
     if(models.crest)emit('crest',...point);
     emit('moving',...point);
    }
   }else emit('idle',actor.x,actor.y);
  }
  for(const handle of owners.keys())if(!current.has(handle))owners.delete(handle);
  Object.assign(stats,{owners:owners.size,waiting:waiting.length,ready:ready.length,moving:0,idle:0,crest:0});
  for(const effect of active)stats[effect.kind]++;
 }
 return {advance,stats,setDetail,supportsDetail,detail:()=>detail,sample:visible=>active.filter(effect=>visible(effect.x,effect.y))};
}

const vertex=`#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in float aWakeAlpha;
uniform mat4 uWakeVP;
out vec2 vUV;
out float vAlpha;
void main(){vUV=aUV;vAlpha=aWakeAlpha;gl_Position=uWakeVP*vec4(aPosition,1.);}`;
const fragment=`#version 300 es
precision highp float;
uniform sampler2D uWakeTexture;
uniform bool uWakeAlphaTest;
in vec2 vUV;
in float vAlpha;
out vec4 color;
void main(){vec4 texel=texture(uWakeTexture,vUV);color=vec4(texel.rgb,texel.a*vAlpha);
 if(uWakeAlphaTest&&color.a<=0.)discard;}`;

function makeProgram(gl){
 const program=gl.createProgram();
 for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
  const shader=gl.createShader(type);gl.shaderSource(shader,source);gl.compileShader(shader);
  const ok=gl.getShaderParameter(shader,gl.COMPILE_STATUS);
  if(ok)gl.attachShader(program,shader);
  gl.deleteShader(shader);
  if(!ok){gl.deleteProgram(program);throw Error('Boat wake shader failed');}
 }
 gl.linkProgram(program);
 if(!gl.getProgramParameter(program,gl.LINK_STATUS)){gl.deleteProgram(program);throw Error('Boat wake shader link failed');}
 return program;
}

function wakeVertices(model,effects,z){
 const vertices=[];
 for(const effect of effects){
  const scale=wakeTrack(model.scale,effect.age),alpha=wakeTrack(model.alpha,effect.age);
  if(alpha<=0)continue;
  const cos=Math.cos(effect.yaw),sin=Math.sin(effect.yaw);
  for(const index of model.indices){
   const [nx,ny,nz]=model.positions[index];
   const x=nx*scale+model.offset[0],y=ny*scale+model.offset[1];
   vertices.push(effect.x+x*cos+y*sin,z+nz*scale+model.offset[2],effect.y+x*sin-y*cos,...model.uvs[index],alpha);
  }
 }
 return new Float32Array(vertices);
}

export async function loadOriginalBoatWake(gl,{enabled,fetchFile=globalThis.fetch}={}){
 if(!enabled)return null;
 const manifestURL=new URL(manifestPath,import.meta.url);
 async function read(url,sha256){
  const response=await fetchFile(url);
  if(!response.ok)throw Error('Private boat wake resource unavailable');
  const bytes=await response.arrayBuffer();
  const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),b=>b.toString(16).padStart(2,'0')).join('');
  if(hash!==sha256)throw Error('Boat wake resource digest mismatch');
  return bytes;
 }
 try{
  const data=JSON.parse(new TextDecoder().decode(await read(manifestURL,manifestSha256)));
  if(data.format!=='ee2-native-boat-wake-v1'||data.detail!==1)return null;
  const readModel=async (kind,model)=>{
   const {texture}=model;
   const bytes=await read(new URL(texture.url,manifestURL),texture.sha256);
   if(bytes.byteLength!==texture.bytes)throw Error('Boat wake pixel length differs');
   return [kind,model,bytes];
  };
  const media=await Promise.all(Object.entries(data.models).map(([kind,model])=>readModel(kind,model)));
  try{
   const high=JSON.parse(new TextDecoder().decode(await read(new URL('high.json',manifestURL),highManifestSha256)));
   if(high.format!=='ee2-native-boat-wake-high-v1'||high.baseSha256!==manifestSha256)throw Error('Boat wake high profile differs');
   media.push(await readModel('crest',high.model));
   data.models.crest=high.model;
  }catch{/* Earlier private media can still provide the Medium profile. */}
  const before=saveSceneGLState(gl);
  const cleanup=[];
  try{
   const program=makeProgram(gl),textures=new Map(),vao=gl.createVertexArray(),buffer=gl.createBuffer();
   cleanup.push(()=>gl.deleteProgram(program),()=>gl.deleteVertexArray(vao),()=>gl.deleteBuffer(buffer));
   const vp=gl.getUniformLocation(program,'uWakeVP'),sampler=gl.getUniformLocation(program,'uWakeTexture'),alphaTest=gl.getUniformLocation(program,'uWakeAlphaTest');
   gl.activeTexture(gl.TEXTURE0);
   for(const [kind,{texture},bytes] of media){
    const handle=gl.createTexture();textures.set(kind,handle);cleanup.push(()=>gl.deleteTexture(handle));
    gl.bindTexture(gl.TEXTURE_2D,handle);
    for(const [level,{w,h,offset}] of texture.mips.entries())
     gl.texImage2D(gl.TEXTURE_2D,level,gl.RGBA8,w,h,0,gl.RGBA,gl.UNSIGNED_BYTE,new Uint8Array(bytes,offset,w*h*4));
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
    for(const axis of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,axis,gl.REPEAT);
   }
   gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
   for(const [index,size,offset] of [[0,3,0],[1,2,12],[2,1,20]]){
    gl.enableVertexAttribArray(index);gl.vertexAttribPointer(index,size,gl.FLOAT,false,24,offset);
   }
   const frames=createBoatWakeFrames(data);
   return Object.assign(frames,{bodySha256:data.hull.source.sha256,draw(projection,visible){
    const effects=frames.sample(visible);
    if(!effects.length)return;
    const state=saveSceneGLState(gl);
    try{
     gl.useProgram(program);gl.uniformMatrix4fv(vp,false,projection);gl.uniform1i(sampler,0);
     gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.activeTexture(gl.TEXTURE0);
     gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);gl.disable(gl.CULL_FACE);
     gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE);
     for(const [kind,model] of media){
      const vertices=wakeVertices(model,effects.filter(effect=>effect.kind===kind),data.descriptor.z);
      if(!vertices.length)continue;
      gl.bindTexture(gl.TEXTURE_2D,textures.get(kind));gl.uniform1i(alphaTest,model.alphaTest?1:0);
      gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STREAM_DRAW);gl.drawArrays(gl.TRIANGLES,0,vertices.length/6);
     }
    }finally{restoreSceneGLState(gl,state);}
   }});
  }catch(error){cleanup.reverse().forEach(release=>release());throw error;}
  finally{restoreSceneGLState(gl,before);}
 }catch{return null;}
}
