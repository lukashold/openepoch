import {createNativeCloth,nativeSailDamageMask} from './source-cloth-kernel.js';
import {nativeClothVertexColors} from './source-cloth-lighting.js';

// Own lifecycle/clock adapter around the source-backed numerical helpers. It
// accepts explicit local forces, never invented weather, positions-as-identity
// or a native Death inferred from health/disappearance. One controller belongs
// to one game WASM instance: session tokens are scoped to that instance.
const uint=v=>Number.isInteger(v)&&v>=0&&v<0xffffffff;
const validVector=v=>v?.length===3&&Array.from(v).every(x=>Number.isFinite(x)&&Math.abs(x)<1e10);
const DEFAULT_FACTORY=Object.freeze({
 create:(plane,options)=>createNativeCloth(plane,options),
 shade:(cloth,plane,lighting,options)=>nativeClothVertexColors({positions:cloth.positions,nx:plane.nx,ny:plane.ny},lighting,options),
 dispose(){},
});

export function createSourceSailController({isAlive,rounding,quality='normal',factory=DEFAULT_FACTORY}={}){
 if(typeof isAlive!=='function'||!['chop53','nearest53'].includes(rounding)||!['normal','uniform'].includes(quality))throw new TypeError('Explicit liveness, arithmetic and source lighting quality required');
 for(const method of ['create','shade','dispose'])if(typeof factory[method]!=='function')throw new TypeError('Incomplete cloth backend');
 let session=null,lastFrame=null;const owners=new Map();
 function dispose(owner){for(const part of owner.parts)factory.dispose(part.cloth);owners.delete(owner.handle);}
 function reset(){for(const owner of [...owners.values()])dispose(owner);session=null;lastFrame=null;}
 function make(actor,source){
  const owner={handle:actor.handle,sourceKey:source.key,source,actor,parts:[],damageMask:0,dead:false};
  try{for(const plane of source.planes)owner.parts.push({plane,cloth:factory.create(plane,{rounding}),shading:null});}
  catch(error){for(const part of owner.parts)factory.dispose(part.cloth);throw error;}
  return owner;
 }
 function advance({metadata,deltaSeconds,runPhysics,resolveSource,localForces,lighting}={}){
  if(!Number.isInteger(metadata?.session)||metadata.session<=0||metadata.session>0xffffffff||!Number.isInteger(metadata.frame)||metadata.frame<0||metadata.frame>0xffffffff||!Array.isArray(metadata.actors)||typeof metadata.forHandle!=='function')throw new TypeError('A checked metadata snapshot is required');
  if(!Number.isFinite(deltaSeconds)||deltaSeconds<0||typeof runPhysics!=='boolean'||typeof resolveSource!=='function')throw new TypeError('Explicit visual clock and source resolver required');
  // Source step(0) changes RNG. A paused scheduler must explicitly pass false.
  if(session===metadata.session&&lastFrame===metadata.frame)return Object.freeze({advanced:false,session,frame:lastFrame,owners:owners.size,steps:0});
  for(const key of ['ambient','diffuse','sunDirection'])if(!validVector(lighting?.[key])||!Array.from(lighting[key]).every(v=>Math.fround(v)===v))throw new TypeError('Raw binary32 source lighting required');
  const plans=[];
  // Reject missing/misaligned inputs before any owner is stepped or discarded.
  for(const actor of metadata.actors){
   if(!uint(actor.handle)||metadata.forHandle(actor.handle)!==actor||!Number.isFinite(actor.hpRatio)||actor.hpRatio<0||actor.hpRatio>1)throw new TypeError('Actual metadata actor identity required');
   const source=resolveSource(actor);if(source==null)continue;
   if(typeof source.key!=='string'||!source.key||!Array.isArray(source.planes))throw new TypeError('Explicit source visual and local grids required');
   for(const plane of source.planes){
    if(!Number.isInteger(plane.nx)||!Number.isInteger(plane.ny)||plane.nx<2||plane.ny<2||plane.nx>255||plane.ny>255||plane.nx*plane.ny>65535||plane.positions?.length!==plane.nx*plane.ny*3||plane.pinned?.length!==plane.nx*plane.ny||!Array.from(plane.positions).every(v=>Number.isFinite(v)&&Math.fround(v)===v&&Math.abs(v)<1e10))throw new TypeError('Local source binary32 grid required');
   }
   const forces=[];
   if(runPhysics&&source.planes.length){
    if(typeof localForces!=='function')throw new TypeError('Source local force provider required');
    const old=session===metadata.session?owners.get(actor.handle):null;
    const previous=old?.sourceKey===source.key?old:null;
    const life=Object.freeze({dead:previous?.dead===true,previousActor:previous?.actor??null,isNewOwner:!previous,session:metadata.session});
    for(let i=0;i<source.planes.length;i++){
     const force=localForces(actor,source,i,life);
     for(const key of ['gravity','windMin','windMax'])if(!validVector(force?.[key]))throw new TypeError('Explicit finite local forces required');
     forces.push(Object.fromEntries(['gravity','windMin','windMax'].map(key=>[key,Array.from(force[key])])));
    }
   }
   plans.push({actor,source,forces});
  }
  if(session!==metadata.session){reset();session=metadata.session;}
  for(const owner of [...owners.values()])if(!isAlive(owner.handle))dispose(owner);
  let steps=0;
  for(const {actor,source,forces}of plans){
   let owner=owners.get(actor.handle);
   if(owner&&owner.sourceKey!==source.key){dispose(owner);owner=null;}
   if(!source.planes.length){if(owner)dispose(owner);continue;}
   if(!isAlive(actor.handle))continue;
   if(!owner){owner=make(actor,source);owners.set(actor.handle,owner);}
   owner.actor=actor;owner.damageMask=nativeSailDamageMask(owner.damageMask,actor.hpRatio);
   for(let i=0;i<owner.parts.length;i++){
    const part=owner.parts[i];
    if(runPhysics){
     const force=owner.dead?{...forces[i],windMin:[0,0,0],windMax:[0,0,0]}:forces[i];
     part.cloth.step(deltaSeconds,force);steps++;
    }
    part.shading=factory.shade(part.cloth,part.plane,lighting,{rounding,mode:quality,colors:part.shading?.colors});
   }
  }
  // Unseen but still live owners retain state without a step. That visibility
  // policy is an explicit application adapter, not a native scheduler claim.
  lastFrame=metadata.frame;
  return Object.freeze({advanced:true,session,frame:lastFrame,owners:owners.size,steps});
 }
 function notifyDeath(handle,worldSession){
  if(!uint(handle)||worldSession!==session)return false;
  const owner=owners.get(handle);if(!owner)return false;
  if(!owner.dead){owner.dead=true;for(const part of owner.parts)part.cloth.releasePins();}
  return true;
 }
 return Object.freeze({advance,reset,notifyDeath,get:handle=>owners.get(handle)||null,
  status:()=>Object.freeze({session,frame:lastFrame,owners:owners.size,rounding,quality,visibility:'retain-without-step-while-culled',death:'explicit-event-only'})});
}
