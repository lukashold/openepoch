// Read-only outer framing for tooling. Validation of gameplay fields belongs to
// the transactional WASM loader; this exposes complete inner7/8 without guessing
// their ATK/PQP/CIV adjacency or registry layout.
export function unwrapPackSave(input){
 const b=input instanceof Uint8Array?input:new Uint8Array(input),v=new DataView(b.buffer,b.byteOffset,b.byteLength),u=o=>v.getUint32(o,true);
 if(b.length<4||u(0)!==0x31574b50)return Object.freeze({wrapped:false,core:b,coreLength:b.length,coreMagic:b.length>=4?u(0):null});
 if(b.length<112||![1,2,3,4].includes(u(4))||u(56)!==48||u(60)!==64)throw new Error('Invalid PKW header');
 const version=u(4),coreLength=u(8),count=u(12),coreMagic=u(16),capacity=8192,recordOffset=64+coreLength,allocatorOffset=recordOffset+count*48,allocatorEnd=allocatorOffset+(capacity-count)*4;
 if(count>capacity||coreLength<36||coreLength>64*1024*1024||allocatorEnd+12>b.length||u(64)!==coreMagic||coreMagic===0x31574b50||u(b.length-12)!==b.length||u(b.length-4)!==0x454b4350)throw new Error('Invalid PKW size/footer');
 if((version===4)!==(coreMagic===0x3955454f))throw new Error('Source identity marker/envelope mismatch');
 let navigation=null,projectiles=null,sourceIdentities=null;
 if(version===1){if(allocatorEnd+12!==b.length)throw new Error('Invalid PKW1 tail');}
 else{
  const p=allocatorEnd;if(p+52+12>b.length||u(p)!==0x3143564e||u(p+4)!==1||u(p+12)!==96||u(p+16)!==65536||u(p+24)>4||u(p+28)!==count||u(p+32)>1||u(p+36)!==8||u(p+40)!==16||u(p+44)!==48)throw new Error('Invalid NVC1 header');
  const size=u(p+8),min=48+96*16+count*8+4;if(size<min||size>min+96*65536||p+size+12>b.length||version===2&&p+size+12!==b.length||u(p+size-4)!==0x4543564e)throw new Error('Invalid NVC1 framing');
  let at=p+48;const fields=[],keys=new Set();
  for(let slot=0;slot<96;slot++){
   if(at+16>p+size-4)throw new Error('Truncated NVC1 field');
   const key=u(at),ver=u(at+4),used=u(at+8),length=u(at+12),offset=at;at+=16;
   if(used>u(64+24)||(key===0xffffffff?(length!==0||ver!==0||used!==0):((key>>>16)>1||length!==65536||keys.has(key))))throw new Error('Invalid NVC1 field');
   if(key!==0xffffffff)keys.add(key);if(at+length>p+size-4)throw new Error('Truncated NVC1 directions');
   const directions=b.subarray(at,at+length);if(directions.some(x=>x>8&&x!==255))throw new Error('Invalid NVC1 direction');at+=length;
   fields.push(Object.freeze({slot,key,ver,used,offset,length,directions}));
  }
  const actors=[];for(let i=0;i<count;i++){if(at+8>p+size-4)throw new Error('Truncated NVC1 actor');const handle=u(at),slot=u(at+4);if(handle!==u(recordOffset+i*48)||slot!==65535&&(slot>=96||fields[slot].length===0))throw new Error('Invalid NVC1 actor identity/slot');actors.push(Object.freeze({handle,slot,offset:at}));at+=8;}
  if(at+4!==p+size)throw new Error('Invalid NVC1 tail');
  navigation=Object.freeze({version:1,offset:p,bytes:size,navVersion:u(p+20),budget:u(p+24),historyUnknown:!!u(p+32),fields:Object.freeze(fields),actors:Object.freeze(actors)});
  if(version>=3){
   const at=p+size;if(at+36+12>b.length||u(at)!==0x314a5250||u(at+4)!==1||u(at+12)>4000||u(at+16)!==64||u(at+20)>1||u(at+24)!==32||u(at+28)!==4000)throw new Error('Invalid PRJ1 header');
   const bytes=u(at+8),count=u(at+12);if(bytes!==36+count*64||at+bytes+12>b.length||version===3&&at+bytes+12!==b.length||u(at+bytes-4)!==0x454a5250)throw new Error('Invalid PRJ1 framing');
   const records=[];for(let i=0;i<count;i++){
    const o=at+32+i*64,f=k=>v.getFloat32(o+k,true);const positions=[0,4,8,12,16,20].map(f),nonnegative=[28,32,44,56].map(f),d0=f(60),tgt=u(o+24),kind=u(o+36),team=u(o+40),targets=u(o+48),profile=u(o+52);
    if(positions.some(x=>!Number.isFinite(x)||x<0||x>256)||nonnegative.some(x=>!Number.isFinite(x)||x<0)||!Number.isFinite(d0)||d0<=0||tgt!==0xffffffff&&(tgt&65535)>=8192||kind>6||team>=4||targets&~15||profile!==65535&&profile>=366)throw new Error('Invalid PRJ1 record');
    records.push(Object.freeze({offset:o,x:positions[0],y:positions[1],px:positions[2],py:positions[3],tx:positions[4],ty:positions[5],targetHandle:tgt,speed:nonnegative[0],damage:nonnegative[1],kind,team,splash:nonnegative[2],targets,attackerProfile:profile,friendly:nonnegative[3],launchDistance:d0}));
   }
   projectiles=Object.freeze({version:1,offset:at,bytes,count,recordStride:64,historyUnknown:!!u(at+20),records:Object.freeze(records)});
   if(version===4){
    const q=at+bytes,actors=u(12),length=64+actors*16+4;
    if(q+length+12!==b.length||u(q)!==0x31524942||u(q+4)!==1||u(q+8)!==length||u(q+12)!==actors||u(q+16)!==16||u(q+24)>1||u(q+28)!==64||u(q+length-4)!==0x45524942)throw new Error('Invalid BIR1 framing');
    if(u(68)!==176||u(72)!==96||u(76)!==80)throw new Error('Invalid source9 numeric layout');
    const rows=[];let previous=-1;
    for(let i=0;i<actors;i++){
     const o=q+64+i*16,handle=u(o),slot=handle&65535,identity=u(o+4),femaleCargo=u(o+8),entity=64+36+4*96+slot*176;
     if(slot<=previous||slot>=u(96)||handle!==u(recordOffset+i*48)||u(o+12)!==0||entity+176>64+coreLength)throw new Error('Invalid BIR1 actor');
     previous=slot;const def=v.getUint16(entity+2,true),ncargo=b[entity+172];
     if(ncargo>10||(def===8?identity>1:identity!==2))throw new Error('Invalid BIR1 identity');
     let allowed=0;for(let k=0;k<ncargo;k++)if(v.getUint16(entity+142+k*2,true)===8)allowed|=1<<k;
     if(femaleCargo&~allowed)throw new Error('Invalid BIR1 cargo identity');
     rows.push(Object.freeze({handle,identity,femaleCargo,offset:o}));
    }
    sourceIdentities=Object.freeze({version:1,offset:q,bytes:length,rngState:u(q+20),rngOrigin:u(q+24),rngHistoryUnknown:u(q+24)===1,
     fingerprint:Array.from(b.subarray(q+32,q+64),x=>x.toString(16).padStart(2,'0')).join(''),records:Object.freeze(rows)});
   }
  }
 }
 let h=2166136261;for(let i=0;i<b.length-12;i++)h=Math.imul(h^b[i],16777619)>>>0;if(h!==u(b.length-8))throw new Error('PKW1 checksum mismatch');
 return Object.freeze({wrapped:true,version,core:b.subarray(64,recordOffset),coreLength,coreMagic,records:b.subarray(recordOffset,allocatorOffset),freeHandles:b.subarray(allocatorOffset,allocatorEnd),recordCount:count,headerBytes:64,recordStride:48,freeHandleStride:4,navigation,navigationHistoryUnknown:navigation?.historyUnknown??true,projectiles,projectileHistoryUnknown:projectiles?.historyUnknown??true,sourceIdentities,sourceHash:Array.from(b.subarray(24,56),x=>x.toString(16).padStart(2,'0')).join('')});
}
