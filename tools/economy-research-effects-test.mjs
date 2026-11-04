// Actual WASM behaviour, with source-derived oracles. No private modifier-table
// reads, engine edits, or generated-Rust regex checks are used by these scenarios.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {assertCurrentSave,revealFixture,DIP_BYTES,TER_BYTES,saveLayout} from './save-fixture.mjs';

const json=name=>JSON.parse(readFileSync(new URL('../docs/'+name,import.meta.url),'utf8'));
const economy=json('ee2-economy.json'),audit=json('research-audit.json');
const catalog=json('ee2-catalog.json'),units=json('ee2-units.json');
const w=(await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url)),{})).instance.exports;
assert.equal(w.ruleset(),1);assert.equal(w.resource_count(),9);
for(const name of ['skirmish_epoch','cmd_research','research_stat','resource_active','unit_cost_stat'])
 assert.equal(typeof w[name],'function','Current 57-definition economy WASM ABI: '+name);
const N=256,NN=N*N,H=36,DT=.05;
const allNodes=new Map(audit.nodes.map((node,index)=>[node.id,{...node,index}]));
const originalNodes=new Map(catalog.nodes.map(node=>[node.NAME,node]));
const hosts={CityCenter:0,Barracks:3,Mill:48,Stable:49,Dock:5,Airport:6,University:47,Warehouse:52,Temple:51,Market:50};
// Existing ABI resource order is explicit, not the original header enum order.
const resources={
 Food:{slot:0,kind:1,field:8},Wood:{slot:1,kind:2,field:9},
 Iron:{slot:2,kind:3,field:10},Stone:{slot:3,kind:4,field:12},
 Gold:{slot:4,kind:5,field:13},Tin:{slot:5,kind:6,field:14},
 Saltpeter:{slot:6,kind:7,field:15},Oil:{slot:7,kind:8,field:16,building:53},
 Uranium:{slot:8,kind:9,field:17,building:54},
};
for(const [name,resource]of Object.entries(resources)){
 resource.name=name;resource.original=economy.naturalResources.find(row=>row.name===name);
 assert.ok(resource.original,'Original economy resource '+name);
 assert.equal(audit.fields['Gather'+name+'_NoTooltip'],resource.field,name+' independent field ABI');
}
const basic=['Economic1_2','Economic6_2','Economic11_2'];
const special=['Economic4_2','Economic9_2','Economic14_2'];
const houseNodes=['Imperial3_1','Imperial8_1','Imperial13_1'];
const save=()=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=bytes=>{const p=w.load_alloc(bytes.length);new Uint8Array(w.memory.buffer,p,bytes.length).set(bytes);assert.equal(w.load_apply(),1,'Controlled current-format fixture loads');};
const run=count=>{for(let i=0;i<count;i++)w.tick();};
const ui=()=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
const stock=(team=0)=>Array.from({length:9},(_,slot)=>w.resource_stat(team,slot));
const close=(actual,expected,label,epsilon=.004)=>assert.ok(Math.abs(actual-expected)<=epsilon,label+': actual '+actual+', expected '+expected);
function layout(bytes){
 const d=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),esz=d.getUint32(4,true),psz=d.getUint32(8,true),high=d.getUint32(32,true);
 const entities=H+4*psz,terrain=entities+high*esz,resource=terrain+NN,amount=resource+NN,fog=amount+2*NN,end=fog+NN;
 return {d,esz,psz,high,entities,terrain,resource,amount,fog,end,eco:end+132};
}
function entities(bytes=save()){
 const l=layout(bytes),out=[];
 for(let i=0;i<l.high;i++){
  const at=l.entities+i*l.esz;
  if(bytes[at])out.push({i,at,def:bytes[at+1],team:bytes[at+2],x:l.d.getFloat32(at+8,true),y:l.d.getFloat32(at+12,true),hp:l.d.getFloat32(at+40,true),built:l.d.getFloat32(at+80,true),carry:l.d.getFloat32(at+116,true)});
 }
 return out;
}
const own=(id,bytes=save())=>entities(bytes).filter(e=>e.team===0&&e.def===id);
function select(e){w.select_none();w.select_point(e.x,e.y,0,0,0,0,N,N);assert.equal(ui().sel.def,e.def,'Select intended fixture entity');}
function move(bytes,e,x,y){
 const d=layout(bytes).d;
 for(const [offset,value]of [[8,x],[12,y],[16,x],[20,y],[24,0],[28,0]])d.setFloat32(e.at+offset,value,true);
}
function clear(bytes,cx,cy,radius){
 const l=layout(bytes);
 for(let y=Math.max(2,cy-radius);y<=Math.min(N-3,cy+radius);y++)for(let x=Math.max(2,cx-radius);x<=Math.min(N-3,cx+radius);x++){
  const t=y*N+x;bytes[l.terrain+t]=3;bytes[l.resource+t]=0;l.d.setUint16(l.amount+2*t,0,true);
 }
}
function fixture(epoch){
 w.init(7,0,0,1,0);assert.equal(w.skirmish_setup(1000,0),1);assert.equal(w.skirmish_epoch(epoch),1);
 const bytes=save(),l=layout(bytes);
 assert.equal(l.d.getUint32(12,true),57,'Source/engine definition version including native Fortress');
 assert.equal(bytes.length-l.end,236+DIP_BYTES+TER_BYTES+saveLayout(bytes).cityBytes,'RES1/ECO1 prefix plus DIP1/TER1 versioned fixture');assertCurrentSave(bytes);
 assert.equal(l.d.getUint32(l.end+12,true),0x31534552,'Research extension marker');
 assert.equal(l.d.getUint32(l.eco,true),0x314f4345,'Nine-bank extension marker');
 for(let team=0;team<4;team++){
  bytes[H+team*l.psz+27]=0; // Disable AI; do not fake completed research bits.
  for(let slot=0;slot<9;slot++){
   const at=slot<3?H+team*l.psz+4*slot:l.eco+8+(team*6+slot-3)*4;
   l.d.setFloat32(at,10000,true);
  }
  l.d.setFloat32(l.end+20+4*team,10000,true);
 }
 revealFixture(bytes);
 clear(bytes,Math.floor(w.start_x()),Math.floor(w.start_y()),24);
 load(bytes);
 assert.equal(w.stat(0,0),epoch);assert.equal(own(0).length,1);assert.equal(own(1).length,0);
}
function instant(){
 assert.equal(w.cheat(0),1);if(!w.cheat_status(1))assert.equal(w.cheat(3),1);
}
function site(id){
 const worker=own(8)[0];assert.ok(worker,'Actual citizen exists before placement preview');select(worker);
 const x=Math.floor(w.start_x()),y=Math.floor(w.start_y());
 for(let r=4;r<24;r++)for(let q=-r;q<=r;q++)for(const at of [[x+q,y-r],[x+q,y+r],[x-r,y+q],[x+r,y+q]])
  if(w.place_error(id,...at)===0)return at;
 assert.fail('No controlled owned site for '+id);
}
function build(id,at=site(id)){
 instant();assert.notEqual(w.select_idle_worker(),0xffffffff,'Idle builder exists');
 assert.equal(w.cmd_place(id,...at),1,'Actual placement gate accepts definition '+id);
 run(5);
 const e=own(id).sort((a,b)=>(a.x-at[0])**2+(a.y-at[1])**2-((b.x-at[0])**2+(b.y-at[1])**2))[0];
 assert.ok(e,'Built entity '+id);assert.equal(e.built,1,'Actual construction completes '+id);return e;
}
function research(keys){
 for(const key of keys){
  const node=allNodes.get(key),original=originalNodes.get(key);
  assert.ok(node&&original,'Verified original research key '+key);
  assert.ok(w.stat(0,0)>=original.EPOCH-1,'Fixture does not bypass the research epoch gate');
  const host=hosts[node.host];assert.notEqual(host,undefined,'Mapped original host '+node.host);
  if(!own(host).length)build(host);
  assert.equal(w.research_stat(0,2,node.index),0,'Research initially incomplete '+key);
  const before=w.research_stat(0,0,0);
  w.select_none();assert.equal(w.cmd_research(node.index),1,'Runtime research command '+key);
  close(w.research_stat(0,0,0),before-original.TECHPTS,'Actual technology payment '+key);
  run(Math.ceil(original.TIME/DT)+3);
  assert.equal(w.research_stat(0,2,node.index),1,'WASM completes original node '+key);
 }
}
function classRow(id,epoch){return units.class_rows.find(row=>row.id===id&&epoch>=row.epoch&&epoch<row.until);}
function active(resource,epoch){
 const source=resource.original;
 return epoch>=source.activeEpochFirst-1&&(source.activeEpochEndExclusive===null||epoch<source.activeEpochEndExclusive-1);
}
function multiplier(resource,keys,target){
 let percent=0;
 for(const key of keys)for(const effect of allNodes.get(key).effects){
  if(effect.targets.includes(target)&&effect.field===resource.field){
   assert.equal(effect.method,0,'Source AddPermMul percentage');percent+=effect.value;
  }
 }
 return 1+percent/100;
}
function natural(resource,epoch,keys){
 fixture(epoch);research(keys);
 const bytes=save(),l=layout(bytes),home=own(0,bytes)[0],worker=own(8,bytes)[0];
 const x=Math.floor(home.x)+10,y=Math.floor(home.y)+4,t=y*N+x;
 clear(bytes,x,y,4);bytes[l.resource+t]=resource.kind;l.d.setUint16(l.amount+2*t,1000,true);
 move(bytes,worker,x+.5,y+1.4);load(bytes);
 select(own(8).find(e=>e.i===worker.i));
 const before=stock(),other=stock(1);
 assert.equal(w.resource_active(0,resource.slot),Number(active(resource,epoch)),resource.name+' original epoch window');
 w.cmd_context(x+.5,y+.5,0);run(100);
 const carry=own(8).find(e=>e.i===worker.i).carry;
 const rate=resource.original.citizenConfiguredRate;
 const expected=active(resource,epoch)?rate*100*DT*multiplier(resource,keys,8):0;
 close(carry,expected,resource.name+' real carry after '+keys.join('+'));
 assert.deepEqual(stock(),before,'No remote stock credit or wrong resource column while carrying '+resource.name);
 assert.deepEqual(stock(1),other,'Research does not affect another owner');
 assert.ok(carry<economy.citizen.gatherFarm.carryLimit,'Observation stays below native hauling cap');
 return carry;
}
function extractor(resource,epoch,keys,{blocked=false}={}){
 fixture(epoch);research(keys);
 const source=economy.productionStructures.find(row=>row.gatherResource===resource.name);
 assert.ok(source,'Original extractor '+resource.name);
 const id=resource.building;
 instant();w.select_idle_worker();
 const action=ui().act.find(row=>row.k==='b'&&row.id===id),size=action?.size||2;
 const bytes=save(),l=layout(bytes),home=own(0,bytes)[0];
 const at=[Math.floor(home.x)+7,Math.floor(home.y)+4],cx=at[0]+Math.floor(size/2),cy=at[1]+Math.floor(size/2);
 clear(bytes,cx,cy,4);bytes[l.resource+cy*N+cx]=resource.kind;l.d.setUint16(l.amount+2*(cy*N+cx),1000,true);load(bytes);
 assert.equal(w.place_error(id,...at),blocked?8:0,'Matching deposit and terrain isolate the original build epoch gate');
 if(blocked){
  // Preview and command now validate the original epoch before payment; retain
  // the actual simulation/conservation checks as well as this stricter result.
  const balances=stock();w.select_idle_worker();assert.equal(w.cmd_place(id,...at),0,'Premature original extractor is rejected before command submission');run(5);
  assert.equal(own(id).length,0,resource.name+' native simulation gate rejects premature extractor');
  assert.deepEqual(stock(),balances,'Rejected extractor charges no resources');return;
 }
 assert.ok(epoch>=source.build.epoch-1,'Extractor is built only after source epoch');
 const e=build(id,at),b=save(),worker=own(8,b)[0];
 move(b,worker,e.x+size/2+.6,e.y);load(b);
 select(own(8).find(item=>item.i===worker.i));w.cmd_context(e.x,e.y,0);run(50);
 const building=own(id)[0];select(building);
 assert.equal(ui().sel.cargo,1,'A real citizen enters '+resource.name+' extractor');
 assert.equal(ui().sel.cargomax,source.garrisonSlots,'Original six-slot capacity remains unchanged by gather research');
 const reset=save(),resetBuilding=own(id,reset)[0];layout(reset).d.setFloat32(resetBuilding.at+116,0,true);load(reset);
 const before=stock(),other=stock(1);
 run(400);
 const after=stock(),remainder=own(id)[0].carry,credited=after[resource.slot]-before[resource.slot];
 assert.equal(credited%1,0,'Building pays whole stock units '+resource.name);
 assert.ok(remainder>=0&&remainder<1,'Building retains only fractional progress '+resource.name);
 const expected=source.gatherPerOccupiedSlotPerSecond*400*DT*multiplier(resource,keys,id);
 close(credited+remainder,expected,resource.name+' real stock plus remainder after '+keys.join('+'),.006);
 for(let slot=0;slot<9;slot++)if(slot!==resource.slot)assert.equal(after[slot],before[slot],'Extractor never credits unrelated resource slot '+slot);
 assert.deepEqual(stock(1),other,'Building research does not affect another owner');
 return credited+remainder;
}

