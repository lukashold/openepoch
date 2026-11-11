import {loadAnimationPayload} from './original-animation-payload.js?v=citizen9';

const basePath='local-original/death-valley2-hunt-attack/';
const keyframeSha='78fd68760031e812df3d2344a02b3fcdc3c68779b417119f1df9783f670c6029';
const modelPath=/^local-original\/citizens9\/models\/citizen-[01]-e(0|3|6|9|12)-r[0-3]-(near|far)\.json$/;
const sha256=/^[a-f0-9]{64}$/;
const frameCount=24, duration=1.6666667461395264;

export async function loadOriginalHuntPoses(manifestSha,{base=import.meta.url,fetchImpl=globalThis.fetch,cryptoImpl=globalThis.crypto}={}){
  if(!sha256.test(manifestSha))throw Error('Invalid Hunt Attack1 manifest digest');
  const response=await fetchImpl(new URL('./'+basePath+'manifest.json',base),{cache:'force-cache'});
  if(!response.ok)throw Error('Original Hunt Attack1 manifest unavailable');
  const bytes=await response.arrayBuffer();
  const digest=await cryptoImpl.subtle.digest('SHA-256',bytes);
  const hex=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
  if(hex!==manifestSha)throw Error('Original Hunt Attack1 manifest SHA differs');
  const source=JSON.parse(new TextDecoder().decode(bytes));
  if(source.format!=='ee2-private-citizen-attack-root0-v1'||source.source?.kf!==keyframeSha||source.source.root!==0
    ||source.frames!==frameCount||source.duration!==duration||Object.keys(source.models??{}).length!==80){
    throw Error('Original Hunt Attack1 manifest source differs');
  }
  const entries=new Map();
  for(const [url,row] of Object.entries(source.models)){
    const spec=row.payload;
    if(!modelPath.test(url)||!sha256.test(row.modelSha256)||!sha256.test(spec?.sourceSha256)||!sha256.test(spec.sha256)
      ||spec.url!==basePath+spec.sha256.slice(0,24)+'.bin'||!Array.isArray(row.nodes)||!row.nodes.length
      ||!row.nodes.every(([id,len])=>Number.isInteger(id)&&id>=0&&Number.isInteger(len)&&len>=3&&len%3===0)
      ||new Set(row.nodes.map(([id])=>id)).size!==row.nodes.length
      ||spec.floatCount!==row.nodes.reduce((count,[,len])=>count+len*2*frameCount,0)){
      throw Error('Original Hunt Attack1 model binding differs');
    }
    entries.set(url,row);
  }
  return {
    modelHash:url=>entries.get(url)?.modelSha256,
    async attach(model,url){
      const row=entries.get(url);
      if(!row)return model;
      if(model.source?.sha256!==row.payload.sourceSha256||model.parts?.length!==row.nodes.length
        ||!model.parts.every((part,index)=>part.sourceNode===row.nodes[index][0]&&part.positions?.length===row.nodes[index][1])){
        throw Error('Original Hunt Attack1 model topology differs');
      }
      let offset=0;
      const parts=model.parts.map(part=>{
        const length=part.positions.length;
        const frames=Array.from({length:frameCount},()=>{
          const frame={positionsOffset:offset,normalsOffset:offset+length};offset+=2*length;return frame;
        });
        return {...part,animations:{huntCombat:{duration,frames,endpointIncluded:true}}};
      });
      const samples=await loadAnimationPayload({...model,parts,animationPayload:row.payload},{base,fetchImpl,cryptoImpl});
      return {...model,parts:model.parts.map((part,index)=>({...part,
        animations:{...part.animations,huntCombat:samples.parts[index].animations.huntCombat},
      }))};
    },
  };
}
