import {fullmapActorCatalog,fullmapActorSize,readFullmapActors} from './fullmap-actors.js?v=fishing81';
import {paintFullmapLayers} from './fullmap-layers.js?v=fishing81';
import {composeFullmapTerritories} from './fullmap-territories.js?v=territory-dispute214';
import {fullmapScreenPoint,fullmapWorldPoint} from './fullmap-pointer.js';
import {composeFullmapTerrain,loadFullmapTerrainArt,sampleFullmapTiles} from './fullmap-terrain.js';
import {loadOriginalFullmapArt} from './original-fullmap-art.js';
import {nativeMetaEpoch} from './original-hud-layout.js';
import {createFullmapSpriteCache} from './fullmap-sprites.js';
import {paintFullmapCityLabels,paintFullmapTerritoryLabels,readFullmapCityLabels,readFullmapTerritoryLabels} from './fullmap-territory-names.js?v=native22';
import {createOriginalWarplanPane} from './original-warplan-pane.js?v=native24';

const FILTERS = {units:'Einheiten', buildings:'Gebäude und Mauern', resources:'Ressourcen und Bäume', territories:'Territorien'};
const PREFS = 'openepoch-fullmap-options-v1';
const RASTER = 512;
const DOUBLE_PRESS_MS = 250;
const RESOURCE = ['#c8b47b','#466331','#a3a1a7','#dbcfb4','#e9b844','#a0b6c1','#d7c1a6','#53462c','#94cb51','#e0e0e0'];
const FRAME_CELLS = {corner_ul:'1 / 1',edge_top:'1 / 2',corner_ur:'1 / 3',edge_left:'2 / 1',edge_right:'2 / 3',
  corner_ll:'3 / 1',edge_bottom:'3 / 2',corner_lr:'3 / 3'};

function surface(side) {
  const node = document.createElement('canvas');
  node.width = node.height = side;
  return node.getContext('2d');
}

const background = (node, sprite) => node.style.backgroundImage = sprite ? `url("${sprite.url}")` : '';

function rect(node, geometry) {
  if (!geometry) return;
  const {x,y,width,height} = geometry;
  Object.assign(node.style,{left:`${x}px`,top:`${y}px`,width:`${width}px`,height:`${height}px`});
}

