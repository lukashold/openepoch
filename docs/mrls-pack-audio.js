const event='fx_mrls_unpack';
export const mrlsPackAudioCatalog={events:new Map([[event,{
 url:new URL('./local-original/artillery-audio/fx_mrls_unpack.mp3',import.meta.url),
 sha256:'57e264ff35f8ea95690c046ef13c6145d7497214f18cf72f4d9e21e3d93f5e52',
 gain:.6,maxInstances:3,pitchVariation:0,
}]]),row:()=>null};

export function createMrlsPackAudio({revisionFor,isAlive,groundAt,play}){
 const draws=new Map();
 let scene=0,committed=0,restoreTime=0;

 return {
  reset(now){draws.clear();scene=0;committed=restoreTime=now;},
  advance(seconds,now){
   scene=Math.fround(scene+seconds);committed=now;
   for(const handle of draws.keys())if(!isAlive(handle))draws.delete(handle);
  },
  frame({actors,epochs,view,running}){
   for(const actor of actors){
    const {handle,pack,simX,simY}=actor,epoch=epochs[actor.team];
    if(actor.def!==22||(epoch!==12&&epoch!==13)||!pack?.capable||actor.hpRatio<=0)continue;
    const revision=revisionFor(handle)>>>0,prior=draws.get(handle);
    const active=pack.phase===pack.visual&&(pack.phase===1||pack.phase===2);
    // 005FDDB0 restores the text clock with the supplied elapsed offset. A
    // restored key at zero is pending only when the restored offset is zero.
    const startsAtZero=revision!==0||(!pack.historyUnknown&&pack.start===restoreTime);
    const pending=active&&startsAtZero&&!(prior?.revision===revision&&prior.consumed);
    const positiveDelta=scene>(prior?.scene??0);
    const due=pending&&positiveDelta&&pack.start<=committed;
    draws.set(handle,{revision,scene,consumed:due||(prior?.revision===revision&&prior.consumed)});
    // The primary visible list advances the text key even if this camera's
    // spatial mix rejects it or the optional local recording is unavailable.
    if(due&&running)play(event,simX,simY,Math.fround(groundAt(simX,simY)+.5),view);
   }
  },
 };
}
