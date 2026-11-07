import {createNativeCloth,nativeInverseTagRotation,nativeLocalForces,nativeRotateForce} from './source-cloth-kernel.js';
import {nativeClothVertexColors} from './source-cloth-lighting.js';
import {dynamicSailVertices,nativeSailTagRotation} from './original-dynamic-sails.js';
import {add,store32} from './source-cloth-arithmetic.js';

const none=0xffffffff, stride=56, limit=8192;
const directory='local-original/air-mission-flag/';
const manifestHash='b42c8f9ec28136114b2e7f4a7f977ef1bade369b5da6e89455be02f0aaa02a18';
const spacing=.5, pathWidth=.0625, pathRise=Math.fround(.1);
const axes=[0,1,2];

// 006D3C10 starts Normal at duration zero. The visual LCG uses its own
// 00C9BF0C stream and constants, independently of the game WASM seed.
export function nativeNormalFlagWind(seed){
  let state=seed>>>0;
  const word=()=>((state=(Math.imul(state,0x91b1022c)+0x323c3a29)>>>0)>>>16)&0x7fff;
  const first=word(), second=word(), initial=first*0.00019175345369149;
  const angle=Math.fround(initial>Math.fround(Math.PI)?initial-Math.fround(2*Math.PI):initial);
  const lower=Math.fround(.2), upper=Math.fround(.3);
  const strength=Math.fround(lower+(upper-lower)*second*0.000030518509447574615);
  return [Math.fround(Math.cos(angle))*strength,Math.fround(Math.sin(angle))*strength];
}

export function readAirMissionVisualRoutes(wasm) {
  if(typeof wasm.air_mission_visual_routes!=='function'||typeof wasm.air_mission_visual_routes_ptr!=='function')return [];
  const count=wasm.air_mission_visual_routes()>>>0;
  if(!count||count>limit)return [];
  const offset=wasm.air_mission_visual_routes_ptr()>>>0, buffer=wasm.memory.buffer;
  if(offset%4||offset+count*stride>buffer.byteLength)return [];
  const data=new DataView(buffer,offset,count*stride), routes=[];
  for(let index=0;index<count;index++){
    const at=index*stride, host=data.getUint32(at,true), target=data.getUint32(at+4,true);
    const length=data.getUint32(at+8,true), team=data.getUint32(at+12,true);
    if(host===none||length>5||team>=4)return [];
    const points=Array.from({length},(_,i)=>[data.getFloat32(at+16+i*8,true),data.getFloat32(at+20+i*8,true)]);
    if(!points.flat().every(value=>Number.isFinite(value)&&value>=0&&value<256))return [];
    routes.push({host,target,points});
  }
  return routes;
}

// 00657270 inserts the owning visual first. Only the final point receives the
// flag; a live terminal actor can supply its authored rally-pole attachment.
export function projectAirMissionFlags(records,draft,{actorAt,attachmentAt,hostAt,targetAt,groundAt}) {
  const routes=[];
  for(const record of records){
    const {host}=record, {points,target}=draft??record;
    const owner=actorAt(host), origin=owner?[owner.renderX,owner.renderAltitude,owner.renderY]:hostAt(host);
    if(!origin)continue;
    const path=[origin,...points.map(([x,z])=>[x,groundAt(x,z),z])];
    if(target!==none){
      const actor=actorAt(target);
      const terminal=actor?(actor.hpRatio>0&&(attachmentAt(actor)??[actor.renderX,actor.renderAltitude,actor.renderY])):targetAt(target);
      if(terminal)path.push(terminal);
    }
    if(path.length>1)routes.push({host,path,flag:path.at(-1)});
  }
  return routes;
}

