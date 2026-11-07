import {nativeMotionWorldForces,nativeInverseTagRotation,nativeLocalForces} from './source-cloth-kernel.js';
import {CHOP53,NEAREST53} from './source-cloth-arithmetic.js';

// Source grids, tag transforms and parent normalization remain separate. The
// renderer never substitutes the rigid NIF mesh for a cloth grid.
const same=(a,b)=>a?.length===b?.length&&Array.from(a).every((v,i)=>v===b[i]);
const finite=a=>a&&Array.from(a).every(Number.isFinite);
const bounds=positions=>{const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];for(let i=0;i<positions.length;i++){min[i%3]=Math.min(min[i%3],positions[i]);max[i%3]=Math.max(max[i%3],positions[i]);}return {min,max};};
const union=(a,b)=>({min:a.min.map((v,i)=>Math.min(v,b.min[i])),max:a.max.map((v,i)=>Math.max(v,b.max[i]))});

export function createDynamicSailBinding(parent,composite,manifest,yawOffset=0){
 if(parent.source?.sha256!==composite.source?.sha256)throw Error('Dynamic sail parent source hash differs');
 const n=parent.normalization;
 if(!n||n.axis!=='x,z,-y'||!Number.isFinite(n.scale)||n.scale<=0||n.offset?.length!==3||!finite(n.offset)||!Number.isFinite(yawOffset))throw Error('Dynamic sail parent normalization is invalid');
 const effect=composite.effect,rows=effect?manifest.effects?.[effect]?.sails:[];
 if(!Array.isArray(rows)||rows.length!==composite.parts?.length)throw Error('Dynamic sail grid/attachment count differs');
 const planes=rows.map((row,i)=>{
  const part=composite.parts[i],g=row.grid,tag=part.sourceTag?.world;
  if(!g||!Number.isInteger(g.nx)||!Number.isInteger(g.ny)||g.nx<2||g.ny<2||g.positions?.length!==g.nx*g.ny*3||!same(g.positions,part.sourcePlanePositions)||!same(g.uvs,part.uvs)||!same(g.indices,part.indices)||!same(row.pinned,part.pinned)||g.positions.some(v=>!Number.isFinite(v)||Math.fround(v)!==v)||!Array.isArray(tag)||tag.length!==4||tag.some(r=>r.length!==4||!finite(r))||!same(tag[3],[0,0,0,1])||part.damageMaskUrls?.length!==9)throw Error('Dynamic sail source grid or tag differs');
  return Object.freeze({nx:g.nx,ny:g.ny,positions:Object.freeze([...g.positions]),pinned:Object.freeze([...part.pinned]),uvs:Object.freeze([...g.uvs]),indices:Object.freeze([...g.indices]),tag:Object.freeze(tag.map(r=>Object.freeze([...r]))),name:part.name,textureUrl:part.textureUrl,damageMaskUrls:Object.freeze([...part.damageMaskUrls])});
 });
 let initialBounds=null;for(const part of parent.parts){if(part.positions?.length){const b=bounds(part.positions);initialBounds=initialBounds?union(initialBounds,b):b;}}
 return Object.freeze({key:parent.source.sha256+':'+(effect||'none'),sourceSha256:parent.source.sha256,effect:effect||null,planes:Object.freeze(planes),normalization:Object.freeze({scale:n.scale,offset:Object.freeze([...n.offset])}),yawOffset,initialBounds});
}

export function dynamicSailVertices(plane,shading,normalization){
 const positions=shading?.positions,colors=shading?.colors,n=plane.nx*plane.ny;
 if(positions?.length!==n*3||colors?.length!==n*4||!finite(positions)||!finite(colors))throw Error('Dynamic sail shading is missing or invalid');
 const packed=new Float32Array(n*12),m=plane.tag,{scale,offset}=normalization;
 for(let i=0;i<n;i++){
  const x=positions[i*3],y=positions[i*3+1],z=positions[i*3+2];
  const sx=m[0][0]*x+m[0][1]*y+m[0][2]*z+m[0][3],sy=m[1][0]*x+m[1][1]*y+m[1][2]*z+m[1][3],sz=m[2][0]*x+m[2][1]*y+m[2][2]*z+m[2][3];
  packed.set([(sx+offset[0])*scale,(sz+offset[1])*scale,(-sy+offset[2])*scale,0,0,1,plane.uvs[i*2],plane.uvs[i*2+1],...colors.subarray(i*4,i*4+4)],i*12);
 }
 if(!finite(packed))throw Error('Dynamic sail GPU vertex overflow');return packed;
}

