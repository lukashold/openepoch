// UnitInfo_StatusIconsTray allocates 11 cells, and 00790891 keeps a gap
// between the positive and non-positive fronts before binding each badge.
const slotCount=11;
const none=0xffffffff;
const manifestPath='local-original/crown-status/manifest.json';
const manifestHash='3bdae0f3be5dbafec51a63712a4ec36bf78c8b16cbfb5bf3f0ac37253e18c0eb';

export async function loadCrownStatusArt({enabled,fetchFile=globalThis.fetch,base=new URL('./',import.meta.url)}={}){
 if(!enabled)return null;
 try{
  const response=await fetchFile(new URL(manifestPath,base));
  if(!response.ok)return null;
  const bytes=await response.arrayBuffer();
  const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if(Array.from(digest,value=>value.toString(16).padStart(2,'0')).join('')!==manifestHash)return null;
  const manifest=JSON.parse(new TextDecoder().decode(bytes));
  if(manifest.format!=='ee2-native-crown-status-v1'||manifest.rows.length!==38)return null;
  function asset(record){
   if(!record)return null;
   const url=new URL(record.url,base);
   url.searchParams.set('v',record.sha256.slice(0,16));
   return url.href;
  }
  return Object.freeze({rows:manifest.rows,
   badge:(row,packed=7)=>asset(manifest.profiles['packed'+packed]?.icons[row?.icon]),
   background:(theme,packed=7)=>asset(manifest.profiles['packed'+packed]?.backgrounds[theme]),
   texture:kind=>asset(manifest.worldTextures[kind]),
  });
 }catch{return null;}
}

// The engine supplies viewer-admitted, currently active effect rows for the
// exact generational actor handle. Foreign status-only Piracy is intentional.
export function readCrownActorVisuals(wasm,rows,{handle,owner,viewer}={}){
 const activeRows=[];
 let good=false,bad=false;
 if(typeof wasm?.crown_actor_visual!=='function'||!Number.isInteger(handle)||handle<0||handle>=none||
    !Number.isInteger(owner)||owner<0||!Number.isInteger(viewer)||viewer<0)return {activeRows,good,bad};
 for(let kind=0;kind<3;kind++){
  const id=(wasm.crown_actor_visual(handle,kind,0)>>>0)-1,row=rows[id];
  if(row?.id!==id||row.branch!==kind||row.ownerOnly&&owner!==viewer)continue;
  activeRows.push(id);
  if(row.suppressGlow)continue;
  const flag=wasm.crown_actor_visual(handle,kind,1)>>>0;
  good ||= flag===1&&row.kind==='good';
  bad ||= flag===2&&row.kind==='bad';
 }
 return {activeRows,good,bad};
}

export function crownStatusSlots(rows,activeRows,owner,viewer){
 const slots=Array(slotCount).fill(null);
 let left=0,right=slotCount-1;
 for(const id of activeRows){
  const row=rows[id];
  if(row?.id!==id||row.ownerOnly&&owner!==viewer)continue;
  const index=row.kind==='good'?left++:right--;
  if(left>right)break;
  slots[index]=row;
 }
 return slots;
}

// Width and height are the already scaled native widget measurements.
// 0078BE85 scales the two-pixel left/right margin with the viewport width.
export function crownStatusRects({viewportWidth,trayWidth,trayHeight,iconWidth,iconHeight}){
 const round=value=>Math.floor(value+.5);
 const margin=round(2*viewportWidth*Math.fround(1/1600));
 const step=Math.fround(iconWidth+(trayWidth-2*margin-iconWidth*slotCount)*Math.fround(.1));
 const y=round((trayHeight-iconHeight)*.5),rects=[];
 let x=margin;
 for(let i=0;i<slotCount;i++){
  rects.push({x:round(x),y,width:iconWidth,height:iconHeight});
  x=Math.fround(x+step);
 }
 return rects;
}

export function crownHasWorldGlow(rows,activeRows,owner,viewer){
 return activeRows.some(id=>{
  const row=rows[id];
  return row?.id===id&&row.kind==='good'&&!row.suppressGlow&&(!row.ownerOnly||owner===viewer);
 });
}

// 00652A17..00652B53 submits Neutral and King independently, then one
// primary color. A source Crown contributes to Good, as does Bless.
export function visualGlowKinds({good=false,bad=false,neutral=false,king=false}){
 const kinds=[];
 if(neutral)kinds.push('neutral');
 if(king)kinds.push('king');
 if(good||bad)kinds.push(good&&bad?'mixed':good?'good':'bad');
 return kinds;
}
