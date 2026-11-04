// Independent behavioural tests of real WASM province ownership and command
// handling. Scenario fixtures use saved maps, never engine edits or mocks.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {Lockstep} from '../docs/net.js';
import {DIP_BYTES,TER_BYTES,assertCurrentSave,revealFixture,resetFixtureClaims,saveLayout,refreshFixtureCityGround} from './save-fixture.mjs';

const root=new URL('../',import.meta.url),docs=new URL('docs/',root),reports=new URL('../../EE2-Research/analysis/',import.meta.url);
const bytes=readFileSync(new URL('openepoch-ee2.wasm',docs)),baseBytes=readFileSync(new URL('openepoch.wasm',docs));
const source=JSON.parse(readFileSync(new URL('ee2-units.json',docs))),audit=JSON.parse(readFileSync(new URL('research-audit.json',docs)));
const hash=b=>createHash('sha256').update(b).digest('hex');
const N=256,NN=N*N,NONE=0xffffffff,GIFT=0,SWAP=1,ACCEPT=2,REJECT=3,WITHDRAW=4;
let checkCount=0;const completed=[];
const check=Object.fromEntries(['ok','equal','notEqual','deepEqual'].map(key=>[key,(...args)=>{checkCount++;assert[key](...args);} ]));
const report={format:'territory-diplomacy-tests-v1',status:'running',scope:'Persistent territory gifts and atomic swaps, actual map/building/unit ownership, declared garrison/queue adapters, recipient native stats, conservation, saves and strict authenticated lockstep. Crew and queue handling are explicit reconstruction policy, not a claim of complete original-game equivalence.',wasmSha256:hash(bytes),baseWasmSha256:hash(baseBytes),sourceUnitAuditSha256:hash(readFileSync(new URL('ee2-units.json',docs))),sourceResearchAuditSha256:hash(readFileSync(new URL('research-audit.json',docs))),checkCount:0,completed};
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
function layout(b){const l=saveLayout(b),d=l.d,psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),ents=36+4*psz,terrain=ents+high*esz;return {...l,d,psz,esz,high,ents,terrain,res:terrain+NN,amt:terrain+2*NN,eco:l.end+132};}
function entities(b){
 const l=layout(b),out=[];check.ok(l.esz>=146,'known complete Ent prefix including ten cargo slots');
 for(let i=0;i<l.high;i++){const at=l.ents+i*l.esz;if(!b[at])continue;const gen=l.d.getUint16(at+4,true),qn=b[at+89],cargo=b[at+145];out.push({i,at,gen,handle:((gen<<16)|i)>>>0,def:b[at+1],team:b[at+2],x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),hp:l.d.getFloat32(at+40,true),order:b[at+48],target:l.d.getUint32(at+60,true),built:l.d.getFloat32(at+80,true),queue:Array.from(b.subarray(at+84,at+84+qn)),returning:b[at+85],cargoResource:b[at+86],queueTime:l.d.getFloat32(at+92,true),carry:l.d.getFloat32(at+116,true),cargo:Array.from(b.subarray(at+125,at+125+cargo)),cargoHealth:Array.from(b.subarray(at+135,at+135+cargo))});}
 return out;
}
const own=(w,t,id)=>entities(save(w)).filter(e=>e.team===t&&(id===undefined||e.def===id));
const find=(w,e)=>entities(save(w)).find(current=>current.handle===e.handle);
const stock=(w,t)=>Array.from({length:w.resource_count()},(_,s)=>w.resource_stat(t,s));
const banks=w=>[0,1,2].map(t=>stock(w,t));
const player=(w,t)=>ui(w).diplomacy.players.find(p=>p.team===t);
const province=(w,id)=>ui(w).diplomacy.territories.find(p=>p.id===id);
const owner=(w,id)=>w.territory_owner(id)>>>0;
const terr=w=>{const p=w.terr_ptr();return new Uint8Array(w.memory.buffer,p,NN*4).slice();};
const select=(w,e)=>{w.select_none();w.select_point(e.x,e.y,0,0,0,0,N,N);};
function move(b,e,x,y){const l=layout(b);for(const [o,v]of [[8,x],[12,y],[16,x],[20,y],[24,0],[28,0]])l.d.setFloat32(e.at+o,v,true);}
function wire(kind,a,b,c,handles=[]){const result=new Uint8Array(14+handles.length*4),d=new DataView(result.buffer);result[0]=kind;result[1]=handles.length;d.setFloat32(2,a,true);d.setFloat32(6,b,true);d.setUint32(10,c>>>0,true);handles.forEach((h,i)=>d.setUint32(14+i*4,h,true));return result;}
function deliver(w,team,b){const p=w.inbox_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);w.net_apply(team);}
function packet(target,action,give=NONE,want=NONE){return wire(15,give===NONE?-1:give,want===NONE?-1:want,target|(action<<8));}
const remote=(w,team,target,action,give=NONE,want=NONE)=>deliver(w,team,packet(target,action,give,want));
function dip(w,from,to,action,resource=0,amount=0){deliver(w,from,wire(14,amount,0,to|(action<<8)|(resource<<16)));}
function peace(w,a,b){dip(w,a,b,0);check.equal(w.diplomacy_stat(a,b,1),1);dip(w,b,a,3);check.equal(w.diplomacy_stat(a,b,0),1);check.equal(w.diplomacy_stat(b,a,0),1);}
function local(w,target,action,give=NONE,want=NONE){const result=w.cmd_territory(target,action,give,want),n=w.outbox_len();if(n){const b=new Uint8Array(w.memory.buffer,w.outbox_ptr(),n).slice();w.outbox_clear();deliver(w,0,b);}return result;}
function invalid(w,fn,label){const h=w.state_hash(),b=banks(w);check.equal(fn(),0,label);check.equal(w.state_hash(),h,label+' is atomic');check.deepEqual(banks(w),b);check.equal(w.outbox_len(),0);}
function close(a,b,label,epsilon=.005){check.ok(Math.abs(a-b)<epsilon,`${label}: ${a}, expected ${b}`);}
function native(id,epoch,key){const r=source.class_rows.filter(r=>r.id===id&&r.epoch<=epoch).sort((a,b)=>b.epoch-a.epoch)[0];check.ok(r&&r.fields[key]?.verified,`source-backed ${id}/${key}`);return r.fields[key].value;}
function instant(w){if(!w.cheat_status(0))check.equal(w.cheat(0),1);if(!w.cheat_status(1))check.equal(w.cheat(3),1);}
function size(w,id){const worker=own(w,0,8)[0];check.ok(worker);w.select_none();w.select_point(worker.x,worker.y,0,1,0,0,N,N);const a=ui(w).act.find(a=>a.k==='b'&&a.id===id);check.ok(a,`native building action ${id}`);return a.size;}
function site(w,id,region){
 const s=size(w,id),cx=Math.floor(w.territory_stat(region,1)),cy=Math.floor(w.territory_stat(region,2)),errors={};
 for(let r=0;r<25;r++)for(let dx=-r;dx<=r;dx++)for(const at of [[cx+dx,cy-r],[cx+dx,cy+r],[cx-r,cy+dx],[cx+r,cy+dx]]){
  const [x,y]=at;if(x<3||y<3||x+s>253||y+s>253)continue;
  if(w.territory_at(x+s/2,y+s/2)!==region)continue;
  const error=w.place_error(id,x,y);if(error===0)return {at,s};errors[error]=(errors[error]||0)+1;
 }
 assert.fail(`no controlled building site for ${id} in ${region}; errors ${JSON.stringify(errors)}; selected ${JSON.stringify(ui(w).sel)}; workers ${JSON.stringify(own(w,0,8).map(e=>[e.x,e.y,e.order]))}`);
}
function build(w,id,region){instant(w);const {at,s}=site(w,id,region);check.equal(w.cmd_place(id,...at),1);run(w,5);const e=own(w,0,id).find(e=>Math.abs(e.x-at[0]-s/2)<.01&&Math.abs(e.y-at[1]-s/2)<.01);check.ok(e);check.equal(e.built,1);if(id===0){for(let i=0;i<40&&owner(w,region)!==0;i++)run(w,1);check.equal(owner(w,region),0,'Completed adjacent city acquires the fixture province before further selection');}return e;}
async function make({ai=2,humans=1,seat=0,scenario=true,wasm=bytes}={}){
 const w=(await WebAssembly.instantiate(wasm,{})).instance.exports;
 for(const name of ['cmd_territory','territory_at','territory_count','territory_owner','territory_stat','territory_offer_stat'])check.equal(typeof w[name],'function','build territory ABI first: '+name);
 w.init(71,ai,0,humans,seat);
 if(scenario){check.equal(humans,1);check.equal(w.skirmish_epoch(3),1);check.equal(w.skirmish_visibility(2),1);const b=save(w),l=layout(b);assertCurrentSave(b);
  for(let t=0;t<4;t++){b[36+t*l.psz+27]=0;for(let s=0;s<w.resource_count();s++)l.d.setFloat32(s<3?36+t*l.psz+4*s:l.eco+8+(t*6+s-3)*4,100000,true);l.d.setFloat32(l.end+20+4*t,10000,true);}
  b.fill(3,l.terrain,l.terrain+NN);b.fill(0,l.res,l.res+NN);b.fill(0,l.amt,l.amt+2*NN);revealFixture(b);check.equal(load(w,b),1);
  for(let a=0;a<3;a++)for(let c=a+1;c<3;c++)peace(w,a,c);
 }
 return w;
}
// Source-native acquisition now requires a completed TC and adjacency, or an
// additional completed Fortress for an isolated province. These diplomacy
// fixtures deliberately choose adjacent provinces, preserving their original
// intended unencumbered TC-only payload and all transfer/count assertions.
function freeRegion(w,exclude=[]){return Array.from({length:w.territory_count()},(_,i)=>i).find(i=>owner(w,i)===255&&!exclude.includes(i)&&w.territory_claim_stat(0,i,1)===1);}
async function simple(){
 const w=await make(),home0=own(w,0,0)[0],home1=own(w,1,0)[0];
 const a=freeRegion(w);check.notEqual(a,undefined);const townA=build(w,0,a);
 const b=freeRegion(w);check.notEqual(b,undefined);const townB=build(w,0,b);
 // Independent initial ownership fixture, not a territory gift used to set up
 // its own expected outcome. The two unencumbered provinces each contain a TC.
 let saved=save(w),l=layout(saved),tc=entities(saved).find(e=>e.handle===townB.handle);saved[tc.at+2]=1;check.equal(load(w,saved),1);
 check.equal(owner(w,a),0);check.equal(owner(w,b),1);check.equal(w.cheat(1),1);
 return {w,a,b,townA:find(w,townA),townB:find(w,townB),home0,home1};
}

