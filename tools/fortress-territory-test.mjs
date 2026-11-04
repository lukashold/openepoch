// Real WASM checks with source-data oracles and explicit saved-map fixtures.
// The historic56 save is genuine output from the previous binary, never a
// current save with a relabelled version. Own tie-break/dispute/crew rules are
// tested as reconstruction policy, not asserted to be complete original rules.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {assertCurrentSave,revealFixture,saveLayout,refreshFixtureCityGround} from './save-fixture.mjs';

const docs=new URL('../docs/',import.meta.url),fixtures=new URL('fixtures/',import.meta.url);
const reports=new URL('../../EE2-Research/analysis/',import.meta.url);
mkdirSync(reports,{recursive:true});
const blob=readFileSync(new URL('openepoch-ee2.wasm',docs)),baseBlob=readFileSync(new URL('openepoch.wasm',docs));
const sha=b=>createHash('sha256').update(b).digest('hex');
const units=JSON.parse(readFileSync(new URL('ee2-units.json',docs))),audit=JSON.parse(readFileSync(new URL('research-audit.json',docs)));
const catalog=JSON.parse(readFileSync(new URL('ee2-catalog.json',docs)));
const nativeNodes=new Map(catalog.nodes.map(n=>[n.NAME,n]));
const N=256,NN=N*N,NONE=0xffffffff,FORT=56;
let checks=0;const completed=[];
const check=Object.fromEntries(['ok','equal','notEqual','deepEqual'].map(k=>[k,(...a)=>{checks++;assert[k](...a);} ]));
const report={format:'fortress-territory-tests-v1',status:'running',wasmSha256:sha(blob),baseWasmSha256:sha(baseBlob),unitAuditSha256:sha(readFileSync(new URL('ee2-units.json',docs))),researchAuditSha256:sha(readFileSync(new URL('research-audit.json',docs))),scope:'Real WASM57/Base53, original requirements/epochs/costs/HP/research masks, authenticated rotation commands, completed-building claims, retained rights/gifts, and genuine prior56 natural-save migration. Controlled fixture objects are cloned complete save records, not engine edits. Full native disputed state/combat matrix/AI and browser graphics are not certified.',checks:0,completed};
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
function layout(b){const l=saveLayout(b),d=l.d,psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),ents=36+4*psz,terrain=ents+high*esz;return {...l,d,psz,esz,high,ents,terrain,res:terrain+NN,amt:terrain+2*NN,eco:l.end+132};}
function entities(b){const l=layout(b),out=[];for(let i=0;i<l.high;i++){const at=l.ents+i*l.esz;if(!b[at])continue;const gen=l.d.getUint16(at+4,true);out.push({i,at,handle:((gen<<16)|i)>>>0,id:b[at+1],team:b[at+2],x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),rot:l.d.getFloat32(at+32,true),hp:l.d.getFloat32(at+40,true),built:l.d.getFloat32(at+80,true),cargo:Array.from(b.subarray(at+125,at+125+b[at+145])),cargoHp:Array.from(b.subarray(at+135,at+135+b[at+145]))});}return out;}
const own=(w,id,team=0)=>entities(save(w)).filter(e=>e.id===id&&e.team===team);
const find=(w,h)=>entities(save(w)).find(e=>e.handle===h);
const select=(w,e)=>{w.select_none();w.select_point(e.x,e.y,0,0,0,0,N,N);};
const stock=(w,t=0)=>Array.from({length:w.resource_count()},(_,s)=>w.resource_stat(t,s));
const close=(a,b,label,eps=.005)=>check.ok(Math.abs(a-b)<=eps,`${label}: ${a}, expected ${b}`);
function source(id,epoch,field){const row=units.class_rows.find(r=>r.id===id&&epoch>=r.epoch&&epoch<r.until);check.ok(row?.fields[field]?.verified,`Verified original ${id}/E${epoch+1}/${field}`);return row.fields[field].value;}
function setStock(b,t,value){const l=layout(b);for(let s=0;s<(l.reference?9:3);s++)l.d.setFloat32(s<3?36+t*l.psz+s*4:l.eco+8+(t*6+s-3)*4,value,true);}
async function make(epoch=0,{base=false}={}){
 const w=(await WebAssembly.instantiate(base?baseBlob:blob,{})).instance.exports;
 w.init(7,1,0,1,0);check.equal(w.skirmish_visibility(2),1);if(!base)check.equal(w.skirmish_epoch(epoch),1);
 const b=save(w),l=layout(b);check.equal(l.d.getUint32(12,true),base?53:57,'Current profile definition count');assertCurrentSave(b);
 for(let t=0;t<4;t++){b[36+t*l.psz+27]=0;setStock(b,t,20000);if(!base)l.d.setFloat32(l.end+20+t*4,20000,true);}
 b.fill(3,l.terrain,l.terrain+NN);b.fill(0,l.res,l.res+NN);b.fill(0,l.amt,l.amt+2*NN);revealFixture(b);
 check.equal(load(w,b),1);return w;
}
function wire(kind,a,b,c,handles=[]){const out=new Uint8Array(14+4*handles.length),d=new DataView(out.buffer);out[0]=kind;out[1]=handles.length;d.setFloat32(2,a,true);d.setFloat32(6,b,true);d.setUint32(10,c>>>0,true);handles.forEach((h,i)=>d.setUint32(14+i*4,h,true));return out;}
function deliver(w,t,b){const p=w.inbox_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);w.net_apply(t);}
function move(b,e,x,y){const l=layout(b);for(const [o,v]of [[8,x],[12,y],[16,x],[20,y],[24,0],[28,0]])l.d.setFloat32(e.at+o,v,true);b[e.at+48]=0;l.d.setUint32(e.at+60,NONE,true);l.d.setUint32(e.at+68,NONE,true);}
// Add an otherwise complete, valid saved entity with clean queue/cargo fields.
function add(w,id,region,{team=0,built=1,x,y,rot=0}={}){
 const b=save(w),l=layout(b),prototype=entities(b).find(e=>e.id===0),extra=new Uint8Array(b.length+l.esz);
 extra.set(b.subarray(0,l.terrain));extra.set(b.subarray(l.terrain),l.terrain+l.esz);
 const at=l.terrain,d=new DataView(extra.buffer);extra.set(b.subarray(prototype.at,prototype.at+l.esz),at);
 d.setUint32(32,l.high+1,true);extra[at+1]=id;extra[at+2]=team;extra[at+3]=0;d.setUint16(at+4,1,true);
 const center=[w.territory_stat(region,1),w.territory_stat(region,2)];
 for(const [off,v]of [[8,x??center[0]],[12,y??center[1]],[16,x??center[0]],[20,y??center[1]],[24,0],[28,0],[32,rot],[36,rot],[40,w.research_stat(team,3,id)],[80,built],[92,0],[116,0]])d.setFloat32(at+off,v,true);
 extra[at+48]=0;d.setUint32(at+60,NONE,true);d.setUint32(at+68,NONE,true);extra.fill(0,at+84,at+92);extra[at+145]=0;
 check.equal(w.territory_at(d.getFloat32(at+8,true),d.getFloat32(at+12,true)),region,'Fixture remains in intended real territory');
 check.equal(load(w,refreshFixtureCityGround(extra)),1,'Complete cloned fixture record is valid');return find(w,((1<<16)|l.high)>>>0);
}
function edit(w,e,fn){const b=save(w),l=layout(b),current=entities(b).find(x=>x.handle===e.handle);check.ok(current);fn(b,l,current);check.equal(load(w,refreshFixtureCityGround(b)),1);}
const region=(w,adj)=>Array.from({length:w.territory_count()},(_,id)=>id).find(id=>w.territory_owner(id)===255&&!!w.territory_claim_stat(0,id,1)===adj);
function instant(w){check.equal(w.cheat(0),1);if(!w.cheat_status(1))check.equal(w.cheat(3),1);}
function site(w,id,turn=0){select(w,own(w,8)[0]);const half=w.footprint_size(id,turn)/2;for(let y=3;y<245;y++)for(let x=3;x<245;x++)if(w.territory_owner(w.territory_at(x+half,y+half))===0&&w.place_error_rotated(id,x,y,turn)===0)return [x,y];assert.fail('No owned building site');}
function build(w,id,turn=0){instant(w);const at=site(w,id,turn),before=new Set(entities(save(w)).map(e=>e.handle));check.equal(w.cmd_place_rotated(id,...at,turn),1);run(w,8);const e=own(w,id).find(e=>!before.has(e.handle));check.ok(e,'Actual command creates building');check.equal(e.built,1,'Actual construction completes');return e;}

