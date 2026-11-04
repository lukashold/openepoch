import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createTypedAudioBindings} from '../docs/original-audio-bindings.js';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const modelFile=new URL('../docs/local-original/models/manifest-roster6.json',import.meta.url);
const wasmFile=new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url);
const modelBytes=await fs.readFile(modelFile),catalogBytes=await fs.readFile(new URL('../docs/local-original/audio/native-families.json',import.meta.url)),wasmBytes=await fs.readFile(wasmFile);
const {instance:{exports:wasm}}=await WebAssembly.instantiate(wasmBytes,{}),models=JSON.parse(modelBytes),catalog=JSON.parse(catalogBytes);
let checks=0;const eq=(a,b,note)=>{assert.deepEqual(a,b,note);checks++;},good=(value,note)=>{assert.ok(value,note);checks++;};
eq([6,7].includes(wasm.schema_revision()),true,'Real supported wide presentation ABI');eq(wasm.definition_count(),61,'Active simulation contains the four new definitions');
const typed=createTypedAudioBindings({wasm,bindings:models.models,familyManifest:catalog}),rows=[];
eq(typed.status().definitions,4,'Actual renderer manifest contains exactly four new typed source-family definitions');
for(const [id,family,flags]of [[57,'Scout',18],[58,'FightingSail',18],[59,'AirDefense',17],[60,'CoastalDefense',17]]){
 eq(wasm.definition_flags(id),flags,'Actual static role flags bind voice versus building effect');
 const bindings=models.models.filter(row=>row.assetKind===0&&row.id===id);good(bindings.length>0,'Source family has actual imported model transitions');
 for(const binding of bindings)eq(binding.sourceFamily,family,'Renderer and audio agree on the source family');
 for(let epoch=0;epoch<15;epoch++)for(const action of ['select','order','attack','build']){
  const interval=catalog.families[family].intervals.find(row=>row.epoch<=epoch&&epoch<row.until),plan=typed.resolve(id,action,epoch);
  const allowed=!!(interval?.effectiveEvents[action]?.length&&(id<59||action==='select'));
  eq(!!plan,allowed,'Actual model/source-visual binding matches the audio interval and native action channel');
  if(plan){eq(plan.sourceVisual,interval.sourceVisual,'Exact source visual joins model and audio records');good(plan.records.every(record=>catalog.assets[record.file]?.sha256),'Every joined recording has an exact media hash');}
 }
 rows.push({id,family,bindings:bindings.length,flags});
}
eq(typed.errors,[],'No visual/role mismatch is hidden');
const report={format:'native-audio6-actual-renderer-bindings-v1',status:'passed',checks,rows,
 sourceHashes:{models:sha(modelBytes),familyAudio:sha(catalogBytes),frozen61Wasm:sha(wasmBytes)},
 scope:'Independent join of active61-definition WASM static roles, actual renderer manifest and imported source-family audio data. No audio graph/device, browser or original executable is launched.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native61-audio-model-bindings.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
