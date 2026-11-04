// Behavioural checks through the actual WASM command/save/render/network APIs.
// Controlled map/entity fixtures isolate diplomacy from procedural scouting and
// combat balance. No implementation source is parsed or substituted by a mock.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {Lockstep} from '../docs/net.js';
import {TER_BYTES,resetFixtureClaims,saveLayout,refreshFixtureCityGround} from './save-fixture.mjs';

const root=new URL('../',import.meta.url),docs=new URL('docs/',root);
const reportDirectory=new URL('../../EE2-Research/analysis/',import.meta.url);
const wasmBytes=readFileSync(new URL('openepoch-ee2.wasm',docs));
const baseBytes=readFileSync(new URL('openepoch.wasm',docs));
const sha=b=>createHash('sha256').update(b).digest('hex');
const N=256,NN=N*N,H=36,NONE=0xffffffff;
const DIP_BYTES=262744,DIP_MAGIC=0x31504944;
const economySource=JSON.parse(readFileSync(new URL('ee2-economy.json',docs)));
const nativeBySlot=[0,1,5,2,3,4,6,7,8];
const WAR=0,PEACE=1,ALLY=2;
const OFFER_PEACE=0,OFFER_ALLY=1,DECLARE_WAR=2,ACCEPT=3,REJECT=4,TRIBUTE=5,WITHDRAW=6,VISION=7;
let checkCount=0;
const check=Object.fromEntries(['ok','equal','notEqual','deepEqual'].map(name=>[name,(...args)=>{checkCount++;assert[name](...args);} ]));
const completed=[];
const report={scope:'WASM diplomacy, nine-resource tribute, natural versus display visibility, controlled live-projectile combat, save migration/validation and deterministic lockstep. No Engine edits or assertions about complete original-game equivalence.',
  wasmSha256:sha(wasmBytes),baseWasmSha256:sha(baseBytes),status:'running',checkCount:0,completed};
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
function layout(b){
 const d=new DataView(b.buffer,b.byteOffset,b.byteLength),psz=d.getUint32(8,true),esz=d.getUint32(4,true),high=d.getUint32(32,true),entities=H+4*psz;
 const terrain=entities+high*esz,resources=terrain+NN,amount=terrain+2*NN,fog=terrain+4*NN,end=terrain+5*NN;
 const prefix=d.getUint32(0,true)===0x3252454f?236:132;
 return {...saveLayout(b),d,psz,esz,high,entities,terrain,resources,amount,fog,end,eco:end+132,dip:end+prefix,prefix};
}
function entities(b){
 const l=layout(b),out=[];
 for(let i=0;i<l.high;i++){const at=l.entities+i*l.esz;out.push({i,at,alive:!!b[at],def:b[at+1],team:b[at+2],handle:((l.d.getUint16(at+4,true)<<16)|i)>>>0,
  x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),hp:l.d.getFloat32(at+40,true),order:b[at+48]});}
 return out;
}
const own=(w,team,id)=>entities(save(w)).filter(e=>e.alive&&e.team===team&&(id===undefined||e.def===id));
const stock=(w,team)=>Array.from({length:w.resource_count()},(_,slot)=>w.resource_stat(team,slot));
function setStock(b,team,slot,value){const l=layout(b),at=slot<3?H+team*l.psz+4*slot:l.eco+8+(team*6+slot-3)*4;l.d.setFloat32(at,value,true);}
function epoch(w,value){const b=save(w),l=layout(b);for(let t=0;t<4;t++)b[H+t*l.psz+24]=value;check.equal(load(w,refreshFixtureCityGround(b)),1);}
function move(b,e,x,y){const d=layout(b).d;for(const [offset,value]of [[8,x],[12,y],[16,x],[20,y],[24,0],[28,0]])d.setFloat32(e.at+offset,value,true);}
function clearPatch(b,cx,cy,r=20){
 const l=layout(b);
 for(let y=cy-r;y<=cy+r;y++)for(let x=cx-r;x<=cx+r;x++){const t=y*N+x;b[l.terrain+t]=3;b[l.resources+t]=0;l.d.setUint16(l.amount+2*t,0,true);}
}
async function make({ai=2,humans=1,seat=0,freeze=true,bytes=wasmBytes}={}){
 const w=(await WebAssembly.instantiate(bytes,{})).instance.exports;
 for(const name of ['cmd_diplomacy','diplomacy_stat','skirmish_visibility'])check.equal(typeof w[name],'function',`rebuild the diplomacy WASM first: ${name}`);
 w.init(71,ai,0,humans,seat);
 // Loading is deliberately restricted to single-player fixtures: a save has no
 // local network-seat identity, which must not silently replace an MP peer.
 if(freeze){check.equal(humans,1);const b=save(w),l=layout(b);for(let t=0;t<4;t++)b[H+t*l.psz+27]=0;check.equal(load(w,b),1);}
 return w;
}
function record(target,action,resource=0,amount=0){
 const bytes=new Uint8Array(14),d=new DataView(bytes.buffer);bytes[0]=14;bytes[1]=0;
 d.setFloat32(2,amount,true);d.setFloat32(6,0,true);d.setUint32(10,(target|(action<<8)|(resource<<16))>>>0,true);return bytes;
}
function deliver(w,team,bytes){const p=w.inbox_alloc(bytes.length);new Uint8Array(w.memory.buffer,p,bytes.length).set(bytes);w.net_apply(team);}
function remote(w,team,target,action,resource=0,amount=0){deliver(w,team,record(target,action,resource,amount));}
function local(w,target,action,resource=0,amount=0){
 const result=w.cmd_diplomacy(target,action,resource,amount);
 const n=w.outbox_len();if(n){const bytes=new Uint8Array(w.memory.buffer,w.outbox_ptr(),n).slice();w.outbox_clear();deliver(w,0,bytes);}
 return result;
}
const stat=(w,a,b,field=0)=>w.diplomacy_stat(a,b,field)>>>0;
function waitOffer(w,a,b){const ticks=stat(w,a,b,4);check.ok(ticks<=100,'offer cooldown has the documented finite limit');run(w,ticks);check.equal(stat(w,a,b,4),0);}
function agree(w,a,b,relation=PEACE,fullVision=0){
 waitOffer(w,a,b);remote(w,a,b,relation===ALLY?OFFER_ALLY:OFFER_PEACE,fullVision);
 check.equal(stat(w,a,b,1),relation===ALLY?2:1,'offer is pending before acceptance');
 remote(w,b,a,ACCEPT);check.equal(stat(w,a,b),relation);check.equal(stat(w,b,a),relation);
}
function invalid(w,fn,label){const hash=w.state_hash(),accounts=[0,1].map(t=>stock(w,t));check.equal(fn(),0,label);check.equal(w.state_hash(),hash,label+' changes no simulation state');check.deepEqual([0,1].map(t=>stock(w,t)),accounts);check.equal(w.outbox_len(),0);}
function render(w,box=[0,0,N,N]){
 const n=w.render(.5,...box,-1,0,0),p=w.inst_ptr(),data=new Float32Array(w.memory.buffer,p,n*12);
 return Array.from({length:n},(_,i)=>Array.from(data.subarray(i*12,i*12+12)));
}
function fog(w){return new Uint8Array(w.memory.buffer,w.fog_ptr(),NN).slice();}
function pixel(w,x,y){const p=w.minimap();return Array.from(new Uint8Array(w.memory.buffer,p+(Math.floor(y)*N+Math.floor(x))*4,4));}
const select=(w,e)=>{w.select_none();w.select_point(e.x,e.y,0,0,0,0,N,N);};

