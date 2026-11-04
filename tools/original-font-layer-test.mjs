import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createBitmapTextLayer} from '../docs/original-fonts.js';
import {makeDOM} from './diplomacy-dom-fixture.mjs';
let checks=0;const eq=(a,b,label)=>{assert.deepEqual(a,b,label);checks++;};
function setup(){
 const env=makeDOM(),doc=env.document;env.window.devicePixelRatio=1;env.window.getComputedStyle=node=>node.style;
 const parent=doc.createElement('div'),button=doc.createElement('button');doc.body.append(parent);parent.append(button);button.textContent='AAA';button.style.color='red';button.style.textShadow='1px 1px black';
 let box={left:10,top:20,width:100,height:20},draws=0;const transforms=[];button.getBoundingClientRect=()=>box;
 const font={flags:0,spacing:0,lineHeight:10,kerning:{},glyphs:{65:{width:6,height:10,file:'atlas',x:0,y:0}}},fonts={font(name){if(name==='missing')throw Error('Missing');return font;},draw(){draws++;}};
 const createCanvas=()=>{const node=doc.createElement('canvas');node.getContext=()=>({setTransform(...row){transforms.push(row);},clearRect(){}});return node;};
 const layer=createBitmapTextLayer(fonts,{...env,createCanvas});return {env,doc,parent,button,layer,setBox:value=>box=value,draws:()=>draws,transforms};
}
for(const [name,change]of [['own visibility',s=>s.button.style.visibility='hidden'],['ancestor visibility',s=>s.parent.style.visibility='hidden'],['display',s=>s.parent.style.display='none'],['HTML hidden',s=>s.parent.hidden=true],['opacity zero',s=>s.button.style.opacity='0']]){
 const s=setup();s.layer.attach(s.button,{font:'source'});s.layer.update();const overlay=s.parent.children.at(-1);change(s);s.layer.update();eq(overlay.hidden,true,name+' hides sibling canvas');eq(s.button.style.color,'red',name+' retains original text presentation');s.layer.dispose();
}
{
 const s=setup();s.layer.attach(s.button,{font:'source'});s.layer.update();s.parent.remove();s.layer.update();eq(s.layer.status().elements,0,'Detached ancestor removes complete record');eq(s.parent.children.length,1,'Disconnected subtree loses its sibling canvas');eq(s.button.style.color,'red','Disconnected source style restored');
}
{
 const s=setup();s.layer.attach(s.button,{font:'source'});s.layer.update();s.env.window.devicePixelRatio=2;s.setBox({left:10,top:20,width:50,height:10});s.layer.update();eq(s.draws(),2,'Inverse CSS/DPR change redraws despite equal physical size');eq(s.transforms.at(-1),[2,0,0,2,0,0],'DPR transform refreshed');s.layer.dispose();
}
{
 const s=setup();s.setBox({left:60,top:20,width:8,height:20});s.layer.attach(s.button,{font:'source',bounds:()=>({left:30,top:20,width:60,height:20}),fallbackOnOverflow:true});s.layer.update();const overlay=s.parent.children.at(-1);eq(overlay.style.width,'60px','Actual narrow counter uses an explicit parent text field');eq(overlay.style.left,'30px','Resource icon area remains excluded');eq(s.layer.status().overflow,0,'All digits fit the real declared text field');s.layer.dispose();
}
{
 const s=setup();s.setBox({left:0,top:0,width:8,height:20});s.layer.attach(s.button,{font:'source',fallbackOnOverflow:true});s.layer.update();eq(s.layer.status().overflow,1,'Too-long resource text has explicit fallback');eq(s.button.style.color,'red','Fallback preserves readable original digits');s.layer.dispose();
}
{
 const s=setup();const listener=()=>{};s.button.onclick=listener;s.layer.attach(s.button,{font:'source'});s.layer.update();s.button.style.color='blue';s.button.style.textShadow='2px 2px green';s.layer.update();s.layer.dispose();eq(s.button.style.color,'blue','Dispose keeps the latest external palette update');eq(s.button.style.textShadow,'2px 2px green','Dispose keeps the latest external shadow update');eq(s.button.onclick,listener,'Repeated styling never changes listener');eq(s.button.textContent,'AAA','Repeated styling never changes accessible text');
}
{
 const s=setup();s.layer.attach(s.button,{font:'source'});s.layer.update();const overlay=s.parent.children.at(-1),other=s.doc.createElement('div');s.doc.body.append(other);other.append(s.button);s.layer.update();eq(overlay.parentElement,other,'Existing overlay follows a reparented live control');eq(s.parent.children.length,0,'No orphan overlay remains in former parent');s.layer.dispose();eq(other.children.length,1,'Dispose removes only presentation');
}
{
 const s=setup();s.button.style.opacity='.5';s.layer.attach(s.button,{font:'source',alpha:Math.fround(128*Math.fround(1/255)),opacityMode:'source'});s.layer.update();eq(s.parent.children.at(-1).style.opacity,'1','Native source alpha is not halved a second time by disabled CSS');s.layer.dispose();s.layer=createBitmapTextLayer({font:()=>({flags:0,spacing:0,lineHeight:10,kerning:{},glyphs:{65:{width:6}}}),draw(){}},{...s.env,createCanvas:()=>{const c=s.doc.createElement('canvas');c.getContext=()=>({setTransform(){},clearRect(){}});return c;}});s.layer.attach(s.button,{font:'app'});s.layer.update();eq(s.parent.children.at(-1).style.opacity,'.5','Application bindings keep CSS opacity');s.layer.dispose();
}
const runtimeSha256=createHash('sha256').update(await fs.readFile(new URL('../docs/original-fonts.js',import.meta.url))).digest('hex'),report={format:'native6-font-layer-lifecycle-tests-v1',status:'passed',checks,runtimeSha256,scope:'Browser-like geometry keeps visibility-hidden rectangles and detached child-parent links; inverse CSS/DPR, explicit narrow-counter text fields, overflow fallback, external refresh, reparenting and dispose. DOM transport only; no browser/original process.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native6-font-layer-lifecycle-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
