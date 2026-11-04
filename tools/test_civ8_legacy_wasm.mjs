import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save,load,persistedState}from'./roster6-state.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),workspace=path.resolve(root,'../../../..'),arg=(k,f)=>process.argv.find(s=>s.startsWith('--'+k+'='))?.slice(k.length+3)||f;
const wasm=arg('wasm',path.resolve(root,'../builds/openepoch-civ8.wasm')),blob=fs.readFileSync(wasm),sha=b=>createHash('sha256').update(b).digest('hex'),make=async b=>(await WebAssembly.instantiate(b,{})).instance.exports;
const baselines=[['OEU5-57',arg('old5',path.join(root,'tools/fixtures/native5-up15.wasm'))],['OEU6-57',arg('old57',path.join(workspace,'work/native-roster6/OpenEpoch/docs/openepoch-ee2-roster6.wasm'))],['OEU6-61',arg('old61',path.join(root,'tools/fixtures/native61v2-up15.wasm'))]];
let checks=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
const projection=b=>{const p=persistedState(b);p.head[0]=0;return sha(JSON.stringify(p));};const cases=[];
function compactDiff(a,b){const x=persistedState(a),y=persistedState(b),out=[];x.head[0]=y.head[0]=0;
 for(const key of['head','players','world']){let count=0;for(let i=0;i<Math.max(x[key].length,y[key].length);i++)if(x[key][i]!==y[key][i]){if(count<4)out.push([key,i,x[key][i],y[key][i]]);count++;}if(count)out.push([key+'Differences',count]);}
 for(let i=0;i<Math.max(x.entities.length,y.entities.length)&&out.length<20;i++){const p=x.entities[i],q=y.entities[i];if(JSON.stringify(p)!==JSON.stringify(q))for(const k of Object.keys(p||q)){if(JSON.stringify(p?.[k])!==JSON.stringify(q?.[k]))out.push(['entity',i,k,p?.[k],q?.[k]]);if(out.length>=20)break;}}
 return out;}
for(const[version,file]of baselines){const oldBlob=fs.readFileSync(file);
 for(const seed of[7,41])for(const epoch of[0,7,14]){
  const source=await make(oldBlob);source.init(seed,0,0,1,0);eq(source.skirmish_setup(500,2),1);eq(source.skirmish_epoch(epoch),1);
  source.select_point(source.start_x(),source.start_y(),0,0,0,0,256,256);
  // All three genuine baselines have Citizen8. Old57 has no Scout57.
  for(let j=0;j<3;j++)eq(source.cmd_train(8),1,'real legacy citizen queue');
  for(let t=0;t<23;t++)source.tick();
  // This is a controlled migration fixture, not an ordinary-gameplay fixture.
  // Disable only saved AI booleans and require the genuine old loader to accept
  // and canonicalize it. Otherwise NEW enemy jobs correctly switch to Paid7's
  // source.01 startup instead of historical.05 and cannot be bit-equivalent.
  const controlled=save(source),cd=new DataView(controlled.buffer);
  for(let team=0;team<4;team++)controlled[36+team*cd.getUint32(8,true)+27]=0;
  eq(load(source,controlled),1,'genuine old-loader validates controlled AI flags');
  const initial=save(source);
  const old=await make(oldBlob),current=await make(blob);eq(load(old,initial),1);eq(load(current,initial),1,'genuine '+version+' migration');
  eq(projection(save(current)),projection(save(old)),'all persisted fields after '+version+' migration');
  for(let t=0;t<4;t++)eq(current.player_civilization(t),14,'unknown historical civ stays neutral');
  for(let tick=0;tick<640;tick++){old.tick();current.tick();if(tick%20===0){const now=save(current),prior=save(old);if(projection(now)!==projection(prior))console.log(JSON.stringify({version,seed,epoch,tick,differences:compactDiff(now,prior)}));eq(projection(now),projection(prior),'full IEEE/world continuation '+version+' '+tick);}}
  const currentSave=save(current),peer=await make(blob);eq(load(peer,currentSave),1,'migrated modern reload');eq(sha(save(peer)),sha(currentSave),'migrated exact modern save');eq(peer.state_hash(),current.state_hash(),'modern migrated peerhash');
  cases.push({version,seed,epoch,ticks:640,sourceWasmSha256:sha(oldBlob),sourceSaveSha256:sha(initial),modernSaveSha256:sha(currentSave)});
 }
}
const report={format:'native-civ8-genuine5-6-migration-v1',status:'PASS',checks,wasmSha256:sha(blob),cases,activeWrites:false,originalExecuted:false,
 controlledScenarioEdits:[{field:'saved Player.ai',value:false,validation:'Genuine historical loader accepts and canonicalizes before either migration/control run.'}],
 limits:['Historical civilization and prior paid prices/durations are not recoverable; legacyneutral14 and existing inferred-ledger policy are explicit.','Cross-format hash equality is not asserted; complete field/IEEE-bit/world projections and subsequent modern peers are compared.','Newly paid tasks after migration use the new source.01 startup, so arbitrary ongoing old5/6 AI traces are not promised byte-equivalent after fresh task creation.']};
fs.writeFileSync(arg('report',path.resolve(root,'../civ8-legacy-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:'PASS',checks,cases:cases.length,wasmSha256:report.wasmSha256}));
