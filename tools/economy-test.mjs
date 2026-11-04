// Independent end-to-end checks of the nine-resource EE2 ruleset.
// Expected resource windows/rates come from original DB/Ghidra research, not Rust.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {Lockstep} from '../docs/net.js';
import {assertCurrentSave,revealFixture,DIP_BYTES,TER_BYTES,saveLayout,refreshFixtureCityGround} from './save-fixture.mjs';

const wasm=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const source=JSON.parse(readFileSync(new URL('../docs/ee2-economy.json',import.meta.url),'utf8'));
const make=async()=> (await WebAssembly.instantiate(wasm,{})).instance.exports;
const w=await make();
for(const name of ['resource_count','resource_stat','resource_active','unit_cost_stat','resource_kind'])assert.equal(typeof w[name],'function',`new economy ABI ${name}; rebuild WASM before this test`);
assert.equal(w.ruleset(),1);assert.equal(w.resource_count(),9);
const N=256,NN=N*N,H=36;
const nativeBySlot=[0,1,5,2,3,4,6,7,8];
const run=(engine,n)=>{for(let i=0;i<n;i++)engine.tick();};
const save=engine=>{const p=engine.save();return new Uint8Array(engine.memory.buffer,p,engine.save_len()).slice();};
const load=(engine,b)=>{const p=engine.load_alloc(b.length);new Uint8Array(engine.memory.buffer,p,b.length).set(b);return engine.load_apply();};
const ui=engine=>{const p=engine.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(engine.memory.buffer,p,engine.ui_len())));};
const pick=(engine,x,y,same=0)=>{engine.select_none();engine.select_point(x,y,0,same,0,0,N,N);};
const stock=(engine,team=0)=>Array.from({length:9},(_,slot)=>engine.resource_stat(team,slot));
const hasUnload=engine=>ui(engine).act.some(action=>action.k==='u');
const close=(actual,expected,label,epsilon=.003)=>assert.ok(Math.abs(actual-expected)<=epsilon,`${label}: ${actual}, expected ${expected}`);

function layout(bytes){
 const d=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),entityBytes=d.getUint32(4,true),playerBytes=d.getUint32(8,true),high=d.getUint32(32,true);
 const entities=H+4*playerBytes,terrain=entities+high*entityBytes,resources=terrain+NN,amount=resources+NN,fog=amount+2*NN,end=fog+NN;
 return {d,entityBytes,playerBytes,high,entities,terrain,resources,amount,fog,end,eco:end+132};
}
function ents(bytes){
 const l=layout(bytes),out=[];
 for(let i=0;i<l.high;i++){
  const at=l.entities+i*l.entityBytes;
  out.push({i,at,alive:!!bytes[at],def:bytes[at+1],team:bytes[at+2],x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),hp:l.d.getFloat32(at+40,true),order:bytes[at+48],built:l.d.getFloat32(at+80,true),carry:l.d.getFloat32(at+116,true)});
 }
 return out;
}
function moveEntity(bytes,e,x,y){const d=layout(bytes).d;for(const [off,v]of [[8,x],[12,y],[16,x],[20,y]])d.setFloat32(e.at+off,v,true);}
function setAccount(bytes,team,slot,value){const l=layout(bytes);const at=slot<3?H+team*l.playerBytes+4*slot:l.eco+8+(team*6+slot-3)*4;l.d.setFloat32(at,value,true);}
function fixture(epoch=0,seed=7){
 w.init(seed,0,0,1,0);const b=save(w),l=layout(b);
 assert.equal(l.d.getUint32(12,true),57,'current native definition count including Fortress');assert.equal(b.length-l.end,236+DIP_BYTES+TER_BYTES+saveLayout(b).cityBytes,'ECO1 plus DIP1/TER1 save trailer');assert.equal(l.d.getUint32(l.eco,true),0x314f4345);assertCurrentSave(b);
 for(let t=0;t<4;t++){b[H+t*l.playerBytes+24]=epoch;b[H+t*l.playerBytes+27]=0;for(let s=0;s<9;s++)setAccount(b,t,s,10000);}
 revealFixture(b);assert.equal(load(w,b),1);return save(w);
}
function own(engine,def,team=0){return ents(save(engine)).filter(e=>e.alive&&e.team===team&&e.def===def);}
function selectEntity(engine,e){pick(engine,e.x,e.y);assert.equal(ui(engine).sel.def,e.def,'selected the intended entity');}
function findSite(engine,id){
 const worker=own(engine,8,ui(engine).local??0)[0];assert.ok(worker,'A real local citizen exists for placement preview');selectEntity(engine,worker);
 const x=Math.floor(engine.start_x()),y=Math.floor(engine.start_y());
 for(let r=4;r<55;r++)for(let q=-r;q<=r;q++)for(const p of [[x+q,y-r],[x+q,y+r],[x-r,y+q],[x+r,y+q]])if(engine.place_error(id,...p)===0)return p;
 assert.fail(`No site found for definition ${id}`);
}
function instantOn(engine){assert.equal(engine.cheat(0),1);if(!engine.cheat_status(1))assert.equal(engine.cheat(3),1);}
function build(engine,id,at=findSite(engine,id)){
 const worker=own(engine,8,ui(engine).local??0)[0];assert.ok(worker,'A real local citizen exists for construction');selectEntity(engine,worker);
 instantOn(engine);assert.equal(engine.cmd_place(id,...at),1,`place definition ${id}`);run(engine,5);
 const b=own(engine,id).find(e=>Math.abs(e.x-at[0])<5&&Math.abs(e.y-at[1])<5);assert.ok(b,`created definition ${id}`);assert.equal(b.built,1,`completed definition ${id}`);return b;
}
function clearPatch(bytes,cx,cy,r=5){
 const l=layout(bytes);
 for(let y=cy-r;y<=cy+r;y++)for(let x=cx-r;x<=cx+r;x++){
  const t=y*N+x;bytes[l.terrain+t]=3;bytes[l.resources+t]=0;l.d.setUint16(l.amount+2*t,0,true);bytes[l.fog+t]=255;
 }
}
function node(bytes,x,y,kind,amount=2000){const l=layout(bytes),t=y*N+x;bytes[l.resources+t]=kind;l.d.setUint16(l.amount+2*t,amount,true);return t;}

