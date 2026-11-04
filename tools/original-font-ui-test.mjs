// Production Native5 components and real UP2; DOM/Canvas transport only.
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {makeDOM,Element,flattenDOM} from '../tools/diplomacy-dom-fixture.mjs';
import {loadNativeFontUI} from '../docs/original-font-ui.js';
import {loadNativeHud} from '../docs/original-hud.js';
import {enhanceOriginalDiplomacy} from '../docs/original-diplomacy.js';
import {createEconomyBar} from '../docs/economy-bar.js';
import {diplomacyPanel} from '../docs/diplomacy.js';


const read=async name=>JSON.parse(await fs.readFile(new URL(name,import.meta.url))),sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const fontManifest=await read('../docs/local-original/fonts-native/bitmap-manifest.json'),hudManifest=await read('../docs/local-original/hud-native/manifest.json'),dipManifest=await read('../docs/local-original/diplomacy-native/manifest.json');
const hookModes={'original-hud.js':'active','original-diplomacy.js':'active'};
const out=new URL('../work/native6-fonts/',import.meta.url);await fs.mkdir(out,{recursive:true});
let checks=0;const eq=(a,b,label)=>{assert.deepEqual(a,b,label);checks++;},check=(value,label)=>{assert.ok(value,label);checks++;};
const env=makeDOM(),doc=env.document;Object.assign(globalThis,env);env.window.devicePixelRatio=1;env.window.getComputedStyle=node=>node.style;
const windowEvents=new Map(),observers=[];
env.window.addEventListener=(kind,fn)=>{const list=windowEvents.get(kind)||new Set();list.add(fn);windowEvents.set(kind,list);};env.window.removeEventListener=(kind,fn)=>windowEvents.get(kind)?.delete(fn);
env.window.MutationObserver=class{constructor(callback){this.callback=callback;this.roots=[];this.active=true;observers.push(this);}observe(root,config){this.roots.push({root,config});}disconnect(){this.active=false;}};
const flush=async records=>{for(const observer of observers)if(observer.active)observer.callback(records);await Promise.resolve();};
Object.defineProperties(Element.prototype,{childNodes:{get(){return this.children;}},parentNode:{get(){return this.parentElement;}},nodeType:{get(){return 1;}}});
Element.prototype.contains=function(node){for(let p=node;p;p=p.parentElement)if(p===this)return true;return false;};
const originalRect=Element.prototype.getBoundingClientRect;
Element.prototype.getBoundingClientRect=function(){for(let p=this;p;p=p.parentElement)if(p.hidden||(p.tagName==='DIALOG'&&!p.open))return {left:0,top:0,width:0,height:0};return this.qaRect||originalRect.call(this);};
const make=(id,parent,tag='div',text)=>{const node=doc.createElement(tag);if(id)node.id=id;if(text!==undefined)node.textContent=text;parent.append(node);return node;};
const top=make('top',doc.body),bottom=make('bottom',doc.body),controls=top;
for(const id of ['pop','science','epoch']){const wrapper=make('',top,'span');wrapper.className='r';make(id,wrapper,'b',id);}
for(const slot of [0,1,3,4,5,2,6,7,8]){const row=make('row'+slot,top,'span');row.className='r';make('r'+slot,row,'b');}
const actions=[];
for(const [id,text]of [['research-button','Forschung'],['diplomacy-button','Diplomatie'],['idle','Freie Bürger'],['pause','Pause'],['cheats','Cheats'],['save','Speichern'],['fullscreen','Vollbild'],['quit','Hauptmenü']]){const button=make(id,controls,'button',text);button.onclick=()=>actions.push(id);}
make('terr',controls,'b');for(const id of ['mini','info','acts'])make(id,bottom,id==='mini'?'canvas':'div');
const bytes=await fs.readFile(new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url)),wasm=(await WebAssembly.instantiate(bytes,{})).instance.exports;wasm.init(71,1,0,1,0);wasm.skirmish_visibility(2);wasm.cheat(0);wasm.cheat(2);
const ui=()=>{const p=wasm.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,p,wasm.ui_len())));};
const panel=diplomacyPanel({wasm,referenceMode:true,onOrder(){},onOpen(){},onClose(){}}),native=await enhanceOriginalDiplomacy(panel,{wasm,...env,manifestURL:new URL('../docs/local-original/diplomacy-native/manifest.json',import.meta.url),fetchFile:async()=>({ok:true,json:async()=>dipManifest})});check(native.enabled,'Native5 diplomacy component active');
// The same existing button -> panel.open connection used by main.js.
doc.getElementById('diplomacy-button').onclick=()=>panel.open();
const textureManifest=await read('../docs/local-original/textures.json'),assets={enabled:true,manifest:textureManifest,hud:textureManifest.hudCropped||textureManifest.hud},economy=createEconomyBar(assets,true);
const hud=await loadNativeHud(assets,true,{...env,base:new URL('../docs/local-original/hud-native/',import.meta.url),fetch:async()=>({ok:true,json:async()=>hudManifest})});check(hud.enabled,'Native5 HUD component active');
function canvas(){const node=doc.createElement('canvas'),stack=[];node.bitmapDraws=[];const context={globalAlpha:1,save(){stack.push(this.globalAlpha);},restore(){this.globalAlpha=stack.pop();},beginPath(){},rect(){},clip(){},setTransform(){},clearRect(){node.bitmapDraws=[];},getImageData(){return {data:new Uint8ClampedArray(node.width*node.height*4).fill(255)};},putImageData(row){node.bitmapColor=[...row.data.slice(0,3)];},drawImage(image,...args){node.file=image.file;node.bitmapDraws.push({file:image.file,color:image.bitmapColor||[255,255,255],alpha:this.globalAlpha,args});}};node.getContext=()=>context;return node;}
let closedImages=0;
const fontUI=await loadNativeFontUI({enabled:true,hud,diplomacy:native,...env,manifestURL:new URL('../docs/local-original/fonts-native/bitmap-manifest.json',import.meta.url),fetchFile:async()=>({ok:true,json:async()=>fontManifest}),loadImage:async url=>{const file=url.pathname.split('/').at(-1),row=fontManifest.images[file];return {file,width:row.width,height:row.height,close(){closedImages++;}};},createCanvas:canvas});
check(fontUI.enabled,'Active native font coordinator loads');eq(fontUI.status().errors,[],'Coordinator loads source styles and variants');
const walk=node=>[node,...node.children.flatMap(walk)];
const update=()=>fontUI.update();
function refreshHud(){
 const value=ui();economy.update(value);hud.update(value,{cycle:false,startMonth:2,minutesPerSeason:5});doc.getElementById('pop').textContent=value.pop+' / '+value.cap;doc.getElementById('science').textContent=String(value.research.points);doc.getElementById('terr').textContent=String(value.terr);doc.getElementById('epoch').textContent=String(value.epoch+1);
 const mini=doc.getElementById('mini');mini.width=mini.height=256;mini.getContext('2d').putImageData(new env.ImageData(new Uint8ClampedArray(wasm.memory.buffer,wasm.minimap(),256*256*4).slice(),256,256),0,0);
 // The component's CSS row flow is represented explicitly by its declared
 // 33*screenY row height for this non-browser transport.
 const list=doc.getElementById('native-hud-resources'),r=list.getBoundingClientRect();let index=0;
 for(const row of list.children){if(row.hidden)continue;Object.assign(row.style,{left:r.left+'px',top:(r.top+index*19.8)+'px',width:r.width+'px',height:'19.8px',paddingLeft:'6.4px',paddingRight:'4.8px'});const image=row.children.find(node=>node.tagName==='IMG');if(image)Object.assign(image.style,{left:(r.left+6.4)+'px',top:(r.top+index*19.8+.3)+'px',width:'25.6px',height:'19.2px'});const value=row.children.find(node=>node.tagName==='B');if(value){
  // The DOM transport keeps an explicitly narrow intrinsic B-box. The font
  // adapter receives a separate row text field; it must not widen the B item.
  const intrinsic=Math.max(6,Math.min(35,value.textContent.length*5.4));value.qaRect={left:r.left+r.width-4.8-intrinsic,top:r.top+index*19.8,width:intrinsic,height:19.8};
 }index++;}
 update();return value;
}
async function snapshot(name){
 const live=walk(doc.body).filter(node=>['BUTTON','INPUT','SELECT','TEXTAREA'].includes(node.tagName)).map(element=>({element,text:element.textContent,disabled:element.disabled,onclick:element.onclick}));
 const counters=walk(doc.body).filter(node=>node.tagName==='B'&&node.qaRect&&!node.parentElement.hidden).map(element=>({element,width:element.getBoundingClientRect().width,text:element.textContent}));
 update();for(const record of live){eq(record.element.textContent,record.text,'Bitmap update retains every live control text');eq(record.element.disabled,record.disabled,'Bitmap update retains every live disabled state');eq(record.element.onclick,record.onclick,'Bitmap update retains every live onclick');}
 for(const record of counters){eq(record.element.getBoundingClientRect().width,record.width,'Resource intrinsic B-box is never widened');eq(record.element.textContent,record.text,'Every original resource digit retained');const overlay=record.element.parentElement.children.find(node=>node.classList.contains('native-bitmap-text'));check(parseFloat(overlay.style.width)>record.width,'Explicit row text field is wider than the narrow intrinsic counter');check(!overlay.hidden,'Native resource glyphs fit the explicit field');}
 const rows=flattenDOM(doc),nodes=walk(doc.body);for(let i=0;i<rows.length;i++){const node=nodes[i];if(node.tagName==='IMG')rows[i].src=node.src;if(node.bitmapDraws){rows[i].bitmapDraws=node.bitmapDraws.filter(row=>row.args.length===8);rows[i].canvasWidth=node.width;rows[i].canvasHeight=node.height;}}
 await fs.writeFile(new URL('ui-'+name+'.json',out),JSON.stringify(rows)+'\n');return {name,bitmapLabels:rows.filter(row=>row.bitmapDraws?.length&&!row.hidden).length,unsupported:fontUI.status().unsupported};
}
const scenes=[];refreshHud();scenes.push(await snapshot('hud'));doc.getElementById('diplomacy-button').click();check(doc.getElementById('diplomacy-dialog').open,'Original F4 button still opens diplomacy');update();scenes.push(await snapshot('relations'));
wasm.skirmish_epoch(6);panel.refresh(ui());refreshHud();native.setView('resources');update();const dialog=doc.getElementById('diplomacy-dialog'),cell=dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget?.startsWith('resources1__'));cell.click({shiftKey:true});update();scenes.push(await snapshot('resources'));
const send=dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_accept'),before=wasm.resource_stat(0,0),handler=send.onclick;update();eq(send.onclick,handler,'Real tribute retains exact onclick');send.click();eq(wasm.resource_stat(0,0),before-500,'Actual bitmap-labeled native tribute debits500');
native.setView('relations');native.openProposal(0);update();scenes.push(await snapshot('proposal'));
native.setView('territories');update();scenes.push(await snapshot('territories'));
const idle=doc.getElementById('idle'),idleHandler=idle.onclick;hud.update({...ui(),idle:0},{cycle:false,startMonth:2,minutesPerSeason:5});update();eq(idle.disabled,true,'Original HUD controls retain disabled state');eq(idle.onclick,idleHandler,'Original HUD idle listener retained');