const failures=[];let passed=0;
function check(name,fn){
 try{fn();passed++;}
 catch(error){failures.push(name+': '+error.message);console.error('FAIL '+name+': '+error.stack);}
}

for(const key of houseNodes)check('separate house PopCap '+key,()=>{
 fixture(12);const nativeTC=classRow(0,12),nativeHouse=classRow(1,12);
 assert.equal(w.stat(0,2),nativeTC.fields.pop_capacity.value,'Native current-epoch TC capacity baseline');
 close(w.research_stat(0,3,0),nativeTC.fields.hp.value,'Native current-epoch TC HP baseline');
 close(w.research_stat(0,3,1),nativeHouse.fields.hp.value,'Native current-epoch house HP baseline');
 const hp=Array.from({length:57},(_,id)=>w.research_stat(0,3,id)),cap=w.stat(0,2),otherCap=w.stat(1,2);
 research([key]);
 assert.equal(w.stat(0,2),cap,'House research does not increase TC/host capacity when no house exists');
 assert.equal(w.stat(1,2),otherCap,'House bonus never leaks to another player');
 assert.deepEqual(Array.from({length:57},(_,id)=>w.research_stat(0,3,id)),hp,'PopCap does not alter any unrelated HP mask, including Fortress');
 const effect=allNodes.get(key).effects.find(item=>item.field===18);
 assert.deepEqual(effect.targets,[1]);assert.equal(effect.method,1);assert.equal(effect.value,2);
 build(1);assert.equal(w.stat(0,2),cap+nativeHouse.fields.pop_capacity.value+effect.value,'Actual first-house capacity includes exactly source +2');
 build(1);assert.equal(w.stat(0,2),cap+2*(nativeHouse.fields.pop_capacity.value+effect.value),'Bonus is per house, not once globally');
});
check('all three house additions stack without changing TC',()=>{
 fixture(12);const cap=w.stat(0,2),native=classRow(1,12).fields.pop_capacity.value;
 research(houseNodes);assert.equal(w.stat(0,2),cap,'No-house capacity remains unchanged after all three technologies');
 build(1);assert.equal(w.stat(0,2),cap+native+6,'Three separate +2 additions produce +6 per house');
});

