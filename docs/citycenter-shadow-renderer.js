import {cityCenterShadowModels,cityCenterWorldVolume} from './citycenter-shadow-mesh.js';
import {createShadowVolumeBuffer} from './valley-ruin-shadow.js';

const catalog={url:'local-original/building-shadows/citycenter.json',
  sha256:'0e87571f2bb5fcf5b7a394366f4107f3fb38a421406cc73c6b2146ff404a7b35'};

export async function loadCityCenterShadowAssets(checked,lighting) {
  if (!lighting?.native || lighting.climate!=='Temperate' || !lighting.sourceTerrainDirection) return null;
  const manifest=JSON.parse(new TextDecoder().decode(await checked(catalog)));
  const models=cityCenterShadowModels(manifest,await checked(manifest.payload));
  const verified=new Map();
  return caster=>{
    const model=models.get(caster.url);
    if (!model || model.source!==caster.source || model.scale!==caster.scale) return null;
    if (!verified.has(caster.url)) {
      verified.set(caster.url,false);
      checked({url:caster.url,sha256:model.sha256}).then(()=>verified.set(caster.url,true)).catch(()=>{});
    }
    return verified.get(caster.url) ? model : null;
  };
}

export function createCityCenterShadowRenderer(gl,match,sourceLight,compile) {
  const buffer=createShadowVolumeBuffer(gl,compile);
  let signature=null, ranges=[], active=[];
  return {
    prepare(casters) {
      const selected=[];
      for (const caster of casters) {
        const model=match(caster);
        if(model) selected.push({caster,model});
      }
      const key=JSON.stringify(selected.map(({caster})=>[caster.url,...caster.pose]));
      if (key!==signature) {
        const nextRanges=[], nextActive=[], points=[], indices=[];
        for(const [index,{caster,model}] of selected.entries()) {
          const volume=cityCenterWorldVolume(model,caster.pose,sourceLight);
          if(!volume.indices.length) continue;
          const base=points.length/3;
          if(base+volume.positions.length/3>1_048_576) break;
          const first=indices.length;
          for(const id of volume.indices) indices.push(base+id);
          for(const value of volume.positions) points.push(value);
          nextRanges.push({indexStart:first,indexCount:volume.indices.length});
          nextActive.push(index);
        }
        if (nextRanges.length) buffer.upload(Float32Array.from(points),Uint32Array.from(indices),gl.DYNAMIC_DRAW);
        signature=key;
        ranges=nextRanges;
        active=nextActive;
      }
      return new Set(active.map(index=>selected[index].caster.instance));
    },
    draw(vp) { if(ranges.length) buffer.draw(vp,ranges); },
  };
}