async function full(){
 const f=await simple(),{w,a,home0,home1}=f;
 const uni=build(w,47,a),house=build(w,1,a),scaffold=build(w,1,a);
 // A recipient university is installed as a valid completed scenario building;
 // research itself is then completed through the real authenticated command.
 const home1Region=w.territory_at(home1.x,home1.y),home0Region=w.territory_at(home0.x,home0.y),recipientUni=build(w,47,home0Region);
 let saved=save(w),l=layout(saved),r=entities(saved).find(e=>e.handle===recipientUni.handle);saved[r.at+2]=1;move(saved,r,home1.x+8,home1.y+5);check.equal(w.territory_at(home1.x+8,home1.y+5),home1Region);check.equal(load(w,saved),1);
 const researchRemote=(id,host)=>{deliver(w,1,wire(11,0,0,64+id,[host.handle]));run(w,305);check.equal(w.research_stat(1,2,id),1,'recipient completes real research');};
 researchRemote(4,home1);researchRemote(3,home1);researchRemote(32,find(w,recipientUni));
 const discount=audit.nodes.find(n=>n.id==='Economic1_4').effects.find(e=>e.field===4&&e.targets.includes(8));check.equal(discount.method,0);check.equal(discount.value,-10);
 check.equal(w.unit_cost_stat(1,8,0),Math.round(native(8,3,'cost9')[0]*.9),'recipient has its actual researched cheaper citizen cost');check.notEqual(w.unit_cost_stat(0,8,0),w.unit_cost_stat(1,8,0),'refund fixture distinguishes old and new cost templates');
 check.equal(w.cmd_research(47),1);run(w,305);check.equal(w.research_stat(0,2,47),1,'donor earns its fourth university slot');
 instant(w);select(w,home0);check.equal(w.cmd_train(8),1);run(w,3);check.equal(own(w,0,8).length,6);
 check.equal(w.cheat(1),1);saved=save(w);l=layout(saved);const workers=entities(saved).filter(e=>e.team===0&&e.def===8),crew=workers.slice(0,4);
 crew.forEach((e,i)=>{move(saved,e,uni.x+2.1*Math.cos(i*Math.PI/2),uni.y+2.1*Math.sin(i*Math.PI/2));if(i===0)l.d.setFloat32(e.at+40,native(8,3,'hp')*.75,true);});
 const carrier=workers[4],outside=workers[5],foreign=entities(saved).find(e=>e.team===2&&e.def===8);
 move(saved,carrier,uni.x+5,uni.y+4);move(saved,outside,home0.x+4,home0.y+4);move(saved,foreign,uni.x-5,uni.y+4);
 l.d.setFloat32(carrier.at+40,native(8,3,'hp')*.5,true);l.d.setFloat32(carrier.at+116,7.5,true);saved[carrier.at+86]=2;
 const under=entities(saved).find(e=>e.handle===scaffold.handle);l.d.setFloat32(under.at+80,.4,true);l.d.setFloat32(under.at+40,native(1,3,'hp')*.3,true);
 check.equal(load(w,saved),1);deliver(w,0,wire(6,uni.x,uni.y,uni.handle,crew.map(e=>e.handle)));run(w,100);
 check.equal(find(w,uni).cargo.length,4);check.equal(w.research_stat(1,3,8),native(8,3,'hp')*1.2);check.equal(w.research_stat(0,3,8),native(8,3,'hp'));
 check.equal(w.cheat(1),1);
 // Real pending work and queues, issued immediately before the tested gift.
 select(w,find(w,outside));w.cmd_context(scaffold.x,scaffold.y,0);check.equal(find(w,outside).order,5);
 select(w,find(w,f.townA));const cost=Array.from({length:9},(_,s)=>w.unit_cost_stat(0,8,s));check.equal(w.cmd_train(8),1);
 const researchBefore=w.research_stat(0,0,0);check.equal(w.cmd_research(32),1);const researchPaid=researchBefore-w.research_stat(0,0,0);
 // Population includes pending training after the next fixed simulation step;
 // both queues remain unfinished and are then transferred/cancelled live.
 run(w,1);
 check.ok(researchPaid>0);check.equal(find(w,f.townA).queue.length,1);check.equal(find(w,uni).queue.length,1);
 report.adapterPolicy=ui(w).diplomacy.territoryPolicy;
 return {...f,uni,house,scaffold,carrier,outside,foreign,cost,researchPaid};
}

