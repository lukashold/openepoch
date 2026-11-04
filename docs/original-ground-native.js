// Native unit-definition factory0053A8D0 -> size payload ->00603A60.
// Every stored dimension/offset is rounded to float as in the native fields.
export const NATIVE_SIZE_FACTOR=1.4142135381698608;
export const NATIVE_SIZE_EPSILON=1.1920928955078125e-7;
const f=Math.fround;
const pair=(value=[0,0])=>value.map(f);
const scaled=value=>pair(value).map(v=>f(v*NATIVE_SIZE_FACTOR));
export function nativeUnitUndertextureSizing({size,footprint=[0,0],centerOffset=[0,0],underSize=[0,0],underOffset=[0,0],footprintType=2,decalScale=0}){
 const rawSize=pair(size),rawFootprint=pair(footprint),bodySize=footprintType===1?rawSize:scaled(rawSize);
 const footprintSize=scaled(Math.hypot(...rawFootprint)<=NATIVE_SIZE_EPSILON?rawSize:rawFootprint);
 const center=scaled(centerOffset),under=scaled(underSize),offset=scaled(underOffset);
 let dimensions,nativeOffset,mode;
 if(under[0]===0||under[1]===0){
  mode='footprint';dimensions=footprintSize.map(v=>f(v*(decalScale===0?1:f(decalScale))));
  nativeOffset=center.map((v,i)=>f(v-(footprintSize[i]-bodySize[i])*.5));
 }else{mode='explicit';dimensions=[under[1],under[0]];nativeOffset=offset;}
 //006037B0 rotates offsets by(-yaw-pi/2),004A05C0 subtracts them.
 //The00603640 pi/2 quad offset cancels the earlier angular correction.
 //In the renderer's existing(x,z,-y) basis this is local(-Y,-X).
 const localOffset=[f(-nativeOffset[1]),f(-nativeOffset[0])];
 return {mode,width:dimensions[0],depth:dimensions[1],offsetX:localOffset[0],offsetY:localOffset[1],bodySize,footprintSize,underSize:under,nativeOffset};
}
