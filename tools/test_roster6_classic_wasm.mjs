import fs from 'node:fs';import path from 'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load,persistedState}from'./roster6-state.mjs';import{saveLayout,refreshFixtureCityGround}from'./save-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),roster=path.resolve(work,'..'),sha=b=>createHash('sha256').update(b).digest('hex');
const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
const paths={classic:option('wasm',path.join(root,'docs/openepoch-ee2-roster6.wasm')),old57:option('old57',path.join(root,'docs/openepoch-ee2-roster6-phase1.wasm')),old5:option('old5',path.join(root,'tools/fixtures/openepoch-ee2-up15-native5.wasm'))};
for(const [key,file]of Object.entries(paths))assert.ok(fs.existsSync(file),`${key} fixture missing: ${file}. Use --wasm=, --old57= and --old5= to select the archived release binaries explicitly.`);
const blobs=Object.fromEntries(Object.entries(paths).map(([k,p])=>[k,fs.readFileSync(p)]));const make=async k=>(await WebAssembly.instantiate(blobs[k],{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};const good=(a,m)=>{assert.ok(a,m);checks++;};
const sameBytes=(a,b,m)=>{eq(sha(a),sha(b),m);};
function projection(b){const p=persistedState(b);p.head[0]=0;return p;}
function sameState(a,b,m){const x=projection(a),y=projection(b);for(const key of Object.keys(x)){eq(sha(JSON.stringify(x[key])),sha(JSON.stringify(y[key])),m+' '+key);}}
const report={format:'native-classic6-actual-wasm-v1',binaries:Object.fromEntries(Object.entries(blobs).map(([k,b])=>[k,{sha256:sha(b),bytes:b.length}])),migrations:[]};
const genuine5=await make('old5');genuine5.init(7,0,0,1,0);eq(saveLayout(save(genuine5)).roster6,false,'The old5 migration oracle must be an actual pre-OEU6 binary');
const api=await make('classic');eq(api.definition_count(),61);eq(api.render_range_len(),634,'static pre-init capacity');for(let id=57;id<61;id++){eq(api.render_asset_kind(256+id),0);eq(api.render_asset_source_id(256+id),id);eq(api.definition_flags(id)&1,id>=59?1:0);}
// Both released user save families migrate through the actual public loader.
for(const key of['old5','old57'])for(const seed of[7,41])for(const epoch of[0,5,10,14]){
 const a=await make(key),b=await make('classic');a.init(seed,0,0,2,0);let old=save(a);const l=saveLayout(old);for(let t=0;t<4;t++)old[36+t*l.d.getUint32(8,true)+24]=epoch;const at=36+4*l.d.getUint32(8,true),stride=l.d.getUint32(4,true),tc=Array.from({length:l.d.getUint32(32,true)},(_,i)=>at+i*stride).find(o=>old[o]&&(l.roster6?l.d.getUint16(o+2,true):old[o+1])===0&&old[o+(l.roster6?4:2)]===0);good(tc!==undefined);if(l.roster6)l.d.setUint32(tc+84,8,true);else old[tc+84]=8;old[tc+(l.roster6?104:89)]=1;old=refreshFixtureCityGround(old);
 eq(load(a,old),1);eq(load(b,old),1,key+' migrates');eq(saveLayout(save(b)).d.getUint32(12,true),61);
 for(let t=0;t<=800;t++){
  if(t%200===0){sameState(save(a),save(b),key+' persisted state@'+t);if(key==='old57')eq(a.state_hash(),b.state_hash(),key+' exact simulation hash');const c=await make('classic'),s=save(b);eq(load(c,s),1);sameBytes(save(c),s,'canonical61 save');eq(c.state_hash(),b.state_hash());}
  if(t<800){a.tick();b.tick();}
 }
 report.migrations.push({source:key,seed,epoch,ticks:800,oldSaveSha256:sha(save(a)),newSaveSha256:sha(save(b))});
}
// Current-epoch scalar diagnostics against the source-derived audit, all16 rows.
const units=JSON.parse(fs.readFileSync(path.join(root,'docs/ee2-roster6-units.json'),'utf8'));
for(const r of units.class_rows.filter(r=>r.id>=57)){
 const w=await make('classic');w.init(7,0,0,1,0);eq(w.skirmish_epoch(r.epoch),1);
 for(const[field,key]of[[0,'hp'],[1,'buildtime'],[2,'sight'],[3,'raw_speed'],[4,'range'],[5,'reload'],[6,'damage'],[7,'pop']]){
  const v=r.fields[key].value;if(v!==null)eq(w.up15_stat(0,r.id,field),Math.fround(v),'compiled61 '+r.source_unit+' '+key+' E'+r.epoch);
 }
}
// Current arithmetic-adapter permanent-sum regression (native setter FPU mode unresolved): real Military3_1 and Military8_1.
const perm=await make('classic');perm.init(41,0,0,1,0);eq(perm.skirmish_epoch(7),1);let permSave=save(perm),pl=saveLayout(permSave);for(const k of[28,88]){const at=pl.end+36+Math.floor(k/32)*4;pl.d.setUint32(at,pl.d.getUint32(at,true)|(1<<(k%32)),true);}eq(load(perm,permSave),1);eq(perm.up15_stat(0,58,6),180,'150 with two permanent10% effects');permSave=save(perm);pl=saveLayout(permSave);permSave[36+24]=9;permSave=refreshFixtureCityGround(permSave);eq(load(perm,permSave),1);eq(perm.up15_stat(0,58,6),204,'170 after epoch upgrade retains20%');const stablePerm=save(perm);eq(load(perm,stablePerm),1);eq(perm.up15_stat(0,58,6),204,'permanent factor survives save');
// A new unit is actually trained through the typed public action ABI.
const w=await make('classic');w.init(7,0,0,1,0);eq(w.skirmish_setup(500,2),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);
const before=save(w),bd=saveLayout(before).d;eq(w.cmd_queue_job(57),1);const paid=save(w);eq(new DataView(paid.buffer).getFloat32(36,true),bd.getFloat32(36,true)-30);for(let i=0;i<655;i++)w.tick();
let state=persistedState(save(w));good(state.entities.some(e=>e.alive&&e.team===0&&e.def===57),'Scout completed');
w.render(1,0,0,256,256,-1,0,0);const range=new Uint32Array(w.memory.buffer,w.ranges_ptr(),w.render_range_len());good(range[(256+57)*2+1]>0,'new Scout render range');
// Controlled real-schema entities exercise the other three new families in WASM.
function findEntity(b,team,def){const l=saveLayout(b),at=36+4*l.d.getUint32(8,true),stride=l.d.getUint32(4,true);return Array.from({length:l.d.getUint32(32,true)},(_,i)=>({slot:i,at:at+i*stride})).find(o=>b[o.at]&&b[o.at+4]===team&&l.d.getUint16(o.at+2,true)===def);}
function rewrite(b,rec,def,x,y,hp){const d=new DataView(b.buffer);d.setUint16(rec.at+2,def,true);for(const[o,v]of[[8,x],[12,y],[16,x],[20,y],[24,0],[28,0],[40,hp],[44,0],[80,1],[108,0],[132,0]])d.setFloat32(rec.at+o,v,true);b[rec.at+48]=0;b[rec.at+104]=0;b[rec.at+172]=0;for(const o of[60,64,68,72])d.setUint32(rec.at+o,0xffffffff,true);}
const ship=await make('classic');ship.init(17,0,0,1,0);eq(ship.skirmish_epoch(5),1);eq(ship.skirmish_setup(500,2),1);let fixture=save(ship),l=saveLayout(fixture),mapStart=36+4*l.d.getUint32(8,true)+l.d.getUint32(4,true)*l.d.getUint32(32,true),ter=fixture.subarray(mapStart,mapStart+65536);
const water=Array.from({length:65536},(_,i)=>i).find(i=>i>3*256&&i<253*256&&(i%256)>3&&(i%256)<253&&[i,i-1,i+1,i-256,i+256].every(t=>ter[t]===0));good(water!==undefined);const dock=findEntity(fixture,0,8);good(dock);const dx=water%256+0.5,dy=Math.floor(water/256)+0.5;rewrite(fixture,dock,5,dx,dy,ship.up15_stat(0,5,0));for(let t=0;t<2;t++)fixture[36+t*l.d.getUint32(8,true)+27]=0;eq(load(ship,fixture),1);ship.select_point(dx,dy,0,0,0,0,256,256);eq(ship.cmd_queue_job(58),1);for(let t=0;t<1305;t++)ship.tick();good(persistedState(save(ship)).entities.some(e=>e.alive&&e.team===0&&e.def===58),'real FightingSail queue completes');
for(const[id,targetDef,epoch]of[[59,37,14],[60,29,1]]){
 const a=await make('classic'),b=await make('classic');a.init(23,0,0,1,0);eq(a.skirmish_epoch(epoch),1);let data=save(a),layout=saveLayout(data),own=findEntity(data,0,8),target=findEntity(data,1,8);good(own);good(target);
 rewrite(data,own,id,100.5,100.5,a.up15_stat(0,id,0));rewrite(data,target,targetDef,105.5,100.5,30);for(let t=0;t<2;t++)data[36+t*layout.d.getUint32(8,true)+27]=0;
 eq(load(a,data),1);eq(load(b,data),1);const canonical=save(a);sameBytes(save(b),canonical);let seenDamage=false;
 for(let tick=0;tick<160;tick++){a.tick();b.tick();if(tick%7===0)a.render(0.4,0,0,256,256,-1,0,0);if(tick%11===0)b.render(0.8,0,0,256,256,-1,0,0);eq(a.state_hash(),b.state_hash(),'defense deterministic with different render cadence');const out=save(a),v=new DataView(out.buffer);if(out[target.at]===0||v.getFloat32(target.at+40,true)<30)seenDamage=true;}
 good(seenDamage,'actual new defense '+id+' damages its native target domain');const end=save(a),c=await make('classic');eq(load(c,end),1);sameBytes(save(c),end,'new defense/ATK roundtrip');
}

const current=save(w),hash=w.state_hash();const malformed=[];
for(const offset of[12,current.length-52+12]){const b=current.slice();b[offset]^=1;malformed.push(b);}
const cross=current.slice();new DataView(cross.buffer).setUint32(12,57,true);malformed.push(cross);
for(const source of['old5','old57']){
 const q=await make(source);q.init(7,0,0,2,0);const b=save(q),l=saveLayout(b),at=36+4*l.d.getUint32(8,true),stride=l.d.getUint32(4,true),wide=l.roster6;
 const bad=b.slice(),d=new DataView(bad.buffer);const live=Array.from({length:d.getUint32(32,true)},(_,i)=>at+i*stride).find(o=>bad[o]&&(wide?d.getUint16(o+2,true):bad[o+1])===8);good(live!==undefined);
 if(wide)d.setUint16(live+2,57,true);else bad[live+1]=57;malformed.push(bad);
}
for(const bad of malformed){eq(load(w,bad),0,'cross namespace rejects');eq(w.state_hash(),hash);sameBytes(save(w),current,'failed load is atomic');}
const old=await make('old57');old.init(7,0,0,2,0);const oldSave=save(old),oldHash=old.state_hash();eq(load(old,current),0,'57 cannot silently load61');eq(old.state_hash(),oldHash);sameBytes(save(old),oldSave,'foreign registry atomic');
report.status='PASS';report.checks=checks;report.malformedCases=malformed.length;report.limits=['Current61 contains four additional genuine source families, not the full132-family native roster.','Migration retains the existing omission of in-flight projectiles.','The permanent-percent state is source-backed; 180/204 test our arithmetic adapter. The original x87 rounding context at the actual setter remains unresolved; these are not unconditional native-bit or original-firetrace oracles.','Native terrain/collision and sail dynamics remain explicit graphical/geometry work.'];fs.writeFileSync(option('report',path.join(root,'../EE2-Research/analysis/native61-classic-wasm-tests.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:report.status,checks,migrations:report.migrations.length,binaries:report.binaries}));
