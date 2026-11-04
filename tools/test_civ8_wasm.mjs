// Actual isolatedWASM8 against the frozen publicAPI7 baseline. No memory writes.
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';
import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load}from'./roster6-state.mjs';import{saveLayout,assertCurrentSave}from'./save-fixture.mjs';
import{ordinaryEntities,ordinaryUI}from'./ordinary-classic-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),workspace=path.resolve(root,'../../../..');
const arg=(key,fallback)=>process.argv.find(s=>s.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
const wasm=arg('wasm',path.resolve(root,'../builds/openepoch-civ8.wasm'));
const old7=arg('old7',path.join(workspace,'work/native-joint7/builds/openepoch-joint7.wasm'));
const report=arg('report',path.resolve(root,'../civ8-wasm-report.json'));
const blob=fs.readFileSync(wasm),oldBlob=fs.readFileSync(old7);
const sha=b=>createHash('sha256').update(b).digest('hex');
const make=async(b=blob)=>(await WebAssembly.instantiate(b,{})).instance.exports;
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
const equalBytes=(a,b,m)=>{eq(a.length,b.length,m+' length');eq(sha(a),sha(b),m+' bytes');};
function comparable(b){
 const l=assertCurrentSave(b),out=b.slice(0,l.payloadEnd),d=new DataView(out.buffer),stride=d.getUint32(4,true),start=36+4*d.getUint32(8,true);
 d.setUint32(0,0,true);d.setUint32(12,0,true); // different public envelope/count only.
 for(let i=0;i<d.getUint32(32,true);i++)d.setUint16(start+i*stride+76,65535,true); // derived flow cache.
 return out;
}
const metadata=await make();eq(metadata.definition_count(),80);eq(metadata.schema_revision(),8);eq(metadata.civilization_count(),14);
for(let c=0;c<14;c++)eq(metadata.civilization_region(c),c<5?0:c<8?1:c<11?2:3);
eq(metadata.civilization_region(14)>>>0,0xffffffff);
const queued=await make();queued.init(7,0,0,1,0);queued.select_point(queued.start_x(),queued.start_y(),0,0,0,0,256,256);eq(queued.cmd_queue_job(57),1);
const queuedSave=save(queued),queuedHash=queued.state_hash();eq(queued.configure_civilizations(0xeee0),0,'paid order locks setup');eq(queued.state_hash(),queuedHash,'rejected setup stays atomic');equalBytes(save(queued),queuedSave,'rejected setup preserves payment and world');
const migrations=[];
for(const seed of[7,41])for(const epoch of[0,7,14]){
 const source=await make(oldBlob);source.init(seed,0,0,1,0);eq(source.skirmish_setup(500,2),1);eq(source.skirmish_epoch(epoch),1);
 source.select_point(source.start_x(),source.start_y(),0,0,0,0,256,256);
 for(let j=0;j<3;j++)eq(source.cmd_queue_job(57),1,'real7 paid scout queue');
 for(let t=0;t<31;t++)source.tick();
 const old=await make(oldBlob),w=await make();const initial=save(source);
 eq(load(old,initial),1,'canonical old7 load');eq(load(w,initial),1,'old7->8 load');
 for(let c=0;c<4;c++)eq(w.player_civilization(c),14,'legacyneutral civilization');
 eq(w.configure_civilizations(0xeee0),0,'loaded old game setup is closed');
 eq(w.state_hash(),old.state_hash(),'migrated legacy hash');
 equalBytes(comparable(save(w)),comparable(save(old)),'complete persisted7->8 projection');
 for(let tick=0;tick<640;tick++){
  old.tick();w.tick();eq(w.state_hash(),old.state_hash(),'neutral continuation '+tick);
  if(tick%80===0)equalBytes(comparable(save(w)),comparable(save(old)),'continued world/paid/attack/city state');
 }
 const current=save(w),l=assertCurrentSave(current);eq(l.civ8,true);eq(current.length,save(old).length+36,'explicit CIV1 extension length');
 const peer=await make();eq(load(peer,current),1,'modern8 load');equalBytes(save(peer),current,'modern8 canonical roundtrip');
 migrations.push({seed,epoch,ticks:640,old7sha256:sha(initial),modern8sha256:sha(current)});
}
const civ=await make();civ.init(7,0,0,1,0);eq(civ.configure_civilizations(0xeee3),1,'American newgame');
eq(civ.configure_civilizations(0xeee0),0,'civilization cannot change after configuration');
eq(civ.definition_ttciv(61),4);eq(civ.definition_ttciv(64),11);eq(civ.definition_ttciv(79),10);
const good=save(civ),l=assertCurrentSave(good),baseHash=civ.state_hash();
let malformed=0;
for(const offset of[0,12,l.civ,l.civ+4,l.civ+8,l.civ+12,l.civ+16,l.civ+20,l.civ+24,l.civ+28,l.civ+32,l.descriptor,l.descriptor+4,l.descriptor+8,l.descriptor+12,good.length-8,good.length-4]){
 const bad=good.slice();new DataView(bad.buffer).setUint32(offset,0xffffffff,true);
 eq(load(civ,bad),0,'malformed civil envelope '+offset);eq(civ.state_hash(),baseHash,'atomic malformed load');equalBytes(save(civ),good,'atomic saved bytes');malformed++;
}
for(let n=1;n<=88;n++){
 eq(load(civ,good.slice(0,good.length-n)),0,'strict wrapper truncation '+n);eq(civ.state_hash(),baseHash,'atomic truncation');malformed++;
}
const outcomes={format:'native-civ8-actual-wasm-progression-v1',status:'PASS',checks,wasmSha256:sha(blob),old7Sha256:sha(oldBlob),migrations,malformedAtomicCases:malformed,
 activeWrites:false,originalExecuted:false,limits:['No19-family ordinary combat/graphics readiness claim; pack/unpack and other ability closures remain separate.','This validates persisted own simulation and migration, not a temporal original-FPU bit guarantee.']};
fs.writeFileSync(report,JSON.stringify(outcomes,null,2)+'\n');console.log(JSON.stringify({status:outcomes.status,checks,wasmSha256:outcomes.wasmSha256,malformed,migrations:migrations.length}));
