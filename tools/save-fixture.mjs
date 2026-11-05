// Explicit current save-format contract for controlled end-to-end fixtures.
// Legacy-format tests must truncate at the derived core end, never remove an
// assumed number of bytes from a growing save trailer.
import assert from 'node:assert/strict';
import {unwrapPackSave}from'./pack8-save-envelope.mjs';
export const DIP_BYTES=262744;
export const TER_BYTES=524;
export const CITY_MAGIC=0x31444743,CITY_END=0x45444743;
export const ATTACK_MAGIC=0x314b5441,ATTACK_END=0x454b5441;
const NN=256*256;
export function saveLayout(bytes){
 const packEnvelope=unwrapPackSave(bytes);bytes=packEnvelope.core;
 const d=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),players=d.getUint32(8,true),entities=d.getUint32(4,true)*d.getUint32(32,true);
 const end=36+4*players+entities+5*NN,magic=d.getUint32(0,true),citizen9=magic===0x3955454f,civ8=magic===0x3855454f||citizen9,paidqueue7=magic===0x3755454f||civ8,roster6=magic===0x3655454f||paidqueue7,up15=magic===0x3555454f||roster6,reference=magic===0x3252454f||up15,prefix=reference?236:132;
 const descriptorBytes=roster6?52:0,descriptor=roster6?bytes.length-52:null,civBytes=civ8?36:0,civ=civ8?descriptor-civBytes:null,payloadEnd=bytes.length-descriptorBytes-civBytes;
 if(roster6){assert.equal(d.getUint32(descriptor,true),0x31533652);assert.equal(d.getUint32(descriptor+4,true),citizen9?9:civ8?8:paidqueue7?7:6);assert.equal(d.getUint32(bytes.length-8,true),52);assert.equal(d.getUint32(bytes.length-4,true),0x45533652);}
 if(civ8){assert.equal(d.getUint32(civ,true),0x31564943);assert.equal(d.getUint32(civ+4,true),1);assert.equal(d.getUint32(civ+8,true),4);for(let i=0;i<4;i++)assert.ok(d.getUint32(civ+12+i*4,true)<=14);assert.equal(d.getUint32(civ+28,true),36);assert.equal(d.getUint32(civ+32,true),0x45564943);}
 const paidBytes=paidqueue7?d.getUint32(payloadEnd-8,true):0,paid=paidqueue7?payloadEnd-paidBytes:null,dataEnd=payloadEnd-paidBytes;
 if(paidqueue7){assert.ok(paidBytes>=20&&paidBytes<=20+8192*5*64);assert.equal(d.getUint32(payloadEnd-4,true),0x45505150);assert.equal(d.getUint32(paid,true),0x31505150);assert.equal(d.getUint32(paid+4,true),2);assert.equal(d.getUint32(descriptor+8,true),civ8?1:2);assert.equal(paidBytes,20+64*d.getUint32(paid+8,true));}
 const attackBytes=up15&&dataEnd>=end+prefix+DIP_BYTES+TER_BYTES+28&&d.getUint32(dataEnd-4,true)===ATTACK_END?d.getUint32(dataEnd-8,true):0;
 assert.ok(attackBytes===0||(attackBytes>=20&&attackBytes<=20+8192*12&&attackBytes<=bytes.length),'bounded ATK1 footer size');
 const currentEnd=dataEnd-attackBytes,attack=attackBytes?currentEnd:null;
 const cityBytes=currentEnd>end+prefix+DIP_BYTES+TER_BYTES&&d.getUint32(currentEnd-4,true)===CITY_END?d.getUint32(currentEnd-8,true):0;
 assert.ok(cityBytes===0||(cityBytes>=20&&cityBytes<=20+8192*8&&cityBytes<=currentEnd),'bounded CGD1 footer size');
 const legacyEnd=currentEnd-cityBytes,city=cityBytes?legacyEnd:null;
 return {d,end,prefix,dip:end+prefix,ter:end+prefix+DIP_BYTES,realFog:end+prefix+600,displayFog:end-NN,reference,up15,roster6,paidqueue7,paid,paidBytes,payloadEnd,legacyEnd,city,cityBytes,attack,attackBytes,currentEnd,dataEnd,descriptor,descriptorBytes,citizen9,civ8,civ,civBytes,core:bytes,packEnvelope};
}
export function assertCurrentSave(bytes){
 const l=saveLayout(bytes);
 assert.equal(l.legacyEnd-l.end,l.prefix+DIP_BYTES+TER_BYTES,'exact legacy prefix plus DIP1 and TER1 extensions');
 assert.equal(l.d.getUint32(l.end+12,true),0x31534552,'RES1 marker');assert.equal(l.d.getUint32(l.end+16,true),1,'RES1 version');
 if(l.reference){assert.equal(l.d.getUint32(l.end+132,true),0x314f4345,'ECO1 marker');assert.equal(l.d.getUint32(l.end+136,true),1,'ECO1 version');}
 assert.equal(l.d.getUint32(l.dip,true),0x31504944,'DIP1 marker');assert.equal(l.d.getUint32(l.dip+4,true),1,'DIP1 version');
 assert.ok([1,2].includes(l.d.getUint32(l.dip+8,true)),'valid saved human-player count');
 assert.ok([2,3,4].includes(l.d.getUint32(l.dip+12,true)),'valid saved total-player count');assert.ok(l.d.getUint32(l.dip+16,true)<=2,'valid saved display mode');
 assert.ok(l.d.getUint32(l.dip+404,true)<=12,'bounded diplomacy log');assert.equal(l.ter-l.realFog,4*NN,'four complete natural exploration maps');
 assert.equal(l.d.getUint32(l.ter,true),0x31524554,'TER1 marker');assert.equal(l.d.getUint32(l.ter+4,true),1,'TER1 version');
 assert.ok(l.d.getUint32(l.ter+328,true)<=12,'bounded territory log');assert.equal(l.legacyEnd-l.ter,TER_BYTES,'complete fixed territory state');
 if(l.reference){
  assert.ok(l.cityBytes>=20,'current EE2 saves include a CGD1 tail');
  assert.equal(l.d.getUint32(l.city,true),CITY_MAGIC,'CGD1 marker');assert.equal(l.d.getUint32(l.city+4,true),1,'CGD1 version');
  const count=l.d.getUint32(l.city+8,true);assert.ok(count<=8192);assert.equal(l.cityBytes,20+count*8,'complete variable City state');
  let previous=-1;
  for(let i=0;i<count;i++){const index=l.d.getUint32(l.city+12+8*i,true)&65535;assert.ok(index>previous,'CGD1 records use ascending entity slots');previous=index;assert.ok([7,8,9,10,11].includes(l.d.getFloat32(l.city+16+8*i,true)),'source BASE radius band');}
 }else assert.equal(l.cityBytes,0,'base save format stays unchanged');
 if(l.attackBytes){assert.ok(l.up15);assert.equal(l.d.getUint32(l.attack,true),ATTACK_MAGIC);assert.equal(l.d.getUint32(l.attack+4,true),1);const count=l.d.getUint32(l.attack+8,true);assert.equal(l.attackBytes,20+12*count);let previous=-1;for(let i=0;i<count;i++){const at=l.attack+12+i*12,handle=l.d.getUint32(at,true),index=handle&65535;assert.ok(index>previous);previous=index;assert.ok(Number.isFinite(l.d.getFloat32(at+4,true))&&l.d.getFloat32(at+4,true)>=0);assert.equal(l.d.getUint32(at+8,true)&~0x87,0);}}
 return l;
}

