// Optional Pack8 presentation data. Existing actor DTO/instance rows remain
// unchanged. This reader never changes a clock, order or WASM memory byte.
const APIS=['render_pack_version','render_pack_stride','render_pack_count','render_pack_len','render_pack_ptr'];
const fail=m=>{throw new Error('Pack render metadata: '+m);};
export function createPackRenderMetadata(wasm){
 const present=APIS.filter(k=>typeof wasm?.[k]==='function');if(!present.length)return null;if(present.length!==APIS.length)fail('incomplete ABI');
 if(wasm.render_pack_version()!==1||wasm.render_pack_stride()!==32)fail('unsupported layout');
 return Object.freeze({snapshot(actorMetadata){
  if(!actorMetadata||actorMetadata.version!==1)fail('validated actor snapshot required');
  const {count,session,frame}=actorMetadata,bytes=wasm.render_pack_len()>>>0,ptr=wasm.render_pack_ptr()>>>0;
  if(wasm.render_pack_count()!==count||bytes!==count*32||wasm.render_meta_session()!==session||wasm.render_meta_frame()!==frame)fail('stale or unaligned frame');
  const memory=wasm.memory.buffer;if(ptr%4||ptr>memory.byteLength||bytes>memory.byteLength-ptr||(count===0)!==(ptr===0))fail('invalid memory range');
  const v=new DataView(new Uint8Array(memory,ptr,bytes).slice().buffer),rows=Array(count).fill(null),byHandle=new Map(),actors=[];
  for(let i=0;i<count;i++){
   const at=i*32,h=v.getUint32(at,true),actor=actorMetadata.forInstance(i);if(!actor){if(h!==0xffffffff)fail('nonactor has pack identity');continue;}if(h!==actor.handle)fail('actor handle mismatch');
   const flags=v.getUint32(at+4,true),phase=v.getUint32(at+8,true),visual=v.getUint32(at+12,true),start=v.getFloat32(at+16,true),duration=v.getFloat32(at+20,true),unknown=v.getUint32(at+24,true),capable=v.getUint32(at+28,true);
   if(flags&~10||phase>2||visual>3||unknown>1||capable>1||!Number.isFinite(start)||start<0||!Number.isFinite(duration)||duration<0||start>actor.renderTime||(visual===1||visual===2?duration<=0:duration!==0))fail('invalid pack state');
   const pack=Object.freeze({flags,phase,visual,start,duration,historyUnknown:!!unknown,capable:!!capable});
   const row=Object.freeze({...actor,pack});rows[i]=row;actors.push(row);byHandle.set(h,row);
  }
  if(wasm.render_meta_session()!==session||wasm.render_meta_frame()!==frame)fail('frame changed during copy');
  return Object.freeze({...actorMetadata,actors:Object.freeze(actors),forInstance:i=>Number.isInteger(i)?rows[i]||null:null,forHandle:h=>byHandle.get(h)||null});
 }});
}

export function appendNativePackAnimations(parent,overlay){
 if(overlay?.format!=='ee2-native-pack-animation-overlay-v1'||parent.source?.sha256!==overlay.source?.sha256||JSON.stringify(parent.normalization)!==JSON.stringify(overlay.normalization))throw new Error('Pack overlay source/normalization mismatch');
 const parts=parent.parts.map(p=>{
  const extra=overlay.parts.find(s=>s.sourceNode===p.sourceNode);if(!extra)return p;
  if(extra.vertices*3!==p.positions.length||extra.indices!==p.indices.length)throw new Error('Pack overlay part topology mismatch');
  for(const [name,clip]of Object.entries(extra.animations)){
   if(!['native_pack','native_unpack','native_unpacked_idle'].includes(name)||!Number.isFinite(clip.duration)||clip.duration<0||!clip.frames?.length)throw new Error('Invalid pack animation clip');
   for(const f of clip.frames)if(f.positions.length!==p.positions.length||f.normals.length!==p.normals.length||!f.positions.every(Number.isFinite)||!f.normals.every(Number.isFinite))throw new Error('Invalid pack animation frame');
  }
  return {...p,nativeStateClips:{...p.nativeStateClips,Pack:'native_pack',Unpack:'native_unpack',UnpackedIdle:'native_unpacked_idle'},animations:{...p.animations,...extra.animations}};
 });
 return {...parent,parts,nativePackAnimation:overlay.binding};
}

export function nativePackPose(actor,{sourceId,team,data,index,parts}={}){
 if(!actor?.pack||actor.def!==sourceId||actor.team!==team||actor.renderX!==data[index*12]||actor.renderY!==data[index*12+1]||actor.renderAltitude!==data[index*12+2]||actor.renderYaw!==data[index*12+3]||Math.round(data[index*12+7])&24)return null;
 const state=actor.pack,name=[null,'Pack','Unpack','UnpackedIdle'][state.visual];if(!name)return null;
 const candidates=parts.map(p=>p.nativeStateClips?.[name]).filter(Boolean),clip=candidates.find(k=>parts.some(p=>p.animations?.[k]));if(!clip)return null;
 const once=state.visual!==3,duration=once?state.duration:(parts.find(p=>p.animations?.[clip])?.animations[clip]?.duration||1);
 const phase=Math.max(0,(actor.renderTime-state.start)/duration);return {clip,once,phase:once?Math.min(1,phase):phase%1};
}
