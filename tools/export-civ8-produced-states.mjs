// Real public-API production fixtures for the combined Pack8 migration/visual QA.
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {buildOrdinaryCiv8Unit} from './ordinary-civ8-fixture.mjs';
const sha=b=>createHash('sha256').update(b).digest('hex');
const bytes=await fs.readFile(new URL('../docs/openepoch-ee2-up15.wasm',import.meta.url));
const source=JSON.parse(await fs.readFile(new URL('./fixtures/civ8-production-inputs.json',import.meta.url)));
const {instance:{exports:w}}=await WebAssembly.instantiate(bytes,{}),out=new URL('./fixtures/civ8-produced-current/',import.meta.url);
await fs.mkdir(out,{recursive:true});const fixtures=[];
for(const candidate of source){
 const {state,...fixture}=buildOrdinaryCiv8Unit(w,candidate,{seed:7});
 const name=`civ8-produced-${fixture.def}.oes`;await fs.writeFile(new URL(name,out),state);
 fixtures.push({name,bytes:state.length,sha256:sha(state),...fixture});
}
const receipt={format:'native-civ8-ordinary-produced-snapshots-v1',wasmSha256:sha(bytes),fixtures,
 scope:'Nineteen source families, normal paid train durations after instant producer construction is switched off. Public Core commands only; save bytes are read and retained without entity edits.'};
await fs.writeFile(new URL('receipt.json',out),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify({snapshots:fixtures.length,bytes:fixtures.reduce((s,f)=>s+f.bytes,0),wasmSha256:sha(bytes)}));
