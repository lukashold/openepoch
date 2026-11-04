const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load}from'./roster6-state.mjs';import{saveLayout,refreshFixtureCityGround}from'./save-fixture.mjs';
import{ordinaryEntities,ordinaryUI,buildOrdinaryClassicFixture}from'./ordinary-classic-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),sha=b=>createHash('sha256').update(b).digest('hex');
const binary=fs.readFileSync(option('wasm',path.join(root,'docs/openepoch-ee2-up15.wasm'))),old61=fs.readFileSync(option('old61',path.join(root,'tools/fixtures/native61v2-up15.wasm')));
const make=async(b=binary)=>(await WebAssembly.instantiate(b,{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};const ok=(v,m)=>{assert.ok(v,m);checks++;};
const bytes=(a,b,m)=>{if(sha(a)!==sha(b)){console.log('byte-diff',m,Array.from(b.entries()).filter(([i,v])=>a[i]!==v).slice(0,12).map(([i,v])=>[i,a[i],v]));fs.writeFileSync(path.join(work,'roundtrip-before.oes'),b);fs.writeFileSync(path.join(work,'roundtrip-after.oes'),a);}eq(sha(a),sha(b),m);};
const entity=(w,id)=>ordinaryEntities(w).find(e=>e.team===0&&e.def===id);
function handle(w,e){const b=save(w),d=new DataView(b.buffer),at=36+4*d.getUint32(8,true)+e.slot*d.getUint32(4,true);return((d.getUint16(at+6,true)<<16)|e.slot)>>>0;}
const select=(w,e)=>w.select_point(e.x,e.y,0,0,0,0,256,256);
function row(w,e){const b=save(w),d=new DataView(b.buffer),at=36+4*d.getUint32(8,true)+e.slot*d.getUint32(4,true);return{qn:b[at+104],q:Array.from({length:b[at+104]},(_,j)=>d.getUint32(at+84+j*4,true)),qt:d.getFloat32(at+108,true)};}
const stock=w=>Array.from({length:9},(_,r)=>w.resource_stat(0,r));
const price=(w,h,p)=>Array.from({length:11},(_,r)=>w.queue_price(h,p,r));

// Construct only the initial controlled terrain/research state with the actual
// older61 loader. All payments, epoch orders and subsequent saves use public7.
const base=await make(old61);base.init(7,0,0,1,0);eq(base.skirmish_epoch(7),1);eq(base.skirmish_setup(500,2),1);eq(base.skirmish_visibility(2),1);
let b=save(base),l=saveLayout(b),d=l.d,at=36+4*d.getUint32(8,true),stride=d.getUint32(4,true),high=d.getUint32(32,true),map=at+stride*high;
const worker=Array.from({length:high},(_,i)=>at+i*stride).find(o=>b[o]&&b[o+4]===0&&d.getUint16(o+2,true)===8);ok(worker!==undefined);
const water=Array.from({length:65536},(_,i)=>i).find(i=>i>3*256&&i<253*256&&(i%256)>3&&(i%256)<253&&[i,i-1,i+1,i-256,i+256].every(t=>b[map+t]===0));ok(water!==undefined);
d.setUint16(worker+2,5,true);for(const[o,v]of[[8,water%256+.5],[12,Math.floor(water/256)+.5],[16,water%256+.5],[20,Math.floor(water/256)+.5],[24,0],[28,0],[40,base.up15_stat(0,5,0)],[80,1],[108,0]])d.setFloat32(worker+o,v,true);
b[worker+48]=0;b[worker+104]=0;for(let j=0;j<5;j++)d.setUint32(worker+84+j*4,0,true);for(let t=0;t<2;t++)b[36+t*d.getUint32(8,true)+27]=0;
d.setFloat32(l.end+20,10000,true);
const research=JSON.parse(fs.readFileSync(path.join(root,'docs/ee2-roster6-research.json'),'utf8'));
for(const epoch of[7,8,9]){
 const nodes=research.nodes.map((n,k)=>({n,k})).filter(({n})=>Number(n.raw_node.EPOCH)===epoch+1&&n.effects.every(e=>e.field!==4&&e.field!==3)).slice(0,6);eq(nodes.length,6);
 for(const{k}of nodes){const o=l.end+36+4*Math.floor(k/32);d.setUint32(o,d.getUint32(o,true)|(1<<(k%32)),true);}
}
b=refreshFixtureCityGround(b);eq(load(base,b),1);const fixture=save(base);
const a=await make(),peer=await make();eq(a.schema_revision(),7);eq(load(a,fixture),1);eq(load(peer,fixture),1);
const dock=entity(a,5),tc=entity(a,0),h=handle(a,dock);ok(dock&&tc);
const before=stock(a);select(a,dock);select(peer,dock);
for(let p=0;p<5;p++){eq(a.cmd_queue_job(58),1);eq(peer.cmd_queue_job(58),1);eq(a.queue_price_flags(h,p),2);eq(price(a,h,p).join(','),'0,200,0,160,0,290,0,0,0,0,3');}
eq(a.cmd_queue_job(58),0,'five-slot cap');eq(a.skirmish_epoch(9),0,'start settings cannot rewrite existing paid orders');
const queued=save(a),stableHash=a.state_hash(),layout=saveLayout(queued);eq(layout.paidqueue7,true);eq(layout.d.getUint32(layout.paid+8,true),5);
for(let p=0;p<5;p++)for(let r=0;r<11;r++)eq(a.queue_price(h,p,r),peer.queue_price(h,p,r));
const malformed=[];
for(const[offset,value]of[[0,0],[4,3],[8,4],[12,h^65536],[16,5],[20,57],[24,4],[28,0x7fc00000],[28,0x7f800000],[28,0xbf800000],[28,0x47800000],[28+9*4,1],[28+10*4,0x3e800000],[28+10*4,0x437e0000],[72,0x7fc00000],[72,0xc0000000],[72,0x4b000000],[72,0x42820000],[72+64,0x42820000]]){
 const bad=queued.slice();new DataView(bad.buffer).setUint32(layout.paid+offset,value,true);malformed.push(bad);
}
const duplicate=queued.slice();duplicate.copyWithin(layout.paid+12+64,layout.paid+12,layout.paid+12+64);malformed.push(duplicate);
for(const n of[1,4,8,20,52,53,60,100])malformed.push(queued.slice(0,queued.length-n));
for(const bad of malformed){eq(load(a,bad),0,'forged/truncated ledger');eq(a.state_hash(),stableHash,'atomic hash');bytes(save(a),queued,'atomic bytes');}
// Both peers follow real cmd_epoch and tick. Rendering/saving/query cadence is
// intentionally different; ledger state must have no UI-driven evolution.
const trace=[];
for(const target of[8,9,10]){
 select(a,tc);select(peer,tc);eq(a.cmd_epoch(),1);eq(peer.cmd_epoch(),1);
 let ticks=0;
 while(ordinaryUI(a).epoch<target&&ticks<400){
  a.tick();peer.tick();ticks++;
  if(ticks%13===0)a.render(.2,0,0,256,256,-1,0,0);
  if(ticks%17===0)peer.render(.8,0,0,256,256,-1,0,0);
  if(ticks%29===0){save(a);price(a,h,0);}
  eq(a.state_hash(),peer.state_hash(),'two peers after payment/epoch/tick');
 }
 eq(ordinaryUI(a).epoch,target,'real epoch completion');ok(ticks>0&&ticks<400);
 const q=row(a,dock);trace.push({epoch:target,ticks,queue:q.q,progress:q.qt,stock:stock(a)});
 if(target===8){eq(q.qn,5);eq(price(a,h,0)[5],290);}
 if(target===9){eq(q.qn,4);eq(price(a,h,0)[5],290,'active retained iron');for(let p=1;p<4;p++){eq(price(a,h,p)[5],0);eq(price(a,h,p)[6],307,'pending converted to saltpeter');}ok(q.qt>29);}
 if(target===10){eq(q.qn,0);eq(a.queue_price_flags(h,0)>>>0,0xffffffff);}
 const s=save(a),round=await make();eq(load(round,s),1);bytes(save(round),s,'canonical complete ledger');eq(round.state_hash(),a.state_hash());
}
eq(stock(a)[1],before[1],'all original wood eventually refunded');eq(stock(a)[2],before[2]-1160,'pending iron sold, active iron refunded');eq(stock(a)[6],before[6]+921,'new pending saltpeter refunded');eq(stock(a)[4],before[4]-30.5,'source market conversion net');
// New ordinary production remains playable through public commands only.
const ordinary=await make(),scene=buildOrdinaryClassicFixture(ordinary,{instantBuild:false});eq(scene.epoch,9);eq(scene.gameTick,4120);
for(const id of[57,58,59,60])ok(scene.entities.some(e=>e.def===id&&e.built>=1));
// Cross-profile saves remain atomic. OEU5/OEU6 migrate; the older programs do
// not silently consume the new financial state.
for(const legacy of[old61,fs.readFileSync(option('old5',path.join(root,'tools/fixtures/native5-up15.wasm'))),fs.readFileSync(option('old57',path.join(root,'docs/openepoch-ee2-roster6-phase1.wasm')))]){
 const old=await make(legacy);old.init(7,0,0,1,0);const oldState=save(old),oldHash=old.state_hash();eq(load(old,save(a)),0);eq(old.state_hash(),oldHash);bytes(save(old),oldState);
 const newer=await make();eq(load(newer,oldState),1);const s=save(newer);eq(saveLayout(s).paidqueue7,true);eq(load(newer,s),1);bytes(save(newer),s);
}
const report={format:'native-paidqueue7-actual-wasm-v1',status:'PASS',checks,wasmSha256:sha(binary),wasmBytes:binary.length,malformed:malformed.length,epochTrace:trace,ordinaryTicks:scene.gameTick,
 limits:['Fixtures establish initial controlled research/shore state through the older real loader; all new payments and epoch orders are public commands.',
 'Ledger preserves the actual smaller-engine price. Its existing u16 cost-calculation adapter is not a proof of fractional native unit costs.',
 'Reprice arithmetic models the source register/store graph in nearest53. Actual temporal native x87 rounding and live manual-market/Crown price modifiers remain explicit source gaps.',
 'Production-cycle mode is not exposed by this smaller engine; the source normal-mode pending-delete branch is implemented. Unpaid front jobs cannot progress without payment.',
 'Legacy saves lack historical invoices and are explicitly inferred; original paid-price reconstruction is impossible from them alone.']};
fs.writeFileSync(option('report',path.join(work,'paidqueue-wasm-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:'PASS',checks,malformed:malformed.length,wasmSha256:sha(binary),trace}));
