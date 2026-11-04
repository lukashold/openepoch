import {createRenderActorMetadata} from './render-actor-metadata.js';
import {createSourceSailController} from './source-sail-controller.js';
import {createWasmSourceClothFactory} from './source-cloth-wasm.js';

export const SOURCE_CLOTH_MODULE=Object.freeze({
 url:new URL('./openepoch-cloth7.wasm?v=cloth7',import.meta.url),bytes:52850,
 sha256:'c8d1207fec111a5b47dbe24b75d51cf3f250f032dfdb4073e75113e65353c625',
});

// Own presentation scheduler. The caller supplies the rendered-world delta and
// explicit pause/wait policy once, before reflection, shadow and main draws.
// All game saves, simulation hashes and network commands remain untouched.
export async function loadSourceSailRuntime({wasm,namespace,models,enabled=false,fetcher=globalThis.fetch}={}){
 if(!enabled)return null;
 const metrics={enabled:false,error:null,backend:null,world:null,frame:null,owners:0,steps:0,
  clock:'explicit-render-delta',visibility:'retain-without-step-while-culled',arithmetic:'conditional-chop53'};
 let reader=null,controller=null,failed=false;
 const fail=error=>{
  failed=true;metrics.enabled=false;metrics.error=String(error?.message??error);
  try{controller?.reset();}catch{/* Keep a broken visual backend isolated. */}
  try{models?.disposeDynamicSails?.();}catch{/* Static geometry remains usable. */}
 };
 try{
  reader=createRenderActorMetadata(wasm);
  if(!reader)return null;
  for(const key of ['resolveSailSource','resolveSailForces','prepareDynamicSails','disposeDynamicSails'])if(typeof models?.[key]!=='function')throw new Error('Incomplete dynamic sail renderer');
  const response=await fetcher(SOURCE_CLOTH_MODULE.url);
  if(!response?.ok)throw new Error('Own cloth module could not be loaded');
  const bytes=await response.arrayBuffer();
  if(bytes.byteLength!==SOURCE_CLOTH_MODULE.bytes)throw new Error('Own cloth module size does not match');
  const digest=Array.from(new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
  if(digest!==SOURCE_CLOTH_MODULE.sha256)throw new Error('Own cloth module checksum does not match');
  const visual=(await WebAssembly.instantiate(bytes,{})).instance.exports;
  const factory=createWasmSourceClothFactory(visual);
  controller=createSourceSailController({isAlive:reader.alive,rounding:'chop53',factory});
  metrics.backend=factory.backend;metrics.enabled=true;
 }catch(error){fail(error);}
 function advance({count,instances,epochs,deltaSeconds,runPhysics}={}){
  if(failed)return null;
  try{
   const metadata=reader.snapshot({count,instances,namespace});
   const context=models.lighting;
   const updated=controller.advance({metadata,deltaSeconds,runPhysics,
    resolveSource:actor=>models.resolveSailSource(actor,epochs),
    localForces:(actor,source,index,life)=>models.resolveSailForces(actor,source,index,life),
    lighting:{ambient:context?.sourceAmbient,diffuse:context?.sourceDiffuse,sunDirection:context?.sourceDirection}});
   if(updated.advanced)models.prepareDynamicSails({controller,metadata});
   Object.assign(metrics,{world:updated.session,frame:updated.frame,owners:updated.owners,steps:updated.steps});
   return metadata;
  }catch(error){fail(error);return null;}
 }
 function reset(){controller?.reset();models?.disposeDynamicSails?.();Object.assign(metrics,{world:null,frame:null,owners:0,steps:0});}
 return Object.freeze({advance,reset,metrics});
}
