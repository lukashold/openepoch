// Source-slot fixtures; native epoch/context selection is checked separately.
import fs from 'node:fs';
import {ORIGINAL_GROUND_SHADERS,groundPlacement} from '../docs/original-ground.js';
if(!process.argv[2])throw new Error('Usage: node export_original_ground_fixture.mjs output.json');
const manifest=JSON.parse(fs.readFileSync(new URL('../docs/local-original/ground-decals.json',import.meta.url),'utf8'));
const fixtures=[];
const seasonalPairs=new Set();
for(const entry of manifest.entries){
 const instance=new Float32Array([128,128,0,0,entry.id,0,1,64,4,0,0,0]);
 const placement=groundPlacement(entry,instance);
 for(const [state,groups]of [['Idle',entry],['Death',entry.death]])for(const category of ['rural','urban']){
  for(const sprite of groups?.[category]||[]){
   fixtures.push({id:entry.id,decal:entry.decal,state,category,slot:sprite.sourceSlot,sprite:sprite.url,placement});
   const urls=sprite.seasonalUrls||[];
   for(let i=0;i<urls.length;i++){
    const next=urls[(i+1)%urls.length],key=JSON.stringify([urls[i],next]);
    if(urls[i]===next||seasonalPairs.has(key))continue;seasonalPairs.add(key);
    fixtures.push({id:entry.id,decal:entry.decal,state,category,slot:sprite.sourceSlot,sprite:urls[i],nextSprite:next,blend:.5,sourceFrame:i,sourceFrameCount:urls.length,placement});
   }
  }
 }
}
fs.writeFileSync(process.argv[2],JSON.stringify({programs:[{name:'original-ground',...ORIGINAL_GROUND_SHADERS}],fixtures}));
