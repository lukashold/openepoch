import {loadAnimationPayload} from './original-animation-payload.js?v=citizen9';
import {originalSpatialMix} from './original-spatial-audio.js?v=native39audio';

const none=0xffffffff, tickSeconds=Math.fround(.05);
const directory='local-original/air-takeoff/';
const catalogHash='4f16711959f39bbf0984636c686ac656ef95e7aef821c5eca57315af5f3f3c5e';
const hashPattern=/^[a-f0-9]{64}$/;
const clipName='native-takeoff';

async function verifiedJSON(url,hash,fetchFile){
  url.searchParams.set('v',hash.slice(0,16));
  const response=await fetchFile(url);
  if(!response.ok)throw Error('Air takeoff asset unavailable');
  const bytes=await response.arrayBuffer();
  const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==hash)throw Error('Air takeoff asset checksum differs');
  return JSON.parse(new TextDecoder().decode(bytes));
}

export async function loadAirTakeoff({enabled,fetchFile=globalThis.fetch,base=new URL('./',import.meta.url)}={}){
  if(!enabled)return null;
  try{
    const catalog=await verifiedJSON(new URL(directory+'manifest.json',base),catalogHash,fetchFile);
    if(catalog.format!=='ee2-native-air-takeoff-v1'||catalog.gaps.length||catalog.rows.length!==15)return null;
    const rows=new Map(), overlays=new Map(), events=new Map();
    for(const [name,event] of Object.entries(catalog.events)){
      if(!/^local-original\/(?:audio|air-takeoff\/audio)\/[a-z0-9_./-]+\.(?:wav|mp3)$/i.test(event.url)||
        event.url.includes('..')||!hashPattern.test(event.sha256)||!Number.isFinite(event.volume)||
        !Number.isInteger(event.maximumInstances)||event.maximumInstances<1)return null;
      events.set(name,{...event,url:new URL(event.url,base)});
    }
    for(const row of catalog.rows){
      if(!Number.isInteger(row.definition)||row.epoch<10||row.epoch>14||!hashPattern.test(row.bodySha256)||
        !events.has(row.event)||!Number.isFinite(row.duration)||row.duration<0)return null;
      rows.set(row.definition+':'+row.epoch,row);
      for(const [lod,spec] of [['near',row.near],['far',row.far]])if(spec)overlays.set(spec.sourceSha256+':'+lod,{row,spec});
    }
    const requests=new Map();
    async function augment(model,lod){
      const binding=overlays.get(model.source?.sha256+':'+lod);
      if(!binding)return model;
      const {row,spec}=binding;
      if(spec.reuseClip&&model.parts.some(part=>part.animations?.[spec.reuseClip]?.endpointIncluded)){
        return {...model,nativeTakeoff:{clip:spec.reuseClip,duration:row.duration}};
      }
      if(spec.bindPose)return {...model,nativeTakeoff:{clip:clipName,duration:0}};
      if(!spec.url?.startsWith(directory)||spec.url.includes('..')||!hashPattern.test(spec.sha256))return model;
      try{
        if(!requests.has(spec.url))requests.set(spec.url,verifiedJSON(new URL(spec.url,base),spec.sha256,fetchFile));
        const overlay=await requests.get(spec.url);
        if(overlay.format!=='ee2-native-takeoff-overlay-v1'||overlay.source?.sha256!==model.source?.sha256||
          JSON.stringify(overlay.normalization)!==JSON.stringify(model.normalization)||overlay.parts.length!==model.parts.length||
          !overlay.animationPayload?.url?.startsWith(directory)||overlay.animationPayload.url.includes('..'))return model;
        const parts=model.parts.map((part,index)=>{
          const extra=overlay.parts[index], clip=extra.animations?.[clipName];
          if(extra.sourceNode!==part.sourceNode||extra.vertices*3!==part.positions.length||extra.indices!==part.indices.length||
            clip?.duration!==row.duration||(row.duration>0&&(!clip.endpointIncluded||clip.sampleIntervals!==clip.frames.length-1)))
            throw Error('Air takeoff pose topology differs');
          return {positions:part.positions,animations:{[clipName]:clip}};
        });
        const loaded=await loadAnimationPayload({source:model.source,parts,animationPayload:overlay.animationPayload},{base,fetchImpl:fetchFile});
        return {...model,nativeTakeoff:{clip:clipName,duration:row.duration},
          parts:model.parts.map((part,index)=>({...part,animations:{...part.animations,...loaded.parts[index].animations}}))};
      }catch{return model;}
    }
    return {events,augment,row:(definition,epoch)=>rows.get(definition+':'+epoch)};
  }catch{
    return null;
  }
}

// Source mode 0 wraps only after the endpoint. The GPU samples endpoint-
// inclusive morphs, so one pass through the normalized phase uses once=true.
export function nativeTakeoffPose(binding,elapsed){
  if(!binding||!Number.isFinite(elapsed)||elapsed<0)return null;
  const duration=binding.duration;
  const time=duration>0&&elapsed>duration?elapsed%duration:elapsed;
  return {clip:binding.clip,once:true,phase:duration>0?time/duration:0};
}

export function takeoffElapsed(tick,alpha,start){
  return start===none||tick<start ? -1 : Math.fround(Math.fround(tick-start+alpha)*tickSeconds);
}

export function takeoffSpatialMix(view,actor){
  return originalSpatialMix(view,actor.renderX,actor.renderY,actor.renderAltitude);
}

export function takeoffKeyCrossings(first,last,duration){
  if(!(last>first))return 0;
  if(!duration)return first===0?1:0;
  // Native text dispatch scans [oldTime,newTime), restarting at zero for a
  // wrapped positive-duration clip. No historical keys after a seek.
  let count=0;
  for(let cycle=Math.ceil(first/duration);cycle*duration<last;cycle++)count++;
  return count;
}

export function createTakeoffAudioClock({presentation,startTick,play,stop,mix=takeoffSpatialMix}){
  let previous=null;
  function reset(){previous=null;stop();}
  function update({tick,alpha,session,actors,epochs,view,running}){
    const last=previous;
    previous={tick,alpha,session,running};
    if(!running){stop();return;}
    if(!last)return;
    if(session!==last.session||tick+alpha<last.tick+last.alpha){stop();return;}
    if(!last.running||tick+alpha===last.tick+last.alpha)return;
    for(const actor of actors){
      if(actor.hpRatio<=0)continue;
      const row=presentation.row(actor.def,epochs[actor.team]);
      if(!row)continue;
      const start=startTick(actor.handle);
      if(start===none||tick<start)continue;
      const first=Math.max(0,takeoffElapsed(last.tick,last.alpha,start));
      const now=takeoffElapsed(tick,alpha,start);
      const count=takeoffKeyCrossings(first,now,row.duration);
      if(!count)continue;
      const spatial=mix(view,actor);
      if(spatial.volume<=Math.fround(.01))continue;
      for(let event=0;event<count;event++)play(row.event,spatial.volume,spatial.pan);
    }
  }
  return {update,reset};
}
