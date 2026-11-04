// Actual production WASM, native-source arithmetic oracle and malformed loads.
// Does not start the installed game or mutate original files.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {assertCurrentSave,saveLayout,refreshFixtureCityGround,resetFixtureClaims,revealFixture} from './save-fixture.mjs';
const root=new URL('../',import.meta.url),blob=fs.readFileSync(new URL('docs/openepoch-ee2-up15.wasm',root));
const baseline=process.argv[2]?new URL(process.argv[2].replaceAll('\\','/')+'/',`file:///${process.cwd().replaceAll('\\','/')}/`):
 [new URL('../../baseline/',import.meta.url),new URL('../../../work/native-combat-abilities/baseline/',import.meta.url)].find(p=>fs.existsSync(new URL('openepoch.wasm',p)));
assert.ok(baseline,'Frozen native4 baseline directory required');
const sha=b=>createHash('sha256').update(b).digest('hex'),f=Math.fround;
const make=async()=> (await WebAssembly.instantiate(blob,{})).instance.exports;
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
let checks=0;const eq=(a,b,m)=>{assert.deepEqual(a,b,m);checks++;};
const attack=JSON.parse(fs.readFileSync(new URL('docs/native-up15-attack.json',root)));
function fixture(w,id=9,epoch=0){
 w.init(7,0,0,2,0);w.skirmish_epoch(epoch);let b=save(w),l=assertCurrentSave(b),d=l.d;
 const stride=d.getUint32(4,true),players=d.getUint32(8,true),high=d.getUint32(32,true),ents=36+4*players;
 const slots=[];for(let i=0;i<high;i++)if(b[ents+i*stride]&&b[ents+i*stride+1]===8)slots.push(i);
 assert.ok(slots.length>=2);const [a,t]=slots;
 for(let i=0;i<high;i++)b[ents+i*stride]=0;
 for(const[i,def,team,x]of[[a,id,0,100.5],[t,1,1,101.5]]){
  const at=ents+i*stride;b[at]=1;b[at+1]=def;b[at+2]=team;
  for(const off of[8,16,52,96])d.setFloat32(at+off,x,true);
  for(const off of[12,20,56,100])d.setFloat32(at+off,100.5,true);
  for(const off of[24,28,32,36,44,92])d.setFloat32(at+off,0,true);
  d.setFloat32(at+40,5000,true);d.setFloat32(at+80,1,true);b[at+48]=0;b[at+88]=0;
  d.setUint32(at+60,0xffffffff,true);d.setUint32(at+68,0xffffffff,true);d.setUint16(at+76,65535,true);
 }
 const target=((d.getUint16(ents+t*stride+4,true)<<16)|t)>>>0,own=((d.getUint16(ents+a*stride+4,true)<<16)|a)>>>0;
 b[ents+a*stride+48]=3;d.setUint32(ents+a*stride+60,target,true);d.setUint32(ents+a*stride+68,target,true);
 // Controlled flat, empty local terrain; no generated obstacles at the duel.
 const map=ents+stride*high;for(let y=90;y<112;y++)for(let x=90;x<112;x++){b[map+y*256+x]=0;b[map+65536+y*256+x]=0;d.setUint16(map+2*65536+2*(y*256+x),0,true);}
 resetFixtureClaims(b);b=refreshFixtureCityGround(b);revealFixture(b);eq(load(w,b),1,'Controlled combat scenario loads');return {a,t,own,target,stride,ents};
}
const hp=(w,o)=>new DataView(save(w).buffer).getFloat32(o.ents+o.t*o.stride+40,true);
// Independent transcription of source007DB4E0's deadlines/strict condition.
function sourceTrace(reload,apply,ticks){let next=0,started=false,executed=false;const fire=[];for(let tick=1;tick<=ticks;tick++){
 const now=f(f(tick)*f(.05));if(!started){if(next<=now){next=f(now+reload);started=true;executed=false;}}
 else if(!executed){if(next-now<reload-apply){fire.push(tick);executed=true;}}
 else if(next<=now)started=false;
 }return fire;}