async function relationshipChecks(){
 const w=await make();check.equal(w.cheat_status(0),0,'diplomacy does not require cheats');
 check.equal(stat(w,0,1),WAR);check.equal(stat(w,1,0),WAR);
 const initialHash=w.state_hash();check.equal(local(w,1,OFFER_PEACE),1);check.notEqual(w.state_hash(),initialHash,'pending diplomacy participates in deterministic state hashing');check.equal(stat(w,0,1,1),1);check.equal(stat(w,0,1),WAR,'a unilateral offer cannot stop an opponent');
 check.equal(local(w,1,OFFER_PEACE),0,'duplicate/cooldown offers are rejected');
 remote(w,1,0,ACCEPT);check.equal(stat(w,0,1),PEACE);check.equal(stat(w,1,0),PEACE);check.equal(stat(w,0,1,1),0);
 waitOffer(w,0,1);check.equal(local(w,1,OFFER_ALLY,1),1);remote(w,1,0,REJECT);
 check.equal(stat(w,0,1),PEACE);check.equal(stat(w,0,1,1),0);check.equal(stat(w,0,1,2),0,'rejected alliance grants no sight');
 for(const full of [0,1]){
  waitOffer(w,0,1);check.equal(local(w,1,OFFER_ALLY,full),1);check.equal(stat(w,0,1,6),full);check.equal(ui(w).diplomacy.players.find(player=>player.team===1).outgoingAllianceVision,!!full,'UI preserves the proposed alliance policy');check.equal(stat(w,0,1,2),0,'an alliance offer cannot grant sight before acceptance');
  remote(w,1,0,ACCEPT);check.equal(stat(w,0,1),ALLY);check.equal(stat(w,1,0),ALLY);
  check.equal(stat(w,0,1,2),full);check.equal(stat(w,1,0,2),full,'alliance sight is a bilateral treaty right');
  if(full===0){check.equal(local(w,1,DECLARE_WAR),1);check.equal(stat(w,0,1),WAR);check.equal(stat(w,1,0),PEACE,'a declaration of war does not forge the other player\'s stance');agree(w,0,1);}
 }
 waitOffer(w,0,1);check.equal(local(w,1,VISION,0,0),1);check.equal(stat(w,0,1,1),4);check.equal(stat(w,0,1,2),1,'proposed sight removal does not change a live treaty');
 remote(w,1,0,REJECT);check.equal(stat(w,0,1,2),1);check.equal(stat(w,1,0,2),1);
 waitOffer(w,0,1);check.equal(local(w,1,VISION,0,0),1);remote(w,1,0,ACCEPT);check.equal(stat(w,0,1,2),0);check.equal(stat(w,1,0,2),0);
 waitOffer(w,0,1);check.equal(local(w,1,VISION,0,1),1);check.equal(stat(w,0,1,1),3);check.equal(stat(w,0,1,2),0);remote(w,1,0,ACCEPT);check.equal(stat(w,0,1,2),1);check.equal(stat(w,1,0,2),1);
 check.equal(local(w,1,DECLARE_WAR),1);check.equal(stat(w,0,1),WAR);check.equal(stat(w,1,0),PEACE);check.equal(stat(w,0,1,2),0);check.equal(stat(w,1,0,2),0);
 invalid(w,()=>local(w,1,VISION,0,1),'non-allies cannot negotiate shared sight');
 waitOffer(w,0,1);check.equal(local(w,1,OFFER_PEACE),1);check.equal(local(w,1,WITHDRAW),1);check.equal(stat(w,0,1,1),0);remote(w,1,0,ACCEPT);check.equal(stat(w,0,1),WAR,'a withdrawn offer cannot be accepted');
 waitOffer(w,0,1);check.equal(local(w,1,OFFER_PEACE),1);run(w,1201);check.equal(stat(w,0,1,1),0,'offers expire rather than becoming permanent permission');
 invalid(w,()=>local(w,0,OFFER_PEACE),'self offers rejected');invalid(w,()=>local(w,99,OFFER_PEACE),'nonexistent player rejected');
 invalid(w,()=>local(w,1,99),'unknown action rejected');invalid(w,()=>local(w,1,OFFER_ALLY,2),'invalid alliance sight policy rejected');
 check.ok(ui(w).diplomacy,'the actual UI exposes diplomacy state');completed.push('bilateral offers, stance direction, alliance sight handshake, reject/withdraw/expiry and validation');
}

