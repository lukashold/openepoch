const option=(key,fallback)=>process.argv.find(arg=>arg.startsWith('--'+key+'='))?.slice(key.length+3)||fallback;
import fs from'node:fs';import path from'node:path';import{fileURLToPath}from'node:url';import{createHash}from'node:crypto';import assert from'node:assert/strict';
import{save}from'./roster6-state.mjs';import{saveLayout}from'./save-fixture.mjs';
const root=fileURLToPath(new URL('../',import.meta.url)),work=path.resolve(root,'..'),binary=fs.readFileSync(option('wasm',path.join(root,'docs/openepoch-ee2-up15.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex'),make=async()=>(await WebAssembly.instantiate(binary,{})).instance.exports;
const peers=[await make(),await make()];let checks=0,packets=0;const eq=(a,b,m)=>{assert.equal(a,b,m);checks++;};
function ledger(w){const b=save(w),l=saveLayout(b);return b.slice(l.paid,l.payloadEnd);}
function route(seat){const w=peers[seat],n=w.outbox_len(),bytes=new Uint8Array(w.memory.buffer,w.outbox_ptr(),n).slice();assert.ok(n>0);checks++;w.outbox_clear();for(const target of peers){const p=target.inbox_alloc(n);new Uint8Array(target.memory.buffer,p,n).set(bytes);target.net_apply(seat);}packets++;}
for(let seat=0;seat<2;seat++){const w=peers[seat];w.init(41,0,0,2,seat);eq(w.cheat(0),3,'network cheats denied');w.select_point(w.start_x(),w.start_y(),0,0,0,0,256,256);}
for(let i=0;i<5;i++)eq(peers[0].cmd_queue_job(57),1);for(let i=0;i<2;i++)eq(peers[1].cmd_queue_job(8),1);
// Submission only serializes commands; neither account nor invoice changes yet.
eq(peers[0].resource_stat(0,0),400);eq(saveLayout(save(peers[0])).d.getUint32(saveLayout(save(peers[0])).paid+8,true),0);
route(0);route(1);eq(peers[0].resource_stat(0,0),250);eq(peers[0].resource_stat(1,0),300);
eq(peers[0].state_hash(),peers[1].state_hash());eq(sha(ledger(peers[0])),sha(ledger(peers[1])));
for(let tick=0;tick<1000;tick++){
 if(tick===100){peers[0].cmd_cancel();route(0);eq(peers[0].resource_stat(0,0),280,'cancel refunds the recorded30');}
 if(tick===200){eq(peers[1].cmd_queue_job(57),1);route(1);}
 for(let i=0;i<2;i++){const w=peers[i];w.tick();if(tick%(i?17:13)===0)w.render(i?.8:.2,0,0,256,256,-1,0,0);if(tick%(i?29:31)===0)save(w);}
 eq(peers[0].state_hash(),peers[1].state_hash(),'actual inbox application lockstep');
 if(tick%100===0)eq(sha(ledger(peers[0])),sha(ledger(peers[1])),'same complete invoices including durations');
}
const report={format:'native-paidqueue7-network-wasm-v1',status:'PASS',checks,packets,ticks:1000,wasmSha256:sha(binary),source:'Actual humans2/local0+1, outbox/inbox/net_apply command transport. No memory or save modifications.',limits:['This exercises the simulation transport; WebRTC/browser negotiation and protocol fingerprint are integrated separately by Root.','Local display fog and selection are client-owned; peer equivalence compares simulation hash and the complete financial ledger.']};
fs.writeFileSync(option('report',path.join(work,'paidqueue-network-report.json')),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
