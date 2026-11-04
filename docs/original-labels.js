// Original text stays in the local media folder; the independent profile uses its own labels.
export function escapeLabelHtml(value){
 return String(value??'').replace(/[&<>"']/g,ch=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch]));
}

function checkedRows(data,profile){
 if(data?.format!=='ee2-native-labels-v1'||data.language!=='de')throw new Error('Unsupported original-label manifest');
 const source=data.profiles?.[profile.id];
 if(!source||source.profile!==profile.id||source.ruleset!==profile.ruleset||!Array.isArray(source.rows))throw new Error('Original-label profile differs');
 const rows=new Map();
 for(const row of source.rows){
  if(!Number.isInteger(row.id)||row.id<0||row.id>=57||!Number.isInteger(row.epoch)||!Number.isInteger(row.until)||row.epoch<0||row.until>15||row.epoch>=row.until||typeof row.name!=='string'||!row.name||typeof row.plural!=='string'||typeof row.description!=='string')throw new Error('Invalid original-label row');
  const list=rows.get(row.id)||[];
  if(list.some(other=>row.epoch<other.until&&other.epoch<row.until))throw new Error('Overlapping original-label class intervals');
  list.push(Object.freeze({...row}));rows.set(row.id,list);
 }
 return rows;
}

export function originalLabelCatalog(data,profile){
 const rows=profile?.reference?checkedRows(data,profile):new Map();
 const lookup=(id,epoch)=>Number.isInteger(id)&&Number.isInteger(epoch)&&epoch>=0&&epoch<15?
  rows.get(id)?.find(row=>epoch>=row.epoch&&epoch<row.until):undefined;
 return Object.freeze({enabled:rows.size>0,profile:profile.id,
  name(id,epoch,fallback,count=1){const row=lookup(id,epoch);return row?(count>1&&row.plural?row.plural:row.name):fallback;},
  description(id,epoch){const value=lookup(id,epoch)?.description||'';
   // Native variable/format tokens need the original formatter. Keep the current
   // runtime-stat tooltip when that formatter has not been reconstructed.
   return /%[A-Za-z_]|[{}]|\\[A-Za-z]/.test(value)?'':value;
  }
 });
}

export async function loadOriginalLabels(profile,{fetchImpl=globalThis.fetch,base=import.meta.url}={}){
 if(!profile?.reference)return originalLabelCatalog(null,profile);
 try{
  const response=await fetchImpl(new URL('./local-original/labels-native.json',base),{cache:'no-store'});
  if(!response.ok)throw new Error('Original labels: HTTP '+response.status);
  return originalLabelCatalog(await response.json(),profile);
 }catch(error){
  console.warn('Originalnamen konnten nicht vollständig geladen werden:',error);
  return Object.freeze({enabled:false,profile:profile.id,name:(_id,_epoch,fallback)=>fallback,description:()=>''});
 }
}
