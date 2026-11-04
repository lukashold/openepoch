import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url)),candidate=path.resolve(root,'..');
const buildDir=process.argv[4]||path.join(candidate,'builds');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const files={up15:path.join(buildDir,'openepoch-up15.wasm'),base:path.join(buildDir,'openepoch-base.wasm'),reference:path.join(buildDir,'openepoch-ee2-reference.wasm'),beforeBase:process.argv[2],beforeReference:process.argv[3]};
const binaries=Object.fromEntries(Object.entries(files).map(([k,p])=>[k,fs.readFileSync(p)]));
const make=async key=>(await WebAssembly.instantiate(binaries[key],{})).instance.exports;
const save=w=>{let p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const canonical=b=>{const out=b.slice(),d=new DataView(out.buffer),stride=d.getUint32(4,true),players=d.getUint32(8,true),high=d.getUint32(32,true);for(let i=0;i<high;i++)d.setUint16(36+4*players+i*stride+76,65535,true);return out;};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
let checks=0;
const equal=(a,b,m)=>{assert.equal(a,b,m);checks++;},same=(a,b,m)=>{assert.deepEqual(a,b,m);checks++;};
const near=(a,b,m)=>{assert.ok(Math.abs(a-b)<Math.max(0.0001,Math.abs(b)*0.000001),`${m}: ${a} != ${b}`);checks++;};
const report={format:'up15-isolated-wasm-validation-v1',checks:0,binaries:Object.fromEntries(Object.entries(binaries).map(([k,b])=>[k,{bytes:b.length,sha256:sha(b)}])),equivalence:[]};
for(const [name,before]of [['base','beforeBase'],['reference','beforeReference']]){
 for(const seed of [7,41])for(const epoch of (name==='base'?[0,2,5,7]:[0,5,10,14])){
  const a=await make(before),b=await make(name);
  for(const w of[a,b]){w.init(seed,2,0,1,0);equal(w.skirmish_epoch(epoch),1);w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);equal(w.cmd_train(8),1);}
  for(let t=0;t<=1200;t++){
   if(t%200===0){equal(a.state_hash(),b.state_hash(),`${name} unchanged hash`);same(save(a),save(b),`${name} byte-identical full save`);}
   if(t<1200){a.tick();b.tick();}
  }
  report.equivalence.push({profile:name,seed,epoch,ticks:1200,stateHash:b.state_hash()>>>0,saveSha256:sha(save(b))});
 }
}
const units=JSON.parse(fs.readFileSync(path.join(root,'docs/ee2-up15-units.json'))),research=JSON.parse(fs.readFileSync(path.join(root,'docs/ee2-up15-research.json')));
const w=await make('up15');equal(w.ruleset(),2);equal(w.resource_count(),9);equal(w.epoch_count(),15);
for(let epoch=0;epoch<15;epoch++){
 w.init(71,1,0,1,0);equal(w.skirmish_epoch(epoch),1);
 for(const row of units.class_rows.filter(r=>r.epoch<=epoch&&epoch<r.until)){
  const f=row.fields;for(const[key,field]of [['hp',0],['buildtime',1],['sight',2],['raw_speed',3],['range',4],['reload',5],['damage',6],['pop',7],['pop_capacity',8],['generation',10],['farm_rate',11],['carry',12],['farm_carry',13],['dropoff_bonus',14]]){
   if(f[key]?.value!=null)near(w.up15_stat(0,row.id,field),f[key].value,`live E${epoch+1} id${row.id} ${key}`);
  }
  for(let k=0;k<9;k++){
   if(f.cost9.value)equal(w.unit_cost_stat(0,row.id,k),f.cost9.value[k],'live nine-slot cost');
   if(f.gather.value)near(w.up15_stat(0,row.id,20+k),f.gather.value[k],'live nine-slot gather');
  }
 }
 equal(w.unit_epoch(28),14,'Exact UP Paladin available E15');equal(w.unit_epoch(43),13,'Exact UP LightHerc available E14');
 const p=w.research_tree(),tree=JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));
 equal(tree.length,180);for(let i=0;i<180;i++){equal(tree[i].id,i);equal(tree[i].cost,research.nodes[i].raw_node.TECHPTS);}
}
// Every compiled profile pair independently queried through the public WASM ABI.
const combat=JSON.parse(fs.readFileSync(path.join(root,'docs/ee2-up15-combat.json'))),oracle=fs.readFileSync(path.join(root,'tools/up15-combat-oracle.bin'));
for(const a of combat.profiles)for(const d of combat.profiles){
 const o=4+(a.index*combat.profiles.length+d.index)*4,b=oracle.readUInt16LE(o),adj=oracle.readUInt16LE(o+2);
 near(w.combat_stat(a.id,a.epoch,d.id,d.epoch,0),Math.fround(b*0.01),'WASM raw RPS');
 near(w.combat_stat(a.id,a.epoch,d.id,d.epoch,1),Math.fround(adj*0.01),'WASM raw upgrade adjust');
 near(w.combat_stat(a.id,a.epoch,d.id,d.epoch,2),Math.fround(Math.fround(b*0.01)*Math.fround(adj*0.01)),'WASM source multiplier');
 equal(w.combat_stat(a.id,a.epoch,d.id,d.epoch,3),b*adj>0?1:0,'WASM native target gate');
}
// The three rulesets reject one another atomically. Own saves roundtrip exactly.
const instances={base:await make('base'),reference:await make('reference'),up15:await make('up15')};
for(const q of Object.values(instances))q.init(29,1,0,1,0);
const saved=Object.fromEntries(Object.entries(instances).map(([k,q])=>[k,save(q)]));
equal(new DataView(saved.up15.buffer).getUint32(0,true),0x3555454f,'distinct UP save magic');
for(const[a,q]of Object.entries(instances))for(const[b,s]of Object.entries(saved)){
 const before=save(q),hash=q.state_hash();equal(load(q,s),a===b?1:0,'profile-isolated load');
 equal(q.state_hash(),hash,'atomic load hash');same(save(q),before,'atomic byte preservation');
}
for(const seed of[7,41]){
 const a=await make('up15'),b=await make('up15'),resumed=await make('up15');for(const q of[a,b,resumed])q.init(seed,3,0,1,0);
 for(let t=0;t<=3600;t++){
  if(t%600===0){equal(a.state_hash(),b.state_hash(),'UP deterministic independent instances');same(save(a),save(b),'UP deterministic save');const s=save(b);equal(load(resumed,s),1);equal(resumed.state_hash(),b.state_hash(),'UP save reload immediate state');same(canonical(save(resumed)),canonical(s),'UP save reload preserves all persisted bytes except reset flow-cache slot');}
  if(t<3600){a.tick();b.tick();}
 }
}
report.saveSemantics='Current UP save preserves persisted state; load resets Ent.fslot flow-cache slots as all existing profiles do. Active projectiles remain transient and are not replay-equivalent across save/load.';report.checks=checks;report.status='PASS';fs.writeFileSync(path.join(buildDir,'../wasm-validation-report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:report.status,checks,equivalenceCases:report.equivalence.length,binaries:report.binaries}));