// Native ground projection samples the route before creating VisualThickLine.
// This adapter follows the existing browser terrain triangles at the same
// half-unit interval and retains the exact independent native ribbon width.
export function airMissionRibbon(path,groundAt){
  const vertices=[];
  const emit=(x,y,z,u,v)=>vertices.push(x,y,z,u,v,1,1,1,1);
  let along=0;
  for(let index=1;index<path.length;index++){
    const a=path[index-1], b=path[index], dx=b[0]-a[0], dz=b[2]-a[2], length=Math.hypot(dx,dz);
    if(length<1e-6)continue;
    const nx=-dz/length*pathWidth*.5,nz=dx/length*pathWidth*.5;
    function side(distance,sign){
      const x=a[0]+dx*distance/length,z=a[2]+dz*distance/length;
      return [x+nx*sign,groundAt(x,z)+pathRise,z+nz*sign,sign<0?0:1,along+distance];
    }
    let left=side(0,-1), right=side(0,1);
    for(let distance=Math.min(spacing,length);;distance=Math.min(distance+spacing,length)){
      const nextLeft=side(distance,-1),nextRight=side(distance,1);
      emit(...left);emit(...right);emit(...nextLeft);emit(...nextLeft);emit(...right);emit(...nextRight);
      if(distance===length)break;
      left=nextLeft;right=nextRight;
    }
    along+=length;
  }
  return new Float32Array(vertices);
}

const vertex=`#version 300 es
precision highp float;
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in vec4 aColor;
uniform mat4 uVP;
uniform vec3 uOrigin;
out vec2 vUV;
out vec4 vColor;
void main(){
  vUV=aUV;
  vColor=aColor;
  gl_Position=uVP*vec4(aPosition+uOrigin,1.);
}`;
const fragment=`#version 300 es
precision highp float;
uniform sampler2D uBase,uDark;
uniform bool uFlag;
in vec2 vUV;
in vec4 vColor;
out vec4 outColor;
void main(){
  vec4 base=texture(uBase,vUV);
  if(uFlag){
    vec4 dark=texture(uDark,vUV);
    outColor=vec4(clamp(base.rgb*vColor.rgb*4.,0.,1.)*dark.rgb,base.a*dark.a);
  }else{
    outColor=base*vColor;
  }
  if(outColor.a<=0.)discard;
}`;

function program(gl){
  const handle=gl.createProgram();
  for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
    const shader=gl.createShader(type);
    gl.shaderSource(shader,source);gl.compileShader(shader);
    if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
    gl.attachShader(handle,shader);gl.deleteShader(shader);
  }
  gl.linkProgram(handle);
  if(!gl.getProgramParameter(handle,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(handle));
  return {handle,u:Object.fromEntries(['uVP','uOrigin','uBase','uDark','uFlag'].map(key=>[key,gl.getUniformLocation(handle,key)]))};
}

function state(gl){
  const keys=['ACTIVE_TEXTURE','CURRENT_PROGRAM','VERTEX_ARRAY_BINDING','ARRAY_BUFFER_BINDING','DEPTH_WRITEMASK',
    'DEPTH_FUNC','BLEND_SRC_RGB','BLEND_DST_RGB','BLEND_SRC_ALPHA','BLEND_DST_ALPHA','BLEND_EQUATION_RGB',
    'BLEND_EQUATION_ALPHA','UNPACK_FLIP_Y_WEBGL'];
  const values=Object.fromEntries(keys.map(key=>[key,gl.getParameter(gl[key])]));
  const enabled=['DEPTH_TEST','BLEND','CULL_FACE'].map(key=>[gl[key],gl.isEnabled(gl[key])]);
  const bindings=[0,7].map(unit=>{gl.activeTexture(gl.TEXTURE0+unit);return [unit,gl.getParameter(gl.TEXTURE_BINDING_2D)];});
  return ()=>{
    for(const [unit,texture] of bindings){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,texture);}
    gl.activeTexture(values.ACTIVE_TEXTURE);gl.useProgram(values.CURRENT_PROGRAM);gl.bindVertexArray(values.VERTEX_ARRAY_BINDING);
    gl.bindBuffer(gl.ARRAY_BUFFER,values.ARRAY_BUFFER_BINDING);
    for(const [capability,on] of enabled)if(on)gl.enable(capability);else gl.disable(capability);
    gl.depthMask(values.DEPTH_WRITEMASK);gl.depthFunc(values.DEPTH_FUNC);
    gl.blendFuncSeparate(values.BLEND_SRC_RGB,values.BLEND_DST_RGB,values.BLEND_SRC_ALPHA,values.BLEND_DST_ALPHA);
    gl.blendEquationSeparate(values.BLEND_EQUATION_RGB,values.BLEND_EQUATION_ALPHA);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,values.UNPACK_FLIP_Y_WEBGL);
  };
}