async function tributeChecks(){
 const w=await make();check.equal(w.resource_count(),9);
 invalid(w,()=>local(w,1,TRIBUTE,2,1),'locked iron cannot be sent before the source resource window');
 for(let slot=0;slot<9;slot++){
  epoch(w,economySource.naturalResources[nativeBySlot[slot]].activeEpochFirst-1);
  const before=[stock(w,0),stock(w,1)];check.equal(local(w,1,TRIBUTE,slot,17),1,`tribute slot ${slot}`);
  check.deepEqual(stock(w,0),before[0].map((v,s)=>v-(s===slot?17:0)));check.deepEqual(stock(w,1),before[1].map((v,s)=>v+(s===slot?17:0)));
 }
 for(const amount of [0,-1,.5,NaN,Infinity,-Infinity,1000000,1e20])invalid(w,()=>local(w,1,TRIBUTE,0,amount),`invalid tribute amount ${amount}`);
 invalid(w,()=>local(w,1,TRIBUTE,9,1),'invalid resource slot rejected');invalid(w,()=>local(w,0,TRIBUTE,0,1),'self tribute rejected');
 invalid(w,()=>local(w,99,TRIBUTE,0,1),'invalid receiver rejected');invalid(w,()=>local(w,1,TRIBUTE,0,999999),'unaffordable tribute is atomic');
 let b=save(w);setStock(b,0,8,100);setStock(b,1,8,999994);check.equal(load(w,b),1);
 invalid(w,()=>local(w,1,TRIBUTE,8,6),'receiver cap overflow is rejected without debit');
 check.equal(local(w,1,TRIBUTE,8,5),1);check.equal(w.resource_stat(0,8),95);check.equal(w.resource_stat(1,8),999999);
 invalid(w,()=>local(w,1,TRIBUTE,8,1),'a full recipient cannot silently destroy tribute');
 completed.push('all nine tribute slots, exact amounts, affordability, recipient cap and atomic invalid transactions');
}

