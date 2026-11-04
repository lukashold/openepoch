import assert from'node:assert/strict';
import fs from'node:fs/promises';
import{pathToFileURL}from'node:url';
const docs=process.env.EE2_FARM_DOCS?pathToFileURL(process.env.EE2_FARM_DOCS.replaceAll('\\','/')+'/'):new URL('../docs/',import.meta.url);
const{nativeUnitUndertextureSizing:sizing,NATIVE_SIZE_FACTOR:F}=await import(new URL('original-ground-native.js',docs));
const{groundPlacement}=await import(new URL('original-ground.js',docs));
const manifest=JSON.parse(await fs.readFile(new URL('local-original/ground-decals.json',docs)));let checks=0;
for(const entry of manifest.entries){
 assert.deepEqual(sizing(entry.nativeUnitSizing.raw),entry.nativeUnitSizing.stored);checks++;
 // Independent native basis pipeline: native Y=-world Z, Euler extraction,
 // quarter-turn correction, source rotation, SUBTRACT, then Y-up conversion.
 for(let k=0;k<8;k++){
  const yaw=k*Math.PI/4,theta=-Math.atan2(Math.sin(yaw),Math.cos(yaw))-Math.PI/2,[ox,oy]=entry.nativeUnitSizing.stored.nativeOffset;
  const nx=10-(Math.cos(theta)*ox-Math.sin(theta)*oy),ny=-20-(Math.sin(theta)*ox+Math.cos(theta)*oy);
  const instance=new Float32Array([10,20,0,yaw,entry.id,0,1,64,9,0,0,0]);const actual=groundPlacement(entry,instance);
  assert.ok(Math.abs(actual[0]-nx)<1e-6&&Math.abs(actual[1]+ny)<1e-6);assert.equal(actual[3],entry.width);assert.equal(actual[4],entry.depth);checks+=3;
 }
}
const f=Math.fround;
const circle=sizing({size:[2,3],footprint:[4,5],footprintType:1,centerOffset:[.25,-.5],decalScale:2});
assert.deepEqual(circle.bodySize,[2,3]);assert.equal(circle.width,f(f(4*F)*2));assert.equal(circle.depth,f(f(5*F)*2));assert.deepEqual(circle.nativeOffset,[f(f(.25*F)-(f(4*F)-2)*.5),f(f(-.5*F)-(f(5*F)-3)*.5)]);checks+=4;
const partial=sizing({size:[1.5,1.5],underSize:[5,0],underOffset:[10,10]});assert.equal(partial.mode,'footprint');assert.equal(partial.width,f(1.5*F));assert.deepEqual(partial.nativeOffset,[0,0]);checks+=3;
const explicit=sizing({size:[1,2],underSize:[3,5],underOffset:[.25,-.5],decalScale:4});assert.equal(explicit.width,f(5*F));assert.equal(explicit.depth,f(3*F));assert.equal(explicit.offsetX,f(.5*F));assert.equal(explicit.offsetY,f(-.25*F));checks+=4;
console.log(JSON.stringify({status:'passed',build:'water-farm3',suite:'native-unit-ground-sizing',checks,bindings:manifest.entries.length,scope:'All14 source entries,112 native-basis rotations plus exact float field/fallback/circle/partial-size fixtures; no screenshot parity claim'}));
