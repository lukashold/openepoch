import {advanceText} from './air-flight-audio.js?v=native66';

const f=Math.fround;
const HIDDEN=0xffffffff;
const firstEpoch=13;
const profiles={
 artemis:{event:'fx_rocket_launch',time:f(.7),
  file:'local-original/air-flight-audio/audio/fx_battle_rocket_launch2_15.mp3',
  sha256:'47ee83396f260b3bd437ca0d19b0c165e3ba7f55eaa41a84505e4e8ed7eb9341',gain:2,
  mesh:'08fbd6892e0686634177f3da5bedec0b7be12877d31c69c62a93a39a6a1a3e28',
  kf:'7ad3cba967d218c6dd86dc02fb9806eefa7fc8eeeb1108efb53570f43a7c2105'},
 minotaurus:{event:'fx_wea_laser_cannon1_4',time:f(.6),
  file:'local-original/audio/heavy-mounted3/fx_wea_laser_cannon1_4.mp3',
  sha256:'4558e4dc5b59249db842b3799047a39791ddaf73935b27f4f4587f2eca7649d9',gain:1,
  mesh:'59492da424e81b9d683f20000fcf8c35736629f95490dcb22e6d76c0582b51a3',
  kf:'f3000cf95a842afbfda7d80162ff61f4c5fe29a9a9bbe0388d88aa3a67f49330'},
};

export function createGuidedHercAttack(wasm,minotaurus,{soundReady,play}){
 const definitions=new Map([[43,profiles.artemis],[minotaurus,profiles.minotaurus]]);
 const events=new Map([...definitions.values()].map(row=>[row.event,{url:new URL(row.file,import.meta.url),
  sha256:row.sha256,gain:row.gain,maxInstances:4,pitchVariation:.4,sampleRate:44100}]));
 let poses=new Map(),last=null,reattach=false;
 function reset(seconds){poses.clear();last=seconds;reattach=Number.isFinite(seconds);}
 function ready(shot){
  if(shot===HIDDEN)return [...definitions.values()].every(row=>soundReady(row.event));
  const profile=definitions.get(shot);
  return !!profile&&soundReady(profile.event);
 }
 function update(seconds,audible){
  const count=wasm.guided_herc_attack_prepare()>>>0,ptr=wasm.guided_herc_attack_ptr()>>>0;
  const buffer=wasm.memory.buffer;
  if(!Number.isFinite(seconds)||count>8192||ptr+count*32>buffer.byteLength){reset(null);return;}
  if(last===null||seconds<last)reset(seconds);
  const delta=f(seconds-last),reader=new DataView(buffer),next=new Map();
  for(let i=0;i<count;i++){
   const at=ptr+i*32,handle=reader.getUint32(at,true),definition=reader.getUint16(at+4,true),epoch=reader.getUint8(at+6);
   const restored=reader.getUint8(at+7)===1;
   const revision=reader.getUint32(at+8,true),start=reader.getFloat32(at+12,true),duration=reader.getFloat32(at+16,true);
   const profile=definitions.get(definition);
   if(!profile||epoch<firstEpoch||!Number.isFinite(start)||!(duration>0))continue;
   const old=poses.get(handle),same=old?.revision===revision;
   // 00840190 and 005FDDB0 restore the raw elapsed cursor. 005FDF10
   // wraps it during the next active visual step, then scans its keys.
   const reattaching=!same&&restored&&reattach;
   // A newly visible old request catches up silently. A request accepted
   // during this frame receives the whole qualifying visual delta.
   const past=!same&&!reattaching&&start<last;
   const initial=same?old.text:reattaching?f(Math.max(0,last-start)):past?f(Math.max(0,seconds-start)%duration):0;
   const poseClock=same?old.pose:initial;
   const step=past?0:delta;
   const text=step>0?advanceText(initial,step,duration,0,[profile],()=>{
    if(audible)play(profile.event,reader.getFloat32(at+20,true),reader.getFloat32(at+24,true),reader.getFloat32(at+28,true));
   }):initial;
   const elapsed=f(poseClock+step);
   const pose=elapsed>duration?f(elapsed%duration):elapsed;
   next.set(handle,{definition,epoch,revision,text,pose,duration});
  }
  poses=next;last=seconds;reattach=false;
 }
 function poseFor(actor){
  const row=poses.get(actor.handle);
  if(row?.definition!==actor.def)return null;
  return {clip:'combat',once:true,phase:row.pose/row.duration};
 }
 function acceptsModel(id,model){
  const profile=definitions.get(id);
  return !!profile&&model.source?.sha256===profile.mesh&&
   !!model.animationSources?.some(clip=>clip.name==='combat'&&clip.sha256===profile.kf);
 }
 return {audio:{events,row:()=>undefined},reset,update,ready,poseFor,acceptsModel};
}