async function wireChecks(){
 const w=await make();
 const invalidPackets=[record(1,99),record(99,OFFER_PEACE),record(0,TRIBUTE,0,1),record(1,TRIBUTE,9,1),record(1,TRIBUTE,0,NaN),record(1,TRIBUTE,0,.5),record(1,TRIBUTE,0,1).slice(0,13)];
 const truncatedHandles=record(1,TRIBUTE,0,1);truncatedHandles[1]=2;invalidPackets.push(truncatedHandles);
 const reserved=record(1,OFFER_PEACE);new DataView(reserved.buffer).setUint32(10,0x01000001,true);invalidPackets.push(reserved);
 const negative=record(1,OFFER_PEACE);new DataView(negative.buffer).setInt32(10,-1,true);invalidPackets.push(negative);
 const unexpectedB=record(1,OFFER_PEACE);new DataView(unexpectedB.buffer).setFloat32(6,1,true);invalidPackets.push(unexpectedB);
 for(const bytes of invalidPackets){const hash=w.state_hash();deliver(w,0,bytes);check.equal(w.state_hash(),hash,'malformed wire record cannot change diplomacy/accounts');}
 for(const team of [3,4,256,NONE]){const hash=w.state_hash();deliver(w,team,record(1,TRIBUTE,0,1));check.equal(w.state_hash(),hash,'invalid or wrapped sender cannot act as another player');}
 const bytes=record(1,DECLARE_WAR),forged=new Uint8Array(18);forged.set(bytes);forged[1]=1;new DataView(forged.buffer).setUint32(14,own(w,2,8)[0].handle,true);
 agree(w,0,1);const thirdBefore=stat(w,2,1),before=w.state_hash();deliver(w,0,forged);check.equal(w.state_hash(),before,'diplomacy packets with unexpected handles are rejected');check.equal(stat(w,2,1),thirdBefore,'foreign handles cannot change the sender of a diplomacy command');
 completed.push('malformed/truncated/forged diplomacy wire records and sender authorization');
}

