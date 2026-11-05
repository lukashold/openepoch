// Independent sampled-frame container, not an original EE2 file format.
// Samples store the same Float32 values uploaded by the existing morph path.
export function attachAnimationPayload(model,input){
 const spec=model?.animationPayload;
 if(!spec)return model;
 if(spec.format!=='ee2-own-sampled-animations-f32-v1'||spec.endianness!=='little'||spec.sourceSha256!==model.source?.sha256)throw new Error('Animation source binding differs');
 let bytes;
 if(input instanceof ArrayBuffer)bytes=new Uint8Array(input);
 else if(ArrayBuffer.isView(input))bytes=new Uint8Array(input.buffer,input.byteOffset,input.byteLength);
 else throw new TypeError('Animation payload must be bytes');
 if(bytes.byteLength!==spec.bytes||bytes.byteLength<20||bytes.byteLength>128*1024*1024)throw new Error('Animation payload length differs');
 const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
 if(view.getUint32(0,true)!==0x31413943||view.getUint32(4,true)!==1||view.getUint32(16,true)!==0)throw new Error('Unsupported animation payload header');
 const count=view.getUint32(8,true),partCount=view.getUint32(12,true);
 if(count!==spec.floatCount||partCount!==model.parts?.length||bytes.byteLength!==20+count*4)throw new Error('Animation payload shape differs');
 let values;
 const littleEndian=new Uint8Array(new Uint32Array([1]).buffer)[0]===1;
 if(littleEndian&&(bytes.byteOffset+20)%4===0)values=new Float32Array(bytes.buffer,bytes.byteOffset+20,count);
 else{values=new Float32Array(count);for(let i=0;i<count;i++)values[i]=view.getFloat32(20+i*4,true);}
 for(const value of values)if(!Number.isFinite(value))throw new Error('Nonfinite animation sample');
 let cursor=0;
 const parts=model.parts.map(part=>{
  if(!part.animations)return part;
  const length=part.positions?.length;
  if(!Number.isInteger(length)||length<3||length%3)throw new Error('Invalid animation vertex count');
  const animations={};
  for(const[name,clip]of Object.entries(part.animations)){
   if(!name||!Number.isFinite(clip.duration)||clip.duration<0||!Array.isArray(clip.frames)||clip.frames.length<1||clip.frames.length>64)throw new Error('Invalid animation clip');
   const frames=clip.frames.map(frame=>{
    if(frame.positionsOffset!==cursor||frame.normalsOffset!==cursor+length||cursor+2*length>count)throw new Error('Animation ranges overlap or differ');
    const positions=values.subarray(cursor,cursor+length),normals=values.subarray(cursor+length,cursor+length*2);cursor+=length*2;
    return{positions,normals};
   });
   animations[name]={...clip,frames};
  }
  return{...part,animations};
 });
 if(cursor!==count)throw new Error('Unused animation payload data');
 return{...model,parts,animationPayloadLoaded:true};
}

export async function loadAnimationPayload(model,{fetchImpl=globalThis.fetch,base=import.meta.url,cryptoImpl=globalThis.crypto}={}){
 if(!model?.animationPayload)return model;
 const spec=model.animationPayload;
 if(!/^[a-f0-9]{64}$/.test(spec.sha256||'')||typeof spec.url!=='string')throw new Error('Invalid animation payload digest');
 const response=await fetchImpl(new URL(spec.url.startsWith('.')?spec.url:'./'+spec.url,base));
 if(!response.ok)throw new Error('Animation payload fetch failed');
 const bytes=await response.arrayBuffer();
 if(bytes.byteLength!==spec.bytes)throw new Error('Animation payload length differs');
 if(!cryptoImpl?.subtle)throw new Error('Animation payload SHA verification unavailable');
 const hash=Array.from(new Uint8Array(await cryptoImpl.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
 if(hash!==spec.sha256)throw new Error('Animation payload SHA differs');
 return attachAnimationPayload(model,bytes);
}

export function withoutAnimationPayload(model){
 // A failed motion sidecar may keep the correctly identified female/male base
 // geometry. It never selects a different source identity or changes actors.
 return{...model,parts:model.parts.map(part=>{const{animations,...staticPart}=part;return staticPart;}),animationPayloadLoaded:false};
}
