// Own Citizen text-key scene cache over sealed original304/308 facts.
// Scheduling is explicit: observe updates the MainModel, updateChildren advances
// existing attachment children, markDrawn follows an actual GL draw. Native
// Composite visits children in stored order; its Citizen factory sibling order
// is not established here. Main's model-first composition is a portable policy.
import {configureTextClock,advanceTextClock,modelTextPeriod,sourceF32,f32Bits}
  from './dv2-work-event-clock.mjs';

const UINT_MAX=0xffffffff, EPSILON=Math.fround(2**-23);
const WORK_STATES=new Set(['MineGold','MineStone','GatherSaltpeter','ChopWood','FarmHarvesting']);
const KEY_TYPES=Object.freeze({PLAY_SOUND:2,ATTACH_VISUAL:8});
const validHandle=value=>Number.isInteger(value)&&value>=0&&value<UINT_MAX;
const requireValue=(condition,message)=>{if(!condition)throw new Error('Citizen work event scene: '+message);};

export function recentlyDrawn(frame,lastDraw){return ((frame-lastDraw)>>>0)<2;}
export function cachedVisualDelta({frame,sceneTime,lastUpdateFrame,lastTime},precision){
  const delta=sourceF32(sceneTime-lastTime,precision);
  return Object.freeze({delta,update:(frame>>>0)!==(lastUpdateFrame>>>0)||delta>=EPSILON});
}

