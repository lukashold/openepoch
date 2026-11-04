// Genuine existingresearch completion drives the two-stage German cost check.
// No synthetic Capture/Convert/Wonder event and no saved-state/memory editing.
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load}from'./roster6-state.mjs';import{ordinaryEntities,ordinaryUI}from'./ordinary-classic-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),arg=(k,f)=>process.argv.find(s=>s.startsWith('--'+k+'='))?.slice(k.length+3)||f,blob=fs.readFileSync(arg('wasm',path.resolve(root,'../builds/openepoch-civ8.wasm'))),sha=b=>createHash('sha256').update(b).digest('hex'),make=async()=>(await WebAssembly.instantiate(blob,{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
function mill(w){
 w.select_none();assert.notEqual(w.select_idle_worker()>>>0,0xffffffff);checks++;
 const builder=ordinaryEntities(w).find(e=>e.team===0&&e.selected&&e.def===8),side=w.footprint_size(48,0),sites=[];
 for(let y=4;y<252-side;y++)for(let x=4;x<252-side;x++)sites.push({x,y,d:(x+side/2-builder.x)**2+(y+side/2-builder.y)**2});
 sites.sort((a,b)=>a.d-b.d||a.y-b.y||a.x-b.x);const site=sites.find(s=>w.place_error_rotated(48,s.x,s.y,0)===0);assert.ok(site);checks++;
 eq(w.cmd_place_rotated(48,site.x,site.y,0),1,'ordinary researchproducer placement');
 let producer;for(let t=0;t<5000&&!producer;t++){w.tick();producer=ordinaryEntities(w).find(e=>e.team===0&&e.def===48&&e.built>=1);}
 assert.ok(producer);checks++;let ui;
 for(const dx of[0,-side*.35,side*.35]){for(const dy of[0,-side*.35,side*.35]){w.select_point(producer.x+dx,producer.y+dy,0,0,0,0,256,256);ui=ordinaryUI(w);if(ui.sel?.def===48)break;}if(ui?.sel?.def===48)break;}
 eq(ui.sel?.def,48,'real source researchhost selection');return producer;
}
const cases=[];
for(const[civ,expectedBefore,expectedAfter]of[[0,41,38],[3,55,50]]){
 const w=await make();w.init(7,0,0,1,0);eq(w.configure_civilizations(0xeee0+civ),1);eq(w.skirmish_epoch(5),1);eq(w.skirmish_setup(500,2),1);eq(w.skirmish_visibility(2),1);eq(w.cheat(0),1);eq(w.cheat(3),1);
 const producer=mill(w);eq(w.cheat(3),2,'normal research clock');eq(w.unit_cost_stat(0,22,2),expectedBefore,'installed55Iron before research');
 eq(w.cheat(4),1,'authorized tech points');eq(w.research_stat(0,0,0),50);eq(w.cmd_queue_job(0x10000+26),1,'actual Economic3_3 research');
 eq(w.research_stat(0,0,0),42,'eight techpoints unchanged by German basic/special');
 const b=save(w),d=new DataView(b.buffer),ent=36+4*d.getUint32(8,true)+d.getUint32(4,true)*producer.slot,handle=((d.getUint16(ent+6,true)<<16)|producer.slot)>>>0;
 eq(w.queue_price(handle,0,9),8,'captured researchtechprice');eq(w.queue_price(handle,0,10),0,'research population untouched');
 let ticks=0;while(!w.research_stat(0,2,26)&&ticks<1000){w.tick();ticks++;}eq(w.research_stat(0,2,26),1,'genuine completion');eq(w.unit_cost_stat(0,22,2),expectedAfter,'sequential normal then German price');
 const state=save(w),peer=await make();eq(load(peer,state),1);eq(peer.unit_cost_stat(0,22,2),expectedAfter);eq(sha(save(peer)),sha(state));
 cases.push({civ,epoch:5,sourceDef:22,coreResourceSlot:2,nativeResourceSlot:5,rawCost:55,sourceResearch:'Economic3_3',researchIndex:26,ticks,before:expectedBefore,after:expectedAfter,context:'Own declared binary53/nearest source-form model, not a universal originalRC claim.'});
}
const report={format:'native-civ8-actual-research-german-billing-v1',status:'PASS',checks,wasmSha256:sha(blob),cases,activeWrites:false,originalExecuted:false};
fs.writeFileSync(arg('report',path.resolve(root,'../civ8-billing-wasm-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
