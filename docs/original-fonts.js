// Portable native bitmap fonts. Original cached glyph pixels are never regenerated.
// Native00A995F0/00A99820 provide pair lookup and multiline measured bounds.
// Canvas placement, clipping and sibling overlays are explicit application adapters.
export function tintFontRGBA(source,color){
 if(source.length%4||!Array.isArray(color)||color.length!==3||color.some(v=>!Number.isFinite(v)||v<0||v>255))throw new RangeError('Invalid bitmap color');
 const out=new Uint8ClampedArray(source.length);
 for(let i=0;i<source.length;i+=4){for(let c=0;c<3;c++)out[i+c]=Math.floor(source[i+c]*color[c]/255+.5);out[i+3]=source[i+3];}
 return out;
}
export function bitmapTextLayout(font,text,{missing='reject'}={}){
 if(text===null||text===undefined)return {width:0,height:0,glyphs:[],lines:[],unsupported:[]};
 let x=0,y=0,previous=0,width=0;const glyphs=[],lines=[],unsupported=[];
 const finish=()=>{const lineWidth=x>0?x-font.spacing:x;lines.push({y,width:lineWidth});width=Math.max(width,lineWidth);};
 for(const char of String(text)){
  const cp=char.codePointAt(0);
  if(cp===10){finish();x=0;y+=font.lineHeight;previous=cp;continue;}
  let glyph=font.glyphs[String(cp)];
  if(!glyph){unsupported.push(cp);if(missing==='question')glyph=font.glyphs['63'];}
  if(!glyph){previous=cp;continue;}
  const kern=font.flags&64?(font.kerning[previous+':'+cp]||0):0;x+=kern;
  glyphs.push({cp,char,x,y,glyph});x+=glyph.width+font.spacing;previous=cp;
 }
 finish();return {width,height:y+font.lineHeight,glyphs,lines,unsupported:[...new Set(unsupported)]};
}
export function sourceBitmapStyle(source,widget,width,{disabled=false,highlighted=false,meta=1}={}){
 const row=source.widgets?.[widget]||source.widgets?.[String(widget).split('__').at(-1)],name=row?.FONTSTYLE,style=source.fontStyles?.[name+(meta===1?'':`_e${meta}`)]||source.fontStyles?.[name];if(!style)return null;
 const widths=source.resolutionWidths||[800,1366,1600,1920,2048,2560];let tier=0;
 for(let i=1;i<widths.length;i++)if(Math.abs(width-widths[i])<Math.abs(width-widths[tier]))tier=i;
 const field=disabled?'DISABLE COLOR':highlighted&&+style['HILITE?']?'HICOL':'TEXTCOL',rgba=(style[field]?.match(/\d+/g)||[]).map(Number),color=rgba.slice(0,3);
 if(color.length!==3)throw Error('Missing native font color');
 const just=row.TEXTJUST||'LeftTop',alpha=Math.fround((rgba[3]??255)*Math.fround(1/255));return {font:style.FONT,tier,color,alpha,opacityMode:'source',align:just.startsWith('Right')?'right':just.startsWith('Cent')?'center':'left',vertical:just.endsWith('Mid')?'middle':just.endsWith('Bot')?'bottom':'top'};
}