async function giftChecks(){
 const f=await full(),{w,a,uni,house,scaffold,carrier,outside,foreign,townA,cost,researchPaid}=f;
 const before=save(w),all=entities(before),provinceObjects=all.filter(e=>e.team===0&&w.territory_at(e.x,e.y)===a),ids=all.map(e=>e.handle).sort((x,y)=>x-y),balances=banks(w),points=[0,1,2].map(t=>w.research_stat(t,0,0));
 const crewBefore=find(w,uni),mobileBefore=find(w,carrier),cap0=w.stat(0,2),cap1=w.stat(1,2),pop0=w.stat(0,1),pop1=w.stat(1,1),texture=terr(w);
 const probe=site(w,1,a).at,miniBefore=(()=>{const p=w.minimap();return Array.from(new Uint8Array(w.memory.buffer,p+(probe[1]*N+probe[0])*4,4));})();
 check.equal(w.place_error(1,...probe),0);check.equal(w.cheat_status(0),0,'territory transfers do not require cheats');
 check.equal(local(w,1,GIFT,a),1);check.equal(owner(w,a),1);check.equal(w.terr_dirty(),1);
 const after=entities(save(w));check.deepEqual(after.map(e=>e.handle).sort((x,y)=>x-y),ids,'a gift changes ownership without cloning or deleting actors');
 for(const e of provinceObjects){const current=find(w,e);check.equal(current.team,1,'all donor objects in the province transfer exactly once');check.equal(current.order,0);check.equal(current.target,NONE);close(current.built,e.built,'construction fraction preserved');close(current.hp/w.research_stat(1,3,e.def),e.hp/w.research_stat(0,3,e.def),'health percentage preserved');}
 check.equal(find(w,foreign).team,2,'third-party troops remain theirs');check.equal(find(w,outside).team,0,'troops outside the province are not given');check.equal(find(w,outside).order,0,'outside donor builder stops following its now-foreign building');
 check.deepEqual(find(w,uni).cargo,crewBefore.cargo);check.deepEqual(find(w,uni).cargoHealth,crewBefore.cargoHealth);check.equal(find(w,uni).cargo.length,4,'excess garrison is preserved despite the receiver\'s lower active slot limit');close(find(w,carrier).carry,mobileBefore.carry,'carried goods are not remotely duplicated into banks');
 check.equal(find(w,townA).queue.length,0);check.equal(find(w,uni).queue.length,0);check.equal(find(w,townA).queueTime,0);
 check.deepEqual(stock(w,0),balances[0].map((v,s)=>v+cost[s]),'old owner alone receives all nine queue cost refunds');check.deepEqual(stock(w,1),balances[1]);check.deepEqual(stock(w,2),balances[2]);close(w.research_stat(0,0,0),points[0]+researchPaid,'old owner receives research-point cancellation refund');close(w.research_stat(1,0,0),points[1],'research bank is not given away');
 check.equal(w.research_stat(0,2,32),0);check.equal(w.research_stat(1,2,32),1);check.equal(w.research_stat(0,2,47),1);check.equal(w.research_stat(1,2,47),0,'completed technologies remain with their original player');
 const nativeTC=native(0,3,'pop_capacity'),nativeHouse=native(1,3,'pop_capacity');close(w.stat(0,2),cap0-nativeTC-nativeHouse,'donor loses the transferred completed building capacity');close(w.stat(1,2),cap1+nativeTC+nativeHouse+2,'recipient house uses its own source-backed +2 population technology');
 check.equal(w.stat(0,1),pop0-6,'donor loses one field citizen, four garrisoned citizens and one cancelled queued citizen');check.equal(w.stat(1,1),pop1+5,'receiver gains only the five existing citizens');
 check.equal(w.place_error(1,...probe),2,'old owner loses build rights');const expected=texture.slice();for(let t=0;t<NN;t++)if(expected[t*4]===a)expected[t*4+1]=2;check.deepEqual(terr(w),expected,'province ID and border texture persist with exactly the new owner channel');
 const p=w.minimap(),miniAfter=Array.from(new Uint8Array(w.memory.buffer,p+(probe[1]*N+probe[0])*4,4));check.notEqual(miniAfter.join(','),miniBefore.join(','),'minimap ownership tint changes');
 const state=save(w),h=w.state_hash();run(w,80);check.equal(owner(w,a),1,'grid recount does not restore the donor');check.equal(load(w,state),1);check.equal(w.state_hash(),h);check.equal(owner(w,a),1,'gift survives save/load');
 invalid(w,()=>local(w,1,GIFT,a),'a donor cannot give the same no-longer-owned province twice');
 const pointsStart=w.research_stat(1,0,0);run(w,1200);const generation=native(47,3,'generation')*native(47,3,'garrison_slots')*60;check.ok(Math.abs(w.research_stat(1,0,0)-pointsStart-generation)<1,'only recipient slot-limited garrison works; all four passengers remain alive');check.equal(find(w,uni).cargo.length,4);
 const workersBefore=own(w,1,8).map(e=>e.handle),beforeProduction=stock(w,1),newCost=Array.from({length:9},(_,s)=>w.unit_cost_stat(1,8,s));deliver(w,1,wire(11,0,0,8,[townA.handle]));check.deepEqual(stock(w,1),beforeProduction.map((v,s)=>v-newCost[s]),'new production pays the recipient researched cost, not the donor refund template');run(w,520);const born=own(w,1,8).filter(e=>!workersBefore.includes(e.handle));check.equal(born.length,1);close(born[0].hp,native(8,3,'hp')*1.2,'new production uses recipient native class and researched HP');
 const beforeUnload=own(w,1,8).length;deliver(w,1,wire(7,uni.x,uni.y,0,[uni.handle]));run(w,2);check.equal(find(w,uni).cargo.length,0);check.equal(own(w,1,8).length,beforeUnload+4,'unloading returns every preserved passenger exactly once');
 completed.push('persistent gift, province texture/minimap/build rights, actor identity, own-versus-third-party troops, construction/health, crew/slot limits, refund conservation and recipient native research/production');
}

