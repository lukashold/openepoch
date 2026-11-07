// Active VisualShimmer SpyShimmer descriptor and its normal branch (0065D4C0).
const spyDefinition=86;
const f32=Math.fround;
const cutoff=f32(.001), blendRate=f32(.1);
const ownOpacity=f32(.3), ownAmplitude=f32(.2), frequency=f32(2*Math.PI)/4;
const initial=Object.freeze({opacity:1,amplitude:0,emissive:0});

function blend(previous,target){
 const difference=target-previous;
 return Math.abs(f32(difference))<=cutoff?target:f32(previous+difference*blendRate);
}

export function stepSpyShimmer(previous=initial,{own,disguised,worldTime,forceOpaque=false}){
 if(forceOpaque)return {opacity:1,amplitude:previous.amplitude,emissive:0};
 const amplitude=blend(previous.amplitude,own&&disguised?ownAmplitude:0);
 const base=blend(previous.opacity,disguised?(own?ownOpacity:0):1);
 const wave=amplitude>0?amplitude*.5*(Math.cos(frequency*f32(worldTime))+1):0;
 const opacity=Math.max(0,Math.min(1,f32(base+wave)));
 return {opacity,amplitude,emissive:own?f32(1-opacity):0};
}

// Feed only current renderer-admitted actors. One outer frame can draw the same
// world in color, shadow, water and PIP. Only the first pass may advance it.
export function createSpyShimmerFrames(){
 let token=null,viewer=null,time=0,previous=new Map(),current=new Map();
 function reset(){token=null;viewer=null;previous.clear();current.clear();}
 function begin(nextToken,nextViewer,worldTime){
  if(!Number.isSafeInteger(nextToken)||nextToken<0||!Number.isInteger(nextViewer)||nextViewer<0||!Number.isFinite(worldTime)){
   reset();return;
  }
  if(nextToken===token&&nextViewer===viewer)return;
  previous=nextViewer===viewer&&nextToken===token+1?current:new Map();
  current=new Map();token=nextToken;viewer=nextViewer;time=worldTime;
 }
 function sample(actor,disguised,forceOpaque=false){
  if(token===null||actor?.def!==spyDefinition||!(actor.hpRatio>0)||typeof disguised!=='boolean' ||
     !Number.isInteger(actor.handle)||actor.handle<0||actor.handle>=0xffffffff||!Number.isInteger(actor.team)||actor.team<0)return null;
  const {handle,team}=actor, last=current.get(handle);
  if(last)return last.team===team?last.shimmer:null;
  const old=previous.get(handle);
  const shimmer=Object.freeze(stepSpyShimmer(old?.team===team?old.shimmer:undefined,
   {own:team===viewer,disguised,worldTime:time,forceOpaque}));
  current.set(handle,{team,shimmer});
  return shimmer;
 }
 return Object.freeze({begin,sample,reset});
}