function uploadTexture(gl,image,repeat){
  const texture=gl.createTexture();
  gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,repeat?gl.REPEAT:gl.CLAMP_TO_EDGE);
  return texture;
}

function createMesh(gl,indices){
  const vao=gl.createVertexArray(), buffer=gl.createBuffer();
  gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
  for(const [loc,size,offset] of [[0,3,0],[1,2,12],[2,4,20]]){
    gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,size,gl.FLOAT,false,36,offset);
  }
  if(indices){
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,gl.createBuffer());
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,Uint16Array.from(indices),gl.STATIC_DRAW);
  }
  return {vao,buffer,count:indices?.length??0};
}

function flagMedia(base,fetchFile,decode){
  const read=async(path,hash)=>{
    const url=new URL(path,base);url.searchParams.set('v',hash.slice(0,16));
    const response=await fetchFile(url);
    if(!response.ok)throw Error('Air mission flag media unavailable');
    const bytes=await response.arrayBuffer();
    const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
    if(Array.from(digest,v=>v.toString(16).padStart(2,'0')).join('')!==hash)throw Error('Air mission flag media checksum differs');
    return bytes;
  };
  const image=({url,sha256})=>read(url,sha256).then(value=>decode(new Blob([value],{type:'image/png'}),{premultiplyAlpha:'none',colorSpaceConversion:'none'}));
  return {read,image};
}

export async function loadAirMissionFlag(gl,{enabled,fetchFile=globalThis.fetch,decode=globalThis.createImageBitmap,
  base=new URL('./',import.meta.url),objectURL=URL.createObjectURL.bind(URL)}={}){
  if(!enabled)return null;
  const {read,image}=flagMedia(base,fetchFile,decode);
  try{
    const manifest=JSON.parse(new TextDecoder().decode(await read(directory+'manifest.json',manifestHash)));
    if(manifest.format!=='ee2-air-mission-flag-v1')return null;
    const {invalid,...records}=manifest.textures;
    const bytes=record=>read(record.url,record.sha256);
    const [pole,clothBase,dark,ribbon,cursor]=await Promise.all([
      image(records.pole),image(records.base),image(records.dark),image(records.ribbon),bytes(invalid)
    ]);
    const restore=state(gl);
    try{
      const textures={pole:uploadTexture(gl,pole,false),base:uploadTexture(gl,clothBase,false),
        dark:uploadTexture(gl,dark,false),ribbon:uploadTexture(gl,ribbon,true)};
      return createAirMissionFlagRenderer(gl,manifest,textures,objectURL(new Blob([cursor],{type:'image/png'})));
    }finally{
      for(const bitmap of [pole,clothBase,dark,ribbon])bitmap.close();
      restore();
    }
  }catch{
    return null;
  }
}

