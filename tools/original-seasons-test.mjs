import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {MINUTES_PER_SEASON,DEFAULT_SEASON_SETTINGS,normalizeSeasonSettings,seasonAtTick,createSeasonClock} from '../docs/original-seasons.js';

let checks=0;
const check=(condition,message)=>{assert.ok(condition,message);checks++;};
const equal=(a,b,message)=>{assert.deepEqual(a,b,message);checks++;};
const defaults=createSeasonClock();
equal(defaults.atTick(0).month0,6,'Application default starts at the existing summer slot');
equal(defaults.atTick(1000).blend,.5,'Half of a100-second month');
equal(defaults.atTick(2000).month0,7,'One complete monthly interval');
equal(defaults.atTick(12000).month0,0,'Calendar crosses the end of the texture list');
equal(defaults.atTick(24000).month0,6,'Full20-minute cycle returns to its start');
equal(defaults.atTick(0).secondsPerYear,1200,'Four selected five-minute seasons per cycle');
for(const minutesPerSeason of MINUTES_PER_SEASON)for(let startMonth=0;startMonth<12;startMonth++){
 const options={minutesPerSeason,startMonth,cycle:true},clock=createSeasonClock(options),period=clock.atTick(0).ticksPerYear;
 for(let i=0;i<90;i++){
  const tick=(i*7919+startMonth*17)%period,a=clock.atTick(tick),b=clock.atTick(tick+period);
  check(a.month0>=0&&a.month0<12&&a.month1===(a.month0+1)%12,'Adjacent monthly indices');
  check(a.blend>=0&&a.blend<1,'Valid interpolation fraction');
  equal(a,b,'Exact periodicity without accumulating frame time');
  equal(a,seasonAtTick(tick,JSON.parse(JSON.stringify(options))),'Serialized host/save settings give the same phase');
 }
 const fixed=createSeasonClock({...options,cycle:false});
 equal(fixed.atTick(0),fixed.atTick(Number.MAX_SAFE_INTEGER),'Disabled cycle is independent of play time');
}
for(const tick of [-1,.1,NaN,Infinity,Number.MAX_SAFE_INTEGER+1]){assert.throws(()=>seasonAtTick(tick),RangeError);checks++;}
for(const options of [{cycle:'false'},{startMonth:-1},{startMonth:12},{startMonth:.5},{minutesPerSeason:0},{minutesPerSeason:6}]){assert.throws(()=>normalizeSeasonSettings(options));checks++;}
const normalized=normalizeSeasonSettings({startMonth:2,minutesPerSeason:1,cycle:true});
check(Object.isFrozen(normalized),'Settings remain stable after game creation');
check(Object.isFrozen(DEFAULT_SEASON_SETTINGS),'Application defaults cannot be mutated');
const before=seasonAtTick(399,normalized),after=seasonAtTick(400,normalized);
check(before.month0===2&&before.month1===3&&before.blend>.99,'Last tick before a monthly boundary');
check(after.month0===3&&after.month1===4&&after.blend===0,'First tick after a monthly boundary');

// Actual WASM saves and independently stepped peers expose a shared integer tick.
const wasmBytes=await fs.readFile(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const sha=value=>createHash('sha256').update(value).digest('hex'),wasmSha256=sha(wasmBytes);
const engine=async()=>{const wasm=(await WebAssembly.instantiate(wasmBytes,{})).instance.exports;wasm.init(7,1,0,1,0);return wasm;};
const a=await engine(),b=await engine();
const tick=wasm=>{const ptr=wasm.ui(),len=wasm.ui_len();return JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,ptr,len))).tick;};
const settings={cycle:true,startMonth:2,minutesPerSeason:1};
const hostClock=createSeasonClock(settings),peerClock=createSeasonClock(JSON.parse(JSON.stringify(settings)));
for(let i=0;i<700;i++){
 a.tick();b.tick();
 if(i%7===0)hostClock.atTick(tick(a)); // Deliberately different render frequency.
 if(i%97===0)equal(hostClock.atTick(tick(a)),peerClock.atTick(tick(b)),'Independent peers agree at the same actual simulation tick');
}
const saveTick=tick(a),ptr=a.save(),saveBytes=new Uint8Array(a.memory.buffer,ptr,a.save_len()).slice();
const record=JSON.parse(JSON.stringify({settings:hostClock.settings,tick:saveTick}));
const expected=hostClock.atTick(saveTick);
for(let i=0;i<333;i++)a.tick();
const destination=a.load_alloc(saveBytes.length);new Uint8Array(a.memory.buffer,destination,saveBytes.length).set(saveBytes);
check(a.load_apply()===1,'Actual WASM save reload succeeds');
equal(tick(a),record.tick,'Simulation tick is restored');
equal(createSeasonClock(record.settings).atTick(tick(a)),expected,'Season phase resumes exactly after actual save/load');
assert.equal(sha(await fs.readFile(new URL('../docs/openepoch-ee2.wasm',import.meta.url))),wasmSha256,'WASM stays fixed throughout the clock test');
const report={format:'openepoch-native-season-clock-tests-v1',passed:true,checks,wasmSha256,
 sourceSha256:{'docs/original-seasons.js':sha(await fs.readFile(new URL('../docs/original-seasons.js',import.meta.url)))},
 nativeDurationOptions:MINUTES_PER_SEASON,applicationDefaults:DEFAULT_SEASON_SETTINGS,
 realWasmSaveTick:saveTick,actualPeerTickChecks:8,saveLoadPhaseStable:true,frameFrequencyIndependent:true,
 scope:'Render-only phase from actual deterministic20Hz simulation ticks and serialized explicit settings; no weather RNG or gameplay rules are changed.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native-season-clock-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