// Explicitly repair only controlled scenario edits (epochs/TC completion), never
// malformed-save validation or ordinary roundtrip loads. Preserve known lifetime
// history for matching generation handles while adding/removing fixture cities.
export function refreshFixtureCityGround(bytes){
 assert.equal(unwrapPackSave(bytes).wrapped,false,'controlled fixture edits require an explicitly unwrapped core; never auto-repair Pack lifetime state');
 const l=saveLayout(bytes);if(!l.reference)return bytes;
 assert.equal(l.legacyEnd,l.ter+TER_BYTES,'scenario edits require a complete current prefix');
 const previous=new Map();
 if(l.cityBytes){const count=l.d.getUint32(l.city+8,true);for(let k=0;k<count;k++)previous.set(l.d.getUint32(l.city+12+k*8,true),l.d.getFloat32(l.city+16+k*8,true));}
 const playerBytes=l.d.getUint32(8,true),entityBytes=l.d.getUint32(4,true),high=l.d.getUint32(32,true),entities=36+4*playerBytes,records=[];
 for(let i=0;i<high;i++){
  const at=entities+i*entityBytes,team=bytes[at+(l.roster6?4:2)],def=l.roster6?l.d.getUint16(at+2,true):bytes[at+1];
  if(!bytes[at]||def!==0||!(l.d.getFloat32(at+80,true)>=1)||!(l.d.getFloat32(at+40,true)>0))continue;
  const handle=((l.d.getUint16(at+(l.roster6?6:4),true)<<16)|i)>>>0,epoch=bytes[36+team*playerBytes+24],base=7+Math.floor(Math.min(14,epoch)/3);
  records.push([handle,Math.max(previous.get(handle)||0,base)]);
 }
 const size=20+records.length*8,cityEnd=l.legacyEnd+size,result=new Uint8Array(cityEnd+l.attackBytes+l.paidBytes+l.descriptorBytes),d=new DataView(result.buffer);result.set(bytes.subarray(0,l.legacyEnd));
 d.setUint32(l.legacyEnd,CITY_MAGIC,true);d.setUint32(l.legacyEnd+4,1,true);d.setUint32(l.legacyEnd+8,records.length,true);
 records.forEach(([handle,radius],i)=>{d.setUint32(l.legacyEnd+12+i*8,handle,true);d.setFloat32(l.legacyEnd+16+i*8,radius,true);});
 d.setUint32(cityEnd-8,size,true);d.setUint32(cityEnd-4,CITY_END,true);if(l.attackBytes)result.set(bytes.subarray(l.attack,l.dataEnd),cityEnd);if(l.paidBytes)result.set(bytes.subarray(l.paid,l.payloadEnd),cityEnd+l.attackBytes);if(l.descriptorBytes)result.set(bytes.subarray(l.descriptor),cityEnd+l.attackBytes+l.paidBytes);return result;
}
export function revealFixture(bytes){
 assert.equal(unwrapPackSave(bytes).wrapped,false,'controlled fixture edits require an explicitly unwrapped core');
 const l=assertCurrentSave(bytes);
 bytes.fill(255,l.displayFog,l.end);
 bytes.fill(255,l.realFog,l.realFog+4*NN);
 // This helper is only for controlled scenarios, which often edit epochs before
 // revealing. Radius growth must agree with those explicit fixture profiles.
 const updated=refreshFixtureCityGround(bytes);assert.equal(updated.length,bytes.length,'revealing does not add/remove fixture city records');bytes.set(updated);
 return bytes;
}
export function resetFixtureClaims(bytes){
 assert.equal(unwrapPackSave(bytes).wrapped,false,'controlled fixture edits require an explicitly unwrapped core');
 const l=assertCurrentSave(bytes);
 // A fixture that relocates city centers must not retain anchors in their old
 // provinces. NONE makes loading derive fresh valid claims from the live TCs.
 bytes.fill(255,l.ter+8,l.ter+136);
 return bytes;
}