// Explicit observer delivery tests the actual callback/scheduling path. The
// fixture supplies no browser layout; every DOM box remains separately declared.
eq(observers.length,1,'One coordinated observer');eq(observers[0].roots.map(x=>x.root.id),['bottom','diplomacy-dialog'],'Both production component roots observed');
const name=doc.body.querySelector('.native-hud-heading'),overlayFor=node=>node.parentElement.children.find(x=>x.classList.contains('native-bitmap-text')&&x.style.left===node.getBoundingClientRect().left+'px'&&x.style.top===node.getBoundingClientRect().top+'px');
name.textContent='Müller ÄÖÜ ß';await flush([{type:'characterData',target:name,addedNodes:[],removedNodes:[]}]);check(name.style.color==='transparent','Mutation refresh renders German live text');check(overlayFor(name).bitmapDraws.length>0,'German source label has real glyph draws');
name.style.visibility='hidden';await flush([{type:'attributes',attributeName:'style',target:name,addedNodes:[],removedNodes:[]}]);eq(overlayFor(name).hidden,true,'Source visibility suppresses sibling canvas');
name.style.visibility='visible';name.style.color='blue';await flush([{type:'attributes',attributeName:'style',target:name,addedNodes:[],removedNodes:[]}]);eq(overlayFor(name).hidden,false,'Source visibility restores canvas');
const labelOverlay=overlayFor(name),initialDraws=labelOverlay.bitmapDraws.length;await flush([{type:'attributes',attributeName:'style',target:name,addedNodes:[],removedNodes:[]}]);eq(labelOverlay.bitmapDraws.length,initialDraws,'Own style mutation does not replace glyph draw list');
await flush([{type:'childList',target:name.parentElement,addedNodes:[labelOverlay],removedNodes:[]}]);eq(fontUI.status().errors,[],'Own canvas insertion is ignored');
const counter=doc.getElementById('r0'),box=counter.qaRect;counter.textContent='9999999999999999999999999';await flush([{type:'characterData',target:counter,addedNodes:[],removedNodes:[]}]);check(fontUI.status().overflow>0,'Too-wide amount has explicit fallback');check(counter.style.color!=='transparent','Overflow keeps all original visible digits');counter.textContent=String(wasm.resource_stat(0,0));update();eq(counter.qaRect,box,'Source counter narrow box remains unchanged');
// The same physical canvas dimensions with changed CSS/DPR must redraw.
const epoch=doc.getElementById('native-hud-epoch'),oldBox=epoch.getBoundingClientRect();env.window.devicePixelRatio=2;epoch.qaRect={...oldBox,width:oldBox.width/2,height:oldBox.height/2};for(const fn of windowEvents.get('resize')||[])fn();await Promise.resolve();eq(overlayFor(epoch).style.width,epoch.qaRect.width+'px','Coordinator resize refreshes actual CSS rectangle');
const beforeDetach=fontUI.status().elements;name.remove();await flush([{type:'childList',target:bottom,addedNodes:[],removedNodes:[name]}]);eq(fontUI.status().elements,beforeDetach-1,'Removed source label detaches coordinated binding');check(!labelOverlay.parentElement,'Removed label has no orphan overlay');bottom.append(name);update();eq(fontUI.status().elements,beforeDetach,'Reinserted source label is attached once');
const disabled=await loadNativeFontUI({enabled:false,hud,...env});eq(disabled.enabled,false,'Disabled option avoids bitmap load');
const unavailable=await loadNativeFontUI({enabled:true,hud:{},diplomacy:{},...env});eq(unavailable.enabled,false,'Unbound components retain existing presentation');
const failed=await loadNativeFontUI({enabled:true,hud,...env,fetchFile:async()=>({ok:false})});eq(failed.enabled,false,'Missing font manifest has explicit safe fallback');check(failed.errors.length===1,'Failed load exposes exact error');
const expectedImages=Object.keys(fontManifest.images).length;
fontUI.dispose();fontUI.dispose();eq(closedImages,expectedImages,'Dispose closes every loaded atlas exactly once');eq(observers[0].active,false,'Dispose disconnects observer');eq(walk(doc.body).filter(node=>node.classList.contains('native-bitmap-text')).length,0,'Dispose removes all glyph overlays');eq(name.style.color,'blue','Dispose preserves the latest external source palette');eq(fontUI.enabled,false,'Dispose updates enabled status');
native.dispose();