async function statsAndRotation(){
 for(let epoch=0;epoch<15;epoch++){
  const w=await make(epoch);close(w.research_stat(0,3,FORT),source(FORT,epoch,'hp'),'Native current-epoch Fortress HP');
  check.deepEqual(Array.from({length:9},(_,s)=>w.unit_cost_stat(0,FORT,s)),source(FORT,epoch,'cost9'),'Native current-epoch nine-column cost');
 }
 for(let turn=0;turn<8;turn++){
  const w=await make(),bank=stock(w),f=build(w,FORT,turn);close(f.rot,turn*Math.PI/4,'Saved actual Fortress orientation');
  check.deepEqual(stock(w),bank.map((v,s)=>v-source(FORT,0,'cost9')[s]),'Owned Fortress actual payment');
  select(w,f);check.equal(ui(w).sel.cargomax,source(FORT,0,'garrison_slots'),'Source10-slot building capacity');
  const b=save(w),peer=await make();check.equal(load(peer,b),1);check.deepEqual(save(peer),b,'57-definition Fortress+rotation roundtrip');
 }
 const w=await make();build(w,FORT);build(w,FORT);select(w,own(w,8)[0]);
 const home=w.territory_at(own(w,0)[0].x,own(w,0)[0].y);let blocked=false;
 for(let y=3;y<245&&!blocked;y++)for(let x=3;x<245;x++)if(w.territory_at(x+2.5,y+2.5)===home&&w.place_error(FORT,x,y)===10){blocked=true;break;}
 check.ok(blocked,'Two own Fortress buildings block a third with explicit maximum error');
 for(const foreignBuilt of [0,1]){
  const mixed=await make(),probe=site(mixed,FORT),home=own(mixed,0)[0],r=mixed.territory_at(home.x,home.y);
  add(mixed,FORT,r,{team:1,built:foreignBuilt,x:home.x+10,y:home.y+10});
  add(mixed,FORT,r,{built:0,x:home.x-10,y:home.y+10});select(mixed,own(mixed,8)[0]);
  check.equal(mixed.place_error(FORT,...probe)===10,!!foreignBuilt,'Native maximum includes foreign completed, excludes foreign foundation');
 }
 const guard=await make(),at=site(guard,FORT),worker=own(guard,8)[0],other=own(guard,8,1)[0];
 for(const [team,packet,label]of [
  [0,wire(16,...at,FORT|(8<<8),[worker.handle]),'rotation8'],
  [0,wire(16,...at,FORT|(1<<8)|(1<<16),[worker.handle]),'extra upperbits'],
  [0,wire(16,NaN,at[1],FORT|(1<<8),[worker.handle]),'nonfinite position'],
  [0,wire(16,...at,FORT|(1<<8),[(worker.handle+0x10000)>>>0]),'stale generation'],
  [0,wire(16,...at,FORT|(1<<8),[other.handle]),'foreign worker'],
 ]){const hash=guard.state_hash(),bank=stock(guard),actors=entities(save(guard));deliver(guard,team,packet);check.equal(guard.state_hash(),hash,'Fortress wire rejects '+label);check.deepEqual(stock(guard),bank);check.deepEqual(entities(save(guard)),actors);}
 const base=await make(0,{base:true});check.equal(base.unit_stat(FORT,0),-1);select(base,own(base,8)[0]);
 const old=base.state_hash();check.equal(base.cmd_place_rotated(FORT,60,60,0),0);check.equal(base.state_hash(),old,'Base53 rejects Fortress without state mutation');
 completed.push('All15 native Fortress HP/cost epochs; actual eight-angle building/payment/save;10-slot capacity; maximum2; malformed/stale/foreign C16 rejection; Base53 excludes ID56');
}
async function claimChecks(){
 let w=await make();const adjacent=region(w,true);check.notEqual(adjacent,undefined);const c=add(w,0,adjacent,{built:.99999});
 check.equal(w.territory_owner(adjacent),255);check.equal(w.territory_claim_stat(0,adjacent,0),2);check.equal(w.territory_claim_stat(0,adjacent,2),1);check.equal(w.territory_claim_stat(0,adjacent,4),0);
 edit(w,c,(b,l,e)=>l.d.setFloat32(e.at+80,1,true));check.equal(w.territory_owner(adjacent),0,'Completed adjacent TC acquires');
 w=await make();const isolated=region(w,false);check.notEqual(isolated,undefined);const city=add(w,0,isolated);
 check.equal(w.territory_owner(isolated),255);check.equal(w.territory_claim_stat(0,isolated,0),3,'Isolated city requires a fortress');
 const center=[city.x,city.y],foreign=add(w,FORT,isolated,{team:1,x:center[0]+8,y:center[1]});
 check.equal(w.territory_owner(isolated),255,'Other-player fortress does not satisfy own requirement');
 const fort=add(w,FORT,isolated,{built:.99999,x:center[0]-8,y:center[1]});
 check.equal(w.territory_claim_stat(0,isolated,0),4);check.equal(w.territory_claim_stat(0,isolated,3),1);check.equal(w.territory_claim_stat(0,isolated,5),0);
 const pending=save(w),peer=await make();check.equal(load(peer,pending),1);check.equal(peer.territory_owner(isolated),255,'Modern save preserves pending isolated claim');
 edit(w,fort,(b,l,e)=>l.d.setFloat32(e.at+80,1,true));check.equal(w.territory_owner(isolated),0,'Completed own TC+Fortress acquires isolated land');
 edit(w,fort,(b,l,e)=>b[e.at]=0);check.equal(w.territory_owner(isolated),0,'Existing city claim survives fortress destruction');
 const balances=[stock(w),stock(w,1)],gift=wire(15,isolated,-1,1);deliver(w,0,gift);
 check.equal(w.territory_owner(isolated),1,'Gift bypasses new acquisition adjacency requirement');check.equal(find(w,city.handle).team,1);check.equal(find(w,foreign.handle).team,1);check.deepEqual([stock(w),stock(w,1)],balances);
 const gifted=save(w);check.equal(load(peer,gifted),1);run(peer,80);check.equal(peer.territory_owner(isolated),1,'Gift persists after load and recount without fortress');
 w=await make();const oldHome=own(w,0)[0];edit(w,oldHome,(b,l,e)=>{b[e.at]=0;const id=w.territory_at(e.x,e.y);l.d.setUint32(l.ter+8+id*4,NONE,true);});check.equal(w.stat(0,7),0);
 const first=region(w,false);add(w,0,first);check.equal(w.territory_owner(first),0,'Landless bootstrap only requires completed TC');
 check.equal(w.territory_claim_stat(99,0,0),-1);check.equal(w.territory_claim_stat(0,999,0),-1);check.equal(w.territory_claim_stat(0,0,6),-1);
 completed.push('Real-map adjacent/isolated requirements, separate foundation counts, wrong owner, pending57 save, retained city rights, isolated gift/load and landless bootstrap');
}
async function migrationChecks(){
 const meta=JSON.parse(readFileSync(new URL('fortress-legacy56.json',fixtures))),oldBlob=readFileSync(new URL('reference-ee2-pre-fortress-56.wasm',fixtures)),oldSave=new Uint8Array(gunzipSync(readFileSync(new URL('fortress-legacy56.sav.gz',fixtures))));
 check.equal(sha(oldBlob),meta.artifacts[0].sourceSha256);check.equal(sha(oldSave),meta.artifacts[1].sourceSha256);
 const l=layout(oldSave);check.equal(l.d.getUint32(12,true),56);check.equal(l.d.getUint32(16,true),17);check.equal(l.d.getUint32(20,true),2);check.equal(l.d.getUint32(24,true),2000);
 const old=(await WebAssembly.instantiate(oldBlob,{})).instance.exports;old.init(17,2,0,1,0);check.equal(load(old,oldSave),1,'Real previous binary loads its own natural save');
 const w=await make(),peer=await make();check.equal(load(w,oldSave),1,'Current57 loads genuine prior56 save');
 const snapshot=engine=>({entities:entities(save(engine)).map(({at,...e})=>e),banks:[0,1,2].map(t=>stock(engine,t)),stats:[0,1,2].map(t=>Array.from({length:9},(_,k)=>engine.stat(t,k))),research:[0,1,2].map(t=>Array.from({length:180},(_,k)=>engine.research_stat(t,2,k))),territories:Array.from({length:engine.territory_count()},(_,id)=>engine.territory_owner(id)),relations:[0,1,2].map(t=>[0,1,2].map(o=>Array.from({length:7},(_,f)=>engine.diplomacy_stat(t,o,f))))});
 check.deepEqual(snapshot(w),snapshot(old),'Migration preserves all actors/cargo/health/rotation, banks/research/population, actual claims and diplomacy');
 const b=save(w),modern=layout(b);check.equal(modern.d.getUint32(12,true),57);check.equal(modern.d.getUint32(24,true),2000);assertCurrentSave(b);
 check.deepEqual(b.subarray(modern.terrain,modern.end),oldSave.subarray(l.terrain,l.end),'Harvested map and displayed exploration survive real56 migration');
 check.deepEqual(b.subarray(modern.realFog,modern.ter),oldSave.subarray(l.realFog,l.ter),'All four actual exploration maps survive');
 check.equal(load(peer,b),1);check.deepEqual(save(peer),b,'Migrated57 save roundtrip');run(w,240);run(peer,240);check.equal(w.state_hash(),peer.state_hash());check.deepEqual(save(w),save(peer),'Two migrated peers remain deterministic');
 report.genuine56={wasmSha256:sha(oldBlob),saveSha256:sha(oldSave),seed:17,ai:2,tick:2000,continuedTicks:240};
 completed.push('Genuine unmodified old56 binary+2000tick natural save provenance, complete state migration57/roundtrip and240 deterministic ticks');
}
const hosts={CityCenter:0,University:47,Barracks:3,Temple:51,Market:50,Warehouse:52};
function research(w,key){const index=audit.nodes.findIndex(n=>n.id===key),n=audit.nodes[index],original=nativeNodes.get(key);check.ok(n&&original);const host=hosts[n.host];check.notEqual(host,undefined);if(!own(w,host).length)add(w,host,w.territory_at(own(w,0)[0].x,own(w,0)[0].y),{x:own(w,0)[0].x+10,y:own(w,0)[0].y+8});w.select_none();check.equal(w.cmd_research(index),1,'Actual paid source research '+key);run(w,Math.ceil(original.TIME/.05)+3);check.equal(w.research_stat(0,2,index),1);return n;}
async function researchChecks(){
 for(const key of ['Imperial3_4','Imperial9_2','Imperial13_4']){
  const original=nativeNodes.get(key),w=await make(original.EPOCH-1),before=Array.from({length:57},(_,id)=>w.research_stat(0,3,id));
  const n=research(w,key),effects=n.effects.filter(e=>e.field===0);check.ok(effects.some(e=>e.targets.includes(FORT)));
  for(let id=0;id<57;id++){const effect=effects.find(e=>e.targets.includes(id));close(w.research_stat(0,3,id),before[id]*(effect?1+effect.value/100:1),key+' exact HP mask ID'+id);check.equal(w.research_stat(1,3,id),before[id],'Other owner unaffected');}
 }
 for(const key of ['Economic2_2','Economic7_2','Economic12_2']){
  const original=nativeNodes.get(key),w=await make(original.EPOCH-1),before=Array.from({length:57},(_,id)=>Array.from({length:9},(_,s)=>w.unit_cost_stat(0,id,s)));
  const n=research(w,key),effect=n.effects.find(e=>e.field===4&&e.targets.includes(FORT));check.ok(effect);check.deepEqual(effect.targets,[7,FORT]);
  for(let id=0;id<57;id++)check.deepEqual(Array.from({length:9},(_,s)=>w.unit_cost_stat(0,id,s)),before[id].map(v=>Math.round(v*(effect.targets.includes(id)?1+effect.value/100:1))),key+' exact cost mask ID'+id);
 }
 const w=await make(1),home=own(w,0)[0],regionId=w.territory_at(home.x,home.y),fort=add(w,FORT,regionId,{x:home.x+13,y:home.y+13});
 const range=source(FORT,1,'range'),radius=w.footprint_size(1,0)/2;
 // Integer-aligned2x2 house center preserves its normal1-tile target radius;
 // fractional centers would expand the declared conservative tile-BBox adapter.
 const targetX=fort.x+range+radius+1;
 // Supply independent genuine LOS: attack range includes target radius while
 // normal sight checks the target center. A noncombat barracks observes it;
 // Full-map display alone deliberately does not grant simulation sight.
 const observerX=fort.x+7,observerY=fort.y+4;
 add(w,3,w.territory_at(observerX,observerY),{x:observerX,y:observerY});
 const target=add(w,1,w.territory_at(targetX,fort.y),{team:1,x:targetX,y:fort.y});
 const hp=target.hp;run(w,100);close(find(w,target.handle).hp,hp,'Integer-aligned target one tile beyond native range is safe');
 const n=research(w,'Military2_3');check.deepEqual(n.effects.find(e=>e.field===6).targets,[7,FORT]);run(w,100);check.ok(find(w,target.handle).hp<hp,'Source +1 range makes actual Fortress projectiles hit stationary target');
 report.researchAttackScope='Source has LOS/Range modifiers forFortress, no direct Damage effect; actual beyond-baseline-range hit tested. Full native RPS coefficients are not asserted.';
 completed.push('All3 source Fortress HP and3 cost tiers via actual paid research with exact57-class/other-player masks; source+1 range enables real projectile damage');
}
async function garrisonChecks(){
 const w=await make(),home=own(w,0)[0],r=w.territory_at(home.x,home.y),fort=add(w,FORT,r,{x:home.x+13,y:home.y+13});
 const row=(id)=>units.class_rows.find(x=>x.id===id&&x.epoch===0);
 check.equal(row(22).fields.fortress_garrison.value,true);check.equal(row(8).fields.fortress_garrison.value,true);
 check.equal(source(22,0,'garrison_size'),3);check.equal(source(8,0,'garrison_size'),1);
 const mobile=[];
 for(let k=0;k<4;k++)mobile.push(add(w,22,r,{x:fort.x+3.3,y:fort.y+(k-1.5)*.25}));
 const citizen=add(w,8,r,{x:fort.x-3.3,y:fort.y});
 edit(w,citizen,(b,l,e)=>l.d.setFloat32(e.at+40,w.research_stat(0,3,8)*.5,true));
 deliver(w,0,wire(6,fort.x,fort.y,fort.handle,[...mobile.slice(0,3),citizen].map(e=>e.handle)));run(w,30);
 select(w,find(w,fort.handle));check.equal(ui(w).sel.cargo,10,'Three native3-slot units plus one1-slot citizen occupy10');check.equal(ui(w).sel.cargoUnits,4,'Unit count differs from occupied slot count');
 close(ui(w).sel.garrisonAttackBonus,source(FORT,0,'garrison_attack_bonus')*10*100,'Native bonus is per occupied slot');
 const before=find(w,fort.handle).cargo.length;deliver(w,0,wire(6,fort.x,fort.y,fort.handle,[mobile[3].handle]));run(w,20);check.equal(find(w,fort.handle).cargo.length,before,'Overflow unit cannot enter a full10-slot garrison');check.ok(find(w,mobile[3].handle),'Rejected passenger remains alive outside');
 let saved=save(w),l=layout(saved),current=entities(saved).find(e=>e.handle===fort.handle),index=current.cargo.indexOf(8);check.ok(index>=0);
 saved[current.at+135+index]=50;l.d.setFloat32(current.at+116,0,true);check.equal(load(w,saved),1);
 run(w,100);const after=find(w,fort.handle).cargoHp[index],rate=source(FORT,0,'garrison_heal'),hp=w.research_stat(0,3,8);
 check.equal(after,50+Math.floor(100*.05*rate*100/hp),'Native per-second heal rate, with documented stored-percent precision');
 // Isolate the Fortress shot from the deliberately rejected armed passenger.
 edit(w,mobile[3],(b,l,e)=>b[e.at]=0);
 const targetX=fort.x+6,target=add(w,1,w.territory_at(targetX,fort.y),{team:1,x:targetX,y:fort.y});
 const firing=save(w),fl=layout(firing),fe=entities(firing).find(e=>e.handle===fort.handle);fl.d.setFloat32(fe.at+44,0,true);check.equal(load(w,firing),1);
 const uncrewed=await make(),control=firing.slice();control[fe.at+145]=0;check.equal(load(uncrewed,control),1);
 const targetHp=find(w,target.handle).hp;run(w,30);run(uncrewed,30);
 const plain=targetHp-find(uncrewed,target.handle).hp,crewed=targetHp-find(w,target.handle).hp;
 check.ok(plain>0&&crewed>plain,'Both real Fortress projectiles hit, occupied one deals more');
 close(crewed/plain,1+source(FORT,0,'garrison_attack_bonus')*10,'Actual projectile damage uses native bonus per10 occupied slots',.0001);
 const b=save(w),peer=await make();check.equal(load(peer,b),1);select(peer,find(peer,fort.handle));check.equal(ui(peer).sel.cargo,10);check.equal(ui(peer).sel.cargoUnits,4);
 const existing=new Set(entities(b).map(e=>e.handle));deliver(peer,0,wire(7,fort.x,fort.y,0,[fort.handle]));run(peer,2);
 check.equal(find(peer,fort.handle).cargo.length,0,'Unload empties all passenger types');check.equal(entities(save(peer)).filter(e=>!existing.has(e.handle)).length,4,'Every real passenger returns exactly once');
 report.garrisonSlotScope='Native7E63B0 and independent53A0F0/53A8D0 property binding establish Leaders1 otherwise popCount; no inferred RPS/pop adapter. Health storage remains existing integer percent representation.';
 completed.push('Native eligibility and mixed3/1-slot boarding,10-slot overflow safety, actual projectile damage +3% per occupied slot, source per-second healing, cargo57 roundtrip and exact unload');
}
try{
 await statsAndRotation();await claimChecks();await migrationChecks();await researchChecks();await garrisonChecks();
 report.status='passed';report.checks=checks;writeFileSync(new URL('fortress-territory-tests.json',reports),JSON.stringify(report,null,2)+'\n');
 console.log(`PASS Fortress/territory: ${checks} checks; ${completed.length} groups; native acquisition/completion/bootstrap, Fortress57 rotation/research and genuine56 migration, Base53.`);
}catch(e){report.status='failed';report.checks=checks;report.failure=e.stack;writeFileSync(new URL('fortress-territory-tests.json',reports),JSON.stringify(report,null,2)+'\n');throw e;}
