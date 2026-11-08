import {calculateHudLayout,nativeHudFrame} from './original-hud-layout.js';
import {controlMediaURL,loadOriginalControlCatalog} from './original-game-options.js?v=native37';
import {ORIGINAL_AUDIO_DEFAULTS} from './original-audio-levels.js?v=native38audio';

const path='local-original/game-controls/settings-manifest.json';
const digest='6ff33963fbcf7709e0469a0d3dae3b940b73056f82fe4fa43da872aa2f3bbbb8';
const tabs=['audio','video','general','hotkeys'];
const pieces=['ul','ll','lr','ur','top','left','bottom','right'];
const fallback={...ORIGINAL_AUDIO_DEFAULTS,muted:false,voices:true,resolution:'2',effectDetail:'1',fullscreen:false};
const cssImage=url=>url?`url("${url}")`:'none';
const position=(node,{x,y,width,height})=>Object.assign(node.style,{left:`${x}px`,top:`${y}px`,width:`${width}px`,height:`${height}px`});

export async function mountOriginalGameSettings({enabled=true,originalArt=false,read=()=>fallback,apply,previewAudio,defaults=fallback,
  hotkeys=()=>[],movie,onClose,dialog=globalThis.document?.getElementById('native-game-settings'),window:win=globalThis.window,
  fetchFile=globalThis.fetch,base=new URL('./',import.meta.url),profile='packed7'}={}){
  const none=()=>false;
  if(!enabled||!dialog)return {open:none,close:none,active:none};
  const catalog=originalArt?await loadOriginalControlCatalog({fetchFile,base,profile,name:path,hash:digest,
    format:'ee2-native-settings-panel-v1'}):null;
  const source=catalog?.source,doc=dialog.ownerDocument;
  const form=dialog.querySelector('form'), controls=new Map([...form.querySelectorAll('[data-setting]')].map(node=>[node.dataset.setting,node]));
  const panelNodes=new Map([...form.querySelectorAll('[data-settings-pane]')].map(node=>[node.dataset.settingsPane,node]));
  const tabNodes=new Map([...form.querySelectorAll('[data-settings-tab]')].map(node=>[node.dataset.settingsTab,node]));
  const artNodes=[...form.querySelectorAll('[data-settings-widget]')];
  const extras=[...form.querySelectorAll('[data-settings-extra]')];
  const frames=[...form.querySelectorAll('[data-settings-frame]')];
  const video=dialog.querySelector('video'),keyList=form.querySelector('.native-settings-keys');
  const standard=form.querySelector('[data-settings-command="defaults"]');
  let activeTab='audio',movieState,opened=false,originalAudio=null,previewed=false;
  for(const frame of frames){
    for(const part of pieces){const node=doc.createElement('span');node.dataset.settingsPiece=part;frame.append(node);}
  }

  function select(tab,focus=false){
    if(!tabs.includes(tab))return;
    activeTab=tab;
    for(const [name,pane] of panelNodes)pane.hidden=name!==tab;
    for(const [name,button] of tabNodes){
      const selected=name===tab;
      button.tabIndex=selected?0:-1;
      button.setAttribute('aria-selected',String(selected));
    }
    standard.disabled=tab==='hotkeys';
    if(focus)tabNodes.get(tab).focus();
  }
  function set(values){
    const valuesWithDefaults={...fallback,...values};
    for(const [name,node] of controls){
      if(node.type==='checkbox')node.checked=!!valuesWithDefaults[name];
      else node.value=String(valuesWithDefaults[name]);
    }
  }
  function current(){
    return Object.fromEntries([...controls].map(([name,node])=>[name,node.type==='checkbox'?node.checked:
      node.type==='range'?Number(node.value):node.value]));
  }
  function audition(){
    if(!previewAudio)return;
    previewed=true;
    const {music,ambience,musicVolume,soundVolume}=current();
    previewAudio({music,ambience,musicVolume,soundVolume});
  }
  function updateKeys(){
    const fragment=doc.createDocumentFragment();
    let group;
    for(const row of hotkeys()??[]){
      if(row.group!==group){
        group=row.group;
        if(group){const heading=doc.createElement('li');heading.className='native-settings-key-group';heading.textContent=group;fragment.append(heading);}
      }
      const line=doc.createElement('li'),label=doc.createElement('span'),key=doc.createElement('kbd');
      label.textContent=row.label;
      key.textContent=Array.isArray(row.keys)?row.keys.join(' · '):row.keys;
      line.append(label,key);
      fragment.append(line);
    }
    keyList.replaceChildren(fragment);
  }
  function draw(){
    if(!source)return;
    const width=win.innerWidth,height=win.innerHeight,layout=calculateHudLayout(source,width,height);
    const variant=(name,state='idle')=>{
      const states=source.spriteStates[name]?.[1];
      return source.sprites[states?.[state]??states?.idle];
    };
    const image=(name,state)=>cssImage(controlMediaURL(variant(name,state),base));
    dialog.classList.add('native-settings-source');
    for(const node of artNodes){
      const name=node.dataset.settingsWidget,row=source.widgets[name],rect=layout.get(name);
      if(!row||!rect)continue;
      position(node,rect);
      const art=variant(row.SPRITE);
      if(art)node.style.backgroundImage=cssImage(controlMediaURL(art,base));
      if(row.LABEL&&node.dataset.sourceLabel!==undefined)node.textContent=source.labels[row.LABEL]?.text??node.textContent;
      if(row.TEXTJUST?.startsWith('Right'))node.style.textAlign='right';
      if(row.TEXTJUST?.startsWith('Center'))node.style.textAlign='center';
      if(row.BUTTONSTYLE){
        node.style.setProperty('--native-settings-rollover',image(row.SPRITE,'r'));
        node.style.setProperty('--native-settings-pressed',image(row.SPRITE,'p'));
      }
      const tile=/(?:^|\W)tileUV(?:\W|$)/.test(row.ARGSTR)&&art?.logicalSize;
      if(tile){
        const scale=Number(/\btilingScale\s*=\s*([\d.]+)/.exec(row.ARGSTR)?.[1]??1);
        node.style.backgroundSize=`${tile[0]*scale*width/1600}px ${tile[1]*scale*height/1200}px`;
        node.style.backgroundRepeat='repeat';
      }
    }
    for(const node of extras){
      const section=layout.get(node.dataset.settingsExtra), x=Math.trunc(width/1600*Number(node.dataset.offsetX)),y=Math.trunc(height/1200*Number(node.dataset.offsetY));
      position(node,{x:section.x+x,y:section.y+y,width:Math.trunc(width/1600*265),height:Math.trunc(height/1200*36)});
    }
    for(const container of frames){
      const name=container.dataset.settingsFrame,row=source.widgets[name],rect=layout.get(name);
      const {inner,outer}=nativeHudFrame(source,row,width),border=inner+outer;
      position(container,{x:rect.x-outer,y:rect.y-outer,width:rect.width+2*outer,height:rect.height+2*outer});
      container.style.setProperty('--native-settings-frame',`${border}px`);
      for(const [index,part] of pieces.entries()){
        const sprite=source.sprites[source.frameSprites[row.FRAMESTYLE]?.[index]],node=container.querySelector(`[data-settings-piece="${part}"]`);
        node.style.backgroundImage=cssImage(controlMediaURL(sprite,base));
        if(index>=4&&sprite){
          const [w,h]=sprite.logicalSize??sprite.size,across=part==='top'||part==='bottom';
          node.style.backgroundSize=across?`${border*w/h}px 100%`:`100% ${border*h/w}px`;
          node.style.backgroundRepeat=across?'repeat-x':'repeat-y';
        }
      }
    }
    dialog.style.setProperty('--native-settings-check-off',image('spr_fe_checkbox'));
    dialog.style.setProperty('--native-settings-check-on',image('spr_fe_checkbox','on'));
    dialog.style.setProperty('--native-settings-volume-track',image('spr_fe_slider_horiz_bkg'));
    dialog.style.setProperty('--native-settings-volume-thumb',image('spr_fe_slider_horiz_thumb'));
    dialog.style.setProperty('--native-settings-volume-track-height',`${12*height/1200}px`);
    dialog.style.setProperty('--native-settings-volume-thumb-width',`${20*width/1600}px`);
  }
  function open(){
    if(dialog.open)return false;
    set(read());
    const {music,ambience,musicVolume,soundVolume}=current();
    originalAudio={music,ambience,musicVolume,soundVolume};
    previewed=false;
    updateKeys();
    select(activeTab);
    draw();
    dialog.showModal();
    opened=true;
    if(originalArt&&movie){movieState??=movie(video);movieState?.(true);}
    return true;
  }
  function finish(){
    if(!opened)return;
    opened=false;
    if(previewed){previewAudio(originalAudio);previewed=false;}
    movieState?.(false);
    onClose?.();
  }
  function close(){
    if(!dialog.open)return false;
    dialog.close();
    finish();
    return true;
  }
  for(const [tab,node] of tabNodes){
    node.addEventListener('click',()=>select(tab));
    node.addEventListener('keydown',event=>{
      const index=tabs.indexOf(tab),delta=event.key==='ArrowLeft'?-1:event.key==='ArrowRight'?1:0;
      const next=event.key==='Home'?tabs[0]:event.key==='End'?tabs.at(-1):delta?tabs[(index+delta+tabs.length)%tabs.length]:null;
      if(next){event.preventDefault();select(next,true);}
    });
  }
  for(const name of ['music','ambience','musicVolume','soundVolume']){
    controls.get(name)?.addEventListener('input',()=>{
      if(!dialog.open)return;
      if(name==='musicVolume')controls.get('music').checked=Number(controls.get(name).value)>0;
      audition();
    });
  }
  form.addEventListener('submit',event=>{
    event.preventDefault();
    if(dialog.open&&apply?.(current())!==false){previewed=false;close();}
  });
  form.querySelector('[data-settings-command="cancel"]').addEventListener('click',close);
  standard.addEventListener('click',()=>{
    if(!dialog.open||standard.disabled)return;
    const previous=current(), defaults_=typeof defaults==='function'?defaults():defaults;
    if(activeTab==='audio'){set({...previous,...ORIGINAL_AUDIO_DEFAULTS});audition();}
    else if(activeTab==='video')set({...previous,resolution:defaults_.resolution,effectDetail:defaults_.effectDetail??fallback.effectDetail});
    else if(activeTab==='general')set({...previous,fullscreen:defaults_.fullscreen});
  });
  dialog.addEventListener('close',()=>{if(!dialog.open)finish();});
  dialog.addEventListener('cancel',event=>{event.preventDefault();close();});
  win.addEventListener('resize',()=>{if(dialog.open)draw();});
  return {open,close,active:()=>dialog.open};
}
