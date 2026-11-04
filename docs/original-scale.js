// Undo the converter's per-file unit cube normalization. Native sourceScale
// is bound to an explicit UnitModel/ScaleType in model-scales.json.
export function nativeModelTransform(model,binding,instanceSize=1){
 const fallback={scale:instanceSize,originOffset:[0,0,0],native:false};
 const normal=model?.normalization;
 if(!binding?.verified||binding.sourceSha256!==model?.source?.sha256||
  !(binding.sourceScale>0)||!Number.isFinite(binding.sourceScale)||
  !(normal?.scale>0)||!Number.isFinite(normal.scale)||
  !Array.isArray(normal.offset)||normal.offset.length!==3||!normal.offset.every(Number.isFinite))return fallback;
 return {scale:binding.sourceScale/normal.scale,
  originOffset:normal.offset.map(value=>-value*normal.scale),native:true};
}
