// Authenticated optional presentation data. The installed NIF/KF-derived
// sidecars live only in the local-original package.
const bindings=new Map([
 ['64469bfd3814a83da19cfc883c7e628ec85595576afad470a07b0c53608a8e03',{
  model:'8af0f7074da5630f91a82f59afd0cb34b3b15719ad5e7289754de4241ad59306',
  sidecar:'b5b789a26a438f14ee784796fd2c70ee71dfa0bd7d32136def956474db92c392',era:1}],
 ['b9f7ebe1eff2797c7bdeb7f2291085cc3db8e4ffc9daaaf07a6cbce27126a4f2',{
  model:'4b2d442bb66e854ae6657589646aa15c060d15a990e0b3bc2c08fb038fdeb849',
  sidecar:'0f06bac4e706b7e737c3b8000ae8dabbcc528db49ac5f03d6455f31114ba7363',era:3}],
]);
const clips=['idle','native_unpacked_idle','native_pack','native_unpack'];
const f=Math.fround;
const center=f(.9590659737586975), factor=f(1.000311017036438);
const intercept=f(factor/Math.sqrt(center)),slope=f(-.5/(Math.sqrt(center)*center)*factor);
const correction=f(.5854921936988831),dotScale=f(.8227968811988831);
const refine1=f(.9152119755744934),refine2=f(.6521196961402893);
const emptyTrack={rotation:null,translation:null,scale:null};
const paletteHeight=64;
const paletteLayers=32;

const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
const read=async url=>{
 const response=await fetch(url,{cache:'no-store'});
 if(!response.ok)throw new Error(`${url}: ${response.status}`);
 return response.arrayBuffer();
};
const decode=bytes=>JSON.parse(new TextDecoder().decode(bytes));

export function packRigBinding(id,source){return id===22?bindings.get(source):null;}

export async function loadPackRigModel(url,source,binding){
 const bytes=await read(url), model=decode(bytes);
 if(!globalThis.crypto?.subtle||model.source?.sha256!==source)return {model,rig:null};
 try{
  if(await digest(bytes)!==binding.model)return {model,rig:null};
  const sidecar=await read(new URL(`local-original/pack-bone-rigs/pack-rig-e${binding.era}.json`,import.meta.url));
  if(await digest(sidecar)!==binding.sidecar)return {model,rig:null};
  const data=decode(sidecar);
  if(data.sourceSha256!==source||data.modelSha256!==binding.model)return {model,rig:null};
  return {model,rig:preparePackRig(data,model)};
 }catch{
  return {model,rig:null};
 }
}

function normalizeWeights(weights){
 const reciprocal=f(1/weights.reduce((sum,row)=>sum+row[1],0));
 return weights.map(([index,weight])=>[index,f(weight*reciprocal)]);
}

export function sourcePackWeights(weights){
 let sorted=normalizeWeights(weights);
 if(sorted.length>2){
  // MSVCRT short sort keeps the first equal minimum when moving it to the end.
  for(let end=sorted.length-1;end>0;end--){
   let minimum=0;
   for(let i=1;i<=end;i++)if(sorted[i][1]<sorted[minimum][1])minimum=i;
   [sorted[minimum],sorted[end]]=[sorted[end],sorted[minimum]];
  }
  sorted=normalizeWeights(sorted.slice(0,2));
 }
 return [sorted[0][0],sorted[1]?.[0]??0,sorted[0][1]];
}

function baseQuaternion(m){
 const values=[m[0]+m[4]+m[8],m[0]-m[4]-m[8],m[4]-m[0]-m[8],m[8]-m[0]-m[4]];
 const slot=values.indexOf(Math.max(...values)),a=.5*Math.sqrt(1+values[slot]),b=.25/a;
 const sx=(m[7]-m[5])*b,sy=(m[2]-m[6])*b,sz=(m[3]-m[1])*b;
 const xy=(m[1]+m[3])*b,xz=(m[2]+m[6])*b,yz=(m[5]+m[7])*b;
 const q=[[a,sx,sy,sz],[sx,a,xy,xz],[sy,xy,a,yz],[sz,xz,yz,a]][slot];
 const norm=Math.hypot(...q);return q.map(value=>value/norm);
}

function transform(matrix,translation,scale){return Float32Array.from([...matrix,...translation,scale]);}
function bindMatrix(a){return transform([a[0],a[1],a[2],a[4],a[5],a[6],a[8],a[9],a[10]],[a[3],a[7],a[11]],1);}

