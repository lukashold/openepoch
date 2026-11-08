import {RESOURCE_NAMES} from './economy-bar.js?v=economy4';
import {territoryPanel} from './territory-panel.js?v=capital139';

const RELATIONS=['Feindlich','Neutral','Verbündet'];
const COLORS=['#448fff','#ef4540','#50ce62','#e5c33d'];
const MODES=['Unerforscht · mit Kriegsnebel','Erkundet · mit Kriegsnebel','Vollständig aufgedeckt · ohne Kriegsnebel'];
const SIGHT_TERMS=['Keine gemeinsame Sicht','Volle gemeinsame Sicht','Nur Gebäude','Nur mobile Einheiten'];
const BORDER_TERMS=['Freier Durchgang','Bürger und Handelswagen','Nur Handelswagen','Geschlossene Grenzen'];
const el=(tag,text,className)=>{const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(className)node.className=className;return node;};
function installStyle(){
 const style=el('style');style.textContent=`
 #diplomacy-dialog{box-sizing:border-box;width:min(920px,94vw);max-height:90vh;padding:24px;color:#e8e7d5;background:linear-gradient(135deg,#293b40,#101d23);border:1px solid #a9976e;border-radius:8px;box-shadow:0 20px 90px #000a;font:14px system-ui}
 #diplomacy-dialog::backdrop{background:#0007}
 #diplomacy-dialog header{display:flex;align-items:center;justify-content:space-between;gap:20px}
 #diplomacy-dialog h2{margin:0;font-size:28px;font-weight:650}
 #diplomacy-dialog h3{margin:0 0 12px;font-size:20px}
 #diplomacy-dialog p{line-height:1.5;margin:8px 0 16px;color:#c6cebf}
 #diplomacy-dialog button,#diplomacy-dialog select,#diplomacy-dialog input{font:inherit;box-sizing:border-box;color:inherit;background:#23343b;border:1px solid #738275;border-radius:4px;padding:8px 11px}
 #diplomacy-dialog button{cursor:pointer}
 #diplomacy-dialog button:hover:not(:disabled){background:#344c4c;border-color:#e7d5a0}
 #diplomacy-dialog button:disabled{cursor:default;opacity:.42}
 #diplomacy-dialog button:focus-visible,#diplomacy-dialog select:focus-visible,#diplomacy-dialog input:focus-visible{outline:2px solid #e5cf8b;outline-offset:2px}
 #diplomacy-dialog .dip-layout{display:grid;grid-template-columns:200px minmax(0,1fr);gap:18px}
 #diplomacy-dialog .dip-players{display:flex;flex-direction:column;gap:8px}
 #diplomacy-dialog .dip-player{display:grid;grid-template-columns:14px 1fr;gap:5px 9px;text-align:left;padding:12px}
 #diplomacy-dialog .dip-player[aria-pressed=true]{border-color:#e0c47e;background:#3e5146;box-shadow:inset 3px 0 #ddc283}
 #diplomacy-dialog .dip-player small{grid-column:2;color:#d6d7c7;font-size:12px}
 #diplomacy-dialog .dip-swatch{width:11px;height:11px;display:inline-block;border:1px solid #fff7;border-radius:2px;align-self:center}
 #diplomacy-dialog .dip-detail{border:1px solid #71807075;border-radius:4px;padding:18px;background:#09171942;min-width:0}
 #diplomacy-dialog .dip-detail>h3{display:inline-block;margin-right:12px;margin-bottom:8px;vertical-align:middle}
 #diplomacy-dialog .dip-relation{display:inline-block;border:1px solid #9b805c;padding:4px 10px;border-radius:3px;margin-bottom:6px}
 #diplomacy-dialog .dip-actions{display:flex;flex-wrap:wrap;gap:7px;margin:12px 0}
 #diplomacy-dialog .dip-war{border-color:#b27668;background:#4a2e2c}
 #diplomacy-dialog .dip-offer{padding:12px;background:#70633144;border:1px solid #bca66b;border-radius:4px;margin:12px 0}
 #diplomacy-dialog .dip-offer[hidden],#diplomacy-dialog [hidden]{display:none}
 #diplomacy-dialog .dip-outgoing{display:flex;align-items:center;justify-content:space-between;gap:8px;margin:10px 0;color:#ddd0a7}
 #diplomacy-dialog .dip-outgoing button{font-size:12px}
 #diplomacy-dialog label{display:block;margin:10px 0 5px;color:#d2d8c8}
 #diplomacy-dialog .dip-gift{display:flex;gap:8px;flex-wrap:wrap}
 #diplomacy-dialog .dip-gift select{min-width:140px;flex:1}
 #diplomacy-dialog .dip-gift input{width:95px}
 #diplomacy-dialog .dip-note{font-size:12px;color:#a9b7aa;margin:7px 0}
 #diplomacy-dialog .dip-vision-label{display:flex;align-items:center;gap:9px;margin:12px 0}
 #diplomacy-dialog .dip-vision-label input{accent-color:#728942;width:17px;height:17px;margin:0}
 #diplomacy-dialog .dip-message{min-height:20px;margin:10px 0 0;color:#f1dc9d}
 #diplomacy-dialog .dip-log{border-top:1px solid #6374607a;margin-top:18px;padding-top:12px;max-height:110px;overflow:auto;font-size:12px;color:#bec9b9;line-height:1.6}
 #diplomacy-dialog .dip-map{font-size:12px;margin:15px 0 0;color:#acbaac}
 #diplomacy-dialog .dip-pause{padding:9px 12px;margin:12px 0;background:#9b7f3c33;border:1px solid #9e884f}
 #diplomacy-dialog .dip-pause button{margin-left:12px;padding:4px 9px}
 #diplomacy-dialog .dip-tabs{display:flex;gap:6px;border-bottom:1px solid #7a876d;margin:7px 0 13px;padding-bottom:7px}
 #diplomacy-dialog .dip-tabs button[aria-selected=true]{border-color:#d0b66a;background:#415444}
 .original-art #diplomacy-dialog .dip-tabs button[aria-selected=true]{border-color:#756a3e;background:#d1d3ad}
 .original-art #diplomacy-dialog{color:#1b2719;background-color:#b9c1ac;background-image:linear-gradient(135deg,#cbd0c1ed,#b4bdaaf2),var(--ee2-panel,none);background-position:center;background-size:100% 100%,calc(100% + 64px) calc(100% + 64px);background-repeat:no-repeat;border:3px ridge #bdc4aa;box-shadow:0 20px 90px #000b}
 .original-art #diplomacy-dialog p,.original-art #diplomacy-dialog label,.original-art #diplomacy-dialog .dip-note,.original-art #diplomacy-dialog .dip-map,.original-art #diplomacy-dialog .dip-log{color:#31402b}
 .original-art #diplomacy-dialog button,.original-art #diplomacy-dialog select,.original-art #diplomacy-dialog input{color:#172514;background-color:#b5bba7;border-color:#66715b}
 .original-art #diplomacy-dialog button:hover:not(:disabled){background-color:#ccd1b9;border-color:#4f643c}
 .original-art #diplomacy-dialog .dip-player small{color:#364530}
 .original-art #diplomacy-dialog .dip-player[aria-pressed=true]{background:#d0d1ac;border-color:#6c6841;box-shadow:inset 3px 0 #8b763c}
 .original-art #diplomacy-dialog .dip-detail{background:#e5e6cd60;border-color:#6d785f}
 .original-art #diplomacy-dialog .dip-war{background:#c2a094;border-color:#8d5949}
 .original-art #diplomacy-dialog .dip-message,.original-art #diplomacy-dialog .dip-outgoing{color:#4f4216}
 @media(max-width:640px){#diplomacy-dialog{padding:16px}#diplomacy-dialog .dip-layout{grid-template-columns:1fr}#diplomacy-dialog .dip-players{flex-direction:row;flex-wrap:wrap}#diplomacy-dialog .dip-player{flex:1;min-width:125px}#diplomacy-dialog .dip-detail{padding:12px}}
 #diplomacy-dialog [hidden]{display:none!important}
 `;document.head.append(style);
}

