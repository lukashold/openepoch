// The original animator freezes the outgoing clip and blends toward the
// incoming clip's first .15 seconds. The portable model stores sampled poses.
const BLEND_SECONDS=Math.fround(.15);
const sourceModels=new Set([
 '64469bfd3814a83da19cfc883c7e628ec85595576afad470a07b0c53608a8e03',
 'b9f7ebe1eff2797c7bdeb7f2291085cc3db8e4ffc9daaaf07a6cbce27126a4f2',
]);

export function hasSourcePackBlend(id,model){
 return id===22&&sourceModels.has(model.source?.sha256)
  &&model.nativeStateClips?.Pack==='native_pack'&&model.nativeStateClips?.Unpack==='native_unpack';
}

export function createNativePackBlendHistory(){
 let session,frame,previous=new Map(),current=new Map(),capture=false;
 return {
  begin(metadata,isCapture){
   capture=isCapture;
   if(!metadata||!Number.isInteger(metadata.session)||!Number.isInteger(metadata.frame))return false;
   if(capture)return session===metadata.session;
   if(session!==metadata.session){
    previous.clear();current.clear();session=metadata.session;frame=metadata.frame;
   }else if(frame!==metadata.frame){
    previous=current;current=new Map();frame=metadata.frame;
   }
   return true;
  },
  pose(actor,variant,{clip,once,phase,period,span,packPose}){
   const {handle,renderTime:time,pack}=actor;
   const transition=packPose&&packPose.once?`${clip}/${pack.start}/${pack.duration}`:null;
   const cached=current.get(handle);
   if(cached?.variant===variant&&cached.transition===transition&&cached.time===time&&cached.clip===clip)return cached.blend;
   if(capture)return null;

   let outgoing=null,blend=null;
   if(transition&&!pack.historyUnknown){
    const prior=previous.get(handle);
    const rate=Math.fround(span/pack.duration),elapsed=Math.max(0,time-pack.start)*rate;
    if(prior?.variant===variant&&elapsed<BLEND_SECONDS){
     if(prior.transition===transition){
      outgoing=prior.outgoing;
     }else{
      const gap=pack.start-prior.time;
      // Interpolation can display the old state slightly ahead of the new
      // commit. A delayed observation cannot recover the previous sample.
      if(!prior.blend&&prior.clip!==clip&&Math.abs(gap)*rate<=BLEND_SECONDS){
       const exit=prior.phase+gap/prior.period;
       outgoing={clip:prior.clip,once:prior.once,phase:prior.once?Math.min(1,Math.max(0,exit)):exit-Math.floor(exit)};
      }
     }
     if(outgoing)blend={outgoing,incomingPhase:Math.min(1,BLEND_SECONDS/span),weight:elapsed/BLEND_SECONDS};
    }
   }
   current.set(handle,{variant,clip,once,phase,period,time,transition,outgoing,blend});
   return blend;
  },
 };
}