async function battle(kind='bullet',{friendlyBystander=true,distance=8}={}){
 const w=await make();if(friendlyBystander)agree(w,0,2);agree(w,1,2);
 let b=save(w),l=layout(b);clearPatch(b,132,128,24);
 for(let t=0;t<4;t++)b[H+t*l.psz+24]=10;
 // Real per-team sight, not the display override, makes combat targets visible.
 b.fill(255,l.dip+600,l.dip+DIP_BYTES);
 const shooter=entities(b).find(e=>e.alive&&e.team===0&&e.def===8),target=entities(b).find(e=>e.alive&&e.team===1&&e.def===0),other=entities(b).find(e=>e.alive&&e.team===2&&e.def===0);
 b[shooter.at+1]=kind==='bomb'?38:16;l.d.setFloat32(shooter.at+40,w.unit_stat(kind==='bomb'?38:16,0),true);move(b,shooter,128,128);move(b,target,128+distance,128);move(b,other,128+distance+.8,129);
 resetFixtureClaims(b);
 check.equal(load(w,refreshFixtureCityGround(b)),1);return {w,shooter:own(w,0,kind==='bomb'?38:16)[0],target:own(w,1,0)[0],other:own(w,2,0)[0],projectile:kind==='bomb'?109:105};
}
const health=(w,e)=>entities(save(w)).find(current=>current.i===e.i)?.hp;
function waitProjectile(w,id){
 for(let tick=0;tick<100;tick++){
  w.tick();if(render(w,[100,100,165,155]).some(row=>row[4]===id&&row[5]===0))return;
 }
 assert.fail('no actual live projectile was emitted by the controlled hostile shooter');
}
async function combatChecks(){
 for(const relation of [PEACE,ALLY]){
  const {w,shooter,target}=await battle();if(relation===ALLY)agree(w,0,1);agree(w,0,1,relation);
  const before=health(w,target);select(w,shooter);w.cmd_context(target.x,target.y,0);run(w,100);
  check.equal(health(w,target),before,'peace/alliance protects against context attacks and automatic target acquisition');
  const current=own(w,0,16)[0];check.notEqual(current.order,3,'a non-hostile click does not retain an attack order');
 }
 for(const kind of ['bullet','bomb']){
  const {w,shooter,target,other,projectile}=await battle(kind),before=health(w,target),otherBefore=health(w,other);
  select(w,shooter);w.cmd_context(target.x,target.y,0);waitProjectile(w,projectile);
  run(w,30);check.ok(health(w,target)<before,`${kind}: a genuine hostile projectile causes damage`);
  if(kind==='bomb')check.equal(health(w,other),otherBefore,'splash does not hit the adjacent peaceful third player');
 }
 for(const kind of ['bullet','bomb']){
  const {w,shooter,target,other,projectile}=await battle(kind);select(w,shooter);w.cmd_context(target.x,target.y,0);waitProjectile(w,projectile);
  const before=health(w,target),otherBefore=health(w,other);
  // No save/load occurs here: the projectile is alive in the real simulation.
  agree(w,0,1);run(w,40);check.equal(health(w,target),before,`${kind}: peace reached in flight protects the former enemy at impact`);
  check.equal(health(w,other),otherBefore);check.notEqual(own(w,0,kind==='bomb'?38:16)[0].order,3,'peace cancels an existing explicit attack');
 }
 completed.push('live direct/splash damage, peaceful third parties, non-hostile context/acquisition and in-flight treaty changes');
}

