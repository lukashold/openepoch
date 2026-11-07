// Optional browser projection of the original aircraft animation audio.
const manifestPath='local-original/air-flight-audio/manifest.json';
const manifestHash='6f7f187a80af5248e912984a1ca20c99c29d62021416b13e9360a05d069b5119';
const digest=/^[a-f0-9]{64}$/;
const oneShotMinimum=Math.fround(.01), loopMinimum=Math.fround(.05);
const stateNames=[null,'walk','attack1','death','takeoff','parked','idle'];
import {sourceRenderSeconds} from './source-solo-clock.js?v=native40clock';

const stepSeconds=Math.fround(.05);

const audioFamily=definition=>{
  if(definition===37||definition===39||definition===44)return 37;
  if(definition===38||definition===40)return 38;
  return definition;
};

async function verifiedBytes(url,hash,fetchFile){
  url.searchParams.set('v',hash.slice(0,16));
  const response=await fetchFile(url);
  if(!response.ok)throw Error('Original flight audio unavailable');
  const bytes=await response.arrayBuffer();
  const actual=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if(Array.from(actual,byte=>byte.toString(16).padStart(2,'0')).join('')!==hash)
    throw Error('Original flight audio checksum differs');
  return bytes;
}

export async function loadAirFlightAudio({enabled,base=new URL('./',import.meta.url),fetchFile=globalThis.fetch}={}){
  if(!enabled)return null;
  try{
    const raw=await verifiedBytes(new URL(manifestPath,base),manifestHash,fetchFile);
    const catalog=JSON.parse(new TextDecoder().decode(raw));
    if(catalog.format!=='ee2-native-air-flight-audio-v1'||catalog.rows.length!==15)return null;
    const events=new Map();
    for(const [name,entry] of Object.entries(catalog.events)){
      if(!/^local-original\/(?:audio|air-flight-audio\/audio)\/[a-z0-9_./-]+\.(?:wav|mp3)$/i.test(entry.url)||
        entry.url.includes('..')||!digest.test(entry.sha256)||!Number.isFinite(entry.gain)||
        !Number.isInteger(entry.maxInstances)||entry.maxInstances<1)return null;
      events.set(name,{...entry,url:new URL(entry.url,base)});
    }
    const rows=new Map(catalog.rows.map(row=>[audioFamily(row.definition)+':'+row.epoch,row]));
    if(rows.size!==15)return null;
    return {events,row:(definition,epoch)=>rows.get(audioFamily(definition)+':'+epoch)};
  }catch{return null;}
}

export function addCarrierFlightAudio(flight, carrier){
  if(!flight||!carrier)return flight??carrier;
  return {events:new Map([...flight.events,...carrier.events]),
    row:(definition,epoch)=>flight.row(definition,epoch)??carrier.row(definition,epoch)};
}

export function readAirFlightAudio(wasm,alpha){
  const sourceSeconds=sourceRenderSeconds(wasm,alpha);
  const count=wasm.air_audio_prepare(alpha)>>>0;
  const currentPtr=wasm.air_audio_current_ptr()>>>0;
  const eventCount=wasm.air_audio_transition_count()>>>0;
  const eventPtr=wasm.air_audio_transition_ptr()>>>0;
  const memory=wasm.memory.buffer;
  if(count>8192||eventCount>16384||currentPtr+count*40>memory.byteLength||eventPtr+eventCount*24>memory.byteLength)
    throw Error('Invalid aircraft audio snapshot');
  const view=new DataView(memory);
  const u32=offset=>view.getUint32(offset,true),f32=offset=>view.getFloat32(offset,true);
  const current=[],transitions=[];
  for(let i=0;i<count;i++){
    const at=currentPtr+40*i;
    const row={handle:u32(at),definition:view.getUint16(at+4,true),team:view.getUint8(at+6),
      epoch:view.getUint8(at+7),state:stateNames[view.getUint8(at+8)],mode:view.getUint8(at+9),
      revision:u32(at+12),at:f32(at+16),duration:f32(at+20),renderX:f32(at+24),renderY:f32(at+28),
      renderAltitude:f32(at+32),hpRatio:f32(at+36)};
    if(row.state)current.push(row);
  }
  for(let i=0;i<eventCount;i++){
    const at=eventPtr+24*i;
    const stamp=sourceSeconds===null?Math.fround(Math.fround(u32(at+4))*stepSeconds):wasm.air_audio_transition_seconds(i);
    const row={revision:u32(at),at:stamp,handle:u32(at+8),
      definition:view.getUint16(at+12,true),epoch:view.getUint8(at+14),state:stateNames[view.getUint8(at+15)],
      duration:f32(at+16),mode:view.getUint8(at+20)};
    if(row.state)transitions.push(row);
  }
  return {current,transitions,actors:current,now:sourceSeconds??Math.fround(Math.fround((wasm.game_tick()>>>0)+alpha)*stepSeconds)};
}