function waitSwap(w,from,to){const n=w.territory_offer_stat(from,to,4)>>>0;check.ok(n<=100,'bounded territory offer cooldown');run(w,n);check.equal(w.territory_offer_stat(from,to,4),0);}
function unchangedOwners(w,a,b,oa=0,ob=1){check.equal(owner(w,a),oa);check.equal(owner(w,b),ob);}
async function swapChecks(){
 const {w,a,b,townA,townB}=await simple();const original=banks(w),territories=[w.stat(0,7),w.stat(1,7)];
 check.equal(local(w,1,SWAP,a,b),1);unchangedOwners(w,a,b);check.equal(w.territory_offer_stat(0,1,0),1);
 check.deepEqual(player(w,1).territoryOutgoing,{give:a,want:b,expires:60});check.equal(player(w,1).territoryCooldown,5,'UI expiry and cooldown are seconds');
 invalid(w,()=>local(w,1,SWAP,a,b),'duplicate pending offer rejected');remote(w,1,0,REJECT);check.equal(player(w,1).territoryOutgoing,null);unchangedOwners(w,a,b);
 waitSwap(w,0,1);check.equal(local(w,1,SWAP,a,b),1);check.equal(local(w,1,WITHDRAW),1);check.equal(w.territory_offer_stat(0,1,0),0);unchangedOwners(w,a,b);
 waitSwap(w,0,1);check.equal(local(w,1,SWAP,a,b),1);run(w,1201);check.equal(w.territory_offer_stat(0,1,0),0);unchangedOwners(w,a,b);
 check.equal(local(w,1,SWAP,a,b),1);dip(w,0,1,2);run(w,1);check.equal(w.territory_offer_stat(0,1,0),0,'war invalidates an offered swap');unchangedOwners(w,a,b);peace(w,0,1);waitSwap(w,0,1);
 check.equal(local(w,1,SWAP,a,b),1);const ids=entities(save(w)).map(e=>e.handle).sort((x,y)=>x-y);remote(w,1,0,ACCEPT);
 unchangedOwners(w,a,b,1,0);check.equal(find(w,townA).team,1);check.equal(find(w,townB).team,0);check.deepEqual(banks(w),original,'an unencumbered swap moves no stock account');check.deepEqual(entities(save(w)).map(e=>e.handle).sort((x,y)=>x-y),ids);
 check.equal(w.stat(0,7),territories[0]);check.equal(w.stat(1,7),territories[1]);check.equal(player(w,1).territoryOutgoing,null);check.equal(player(w,1).territoryIncoming,null);
 const saved=save(w),h=w.state_hash();run(w,200);unchangedOwners(w,a,b,1,0);check.equal(load(w,saved),1);check.equal(w.state_hash(),h);unchangedOwners(w,a,b,1,0);invalid(w,()=>local(w,1,ACCEPT),'an already consumed swap cannot be accepted twice');
 // Ownership changes are rechecked at acceptance; neither leg may execute.
 for(const change of ['give','want']){
  const f=await simple();check.equal(local(f.w,1,SWAP,f.a,f.b),1);
  if(change==='give')check.equal(local(f.w,2,GIFT,f.a),1);else remote(f.w,1,2,GIFT,f.b);
  check.equal(f.w.territory_offer_stat(0,1,0),0,'ownership change invalidates the old contract');const before=f.w.state_hash();remote(f.w,1,0,ACCEPT);check.equal(f.w.state_hash(),before,'invalid acceptance cannot execute even one transfer leg');unchangedOwners(f.w,f.a,f.b,change==='give'?2:0,change==='want'?2:1);
 }
 // Older saves permitted several cities in one province. New placement follows
 // the original one-TC limit, but legacy loading and anchor-generation safety
 // must retain their coverage. Relocate an existing city only in this explicit
 // saved-world fixture instead of issuing an illegal new placement command.
 const loss=await simple(),second=loss.home0;let legacyPoint;
 for(const r of [10,12,8])for(const [dx,dy]of [[r,0],[-r,0],[0,r],[0,-r]]){
  const x=loss.townA.x+dx,y=loss.townA.y+dy;
  if(x>4&&y>4&&x<252&&y<252&&loss.w.territory_at(x,y)===loss.a){legacyPoint=[x,y];break;}
 }
 check.ok(legacyPoint,'separate valid city centers in the legacy province');let legacy=save(loss.w);move(legacy,entities(legacy).find(e=>e.handle===second.handle),...legacyPoint);resetFixtureClaims(legacy);check.equal(load(loss.w,legacy),1,'legacy multi-TC province remains loadable');
 check.equal(owner(loss.w,loss.a),0);deliver(loss.w,0,wire(13,0,0,0,[loss.townA.handle]));run(loss.w,25);check.equal(owner(loss.w,loss.a),0,'a second live legacy city anchors the same claim');
 const replacement=build(loss.w,1,loss.a);check.notEqual(replacement.handle,loss.townA.handle,'reused entity slots have fresh generation handles');deliver(loss.w,0,wire(13,0,0,0,[second.handle]));run(loss.w,25);check.equal(owner(loss.w,loss.a),255,'losing the last city unclaims that province despite a surviving house');invalid(loss.w,()=>local(loss.w,1,GIFT,loss.a),'lost claim cannot be gifted');const afterLoss=save(loss.w);check.equal(load(loss.w,afterLoss),1);check.equal(owner(loss.w,loss.a),255);
 completed.push('reject/withdraw/expiry/cooldown, atomic swap, ownership races, claim destruction/generation reuse and persistent two-way ownership');
}