export function diplomacyPanel({wasm,referenceMode,onOrder=()=>{},onOpen=()=>{},onClose=()=>{},isPaused=()=>false,resume=()=>{},onTerritoryFocus=()=>{}}){
 installStyle();
 const dialog=el('dialog');dialog.id='diplomacy-dialog';dialog.setAttribute('aria-labelledby','diplomacy-title');
 const panelTexture=document.getElementById('bottom')?.style.getPropertyValue('--ee2-panel');if(panelTexture)dialog.style.setProperty('--ee2-panel',panelTexture);
 const header=el('header'),heading=el('h2','Diplomatie');heading.id='diplomacy-title';const closeButton=el('button','Schließen · Esc');closeButton.type='button';header.append(heading,closeButton);
 const intro=el('p','Verhandle Frieden und Bündnisse, schenke Rohstoffe oder tausche Territorien.');
 const pause=el('div',undefined,'dip-pause');pause.append(el('span','Das Spiel ist pausiert.'));const resumeButton=el('button','Spiel fortsetzen');resumeButton.onclick=resume;pause.append(resumeButton);
 const layout=el('div',undefined,'dip-layout'),players=el('nav',undefined,'dip-players');players.setAttribute('aria-label','Spieler');
 const detail=el('section',undefined,'dip-detail'),name=el('h3'),relation=el('span',undefined,'dip-relation'),theirRelation=el('p',undefined,'dip-note'),rights=el('p',undefined,'dip-note');
 const offer=el('div',undefined,'dip-offer'),offerText=el('strong'),offerButtons=el('div',undefined,'dip-actions');
 const accept=el('button','Annehmen'),reject=el('button','Ablehnen');offerButtons.append(accept,reject);offer.append(offerText,offerButtons);
 const outgoing=el('div',undefined,'dip-outgoing'),outgoingText=el('span'),withdraw=el('button','Zurückziehen');outgoing.append(outgoingText,withdraw);
 const actions=el('div',undefined,'dip-actions'),peace=el('button','Frieden anbieten'),alliance=el('button','Bündnis anbieten'),war=el('button','Krieg erklären','dip-war');actions.append(peace,alliance,war);const cooldown=el('p',undefined,'dip-note');
 const visionLabel=el('label',undefined,'dip-vision-label');visionLabel.htmlFor='dip-vision';
 const vision=el('input');vision.type='checkbox';vision.checked=true;vision.id='dip-vision';vision.setAttribute('aria-label','Bündnis mit gemeinsamer Sicht anbieten');visionLabel.append(vision,el('span','Bündnis mit gemeinsamer Sicht anbieten'));
 const sightLabel=el('label','Gemeinsame Sicht');sightLabel.htmlFor='dip-sight-term';sightLabel.hidden=true;
 const sight=el('select');sight.id='dip-sight-term';sight.hidden=true;
 for(const term of [1,2,3,0])sight.add(new Option(SIGHT_TERMS[term],String(term)));
 const durationLabel=el('label','Bündnisdauer');durationLabel.htmlFor='dip-duration';durationLabel.hidden=true;
 const duration=el('select');duration.id='dip-duration';duration.hidden=true;
 duration.add(new Option('Bis zur Kriegserklärung','0'));
 for(let minutes=2;minutes<=30;minutes++)duration.add(new Option(`${minutes} Minuten`,String(minutes)));
 const borderLabel=el('label','Grenzdurchgang');borderLabel.htmlFor='dip-border';borderLabel.hidden=true;
 const border=el('select');border.id='dip-border';border.hidden=true;
 BORDER_TERMS.forEach((term,i)=>border.add(new Option(term,String(i))));
 const visionChange=el('button','Gemeinsame Sicht anbieten');visionChange.type='button';visionChange.id='dip-sight-change';
 const giftLabel=el('label','Rohstoffe schenken');giftLabel.htmlFor='dip-resource';
 const gift=el('form',undefined,'dip-gift'),resource=el('select');resource.id='dip-resource';resource.setAttribute('aria-label','Geschenk-Rohstoff');
 const amount=el('input');amount.type='number';amount.min='1';amount.max='999999';amount.step='1';amount.value='100';amount.required=true;amount.setAttribute('aria-label','Geschenk-Menge');
 const send=el('button','Schenken');send.type='submit';gift.append(resource,amount,send);
 const funds=el('p',undefined,'dip-note'),status=el('p',undefined,'dip-message');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 const tabs=el('div',undefined,'dip-tabs');tabs.setAttribute('role','tablist');tabs.setAttribute('aria-label','Diplomatiebereich');
 const relationsTab=el('button','Beziehungen und Rohstoffe'),territoriesTab=el('button','Territorien');relationsTab.id='dip-relations-tab';territoriesTab.id='dip-territories-tab';
 for(const tab of [relationsTab,territoriesTab]){tab.type='button';tab.setAttribute('role','tab');}relationsTab.setAttribute('aria-controls','dip-relations-panel');territoriesTab.setAttribute('aria-controls','dip-territory-panel');tabs.append(relationsTab,territoriesTab);
 const relationsBody=el('section');relationsBody.id='dip-relations-panel';relationsBody.setAttribute('role','tabpanel');relationsBody.setAttribute('aria-labelledby','dip-relations-tab');
 relationsBody.append(theirRelation,rights,offer,outgoing,visionLabel,sightLabel,sight,durationLabel,duration,borderLabel,border,actions,visionChange,cooldown,giftLabel,gift,funds,status);
 const territories=territoryPanel({wasm,referenceMode,onOrder,onRefresh:()=>refresh(read()),onFocus(territory){close();onTerritoryFocus(territory);}});
 detail.append(name,relation,tabs,relationsBody,territories.element);layout.append(players,detail);
 const log=el('div',undefined,'dip-log');log.setAttribute('aria-label','Diplomatieverlauf');const map=el('p',undefined,'dip-map');
 dialog.append(header,intro,pause,layout,log,map);document.body.append(dialog);
 let ui=null,target=-1,playerKey='',resourceKey='',logKey='',sightKey='',view='relations';const playerButtons=new Map();
 const read=()=>{const ptr=wasm.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,ptr,wasm.ui_len())));};
 const state=()=>ui?.diplomacy;
 const current=()=>state()?.players?.find(p=>p.team===target);
 function setView(next){view=next;relationsBody.hidden=view!=='relations';territories.element.hidden=view!=='territories';relationsTab.setAttribute('aria-selected',String(view==='relations'));territoriesTab.setAttribute('aria-selected',String(view==='territories'));relationsTab.tabIndex=view==='relations'?0:-1;territoriesTab.tabIndex=view==='territories'?0:-1;if(view==='territories')territories.activate();}
 relationsTab.onclick=()=>setView('relations');territoriesTab.onclick=()=>setView('territories');
 for(const tab of [relationsTab,territoriesTab])tab.onkeydown=event=>{if(event.key==='ArrowLeft'||event.key==='ArrowRight'){event.preventDefault();setView(view==='relations'?'territories':'relations');(view==='relations'?relationsTab:territoriesTab).focus();}};
 function updateFunds(){
  const slot=+resource.value,stock=ui?.res?.[slot]||0,value=+amount.value,p=current();
  funds.textContent=`Verfügbar: ${Math.floor(stock).toLocaleString('de-DE')} ${!referenceMode&&slot===2?'Metall':RESOURCE_NAMES[slot]||''}`;
  send.disabled=!p?.alive||!Number.isSafeInteger(value)||value<1||value>stock;
 }
 function command(action,slot=0,value=0){
  if(!current()?.alive)return;
  const result=wasm.cmd_diplomacy(target,action,slot,value);
  if(result){status.textContent=action===5?'Rohstoffgeschenk gesendet.':action===2?'Kriegserklärung übermittelt.':action===3?'Angebot angenommen.':action===4?'Angebot abgelehnt.':action===6?'Angebot zurückgezogen.':'Angebot übermittelt.';onOrder();}
  else status.textContent='Aktion derzeit nicht möglich. Prüfe Beziehung, laufende Angebote und deinen Vorrat.';
  refresh(read());
 }
 peace.onclick=()=>command(0);
 alliance.onclick=()=>{
  const data=state();
  const sightTerm=data?.sightTerms?+sight.value:Number(vision.checked);
  const borderTerm=data?.borderTreaties?+border.value:0;
  command(1,sightTerm|(borderTerm<<2),data?.timedTreaties?+duration.value:0);
 };
 war.onclick=()=>command(2);accept.onclick=()=>command(3);reject.onclick=()=>command(4);withdraw.onclick=()=>command(6);
 visionChange.onclick=()=>command(7,0,state()?.sightTerms?+sight.value:current()?.sharedVision?0:1);
 sight.onchange=()=>refresh(ui);
 gift.onsubmit=e=>{e.preventDefault();updateFunds();if(!send.disabled)command(5,+resource.value,+amount.value);};resource.onchange=updateFunds;amount.oninput=updateFunds;
 function close(){if(!dialog.open)return;dialog.close();onClose();}
 closeButton.onclick=close;dialog.addEventListener('cancel',e=>{e.preventDefault();close();});
 function refresh(next){
  ui=next;const data=state();if(!data)return;
  const incomingCount=data.players.filter(p=>p.team!==data.local&&p.alive&&(p.incoming||p.territoryIncoming)).length;
  const toggle=document.getElementById('diplomacy-button');if(toggle)toggle.textContent=incomingCount?`Diplomatie (${incomingCount})`:'Diplomatie';
  if(!dialog.open)return;
  pause.hidden=!isPaused();const opponents=data.players.filter(p=>p.team!==data.local);
  if(!opponents.some(p=>p.team===target))target=opponents.find(p=>p.alive)?.team??opponents[0]?.team??-1;
  const key=opponents.map(p=>p.team).join(',');
  if(key!==playerKey){playerKey=key;players.replaceChildren();playerButtons.clear();for(const p of opponents){
   const button=el('button',undefined,'dip-player');button.type='button';const swatch=el('span',undefined,'dip-swatch');swatch.style.background=COLORS[p.team%COLORS.length];const title=el('strong'),sub=el('small');button.append(swatch,title,sub);
   button.onclick=()=>{target=p.team;status.textContent='';refresh(ui);};players.append(button);playerButtons.set(p.team,{button,title,sub});
  }}
  for(const p of opponents){const item=playerButtons.get(p.team);item.title.textContent=p.name||`Spieler ${p.team+1}${p.ai?' · KI':''}`;item.sub.textContent=(p.alive?RELATIONS[p.relation]||'Neutral':'Ausgeschieden')+(p.territoryIncoming?' · Tauschangebot':'');item.button.setAttribute('aria-pressed',String(target===p.team));}
  const p=current();detail.hidden=!p;
  if(p){
   const allianceAllowed=ui.soleSurvivor?.allianceAllowed!==false;
   const locked=p.fixedTeam!==undefined;
   const term=Number(p.sightTerm??p.sharedVision);
   const key=`${p.team}/${p.relation}/${term}`;
   if(sightKey!==key){sightKey=key;sight.value=String(p.relation===2?term:1);duration.value='0';border.value='0';}
   name.textContent=p.name||`Spieler ${p.team+1}${p.ai?' · KI':''}`;relation.textContent=p.alive?RELATIONS[p.relation]||'Neutral':'Ausgeschieden';
   theirRelation.hidden=!p.alive||p.theirRelation===undefined||p.theirRelation===p.relation;theirRelation.textContent=`Haltung zu dir: ${RELATIONS[p.theirRelation]||'Neutral'}`;
   rights.textContent=locked?`Feste Teams · Team ${p.fixedTeam}`:p.relation===2?(data.sightTerms?SIGHT_TERMS[term]:p.sharedVision?'Ihr teilt eure Sicht miteinander.':'Bündnis ohne gemeinsame Sicht.')
    :allianceAllowed?'Frieden und Bündnisse benötigen die Zustimmung des anderen Spielers.':'Die letzten beiden Spieler kämpfen einzeln um den Sieg.';
   if(p.treatySeconds>0){
    const seconds=p.treatySeconds;
    rights.textContent+=` · ${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')} verbleiben`;
   }
   if(data.borderTreaties&&p.relation===2&&!locked)rights.textContent+=' · '+BORDER_TERMS[p.borderTerm];
   const offerName=(kind,full,term)=>data.sightTerms&&kind>=2
    ? (kind===2?'Bündnis · ':'Sichtregel · ')+SIGHT_TERMS[term]
    : kind===2?(full?'Bündnis mit gemeinsamer Sicht':'Bündnis ohne gemeinsame Sicht'):kind===3?'Gemeinsame Sicht':kind===4?'Gemeinsame Sicht beenden':'Frieden';
   const offeredDuration=minutes=>minutes>0?` · ${minutes} Minuten`:'';
   const offeredBorder=(kind,term)=>data.borderTreaties&&kind===2?' · '+BORDER_TERMS[term]:'';
   offer.hidden=!p.alive||!p.incoming;offerText.textContent=offerName(p.incoming,p.incomingAllianceVision,p.incomingSightTerm)+offeredDuration(p.incomingMinutes)+offeredBorder(p.incoming,p.incomingBorderTerm)+' · Angebot erhalten';
   outgoing.hidden=!p.alive||!p.outgoing;outgoingText.textContent=offerName(p.outgoing,p.outgoingAllianceVision,p.outgoingSightTerm)+offeredDuration(p.outgoingMinutes)+offeredBorder(p.outgoing,p.outgoingBorderTerm)+' · Antwort ausstehend';
   const waiting=p.cooldown>0;cooldown.hidden=!p.alive||!waiting||!!p.outgoing;cooldown.textContent=`Neues Angebot in ${Math.ceil(p.cooldown||0)} Sek.`;
   peace.disabled=locked||!p.alive||(p.relation!==0&&p.theirRelation!==0)||!!p.outgoing||waiting;alliance.disabled=locked||!allianceAllowed||!p.alive||p.relation!==1||(p.theirRelation!==undefined&&p.theirRelation!==1)||!!p.outgoing||waiting;war.disabled=locked||!!p.treatyUnbreakable||!p.alive||p.relation===0;
   accept.disabled=locked||!allianceAllowed&&p.incoming===2;
   visionLabel.hidden=!!data.sightTerms||!p.alive||p.relation!==1;vision.disabled=alliance.disabled;
   sightLabel.hidden=sight.hidden=!data.sightTerms||!p.alive||p.relation===0||locked;
   sight.disabled=p.relation===1?alliance.disabled:!!p.outgoing||waiting;
   durationLabel.hidden=duration.hidden=!data.timedTreaties||!p.alive||p.relation!==1||locked;
   duration.disabled=alliance.disabled;
   borderLabel.hidden=border.hidden=!data.borderTreaties||!p.alive||p.relation!==1||locked;
   border.disabled=alliance.disabled;
   visionChange.hidden=!p.alive||p.relation!==2;visionChange.disabled=locked||!!p.outgoing||waiting||(!!data.sightTerms&&+sight.value===term);
   visionChange.textContent=data.sightTerms?'Sichtregel anbieten':p.sharedVision?'Gemeinsame Sicht beenden':'Gemeinsame Sicht anbieten';
   giftLabel.hidden=!p.alive;gift.hidden=!p.alive;funds.hidden=!p.alive;
  }
  const slots=(referenceMode?[0,1,3,4,5,2,6,7,8]:[0,1,2]).filter(slot=>!referenceMode||ui.resourceActive?.[slot]!==false);
  const rk=slots.join(',');if(rk!==resourceKey){resourceKey=rk;const previous=+resource.value;resource.replaceChildren(...slots.map(slot=>new Option(!referenceMode&&slot===2?'Metall':RESOURCE_NAMES[slot],String(slot))));resource.value=String(slots.includes(previous)?previous:slots.includes(1)?1:slots[0]);}
  updateFunds();
  territoriesTab.textContent=p?.territoryIncoming?'Territorien (1)':'Territorien';
  if(p)territories.refresh(ui,target);
  const entries=data.log||[],lk=JSON.stringify(entries);if(lk!==logKey){logKey=lk;log.replaceChildren(...entries.slice(-6).map(text=>el('div',String(text))));log.scrollTop=log.scrollHeight;}
  map.textContent=`Karte: ${MODES[data.visibility]||MODES[0]}`;
 }
 setView('relations');
 return {refresh,close,isOpen:()=>dialog.open,open(){if(dialog.open)return;ui=read();const pending=ui.diplomacy?.players.find(player=>player.territoryIncoming);if(pending){target=pending.team;view='territories';}playerKey='';resourceKey='';logKey='';status.textContent='';territories.reset();onOpen();dialog.showModal();setView(view);refresh(ui);}};
}
