import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createNativeLightingContext,nativeModelPipeline,nativeIndexPalVertex,nativeIndexPalStage,APPLICATION_LIGHTING_PRESET} from '../docs/original-lighting.js';
const url=new URL('../docs/local-original/climate-lighting.json',import.meta.url),bytes=await fs.readFile(url),data=JSON.parse(bytes);
let checks=0;const equal=(a,b,message)=>{assert.deepEqual(a,b,message);checks++;};
const close=(a,b)=>{assert.ok(Math.abs(a-b)<2e-7,`${a} != ${b}`);checks++;};
equal(APPLICATION_LIGHTING_PRESET,{profile:'native155',climate:'Temperate',period:'Afternoon',weather:'Normal',detail:'High'});
equal(data.sceneLight.selected.Dimmer,1.5);
equal(data.nativeColorToFloat.sourceOrder,'RGBA');equal(data.nativeColorToFloat.packedOrder,'BGRA');
for(const [profile,source] of Object.entries(data.profiles))for(const [climate,periods] of Object.entries(source.climates))for(const [period,todKey] of Object.entries(periods)){
 const tod=source.timeOfDay[todKey];
 for(const [weather,rowKey] of Object.entries(tod.atmosphere))for(const detail of ['High','Low']){
  const l=createNativeLightingContext(data,{profile,climate,period,weather,detail}),row=source.atmospheres[rowKey];
  equal(l.rowKey,rowKey);equal(l.todKey,todKey);equal(l.native,true);
  for(const [runtime,key] of [['ambient','ambient'],['sun','diffuse'],['specular','specular']])for(let i=0;i<3;i++){
   const expected=Math.fround(Math.fround(row.sourceRGBA[key][i]*Math.fround(1/255))*Math.fround(1.5));equal(l[runtime][i],expected);
  }
  const norm=Math.hypot(...tod.sunDirectionSource),r=tod.sunDirectionSource.map(x=>Math.fround(x/norm));
  equal(l.direction,[-r[0],-r[2],r[1]].map(Math.fround));close(Math.hypot(...l.direction),1);
  equal(nativeModelPipeline({skinned:true},l),{shader:detail==='Low'?2:1,textureScale:4});
  equal(nativeModelPipeline({textureApplyMode:4,textureUrl:'actual.png'},l),{shader:0,textureScale:4});
  equal(nativeModelPipeline({textureApplyMode:4},l),{shader:0,textureScale:1});
  const perpendicular=[l.direction[2],0,-l.direction[0]];
  for(const [n,dot] of [[l.direction,1],[perpendicular,0],[l.direction.map(x=>-x),-1]]){
   const colour=nativeIndexPalVertex(n,l),coefficient=detail==='Low'?.5:dot;
   for(let i=0;i<3;i++)close(colour[i],Math.max(0,Math.min(1,l.ambient[i]+coefficient*l.sun[i])));
   const texel=[.25,.5,.75,.5],pixel=nativeIndexPalStage(texel,colour,.25);
   for(let i=0;i<3;i++)close(pixel[i],Math.min(1,texel[i]*colour[i]*4));equal(pixel[3],.125);
  }
 }
}
for(const settings of [{climate:'Unknown'},{period:'Dawn'},{weather:'Random'},{detail:'Ultra'},{profile:'missing'},{profile:'base',climate:'Tundra'}]){assert.throws(()=>createNativeLightingContext(data,settings),RangeError);checks++;}
for(const part of [{skinned:true},{textureApplyMode:4,textureUrl:'actual.png'},{}])equal(nativeModelPipeline(part,null),{shader:0,textureScale:1});
const result={status:'passed',build:'native5',format:'ee2-native-lighting-source-runtime-test-v1',checks,sourceSha256:createHash('sha256').update(bytes).digest('hex'),scope:'Every declared BASE/155 climate, static period, seven weather rows and High/Low selectors. Independent source-byte colour oracle, source1.5 Dimmer, direction axes, shader selection, source apply modes and exact alpha.'};
await fs.writeFile(process.argv[2]||new URL('../../EE2-Research/analysis/native5-lighting-source-tests.json',import.meta.url),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
