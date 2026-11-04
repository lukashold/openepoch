// Execute production HUD and action handlers against actual rules exports.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {createFrontendNamespace} from '../docs/render-namespace.js';
import {originalLabelCatalog,escapeLabelHtml} from '../docs/original-labels.js';
const arg=name=>process.argv.find(value=>value.startsWith('--'+name+'='))?.split('=').slice(1).join('=');
const url=(name,fallback)=>arg(name)?pathToFileURL(path.resolve(arg(name))):new URL(fallback,import.meta.url);
const mainURL=url('main','../docs/main.js'),wideURL=url('wasm','../docs/openepoch-ee2-up15.wasm'),mappingURL=url('mapping','../docs/original-render-namespace.js');
const {createRenderAssetMapping}=await import(mappingURL),main=await fs.readFile(mainURL),source=main.toString();
const section=source.slice(source.indexOf('const HOT ='),source.indexOf('\nconst mini ='));
assert.ok(section.includes('cmd_queue_job')&&section.includes('renderNamespace.queueJob'));
const data=JSON.parse(await fs.readFile(new URL('../docs/local-original/labels-native.json',import.meta.url)));
let checks=0;const check=(value,note)=>{assert.ok(value,note);checks++;},equal=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;};
class Node{
 constructor(tag='div'){this.tagName=tag;this.children=[];this.style={};this.dataset={};this.textContent='';this.classList={toggle(){}};this.draws=[];}
 set innerHTML(value){this.html=value;this.children=[];}get innerHTML(){return this.html||'';}
 append(...nodes){this.children.push(...nodes);}getContext(){return {drawImage:(...args)=>this.draws.push(args)};}
}
const scenarios=[];
for(const profile of [{id:'own',reference:false},{id:'base',reference:true},{id:'up15',reference:true}]){
 const binary=await fs.readFile(profile.id==='up15'?wideURL:new URL('../docs/'+(profile.id==='base'?'openepoch-ee2.wasm':'openepoch.wasm'),import.meta.url));
 const {instance:{exports:w}}=await WebAssembly.instantiate(binary,{}),renderNamespace=createFrontendNamespace(w,{referenceMode:profile.reference,assetMapping:createRenderAssetMapping(w)});
 if(profile.id==='up15')check(renderNamespace.wide,'Current UP uses the typed presentation API');
 w.init(7,1,0,1,0);w.skirmish_visibility(2);w.cheat(0);w.cheat(4);
 const n=w.render(1,0,0,256,256,-1,0,0),instances=new Float32Array(w.memory.buffer,w.inst_ptr(),n*12),tcId=renderNamespace.unitRenderId(0);
 let center;for(let i=0;i<n;i++)if(instances[i*12+4]===tcId&&instances[i*12+5]===0){center=[instances[i*12],instances[i*12+1]];break;}
 check(!!center,'Real owned Town Center is located through its typed/legacy unit render ID');
 const selectTC=()=>{w.select_none();w.select_point(...center,0,0,0,0,256,256);};selectTC();
 const readUi=()=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
 const nodes=new Map(['r0','r1','r2','pop','epoch','science','terr','idle','perf','info','acts','tip','diplomacy-button','end','endtext','queue'].map(id=>[id,new Node()]));
 const calls=[],sounds=[],messages=[],originalIconRequests=[],wasm={...w};let override;
 for(const method of ['cmd_train','cmd_queue_job'])if(typeof w[method]==='function')wasm[method]=id=>{calls.push({method,id});return w[method](id);};
 const context=vm.createContext({document:{createElement:tag=>new Node(tag)},$:id=>nodes.get(id),escapeLabelHtml,
  originalLabels:originalLabelCatalog(data,{...profile,ruleset:profile.reference?profile.id==='base'?1:2:0},{definitionCount:renderNamespace.definitionCount}),originalAssets:{},originalIcon:(_assets,id)=>{originalIconRequests.push(id);return null;},iconAtlas:{},CELL:64,
  wasm,u8:(ptr,len)=>override?new TextEncoder().encode(JSON.stringify(override)):new Uint8Array(w.memory.buffer,ptr,len),dec:new TextDecoder(),renderNamespace,
  referenceMode:profile.reference,RESOURCE_NAMES:['Nahrung','Holz','Eisen'],frameMs:16,worstMs:17,simMs:0.1,bench:0,net:null,
  economyBar:{update(){}},sciencePanel:{refresh(){}},diplomacy:{refresh(){},close(){}},nativeHud:{update(){}},nativeBitmapFonts:{update(){}},seasonSession:{settings:{}},
  sfx:{play:code=>sounds.push(code)},toast:message=>messages.push(message),console});
 vm.runInContext('let ui=null,actSig="";'+section+'\nglobalThis.refresh=()=>updateUi(0,0);globalThis.act=doAct;',context);
 const refresh=()=>{nodes.get('queue').children=[];context.refresh();return readUi();};
 let ui=refresh();equal(ui.sel.def,0,'Actual selected producer reaches the HUD');
 const train=ui.act.findIndex(action=>action.k==='t'&&action.id===8);check(train>=0,'Actual producer offers a citizen');context.act(train);w.tick();ui=refresh();
 equal(calls.at(-1),{method:renderNamespace.wide?'cmd_queue_job':'cmd_train',id:8},'Training action uses its proper production API');check(ui.sel.q.includes(8),'The real citizen order reaches the actual queue');
 check(originalIconRequests.includes(8),'Actual unit queue produces a unit icon');w.cmd_cancel();w.tick();refresh();
 if(profile.reference){
  ui=readUi();const research=ui.act.findIndex(action=>action.k==='r'&&action.ok);check(research>=0,'Granted research points make a real selected-host research action available');
  const job=ui.act[research].id;context.act(research);w.tick();ui=refresh();check(ui.sel.q.includes(job),'Actual selected-host research job reaches queue');
  equal(calls.at(-1),{method:renderNamespace.wide?'cmd_queue_job':'cmd_train',id:job},'Research action preserves the full encoded job');check(nodes.get('queue').children.some(node=>node.textContent==='⚗'),'Actual research queue displays a research glyph');
  w.cmd_cancel();w.tick();ui=refresh();const staleResearch=ui.act.findIndex(action=>action.k==='r'&&action.ok);w.select_none();context.act(staleResearch);w.tick();selectTC();
  check(!readUi().sel.q.length,'Stale selected-host action cannot fall back to an unselected producer');refresh();
 }
 if(renderNamespace.wide){
  ui=readUi();const epoch=ui.act.findIndex(action=>action.k==='e');check(epoch>=0,'Actual epoch action exists');context.act(epoch);
  equal(calls.at(-1),{method:'cmd_queue_job',id:196608},'Epoch action preserves its tagged job without narrowing');w.tick();equal(readUi().epoch,0,'Native prerequisite validation rejects premature epoch advance');
  override=readUi();override.sel.q=[100,65536,196608];refresh();const queue=nodes.get('queue').children;
  equal(queue.map(node=>node.tagName),['canvas','div','div'],'Wide unit100 remains an icon, distinct from research and epoch');equal(queue[1].textContent,'⚗','Research glyph');equal(queue[2].textContent,'⬆','Epoch glyph');
  // Def100 is a synthetic queue boundary, absent from the phase1 registry;
  // rejecting its fallback is safer than displaying resource/projectile art.
  equal(queue[0].draws.length,0,'Unknown unit fallback cannot become a resource image');override=undefined;
 }
 scenarios.push({profile:profile.id,wide:renderNamespace.wide,calls,sha256:createHash('sha256').update(binary).digest('hex')});
}
const report={format:'openepoch-native6-main-namespace-tests-v1',passed:true,checks,mainSha256:createHash('sha256').update(main).digest('hex'),scenarios,
 scope:'Actual production Main HUD/action handlers, real selected producers, queue commands/ticks and stale-selection validation. DOM/Canvas are substituted; synthetic queue boundary is explicitly marked. No browser or original executable.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native6-main-namespace-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
