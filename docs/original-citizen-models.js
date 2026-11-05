// Source identity is supplied by the authoritative core through the actor's
// full opaque handle. Rendering never chooses gender or consumes simulation RNG.
export function citizenModelEntries(manifest){
 if(manifest?.format!=='ee2-native-citizen-model-manifest-v1'||manifest.incomplete||!Array.isArray(manifest.models))throw new Error('Unsupported or incomplete Citizen model manifest');
 const seen=new Set();
 return manifest.models.map(row=>{
  if(row.assetKind!==0||row.id!==8||![0,1].includes(row.sourceIdentity)||![0,3,6,9,12].includes(row.epoch)||row.until!==row.epoch+3||!Array.isArray(row.regionalCivilizations)||!row.regionalCivilizations.length||!Number.isInteger(row.region)||row.region<0||row.region>3)throw new Error('Invalid source Citizen model identity');
  if(row.nativeScaleBinding?.sourceSha256!==row.source?.sha256||!row.nativeScaleBinding?.verified)throw new Error('Citizen scale/source differs');
  for(const civ of row.regionalCivilizations){const key=`${row.sourceIdentity}:${row.epoch}:${civ}`;if(!Number.isInteger(civ)||civ<0||civ>13||seen.has(key))throw new Error('Invalid or duplicate Citizen context');seen.add(key);}
  return{...row,regionalCivilizations:Object.freeze([...row.regionalCivilizations])};
 });
}

export function selectCitizenModel(variants,epoch,civilization,identity,{neutralFallback=null}={}){
 if(identity===undefined)return undefined;
 if(identity!==0&&identity!==1)return null;
 const candidates=variants.filter(v=>v.sourceIdentity===identity&&v.epoch<=epoch&&epoch<v.until);
 if(Number.isInteger(civilization)&&civilization>=0&&civilization<=13)return candidates.find(v=>v.regionalCivilizations.includes(civilization))||null;
 // Explicit own neutral-context adapter, not a nonexistent native civ14. A
 // female identity can only select a female asset, including on this fallback.
 if(neutralFallback==='west'&&(civilization===14||civilization===-1||civilization===undefined))return candidates.find(v=>v.region===0)||null;
 return null;
}

export function citizenPresentationCatalog(manifest,{neutralFallback=null}={}){
 if(manifest?.format!=='ee2-native-citizen-presentation-v1'||!Array.isArray(manifest.rows))throw new Error('Unsupported Citizen presentation manifest');
 function row(identity,civ,epoch){
  if(![0,1].includes(identity)||!Number.isInteger(epoch)||epoch<0||epoch>14)return null;
  const found=manifest.rows.filter(r=>r.sourceIdentity===identity&&r.epoch<=epoch&&epoch<r.until);
  if(Number.isInteger(civ)&&civ>=0&&civ<=13)return found.find(r=>r.regionalCivilizations.includes(civ))||null;
  if(neutralFallback==='west'&&(civ===14||civ===-1||civ===undefined))return found.find(r=>r.region===0)||null;
  return null;
 }
 return Object.freeze({
  lookup(identity,civ,epoch){const value=row(identity,civ,epoch);if(!value)return null;return{...value,icon:manifest.icons[value.icon],legacyContextFallback:!(Number.isInteger(civ)&&civ>=0&&civ<=13)};},
  sounds(identity,civ,epoch,action){const value=row(identity,civ,epoch);if(!value)return null;const key={select:'sndSelection',order:'sndCmdMove',attack:'sndCmdAttack',death:'sndDeath'}[action];if(!key)return null;return(value.soundEvents[key]||[]).map(name=>manifest.events[name]);}
 });
}
