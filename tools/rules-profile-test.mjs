// Production profile loader + actual main start/IndexedDB/save/channel code.
// No browser, native executable or network socket is launched.
import fs from 'node:fs/promises';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {resolveRulesProfile,rulesProfileURL,loadRulesWasm,assertRulesProfile,savedRulesProfileMatches,UP15_PROFILE_ENABLED} from '../docs/rules-profile.js';
import {createSeasonSession,peerSeasonSettings,savedSeasonSettings} from '../docs/season-session.js';
import {checkedClimate,peerClimates,savedClimate} from '../docs/skirmish-climates.js';
import {normalizeSeasonSettings} from '../docs/original-seasons.js';
import {normalizeOriginalHeading,originalFractionForZoom} from '../docs/original-camera.js';
import {Lockstep,PROTOCOL_VERSION} from '../docs/net.js';
import {createCivilizationSetup,civilizationChoice,resolveCivilizations,checkedCivilizations,applyMatchSettings,checkedEpochRange,resolveEpochRange} from '../docs/civilization-setup.js';

const root=new URL('../',import.meta.url),analysis=new URL('../../EE2-Research/analysis/',import.meta.url);
const sha=b=>createHash('sha256').update(b).digest('hex');
const sourceNames=['main.js','index.html','rules-profile.js','net.js','season-session.js','skirmish-climates.js','civilization-setup.js'];
const sourceBytes=Object.fromEntries(await Promise.all(sourceNames.map(async name=>[name,await fs.readFile(new URL('docs/'+name,root))])));
const sourceHashes=Object.fromEntries(Object.entries(sourceBytes).map(([name,bytes])=>[name,sha(bytes)]));
const main=sourceBytes['main.js'].toString(),html=sourceBytes['index.html'].toString();
const segment=(a,b)=>{const start=main.indexOf(a),end=main.indexOf(b,start);assert.ok(start>=0&&end>start,a);return main.slice(start,end);};
const controls=segment('function menuSeasonSettings()',"$('season-options').hidden");
const profileBoot=segment("const profileSelect=$('rules-profile');",'try { const preference=');
const start=segment('function resolvedSkirmishChoices', 'const db =');
const saves=segment('const db =','// ---------------------------------------------------------------- simulation stepping');
const channel=segment('function channel(local)',"$('mpopen').onclick");
const seed=main.match(/^const seedOf = .*$/m)[0];
let checks=0;const check=(a,m)=>{assert.ok(a,m);checks++;},equal=(a,b,m)=>{assert.deepEqual(a,b,m);checks++;};
const plain=a=>JSON.parse(JSON.stringify(a));
const binaryFiles={own:'openepoch.wasm',base:'openepoch-ee2.wasm',up15:'openepoch-ee2-up15.wasm'};
const binaries=Object.fromEntries(await Promise.all(Object.entries(binaryFiles).map(async([id,file])=>[id,await fs.readFile(new URL('docs/'+file,root))])));
const profiles=Object.fromEntries(['own','base','up15'].map(id=>[id,resolveRulesProfile(`http://localhost/${id==='own'?'':'?rules=ee2&profile='+id}`,{up15Enabled:true})]));
const fetches=[];
const fetchFile=async url=>{const file=url.pathname.split('/').at(-1),id=Object.keys(binaryFiles).find(key=>binaryFiles[key]===file);fetches.push(url.href);return id?{ok:true,status:200,arrayBuffer:async()=>binaries[id]}:{ok:false,status:404};};
const fresh=id=>loadRulesWasm(profiles[id],new URL('docs/',root),{fetchFile});

