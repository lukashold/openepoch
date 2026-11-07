// Membership is authoritative, sampled for this visible actor snapshot.
// This uses the same source global clock as the verified capture ground pass.
const TAU=Math.fround(2*Math.PI),MIN=Math.fround(.95),MAX=Math.fround(1.15);
const INITIAL_PHASE=Math.fround(1.5*Math.PI);
const CATALOG_SHA='a1ea39d81b2d58677184dc55c1722b5f3b46d1ba0ba683db8da3053bc006bae1';
const CROWN_CATALOG_SHA='11879f5dd3089cf9fe632c42764cc3545a3022921fddf96dd47a432967efa1ac';

export async function loadBlessGroundGlow({enabled=false,crowns=false,fetchFile=globalThis.fetch,
 url=new URL('./local-original/priest-fx/bless-ground-catalog.json',import.meta.url)}={}){
 if(!enabled)return null;
 const sources=[[url,CATALOG_SHA]];
 if(crowns)sources.unshift([new URL('./local-original/crown-status/world-glow.json',import.meta.url),CROWN_CATALOG_SHA]);
 for(const [path,sha] of sources){
  try{
   const response=await fetchFile(path);
   if(!response.ok)continue;
   const bytes=await response.arrayBuffer();
   const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
   if(Array.from(digest,byte=>byte.toString(16).padStart(2,'0')).join('')!==sha)continue;
   const glow=createBlessGroundGlow(JSON.parse(new TextDecoder().decode(bytes)));
   if(glow)return glow;
  }catch{continue;}
 }
 return null;
}

export function createBlessGroundGlow(source){
 if(source?.format!=='ee2-native-bless-ground-source-v1')return null;
 const templates=new Map(source.templates.map(row=>[row.name,row]));
 const scales=new Map(Object.entries(source.singleTemplateBySourceVisual).flatMap(([visual,binding])=>{
  const template=templates.get(binding.template);
  return template?[[visual,template]]:[];
 }));
 let phase=INITIAL_PHASE;
 return {
  reset(){phase=INITIAL_PHASE;},
  get phase(){return phase;},
  isAir(visual){return scales.get(visual)?.geometry===true;},
  isGround(visual){return scales.get(visual)?.geometry===false;},
  modelVisual(model,url){
   const binding=model?.databaseVisualBinding;
   const visual=binding?.visualDefinition??binding?.sourceVisual;
   if(visual)return scales.has(visual)?visual:null;
   const body=source.sourceBodies?.[model?.source?.sha256];
   if(scales.has(body))return body;
   const legacy=source.legacyModels?.[url];
   return legacy?.[0]===model?.source?.sha256&&scales.has(legacy?.[1])?legacy[1]:null;
  },
  advance(seconds){phase=Math.fround((phase+Math.fround(seconds)/1.5*TAU)%TAU);},
  project({actors,local,visualForActor,isBlessed,hasPreview,isMixed=()=>false}){
   const ratio=(Math.fround(Math.sin(phase))+1)*.5;
   const radius=Math.fround((MAX-MIN)*ratio+MIN);
   const alpha=Math.trunc(75+75*ratio)/255,good=[],neutral=[],mixed=[];
   for(const actor of actors){
    const scale=scales.get(visualForActor(actor));
    if(!scale||scale.geometry)continue;
    const position=[actor.renderX,actor.renderY,actor.renderYaw];
    if(actor.team===local&&hasPreview(actor.handle)){
     const diameter=2*Math.fround(radius*scale.neutralScale);
     neutral.push(...position,diameter,diameter);
    }
    if(isBlessed(actor.handle)){
     const diameter=2*Math.fround(radius*scale.goodScale);
     const layer=isMixed(actor.handle)?mixed:good;
     layer.push(...position,diameter,diameter);
    }
   }
   return {good,neutral,mixed,alpha};
  },
 };
}
