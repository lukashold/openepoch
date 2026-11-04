// Native005F8450 reads the explicit per-mesh AnimatedTextureSetName extra data.
// Source DbTextureSetinfo lists are uniformly interpolated over the whole year;
// their actual lengths and repeated images are retained by the asset importer.
export function modelYearPhase({month0,month1=(month0+1)%12,blend=0}){
 if(!Number.isInteger(month0)||month0<0||month0>11||month1!==(month0+1)%12||!Number.isFinite(blend)||blend<0||blend>1)throw new RangeError('Invalid model season phase');
 return (month0+blend)/12;
}
export function modelSeasonFrames(sequence,phase,fallback){
 if(!sequence?.length||!Number.isFinite(phase))return {first:fallback,next:fallback,blend:0};
 let normalized=phase%1;if(normalized<0)normalized+=1;
 const position=normalized*sequence.length,index=Math.floor(position);
 const first=sequence[index],next=sequence[(index+1)%sequence.length];
 return {first,next,blend:first===next?0:position-index};
}
export function modelSeasonBinding(manifest,modelUrl,model,part){
 const entry=manifest?.models?.[modelUrl],binding=entry?.parts?.[part.sourceNode];
 if(!binding||entry.source?.sha256!==model.source?.sha256||binding.name!==part.name||binding.baseTextureSource!==part.textureSource)return null;
 return binding.key.toLowerCase();
}