equal(resolveRulesProfile('http://localhost/?qa&profile=base').id,'own','EE2 mode is explicit');
equal(resolveRulesProfile('http://localhost/?rules=ee2',{up15Enabled:false}).id,'base','Unreleased UP never becomes implicit default');
equal(resolveRulesProfile('http://localhost/?rules=ee2',{up15Enabled:true}).id,'up15','Released EE2 default is installed UP');
equal(resolveRulesProfile('http://localhost/?rules=ee2').id,UP15_PROFILE_ENABLED?'up15':'base','Production feature switch controls the actual default');
for(const id of ['own','base','up15']){
 const u=rulesProfileURL('http://localhost/game/?rules=ee2&profile=base&qa=1&epoch=4&seed=71#menu',id,{up15Enabled:true});
 equal(u.searchParams.get('qa'),'1','Profile URL preserves QA suffix source');equal(u.searchParams.get('epoch'),'4','Profile URL preserves epoch');equal(u.searchParams.get('seed'),'71','Profile URL preserves seed');equal(u.hash,'#menu','Profile URL preserves hash');
 const p=resolveRulesProfile(u,{up15Enabled:true});equal(p.id,id,'Profile selector URL roundtrip');check(p.database.endsWith('-qa'),'QA uses a separate database suffix');
}
for(const url of ['http://localhost/?rules=ee2&profile=nope','http://localhost/?rules=ee2&profile=own','http://localhost/?rules=ee2&profile=__proto__']){assert.throws(()=>resolveRulesProfile(url,{up15Enabled:true}));checks++;}
assert.throws(()=>resolveRulesProfile('http://localhost/?rules=ee2&profile=up15',{up15Enabled:false}));checks++;
equal(profiles.own.database,'openepoch-v2','Existing independent save DB retained');equal(profiles.base.database,'openepoch-ee2-reference-v2','Existing BASE save DB retained');equal(profiles.up15.database,'openepoch-ee2-up15-v1','New UP DB is isolated');
equal(new Set(Object.values(profiles).map(p=>p.database)).size,3,'No two numerical profiles share save storage');
for(const id of ['own','base','up15']){
 const elements=new Map(),navigation=[],url=id==='own'?'http://localhost/game/?qa&seed=71':`http://localhost/game/?rules=ee2&profile=${id}&qa&seed=71`;
 const $=key=>{if(!elements.has(key))elements.set(key,{options:['own','base','up15'].map(value=>({value,disabled:false})),value:'',disabled:false,textContent:''});return elements.get(key);};
 const context=vm.createContext({$,UP15_PROFILE_ENABLED,resolveRulesProfile,rulesProfileURL,location:{href:url,assign:value=>navigation.push(value)}});
 vm.runInContext(profileBoot+'\nglobalThis.boot={rulesProfile,referenceMode,qaMode};',context);
 equal(context.boot.rulesProfile.id,id,'Actual main startup resolves the requested profile');equal(context.boot.qaMode,true,'Actual main startup preserves QA mode');equal($('rules-profile').value,id,'Actual menu selection shows the active profile');
 for(const target of ['own','base','up15']){$('rules-profile').value=target;$('rules-profile').onchange();const moved=resolveRulesProfile(navigation.at(-1),{up15Enabled:true});equal(moved.id,target,'Actual selector handler navigates to the chosen profile');check(moved.qa,'Actual selector handler retains QA separation');}
 context.rulesLoadFailure(Error('UP-Datei fehlt'));check(['play','mpopen','continue','previous-save'].every(key=>$(key).disabled),'Actual main failure handler disables game/MP/save launch');equal($('rules-note').textContent,'UP-Datei fehlt','Actual main shows the actionable load error');
}
for(const [id,p]of Object.entries(profiles)){const before=fetches.length,w=await fresh(id);equal(w.ruleset(),p.ruleset,'Requested binary has exact ruleset ID');equal(fetches.length,before+1,'One requested file fetch per successful profile load');assert.throws(()=>assertRulesProfile(w,profiles[id==='own'?'base':'own']));checks++;}
for(const schema of [undefined,5,6,7]){
 const old={ruleset:()=>2,...(schema===undefined?{}:{schema_revision:()=>schema})};
 assert.throws(()=>assertRulesProfile(old,profiles.up15),/Speicher-\/Einheitenformat/);checks++;
}
let failures=0;
await assert.rejects(loadRulesWasm(profiles.up15,new URL('docs/',root),{fetchFile:async()=>{failures++;return{ok:false,status:404};}}),/UP-Regeln.*HTTP 404/);checks++;equal(failures,1,'Missing UP file never falls back to BASE');
await assert.rejects(loadRulesWasm(profiles.up15,new URL('docs/',root),{fetchFile:async()=>({ok:true,arrayBuffer:async()=>binaries.base})}),/Falsche Spieldatei/);checks++;
let streams=0,decoded=0;
const mimeWasm={instantiateStreaming:async()=>{streams++;throw Error('bad MIME');},instantiate:async(...args)=>{decoded++;return WebAssembly.instantiate(...args);}};
const mime=await loadRulesWasm(profiles.base,new URL('docs/',root),{fetchFile:async()=>({ok:true,clone(){return this;},arrayBuffer:async()=>binaries.base}),webAssembly:mimeWasm});equal(mime.ruleset(),1,'MIME fallback decodes the same requested binary');equal([streams,decoded],[1,1],'MIME fallback uses one streaming attempt and one byte decode');

