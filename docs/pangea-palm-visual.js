const MEDIUM_SIZE=160;
const MEDIUM_SEED=1109694050;
const TREE_SCALE=0.03181980550289154;
const ROOT='local-original/pangea-palm115/';
const TROPIC2=Object.freeze([
  Object.freeze({species:'TRE_PALM',weight:10,sourceSha:'b7911501cb63613f7673ef8561ea02d33559a2d931fc9f27f2b20f7b7fb3f033'}),
  Object.freeze({species:'TRE_PALM_DARK',weight:5,sourceSha:'486d790a5baddb15a794c19f4958ad9c8c1bd467233ba84e06c484f29fa28d85'}),
  Object.freeze({species:'TRE_PALMETTO',weight:7,sourceSha:'675dd68980683f10b4402913ac7c7142934cb7d48ae3963ee08219b676fafe1e'}),
]);
const totalWeight=TROPIC2.reduce((sum,row)=>sum+row.weight,0);
const next=state=>(Math.imul(state,1664525)+1013904223)>>>0;
const digest=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
const privateURL=value=>typeof value==='string'&&value.startsWith(ROOT)&&/^[a-z0-9_./-]+$/.test(value)&&!value.includes('..');

export function createPangeaPalmSelector(){
  // PG45 already owns these resource tiles. Sample the source brush in tile
  // order, with a private LCG, so viewport, picking and load order cannot move
  // or reroll a tree. The original forest-placement mask is a separate stage.
  const speciesByTile=new Uint8Array(MEDIUM_SIZE*MEDIUM_SIZE);
  let state=MEDIUM_SEED;
  for(let tile=0;tile<speciesByTile.length;tile++){
    state=next(next(next(state)));
    const roll=((state>>>16)&0x7fff)%totalWeight;
    speciesByTile[tile]=roll<10?0:roll<15?1:2;
  }
  return (x,z)=>{
    const column=Math.floor(x),row=Math.floor(z);
    if(!(column>=0&&column<MEDIUM_SIZE&&row>=0&&row<MEDIUM_SIZE))return -1;
    return speciesByTile[row*MEDIUM_SIZE+column];
  };
}

export async function loadPangeaPalmPresentation({enabled,active,fetcher=fetch}){
  if(!enabled)return null;
  try{
    const response=await fetcher(new URL(ROOT+'runtime-manifest.json',import.meta.url),{cache:'no-store'});
    if(!response.ok)return null;
    const pack=await response.json();
    if(pack?.format!=='openepoch-pangea-palm-visual-v1'||pack.sourceScale!==TREE_SCALE||
      !Array.isArray(pack.rows)||pack.rows.length!==TROPIC2.length)return null;
    const models=pack.rows.map((row,index)=>{
      const source=TROPIC2[index];
      if(row.species!==source.species||row.sourceSha256!==source.sourceSha||!privateURL(row.url)||!digest(row.sha256)||
        !Array.isArray(row.textures)||row.textures.length!==1||
        !row.textures.every(texture=>privateURL(texture.url)&&digest(texture.sha256)))throw Error('Ungültiges Palmenmodell');
      return Object.freeze({...row,sourceScale:TREE_SCALE});
    });
    return Object.freeze({active,select:createPangeaPalmSelector(),models:Object.freeze(models)});
  }catch{return null;}
}
