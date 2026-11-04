// Minimal DOM transport for the real existing diplomacy controllers. This is
// deliberately not a browser/layout engine; source geometry has separate tests.
function style(){
 const values={};return new Proxy({setProperty(key,value){values[key]=String(value);},getPropertyValue(key){return values[key]||'';}},{get(target,key){if(key==='cssText')return JSON.stringify(values);return key in target?target[key]:values[key]||'';},set(target,key,value){if(key==='cssText'){for(const k of Object.keys(values))delete values[k];if(value)Object.assign(values,JSON.parse(value));}else values[key]=String(value);return true;},ownKeys(){return Reflect.ownKeys(values);},getOwnPropertyDescriptor(){return {configurable:true,enumerable:true};}});
}
export class Element{
 constructor(tag='div',doc){this.tagName=tag.toUpperCase();this.ownerDocument=doc;this.children=[];this.parentElement=null;this.attributes={};this.dataset={};this.style=style();this.hidden=false;this.disabled=false;this.open=false;this._classes=new Set();this.listeners=new Map();this._text='';}
 set className(value){this._classes=new Set(String(value).split(/\s+/).filter(Boolean));}get className(){return [...this._classes].join(' ');}
 get classList(){return {add:(...values)=>values.forEach(v=>this._classes.add(v)),remove:(...values)=>values.forEach(v=>this._classes.delete(v)),contains:value=>this._classes.has(value),toggle:(value,force)=>{const active=force??!this._classes.has(value);active?this._classes.add(value):this._classes.delete(value);return active;}};}
 set textContent(value){this._text=String(value);for(const child of [...this.children])child.remove();}get textContent(){return this._text+this.children.map(child=>child.textContent).join('');}
 set value(value){this._value=String(value);}get value(){return this._value??(this.tagName==='SELECT'?this.children[0]?.value??'':'');}
 get options(){return this.children;}get firstChild(){return this.children[0]||null;}get nextSibling(){const i=this.parentElement?.children.indexOf(this);return i>=0?this.parentElement.children[i+1]||null:null;}get nextElementSibling(){return this.nextSibling;}
 append(...nodes){for(let node of nodes){if(typeof node==='string'){const text=new Element('span',this.ownerDocument);text.textContent=node;node=text;}node.remove();node.parentElement=this;this.children.push(node);}}
 prepend(...nodes){for(const node of [...nodes].reverse()){node.remove();node.parentElement=this;this.children.unshift(node);}}
 remove(){if(this.parentElement){this.parentElement.children=this.parentElement.children.filter(child=>child!==this);this.parentElement=null;}}
 insertBefore(node,next){node.remove();const index=this.children.indexOf(next);if(index<0)return this.append(node);node.parentElement=this;this.children.splice(index,0,node);}
 after(node){this.parentElement?.insertBefore(node,this.nextSibling);}
 replaceChildren(...nodes){for(const child of [...this.children])child.remove();this._text='';if(this.tagName==='SELECT')delete this._value;this.append(...nodes);}
 setAttribute(key,value){this.attributes[key]=String(value);if(key==='id')this.id=String(value);}
 getAttribute(key){if(key==='class')return this.className;if(key==='id')return this.id??null;if(key==='for')return this.htmlFor??this.attributes[key]??null;return this.attributes[key]??null;}
 removeAttribute(key){delete this.attributes[key];}
 matches(selector){
  const attribute=/^(.*?)\[([\w-]+)(?:=["']?([^\]"']+)["']?)?\]$/.exec(selector);
  if(attribute){if(attribute[1]&&!this.matches(attribute[1]))return false;const value=this.getAttribute(attribute[2]);return value!==null&&(attribute[3]===undefined||value===attribute[3]);}
  if(selector.startsWith('.'))return this.classList.contains(selector.slice(1));if(selector.startsWith('#'))return this.id===selector.slice(1);return this.tagName.toLowerCase()===selector.toLowerCase();
 }
 querySelectorAll(selector){const result=[];for(const child of this.children){if(child.matches(selector))result.push(child);result.push(...child.querySelectorAll(selector));}return result;}
 querySelector(selector){return this.querySelectorAll(selector)[0]||null;}
 closest(selector){return this.matches(selector)?this:this.parentElement?.closest(selector)||null;}
 addEventListener(type,fn,capture=false){const list=this.listeners.get(type)||[];list.push({fn,capture});this.listeners.set(type,list);}
 removeEventListener(type,fn){this.listeners.set(type,(this.listeners.get(type)||[]).filter(item=>item.fn!==fn));}
 dispatchEvent(event){event.target??=this;event.preventDefault??=function(){this.defaultPrevented=true;};event.stopImmediatePropagation??=function(){this.stopped=true;};for(const {fn}of [...(this.listeners.get(event.type)||[])].sort((a,b)=>Number(b.capture)-Number(a.capture))){fn(event);if(event.stopped)break;}return !event.defaultPrevented;}
 click(event={}){if(this.disabled)return;this.onclick?.({...event,target:this,preventDefault(){}});}
 focus(){this.ownerDocument.activeElement=this;}
 showModal(){this.open=true;}close(){this.open=false;}
 getBoundingClientRect(){return {left:parseFloat(this.style.left)||0,top:parseFloat(this.style.top)||0,width:parseFloat(this.style.width)||this.width||256,height:parseFloat(this.style.height)||this.height||256};}
 getContext(kind){if(kind!=='2d')return null;this.context??={putImageData:image=>{this.pixels=image;}};return this.context;}
 toDataURL(){return `data:image/x-test-rgba;w=${this.width};h=${this.height};base64,${Buffer.from(this.pixels.data).toString('base64')}`;}
}
export function makeDOM(width=1280,height=720){
 const document={createElement:tag=>new Element(tag,document)};
 document.documentElement=new Element('html',document);document.head=new Element('head',document);document.body=new Element('body',document);document.documentElement.append(document.head,document.body);
 document.getElementById=id=>document.documentElement.querySelector('#'+id);
 const events=new Map(),window={innerWidth:width,innerHeight:height,addEventListener:(kind,fn)=>events.set(kind,fn),removeEventListener:kind=>events.delete(kind),events};
 const Option=function(text,value){const option=new Element('option',document);option.textContent=text;option.value=value;return option;};
 const ImageData=class{constructor(data,width,height){this.data=data;this.width=width;this.height=height;}};
 return {document,window,Option,ImageData};
}
export function flattenDOM(document){
 const rows=[];function visit(node,hidden=false,flowRegion=null){hidden=hidden||node.hidden;if(node.classList.contains('native-dip-proposal-body')||node.classList.contains('native-dip-territory-body'))flowRegion=node.className;rows.push({tag:node.tagName,id:node.id,className:node.className,text:node._text,value:node.value,hidden,disabled:node.disabled,flowRegion,dataset:{...node.dataset},style:Object.fromEntries(Object.keys(node.style).map(key=>[key,node.style[key]])),pixels:node.pixels?{width:node.pixels.width,height:node.pixels.height,rgba:Buffer.from(node.pixels.data).toString('base64')}:undefined});for(const child of node.children)visit(child,hidden,flowRegion);}visit(document.body);return rows;
}
