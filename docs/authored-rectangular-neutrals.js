import {loadOriginalModels} from './dv2-original-renderer.js?v=aircraft-curve257-private';
import {activeRectangularLayout} from './authored-rectangular-layout.js';
import {sourceForestYaw} from './original-forest-yaw.js?v=death-valley2-neutrals-yaw236';
import {portableResourceSlot,sourceMineralRenderId} from './original-mineral-variants.js?v=death-valley2-mineral-visual237';
import {createBirdSceneClock} from './dv2-bird-scene-clock.js?v=aircraft-scene258-private';

const base='local-original/death-valley2-neutrals/';
const catalogPin='c3cd9078c03783b11397c40507b044a75a1355f7e76546fc2bdffcd1e7556998';
const mineralsURL='local-original/death-valley2-minerals/manifest.json';
const mineralsPin='fef2306bd337b8b3fbfcd8cc246f3bfc033b26a1cd165d52746505d39b7f691f';
const huntURL='local-original/death-valley2-hunt/manifest.json';
const huntPin='8345589aef643e68eb33f2ff43ba3f307d8cab9a0c69caff97735caf3c822157';
const deerWalkURL='local-original/death-valley2-deer-motion/deer-walk.json';
const deerWalkPin='7afe9dfb0f10987887ed4589d5136a4ffc17a2cc0a17593f923c2a6da3882d9f';
const deerIdlePin='f794564be915d5c32fc3dbe3db7fc867fcb6749ffe836417c58267a83fe5a3f6';
const birdModels=[
  ['Hawk','hawk-flight.json','3278adf83bd9464047bffb04a9f070df84893d8281278c325a4bf1d915f0a078','4827f0de5634a636b2eb77c492f886b65704163da9297e416ef55d11af00b7e0',
    'hawk-flight-glide.json','57478dee6fc2049c90d2d0b1c2a426886ff49bca1f4fd8f243347d2bc1e8fd1e',
    'hawk-rig.json','d8e597b36d496843b6ca41752891c2e3b25d577dd6031f1c582ffe98efb7a072'],
  ['Vulture','vulture-flight.json','5c24f54165d3cb36494b82acf231b37289872024304486fe14f2597199dd9f4a','84fa948cb3a64aa639a7c02d3d87936dafdd3c849152d75a4975ef7748320e8b',
    'vulture-flight-glide.json','473b85812dab3468cba5af420ce7231dbe0bf99a9a8ae8b54b611ed3c89a8b7a',
    'vulture-rig.json','ad5ddeae2ef8b2c048aaf995c265bf7350aea91c016269010777460c7b667e4d'],
];
const populationPin='78d729c50ddc80fe833f2f66b69f6a822060bfbe068bf10ce8320a264750798e';
const missing=0xffffffff, noPick=0xffff, corpseKey=0x4000, forestKey=0x8000;
const readWord=new DataView(new ArrayBuffer(4));
const epochs=[0,0,0,0];
const fishKind=3, liveAnimalKind=4;
const aircraftGlideActivity=7, initialAircraftBlend=Math.fround(0.15);
const archivedUnitCount=766, maximumUnitCount=8192;

async function read(url){
  const response=await fetch(new URL(url,import.meta.url),{cache:'no-store'});
  if(!response.ok)throw Error('Originale Kartenobjekte fehlen.');
  return new Uint8Array(await response.arrayBuffer());
}

async function digest(bytes){
  return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),byte=>byte.toString(16).padStart(2,'0')).join('');
}

function float(word){
  readWord.setUint32(0,word,true);
  return readWord.getFloat32(0,true);
}

function sourceTypes(bytes){
  const header=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  const decoder=new TextDecoder();
  const names=[];
  let offset=112;
  for(let i=0;i<header.getUint32(20,true);i++){
    const length=header.getUint16(offset,true);
    names.push(decoder.decode(bytes.subarray(offset+2,offset+2+length)));
    offset+=length+2;
  }
  return names;
}

function groupFrame(groups){
  const ranges=new Uint32Array(groups.length*2);
  const count=groups.reduce((n,group)=>n+group.keys.length,0);
  const data=new Float32Array(count*12),resourceIdentities=new Uint16Array(count);
  let index=0;
  for(let id=0;id<groups.length;id++){
    const {poses,keys}=groups[id];
    ranges[id*2]=index;ranges[id*2+1]=keys.length;
    data.set(poses,index*12);resourceIdentities.set(keys,index);
    index+=keys.length;
  }
  return {data,ranges,resourceIdentities,count};
}