async function validationChecks(){
 const {w,a,b}=await simple();
 for(const [target,action,give,want,label]of [
  [0,GIFT,a,NONE,'self gift'],[99,GIFT,a,NONE,'invalid player'],[257,GIFT,a,NONE,'wrapped player alias'],[1,99,a,NONE,'invalid action'],[1,256,a,NONE,'wrapped action alias'],[1,GIFT,b,NONE,'not our province'],[1,GIFT,999,NONE,'invalid province'],[1,GIFT,a+256,NONE,'wrapped province alias'],[1,GIFT,a,b,'gift has no wanted province'],[1,SWAP,a,a,'same province both ways'],[1,SWAP,a,999,'invalid requested province'],[1,SWAP,a,b+256,'wrapped wanted province alias'],[1,ACCEPT,a,NONE,'accept with unexpected fields'],[1,ACCEPT,NONE,NONE,'no incoming contract'],[1,REJECT,NONE,NONE,'no incoming rejection'],[1,WITHDRAW,NONE,NONE,'no outgoing withdrawal']
 ])invalid(w,()=>local(w,target,action,give,want),label);
 const packets=[packet(1,99,a),packet(99,GIFT,a),wire(15,.5,-1,1),wire(15,-2,-1,1),wire(15,a,Infinity,1),wire(15,NaN,-1,1),wire(15,a,-1,-1),wire(15,a,-1,0x10001),packet(1,GIFT,a).slice(0,13)];
 const handles=wire(15,a,-1,1,[own(w,2,0)[0].handle]);packets.push(handles);
 for(const p of packets){const h=w.state_hash();deliver(w,0,p);check.equal(w.state_hash(),h,'malformed C15 does not alias a valid action or entity owner');}
 for(const sender of [3,4,256,NONE]){const h=w.state_hash();deliver(w,sender,packet(1,GIFT,a));check.equal(w.state_hash(),h,'strict sender validation before u8 conversion');}
 const h=w.state_hash();deliver(w,2,packet(1,GIFT,a));check.equal(w.state_hash(),h,'valid third player cannot give another player\'s province');
 deliver(w,2,wire(13,0,0,0,own(w,2).filter(e=>e.def<8||e.def>=47).map(e=>e.handle)));run(w,25);check.equal(player(w,2).alive,false);invalid(w,()=>local(w,2,GIFT,a),'departed recipient rejected');
 // Field counts from an unknown foreign region must not become a map cheat.
 const hidden=await make({scenario:false});const unknown=ui(hidden).diplomacy.territories.find(p=>!p.known);check.ok(unknown);check.equal(unknown.owner,255);check.equal(unknown.lastTerritory,false);check.equal(unknown.buildings,0);check.equal(unknown.units,0);check.equal(unknown.garrison,0);check.equal(unknown.countsComplete,false);
 completed.push('invalid IDs/fields/players, dead recipients, authenticated strict C15 wire and unknown-province information limits');
}

