import {addF64,storeF32} from './source-solo-clock.js?v=time-of-day160';

const blendEnd=Math.fround(.15);
const unset=-1;

export function stepBirdAnimator(elapsed,delta,blending){
  // 005FC120 stores elapsed as f32, but compares the unspilled sum this frame.
  const sum=addF64(elapsed,delta);
  return {elapsed:storeF32(sum),blending:blending&&sum<blendEnd};
}

export function createBirdSceneClock(){
  let scene=0,world=0,before=0,tick=0,frame=0,delta=0;
  const birds=new Map();
  return {
    reset(committed,committedTick){
      scene=delta=0;world=before=committed;tick=committedTick;frame=0;
      birds.clear();
    },
    advance(seconds,committed,committedTick){
      // 00605F70 consumes the admitted per-frame simulation sum exactly once.
      scene=storeF32(addF64(scene,seconds));
      delta=seconds;before=world;world=committed;tick=committedTick;frame++;
    },
    sample(id,start=unset){
      const previous=birds.get(id);
      if(previous?.frame===frame&&previous.start===start)return previous;
      let pose={elapsed:0,blending:true};
      if(start!==unset){
        if(previous?.start===start||previous&&start>before){
          const delta=storeF32(Math.max(0,addF64(scene,-previous.stamp)));
          pose=stepBirdAnimator(previous.start===start?previous.elapsed:0,delta,
            previous.start===start?previous.blending:true);
        }else{
          // A newly reconstructed animator starts with the committed request offset.
          const elapsed=storeF32(Math.max(0,addF64(world,-start)));
          pose={elapsed,blending:elapsed===0};
        }
      }
      const sample={...pose,start,stamp:scene,frame};
      birds.set(id,sample);
      return sample;
    },
    inspect(){
      return {scene,world,tick,delta,rows:[...birds].map(([id,pose])=>
        [id,pose.start,pose.elapsed,pose.stamp,pose.blending])};
    },
  };
}
