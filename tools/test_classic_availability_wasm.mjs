import fs from'node:fs';
import path from'node:path';
import{fileURLToPath}from'node:url';
import{createHash}from'node:crypto';
import assert from'node:assert/strict';
import{save,load}from'./roster6-state.mjs';
import{buildOrdinaryClassicFixture,ordinaryEntities,ordinaryUI}from'./ordinary-classic-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..');
const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
const binary=fs.readFileSync(option('wasm',path.join(root,'docs/openepoch-ee2-up15.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const make=async()=>(await WebAssembly.instantiate(binary,{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
const instances=[];
// Real typed command/HUD path at every start epoch; controlled legal-schema
// dock replacement avoids conflating availability with shoreline placement.
for(let epoch=0;epoch<15;epoch++){
 const w=await make();w.init(7,0,0,1,0);eq(w.skirmish_epoch(epoch),1);eq(w.skirmish_setup(500,2),1);eq(w.skirmish_visibility(2),1);
 w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);
 eq(ordinaryUI(w).act.some(a=>a.k==='t'&&a.id===57),true,'Scout menu@'+epoch);
 w.select_none();w.select_idle_worker();const u=ordinaryUI(w);
 eq(u.act.some(a=>a.k==='b'&&a.id===59),epoch>=9,'AA menu@'+epoch);
 eq(u.act.some(a=>a.k==='b'&&a.id===60),epoch>=1,'Coastal menu@'+epoch);
 let b=save(w),d=new DataView(b.buffer),stride=d.getUint32(4,true),at=36+4*d.getUint32(8,true),high=d.getUint32(32,true);
 const own=Array.from({length:high},(_,i)=>at+i*stride).find(o=>b[o]&&b[o+4]===0&&d.getUint16(o+2,true)===8);
 assert.ok(own!==undefined);checks++;
 d.setUint16(own+2,5,true);d.setFloat32(own+80,1,true);d.setFloat32(own+40,w.up15_stat(0,5,0),true);b[own+78]=0;
 eq(load(w,b),1);w.select_point(d.getFloat32(own+8,true),d.getFloat32(own+12,true),0,0,0,0,256,256);
 eq(ordinaryUI(w).sel.def,5);
 const allowed=epoch>=5&&epoch<10;
 eq(ordinaryUI(w).act.some(a=>a.k==='t'&&a.id===58),allowed,'Sail HUD@'+epoch);
 eq(w.cmd_queue_job(58),allowed?1:0,'Sail queue@'+epoch);
 instances.push({epoch,scout:true,sail:allowed,airDefense:epoch>=9,coastalDefense:epoch>=1});
}
// Public commands produce all four families in their shared legal epoch.
const w=await make(),ordinary=buildOrdinaryClassicFixture(w,{instantBuild:false});
eq(ordinary.epoch,9);for(const id of[57,58,59,60])eq(ordinary.entities.some(e=>e.def===id&&e.built>=1),true);
const stateOutput=option('save-output',null);if(stateOutput)fs.writeFileSync(stateOutput,ordinary.state);delete ordinary.state;
// Old61's later-epoch existing ships must still load and use their future row.
const oldFixture=option('legacy-save',path.join(root,'tools/fixtures/native61v1-future-sail.oes'));
const survivor=await make(),old=fs.readFileSync(oldFixture);eq(load(survivor,old),1);
eq(ordinaryEntities(survivor).some(e=>e.team===0&&e.def===58),true);
eq(survivor.up15_stat(0,58,6),170,'survivor still has future numerical row');
const canonical=save(survivor);eq(load(survivor,canonical),1);eq(sha(save(survivor)),sha(canonical));
survivor.select_none();const dock=ordinaryEntities(survivor).find(e=>e.team===0&&e.def===5);
survivor.select_point(dock.x,dock.y,0,0,0,0,256,256);eq(survivor.cmd_queue_job(58),0,'loaded future dock cannot order obsolete Sail');
const report={format:'native61v2-availability-wasm-v1',status:'PASS',checks,wasmSha256:sha(binary),instances,ordinary,
 limits:['Only the four additional production families change. Existing57 availability is retained.',
 'Surviving ships retain later model/audio/label/numeric rows.',
 'Native obsolete cancellation refunds a stored paid job price. This bounded availability fix does not introduce cancellation until paid-price sidecar/save migration is implemented; existing queued units may complete.',
 'Permanent percent arithmetic result bits are implementation regression values; native setter x87 context remains unresolved.']};
fs.writeFileSync(option('report',path.join(root,'../EE2-Research/analysis/native61v2-availability-tests.json')),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({status:'PASS',checks,wasmSha256:report.wasmSha256,ordinaryTicks:ordinary.gameTick,trace:ordinary.trace}));
