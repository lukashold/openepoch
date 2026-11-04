import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {bitmapTextLayout,tintFontRGBA,sourceBitmapStyle,loadBitmapFonts,createBitmapTextLayer} from '../docs/original-fonts.js';
import {makeDOM} from './diplomacy-dom-fixture.mjs';
import {diplomacyPanel} from '../docs/diplomacy.js';
import {enhanceOriginalDiplomacy} from '../docs/original-diplomacy.js';
const read=async name=>JSON.parse(await fs.readFile(new URL(name,import.meta.url))),manifest=await read('../docs/local-original/fonts-native/bitmap-manifest.json');
let checks=0;const eq=(a,b,label)=>{assert.deepEqual(a,b,label);checks++;},check=(v,label)=>{assert.ok(v,label);checks++;};
for(const row of await read('./fixtures/native6-font-metrics.json')){const value=bitmapTextLayout(manifest.fonts[row.font],row.text);eq([value.width,value.height],[row.width,row.height],'Independent native CalcTextBounds');eq(value.unsupported,[],'Every tested German label is portable');}
eq(bitmapTextLayout(manifest.fonts.font20,null).height,0,'Native null text returns zero bounds');
const unsupported=bitmapTextLayout(manifest.fonts.font20,'A—B');eq(unsupported.unsupported,[0x2014],'Unbound alternate font is explicit');
for(const color of [[255,255,255],[200,100,20],[0,0,0]]){
 const source=new Uint8ClampedArray([255,255,255,0,85,119,221,68,0,0,0,255,255,255,255,136]),value=tintFontRGBA(source,color);
 for(let i=0;i<source.length;i++)eq(value[i],i%4===3?source[i]:Math.floor(source[i]*color[i%4]/255+.5),'Native glyph RGB modulation/source alpha');
}
assert.throws(()=>tintFontRGBA(new Uint8Array(3),[0,0,0]));checks++;
const source=await read('../docs/local-original/diplomacy-native/manifest.json');
for(const width of [800,1083,1280,1366,1600,1920,2048,2560]){
 const style=sourceBitmapStyle(source,'diplo_panel_title',width);check(manifest.bindings[style.font][style.tier],'Actual source title font bound');eq(style.color,[255,255,255],'Actual source title RGBA role');
}
eq(sourceBitmapStyle(source,'diplo_panel_title',1280,{disabled:true}).color,[128,128,128],'Original disabled font color');
const styleSource={widgets:{label:{FONTSTYLE:'plain',TEXTJUST:'RightMid'}},fontStyles:{plain:{FONT:'fnt_pal_med',TEXTCOL:'[255 255 255]',HICOL:'[10 20 30]','HILITE?':'0'},plain_e2:{FONT:'fnt_pal_small',TEXTCOL:'[20 20 20]',HICOL:'[240 240 100]','HILITE?':'1'}}};
eq(sourceBitmapStyle(styleSource,'label',1280,{highlighted:true}).color,[255,255,255],'Unhighlighted source font style keeps base color');eq(sourceBitmapStyle(styleSource,'label',1280,{meta:2,highlighted:true}).color,[240,240,100],'Source theme font style/highlight declaration');
const dom=makeDOM(),draws=[];
function canvas(doc=dom.document){const node=doc.createElement('canvas'),stack=[];const context={globalAlpha:1,save(){stack.push(this.globalAlpha);},restore(){this.globalAlpha=stack.pop();},beginPath(){},rect(){},clip(){},setTransform(){},clearRect(){},getImageData(){return {data:new Uint8ClampedArray(node.width*node.height*4).fill(255)};},putImageData(row){node.color=[...row.data.slice(0,3)];},drawImage(image,...args){node.file=image.file;draws.push({file:image.file,color:image.color||[255,255,255],alpha:this.globalAlpha,args});}};node.getContext=()=>context;return node;}
const base=new URL('../docs/local-original/fonts-native/bitmap-manifest.json',import.meta.url),runtime=await loadBitmapFonts({manifestURL:base,fetchFile:async()=>({ok:true,json:async()=>manifest}),loadImage:async url=>{const file=url.pathname.split('/').at(-1),row=manifest.images[file];return {file,width:row.width,height:row.height};},createCanvas:canvas});
eq(runtime.font('fnt_pal_med',1).nominalSize,18,'Effective source14/18/... variants select exactly');
for(const [name,style]of Object.entries(manifest.fontStyles)){
 const rgba=(style['DISABLE COLOR'].match(/\d+/g)||[]).map(Number);if(rgba.length!==4)continue;
 const input={widgets:{label:{FONTSTYLE:name}},fontStyles:manifest.fontStyles},settings=sourceBitmapStyle(input,'label',1280,{disabled:true}),ctx=canvas().getContext('2d');ctx.globalAlpha=.4;draws.length=0;runtime.draw(ctx,runtime.font(settings.font,settings.tier),'A',{...settings});eq(draws.at(-1).alpha,.4*Math.fround(rgba[3]*Math.fround(1/255)),'Source RGBA alpha reaches every disabled glyph quad');eq(ctx.globalAlpha,.4,'Glyph draw restores caller alpha');eq(settings.opacityMode,'source','Native text opacity bypasses the application CSS approximation');
}
const button=dom.document.createElement('button');button.textContent='Bündnis anbieten';button.style.width='300px';button.style.height='40px';button.style.color='red';let clicks=0;const handler=()=>clicks++;button.onclick=handler;dom.document.body.append(button);
const parent=button.parentElement,layer=createBitmapTextLayer(runtime,{...dom,createCanvas:canvas});check(layer.attach(button,{font:'fnt_pal_med',tier:1,color:[255,255,255]}),'Attach source label without replacing node');layer.update();eq(button.textContent,'Bündnis anbieten','Accessible native text is retained');eq(button.onclick,handler,'Button listener identity retained');eq(button.parentElement,parent,'Original control parent retained');eq(button.style.color,'transparent','Known glyphs replace visual browser text');button.click();eq(clicks,1,'Original action remains clickable');eq(parent.children.at(-1).style.pointerEvents,'none','Bitmap overlay cannot intercept controls');
button.disabled=true;layer.update();eq(button.disabled,true,'Disabled state untouched');button.textContent='—';layer.update();eq(button.style.color,'red','Unknown glyph keeps existing presentation');eq(layer.status().unsupported,[0x2014],'Unknown source role is diagnosed');
button.textContent='Öl und Uran';button.disabled=false;layer.update();eq(layer.status().unsupported,[],'Updated German text redraws from bitmap');check(draws.some(row=>row.args.length===8),'Production9-argument drawImage glyph path executed');
const input=dom.document.createElement('input');dom.document.body.append(input);eq(layer.attach(input,{font:'fnt_pal_med'}),false,'Input controls are preserved');
eq(layer.attach(button,{font:'missing'}),false,'Invalid font cannot replace an already registered control');
layer.dispose();eq(button.onclick,handler,'Dispose preserves handler');eq(button.style.color,'red','Dispose restores original text presentation');eq(parent.children.filter(row=>row.className==='native-bitmap-text').length,0,'Dispose removes only its overlay');
const globals=Object.fromEntries(['document','window','Option','ImageData'].map(key=>[key,globalThis[key]])),actualWasm=[];
try{for(const file of ['openepoch-ee2.wasm','openepoch-ee2-up15.wasm']){
 const env=makeDOM();Object.assign(globalThis,env);const bottom=env.document.createElement('div');bottom.id='bottom';env.document.body.append(bottom);const trigger=env.document.createElement('button');trigger.id='diplomacy-button';env.document.body.append(trigger);
 const bytes=await fs.readFile(new URL('../docs/'+file,import.meta.url)),wasm=(await WebAssembly.instantiate(bytes,{})).instance.exports;wasm.init(71,1,0,1,0);wasm.skirmish_visibility(2);wasm.cheat(0);wasm.cheat(2);
 const panel=diplomacyPanel({wasm,referenceMode:true,onOrder(){},onOpen(){},onClose(){}}),native=await enhanceOriginalDiplomacy(panel,{wasm,...env,manifestURL:new URL('../docs/local-original/diplomacy-native/manifest.json',import.meta.url),fetchFile:async()=>({ok:true,json:async()=>source})});check(native.enabled,'Actual production diplomacy presentation attached');panel.open();native.setView('resources');const dialog=env.document.getElementById('diplomacy-dialog'),cell=dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget?.startsWith('resources1__')),send=dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_accept');cell.click();
 const originalHandler=send.onclick,local=wasm.resource_stat(0,0),text=send.textContent,overlay=createBitmapTextLayer(runtime,{...env,createCanvas:()=>canvas(env.document)});overlay.attach(send,{font:'fnt_pal_med',tier:1,color:[255,255,255]});overlay.update();eq(send.onclick,originalHandler,'Actual native tribute listener preserved under glyph overlay');eq(send.textContent,text,'Actual native tribute accessible label preserved');send.click();eq(wasm.resource_stat(0,0),local-100,'Actual WASM tribute succeeds through bitmap-labeled live button');overlay.dispose();native.dispose();actualWasm.push({file,ruleset:wasm.ruleset(),sha256:createHash('sha256').update(bytes).digest('hex')});
}}finally{for(const [key,value]of Object.entries(globals))if(value===undefined)delete globalThis[key];else globalThis[key]=value;}
// Capture actual production placements for independent source-PNG composition.
const captures=[];
for(const [name,text,tier,color]of [['fnt_pal_med','Bündnis anbieten',1,[255,255,255]],['fnt_pal_large','Diplomatie',1,[255,255,255]],['fnt_pal_med_sc','ÄÖÜ Straße größer',2,[240,240,100]],['fnt_pal_large_outline','Territorium verschenken',1,[255,230,180]],['fnt_pal_small','Nahrung 2500 | Holz 1200',2,[255,255,255]]]){
 draws.length=0;const target=canvas();runtime.draw(target.getContext('2d'),runtime.font(name,tier),text,{x:12,y:8,width:760,height:70,color,vertical:'middle',align:'left'});captures.push({name,text,tier,color,width:784,height:86,draws:draws.filter(row=>row.args.length===8)});
}
await fs.writeFile(new URL('../work/native6-fonts/bitmap-draw-captures.json',import.meta.url),JSON.stringify(captures,null,2)+'\n');runtime.dispose();
const sha=data=>createHash('sha256').update(data).digest('hex'),report={format:'native6-bitmap-font-runtime-tests-v1',passed:true,checks,metricCases:(await read('./fixtures/native6-font-metrics.json')).length,captures:captures.length,actualWasm,sourceHashes:{runtime:sha(await fs.readFile(new URL('../docs/original-fonts.js',import.meta.url))),manifest:sha(await fs.readFile(base))},scope:'Actual active layout/loader/draw and sibling-overlay code; unchanged control nodes/listeners/accessible text; independent native metric oracle. Two production WASMs and live native diplomacy tribute controls execute a real resource debit. Canvas/DOM transport substituted, source atlas composition verified separately. No browser/original process.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native6-font-runtime-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
