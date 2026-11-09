import {cityCenterShadowModels,cityCenterWorldVolume} from './citycenter-shadow-mesh.js?v=house156';
import {createShadowVolumeBuffer} from './valley-ruin-shadow.js';

const catalogs=[
  {url:'local-original/building-shadows/citycenter.json',format:'ee2-private-citycenter-topology-v1',
    sha256:'0e87571f2bb5fcf5b7a394366f4107f3fb38a421406cc73c6b2146ff404a7b35'},
  {url:'local-original/building-shadows/house.json',format:'ee2-private-house-topology-v1',
    sha256:'f7a7f1caa85a6f1051fcff5a19894847d307800baefeb563130e020eaf4a0062'},
];

export async function loadBuildingShadowAssets(checked,lighting) {
  if (!lighting?.native || lighting.climate!=='Temperate' || !lighting.sourceTerrainDirection) return null;
  const results=await Promise.allSettled(catalogs.map(async catalog=>{
    const manifest=JSON.parse(new TextDecoder().decode(await checked(catalog)));
    return cityCenterShadowModels(manifest,await checked(manifest.payload),catalog.format);
  }));
  const models=new Map(results.flatMap(result=>result.status==='fulfilled'?[...result.value]:[]));
  if(!models.size)return null;
  const verified=new Map();
  function modelFor(url,{source,scale}) {
    const model=models.get(url);
    if (!model || model.source!==source || model.scale!==scale) return null;
    if (!verified.has(url)) {
      verified.set(url,false);
      checked({url,sha256:model.sha256}).then(()=>verified.set(url,true)).catch(()=>{});
    }
    return verified.get(url) ? model : null;
  }
  return caster=>{
    if(!caster.modelParts)return modelFor(caster.url,caster);
    const parts=[];
    for(const {url,node} of caster.modelParts){
      const part=modelFor(url,caster)?.parts.find(candidate=>candidate.node===node);
      if(!part)return null;
      parts.push(part);
    }
    return {scale:caster.scale,parts};
  };
}

export function createBuildingShadowRenderer(gl,match,sourceLight,compile) {
  const buffer=createShadowVolumeBuffer(gl,compile);
  let signature=null, ranges=[], active=[];
  return {
    prepare(casters) {
      const selected=[];
      for (const caster of casters) {
        const model=match(caster);
        if(model) selected.push({caster,model});
      }
      const key=JSON.stringify(selected.map(({caster})=>[caster.url,caster.modelParts,...caster.pose]));
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