async function finalProvinceChecks(){
 const w=await make(),home=own(w,0,0)[0],id=w.territory_at(home.x,home.y);check.equal(province(w,id).lastTerritory,true);
 run(w,100);dip(w,0,1,1,1);dip(w,1,0,3);dip(w,0,2,1,1);dip(w,2,0,3);check.equal(w.diplomacy_stat(0,1,0),2);check.equal(w.diplomacy_stat(0,2,0),2);
 check.equal(local(w,1,GIFT,id),1,'the final province can be knowingly given');check.equal(w.stat(0,7),0);for(const other of [1,2]){check.equal(w.diplomacy_stat(0,other,0),1);check.equal(w.diplomacy_stat(other,0,0),1);check.equal(w.diplomacy_stat(0,other,2),0);check.equal(w.diplomacy_stat(other,0,2),0);}
 run(w,25);check.equal(ui(w).win,2,'giving every building away has the advertised self-defeat consequence');const lost=save(w);check.equal(load(w,lost),1,'last-province treaty termination logs are valid saved state');check.equal(w.stat(0,7),0);
 const s=await make(),a=s.territory_at(own(s,0,0)[0].x,own(s,0,0)[0].y),b=s.territory_at(own(s,1,0)[0].x,own(s,1,0)[0].y);run(s,100);dip(s,0,1,1,1);dip(s,1,0,3);check.equal(local(s,1,SWAP,a,b),1);remote(s,1,0,ACCEPT);check.equal(s.stat(0,7),1);check.equal(s.stat(1,7),1);check.equal(s.diplomacy_stat(0,1,0),2);check.equal(s.diplomacy_stat(1,0,0),2);check.equal(s.diplomacy_stat(0,1,2),1);run(s,25);check.equal(ui(s).win,0,'atomic last-province swap never creates an intermediate defeat');
 completed.push('allowed final-province gift/self-defeat and alliance termination; atomic final-province swap retains both players/treaties');
}

