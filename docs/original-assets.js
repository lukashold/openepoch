// Local assets extracted from this machine's EE2 installation. The simulation is independent.
const TERRAIN_KEYS=['grass','grass_patchy','dirt','sand','rock','waves','oceanfloor','gravel'];
async function bitmap(url){
  const response=await fetch(new URL(url,import.meta.url));
  if(!response.ok)throw new Error(`Asset ${url}: ${response.status}`);
  return createImageBitmap(await response.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
}

export async function loadOriginalAssets(gl,enabled){
  const result={enabled:false,icons:new Map(),iconVariants:new Map(),hud:{},manifest:null,terrainTexture:null,errors:[]};
  // Active samplers need complete textures even when their uniform branch is off.
  const fallback=gl.createTexture();gl.activeTexture(gl.TEXTURE4);gl.bindTexture(gl.TEXTURE_2D_ARRAY,fallback);
  gl.texStorage3D(gl.TEXTURE_2D_ARRAY,1,gl.RGBA8,1,1,TERRAIN_KEYS.length);
  gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,0,1,1,TERRAIN_KEYS.length,gl.RGBA,gl.UNSIGNED_BYTE,new Uint8Array(TERRAIN_KEYS.length*4).fill(255));
  gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
  gl.activeTexture(gl.TEXTURE0);result.terrainTexture=fallback;
  if(!enabled)return result;
  try{
    const r=await fetch(new URL('./local-original/textures.json',import.meta.url),{cache:'no-store'});
    if(!r.ok)return result;
    const manifest=await r.json();result.manifest=manifest;
    const images=await Promise.all(TERRAIN_KEYS.map(key=>bitmap(manifest.terrain[key]||manifest.terrain.grass)));
    const size=256, canvas=document.createElement('canvas');canvas.width=canvas.height=size;
    const context=canvas.getContext('2d',{willReadFrequently:true});
    const texture=gl.createTexture();gl.activeTexture(gl.TEXTURE4);gl.bindTexture(gl.TEXTURE_2D_ARRAY,texture);
    gl.texStorage3D(gl.TEXTURE_2D_ARRAY,9,gl.RGBA8,size,size,images.length);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
    for(let i=0;i<images.length;i++){
      context.clearRect(0,0,size,size);context.drawImage(images[i],0,0,size,size);
      gl.texSubImage3D(gl.TEXTURE_2D_ARRAY,0,0,0,i,size,size,1,gl.RGBA,gl.UNSIGNED_BYTE,context.getImageData(0,0,size,size).data);
      images[i].close();
    }
    gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_S,gl.REPEAT);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_WRAP_T,gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,gl.LINEAR_MIPMAP_LINEAR);gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
    gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
    const anisotropy=gl.getExtension('EXT_texture_filter_anisotropic');
    if(anisotropy)gl.texParameterf(gl.TEXTURE_2D_ARRAY,anisotropy.TEXTURE_MAX_ANISOTROPY_EXT,Math.min(8,gl.getParameter(anisotropy.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
    result.terrainTexture=texture;result.enabled=true;
    const pairs=Object.entries(manifest.icons||{});
    await Promise.all(pairs.map(async([id,url])=>{try{result.icons.set(+id,await bitmap(url));}catch(e){result.errors.push(e.message);}}));
    await Promise.all(Object.entries(manifest.iconVariants||{}).map(async([key,url])=>{try{result.iconVariants.set(key,await bitmap(url));}catch(e){result.errors.push(e.message);}}));
    result.hud=manifest.hudCropped||manifest.hud||{};
    return result;
  }catch(e){result.errors.push(e.message);console.warn('Originalgrafiken konnten nicht vollständig geladen werden:',e);return result;}
  finally{gl.activeTexture(gl.TEXTURE0);}
}

const ICON_EPOCH_PATTERNS={
  0:/^icon_unit_citycenter_(\d+)_we$/,1:/^icon_unit_house_(\d+)(?:_we)?$/,3:/^icon_unit_barracks_(\d+)$/,
  4:/^icon_unit_mill_(\d+)$/,5:/^icon_unit_dock_(\d+)$/,6:/^icon_unit_airport_(\d+)$/,
  7:/^icon_unit_outpost_(\d+)$/,8:/^icon_unit_citizen_m_we_(\d+)$/,47:/^icon_unit_university_(\d+)$/,
  48:/^icon_unit_mill_(\d+)$/,49:/^icon_unit_stable_(\d+)$/,50:/^icon_unit_market_(\d+)$/,
  51:/^icon_unit_temple_(\d+)(?:_we)?$/,52:/^icon_unit_warehouse_(\d+)$/,
  56:/^icon_unit_fortress_(\d+)(?:_we)?$/
};
export function originalIcon(assets,id,epoch){
  const pattern=ICON_EPOCH_PATTERNS[id];let chosen=assets.icons.get(id),latest=0;
  if(pattern)for(const [key,image] of assets.iconVariants){const m=key.match(pattern),score=m?+m[1]*2+(key.endsWith('_we')?1:0):0;if(m&&+m[1]<=epoch+1&&score>=latest){chosen=image;latest=score;}}
  // Follow every exported model transition, including those lacking a proven
  // native SpriteName. Their explicit model-associated fallback uses the base
  // icon of the identical reused geometry, never a stale preceding model icon.
  for(const row of assets.epochIcons?.get(id)||[])if(row.epoch<=epoch)chosen=row.image;
  return chosen;
}

export async function loadOriginalEpochIcons(assets,modelManifest){
  assets.epochIcons=new Map();if(!assets.enabled)return;
  const images=new Map(),exact=new Map((modelManifest?.epochIconBindings||[]).map(row=>[`${row.id}:${row.epoch}`,row]));
  const transitions=modelManifest?.models?.filter(row=>row.progressionBinding)||modelManifest?.epochIconBindings||[];
  await Promise.all(transitions.map(async transition=>{
    const row=exact.get(`${transition.id}:${transition.epoch}`),id=transition.id;
    try{
      if(row&&!images.has(row.url))images.set(row.url,bitmap(row.url));
      const fallback=assets.icons.get(transition.progressionBinding?.reusedFrom?.id);
      const image=row?await images.get(row.url):fallback,list=assets.epochIcons.get(id)||[];
      list.push({...transition,image,nativeExact:!!row,iconFallback:row?null:'associated-model-base-icon'});assets.epochIcons.set(id,list);
    }catch(error){
      assets.errors.push(error.message);const list=assets.epochIcons.get(id)||[];
      list.push({...transition,image:assets.icons.get(transition.progressionBinding?.reusedFrom?.id)??assets.icons.get(id),nativeExact:false,iconFallback:'unavailable-native-sprite'});
      assets.epochIcons.set(id,list);
    }
  }));
  for(const list of assets.epochIcons.values())list.sort((a,b)=>a.epoch-b.epoch);
}

export function applyOriginalInterface(assets){
  if(!assets.enabled)return;
  document.documentElement.classList.add('original-art');
  const hud=assets.hud;
  const set=(id,key)=>{const element=document.getElementById(id);if(element&&hud[key])element.style.setProperty('--ee2-panel',`url("${hud[key]}")`);};
  set('bottom','ui_unitinfo_bkg_stone');set('info','ui_unitinfo_bkg_stone');set('acts','ui_unitinfo_bkg_stone');
  if(hud.ui_ingame_button_small_stone)document.documentElement.style.setProperty('--ee2-button',`url("${hud.ui_ingame_button_small_stone}")`);
  if(hud.ui_ingame_top_bkg_stone)document.getElementById('top').style.setProperty('--ee2-panel',`url("${hud.ui_ingame_top_bkg_stone}")`);
  if(hud.ui_fe_bkg_main)document.getElementById('menu').style.backgroundImage=`linear-gradient(#0001,#0003),url("${hud.ui_fe_bkg_main}")`;
  if(hud.EE2_logo){
    const heading=document.querySelector('#menu h1'),logo=document.createElement('img');
    logo.src=hud.EE2_logo;logo.alt='Empire Earth II';logo.className='ee2-logo';heading.replaceChildren(logo);
  }
  for(const [i,key] of ['icon_resource_food','icon_resource_wood','icon_resource_iron'].entries()){
    if(!hud[key])continue;const dot=document.getElementById(`r${i}`).parentElement.querySelector('.dot');
    if(dot){const image=document.createElement('img');image.src=hud[key];image.className='resource-icon';image.alt='';dot.replaceWith(image);}
  }
  if(hud.icon_resource_tech)document.getElementById('science').style.cssText=`background:url("${hud.icon_resource_tech}") no-repeat left center/18px;padding-left:23px;`;
}