// Browser transaction timing and storage namespaces are substituted; every
// main db()/dbDo()/saveGame()/loadGame()/refreshContinue() function is real.
const databases=new Map(),opened=[];
const indexedDB={open(name,version){
 opened.push(name);const request={};queueMicrotask(()=>{
  const upgrade=!databases.has(name);if(upgrade)databases.set(name,new Map());const records=databases.get(name);
  request.result={createObjectStore(store){equal(store,'saves','Production creates the expected object store');},close(){},transaction(store,mode){
   equal(store,'saves','Production transactions use the saves store');const tx={};
   tx.objectStore=()=>({get(key){const q={};queueMicrotask(()=>{q.result=structuredClone(records.get(key));q.onsuccess?.();queueMicrotask(()=>tx.oncomplete?.());});return q;},put(value,key){records.set(key,structuredClone(value));return{};}});return tx;
  }};if(upgrade)request.onupgradeneeded?.();request.onsuccess?.();
 });return request;
}};
async function environment(id,seat=0,qa=false,menu={}){
 const p=qa?resolveRulesProfile(`http://localhost/?${id==='own'?'':'rules=ee2&profile='+id+'&'}qa`,{up15Enabled:true}):profiles[id],w=await fresh(id),elements=new Map(),messages=[],begins=[];
 const $=id=>{if(!elements.has(id))elements.set(id,{value:'',checked:false,disabled:false,style:{},textContent:''});return elements.get(id);};
 for(const [key,value] of Object.entries({...{'pop-limit':'500',supplies:'1','start-epoch':'0','end-epoch':'14','map-visibility':'2','map-climate':'0','season-start':seat?'10':'2','season-length':seat?'30':'5',seed:seat?'99':'7',mpai:seat?'2':'0'},...menu.values}))$(key).value=value;
 $('season-cycle').checked=!seat;
 const render={setSeasonPhase(){},setSeasonClimate(){},setLightingSettings(){},setLightingContext(){}};
 const climates=menu.climates??[0],climateControl=$('map-climate'),menuClimate=()=>checkedClimate(+climateControl.value,climates);
 const session=createSeasonSession(w,render,render,render,climates),cam={x:0,y:0,zoom:28,heading:0};
 class Element{constructor(){this.children=[];this.style={};this.value='';}append(...nodes){this.children.push(...nodes);}before(node){this.previous=node;}}
 const anchor=new Element(),civilizationSetup=createCivilizationSetup({wasm:w,document:{createElement:()=>new Element()},anchor});
 if(civilizationSetup.enabled&&menu.choices)anchor.previous.children.slice(1).forEach((row,i)=>row.children[1].value=String(menu.choices[i]));
 let draws=0;
 const context=vm.createContext({wasm:w,rulesProfile:p,savedRulesProfileMatches,indexedDB,$,seasonSession:session,savedSeasonSettings,normalizeSeasonSettings,peerSeasonSettings,
  checkedClimate,peerClimates,savedClimate,climates,climateControl,menuClimate,
  civilizationSetup,civilizationChoice,resolveCivilizations,checkedCivilizations,applyMatchSettings,checkedEpochRange,resolveEpochRange,
  sfx:{presentationWord:()=>{const word=menu.words?.[draws];draws++;return word??draws*29;}},
  referenceMode:p.reference,qaMode:false,normalizeOriginalHeading,originalFractionForZoom,innerWidth:1280,innerHeight:720,MAP:256,performance,queueMicrotask,
  Lockstep,PROTOCOL_VERSION,nameSettings:new URLSearchParams(),originalEnabled:false,fullmap:null,
  mpStatus:t=>messages.push(t),toast:t=>messages.push(t),begin:()=>begins.push(w.game_tick()),
 });
 vm.runInContext(`let running=true,bench=0,net=null,ui=null,lastUi=0;const cam=${JSON.stringify(cam)},u8=(p,n)=>new Uint8Array(wasm.memory.buffer,p,n);
 ${controls}\n${seed}\n${start}\n${saves}\n${channel}
 globalThis.api={start,saveGame,loadGame,refreshContinue,channel,cam,getNet:()=>net,getSettings:()=>seasonSession.settings,getUi:()=>ui};`,context,{filename:'main.js actual profile start/IndexedDB/save/channel'});
 return {id,p,w,$,session,messages,begins,api:context.api,draws:()=>draws};
}
const sessions={};const saved={};
for(const id of ['own','base','up15']){
 const e=await environment(id);sessions[id]=e;e.api.start(1,77,0);equal(e.w.ruleset(),e.p.ruleset,'Actual start uses the selected WASM');equal(e.begins.length,1,'Singleplayer starts once');
 for(let t=0;t<37;t++)e.w.tick();const hash=e.w.state_hash(),phase=plain(e.session.update());await e.api.saveGame(true);
 const store=databases.get(e.p.database);check(store.has('slot1'),'Production save writes selected profile DB');const record=store.get('slot1');saved[id]=structuredClone(record);equal([record.rules,record.profile],[e.p.ruleset,id],'Save record contains exact profile metadata');
 for(let t=0;t<5;t++)e.w.tick();await e.api.loadGame();equal(e.w.state_hash(),hash,'Actual saved world restores within same profile');equal(plain(e.session.update()),phase,'Actual saved season phase restores within same profile');
 await e.api.refreshContinue();equal(e.$('continue').style.display,'','Continue finds only its selected namespace');
 const other=structuredClone(record);other.rules=(record.rules+1)%3;store.set('slot1',other);const before=e.w.state_hash(),begins=e.begins.length;await e.api.loadGame();equal(e.w.state_hash(),before,'Wrong profile metadata rejects without changing world');equal(e.begins.length,begins,'Wrong profile save cannot begin a session');await e.api.refreshContinue();equal(e.$('continue').style.display,'none','Wrong profile record is hidden from Continue');
 store.set('slot1',structuredClone(record));const legacy=structuredClone(record);delete legacy.rules;delete legacy.profile;equal(savedRulesProfileMatches(legacy,e.p),id!=='up15','Legacy metadata support is restricted to established own/BASE databases');
 if(id!=='up15'){store.set('slot1',legacy);await e.api.loadGame();equal(e.begins.length,begins+1,'Existing legacy own/BASE records continue to load');}
 store.set('slot1',structuredClone(record));
}
for(const [id,e]of Object.entries(sessions))for(const [other,s]of Object.entries(saved))if(other!==id){
 const store=databases.get(e.p.database),forged=structuredClone(s);forged.rules=e.p.ruleset;forged.profile=id;store.set('slot1',forged);const before=e.w.state_hash(),begins=e.begins.length;await e.api.loadGame();equal(e.w.state_hash(),before,'WASM magic rejects foreign save even if metadata is relabeled');equal(e.begins.length,begins,'Foreign save never starts a session');store.set('slot1',structuredClone(saved[id]));
}
for(const id of ['own','base','up15']){const e=await environment(id,0,true);e.api.start(1,31,0);await e.api.saveGame(true);check(databases.has(profiles[id].database+'-qa'),'Actual main IndexedDB preserves the QA namespace suffix');equal(databases.get(profiles[id].database).get('slot1').bytes,saved[id].bytes,'QA save leaves ordinary profile saves untouched');}
const network=[];
for(const id of ['own','base','up15']){
 const peers=await Promise.all([environment(id,0),environment(id,1)]),queue=[[],[]],handlers=peers.map((p,i)=>p.api.channel(i));let packets=0,commandPackets=0;
 const transports=peers.map((p,i)=>({readyState:'open',send(data){if(typeof data!=='string'&&data.byteLength>12)commandPackets++;queue[1-i].push(typeof data==='string'?data:data.slice(0));packets++;},close(){this.readyState='closed';handlers[i].close();}}));
 const flush=()=>{for(let guard=0;guard<100;guard++){let any=false;for(let i=0;i<2;i++)for(const message of queue[i].splice(0)){handlers[i].message(message);any=true;}if(!any)return;}throw Error('Channel queue did not drain');};
 handlers.forEach((h,i)=>{h.setPc({close(){}});h.open(transports[i]);});flush();
 for(const p of peers){check(p.api.getNet()?.ls instanceof Lockstep,'Production channel creates real Lockstep for '+id);equal(plain(p.session.settings),plain(peers[0].session.settings),'Host calendar transfers across differing guest menu settings');}
 equal(peers[0].w.state_hash(),peers[1].w.state_hash(),'Host seed/AI/visibility initialize identical '+id+' worlds');
 for(const p of peers){const worker=p.w.select_idle_worker();check(worker!==0xffffffff,'Actual local worker can issue a network command');p.w.select_none();p.w.select_point(p.w.ent_x(worker),p.w.ent_y(worker),0,0,0,0,256,256);p.w.cmd_context(p.w.start_x()+5,p.w.start_y()+3,0);check(p.w.outbox_len()>0,'Actual profile command enters the network outbox');}
 for(let tick=0;tick<180;tick++){flush();for(const p of peers)check(p.api.getNet().ls.step(),'Actual profile lockstep tick advances');flush();}
 equal(peers[0].w.state_hash(),peers[1].w.state_hash(),'Actual profile simulation stays synchronized');check(peers.every(p=>!p.api.getNet().ls.desync),'No profile lockstep desync');check(commandPackets>=2,'Genuine player commands cross the queued transport');network.push({profile:id,ruleset:peers[0].w.ruleset(),ticks:peers[0].w.game_tick(),packets,commandPackets,stateHash:peers[0].w.state_hash()>>>0});
}
{
 const climates=[0,1,2,3];
 const solo=await environment('up15',0,false,{climates,values:{'map-climate':'3'}});
 solo.api.start(1,71,0);
 equal(solo.session.climate,3,'Singleplayer starts with the chosen climate');
 await solo.api.saveGame(true);
 const store=databases.get(solo.p.database),record=store.get('slot1');
 equal(record.climate,3,'New save persists a nondefault climate');
 solo.$('map-climate').value='1';solo.api.start(1,72,0);
 await solo.api.loadGame();equal(solo.session.climate,3,'Load restores the saved climate');
 const legacy=structuredClone(record);delete legacy.climate;store.set('slot1',legacy);
 await solo.api.loadGame();equal(solo.session.climate,0,'Old saves restore Temperate');
 const forged=structuredClone(record);forged.climate='3';store.set('slot1',forged);
 const before=solo.w.state_hash(),begins=solo.begins.length;
 await solo.api.loadGame();equal(solo.w.state_hash(),before,'Malformed climate leaves current simulation intact');
 equal(solo.begins.length,begins,'Malformed climate does not start a session');
 store.set('slot1',structuredClone(saved.up15));

 for(const [selected,guestClimates] of [[3,climates],[0,[0]],[3,[0]]]){
  const peers=await Promise.all([environment('up15',0,false,{climates,values:{'map-climate':String(selected)}}),
   environment('up15',1,false,{climates:guestClimates})]);
  const queue=[[],[]],handlers=peers.map((p,i)=>p.api.channel(i));
  let startMessage;
  const channels=peers.map((_,i)=>({readyState:'open',send(message){
   if(typeof message==='string'&&JSON.parse(message).t==='start')startMessage=JSON.parse(message);
   queue[1-i].push(typeof message==='string'?message:message.slice(0));
  },close(){this.readyState='closed';handlers[i].close();}}));
  handlers.forEach((h,i)=>h.open(channels[i]));
  for(let guard=0;queue.some(q=>q.length);guard++){
   assert.ok(guard<20,'Climate handshake drains');
   for(let i=0;i<2;i++)for(const msg of queue[i].splice(0))handlers[i].message(msg);
  }
  if(selected&&!guestClimates.includes(selected)){
   equal(startMessage,undefined,'Unsupported guest climate prevents the match from starting');
   check(peers[0].messages.some(text=>text.includes('Originalgrafiken für dieses Klima')));
  }else{
   equal(startMessage.climate,selected||undefined,'Handshake uses a backward-compatible default climate');
   for(const peer of peers){check(peer.api.getNet()?.ls instanceof Lockstep);equal(peer.session.climate,selected);}
   equal(peers[0].w.state_hash(),peers[1].w.state_hash(),'Climate negotiation preserves peer simulation state');
  }
 }
}
for(const id of ['own','base','up15'])for(const other of ['own','base','up15'])if(id!==other){const e=await environment(id),h=e.api.channel(1),ch={readyState:'open',send(){},close(){this.readyState='closed';h.close();}};h.open(ch);h.message(JSON.stringify({t:'hello',protocol:PROTOCOL_VERSION,rules:profiles[other].ruleset}));equal(ch.readyState,'closed','Different numerical profiles reject in real channel');check(!e.api.getNet(),'Cross-profile hello cannot initialize multiplayer');}
for(const id of ['own','base','up15']){const e=await environment(id),h=e.api.channel(1),ch={readyState:'open',send(){},close(){this.readyState='closed';h.close();}};h.open(ch);h.message(JSON.stringify({t:'hello',protocol:PROTOCOL_VERSION-1,rules:e.p.ruleset}));equal(ch.readyState,'closed','All profiles reject an older network protocol');check(!e.api.getNet(),'Old protocol cannot initialize multiplayer');}
check(html.includes('for="rules-profile">Regelversion'),'Rule selector has a visible accessible label');equal((html.match(/id="rules-profile"/g)||[]).length,1,'Rule selector ID is unique');check(main.includes('await loadNativeHud(originalAssets,referenceMode)'),'Native HUD loading remains in both EE2 profiles');
// New choices flow through the actual Main start/channel functions and real Core.
{
 const solo=await environment('up15',0,false,{values:{'start-epoch':'5','end-epoch':'6','pop-limit':'1000',supplies:'2'},choices:[0,6,5,2]});
 solo.api.start(3,713,0);equal(Array.from({length:4},(_,i)=>solo.w.player_civilization(i)),[0,6,5,2],'Singleplayer uses all four visible civilizations');
 equal(solo.w.stat(0,0),5,'Singleplayer retains the selected epoch after applying civilization traits');
 equal(solo.w.match_final_epoch(),6,'Singleplayer uses the selected end epoch');
 equal(solo.draws(),0,'Fixed civilizations and epochs use no presentation draws');
 await solo.api.saveGame(true);const hash=solo.w.state_hash();for(let i=0;i<7;i++)solo.w.tick();await solo.api.loadGame();
 equal(solo.w.state_hash(),hash,'Civilization-bearing save restores without applying menu traits a second time');
 equal(solo.w.match_final_epoch(),6,'The actual save handler restores the end epoch');
 equal(Array.from({length:4},(_,i)=>solo.w.player_civilization(i)),[0,6,5,2]);
 const peers=await Promise.all([
  environment('up15',0,false,{values:{mpai:'2','start-epoch':'-1','end-epoch':'-1','pop-limit':'1000',supplies:'2','map-visibility':'0'},
   choices:[0,1,15,2],words:[19,35,8]}),
  environment('up15',1,false,{values:{mpai:'0','start-epoch':'0','end-epoch':'12','pop-limit':'250',supplies:'0','map-visibility':'2'},choices:[6,3,4,7]}),
 ]),pending=[[],[]],handlers=peers.map((p,i)=>p.api.channel(i));let startMessage;
 const transport=peers.map((p,i)=>({readyState:'open',send(value){if(typeof value==='string'&&JSON.parse(value).t==='start')startMessage=JSON.parse(value);pending[1-i].push(typeof value==='string'?value:value.slice(0));},close(){this.readyState='closed';handlers[i].close();}}));
 const flush=()=>{for(let guard=0;guard<100;guard++){let work=false;for(let i=0;i<2;i++)for(const value of pending[i].splice(0)){handlers[i].message(value);work=true;}if(!work)return;}throw Error('New settings handshake did not drain');};
 handlers.forEach((h,i)=>h.open(transport[i]));flush();
 equal(startMessage.civilizations,[0,6,5,2],'Host honors guest own choice and resolves the exact playing-seat civilization array');
 equal([startMessage.ai,startMessage.epoch,startMessage.finalEpoch,startMessage.popLimit,startMessage.supplies,startMessage.visibility],[2,5,13,1000,2,0],'Host transfers actual complete skirmish choices');
 equal([peers[0].draws(),peers[1].draws()],[3,0],'Host resolves civ then start then end from the shared presentation sequence');
 for(const p of peers){check(p.api.getNet()?.ls instanceof Lockstep);equal(Array.from({length:4},(_,i)=>p.w.player_civilization(i)),[0,6,5,2]);equal(p.w.stat(0,0),5);equal(p.w.match_final_epoch(),13);}
 for(let tick=0;tick<100;tick++){flush();for(const p of peers)check(p.api.getNet().ls.step());flush();}
 equal(peers[0].w.state_hash(),peers[1].w.state_hash(),'Four-civilization actual worlds retain a common peer hash');
}
for(const initial of [-1,0,6,14])for(const final of [-1,0,6,14]){
 if(initial>=0&&final>=0&&initial>final)continue;
 for(const [first,second] of [[0,0],[1,32767],[32767,1],[32767,32767]]){
  let draws=0;
  const a=initial===-1?1+first%(final===-1?15:final+1):initial+1;
  const b=final===-1?a+(initial===-1?second:first)%(16-a):final+1;
  equal(resolveEpochRange(initial,final,15,()=>[first,second][draws++]),{epoch:a-1,finalEpoch:b-1});
  equal(draws,Number(initial===-1)+Number(final===-1));
 }
}
const validCfg={t:'start',protocol:PROTOCOL_VERSION,rules:2,seed:713,ai:0,epoch:0,finalEpoch:14,popLimit:500,supplies:1,visibility:2,civilizations:[0,1,14,14],seasons:normalizeSeasonSettings({cycle:true,startMonth:6,minutesPerSeason:5})};
const bytesOf=w=>{const p=w.save();return new Uint8Array(w.memory.buffer,p,w.save_len()).slice();};
for(const change of [c=>delete c.civilizations,c=>c.civilizations=[0,1,0,14],c=>c.civilizations=[0,15,14,14],c=>c.civilizations=[14,1,14,14],c=>c.civilizations=[0,1,14],c=>c.civilizations='0,1,14,14',c=>c.civilizations=[0,'1',14,14],c=>c.civilizations=[0,1,-1,14],c=>c.epoch=15,c=>c.epoch=-1,c=>c.epoch='0',c=>c.popLimit=999,c=>c.popLimit='500',c=>c.supplies=3,c=>c.visibility=3,c=>c.ai=3,c=>c.seed=0,c=>c.seasons=null,c=>delete c.finalEpoch,c=>c.finalEpoch=15,c=>c.finalEpoch='14',c=>{c.epoch=3;c.finalEpoch=2;}]){
 const e=await environment('up15',1),handler=e.api.channel(1),ch={readyState:'open',send(){},close(){this.readyState='closed';handler.close();}};
 e.api.start(1,917,0);e.w.tick();handler.open(ch);const before=bytesOf(e.w),hash=e.w.state_hash();const cfg=structuredClone(validCfg);change(cfg);handler.message(JSON.stringify(cfg));
 equal(ch.readyState,'closed','Invalid new-match choice closes transport before starting');check(!e.api.getNet());equal(e.w.state_hash(),hash);equal(bytesOf(e.w),before,'Malformed match settings preserve complete world bytes');
}
for(const badChoice of [undefined,null,14,16,-1,.5,'1']){
 const e=await environment('up15',0),handler=e.api.channel(0),ch={readyState:'open',send(){},close(){this.readyState='closed';handler.close();}};handler.open(ch);
 e.api.start(1,917,0);e.w.tick();const before=bytesOf(e.w);handler.message(JSON.stringify({t:'hello',protocol:PROTOCOL_VERSION,rules:2,civilizationChoice:badChoice}));
 equal(ch.readyState,'closed');check(!e.api.getNet());equal(bytesOf(e.w),before,'Invalid guest civilization does not initialize a host game');
}
assert.throws(()=>assertRulesProfile({ruleset:()=>2,schema_revision:()=>8,definition_count:()=>61},profiles.up15),/Speicher-\/Einheitenformat/);checks++;
for(const [name,digest]of Object.entries(sourceHashes))equal(sha(await fs.readFile(new URL('docs/'+name,root))),digest,'Source remains frozen through profile QA');
const report={format:'openepoch-rules-profile-integration-tests-v2',passed:true,checks,productionUp15Enabled:UP15_PROFILE_ENABLED,sourceHashes,binaries:Object.fromEntries(Object.entries(binaries).map(([id,b])=>[id,{file:binaryFiles[id],bytes:b.length,sha256:sha(b)}])),network,openedDatabases:[...new Set(opened)],protocol:PROTOCOL_VERSION,
 scope:'Actual profile resolver/one-file WASM loader and actual main start/db/dbDo/save/load/Continue/channel functions evaluated in vm. Three real WASM rulesets and six same-profile Lockstep peers; DOM, IndexedDB storage-event transport and queued channel transport substituted. No browser/WebRTC socket/original EXE launched.',
 limitations:['The in-memory IndexedDB event adapter validates production namespace/transaction choices, not browser disk persistence.','Native HUD interaction/source crop checks are separate; this test checks that both EE2 numerical profiles keep native HUD loading.']};
const reportName=process.argv.find(arg=>arg.startsWith('--report='))?.slice(9)||'rules-profile-tests.json';
if(!/^[a-z0-9-]+\.json$/i.test(reportName))throw Error('Invalid report filename');
await fs.mkdir(analysis,{recursive:true});await fs.writeFile(new URL(reportName,analysis),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({passed:true,checks,profiles:network,sourceHashes,productionUp15Enabled:UP15_PROFILE_ENABLED}));
