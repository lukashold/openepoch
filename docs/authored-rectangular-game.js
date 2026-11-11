import {prepareAuthoredRectangularView} from './authored-rectangular-view.js';
import {loadAuthoredCliffTerrain} from './authored-cliff-terrain.js?v=death-valley2-water240';
import {loadAuthoredCliffLightmap} from './authored-cliff-lightmap.js';
import {activeRectangularLayout,rectangularCliffFaces,rectangularBlockBounds,rectangularVisibleBlocks} from './authored-rectangular-layout.js';

const prefix='local-original/death-valley2-terrain/';
const files=Object.freeze({map:[prefix+'world.w8in','bad5aaa92c694ace5c467cb7c460c87c2ede4706d95a958c96368e006ad28b3e'],
  population:[prefix+'world0.dvp1','78d729c50ddc80fe833f2f66b69f6a822060bfbe068bf10ce8320a264750798e'],
  animals:[prefix+'animal-manager.anm','4c85b954463e8a7b468393d7d22b660a7405947501893bef5f7ad329a7612dcc']});

export const RECTANGULAR_GAME_EXPORTS=Object.freeze(['active_map_layout','native_rect_alloc','native_rect_prepare',
  'native_rect_info','native_rect_ptr','native_rect_adjacent','native_rect_start','native_rect_commit','native_rect_focus',
  'native_rect_population_alloc','native_rect_population_prepare']);

export function isRectangularSave(bytes){
  return bytes instanceof Uint8Array && bytes.length>=4 && bytes[0]===68&&bytes[1]===86&&bytes[2]===83&&bytes[3]===49;
}

function needsAnimalManager(bytes){
  if(!isRectangularSave(bytes)||bytes.length<8)return false;
  const version=new DataView(bytes.buffer,bytes.byteOffset,8).getUint32(4,true);
  return version>=7&&version<=21;
}

export async function readRectangularGameMedia(read=async (name,optional)=>{
  const response=await fetch(new URL(name,import.meta.url),{cache:'no-store'});
  if(optional&&response.status===404)return null;
  if(!response.ok)throw Error('Lokale Originalkarte fehlt.');
  return new Uint8Array(await response.arrayBuffer());
}) {
  const checked=async ([url,pin],optional=false)=>{
    const bytes=await read(url,optional);
    if(optional&&bytes===null)return null;
    if(!(bytes instanceof Uint8Array))throw Error('Ungültige Daten der lokalen Originalkarte.');
    const actual=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),n=>n.toString(16).padStart(2,'0')).join('');
    if(actual!==pin)throw Error('Ungültige Daten der lokalen Originalkarte.');
    return bytes;
  };
  const [map,population,animals]=await Promise.all([checked(files.map),checked(files.population),
    checked(files.animals,true).catch(error=>{console.warn('Originale Tierbewegung Tal des Todes 2:',error.message);return null;})]);
  return {map,population,animals};
}

export function prepareRectangularGameInputs(wasm,media,save){
  const canStageAnimals=typeof wasm.native_rect_animal_alloc==='function'&&typeof wasm.native_rect_animal_prepare==='function';
  if(needsAnimalManager(save)&&(!canStageAnimals||!media.animals))return false;
  const stage=(bytes,alloc,apply)=>{
    const ptr=alloc(bytes.length);
    if(!ptr)return false;
    new Uint8Array(wasm.memory.buffer,ptr,bytes.length).set(bytes);
    return apply()===1;
  };
  return stage(media.map,wasm.native_rect_alloc,wasm.native_rect_prepare)&&
    stage(media.population,wasm.native_rect_population_alloc,wasm.native_rect_population_prepare)&&
    (!canStageAnimals||!media.animals||stage(media.animals,wasm.native_rect_animal_alloc,wasm.native_rect_animal_prepare));
}

export async function loadRectangularPresentation(wasm,gl,map,read){
  const geometry=prepareAuthoredRectangularView(wasm,map);
  if(!geometry)return null;
  const faces=rectangularCliffFaces(map,geometry),world={...geometry,faces,rectangular:true};
  const bounds=rectangularBlockBounds(world);
  const terrain=await loadAuthoredCliffTerrain(gl,{...geometry,backing:512},{detail:2,read});
  if(!terrain)return null;
  let lightmaps;
  try{
    lightmaps=await loadAuthoredCliffLightmap(gl,geometry,read);
    if(!lightmaps)throw Error('Lokale Klippenbeleuchtung fehlt.');
  }catch(error){terrain.dispose();throw error;}
  let visibleBlocks=[];
  return {
    world,
    drawTerrain(vp,lighting,{clipPlane=null,fogTexture=null,fogSize}={}){
      const blocks=rectangularVisibleBlocks(bounds,vp);
      if(!clipPlane)visibleBlocks=blocks;
      // The authored camera mirrors the generic renderer's Z plane.
      terrain.draw(vp,fogTexture,{visibleBlocks:blocks,deferFog:!clipPlane,reverseCull:true,clipPlane,fogSize});
      const lightmap=lightmaps.select(lighting);
      if(lightmap)terrain.drawLightmap(vp,{...lightmap,visibleBlocks:blocks,reverseCull:true,clipPlane});
    },
    drawLateFog(fog,vp,quality){return terrain.drawLateFog(fog,vp,quality,{visibleBlocks});},
    get metrics(){return {...terrain.metrics,visibleBlocks:visibleBlocks.length};},
  };
}

export async function loadRectangularGame(wasm,gl,media){
  if(!media||RECTANGULAR_GAME_EXPORTS.some(name=>typeof wasm[name]!=='function'))return null;
  const presentation=await loadRectangularPresentation(wasm,gl,media.map);
  if(!presentation)return null;

  const prepare=save=>prepareRectangularGameInputs(wasm,media,save);

  return {
    prepare,
    start(seed,ai,adaptive=false){
      const commit=adaptive?wasm.native_rect_commit_adaptive:wasm.native_rect_commit;
      return prepare() && commit(seed,ai)===1;
    },
    focus(){return [wasm.native_rect_focus(0),wasm.native_rect_focus(1)];},
    world(){
      const layout=activeRectangularLayout(wasm);
      if(!layout)return null;
      return {...presentation.world,...layout};
    },
    drawTerrain:presentation.drawTerrain,
    drawLateFog:presentation.drawLateFog,
    get metrics(){return presentation.metrics;},
  };
}
