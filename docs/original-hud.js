import {calculateHudLayout,nativeMetaEpoch,nativeHudFrame} from './original-hud-layout.js';
import {seasonAtTick} from './original-seasons.js';

const NONE=Object.freeze({enabled:false,update(){},resize(){},status(){return {enabled:false};}});
const BUTTONS={
 'research-button':'game_bottombuttons_techtree',
 'diplomacy-button':'game_bottombuttons_diplomacy',
 idle:'game_bottombuttons_idlecit',pause:'game_button_pause',cheats:'game_button_chat'
};
const PANEL_WIDGETS={mini:'game_minimap_container',acts:'game_unitbuttons',info:'game_unitinfo'};

// Existing controls and canvas nodes move intact so their command, save,
// multiplayer, accessibility and input listeners remain attached.
export async function loadNativeHud(assets,enabled,options={}){
 if(!enabled||!assets?.enabled)return NONE;
 const doc=options.document??globalThis.document,win=options.window??globalThis.window;
 const fetchFile=options.fetch??globalThis.fetch;
 const base=options.base??new URL('./local-original/hud-native/',import.meta.url);
 let manifest;
 try{
  const response=await fetchFile(new URL('manifest.json',base));if(!response.ok)return NONE;
  manifest=await response.json();if(manifest.format!=='ee2-native-hud-v1'||!manifest.art||!manifest.sprites)return NONE;
  calculateHudLayout(manifest,1600,1200);
 }catch{return NONE;}
 const get=id=>doc.getElementById(id),bottom=get('bottom'),top=get('top');
 if(!bottom||!top||!get('mini')||!get('acts')||!get('info'))return NONE;
 const root=doc.createElement('div');root.id='native-hud-art';root.setAttribute('aria-hidden','true');bottom.prepend(root);
 const style=doc.createElement('style');style.id='native-hud-style';
 style.textContent=`
 #bottom.ee2-native-bottom{position:fixed;inset:0;height:auto;display:block;gap:0;padding:0;border:0;background:none;box-shadow:none;pointer-events:none;z-index:2;font-family:"Palatino Linotype",Palatino,"Times New Roman",serif}
 #native-hud-art{position:fixed;inset:0;pointer-events:none}#native-hud-art>div{position:fixed;background-size:100% 100%;background-repeat:no-repeat;pointer-events:none}
 #native-hud-art>.native-hud-block{pointer-events:auto}
 #bottom.ee2-native-bottom #mini,#bottom.ee2-native-bottom #info,#bottom.ee2-native-bottom #acts{position:fixed;margin:0;box-shadow:none;pointer-events:auto;flex:none}
 #bottom.ee2-native-bottom #mini{border:0;background:#050705}
 #bottom.ee2-native-bottom #info{padding:calc(var(--native-sy)*7px) calc(var(--native-sx)*9px);border:0;background:none;color:#171b13;font-size:var(--native-font);line-height:1.28;overflow:auto}
 #bottom.ee2-native-bottom #info h3{font-size:calc(var(--native-font)*1.15);text-align:center;margin:0 0 6px}
 #bottom.ee2-native-bottom #info .dim,#bottom.ee2-native-bottom #info .activity{color:#293123;font-size:inherit}
 #bottom.ee2-native-bottom #acts{padding:0;border:0;background:none;grid-template-columns:repeat(4,minmax(0,1fr));grid-auto-rows:calc(var(--native-sy)*61px);gap:0;overflow-y:auto;align-content:start}
 #bottom.ee2-native-bottom .act{padding:0;border-radius:0;min-width:0;overflow:hidden;background:transparent;border:1px solid #69745f80;display:flex;align-items:center;justify-content:center}
 #bottom.ee2-native-bottom .act canvas{width:calc(var(--native-sy)*57px);height:calc(var(--native-sy)*57px);max-width:100%;object-fit:contain;margin:0}
 #bottom.ee2-native-bottom .act small{display:none}#bottom.ee2-native-bottom .act kbd{font-size:10px;color:#10190d;text-shadow:0 1px #fffc}
 #bottom.ee2-native-bottom .act:hover{background:#f7efb73d;border-color:#f4efb1}
 #bottom.ee2-native-bottom .native-hud-button{position:fixed;pointer-events:auto;margin:0;padding:0;border:0;border-radius:0;background:transparent center/100% 100% no-repeat;font-size:0;color:transparent;cursor:pointer;box-shadow:none;text-shadow:none}
 #bottom.ee2-native-bottom .native-hud-button:focus-visible{outline:2px solid #fff3a1;outline-offset:1px}
 #bottom.ee2-native-bottom .native-hud-button:hover{filter:brightness(1.16)}
 #top.ee2-economy-top.ee2-native-top{position:fixed;left:0;top:0;right:auto;width:0;height:0;padding:0;border:0;display:block;background:none;box-shadow:none;overflow:visible;z-index:5}
 #native-hud-options{color:#e1e5d5;border:0;pointer-events:none}
 #native-hud-options summary{position:fixed;cursor:pointer;padding:0;font-size:0;list-style:none;background:transparent center/100% 100% no-repeat;pointer-events:auto}
 #native-hud-options summary::marker{content:''}#native-hud-options summary::-webkit-details-marker{display:none}
 #native-hud-options-content{position:fixed;display:flex;flex-wrap:wrap;align-items:center;gap:6px;padding:9px;width:min(650px,95vw);max-height:75vh;overflow:auto;color:#e1e5d5;background:#172018f5;border:1px solid #9ba38b;border-radius:3px;pointer-events:auto}
 #native-hud-options:not([open]) #native-hud-options-content{display:none}
 #native-hud-options-content button{height:auto;min-height:25px;font-size:12px}
 #native-hud-options-content #perf{max-width:300px;font-size:10px}
 #native-hud-resources{position:fixed;pointer-events:auto;display:flex;flex-direction:column;gap:0;padding:0;color:#171a12;font-size:var(--native-font);font-variant-numeric:tabular-nums}
 #native-hud-resources>.native-resource-row{height:calc(var(--native-sy)*33px);min-height:0;position:relative;display:flex;align-items:center;justify-content:flex-end;gap:0;padding:0 calc(var(--native-sx)*6px) 0 calc(var(--native-sx)*8px);border:0;background:center/100% 100% no-repeat;white-space:nowrap;flex:none}
 #native-hud-resources>.native-resource-row[hidden]{display:none}
 #native-hud-resources .ee2-economy-name{display:none}#native-hud-resources b{font-size:inherit;min-width:0;color:inherit;line-height:1.1}
 #native-hud-resources img{width:calc(var(--native-sx)*32px);height:calc(var(--native-sy)*32px);object-fit:contain;margin-right:auto;flex:none}
 #native-hud-resources #science{padding:0!important;background:none!important;font-size:inherit;color:inherit;min-width:0;white-space:nowrap}
 .native-hud-heading{position:fixed;pointer-events:none;display:flex;align-items:center;justify-content:center;overflow:hidden;color:#fff;text-shadow:1px 1px #000;font-size:var(--native-font);font-variant:small-caps}
 #native-hud-epoch{cursor:pointer;pointer-events:auto;font-family:inherit;font-size:calc(var(--native-font)*1.8);font-weight:normal;border:0;background:none;color:#606060;text-shadow:none;font-variant:normal}
 .native-hud #tip{left:30%;bottom:calc(var(--native-sy)*345px);z-index:6}
 .native-hud #camera-tools{top:45px;right:12px}
 `;
 doc.head.append(style);bottom.classList.add('ee2-native-bottom');top.classList.add('ee2-native-top');doc.documentElement.classList.add('native-hud');
 const art=new Map(),blocks=new Map(),buttonWidgets=new Set(Object.values(BUTTONS));
 for(const [name,sprite]of Object.entries(manifest.art)){
  if(buttonWidgets.has(name)||name==='game_button_options'||name==='game_button_encyclopedia')continue;
  const element=doc.createElement('div');element.dataset.widget=name;root.append(element);art.set(name,{element,sprite});
 }
 for(let index=1;index<=3;index++){
  const name='game_bottom_bkg_blockCursor'+index;if(!manifest.widgets[name])continue;
  const element=doc.createElement('div');element.className='native-hud-block';root.append(element);blocks.set(name,element);
 }
 const details=doc.createElement('details');details.id='native-hud-options';
 const summary=doc.createElement('summary');summary.textContent='Spielmenü';summary.title='Spielmenü · Speichern, Ton, Vollbild und Hauptmenü';details.append(summary);
 const content=doc.createElement('div');content.id='native-hud-options-content';details.append(content);
 const controls=top.querySelector('.ee2-economy-controls');
 if(controls)while(controls.firstChild)content.append(controls.firstChild);
 const resourceList=doc.createElement('div');resourceList.id='native-hud-resources';bottom.append(resourceList);
 const rows=[];
 function special(id,key,label){
  const value=get(id);if(!value)return;
  const row=doc.createElement('span');row.className='native-resource-row';row.title=label;
  const sprite=manifest.sprites[key];
  if(sprite){const image=doc.createElement('img');image.src=new URL(sprite.file,base).href;image.alt='';row.append(image);}
  row.append(value);resourceList.append(row);rows.push(row);
 }
 special('pop','icon_resource_popcap','Bevölkerung / Bevölkerungslimit');
 special('science','icon_resource_tech','Technologiepunkte');
 for(const slot of [0,1,3,4,5,2,6,7,8]){
  const value=get('r'+slot),row=value?.closest('.ee2-economy-resource');if(!row)continue;
  row.classList.add('native-resource-row');resourceList.append(row);rows.push(row);
 }
 // Retain all still-connected useful controls in the drawer; discard only the
 // now-empty economy wrappers. Moving a live button never clones its handler.
 for(const id of ['terr','epoch']){
  const node=get(id);if(node){const row=doc.createElement('span');row.className='r';const label=doc.createElement('span');label.textContent=id==='terr'?'Gebiete:':'Epoche:';row.append(label,node);content.append(row);}
 }
 top.replaceChildren(details);
 const buttons=new Map();
 for(const [id,name]of Object.entries(BUTTONS)){
  const button=get(id);if(!button)continue;
  button.setAttribute('aria-label',button.textContent.trim()||id);button.classList.add('native-hud-button');bottom.append(button);buttons.set(name,button);
 }
 const name=doc.createElement('div');name.className='native-hud-heading';name.textContent='Spieler 1';bottom.append(name);
 const mode=doc.createElement('div');mode.className='native-hud-heading';mode.textContent='Scharmützel';bottom.append(mode);
 const epoch=doc.createElement('button');epoch.id='native-hud-epoch';epoch.className='native-hud-heading';epoch.title='Epoche und Forschung';epoch.onclick=()=>get('research-button')?.click();bottom.append(epoch);
 const clock=doc.createElement('div');clock.id='native-hud-clock';clock.className='native-hud-heading';clock.title='Verstrichene Spielzeit';bottom.append(clock);
 const calendar=doc.createElement('div');calendar.id='native-hud-month';calendar.className='native-hud-heading';calendar.title='Aktueller Monat';bottom.append(calendar);
 let layout,meta=0,lastUi=null;
 const place=(element,rect)=>{if(!element||!rect)return;Object.assign(element.style,{left:rect.x+'px',top:rect.y+'px',width:rect.width+'px',height:rect.height+'px'});};
 const frameColor=value=>{const parts=(value?.match(/\d+/g)||['0','0','0']).map(Number);return `rgba(${parts.slice(0,3).join(',')},${(parts[3]??255)/255})`;};
 function placeButton(element,widget){
  const r=layout.get(widget);if(!element||!r)return;
  const frame=nativeHudFrame(manifest,manifest.widgets[widget],win.innerWidth,meta||1);
  place(element,{...r,x:r.x-frame.outer,y:r.y-frame.outer});
  Object.assign(element.style,{boxSizing:'content-box',backgroundOrigin:'content-box',backgroundClip:'content-box',borderStyle:'solid',borderWidth:frame.outer+'px'});
  if(frame.definition){const d=frame.definition;element.style.borderColor=['TOP OUTER','RIGHT OUTER','BOTTOM OUTER','LEFT OUTER'].map(key=>frameColor(d[key])).join(' ');}
 }
 function imageFor(sprite){const suffix=meta===1?'':`_e${meta}`,value=manifest.sprites[sprite+suffix]||manifest.sprites[sprite];return value?`url("${new URL(value.file,base).href}")`:'';}
 function theme(next){
  if(next===meta)return;meta=next;
  for(const {element,sprite}of art.values())element.style.backgroundImage=imageFor(sprite);
  for(const [widget,button]of buttons)button.style.backgroundImage=imageFor(manifest.art[widget]);
  summary.style.backgroundImage=imageFor(manifest.art.game_button_options);
  if(layout){for(const [widget,button]of buttons)placeButton(button,widget);placeButton(summary,'game_button_options');}
  rows.forEach((row,index)=>row.style.backgroundImage=imageFor(index===0?'spr_game_dash_res_pop':`spr_game_dash_res_${Math.min(index,8)}`));
 }
 function resize(){
  const width=win.innerWidth,height=win.innerHeight;layout=calculateHudLayout(manifest,width,height);
  const css=doc.documentElement.style;css.setProperty('--native-sx',String(width/1600));css.setProperty('--native-sy',String(height/1200));css.setProperty('--native-font',Math.max(10,height/1200*18)+'px');
  for(const [widget,{element}]of art)place(element,layout.get(widget));
  for(const [widget,element]of blocks)place(element,layout.get(widget));
  for(const [widget,button]of buttons)placeButton(button,widget);
  placeButton(summary,'game_button_options');
  const optionsRect=layout.get('game_button_options');Object.assign(content.style,{left:Math.min(optionsRect.x,Math.max(8,width-660))+'px',top:(optionsRect.y+optionsRect.height+8)+'px'});
  for(const [id,widget]of Object.entries(PANEL_WIDGETS))place(get(id),layout.get(widget));
  place(resourceList,layout.get('game_resources'));place(name,layout.get('game_playerinfo_name'));place(mode,layout.get('game_playerinfo_civ'));place(epoch,layout.get('game_playerinfo_epoch'));
  place(clock,layout.get('game_top_clock'));place(calendar,layout.get('game_calendar'));
 }
 const api={enabled:true,resize,fontBindings(){
  const labels=[{element:name,widget:'game_playerinfo_name'},{element:mode,widget:'game_playerinfo_civ'},{element:epoch,widget:'game_playerinfo_epoch'},{element:clock,widget:'game_top_clock'},
   {element:calendar,widget:'game_top_clock',role:'calendar-label-adapter'}];
  const resources=['pop','science',...Array.from({length:9},(_,slot)=>'r'+slot)].map(id=>{const element=get(id),row=element?.closest('.native-resource-row');return row?.parentElement===resourceList?{element,row}:null;}).filter(Boolean);
  return {source:manifest,meta:meta||1,labels,resources};
 },update(ui,seasonSettings){
  lastUi=ui;theme(nativeMetaEpoch(ui.epoch||0));epoch.textContent=String((ui.epoch||0)+1);epoch.title=ui.ename||'Epoche und Forschung';
  name.textContent='Spieler '+((ui.local??0)+1);mode.textContent=ui.net?'Mehrspieler':'Scharmützel';
  if(typeof ui.paused==='boolean')get('pause')?.setAttribute('aria-pressed',String(ui.paused));
  const idle=get('idle');if(idle&&Number.isFinite(ui.idle)){idle.style.visibility='visible';idle.disabled=ui.idle<=0;idle.title='Freie Bürger: '+ui.idle+' · Nächsten auswählen (F)';}
  if(Number.isSafeInteger(ui.tick)&&ui.tick>=0){
   const seconds=Math.floor(ui.tick/20),minutes=Math.floor(seconds/60),hours=Math.floor(minutes/60);
   clock.textContent=`${hours}:${String(minutes%60).padStart(2,'0')}:${String(seconds%60).padStart(2,'0')}`;
   calendar.textContent=seasonAtTick(ui.tick,seasonSettings).monthName;
  }
  for(const [widget,button]of buttons)button.setAttribute('aria-label',button.textContent.trim()||widget);
 },status(){return {enabled:true,widgets:manifest.widgets?Object.keys(manifest.widgets).length:0,sprites:Object.keys(manifest.sprites).length,metaEpoch:meta,epoch:lastUi?.epoch??null};}};
 content.addEventListener('click',event=>{if(event.target.closest('button'))details.open=false;});
 win.addEventListener('keydown',event=>{if(event.key==='Escape'&&details.open){details.open=false;event.preventDefault();event.stopImmediatePropagation();}},true);
 win.addEventListener('resize',resize);resize();theme(1);return api;
}