// 005FDF10 folds the whole delta and submits at most the old tail and new head.
function advanceText(clock,delta,period,mode,keys,emit){
  const next=Math.fround(clock+delta);
  const scan=(first,last)=>{
    for(const key of keys)if(first<=key.time&&key.time<last)emit(key);
  };
  if(mode===0&&next>=period){
    const wrapped=Math.fround(next%period);
    scan(clock,period);
    scan(0,wrapped);
    return wrapped;
  }
  scan(clock,next);
  return next;
}

export function playbackRate(word,event){
  const variance=Math.fround(event.pitchVariation);
  const exact=22050*(1-variance)+word*(2*variance)*Math.fround(22050/32767);
  const base=Math.floor(exact), fraction=exact-base;
  return (fraction>.5||fraction===.5&&base%2===1?base+1:base)/event.sampleRate;
}

// Actor rows are current local/visible render metadata. Records are accepted
// original-equivalent state transitions, in authoritative timestamp order.
// Caller rehydrates current accepted states after a load or pause.
export function createAirFlightPlayback({presentation,context,destination,mix,fetchFile=globalThis.fetch,
  audioSeed=12345}){
  const buffers=new Map(), poses=new Map(), loops=new Set(), oneShots=new Map();
  const events=presentation.events;
  let session=null,lastTime=null,pitchSeed=audioSeed>>>0;
  const ready=Promise.all([...events].map(async([name,event])=>{
    try{buffers.set(name,await context.decodeAudioData(await verifiedBytes(new URL(event.url),event.sha256,fetchFile)));}
    catch{/* Keep the existing fallback when a local optional sample is absent. */}
  }));
  function stopVoice(voice){
    try{voice.source.stop();}catch{}
    voice.source.disconnect();voice.gain.disconnect();voice.panner?.disconnect();
  }
  function writeMix(voice,volume,pan){
    voice.gain.gain.value=volume;
    if(voice.panner)voice.panner.pan.value=pan;
  }
  function createVoice(name,volume,pan,loop){
    const event=events.get(name),buffer=buffers.get(name);
    if(!buffer||context.state!=='running')return null;
    const source=context.createBufferSource(),gain=context.createGain(),panner=context.createStereoPanner?.();
    const voice={source,gain,panner};
    source.buffer=buffer;source.loop=loop;source.connect(gain);
    if(panner)gain.connect(panner);
    (panner??gain).connect(destination);
    writeMix(voice,volume,pan);
    if(event.pitchVariation>0){
      pitchSeed=(Math.imul(pitchSeed,1664525)+1013904223)>>>0;
      source.playbackRate.value=playbackRate((pitchSeed>>>16)&0x7fff,event);
    }
    source.start();
    return voice;
  }
  function oneShot(name,spatial){
    if(!buffers.has(name))return;
    const event=events.get(name),active=oneShots.get(name)??new Set();
    if(active.size>=event.maxInstances){
      const oldest=active.values().next().value;
      active.delete(oldest);stopVoice(oldest);
    }
    if(spatial.volume<=oneShotMinimum)return;
    const voice=createVoice(name,Math.fround(spatial.volume*event.gain),spatial.pan,false);
    if(!voice)return;
    active.add(voice);oneShots.set(name,active);
    voice.source.onended=()=>{
      active.delete(voice);
      voice.source.disconnect();voice.gain.disconnect();voice.panner?.disconnect();
    };
  }
  function stopLoop(pose,name){
    const loop=pose.loops.get(name);
    if(!loop)return;
    if(loop.voice)stopVoice(loop.voice);
    pose.loops.delete(name);loops.delete(loop);
  }
  function startLoop(pose,name){
    if(pose.loops.has(name))return;
    const loop={handle:pose.handle,name,voice:null,fadeEnd:null,spatial:{volume:0,pan:0}};
    pose.loops.set(name,loop);loops.add(loop);
  }
  function clearPose(pose){
    for(const name of pose.loops.keys())stopLoop(pose,name);
  }
  function reset(){
    for(const pose of poses.values())clearPose(pose);
    for(const active of oneShots.values())for(const voice of active)stopVoice(voice);
    poses.clear();oneShots.clear();session=null;lastTime=null;
  }
  function replace(record){
    const previous=poses.get(record.handle);
    if(previous)clearPose(previous);
    if(record.state==='removed'||record.state==='death'||record.state==='parked'){
      poses.delete(record.handle);return null;
    }
    const definition=presentation.row(record.definition,record.epoch)?.states[record.state];
    const period=record.duration>0?record.duration:definition?.clipDuration;
    const pose={...record,period,mode:record.mode??definition?.mode??0,keys:definition?.keys??[],
      time:record.at,clock:0,loops:new Map()};
    poses.set(record.handle,pose);
    return pose;
  }
  function advance(pose,until,actor,view,restore=false){
    if(!(until>pose.time)||!pose.keys.length||!(pose.period>0))return;
    const delta=Math.fround(until-pose.time);
    pose.clock=advanceText(pose.clock,delta,pose.period,pose.mode,pose.keys,key=>{
      if(key.type==='STOP_SOUND')stopLoop(pose,key.event);
      else if((key.type==='START_SOUND'||key.type==='START_SOUND_UNTIL_ANIM_CHANGE')&&pose.alive!==false)startLoop(pose,key.event);
      else if(key.type==='PLAY_SOUND'&&!restore&&actor)oneShot(key.event,mix(view,actor,true));
    });
    pose.time=until;
  }
  function reconcile(actors,view){
    const now=context.currentTime,candidates=[], counts=new Map();
    for(const loop of loops){
      if(loop.voice&&loop.fadeEnd===null)counts.set(loop.name,(counts.get(loop.name)??0)+1);
      const actor=actors.get(loop.handle);
      if(actor)loop.spatial=mix(view,actor,false);
      if(actor?.hpRatio>0&&loop.spatial.volume>loopMinimum&&buffers.has(loop.name))
        candidates.push(loop);
    }
    const priority=loop=>Math.fround(.5-(counts.get(loop.name)??0)/32+
      loop.spatial.volume*(loop.voice&&loop.fadeEnd===null?1.25:1));
    candidates.sort((a,b)=>priority(b)-priority(a));
    const admitted=new Set(),selected=new Map();
    for(const loop of candidates){
      if(admitted.size>=16)break;
      const count=selected.get(loop.name)??0;
      if(count>=events.get(loop.name).maxInstances)continue;
      admitted.add(loop);selected.set(loop.name,count+1);
    }
    for(const loop of admitted){
      const {volume,pan}=loop.spatial,gain=Math.fround(volume*events.get(loop.name).gain);
      if(!loop.voice)loop.voice=createVoice(loop.name,gain,pan,true);
      else writeMix(loop.voice,gain,pan);
      loop.fadeEnd=null;
    }
    for(const loop of loops){
      if(!admitted.has(loop)&&loop.voice){
        const {volume,pan}=loop.spatial;
        loop.fadeEnd??=now+1;
        const left=loop.fadeEnd-now;
        if(left>0)writeMix(loop.voice,Math.fround(left*volume),pan);
        else{stopVoice(loop.voice);loop.voice=null;}
      }
    }
  }
  function frame({world,now,actors,view,transitions=[],current=[],running}){
    if(!running||context.state!=='running'){reset();return;}
    const byHandle=new Map(actors.map(actor=>[actor.handle,actor]));
    if(session!==world||lastTime===null||now<lastTime){
      reset();session=world;
      for(const record of current){
        if(record.at>now)continue;
        const pose=replace(record);
        if(pose)advance(pose,now,byHandle.get(record.handle),view,true);
      }
    }else{
      for(const record of transitions){
        if(record.at<lastTime||record.at>now)continue;
        const previous=poses.get(record.handle);
        if(previous&&previous.at===record.at&&previous.state===record.state&&previous.revision===record.revision)continue;
        if(previous)advance(previous,record.at,byHandle.get(record.handle),view);
        replace(record);
      }
      for(const record of current){
        if(record.at>now||poses.has(record.handle))continue;
        const pose=replace(record);
        if(pose)advance(pose,now,byHandle.get(record.handle),view,true);
      }
      for(const pose of poses.values())advance(pose,now,byHandle.get(pose.handle),view);
    }
    lastTime=now;
    reconcile(byHandle,view);
    for(const [handle,pose] of poses){
      if(!byHandle.has(handle)&&![...pose.loops.values()].some(loop=>loop.voice)){
        clearPose(pose);poses.delete(handle);
      }
    }
  }
  function handlesAttack(definition,epoch,duration){
    if(context.state!=='running')return false;
    const keys=presentation.row(definition,epoch)?.states.attack1?.keys.filter(key=>key.type==='PLAY_SOUND');
    return !!keys?.length&&keys.every(key=>key.time<duration&&buffers.has(key.event));
  }
  return {ready,frame,reset,hasEvent:name=>buffers.has(name),handlesAttack};
}