export function preparePackRig(rig,model){
 if(rig.format!=='openepoch-private-source-pack-rig-prototype-v1'||rig.normalization.axis!=='x,z,-y'
    ||JSON.stringify(rig.normalization)!==JSON.stringify(model.normalization)||rig.parts.length!==model.parts.length
    ||!clips.every(name=>rig.clips[name]?.duration>0))throw new Error('Pack rig binding mismatch');
 const parts=new Map(),nodes=rig.nodes.map((node,i)=>{
  if(!Number.isInteger(node.parent)||node.parent < -1||node.parent>=i)throw new Error('Invalid pack rig hierarchy');
  return {parent:node.parent,base:transform(node.rotationMatrix,node.translation,node.scale),q:baseQuaternion(node.rotationMatrix)};
 });
 let cursor=0;
 for(const p of rig.parts){
  const owner=model.parts.find(part=>part.sourceNode===p.sourceNode),skin=p.skin;
  if(!owner||parts.has(p.sourceNode)||p.positions.length*3!==owner.positions.length||p.normals.length!==p.positions.length
     ||!nodes[p.transform]||!!skin!==!!owner.skinned||skin&&(!nodes[skin.root]||!skin.joints.length||skin.joints.length>20
       ||skin.weights.length!==p.positions.length))throw new Error('Pack rig topology mismatch');
  const vertices=new Float32Array(p.positions.length*9);
  for(let i=0;i<p.positions.length;i++){
   const weights=skin?.weights[i];
   if(skin&&(!weights.length||weights.length>4||weights.some(([index,w])=>!skin.joints[index]||w<0||!Number.isFinite(w))))throw new Error('Invalid pack skin weights');
   vertices.set(p.positions[i],i*9);vertices.set(p.normals[i],i*9+3);
   vertices.set(weights?sourcePackWeights(weights):[0,0,1],i*9+6);
  }
  const sourcePart={vertices,offset:cursor,transform:p.transform,skin:skin&&{root:skin.root,global:bindMatrix(skin.skinGlobal),
   joints:skin.joints.map(joint=>({transform:joint.transform,bind:bindMatrix(joint.bind)}))}};
  parts.set(p.sourceNode,sourcePart);cursor+=skin?.joints.length??1;
 }
 if(cursor>paletteHeight)throw new Error('Pack rig palette exceeds supported size');
 return {parts,nodes,clips:rig.clips,normalization:Float32Array.from([...rig.normalization.offset,rig.normalization.scale])};
}

function bracket(keys,seconds){
 if(keys.length===1)return [keys[0],null,0];
 let low=0,high=keys.length-1;
 while(low+1<high){const mid=(low+high)>>1;if(seconds<=keys[mid][0])high=mid;else low=mid;}
 const a=keys[low],b=keys[high];
 return [a,b,f(Math.min(1,Math.max(0,(seconds-a[0])/(b[0]-a[0]))))];
}

const linear=(a,b,t)=>f((1-t)*a+t*b);
function sample(track,seconds,fallback){
 if(!track)return fallback;
 const [a,b,t]=bracket(track.keys,seconds);
 if(!b)return a[1];
 if(Array.isArray(a[1]))return a[1].map((v,i)=>linear(v,b[1][i],t));
 if(track.mode===1)return linear(a[1],b[1],t);
 // Original type 2 scalar reads the previous disk second tangent and next first.
 const delta=b[1]-a[1],out=a[3],incoming=b[2];
 return f((((out+incoming-2*delta)*t+3*delta-2*out-incoming)*t+out)*t+a[1]);
}

function mixRotation(a,b,t,shortest){
 let dot=f(a[0]*b[0]+a[1]*b[1]+a[2]*b[2]+a[3]*b[3]);
 if(shortest&&dot<=0){b=b.map(v=>-v);dot=-dot;}
 const small=t>.5?1-t:t,k=1-dot*dotScale;
 const w=small*(1+correction*k*k*(1+(2*small-3)*small)),alpha=t>.5?1-w:w;
 const q=a.map((v,i)=>f(v+(b[i]-v)*alpha));
 const squared=f(q[0]**2+q[1]**2+q[2]**2+q[3]**2);
 const estimate=v=>intercept+(v-center)*slope;
 let inverse=estimate(squared);
 if(squared<=refine1)inverse*=estimate(inverse*inverse*squared);
 if(squared<=refine2)inverse*=estimate(inverse*inverse*squared);
 return q.map(v=>f(v*inverse));
}

function quaternionMultiply([aw,ax,ay,az],[bw,bx,by,bz]){
 return [f(aw*bw-ax*bx-ay*by-az*bz),f(aw*bx+ax*bw+ay*bz-az*by),
         f(aw*by+ay*bw+az*bx-ax*bz),f(aw*bz+az*bw+ax*by-ay*bx)];
}

function sampleRotation(track,seconds,base){
 if(!track)return base;
 if(track.mode===1){
  const [a,b,t]=bracket(track.keys,seconds);
  return b?mixRotation(a[1],b[1],t,false):a[1];
 }
 let result=[1,0,0,0];
 for(let axis=0;axis<3;axis++){
  const half=sample(track.groups[axis],seconds,0)*.5,q=[f(Math.cos(half)),0,0,0];
  q[axis+1]=f(Math.sin(half));result=quaternionMultiply(q,result);
 }
 return result;
}

const rotation=([w,x,y,z])=>[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w),2*(x*y+z*w),1-2*(x*x+z*z),
 2*(y*z-x*w),2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)];