async function visibilityChecks(){
 const snapshots=[];
 for(const mode of [0,1,2]){
  const w=await make(),natural=save(w),l=layout(natural),enemy=own(w,1,8)[0],town=own(w,1,0)[0];
  const unseen=Array.from({length:NN},(_,t)=>t).find(t=>natural[l.fog+t]===0&&natural[l.terrain+t]===3);
  check.notEqual(unseen,undefined);check.equal(w.skirmish_visibility(mode),1);check.equal(ui(w).diplomacy.visibility,mode);
  const display=fog(w);check.equal(display[unseen],mode===0?0:mode===1?128:255);
  const view=render(w);check.equal(view.some(row=>row[4]===8&&row[5]===1),mode===2,'start visibility shows mobile opponents only when fog is disabled');
  check.equal(view.some(row=>row[4]===0&&row[5]===1),mode===2,'revealed terrain does not reveal an undiscovered enemy town center');
  const px=pixel(w,unseen%N,Math.floor(unseen/N));check.equal(px.slice(0,3).some(v=>v>0),mode!==0,'minimap terrain agrees with the chosen start mode');
  check.equal(w.place_error(1,Math.floor(town.x),Math.floor(town.y)),mode===0?3:2,'known start terrain changes building knowledge while foreign ownership still forbids placement');
  const probe=save(w);check.deepEqual(probe.subarray(layout(probe).dip+600),natural.subarray(l.dip+600),'display modes do not alter genuine per-team exploration');
  w.tick();const hash=w.state_hash();check.equal(w.skirmish_visibility((mode+1)%3),0,'visibility is a start option');check.equal(w.state_hash(),hash);
  check.ok(enemy.x>=0);snapshots.push({mode,display,pixel:px});
 }
 check.notEqual(snapshots[1].pixel.join(','),snapshots[2].pixel.join(','),'explored fog and fully visible minimap brightness differ');
 const stealth=await make();check.equal(stealth.skirmish_visibility(2),1);let b=save(stealth),l=layout(b),spy=entities(b).find(e=>e.alive&&e.team===1&&e.def===8);
 b[spy.at+1]=34;b[spy.at+104]=0;l.d.setFloat32(spy.at+108,0,true);move(b,spy,220.5,220.5);b[l.terrain+220*N+220]=0;b[l.resources+220*N+220]=0;l.d.setUint16(l.amount+2*(220*N+220),0,true);
 check.equal(load(stealth,b),1);check.equal(render(stealth).some(row=>row[4]===34&&row[5]===1),false,'full-map visibility does not detect a hidden submarine');
 b=save(stealth);spy=entities(b).find(e=>e.alive&&e.team===1&&e.def===34);b[spy.at+104]=1;check.equal(load(stealth,b),1);check.ok(render(stealth).some(row=>row[4]===34&&row[5]===1),'a detected submarine is displayed in full visibility');
 // Actual ally sight is a separate, consented source of natural visibility.
 const shared=await make();agree(shared,1,2);b=save(shared);const ally=entities(b).find(e=>e.alive&&e.team===1&&e.def===0),guest=entities(b).find(e=>e.alive&&e.team===2&&e.def===8);move(b,guest,ally.x+2,ally.y);check.equal(load(shared,b),1);run(shared,10);
 check.equal(render(shared).some(row=>row[4]===8&&row[5]===2&&Math.abs(row[0]-ally.x-2)<1),false);
 agree(shared,0,1);agree(shared,0,1,ALLY,1);check.ok(render(shared).some(row=>row[4]===8&&row[5]===2&&Math.abs(row[0]-ally.x-2)<1),'accepted full alliance sight reveals a unit visible to the ally');
 waitOffer(shared,0,1);check.equal(local(shared,1,VISION,0,0),1);check.ok(render(shared).some(row=>row[4]===8&&row[5]===2&&Math.abs(row[0]-ally.x-2)<1),'sight remains while the change is only an offer');remote(shared,1,0,ACCEPT);
 check.equal(render(shared).some(row=>row[4]===8&&row[5]===2&&Math.abs(row[0]-ally.x-2)<1),false,'accepted sight separation hides units outside own natural sight');
 completed.push('all start visibility modes, minimap/render consistency, mode-aware building terrain knowledge without natural-fog changes, stealth and consented bilateral ally sight');
}

async function aiChecks(){
 const w=await make({ai:2,freeze:false});check.equal(local(w,1,OFFER_PEACE),1);run(w,80);check.equal(stat(w,0,1),WAR);check.equal(stat(w,0,1,1),0,'AI rejects unsupported early peace');
 check.equal(local(w,1,TRIBUTE,1,100),1);waitOffer(w,0,1);check.equal(local(w,1,OFFER_PEACE),1);run(w,80);check.equal(stat(w,0,1),PEACE);check.equal(stat(w,1,0),PEACE);
 waitOffer(w,0,1);check.equal(local(w,1,OFFER_ALLY,1),1);run(w,80);check.equal(stat(w,0,1),PEACE,'AI does not skip the minimum peace period');
 check.equal(local(w,1,TRIBUTE,1,100),1);run(w,600);check.equal(local(w,1,OFFER_ALLY,1),1);run(w,80);check.equal(stat(w,0,1),ALLY,'AI accepts a mature supported alliance with a shared opponent');check.equal(stat(w,0,1,2),1);check.equal(stat(w,1,0,2),1);
 waitOffer(w,0,1);check.equal(local(w,1,VISION,0,0),1);check.equal(stat(w,1,0,2),1);run(w,80);check.equal(stat(w,0,1,2),0);check.equal(stat(w,1,0,2),0,'AI accepts valid allied sight changes after its negotiation delay');
 check.equal(local(w,1,DECLARE_WAR),1);check.equal(stat(w,1,0),PEACE);run(w,45);check.equal(stat(w,1,0),WAR,'AI makes its own delayed war declaration');
 const solo=await make({ai:1,freeze:false});check.equal(local(solo,1,TRIBUTE,1,400),1);check.equal(local(solo,1,OFFER_PEACE),1);run(solo,80);check.equal(stat(solo,0,1),PEACE);run(solo,600);check.equal(local(solo,1,OFFER_ALLY,1),1);run(solo,80);check.equal(stat(solo,0,1),ALLY,'1v1 can form a mature alliance with the documented higher tribute threshold');check.equal(ui(solo).win,0,'forming an alliance does not manufacture a victory over a living opponent');
 const a=await make({ai:1,freeze:false}),c=await make({ai:1,freeze:false});check.equal(a.skirmish_visibility(0),1);check.equal(c.skirmish_visibility(2),1);
 run(a,1000);run(c,1000);const ab=save(a),cb=save(c),al=layout(ab),cl=layout(cb);
 check.deepEqual(ab.subarray(0,al.fog),cb.subarray(0,cl.fog),'AI/player/entity/resource evolution does not change with display visibility');
 check.deepEqual(ab.subarray(al.dip+600),cb.subarray(cl.dip+600),'AI genuine sight maps remain identical across the display modes');
 check.notEqual(a.state_hash(),c.state_hash(),'different shared visibility settings participate in the state hash');
 completed.push('AI rejection/acceptance timing, mature/common-enemy/1v1 alliances, sight negotiation, war response and display-independent AI evolution');
}

