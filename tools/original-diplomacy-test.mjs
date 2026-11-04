import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {diplomacyPanel} from '../docs/diplomacy.js';
import {enhanceOriginalDiplomacy} from '../docs/original-diplomacy.js';
import {calculateDiplomacyLayout,sourceMoneyRule,nativeDiplomacyTextStyle,modulatePlayerStrip} from '../docs/original-diplomacy-layout.js';
import {makeDOM,flattenDOM} from './diplomacy-dom-fixture.mjs';

const reportName=process.argv.find(arg=>arg.startsWith('--report='))?.slice(9)||'original-diplomacy-tests.json';
if(!/^[a-z0-9-]+\.json$/i.test(reportName))throw Error('Invalid report filename');
const reportURL=new URL(reportName,new URL('../../EE2-Research/analysis/',import.meta.url));
const manifest=JSON.parse(await fs.readFile(new URL('../docs/local-original/diplomacy-native/manifest.json',import.meta.url)));
const sha=b=>createHash('sha256').update(b).digest('hex');
let checks=0;const check=(ok,message)=>{assert.ok(ok,message);checks++;},equal=(a,b,message)=>{assert.deepEqual(a,b,message);checks++;};
const none=0xffffffff;
const fixture=()=>({epoch:5,res:Array(9).fill(2500),resourceActive:[true,true,true,true,true,true,false,false,false],diplomacy:{local:0,visibility:2,log:[],territoryPolicy:'Eigene Gebäude werden übergeben.',players:Array.from({length:4},(_,team)=>({team,name:'Spieler '+(team+1),ai:team>0,alive:true,relation:team?1:2,theirRelation:team?1:2,sharedVision:false,incoming:0,outgoing:0,cooldown:0,territoryIncoming:null,territoryOutgoing:null,territoryCooldown:0})),territories:[{id:0,name:'Westland',owner:0,x:50,y:50,known:true,canGive:true,transferable:true,buildings:2,units:1,garrison:0,countsComplete:true,lastTerritory:false},{id:1,name:'Ostland',owner:1,x:180,y:50,known:true,canGive:false,transferable:true,buildings:2,countsComplete:true},{id:2,name:'Nordland',owner:255,x:80,y:180,known:true,buildings:0,countsComplete:true}]}});
function fakeWasm(state){
 const memory={buffer:new ArrayBuffer(4*1024*1024)},calls=[];const image=new Uint8Array(memory.buffer,1024*1024,256**2*4);for(let i=0;i<256**2;i++)image.set([60,90,60,255],i*4);let length=0,reject=false;
 return {memory,calls,state,ui(){const bytes=new TextEncoder().encode(JSON.stringify(state));length=bytes.length;new Uint8Array(memory.buffer,16,length).set(bytes);return 16;},ui_len:()=>length,minimap:()=>1024*1024,territory_at:(x,y)=>y>128?2:x<128?0:1,cmd_diplomacy(...args){calls.push(['diplomacy',...args]);return reject?0:1;},cmd_territory(...args){calls.push(['territory',...args]);return reject?0:1;},reject(value){reject=value;}};
}
const savedGlobals=Object.fromEntries(['document','window','Option','ImageData'].map(key=>[key,globalThis[key]]));
async function environment(wasm){
 const dom=makeDOM();Object.assign(globalThis,dom);const bottom=dom.document.createElement('div');bottom.id='bottom';dom.document.body.append(bottom);const trigger=dom.document.createElement('button');trigger.id='diplomacy-button';dom.document.body.append(trigger);
 const events={orders:0,opens:0,closes:0,focus:[]};
 const panel=diplomacyPanel({wasm,referenceMode:true,onOrder:()=>events.orders++,onOpen:()=>events.opens++,onClose:()=>events.closes++,onTerritoryFocus:t=>events.focus.push(t.id)});
 const dialog=dom.document.getElementById('diplomacy-dialog'),originalButtons=dialog.querySelectorAll('button'),handlers=new Map(originalButtons.map(node=>[node,node.onclick]));
 const options={wasm,document:dom.document,window:dom.window,manifestURL:new URL('../docs/local-original/diplomacy-native/manifest.json',import.meta.url),fetchFile:async()=>({ok:true,json:async()=>manifest})};
 const native=await enhanceOriginalDiplomacy(panel,options);check(native.enabled&&!native.errors.length,'Native presentation attaches without replacing original controllers');panel.open();
 const find=text=>dialog.querySelectorAll('button').find(node=>node.textContent===text);
 const data=(prefix)=>dialog.querySelectorAll('button').filter(node=>node.dataset.nativeWidget?.startsWith(prefix));
 return {...dom,wasm,panel,dialog,native,events,handlers,find,data,options};
}
try{
 for(const width of [800,1366,1600,1920,2048,2560])for(const height of [600,768,1200,1440])for(const meta of [1,2,3]){
  const layout=calculateDiplomacyLayout(manifest,width,height,{playerCount:9,meta}),root=layout.get('diplo_panel'),proposal=layout.get('prop_panel');
  equal(root.x,Math.trunc((width-Math.trunc(1160*width/1600))/2),'NativeA7F930 centered diplomacy content x');equal(root.y,Math.trunc((height-Math.trunc(885*height/1200))/2),'NativeA7F930 centered diplomacy content y');
  equal(proposal.x,Math.trunc((width-Math.trunc(800*width/1600))/2),'NativeA7F930 centered proposal content x');
  for(let n=1;n<=9;n++){const name=layout.get(`player${n}__diplo_pl_name`),previous=layout.get(`diplo_tribbutt_pl${n}`);check(name.x>=previous.x&&name.y>=previous.y,'Player template resolves inside its actual row context');const money=layout.get(`resources${n}__diplo_tribres_0`);check(money.width>0&&money.height>0,'Original resource control has valid per-player bounds');}
 }
 equal(sourceMoneyRule(manifest.widgets.diplo_tribres_0),{increment:100,shiftIncrement:500,min:0,max:9900},'Original money control argument contract');
 const font=nativeDiplomacyTextStyle(manifest,manifest.widgets.diplo_panel_title,1280);equal(font.color,'rgb(255,255,255)','Source title font color');equal(font.fontSize,'21px','Source autofont resolution tier');equal(font.textAlign,'left','Source LeftMid alignment');
 for(const record of Object.values(manifest.playerStripPixels))for(const color of [[68,143,255],[239,69,64],[80,206,98]]){const source=Buffer.from(record.rgba,'base64'),out=modulatePlayerStrip(source,color);check(out.every((value,index)=>value===(index%4===3?source[index]:Math.floor(source[index]*color[index%4]/255+.5))),'Every player-strip RGB byte is modulated and source alpha preserved');}
 const state=fixture(),wasm=fakeWasm(state),e=await environment(wasm);let {native,panel,dialog}=e;
 equal(e.events.opens,1,'Native opening calls the original lifecycle once');equal(native.status().meta,2,'Epoch6 selects source parchment theme');
 for(const [node,handler]of e.handlers)equal(node.onclick,handler,'Every original command/close handler remains the same function');
 check(dialog.querySelectorAll('div').some(node=>node.dataset.nativeFrame==='spr_frame_parch_corner_ul'),'Source frame corners are drawn');
 check(dialog.querySelectorAll('div').some(node=>node.dataset.nativePlayerColor==='239,69,64'&&node.style.backgroundImage.startsWith('url("data:')),'Native player color binds the actual source RGBA strip');
 equal(e.data('actions').length,9,'Three opponents use native proposal/peace/war rows');
 const peace=e.data('actions1__').find(node=>node.dataset.command==='neutral');state.diplomacy.players[1].relation=state.diplomacy.players[1].theirRelation=0;panel.refresh(state);peace.click();equal(wasm.calls.at(-1),['diplomacy',1,0,0,0],'Native peace proxy invokes original live command');
 state.diplomacy.players[1].relation=state.diplomacy.players[1].theirRelation=1;panel.refresh(state);e.data('actions1__').find(node=>node.dataset.command==='war').click();equal(wasm.calls.at(-1),['diplomacy',1,2,0,0],'Native war proxy invokes original live command');
 native.openProposal(0);equal(native.status().proposalOpen,true,'Source proposal panel opens without issuing a treaty');
 e.find('Bündnis anbieten').click();equal(wasm.calls.at(-1),['diplomacy',1,1,1,0],'Original alliance and shared-vision choice stay intact');
 for(const [field,button,action]of [['incoming','Annehmen',3],['incoming','Ablehnen',4],['outgoing','Zurückziehen',6]]){state.diplomacy.players[1].incoming=0;state.diplomacy.players[1].outgoing=0;state.diplomacy.players[1][field]=1;panel.refresh(state);e.find(button).click();equal(wasm.calls.at(-1),['diplomacy',1,action,0,0],'Existing treaty review command '+action);}
 state.diplomacy.players[1].incoming=state.diplomacy.players[1].outgoing=0;state.diplomacy.players[1].relation=state.diplomacy.players[1].theirRelation=2;panel.refresh(state);e.find('Gemeinsame Sicht anbieten').click();equal(wasm.calls.at(-1),['diplomacy',1,7,0,1],'Existing shared-vision command');
 native.setView('resources');const cell=e.data('resources1__')[0];cell.click();cell.click({shiftKey:true});equal(cell.textContent,'600','Source click100/shift500 staging');cell.oncontextmenu({preventDefault(){},shiftKey:false});equal(cell.textContent,'500','Source right-click decrement');
 dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_accept').click();equal(wasm.calls.at(-1),['diplomacy',1,5,0,500],'Native staged tribute delegates to the unchanged gift form');equal(native.status().staged.length,0,'Sent tribute clears its native staging entry');
 const second=e.data('resources2__')[0];for(let i=0;i<7;i++)cell.click({shiftKey:true});second.click({shiftKey:true});check(Number(cell.textContent)+Number(second.textContent)<=state.res[0],'Staging across recipients never overcommits stock');
 dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_clear').click();equal(native.status().staged.length,0,'Native clear resets all staged recipients');
 const metalCell=e.data('resources1__')[5];metalCell.click();state.epoch=14;state.resourceActive=[true,true,false,true,true,false,false,true,true];panel.refresh(state);equal(native.status().staged.length,0,'Epoch resource replacement cannot send stale Tin/Iron as another resource');equal(native.status().meta,3,'Source metal theme is selected');
 const beforeTerr=wasm.calls.length;native.setView('territories');dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_accept').click();check(!dialog.querySelector('.territory-review').hidden,'Native territory footer opens unchanged mandatory gift review');equal(wasm.calls.length,beforeTerr,'Gift review itself never issues an irreversible command');e.find('Übergabe bestätigen').click();equal(wasm.calls.at(-1),['territory',1,0,0,none],'Original reviewed territory gift');
 dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_clear').click();check(native.status().proposalOpen,'Native territory footer opens source contract panel');equal(wasm.calls.length,beforeTerr+1,'Opening contract never emits an order');dialog.dispatchEvent({type:'cancel'});
 e.find('Territorientausch anbieten').click();equal(wasm.calls.at(-1),['territory',1,1,0,1],'Original territory exchange offer');
 state.diplomacy.players[1].territoryIncoming={give:1,want:0,expires:60};panel.refresh(state);e.find('Tausch annehmen').click();equal(wasm.calls.at(-1),['territory',1,2,none,none],'Original territory accept');e.find('Tausch ablehnen').click();equal(wasm.calls.at(-1),['territory',1,3,none,none],'Original territory reject');
 state.diplomacy.players[1].territoryOutgoing={give:0,want:1,expires:60};panel.refresh(state);e.find('Tauschangebot zurückziehen').click();equal(wasm.calls.at(-1),['territory',1,4,none,none],'Original territory withdrawal');
 const territoriesBefore=state.diplomacy.territories;state.diplomacy.territories=territoriesBefore.filter(area=>area.owner!==1);panel.refresh(state);check(dialog.querySelectorAll('div').find(node=>node.dataset.nativeMapMarker==='demand').hidden,'No requested territory does not become a false territory0 marker');state.diplomacy.territories=territoriesBefore;panel.refresh(state);
 const rejected=wasm.calls.length;state.diplomacy.players[1].alive=false;panel.refresh(state);e.data('actions1__').find(node=>node.dataset.command==='proposal').click();equal(wasm.calls.length,rejected,'Dead players never receive a proxy command');
 state.diplomacy.players[1].alive=true;state.diplomacy.players[1].territoryIncoming=state.diplomacy.players[1].territoryOutgoing=null;state.diplomacy.players=state.diplomacy.players.slice(0,3);panel.refresh(state);equal(e.data('actions').length,6,'Roster reduction removes stale source row controls');
 native.setView('relations');native.openProposal(0);dialog.dispatchEvent({type:'cancel'});equal(native.status().proposalOpen,false,'Escape first closes the source proposal layer');check(dialog.open,'Closing proposal keeps F4 open');
 state.epoch=0;state.resourceActive=[true,true,false,true,true,true,false,false,false];panel.refresh(state);await fs.writeFile(new URL('../../../work/native-diplomacy-ui/snapshot-relations.json',import.meta.url),JSON.stringify(flattenDOM(e.document)));
 native.setView('resources');e.data('resources1__')[0].click({shiftKey:true});await fs.writeFile(new URL('../../../work/native-diplomacy-ui/snapshot-resources.json',import.meta.url),JSON.stringify(flattenDOM(e.document)));
 native.openProposal(0);await fs.writeFile(new URL('../../../work/native-diplomacy-ui/snapshot-proposal.json',import.meta.url),JSON.stringify(flattenDOM(e.document)));
 native.setView('territories');await fs.writeFile(new URL('../../../work/native-diplomacy-ui/snapshot-territories.json',import.meta.url),JSON.stringify(flattenDOM(e.document)));
 native.dispose();check(!native.enabled,'Native presentation disposes reversibly');check(!dialog.classList.contains('native-diplomacy'),'Original dialog class restored');for(const [node,handler]of e.handlers)equal(node.onclick,handler,'Disposal preserves original live command handler');
 check(e.document.getElementById('dip-resource')===dialog.querySelector('#dip-resource'),'Gift control remains connected after disposal');
 // Two real production WASMs additionally prove that native tribute dispatch
 // really debits stock, rather than merely reaching a mocked method name.
 const actual=[];
 for(const file of ['openepoch-ee2.wasm','openepoch-ee2-up15.wasm']){
  const bytes=await fs.readFile(new URL('../docs/'+file,import.meta.url)),real=(await WebAssembly.instantiate(bytes,{})).instance.exports;real.init(71,1,0,1,0);real.skirmish_visibility(2);real.cheat(0);real.cheat(2);const calls=[];
  const wrapped={...real,cmd_diplomacy(...args){calls.push(args);return real.cmd_diplomacy(...args);}};
  const q=await environment(wrapped),before=real.resource_stat(0,0);q.native.setView('resources');q.data('resources1__')[0].click();q.dialog.querySelectorAll('button').find(node=>node.dataset.nativeWidget==='diplo_tribute_accept').click();equal(calls.at(-1),[1,5,0,100],'Actual production WASM receives native UI tribute');equal(real.resource_stat(0,0),before-100,'Actual production stock changes through existing command validation');if(file.includes('up15')){q.native.setView('territories');check(q.dialog.querySelector('canvas').pixels.data.length===256**2*4,'Actual production territory map is drawn');await fs.writeFile(new URL('../../../work/native-diplomacy-ui/snapshot-actual-up-territories.json',import.meta.url),JSON.stringify(flattenDOM(q.document)));}q.native.dispose();actual.push({file,ruleset:real.ruleset(),sha256:sha(bytes)});
 }
 const report={format:'ee2-native-diplomacy-tests-v1',passed:true,checks,widgets:Object.keys(manifest.widgets).length,sprites:Object.keys(manifest.sprites).length,actualWasmTribute:actual,sourceHashes:Object.fromEntries(await Promise.all(['original-diplomacy.js','original-diplomacy-layout.js','original-hud-layout.js','local-original/diplomacy-native/manifest.json'].map(async name=>[name,sha(await fs.readFile(new URL('../docs/'+name,import.meta.url)))]))),productionControllers:Object.fromEntries(await Promise.all(['diplomacy.js','territory-panel.js'].map(async name=>[name,sha(await fs.readFile(new URL('../docs/'+name,import.meta.url)))]))),scope:'Actual existing diplomacy/territory controller DOM nodes and handlers, native presentation, source-centered geometry and source-declared font styles; DOM/canvas transport substituted. All existing command types exercised against spies, plus actual BASE1/UP2 stock debit through two production WASMs. No browser, original executable or UI automation launched.',limits:['Browser/CSS raster is not simulated by this DOM transport; exact source crop previews and independent image tests are separate.','Complete native treaty simulation, unit tribute and civilization/team data remain unbound. This controller suite does not load the separate active bitmap-font layer; native6-font-ui-tests covers that integration.']};
 await fs.writeFile(reportURL,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({passed:true,checks,actual}));
}finally{for(const [key,value]of Object.entries(savedGlobals))if(value===undefined)delete globalThis[key];else globalThis[key]=value;}
