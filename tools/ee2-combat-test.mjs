// Read-only source oracle against the real compiled WASM diagnostics.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
const docs=new URL('../docs/',import.meta.url),reportUrl=new URL('../../EE2-Research/analysis/native-combat-wasm-tests.json',import.meta.url);
const blob=readFileSync(process.argv[2]||new URL('openepoch-ee2.wasm',docs));
const sourceBlob=readFileSync(new URL('ee2-combat.json',docs)),data=JSON.parse(sourceBlob);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const w=(await WebAssembly.instantiate(blob,{})).instance.exports;
let checks=0;const equal=(a,b,message)=>{checks++;assert.equal(a,b,message);};
const report={format:'native-combat-wasm-tests-v1',status:'running',wasmSha256:sha(blob),combatAuditSha256:sha(sourceBlob),checks:0};
const f32=Math.fround,percentScale=f32(.01);
const adjustments=new Map(data.adjustments.map(([a,d,value])=>[`${a}:${d}`,value]));
function expected(attacker,defender){
 const base=f32(data.matrix[defender.rps][attacker.rps]*percentScale);
 const adjust=f32((adjustments.get(`${attacker.upgradeIndex}:${defender.upgradeIndex}`)??100)*percentScale);
 return [base,adjust,f32(base*adjust),f32(base*adjust)>0?1:0];
}
try{
 assert.equal(typeof w.combat_stat,'function','Current EE2 WASM combat_stat diagnostics required');
 for(const attacker of data.profiles)for(const defender of data.profiles){
  const oracle=expected(attacker,defender);
  for(let field=0;field<4;field++)equal(w.combat_stat(attacker.id,attacker.epoch,defender.id,defender.epoch,field),oracle[field],`Profiles${attacker.index}/${defender.index} field${field}`);
 }
 // All fifteen epoch slots, including unmapped unavailable units.
 for(let id=0;id<57;id++)for(let epoch=0;epoch<15;epoch++){
  const profile=data.lookup[id][epoch],value=w.combat_stat(id,epoch,id,epoch,0);
  equal(value,profile===65535?-1:expected(data.profiles[profile],data.profiles[profile])[0],`Definition${id}/epoch${epoch}`);
 }
 for(const args of [[255,0,0,0,0],[0,15,0,0,0],[0,0,255,0,0],[0,0,0,15,0],[0,0,0,0,99]])equal(w.combat_stat(...args),-1,'Invalid diagnostic input');
 // Golden source matchups catch a transpose or an extra old weapon/armor factor.
 equal(w.combat_stat(10,0,11,1,0),1.5,'Light infantry against heavy infantry');
 equal(w.combat_stat(11,1,10,0,0),1,'Reverse matchup');
 equal(w.combat_stat(26,10,10,10,0),2.5,'Heavy tank class against light infantry');
 equal(w.combat_stat(26,10,10,10,2),f32(2.5*f32(60*percentScale)),'E11 upgrade matchup multiplies class factor');
 equal(w.combat_stat(56,10,37,10,3),0,'Fortress cannot target air superiority');
 equal(w.game_tick(),0,'Clock safe before init');equal(w.ground_urban_at(0,0),0,'Urban query safe before init');
 w.init(7,1,0,1,0);equal(w.game_tick(),0,'Initial simulation tick');
 const initialX=w.start_x(),initialY=w.start_y();equal(w.ground_urban_at(initialX,initialY),1,'Completed starting city band');
 equal(w.ground_urban_at(NaN,0),0,'Nonfinite urban point');
 for(let i=0;i<20;i++)w.tick();equal(w.game_tick(),20,'Twenty real simulation ticks');
 const savedPointer=w.save(),saved=new Uint8Array(w.memory.buffer,savedPointer,w.save_len()).slice();
 for(let i=0;i<20;i++)w.tick();equal(w.game_tick(),40,'Clock advances');
 const loadPointer=w.load_alloc(saved.length);new Uint8Array(w.memory.buffer,loadPointer,saved.length).set(saved);
 equal(w.load_apply(),1,'Current save reloads');equal(w.game_tick(),20,'Clock restores saved tick');
 for(let epoch=0;epoch<15;epoch++){
  w.init(7,1,0,1,0);equal(w.skirmish_epoch(epoch),1,'Epoch setup');
  const x=w.start_x(),y=w.start_y(),radius=7+Math.floor(epoch/3),reach=radius*f32(1.4142135381698608);
  equal(w.ground_urban_at(x+reach-.0001,y),1,'Native BASE city radius inside');
  equal(w.ground_urban_at(x+reach+.0001,y),0,'Native BASE city radius outside');
 }
 report.status='passed';report.profiles=data.profiles.length;report.profilePairChecks=data.profiles.length**2;
}catch(error){report.status='failed';report.error=error.stack;throw error;}
finally{report.checks=checks;writeFileSync(reportUrl,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));}