async function economyGateChecks(){
 for(const [def,kind,slot,unlock]of [[53,8,7,9],[54,9,8,12]]){
  const f=await simple(),{w,a}=f;let b=save(w),l=layout(b);b[36+24]=unlock;b[36+l.psz+24]=0;check.equal(load(w,refreshFixtureCityGround(b)),1);
  const at=site(w,1,a).at,s=size(w,def),tx=at[0],ty=at[1],tile=(ty+Math.floor(s/2))*N+tx+Math.floor(s/2);b=save(w);l=layout(b);b[l.res+tile]=kind;l.d.setUint16(l.amt+2*tile,2000,true);check.equal(load(w,b),1);const rig=build(w,def,a);
  b=save(w);const crew=entities(b).filter(e=>e.team===0&&e.def===8).slice(0,2);crew.forEach((e,i)=>move(b,e,rig.x+2,rig.y+i*.6));check.equal(load(w,b),1);deliver(w,0,wire(6,rig.x,rig.y,rig.handle,crew.map(e=>e.handle)));run(w,80);check.equal(find(w,rig).cargo.length,2);check.equal(w.cheat(1),1);
  const targetBefore=w.resource_stat(1,slot);check.equal(local(w,1,GIFT,a),1);check.equal(w.resource_active(1,slot),0);run(w,200);check.equal(w.resource_stat(1,slot),targetBefore,'inactive recipient cannot generate gifted special resource');check.equal(find(w,rig).carry,0);check.equal(find(w,rig).cargo.length,2,'locked extraction retains its citizens');
  b=save(w);l=layout(b);b[36+l.psz+24]=unlock;check.equal(load(w,refreshFixtureCityGround(b)),1);const before=w.resource_stat(1,slot);run(w,200);close(w.resource_stat(1,slot)-before+find(w,rig).carry,15,'unlocked recipient resumes two native 0.75/s slots',.03);
 }
 const f=await simple(),{w,a}=f;let b=save(w),l=layout(b);b[36+l.psz+24]=9;const workers=entities(b).filter(e=>e.team===0&&e.def===8).slice(0,2),tc=find(w,f.townA);
 workers.forEach((e,i)=>{move(b,e,tc.x+4,tc.y+4+i);l.d.setFloat32(e.at+116,7.5,true);b[e.at+86]=i===0?3:2;});check.equal(load(w,refreshFixtureCityGround(b)),1);const balances=banks(w);check.equal(local(w,1,GIFT,a),1);check.equal(find(w,workers[0]).carry,0,'expired iron cargo follows normal return-cargo clearing');check.equal(find(w,workers[0]).cargoResource,0);check.equal(find(w,workers[0]).returning,0,'expired delivery has no stale hauling state');check.equal(find(w,workers[1]).carry,7.5,'active wood cargo is conserved');check.equal(find(w,workers[1]).cargoResource,2);check.deepEqual(banks(w),balances,'cargo migration never credits remote or duplicate bank resources');
 completed.push('gifted OilRig/UraniumMine retain crew but respect recipient unlocks; expired versus valid carried resource conservation');
}

async function aiChecks(){
 const {w,a,b}=await simple();let state=save(w),l=layout(state);state[36+l.psz+27]=1;check.equal(load(w,state),1);
 check.equal(local(w,1,SWAP,a,b),1);run(w,79);unchangedOwners(w,a,b);run(w,2);check.equal(w.territory_offer_stat(0,1,0),0);unchangedOwners(w,a,b);check.ok(ui(w).diplomacy.log.some(text=>text.includes('gleichwertig')||text.includes('10 Prozent')),'AI explains rejection of an equal neutral swap requiring a premium');
 // A clearly more valuable, healthy package should be accepted without gifts
 // or hidden command privileges. Several actual houses dominate a single TC.
 for(let i=0;i<4;i++)build(w,1,a);check.equal(w.cheat(1),1);waitSwap(w,0,1);check.equal(local(w,1,SWAP,a,b),1);run(w,81);unchangedOwners(w,a,b,1,0);check.equal(w.territory_offer_stat(0,1,0),0);
 completed.push('deterministic delayed AI valuation: rejects insufficient neutral offer with reason, accepts a strictly richer package');
}

