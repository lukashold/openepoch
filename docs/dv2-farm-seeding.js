const POSE_URL=new URL('./local-original/farm-seeding/poses.json',import.meta.url);
const POSE_SHA256='d6599cae88455a011106d0dec7fd5e509f807b91c4ab14f077a6c7b8005a56c9';
const FAMILY_NAMES=['FarmWheat','FarmSugarCane','FarmYam'];
const EMPTY_TRANSLATION=[0,0,0];

export const FARM_SEEDING_POSITION_GLSL=`vec3 farmSeedingPosition(vec3 base,vec3 delta,float weight,vec3 shift){
 return base+delta*weight+shift;
}`;

function floats(value,length){
 const bytes=Uint8Array.from(atob(value),letter=>letter.charCodeAt(0));
 if(bytes.length!==length*4)throw new Error('Original seeding vertex count differs');
 const view=new DataView(bytes.buffer),result=new Float32Array(length);
 for(let i=0;i<result.length;i++)result[i]=view.getFloat32(i*4,true);
 return result;
}

function trackAt(track,time){
 const keys=track.keys;
 if(time<=keys[0][0])return keys[0][1];
 for(let i=1;i<keys.length;i++){
  const [end,last]=keys[i];
  if(time>end)continue;
  const [start,first]=keys[i-1];
  let fraction=(time-start)/(end-start);
  if(track.type===2)fraction=fraction*fraction*(3-2*fraction);
  return typeof first==='number'?first+(last-first)*fraction:
   first.map((value,j)=>value+(last[j]-value)*fraction);
 }
 return keys.at(-1)[1];
}

export async function loadFarmSeeding(){
 const response=await fetch(POSE_URL);
 if(!response.ok)throw new Error(`Original Farm seeding assets: ${response.status}`);
 const raw=await response.arrayBuffer();
 const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',raw));
 if(Array.from(digest,value=>value.toString(16).padStart(2,'0')).join('')!==POSE_SHA256)
  throw new Error('Original Farm seeding asset SHA-256 differs');
 const source=JSON.parse(new TextDecoder().decode(raw));
 if(source.format!=='ee2-private-farm-seeding-v1'||source.requestSeconds!==8||
    !source.families.every((family,index)=>family.name===FAMILY_NAMES[index])||source.families.length!==FAMILY_NAMES.length)
  throw new Error('Original Farm seeding asset binding differs');
 return source.families.map(family=>{
  const rate=Math.fround(family.kf.seconds/source.requestSeconds);
  const parts=new Map(family.parts.map(row=>{
   const length=row.vertices*3;
   return [row.sourceNode,{
    position:floats(row.position,length),morphDelta:floats(row.morphDelta,length),normal:floats(row.normal,length),
    morph:row.morph,translation:row.translation,color:row.color,
   }];
  }));
  return {name:family.name,meshSha256:family.mesh.sha256,modelSha256:family.mesh.modelSha256,parts,
   sourceSeconds:family.kf.seconds,requestSeconds:source.requestSeconds,rate};
 });
}

export function farmSeedingPose(family,sourceNode,elapsed){
 const part=family?.parts.get(sourceNode);
 if(!part||!Number.isFinite(elapsed)||elapsed<0)return null;
 const time=Math.min(family.sourceSeconds,Math.fround(Math.fround(elapsed)*family.rate));
 return {part,time,weight:trackAt(part.morph,time),
  translation:part.translation.keys.length?trackAt(part.translation,time):EMPTY_TRANSLATION,
  colors:Object.fromEntries(Object.entries(part.color).map(([channel,track])=>[channel,trackAt(track,time)]))};
}
