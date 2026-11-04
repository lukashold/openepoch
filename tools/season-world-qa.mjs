// End-to-end rendering proof from actual WASM ticks and production render calls.
// Python loads EGL/GLES libraries directly; no browser or app is launched.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';

const game=fileURLToPath(new URL('../',import.meta.url)),workspace=path.resolve(game,'../..');
const research=path.resolve(game,'../EE2-Research/analysis');
const python=process.env.OPENEPOCH_PYTHON||'C:\\Python314\\python.exe';
const sha=value=>createHash('sha256').update(value).digest('hex');
const wasmFile=path.join(game,'docs/openepoch-ee2.wasm'),wasmBytes=await fs.readFile(wasmFile),wasmSha256=sha(wasmBytes);
const exported=new Set(WebAssembly.Module.exports(new WebAssembly.Module(wasmBytes)).map(row=>row.name));
for(const name of ['ground_urban_at','game_tick'])assert.ok(exported.has(name),`Rebuild current WASM with ${name}`);
const groundManifest=JSON.parse(await fs.readFile(path.join(game,'docs/local-original/ground-decals.json'),'utf8'));
const entries=new Map(groundManifest.entries.map(row=>[row.id,row]));
const cases=[
 {name:'summer',epoch:0,scene:'town',month:6},
 {name:'winter',epoch:0,scene:'town',month:0},
 {name:'halfblend',epoch:0,scene:'town',month:2.5},
 {name:'future',epoch:14,scene:'town',month:6},
];
const run=(command,args)=>{
 const result=spawnSync(command,args,{cwd:workspace,encoding:'utf8',windowsHide:true,maxBuffer:16*1024*1024});
 if(result.error)throw result.error;
 if(result.status!==0)throw Error(`${path.basename(command)} exited${result.status}: ${result.stderr}\n${result.stdout}`);
 return JSON.parse(result.stdout.trim());
};
const relativeTexture=filename=>path.relative(path.join(game,'docs'),filename).split(path.sep).join('/');
const pairKey=(first,next,blend)=>JSON.stringify([first,next,Math.round(blend*1e10)/1e10]);
const add=(map,key,count=1)=>map.set(key,(map.get(key)||0)+count);
const sorted=map=>[...map].sort(([a],[b])=>a.localeCompare(b));
let sourceFiles,checks=0;
const check=(condition,message)=>{assert.ok(condition,message);checks++;};
const results=[];
for(const fixture of cases){
 const captured=run(process.execPath,[path.join(game,'tools/headless-world-capture.mjs'),String(fixture.epoch),fixture.scene,String(fixture.month)]);
 const input=captured.output,payload=JSON.parse(await fs.readFile(input,'utf8'));
 check(payload.sourceHashes.wasmSha256===wasmSha256,'Every scene uses the same current WASM');
 check(payload.season.tickSource==='WASM game_tick','Calendar reads the actual exported tick');
 check(Math.abs(payload.season.monthPosition-fixture.month)<1e-10,'Requested phase comes from actual stepped simulation time');
 check(payload.season.actualSimulationTick===(fixture.month%1)*2000,'Half-month fixture steps the real five-minute-per-season clock');
 check(payload.ground.urbanSource.startsWith('WASM ground_urban_at'),'Urban membership comes from the actual simulation');
 check(payload.ground.urbanQueries.length>0,'Actual buildings exercise the ground pass');
 if(sourceFiles)assert.deepEqual(payload.sourceHashes.files,sourceFiles,'No production sources changed during the scene suite');
 else sourceFiles=payload.sourceHashes.files;
 const expected=new Map(),actual=new Map(),slots=new Set();
 for(const query of payload.ground.urbanQueries){
  const entry=entries.get(query.id),owned=payload.epochs[query.team]!==undefined;
  const nativeEpoch=(payload.epochs[query.team]??0)+1;
  const slot=!owned?0:nativeEpoch>=11?2:nativeEpoch>=6?1:0;slots.add(slot);
  const sprite=entry[query.urban?'urban':'rural'][slot];
  check(!!sprite,'Source epoch/urban slot exists for an actual building');
  const urls=sprite.seasonalUrls?.length?sprite.seasonalUrls:[sprite.url];
  const phase=(payload.season.month0+payload.season.blend)/12;
  const position=phase*urls.length,index=Math.floor(position)%urls.length,mix=position-Math.floor(position);
  add(expected,pairKey(urls[index],urls[(index+1)%urls.length],mix));
 }
 const terrainDraws=[];
 for(const draw of payload.draws){
  if(draw.uniforms.uSeasonBlend){
   terrainDraws.push(draw);
   assert.deepEqual(draw.uniforms.uSeasonMonths.value,[payload.season.month0,payload.season.month1]);checks++;
   check(draw.uniforms.uSeasonBlend.value===payload.season.blend,'Terrain receives the calendar interpolation fraction');
  }
  if(draw.uniforms.uSeasonMix){
   const first=payload.textures[draw.textures['0:3553']],next=payload.textures[draw.textures['7:3553']];
   add(actual,pairKey(relativeTexture(first.image),relativeTexture(next.image),draw.uniforms.uSeasonMix.value),draw.instances);
   check(draw.state.depthWrite===false,'Ground overlays preserve scene depth');
  }
 }
 check(terrainDraws.length===1,'One production terrain pass');
 assert.deepEqual(sorted(actual),sorted(expected),'Ground draw textures and interpolation match independently selected source epoch/urban/month bindings');checks++;
 check(payload.ground.metrics.instances===payload.ground.urbanQueries.length,'All eligible ground instances reach the GPU draw');
 check(slots.has(fixture.epoch>=10?2:0),'World proof exercises the required native epoch bracket');
 const output=path.join(game,`native-season-${fixture.name}.png`),reportFile=path.join(research,`native-season-world-${fixture.name}.json`);
 const gpu=run(python,[path.join(game,'tools/replay_world_gles.py'),input,'--output',output,'--report',reportFile]);
 check(gpu.passed&&gpu.uiLaunched===false,'Actual GPU replay passes without UI');
 check(gpu.sourceHashes.wasmSha256===wasmSha256,'GPU report carries the production WASM hash');
 check(gpu.compiledPrograms.every(program=>program.compiled&&program.linked),'All production world shaders compile and link');
 results.push({name:fixture.name,epoch:fixture.epoch+1,season:payload.season,ground:{...payload.ground.metrics,urbanQueries:payload.ground.urbanQueries,epochSlots:[...slots]},
  capture:path.relative(workspace,input),captureSha256:gpu.captureSha256,png:path.relative(game,output),pngSha256:gpu.pngSha256,
  replayReport:path.relative(research,reportFile),drawCalls:gpu.stats.drawCalls,triangles:gpu.stats.triangles,shaderHashes:payload.sourceHashes.shaders});
 console.log(JSON.stringify({case:fixture.name,passed:true,tick:payload.season.actualSimulationTick,phase:payload.season.monthPosition,groundInstances:payload.ground.metrics.instances,draws:gpu.stats.drawCalls}));
}
check(results[0].pngSha256!==results[1].pngSha256,'Identical initial worlds render different summer and winter pixels');
check(results[2].pngSha256!==results[0].pngSha256&&results[2].pngSha256!==results[1].pngSha256,'Half-month world renders its own continuous seasonal state');
check(sha(await fs.readFile(wasmFile))===wasmSha256,'WASM did not change during the suite');
const report={format:'openepoch-native-season-world-qa-v1',passed:true,checks,wasmSha256,sourceFiles,scenes:results,
 scope:'Actual WASM worlds, simulation-tick calendar, terrain/ground monthly interpolation and epoch/urban source bindings through production JS loaders and ANGLE GLES draw replay. No GUI, browser or original executable launched.',
 limitations:['World-only rendering proof; HUD and browser interaction are outside this check.','Ground city membership follows the source-backed completed Town Center query; complete native CityManager registration/priority behavior remains separate.','Vegetation, building skins and weather events are not automatically made seasonal by terrain/ground interpolation.']};
const destination=path.join(research,'native-season-world-qa.json');await fs.writeFile(destination,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({report:destination,passed:true,checks,scenes:results.length,wasmSha256}));
