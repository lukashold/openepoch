import {calculateHudLayout,nativeHudFrame,nativeHudRect,nativeMetaEpoch} from './original-hud-layout.js';

const manifestName='local-original/game-controls/pause-manifest.json';
const manifestHash='abcfa68d506575da42416f1858b9252f6895dae14d66dac8d4e31e5280ea3c8f';
const panel='pause_panel',glass='pause_panel_glass';
const widgets=Object.freeze({resume:'return',save:'save',load:'load',restart:'restart',resign:'resign',
  settings:'settings',record:'recordorstop',exit:'quit',desktop:'quitDesktop'});
const keys=Object.freeze({S:'save',L:'load',E:'restart',R:'resign',G:'settings',X:'exit'});
const pieces=['ul','ll','lr','ur','top','left','bottom','right'];
const artLayers=['bkg_top','bkg_bot','glass','divider_1','divider_2'];

export function controlMediaURL(row,base){
  if(!row||!/^local-original\/[\w./-]+$/.test(row.url)||row.url.split('/').includes('..')||!/^[a-f0-9]{64}$/.test(row.sha256))return '';
  const url=new URL(row.url,base);
  url.searchParams.set('v',row.sha256.slice(0,16));
  return url.href;
}

export async function loadOriginalControlCatalog({fetchFile,base,profile,name,hash,format}){
  if(profile!=='packed7'&&profile!=='packed5')return null;
  try{
    const response=await fetchFile(new URL(name,base),{cache:'no-cache'});
    if(!response.ok)return null;
    const bytes=await response.arrayBuffer();
    const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
    if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==hash)return null;
    const data=JSON.parse(new TextDecoder().decode(bytes));
    if(data.format!==format||!data.profiles?.[profile])return null;
    return {source:{...data.profiles[profile],resolutionWidths:data.resolutionWidths},sounds:data.sounds};
  }catch{return null;}
}

// 00741920 constructs the active buttons before resizing glass and panel to
// match their top and bottom margins. Dividers are constructed after the resize.
export function nativePauseLayout(source,width,height,allowRecording){
  const all=source.widgets, definitions={...all};
  delete definitions.pause_panel_divider_1;
  delete definitions.pause_panel_divider_2;
  delete definitions.pause_panel_quit_norecord;
  if(!allowRecording){
    delete definitions.pause_panel_recordorstop;
    definitions.pause_panel_quit={...all.pause_panel_quit_norecord,NAME:'pause_panel_quit'};
  }
  const layout=calculateHudLayout({...source,widgets:definitions},width,height);
  const root=layout.get(panel),inner=layout.get(glass), frame=nativeHudFrame(source,all[glass],width);
  const bounds=Object.values(widgets).map(name=>layout.get(`${panel}_${name}`)).filter(Boolean);
  const start=Math.min(...bounds.map(row=>row.y));
  const end=Math.max(...bounds.map(row=>row.y+row.height));
  const delta=start-(inner.y+frame.inner)+end-(inner.y+inner.height-frame.inner);
  root.height+=delta;
  inner.height+=delta;
  const outerGlass={x:inner.x-frame.outer,y:inner.y-frame.outer,width:inner.width+2*frame.outer,height:inner.height+2*frame.outer};
  for(const name of ['pause_panel_divider_1','pause_panel_divider_2'])
    layout.set(name,nativeHudRect(all[name],outerGlass,outerGlass,width,height));
  const x=root.x,y=root.y;
  for(const [name,rect] of layout)if(name!==panel){rect.x-=x;rect.y-=y;}
  root.x=Math.trunc((width-root.width)/2);
  root.y=Math.trunc((height-root.height)/2);
  return layout;
}

const place=(element,{x,y,width,height})=>Object.assign(element.style,{left:`${x}px`,top:`${y}px`,width:`${width}px`,height:`${height}px`});
const cssImage=url=>url?`url("${url}")`:'none';

