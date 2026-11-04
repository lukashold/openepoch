// Actual Main action construction/dispatch with normally produced saved units.
import fs from 'node:fs/promises';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const bytes=await fs.readFile(new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url));
const mainBytes=await fs.readFile(new URL('../docs/main.js',import.meta.url)),main=mainBytes.toString();
const at=main.indexOf('  const p = wasm.ui();',main.indexOf('function updateUi(')),end=main.indexOf('  for (let i = 0; i < 3;',at),uiSource=main.slice(at,end);
const actionStart=main.indexOf('function doAct('),actionEnd=main.indexOf('const mini =',actionStart),actionSource=main.slice(actionStart,actionEnd);
assert.ok(at>0&&end>at&&actionStart>0&&actionEnd>actionStart);
const receipt=JSON.parse(await fs.readFile(new URL('./fixtures/civ8-produced/receipt.json',import.meta.url)));
let checks=0;const eq=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;},cases=[];
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
for(const id of [65,68,76,78]){
 const fixture=receipt.fixtures.find(f=>f.def===id),raw=await fs.readFile(new URL('./fixtures/civ8-produced/'+fixture.name,import.meta.url));eq(sha(raw),fixture.sha256);
 const {instance:{exports:w}}=await WebAssembly.instantiate(bytes,{});w.init(7,0,0,1,0);
 const ptr=w.load_alloc(raw.length);new Uint8Array(w.memory.buffer,ptr,raw.length).set(raw);eq(w.load_apply(),1,'real OEU8/CIV1 produced snapshot loads into joint PKW1');
 const events=[];const context=vm.createContext({wasm:w,dec:new TextDecoder(),u8:(p,n)=>new Uint8Array(w.memory.buffer,p,n),
  sfx:{play:value=>events.push(['effect',value])},selectedUnitVoice:action=>{events.push(['voice',action]);return true;}});
 vm.runInContext('let ui=null,lastUi=0;'+actionSource+';globalThis.actions={refresh(){'+uiSource+'return ui;},dispatch:doAct};',context);
 const ui=context.actions.refresh();eq(ui.sel.def,id);eq(ui.act.filter(a=>a.k==='p').map(a=>a.id).join(','),'0,1');
 const before=save(w),beforeHash=w.state_hash();for(let i=0;i<100;i++)eq(w.pack_selected_count(),1,'eligibility comes from source capability and real selected actor');
 eq(w.state_hash(),beforeHash);eq(save(w),before,'selection query and actual Main UI construction preserve full save bytes');
 context.actions.dispatch(ui.act.findIndex(a=>a.k==='p'&&a.id===1));eq(events,[['voice','order']],'Main invokes the real manual Unpack command and selected unit acknowledgment');
 const read=()=>{
  const guard=save(w),hash=w.state_hash();w.render(1,fixture.unit.x-5,fixture.unit.y-5,fixture.unit.x+5,fixture.unit.y+5,-1,0,0);
  const count=w.render_meta_count(),v=new DataView(w.memory.buffer),meta=w.render_meta_ptr(),pack=w.render_pack_ptr();let found;
  for(let i=0;i<count;i++)if(v.getUint32(meta+i*64+4,true)===id){found={handle:v.getUint32(meta+i*64,true),flags:v.getUint32(pack+i*32+4,true),phase:v.getUint32(pack+i*32+8,true),visual:v.getUint32(pack+i*32+12,true),duration:v.getFloat32(pack+i*32+20,true),capable:v.getUint32(pack+i*32+28,true)};break;}
  assert.ok(found);checks++;eq(w.state_hash(),hash);eq(save(w),guard,'render metadata does not advance pack state');return found;
 };
 const unpack=read();eq(unpack.capable,1);eq(unpack.flags&10,10,'logical unpack/manual flags belong to the actual opaque handle');
 for(let tick=0;tick<180;tick++)w.tick();
 context.actions.refresh();context.actions.dispatch(context.actions.refresh().act.findIndex(a=>a.k==='p'&&a.id===0));const pack=read();eq(pack.flags&10,0,'manual Pack clears native unpack/manual flags through the command path');
 w.select_none();eq(w.pack_selected_count(),0);eq(context.actions.refresh().act.filter(a=>a.k==='p').length,0,'manual controls disappear with no eligible selected unit');
 cases.push({id,handle:unpack.handle,unpack,pack,sourceSaveSha256:fixture.sha256,normalProductionTicks:fixture.trainTicks});
}
const report={format:'native-joint8-real-main-manual-pack-controls-v1',passed:true,checks,wasmSha256:sha(bytes),mainSha256:sha(mainBytes),cases,
 limits:['Actual Main UI action prefix/handler evaluated with a substituted DOM/audio feedback transport. Public normally-produced source snapshots, actual Core command transitions and opaque-handle render metadata; GPU phase and full native AI parity are separate.']};
await fs.writeFile(new URL('../../main-pack-controls-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
