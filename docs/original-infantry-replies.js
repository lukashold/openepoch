const identities=Object.freeze({
  9:['HeavyInfantry1',9],10:['LightInfantry',10],11:['HeavyInfantry2',11],
  12:['LightInfantry',10],13:['HeavyInfantry1',9],14:['LightInfantry',10],
  15:['HeavyInfantry1',9],16:['HeavyInfantry1',9],17:['HeavyInfantry1',9],
});
const actions=['select','order','attack'];
const checksum='718b2eeb63b20b083b3221940e99a926180a504b7d5116da8ac16f74c6a316a2';

export const handlesOriginalInfantryReplies=definition=>Number.isInteger(definition)&&Object.hasOwn(identities,definition);

export async function loadInfantryReplyCatalog(fetchFile,base) {
  const url=new URL('infantry-replies/manifest.json',base);
  url.searchParams.set('v',checksum.slice(0,16));
  const response=await fetchFile(url,{cache:'force-cache'});
  if(!response.ok)throw Error('Original infantry replies unavailable');
  const bytes=await response.arrayBuffer();
  const hash=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if(Array.from(hash,byte=>byte.toString(16).padStart(2,'0')).join('')!==checksum)throw Error('Original infantry reply catalog differs');
  const data=JSON.parse(new TextDecoder().decode(bytes));
  if(data.format!=='ee2-private-infantry-replies-v1')throw Error('Unknown infantry reply catalog');
  const records=new Map(Object.entries(data.events).map(([name,record])=>[name,{...record,url:new URL(record.file,base)}]));
  return {
    records:[...records.values()],
    plan(definition,action,epoch) {
      if(!handlesOriginalInfantryReplies(definition)||!actions.includes(action)||!Number.isInteger(epoch))return null;
      const [family,sourceIdentity]=identities[definition];
      const row=data.families[family].find(value=>value.epoch<=epoch&&epoch<value.until);
      if(!row)return null;
      return {sourceIdentity,action,records:row.banks[action].map(name=>records.get(name)),gain:1};
    },
  };
}
