// Source-backed native diplomacy presentation. Every issued order goes through the existing
// live diplomacy/territory controls; this module never calls a WASM command.
import {calculateDiplomacyLayout,sourceButtonSprite,sourceMoneyRule,nativeDiplomacyTextStyle,modulatePlayerStrip} from './original-diplomacy-layout.js';
import {nativeHudFrame,nativeMetaEpoch} from './original-hud-layout.js';

const RESOURCE_ORDER=[0,1,3,4,5,2,6,7,8];
const RESOURCES=['Nahrung','Holz','Eisen','Stein','Gold','Zinn','Salpeter','Öl','Uran'];
const RESOURCE_KEYS=['food','wood','iron','stone','gold','tin','saltpeter','oil','uranium'];
const FRAME_PARTS=['corner_ul','corner_ll','corner_lr','corner_ur','edge_top','edge_left','edge_bottom','edge_right'];

export async function enhanceOriginalDiplomacy(panel,{wasm,enabled=true,manifestURL=new URL('./local-original/diplomacy-native/manifest.json',import.meta.url),document:doc=globalThis.document,window:win=globalThis.window,fetchFile=globalThis.fetch}={}){
 const result={enabled:false,errors:[],status(){return {enabled:false};},dispose(){}};
 if(!enabled)return result;
 let manifest;
 try{const response=await fetchFile(manifestURL);if(!response.ok)throw Error('Diplomacy manifest unavailable');manifest=await response.json();if(manifest.format!=='ee2-native-diplomacy-v1')throw Error('Unsupported native diplomacy format');calculateDiplomacyLayout(manifest,1600,1200).get('diplo_panel');}
 catch(error){result.errors.push(error.message);return result;}
 const dialog=doc.getElementById('diplomacy-dialog');
 if(!dialog||!panel?.open||!wasm){result.errors.push('Existing diplomacy controller unavailable');return result;}
 const query=(selector,base=dialog)=>base?.querySelector(selector)||null;
 const old={open:panel.open,refresh:panel.refresh,close:panel.close};
 const title=doc.getElementById('diplomacy-title'),header=query('header'),close=query('button',header),players=query('.dip-players'),detail=query('.dip-detail'),tabs=query('.dip-tabs');
 const relations=doc.getElementById('dip-relations-panel'),territory=doc.getElementById('dip-territory-panel'),relationTab=doc.getElementById('dip-relations-tab'),territoryTab=doc.getElementById('dip-territories-tab');
 const giftForm=query('.dip-gift'),giftLabel=query('label[for="dip-resource"]'),funds=giftForm?.nextElementSibling;
 const resource=doc.getElementById('dip-resource'),amount=query('input',giftForm),giftSend=query('button',giftForm);
 const byText=(text,base=relations)=>Array.from(base?.querySelectorAll('button')||[]).find(button=>button.textContent===text);
 const controls={peace:byText('Frieden anbieten'),alliance:byText('Bündnis anbieten'),war:byText('Krieg erklären'),accept:byText('Annehmen'),reject:byText('Ablehnen'),withdraw:byText('Zurückziehen'),vision:doc.getElementById('dip-sight-change')};
 if([title,header,close,players,detail,tabs,relations,territory,relationTab,territoryTab,giftForm,giftLabel,funds,resource,amount,giftSend,...Object.values(controls)].some(node=>!node)){result.errors.push('Required live diplomacy controls are missing');return result;}
 const sourceAttrs=new Map(),moved=new Map(),sourceBindings=[],frameBindings=[],addedRowLabels=[],mapMarkers=[];
 const skinHandlers=['onmouseenter','onmouseleave','onmousedown','onmouseup'];
 const el=(tag,text,className)=>{const node=doc.createElement(tag);if(text!==undefined)node.textContent=text;if(className)node.className=className;return node;};
 const remember=node=>{if(node&&!sourceAttrs.has(node))sourceAttrs.set(node,{className:node.className,cssText:node.style.cssText,hidden:node.hidden,nativeWidget:node.dataset.nativeWidget,nativeSprite:node.dataset.nativeSprite,nativeSkinBase:node.dataset.nativeSkinBase,handlers:Object.fromEntries(skinHandlers.map(key=>[key,node[key]]))});};
 const move=(node,parent)=>{if(!node)return;remember(node);if(!moved.has(node))moved.set(node,{parent:node.parentElement,next:node.nextSibling});parent.append(node);};
 const layer=el('div',undefined,'native-dip-layer'),proposal=el('div',undefined,'native-dip-proposal'),proposalBody=el('div',undefined,'native-dip-proposal-body');
 const resourceBody=el('div',undefined,'native-dip-resource-body'),territoryBody=el('div',undefined,'native-dip-territory-body');
 proposal.setAttribute('role','dialog');proposal.setAttribute('aria-label','Vertragsvorschlag');
 dialog.append(layer,proposal);proposal.append(proposalBody);move(detail,proposalBody);move(giftLabel,resourceBody);move(giftForm,resourceBody);move(funds,resourceBody);proposalBody.append(resourceBody);
 const pauseBanner=query('.dip-pause');move(pauseBanner,layer);
 layer.append(territoryBody);move(players,layer);move(title,layer);move(close,layer);
 remember(header);remember(tabs);header.hidden=true;tabs.hidden=true;
 const savedClass=dialog.className;dialog.classList.add('native-diplomacy');
 const style=el('style');style.textContent=`
 #diplomacy-dialog.native-diplomacy{position:fixed;inset:0;margin:0;padding:0;width:100vw;height:100vh;max-width:none;max-height:none;border:0;border-radius:0;background:transparent;box-shadow:none;overflow:hidden;color:#f0efdf;font:var(--native-dip-font,14px) "Palatino Linotype",Palatino,serif}
 #diplomacy-dialog.native-diplomacy::backdrop{background:transparent}
 #diplomacy-dialog.native-diplomacy>p,#diplomacy-dialog.native-diplomacy>.dip-layout,#diplomacy-dialog.native-diplomacy>.dip-log,#diplomacy-dialog.native-diplomacy>.dip-map{display:none}
 #diplomacy-dialog.native-diplomacy .native-dip-layer,#diplomacy-dialog.native-diplomacy .native-dip-proposal{position:fixed;inset:0;pointer-events:none}
 #diplomacy-dialog.native-diplomacy .native-dip-art{position:fixed;background-position:center;background-size:100% 100%;background-repeat:no-repeat;pointer-events:none}
 #diplomacy-dialog.native-diplomacy .native-dip-widget{position:fixed;box-sizing:border-box;margin:0;padding:0;border:0;border-radius:0;background:transparent center/100% 100% no-repeat;font:inherit;color:inherit;pointer-events:auto;text-align:center;overflow:hidden}
 #diplomacy-dialog.native-diplomacy button:focus-visible{outline:2px solid #fff4a1;outline-offset:1px}
 #diplomacy-dialog.native-diplomacy button:disabled{opacity:.5;cursor:default}
 #diplomacy-dialog.native-diplomacy .dip-players{display:contents}
 #diplomacy-dialog.native-diplomacy .dip-player{position:fixed;box-sizing:border-box;display:grid;grid-template-columns:48px minmax(0,1fr) 50px 90px;align-items:center;gap:5px;text-align:left;padding:0 12px;border:0;border-radius:0;background:transparent center/100% 100%;font:inherit;color:#15200f;box-shadow:none}
 #diplomacy-dialog.native-diplomacy .dip-player strong{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;font-weight:400}
 #diplomacy-dialog.native-diplomacy .dip-player small{display:none}
 #diplomacy-dialog.native-diplomacy .dip-swatch{width:10px;height:10px;border:1px solid #fff8;border-radius:0;justify-self:center}
 #diplomacy-dialog.native-diplomacy .native-dip-team,#diplomacy-dialog.native-diplomacy .native-dip-civ{font-size:.9em;text-align:center}
 #diplomacy-dialog.native-diplomacy .native-dip-proposal-body,#diplomacy-dialog.native-diplomacy .native-dip-territory-body{position:fixed;box-sizing:border-box;overflow:auto;pointer-events:auto;padding:10px}
 #diplomacy-dialog.native-diplomacy .dip-detail{border:0;border-radius:0;background:transparent;padding:0;min-width:0}
 #diplomacy-dialog.native-diplomacy .dip-detail>h3,#diplomacy-dialog.native-diplomacy .dip-detail>.dip-relation{display:none}
 #diplomacy-dialog.native-diplomacy .dip-detail p,#diplomacy-dialog.native-diplomacy .dip-detail label,#diplomacy-dialog.native-diplomacy #dip-territory-panel p,#diplomacy-dialog.native-diplomacy #dip-territory-panel label{color:#f0efdf;font-size:inherit}
 #diplomacy-dialog.native-diplomacy .dip-detail button,#diplomacy-dialog.native-diplomacy .dip-detail input,#diplomacy-dialog.native-diplomacy .dip-detail select,#diplomacy-dialog.native-diplomacy #dip-territory-panel button,#diplomacy-dialog.native-diplomacy #dip-territory-panel select{font:inherit;color:#172415;background:#d4d3b5;border:1px solid #71765c;border-radius:0;padding:5px 7px}
 #diplomacy-dialog.native-diplomacy .territory-layout{display:block;margin:0}
 #diplomacy-dialog.native-diplomacy .territory-map-wrap{max-width:210px;margin:0 auto 12px}
 #diplomacy-dialog.native-diplomacy .territory-map{width:100%;height:auto}
 #diplomacy-dialog.native-diplomacy .territory-field{margin:10px 0}
 #diplomacy-dialog.native-diplomacy .territory-review,#diplomacy-dialog.native-diplomacy .territory-proposal{border-color:#827450;background:#c7c3a4}
 #diplomacy-dialog.native-diplomacy .native-dip-status{position:fixed;color:#f0efdf;pointer-events:none;overflow:hidden}
 #diplomacy-dialog.native-diplomacy .native-dip-money{font-variant-numeric:tabular-nums;font-family:serif;color:#202713;padding-bottom:2px}
 #diplomacy-dialog.native-diplomacy [hidden]{display:none!important}
 `;doc.head.append(style);
 let ui=null,view='relations',proposalOpen=false,proposalView='terms',meta=1,layout,disposed=false,rosterKey='';
 const rowActions=new Map(),money=new Map(),staged=new Map(),stripCache=new Map();
 const url=key=>new URL(manifest.sprites[key].file,manifestURL).href;
 const image=(node,base,state='normal')=>{const key=sourceButtonSprite(manifest,base,meta,state);node.style.backgroundImage=manifest.sprites[key]?`url("${url(key)}")`:'';node.dataset.nativeSprite=key;};
 const tintStrip=(node,key,color)=>{
  const cacheKey=key+':'+color.join(',');let dataURL=stripCache.get(cacheKey);
  if(!dataURL){const record=manifest.playerStripPixels[key],source=Uint8Array.from(atob(record.rgba),char=>char.charCodeAt(0)),canvas=doc.createElement('canvas');canvas.width=record.width;canvas.height=record.height;canvas.getContext('2d').putImageData(new ImageData(modulatePlayerStrip(source,color),record.width,record.height),0,0);dataURL=canvas.toDataURL('image/png');stripCache.set(cacheKey,dataURL);}
  node.style.backgroundImage=`url("${dataURL}")`;node.style.backgroundSize='100% 100%';node.style.pointerEvents='none';node.dataset.nativeSprite=key;node.dataset.nativePlayerColor=color.join(',');
 };
 const bind=(node,widget,{sprite=true}={})=>{sourceBindings.push({node,widget,sprite});node.dataset.nativeWidget=widget;return node;};
 const paint=(name,parent=layer)=>{const row=manifest.widgets[name];if(!row?.SPRITE)return null;const node=el('div',undefined,'native-dip-art');parent.append(node);bind(node,name);return node;};
 const mainArt=['game_modal_overlay','diplo_panel_bkg_top','diplo_panel_bkg_bot','diplo_panel_glass','diplo_middle_divide',...Array.from({length:4},(_,i)=>`diplo_tab_trib_line${i+1}`),...Array.from({length:9},(_,i)=>`diplo_ledger_full_pl${i+1}`)];
 for(const name of mainArt)paint(name);
 // Keep art behind controls while retaining table-defined geometry.
 for(const node of [...layer.children].filter(node=>node.classList.contains('native-dip-art')).reverse())layer.prepend(node);
 const proposalArt=['prop_panel_bkg_top','prop_panel_bkg_bot','prop_panel_glass',...Array.from({length:4},(_,i)=>`prop_tab_text_line${i+1}`)];for(const name of proposalArt)paint(name,proposal);
 for(const node of [...proposal.children].filter(node=>node.classList.contains('native-dip-art')).reverse())proposal.prepend(node);
 function frame(widget,parent){
  const nodes=Array.from({length:8},()=>el('div',undefined,'native-dip-art'));parent.append(...nodes);frameBindings.push({widget,nodes});
 }
 frame('diplo_panel',layer);frame('diplo_panel_glass',layer);frame('prop_panel',proposal);frame('prop_panel_glass',proposal);
 const label=name=>manifest.labels[manifest.widgets[name]?.LABEL]||'';
 function skinButton(node){node.onmouseenter=()=>{node.dataset.hover='1';redrawButton(node);};node.onmouseleave=()=>{node.dataset.hover='';redrawButton(node);};node.onmousedown=()=>{node.dataset.down='1';redrawButton(node);};node.onmouseup=()=>{node.dataset.down='';redrawButton(node);};}
 function sourceButton(widget,text,parent=layer){
  const node=el('button',text??label(widget),'native-dip-widget');node.type='button';parent.append(node);bind(node,widget);
  skinButton(node);return node;
 }
 function redrawButton(node){const row=layout?.widgets[node.dataset.nativeWidget]||manifest.widgets[node.dataset.nativeWidget],selected=node.getAttribute('aria-selected')==='true'||node.getAttribute('aria-pressed')==='true'||node.dataset.down==='1',base=node.dataset.nativeSkinBase||row?.SPRITE;if(base)image(node,base,selected?(node.dataset.hover?'pressedHover':'pressed'):(node.dataset.hover?'hover':'normal'));}
 const liveActionButtons=[...relations.querySelectorAll('button'),...territory.querySelectorAll('button'),giftSend];
 for(const button of liveActionButtons){remember(button);button.dataset.nativeSkinBase='spr_game_metaBut_med';skinButton(button);button.style.backgroundSize='100% 100%';button.style.border='0';button.style.minHeight='28px';}
 const nativeTabs={};for(const [key,name]of Object.entries({relations:'diplo_tab_diplomacy',resources:'diplo_tab_tribres',units:'diplo_tab_tribunit',territories:'diplo_tab_tribterr'})){const button=sourceButton(name);button.setAttribute('role','tab');button.onclick=()=>setView(key);nativeTabs[key]=button;}
 nativeTabs.units.disabled=true;nativeTabs.units.title='Einheiten übertragen ist noch nicht verfügbar.';
 bind(title,'diplo_panel_title',{sprite:false});title.className='native-dip-widget';title.textContent=label('diplo_panel_title');
 bind(close,'diplo_close');close.className='native-dip-widget';close.textContent=label('diplo_close');
 skinButton(close);
 const send=sourceButton('diplo_tribute_accept'),clear=sourceButton('diplo_tribute_clear');
 send.onclick=()=>{if(view==='resources')sendStaged();else{byText('Territorium verschenken',territory)?.click();refreshNative(ui);}};
 clear.onclick=()=>{if(view==='resources'){staged.clear();refreshNative(ui);}else openProposal(Math.max(0,opponents().findIndex(player=>player.team===selected()?.team)));};
 const status=el('p',undefined,'native-dip-status');status.setAttribute('role','status');layer.append(status);
 const propTitle=el('h3',undefined,'native-dip-widget');proposal.append(propTitle);bind(propTitle,'prop_panel_player',{sprite:false});
 const proposalTabs={};for(const [key,name]of Object.entries({terms:'prop_panel_tab_terms',resources:'prop_panel_tab_resource',units:'prop_panel_tab_unit',territories:'prop_panel_tab_territory'})){const button=sourceButton(name,undefined,proposal);button.onclick=()=>setProposalView(key);button.setAttribute('role','tab');proposalTabs[key]=button;}
 proposalTabs.units.disabled=true;proposalTabs.units.title=nativeTabs.units.title;
 const propCancel=sourceButton('prop_edit_cancel',label('prop_edit_cancel'),proposal);propCancel.onclick=()=>{
  proposalOpen=false;panel.clearAllianceTerritories?.();panel.setAllianceTerritoryEditing?.(false);refreshNative(ui);nativeTabs[view]?.focus();
 };
 function proposalCommand(){
  const p=selected();
  if(proposalView==='territories'&&(!panel.allianceTerritoriesSupported||p?.territoryIncoming&&!p?.incomingAllianceTerritories)){
   return byText(p?.territoryIncoming?'Tausch annehmen':'Territorientausch anbieten',territory);
  }
  if(proposalView==='territories')return p?.incoming?controls.accept:controls.alliance;
  return p?.incoming?controls.accept:p?.relation===0?controls.peace:p?.relation===1?controls.alliance:controls.vision;
 }
 const propSend=sourceButton('prop_edit_send',label('prop_edit_send'),proposal);propSend.onclick=()=>{
  if(proposalView==='resources'){if(!giftSend.disabled)giftForm.onsubmit({preventDefault(){}});}
  else{const button=proposalCommand();if(button&&!button.disabled)button.click();}
  refreshNative(read());
 };
 function read(){const pointer=wasm.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,pointer,wasm.ui_len())));}
 function opponents(){return ui?.diplomacy?.players?.filter(player=>player.team!==ui.diplomacy.local)||[];}
 function selected(){const buttons=Array.from(players.children),index=buttons.findIndex(button=>button.getAttribute('aria-pressed')==='true');return opponents()[index]||opponents()[0];}
 function select(index){players.children[index]?.click();ui=read();}
 function activeSlots(){return RESOURCE_ORDER.filter(slot=>ui?.resourceActive?.[slot]!==false).slice(0,6);}
 function setView(next){
  if(next==='units')return;
  view=next;proposalOpen=false;
  panel.clearAllianceTerritories?.();panel.setAllianceTerritoryEditing?.(false);
  (next==='territories'?territoryTab:relationTab).click();
  if(next==='territories')move(territory,territoryBody);
  refreshNative(ui||read());
 }
 function setProposalView(next){
  if(next==='units')return;
  proposalView=next;
  const p=selected();
  panel.setAllianceTerritoryEditing?.(next==='territories'&&panel.allianceTerritoriesSupported&&(!p?.territoryIncoming||!!p?.incomingAllianceTerritories));
  (next==='territories'?territoryTab:relationTab).click();
  if(next==='territories')move(territory,proposalBody);
  refreshNative(ui||read());
 }
 function openProposal(index){
  panel.clearAllianceTerritories?.();select(index);proposalOpen=true;
  const p=opponents()[index];setProposalView(p?.territoryIncoming||p?.incomingAllianceTerritories?'territories':'terms');
 }
 function put(node,r){for(const [key,value]of Object.entries({left:r.x,top:r.y,width:r.width,height:r.height}))node.style[key]=value+'px';}
 function sourceLabel(widget,parent=layer){const node=el('div',label(widget),'native-dip-widget');parent.append(node);bind(node,widget,{sprite:false});return node;}
 for(const name of ['diplo_playerName_label','diplo_playerTeam_label','diplo_playerCiv_label'])sourceLabel(name);
 const resourceHeaders=Array.from({length:6},(_,i)=>paint(`diplo_tribres_icon_${i}`));
 function ensureRows(){
  const key=opponents().map(player=>player.team).join(',');
  if(key!==rosterKey){
   for(const buttons of rowActions.values())Object.values(buttons).forEach(node=>node.remove());for(const node of money.values())node.remove();rowActions.clear();money.clear();
   for(let i=sourceBindings.length-1;i>=0;i--)if(/^(actions|resources)\d+__/.test(sourceBindings[i].widget))sourceBindings.splice(i,1);
   rosterKey=key;
  }
  opponents().forEach((player,index)=>{
   const number=index+1,button=players.children[index];if(!button)return;
   button.dataset.nativeWidget=`diplo_tribbutt_pl${number}`;
   remember(button);
   if(!query('.native-dip-team',button)){const cells=[el('span','—','native-dip-team'),el('span','—','native-dip-civ'),el('span',undefined,'native-dip-row-status')],glow=el('div',undefined,'native-dip-player-glow'),color=el('div',undefined,'native-dip-player-color');glow.append(color);button.prepend(glow);button.append(...cells);addedRowLabels.push(glow,...cells);}
   if(!rowActions.has(index)){
    const buttons={};for(const action of ['proposal','neutral','war']){const node=el('button','','native-dip-widget');node.type='button';node.dataset.command=action;node.setAttribute('aria-label',action==='proposal'?'Vertrag prüfen':action==='neutral'?'Frieden anbieten':'Krieg erklären');layer.append(node);bind(node,`actions${number}__diplo_dbutton_${action}`,{sprite:false});skinButton(node);node.onclick=()=>{select(index);if(action==='proposal')openProposal(index);else{const command=action==='neutral'?controls.peace:controls.war;if(command&&!command.disabled)command.click();refreshNative(read());}};buttons[action]=node;}rowActions.set(index,buttons);
   }
   for(let column=0;column<6;column++)if(!money.has(`${index}:${column}`)){
    const node=el('button','0','native-dip-widget native-dip-money');node.type='button';node.dataset.column=column;layer.append(node);bind(node,`resources${number}__diplo_tribres_${column}`);skinButton(node);
    const change=(direction,event)=>{const slot=activeSlots()[column];if(slot===undefined||!player.alive)return;const rule=sourceMoneyRule(manifest.widgets[`diplo_tribres_${column}`]),key=`${player.team}:${slot}`,others=[...staged].reduce((n,[k,v])=>n+(k!==key&&k.endsWith(':'+slot)?v:0),0),amount=Math.max(rule.min,Math.min(rule.max,Math.floor((ui.res?.[slot]||0)-others),(staged.get(key)||0)+direction*(event?.shiftKey?rule.shiftIncrement:rule.increment)));staged.set(key,amount);refreshNative(ui);};
    node.onclick=event=>change(1,event);node.oncontextmenu=event=>{event.preventDefault();change(-1,event);};node.onkeydown=event=>{if(['ArrowUp','ArrowDown'].includes(event.key)){event.preventDefault();change(event.key==='ArrowUp'?1:-1,event);}};money.set(`${index}:${column}`,node);
   }
  });
 }
 function sendStaged(){
  for(const [key,value]of [...staged].sort(([a],[b])=>a.localeCompare(b))){if(!value)continue;const [team,slot]=key.split(':').map(Number),index=opponents().findIndex(player=>player.team===team);if(index<0)continue;select(index);resource.value=String(slot);amount.value=String(value);resource.onchange?.();amount.oninput?.();if(!giftSend.disabled){giftForm.onsubmit({preventDefault(){}});staged.delete(key);}}
  refreshNative(read());
 }
 function redraw(){
  const width=win.innerWidth,height=win.innerHeight;layout=calculateDiplomacyLayout(manifest,width,height,{playerCount:opponents().length,meta});dialog.style.setProperty('--native-dip-font',Math.max(10,Math.trunc(14*width/1600))+'px');
  for(const button of liveActionButtons)redrawButton(button);
  const textStyle=(node,row,r)=>{const style=nativeDiplomacyTextStyle(manifest,row,width,meta,node.disabled);for(const [key,value]of Object.entries(style))if(key!=='verticalMiddle')node.style[key]=value;if(style.verticalMiddle)node.style.lineHeight=r.height+'px';};
  for(const binding of sourceBindings){const r=layout.get(binding.widget);put(binding.node,r);const row=layout.widgets[binding.widget];textStyle(binding.node,row,r);if(binding.sprite&&row.SPRITE)image(binding.node,row.SPRITE);}
  for(const binding of frameBindings){
   const r=layout.get(binding.widget),f=nativeHudFrame(manifest,layout.widgets[binding.widget],width,meta),w=f.outer+f.inner,d=f.definition;
   if(!d){binding.nodes.forEach(node=>{node.hidden=true;});continue;}
   if(d['FRAME TYPE']==='FourCornersFourTiledEdges'){
    const boxes=[[r.x-w,r.y-w,w,w],[r.x-w,r.y+r.height,w,w],[r.x+r.width,r.y+r.height,w,w],[r.x+r.width,r.y-w,w,w],[r.x,r.y-w,r.width,w],[r.x-w,r.y,w,r.height],[r.x,r.y+r.height,r.width,w],[r.x+r.width,r.y,w,r.height]];
    binding.nodes.forEach((node,i)=>{node.hidden=false;put(node,{x:boxes[i][0],y:boxes[i][1],width:boxes[i][2],height:boxes[i][3]});const key=d.SPRITE.replace('corner_ul',FRAME_PARTS[i]);image(node,key);node.dataset.nativeFrame=key;if(i>=4){const sprite=manifest.sprites[key];node.style.backgroundRepeat=i===4||i===6?'repeat-x':'repeat-y';node.style.backgroundSize=i===4||i===6?`${sprite.logicalWidth*width/1600}px ${w}px`:`${w}px ${sprite.logicalHeight*height/1200}px`;}});
   }else{
    const boxes=[[r.x-f.outer,r.y-f.outer,r.width+2*f.outer,w],[r.x-f.outer,r.y-f.outer,w,r.height+2*f.outer],[r.x-f.outer,r.y+r.height+f.outer-w,r.width+2*f.outer,w],[r.x+r.width+f.outer-w,r.y-f.outer,w,r.height+2*f.outer]];
    binding.nodes.forEach((node,i)=>{node.hidden=i>=4;if(i>=4)return;put(node,{x:boxes[i][0],y:boxes[i][1],width:boxes[i][2],height:boxes[i][3]});const rgba=(d[['TOP OUTER','LEFT OUTER','BOTTOM OUTER','RIGHT OUTER'][i]]?.match(/\d+/g)||[]).map(Number);node.style.backgroundImage='none';node.style.backgroundColor=`rgba(${rgba[0]||0},${rgba[1]||0},${rgba[2]||0},${(rgba[3]??255)/255})`;});
   }
  }
  opponents().forEach((player,index)=>{const button=players.children[index];if(button){put(button,layout.get(`diplo_tribbutt_pl${index+1}`));button.style.background='transparent';button.style.backgroundImage='none';button.style.boxShadow='none';}});
  opponents().forEach((player,index)=>{
   const button=players.children[index];if(!button)return;const prefix=`player${index+1}__`;
   const swatch=query('.dip-swatch',button),css=swatch?.style.background||'#ffffff',channels=css.startsWith('#')?css.slice(1).match(/../g).map(value=>parseInt(value,16)):css.match(/[\d.]+/g).slice(0,3).map(Number),selected=button.getAttribute('aria-pressed')==='true';
   for(const [selector,name,base]of [['.native-dip-player-glow','diplo_pl_color_glow','spr_dip_pcolor_strip_glow'],['.native-dip-player-color','diplo_pl_color','spr_dip_pcolor_strip_2state']]){const node=query(selector,button);put(node,layout.get(prefix+name));node.style.position='fixed';tintStrip(node,base+(selected?'_on':''),channels);}
   for(const [selector,name]of [['strong','diplo_pl_name'],['.native-dip-team','diplo_pl_team'],['.native-dip-civ','diplo_pl_civ'],['.native-dip-row-status','diplo_pl_currstat_from']]){const node=query(selector,button);if(node){remember(node);const r=layout.get(prefix+name);put(node,r);textStyle(node,layout.widgets[prefix+name],r);node.style.position='fixed';if(name==='diplo_pl_currstat_from'){node.style.pointerEvents='none';image(node,player.relation===0?'icon_diplo_atWar':player.relation===2?'icon_diplo_inAlliance':'icon_diplo_neutral');}}}
  });
  put(proposalBody,layout.get('prop_below_tabs'));put(territoryBody,layout.get('diplo_tribterr_pane'));
  const mapWrap=query('.territory-map-wrap',territory),mapCanvas=query('canvas',territory);
  if(mapWrap&&mapCanvas){
   remember(mapWrap);remember(mapCanvas);const inProposal=proposalOpen&&proposalView==='territories',map=layout.get(inProposal?'prop_tribterr_mm':'diplo_minimap'),parent=layout.get(inProposal?'prop_below_tabs':'diplo_tribterr_pane');
   mapWrap.style.position='relative';mapWrap.style.width=map.width+'px';mapWrap.style.maxWidth=map.width+'px';mapWrap.style.marginLeft=Math.max(0,map.x-parent.x-10)+'px';mapWrap.style.marginTop=Math.max(0,map.y-parent.y-10)+'px';mapCanvas.style.width=map.width+'px';mapCanvas.style.height=map.height+'px';
   for(const [index,key]of ['tribute','demand'].entries()){
    let marker=mapMarkers[index];if(!marker){marker=el('div',undefined,'native-dip-map-marker');marker.dataset.nativeMapMarker=key;marker.style.position='absolute';marker.style.backgroundSize='100% 100%';marker.style.pointerEvents='none';mapWrap.append(marker);mapMarkers[index]=marker;}
    const value=doc.getElementById(index?'territory-want':'territory-give').value,id=value===''?null:Number(value),area=ui?.diplomacy?.territories.find(area=>area.id===id),size=layout.get('diplo_territory_'+key);marker.hidden=!area?.known;if(area){marker.style.left=(area.x/256*map.width-size.width/2)+'px';marker.style.top=(area.y/256*map.height-size.height/2)+'px';marker.style.width=size.width+'px';marker.style.height=size.height+'px';image(marker,'spr_dip_territory_'+key);}
   }
  }
  const r=layout.get('diplo_panel');put(status,{x:r.x+8,y:r.y+r.height-54,width:Math.max(1,r.width-220),height:40});
  if(pauseBanner){put(pauseBanner,{x:12,y:12,width:Math.max(1,width-24),height:40});pauseBanner.style.position='fixed';pauseBanner.style.pointerEvents='auto';}
 }
 function refreshNative(next){
  if(disposed||!next)return;ui=next;meta=nativeMetaEpoch(ui.epoch??0);ensureRows();redraw();
  for(const [key,tab]of Object.entries(nativeTabs)){tab.setAttribute('aria-selected',String(view===key));redrawButton(tab);}
  const recipients=opponents(),slots=activeSlots();
  for(const key of [...staged.keys()]){const [team,slot]=key.split(':').map(Number);if(!slots.includes(slot)||!recipients.some(player=>player.team===team&&player.alive))staged.delete(key);}
  recipients.forEach((player,index)=>{const buttons=rowActions.get(index),locked=player.fixedTeam!==undefined;for(const node of Object.values(buttons))node.hidden=view!=='relations'||proposalOpen;buttons.proposal.disabled=!player.alive;buttons.neutral.disabled=locked||!player.alive||player.relation!==0&&player.theirRelation!==0||!!player.outgoing||player.cooldown>0;buttons.war.disabled=locked||!!player.treatyUnbreakable||!player.alive||player.relation===0;image(buttons.proposal,player.incoming?'icon_diplo_alliance_review':player.outgoing?'icon_diplo_alliance_withdraw':'icon_diplo_alliance_propose');image(buttons.neutral,'icon_diplo_neutral_radio',player.relation===1?'pressed':'normal');image(buttons.war,'icon_diplo_decWar_radio',player.relation===0?'pressed':'normal');
   const teamCell=query('.native-dip-team',players.children[index]);
   if(teamCell)teamCell.textContent=locked?String(player.fixedTeam):'—';
   buttons.proposal.dataset.nativeSkinBase=player.incoming?'icon_diplo_alliance_review':player.outgoing?'icon_diplo_alliance_withdraw':'icon_diplo_alliance_propose';buttons.neutral.dataset.nativeSkinBase='icon_diplo_neutral_radio';buttons.war.dataset.nativeSkinBase='icon_diplo_decWar_radio';buttons.neutral.setAttribute('aria-pressed',String(player.relation===1));buttons.war.setAttribute('aria-pressed',String(player.relation===0));
   for(let column=0;column<6;column++){const node=money.get(`${index}:${column}`),slot=slots[column];node.hidden=view!=='resources'||proposalOpen||slot===undefined;node.disabled=!player.alive;node.textContent=String(staged.get(`${player.team}:${slot}`)||0);node.title=`${RESOURCES[slot]||''} an ${player.name||'Spieler '+(player.team+1)} · Klick100, Umschalt500, Rechtsklick abziehen`;}
  });
  resourceHeaders.forEach((node,index)=>{const slot=slots[index];node.hidden=view!=='resources'||proposalOpen||slot===undefined;if(slot!==undefined)image(node,'icon_resource_'+RESOURCE_KEYS[slot]);});
  for(const binding of sourceBindings)if(binding.widget.startsWith('diplo_tribres_icon_'))binding.node.hidden=view!=='resources'||proposalOpen;
  for(const [key,tab]of Object.entries(proposalTabs)){tab.setAttribute('aria-selected',String(proposalView===key));redrawButton(tab);}
  layer.inert=proposalOpen;territoryBody.hidden=view!=='territories'||proposalOpen;proposal.hidden=!proposalOpen;resourceBody.hidden=!proposalOpen||proposalView!=='resources';relations.hidden=!proposalOpen||proposalView!=='terms';
  if(proposalOpen&&proposalView==='territories')territory.hidden=false;
  else if(view!=='territories')territory.hidden=true;
  if(!proposalOpen&&view==='territories')territory.hidden=false;
  detail.hidden=proposalView!=='terms';propTitle.textContent=selected()?.name||'Vertragsvorschlag';
  propSend.disabled=proposalView!=='resources'&&proposalCommand()?.disabled===true;
  send.hidden=clear.hidden=!['resources','territories'].includes(view)||proposalOpen;
  if(view==='resources'){send.textContent=label('diplo_tribute_accept');clear.textContent=label('diplo_tribute_clear');send.disabled=![...staged.values()].some(value=>value>0);clear.disabled=send.disabled;}
  else{send.textContent='Verschenken';clear.textContent='Vertrag';send.disabled=!!query('.territory-review',territory)&&!query('.territory-review',territory).hidden||!!byText('Territorium verschenken',territory)?.disabled;clear.disabled=!selected()?.alive;}
  status.textContent=staged.size&&view==='resources'?`Vorgemerkt: ${[...staged.values()].reduce((a,b)=>a+b,0)} · Mit Schicken übertragen.`:'';
 }
 panel.open=function(){if(dialog.open)return;old.open();ui=read();view=territory.hidden?'relations':'territories';proposalOpen=false;staged.clear();refreshNative(ui);};
 panel.refresh=function(next){old.refresh(next);if(dialog.open)refreshNative(next);};
 const resize=()=>{if(dialog.open)refreshNative(ui);};win.addEventListener('resize',resize);
 const cancelProposal=event=>{if(proposalOpen){event.preventDefault();event.stopImmediatePropagation?.();propCancel.click();}};dialog.addEventListener('cancel',cancelProposal,true);
 result.enabled=true;result.status=()=>({enabled:!disposed,view,proposalOpen,proposalView,meta,opponents:opponents().length,staged:[...staged],widgets:Object.keys(manifest.widgets).length,sprites:Object.keys(manifest.sprites).length});
 result.setView=setView;result.openProposal=openProposal;result.setProposalView=setProposalView;
 result.fontBindings=()=>{
  if(disposed)return null;
  const labels=sourceBindings.map(binding=>({element:binding.node,widget:binding.widget}));
  for(const button of players.children)for(const [selector,widget]of [['strong','diplo_pl_name'],['.native-dip-team','diplo_pl_team'],['.native-dip-civ','diplo_pl_civ']]){const element=query(selector,button);if(element)labels.push({element,widget});}
  return {source:manifest,meta,labels};
 };
 result.dispose=()=>{if(disposed)return;disposed=true;result.enabled=false;panel.open=old.open;panel.refresh=old.refresh;panel.close=old.close;win.removeEventListener?.('resize',resize);dialog.removeEventListener?.('cancel',cancelProposal,true);for(const [node,original]of [...moved].reverse()){if(original.next?.parentElement===original.parent)original.parent.insertBefore(node,original.next);else original.parent?.append(node);}for(const [node,value]of sourceAttrs){node.className=value.className;node.style.cssText=value.cssText||'';node.hidden=value.hidden;for(const key of skinHandlers)node[key]=value.handlers[key];for(const key of ['nativeWidget','nativeSprite','nativeSkinBase'])if(value[key]===undefined)delete node.dataset[key];else node.dataset[key]=value[key];}addedRowLabels.forEach(node=>node.remove());mapMarkers.forEach(node=>node.remove());dialog.className=savedClass;layer.remove();proposal.remove();style.remove();if(ui&&dialog.open)old.refresh(ui);};
 return result;
}