async function saveChecks(){
 const w=await make();check.equal(w.skirmish_visibility(1),1);agree(w,0,1);agree(w,0,1,ALLY,1);waitOffer(w,0,1);check.equal(local(w,1,VISION,0,0),1);check.equal(local(w,2,TRIBUTE,1,17),1);
 const b=save(w),l=layout(b);check.equal(b.length-l.end,236+DIP_BYTES+TER_BYTES+l.cityBytes);check.equal(l.d.getUint32(l.eco,true),0x314f4345);check.equal(l.d.getUint32(l.eco+4,true),1);
 check.equal(l.d.getUint32(l.dip+DIP_BYTES,true),0x31524554);check.equal(l.d.getUint32(l.dip+DIP_BYTES+4,true),1);
 check.equal(l.d.getUint32(l.dip,true),DIP_MAGIC);check.equal(l.d.getUint32(l.dip+4,true),1);check.equal(l.d.getUint32(l.dip+8,true),1);check.equal(l.d.getUint32(l.dip+12,true),3);check.equal(l.d.getUint32(l.dip+16,true),1);
 const hash=w.state_hash(),before=ui(w).diplomacy;run(w,10);check.equal(load(w,b),1);check.equal(w.state_hash(),hash);check.deepEqual(ui(w).diplomacy,before,'pending sight policy, treaties, logs and visibility survive saving');
 for(const [offset,type,value]of [[0,'u32',0],[4,'u32',2],[8,'u32',3],[12,'u32',99],[16,'u32',3],[20+1,'u8',3],[36+1,'u8',5],[52+1,'u8',2],[68+1,'u8',2],[404,'u32',13],[600,'u8',127]]){
  const bad=b.slice(),d=new DataView(bad.buffer);type==='u32'?d.setUint32(l.dip+offset,value,true):bad[l.dip+offset]=value;check.equal(load(w,bad),0,`malformed DIP1 ${offset}`);check.equal(w.state_hash(),hash,'failed diplomacy load preserves the active game');
 }
 check.equal(load(w,b.slice(0,-1)),0);check.equal(w.state_hash(),hash);
 const legacyMeta=JSON.parse(readFileSync(new URL('fixtures/diplomacy-legacy56.json',import.meta.url))),legacy=gunzipSync(readFileSync(new URL('fixtures/diplomacy-legacy56.sav.gz',import.meta.url)));
 check.equal(sha(legacy),legacyMeta.saveSha256);check.equal(legacy.length-layout(legacy).end,236,'genuine old ECO1 fixture has no fabricated DIP1');check.equal(load(w,legacy),1);
 for(let t=0;t<2;t++){check.deepEqual(stock(w,t),legacyMeta.stockByTeam[t]);check.equal(w.stat(t,1),legacyMeta.popByTeam[t]);}
 check.equal(stat(w,0,1),WAR);check.equal(stat(w,1,0),WAR);check.equal(stat(w,0,1,1),0);check.equal(stat(w,0,1,2),0);check.equal(ui(w).diplomacy.visibility,0);
 report.legacyFixtureSha256=legacyMeta.saveSha256;
 completed.push('exact ECO1+DIP1 structure, pending treaties/logs/vision roundtrip, atomic malformed-save rejection and genuine 56-definition legacy migration');
}

