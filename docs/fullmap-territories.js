import {sampleFullmapTiles} from './fullmap-terrain.js';

// Installed fullmap colors for the current four player slots and neutral land.
const OWNER_COLORS = [[84,142,213],[194,69,69],[54,183,72],[255,255,106]];
const NEUTRAL = [25,25,25];
const BORDER = [90,70,30];
const RASTER = 512;
const MAX_RAY = 20;

export function fullmapTerritoryAlpha(ids,width) {
  const nearest=new Uint8Array(ids.length).fill(MAX_RAY+1);
  const ray=new Uint8Array(ids.length);
  // The installed bitmap walk compares linear indices, including row crossings.
  for(const offset of [-1,1,-width,width,-width-1,width+1,1-width,width-1]) {
    const start=offset<0?0:ids.length-1, end=offset<0?ids.length:-1, step=offset<0?1:-1;
    for(let index=start;index!==end;index+=step){
      const next=index+offset;
      const run=ids[next]===ids[index]?Math.min(MAX_RAY+1,ray[next]+1):1;
      ray[index]=run;
      if(run<nearest[index]) nearest[index]=run;
    }
  }
  return nearest.map((distance,index)=>ids[index]===255?0:distance===1?255:distance>MAX_RAY?62:264-10*distance);
}

export function composeFullmapTerritories(tiles,heights,terrain,size) {
  const sample=sampleFullmapTiles(heights,terrain,size,size);
  const ids=new Uint8Array(RASTER*RASTER),owners=new Uint8Array(ids.length);
  for(let y=0;y<RASTER;y++)for(let x=0;x<RASTER;x++){
    const pixel=y*RASTER+x;
    const source=(Math.trunc(y*(size-1)/RASTER)*size+Math.trunc(x*(size-1)/RASTER))*4;
    ids[pixel]=sample(x,y).height<0?255:tiles[source];
    owners[pixel]=tiles[source+1];
  }
  const alpha=fullmapTerritoryAlpha(ids,RASTER);
  const rgba=new Uint8ClampedArray(alpha.length*4);
  for(let i=0;i<alpha.length;i++){
    if(alpha[i]===0)continue;
    const color=alpha[i]===255?BORDER:OWNER_COLORS[owners[i]-1]??NEUTRAL;
    const offset=i*4;
    rgba[offset]=color[0];rgba[offset+1]=color[1];rgba[offset+2]=color[2];rgba[offset+3]=alpha[i];
  }
  return rgba;
}