// Read-only ABI and all 15 epochs: source windows have both lower and upper limits.
const base=fixture();
for(let epoch=0;epoch<15;epoch++){
 const b=base.slice(),l=layout(b);b[H+24]=epoch;assert.equal(load(w,refreshFixtureCityGround(b)),1);
 for(let slot=0;slot<9;slot++){
  const original=source.naturalResources[nativeBySlot[slot]],first=original.activeEpochFirst-1,end=original.activeEpochEndExclusive===null?15:original.activeEpochEndExclusive-1;
  assert.equal(w.resource_active(0,slot),Number(epoch>=first&&epoch<end),`resource ${original.name}, epoch ${epoch+1}`);
 }
}
assert.equal(w.resource_active(99,0),0);assert.equal(w.resource_active(0,99),0);assert.equal(w.resource_stat(0,99),-1);
fixture();const counts=new Array(10).fill(0);for(let t=0;t<NN;t++){const kind=w.resource_kind(t);assert.ok(kind>=0&&kind<=9);counts[kind]++;}
for(let kind=1;kind<=9;kind++)assert.ok(counts[kind]>0,`map contains resource kind ${kind}`);

// Native cost/stats: TC requires stone, and a citizen takes 25 seconds with 30 HP.
assert.deepEqual(Array.from({length:9},(_,s)=>w.unit_cost_stat(0,0,s)),[0,300,0,300,0,0,0,0,0]);
assert.deepEqual(Array.from({length:9},(_,s)=>w.unit_cost_stat(0,8,s)),[50,0,0,0,0,0,0,0,0]);
for(const e of own(w,8))close(e.hp,30,'initial citizen HP');
selectEntity(w,own(w,0)[0]);const oldCount=own(w,8).length,beforeCitizen=stock(w);
assert.equal(w.cmd_train(8),1);assert.deepEqual(stock(w),beforeCitizen.map((v,s)=>v-(s===0?50:0)));
run(w,490);assert.equal(own(w,8).length,oldCount,'citizen does not finish before 25 seconds');run(w,12);assert.equal(own(w,8).length,oldCount+1,'citizen finishes at 25 seconds');
const beforeTC=stock(w);build(w,0);assert.deepEqual(stock(w),beforeTC.map((v,s)=>v-([1,3].includes(s)?300:0)),'TC deducts wood AND stone');
const barracks=build(w,3);selectEntity(w,barracks);const cost=Array.from({length:9},(_,s)=>w.unit_cost_stat(0,9,s));
assert.deepEqual(cost,[35,0,0,0,15,10,0,0,0],'source-backed maceman archetype costs food/gold/tin');
const before=stock(w);assert.equal(w.cmd_train(9),1);assert.deepEqual(stock(w),before.map((v,s)=>v-cost[s]),'all nine slots participate in payment');w.cmd_cancel();assert.deepEqual(stock(w),before,'all nine slots refund exactly');
const poor=save(w);setAccount(poor,0,5,0);assert.equal(load(w,poor),1);selectEntity(w,own(w,3)[0]);const rejected=stock(w);assert.equal(w.cmd_train(9),0,'missing tin blocks training despite food/gold');assert.deepEqual(stock(w),rejected,'failed training changes no account');

