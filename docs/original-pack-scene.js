import {addF64,storeF32} from './source-solo-clock.js?v=native44';

const BLEND_SECONDS=Math.fround(.15);
const add=(a,b)=>storeF32(addF64(a,b));

export function createNativePackSceneClock(revisionFor,isAlive){
 const records=new Map();
 let scene=0,world=0,generation=0;

 return {
  reset(now){records.clear();scene=0;world=now;generation=0;},
  advance(seconds,now){
   // 00605F70 advances the scene by the sum of admitted game steps.
   scene=add(scene,seconds);world=now;generation++;
   for(const handle of records.keys())if(!isAlive(handle))records.delete(handle);
  },
  pose(actor,variant,{clip,once,phase,span,packPose},capture){
   const {handle,pack}=actor,revision=revisionFor(handle)>>>0;
   const prior=records.get(handle);
   if(!packPose?.once){
    if(!capture&&prior?.generation!==generation){
     records.set(handle,{variant,clip,once,phase,revision,generation,stamp:scene});
    }
    // Ordinary motion and Attack1 keep their independent clip clock.
    return {phase,blend:null};
   }
   const key=`${clip}/${pack.start}/${pack.duration}`;
   const sameModel=prior?.variant===variant;
   const retained=sameModel&&prior.key===key&&prior.revision===revision;
   if(retained&&prior.generation===generation)return prior.result;

   const consecutive=sameModel&&prior.generation===generation-1;
   const delta=sameModel?storeF32(Math.max(0,scene-prior.stamp)):0;
   // Model reconstruction reads the committed game clock and the saved visual
   // start. Continuous objects instead consume the per-object scene delta.
   const elapsed=retained?add(prior.elapsed,delta):consecutive?delta:
    storeF32(Math.max(0,world-pack.start));
   const rate=storeF32(span/pack.duration),scaled=elapsed*rate;
   const sampled=Math.min(span,storeF32(scaled))/span;
   let outgoing=null;
   if(scaled<BLEND_SECONDS&&!pack.historyUnknown){
    if(retained)outgoing=prior.outgoing;
    else if(consecutive&&((revision-prior.revision)>>>0)===1){
     // 005FCC40 bypasses blending when a previous request is still pending.
     // Freeze the previous raw clip sample instead of the mixed display pose.
     outgoing={clip:prior.clip,once:prior.once,phase:prior.phase};
    }
   }
   const blend=outgoing?{outgoing,incomingPhase:Math.min(span,BLEND_SECONDS)/span,
    weight:storeF32(scaled/BLEND_SECONDS)}:null;
   const result={phase:sampled,blend};
   if(!capture)records.set(handle,{variant,clip,once,phase:sampled,revision,generation,stamp:scene,key,elapsed,outgoing,result});
   return result;
  },
 };
}
