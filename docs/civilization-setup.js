// The source Player civilization enum (TTCiv is this value+1).
// Random lobby choices are resolved once by the host and transmitted explicitly;
// they never consume the deterministic simulation's RNG.
export const CIVILIZATIONS=Object.freeze([
 {id:0,source:'German',name:'Deutsche'},
 {id:1,source:'English',name:'Engländer'},
 {id:2,source:'Roman',name:'Römer'},
 {id:3,source:'American',name:'Amerikaner'},
 {id:4,source:'Greek',name:'Griechen'},
 {id:5,source:'Chinese',name:'Chinesen'},
 {id:6,source:'Korean',name:'Koreaner'},
 {id:7,source:'Japanese',name:'Japaner'},
 {id:8,source:'Egyptian',name:'Ägypter'},
 {id:9,source:'Turkish',name:'Türken'},
 {id:10,source:'Babylonian',name:'Babylonier'},
 {id:11,source:'Inca',name:'Inka'},
 {id:12,source:'Aztec',name:'Azteken'},
 {id:13,source:'Mayan',name:'Maya'},
]);
export const RANDOM_CIVILIZATION=15,UNUSED_CIVILIZATION=14;
export function civilizationChoice(value){
 if(!Number.isInteger(value)||value<0||value>15||value===UNUSED_CIVILIZATION)throw Error('Ungültige Völkerwahl.');
 return value;
}
export function resolveCivilizations(choices,nplayers,{random=()=>Math.floor(Math.random()*CIVILIZATIONS.length)}={}){
 if(!Array.isArray(choices)||choices.length!==4||!Number.isInteger(nplayers)||nplayers<2||nplayers>4)throw Error('Ungültige Spielereinstellung.');
 const resolved=choices.map((value,index)=>{
  if(index>=nplayers)return UNUSED_CIVILIZATION;
  const choice=civilizationChoice(value),result=choice===RANDOM_CIVILIZATION?random():choice;
  if(!Number.isInteger(result)||result<0||result>=CIVILIZATIONS.length)throw Error('Zufällige Völkerwahl fehlgeschlagen.');
  return result;
 });
 return checkedCivilizations(resolved,nplayers);
}
export function checkedCivilizations(values,nplayers){
 if(!Array.isArray(values)||values.length!==4||!Number.isInteger(nplayers)||nplayers<2||nplayers>4||
  Array.from(values).some((v,i)=>!Number.isInteger(v)||(i<nplayers?v<0||v>=CIVILIZATIONS.length:v!==UNUSED_CIVILIZATION)))throw Error('Ungültige Völkerdaten für die Spieler.');
 return [...values];
}
export function applyCivilizations(wasm,values,nplayers){
 if(typeof wasm.configure_civilizations!=='function'){
  if(values!==null&&values!==undefined)throw Error('Diese Regelversion unterstützt keine Völkerwahl.');
  return false;
 }
 const checked=checkedCivilizations(values,nplayers),packed=checked.reduce((n,v,i)=>n|(v<<(i*4)),0);
 if(wasm.configure_civilizations(packed)!==1)throw Error('Die Völker konnten für dieses neue Spiel nicht übernommen werden.');
 return true;
}
export function applyMatchSettings(wasm,settings,nplayers){
 const values=checkedCivilizations(settings?.civilizations,nplayers);
 if(![250,500,1000].includes(settings.popLimit)||![0,1,2].includes(settings.supplies)||
  !Number.isInteger(settings.epoch)||settings.epoch<0||settings.epoch>=wasm.epoch_count())throw Error('Ungültige Scharmützeleinstellungen.');
 if(typeof wasm.configure_match_settings!=='function')throw Error('Die Spieldatei unterstützt diese Scharmützeleinstellungen noch nicht.');
 const packed=values.reduce((n,v,i)=>n|(v<<(i*4)),0);
 if(wasm.configure_match_settings(packed,settings.popLimit,settings.supplies,settings.epoch)!==1)throw Error('Die Einstellungen konnten für dieses neue Spiel nicht übernommen werden.');
 return true;
}

export function createCivilizationSetup({wasm,document,anchor}={}){
 const enabled=typeof wasm?.configure_civilizations==='function'&&wasm.civilization_count?.()===CIVILIZATIONS.length;
 if(!enabled)return Object.freeze({enabled:false,choice:()=>null,choices:()=>null,resolve:()=>null});
 if(!document||!anchor)throw Error('Völkerwahl konnte nicht angezeigt werden.');
 const section=document.createElement('fieldset');section.id='civilization-options';
 section.style.cssText='margin:12px 0;padding:10px;border:1px solid #526067;border-radius:6px';
 const legend=document.createElement('legend');legend.textContent='Völker';section.append(legend);
 const selects=[];
 for(let seat=0;seat<4;seat++){
  const row=document.createElement('div');row.className='row';
  const label=document.createElement('label');label.htmlFor='civilization-'+seat;label.textContent=seat===0?'Dein Volk':'Spieler '+(seat+1);
  const select=document.createElement('select');select.id=label.htmlFor;
  for(const entry of [...CIVILIZATIONS,{id:RANDOM_CIVILIZATION,name:'Zufällig'}]){
   const option=document.createElement('option');option.value=String(entry.id);option.textContent=entry.name;select.append(option);
  }
  select.value=seat===0?'0':String(RANDOM_CIVILIZATION);selects.push(select);row.append(label,select);section.append(row);
 }
 anchor.before(section);
 const choice=seat=>{
  if(!Number.isInteger(seat)||seat<0||seat>3)throw Error('Ungültiger Spieler.');
  return civilizationChoice(Number(selects[seat].value));
 };
 const choices=()=>selects.map((_,i)=>choice(i));
 return Object.freeze({enabled:true,choice,choices,resolve:(nplayers,options)=>resolveCivilizations(choices(),nplayers,options)});
}
