import {nativeHudFrame,nativeHudRect} from './original-hud-layout.js';

export const PLAYER_TEMPLATE=['diplo_pl_currstat_from','diplo_pl_color_glow','diplo_pl_color','diplo_pl_name','diplo_pl_team','diplo_pl_civ'];
export const RELATION_TEMPLATE=['diplo_dbutton_proposal','diplo_dbutton_inAlliance','diplo_dbutton_neutral','diplo_dbutton_war'];
export const RESOURCE_TEMPLATE=Array.from({length:6},(_,i)=>`diplo_tribres_${i}`);

// Native007B01D0 creates the player template set inside each diplo_base_plN;
// 007B24E0 similarly creates action/resource sets in their declared row holders.
export function instantiateDiplomacyRows(manifest,playerCount=3){
 if(!Number.isInteger(playerCount)||playerCount<0||playerCount>9)throw RangeError('Native diplomacy supports0..9 opponent rows');
 const templates=new Set([...PLAYER_TEMPLATE,...RELATION_TEMPLATE,...RESOURCE_TEMPLATE]);
 const widgets=Object.fromEntries(Object.entries(manifest.widgets).filter(([key])=>!templates.has(key)));
 widgets.diplo_minimap={...widgets.diplo_minimap,PARENT:'diplo_tribterr_pane'};
 function clone(names,parent,prefix){
  const local=new Set(names);
  for(const key of names){
   const source=manifest.widgets[key],row={...source,NAME:prefix+key};
   row.PARENT=local.has(source.PARENT)?prefix+source.PARENT:source.PARENT||parent;
   for(const field of ['ANCHORNAME1','ANCHORNAME2'])if(local.has(source[field]))row[field]=prefix+source[field];
   widgets[row.NAME]=row;
  }
 }
 for(let i=1;i<=playerCount;i++){
  clone(PLAYER_TEMPLATE,`diplo_base_pl${i}`,`player${i}__`);
  clone(RELATION_TEMPLATE,`diplo_dbuttons_pl${i}`,`actions${i}__`);
  clone(RESOURCE_TEMPLATE,`diplo_tribres_pl${i}`,`resources${i}__`);
 }
 return widgets;
}

export function calculateDiplomacyLayout(manifest,width,height,{playerCount=3,meta=1}={}){
 if(!(width>0&&height>0&&Number.isFinite(width+height)))throw RangeError('Invalid diplomacy viewport');
 width=Math.trunc(width);height=Math.trunc(height);
 const widgets=instantiateDiplomacyRows(manifest,playerCount),values=new Map(),visiting=new Set();
 const screen={x:0,y:0,width:width-1,height:height-1};
 function rect(key){
  if(!key)return screen;
  if(values.has(key))return values.get(key);
  if(visiting.has(key))throw Error('Cyclic source diplomacy anchor '+key);
  const row=widgets[key];if(!row)throw Error('Missing source diplomacy widget '+key);
  visiting.add(key);
  const ancestor=source=>{const seen=new Set();for(let parent=row.PARENT;parent;parent=widgets[parent]?.PARENT){if(seen.has(parent))throw Error('Cyclic source parent');seen.add(parent);if(parent===source)return true;}return false;};
  const bounds=source=>{
   if(!source)return screen;
   const r=rect(source),frame=nativeHudFrame(manifest,widgets[source],width,meta),inset=ancestor(source)?frame.inner:-frame.outer;
   return {x:r.x+inset,y:r.y+inset,width:r.width-2*inset,height:r.height-2*inset};
  };
  const value=nativeHudRect(row,bounds(row.ANCHORNAME1||row.PARENT),bounds(row.ANCHORNAME2||row.PARENT),width,height,nativeHudFrame(manifest,row,width,meta).outer);
  // NativeA7F270 parses center; A7F930 sets content left/top to signed-truncated
  // (screenDimension - (right-left))/2. Frame extents remain separate.
  if(!row.PARENT&&/\bcenter\b/.test(row.ARGSTR||'')){value.x=Math.trunc((width-value.width)/2);value.y=Math.trunc((height-value.height)/2);}
  values.set(key,value);visiting.delete(key);return value;
 }
 return {get:rect,widgets,values,screen,width,height,meta};
}

export function sourceButtonSprite(manifest,base,meta=1,state='normal'){
 const themed=meta===1?base:`${base}_e${meta}`;
 const key=manifest.sprites[themed]?themed:base;
 const stateSuffix={normal:[''],hover:['_r','_roll'],pressed:['_p','_press','_on'],pressedHover:['_rp','_rollpress','_roll_press','_on_r','_on_roll','_roll_on']};
 for(const suffix of stateSuffix[state]||['']){
  // Original tables use both state-before-meta and meta-before-state names.
  for(const candidate of [key+suffix,base+suffix+(meta===1?'':`_e${meta}`)])if(manifest.sprites[candidate])return candidate;
 }
 return key;
}

export function sourceMoneyRule(row){
 const parse=(key,fallback)=>Number(new RegExp(`\\b${key}=(-?\\d+)`).exec(row.ARGSTR||'')?.[1]??fallback);
 return {increment:parse('increment',100),shiftIncrement:parse('shiftincrement',500),min:parse('min',0),max:parse('max',9900)};
}

export function nativeDiplomacyTextStyle(manifest,row,width,meta=1,disabled=false){
 const base=row.FONTSTYLE,key=meta===1?base:`${base}_e${meta}`,style=manifest.fontStyles?.[key]||manifest.fontStyles?.[base];
 if(!style)return {};
 const font=manifest.fonts?.[style.FONT],widths=manifest.resolutionWidths||[800,1366,1600,1920,2048,2560];
 let slot=0;for(let i=1;i<widths.length;i++)if(Math.abs(width-widths[i])<Math.abs(width-widths[slot]))slot=i;
 const values=(field)=>(field?.match(/-?\d+/g)||[]).map(Number);
 const color=values(style[disabled?'DISABLE COLOR':'TEXTCOL']),sizes=values(font?.SIZE);
 const families=[...(font?.FONTNAME?.matchAll(/"([^\"]+)"/g)||[])].map(match=>match[1]);
 const just=row.TEXTJUST||'Left';
 return {color:`rgb(${color.slice(0,3).join(',')})`,fontSize:(sizes[slot]||sizes[0]||14)+'px',fontFamily:families.map(name=>`"${name}"`).join(',')||'serif',fontWeight:font?.FLAGS?.includes('Bold')?'bold':'normal',fontVariant:font?.FLAGS?.includes('SmallCaps')?'small-caps':'normal',textAlign:just.startsWith('Right')?'right':just.startsWith('Center')?'center':'left',verticalMiddle:just.endsWith('Mid')};
}

// Native007B01D0 binds each player's color to the strip sprite. Browser canvas
// is the presentation adapter for source RGB modulation; alpha stays original.
export function modulatePlayerStrip(source,color){
 if(source.length%4||color.length!==3||color.some(value=>!Number.isFinite(value)||value<0||value>255))throw RangeError('Invalid player sprite modulation');
 const out=new Uint8ClampedArray(source.length);
 for(let i=0;i<source.length;i+=4){for(let c=0;c<3;c++)out[i+c]=Math.round(source[i+c]*color[c]/255);out[i+3]=source[i+3];}
 return out;
}