async function saveChecks(){
 const {w,a,b}=await simple();check.equal(local(w,1,SWAP,a,b),1);const saved=save(w),l=layout(saved);assertCurrentSave(saved);check.equal(saved.length-l.end,236+DIP_BYTES+TER_BYTES+l.cityBytes);check.equal(l.d.getUint32(l.ter,true),0x31524554);check.equal(l.d.getUint32(l.ter+4,true),1);check.equal(TER_BYTES,524);
 const h=w.state_hash(),before=ui(w).diplomacy;run(w,5);check.equal(load(w,saved),1);check.equal(w.state_hash(),h);check.deepEqual(ui(w).diplomacy,before,'pending province offers, claims, logs and timers survive save/load');
 for(const [offset,type,value]of [[0,'u32',0],[4,'u32',2],[8,'u32',0xffff0000],[136,'u8',2],[139,'u8',1],[136+12+1,'u8',255],[328,'u32',13]]){
  const bad=saved.slice(),d=new DataView(bad.buffer);type==='u32'?d.setUint32(l.ter+offset,value,true):bad[l.ter+offset]=value;check.equal(load(w,bad),0,'malformed territory extension '+offset);check.equal(w.state_hash(),h,'invalid load leaves active ownership intact');
 }
 check.equal(load(w,saved.slice(0,-1)),0);check.equal(w.state_hash(),h);
 const meta=JSON.parse(readFileSync(new URL('fixtures/territory-legacy-dip1.json',import.meta.url))),legacy=gunzipSync(readFileSync(new URL('fixtures/territory-legacy-dip1.sav.gz',import.meta.url)));
 check.equal(hash(legacy),meta.saveSha256);check.equal(meta.wasmSha256,'991e4736d420a14bc853d81b99c6d5ee6f9b0884809e4314051aa294739a3d75');check.equal(legacy.length-layout(legacy).end,236+DIP_BYTES,'genuine 991E/DIP1 save contains no invented TER1');check.equal(load(w,legacy),1);
 for(let t=0;t<2;t++){check.deepEqual(stock(w,t),meta.stockByTeam[t]);check.equal(w.stat(t,1),meta.popByTeam[t]);for(let to=0;to<2;to++)for(let field=0;field<7;field++)check.equal(w.diplomacy_stat(t,to,field)>>>0,meta.diplomacyByPair[t][to][field]);}
 for(const town of own(w,0,0))check.equal(owner(w,w.territory_at(town.x,town.y)),0);for(const town of own(w,1,0))check.equal(owner(w,w.territory_at(town.x,town.y)),1);const upgraded=save(w),newHash=w.state_hash();assertCurrentSave(upgraded);check.equal(load(w,upgraded),1);check.equal(w.state_hash(),newHash,'legacy upgrade is applied only once');report.legacyFixtureSha256=meta.saveSha256;
 completed.push('exact TER1 structure, claims/pending offer roundtrip, atomic malformed saves and genuine 991E/DIP1 migration with full old diplomacy state');
}

async function networkChecks(){
 const ws=[await make({ai:0,humans:2,seat:0,scenario:false}),await make({ai:0,humans:2,seat:1,scenario:false})],queues=[[],[]];for(const w of ws)check.equal(w.skirmish_visibility(2),1);
 const ls=ws.map((w,seat)=>new Lockstep(w,seat,b=>queues[1-seat].push(b)));
 function advance(n){for(let t=0;t<n;t++)for(let seat=0;seat<2;seat++){for(const b of queues[seat].splice(0))ls[seat].receive(b);check.equal(ls[seat].step(),true);}check.equal(ws[0].state_hash(),ws[1].state_hash());check.ok(!ls[0].desync&&!ls[1].desync);}
 check.equal(ws[0].cmd_diplomacy(1,0,0,0),1);advance(8);check.equal(ws[1].cmd_diplomacy(0,3,0,0),1);advance(108);check.equal(ws[0].cmd_diplomacy(1,1,1,0),1);advance(8);check.equal(ws[1].cmd_diplomacy(0,3,0,0),1);advance(8);
 const a=ws[0].territory_at(own(ws[0],0,0)[0].x,own(ws[0],0,0)[0].y),b=ws[0].territory_at(own(ws[0],1,0)[0].x,own(ws[0],1,0)[0].y);
 check.equal(ws[0].cmd_territory(1,SWAP,a,b),1);check.equal(ws[0].territory_offer_stat(0,1,0),0,'MP offers do not apply before their scheduled turn');advance(8);check.equal(ws[1].territory_offer_stat(0,1,0),1);check.equal(ws[1].cmd_territory(0,ACCEPT,NONE,NONE),1);advance(8);
 for(const w of ws){unchangedOwners(w,a,b,1,0);check.equal(w.diplomacy_stat(0,1,0),2);check.equal(w.diplomacy_stat(0,1,2),1);check.equal(w.stat(0,7),1);check.equal(w.stat(1,7),1);}advance(1000);
 check.equal(ws[1].cmd_territory(0,GIFT,a,NONE),1);advance(8);for(const w of ws){unchangedOwners(w,a,b,0,0);check.equal(w.stat(1,7),0);}advance(80);check.equal(ui(ws[0]).win,1);check.equal(ui(ws[1]).win,2);report.lockstepTurns=ls[0].turn;
 const base=await make({ai:1,scenario:false,wasm:baseBytes});const home=own(base,0,0)[0],id=base.territory_at(home.x,home.y);check.equal(base.resource_count(),3);check.equal(base.cmd_territory(1,GIFT,id,NONE),1);check.equal(owner(base,id),1);assertCurrentSave(save(base));
 completed.push('two actual peers: queued swap/accept, persistent claims/alliance and final gift with matching hashes; base-profile gift/save smoke');
}

async function runTests(){await giftChecks();await swapChecks();await validationChecks();await finalProvinceChecks();await economyGateChecks();await aiChecks();await saveChecks();await networkChecks();}
try{await runTests();report.status='passed';}catch(error){report.status='failed';report.error=error.stack||String(error);throw error;}
finally{report.checkCount=checkCount;mkdirSync(reports,{recursive:true});writeFileSync(new URL('territory-tests.json',reports),JSON.stringify(report,null,2)+'\n');}
console.log(`PASS territory diplomacy: ${checkCount} checks; ${completed.join('; ')}`);
