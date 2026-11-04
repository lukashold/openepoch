// Numerical rule profiles are explicit; assets keep the declared visual adapters.
// Enable only after the imported UP binary and its source/ABI oracle are released.
export const UP15_PROFILE_ENABLED=true;
const PROFILES=Object.freeze({
 own:Object.freeze({id:'own',ruleset:0,label:'OpenEpoch-Regeln',wasm:'openepoch.wasm?v=combat-season1',database:'openepoch-v2'}),
 base:Object.freeze({id:'base',ruleset:1,label:'EE2 · Basisdaten',wasm:'openepoch-ee2.wasm?v=combat-season1',database:'openepoch-ee2-reference-v2'}),
 up15:Object.freeze({id:'up15',ruleset:2,label:'EE2 · installierte UP-Regeln',wasm:'openepoch-ee2-up15.wasm?v=up15-profile5',database:'openepoch-ee2-up15-v1'}),
});

export function resolveRulesProfile(input,{up15Enabled=UP15_PROFILE_ENABLED}={}){
 const url=new URL(input),reference=url.searchParams.get('rules')==='ee2';
 const id=reference?(url.searchParams.get('profile')??(up15Enabled?'up15':'base')):'own';
 if(!Object.hasOwn(PROFILES,id)||reference&&id==='own')throw Error('Unbekannte Regelversion. Wähle Basisdaten oder installierte UP-Regeln.');
 if(id==='up15'&&!up15Enabled)throw Error('Die installierten UP-Regeln sind noch nicht freigegeben. Wähle vorerst EE2 · Basisdaten.');
 const qa=url.searchParams.has('qa');
 return Object.freeze({...PROFILES[id],reference,qa,database:PROFILES[id].database+(qa?'-qa':'')});
}

export function rulesProfileURL(input,id,{up15Enabled=UP15_PROFILE_ENABLED}={}){
 if(!Object.hasOwn(PROFILES,id))throw Error('Unbekannte Regelversion.');
 if(id==='up15'&&!up15Enabled)throw Error('Die installierten UP-Regeln sind noch nicht freigegeben.');
 const url=new URL(input);
 if(id==='own'){url.searchParams.delete('rules');url.searchParams.delete('profile');}
 else{url.searchParams.set('rules','ee2');url.searchParams.set('profile',id);}
 return url;
}

export function assertRulesProfile(wasm,profile){
 if(typeof wasm?.ruleset!=='function'||wasm.ruleset()!==profile.ruleset)throw Error(`Falsche Spieldatei für ${profile.label}. Entpacke das vollständige Paket erneut; Regelprofile werden nicht automatisch ersetzt.`);
 return wasm;
}

/** One requested file, with MIME-compatible decoding fallback of the same bytes. */
export async function loadRulesWasm(profile,base,{fetchFile=globalThis.fetch,webAssembly=globalThis.WebAssembly}={}){
 try{
  const response=await fetchFile(new URL(profile.wasm,base));
  if(!response.ok)throw Error(`Datei fehlt oder ist nicht lesbar (HTTP ${response.status}).`);
  let result;
  if(typeof webAssembly.instantiateStreaming==='function'&&typeof response.clone==='function'){
   try{result=await webAssembly.instantiateStreaming(response.clone(),{});}catch{result=await webAssembly.instantiate(await response.arrayBuffer(),{});}
  }else result=await webAssembly.instantiate(await response.arrayBuffer(),{});
  return assertRulesProfile(result.instance.exports,profile);
 }catch(error){throw Error(`${profile.label} konnte nicht geladen werden. ${error.message}`);}
}

// Legacy records in the existing own/BASE databases predate these two fields.
// The new UP database has no implicit legacy import or cross-profile migration.
export function savedRulesProfileMatches(record,profile){
 if(!record||typeof record!=='object')return false;
 if(record.rules===undefined&&record.profile===undefined)return profile.ruleset!==2;
 return record.rules===profile.ruleset&&record.profile===profile.id;
}
