import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createTypedAudioBindings} from '../docs/original-audio-bindings.js';
const read=async name=>fs.readFile(new URL(name,import.meta.url));
const sha=b=>createHash('sha256').update(b).digest('hex');
const modelBytes=await read('../docs/local-original/models/manifest-civ8.json'),audioBytes=await read('../docs/local-original/audio/native-families-civ8.json'),wasmBytes=await read('../docs/openepoch-ee2-up15.wasm');
const oracleBytes=await read('./fixtures/civ8-audio-source-oracle.json');
const models=JSON.parse(modelBytes),catalog=JSON.parse(audioBytes),oracle=JSON.parse(oracleBytes);
const {instance:{exports:wasm}}=await WebAssembly.instantiate(wasmBytes,{});
let checks=0;const eq=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;};
const typed=createTypedAudioBindings({wasm,bindings:models.models,familyManifest:catalog});
eq(wasm.definition_count(),80);
for(const row of oracle.rows){
 const plan=typed.resolve(row.id,row.action,row.epoch);
 eq(plan?.records.map(r=>r.sourceEvent)||[],row.events,`source CSV own-array carry and live parent lookup ${row.id}/${row.epoch}/${row.action}`);
 if(row.events.length){
  eq(plan.resolvedSoundFamily||plan.sourceFamily,row.resolvedFamily);
  for(const recording of plan.records)eq(sha(await read('../docs/local-original/audio/'+recording.file)),recording.sha256,'exact original recording bytes');
 }
}
eq(typed.errors,[]);
eq(typed.resolve(61,'select',6).records[0].sourceEvent,'v_spy_13_sel01');
eq(typed.resolve(61,'select',14).records[0].sourceEvent,'v_spy_04_sel01','surviving obsolete American follows later parent');
eq(typed.resolve(76,'select',5).records[0].sourceEvent,'v_LightArtillery1_05_sel01');
eq(typed.resolve(76,'select',6).records[0].sourceEvent,'v_LightArtillery1_07_sel01');
eq(typed.resolve(77,'select',14).records[0].sourceEvent,'v_LightArtillery1_15_sel01');
// A cache query follows the parent object on every call, not an import-time copy.
const live=structuredClone(catalog),resolver=createTypedAudioBindings({wasm,bindings:models.models,familyManifest:live});
eq(resolver.resolve(61,'select',6).records[0].sourceEvent,'v_spy_13_sel01');
live.families.Scout.intervals.find(x=>x.epoch===6).effectiveEvents.select=['v_spy_04_sel01'];
eq(resolver.resolve(61,'select',6).records[0].sourceEvent,'v_spy_04_sel01');
const local=live.families.UniqueUnitAmerican1.intervals[0];local.effectiveEvents.select=['v_spy_07_sel01'];
eq(resolver.resolve(61,'select',6).records[0].sourceEvent,'v_spy_07_sel01','a nonempty local cache overrides parent');
for(const mutate of [m=>m.families.UniqueUnitAmerican1.parentSoundFamily='missing',m=>m.families.Scout.parentSoundFamily='UniqueUnitAmerican1',m=>m.families.UniqueUnitAmerican1.parentSoundFamily=3]){
 const bad=structuredClone(catalog);mutate(bad);assert.throws(()=>createTypedAudioBindings({wasm,bindings:models.models,familyManifest:bad}));checks++;
}
eq(typed.resolve(80,'select',0),null,'parent-only source cache never creates runtime unit80');
const used=new Set(typed.usedFiles());for(const row of oracle.rows)if(row.events.length)for(const event of row.events)eq(used.has(catalog.events[event].file),true,'all inherited recordings are warmed');
const report={format:'native-civ8-independent-live-parent-audio-bindings-v1',passed:true,checks,sourceOracleRows:oracle.rows.length,
 sourceHashes:{models:sha(modelBytes),audio:sha(audioBytes),wasm:sha(wasmBytes),independentSourceOracle:sha(oracleBytes),resolver:sha(await read('../docs/original-audio-bindings.js'))},
 limits:['Actual80-definition WASM roles and independent active-source CSV/parent oracle; playback transport/device and native RNG/scheduling remain separate.']};
await fs.writeFile(new URL('../../audio-binding-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