/** Scope live rows to the active rectangular map and keep the two source ID namespaces separate. */
export function rectangularNeutralFrame(wasm,bindings,{bounds,visibility,layout,local=0,time=0,playerEpochs=epochs,corpseModels,deerWalkType=-1,birdTypes,birdAlternateTypes,birdRigTypes,birdClock,wingTime=time}){
  const groups=Array.from({length:bindings.length},()=>({poses:[],keys:[]}));
  const unitCount=wasm.native_rect_world0_unit_count?.()??0;
  const birthCount=wasm.native_rect_world0_birth_count?.()??0;
  const [x0,y0,x1,y1]=bounds;
  const inside=(x,y)=>x>=Math.max(0,x0)&&y>=Math.max(0,y0)&&x<Math.min(layout.width,x1)&&y<Math.min(layout.height,y1);
  const put=(id,x,y,z,yaw,key,walkTime,activity=1,birdRig=false)=>{
    const walking=Number.isFinite(walkTime)&&walkTime>=0;
    groups[id].poses.push(x,y,z,yaw,id,0,0,0,1,Number(birdRig),walking?walkTime:time,walking?activity:0);
    groups[id].keys.push(key);
  };
  const sharedEpoch=Math.max(...playerEpochs),viewerEpoch=playerEpochs[local]??0;
  const fishIdentities=new Set(), huntIdentities=new Set(), corpseTargets=new Map();
  let units=0,forest=0,corpses=0,flying=0;
  const flightClips={walk:0,'walk-alt':0,legacy:0,pending:0,rest:0,alternateFallback:0};
  if(Number.isInteger(unitCount)&&unitCount>=archivedUnitCount&&unitCount<=maximumUnitCount&&
      birthCount===unitCount-archivedUnitCount){
    const word=wasm.native_rect_world0_unit_word;
    for(let slot=0;slot<unitCount;slot++){
      const type=word(slot,1)>>>0;
      const resource=word(slot,3)>>>0,identity=word(slot,0)>>>0;
      if(slot>=archivedUnitCount&&(word(slot,37)!==1||word(slot,2)!==liveAnimalKind||identity>=corpseKey))continue;
      const mineral=resource>=5&&resource<=7;
      const model=mineral?sourceMineralRenderId(resource,identity,viewerEpoch,sharedEpoch):type;
      if(model===null||!bindings[model])continue;
      const x=float(word(slot,4)),y=float(word(slot,5));
      if(!inside(x,y))continue;
      const fog=word(slot,12)>>>0,kind=word(slot,2)>>>0;
      if(fog===0||fog>255)continue;
      let huntable=false;
      if(corpseModels&&kind===liveAnimalKind){
        const corpseModel=corpseModels.get(type);
        const hpWord=word(slot,13)>>>0;
        if(hpWord!==missing&&float(hpWord)<=0){
          const corpse=word(slot,14)>>>0;
          if(corpseModel!==undefined&&bindings[corpseModel]&&corpse!==0&&corpse!==missing&&corpse!==identity&&float(word(slot,15))>0){
            const token=fog===255?corpseKey+slot:noPick;
            if(token!==noPick)corpseTargets.set(token,corpse);
            put(corpseModel,x,y,float(word(slot,6)),float(word(slot,7)),token);
            units++;corpses++;
          }
          continue;
        }
        huntable=corpseModel!==undefined&&identity<corpseKey&&word(slot,16)===1;
      }
      if(kind===liveAnimalKind&&fog!==255)continue;
      const initial=word(slot,9)>>>0,remaining=initial>0?float(word(slot,10)):0;
      if(initial>0&&!(remaining>0))continue;
      const slotResource=portableResourceSlot(resource);
      const canGather=slotResource!==null&&wasm.resource_active?.(local,slotResource)!==0;
      if(kind===2&&!mineral&&!canGather)continue;
      const fish=kind===fishKind;
      const key=huntable||initial>0&&resource!==7&&identity<corpseKey&&(fish||fog===255)&&canGather?identity:noPick;
      if(fish&&key!==noPick)fishIdentities.add(key);
      if(huntable)huntIdentities.add(key);
      let walkTime=kind===liveAnimalKind&&type===deerWalkType?float(word(slot,17)):NaN;
      let activity=1,birdRig=false;
      if(kind===liveAnimalKind&&birdTypes?.has(type)){
        const phase=word(slot,40)>>>0;
        if(phase===0)birdClock?.sample(identity);
        if(phase>=1&&phase<=3){
          flying++;
          const binding=word(slot,56)>>>0;
          birdRig=binding<=2&&birdRigTypes?.has(type)===true;
          if(binding===missing){
            walkTime=wingTime;
            flightClips.legacy++;
            flightClips.walk++;
          }else if(binding===2){
            flightClips.pending++;
            birdClock?.sample(identity);
          }else if(binding===0||binding===1){
            const start=float(word(slot,57));
            const pose=birdClock?.sample(identity,start);
            const elapsed=pose?.elapsed??Math.max(0,wingTime-start);
            // While the first Walk bind is blending, the previous KF is absent.
            if((pose?!pose.blending:elapsed>=initialAircraftBlend)&&Number.isFinite(elapsed)){
              walkTime=elapsed;
              if(binding===1&&birdAlternateTypes?.has(type)){
                activity=aircraftGlideActivity;
                flightClips['walk-alt']++;
              }else{
                flightClips.walk++;
                if(binding===1)flightClips.alternateFallback++;
              }
            }else flightClips.rest++;
          }
        }
      }
      put(model,x,y,float(word(slot,6)),float(word(slot,7)),key,walkTime,activity,birdRig);
      units++;
    }
  }
  const word=wasm.native_rect_forest_source_word;
  if(typeof word==='function')for(let ordinal=0;ordinal<1328;ordinal++){
    const qx=word(ordinal,0)>>>0,qy=word(ordinal,1)>>>0;
    if(qx===missing||!inside(qx/128,qy/128))continue;
    const x=qx/128,y=qy/128,fog=visibility[(qy>>>7)*layout.stride+(qx>>>7)];
    if(!fog)continue;
    const state=word(ordinal,5)>>>0;
    if(state!==0&&state!==3)continue;
    const type=word(ordinal,3)>>>0,phase=wasm.native_rect_forest_actor_word?.(ordinal,0)>>>0;
    const stump=state===3||phase===2,id=stump?37+type:type;
    if(!bindings[id])continue;
    const key=!stump&&fog===255&&(phase===0||phase===1)?forestKey+ordinal:noPick;
    put(id,x,y,(word(ordinal,2)>>>0)/128,sourceForestYaw(qx,qy),key);
    forest++;
  }
  return {...groupFrame(groups),units,forest,corpses,flying,flightClips,fishIdentities,huntIdentities,corpseTargets};
}

