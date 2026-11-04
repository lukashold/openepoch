// Execute the actual main.js channel and stepping functions with real WASMs.
// Only DOM/status/begin/camera surfaces and the queued data-channel transport
// are substituted. Calendar, lockstep, commands, saves and render modules run.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
import {Lockstep,PROTOCOL_VERSION} from '../docs/net.js';
import {createSeasonSession,peerSeasonSettings,savedSeasonSettings,LEGACY_SEASON_SETTINGS} from '../docs/season-session.js';
import {normalizeSeasonSettings,MINUTES_PER_SEASON} from '../docs/original-seasons.js';
import {loadOriginalTerrain} from '../docs/original-terrain.js';
import {loadOriginalGround} from '../docs/original-ground.js';
import {RecordingGL} from './recording-webgl.mjs';
import * as sfx from '../docs/audio.js';

const root=new URL('../',import.meta.url),analysis=new URL('../../EE2-Research/analysis/',import.meta.url);
const sourceNames=['docs/main.js','docs/season-session.js','docs/net.js','docs/index.html','docs/original-seasons.js','docs/original-terrain.js','docs/original-ground.js'];
const sha=value=>createHash('sha256').update(value).digest('hex');
const sourceBytes=Object.fromEntries(await Promise.all(sourceNames.map(async name=>[name,await fs.readFile(new URL(name,root))])));
const sources=Object.fromEntries(Object.entries(sourceBytes).map(([name,bytes])=>[name,sha(bytes)]));
const main=sourceBytes['docs/main.js'].toString(),html=sourceBytes['docs/index.html'].toString();
const wasmBytes=await fs.readFile(new URL('docs/openepoch-ee2.wasm',root)),wasmSha256=sha(wasmBytes);
const segment=(start,end)=>{const a=main.indexOf(start),b=main.indexOf(end,a);assert.ok(a>=0&&b>a,`Source block ${start}`);return main.slice(a,b);};
const controlsSource=segment('function menuSeasonSettings()',"$('season-options').hidden");
const channelSource=segment('function channel(local)',"$('mpopen').onclick");
const advanceSource=segment('function advance(dt, now)', '// A hidden tab gets no animation frames');
const seedSource=main.match(/^const seedOf = .*$/m)?.[0],pauseSource=main.match(/^function togglePause\(\).*$/m)?.[0];
assert.ok(seedSource&&pauseSource,'Production seed/pause functions found');
let checks=0;
const check=(value,label)=>{assert.ok(value,label);checks++;};
const equal=(a,b,label)=>{assert.deepEqual(a,b,label);checks++;};
const plain=value=>JSON.parse(JSON.stringify(value));
const oldFetch=globalThis.fetch,oldBitmap=globalThis.createImageBitmap,oldWindow=globalThis.window;
globalThis.fetch=async url=>{assert.equal(new URL(url).protocol,'file:');const path=fileURLToPath(url),bytes=await fs.readFile(path);return {ok:true,status:200,json:async()=>JSON.parse(bytes),blob:async()=>({bytes,url:path})};};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
// Real audio code sees an ordinary empty DOM window, with no AudioContext.
// No audio device or browser is created; optional simulation sounds stay silent.
globalThis.window={};