// Exercise every special payment column, including the legacy third iron slot.
for(const [epoch,def,host,expected]of [
 [5,20,48,[0,95,65,0,95,0,0,0,0]],
 [6,15,3,[44,0,0,0,24,0,19,0,0]],
 [10,38,6,[0,100,0,0,180,0,0,140,0]],
 [12,39,6,[0,50,0,0,95,0,0,0,95]],
]){
 fixture(epoch);const producer=build(w,host);selectEntity(w,producer);
 assert.deepEqual(Array.from({length:9},(_,s)=>w.unit_cost_stat(0,def,s)),expected,`original nine-column cost for definition ${def}`);
 const balances=stock(w);assert.equal(w.cmd_train(def),1,`source producer trains ${def}`);
 assert.deepEqual(stock(w),balances.map((v,s)=>v-expected[s]),`definition ${def} pays the correct resource columns`);
 w.cmd_cancel();assert.deepEqual(stock(w),balances,`definition ${def} refunds every account exactly`);
}

// Carry cap and real hauling: stock is unchanged while the citizen holds wood.
fixture();let b=save(w),l=layout(b),tc=ents(b).find(e=>e.alive&&e.team===0&&e.def===0),worker=ents(b).find(e=>e.alive&&e.team===0&&e.def===8);
const nx=Math.floor(tc.x)+9,ny=Math.floor(tc.y)+1;clearPatch(b,nx,ny,5);const woodTile=node(b,nx,ny,2);moveEntity(b,worker,nx+.5,ny+1.4);assert.equal(load(w,b),1);
worker=own(w,8).find(e=>e.i===worker.i);selectEntity(w,worker);const woodBefore=w.resource_stat(0,1);w.cmd_context(nx+.5,ny+.5,0);run(w,100);
let carrying=own(w,8).find(e=>e.i===worker.i);assert.ok(carrying.carry>3.8&&carrying.carry<4.7,'citizen actually gathers into a carry account');assert.equal(w.resource_stat(0,1),woodBefore,'no remote stock credit while carrying');
let delivered=false;
for(let step=0;step<120;step++){
 run(w,10);carrying=own(w,8).find(e=>e.i===worker.i);assert.ok(carrying.carry<=15.001,'15-unit carry cap');
 if(w.resource_stat(0,1)>woodBefore){close(w.resource_stat(0,1)-woodBefore,15,'first complete load reaches TC');assert.ok(carrying.carry<.1,'delivery empties carry account');delivered=true;break;}
}
assert.ok(delivered,'worker walks back and delivers to town center');

// Warehouse garrison bonus: exactly +2% per citizen at the actual deposit.
fixture();const warehouse=build(w,52);b=save(w);worker=ents(b).find(e=>e.alive&&e.team===0&&e.def===8);moveEntity(b,worker,warehouse.x+2,warehouse.y);assert.equal(load(w,b),1);
selectEntity(w,own(w,8).find(e=>e.i===worker.i));w.cmd_context(warehouse.x,warehouse.y,0);run(w,10);selectEntity(w,own(w,52)[0]);assert.equal(ui(w).sel.cargo,1,'warehouse accepts a citizen');
assert.equal(hasUnload(w),true,'occupied warehouse exposes the unload action');
b=save(w);l=layout(b);worker=ents(b).find(e=>e.alive&&e.team===0&&e.def===8);moveEntity(b,worker,warehouse.x+1.2,warehouse.y+1.2);
b[worker.at+48]=4;b[worker.at+84]=2;b[worker.at+85]=1;b[worker.at+86]=2;l.d.setFloat32(worker.at+116,15,true);l.d.setUint32(worker.at+60,0xffffffff,true);l.d.setUint32(worker.at+64,woodTile,true);
assert.equal(load(w,b),1);const bonusBefore=w.resource_stat(0,1);run(w,1);close(w.resource_stat(0,1)-bonusBefore,15.3,'one garrisoned citizen gives two percent dropoff bonus');
selectEntity(w,own(w,52)[0]);assert.equal(hasUnload(w),true);w.cmd_unload();run(w,3);selectEntity(w,own(w,52)[0]);assert.equal(ui(w).sel.cargo,0);assert.equal(hasUnload(w),false,'empty warehouse hides unload action');