export async function loadRectangularNeutralModels(gl,teams,population,{hunting=false,flying=false}={}){
  if(await digest(population)!==populationPin)return null;
  const names=sourceTypes(population);
  const raw=await read(base+'manifest.json');
  if(await digest(raw)!==catalogPin)throw Error('Die Originalmodellliste stimmt nicht mit Death Valley 2 überein.');
  const decoder=new TextDecoder(),manifest=JSON.parse(decoder.decode(raw));
  const scaleRaw=await read(base+'scales.json');
  if(await digest(scaleRaw)!==manifest.scales||manifest.population!==populationPin)throw Error('Die Originalmodellgrößen stimmen nicht.');
  const scales=JSON.parse(decoder.decode(scaleRaw)),bindings=Array(85).fill(false);
  const assetMapping=manifest.models.map(row=>{
    if(row.sourceType!==names[row.typeIndex]||row.id!==row.typeIndex+(row.stump?37:0)||bindings[row.id])throw Error('Ungültige Originalmodellbindung.');
    bindings[row.id]=true;
    return {renderId:row.id,kind:1,sourceId:row.id};
  });
  const mineralRaw=await read(mineralsURL).catch(()=>null);
  if(mineralRaw){
    if(await digest(mineralRaw)!==mineralsPin)throw Error('Die Originalmodelle der Erzvorkommen stimmen nicht.');
    const pack=JSON.parse(decoder.decode(mineralRaw));
    if(pack.population!==populationPin)throw Error('Die Erzmodelle passen nicht zur Originalkarte.');
    for(const row of pack.models){
      if(row.id<74||row.id>=82||bindings[row.id])throw Error('Ungültige Erzmodellbindung.');
      bindings[row.id]=true;
      assetMapping.push({renderId:row.id,kind:1,sourceId:row.id});
    }
    manifest.models.push(...pack.models);
    Object.assign(manifest.texturePins,pack.texturePins);
    Object.assign(scales.variants,pack.variants);
  }
  const corpseModels=hunting?new Map():null;
  const birdTypes=new Set(),birdAlternateTypes=new Set(),birdRigs=new Map();
  let deerWalkType=-1;
  if(corpseModels){
    for(const name of ['Tiger','WhiteWolf','Wolf']){
      const dead=manifest.models.find(row=>row.sourceType==='DeadAnimal'+name);
      const live=names.indexOf(name);
      if(dead&&live!==-1)corpseModels.set(live,dead.id);
    }
    const huntRaw=await read(huntURL).catch(()=>null);
    if(huntRaw&&await digest(huntRaw)===huntPin){
      const pack=JSON.parse(decoder.decode(huntRaw));
      if(pack.population!==populationPin)throw Error('Die Jagdmodelle passen nicht zur Originalkarte.');
      for(const row of pack.models){
        const type=names.indexOf(row.sourceAnimal);
        if(type===-1||row.id<82||row.id>=85||bindings[row.id]||corpseModels.has(type))throw Error('Ungültige Jagdmodellbindung.');
        bindings[row.id]=true;
        corpseModels.set(type,row.id);
        assetMapping.push({renderId:row.id,kind:1,sourceId:row.id});
      }
      manifest.models.push(...pack.models);
      Object.assign(manifest.texturePins,pack.texturePins);
      Object.assign(scales.variants,pack.variants);
    }
    const deer=manifest.models.find(row=>row.sourceType==='Deer'&&row.sha256===deerIdlePin);
    const deerWalk=deer&&await read(deerWalkURL).catch(()=>null);
    if(deerWalk&&await digest(deerWalk)===deerWalkPin){
      deer.url=deerWalkURL;deer.sha256=deerWalkPin;deerWalkType=deer.typeIndex;
    }
  }
  if(flying)await Promise.all(birdModels.map(async ([name,file,sourcePin,pin,glideFile,glidePin,rigFile,rigPin])=>{
    const row=manifest.models.find(model=>model.sourceType===name&&model.sha256===sourcePin);
    if(!row)return;
    const glide=await read(base+glideFile).catch(()=>null);
    const url=base+(glide?glideFile:file),expected=glide?glidePin:pin;
    const bytes=glide??await read(url).catch(()=>null);
    if(!bytes)return;
    if(await digest(bytes)!==expected)throw Error('Die Originalflügelanimation stimmt nicht.');
    row.url=url;row.sha256=expected;
    birdTypes.add(row.typeIndex);
    if(glide){
      birdAlternateTypes.add(row.typeIndex);
      const rig=await read(base+rigFile).catch(()=>null);
      if(rig){
        if(await digest(rig)!==rigPin)throw Error('Das Original-Vogelskelett stimmt nicht.');
        birdRigs.set(row.typeIndex,{source:JSON.parse(decoder.decode(rig)),modelPin:expected});
      }
    }
  }));
  const renderer=await loadOriginalModels(gl,true,teams,undefined,{assetMapping,initialSails:false,nativeLighting:false,privateModelPack:{manifest,scales},birdRigs});
  if(renderer.errors.length||renderer.models.size!==manifest.models.length)throw Error(renderer.errors.join('\n')||'Originalmodelle fehlen.');
  const birdClock=createBirdSceneClock();
  let fishIdentities=new Set(),huntIdentities=new Set(),corpseTargets=new Map();
  return {
    resetFlightClock(wasm){birdClock.reset(wasm.sim_elapsed_seconds(),wasm.game_tick()>>>0);},
    advanceFlightClock(wasm,seconds){birdClock.advance(seconds,wasm.sim_elapsed_seconds(),wasm.game_tick()>>>0);},
    inspectFlightClock(){return birdClock.inspect();},
    draw(wasm,options){
      if(!activeRectangularLayout(wasm))return null;
      const time=wasm.sim_elapsed_seconds?.()??0;
      const wingTime=wasm.sim_render_seconds?.(options.alpha??1)??time;
      const playerEpochs=epochs.map((_,seat)=>wasm.stat(seat,0));
      const frame=rectangularNeutralFrame(wasm,bindings,{...options,time,wingTime,playerEpochs,corpseModels,deerWalkType,birdTypes,birdAlternateTypes,birdRigTypes:birdRigs,birdClock});
      if(!options.capture)({fishIdentities,huntIdentities,corpseTargets}=frame);
      renderer.draw({...frame,vp:options.vp,eye:options.eye,zoom:options.zoom,lighting:options.lighting,epochs,
        clipPlane:options.clipPlane,capture:options.capture});
      return {units:frame.units,forest:frame.forest,corpses:frame.corpses,flying:frame.flying,flightClips:frame.flightClips,huntable:frame.huntIdentities.size,
        instances:frame.count,calls:renderer.metrics.calls,triangles:renderer.metrics.triangles};
    },
    pick(origin,direction,maxDistance,{workers=true,fishing=false}={}){
      const hit=renderer.pickResourceHit(origin,direction,maxDistance,id=>fishIdentities.has(id)?fishing:workers);
      return hit?{...hit,fish:fishIdentities.has(hit.identity),hunt:huntIdentities.has(hit.identity),
        target:corpseTargets.get(hit.identity)??hit.identity}:null;
    },
    order(wasm,key){
      if(!Number.isInteger(key)||key<0||key===noPick)return false;
      if(key>=forestKey)return wasm.cmd_native_rect_forest_gather?.(key-forestKey)===1;
      if(key>=corpseKey){const target=corpseTargets.get(key);return target!==undefined&&wasm.cmd_rect_gather?.(target)===1;}
      if(huntIdentities.has(key))return wasm.cmd_rect_hunt?.(key)===1;
      return wasm.cmd_rect_gather?.(key)===1;
    },
  };
}
