import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {CIVILIZATIONS,createCivilizationSetup,resolveCivilizations,checkedCivilizations,applyCivilizations} from '../docs/civilization-setup.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
const bytes=await fs.readFile(new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url));
const {instance:{exports:w}}=await WebAssembly.instantiate(bytes,{});
let checks=0;const eq=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;};
const source=JSON.parse(await fs.readFile(new URL('../../../native-classic-next/civ8/civ8-sources.json',import.meta.url)));
eq(CIVILIZATIONS.map(c=>c.source),source.civs,'UI uses the actual source civilization enum order');
const regions=[0,0,0,0,0,1,1,1,2,2,2,3,3,3];
for(const civ of CIVILIZATIONS)eq(w.civilization_region(civ.id),regions[civ.id]);
class Element{
 constructor(tag){this.tagName=tag;this.children=[];this.style={};this.value='';}
 append(...children){this.children.push(...children);}
 before(node){this.beforeNode=node;}
}
const document={createElement:tag=>new Element(tag)},anchor=new Element('anchor');
const setup=createCivilizationSetup({wasm:w,document,anchor});eq(setup.enabled,true);eq(anchor.beforeNode.id,'civilization-options');
eq(setup.choices(),[0,15,15,15]);
const controls=anchor.beforeNode.children.slice(1).map(row=>row.children[1]);
eq(controls.map(c=>c.id),['civilization-0','civilization-1','civilization-2','civilization-3']);
for(const select of controls)eq(select.children.length,15,'all14 source civilizations and one lobby random option');
for(let civ=0;civ<14;civ++)for(let players=2;players<=4;players++){
 controls[0].value=String(civ);let draws=0;
 const values=setup.resolve(players,{random:()=>{draws++;return (civ+draws)%14;}});
 eq(draws,players-1,'unused seats never consume lobby random choices');
 w.init(713,players-1,0,1,0);eq(applyCivilizations(w,values,players),true);
 eq(Array.from({length:4},(_,i)=>w.player_civilization(i)),values,'real Core receives exactly the visible/resolved seat choices');
 const before=w.state_hash();assert.throws(()=>applyCivilizations(w,values,players));checks++;eq(w.state_hash(),before,'setup cannot be reapplied to a live configured game');
}
for(const values of [[0,1,0,0],[0,1,15,14],[0,14,14,14],[0,1,14],null,[0,1,14,Infinity]]){
 assert.throws(()=>checkedCivilizations(values,2));checks++;
}
assert.throws(()=>checkedCivilizations(new Array(4),2));checks++;
for(const random of [()=>14,()=>15,()=>-1,()=>NaN,()=>.5]){assert.throws(()=>resolveCivilizations([15,15,15,15],2,{random}));checks++;}
eq(applyCivilizations({},null,2),false,'older rules remain unchanged');
assert.throws(()=>applyCivilizations({},[0,1,14,14],2));checks++;
const unusedAnchor=new Element('anchor');eq(createCivilizationSetup({wasm:{},document,anchor:unusedAnchor}).enabled,false);eq(unusedAnchor.beforeNode,undefined);
const report={format:'native-civ8-civilization-menu-component-v1',passed:true,checks,wasmSha256:sha(bytes),sourceSha256:sha(await fs.readFile(new URL('../docs/civilization-setup.js',import.meta.url))),
 limits:['Actual14-civilization Core configuration and source enum; DOM transport is substituted. Main start/save/network wiring is still a separate integration step.',
 'Lobby random choices use the host presentation RNG once and are sent explicitly; no original native lobby RNG or source initialization parity is claimed.']};
await fs.writeFile(new URL('../../civilization-menu-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
