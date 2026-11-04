// Explicit current save-format contract for controlled end-to-end fixtures.
// Legacy-format tests must truncate at the derived core end, never remove an
// assumed number of bytes from a growing save trailer.
import assert from 'node:assert/strict';
export const DIP_BYTES=262744;
export const TER_BYTES=524;
export const CITY_MAGIC=0x31444743,CITY_END=0x45444743;
const NN=256*256;
export function saveLayout(bytes){
 const d=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),players=d.getUint32(8,true),entities=d.getUint32(4,true)*d.getUint32(32,true);
 const end=36+4*players+entities+5*NN,reference=d.getUint32(0,true)===0x3252454f,prefix=reference?236:132;
 const cityBytes=bytes.length>end+prefix+DIP_BYTES+TER_BYTES&&d.getUint32(bytes.length-4,true)===CITY_END?d.getUint32(bytes.length-8,true):0;
 assert.ok(cityBytes===0||(cityBytes>=20&&cityBytes<=20+8192*8&&cityBytes<=bytes.length),'bounded CGD1 footer size');
 const legacyEnd=bytes.length-cityBytes,city=cityBytes?legacyEnd:null;
 return {d,end,prefix,dip:end+prefix,ter:end+prefix+DIP_BYTES,realFog:end+prefix+600,displayFog:end-NN,reference,legacyEnd,city,cityBytes};
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
 return l;
}

// Explicitly repair only controlled scenario edits (epochs/TC completion), never
// malformed-save validation or ordinary roundtrip loads. Preserve known lifetime
// history for matching generation handles while adding/removing fixture cities.
export function refreshFixtureCityGround(bytes){
 const l=saveLayout(bytes);if(!l.reference)return bytes;
 assert.equal(l.legacyEnd,l.ter+TER_BYTES,'scenario edits require a complete current prefix');
 const previous=new Map();
 if(l.cityBytes){const count=l.d.getUint32(l.city+8,true);for(let k=0;k<count;k++)previous.set(l.d.getUint32(l.city+12+k*8,true),l.d.getFloat32(l.city+16+k*8,true));}
 const playerBytes=l.d.getUint32(8,true),entityBytes=l.d.getUint32(4,true),high=l.d.getUint32(32,true),entities=36+4*playerBytes,records=[];
 for(let i=0;i<high;i++){
  const at=entities+i*entityBytes,team=bytes[at+2];
  if(!bytes[at]||bytes[at+1]!==0||!(l.d.getFloat32(at+80,true)>=1)||!(l.d.getFloat32(at+40,true)>0))continue;
  const handle=((l.d.getUint16(at+4,true)<<16)|i)>>>0,epoch=bytes[36+team*playerBytes+24],base=7+Math.floor(Math.min(14,epoch)/3);
  records.push([handle,Math.max(previous.get(handle)||0,base)]);
 }
 const size=20+records.length*8,result=new Uint8Array(l.legacyEnd+size),d=new DataView(result.buffer);result.set(bytes.subarray(0,l.legacyEnd));
 d.setUint32(l.legacyEnd,CITY_MAGIC,true);d.setUint32(l.legacyEnd+4,1,true);d.setUint32(l.legacyEnd+8,records.length,true);
 records.forEach(([handle,radius],i)=>{d.setUint32(l.legacyEnd+12+i*8,handle,true);d.setFloat32(l.legacyEnd+16+i*8,radius,true);});
 d.setUint32(result.length-8,size,true);d.setUint32(result.length-4,CITY_END,true);return result;
}
export function revealFixture(bytes){
 const l=assertCurrentSave(bytes);
 bytes.fill(255,l.displayFog,l.end);
 bytes.fill(255,l.realFog,l.realFog+4*NN);
 // This helper is only for controlled scenarios, which often edit epochs before
 // revealing. Radius growth must agree with those explicit fixture profiles.
 const updated=refreshFixtureCityGround(bytes);assert.equal(updated.length,bytes.length,'revealing does not add/remove fixture city records');bytes.set(updated);
 return bytes;
}
export function resetFixtureClaims(bytes){
 const l=assertCurrentSave(bytes);
 // A fixture that relocates city centers must not retain anchors in their old
 // provinces. NONE makes loading derive fresh valid claims from the live TCs.
 bytes.fill(255,l.ter+8,l.ter+136);
 return bytes;
}
