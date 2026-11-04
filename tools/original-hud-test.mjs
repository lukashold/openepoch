import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {calculateHudLayout,anchorFlags,nativeHudRect,nativeMetaEpoch} from '../docs/original-hud-layout.js';
import {loadNativeHud} from '../docs/original-hud.js';

const manifest=JSON.parse(await fs.readFile(new URL('../docs/local-original/hud-native/manifest.json',import.meta.url)));
let checks=0;const check=(ok,text)=>{assert.ok(ok,text);checks++;};
const eq=(a,b,text)=>{assert.deepEqual(a,b,text);checks++;};
// Independent named-corner oracle, with native signed integer truncation.
const corners={UL:[0,0],LL:[0,1],UR:[1,0],LR:[1,1]};
for(const width of [800,1366,1600,1920,2048,2560])for(const height of [600,768,900,1080,1152,1440]){
 for(const [source,sc]of Object.entries(corners))for(const [target,tc]of Object.entries(corners)){
  for(const [x,y]of [[0,0],[-53,37],[43,-71],[-1,-1]]){
   const row={NAME:'fixture',ANCHORTYPE1:`Source${source}Target${target}`,XOFF:x,YOFF:y,WIDTH:57,HEIGHT:45};
   const ref={x:31,y:67,width:307,height:191};
   const w=Math.max(1,Math.trunc(57*width/1600)),h=Math.max(1,Math.trunc(45*height/1200));
   eq(nativeHudRect(row,ref,ref,width,height),{x:31+sc[0]*307+Math.trunc(x*width/1600)-tc[0]*w,
      y:67+sc[1]*191+Math.trunc(y*height/1200)-tc[1]*h,width:w,height:h},'Named-corner native placement');
  }
 }
 const layout=calculateHudLayout(manifest,width,height);
 for(const [name,r]of layout){check(Number.isInteger(r.x+r.y+r.width+r.height),'Integer layout '+name);check(r.width>=0&&r.height>=0,'Positive dimensions '+name);}
}
eq(anchorFlags('SourceULTargetUL'),1);eq(anchorFlags('SourceLRTargetLR'),31);
for(let epoch=0;epoch<15;epoch++)eq(nativeMetaEpoch(epoch),epoch<=4?1:epoch<=9?2:3,'Header thresholds5/10');
const actual=calculateHudLayout(manifest,1600,1200);
eq(actual.get('game_minimap_container'),{x:7,y:883,width:252,height:251},'Minimap source fixture');
eq(actual.get('game_unitinfo'),{x:794,y:930,width:380,height:254},'Info source fixture');
eq(actual.get('game_unitbuttons'),{x:471,y:933,width:316,height:245},'Command source fixture');
eq(actual.get('game_playerinfo_civ'),{x:851,y:13,width:230,height:22},'Negative implicit right anchor');
eq(actual.get('game_button_options'),{x:1103,y:2,width:40,height:36},'Native Outer frame adds2px to toolbar content');
eq(actual.get('game_button_pause'),{x:1146,y:2,width:40,height:36},'Sibling anchor uses preceding outer frame bounds');
eq(actual.get('game_button_chat'),{x:1189,y:2,width:40,height:36},'Chained toolbar anchor retains native frame extent');
const framed={widgets:{
 parent:{NAME:'parent',WIDTH:'100',HEIGHT:'100',FRAMESTYLE:'outer'},
 child:{NAME:'child',PARENT:'parent',ANCHORTYPE1:'SourceULTargetUL',WIDTH:'10',HEIGHT:'10'},
 sibling:{NAME:'sibling',ANCHORNAME1:'parent',ANCHORTYPE1:'SourceURTargetUL',WIDTH:'10',HEIGHT:'10'}
},frameStyles:{outer:{'WIDTH[]':'[3 3 3 3 3 3]',POSITION:'Outer'}}};
const framedLayout=calculateHudLayout(framed,1600,1200);
eq(framedLayout.get('child'),{x:3,y:3,width:10,height:10},'Ancestor source anchors to content inside its outer frame');
eq(framedLayout.get('sibling'),{x:106,y:0,width:10,height:10},'Sibling source anchors to the outside of its outer frame');
// Minimal DOM transport. The production loader moves original live nodes;
// geometry/CSS raster are reviewed separately in source-sprite previews.
class Element{
 constructor(tag='div'){this.tagName=tag.toUpperCase();this.children=[];this.parentElement=null;this.attributes={};this.dataset={};this.style={setProperty(k,v){this[k]=v;}};this._classes=new Set();this.classList={add:(...x)=>x.forEach(v=>this._classes.add(v)),contains:x=>this._classes.has(x)};this.textContent='';}
 set className(v){this._classes=new Set(v.split(/\s+/).filter(Boolean));}get className(){return [...this._classes].join(' ');}
 get firstChild(){return this.children[0]||null;}
 append(...nodes){for(const n of nodes){n.remove();n.parentElement=this;this.children.push(n);}}
 prepend(n){n.remove();n.parentElement=this;this.children.unshift(n);}
 remove(){if(this.parentElement){this.parentElement.children=this.parentElement.children.filter(x=>x!==this);this.parentElement=null;}}
 replaceChildren(...nodes){for(const c of [...this.children])c.remove();this.append(...nodes);}
 setAttribute(k,v){this.attributes[k]=String(v);}
 matches(s){return s.startsWith('.')?this.classList.contains(s.slice(1)):s.startsWith('#')?this.id===s.slice(1):this.tagName.toLowerCase()===s;}
 querySelector(s){return this.children.find(x=>x.matches(s))||this.children.map(x=>x.querySelector(s)).find(Boolean)||null;}
 closest(s){return this.matches(s)?this:this.parentElement?.closest(s)||null;}
 addEventListener(type,callback){this.listeners??=new Map();this.listeners.set(type,callback);}
 click(){this.onclick?.();for(let node=this;node;node=node.parentElement)node.listeners?.get('click')?.({target:this});}
}
const doc={documentElement:new Element('html'),createElement:tag=>new Element(tag)};
doc.head=new Element('head');doc.body=new Element('body');doc.documentElement.append(doc.head,doc.body);
doc.getElementById=id=>doc.documentElement.querySelector('#'+id);
const make=(id,parent,tag='div')=>{const n=new Element(tag);n.id=id;parent.append(n);return n;};
const top=make('top',doc.body),bottom=make('bottom',doc.body),first=make('first',top),resource=make('resource',first),summary=make('summary',first);
const controls=make('controls',top);controls.className='ee2-economy-controls';
for(const id of ['pop','science','epoch'])make(id,summary,'b').textContent=id;
for(const slot of [0,1,3,4,5,2,6,7,8]){const row=make('row'+slot,resource,'span');row.className='ee2-economy-resource';make('r'+slot,row,'b');}
let invoked=0;const original=new Map();
for(const id of ['research-button','diplomacy-button','idle','pause','cheats','save','fullscreen','quit']){
 const n=make(id,controls,'button');n.textContent=id;n.onclick=()=>invoked++;original.set(id,n);
}
make('terr',controls,'b');for(const id of ['mini','info','acts'])original.set(id,make(id,bottom,id==='mini'?'canvas':'div'));
const listeners=new Map(),win={innerWidth:1280,innerHeight:720,addEventListener:(k,fn)=>listeners.set(k,fn)};
const options={document:doc,window:win,base:new URL('../docs/local-original/hud-native/',import.meta.url),fetch:async()=>({ok:true,json:async()=>manifest})};
eq((await loadNativeHud({enabled:false},true,options)).enabled,false,'Missing originals use existing interface');
const hud=await loadNativeHud({enabled:true},true,options);check(hud.enabled,'Native HUD loaded');
for(const [id,node]of original){check(doc.getElementById(id)===node,'Live ID retained '+id);if(node.tagName==='BUTTON')node.click();}
eq(invoked,8,'Every original button still runs its original listener');
const optionsMenu=doc.getElementById('native-hud-options');optionsMenu.open=true;
original.get('save').click();eq(invoked,9,'Save still runs with the drawer open');check(!optionsMenu.open,'A menu command closes the drawer');
optionsMenu.open=true;let escaped=false;listeners.get('keydown')({key:'Escape',preventDefault(){escaped=true;},stopImmediatePropagation(){escaped&&=true;}});
check(!optionsMenu.open&&escaped,'Escape closes the native options drawer before game commands');
check(doc.getElementById('native-hud-options-content').children.includes(original.get('save')),'Save reachable in drawer');
check(doc.getElementById('native-hud-options-content').children.includes(original.get('fullscreen')),'Fullscreen reachable in drawer');
hud.update({epoch:6,ename:'Renaissance',tick:1200},{cycle:false,startMonth:11,minutesPerSeason:5});eq(hud.status().metaEpoch,2,'Parchment theme');
eq(doc.getElementById('native-hud-clock').textContent,'0:01:00','Clock shows actual simulation time');
eq(doc.getElementById('native-hud-month').textContent,'Dezember','Month follows the saved fixed-month setting');
hud.update({epoch:6,tick:1200},{cycle:true,startMonth:6,minutesPerSeason:1});
eq(doc.getElementById('native-hud-month').textContent,'Oktober','Month follows the shared cyclic calendar');
check(doc.getElementById('native-hud-epoch').textContent==='7','One-based epoch shown');
doc.getElementById('native-hud-epoch').click();eq(invoked,10,'Epoch opens original research control');
win.innerWidth=1920;win.innerHeight=1080;listeners.get('resize')();
eq(original.get('mini').style.width,'302px','Resize uses native truncated width');
hud.update({epoch:14});eq(hud.status().metaEpoch,3,'Metal theme');
const report={format:'ee2-native-hud-tests-v1',status:'passed',checks,
 scope:'Independent named-corner/integer fixtures for native geometry; exact live DOM identity, command listeners, drawer access, themes and resize. Browser raster and full native widget state are not asserted.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native4-hud-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