function drawFlagMeshes(gl,shader,vp,dark,draw){
  const restore=state(gl), u=shader.u;
  try{
    gl.useProgram(shader.handle);gl.uniformMatrix4fv(u.uVP,false,vp);
    gl.uniform1i(u.uBase,0);gl.uniform1i(u.uDark,7);
    gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.disable(gl.CULL_FACE);
    gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);
    gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,dark);
    draw((part,data,texture,origin,flag)=>{
      gl.uniform1i(u.uFlag,flag?1:0);gl.uniform3fv(u.uOrigin,origin);
      gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
      gl.bindVertexArray(part.vao);gl.bindBuffer(gl.ARRAY_BUFFER,part.buffer);gl.bufferData(gl.ARRAY_BUFFER,data,gl.DYNAMIC_DRAW);
      if(part.count)gl.drawElementsInstanced(gl.TRIANGLES,part.count,gl.UNSIGNED_SHORT,0,1);
      else gl.drawArrays(gl.TRIANGLES,0,data.length/9);
    });
  }finally{restore();}
}

function createAirMissionFlagRenderer(gl,manifest,textures,cursor){
  const {scale,pole,cloth,targetTags}=manifest, shader=program(gl);
  const poleMesh=createMesh(gl,pole.indices),clothMesh=createMesh(gl,cloth.indices),lineMesh=createMesh(gl);
  const rotation=cloth.tag.slice(0,3).flatMap(row=>row.slice(0,3).map(Math.fround));
  const inverse=nativeInverseTagRotation(rotation);
  const clothOwners=new Map(), lineCache=new Map();
  const poleData=new Float32Array(pole.positions.length/3*9);
  let frame=0, defaultWind=nativeNormalFlagWind(Math.floor(Date.now()/1000));
  function prepare(routes,{deltaSeconds,runPhysics,lighting,wind=defaultWind}){
    frame++;
    if(!routes.length){clear();return true;}
    const light={ambient:lighting.sourceAmbient,diffuse:lighting.sourceDiffuse,sunDirection:lighting.sourceDirection};
    if(!Object.values(light).every(row=>row?.length===3))return false;
    const direction=lighting.direction??[0,1,0];
    for(let i=0;i<poleData.length/9;i++){
      const normal=pole.normals.slice(i*3,i*3+3);
      const dot=Math.max(0,axes.reduce((sum,k)=>sum+normal[k]*direction[k],0));
      const rgb=axes.map(k=>Math.min(1,light.ambient[k]+dot*light.diffuse[k]));
      poleData.set([...pole.positions.slice(i*3,i*3+3).map(value=>value*scale),...pole.uvs.slice(i*2,i*2+2),...rgb,1],i*9);
    }
    // The current game uses a separately sampled Normal-start wind. An
    // explicit direction multiplied by live force can replace it per frame.
    const low=[Math.fround(wind[0]*200),Math.fround(wind[1]*200),0];
    const forces=nativeLocalForces({gravity:[0,0,1],windMin:low,windMax:low.map(value=>Math.fround(value*4))},inverse,{rounding:'chop53'});
    for(const route of routes){
      let owner=clothOwners.get(route.host);
      if(!owner){owner={cloth:createNativeCloth(cloth)};clothOwners.set(route.host,owner);}
      if(runPhysics)owner.cloth.step(deltaSeconds,forces);
      const positions=owner.cloth.positions, colors=nativeClothVertexColors({...cloth,positions},light).colors;
      const packed=dynamicSailVertices(cloth,{positions,colors},{scale,offset:[0,0,0]});
      const data=owner.data??new Float32Array(cloth.nx*cloth.ny*9);
      for(let i=0;i<cloth.nx*cloth.ny;i++)data.set([...packed.subarray(i*12,i*12+3),...packed.subarray(i*12+6,i*12+12)],i*9);
      owner.data=data;owner.frame=frame;
    }
    for(const [host,owner] of clothOwners)if(owner.frame!==frame){clothOwners.delete(host);lineCache.delete(host);}
    return true;
  }
  function draw({vp,routes,groundAt}){
    if(!routes.length)return;
    drawFlagMeshes(gl,shader,vp,textures.dark,mesh=>{
      gl.depthMask(false);
      for(const route of routes){
        const key=route.path.flat().join(',');
        let line=lineCache.get(route.host);
        if(line?.key!==key){line={key,data:airMissionRibbon(route.path,groundAt)};lineCache.set(route.host,line);}
        if(line.data.length)mesh(lineMesh,line.data,textures.ribbon,[0,0,0],false);
      }
      gl.depthMask(true);
      for(const route of routes){
        const owner=clothOwners.get(route.host);
        if(owner?.frame!==frame)continue;
        mesh(poleMesh,poleData,textures.pole,route.flag,false);
        mesh(clothMesh,owner.data,textures.base,route.flag,true);
      }
    });
  }
  function clear(){clothOwners.clear();lineCache.clear();frame=0;}
  function reset(seed=Math.floor(Date.now()/1000)){clear();defaultWind=nativeNormalFlagWind(seed);}
  return {prepare,draw,reset,targetTags,normalWind:()=>defaultWind,invalidCursor:`url("${cursor}") 16 16, not-allowed`};
}

