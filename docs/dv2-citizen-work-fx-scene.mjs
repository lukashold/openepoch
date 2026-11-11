// Own per-parent queue/lifetime adapter over original307/311 source facts.
// Callers explicitly supply admitted ATTACH events, state notifications, actor
// update boundaries/deltas and actual ROOT transforms. No global time is read.
export const WHEAT_HARVEST_SECONDS = Math.fround(1.933333396911621);
const WHEAT='FX_Wheat_Harvest_1',f32=Math.fround;
const assert=(value,message)=>{if(!value)throw new Error('Citizen work FX scene: '+message);};
const actorValid=value=>Number.isInteger(value)&&value>=0&&value<0xffffffff;
const finite=(array,length)=>array?.length===length&&Array.from(array).every(Number.isFinite);

export function createCitizenWorkFXScene({renderer,create,rootFor,onLink=()=>{},onUnlink=()=>{},onDestroy=()=>{},precision}={}){
 assert(precision==='nearest','explicit portable nearest lifetime precision required');
 assert(typeof create==='function'&&typeof rootFor==='function','explicit creation and ROOT callbacks required');
 assert(renderer&&typeof renderer.draw==='function','source renderer delegate required');
 assert([onLink,onUnlink,onDestroy].every(fn=>typeof fn==='function'),'invalid lifecycle callback');
 const owners=new Map(),children=new Map();let serial=0;
 const view=child=>Object.freeze({id:child.id,actor:child.actor,visual:child.visual,token:child.token,
  mode:child.mode,elapsed:child.elapsed,remaining:child.remaining,stream:'embedded',precision});
 const ownerFor=actor=>{assert(actorValid(actor),'full actor handle required');return owners.get(actor);};
 function owner(actor){
  if(!owners.has(actor))owners.set(actor,{actor,active:[],pending:[],removals:[],removalSet:new Set()});
  return owners.get(actor);
 }
 function queueRemove(id){
  const child=children.get(id);if(!child)return false;
  const state=owner(child.actor);
  if(!state.removalSet.has(id)){state.removalSet.add(id);state.removals.push(id);}
  return true;
 }
 function attach(event,{onlyOnce=false}={}){
  assert(event&&actorValid(event.actor)&&event.visual===WHEAT,'explicit normalized actor/Wheat event required');
  assert(typeof onlyOnce==='boolean','invalid OnlyOnce control');
  const current=owners.get(event.actor);
  // Original OnlyOnce lookup precedes both factory creation and force/readiness.
  if(onlyOnce&&current&&[...current.active,...current.pending].some(child=>child.visual===event.visual))return null;
  const result=create(event);if(!result)return null;
  if(typeof result==='object'){
   assert(result.actor===undefined||result.actor===event.actor,'creation changed owner');
   assert(result.visual===undefined||result.visual===event.visual,'creation changed source visual');
  }
  const child={id:++serial,actor:event.actor,visual:WHEAT,token:typeof result==='object'?result.token:undefined,
   mode:2,elapsed:0,remaining:WHEAT_HARVEST_SECONDS};
  children.set(child.id,child);owner(child.actor).pending.push(child);return child.id;
 }
 function flush(actor){
  const state=ownerFor(actor);if(!state)return {added:[],removed:[]};
  const additions=state.pending.splice(0),added=[],removed=[];
  // Append all pending children before applying any pending removal.
  for(const child of additions){state.active.push(child);added.push(child.id);onLink(view(child));}
  const removals=state.removals.splice(0);state.removalSet.clear();
  for(const id of removals){
   const index=state.active.findIndex(child=>child.id===id);if(index<0)continue;
   const child=state.active[index];onUnlink(view(child));
   state.active[index]=state.active.at(-1);state.active.pop();children.delete(id);
   removed.push(id);onDestroy(view(child));
  }
  return {added,removed};
 }
 function stateChange(actor){
  const state=ownerFor(actor);if(!state)return 0;
  let changed=0;
  // Factory virtual+4 subscribes before the child is put on the parent queue.
  // Constructed pending children therefore receive this notification too.
  for(const child of [...state.active,...state.pending])if(child.mode===2){child.mode=3;changed++;}
  return changed;
 }
 function advance(actor,delta){
  assert(Number.isFinite(delta)&&delta>=0&&f32(delta)===delta,'supplied actor delta must be nonnegative f32');
  const state=ownerFor(actor);if(!state)return {updated:[],queued:[]};
  const updated=[],queued=[];
  for(const child of state.active){
   child.remaining=f32(child.remaining-delta);
   if(child.mode===3&&child.remaining<=0){queueRemove(child.id);queued.push(child.id);}
   else{child.elapsed=f32(child.elapsed+delta);updated.push(child.id);}
  }
  return {updated,queued};
 }
 function instances(actor){
  const state=ownerFor(actor);if(!state)return [];
  const result=[];
  for(const child of state.active){
   const entry=view(child),root=rootFor(entry);if(!root||root.visible===false)continue;
   assert(finite(root.position,3)&&Number.isFinite(root.yaw)&&Number.isFinite(root.scale)&&root.scale>0,
    'ROOT callback must supply actual position/yaw/scale');
   result.push({...root,elapsed:child.elapsed,stream:'embedded',precision,actor:child.actor,attachment:child.id});
  }
  return result;
 }
 function snapshot(actor){
  const state=ownerFor(actor);
  const summarize=child=>{const {token,...rest}=view(child);return rest;};
  return {actor,active:state?.active.map(summarize)??[],pending:state?.pending.map(summarize)??[],removals:[...state?.removals??[]]};
 }
 return Object.freeze({attach,flush,stateChange,advance,queueRemove,instances,snapshot,
  draw(actor,pass){return renderer.draw(instances(actor),pass);},
  reset(){for(const child of children.values())onDestroy(view(child));children.clear();owners.clear();serial=0;},
  precision,stream:'embedded',
  boundaries:Object.freeze(['Readiness, event crossing/admission and global scene-clock/culling updates belong to the caller.',
   'No inferred ROOT transform, accepted timestamp, automatic flush or lifetime reset.',
   'Mode2 retains negative remaining time; only mode3 expiry queues removal.',
   'Native full XYZ matrix/trigonometric precision and database-default loading retain the frozen sampler/source evidence boundaries.']),
 });
}