const paths=['docs/original-fonts.js','docs/original-font-ui.js','docs/main.js','docs/index.html','docs/original-hud.js','docs/original-hud-layout.js','docs/economy-bar.js','docs/original-diplomacy.js','docs/original-diplomacy-layout.js','docs/local-original/hud-native/manifest.json','docs/local-original/diplomacy-native/manifest.json','docs/local-original/textures.json'];const sourceHashes=Object.fromEntries(await Promise.all(paths.map(async path=>[path,sha(await fs.readFile(new URL('../'+path,import.meta.url)))])));
const report={format:'native6-bitmap-font-live-ui-tests-v1',status:'passed',checks,scenes,hookModes,wasmSha256:sha(bytes),wasmRuleset:wasm.ruleset(),sourceHashes,fontModuleSha256:sha(await fs.readFile(new URL('../docs/original-fonts.js',import.meta.url))),fontManifestSha256:sha(await fs.readFile(new URL('../docs/local-original/fonts-native/bitmap-manifest.json',import.meta.url))),limits:['Production HUD/diplomacy controllers, active font coordinator/modules, real WASM and actual glyph placements; actual active component fontBindings are used directly. This is not a browser/CSS raster or original executable capture.','Resource-row CSS flow is explicitly represented from Native5 CSS declarations. Resource typography and newly added calendar label font are identified application bindings.','Legacy proposal/territory form flow and unbound alternate-font glyphs keep their previous presentation. Main entry point bindings are active and their source hash is recorded.']};
await fs.writeFile(new URL('../../EE2-Research/analysis/native6-font-ui-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));

