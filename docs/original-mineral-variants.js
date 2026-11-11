// UP15 0060D990 uses a local word and leaves shared RNG state untouched.
export function sourceDefaultModelIndex(packedId,count,zeroSeed){
  if(!Number.isInteger(count)||count<1)throw new RangeError('Invalid source model list');
  const seed=(packedId>>>0)||(zeroSeed>>>0);
  const word=(Math.imul(seed,1664525)+1013904223)>>>0;
  return ((word>>>16)&0x7fff)%count;
}

const mineralVisuals=Object.freeze({5:[3,9,77],6:[6,12,78],7:[9,15,79]});
const portableResourceSlots=Object.freeze([0,1,3,4,5,2,6,7,8]);

export function portableResourceSlot(nativeId){return portableResourceSlots[nativeId]??null;}

export function sourceMineralRenderId(resource,packedId,viewerEpoch,sharedEpoch){
  const entry=mineralVisuals[resource];
  if(!entry)return null;
  const [reveal,retire,model]=entry;
  if(viewerEpoch>=retire)return null;
  const variation=sourceDefaultModelIndex(packedId,3);
  if(sharedEpoch<reveal)return 74+variation;
  return resource===7?model+variation:model;
}