try{
 const teams=Array.from({length:8},()=>[1,1,1]);
 const renderer=async()=>{const gl=new RecordingGL(),terrain=await loadOriginalTerrain(gl,true,teams),ground=await loadOriginalGround(gl,true);check(terrain.enabled&&ground.enabled,'Production terrain/ground load');check(!terrain.errors.length&&!ground.errors.length,'Production render inputs have no errors');return {gl,terrain,ground};};
 const renderers=await Promise.all([renderer(),renderer()]);
 const makeWasm=async()=>{const wasm=(await WebAssembly.instantiate(wasmBytes,{})).instance.exports;check(typeof wasm.game_tick==='function','Actual clock export');return wasm;};
 const hostSettings={cycle:true,startMonth:2,minutesPerSeason:1},guestSettings={cycle:false,startMonth:10,minutesPerSeason:30};
 async function environment(local,settings,render=renderers[local]){
  const wasm=await makeWasm(),elements=new Map(),statuses=[],toasts=[],begins=[];
  const $=id=>{if(!elements.has(id))elements.set(id,{value:'',checked:false,disabled:false,textContent:'',style:{}});return elements.get(id);};
  $('seed').value=local?'99':'7';$('mpai').value=local?'2':'0';$('map-visibility').value=local?'0':'2';
  $('season-cycle').checked=settings.cycle;$('season-start').value=String(settings.startMonth);$('season-length').value=String(settings.minutesPerSeason);
  const session=createSeasonSession(wasm,render.terrain,render.ground),cam={x:0,y:0,zoom:28};
  const context=vm.createContext({wasm,Lockstep,PROTOCOL_VERSION,seasonSession:session,peerSeasonSettings,savedSeasonSettings,normalizeSeasonSettings,referenceMode:true,
   $,cam,performance,sfx,mpStatus:text=>statuses.push(text),toast:text=>toasts.push(text),begin:()=>{begins.push(wasm.game_tick());wasm.events_clear();}});
  vm.runInContext(`const TICK=.05;let net=null,bench=0,paused=false,speed=1,acc=0,simMs=0,waitSince=0;
   const f32=(ptr,len)=>new Float32Array(wasm.memory.buffer,ptr,len);
   ${controlsSource}\n${seedSource}\n${pauseSource}\n${advanceSource}\n${channelSource}
   globalThis.api={channel,advance,togglePause,menuSeasonSettings,setSeasonControls,getNet:()=>net,getPaused:()=>paused,
    detach:()=>{net=null;acc=0;},restoreSettings:record=>{seasonSession.configure(savedSeasonSettings(record));setSeasonControls(seasonSession.settings);}};`,context,{filename:'main.js actual channel/advance extraction'});
  return {local,wasm,$,session,cam,statuses,toasts,begins,api:context.api,context};
 }
 const peers=await Promise.all([environment(0,hostSettings),environment(1,guestSettings)]),queues=[[],[]],traffic={json:0,binary:0,binaryCommandPackets:0};
 const handlers=peers.map(peer=>peer.api.channel(peer.local));
 const transports=peers.map((peer,seat)=>({readyState:'open',send(data){if(typeof data==='string'){traffic.json++;queues[1-seat].push(data);}else{traffic.binary++;if(data.byteLength>12)traffic.binaryCommandPackets++;queues[1-seat].push(data.slice(0));}},close(){this.readyState='closed';handlers[seat].close();}}));
 const flush=()=>{let delivered=0;for(let guard=0;guard<100;guard++){let any=false;for(let seat=0;seat<2;seat++)for(const data of queues[seat].splice(0)){handlers[seat].message(data);delivered++;any=true;}if(!any)return delivered;}throw Error('Queued channel did not drain');};
 for(let seat=0;seat<2;seat++){handlers[seat].setPc({close(){}});handlers[seat].open(transports[seat]);}
 flush();
 for(const peer of peers){check(peer.api.getNet()?.ls instanceof Lockstep,'Actual main channel creates the production Lockstep');check(peer.begins.length===1,'One match starts');equal(peer.session.settings,hostSettings,'Guest adopts all host calendar fields');equal(plain(peer.api.menuSeasonSettings()),hostSettings,'Production controls reflect host settings');equal(peer.wasm.game_tick(),0,'Handshake does not advance simulation');}
 equal(peers[0].wasm.state_hash(),peers[1].wasm.state_hash(),'Host seed/AI/visibility override different guest menu values');
 check(PROTOCOL_VERSION===9,'Current shared season protocol is9');
 // Issue genuine commands through both WASM outboxes, not artificial packets.
 for(const peer of peers){
  const w=peer.wasm,id=w.select_idle_worker();check(id!==0xffffffff,'Real worker available');
  w.select_none();w.select_point(w.ent_x(id),w.ent_y(id),0,0,0,0,256,256);w.cmd_context(w.start_x()+6,w.start_y()+4,0);
  check(w.outbox_len()>0,'Actual player command enters the lockstep outbox');
 }
 for(let iteration=0;iteration<700;iteration++){
  flush();for(const peer of peers)peer.api.advance(.05,iteration*50);flush();
  peers[0].session.update();if(iteration%17===0)peers[1].session.update();
 }
 for(const peer of peers)check(peer.wasm.game_tick()===700,'Production advance steps actual network ticks');
 equal(peers[0].wasm.state_hash(),peers[1].wasm.state_hash(),'Genuine commands remain synchronized');
 equal(peers[0].session.update(),peers[1].session.update(),'Different display frequencies share the exact calendar phase');
 check(traffic.binaryCommandPackets>=2,'Actual encoded command packets crossed the queued channel');
 for(const peer of peers)check(!peer.api.getNet().ls.desync,'Production lockstep reports no desync');
 peers[0].api.togglePause();check(!peers[0].api.getPaused(),'Production multiplayer pause guard remains in force');
 // Holding the other peer eventually exhausts the real delay window.
 const leader=peers[0],follower=peers[1];let waiting=false;
 for(let i=0;i<20;i++)if(!leader.api.getNet().ls.step()){waiting=true;break;}
 check(waiting&&leader.api.getNet().ls.waiting,'Real lockstep waits for missing peer input');
 const heldTick=leader.wasm.game_tick(),heldPhase=leader.session.update();
 for(let i=0;i<20;i++){check(!leader.api.getNet().ls.step(),'Missing input cannot advance');equal(leader.session.update(),heldPhase,'Network wait freezes the calendar');}
 equal(leader.wasm.game_tick(),heldTick,'Waiting does not alter the actual tick');
 const target=Math.max(...peers.map(peer=>peer.wasm.game_tick()))+200;
 for(let guard=0;peers.some(peer=>peer.wasm.game_tick()<target);guard++){
  check(guard<1000,'Peers recover after delayed packets');flush();
  for(const peer of peers)if(peer.wasm.game_tick()<target)peer.api.getNet().ls.step();
 }
 flush();equal(leader.wasm.state_hash(),follower.wasm.state_hash(),'Peers recover without losing deterministic state');
 equal(leader.session.update(),follower.session.update(),'Calendar agrees after network recovery');
 // Repeated starts cannot reinitialize a running peer.
 const liveTick=follower.wasm.game_tick();handlers[1].message(JSON.stringify({t:'start',protocol:9,rules:follower.wasm.ruleset(),seed:123,ai:0,visibility:2,seasons:guestSettings}));
 equal(follower.wasm.game_tick(),liveTick,'Duplicate start does not reset a match');equal(follower.session.settings,hostSettings,'Duplicate start cannot replace the host calendar');
 // Exercise the actual single-player pause branch and actual WASM serialization.
 leader.api.detach();leader.wasm.events_clear();leader.api.togglePause();check(leader.api.getPaused(),'Production pause toggles in a detached single-player stepping fixture');
 const pausedTick=leader.wasm.game_tick(),pausedPhase=leader.session.update();
 for(let i=0;i<20;i++)leader.api.advance(.1,i*100);
 equal(leader.wasm.game_tick(),pausedTick,'Actual paused advance never calls tick');equal(leader.session.update(),pausedPhase,'Paused rendering keeps its phase');
 leader.api.togglePause();leader.api.advance(.05,2100);equal(leader.wasm.game_tick(),pausedTick+1,'Unpause resumes actual stepping');
 const saveTick=leader.wasm.game_tick(),saveHash=leader.wasm.state_hash(),ptr=leader.wasm.save(),saveBytes=new Uint8Array(leader.wasm.memory.buffer,ptr,leader.wasm.save_len()).slice();
 const record={bytes:saveBytes,seasons:plain(leader.session.settings)};const savedPhase=leader.session.update();
 leader.$('season-start').value='11';leader.$('season-length').value='30';leader.$('season-cycle').checked=false;
 equal(leader.session.settings,hostSettings,'Changing menu preferences does not mutate the active game calendar');
 for(let i=0;i<50;i++)leader.api.advance(.05,2200+i*50);
 const loadPtr=leader.wasm.load_alloc(saveBytes.length);new Uint8Array(leader.wasm.memory.buffer,loadPtr,saveBytes.length).set(saveBytes);
 check(leader.wasm.load_apply()===1,'Actual WASM save loads');leader.api.restoreSettings(record);
 equal(leader.wasm.game_tick(),saveTick,'Saved tick is restored');equal(leader.wasm.state_hash(),saveHash,'Actual world state is restored');equal(leader.session.update(),savedPhase,'Saved calendar phase is restored');
 equal(plain(leader.api.menuSeasonSettings()),hostSettings,'Production restore helper updates controls');
 equal(savedSeasonSettings({bytes:saveBytes}),LEGACY_SEASON_SETTINGS,'Legacy saves explicitly keep fixed summer');
 leader.api.restoreSettings({bytes:saveBytes});const legacy=leader.session.update();for(let i=0;i<50;i++)leader.wasm.tick();equal(leader.session.update(),legacy,'Legacy fixed-summer save does not begin cycling');
 for(const bad of [null,{},[],{cycle:true},{cycle:'true',startMonth:0,minutesPerSeason:5},{cycle:true,startMonth:12,minutesPerSeason:5},{cycle:true,startMonth:0,minutesPerSeason:6}]){assert.throws(()=>peerSeasonSettings(bad));checks++;assert.throws(()=>savedSeasonSettings({seasons:bad}));checks++;}

 const validStart={t:'start',protocol:PROTOCOL_VERSION,rules:leader.wasm.ruleset(),seed:7,ai:0,visibility:2,seasons:hostSettings};
 const invalidStarts=[
  {...validStart,protocol:8},{...validStart,protocol:undefined},{...validStart,rules:validStart.rules^1},
  ...[3,-1,'2'].map(visibility=>({...validStart,visibility})),
  ...[0,-1,0x100000000,1.5,'7'].map(seed=>({...validStart,seed})),
  ...[3,-1,'0'].map(ai=>({...validStart,ai})),
  ...[undefined,null,{},[],{cycle:true,startMonth:0},{cycle:true,startMonth:13,minutesPerSeason:5},{cycle:true,startMonth:0,minutesPerSeason:6}].map(seasons=>({...validStart,seasons})),
 ];
 let rejections=0;
 for(const message of [...invalidStarts.map(value=>JSON.stringify(value)),'{broken','null','[]','5']){
  const peer=await environment(1,guestSettings),handler=peer.api.channel(1),channel={readyState:'open',send(){},close(){this.readyState='closed';handler.close();}};
  handler.open(channel);handler.message(message);check(channel.readyState==='closed','Invalid start data closes the channel');check(!peer.api.getNet()&&peer.begins.length===0,'Rejected metadata cannot initialize WASM');
  handler.message(JSON.stringify(validStart));check(!peer.api.getNet(),'A rejected channel cannot be revived by a subsequent valid packet');rejections++;
 }
 for(const hello of [{t:'hello',protocol:8,rules:validStart.rules},{t:'hello',protocol:PROTOCOL_VERSION,rules:validStart.rules^1}]){
  const host=await environment(0,hostSettings),handler=host.api.channel(0),channel={readyState:'open',send(){},close(){this.readyState='closed';handler.close();}};
  handler.open(channel);handler.message(JSON.stringify(hello));check(channel.readyState==='closed'&&!host.api.getNet(),'Host rejects old protocol or different rules before launch');rejections++;
 }
 for(const id of ['season-options','season-start','season-cycle','season-length'])check((html.match(new RegExp(`id="${id}"`,'g'))||[]).length===1,'Season control ID is unique');
 const options=id=>[...html.match(new RegExp(`<select id="${id}"[\\s\\S]*?</select>`))[0].matchAll(/<option value="([^"]+)"/g)].map(match=>Number(match[1]));
 equal(options('season-start'),Array.from({length:12},(_,i)=>i),'All twelve months are selectable');equal(options('season-length'),MINUTES_PER_SEASON,'Menu uses the native duration choices');
 check(html.includes('Gebäudeböden'),'Menu describes the implemented building-ground scope');
 for(const [name,hash]of Object.entries(sources))check(sha(await fs.readFile(new URL(name,root)))===hash,'Runtime source stayed unchanged during the test');
 check(sha(await fs.readFile(new URL('docs/openepoch-ee2.wasm',root)))===wasmSha256,'WASM stayed unchanged during the test');
 const report={format:'openepoch-season-session-tests-v1',passed:true,checks,wasmSha256,sources,protocol:PROTOCOL_VERSION,
  sourceExtraction:{channelSha256:sha(channelSource),controlsSha256:sha(controlsSource),advanceSha256:sha(advanceSource),pauseSha256:sha(pauseSource)},
  hostSettings,guestMenuBefore:guestSettings,transferredSettings:hostSettings,traffic,rejectedCases:rejections,
  actualNetworkTick:target,actualSaveTick:saveTick,legacyFixedSummer:true,sourceSaveLoadPhaseStable:true,
  scope:'Actual main.js channel, controls, seed, pause and advance functions evaluated in vm; two real WASMs and two production Net.Locksteps. Production calendar and terrain/ground session methods execute. DOM/status/begin/camera surfaces and queued transport are substituted; no browser, WebRTC sockets, original executable or GUI is launched.',
  limitations:['Actual WASM serializer and production season metadata helpers are tested; IndexedDB/browser save-dialog interaction is not.','Queued transport validates channel protocol and lockstep behavior, not Internet/WebRTC connectivity.','Rendering modules use the recording GL adapter here; separate native GLES world/pixel tests cover actual GPU rendering.']};
 const reportName=process.argv.find(arg=>arg.startsWith('--report='))?.slice(9)||'season-session-tests.json';
 if(!/^[a-z0-9-]+\.json$/i.test(reportName))throw Error('Invalid report filename');
 await fs.writeFile(new URL(reportName,analysis),JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify(report,null,2));
}finally{globalThis.fetch=oldFetch;globalThis.createImageBitmap=oldBitmap;if(oldWindow===undefined)delete globalThis.window;else globalThis.window=oldWindow;}