// The uploaded body basis is column-major in X/Z/-Y. Convert it back to the
// native Z-up rows, then compose the authored model-space tag rotation.
export function nativeSailTagRotation(tag,basis,rounding){
 const [a,b,c,d,e,f,g,h,i]=basis;
 const parent=[a,-g,d,-c,i,-f,b,-h,e];
 const order=[[1,0,2],[2,0,1],[2,0,1],[2,1,0],[2,1,0],[2,1,0],[2,1,0],[0,2,1],[0,2,1]];
 const {store32,add}=rounding==='chop53'?CHOP53:NEAREST53;
 return order.map(([first,second,last],index)=>{
  const row=Math.floor(index/3)*3,col=index%3;
  const term=k=>parent[row+k]*tag[k][col];
  return store32(add(add(term(first),term(second)),term(last)));
 });
}

// Own-world motion adapter. Native helpers receive original Z-up XY and the
// inverse of the current world tag rotation. Scale and translation stay out.
// First appearance has zero delta; moving means a nonzero exported velocity.
export function dynamicSailForces(actor,source,planeIndex,life={},options={}){
 const plane=source.planes[planeIndex];if(!plane)throw Error('Dynamic sail force plane is missing');
 const yaw=actor.renderYaw+source.yawOffset;if(!Number.isFinite(yaw))throw Error('Dynamic sail actor yaw is invalid');
 const previous=life.isNewOwner?null:life.previousActor;
 const delta=previous?[actor.renderX-previous.renderX,-(actor.renderY-previous.renderY)]:[0,0];
 const moving=!!(actor.velocityX||actor.velocityY),rounding=options.rounding||'chop53';
 const world=nativeMotionWorldForces(delta,{moving,windDamping:options.windDamping??0,suppressWind:!!life.dead,rounding});
 const c=Math.cos(-yaw),s=Math.sin(-yaw),m=plane.tag;
 const rotation=options.bodyBasis?nativeSailTagRotation(m,options.bodyBasis,rounding):
  [...m[0].slice(0,3).map((v,i)=>Math.fround(c*v-s*m[1][i])),...m[0].slice(0,3).map((v,i)=>Math.fround(s*v+c*m[1][i])),...m[2].slice(0,3).map(Math.fround)];
 return nativeLocalForces(world,nativeInverseTagRotation(rotation),{rounding});
}

