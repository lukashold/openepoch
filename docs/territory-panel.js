// Territory selection and review. Ownership and transaction validation live in WASM.
import {territoryClaimMessage} from './territory-help.js?v=fortress1';
const NONE=0xffffffff,SIZE=256;
const el=(tag,text,className)=>{const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(className)node.className=className;return node;};
export function territoryPanel({wasm,referenceMode=false,onOrder,onRefresh,onFocus=()=>{}}){
 const style=el('style');style.textContent=`
 #diplomacy-dialog .territory-layout{display:grid;grid-template-columns:minmax(170px,.7fr) minmax(240px,1.3fr);gap:16px;margin-top:10px}
 #diplomacy-dialog .territory-map-wrap{width:100%;max-width:210px}
 #diplomacy-dialog .territory-map{display:block;width:100%;height:auto;aspect-ratio:1;image-rendering:pixelated;background:#111a19;border:1px solid #68765f;cursor:crosshair}
 #diplomacy-dialog .territory-legend{display:flex;gap:12px;flex-wrap:wrap;font-size:11px;margin-top:6px}
 #diplomacy-dialog .territory-legend i{display:inline-block;width:8px;height:8px;margin-right:4px;background:#4f9ee8}
 #diplomacy-dialog .territory-legend .wanted{background:#e5bd57}
 #diplomacy-dialog .territory-legend .inspected{background:#eef3de}
 #diplomacy-dialog .territory-field label{font-size:13px;margin-top:0}
 #diplomacy-dialog .territory-field+ .territory-field{margin-top:16px}
 #diplomacy-dialog .territory-field select{width:100%;min-width:0}
 #diplomacy-dialog .territory-field button{font-size:12px;padding:5px 8px;margin-top:5px}
 #diplomacy-dialog .territory-buttons{display:flex;flex-wrap:wrap;gap:7px;margin-top:14px}
 #diplomacy-dialog .territory-note{font-size:12px;line-height:1.5;margin:8px 0}
 #diplomacy-dialog .territory-details{margin-top:12px;font-size:12px}
 #diplomacy-dialog .territory-details summary{cursor:pointer;color:inherit}
 #diplomacy-dialog .territory-review{background:#7a693633;border:1px solid #a69664;border-radius:4px;padding:12px;margin-top:12px}
 #diplomacy-dialog .territory-review h4{font-size:15px;margin:0 0 7px}
 #diplomacy-dialog .territory-proposal{border:1px solid #839173;border-radius:4px;padding:10px;margin:10px 0;background:#d2d5bb1a}
 #diplomacy-dialog .territory-proposal p{margin:5px 0;font-size:13px}
 #diplomacy-dialog .territory-status{min-height:18px;font-size:13px;color:#efd69a;margin:10px 0 0}
 #diplomacy-dialog .territory-inspect{padding:10px;border:1px solid #839173;border-radius:4px;margin:12px 0}
 #diplomacy-dialog .territory-inspect select{max-width:100%;margin:4px 8px 4px 0}
 #diplomacy-dialog .territory-inspect p{font-size:13px;margin:6px 0}
 .original-art #diplomacy-dialog .territory-status{color:#564414}
 #diplomacy-dialog .territory-warning{color:#dfa487}
 .original-art #diplomacy-dialog .territory-warning{color:#883c23}
 @media(max-width:900px){#diplomacy-dialog .territory-layout{grid-template-columns:1fr}#diplomacy-dialog .territory-map-wrap{max-width:250px;width:100%}}
 `;document.head.append(style);
 const pane=el('section');pane.id='dip-territory-panel';pane.setAttribute('role','tabpanel');pane.setAttribute('aria-labelledby','dip-territories-tab');
 const incoming=el('div',undefined,'territory-proposal'),incomingTitle=el('strong','Tauschangebot erhalten'),incomingText=el('p'),incomingTime=el('p',undefined,'territory-note'),incomingActions=el('div',undefined,'territory-buttons');
 const accept=el('button','Tausch annehmen'),reject=el('button','Tausch ablehnen');incomingActions.append(accept,reject);incoming.append(incomingTitle,incomingText,incomingTime,incomingActions);
 const outgoing=el('div',undefined,'territory-proposal'),outgoingText=el('p'),outgoingTime=el('p',undefined,'territory-note'),withdraw=el('button','Tauschangebot zurückziehen');outgoing.append(el('strong','Dein Tauschangebot'),outgoingText,outgoingTime,withdraw);
 const layout=el('div',undefined,'territory-layout'),mapWrap=el('div',undefined,'territory-map-wrap'),canvas=el('canvas',undefined,'territory-map');canvas.width=SIZE;canvas.height=SIZE;canvas.setAttribute('role','img');canvas.setAttribute('aria-label','Territorienkarte. Blau: dein Angebot, Gold: gewünschtes Gebiet.');
 const legend=el('div',undefined,'territory-legend'),giveLegend=el('span'),wantLegend=el('span');giveLegend.append(el('i'),el('span','Dein Angebot'));wantLegend.append(el('i',undefined,'wanted'),el('span','Gewünschtes Gebiet'));const inspectLegend=el('span');inspectLegend.append(el('i',undefined,'inspected'),el('span','Ansicht'));legend.append(giveLegend,wantLegend,inspectLegend);mapWrap.append(canvas,legend);
 const fields=el('div'),giveField=el('div',undefined,'territory-field'),wantField=el('div',undefined,'territory-field');
 const giveLabel=el('label','Dein Territorium');giveLabel.htmlFor='territory-give';const give=el('select');give.id='territory-give';give.setAttribute('aria-label','Dein Territorium');
 const giveFocus=el('button','Eigenes Gebiet auf Karte zeigen');giveFocus.type='button';giveField.append(giveLabel,give,giveFocus);
 const wantLabel=el('label','Gewünschtes Territorium');wantLabel.htmlFor='territory-want';const want=el('select');want.id='territory-want';want.setAttribute('aria-label','Gewünschtes Territorium');
 const wantFocus=el('button','Gewünschtes Gebiet auf Karte zeigen');wantFocus.type='button';const wantSummary=el('p',undefined,'territory-note');wantField.append(wantLabel,want,wantFocus,wantSummary);fields.append(giveField,wantField);layout.append(mapWrap,fields);
 const counts=el('p',undefined,'territory-note'),policy=el('p',undefined,'territory-note'),reason=el('p',undefined,'territory-note'),warning=el('p',undefined,'territory-note territory-warning');
 const policyDetails=el('details',undefined,'territory-details');policyDetails.append(el('summary','Was wird mitgegeben?'),policy);
 const buttons=el('div',undefined,'territory-buttons'),gift=el('button','Territorium verschenken'),trade=el('button','Territorientausch anbieten');buttons.append(gift,trade);
 const review=el('div',undefined,'territory-review'),reviewTitle=el('h4'),reviewText=el('p',undefined,'territory-note'),reviewRisk=el('p',undefined,'territory-note territory-warning'),reviewActions=el('div',undefined,'territory-buttons'),confirm=el('button','Übergabe bestätigen'),cancel=el('button','Abbrechen');reviewActions.append(confirm,cancel);review.append(reviewTitle,reviewText,reviewRisk,reviewActions);
 reviewTitle.id='territory-review-title';reviewText.id='territory-review-summary';reviewRisk.id='territory-review-risk';review.setAttribute('role','group');review.setAttribute('aria-labelledby',reviewTitle.id);
 const status=el('p',undefined,'territory-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');pane.append(incoming,outgoing,layout,counts,reason,warning,buttons,policyDetails,review,status);
 const inspect=el('section',undefined,'territory-inspect'),inspectLabel=el('label','Gebiet ansehen'),inspectSelect=el('select'),inspectFocus=el('button','Auf der Karte zeigen'),inspectText=el('p'),inspectCounts=el('p',undefined,'territory-note');
 inspectSelect.id='territory-inspect';inspectLabel.htmlFor=inspectSelect.id;inspectFocus.type='button';inspectText.id='territory-claim-status';inspect.append(inspectLabel,inspectSelect,inspectFocus,inspectText,inspectCounts);layout.after(inspect);
 inspectFocus.onclick=()=>{const territory=area(+inspectSelect.value);if(territory?.known)onFocus(territory);};
 inspectSelect.onchange=()=>{dirty=true;refresh(ui,target);};
 let ui=null,target=-1,selectionKey='',reviewed=null,mapKey='',tileIds=null,lastMap=-1,dirty=true;
 const ctx=canvas.getContext('2d');
 const data=()=>ui?.diplomacy;
 const opponent=()=>data()?.players.find(player=>player.team===target);
 const area=id=>data()?.territories?.find(territory=>territory.id===id);
 const selectedGive=()=>give.value===''?null:area(+give.value);
 const selectedWant=()=>want.value===''?null:area(+want.value);
 const title=id=>area(id)?.name||`Territorium ${id+1}`;
 const label=territory=>`${territory.name} · ${territory.countsComplete===false?'mind. ':''}${territory.buildings} Gebäude`;
 function renderReview(territory){reviewTitle.textContent=`${territory.name} an Spieler ${target+1} verschenken?`;reviewText.textContent=`${territory.buildings} Gebäude${territory.units!==undefined?`, ${territory.units} Einheiten`:''}${territory.garrison?` und ${territory.garrison} Insassen`:''}. ${data().territoryPolicy||'Die Übergabe ändert den Besitzer des Gebiets.'}`;reviewRisk.hidden=!territory.lastTerritory;reviewRisk.textContent='Dies ist dein letztes Territorium. Bestehende Bündnisse werden aufgehoben. Ohne eigene Gebäude verlierst du die Partie.';confirm.setAttribute('aria-describedby',`${reviewTitle.id} ${reviewText.id}${territory.lastTerritory?' '+reviewRisk.id:''}`);}
 function replaceOptions(select,list){const previous=select.value;select.replaceChildren();if(!list.length){select.append(new Option('Kein Territorium verfügbar',''));select.disabled=true;}else{select.disabled=false;select.append(...list.map(territory=>new Option(label(territory),String(territory.id))));if(list.some(territory=>String(territory.id)===previous))select.value=previous;}}
 function clearReview(){reviewed=null;review.hidden=true;}
 function command(action,giveId=NONE,wantId=NONE){
  const result=wasm.cmd_territory(target,action,giveId,wantId);
  if(result){status.textContent=action===0?'Gebietsgeschenk gesendet.':action===1?'Tauschangebot gesendet.':action===2?'Tauschangebot angenommen.':action===3?'Tauschangebot abgelehnt.':'Tauschangebot zurückgezogen.';clearReview();onOrder();}
  else status.textContent='Übergabe derzeit nicht möglich. Prüfe den Gebietsbesitz und laufende Angebote.';
  onRefresh();dirty=true;
 }
 gift.onclick=()=>{const territory=selectedGive();if(!territory?.canGive)return;reviewed={id:territory.id,target};renderReview(territory);review.hidden=false;confirm.focus();};
 confirm.onclick=()=>{if(!reviewed||reviewed.target!==target||reviewed.id!==selectedGive()?.id)return clearReview();command(0,reviewed.id);};cancel.onclick=clearReview;
 trade.onclick=()=>{const a=selectedGive(),b=selectedWant();if(a&&b)command(1,a.id,b.id);};accept.onclick=()=>command(2);reject.onclick=()=>command(3);withdraw.onclick=()=>command(4);
 giveFocus.onclick=()=>{const territory=selectedGive();if(territory)onFocus(territory);};wantFocus.onclick=()=>{const territory=selectedWant();if(territory)onFocus(territory);};
 give.onchange=want.onchange=()=>{clearReview();status.textContent='';dirty=true;refresh(ui,target);};
 canvas.onclick=event=>{if(!data())return;const rect=canvas.getBoundingClientRect(),x=(event.clientX-rect.left)/rect.width*SIZE,y=(event.clientY-rect.top)/rect.height*SIZE,id=wasm.territory_at(x,y),territory=area(id);if(!territory?.known){status.textContent='Erkunde dieses Gebiet zuerst.';return;}
  inspectSelect.value=String(id);if(territory.owner===data().local)give.value=String(id);else if(territory.owner===target)want.value=String(id);clearReview();status.textContent='';dirty=true;refresh(ui,target);
 };
 function draw(){
  if(pane.hidden||!data()||!ctx)return;const now=performance.now();if(!dirty&&now-lastMap<400)return;lastMap=now;dirty=false;
  const key=data().territories.map(territory=>`${territory.id}:${territory.x}:${territory.y}`).join('|');
  if(!tileIds||mapKey!==key){mapKey=key;tileIds=new Uint8Array(SIZE*SIZE);for(let y=0;y<SIZE;y++)for(let x=0;x<SIZE;x++)tileIds[y*SIZE+x]=wasm.territory_at(x+.5,y+.5);}
  const ptr=wasm.minimap(),pixels=new Uint8ClampedArray(wasm.memory.buffer,ptr,SIZE*SIZE*4).slice(),a=selectedGive()?.id,b=selectedWant()?.id,inspected=inspectSelect.value===''?null:+inspectSelect.value;
  for(let y=0;y<SIZE;y++)for(let x=0;x<SIZE;x++){const i=y*SIZE+x,id=tileIds[i];if(id!==a&&id!==b&&id!==inspected)continue;const border=x===0||y===0||x===SIZE-1||y===SIZE-1||tileIds[i-1]!==id||tileIds[i+1]!==id||tileIds[i-SIZE]!==id||tileIds[i+SIZE]!==id,c=id===a?[79,158,232]:id===b?[229,189,87]:[238,243,222],alpha=border ? .95 : ((id===a||id===b) ? .18 : 0);for(let k=0;k<3;k++)pixels[i*4+k]=pixels[i*4+k]*(1-alpha)+c[k]*alpha;}
  ctx.putImageData(new ImageData(pixels,SIZE,SIZE),0,0);
 }
 function refresh(next,nextTarget){
  ui=next;const d=data();if(!d?.territories)return;const p=d.players.find(player=>player.team===nextTarget);
  if(target!==nextTarget){target=nextTarget;selectionKey='';clearReview();status.textContent='';dirty=true;}
  const own=d.territories.filter(territory=>territory.owner===d.local),theirs=d.territories.filter(territory=>territory.owner===target),key=JSON.stringify([own,theirs].map(list=>list.map(territory=>[territory.id,territory.name,territory.buildings,territory.countsComplete])));
  if(selectionKey!==key){selectionKey=key;replaceOptions(give,own);replaceOptions(want,theirs);dirty=true;}
  const a=selectedGive(),b=selectedWant();
  const known=d.territories.filter(territory=>territory.known),inspected=inspectSelect.value;
  if(inspectSelect.dataset.key!==known.map(territory=>territory.id).join(',')){
   inspectSelect.dataset.key=known.map(territory=>territory.id).join(',');inspectSelect.replaceChildren(...known.map(territory=>new Option(territory.name,String(territory.id))));
   inspectSelect.value=known.some(territory=>String(territory.id)===inspected)?inspected:String(own[0]?.id??known[0]?.id??'');
  }
  const inspectedArea=area(+inspectSelect.value);inspectFocus.disabled=!inspectedArea?.known;
  inspectText.textContent=referenceMode?territoryClaimMessage(inspectedArea):!inspectedArea?.known?'Erkunde dieses Gebiet zuerst.':inspectedArea.owner===d.local?'Dieses Gebiet gehört dir.':inspectedArea.owner===255?'Freies Gebiet: Ein eigenes Stadtzentrum beansprucht es.':`Dieses Gebiet gehört Spieler ${inspectedArea.owner+1}.`;
  inspectCounts.hidden=!referenceMode||!inspectedArea?.known||!(inspectedArea.townCenters||inspectedArea.fortresses);
  inspectCounts.textContent=`Eigene Stadtzentren: ${inspectedArea?.townCenters||0} · Eigene Festungen: ${inspectedArea?.fortresses||0} / 2`;
  if(reviewed&&(!p?.alive||!a?.canGive||reviewed.id!==a.id||reviewed.target!==target))clearReview();
  if(reviewed)renderReview(a);
  const hostile=p?.relation===0||p?.theirRelation===0;
  gift.disabled=!p?.alive||!a?.canGive;trade.disabled=!p?.alive||hostile||!a||!b||a.transferable===false||b.transferable===false||!!p.territoryOutgoing||p.territoryCooldown>0;accept.disabled=!p?.alive||hostile;
  giveFocus.disabled=!a;wantFocus.disabled=!b;
  wantSummary.textContent=b?`${b.countsComplete===false?'Sichtbar':'Mitgabe'}: ${b.buildings} Gebäude${b.units!==undefined?`, ${b.units} Einheiten`:''}${b.garrison?`, ${b.garrison} Insassen`:''}.${b.countsComplete===false?' Weitere Objekte können noch unbekannt sein.':''}`:'';
  counts.textContent=a?`${a.name}: ${a.buildings} eigene Gebäude${a.units!==undefined?` · ${a.units} Einheiten`:''}${a.garrison?` · ${a.garrison} stationiert`:''}`:'Du besitzt derzeit kein Territorium.';
  policy.textContent=d.territoryPolicy||'Die eigenen Gebäude im Gebiet gehören zur Übergabe.';
  reason.textContent=a?.reason||(hostile?'Für einen Tausch zuerst gegenseitigen Frieden schließen.':!b?'Kein bekanntes Gebiet dieses Spielers für einen Tausch. Erkunde seine Gebiete zuerst.':p?.territoryCooldown>0?`Neues Tauschangebot in ${Math.ceil(p.territoryCooldown)} Sek.`:'Beide Seiten stimmen einem Tausch zu, bevor sich der Besitzer ändert.');
  const risks=[];if(a?.lastTerritory)risks.push('Beim Verschenken deines letzten Gebiets enden Bündnisse; ohne eigene Gebäude verlierst du.');if(p?.alive&&(p.relation===0||p.theirRelation===0))risks.push('Ein Geschenk an diesen Feind ist sofort wirksam.');warning.hidden=!risks.length;warning.textContent=risks.join(' ');
  incoming.hidden=!p?.territoryIncoming;outgoing.hidden=!p?.territoryOutgoing;
  if(p?.territoryIncoming){const offer=p.territoryIncoming;incomingText.textContent=`Spieler ${target+1} bietet ${title(offer.give)} gegen dein ${title(offer.want)}.`;incomingTime.textContent=`Läuft ab in ${Math.max(0,Math.ceil(offer.expires))} Sek.`;}
  if(p?.territoryOutgoing){const offer=p.territoryOutgoing;outgoingText.textContent=`Dein ${title(offer.give)} gegen ${title(offer.want)} von Spieler ${target+1}.`;outgoingTime.textContent=`Läuft ab in ${Math.max(0,Math.ceil(offer.expires))} Sek.`;}
  draw();
 }
 clearReview();incoming.hidden=true;outgoing.hidden=true;
 return {element:pane,refresh,activate(){dirty=true;draw();},reset(){clearReview();status.textContent='';selectionKey='';dirty=true;}};
}