// The town center is also a dropoff garrison, with the same visible command.
fixture();tc=own(w,0)[0];selectEntity(w,tc);assert.equal(hasUnload(w),false,'empty town center has no unload action');
b=save(w);worker=ents(b).find(e=>e.alive&&e.team===0&&e.def===8);moveEntity(b,worker,tc.x+2.4,tc.y);assert.equal(load(w,b),1);
selectEntity(w,own(w,8).find(e=>e.i===worker.i));w.cmd_context(tc.x,tc.y,0);run(w,10);selectEntity(w,own(w,0)[0]);assert.equal(ui(w).sel.cargo,1);assert.equal(hasUnload(w),true,'occupied town center exposes unload action');
w.cmd_unload();run(w,3);selectEntity(w,own(w,0)[0]);assert.equal(ui(w).sel.cargo,0);assert.equal(hasUnload(w),false,'town-center unload action disappears when empty');

// Oil/Uranium: valid deposit placement, six garrison slots, integer generation, unload.
for(const [def,kind,slot,epoch]of [[53,8,7,9],[54,9,8,12]]){
 fixture(epoch);b=save(w);tc=ents(b).find(e=>e.alive&&e.team===0&&e.def===0);const tx=Math.floor(tc.x)+7,ty=Math.floor(tc.y)+4;
 clearPatch(b,tx+1,ty+1,5);node(b,tx+1,ty+1,kind,1000);assert.equal(load(w,b),1);
 const available=stock(w),testWorker=own(w,8)[0];selectEntity(w,testWorker);w.cmd_context(tx+1.5,ty+1.5,0);run(w,20);assert.equal(w.resource_stat(0,slot),available[slot],'direct citizen extraction is forbidden');assert.ok(own(w,8).every(e=>e.carry===0));
 assert.equal(w.place_error(def,tx,ty),0,'extractor accepts the matching deposit');assert.notEqual(w.place_error(def,tx+4,ty),0,'extractor rejects ordinary ground');
 const extractor=build(w,def,[tx,ty]);const emptyBefore=w.resource_stat(0,slot);run(w,100);assert.equal(w.resource_stat(0,slot),emptyBefore,'empty extractor generates nothing');
 selectEntity(w,extractor);assert.equal(hasUnload(w),false,'empty extractor hides unload action');
 selectEntity(w,own(w,0)[0]);assert.equal(w.cmd_train(8),1);assert.equal(w.cmd_train(8),1);run(w,5);assert.equal(own(w,8).length,7);
 b=save(w);for(const [k,e]of ents(b).filter(e=>e.alive&&e.team===0&&e.def===8).entries()){const angle=k*2.4;moveEntity(b,e,extractor.x+2.1*Math.cos(angle),extractor.y+2.1*Math.sin(angle));}
 assert.equal(load(w,b),1);const one=own(w,8)[0];pick(w,one.x,one.y,1);assert.equal(ui(w).sel.n,7);w.cmd_context(extractor.x,extractor.y,0);run(w,20);selectEntity(w,own(w,def)[0]);assert.equal(ui(w).sel.cargo,6);assert.equal(ui(w).sel.cargomax,6);assert.equal(own(w,8).length,1,'seventh citizen cannot enter a full extractor');
 assert.equal(hasUnload(w),true,'occupied extractor exposes unload action');
 const generatedBefore=w.resource_stat(0,slot);run(w,400);const generated=w.resource_stat(0,slot)-generatedBefore;assert.ok(generated>=89&&generated<=91,'six citizens generate 4.5 per second');assert.equal(generated%1,0,'native fractional accumulation pays whole units');
 const loadedSave=save(w),loadedHash=w.state_hash();run(w,20);assert.equal(load(w,loadedSave),1);assert.equal(w.state_hash(),loadedHash,'active extractor and fractional remainder survive reload');
 selectEntity(w,own(w,def)[0]);assert.equal(hasUnload(w),true);w.cmd_unload();run(w,3);selectEntity(w,own(w,def)[0]);assert.equal(ui(w).sel.cargo,0);assert.equal(hasUnload(w),false,'extractor unload action disappears when empty');const stopped=w.resource_stat(0,slot);run(w,100);assert.equal(w.resource_stat(0,slot),stopped,'unloading stops generation');
}