function compose(a,b){
 const out=new Float32Array(13);
 for(let i=0;i<3;i++){
  const row=3*i,x=a[row],y=a[row+1],z=a[row+2];
  for(let j=0;j<3;j++)out[row+j]=x*b[j]+y*b[j+3]+z*b[j+6];
  out[9+i]=a[9+i]+f(a[12]*f(x*b[9]+y*b[10]+z*b[11]));
 }
 out[12]=a[12]*b[12];return out;
}

function inverse(a){
 const transposed=[a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]],scale=f(1/a[12]);
 const translation=[0,3,6].map(i=>scale*f(-transposed[i]*a[9]-transposed[i+1]*a[10]-transposed[i+2]*a[11]));
 return transform(transposed,translation,scale);
}

function evaluate(rig,name,phase,blend){
 const clip=rig.clips[name],prior=blend&&rig.clips[blend.outgoing.clip];
 if(!clip||blend&&!prior)return null;
 const time=(blend?.incomingPhase??phase)*clip.duration,previousTime=blend&&blend.outgoing.phase*prior.duration;
 const world=[];
 for(let i=0;i<rig.nodes.length;i++){
  const node=rig.nodes[i],track=clip.tracks[i],old=prior?.tracks[i];
  let local=node.base;
  if(track||old){
   const b=track??emptyTrack,a=old??emptyTrack,translation=node.base.subarray(9,12),scale=node.base[12];
   let q=sampleRotation(b.rotation,time,node.q),t=sample(b.translation,time,translation),s=sample(b.scale,time,scale);
   if(prior){
    q=mixRotation(sampleRotation(a.rotation,previousTime,node.q),q,blend.weight,true);
    const from=sample(a.translation,previousTime,translation);
    t=[0,1,2].map(j=>linear(from[j],t[j],blend.weight));s=linear(sample(a.scale,previousTime,scale),s,blend.weight);
   }
   if(s<0)return null;
   const basis=prior||b.rotation?rotation(q):node.base.subarray(0,9);
   local=transform(basis,t,s);
  }
  world.push(node.parent<0?local:compose(world[node.parent],local));
 }
 const data=new Float32Array(paletteHeight*12);
 const put=(offset,a)=>{
  for(let i=0;i<3;i++)data.set([a[i*3]*a[12],a[i*3+1]*a[12],a[i*3+2]*a[12],a[9+i]],offset*12+i*4);
 };
 for(const part of rig.parts.values()){
  const geometry=world[part.transform],skin=part.skin;
  if(!skin){put(part.offset,geometry);continue;}
  const root=world[skin.root];if(root[12]===0)return null;
  const prefix=compose(geometry,compose(skin.global,inverse(root)));
  for(let j=0;j<skin.joints.length;j++){
   const joint=skin.joints[j];put(part.offset+j,compose(prefix,compose(world[joint.transform],joint.bind)));
  }
 }
 return data.every(Number.isFinite)?data:null;
}

export function createPackRigFrames(gl){
 let frame,session,capture=false,used=0;
 const active=new Map(),blocked=new Map(),pool=[];
 return {
  begin(metadata,isCapture){
   capture=isCapture;
   if(!metadata)return false;
   if(capture)return metadata.session===session;
   if(session!==metadata.session||frame!==metadata.frame){
    if(session!==metadata.session)blocked.clear();
    else for(const handle of blocked.keys())if(!metadata.forHandle(handle))blocked.delete(handle);
    session=metadata.session;frame=metadata.frame;active.clear();used=0;
   }
   return true;
  },
  pose(actor,rig,clip,phase,blend){
   const cached=active.get(actor.handle);
   if(cached?.rig===rig&&cached.clip===clip&&cached.phase===phase&&cached.time===actor.renderTime)return cached;
   if(capture)return null;
   const task=actor.pack?.duration?actor.pack.start:null;
   if(blocked.get(actor.handle)!==task)blocked.delete(actor.handle);
   if(blend&&!rig.clips[blend.outgoing.clip]&&task!==null)blocked.set(actor.handle,task);
   if(blocked.has(actor.handle))return null;
   const data=evaluate(rig,clip,phase,blend);if(!data)return null;
   gl.activeTexture(gl.TEXTURE0+8);
   const page=Math.floor(used/paletteLayers),layer=used%paletteLayers;
   let texture=pool[page];
   if(!texture){
    texture=gl.createTexture();if(!texture)return null;
    gl.bindTexture(gl.TEXTURE_2D_ARRAY,texture);
    gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.RGBA32F,3,paletteHeight,paletteLayers);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
    pool[page]=texture;
   }else gl.bindTexture(gl.TEXTURE_2D_ARRAY,texture);
   used++;
   gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,layer,3,paletteHeight,1,gl.RGBA,gl.FLOAT,data);
   const record={texture,page,layer,rig,clip,phase,time:actor.renderTime};active.set(actor.handle,record);return record;
  },
 };
}
