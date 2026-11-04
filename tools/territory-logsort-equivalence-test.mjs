// Point-in-time comparison of the actual pre/post UI-log-sort WASM builds.
// The UI intentionally changes; simulation hashes and complete saves must not.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
const before=readFileSync(new URL('fixtures/reference-ee2-territory-5c92.wasm',import.meta.url));
const after=readFileSync(new URL('../docs/openepoch-ee2.wasm',import.meta.url));
const directory=new URL('../../EE2-Research/analysis/',import.meta.url);
const sha=b=>createHash('sha256').update(b).digest('hex');
let checks=0;const eq=(a,b,label)=>{checks++;assert.deepEqual(a,b,label);};
const ok=(value,label)=>{checks++;assert.ok(value,label);};
const report={format:'territory-ui-logsort-equivalence-v1',status:'running',beforeSha256:sha(before),afterSha256:sha(after),scope:'Representative actual WASM simulation and byte-for-byte save equivalence across a UI-only log-order change. Different UI-read frequencies are intentional. A separately controlled mixed DIP/TER chronology checks the displayed fix.',cases:[],checkCount:0};
const priorSoak=JSON.parse(readFileSync(new URL('economy-ai-soak.json',directory)));
const economy=JSON.parse(readFileSync(new URL('../docs/ee2-economy.json',import.meta.url)));
const standard=economy.startingResourceLevels.find(row=>row.NAME==='Standard_1');
report.priorSoak={wasmSha256:priorSoak.wasmSha256,simulatedMinutes:priorSoak.cases.reduce((sum,item)=>sum+item.minutes,0),warningCount:priorSoak.warnings.length,scope:'The full20+10-minute soak was performed on5C92, not rerun on6E13. Both genuine first-minute traces are replayed below on both builds, in addition to the representative comparisons.',anchors:[]};
eq(report.beforeSha256,'5c9299b10683a81624b87c51efa1904529c9390e0f4bf0e6beb0990a52e66560','genuine preceding territory build');
eq(report.afterSha256,'6e133ae7dc456ff196d0a6ea98eeb0848688dc4cd9bedb1126899ba37d860724','approved final log-sort build');
const make=async b=>(await WebAssembly.instantiate(b,{})).instance.exports;
const save=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
const load=(w,b)=>{const p=w.load_alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);return w.load_apply();};
const ui=w=>{const p=w.ui();return JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,w.ui_len())));};
const run=(w,n)=>{for(let i=0;i<n;i++)w.tick();};
function packet(w,team,kind,a,b,c){const bytes=new Uint8Array(14),d=new DataView(bytes.buffer);bytes[0]=kind;d.setFloat32(2,a,true);d.setFloat32(6,b,true);d.setUint32(10,c>>>0,true);const p=w.inbox_alloc(bytes.length);new Uint8Array(w.memory.buffer,p,bytes.length).set(bytes);w.net_apply(team);}
function same(a,b,label){eq(a.state_hash(),b.state_hash(),label+' state hash');eq(save(a),save(b),label+' complete save bytes');}
function uiIsPure(w,label){const bytes=save(w),h=w.state_hash();ui(w);eq(w.state_hash(),h,label+' UI does not change hash');eq(save(w),bytes,label+' UI does not change any saved byte');}
try{
 eq(priorSoak.wasmSha256,report.beforeSha256,'prior long soak really used the archived preceding build');
 for(const seed of [7,41])for(const epoch of [0,3,9,12,14]){
  const a=await make(before),b=await make(after);
  for(const w of [a,b]){w.init(seed,3,0,1,0);eq(w.skirmish_epoch(epoch),1);eq(w.cmd_diplomacy(1,5,1,100),1);eq(w.cmd_diplomacy(1,0,0,0),1);}
  same(a,b,'initial');
  for(let tick=1;tick<=1200;tick++){
   for(const w of [a,b])w.tick();
   if(tick===700)eq(a.cmd_diplomacy(1,5,1,100),b.cmd_diplomacy(1,5,1,100),'same additional tribute result');
   if(tick===750)eq(a.cmd_diplomacy(1,1,1,0),b.cmd_diplomacy(1,1,1,0),'same alliance negotiation result');
   if(tick%17===0)ui(b);if(tick%31===0)ui(a);
   if(tick%100===0){uiIsPure(a,'previous');uiIsPure(b,'final');same(a,b,`seed${seed}/epoch${epoch+1}/tick${tick}`);}
  }
  report.cases.push({seed,epoch:epoch+1,ticks:1200,finalStateHash:a.state_hash()>>>0,finalSaveSha256:sha(save(a))});
 }
 for(const test of priorSoak.cases){
  const a=await make(before),b=await make(after),keys=['FOOD','WOOD','IRON','STONE','GOLD','TIN','SALTPETER','OIL','URANIUM'];
  for(const w of [a,b]){
   w.init(test.seed,3,0,1,0);const bytes=save(w),d=new DataView(bytes.buffer),ps=d.getUint32(8,true),end=36+4*ps+d.getUint32(4,true)*d.getUint32(32,true)+5*65536;
   for(let team=0;team<4;team++){const p=36+team*ps;bytes[p+24]=test.startingEpoch-1;bytes[p+27]=1;d.setUint32(p+28,0x7fffffff,true);for(let slot=0;slot<9;slot++)d.setFloat32(slot<3?p+4*slot:end+140+(team*6+slot-3)*4,standard[keys[slot]],true);}
   eq(load(w,bytes),1);eq(w.state_hash()>>>0,test.snapshots[0].stateHash,'genuine soak initial state is reproduced');
  }
  for(let tick=1;tick<=1200;tick++){a.tick();b.tick();if(tick%17===0)ui(b);if(tick%31===0)ui(a);if(tick%200===0)same(a,b,'genuine soak first-minute trace');}
  eq(a.state_hash()>>>0,test.snapshots[1].stateHash,'preceding build matches the recorded first soak minute');eq(b.state_hash()>>>0,test.snapshots[1].stateHash,'final build matches the same recorded first soak minute');
  report.priorSoak.anchors.push({seed:test.seed,startingEpoch:test.startingEpoch,ticks:1200,recordedStateHash:test.snapshots[1].stateHash,finalSaveSha256:sha(save(b))});
 }
 const a=await make(before),b=await make(after);
 for(const w of [a,b]){
  w.init(71,1,0,1,0);let bytes=save(w),d=new DataView(bytes.buffer),ps=d.getUint32(8,true);for(let t=0;t<4;t++)bytes[36+t*ps+27]=0;eq(load(w,bytes),1);
  eq(w.cmd_diplomacy(1,0,0,0),1);packet(w,1,14,0,0,3<<8);run(w,1);
  const map=new Uint8Array(w.memory.buffer,w.terr_ptr(),65536*4),psize=d.getUint32(8,true),esize=d.getUint32(4,true),high=d.getUint32(32,true),towns=[];
  bytes=save(w);d=new DataView(bytes.buffer);
  for(let i=0;i<high;i++){const at=36+4*psize+i*esize;if(bytes[at]&&bytes[at+1]===0){const team=bytes[at+2],x=d.getFloat32(at+8,true),y=d.getFloat32(at+12,true);towns[team]=map[(Math.floor(y)*256+Math.floor(x))*4];}}
  eq(w.cmd_territory(1,1,towns[0],towns[1]),1);run(w,1);packet(w,1,15,-1,-1,2<<8);run(w,3);eq(w.cmd_diplomacy(1,2,0,0),1);
 }
 same(a,b,'mixed DIP/TER scenario before UI');uiIsPure(a,'previous mixed');uiIsPure(b,'final mixed');same(a,b,'mixed DIP/TER scenario after UI');
 const oldLogs=ui(a).diplomacy.log,logs=ui(b).diplomacy.log;
 const accepted=list=>list.findIndex(text=>text.includes('nimmt den Gebietstausch an'));
 const war=list=>list.findIndex(text=>text.includes('erklärt Krieg'));
 ok(accepted(oldLogs)>=0&&war(oldLogs)>=0&&war(oldLogs)<accepted(oldLogs),'controlled case actually reproduces the previous chronology defect');
 ok(accepted(logs)>=0&&war(logs)>accepted(logs),'final UI displays the earlier accepted swap before the later war');
 report.chronology={previous:oldLogs,final:logs};report.status='passed';
}catch(error){report.status='failed';report.error=error.stack||String(error);throw error;}
finally{report.checkCount=checks;mkdirSync(directory,{recursive:true});writeFileSync(new URL('territory-logsort-equivalence.json',directory),JSON.stringify(report,null,2)+'\n');}
console.log(`PASS UI-only5C92→6E13: ${checks} checks,10 cases ×1200 ticks plus both recorded first-minute soak traces, byte-identical saves/hashes under different UI-read frequencies, and actual mixed DIP/TER log chronology corrected.`);