// 007E7B00 copies the socket's raw translation. The GarrisonShip child keeps
// local scale 1 and inherits the hull basis and scale in 00609403.
function carrierFlagPose(actor,row,basis){
  const scale=row.model.nativeScaleBinding.sourceScale;
  const [x,y,z]=row.bodyEvidence.markers.tag_garrison_pole.point;
  if(basis){
    const [a,b,c,d,e,f,g,h,i]=basis;
    const rotate=(x,y,z)=>[a*x+d*y+g*z,b*x+e*y+h*z,c*x+f*y+i*z];
    // 00609444 rotates the socket before 005F9C00 scales and stores all
    // three components. 005F7850 then adds the parent world translation.
    const local=[add(add(d*z,-g*y),a*x),add(add(e*z,b*x),-h*y),add(add(f*z,c*x),-i*y)];
    const parent=[actor.renderX,actor.renderAltitude,actor.renderY];
    const origin=local.map((value,axis)=>store32(add(parent[axis],store32(store32(value)*scale))));
    return {origin,scale,rotate};
  }
  const yaw=Math.fround(actor.renderYaw+row.model.yawOffset);
  const c=Math.cos(yaw),s=Math.sin(yaw);
  const rotate=(x,y,z)=>[x*c-z*s,y,x*s+z*c];
  return {origin:[actor.renderX+(x*c+y*s)*scale,actor.renderAltitude+z*scale,actor.renderY+(x*s-y*c)*scale],scale,rotate,c,s};
}

export async function loadCarrierGarrisonFlag(gl,{source,navalBasisForActor,fetchFile=globalThis.fetch,decode=globalThis.createImageBitmap,
  base=new URL('./',import.meta.url)}={}){
  if(!source)return null;
  const {read,image}=flagMedia(base,fetchFile,decode);
  try{
    const data=JSON.parse(new TextDecoder().decode(await read(source.url,source.sha256)));
    if(data.format!=='ee2-native-garrison-flag-source-v1'||data.name!=='GarrisonShip')return null;
    const records=[data.pole.texture,data.textures.base,data.textures.dark];
    const images=await Promise.all(records.map(image)),restore=state(gl);
    try{
      const textures=images.map(bitmap=>uploadTexture(gl,bitmap,false));
      return createCarrierGarrisonRenderer(gl,data,textures,source.carrierAt,navalBasisForActor);
    }finally{
      for(const bitmap of images)bitmap.close();
      restore();
    }
  }catch{return null;}
}

