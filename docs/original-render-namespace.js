// Transport metadata, not an original-game asset alias table. Source manifests
// retain their source IDs; only draw/range keys use the raw WASM namespace.
export const RENDER_ASSET_KIND=Object.freeze({UNIT:0,RESOURCE:1,PROJECTILE:2,QUAD:3,DECORATION:4});
export function normalizeRenderAssetMapping(mapping){
 if(mapping===undefined||mapping===null)return null;
 if(!Array.isArray(mapping))throw new RangeError('Render asset mapping must be an array of typed bindings');
 const used=new Set(),rows=[];
 for(const row of mapping){
  const {renderId,kind,sourceId}=row||{};
  if(!Number.isSafeInteger(renderId)||renderId<0||renderId>0xffffff||!Number.isInteger(kind)||kind<0||kind>4||!Number.isSafeInteger(sourceId)||sourceId<0||sourceId>0xffffffff||used.has(renderId))throw new RangeError('Invalid or duplicate typed render asset binding');
  used.add(renderId);rows.push(Object.freeze({renderId,kind,sourceId}));
 }
 return Object.freeze(rows.sort((a,b)=>a.renderId-b.renderId));
}
export function createRenderAssetMapping(wasm){
 if(typeof wasm?.schema_revision!=='function'||wasm.schema_revision()<6)return null;
 if(typeof wasm.render_asset_kind!=='function'||typeof wasm.render_asset_source_id!=='function')throw new Error('Schema6 lacks typed render asset metadata');
 const length=typeof wasm.render_range_len==='function'?wasm.render_range_len():wasm.ranges_len?.();
 if(!Number.isInteger(length)||length<2||length%2||length>0x2000000)throw new RangeError('Invalid schema6 render range length');
 const rows=[];
 for(let renderId=0;renderId<length/2;renderId++){
  const kind=wasm.render_asset_kind(renderId)>>>0;if(kind===0xffffffff)continue;
  rows.push({renderId,kind,sourceId:wasm.render_asset_source_id(renderId)>>>0});
 }
 return normalizeRenderAssetMapping(rows);
}
export function sourceRenderBindings(mapping,kind,sourceId){
 return mapping===null?[{renderId:sourceId,kind,sourceId}]:mapping.filter(row=>row.kind===kind&&row.sourceId===sourceId);
}
export function manifestAssetKind(entry){
 // Current manifests precede typed render IDs: role rows0..56 and original
 // resource rows100..110. Explicit kind is required if future role IDs overlap.
 const id=+(entry.id??entry.def),kind=entry.assetKind??(id>=100&&id<=110?RENDER_ASSET_KIND.RESOURCE:RENDER_ASSET_KIND.UNIT);
 if(kind!==RENDER_ASSET_KIND.UNIT&&kind!==RENDER_ASSET_KIND.RESOURCE)throw new RangeError('Original NIF manifests support only unit or resource bindings');
 return kind;
}