export async function loadBitmapFonts({manifestURL=new URL('./local-original/fonts-native/bitmap-manifest.json',import.meta.url),fetchFile=globalThis.fetch,loadImage,createCanvas}={}){
 const response=await fetchFile(manifestURL);if(!response.ok)throw Error('Bitmap font manifest unavailable');const manifest=await response.json();
 if(manifest.format!=='ee2-native-bitmap-font-v1')throw Error('Unsupported bitmap font manifest');
 const canvas=createCanvas||(()=>document.createElement('canvas'));
 const getImage=loadImage||(async(url)=>{const r=await fetchFile(url);if(!r.ok)throw Error('Bitmap atlas unavailable');return createImageBitmap(await r.blob());});
 const images=new Map(),tinted=new Map();
 for(const [key,row]of Object.entries(manifest.images)){
  if(!row.file||row.file.includes('\\')||row.file.includes(':')||row.file.startsWith('/')||row.file.split('/').includes('..'))throw Error('Unsafe bitmap atlas URL');
  const image=await getImage(new URL(row.file,manifestURL));if(image.width!==row.width||image.height!==row.height)throw Error('Bitmap atlas dimensions differ');images.set(key,image);
 }
 function font(name,tier=0){const key=manifest.bindings[name]?.[tier],value=manifest.fonts[key];if(!value)throw new RangeError('No declared bitmap font/tier');return value;}
 function image(file,color){
  if(!images.has(file))throw Error('Unknown bitmap atlas');if(color.every(v=>v===255))return images.get(file);
  const key=file+':'+color.join(',');if(tinted.has(key))return tinted.get(key);
  const source=images.get(file),out=canvas();out.width=source.width;out.height=source.height;const ctx=out.getContext('2d');ctx.drawImage(source,0,0);const rgba=ctx.getImageData(0,0,out.width,out.height);rgba.data.set(tintFontRGBA(rgba.data,color));ctx.putImageData(rgba,0,0);tinted.set(key,out);return out;
 }
 function draw(ctx,font,text,{x=0,y=0,width=Infinity,height=Infinity,color=[255,255,255],alpha=1,align='left',vertical='top',clip=true,missing='reject'}={}){
  if(!Number.isFinite(alpha)||alpha<0||alpha>1)throw new RangeError('Invalid bitmap text alpha');
  const layout=bitmapTextLayout(font,text,{missing});if(missing==='reject'&&layout.unsupported.length)return layout;
  const dy=vertical==='middle'?Math.trunc((height-layout.height)/2):vertical==='bottom'?height-layout.height:0;
  ctx.save();try{
   ctx.globalAlpha=(Number.isFinite(ctx.globalAlpha)?ctx.globalAlpha:1)*alpha;
   if(clip&&Number.isFinite(width)&&Number.isFinite(height)){ctx.beginPath();ctx.rect(x,y,width,height);ctx.clip();}
   ctx.imageSmoothingEnabled=false;
   for(const row of layout.glyphs){const line=layout.lines[Math.trunc(row.y/font.lineHeight)],dx=align==='center'?Math.trunc((width-line.width)/2):align==='right'?width-line.width:0,g=row.glyph;ctx.drawImage(image(g.file,color),g.x,g.y,g.width,g.height,x+dx+row.x,y+dy+row.y,g.width,g.height);}
  }finally{ctx.restore();}
  return layout;
 }
 return {manifest,font,draw,images,dispose(){for(const value of images.values())value.close?.();images.clear();tinted.clear();}};
}