export async function mountOriginalGameOptions({enabled=true,originalArt=false,dialog=globalThis.document?.getElementById('native-game-pause'),
  view=()=>({}),onOpen,onClose,onAction,onCue,fetchFile=globalThis.fetch,base=new URL('./',import.meta.url),profile='packed7',
  window:win=globalThis.window}={}){
  const noop=()=>false;
  if(!enabled||!dialog)return {open:noop,close:noop,toggle:noop,active:noop,update:noop,dispatch:noop};
  const art=originalArt?await loadOriginalControlCatalog({fetchFile,base,profile,name:manifestName,
    hash:manifestHash,format:'ee2-native-pause-panel-v1'}):null;
  const buttons=new Map([...dialog.querySelectorAll('[data-pause-action]')].map(button=>[button.dataset.pauseAction,button]));
  const layers=new Map(artLayers.map(name=>[name,dialog.querySelector(`[data-pause-layer="${name}"]`)]));
  const frame=dialog.querySelector('.native-pause-frame');
  let hold=false,shape='',lastAvailability='';
  const cue=role=>{
    const row=art?.sounds?.[role],url=controlMediaURL(row,base);
    if(url)onCue?.({...row,role,url});
  };
  function update(){
    const state=view()??{}, epoch=state.epoch??0, allowRecording=!!state.allowRecording;
    const disabled=new Set(state.disabled??[]);
    const availability=[allowRecording,!!state.recording,...disabled].join('/');
    if(availability!==lastAvailability){
      for(const [action,button] of buttons){
        button.hidden=action==='record'&&!allowRecording;
        button.disabled=disabled.has(action);
      }
      const record=buttons.get('record');
      if(record)record.textContent=state.recording?'AUFNAHME STOPPEN':'SPIEL AUFZEICHNEN';
      lastAvailability=availability;
    }
    if(!art)return false;
    const source=art.source, meta=nativeMetaEpoch(epoch), width=win.innerWidth,height=win.innerHeight;
    const nextShape=[width,height,meta,allowRecording].join('/');
    if(nextShape===shape)return true;
    shape=nextShape;
    const layout=nativePauseLayout(source,width,height,allowRecording),root=layout.get(panel);
    dialog.classList.add('native-pause-source');
    place(dialog,root);
    const nearest=source.resolutionWidths.reduce((best,value,index)=>
      Math.abs(value-width)<Math.abs(source.resolutionWidths[best]-width)?index:best,0);
    dialog.style.setProperty('--native-pause-font',`${[16,21,24,29,29,38][nearest]}px`);
    const variant=(name,state='idle')=>{
      const choices=source.spriteStates[name];
      const key=choices?.[meta]?.[state]??choices?.[meta]?.idle??choices?.[1]?.[state]??choices?.[1]?.idle;
      return source.sprites[key];
    };
    for(const [layer,node] of layers){
      const name=`${panel}_${layer}`,row=source.widgets[name],sprite=variant(row.SPRITE);
      place(node,layout.get(name));
      node.style.backgroundImage=cssImage(controlMediaURL(sprite,base));
      const tile=/(?:^|\W)tileUV(?:\W|$)/.test(row.ARGSTR)&&sprite?.logicalSize;
      node.style.backgroundSize=tile?`${Math.max(1,Math.trunc(tile[0]*width/1600))}px ${Math.max(1,Math.trunc(tile[1]*height/1200))}px`:'100% 100%';
      node.style.backgroundRepeat=tile?'repeat':'no-repeat';
    }
    const outer=nativeHudFrame(source,source.widgets[panel],width,meta).outer;
    frame.style.setProperty('--native-pause-frame',`${outer}px`);
    const parts=source.frameSprites[`wfs_game_dialog_meta${meta===1?'':`_e${meta}`}`];
    for(const [i,part] of pieces.entries()){
      const node=frame.querySelector(`[data-pause-piece="${part}"]`);
      const sprite=source.sprites[parts?.[i]];
      node.style.backgroundImage=cssImage(controlMediaURL(sprite,base));
      if(i>=4&&sprite){
        const [w,h]=sprite.logicalSize??sprite.size;
        const across=part==='top'||part==='bottom';
        node.style.backgroundSize=across?`${outer*w/h}px 100%`:`100% ${outer*h/w}px`;
        node.style.backgroundRepeat=across?'repeat-x':'repeat-y';
      }
    }
    const glassWidth=nativeHudFrame(source,source.widgets[glass],width).outer;
    layers.get('glass').style.setProperty('--native-pause-glass',`${glassWidth}px`);
    for(const [action,button] of buttons){
      const name=`${panel}_${widgets[action]}`,row=source.widgets[name],rect=layout.get(name);
      if(rect)place(button,rect);
      for(const [property,state] of [['idle','idle'],['rollover','r'],['pressed','p']])
        button.style.setProperty(`--native-pause-${property}`,cssImage(controlMediaURL(variant(row.SPRITE,state),base)));
    }
    return true;
  }
  function release(){
    if(!hold)return;
    hold=false;
    cue('close');
    onClose?.();
  }
  function open(){
    if(dialog.open||onOpen?.()===false)return false;
    update();
    dialog.showModal();
    hold=true;
    cue('open');
    return true;
  }
  function close(){
    if(!dialog.open)return false;
    dialog.close();
    release();
    return true;
  }
  function dispatch(action){
    if(!dialog.open||!buttons.has(action))return false;
    const button=buttons.get(action);
    if(button.hidden||button.disabled)return false;
    if(action==='resume'){close();return true;}
    onAction?.(action);
    return true;
  }
  for(const [action,button] of buttons){
    button.addEventListener('click',()=>dispatch(action));
    const key=Object.keys(keys).find(value=>keys[value]===action);
    if(key)button.setAttribute('aria-keyshortcuts',key);
  }
  dialog.addEventListener('close',()=>{if(!dialog.open)release();});
  dialog.addEventListener('cancel',event=>event.preventDefault());
  dialog.addEventListener('keydown',event=>{
    if(event.ctrlKey||event.altKey||event.metaKey||event.shiftKey||event.repeat)return;
    const action=keys[event.key.toUpperCase()];
    if(action){event.preventDefault();event.stopPropagation();dispatch(action);}
  });
  win.addEventListener('resize',()=>{if(dialog.open)update();});
  return {open,close,toggle:()=>dialog.open?close():open(),active:()=>dialog.open,update,dispatch};
}
