import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {originalLabelCatalog,loadOriginalLabels,escapeLabelHtml} from '../docs/original-labels.js';
const file=new URL('../docs/local-original/labels-native.json',import.meta.url);
const raw=await readFile(file),data=JSON.parse(raw);
let checks=0;const check=(condition,note)=>{assert.ok(condition,note);checks++;};
const profiles=[{id:'base',ruleset:1,reference:true},{id:'up15',ruleset:2,reference:true}];
for(const profile of profiles){
 const catalog=originalLabelCatalog(data,profile),rows=data.profiles[profile.id].rows;
 check(catalog.enabled,profile.id+' enabled');
 for(let id=0;id<57;id++)for(let epoch=0;epoch<15;epoch++){
  const matches=rows.filter(row=>row.id===id&&row.epoch<=epoch&&epoch<row.until);
  check(matches.length<2,'unique class interval');
  const row=matches[0],fallback='fallback <>'+id;
  check(catalog.name(id,epoch,fallback)===(row?.name??fallback),'class singular');
  check(catalog.name(id,epoch,fallback,4)===(row?(row.plural||row.name):fallback),'class plural');
 }
 check(catalog.name(0,0,'fallback')==='Stadtzentrum','actual city-center source name');
 check(catalog.name(8,0,'fallback')==='Bürger','inherited CitizenM name');
 check(catalog.name(15,0,'fallback')==='Streitkolbenträger','early heavy-infantry class');
 check(catalog.name(15,10,'fallback')==='Gewehrschütze','later heavy-infantry class');
 check(catalog.name(15,14,'fallback')==='Objective Force Warrior','future heavy-infantry class');
 check(catalog.name(55,14,'HERC-Fabrik')==='HERC-Fabrik','absent native token stays explicit fallback');
 for(const [id,epoch]of [[-1,0],[57,0],[8,-1],[8,15],[8,1.5],[8,NaN],['8',0]])check(catalog.name(id,epoch,'fallback')==='fallback','invalid lookup falls back');
 for(const rowsMutation of [rows=>[...rows,{...rows[0]}],rows=>[{...rows[0],until:16}],rows=>[{...rows[0],name:42}]]){
  const changed=structuredClone(data);changed.profiles[profile.id].rows=rowsMutation(rows);
  assert.throws(()=>originalLabelCatalog(changed,profile));checks++;
 }
 const changed=structuredClone(data);changed.profiles[profile.id].ruleset=999;
 assert.throws(()=>originalLabelCatalog(changed,profile));checks++;
 let requests=[];
 const loaded=await loadOriginalLabels(profile,{base:'https://game.invalid/path/original-labels.js?v=5',fetchImpl:async(url,options)=>{
  requests.push({url:String(url),options});return {ok:true,json:async()=>data};
 }});
 check(loaded.name(8,0,'fallback')==='Bürger','actual loader');
 check(requests.length===1&&requests[0].url==='https://game.invalid/path/local-original/labels-native.json'&&requests[0].options.cache==='no-store','relative portable URL');
 const warn=console.warn;console.warn=()=>{};
 try{
  for(const fetchImpl of [async()=>({ok:false,status:404}),async()=>{throw Error('unavailable');},async()=>({ok:true,json:async()=>({})})]){
   const failed=await loadOriginalLabels(profile,{fetchImpl});
   check(!failed.enabled&&failed.name(8,0,'own')==='own'&&failed.description(8,0)==='','optional missing media remains playable');
  }
 }finally{console.warn=warn;}
}
let fetched=false;const own=await loadOriginalLabels({id:'own',ruleset:0,reference:false},{fetchImpl:async()=>{fetched=true;throw Error('unexpected');}});
check(!fetched&&!own.enabled&&own.name(8,0,'Worker')==='Worker','independent ruleset never fetches original labels');
check(escapeLabelHtml('<b>"A&B\'</b>')==='&lt;b&gt;&quot;A&amp;B&#39;&lt;/b&gt;','original text cannot become HTML');
const special=structuredClone(data);special.profiles.base.rows[0].description='Damage %DAMAGE%';
check(originalLabelCatalog(special,profiles[0]).description(0,0)==='','unimplemented native formatter is not shown');
const report={format:'openepoch-native5-label-runtime-tests-v1',status:'passed',checks,profiles:profiles.map(p=>p.id),
 dataSha256:createHash('sha256').update(raw).digest('hex'),runtimeSha256:createHash('sha256').update(await readFile(new URL('../docs/original-labels.js',import.meta.url))).digest('hex'),
 scope:'Actual source-class selection and optional-media loader; independent ruleset, malformed media, profile mismatch and HTML escaping controls.'};
await writeFile(new URL('../../EE2-Research/analysis/native5-label-runtime-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