export function createCitizenWorkEventScene({precision,emit=()=>{},stateChange=()=>{},
  flushChildren=()=>{},advanceChildren=()=>{},disposeChildren=()=>{}}={}){
  requireValue(precision==='nearest'||precision==='towardZero','explicit numeric profile required');
  requireValue([emit,stateChange,flushChildren,advanceChildren,disposeChildren].every(fn=>typeof fn==='function'),
    'callbacks must be functions');
  const actors=new Map(),bindings=new WeakMap();
  let session,started=false,frame=0,total=0;

  function descriptor(work,stateName){
    if(!work||typeof work!=='object'||!work.binding||!work.states)return null;
    let states=bindings.get(work);
    if(!states){states=new Map();bindings.set(work,states);}
    if(states.has(stateName))return states.get(stateName);
    const clip=work.states.nativeStateClips?.[stateName],state=work.states.states?.[stateName];
    const duration=work.binding.clips?.[clip]?.duration;
    if(!state||!Array.isArray(state.textKeys)||!Number.isFinite(duration)||duration<=0)return null;
    // Source text-key storage and exported intrinsic durations are binary32.
    // Materialize their encoded source words independently of arithmetic RC.
    const keys=WORK_STATES.has(stateName)?state.textKeys.flatMap(key=>{
      const type=KEY_TYPES[key.name];
      return type&&Number.isFinite(key.time)?[{type,data:key.data,time:Math.fround(key.time)}]:[];
    }):[];
    const result=Object.freeze({stateName,duration:Math.fround(duration),keys:Object.freeze(keys),
      stateSet:work.binding.stateSet??null,clip});
    states.set(stateName,result);return result;
  }
  function normalize(request,work){
    if(!request||typeof request.stateName!=='string'||request.mode!==0)return null;
    const desc=descriptor(work,request.stateName);
    if(!desc||![request.span,request.started,request.rawElapsed].every(Number.isFinite)
      ||request.span<0||request.started<0||request.rawElapsed<0)return null;
    const span=Math.fround(request.span),origin=Math.fround(request.started);
    return {desc,span,started:origin,rawElapsed:Math.fround(request.rawElapsed),mode:0,
      key:[request.stateName,f32Bits(span),f32Bits(origin),desc.stateSet??''].join('/')};
  }
  function remove(handle){
    requireValue(validHandle(handle),'full actor handle required');
    if(!actors.has(handle))return false;
    disposeChildren(handle);actors.delete(handle);return true;
  }
  function reset(){
    for(const handle of actors.keys())disposeChildren(handle);
    actors.clear();session=undefined;started=false;frame=0;total=0;
  }
  function beginFrame(sessionToken,admittedSceneDelta){
    requireValue(sessionToken!==undefined&&sessionToken!==null,'explicit world/session identity required');
    requireValue(Number.isFinite(admittedSceneDelta)&&admittedSceneDelta>=0,'nonnegative admitted scene delta required');
    if(!started||sessionToken!==session){reset();session=sessionToken;started=true;}
    frame=(frame+1)>>>0;
    total=sourceF32(total+sourceF32(admittedSceneDelta,precision),precision);
    requireValue(Number.isFinite(total),'scene time overflow');
    return Object.freeze({frame,sceneTime:total});
  }
  function register(handle,identity){
    const actor={handle,modelIdentity:identity,lastUpdateFrame:UINT_MAX,lastDraw:UINT_MAX,lastTime:total,
      seenRequest:false,request:null,clock:null,pendingChildren:null,updating:false};
    actors.set(handle,actor);return actor;
  }
  function observe({handle,request,workBinding,modelIdentity,alive=true}={}){
    requireValue(started,'beginFrame must precede observation');
    requireValue(validHandle(handle),'full actor handle required');
    if(alive===false){remove(handle);return Object.freeze({handle,removed:true,updated:false});}
    const identity=modelIdentity??workBinding?.sourceSha256;
    requireValue(identity!==undefined&&identity!==null,'stable source-model identity required');
    let actor=actors.get(handle);
    if(actor&&actor.modelIdentity!==identity){remove(handle);actor=null;}
    if(!actor)actor=register(handle,identity);
    requireValue(!actor.updating,'reentrant actor update is not supported');
    const current=normalize(request,workBinding),changed=actor.seenRequest&&actor.request?.key!==current?.key;
    // Core request notification precedes the later scene update/queue flush.
    if(changed)stateChange(handle);
    let configured=null;
    if(!actor.seenRequest||actor.request?.key!==current?.key){
      if(current){
        configured=actor.seenRequest?'warm':'cold';
        const period=modelTextPeriod({span:current.span,intrinsic:current.desc.duration},precision);
        actor.clock=configureTextClock(actor.clock,{period,mode:0,
          elapsed:configured==='cold'?current.rawElapsed:0,keys:current.desc.keys},precision);
      }else actor.clock=null;
      actor.request=current;actor.seenRequest=true;
    }
    const update=cachedVisualDelta({frame,sceneTime:total,lastUpdateFrame:actor.lastUpdateFrame,lastTime:actor.lastTime},precision);
    if(!update.update)return Object.freeze({handle,frame,sceneTime:total,updated:false,configured,
      delta:update.delta,ready:recentlyDrawn(frame,actor.lastDraw),needsChildrenUpdate:false});
    requireValue(!actor.pendingChildren,'updateChildren must complete the previous actor update');
    requireValue(update.delta>=0,'actor scene time moved backwards');
    actor.updating=true;
    try{
      // Original Composite flush is before any of its existing child updates.
      flushChildren(handle);
      if(actor.clock)advanceTextClock(actor.clock,update.delta,precision,key=>{
        if(key.type!==2&&key.type!==8)return;
        const ready=recentlyDrawn(frame,actor.lastDraw);
        emit(Object.freeze({handle,actor:handle,type:key.type,data:key.data,time:key.time,
          index:key.index,phase:key.phase,frame,sceneTime:total,delta:update.delta,ready,
          stateName:actor.request.desc.stateName,started:actor.request.started,span:actor.request.span,
          modelIdentity:identity,onlyOnce:false,force:false}));
      });
      actor.lastTime=total;actor.lastUpdateFrame=frame;
      actor.pendingChildren={frame,delta:update.delta};
    }finally{actor.updating=false;}
    return Object.freeze({handle,frame,sceneTime:total,updated:true,configured,delta:update.delta,
      ready:recentlyDrawn(frame,actor.lastDraw),needsChildrenUpdate:true});
  }
  function updateChildren(handle){
    requireValue(validHandle(handle),'full actor handle required');
    const actor=actors.get(handle),pending=actor?.pendingChildren;
    if(!pending)return false;
    requireValue(pending.frame===frame,'complete child update before beginning another frame');
    advanceChildren(handle,pending.delta);actor.pendingChildren=null;return true;
  }
  function markDrawn(handle){
    requireValue(validHandle(handle),'full actor handle required');
    const actor=actors.get(handle);if(!actor)return false;
    requireValue(!actor.pendingChildren,'updateChildren must precede actual draw stamp');
    actor.lastDraw=frame;return true;
  }
  function snapshot(handle){
    const actor=actors.get(handle);if(!actor)return null;
    return Object.freeze({handle,modelIdentity:actor.modelIdentity,frame,sceneTime:total,
      lastUpdateFrame:actor.lastUpdateFrame,lastDraw:actor.lastDraw,lastTime:actor.lastTime,
      ready:recentlyDrawn(frame,actor.lastDraw),stateName:actor.request?.desc.stateName??null,
      started:actor.request?.started??null,span:actor.request?.span??null,
      clock:actor.clock?Object.freeze({time:actor.clock.time,period:actor.clock.period,
        mode:actor.clock.mode,start:actor.clock.start,speed:actor.clock.speed}):null,
      pendingChildren:actor.pendingChildren?Object.freeze({...actor.pendingChildren}):null});
  }
  return Object.freeze({beginFrame,observe,updateChildren,markDrawn,remove,reset,snapshot,
    handles:()=>Object.freeze([...actors.keys()]),
    current:()=>Object.freeze({frame,sceneTime:total,started}),precision,
    boundaries:Object.freeze([
      'Caller supplies admitted/scaled scene delta; this module does not choose wall time or simulation admission.',
      'Caller observes actual cached visual updates and marks only actual draws; complete native camera/reflection scheduling is not reconstructed.',
      'Schedule observe then updateChildren for the portable MainModel-first sibling layout. Native Citizen factory sibling construction order is not proved here.',
      'Only the five supported economic work states dispatch PLAY_SOUND2/ATTACH_VISUAL8. Sound audibility/cap/pitch and attachment creation/lifetime remain separate.',
      'Type8 readiness is the original last-draw gate; type2 sound admission must not use that gate.',
      'First observed model is configured cold from raw sim age; warm observed tuple changes use elapsed0. Unobserved intermediate Core requests are not reconstructed.',
      'disposeChildren must implement actual owner/model/session destruction; stateChange alone only starts child draining.',
    ]),
  });
}
