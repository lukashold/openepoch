import fs from 'node:fs';import path from 'node:path';import {fileURLToPath} from 'node:url';import {createHash} from 'node:crypto';import assert from 'node:assert/strict';
import {save,load,persistedState} from './roster6-state.mjs';import {assertCurrentSave,saveLayout} from './save-fixture.mjs';
const fixtureArg=process.argv.find(arg=>arg.startsWith('--fixtures='));
if(!fixtureArg)throw Error('Historical57-role equivalence tests require the archived build fixture directory: --fixtures=PATH (baseline5/ and builds/). The active61-role binary is not a valid57-role oracle.');
const work=path.resolve(fixtureArg.slice(11)),sha=b=>createHash('sha256').update(b).digest('hex');
const files={up6:'builds/openepoch-roster6.wasm',up5:'baseline5/openepoch-ee2-up15.wasm',old0:'baseline5/openepoch.wasm',old1:'baseline5/openepoch-ee2.wasm',new0:'builds/openepoch-own.wasm',new1:'builds/openepoch-base.wasm',rebuilt5:'builds/openepoch-up5.wasm'};
const blobs=Object.fromEntries(Object.entries(files).map(([k,p])=>[k,fs.readFileSync(path.join(work,p))]));const make=async k=>(await WebAssembly.instantiate(blobs[k],{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.deepEqual(a,b,m);checks++;};const good=(v,m)=>{assert.ok(v,m);checks++;};const report={format:'native-roster6-phase1-wasm-v1',binaries:Object.fromEntries(Object.entries(blobs).map(([k,b])=>[k,{sha256:sha(b),bytes:b.length}])),equivalence:[]};
for(const[newKey,oldKey]of[['new0','old0'],['new1','old1'],['rebuilt5','up5']])for(const seed of[7,41])for(const epoch of(newKey==='new0'?[0,2,5,7]:[0,5,10,14])){
 const a=await make(oldKey),b=await make(newKey);for(const w of[a,b]){w.init(seed,2,0,1,0);eq(w.skirmish_epoch(epoch),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);eq(w.cmd_train(8),1);}
 for(let t=0;t<=1200;t++){if(t%200===0){eq(a.state_hash(),b.state_hash(),newKey+' exact frozen hash');eq(save(a),save(b),newKey+' byte-identical saved state');}if(t<1200){a.tick();b.tick();}}
 report.equivalence.push({profile:newKey,seed,epoch,ticks:1200,hash:b.state_hash()>>>0,saveSha256:sha(save(b))});
}
const w=await make('up6');eq(w.schema_revision(),6);eq(w.definition_count(),57);eq(w.render_range_len(),626,'range length before init');eq(w.ranges_len(),626,'range alias before init');for(let raw=0;raw<313;raw++){const kind=w.render_asset_kind(raw)>>>0;const source=w.render_asset_source_id(raw)>>>0;if(kind===0)eq(source,raw-256);else if(kind===1)eq(source,raw+100);else if(kind===2)eq(source,raw-16);else if(kind===3)eq(source,raw+200);else if(kind===4)eq(source,raw+78);else eq(source,0xffffffff);}w.init(7,1,0,1,0);eq(w.schema_revision(),6);eq(w.ruleset(),2);eq(w.definition_count(),57);eq(w.render_range_len(),626);eq(w.ranges_len(),626);
for(const id of[0,56,63,64,99,100,243,255,256,310,3035,65535]){eq(w.queue_job_encode(0,id),id);eq(w.queue_item_kind(id),0);eq(w.queue_item_value(id),id);}
for(let id=0;id<180;id++){const item=w.queue_job_encode(1,id);eq(item,0x10000+id);eq(w.queue_item_kind(item),1);eq(w.queue_item_value(item),id);}
eq(w.queue_job_encode(1,180)>>>0,0xffffffff);eq(w.queue_job_encode(0,65536)>>>0,0xffffffff);eq(w.queue_item_kind(0x100b4)>>>0,0xffffffff);eq(w.queue_item_kind(0x40000)>>>0,0xffffffff);eq(w.queue_item_kind(0x30000),3);eq(w.queue_item_value(0x30000),0);
for(let id=0;id<57;id++){eq(w.render_asset_kind(256+id),0);eq(w.render_asset_source_id(256+id),id);good((w.definition_flags(id)>>>0)!==0xffffffff);}
for(let id=0;id<11;id++){eq(w.render_asset_kind(id),1);eq(w.render_asset_source_id(id),100+id);}for(let id=16;id<23;id++){eq(w.render_asset_kind(id),2);eq(w.render_asset_source_id(id),id-16);}eq(w.definition_flags(57)>>>0,0xffffffff);
// Actual exported selected-action ABI, with legacy full-tree fallback separate.
const qa=await make('up6'),qb=await make('up6');for(const q of[qa,qb]){q.init(71,1,0,1,0);q.cheat(0);q.cheat(4);q.select_point(q.start_x(),q.start_y(),0,0,0,0,256,256);}
eq(qa.cmd_queue_job(8),1);eq(qb.cmd_train(8),1);eq(qa.state_hash(),qb.state_hash());eq(save(qa),save(qb),'unit queue API equals existing selected train');
eq(qa.cmd_queue_job(0x10000),1);eq(qb.cmd_research(0),1);eq(save(qa),save(qb),'research tagged job stays intact');
for(const bad of[0x100b4,0x20000,0x30001,0x40000,0xffffffff]){const before=save(qa);eq(qa.cmd_queue_job(bad),0);eq(save(qa),before);}
const noSelection=await make('up6');noSelection.init(71,1,0,1,0);noSelection.cheat(0);noSelection.cheat(4);noSelection.select_none();const beforeNoSelection=save(noSelection);eq(noSelection.cmd_queue_job(0x10000),0);eq(save(noSelection),beforeNoSelection);eq(noSelection.cmd_research(0),1,'full tree global fallback remains intentional');
const migration=[];
for(const seed of[7,41])for(const epoch of[0,5,10,14]){
 const a=await make('up5'),b=await make('up6');a.init(seed,2,0,1,0);a.skirmish_epoch(epoch);a.select_point(a.start_x(),a.start_y(),0,0,0,0,256,256);a.cmd_train(8);const original=save(a);eq(load(a,original),1);eq(load(b,original),1,'Real frozenOEU5 migrates');
 for(let tick=0;tick<=1200;tick++){if(tick%200===0){const old=save(a),now=save(b);eq(persistedState(old),persistedState(now),'Every persisted field is preserved across actual5→6 simulation at'+tick);eq(assertCurrentSave(now).descriptorBytes,52);if(tick===400){const p=await make('up6');eq(load(p,now),1,'OEU6 roundtrip');eq(p.state_hash(),b.state_hash());eq(save(p),save(b),'OEU6 canonical save');}}
 if(tick<1200){a.tick();b.tick();}}
 migration.push({seed,epoch,ticks:1200,oldSaveSha256:sha(save(a)),newSaveSha256:sha(save(b))});
}
report.migration=migration;
// Malformed legacy and new DTOs must never replace a previously loaded world.
const a=await make('up5');a.init(17,1,0,1,0);const legacy=save(a);eq(load(w,legacy),1);const valid=save(w),before=w.state_hash(),cases=[];
for(const original of[legacy,valid]){const l=saveLayout(original),d=l.d,wide=l.roster6,stride=d.getUint32(4,true),ents=36+4*d.getUint32(8,true);
 for(const[at,value,size]of[[0,0,4],[4,149,4],[8,95,4],[12,58,4],[32,8193,4],[36+25,2,1],[36+32,255,1],[36,0x7fc00000,4],[ents,2,1],[ents+78,2,1],[ents+8,0x7fc00000,4],[ents+40,0x7f800000,4],[ents+(wide?104:89),6,1],[ents+(wide?172:145),11,1]]){const b=original.slice(),q=new DataView(b.buffer);size===1?q.setUint8(at,value):q.setUint32(at,value,true);cases.push(b);}
 for(const n of[1,4,8,19,20,51,52,100]){if(!wide&&n===l.attackBytes)continue;cases.push(original.slice(0,-n));}cases.push(new Uint8Array([...original,0]));
 if(wide){for(const[at,value,size]of[[ents+2,256,2],[ents+84,0x100b4,4],[ents+84,0x20000,4],[ents+84,0x30001,4]]){const b=original.slice(),q=new DataView(b.buffer);size===2?q.setUint16(at,value,true):q.setUint32(at,value,true);if(size===4)b[ents+104]=1;cases.push(b);}const b=original.slice(),q=new DataView(b.buffer);q.setUint16(ents+142,256,true);b[ents+172]=1;cases.push(b);const worker=Array.from({length:d.getUint32(32,true)},(_,i)=>i).find(i=>d.getUint16(ents+i*stride+2,true)===8&&original[ents+i*stride]);if(worker!==undefined){const b=original.slice();new DataView(b.buffer).setUint32(ents+worker*stride+84,0x10001,true);cases.push(b);}}
 if(wide){for(const at of[l.descriptor,l.descriptor+4,l.descriptor+8,l.descriptor+12,original.length-8,original.length-4]){const b=original.slice();b[at]^=1;cases.push(b);}}
}
for(const bad of cases){eq(load(w,bad),0,'Malformed portable/legacy DTO rejects');eq(w.state_hash(),before,'Failed load preserves hash');eq(save(w),valid,'Failed load preserves complete world');}
for(const key of['old0','old1']){const q=await make(key);q.init(7,1,0,1,0);eq(load(w,save(q)),0,'Foreign0/1 save rejects');}
const old=await make('up5');old.init(7,1,0,1,0);const oldBefore=save(old),oldHash=old.state_hash();eq(load(old,valid),0,'Frozen5 cannot misread6');eq(old.state_hash(),oldHash);eq(save(old),oldBefore);
report.status='PASS';report.checks=checks;report.malformedCases=cases.length;report.limits=['Phase1 retains exactly57 gameplay roles; full classic roster expansion is separate.','Projectiles remain transient exactly as in5; migration comparisons begin from the same no-flight initial state.'];fs.writeFileSync(path.join(work,'phase1-wasm-report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:report.status,checks,migrationCases:migration.length,equivalenceCases:report.equivalence.length,malformedCases:cases.length,binaries:report.binaries}));