// Overlay siblings keep each live button/label node, its accessible text, and
// all listeners intact. Inputs/selects remain native controls. Unsupported text
// retains the existing presentation and is reported instead of losing glyphs.
export function createBitmapTextLayer(fonts,{document:doc=globalThis.document,window:win=globalThis.window,createCanvas=()=>doc.createElement('canvas')}={}){
 const records=new Map();let disposed=false;
 const connected=node=>{if(typeof node.isConnected==='boolean')return node.isConnected;if(typeof doc.contains==='function')return doc.contains(node);for(let p=node;p;p=p.parentElement)if(p===doc.documentElement||p===doc.body)return true;return false;};
 const computed=node=>win.getComputedStyle?.(node)||node.style;
 function visible(element){
  for(let node=element;node;node=node.parentElement){const css=computed(node),opacity=css.opacity===''||css.opacity===undefined?1:Number(css.opacity);if(node.hidden||node.tagName==='DIALOG'&&!node.open||css.display==='none'||['hidden','collapse'].includes(css.visibility)||opacity===0)return false;}
  return true;
 }
 function adopt(element,row){if(!row.applied||element.style.color!=='transparent')row.oldColor=element.style.color;if(!row.applied||element.style.textShadow!=='none')row.oldTextShadow=element.style.textShadow;}
 function release(element,row){if(row.applied){if(element.style.color==='transparent')element.style.color=row.oldColor;if(element.style.textShadow==='none')element.style.textShadow=row.oldTextShadow;row.applied=false;}}
 function attach(element,settings){
  if(disposed||!element?.parentElement||!connected(element)||['INPUT','SELECT','TEXTAREA','OPTION'].includes(element.tagName))return false;
  if(element.querySelector?.('input,select,textarea,button'))return false;
  const initial=typeof settings==='function'?settings(element):settings;
  try{fonts.font(initial?.font,initial?.tier||0);}catch{return false;}
  if(records.has(element))return false;
  const overlay=createCanvas();overlay.className='native-bitmap-text';overlay.setAttribute('aria-hidden','true');overlay.style.pointerEvents='none';overlay.style.position='fixed';overlay.style.margin='0';overlay.style.padding='0';overlay.style.border='0';element.parentElement.append(overlay);
  records.set(element,{overlay,settings,oldColor:element.style.color,oldTextShadow:element.style.textShadow,last:null,unsupported:[],applied:false,error:null,overflow:false});return true;
 }
 function restore(element,row){adopt(element,row);release(element,row);row.overlay.remove();records.delete(element);}
 function update(){
  if(disposed)return [];
  const results=[];
  for(const [element,row]of [...records]){
   if(!connected(element)){restore(element,row);continue;}
   if(row.overlay.parentElement!==element.parentElement)element.parentElement.append(row.overlay);
   adopt(element,row);let config,font,layout,rect;const text=element.textContent||'',elementRect=element.getBoundingClientRect();
   try{config=typeof row.settings==='function'?row.settings(element):row.settings;font=fonts.font(config.font,config.tier||0);layout=bitmapTextLayout(font,text);rect=typeof config.bounds==='function'?config.bounds(element,elementRect):elementRect;row.error=null;}
   catch(error){row.error=error.message;row.overlay.hidden=true;release(element,row);results.push({element,error:row.error});continue;}
   row.unsupported=layout.unsupported;row.overflow=!!config.fallbackOnOverflow&&(layout.width>rect.width||layout.height>rect.height);
   const show=elementRect.width>0&&elementRect.height>0&&rect.width>0&&rect.height>0&&visible(element)&&!layout.unsupported.length&&!row.overflow;row.overlay.hidden=!show;
   if(!show){release(element,row);results.push({element,unsupported:row.unsupported,overflow:row.overflow});continue;}
   const ratio=win.devicePixelRatio||1,width=Math.ceil(rect.width*ratio),height=Math.ceil(rect.height*ratio),stamp=JSON.stringify([text,config,ratio,rect.width,rect.height,width,height]),css=computed(element);
   Object.assign(row.overlay.style,{left:rect.left+'px',top:rect.top+'px',width:rect.width+'px',height:rect.height+'px',zIndex:config.zIndex||'1',opacity:config.opacityMode==='source'?'1':css.opacity||'1',filter:css.filter||'none'});
   if(stamp!==row.last){row.overlay.width=width;row.overlay.height=height;const ctx=row.overlay.getContext('2d');ctx.setTransform(ratio,0,0,ratio,0,0);ctx.clearRect(0,0,rect.width,rect.height);fonts.draw(ctx,font,text,{width:rect.width,height:rect.height,align:'center',vertical:'middle',...config});row.last=stamp;}
   element.style.color='transparent';element.style.textShadow='none';row.applied=true;results.push({element,unsupported:[]});
  }
  return results;
 }
 return {attach,update,detach(element){const row=records.get(element);if(row)restore(element,row);},dispose(){for(const [element,row]of [...records])restore(element,row);disposed=true;},status(){return {elements:records.size,unsupported:[...records.values()].flatMap(row=>row.unsupported),errors:[...records.values()].map(row=>row.error).filter(Boolean),overflow:[...records.values()].filter(row=>row.overflow).length};}};
}
