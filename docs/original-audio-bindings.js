// Typed audio presentation adapter. No simulation command or raw render ID is
// interpreted here. Legacy flags describe the established 57-role adapter only.
const LEGACY_FLAGS=Object.freeze({0:1,1:1,2:1,3:1,4:1,5:1,6:1,7:1,8:6,9:2,10:2,11:2,12:2,13:2,14:2,15:2,16:2,17:2,18:2,19:2,20:2,21:2,22:2,23:2,24:2,25:2,26:2,27:2,28:2,29:2,30:2,31:2,32:2,33:2,34:2,35:2,36:2,37:2,38:2,39:2,40:2,41:2,42:2,43:2,44:2,45:2,46:2,47:1,48:1,49:1,50:1,51:1,52:1,53:1,54:1,55:1,56:1});
export function createTypedAudioBindings({wasm=null,bindings=[],familyManifest=null}={}){
 const definitions=new Map(),errors=[],reported=new Set();
 const fail=message=>{if(!reported.has(message)){reported.add(message);errors.push(message);}};
 for(const row of bindings){
  if(row.assetKind!==0||!row.sourceFamily)continue;
  if(!Number.isInteger(row.id)||row.id<0||row.id>65535||!Number.isInteger(row.epoch)||row.epoch<0||row.epoch>=15||typeof row.sourceFamily!=='string'){fail('Invalid source-family audio definition');continue;}
  const list=definitions.get(row.id)||[];
  if(list.some(other=>other.epoch===row.epoch)){fail('Duplicate source-family audio interval '+row.id+':'+row.epoch);continue;}
  list.push({...row});definitions.set(row.id,list);
 }
 for(const list of definitions.values())list.sort((a,b)=>a.epoch-b.epoch);
 if(familyManifest){
  if(familyManifest.format!=='ee2-native-family-audio-v1'||!familyManifest.families||!familyManifest.events)throw Error('Unsupported native family audio manifest');
  for(const row of Object.values(familyManifest.events))if(typeof row.file!=='string'||!row.file||row.file.includes('\\')||row.file.includes(':')||row.file.startsWith('/')||row.file.split('/').includes('..')||!Number.isFinite(row.volume)||row.volume<0||row.volume>1)throw Error('Invalid native audio event');
  for(const family of Object.values(familyManifest.families)){
   if(!['voice','selection-effect'].includes(family.role)||!Array.isArray(family.intervals))throw Error('Invalid native audio family');
   let until=0;
   for(const row of family.intervals){
    if(!Number.isInteger(row.epoch)||!Number.isInteger(row.until)||row.epoch<until||row.epoch>=row.until||row.until>15||!row.effectiveEvents)throw Error('Invalid native audio interval');
    until=row.until;
    for(const action of ['select','order','attack']){const events=row.effectiveEvents[action];if(!Array.isArray(events)||events.length>8||events.some(name=>!familyManifest.events[name]))throw Error('Unbound native audio event array');}
   }
  }
 }
 function role(id){
  if(!Number.isInteger(id)||id<0||id>65535)return null;
  let flags;
  if(typeof wasm?.definition_flags==='function'){
   const count=typeof wasm.definition_count==='function'?wasm.definition_count():65536;
   if(id>=count)return null;flags=wasm.definition_flags(id)>>>0;if(flags===0xffffffff)return null;
  }else flags=LEGACY_FLAGS[id];
  if(flags===undefined)return null;
  return {building:!!(flags&1),mobile:!!(flags&2),worker:!!(flags&4),flags,legacy:Object.hasOwn(LEGACY_FLAGS,id)};
 }
 function resolve(id,action,epoch){
  const kind=role(id);if(!kind||kind.legacy||!Number.isInteger(epoch)||epoch<0||epoch>=15)return null;
  const definition=definitions.get(id)?.findLast(row=>row.epoch<=epoch&&(row.until===undefined||epoch<row.until));if(!definition)return null;
  const family=familyManifest?.families?.[definition.sourceFamily],interval=family?.intervals?.find(row=>row.epoch<=epoch&&epoch<row.until);if(!interval)return null;
  if(definition.sourceVisual&&definition.sourceVisual!==interval.sourceVisual){fail('Source-family audio visual differs for '+id+':'+epoch);return null;}
  if(family.role==='selection-effect'&&!kind.building||family.role==='voice'&&!kind.mobile){fail('Source-family audio role differs for '+id);return null;}
  // DbUnitTypeSounds has selection/move/attack/death, no build channel. The
  // existing Citizen build-to-order adapter remains solely in legacy audio.
  if(!['select','order','attack'].includes(action)||family.role==='selection-effect'&&action!=='select')return null;
  const names=interval.effectiveEvents[action]||[],records=names.map(name=>familyManifest.events[name]);
  if(!records.length||records.some(row=>!row?.file))return null;
  return {namespace:'native-family',sourceFamily:definition.sourceFamily,sourceVisual:interval.sourceVisual,sourceEpoch:interval.sourceEpoch,
   action,records,group:family.role==='selection-effect'?'effect':'voice',gain:family.role==='selection-effect'?1:.75,
   key:id+':'+interval.epoch+':'+action,sourceEventBinding:interval.effectiveEventSources[action]};
 }
 return {role,resolve,errors,isLegacy:id=>Object.hasOwn(LEGACY_FLAGS,id),usedFiles(){
  const files=new Set();for(const [id,list]of definitions){if(Object.hasOwn(LEGACY_FLAGS,id))continue;for(const definition of list){const family=familyManifest?.families?.[definition.sourceFamily];for(const interval of family?.intervals||[])for(const names of Object.values(interval.effectiveEvents))for(const event of names){const file=familyManifest.events[event]?.file;if(file)files.add(file);}}}return [...files];
 },status(){return {definitions:definitions.size,families:familyManifest?Object.keys(familyManifest.families).length:0,errors:[...errors]};}};
}