function createCarrierGarrisonRenderer(gl,{pole,cloth:grid,flagTag},[poleTexture,base,dark],carrierAt,navalBasisForActor){
  const cloth={...grid,tag:flagTag.matrix};
  const shader=program(gl),poleMesh=createMesh(gl,pole.indices),clothMesh=createMesh(gl,cloth.indices);
  const owners=new Map(), fallbackWind=nativeNormalFlagWind(Math.floor(Date.now()/1000));
  let frame=0, session=null, settings;
  function begin(next){
    frame++;
    if(next.session!==session){owners.clear();session=next.session;}
    settings=next;
    for(const handle of owners.keys())if(!next.occupied(handle))owners.delete(handle);
  }
  function geometry({scale,rotate,c,s},basis,owner){
    const {lighting,runPhysics,deltaSeconds,wind=fallbackWind}=settings;
    const m=cloth.tag, rotation=basis?nativeSailTagRotation(m,basis,'chop53'):[...m[0].slice(0,3).map((v,i)=>Math.fround(c*v+s*m[1][i])),
      ...m[0].slice(0,3).map((v,i)=>Math.fround(-s*v+c*m[1][i])),...m[2].slice(0,3).map(Math.fround)];
    const inverse=nativeInverseTagRotation(rotation), direction=lighting.direction??[0,1,0];
    const low=[Math.fround(wind[0]*200),Math.fround(wind[1]*200),0];
    const forces=nativeLocalForces({gravity:[0,0,1],windMin:low,windMax:low.map(value=>Math.fround(value*4))},inverse,{rounding:'chop53'});
    if(runPhysics)owner.cloth.step(deltaSeconds,forces);
    const light={ambient:lighting.sourceAmbient,diffuse:lighting.sourceDiffuse,
      sunDirection:nativeRotateForce(inverse,lighting.sourceDirection)};
    const positions=owner.cloth.positions, colors=nativeClothVertexColors({...cloth,positions},light).colors;
    const packed=dynamicSailVertices(cloth,{positions,colors},{scale,offset:[0,0,0]});
    for(let i=0;i<cloth.nx*cloth.ny;i++)
      owner.flag.set([...rotate(...packed.subarray(i*12,i*12+3)),...packed.subarray(i*12+6,i*12+12)],i*9);
    for(let i=0;i<pole.positions.length/3;i++){
      const normal=rotate(...pole.normals.slice(i*3,i*3+3));
      const dot=Math.max(0,axes.reduce((sum,k)=>sum+normal[k]*direction[k],0));
      const rgb=axes.map(k=>Math.min(1,light.ambient[k]+dot*light.diffuse[k]));
      owner.pole.set([...rotate(...pole.positions.slice(i*3,i*3+3).map(value=>value*scale)),...pole.uvs.slice(i*2,i*2+2),...rgb,1],i*9);
    }
    owner.frame=frame;
  }
  function draw({vp,actors,epochs,local}){
    if(!settings||local===undefined||!actors?.length)return;
    const {lighting,occupied}=settings;
    if(![lighting.sourceAmbient,lighting.sourceDiffuse,lighting.sourceDirection].every(row=>row?.length===3))return;
    const projected=[];
    for(const actor of actors){
      if(actor.def!==84||actor.team!==local||!(actor.hpRatio>0)||!occupied(actor.handle))continue;
      const row=carrierAt(epochs[actor.team]);
      if(!row)continue;
      let owner=owners.get(actor.handle);
      if(!owner){
        owner={cloth:createNativeCloth(cloth),pole:new Float32Array(pole.positions.length*3),flag:new Float32Array(cloth.nx*cloth.ny*9)};
        owners.set(actor.handle,owner);
      }
      const basis=navalBasisForActor?.(actor);
      const pose=carrierFlagPose(actor,row,basis);
      if(owner.frame!==frame)geometry(pose,basis,owner);
      projected.push({owner,origin:pose.origin});
    }
    if(projected.length)drawFlagMeshes(gl,shader,vp,dark,mesh=>{
      gl.depthMask(true);
      for(const {owner,origin} of projected){
        mesh(poleMesh,owner.pole,poleTexture,origin,false);
        mesh(clothMesh,owner.flag,base,origin,true);
      }
    });
  }
  return {begin,draw};
}