export function createDynamicSailGPU(gl,instanceBuffer){
 const owners=new Map(),retiredVAOs=new WeakSet(),retiredBuffers=new WeakSet();let session=null,frame=null,disposed=false;const stats={uploads:0,owners:0,planes:0,bytes:0};
 function drop(owner){for(const part of owner.parts){retiredBuffers.add(part.vertices);retiredBuffers.add(part.indices);retiredVAOs.add(part.vao);gl.deleteBuffer?.(part.vertices);gl.deleteBuffer?.(part.indices);gl.deleteVertexArray?.(part.vao);}owners.delete(owner.handle);}
 function reset(){for(const owner of [...owners.values()])drop(owner);stats.owners=0;stats.planes=0;}
 function createPart(plane){
  const vao=gl.createVertexArray(),vertices=gl.createBuffer(),indices=gl.createBuffer();gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint32Array(plane.indices),gl.STATIC_DRAW);
  gl.bindBuffer(gl.ARRAY_BUFFER,vertices);
  for(const [loc,size,offset]of [[0,3,0],[1,3,12],[2,2,24],[6,4,32]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,size,gl.FLOAT,false,48,offset);}
  gl.bindBuffer(gl.ARRAY_BUFFER,instanceBuffer);
  for(const [loc,offset]of [[3,0],[4,16],[5,32]]){gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,4,gl.FLOAT,false,48,offset);gl.vertexAttribDivisor(loc,1);}
  return {vao,vertices,indices,count:plane.indices.length};
 }
 function prepare({controller,metadata}){
  const status=controller?.status?.();
  if(!status||!metadata||status.session!==metadata.session||status.frame!==metadata.frame||typeof metadata.forHandle!=='function'||typeof metadata.forInstance!=='function')throw Error('Dynamic sails require the already advanced matching metadata frame');
  if(session===metadata.session&&frame===metadata.frame)return false;
  const plans=[];
  for(const actor of metadata.actors){const owner=controller.get(actor.handle);if(!owner||owner.actor!==actor)continue;
   if(owner.handle!==actor.handle||owner.sourceKey!==owner.source?.key||owner.parts.length!==owner.source.planes.length||!Number.isInteger(owner.damageMask)||owner.damageMask<0||owner.damageMask>8)throw Error('Dynamic sail owner/source state differs');
   plans.push({actor,owner,packed:owner.parts.map((part,i)=>dynamicSailVertices(owner.source.planes[i],part.shading,owner.source.normalization))});
  }
  const previousVAO=gl.getParameter(gl.VERTEX_ARRAY_BINDING),previousBuffer=gl.getParameter(gl.ARRAY_BUFFER_BINDING);
  try{
   disposed=false;if(session!==metadata.session){reset();session=metadata.session;}
   for(const owner of [...owners.values()])if(controller.get(owner.handle)?.sourceKey!==owner.sourceKey)drop(owner);
   for(const {actor,owner,packed}of plans){
    let gpu=owners.get(actor.handle);if(!gpu||gpu.sourceKey!==owner.sourceKey){if(gpu)drop(gpu);gpu={handle:actor.handle,sourceKey:owner.sourceKey,parts:owner.source.planes.map(createPart)};owners.set(actor.handle,gpu);}
    let localBounds=owner.source.initialBounds;
    for(let i=0;i<packed.length;i++){
     const data=packed[i],part=gpu.parts[i];gl.bindBuffer(gl.ARRAY_BUFFER,part.vertices);gl.bufferData(gl.ARRAY_BUFFER,data,gl.DYNAMIC_DRAW);
     const positions=[];for(let k=0;k<data.length;k+=12)positions.push(data[k],data[k+1],data[k+2]);const b=bounds(positions);localBounds=localBounds?union(localBounds,b):b;
     part.height=b.max[1];stats.uploads++;stats.bytes+=data.byteLength;
    }
    gpu.frame=metadata.frame;gpu.session=session;gpu.actor=actor;gpu.damageMask=owner.damageMask;gpu.localBounds=localBounds;
   }
   frame=metadata.frame;stats.owners=owners.size;stats.planes=[...owners.values()].reduce((n,o)=>n+o.parts.length,0);return true;
  }finally{
   // A new session/visual can retire the VAO left bound by the prior draw.
   // WebGL rejects rebinding a deleted object; restore null in that case.
   gl.bindVertexArray(previousVAO&&retiredVAOs.has(previousVAO)?null:previousVAO);gl.bindBuffer(gl.ARRAY_BUFFER,previousBuffer&&retiredBuffers.has(previousBuffer)?null:previousBuffer);
  }
 }
 function get(actor,metadata,key,prepared=metadata){
  if(disposed||!actor||metadata?.session!==session||metadata.forHandle?.(actor.handle)!==actor||
     prepared?.session!==session||prepared.frame!==frame)return null;
  const owner=owners.get(actor.handle),source=prepared.forHandle?.(actor.handle);
  if(owner?.frame!==frame||owner.actor!==source||owner.sourceKey!==key)return null;
  if(source!==actor&&['def','team','hpRatio','renderTime','renderX','renderY','renderAltitude','renderYaw']
     .some(field=>source[field]!==actor[field]))return null;
  return owner;
 }
 function ownerBounds(handle,variant,basisForActor){
  const owner=owners.get(handle);if(!owner||!variant)return null;
  const {actor,localBounds:b}=owner;
  // Without the verified native scale, metadata has no generic instance-size
  // channel. Keep useful local bounds but do not invent a world-sized box.
  if(!Number.isFinite(variant.nativeScale))return {local:b,world:null,session:owner.session,frame:owner.frame,policy:'World bounds unavailable without verified native scale; own instance-size is not present in actor metadata.'};
  const scale=variant.nativeScale,o=variant.originOffset,yaw=actor.renderYaw+variant.yawOffset,c=Math.cos(yaw),s=Math.sin(yaw),points=[];
  const basis=basisForActor?.(actor)??[c,0,s,0,1,0,-s,0,c];
  for(const x of [b.min[0],b.max[0]])for(const y of [b.min[1],b.max[1]])for(const z of [b.min[2],b.max[2]]){
   const px=(x+o[0])*scale,py=(y+o[1]+variant.yOffset)*scale,pz=(z+o[2])*scale;
   points.push(actor.renderX+basis[0]*px+basis[3]*py+basis[6]*pz,
    actor.renderAltitude+basis[1]*px+basis[4]*py+basis[7]*pz,actor.renderY+basis[2]*px+basis[5]*py+basis[8]*pz);
  }
  return {local:b,world:bounds(points),session:owner.session,frame:owner.frame,policy:'Conservative own AABB of immutable body plus deformed local source grids; does not replace simulation visibility/culling.'};
 }
 return Object.freeze({prepare,get,bounds:ownerBounds,sourceKey:handle=>owners.get(handle)?.sourceKey??null,status:()=>Object.freeze({session,frame,...stats,disposed,colorUpload:'unclamped binary32 RGBA attribute; native packed vertex format remains unproven'}),dispose(){reset();session=null;frame=null;disposed=true;}});
}