export function createFullmap({dialog,wasm,metadata,size,teams,originalArt,territoryNameCatalog,cityNameCatalog,bitmapFonts,hillDefinition,fishingId,
  onAccept,onToggle,onPreview,pip,drawFlares,drawBattleFlares,warplanCue,
  warplanPeer,warplanRecipient,warplanClock}) {
  const canvas = dialog.querySelector('canvas');
  const draw = canvas.getContext('2d', {alpha:false});
  const close = dialog.querySelector('.fullmap-close');
  const sidebarTop = dialog.querySelector('.fullmap-top');
  const sidebarBottom = dialog.querySelector('.fullmap-bottom');
  const pane = dialog.querySelector('.fullmap-options');
  const border = dialog.querySelector('.fullmap-border');
  const framePieces = new Map();
  for (const [part,cell] of Object.entries(FRAME_CELLS)) {
    const item = document.createElement('span');
    item.style.gridArea = cell;
    border.append(item);
    framePieces.set(part,item);
  }
  const pipCanvas = dialog.querySelector('.fullmap-pip');
  const pipLogo = dialog.querySelector('.fullmap-pip-logo');
  const pipControls = dialog.querySelector('.fullmap-pip-controls');
  const terrainDraw = surface(RASTER), territoryDraw = surface(RASTER), layerDraw = surface(size);
  const overlay = layerDraw.createImageData(size,size);
  const bounds = {minX:0,minY:0,maxX:size-1,maxY:size-1};
  const {image,colorize} = createFullmapSpriteCache(teams);
  const switches = new Map();
  let layout, lastShape = '', lastOptions = '', lastDraw = -Infinity;
  let previousPrimaryDown = -Infinity;
  let art, terrainArt, catalog, started, terrainReady, territoryReady, previewPointer, pipDrag;
  let terrain;
  const warplans = createOriginalWarplanPane({host:dialog,options:pane,playCue:warplanCue,
    peer:warplanPeer,recipient:warplanRecipient,clock:warplanClock,teams,refresh:()=>{
    lastDraw = -Infinity;
    previousPrimaryDown = -Infinity;
  }});

  let saved;
  try { saved = JSON.parse(localStorage.getItem(PREFS)); } catch { /* optional preferences */ }
  const filters = {...FILTERS};
  if (typeof wasm.fullmap_city_labels_build === 'function' && typeof wasm.fullmap_city_labels_ptr === 'function') {
    filters.cityNames = 'Städtenamen';
  }
  if (typeof wasm.fullmap_territory_labels_build === 'function' && typeof wasm.fullmap_territory_labels_ptr === 'function') {
    filters.terrNames = 'Territoriennamen';
  }
  if (drawBattleFlares) filters.battles = 'Schlachten';
  const options = Object.fromEntries(Object.keys(filters).map(key=>[key,saved?.[key] !== false]));
  const pipWidgets = new Map();
  const pipViewport = () => pipCanvas.hidden ? null : pipCanvas.getBoundingClientRect();
  const bookmark = (slot,action,time=performance.now()/1000) => pip?.bookmark(slot,action,
    {time,unitHandle:wasm.selected_actor_handle?.()});
  if(pip){
    for(let slot=0;slot<=6;slot++){
      const button=document.createElement('button');
      const name=slot?`fullmap_pip_bookmark_${slot}`:'fullmap_pip_cycle';
      button.type='button';
      button.title=slot?`Lesezeichen ${slot}`:'Lesezeichen durchblättern';
      button.setAttribute('aria-label',button.title);
      button.onclick=()=>slot?bookmark(slot,'select-or-set'):pip.cycle();
      if(slot)button.addEventListener('contextmenu',event=>{event.preventDefault();bookmark(slot,'clear');});
      pipControls.append(button);
      pipWidgets.set(name,{slot,button});
    }
  }
  for (const [key,label] of Object.entries(filters)) {
    const row = document.createElement('label'), input = document.createElement('input');
    const mark = document.createElement('i');
    input.type = 'checkbox'; input.checked = options[key];
    row.append(input,mark,document.createTextNode(label));
    input.onchange = () => {
      options[key] = input.checked;
      try { localStorage.setItem(PREFS,JSON.stringify(options)); } catch { /* optional preferences */ }
      lastOptions = '';
      lastDraw = -Infinity;
    };
    pane.append(row);
    switches.set(key,mark);
  }

  function prepare() {
    if (started) return;
    started = true;
    fetch(new URL('./ee2-fullmap-actor-overlays.json',import.meta.url)).then(response=>{
      if (!response.ok) throw new Error(`Fullmap actor catalog ${response.status}`);
      return response.json();
    }).then(value=>catalog = fullmapActorCatalog(value)).catch(error=>console.warn(error));
    if (!originalArt) return;
    loadOriginalFullmapArt(true).then(value=>{ art=value; lastShape=''; }).catch(error=>console.warn(error));
    loadFullmapTerrainArt().then(value=>{ terrainArt=value; terrainReady=false; }).catch(error=>console.warn(error));
  }

  function resize(epoch) {
    const width = dialog.clientWidth, height = dialog.clientHeight;
    if (!width || !height) return false;
    const meta = nativeMetaEpoch(epoch), shape = `${width}:${height}:${meta}`;
    if (shape !== lastShape) {
      const source = width>=800 && height>=600 ? art?.referenceLayout({width,height}) : null;
      const previewRect = source?.get('fullmap_PIP');
      const logoRect = source?.get('fullmap_pip_hidden_logo');
      const showPip = !!previewRect && !!pip?.isOpen();
      pipCanvas.hidden=!showPip;
      pipControls.hidden=!showPip;
      pipLogo.hidden=showPip||!logoRect;
      if(showPip){
        rect(pipCanvas,previewRect);
        for(const [name,{button}] of pipWidgets)rect(button,source.get(name));
      }else if(logoRect){
        rect(pipLogo,logoRect);
        background(pipLogo,art.sprite('spr_logo_ee2'));
      }
      const fallbackWidth = Math.floor(width*.75);
      const map = source?.get('fullmap_map') ?? {x:0,y:0,width:fallbackWidth,height};
      const side = {x:map.width,y:0,width:width-map.width,height};
      const top = source?.get('fullmap_rightbar_bkg1') ?? {...side,height:Math.floor(height*.42)};
      const bottom = source?.get('fullmap_rightbar_bkg2') ?? {...side,y:top.height,height:height-top.height};
      rect(sidebarTop,top); rect(sidebarBottom,bottom); rect(canvas,map);
      rect(border,map);
      const frame = art?.frame('wfs_game_dialog_meta_inner',epoch);
      for (const [part,item] of framePieces) background(item,frame?.[part]);
      rect(close,source?.get('fullmap_close') ?? {x:width-55,y:8,width:48,height:48});
      rect(pane,source?.get('fullmap_pane_container') ?? {x:side.x+8,y:60,width:Math.max(1,side.width-16),height:height-68});
      background(sidebarTop,art?.role('top',epoch));
      background(sidebarBottom,art?.role('bottom',epoch));
      background(pane,art?.role('panelInset',epoch));
      const closeArt = art?.role('close',epoch);
      background(close,closeArt);
      close.textContent = closeArt ? '' : '×';
      for (const [property,state] of [['--rollover','hover'],['--pressed','pressed']]) {
        const sprite = art?.role('close',epoch,state);
        close.style.setProperty(property,sprite ? `url("${sprite.url}")` : 'none');
      }
      const dpr = Math.min(devicePixelRatio||1,2);
      canvas.width=Math.max(1,Math.round(map.width*dpr));
      canvas.height=Math.max(1,Math.round(map.height*dpr));
      layout = {dpr,width:map.width,height:map.height,viewportWidth:width,viewportHeight:height,
        projection:{left:0,top:0,right:map.width,bottom:map.height}};
      lastShape = shape;
      lastOptions = '';
    }
    const optionKey = `${meta}:${Object.values(options).join()}`;
    if (lastOptions !== optionKey) {
      const theme = meta === 1 ? '' : `_e${meta}`;
      for (const [key,mark] of switches) background(mark,art?.sprite(`spr_game_dialog_checkbox${options[key]?'_on':''}${theme}`));
      dialog.classList.toggle('fullmap-original',!!art);
      lastOptions = optionKey;
    }
    return true;
  }

  function paintPipControls(epoch){
    if(pipControls.hidden)return;
    const state=pip.snapshot(),meta=nativeMetaEpoch(epoch),theme=meta===1?'':`_e${meta}`;
    for(const [name,{slot,button}] of pipWidgets){
      const base=art.source.widgets[name].SPRITE;
      const pressed=slot?state.current===slot:state.cycling;
      const key=slot?base.replace(/unset$/,pressed?'selected':state.bookmarks[slot]?'set':'unset'):base+(pressed?'_on':'');
      const sprite=art.sprite(key+theme);
      background(button,sprite);
      const hover=art.sprite(key+'_roll'+theme)??sprite;
      button.style.setProperty('--rollover',hover?`url("${hover.url}")`:'none');
      button.setAttribute('aria-pressed',String(pressed));
    }
  }

  function refreshTerrain() {
    terrain = new Uint8Array(wasm.memory.buffer,wasm.terrain_types_ptr(),size*size).slice();
    if (terrainArt) {
      const heights = new Float32Array(wasm.memory.buffer,wasm.hmap_ptr(),(size+1)**2);
      const pixels = composeFullmapTerrain(terrainArt,sampleFullmapTiles(heights,terrain,size,size));
      terrainDraw.putImageData(new ImageData(pixels,RASTER,RASTER),0,0);
    } else {
      const colors = new Uint8ClampedArray(wasm.memory.buffer,wasm.map_ptr(),size*size*4);
      terrainDraw.putImageData(new ImageData(colors,size,size),0,0);
    }
    terrainReady = true;
  }

  function refreshTerritories() {
    const tiles=new Uint8Array(wasm.memory.buffer,wasm.terr_ptr(),size*size*4);
    const heights=new Float32Array(wasm.memory.buffer,wasm.hmap_ptr(),(size+1)**2);
    const pixels=composeFullmapTerritories(tiles,heights,terrain,size);
    territoryDraw.putImageData(new ImageData(pixels,RASTER,RASTER),0,0);
    territoryReady=true;
  }

  function paint(now,ui,corners) {
    if (!dialog.open || !ui || now-lastDraw<120 || !resize(ui.epoch)) return;
    lastDraw = now;
    paintPipControls(ui.epoch);
    if (!terrainReady) refreshTerrain();
    const {width,height,dpr,projection} = layout;
    const project = (x,y) => fullmapScreenPoint(x,bounds.maxY-y,projection,bounds);
    warplans.layout(art,dialog.clientWidth,dialog.clientHeight,ui.epoch);
    const sourceSide = terrainArt ? RASTER : size;
    draw.setTransform(dpr,0,0,dpr,0,0);
    draw.imageSmoothingEnabled = true;
    draw.drawImage(terrainDraw.canvas,0,0,sourceSide,sourceSide,0,0,width,height);
    if (options.territories) {
      if (!territoryReady) refreshTerritories();
      draw.drawImage(territoryDraw.canvas,0,0,width,height);
    }
    const resources = paintFullmapLayers(wasm,size,terrain,{resources:options.resources,local:ui.local},overlay.data);
    layerDraw.putImageData(overlay,0,0);
    draw.imageSmoothingEnabled = false;
    draw.drawImage(layerDraw.canvas,0,0,width,height);
    draw.imageSmoothingEnabled = true;
    for (const {kind,x,y} of resources) {
      const [px,py] = project(x,y);
      const fish = kind===10 ? image(art?.sprite('spr_fullmap_fish')) : null;
      if (fish) {
        draw.drawImage(fish,px-6,py-6,12,12);
        continue;
      }
      draw.fillStyle = RESOURCE[kind-1];
      draw.fillRect(px-2,py-2,4,4);
    }
    if (catalog) {
      const actors = readFullmapActors(wasm,metadata,catalog,{width:size,height:size,
        showUnits:options.units,showBuildings:options.buildings,hillDefinition,fishingId});
      for (const actor of actors) {
        const [x,y] = project(actor.x,actor.y);
        const sprite = art?.sprite(actor.overlay.sprite), icon = image(sprite);
        const [w,h] = fullmapActorSize(actor.overlay,width,height,bounds.maxX,bounds.maxY);
        draw.save(); draw.translate(x,y);
        // The installed renderer negates native yaw. OpenEpoch's world Y points south.
        draw.rotate(actor.yaw);
        if (icon) {
          draw.drawImage(colorize(sprite,icon,actor.owner),-w/2,-h/2,w,h);
        } else {
          draw.fillStyle = `rgb(${teams[actor.owner].map(value=>Math.round(value*255)).join(',')})`;
          draw.fillRect(-w/2,-h/2,w,h);
        }
        draw.restore();
      }
    }
    for (const king of ui.fullmapKings ?? []) {
      const [x,y] = project(king.x,king.y);
      const sprite = art?.role('crownPlayerColor',ui.epoch), mask = image(sprite);
      const crown = image(art?.role('crown',ui.epoch));
      if (crown) draw.drawImage(crown,x-32,y-32,64,64);
      else {
        draw.font = 'bold 22px serif'; draw.textAlign = 'center'; draw.fillStyle = '#e5ba50';
        draw.fillText('♛',x,y+8);
      }
      if (mask) draw.drawImage(colorize(sprite,mask,king.owner),x-32,y-32,64,64);
    }
    if (corners) {
      draw.strokeStyle = '#fff'; draw.lineWidth = 1.5;
      draw.beginPath();
      corners().forEach(([x,y],i)=>draw[i?'lineTo':'moveTo'](...project(x,y)));
      draw.closePath(); draw.stroke();
    }
    if (options.terrNames) paintFullmapTerritoryLabels(draw,readFullmapTerritoryLabels(wasm),project,layout,
      {names:territoryNameCatalog?.names,bitmapFonts});
    if (options.cityNames) paintFullmapCityLabels(draw,readFullmapCityLabels(wasm),project,layout,
      {names:cityNameCatalog?.names,bitmapFonts});
    warplans.paint(draw,(x,y)=>fullmapScreenPoint(x,y,projection,bounds),dialog.clientWidth,dialog.clientHeight);
    drawFlares?.(draw,project,64,'fullmap');
    if (options.battles) drawBattleFlares(draw,project,64,'fullmap');
  }

  function nativePoint(event, integerInput = false) {
    const {left,top,right,bottom} = canvas.getBoundingClientRect();
    const x = integerInput ? Math.trunc(event.clientX) : event.clientX;
    const y = integerInput ? Math.trunc(event.clientY) : event.clientY;
    return fullmapWorldPoint(x,y,{left,top,right,bottom},bounds);
  }

  function point(event) {
    const world = nativePoint(event);
    return world ? [world[0],bounds.maxY-world[1]] : null;
  }

  function closeMap() {
    if (dialog.open) { pip?.close(); dialog.close(); }
  }

  function openMap() {
    if (dialog.open) return;
    prepare();
    lastDraw = -Infinity;
    lastShape = '';
    previousPrimaryDown = -Infinity;
    dialog.showModal();
    pip?.open({pane:'options',getViewport:pipViewport,getPreviewCanvas:()=>pipCanvas});
    onToggle?.(true);
  }

  dialog.addEventListener('cancel',event=>{ event.preventDefault(); closeMap(); });
  dialog.addEventListener('close',()=>{
    if (dialog.open) return;
    previewPointer = undefined;
    pipDrag = undefined;
    pip?.close();
    warplans.close();
    onPreview?.(null);
    onToggle?.(false);
  });
  close.onclick = closeMap;
  canvas.addEventListener('contextmenu',event=>{
    if(warplans.capturing())event.preventDefault();
  });
  canvas.addEventListener('pointerdown',event=>{
    if(warplans.modal()){event.preventDefault();return;}
    if(warplans.capturing()&&warplans.pointer('down',event,nativePoint(event,true))){event.preventDefault();return;}
    if (event.button !== 0 || event.isPrimary === false) return;
    const world = point(event);
    if (!world) return;
    const elapsed = event.timeStamp - previousPrimaryDown;
    previousPrimaryDown = event.timeStamp;
    if (onPreview || pip?.isOpen()) {
      previewPointer = event.pointerId;
      canvas.setPointerCapture(previewPointer);
      onPreview?.(world);
      pip?.preview(world);
    }
    if (elapsed >= 0 && elapsed < DOUBLE_PRESS_MS) {
      onAccept(world);
      closeMap();
    }
  });
  canvas.addEventListener('pointermove',event=>{
    if(warplans.capturing()&&warplans.pointer('move',event,nativePoint(event,true)))return;
    if (event.pointerId !== previewPointer) return;
    const world=point(event);
    onPreview?.(world);
    pip?.preview(world);
  });
  canvas.addEventListener('lostpointercapture',()=>{
    previewPointer = undefined;
    onPreview?.(null);
    pip?.preview(null);
  });

  pipCanvas.addEventListener('contextmenu',event=>event.preventDefault());
  pipCanvas.addEventListener('pointerdown',event=>{
    if(!pip?.isOpen()||event.button!==2)return;
    pipDrag={id:event.pointerId,x:Math.trunc(event.clientX),y:Math.trunc(event.clientY)};
    pipCanvas.setPointerCapture(pipDrag.id);
    event.preventDefault();
  });
  pipCanvas.addEventListener('pointermove',event=>{
    if(!pipDrag||pipDrag.id!==event.pointerId)return;
    const x=Math.trunc(event.clientX),y=Math.trunc(event.clientY);
    pip.drag(x-pipDrag.x,y-pipDrag.y);
    pipDrag={id:pipDrag.id,x,y};
  });
  pipCanvas.addEventListener('lostpointercapture',()=>{pipDrag=undefined;});
  pipCanvas.addEventListener('wheel',event=>{
    // Browser line/page deltas represent a discrete notch. Pixel deltas are
    // forwarded proportionally for trackpads through the native-unit adapter.
    const units=event.deltaMode?Math.sign(-event.deltaY)*120:Math.round(-event.deltaY);
    if(pip?.wheel(event.clientX,event.clientY,units))event.preventDefault();
  },{passive:false});

  function pipKey(event){
    if(!dialog.open||pipCanvas.hidden||event.altKey||event.metaKey||event.shiftKey)return false;
    const key=/^F([1-7])$/.exec(event.key);
    if(!key||(+key[1]===7&&event.ctrlKey))return false;
    event.preventDefault();
    if(!event.repeat){
      if(+key[1]===7)pip.cycle();
      else bookmark(+key[1],event.ctrlKey?'set':'select-or-set');
    }
    return true;
  }

  return {open:openMap,close:closeMap,isOpen:()=>dialog.open,paint,pipKey,warplans,
    invalidate(){terrainReady=false;territoryReady=false;}, invalidateTerritories(){territoryReady=false;}};
}
