import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {nativeModelTransform} from '../docs/original-scale.js';
const read=async relative=>JSON.parse(await fs.readFile(new URL('../docs/'+relative,import.meta.url),'utf8'));
const manifest=await read('local-original/models/manifest.json');
const scales=await read('local-original/model-scales.json');
const close=(a,b,tolerance=1e-6)=>assert.ok(Math.abs(a-b)<=tolerance,`${a} != ${b}`);
let checked=0,fallback=0,checks=0;const models=new Map();
for(const row of manifest.models){
 const key=`${row.id}:${row.epoch??0}`,binding=scales.variants[key];assert.ok(binding);checks++;
 let model=models.get(row.url);if(!model){model=await read(row.url);models.set(row.url,model);}
 const transform=nativeModelTransform(model,binding,7);
 if(!binding.verified){assert.deepEqual(transform,{scale:7,originOffset:[0,0,0],native:false});fallback++;checks++;continue;}
 checked++;assert.equal(transform.native,true);checks++;
 // The original source position must survive arbitrary exporter centering,
 // source scale and instance-size adapters. This catches order/sign mistakes.
 for(const source of [[0,0,0],[-13,27,5],[4,-.1,-11]])for(let axis=0;axis<3;axis++){
  const converted=(source[axis]+model.normalization.offset[axis])*model.normalization.scale;
  close((converted+transform.originOffset[axis])*transform.scale,source[axis]*binding.sourceScale);checks++;
 }
 // Own simulation footprint is not allowed to scale native artwork again.
 assert.deepEqual(nativeModelTransform(model,binding,3),nativeModelTransform(model,binding,20));checks++;
 assert.equal(nativeModelTransform(model,{...binding,sourceSha256:'mismatch'},7).native,false);checks++;
 if(row.lodFarUrl){
  const far=await read(row.lodFarUrl);assert.deepEqual(nativeModelTransform(far,binding,9),transform);checks++;
 }
}
assert.equal(checked+fallback,manifest.models.length);
const city=scales.variants['0:0'],fort=scales.variants['56:0'];
assert.equal(city.scaleType,'kScaleBuilding');assert.equal(fort.scaleType,'kScaleBuilding');
close(city.sourceScale,.04*Math.SQRT1_2);close(fort.sourceScale,.04*Math.SQRT1_2);checks+=5;
const height=b=>b.sourceBounds.max[1]-b.sourceBounds.min[1];
close(height(fort)*fort.sourceScale/(height(city)*city.sourceScale),1.4152671157249417);checks++;
assert.ok(checked>100);assert.ok(fallback>0);
console.log(JSON.stringify({suite:'native-visual-scale',checks,passed:true,verifiedVariants:checked,fallbackVariants:fallback,sourceScope:scales.scope}));
