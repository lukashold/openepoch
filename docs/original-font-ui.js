// Native-font presentation coordinator. It receives existing live nodes from
// the native HUD/diplomacy components and never issues a simulation command.
import {loadBitmapFonts,sourceBitmapStyle,createBitmapTextLayer} from './original-fonts.js';

export async function loadNativeFontUI({enabled=true,hud,diplomacy,document:doc=globalThis.document,window:win=globalThis.window,...fontOptions}={}){
 const off={enabled:false,errors:[],update(){return [];},dispose(){},status(){return {enabled:false};}};
 if(!enabled)return off;
 if(!hud?.fontBindings&&!diplomacy?.fontBindings){off.errors.push('Native text bindings unavailable');return off;}
 let fonts;try{fonts=await loadBitmapFonts(fontOptions);}catch(error){off.errors.push(error.message);return off;}
 const layer=createBitmapTextLayer(fonts,{document:doc,window:win,createCanvas:fontOptions.createCanvas}),bindings=new Map(),ownedStyles=new WeakMap();
 let disposed=false,pending=false,updating=false;const listeners=[],observers=[];
 const tier=()=>{const widths=fonts.manifest.resolutionWidths;let n=0;for(let i=1;i<widths.length;i++)if(Math.abs(win.innerWidth-widths[i])<Math.abs(win.innerWidth-widths[n]))n=i;return n;};
 const connected=node=>{if(typeof node.isConnected==='boolean')return node.isConnected;if(typeof doc.contains==='function')return doc.contains(node);for(let p=node;p;p=p.parentElement)if(p===doc.body||p===doc.documentElement)return true;return false;};
 function resourceBounds(element,row){
  const rect=row.getBoundingClientRect(),css=win.getComputedStyle?.(row)||row.style,icon=row.querySelector('img'),iconRect=icon?.getBoundingClientRect(),leftPadding=parseFloat(css.paddingLeft)||0,rightPadding=parseFloat(css.paddingRight)||0;
  const left=Math.max(rect.left+leftPadding,iconRect?iconRect.left+iconRect.width:rect.left),right=rect.left+rect.width-rightPadding;
  return {left,top:rect.top,width:Math.max(0,right-left),height:rect.height};
 }
 function settings(element){
  const binding=bindings.get(element);
  if(binding.application==='resource')return {font:'fnt_pal_med',tier:tier(),color:[23,26,18],align:'right',vertical:'middle',bounds:()=>resourceBounds(element,binding.row),fallbackOnOverflow:true};
  const source={...binding.source,fontStyles:fonts.manifest.fontStyles};
  return sourceBitmapStyle(source,binding.widget,win.innerWidth,{meta:binding.meta||1,disabled:!!element.disabled,highlighted:element.dataset.hover==='1'});
 }
 function update(){
  if(disposed||updating)return [];updating=true;
  try{
   const wanted=new Map();
   for(const component of [hud,diplomacy]){
    const data=component?.fontBindings?.();if(!data)continue;
    for(const row of data.labels||[])if(row.element&&connected(row.element)&&row.element.textContent)wanted.set(row.element,{...row,source:data.source,meta:data.meta});
    for(const row of data.resources||[])if(row.element&&connected(row.element)&&row.row)wanted.set(row.element,{...row,application:'resource'});
   }
   for(const element of bindings.keys())if(!wanted.has(element)){layer.detach(element);bindings.delete(element);}
   for(const [element,binding]of wanted){const known=bindings.has(element);bindings.set(element,binding);if(!known&&!layer.attach(element,()=>settings(element)))bindings.delete(element);}
   const result=layer.update();for(const element of bindings.keys())ownedStyles.set(element,element.style.cssText);return result;
  }finally{updating=false;}
 }
 function schedule(){if(disposed||pending)return;pending=true;queueMicrotask(()=>{pending=false;if(!disposed)update();});}
 const isOverlay=node=>node?.classList?.contains('native-bitmap-text');
 if(win.MutationObserver){
  const observer=new win.MutationObserver(records=>{
   if(records.some(record=>{
    if(isOverlay(record.target))return false;
    if(record.type==='attributes'&&record.attributeName==='style'&&ownedStyles.get(record.target)===record.target.style.cssText)return false;
    if(record.type==='childList'&&[...record.addedNodes,...record.removedNodes].every(isOverlay))return false;
    return true;
   }))schedule();
  });
  for(const root of [doc.getElementById('bottom'),doc.getElementById('diplomacy-dialog')])if(root)observer.observe(root,{subtree:true,attributes:true,characterData:true,childList:true});observers.push(observer);
 }
 win.addEventListener?.('resize',schedule);listeners.push(['resize',schedule]);
 update();
 const api={enabled:true,errors:[],bitmapFonts:fonts,update,dispose(){if(disposed)return;disposed=true;api.enabled=false;for(const observer of observers)observer.disconnect();for(const [type,fn]of listeners)win.removeEventListener?.(type,fn);layer.dispose();fonts.dispose();bindings.clear();},status(){return {enabled:!disposed,...layer.status(),fontVariants:Object.keys(fonts.manifest.fonts).length,source:'shipped-cache-and-current-GDI',limits:['Native wrap and alternate-font layers remain unbound.']};}};
 return api;
}
