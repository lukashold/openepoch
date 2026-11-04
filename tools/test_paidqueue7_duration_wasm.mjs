const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load}from'./roster6-state.mjs';import{saveLayout,refreshFixtureCityGround}from'./save-fixture.mjs';import{ordinaryEntities,ordinaryUI}from'./ordinary-classic-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),blob=fs.readFileSync(option('wasm',path.join(root,'docs/openepoch-ee2-up15.wasm'))),oldBlob=fs.readFileSync(option('old61',path.join(root,'tools/fixtures/native61v2-up15.wasm')));
const make=async(b=blob)=>(await WebAssembly.instantiate(b,{})).instance.exports,sha=b=>createHash('sha256').update(b).digest('hex');let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
const old=await make(oldBlob);old.init(7,0,0,1,0);eq(old.skirmish_setup(500,2),1);eq(old.skirmish_visibility(2),1);
let b=save(old),l=saveLayout(b),d=l.d,at=36+4*d.getUint32(8,true),stride=d.getUint32(4,true),high=d.getUint32(32,true),map=at+stride*high;
const worker=Array.from({length:high},(_,i)=>at+i*stride).find(o=>b[o]&&b[o+4]===0&&d.getUint16(o+2,true)===8),tc0=ordinaryEntities(old).find(e=>e.team===0&&e.def===0);
// A controlled older-loader world grants a second legal-schema research host.
// Payment, research completion, timing and serialization use public new APIs.
let site;
for(let y=30;y<220&&!site;y++)for(let x=30;x<220;x++)if((x-tc0.x)**2+(y-tc0.y)**2>30**2&&Array.from({length:25},(_,i)=>(y+Math.floor(i/5))*256+x+i%5).every(t=>b[map+t]===3&&!b[map+65536+t])){site={x:x+2.5,y:y+2.5};break;}
assert.ok(site);checks++;d.setUint16(worker+2,0,true);for(const[o,v]of[[8,site.x],[12,site.y],[16,site.x],[20,site.y],[24,0],[28,0],[40,old.up15_stat(0,0,0)],[80,1],[108,0]])d.setFloat32(worker+o,v,true);
b[worker+48]=0;b[worker+104]=0;for(let j=0;j<5;j++)d.setUint32(worker+84+j*4,0,true);for(let t=0;t<2;t++)b[36+t*d.getUint32(8,true)+27]=0;d.setFloat32(l.end+20,1000,true);
b=refreshFixtureCityGround(b);eq(load(old,b),1);
const w=await make();eq(load(w,save(old)),1);w.select_point(tc0.x,tc0.y,0,0,0,0,256,256);eq(w.cmd_queue_job(57),1);eq(w.cmd_queue_job(57),1);
const state=save(w),layout=saveLayout(state),tc=ordinaryEntities(w).find(e=>e.team===0&&e.def===0&&Math.abs(e.x-tc0.x)<.1),record=36+4*96+tc.slot*176,h=((layout.d.getUint16(record+6,true)<<16)|tc.slot)>>>0;
eq(w.queue_duration_raw(h,0),-1);eq(w.queue_duration_raw(h,1),-1);const hash=w.state_hash();for(let i=0;i<5;i++){ordinaryUI(w);w.queue_duration_raw(h,0);w.render(.4,0,0,256,256,-1,0,0);}eq(w.state_hash(),hash,'reading UI never starts the clock');
w.tick();eq(w.queue_duration_raw(h,0),32.5);eq(w.queue_duration_raw(h,1),-1);let s=save(w);eq(new DataView(s.buffer).getFloat32(record+108,true),Math.fround(.01));
w.select_point(site.x,site.y,0,0,0,0,256,256);eq(ordinaryUI(w).sel.def,0);eq(w.cmd_research(3),1);
for(let i=0;i<301;i++)w.tick();eq(w.up15_stat(0,57,1),29.25,'actual research changes current buildtime');eq(w.queue_duration_raw(h,0),32.5,'active duration remains captured at start');eq(w.queue_duration_raw(h,1),-1);
w.select_point(tc0.x,tc0.y,0,0,0,0,256,256);s=save(w);const elapsed=new DataView(s.buffer).getFloat32(record+108,true),ui=ordinaryUI(w);
eq(ui.sel.qp,Number((elapsed/32.5).toFixed(2)),'HUD divides by active duration');assert.notEqual(ui.sel.qp,Number((elapsed/29.25).toFixed(2)));checks++;
const peer=await make();eq(load(peer,s),1);eq(peer.queue_duration_raw(h,0),32.5);eq(sha(save(peer)),sha(s));eq(peer.state_hash(),w.state_hash());
for(let i=0;i<400;i++){w.tick();peer.tick();eq(w.state_hash(),peer.state_hash());}
eq(w.queue_duration_raw(h,0),29.25,'next job binds the new time only after starting');eq(w.queue_price(h,0,0),30,'time research did not rewrite the already-paid invoice');
const report={format:'native-paidqueue7-duration-wasm-v2',status:'PASS',checks,wasmSha256:sha(blob),originalActiveTime:32.5,newlyResearchedTime:29.25,elapsedAtResearch:elapsed,hudProgress:ui.sel.qp,source:'Controlled old-loader second host; actual cmd_queue_job, cmd_research, tick, save/load and readonly UI/duration exports.',limits:['UnitProduce start-cache source is proven. Research/Epoch retain their separate existing time/host adapters.','No native area-effect production-rate simulation is newly claimed.']};
fs.writeFileSync(option('report',path.join(work,'paidqueue-duration-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
