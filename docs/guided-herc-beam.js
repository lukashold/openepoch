import {advanceMiddle,authoredMiddleVertices,middleBasis,middleEnd,MIDDLE_VERTEX,MIDDLE_FRAGMENT} from './guided-herc-beam-math.js?v=herc-beam1';
import {createBeamChildLayer,splashEnd} from './guided-herc-beam-children.js?v=herc-beam-children1';

const f=Math.fround;
const modelDigest='59492da424e81b9d683f20000fcf8c35736629f95490dcb22e6d76c0582b51a3';
const beamDigest='711413a774cb9e90fbef0348d18e3fb55a8c91d75fb1043eee8e6f8707f8a236';
const manifestDigest='7c2947b843193a0fed1c8d6b279fc2832f6d2974f7f406007818e0aff5fc7b00';
const manifestURL=new URL('./local-original/herc-beam/manifest.json',import.meta.url);
const childURL=new URL('./local-original/herc-beam-children/manifest.json',import.meta.url);
const childDigest='60e8e3ca7213d3d8f8400847f08ba1482ae3ccacf85a9c01e5e6eb22ed010a03';

function program(gl,vertex=MIDDLE_VERTEX,fragment=MIDDLE_FRAGMENT){
 const compiled=[];
 const result=gl.createProgram();
 try{
  for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
   const shader=gl.createShader(type);compiled.push(shader);
   gl.shaderSource(shader,source);gl.compileShader(shader);
   if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
   gl.attachShader(result,shader);
  }
  gl.linkProgram(result);
  if(!gl.getProgramParameter(result,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(result));
  return result;
 }catch(error){gl.deleteProgram(result);throw error;}
 finally{for(const shader of compiled)gl.deleteShader(shader);}
}

async function fetchBytes(url){
 const response=await fetch(url);
 if(!response.ok)throw Error('HERC beam asset unavailable');
 return new Uint8Array(await response.arrayBuffer());
}

async function sha(bytes){
 return [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(value=>value.toString(16).padStart(2,'0')).join('');
}

function setupTexture(gl,mips,bytes){
 const texture=gl.createTexture(),previous=gl.getParameter(gl.TEXTURE_BINDING_2D);
 gl.bindTexture(gl.TEXTURE_2D,texture);
 for(const [i,{width,height,offset,bytes:length}] of mips.levels.entries()){
  gl.texImage2D(gl.TEXTURE_2D,i,gl.RGBA8,width,height,0,gl.RGBA,gl.UNSIGNED_BYTE,bytes.subarray(offset,offset+length));
 }
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.REPEAT);
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.REPEAT);
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);
 gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
 gl.bindTexture(gl.TEXTURE_2D,previous);
 return texture;
}

function socketFor(actor,phase,samples,worldScale){
 const at=Math.max(0,Math.min(1,phase))*(samples.length-1),frame=Math.floor(at),alpha=at-frame;
 const first=samples[frame],last=samples[Math.min(frame+1,samples.length-1)];
 const m=first.map((row,i)=>row.map((value,j)=>value+(last[i][j]-value)*alpha));
 const c=Math.cos(actor.renderYaw),s=Math.sin(actor.renderYaw);
 const transform=([x,y,z])=>[-y*c-x*s,-y*s+x*c,z];
 const origin=transform(m.map(row=>row[3]));
 const columns=[0,1,2].map(col=>transform(m.map(row=>row[col])));
 const tagScale=Math.hypot(...columns[0]);
 if(!(tagScale>0))return null;
 return {point:[actor.renderX,actor.renderY,actor.renderAltitude].map((value,i)=>f(value+worldScale*origin[i])),
  axes:columns.map(column=>column.map(value=>value/tagScale)),scale:worldScale*tagScale};
}

