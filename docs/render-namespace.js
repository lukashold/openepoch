// Presentation adapters for the explicitly typed roster ABI. Source model IDs
// stay separate from the raw IDs that index the simulation's render ranges.
const INVALID=0xffffffff,STRIDE=12;
export function createFrontendNamespace(wasm,{referenceMode=false,assetMapping=null}={}){
 const revision=typeof wasm.schema_revision==='function'?wasm.schema_revision()>>>0:0;
 if(![0,6,7,8].includes(revision))throw Error('Unsupported presentation schema '+revision);
 // Save revisions7/8 retain the typed presentation layout6.
 const wide=revision===6||revision===7||revision===8,definitionCount=wide?wasm.definition_count()>>>0:57;
 if(!definitionCount||definitionCount>65536)throw Error('Invalid definition count');
 const rangeLength=wide?wasm.render_range_len()>>>0:512;
 if(!rangeLength||rangeLength%2||rangeLength>2*0x1000000)throw Error('Invalid render range length');
 const slotCount=rangeLength/2,rows=new Map(),units=new Map(),quads=new Map();
 if(wide){
  if(!Array.isArray(assetMapping))throw Error('Typed render asset mapping required');
  for(const row of assetMapping){
   const {renderId,kind,sourceId}=row;
   if(!Number.isInteger(renderId)||renderId<0||renderId>=slotCount||rows.has(renderId)||!Number.isInteger(kind)||kind<0||kind>4||!Number.isInteger(sourceId)||sourceId<0||sourceId>65535)throw Error('Invalid typed render asset');
   if((wasm.render_asset_kind(renderId)>>>0)!==kind||(wasm.render_asset_source_id(renderId)>>>0)!==sourceId)throw Error('Render asset metadata differs');
   const saved=Object.freeze({renderId,kind,sourceId});rows.set(renderId,saved);
   if(kind===0){if(sourceId>=definitionCount||units.has(sourceId))throw Error('Invalid unit render binding');units.set(sourceId,renderId);}
   if(kind===3){if(quads.has(sourceId))throw Error('Duplicate quad binding');quads.set(sourceId,renderId);}
  }
  for(let id=0;id<slotCount;id++)if((wasm.render_asset_kind(id)>>>0)!==INVALID&&!rows.has(id))throw Error('Missing render asset binding');
  if(units.size!==definitionCount||[240,241,242,243].some(id=>!quads.has(id)))throw Error('Incomplete presentation namespace');
 }else if(assetMapping!==null)throw Error('Legacy rules cannot consume typed render assets');
 function unitFallbackId(id){
  if(!wide||id<57)return id;
  if(!Number.isInteger(id)||id<0||id>=definitionCount)return -1;
  const flags=wasm.definition_flags(id)>>>0;
  return flags&1?0:flags&4?8:15;
 }
 function fallbackModelId(renderId){
  if(!wide)return renderId;
  const row=rows.get(renderId);if(!row)return -1;
  if(row.kind===0){
   if(row.sourceId<57)return row.sourceId;
   // Only the first57 definitions have authored procedural unit templates.
   // Later source IDs can overlap the legacy resource/projectile atlas. Use an
   // explicitly generic building/citizen/soldier when its native mesh is absent.
   return unitFallbackId(row.sourceId);
  }
  // The procedural atlas has two trees, iron ore and berries. Other native
  // resources use the generic ore fallback; their exact NIF bindings remain
  // source IDs100..110 in original-renderer, never projectile/flag templates.
  if(row.kind===1)return row.sourceId<=103?row.sourceId:102;
  if(row.kind===2)return 103+row.sourceId;
  return row.sourceId;
 }
 let fallbackBuffer=new Float32Array(0);
 function fallbackInstances(data){
  if(!(data instanceof Float32Array)||data.length%STRIDE)throw Error('Invalid instance data');
  if(!wide)return data;
  if(fallbackBuffer.length<data.length)fallbackBuffer=new Float32Array(Math.max(data.length,fallbackBuffer.length*2));
  const result=fallbackBuffer.subarray(0,data.length);result.set(data);
  for(let offset=4;offset<result.length;offset+=STRIDE){const raw=result[offset],id=fallbackModelId(raw);if(!Number.isInteger(raw)||id<0)throw Error('Unknown render instance asset');result[offset]=id;}
  return result;
 }
 function queueJob(raw){
  if(!Number.isInteger(raw)||raw<0||raw>INVALID)return null;
  if(wide){
   const kind=wasm.queue_item_kind(raw)>>>0,value=wasm.queue_item_value(raw)>>>0;
   return kind<=3&&value!==INVALID?{kind,value}:null;
  }
  if(raw===255)return {kind:3,value:0};
  if(referenceMode&&raw>=64&&raw<244)return {kind:1,value:raw-64};
  if(!referenceMode&&raw>=200&&raw<255)return {kind:2,value:raw-200};
  return raw<definitionCount?{kind:0,value:raw}:null;
 }
 return Object.freeze({revision,wide,definitionCount,rangeLength,slotCount,assetMapping,
  asset:renderId=>rows.get(renderId)||null,
  unitRenderId:id=>wide?(units.get(id)??-1):id,
  quadRenderId:id=>wide?(quads.get(id)??-1):id,
  isMobile:id=>wide?Number.isInteger(id)&&id>=0&&id<definitionCount&&!!((wasm.definition_flags(id)>>>0)&2):id>=8&&id<=46,
  fallbackModelId,fallbackInstances,queueJob,unitFallbackId,
 });
}