const w=await make();
for(const row of attack.rows){const values=[+row.present,row.minMobile,row.minImmobile,row.angularRange,row.applyTime,row.packTime,row.unpackTime,row.splashRadius,row.friendlyRatio,+!!row.missile,+row.attackOnMove,+!row.disableAutoTarget,+row.latent,row.ordnance,row.projectileSpeed??-1,row.visualKind,+row.mobile,+row.air];for(let field=0;field<values.length;field++)eq(w.native_attack_stat(row.id,row.epoch,field),f(values[field]),'Compiled native attack source row '+row.id+'/'+row.epoch+'/'+field);}
for(const args of[[57,0,0],[0,15,0],[0,0,99]])eq(w.native_attack_stat(...args),-1,'Invalid diagnostic input');
const o=fixture(w),source=attack.rows.find(r=>r.id===9&&r.epoch===0),expected=sourceTrace(f(source.effectiveStats.reload.value),source.applyTime,130),actual=[];
let previous=hp(w,o),mid;
for(let tick=1;tick<=130;tick++){w.tick();const current=hp(w,o);if(current<previous)actual.push(tick);previous=current;if(tick===4)mid=save(w);}
eq(actual,expected,'Live combat damage events match native start/apply/end clock, not instantaneous reload');
const midLayout=assertCurrentSave(mid);eq(midLayout.attackBytes>=32,true,'Windup has a persisted ATK1 record');
const resumed=await make();eq(load(resumed,mid),1,'Windup reloads');const replay=await make();eq(load(replay,mid),1,'Second replay starts same persisted state');
for(let i=0;i<125;i++){resumed.tick();replay.tick();eq(resumed.state_hash(),replay.state_hash(),'Round save replay stays deterministic');eq(save(resumed),save(replay),'Round save replay persists identical bytes');}
// Existing native4 UP saves migrate; the old loader is never asked to read ATK1.
const legacy=fs.readFileSync(new URL('openepoch-ee2-up15.wasm',baseline));const old=(await WebAssembly.instantiate(legacy,{})).instance.exports;
old.init(71,1,0,1,0);for(let i=0;i<77;i++)old.tick();const oldSave=save(old);eq(load(resumed,oldSave),1,'Native4 OEU5 save migrates');eq(assertCurrentSave(save(resumed)).attackBytes>=20,true,'Migrated save writes explicit current ATK1');
// Every malformed explicit tail must preserve the preexisting live world.
eq(load(resumed,mid),1);const valid=save(resumed),l=assertCurrentSave(valid),before=resumed.state_hash(),cases=[];
for(const[at,value]of[[l.attack,0],[l.attack+4,2],[l.attack+8,8193],[l.attack+12,0xffffffff],[l.attack+16,0x7fc00000],[l.attack+16,0x7f800000],[l.attack+16,0xbf800000],[l.attack+20,0],[l.attack+20,0xff],[valid.length-8,19],[valid.length-4,0]]){const b=valid.slice();new DataView(b.buffer).setUint32(at,value,true);cases.push(b);}
for(let n=1;n<l.attackBytes;n++)cases.push(valid.slice(0,-n));cases.push(new Uint8Array([...valid,0]));
// Construct two individually valid records, then reject duplication/reordering.
const pair=new Uint8Array(valid.length+12);pair.set(valid.subarray(0,l.attack+24));
const pd=new DataView(pair.buffer);pd.setUint32(l.attack+8,2,true);
pd.setUint32(l.attack+24,o.target,true);pd.setFloat32(l.attack+28,0,true);pd.setUint32(l.attack+32,0x80,true);
pd.setUint32(pair.length-8,44,true);pd.setUint32(pair.length-4,0x454b5441,true);
eq(load(resumed,pair),1,'Two ascending live-generation clocks are accepted');eq(load(resumed,valid),1);
const duplicate=pair.slice();duplicate.set(pair.subarray(l.attack+12,l.attack+24),l.attack+24);cases.push(duplicate);
const reversed=pair.slice();reversed.set(pair.subarray(l.attack+24,l.attack+36),l.attack+12);reversed.set(pair.subarray(l.attack+12,l.attack+24),l.attack+24);cases.push(reversed);
const oldPrefixStart=valid.slice();new DataView(oldPrefixStart.buffer).setUint32(valid.length-8,valid.length-l.end+4,true);cases.push(oldPrefixStart);
for(const bad of cases){eq(load(resumed,bad),0,'Malformed attack tail rejected');eq(resumed.state_hash(),before,'Malformed load is atomic');eq(save(resumed),valid,'Malformed load preserves full saved state');}
// ATK1 belongs solely to profile2; the existing foreign-save guard remains.
for(const name of['openepoch.wasm','openepoch-ee2.wasm']){const bytes=fs.readFileSync(new URL(name,baseline)),q=(await WebAssembly.instantiate(bytes,{})).instance.exports;q.init(7,1,0,1,0);const before=save(q),hash=q.state_hash();eq(load(q,mid),0,'Old profile cannot accept combat5 save');eq(q.state_hash(),hash);eq(save(q),before);}
const report={format:'native-combat5-actual-wasm-v1',status:'PASS',checks,wasmSha256:sha(blob),wasmBytes:blob.length,attackAuditSha256:sha(fs.readFileSync(new URL('docs/native-up15-attack.json',root))),expectedFireTicks:expected,actualFireTicks:actual,malformedCases:cases.length,limits:['Packed/unpacked state, turret banks, native blocking geometry and ordnance/rearm remain explicit adapters.','As in existing saves, active projectiles are transient; this replay fixture uses direct attacks.']};
fs.writeFileSync(new URL('../../EE2-Research/analysis/native-combat5-attack-wasm-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
