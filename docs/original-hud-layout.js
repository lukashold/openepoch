// Native GWidget construction: source corner + scaled signed offsets, then
// target corner placement. Two anchors constrain dimensions on differing axes.
// Source: GDbWidgetDefs.h and Ghidra 00A70B00; frame inset is an explicit input.
export function anchorFlags(value=''){
  if(!value||value==='NoAnchor')return 0;
  const match=/^Source(UL|LL|UR|LR)Target(UL|LL|UR|LR)$/.exec(value);
  if(!match)throw Error('Unknown native anchor '+value);
  return 1+(match[1][0]==='L'?2:0)+
    (match[1][1]==='R'?4:0)+(match[2][0]==='L'?8:0)+(match[2][1]==='R'?16:0);
}
export function scaledInt(value,pixels,reference,dimension=false){
  value=Number(value||0);const result=Math.trunc(value*pixels/reference);
  return dimension&&result===0?Math.sign(value):result;
}
export function nativeHudFrame(manifest,row,width,meta=1){
  const name=row?.FRAMESTYLE,base=manifest.frameStyles?.[name];
  const def=manifest.frameStyles?.[name+(meta===1?'':`_e${meta}`)]||base;
  if(!def)return {outer:0,inner:0,definition:null};
  const widths=manifest.resolutionWidths||[800,1366,1600,1920,2048,2560];
  let slot=0;for(let index=1;index<widths.length;index++)if(Math.abs(width-widths[index])<Math.abs(width-widths[slot]))slot=index;
  const sizes=(def['WIDTH[]'].match(/-?\d+/g)||[]).map(Number),size=sizes[slot]||0;
  const outer=def.POSITION==='Inner'?0:def.POSITION==='Middle'?Math.trunc(size/2):size;
  return {outer,inner:size-outer,definition:def};
}
export function nativeHudRect(row,source1,source2,width,height,inset=0){
  const raw1=anchorFlags(row.ANCHORTYPE1),raw2=anchorFlags(row.ANCHORTYPE2);
  let first=raw1;
  if(first===0){if(+row.XOFF<0)first=0x14;if(+row.YOFF<0)first|=0x0a;}
  const point=(source,flags,x,y)=>[
    source.x+(flags&4?source.width:0)+scaledInt(x,width,1600),
    source.y+(flags&2?source.height:0)+scaledInt(y,height,1200)];
  const p1=point(source1,first,row.XOFF,row.YOFF);
  let w=scaledInt(row.WIDTH,width,1600,true),h=scaledInt(row.HEIGHT,height,1200,true);
  if(raw2){
    const p2=point(source2,raw2,row['XOFF ANCHOR2'],row['YOFF ANCHOR2']);
    if((raw1^raw2)&16)w=Math.abs(p2[0]-p1[0])-2*inset;
    if((raw1^raw2)&8)h=Math.abs(p2[1]-p1[1])-2*inset;
  }
  if(w<0||h<0)throw Error('Unconstrained native widget '+row.NAME);
  return {x:p1[0]+(first&16?-inset-w:inset),y:p1[1]+(first&8?-inset-h:inset),width:w,height:h};
}
export function calculateHudLayout(manifest,width,height){
  if(!(width>0&&height>0&&Number.isFinite(width+height)))throw Error('Invalid HUD viewport');
  width=Math.trunc(width);height=Math.trunc(height);
  const widgets=manifest.widgets,result=new Map(),visiting=new Set();
  const screen={x:0,y:0,width:width-1,height:height-1};
  function rect(name){
    if(!name)return screen;
    if(result.has(name))return result.get(name);
    if(visiting.has(name))throw Error('Cyclic HUD anchor '+name);
    const row=widgets[name];if(!row)throw Error('Missing HUD widget '+name);
    visiting.add(name);
    const ancestor=source=>{for(let parent=row.PARENT;parent;parent=widgets[parent]?.PARENT)if(parent===source)return true;return false;};
    const bounds=source=>{
      if(!source)return screen;
      const r=rect(source),frame=nativeHudFrame(manifest,widgets[source],width);
      // 00A74D10 chooses inner bounds for ancestors, outer for siblings.
      const inset=ancestor(source)?frame.inner:-frame.outer;
      return {x:r.x+inset,y:r.y+inset,width:r.width-2*inset,height:r.height-2*inset};
    };
    const source1=bounds(row.ANCHORNAME1||row.PARENT);
    const source2=bounds(row.ANCHORNAME2||row.PARENT);
    const value=nativeHudRect(row,source1,source2,width,height,nativeHudFrame(manifest,row,width).outer);
    result.set(name,value);visiting.delete(name);return value;
  }
  for(const name of Object.keys(widgets))rect(name);
  return result;
}
export function nativeMetaEpoch(epoch){return epoch<5?1:epoch<10?2:3;}