// All stock accounts clamp, preserve exact floats, and reject invalid extension data.
fixture();w.grant(1e9);assert.deepEqual(stock(w),new Array(9).fill(999999));w.grant(-1e9);assert.deepEqual(stock(w),new Array(9).fill(0));
b=save(w);for(let t=0;t<4;t++)for(let slot=0;slot<9;slot++)setAccount(b,t,slot,111*t+13*slot+.25);assert.equal(load(w,b),1);
const precise=Array.from({length:4},(_,t)=>stock(w,t)),roundtrip=save(w),roundHash=w.state_hash();run(w,10);assert.equal(load(w,roundtrip),1);assert.equal(w.state_hash(),roundHash);assert.deepEqual(Array.from({length:4},(_,t)=>stock(w,t)),precise);
const changed=roundtrip.slice();setAccount(changed,0,8,precise[0][8]+1);assert.equal(load(w,changed),1);assert.notEqual(w.state_hash(),roundHash,'extra stock participates in lockstep state hash');assert.equal(load(w,roundtrip),1);
const bad=roundtrip.slice();setAccount(bad,0,8,NaN);assert.equal(load(w,bad),0);assert.equal(w.state_hash(),roundHash,'invalid save does not replace active game');

// Genuine format migration: old 53/47 saves had three stocks and no ECO1 extension.
for(const defs of [53,47]){
 const current=fixture(),l=layout(current),legacy=current.slice(0,l.end+132),d=new DataView(legacy.buffer);d.setUint32(12,defs,true);
 for(let t=0;t<4;t++){legacy[H+t*l.playerBytes+24]=0;for(let s=0;s<3;s++)d.setFloat32(H+t*l.playerBytes+s*4,70+100*t+s+.5,true);}
 for(let tile=0;tile<NN;tile++)if(legacy[l.resources+tile]>3)legacy[l.resources+tile]=0;
 assert.equal(load(w,legacy),1,`legacy definition count ${defs}`);
 for(let t=0;t<4;t++)assert.deepEqual(stock(w,t).slice(0,3),[70+100*t+.5,71+100*t+.5,72+100*t+.5],`legacy ${defs} preserves old food/wood/iron accounts exactly`);
 assert.ok(stock(w).every(Number.isFinite));
}
// The immediately preceding 55-definition economy build already carried ECO1.
b=fixture();b=b.slice(0,saveLayout(b).legacyEnd);layout(b).d.setUint32(12,55,true);for(let s=0;s<9;s++)setAccount(b,0,s,230+s+.5);assert.equal(load(w,b),1,'55-definition economy save without future CGD1 remains compatible');assert.deepEqual(stock(w),Array.from({length:9},(_,s)=>230+s+.5));

// Two human clients plus an AI: real commands pass through the lockstep scheduler.
const [a,c]=[await make(),await make()];a.init(41,1,0,2,0);c.init(41,1,0,2,1);
const inboxes=[[],[]],clients=[new Lockstep(a,0,msg=>inboxes[1].push(msg)),new Lockstep(c,1,msg=>inboxes[0].push(msg))],engines=[a,c];
function networkRun(ticks){
 for(let tick=0;tick<ticks;tick++)for(let seat=0;seat<2;seat++){for(const msg of inboxes[seat].splice(0))clients[seat].receive(msg);assert.equal(clients[seat].step(),true,'peer commands arrive');}
 assert.equal(a.state_hash(),c.state_hash(),'two clients and AI remain deterministic');assert.ok(clients.every(client=>!client.desync),'no OOS');
 for(let team=0;team<3;team++)assert.deepEqual(stock(a,team),stock(c,team),'all nine resource accounts synchronize');
}
for(const [seat,engine]of engines.entries()){
 const workers=own(engine,8,seat),home=own(engine,0,seat)[0],at=findSite(engine,1);selectEntity(engine,workers[0]);assert.equal(engine.cmd_place(1,...at),1);
 let target,best=Infinity;for(let tile=0;tile<NN;tile++)if(engine.resource_kind(tile)===2){const distance=(tile%N-home.x)**2+(Math.floor(tile/N)-home.y)**2;if(distance<best){target=tile;best=distance;}}
 assert.notEqual(target,undefined);selectEntity(engine,workers[1]);engine.cmd_context(target%N+.5,Math.floor(target/N)+.5,0);selectEntity(engine,home);assert.equal(engine.cmd_train(8),1);assert.equal(engine.cheat(2),3,'multiplayer cheats remain blocked');
}
networkRun(2400);
for(const [seat,engine]of engines.entries()){assert.ok(own(engine,1,seat).some(e=>e.built===1),'each human completes a house');assert.ok(own(engine,8,seat).length>=6,'each human trains a citizen');}
assert.ok(stock(a,2).every(v=>Number.isFinite(v)&&v>=0),'AI handles the nine-account economy');
console.log('PASS economy: original costs/stats, all resource/epoch gates, hauling15 and dropoff bonus, Oil/Uran six-slot generation, nine-account cap/hash/save/legacy migration, two clients plus AI without OOS.');