async function networkChecks(){
 const peers=[await make({ai:0,humans:2,seat:0,freeze:false}),await make({ai:0,humans:2,seat:1,freeze:false})],queues=[[],[]];
 for(const w of peers)check.equal(w.skirmish_visibility(2),1);
 const locks=peers.map((w,seat)=>new Lockstep(w,seat,message=>queues[1-seat].push(message)));
 function advance(n){for(let tick=0;tick<n;tick++){for(let seat=0;seat<2;seat++){for(const message of queues[seat].splice(0))locks[seat].receive(message);check.equal(locks[seat].step(),true);}}check.equal(peers[0].state_hash(),peers[1].state_hash());check.ok(!locks[0].desync&&!locks[1].desync);}
 check.equal(peers[0].cmd_diplomacy(1,OFFER_PEACE,0,0),1);check.equal(stat(peers[0],0,1,1),0,'network orders are queued rather than applied early');advance(8);check.equal(stat(peers[1],0,1,1),1);
 check.equal(peers[1].cmd_diplomacy(0,ACCEPT,0,0),1);advance(8);check.equal(stat(peers[0],0,1),PEACE);check.equal(stat(peers[1],1,0),PEACE);
 advance(100);check.equal(peers[0].cmd_diplomacy(1,OFFER_ALLY,1,0),1);advance(8);check.equal(peers[1].cmd_diplomacy(0,ACCEPT,0,0),1);advance(8);check.equal(stat(peers[0],0,1),ALLY);check.equal(stat(peers[1],1,0,2),1);
 const balances=[stock(peers[0],0),stock(peers[0],1)];check.equal(peers[1].cmd_diplomacy(0,TRIBUTE,1,23),1);advance(8);check.equal(peers[0].resource_stat(0,1),balances[0][1]+23);check.equal(peers[1].resource_stat(1,1),balances[1][1]-23);
 advance(100);check.equal(peers[0].cmd_diplomacy(1,VISION,0,0),1);advance(8);check.equal(stat(peers[1],0,1,2),1);check.equal(peers[1].cmd_diplomacy(0,ACCEPT,0,0),1);advance(8);check.equal(stat(peers[0],0,1,2),0);check.equal(stat(peers[1],1,0,2),0);
 check.equal(peers[0].cmd_diplomacy(1,DECLARE_WAR,0,0),1);advance(8);check.equal(stat(peers[0],0,1),WAR);check.equal(stat(peers[1],1,0),PEACE);advance(1000);
 report.lockstepTurns=locks[0].turn;completed.push('two real lockstep peers: queued offers/acceptance, shared sight, tribute, directed war and long-run hashes');
 // The base profile shares diplomacy/settings without importing the nine-resource economy.
 const base=await make({ai:1,bytes:baseBytes});check.equal(base.ruleset(),0);check.equal(base.resource_count(),3);check.equal(base.skirmish_visibility(1),1);agree(base,0,1);check.equal(local(base,1,TRIBUTE,1,1),1);const saved=save(base),bl=layout(saved);check.equal(saved.length-bl.end,132+DIP_BYTES+TER_BYTES);check.equal(bl.d.getUint32(bl.dip,true),DIP_MAGIC);check.equal(bl.d.getUint32(bl.dip+DIP_BYTES,true),0x31524554);
 completed.push('base-profile diplomacy with its unchanged three-resource save prefix');
}

async function runTests(){
 await relationshipChecks();await tributeChecks();await wireChecks();await combatChecks();await visibilityChecks();await aiChecks();await saveChecks();await networkChecks();
}

try{await runTests();report.status='passed';}
catch(error){report.status='failed';report.error=error.stack||String(error);throw error;}
finally{report.checkCount=checkCount;mkdirSync(reportDirectory,{recursive:true});writeFileSync(new URL('diplomacy-tests.json',reportDirectory),JSON.stringify(report,null,2)+'\n');}
console.log(`PASS diplomacy: ${checkCount} checks; ${completed.join('; ')}`);