for(const key of basic){
 const epoch=Math.max(3,originalNodes.get(key).EPOCH-1);
 for(const name of ['Food','Wood','Stone','Gold'])check(key+' actual '+name+' gather',()=>natural(resources[name],epoch,[key]));
 const control=epoch<9?'Iron':'Saltpeter';
 check(key+' unrelated '+control+' stays at native rate',()=>natural(resources[control],epoch,[key]));
}
for(const name of ['Stone','Gold'])check('three basic tiers '+name+' additive/no tooltip double effect',()=>natural(resources[name],10,basic));
for(const key of special)for(const name of ['Tin','Iron','Saltpeter','Oil','Uranium']){
 const resource=resources[name],epoch=Math.max(originalNodes.get(key).EPOCH-1,resource.original.activeEpochFirst-1);
 check(key+' actual '+name+' rate/era gate',()=>resource.building?extractor(resource,epoch,[key]):natural(resource,epoch,[key]));
}
for(const name of ['Oil','Uranium']){
 const resource=resources[name];
 check('three special tiers '+name+' actual additive building generation',()=>extractor(resource,13,special));
 check('basic worker bonuses do not affect '+name+' building mask',()=>extractor(resource,13,basic));
 check(name+' native building unlock blocks earlier production',()=>extractor(resource,resource.original.activeEpochFirst-2,[special[0]],{blocked:true}));
}
check('Saltpeter remains gated before original epoch7 despite researched bonus',()=>natural(resources.Saltpeter,3,[special[0]]));
assert.equal(failures.length,0,failures.length+' WASM economy/research scenario(s) failed:\n'+failures.join('\n'));
console.log('PASS economy research effects: '+passed+' actual WASM scenarios; three house +2 factors and native baselines, separate basic/stone/gold/iron carry rates, tin/iron/saltpeter/oil/uranium fields, era/build gates, real extractor garrisons and integer stock+remainder, additive tiers and no tooltip/unrelated-owner/mask bonuses.');
