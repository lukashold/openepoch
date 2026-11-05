// Regional alternatives are source-bound manifest rows, never filename guesses.
export function regionalModelEntries(manifest){
 if(manifest?.format!=='ee2-native-regional-model-manifest-v1'||!Array.isArray(manifest.models))throw new Error('Unsupported native regional model manifest');
 const claimed=new Set();
 return manifest.models.map(row=>{
  if(row.assetKind!==0||!Number.isInteger(row.id)||row.id<0||!Number.isInteger(row.epoch)||row.epoch<0||!Number.isInteger(row.until)||row.until<=row.epoch||typeof row.url!=='string'||!Array.isArray(row.regionalCivilizations)||!row.regionalCivilizations.length)throw new Error('Invalid native regional model row');
  if(!row.nativeScaleBinding?.verified||row.nativeScaleBinding.sourceSha256!==row.source?.sha256)throw new Error('Regional model scale/source mismatch');
  const civilizations=[...row.regionalCivilizations];
  for(const civ of civilizations){
   if(!Number.isInteger(civ)||civ<0||civ>13)throw new Error('Invalid native regional civilization');
   const key=`${row.id}:${row.epoch}:${civ}`;if(claimed.has(key))throw new Error('Duplicate regional model identity');claimed.add(key);
  }
  return{...row,regionalCivilizations:Object.freeze(civilizations)};
 });
}
export function selectRegionalModel(variants,epoch,civilization){
 let base=null;
 for(const variant of variants){
  if(variant.regionalCivilizations)continue;
  if(base===null)base=variant;
  if(variant.epoch<=epoch)base=variant;else break;
 }
 // Own neutral14 and unavailable source contexts retain the existing asset.
 if(base===null||!Number.isInteger(civilization)||civilization<0||civilization>13)return base;
 const selected=variants.find(variant=>variant.regionalCivilizations?.includes(civilization)&&variant.epoch===base.epoch&&epoch>=variant.epoch&&epoch<variant.until);
 return selected||base;
}
