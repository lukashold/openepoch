const folder='local-original/scout-idle/';
const manifestSha='c41f58c38792e9554a723dddc29a00912edeee8ad6d6e4b6bbca5160e860d45c';
const payloadName='scout-idle-primary-v1.c9a1';

async function verifiedBytes(url,expected,fetchFile) {
  const response=await fetchFile(url);
  if(!response.ok)throw Error('Scout idle animation unavailable');
  const bytes=await response.arrayBuffer();
  const hash=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if(Array.from(hash,value=>value.toString(16).padStart(2,'0')).join('')!==expected)throw Error('Scout idle checksum differs');
  return bytes;
}

export async function loadOriginalScoutIdle({attachAnimationPayload,fetchFile=globalThis.fetch,base=import.meta.url}) {
  try {
    const meta=JSON.parse(new TextDecoder().decode(await verifiedBytes(new URL(folder+'primary-v1.json',base),manifestSha,fetchFile)));
    const spec=meta.animationPayload;
    if(meta.format!=='openepoch-scout-idle-primary-v1'||spec.url!==folder+payloadName) return null;
    const bytes=await verifiedBytes(new URL(spec.url,base),spec.sha256,fetchFile);
    let samples;
    return model=>{
      if(model.source?.sha256!==meta.source.sha256||JSON.stringify(model.normalization)!==JSON.stringify(meta.normalization)||
          model.parts?.length!==meta.parts.length) return model;
      const compatible=model.parts.every((part,index)=>{
        const source=meta.parts[index];
        return part.sourceNode===source.sourceNode&&part.positions?.length===source.vertices*3&&
          part.indices?.length===source.indices&&part.animations?.idle?.frames?.length>0;
      });
      if(!compatible) return model;
      try {
        if(!samples){
          const parts=model.parts.map((part,index)=>({positions:part.positions,animations:meta.parts[index].animations}));
          samples=attachAnimationPayload({source:model.source,parts,animationPayload:spec},bytes).parts.map(part=>part.animations.idle);
        }
        return {...model,parts:model.parts.map((part,index)=>({...part,animations:{...part.animations,idle:samples[index]}}))};
      } catch {
        return model;
      }
    };
  } catch {
    return null;
  }
}
