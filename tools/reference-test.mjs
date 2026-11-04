import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const base = (await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch.wasm', import.meta.url)), {})).instance.exports;
const ref = (await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch-ee2.wasm', import.meta.url)), {})).instance.exports;
const data = JSON.parse(readFileSync(new URL('../docs/ee2-reference.json', import.meta.url)));
const originalUnits = JSON.parse(readFileSync(new URL('../docs/ee2-units.json', import.meta.url)));
assert.equal(base.ruleset(),0);assert.equal(ref.ruleset(),1);
const ids=[32,33,34,36,37,38,39,40];
for(let i=0;i<ids.length;i++){
  const id=ids[i], p=data.profile[i], r=data.rows.find(r=>r.UNIT===p.ee2&&r.EPOCH===p.epoch);
  // unit_stat is a static definition diagnostic, even before init(). Its native
  // scalar baseline follows the earliest strictly matched original visual; the
  // live class progression is checked separately by the unit/economy tests.
  const original=originalUnits.rows.filter(row=>row.id===id).sort((a,b)=>a.epoch-b.epoch)[0];
  assert.ok(original,`${p.openepoch}: native visual record`);
  assert.equal(original.identity_verified,true);assert.equal(original.model_verified,true);
  assert.equal(original.source_unit,p.ee2);
  for(const key of ['hp','sight','buildtime']){
    const field=original.fields[key];assert.equal(field.verified,true,`${p.openepoch}: verified ${key}`);
    assert.ok(Number.isFinite(field.value)&&field.sources.length>0,`${p.openepoch}: source evidence for ${key}`);
  }
  // Speed and weapon distances/timing keep the separately verified legacy
  // adapter values; importing a different world-unit scale is not implied.
  for(const [field,value] of [original.fields.hp.value,original.fields.sight.value,p.stats.speed,original.fields.buildtime.value,r.DAMAGE,r.RANGE,r.RELOAD,0].entries())
    assert.ok(Math.abs(ref.unit_stat(id,field)-value)<1e-4,`${p.openepoch} field ${field}`);
  assert.ok(Math.abs(ref.unit_stat(id,8)-p.movement.angSpeed*Math.PI/180)<1e-5,`${p.openepoch} angular speed`);
}
assert.equal(base.unit_stat(36,7),400);assert.equal(ref.unit_stat(36,7),0);
assert.equal(data.combat.Destroyer.Submarine,220);
assert.equal(data.combat.Submarine.Destroyer,165); // rows defend; columns attack
assert.equal(new Set(data.rows.map(r=>r.EPOCH)).size,15);
const load=async()=> (await WebAssembly.instantiate(readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url)),{})).instance.exports;
const a=await load(),b=await load();a.init(42,1,1000,1,0);b.init(42,1,1000,1,0);
for(let i=0;i<1000;i++){a.tick();b.tick();}
assert.equal(a.state_hash(),b.state_hash());
const ptr=a.save(), bytes=new Uint8Array(a.memory.buffer,ptr,a.save_len()).slice(), hash=a.state_hash();
a.tick();const dst=a.load_alloc(bytes.length);new Uint8Array(a.memory.buffer,dst,bytes.length).set(bytes);
assert.equal(a.load_apply(),1);assert.equal(a.state_hash(),hash);
base.init(42,1,0,1,0);const bp=base.load_alloc(bytes.length);new Uint8Array(base.memory.buffer,bp,bytes.length).set(bytes);
assert.equal(base.load_apply(),0,'base game must reject a reference-profile save');
console.log('PASS: 8 source-backed static visual baselines and legacy movement/weapon adapters, Ghidra degree/radian conversion, nuclear-sub secondary removal, 15 epochs, matrix orientation, separate rulesets/save rejection, deterministic 2000-unit simulation and save roundtrip');
