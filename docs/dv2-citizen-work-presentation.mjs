// Own bridge between authoritative Citizen requests, the cached native text-key
// clock, original sound declarations and the original Wheat child graph.
import {loadCitizenWorkEventAssets} from './dv2-citizen-work-event-assets.mjs?v=work-events290-private';
import {createCitizenWorkEventScene} from './dv2-citizen-work-event-scene.mjs?v=work-events290-private';
import {createCitizenWorkFXScene} from './dv2-citizen-work-fx-scene.mjs?v=work-events290-private';

const f32=Math.fround;
const resourceSounds=new Map([[25,'Resource_gold'],[26,'Resource_stone'],
  [27,'Resource_saltpeter'],[28,'Resource_wood']]);

export async function loadCitizenWorkPresentation(gl,{enabled=false,privateMode=false,wasm,audio}={}){
 const assets=await loadCitizenWorkEventAssets(gl,{enabled,privateMode});
 if(!assets)return null;
 if(assets.attachmentScale!=='parentSourceScale'||!Number.isFinite(assets.emitterCenterZ))
  throw Error('Citizen work source attachment metadata is unavailable');
 if(!wasm||!audio||typeof wasm.entity_alive!=='function'||typeof wasm.active_map_ground!=='function')
  throw Error('Citizen work presentation requires the actor and ground ABI');
 audio.configureCitizenWork({sounds:assets.sounds});
 const actors=new Map();
 let context={session:null,view:null,audible:false},scene;
 const counters={playKeys:0,played:0,attachKeys:0,attached:0,draws:0};
 const recentEvents=[];
 const fx=createCitizenWorkFXScene({precision:'nearest',renderer:assets.renderer,
  create:()=>true,
  rootFor:child=>actors.get(child.actor)?.root??null,
 });
 function disposeChildren(handle){
  const snapshot=fx.snapshot(handle);
  for(const child of [...snapshot.active,...snapshot.pending])fx.queueRemove(child.id);
  fx.flush(handle);actors.delete(handle);
 }
 scene=createCitizenWorkEventScene({precision:'nearest',
  stateChange:handle=>fx.stateChange(handle),
  flushChildren:handle=>fx.flush(handle),
  advanceChildren:(handle,delta)=>fx.advance(handle,delta),
  disposeChildren,
  emit(event){
   const row=actors.get(event.handle);
   if(!row||context.session===null||wasm.entity_alive(event.handle)!==1)return;
   const receipt={handle:event.handle,type:event.type,data:event.data,time:event.time,
    frame:event.frame,sceneTime:event.sceneTime,ready:event.ready,accepted:false};
   recentEvents.push(receipt);if(recentEvents.length>128)recentEvents.shift();
   if(event.type===2&&resourceSounds.has(row.state)){
    counters.playKeys++;
    if(!context.audible)return;
    const x=row.actor.simX,y=row.actor.simY;
    // Native8681B0 emits from Actor position plus the physical bbox's Z centre.
    // The source metadata loader supplies the verified centre, not a guessed
    // constant or WebGL model-normalization extent.
    const center=assets.emitterCenterZ;
    if(!Number.isFinite(center))return;
    const z=f32(wasm.active_map_ground(x,y)+center);
    if(audio.playCitizenWork(event.data,{handle:event.handle,session:context.session,
       position:[x,y,z]},context.view)){counters.played++;receipt.accepted=true;}
   }else if(event.type===8&&event.data==='FX_Wheat_Harvest_1'){
    counters.attachKeys++;
    if(event.ready&&fx.attach({actor:event.handle,visual:event.data})!==null){counters.attached++;receipt.accepted=true;}
   }
  },
 });
 function reset(){scene.reset();fx.reset();actors.clear();audio.resetCitizenWork();
  recentEvents.length=0;
  for(const name of Object.keys(counters))counters[name]=0;
  context={session:null,view:null,audible:false};}
 function frame({session,sceneDelta,view,audible}){
  if(!Number.isInteger(session)||session<=0||!Number.isFinite(sceneDelta)||sceneDelta<0)
   throw Error('Invalid Citizen work presentation frame');
  if(context.session!==session&&context.session!==null)reset();
  context={session,view,audible:audible===true};
  for(const handle of actors.keys())if(wasm.entity_alive(handle)!==1)scene.remove(handle);
  audio.updateCitizenWork(session,view);
  scene.beginFrame(session,f32(sceneDelta));
 }
 function observe(actor,variant,request){
  if(context.session===null||wasm.render_meta_session()!==context.session||actor?.def!==8
    ||!variant?.nativeCitizenWork||wasm.entity_alive(actor.handle)!==1)return false;
  if(!(variant.sourceScale>0))throw Error('Citizen original source scale is unavailable');
  const previous=scene.snapshot(actor.handle);
  if(previous&&previous.modelIdentity!==variant.nativeCitizenWork.sourceSha256)scene.remove(actor.handle);
  actors.set(actor.handle,{actor,state:wasm.citizen_work_visual_state(actor.handle)>>>0,
   root:{position:[actor.renderX,actor.renderAltitude,actor.renderY],
    yaw:actor.renderYaw+variant.yawOffset,scale:variant.sourceScale}});
  scene.observe({handle:actor.handle,request,workBinding:variant.nativeCitizenWork,
   modelIdentity:variant.nativeCitizenWork.sourceSha256});
  // Composite flush occurs before model updates. These presentation composites
  // place their MainModel before appended Wheat children; the split API keeps
  // that scheduling policy explicit rather than hiding it in the text clock.
  scene.updateChildren(actor.handle);
  return true;
 }
 function draw(actor,variant,pass){
  if(context.session===null||wasm.render_meta_session()!==context.session||!actors.has(actor.handle))return;
  const instances=fx.instances(actor.handle);
  const result=instances.length?assets.renderer.draw(instances,pass):0;
  scene.markDrawn(actor.handle);
  counters.draws+=typeof result==='number'?result:Number(result?.draws??result?.calls??0);
 }
 return Object.freeze({frame,observe,draw,reset,
  hasExactSound(actor){return !!actor&&Number.isFinite(assets.emitterCenterZ)&&resourceSounds.has(wasm.citizen_work_visual_state(actor.handle)>>>0)
    &&audio.citizenWorkReady(resourceSounds.get(wasm.citizen_work_visual_state(actor.handle)>>>0));},
  status(){return {configured:true,session:context.session,...counters,recentEvents:recentEvents.map(row=>({...row})),
    actors:[...actors].map(([handle,row])=>({handle,root:row.root,state:row.state,
      clock:scene.snapshot(handle),fx:fx.snapshot(handle)})),
    audio:audio.citizenWorkAudioStatus(),
    scope:'Portable nearest presentation; original full root construction/culling and process-wide audio RNG remain separately qualified.'};},
  dispose(){reset();assets.dispose();},
 });
}