async function loadChildren(gl,model,detail){
 if(!detail)return null;
 const textures=new Map();
 try{
  const raw=await fetchBytes(childURL);
  if(await sha(raw)!==childDigest)throw Error('HERC beam children manifest mismatch');
  const data=JSON.parse(new TextDecoder().decode(raw));
  if(data.format!=='openepoch-local-beam-children-v1'||
     ['muzzle','splash'].some(key=>data.children[key]?.source.sha256!==model.otherChildren[key]?.source.sha256))throw Error('HERC beam children source mismatch');
  const bytes=await fetchBytes(new URL(data.pixels.url,childURL));
  if(bytes.length!==data.pixels.bytes||await sha(bytes)!==data.pixels.sha256)throw Error('HERC beam children texture mismatch');
  for(const [name,texture] of Object.entries(data.textures))textures.set(name,setupTexture(gl,texture,bytes));
  return createBeamChildLayer(gl,data.children,textures,program,detail);
 }catch{
  for(const texture of textures.values())gl.deleteTexture(texture);
  return null;
 }
}

function binding(gl,model,texture,minotaurus,children){
 const p=program(gl),uVP=gl.getUniformLocation(p,'uVP'),vao=gl.createVertexArray();
 const vbo=gl.createBuffer(),indices=gl.createBuffer(),indexCount=model.mesh.indices.length;
 const previousVAO=gl.getParameter(gl.VERTEX_ARRAY_BINDING),previousBuffer=gl.getParameter(gl.ARRAY_BUFFER_BINDING);
 gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vbo);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);
 gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint16Array(model.mesh.indices),gl.STATIC_DRAW);
 for(const [location,count,offset] of [[0,3,0],[1,2,12],[2,1,20]]){
  gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,count,gl.FLOAT,false,24,offset);
 }
 gl.bindVertexArray(previousVAO);gl.bindBuffer(gl.ARRAY_BUFFER,previousBuffer);
 const pending=new Map(),active=new Map();
 let last=null,delta=0,lastChildDraws=null;
 function enqueue(wasm,count){
  const buffer=wasm.memory.buffer,ptr=wasm.guided_herc_beam_ptr()>>>0;
  if(count>64||ptr+count*24>buffer.byteLength)return;
  const rows=new DataView(buffer);
  for(let i=0;i<count;i++){
   const at=ptr+i*24,source=rows.getUint32(at,true),target=rows.getUint32(at+4,true);
   const point=[8,12,16].map(offset=>rows.getFloat32(at+offset,true));
   const targetOffsetZ=rows.getFloat32(at+20,true);
   if(point.every(Number.isFinite)&&Number.isFinite(targetOffsetZ))pending.set(source,{target,point,targetOffsetZ});
  }
 }
 function reset(seconds){active.clear();pending.clear();last=seconds;delta=0;lastChildDraws=null;}
 function advance(seconds){
  if(last===null||seconds<last)reset(seconds);
  delta=f(seconds-last);last=seconds;
  if(!(delta>0))return;
  for(const [source,beam] of active){
   beam.previousAge=beam.age;beam.age=f(beam.age+delta);beam.step=delta;beam.stepped=false;
   if(beam.age>=(children?splashEnd:middleEnd))active.delete(source);
  }
  for(const [source,row] of pending)active.set(source,{...row,age:delta,previousAge:0,step:delta,reached:0,stepped:false,
   children:children?.create(Math.imul(source,16777619)^row.target^Math.round(seconds*65536))});
  pending.clear();
 }
 function capture(muzzles,actor,clip,phase){
  if(actor.def!==minotaurus||!active.has(actor.handle))return;
  const samples=model.socket.clips[clip]?.matrixRows;
  if(!samples)return;
  const socket=socketFor(actor,phase,samples,model.sourceScale);
  if(socket)muzzles.set(actor.handle,socket);
 }
 function draw(vp,muzzles,metadata,view){
  lastChildDraws=children?{muzzle:0,splash:0,particles:0}:null;
  if(!muzzles?.size)return 0;
  const meshes=[],effects=[];
  for(const [source,muzzle] of muzzles){
   const beam=active.get(source);
   if(!beam)continue;
   const current=metadata?.forHandle(beam.target);
   if(current?.hpRatio>0)beam.point=[current.renderX,current.renderY,f(current.renderAltitude+beam.targetOffsetZ)];
   const endpoint=beam.point;
   const distance=f(Math.hypot(...endpoint.map((value,i)=>value-muzzle.point[i])));
   const frame=advanceMiddle({age:beam.stepped?beam.age:beam.previousAge,reached:beam.reached},beam.stepped?0:beam.step,distance);
   if(!beam.stepped){beam.stepped=true;beam.reached=frame.reached;}
   if(beam.children&&view){
    children.update(beam.children,beam.age,distance>0&&frame.reached>=distance);
    effects.push({state:beam.children,muzzle,point:endpoint});
   }
   const axes=frame.draw?middleBasis(muzzle.axes,muzzle.point,endpoint):null;
   if(axes)meshes.push(authoredMiddleVertices(model.mesh,frame,muzzle.point,axes,muzzle.scale));
  }
  if(effects.length)lastChildDraws=children.draw(view,effects);
  if(!meshes.length)return 0;
  const previous={program:gl.getParameter(gl.CURRENT_PROGRAM),vao:gl.getParameter(gl.VERTEX_ARRAY_BINDING),
   buffer:gl.getParameter(gl.ARRAY_BUFFER_BINDING),active:gl.getParameter(gl.ACTIVE_TEXTURE),
   depth:gl.isEnabled(gl.DEPTH_TEST),cull:gl.isEnabled(gl.CULL_FACE),blend:gl.isEnabled(gl.BLEND),
   write:gl.getParameter(gl.DEPTH_WRITEMASK),test:gl.getParameter(gl.DEPTH_FUNC),mode:gl.getParameter(gl.CULL_FACE_MODE),
   equation:[gl.getParameter(gl.BLEND_EQUATION_RGB),gl.getParameter(gl.BLEND_EQUATION_ALPHA)],
   mix:[gl.BLEND_SRC_RGB,gl.BLEND_DST_RGB,gl.BLEND_SRC_ALPHA,gl.BLEND_DST_ALPHA].map(value=>gl.getParameter(value))};
  gl.activeTexture(gl.TEXTURE0);const oldTexture=gl.getParameter(gl.TEXTURE_BINDING_2D);
  try{
   gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);
   gl.enable(gl.CULL_FACE);gl.cullFace(gl.BACK);
   gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);
   gl.useProgram(p);gl.uniformMatrix4fv(uVP,false,vp);gl.bindTexture(gl.TEXTURE_2D,texture);
   gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vbo);
   for(const vertices of meshes){
    gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STREAM_DRAW);
    gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_SHORT,0);
   }
  }finally{
   gl.bindTexture(gl.TEXTURE_2D,oldTexture);gl.activeTexture(previous.active);
   gl.bindVertexArray(previous.vao);gl.bindBuffer(gl.ARRAY_BUFFER,previous.buffer);gl.useProgram(previous.program);
   gl.depthMask(previous.write);gl.depthFunc(previous.test);gl.cullFace(previous.mode);
   gl.blendEquationSeparate(...previous.equation);gl.blendFuncSeparate(...previous.mix);
   for(const [flag,enabled] of [[gl.DEPTH_TEST,previous.depth],[gl.CULL_FACE,previous.cull],[gl.BLEND,previous.blend]]){
    if(enabled)gl.enable(flag);else gl.disable(flag);
   }
  }
  return meshes.length;
 }
 return {enqueue,reset,advance,capture,draw,get childDraws(){return lastChildDraws;}};
}

export async function loadGuidedHercBeam(gl,enabled,minotaurus,particleDetail=2){
 if(!enabled)return null;
 try{
  const raw=await fetchBytes(manifestURL);
  if(await sha(raw)!==manifestDigest)throw Error('HERC beam manifest mismatch');
  const model=JSON.parse(new TextDecoder().decode(raw));
  if(model.format!=='openepoch-local-herc-beam-v1'||model.sourceModel?.sha256!==modelDigest||
    model.meshSource?.sha256!==beamDigest||!(model.sourceScale>0)||model.socket?.name!=='tag_muzzle_1'||
    model.mesh?.positions.length!==8||model.mesh?.indices.length!==24)throw Error('HERC beam source mismatch');
  const mips=model.texture.mips;
  const bytes=await fetchBytes(new URL('laser-linear-mips.rgba',manifestURL));
  if(await sha(bytes)!==mips.sha256||bytes.length!==mips.bytes||mips.levels.length!==8)throw Error('HERC beam texture mismatch');
  const children=await loadChildren(gl,model,particleDetail);
  return binding(gl,model,setupTexture(gl,mips,bytes),minotaurus,children);
 }catch{return null;}
}
